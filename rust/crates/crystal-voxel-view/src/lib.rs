#![forbid(unsafe_code)]

//! Optional, presentation-only voxel view for Crystal's Bevy shell.

mod azalea_gym;
mod barn;
mod battle_tower;
mod building_catalog;
mod building_style;
mod cafe;
mod camera;
mod casino;
mod cave;
mod celadon_gym;
mod cerulean_gym;
mod cut_tree;
mod dance_theater;
mod elevation;
mod elite_four_room;
mod facility;
mod facility_divider;
mod flower;
mod flower_shop;
mod footing;
mod forest;
mod fuchsia_gym;
mod gate;
mod goldenrod_underground;
mod grass;
mod hall_of_fame;
mod house;
mod ice_path;
mod interior;
mod johto_fence;
mod kanto_cliff;
mod kanto_post;
mod lab;
mod mart;
mod mesh;
mod modern_route;
mod olivine_gym;
mod park;
mod players_house;
mod pokecenter;
mod pokecom;
mod port;
mod power_plant;
mod profile;
mod rock_platform;
mod rocket_base;
mod ruins_of_alph;
mod saffron_gym;
mod ship;
mod sign;
mod tower;
mod train_station;
mod underground_boundary;
mod underground_path;
mod vermilion;
mod vermilion_gym;
mod violet_gym;
mod viridian_gym;
mod warehouse;
mod waterfall;
mod wise_trios;

use std::collections::{HashMap, HashSet};

#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::AsyncComputeTaskPool;
#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::Task;
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
    tasks::futures_lite::future,
};
use crystal_render_api::{VisualActor, VisualActorId, VisualWorldFrame, WorldRenderSet};

pub use camera::{CAMERA_PITCH_DEGREES, VoxelCameraPose, camera_pose};
pub use footing::{
    actor_foot, footing_height, resolved_footing_height, tile_at_visual_point,
    visual_point_to_voxel,
};
pub use mesh::{
    CellCoverageKind, SurfaceMeshData, TerrainImageSamples, TerrainMeshError, audit_cell_coverage,
    audit_cell_coverage_on_map, build_terrain_mesh, build_terrain_mesh_with_images,
    build_terrain_mesh_with_samples,
};
pub use profile::{
    COMPACT_BUILDING_HEIGHT, CellShape, GROUND_HEIGHT, LARGE_BUILDING_HEIGHT, MAX_PROFILE_HEIGHT,
    MIN_PROFILE_HEIGHT, SOURCE_TILE_HEIGHT, SolidKind, WATER_HEIGHT, shape_for_source,
    support_height, supports_frame_profile,
};

/// Parking layer used to keep the classic overworld out of every active
/// camera while the user has manually selected 2.5D.
pub const HIDDEN_CLASSIC_WORLD_RENDER_LAYER: usize = 30;

const VOXEL_RENDER_LAYER: usize = 31;
/// Logical LCD grid used only to derive source-pixel camera scale. The
/// optional terrain grid may extend beyond it in every direction.
pub const EXPECTED_GRID_SIZE: UVec2 = UVec2::new(20, 18);
const ACTOR_BASE_CAMERA_PULL: f32 = 6.0;
const ACTOR_CARD_HEIGHT: f32 = 16.0;
const ACTOR_FOOT_ANCHOR: f32 = 8.0;
const MIN_PULL_SINE: f32 = 0.2;
const ABOVE_PRIORITY_EXTRA_PULL: f32 = 0.70;
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
            .init_resource::<TerrainBuildQueue>()
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
    pub inactive_reason: Option<String>,
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
    viewport_bits: [u32; 2],
    tile_bits: [u32; 2],
    grid_size: UVec2,
}

