#![forbid(unsafe_code)]

//! Optional, presentation-only voxel view for Crystal's Bevy shell.

mod camera;
mod footing;
mod mesh;
mod profile;

use std::collections::{HashMap, HashSet};

use bevy::{
    asset::{AssetId, load_internal_asset},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::*,
    render::{
        camera::{ClearColorConfig, OrthographicProjection, Projection, RenderTarget, ScalingMode},
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{
            AsBindGroup, CompareFunction, DepthStencilState, Face, RenderPipelineDescriptor,
            ShaderRef, SpecializedMeshPipelineError,
        },
        view::RenderLayers,
    },
};
use crystal_render_api::{VisualActor, VisualActorId, VisualWorldFrame, WorldRenderSet};

pub use camera::{CAMERA_PITCH_DEGREES, VoxelCameraPose, camera_pose};
pub use footing::{actor_foot, footing_height, tile_at_visual_point, visual_point_to_voxel};
pub use mesh::{
    SurfaceMeshData, TerrainMeshData, TerrainMeshError, build_terrain_mesh,
    build_terrain_mesh_with_images,
};
pub use profile::{
    COMPACT_BUILDING_HEIGHT, CellShape, GROUND_HEIGHT, LARGE_BUILDING_HEIGHT, MAX_PROFILE_HEIGHT,
    MIN_PROFILE_HEIGHT, SOURCE_TILE_HEIGHT, SolidKind, WATER_HEIGHT, shape_for_source,
    support_height, supports_frame_profile,
};

pub const EXPECTED_GRID_SIZE: UVec2 = UVec2::new(20, 18);
/// Layer used by the host's faithful world while 2.5D is active. A dedicated
/// camera draws it first, providing exact coverage outside the pitched mesh
/// and during viewport scrolling without inventing geometry.
pub const CLASSIC_FALLBACK_RENDER_LAYER: usize = 30;

const VOXEL_RENDER_LAYER: usize = 31;
const NORMAL_ACTOR_CAMERA_PULL: f32 = 0.05;
const ABOVE_PRIORITY_CAMERA_PULL: f32 = 0.75;
const SILHOUETTE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x7a88_22d4_d773_4874_94d3_c20a_0dcc_f21a);
#[derive(Clone, Copy, Debug, Default)]
pub struct VoxelViewPlugin;

impl Plugin for VoxelViewPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SILHOUETTE_SHADER_HANDLE,
            "occlusion_silhouette.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<OcclusionSilhouetteMaterial>::default());
        app.init_resource::<VoxelViewSettings>()
            .init_resource::<VoxelViewStatus>()
            .init_resource::<VoxelScene>()
            .init_resource::<TerrainRevisionCache>()
            .init_resource::<ActorIdCache>()
            .init_resource::<PlayerSilhouetteCache>()
            .add_systems(Startup, setup_voxel_view)
            .add_systems(Update, toggle_voxel_view.before(sync_voxel_view))
            .add_systems(Update, sync_voxel_view.in_set(WorldRenderSet::RenderSync))
            .add_systems(
                Update,
                sync_player_silhouette_system
                    .after(sync_voxel_view)
                    .in_set(WorldRenderSet::RenderSync),
            );
    }
}

/// Runtime presentation switch used by the location tester and optional-mod
/// builds. It cannot affect simulation because only the renderer reads it.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoxelViewSettings {
    pub enabled: bool,
    pub allow_f3_toggle: bool,
}

/// Observable presentation state for the developer location tester. Gameplay
/// never reads this resource.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct VoxelViewStatus {
    pub active: bool,
    pub active_frames: u32,
    pub fallback_reason: Option<String>,
}

impl Default for VoxelViewSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_f3_toggle: true,
        }
    }
}

fn toggle_voxel_view(keyboard: Res<ButtonInput<KeyCode>>, mut settings: ResMut<VoxelViewSettings>) {
    if settings.allow_f3_toggle && keyboard.just_pressed(KeyCode::F3) {
        settings.enabled = !settings.enabled;
    }
}

#[derive(Component)]
struct VoxelWorldCamera;

#[derive(Component)]
struct VoxelClassicFallbackCamera;

#[derive(Component)]
struct VoxelTerrain;

#[derive(Component)]
struct VoxelActorCard;

#[derive(Component)]
struct VoxelActorSilhouette;

type VoxelWorldCameraFilter = (
    With<VoxelWorldCamera>,
    Without<VoxelTerrain>,
    Without<VoxelActorCard>,
);
type VoxelTerrainFilter = (With<VoxelTerrain>, Without<VoxelActorCard>);

#[derive(Resource, Default)]
struct VoxelScene {
    camera: Option<Entity>,
    actor_quad: Option<Handle<Mesh>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerrainCacheKey {
    revision: u64,
    texture: AssetId<Image>,
    viewport_bits: [u32; 2],
    tile_bits: [u32; 2],
    grid_size: UVec2,
}

impl TerrainCacheKey {
    fn from_frame(frame: &VisualWorldFrame) -> Self {
        Self {
            revision: frame.terrain_revision,
            texture: frame.map_texture.id(),
            viewport_bits: [
                frame.viewport_size.x.to_bits(),
                frame.viewport_size.y.to_bits(),
            ],
            tile_bits: [frame.tile_size.x.to_bits(), frame.tile_size.y.to_bits()],
            grid_size: frame.grid_size,
        }
    }
}

#[derive(Resource, Default)]
struct TerrainRevisionCache {
    key: Option<TerrainCacheKey>,
    textured_entity: Option<Entity>,
    solid_entity: Option<Entity>,
    textured_mesh: Option<Handle<Mesh>>,
    solid_mesh: Option<Handle<Mesh>>,
    textured_material: Option<Handle<StandardMaterial>>,
    solid_material: Option<Handle<StandardMaterial>>,
}

struct ActorTextureAssets {
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct ActorIdCache {
    entities: HashMap<VisualActorId, Entity>,
    textures: HashMap<AssetId<Image>, ActorTextureAssets>,
}

#[derive(Resource, Default)]
struct PlayerSilhouetteCache {
    entity: Option<Entity>,
    texture: Option<AssetId<Image>>,
    material: Option<Handle<OcclusionSilhouetteMaterial>>,
}

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
struct OcclusionSilhouetteMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[uniform(2)]
    color: LinearRgba,
}

impl Material for OcclusionSilhouetteMaterial {
    fn fragment_shader() -> ShaderRef {
        SILHOUETTE_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            // Bevy's 3D camera uses reversed depth. Strict Less draws the
            // silhouette only where a closer terrain depth already exists;
            // equal-depth pixels from the normal player card remain untouched.
            configure_silhouette_depth(depth_stencil);
        }
        Ok(())
    }
}

fn configure_silhouette_depth(depth_stencil: &mut DepthStencilState) {
    depth_stencil.depth_compare = CompareFunction::Less;
    depth_stencil.depth_write_enabled = false;
}

#[allow(clippy::too_many_arguments)]
fn setup_voxel_view(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut scene: ResMut<VoxelScene>,
) {
    let initial_viewport = Vec2::new(160.0, 144.0);
    let pose = camera_pose(initial_viewport);
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                order: -1,
                ..default()
            },
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize(1.0),
                ..default()
            },
            ..default()
        },
        RenderLayers::layer(CLASSIC_FALLBACK_RENDER_LAYER),
        VoxelClassicFallbackCamera,
    ));
    let camera = commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    order: 0,
                    is_active: false,
                    target: RenderTarget::Window(bevy::window::WindowRef::Primary),
                    // The dedicated camera above already drew the faithful 2D
                    // world. Preserve it anywhere the pitched mesh has no
                    // coverage instead of clearing to a letterbox color.
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                projection: Projection::Orthographic(OrthographicProjection {
                    near: 0.1,
                    far: 4096.0,
                    scaling_mode: ScalingMode::Fixed {
                        width: initial_viewport.x,
                        height: initial_viewport.y,
                    },
                    ..default()
                }),
                transform: pose.transform(),
                tonemapping: Tonemapping::None,
                deband_dither: DebandDither::Disabled,
                ..default()
            },
            RenderLayers::layer(VOXEL_RENDER_LAYER),
            VoxelWorldCamera,
        ))
        .id();
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::srgb(1.0, 0.93, 0.82),
                illuminance: 3_000.0,
                shadows_enabled: true,
                shadow_depth_bias: 0.01,
                shadow_normal_bias: 0.8,
            },
            // A southeast light gives the small world readable northwest cast
            // shadows while keeping the source sprites' front faces bright.
            transform: Transform::from_xyz(-180.0, 320.0, -220.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        RenderLayers::layer(VOXEL_RENDER_LAYER),
    ));

    scene.camera = Some(camera);
    scene.actor_quad = Some(meshes.add(actor_quad_mesh()));
}