impl TerrainCacheKey {
    fn from_frame(frame: &VisualWorldFrame) -> Self {
        Self {
            revision: frame.terrain_revision,
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
    footing_heights: Vec<f32>,
}

#[derive(Resource, Default)]
struct TerrainBuildQueue {
    key: Option<TerrainCacheKey>,
    #[cfg(not(target_arch = "wasm32"))]
    task: Option<Task<TerrainBuildResult>>,
    #[cfg(target_arch = "wasm32")]
    completed: Option<TerrainBuildResult>,
}

struct TerrainBuildResult {
    key: TerrainCacheKey,
    frame: VisualWorldFrame,
    terrain: Result<BuiltTerrain, TerrainMeshError>,
}

struct BuiltTerrain {
    footing_heights: Vec<f32>,
    textured_mesh: Mesh,
    solid_mesh: Mesh,
}

fn should_start_terrain_build(
    cached_key: Option<&TerrainCacheKey>,
    queued_key: Option<&TerrainCacheKey>,
    next_key: &TerrainCacheKey,
) -> bool {
    cached_key != Some(next_key) && queued_key.is_none()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerrainSyncState {
    Ready,
    Pending,
}

fn must_wait_for_initial_terrain(state: TerrainSyncState, cache: &TerrainRevisionCache) -> bool {
    state == TerrainSyncState::Pending && cache.key.is_none()
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
    // The reference renderer assigns every face a readable base shade and
    // layers directional light/AO over it. Bevy's fully shadowed PBR faces
    // otherwise fall almost to black, erasing the mapped building courses
    // on the side opposite the sun. This resource exists only when the
    // optional voxel plugin is installed; faithful 2D sprites are unlit.
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.92, 0.96, 1.0),
        brightness: 220.0,
    });
    let initial_viewport = Vec2::new(160.0, 144.0);
    let pose = camera_pose(initial_viewport);
    let camera = commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    order: 0,
                    is_active: false,
                    target: RenderTarget::Window(bevy::window::WindowRef::Primary),
                    // Full-map terrain extends beyond the LCD crop. Clear the
                    // finite horizon to sky so the faithful 2D viewport can
                    // never leak through as a vertical backdrop behind it.
                    clear_color: ClearColorConfig::Custom(Color::srgb(0.72, 0.83, 0.78)),
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
                shadows_enabled: false,
                shadow_depth_bias: 0.01,
                shadow_normal_bias: 0.8,
            },
            // A southeast light gives the small world readable northwest cast
            // shadows while keeping the source sprites' front faces bright.
            // Voxel world coordinates are +X east and +Z south. Put the sun
            // in that actual quadrant so the camera-facing facade receives
            // direct light, matching the reference renderer's south/east
            // face model. The previous negative X/Z position lit every
            // house from behind and made its mapped windows and door nearly
            // black even though their source pixels were correct.
            transform: Transform::from_xyz(180.0, 320.0, 220.0).looking_at(Vec3::ZERO, Vec3::Y),
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
    mut terrain_builds: ResMut<TerrainBuildQueue>,
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
            &mut Handle<Mesh>,
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
    if status.active
        && !frame.is_changed()
        && !settings.is_changed()
        && terrain_builds.key.is_none()
    {
        // The published frame is retained and Bevy change detection proves
        // there is no camera, actor, texture, or terrain work to synchronize.
        // Avoid validating and profile-scanning the complete terrain grid on
        // every vsynced host frame while the game is visually idle.
        status.active_frames = status.active_frames.saturating_add(1);
        return;
    }
    let failure = if !settings.enabled {
        Some("disabled")
    } else if !frame.active {
        Some("waiting for an active world frame")
    } else if frame.validate().is_err() {
        Some("world frame validation failed")
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
        status.inactive_reason = Some(failure.to_owned());
        report_inactive_change(&mut last_failure, failure);
        return;
    }
    if !valid {
        let failure = failure.unwrap_or("unknown reason");
        status.active = false;
        status.active_frames = 0;
        status.inactive_reason = Some(failure.to_owned());
        report_inactive_change(&mut last_failure, failure);
        return;
    }

    let terrain_state = match sync_terrain(
        &frame,
        &mut commands,
        &mut terrain_cache,
        &mut terrain_builds,
        &images,
        &mut meshes,
        &mut materials,
        &mut terrain_entities,
    ) {
        Ok(state) => state,
        Err(error) => {
            let failure = format!("terrain sync failed: {error:?}");
            status.active = false;
            status.active_frames = 0;
            status.inactive_reason = Some(failure.clone());
            report_inactive_change(&mut last_failure, &failure);
            set_output_active(&scene, false, &mut cameras, &frame);
            return;
        }
    };
    if must_wait_for_initial_terrain(terrain_state, &terrain_cache) {
        // Initial activation has no authored mesh to present yet. Keep the
        // manually selected 2.5D presentation inactive until the first build
        // completes; the classic world remains parked on its hidden layer.
        // Subsequent revisions keep the last complete terrain alive while its
        // replacement builds.
        status.active = false;
        status.active_frames = 0;
        status.inactive_reason = Some("building authored terrain".to_owned());
        set_output_active(&scene, false, &mut cameras, &frame);
        return;
    }

    let Some(actor_quad) = scene.actor_quad.as_ref() else {
        status.active = false;
        status.active_frames = 0;
        status.inactive_reason = Some("actor card mesh is unavailable".to_owned());
        report_inactive_change(&mut last_failure, "actor card mesh is unavailable");
        set_output_active(&scene, false, &mut cameras, &frame);
        return;
    };
    if let Err(error) = sync_actor_cards(
        &frame,
        &terrain_cache.footing_heights,
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
        status.inactive_reason = Some(failure.clone());
        report_inactive_change(&mut last_failure, &failure);
        set_output_active(&scene, false, &mut cameras, &frame);
        return;
    }
    status.active = true;
    status.active_frames = status.active_frames.saturating_add(1);
    status.inactive_reason = None;
    last_failure.take();
}