#[allow(clippy::too_many_arguments)]
fn sync_voxel_view(
    frame: Res<VisualWorldFrame>,
    settings: Res<VoxelViewSettings>,
    mut status: ResMut<VoxelViewStatus>,
    mut last_failure: Local<Option<String>>,
    mut commands: Commands,
    scene: Res<VoxelScene>,
    mut terrain_cache: ResMut<TerrainRevisionCache>,
    mut actor_cache: ResMut<ActorIdCache>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cameras: Query<(&mut Camera, &mut Projection, &mut Transform), VoxelWorldCameraFilter>,
    mut terrain_entities: Query<(&mut Visibility, &mut Transform), VoxelTerrainFilter>,
    mut actor_entities: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Handle<StandardMaterial>,
        ),
        (
            With<VoxelActorCard>,
            Without<VoxelActorSilhouette>,
            Without<VoxelWorldCamera>,
            Without<VoxelTerrain>,
        ),
    >,
) {
    let failure = if !settings.enabled {
        Some("disabled")
    } else if !frame.active {
        Some("waiting for an active world frame")
    } else if frame.validate().is_err() {
        Some("world frame validation failed")
    } else if frame.grid_size != EXPECTED_GRID_SIZE {
        Some("world frame grid is not 20x18")
    } else if !grid_extent_matches_viewport(&frame) {
        Some("world frame extent does not match its viewport")
    } else if !supports_frame_profile(&frame) {
        Some("world frame is not supported by the visual profile")
    } else if !images.contains(&frame.map_texture) {
        Some("composed map texture is unavailable")
    } else {
        None
    };
    let valid = failure.is_none();
    let output_ready = set_output_active(&scene, valid, &mut cameras, &frame);
    if valid && !output_ready {
        let failure = "voxel world camera is unavailable";
        status.active = false;
        status.active_frames = 0;
        status.fallback_reason = Some(failure.to_owned());
        report_fallback_change(&mut last_failure, failure);
        return;
    }
    if !valid {
        let failure = failure.unwrap_or("unknown reason");
        status.active = false;
        status.active_frames = 0;
        status.fallback_reason = Some(failure.to_owned());
        report_fallback_change(&mut last_failure, failure);
        return;
    }

    if let Err(error) = sync_terrain(
        &frame,
        &mut commands,
        &mut terrain_cache,
        &images,
        &mut meshes,
        &mut materials,
        &mut terrain_entities,
    ) {
        let failure = format!("terrain sync failed: {error:?}");
        status.active = false;
        status.active_frames = 0;
        status.fallback_reason = Some(failure.clone());
        report_fallback_change(&mut last_failure, &failure);
        set_output_active(&scene, false, &mut cameras, &frame);
        return;
    }

    let Some(actor_quad) = scene.actor_quad.as_ref() else {
        status.active = false;
        status.active_frames = 0;
        status.fallback_reason = Some("actor card mesh is unavailable".to_owned());
        report_fallback_change(&mut last_failure, "actor card mesh is unavailable");
        set_output_active(&scene, false, &mut cameras, &frame);
        return;
    };
    if let Err(error) = sync_actor_cards(
        &frame,
        actor_quad,
        &mut commands,
        &mut actor_cache,
        &images,
        &mut materials,
        &mut actor_entities,
    ) {
        let failure = format!("actor sync failed: {error:?}");
        status.active = false;
        status.active_frames = 0;
        status.fallback_reason = Some(failure.clone());
        report_fallback_change(&mut last_failure, &failure);
        set_output_active(&scene, false, &mut cameras, &frame);
        return;
    }
    status.active = true;
    status.active_frames = status.active_frames.saturating_add(1);
    status.fallback_reason = None;
    last_failure.take();
}

fn report_fallback_change(last_failure: &mut Option<String>, failure: &str) {
    if last_failure.as_deref() != Some(failure) {
        if failure != "disabled" {
            bevy::log::warn!("optional 2.5D renderer fell back to the classic view: {failure}");
        }
        *last_failure = Some(failure.to_owned());
    }
}

fn grid_extent_matches_viewport(frame: &VisualWorldFrame) -> bool {
    let extent = frame.tile_size * frame.grid_size.as_vec2();
    (extent - frame.viewport_size).abs().max_element() <= 0.001
}

fn set_output_active(
    scene: &VoxelScene,
    active: bool,
    cameras: &mut Query<(&mut Camera, &mut Projection, &mut Transform), VoxelWorldCameraFilter>,
    frame: &VisualWorldFrame,
) -> bool {
    let mut camera_ready = false;
    if let Some(camera_entity) = scene.camera
        && let Ok((mut camera, mut projection, mut transform)) = cameras.get_mut(camera_entity)
    {
        camera_ready = true;
        // The direct world camera is active only for a complete validated
        // frame; otherwise the untouched classic layer remains authoritative.
        camera.is_active = active;
        if active {
            let pose = camera_pose(frame.viewport_size);
            *transform = pose.transform();
            *projection = Projection::Orthographic(OrthographicProjection {
                near: 0.1,
                far: 4096.0,
                scaling_mode: ScalingMode::Fixed {
                    width: pose.projection_size.x,
                    height: pose.projection_size.y,
                },
                ..default()
            });
        }
    }
    camera_ready
}

#[allow(clippy::too_many_arguments)]
fn sync_terrain(
    frame: &VisualWorldFrame,
    commands: &mut Commands,
    cache: &mut TerrainRevisionCache,
    images: &Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_entities: &mut Query<(&mut Visibility, &mut Transform), VoxelTerrainFilter>,
) -> Result<(), TerrainSyncError> {
    let next_key = TerrainCacheKey::from_frame(frame);
    if cache.key.as_ref() != Some(&next_key) {
        let terrain =
            build_terrain_mesh_with_images(frame, images).map_err(TerrainSyncError::Mesh)?;
        let (textured_mesh, solid_mesh) = terrain.into_meshes();

        let textured_mesh_handle = update_mesh_asset(
            meshes,
            &mut cache.textured_mesh,
            textured_mesh,
            TerrainSyncError::CachedTexturedMeshUnavailable,
        )?;
        let solid_mesh_handle = update_mesh_asset(
            meshes,
            &mut cache.solid_mesh,
            solid_mesh,
            TerrainSyncError::CachedSolidMeshUnavailable,
        )?;
        let textured_material_handle = if let Some(handle) = cache.textured_material.as_ref() {
            let Some(material) = materials.get_mut(handle) else {
                return Err(TerrainSyncError::CachedTexturedMaterialUnavailable);
            };
            material.base_color_texture = Some(frame.map_texture.clone());
            handle.clone()
        } else {
            let handle = materials.add(textured_terrain_material(frame.map_texture.clone()));
            cache.textured_material = Some(handle.clone());
            handle
        };
        let solid_material_handle = if let Some(handle) = cache.solid_material.as_ref() {
            if materials.get(handle).is_none() {
                return Err(TerrainSyncError::CachedSolidMaterialUnavailable);
            }
            handle.clone()
        } else {
            let handle = materials.add(solid_terrain_material());
            cache.solid_material = Some(handle.clone());
            handle
        };

        if cache.textured_entity.is_none() {
            cache.textured_entity = Some(spawn_terrain_entity(
                commands,
                textured_mesh_handle,
                textured_material_handle,
                frame,
            ));
        }
        if cache.solid_entity.is_none() {
            cache.solid_entity = Some(spawn_terrain_entity(
                commands,
                solid_mesh_handle,
                solid_material_handle,
                frame,
            ));
        }

        cache.key = Some(next_key);
    }

    for entity in [cache.textured_entity, cache.solid_entity]
        .into_iter()
        .flatten()
    {
        if let Ok((mut visibility, mut transform)) = terrain_entities.get_mut(entity) {
            *visibility = Visibility::Visible;
            *transform = terrain_transform(frame);
        }
    }
    Ok(())
}

fn update_mesh_asset(
    meshes: &mut Assets<Mesh>,
    cache: &mut Option<Handle<Mesh>>,
    mesh: Mesh,
    unavailable: TerrainSyncError,
) -> Result<Handle<Mesh>, TerrainSyncError> {
    if let Some(handle) = cache.as_ref() {
        let Some(asset) = meshes.get_mut(handle) else {
            return Err(unavailable);
        };
        *asset = mesh;
        Ok(handle.clone())
    } else {
        let handle = meshes.add(mesh);
        *cache = Some(handle.clone());
        Ok(handle)
    }
}