fn report_inactive_change(last_failure: &mut Option<String>, failure: &str) {
    if last_failure.as_deref() != Some(failure) {
        if failure != "disabled" {
            bevy::log::warn!("optional 2.5D renderer is inactive: {failure}");
        }
        *last_failure = Some(failure.to_owned());
    }
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
            camera.clear_color = ClearColorConfig::Custom(voxel_clear_color(frame));
            let pose = camera_pose(frame.viewport_size);
            *transform = pose.transform();
            *projection = Projection::Perspective(PerspectiveProjection {
                fov: pose.vertical_fov_radians,
                near: pose.near,
                far: pose.far,
                ..default()
            });
        }
    }
    camera_ready
}

fn voxel_clear_color(frame: &VisualWorldFrame) -> Color {
    let tileset = frame
        .tiles
        .first()
        .map(|tile| tile.source.tileset_id.as_ref());
    if tileset == Some("game_corner") {
        // The casino wall backs onto unlit interior void, not an outdoor
        // horizon. This also matches the black source course above the wall.
        Color::srgb(0.10, 0.09, 0.08)
    } else if matches!(tileset, Some("cave" | "dark_cave")) {
        // A cave is an enclosed room. Exposed space beyond its authored rock
        // boundary is unlit void, never the outdoor horizon color.
        Color::srgb(0.035, 0.025, 0.065)
    } else if tileset.is_some_and(crate::interior::has_back_wall) {
        // Authored wall courses define the room. Anything beyond their finite
        // edges is neutral void; never synthesize a full-width gray backdrop.
        Color::srgb(0.055, 0.050, 0.045)
    } else {
        Color::srgb(0.72, 0.83, 0.78)
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_terrain(
    frame: &VisualWorldFrame,
    commands: &mut Commands,
    cache: &mut TerrainRevisionCache,
    builds: &mut TerrainBuildQueue,
    images: &Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_entities: &mut Query<(&mut Visibility, &mut Transform), VoxelTerrainFilter>,
) -> Result<TerrainSyncState, TerrainSyncError> {
    let next_key = TerrainCacheKey::from_frame(frame);
    if should_start_terrain_build(cache.key.as_ref(), builds.key.as_ref(), &next_key) {
        let build_frame = frame.clone();
        let samples = TerrainImageSamples::capture(frame, images);
        let build_key = next_key.clone();
        builds.key = Some(next_key.clone());
        let build = async move {
            // SurfaceMeshData -> Bevy Mesh conversion walks and moves every
            // vertex/index buffer. Keep that work on the compute task too;
            // doing it when polling the completed build caused a deterministic
            // 30-45 ms main-thread hitch several seconds into 2.5D movement.
            let terrain = build_terrain_mesh_with_samples(&build_frame, &samples).map(|terrain| {
                let footing_heights = terrain.footing_heights.clone();
                let (textured_mesh, solid_mesh) = terrain.into_meshes();
                BuiltTerrain {
                    footing_heights,
                    textured_mesh,
                    solid_mesh,
                }
            });
            TerrainBuildResult {
                key: build_key,
                frame: build_frame,
                terrain,
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            builds.task = Some(AsyncComputeTaskPool::get().spawn(build));
        }
        #[cfg(target_arch = "wasm32")]
        {
            builds.completed = Some(future::block_on(build));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let completed = builds
        .task
        .as_mut()
        .and_then(|task| future::block_on(future::poll_once(task)));
    #[cfg(target_arch = "wasm32")]
    let completed = builds.completed.take();
    if let Some(completed) = completed {
        #[cfg(not(target_arch = "wasm32"))]
        {
            builds.task = None;
        }
        builds.key = None;
        if completed.key == next_key {
            let terrain = completed.terrain.map_err(TerrainSyncError::Mesh)?;
            apply_built_terrain(
                &completed.frame,
                completed.key,
                terrain,
                commands,
                cache,
                meshes,
                materials,
            )?;
        }
    }

    let ready = cache.key.as_ref() == Some(&next_key);
    if ready {
        if let Some(handle) = cache.textured_material.as_ref() {
            let Some(material) = materials.get_mut(handle) else {
                return Err(TerrainSyncError::CachedTexturedMaterialUnavailable);
            };
            if material.base_color_texture.as_ref() != Some(&frame.map_texture) {
                material.base_color_texture = Some(frame.map_texture.clone());
            }
        }
    }
    // A viewport-origin change starts an asynchronous replacement build. The
    // last complete mesh remains the terrain shown during that build, so it
    // must consume the same live camera-scroll transform as actors. Leaving
    // it at the preceding frame center makes every actor slide or float over
    // stationary geometry until the replacement happens to finish.
    if let Some(live_transform) = retained_terrain_transform(frame, cache) {
        for entity in [cache.textured_entity, cache.solid_entity]
            .into_iter()
            .flatten()
        {
            if let Ok((mut visibility, mut transform)) = terrain_entities.get_mut(entity) {
                *visibility = Visibility::Visible;
                *transform = live_transform;
            }
        }
    }
    Ok(if ready {
        TerrainSyncState::Ready
    } else {
        TerrainSyncState::Pending
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_built_terrain(
    frame: &VisualWorldFrame,
    key: TerrainCacheKey,
    terrain: BuiltTerrain,
    commands: &mut Commands,
    cache: &mut TerrainRevisionCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Result<(), TerrainSyncError> {
    cache.footing_heights = terrain.footing_heights;

    let textured_mesh_handle = update_mesh_asset(
        meshes,
        &mut cache.textured_mesh,
        terrain.textured_mesh,
        TerrainSyncError::CachedTexturedMeshUnavailable,
    )?;
    let solid_mesh_handle = update_mesh_asset(
        meshes,
        &mut cache.solid_mesh,
        terrain.solid_mesh,
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

    cache.key = Some(key);
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
        unlit: voxel_material_unlit(),
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
        unlit: voxel_material_unlit(),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
        ..default()
    }
}

/// WebGL's reduced PBR path visibly washes the four-color Game Boy palettes
/// toward the scene light colors. The browser renderer should preserve the
/// already-resolved palette texels; native backends retain dimensional PBR
/// lighting.
const fn voxel_material_unlit() -> bool {
    cfg!(target_arch = "wasm32")
}

#[allow(clippy::too_many_arguments)]
fn sync_actor_cards(
    frame: &VisualWorldFrame,
    footing_heights: &[f32],
    actor_quad: &Handle<Mesh>,
    commands: &mut Commands,
    cache: &mut ActorIdCache,
    images: &Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    actor_entities: &mut Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Handle<Mesh>,
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
    // The player is mandatory. An NPC whose foot lies outside the published
    // terrain halo is simply clipped, exactly like the classic sprite pass;
    // retiring the whole optional world here caused a 2D flash while walking.
    let mut prepared = Vec::with_capacity(frame.actors.len());
    for actor in &frame.actors {
        if !images.contains(&actor.texture) {
            return Err(ActorSyncError::TextureUnavailable(actor.id));
        }
        let Some(transform) = actor_transform(frame, actor, footing_heights) else {
            if actor.id == VisualActorId::Player {
                return Err(ActorSyncError::FootingUnavailable(actor.id));
            }
            continue;
        };
        let mesh = actor_quad.clone();
        prepared.push((actor, transform, mesh));
    }

    let visible_ids: HashSet<_> = prepared.iter().map(|(actor, _, _)| actor.id).collect();
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
    for (actor, transform, mesh) in prepared {
        let texture_id = actor.texture.id();
        used_textures.insert(texture_id);
        let material = actor_material(actor, cache, materials);

        let existing = cache.entities.get(&actor.id).copied();
        if let Some(entity) = existing
            && let Ok((
                mut current_transform,
                mut visibility,
                mut current_mesh,
                mut current_material,
            )) = actor_entities.get_mut(entity)
        {
            *current_transform = transform;
            *visibility = Visibility::Visible;
            *current_mesh = mesh;
            *current_material = material;
            continue;
        }

        let entity = commands
            .spawn((
                PbrBundle {
                    mesh,
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
    terrain_cache: Res<TerrainRevisionCache>,
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
        &terrain_cache.footing_heights,
        actor_quad,
        &mut commands,
        &mut cache,
        &mut materials,
        &mut entities,
    );
}

fn sync_player_silhouette(
    frame: &VisualWorldFrame,
    footing_heights: &[f32],
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
    let Some(transform) = actor_transform(frame, player, footing_heights) else {
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
            color: LinearRgba::new(1.0, 1.0, 1.0, 0.82),
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
        unlit: voxel_material_unlit(),
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

fn actor_transform(
    frame: &VisualWorldFrame,
    actor: &VisualActor,
    footing_heights: &[f32],
) -> Option<Transform> {
    let foot = actor_foot(actor);
    let height = resolved_footing_height(frame, foot, footing_heights)?;
    let mut position = visual_point_to_voxel(foot, height + 0.05);
    let profile_scale = frame.tile_size.y / SOURCE_TILE_HEIGHT;
    let pose = camera_pose(frame.viewport_size);
    let camera_pull = actor_camera_pull(actor, CAMERA_PITCH_DEGREES.to_radians()) * profile_scale;
    // A camera-facing card leans across the ground in screen space. Pull it
    // forward by exactly the depth needed for its upper half to clear terrain
    // at the actor's own footing height. Real trees and buildings remain much
    // farther apart in depth and still occlude the actor normally.
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

fn actor_camera_pull(actor: &VisualActor, pitch_radians: f32) -> f32 {
    let sine = pitch_radians.sin().max(MIN_PULL_SINE);
    let lean_overlap = (ACTOR_CARD_HEIGHT * pitch_radians.cos() - ACTOR_FOOT_ANCHOR).max(0.0);
    let normal_pull = ACTOR_BASE_CAMERA_PULL + lean_overlap / sine;
    normal_pull
        + if actor.above_priority {
            ABOVE_PRIORITY_EXTRA_PULL
        } else {
            0.0
        }
}

fn terrain_transform(frame: &VisualWorldFrame) -> Transform {
    Transform::from_xyz(frame.center.x, 0.0, -frame.center.y)
}

fn retained_terrain_transform(
    frame: &VisualWorldFrame,
    cache: &TerrainRevisionCache,
) -> Option<Transform> {
    cache.key.as_ref()?;
    Some(terrain_transform(frame))
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

    #[test]
    fn cave_clear_color_is_enclosed_void_not_outdoor_horizon() {
        let mut frame = VisualWorldFrame::default();
        frame.tiles.push(crystal_render_api::VisualTile {
            column: 0,
            row: 0,
            source: crystal_render_api::VisualTileSource {
                tileset_id: "cave".into(),
                metatile_id: 0,
                subtile_column: 0,
                subtile_row: 0,
                tile_index: 0,
            },
            texture: Handle::default(),
            priority: false,
        });

        assert_eq!(voxel_clear_color(&frame), Color::srgb(0.035, 0.025, 0.065));

        frame.tiles[0].source.tileset_id = "house".into();
        assert_eq!(
            voxel_clear_color(&frame),
            Color::srgb(0.055, 0.050, 0.045),
            "enclosed ordinary interiors must not inherit the outdoor horizon"
        );
    }

    #[test]
    fn terrain_rebuild_keeps_the_last_complete_voxel_frame_visible() {
        let mut cache = TerrainRevisionCache::default();
        assert!(must_wait_for_initial_terrain(
            TerrainSyncState::Pending,
            &cache
        ));
        cache.key = Some(TerrainCacheKey {
            revision: 1,
            viewport_bits: [160.0_f32.to_bits(), 144.0_f32.to_bits()],
            tile_bits: [8.0_f32.to_bits(), 8.0_f32.to_bits()],
            grid_size: UVec2::new(20, 18),
        });
        assert!(!must_wait_for_initial_terrain(
            TerrainSyncState::Pending,
            &cache
        ));
        assert!(!must_wait_for_initial_terrain(
            TerrainSyncState::Ready,
            &cache
        ));

        let frame = VisualWorldFrame {
            center: Vec2::new(24.0, -12.0),
            ..default()
        };
        let retained = retained_terrain_transform(&frame, &cache)
            .expect("a pending replacement keeps the completed terrain posed");
        assert_eq!(retained.translation, Vec3::new(24.0, 0.0, 12.0));
    }

    #[test]
    fn movement_coalesces_terrain_rebuilds_while_one_is_in_flight() {
        let key = |revision| TerrainCacheKey {
            revision,
            viewport_bits: [160.0_f32.to_bits(), 144.0_f32.to_bits()],
            tile_bits: [8.0_f32.to_bits(), 8.0_f32.to_bits()],
            grid_size: UVec2::new(84, 82),
        };
        let cached = key(1);
        let queued = key(2);
        let latest_movement = key(3);

        assert!(should_start_terrain_build(Some(&cached), None, &queued));
        assert!(
            !should_start_terrain_build(Some(&cached), Some(&queued), &latest_movement),
            "a newer walking viewport must not replace work already running on the compute pool"
        );
    }

    #[test]
    fn player_depth_pull_clears_same_level_terrain_at_forty_five_degrees() {
        let player = VisualActor {
            id: VisualActorId::Player,
            source_id: "player".into(),
            texture: Handle::weak_from_u128(1),
            center: Vec2::ZERO,
            size: Vec2::splat(16.0),
            flip_x: false,
            above_priority: false,
        };
        let pull = actor_camera_pull(&player, 45.0_f32.to_radians());
        let expected = ACTOR_BASE_CAMERA_PULL
            + (ACTOR_CARD_HEIGHT * 45.0_f32.to_radians().cos() - ACTOR_FOOT_ANCHOR)
                / 45.0_f32.to_radians().sin();
        assert!((pull - expected).abs() < 0.001);
        assert!(pull > 10.0 && pull < 11.0);
    }

    #[test]
    fn scrolling_frame_keeps_actor_locked_to_retained_terrain() {
        let mut frame = VisualWorldFrame {
            center: Vec2::ZERO,
            viewport_size: Vec2::splat(16.0),
            tile_size: Vec2::splat(8.0),
            grid_size: UVec2::new(2, 2),
            ..default()
        };
        let mut player = VisualActor {
            id: VisualActorId::Player,
            source_id: "player".into(),
            texture: Handle::weak_from_u128(1),
            center: Vec2::ZERO,
            size: Vec2::splat(16.0),
            flip_x: false,
            above_priority: false,
        };
        let heights = vec![0.0; 4];
        let actor_before =
            actor_transform(&frame, &player, &heights).expect("player has footing before scroll");
        let terrain_before = terrain_transform(&frame);

        let scroll = Vec2::new(6.0, -3.0);
        frame.center += scroll;
        player.center += scroll;
        let actor_after =
            actor_transform(&frame, &player, &heights).expect("player has footing during scroll");
        let terrain_after = terrain_transform(&frame);

        assert_eq!(
            actor_after.translation - terrain_after.translation,
            actor_before.translation - terrain_before.translation,
            "actor and retained terrain must consume the identical live scroll"
        );
    }

    #[test]
    fn live_animation_texture_does_not_invalidate_terrain_geometry() {
        let mut first = VisualWorldFrame {
            terrain_revision: 42,
            map_texture: Handle::weak_from_u128(1),
            viewport_size: Vec2::new(160.0, 144.0),
            tile_size: Vec2::splat(8.0),
            grid_size: UVec2::new(20, 18),
            ..default()
        };
        let first_key = TerrainCacheKey::from_frame(&first);
        first.map_texture = Handle::weak_from_u128(2);
        assert_eq!(first_key, TerrainCacheKey::from_frame(&first));
    }
}