fn spawn_terrain_entity(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    frame: &VisualWorldFrame,
) -> Entity {
    commands
        .spawn((
            PbrBundle {
                mesh,
                material,
                transform: terrain_transform(frame),
                ..default()
            },
            RenderLayers::layer(VOXEL_RENDER_LAYER),
            VoxelTerrain,
        ))
        .id()
}

fn textured_terrain_material(texture: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
        ..default()
    }
}

fn solid_terrain_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: None,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
        ..default()
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_actor_cards(
    frame: &VisualWorldFrame,
    actor_quad: &Handle<Mesh>,
    commands: &mut Commands,
    cache: &mut ActorIdCache,
    images: &Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    actor_entities: &mut Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Handle<StandardMaterial>,
        ),
        (
            With<VoxelActorCard>,
            Without<VoxelActorSilhouette>,
            Without<VoxelWorldCamera>,
            Without<VoxelTerrain>,
        ),
    >,
) -> Result<(), ActorSyncError> {
    // A published frame is atomic. If any visible actor cannot be rendered,
    // hide the entire optional output so the complete stock view remains.
    let mut prepared = Vec::with_capacity(frame.actors.len());
    for actor in &frame.actors {
        if !images.contains(&actor.texture) {
            return Err(ActorSyncError::TextureUnavailable(actor.id));
        }
        let transform =
            actor_transform(frame, actor).ok_or(ActorSyncError::FootingUnavailable(actor.id))?;
        prepared.push((actor, transform));
    }

    let visible_ids: HashSet<_> = frame.actors.iter().map(|actor| actor.id).collect();
    let stale_ids: Vec<_> = cache
        .entities
        .keys()
        .copied()
        .filter(|id| !visible_ids.contains(id))
        .collect();
    for id in stale_ids {
        if let Some(entity) = cache.entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }

    let mut used_textures = HashSet::with_capacity(frame.actors.len());
    for (actor, transform) in prepared {
        let texture_id = actor.texture.id();
        used_textures.insert(texture_id);
        let material = actor_material(actor, cache, materials);

        let existing = cache.entities.get(&actor.id).copied();
        if let Some(entity) = existing
            && let Ok((mut current_transform, mut visibility, mut current_material)) =
                actor_entities.get_mut(entity)
        {
            *current_transform = transform;
            *visibility = Visibility::Visible;
            *current_material = material;
            continue;
        }

        let entity = commands
            .spawn((
                PbrBundle {
                    mesh: actor_quad.clone(),
                    material,
                    transform,
                    ..default()
                },
                RenderLayers::layer(VOXEL_RENDER_LAYER),
                VoxelActorCard,
            ))
            .id();
        cache.entities.insert(actor.id, entity);
    }

    let unused_texture_ids: Vec<_> = cache
        .textures
        .keys()
        .copied()
        .filter(|id| !used_textures.contains(id))
        .collect();
    for id in unused_texture_ids {
        if let Some(assets) = cache.textures.remove(&id) {
            materials.remove(assets.material.id());
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn sync_player_silhouette_system(
    frame: Res<VisualWorldFrame>,
    status: Res<VoxelViewStatus>,
    scene: Res<VoxelScene>,
    mut commands: Commands,
    mut cache: ResMut<PlayerSilhouetteCache>,
    mut materials: ResMut<Assets<OcclusionSilhouetteMaterial>>,
    mut entities: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Handle<OcclusionSilhouetteMaterial>,
        ),
        (
            With<VoxelActorSilhouette>,
            Without<VoxelActorCard>,
            Without<VoxelWorldCamera>,
            Without<VoxelTerrain>,
        ),
    >,
) {
    let Some(actor_quad) = scene.actor_quad.as_ref() else {
        return;
    };
    if !status.active {
        if let Some(entity) = cache.entity
            && let Ok((_, mut visibility, _)) = entities.get_mut(entity)
        {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    sync_player_silhouette(
        &frame,
        actor_quad,
        &mut commands,
        &mut cache,
        &mut materials,
        &mut entities,
    );
}

fn sync_player_silhouette(
    frame: &VisualWorldFrame,
    actor_quad: &Handle<Mesh>,
    commands: &mut Commands,
    cache: &mut PlayerSilhouetteCache,
    materials: &mut Assets<OcclusionSilhouetteMaterial>,
    entities: &mut Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Handle<OcclusionSilhouetteMaterial>,
        ),
        (
            With<VoxelActorSilhouette>,
            Without<VoxelActorCard>,
            Without<VoxelWorldCamera>,
            Without<VoxelTerrain>,
        ),
    >,
) {
    let Some(player) = frame
        .actors
        .iter()
        .find(|actor| actor.id == VisualActorId::Player)
    else {
        if let Some(entity) = cache.entity.take() {
            commands.entity(entity).despawn();
        }
        if let Some(material) = cache.material.take() {
            materials.remove(material.id());
        }
        cache.texture = None;
        return;
    };
    let Some(transform) = actor_transform(frame, player) else {
        return;
    };
    let texture_id = player.texture.id();
    let material = if cache.texture == Some(texture_id) {
        cache.material.clone()
    } else {
        if let Some(previous) = cache.material.take() {
            materials.remove(previous.id());
        }
        let material = materials.add(OcclusionSilhouetteMaterial {
            texture: player.texture.clone(),
            color: LinearRgba::new(0.20, 0.06, 0.32, 0.72),
        });
        cache.texture = Some(texture_id);
        cache.material = Some(material.clone());
        Some(material)
    };
    let Some(material) = material else {
        return;
    };

    if let Some(entity) = cache.entity
        && let Ok((mut current_transform, mut visibility, mut current_material)) =
            entities.get_mut(entity)
    {
        *current_transform = transform;
        *visibility = Visibility::Visible;
        *current_material = material;
        return;
    }
    cache.entity = Some(
        commands
            .spawn((
                MaterialMeshBundle {
                    mesh: actor_quad.clone(),
                    material,
                    transform,
                    ..default()
                },
                RenderLayers::layer(VOXEL_RENDER_LAYER),
                VoxelActorSilhouette,
            ))
            .id(),
    );
}

fn actor_material(
    actor: &VisualActor,
    cache: &mut ActorIdCache,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    if let Some(assets) = cache.textures.get(&actor.texture.id()) {
        return assets.material.clone();
    }
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(actor.texture.clone()),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        unlit: false,
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    });
    cache.textures.insert(
        actor.texture.id(),
        ActorTextureAssets {
            material: material.clone(),
        },
    );
    material
}

fn actor_transform(frame: &VisualWorldFrame, actor: &VisualActor) -> Option<Transform> {
    let foot = actor_foot(actor);
    let height = footing_height(frame, foot)?;
    let mut position = visual_point_to_voxel(foot, height + 0.05);
    let pose = camera_pose(frame.viewport_size);
    let profile_scale = frame.tile_size.y / SOURCE_TILE_HEIGHT;
    let camera_pull = if actor.above_priority {
        ABOVE_PRIORITY_CAMERA_PULL
    } else {
        NORMAL_ACTOR_CAMERA_PULL
    } * profile_scale;
    position += (pose.eye - pose.target).normalize_or_zero() * camera_pull;
    let mut transform = Transform::from_translation(position)
        .with_rotation(camera::card_rotation_toward_camera(pose));
    transform.scale = Vec3::new(
        if actor.flip_x {
            -actor.size.x
        } else {
            actor.size.x
        },
        actor.size.y,
        1.0,
    );
    Some(transform)
}

fn terrain_transform(frame: &VisualWorldFrame) -> Transform {
    Transform::from_xyz(frame.center.x, 0.0, -frame.center.y)
}

fn actor_quad_mesh() -> Mesh {
    use bevy::render::{
        mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology,
    };

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, 0.0, 0.0],
            [0.5, 0.0, 0.0],
            [0.5, 1.0, 0.0],
            [-0.5, 1.0, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerrainSyncError {
    Mesh(TerrainMeshError),
    CachedTexturedMeshUnavailable,
    CachedSolidMeshUnavailable,
    CachedTexturedMaterialUnavailable,
    CachedSolidMaterialUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActorSyncError {
    TextureUnavailable(VisualActorId),
    FootingUnavailable(VisualActorId),
}

#[cfg(test)]
mod renderer_tests {
    use bevy::render::render_resource::{DepthBiasState, StencilState, TextureFormat};

    use super::*;

    #[test]
    fn silhouette_pass_reads_only_strictly_closer_reverse_depth() {
        let mut depth = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

        configure_silhouette_depth(&mut depth);

        assert_eq!(depth.depth_compare, CompareFunction::Less);
        assert!(!depth.depth_write_enabled);
    }

    #[test]
    fn optional_view_remains_disabled_by_default() {
        assert!(!VoxelViewSettings::default().enabled);
    }
}
