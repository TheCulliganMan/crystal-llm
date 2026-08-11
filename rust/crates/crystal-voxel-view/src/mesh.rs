//! Pure authored-profile mesh construction for the optional voxel renderer.

use std::collections::{HashMap, VecDeque};

use bevy::{
    asset::AssetId,
    prelude::{Assets, Image, Mesh, Vec3},
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use crystal_render_api::{VisualTile, VisualTileSource, VisualWorldFrame};

use crate::battle_tower::tree_group as battle_tower_tree_group;
use crate::building_catalog::BUILDING_TEMPLATES;
use crate::building_style::{
    HOUSE_WALL_INSET_PIXELS, burned_tower_roof_style, uses_center_ridge_roof,
};
use crate::elevation::{resolve_authored_mountain_tiers, resolve_jump_ledge_ground};
use crate::park::{FountainPlacement, fountain_placements};
use crate::profile::{
    CellShape, KANTO_GROUND_TILE_INDEX, LedgeFace, SOURCE_TILE_HEIGHT, SolidKind, shape_for_source,
    shape_for_source_on_map, support_height,
};
use crate::rock_platform::resolve_rock_platform_tiers;
use crate::waterfall::{WaterfallPlacement, waterfall_placements};

const TEXTURED_SHADE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// How one source cell is consumed by the optional renderer. This is exposed
/// for the full-game coverage auditor; gameplay never reads it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CellCoverageKind {
    Flat,
    Building,
    Tree,
    Water,
    Plane,
    Waterfall,
    Cutout,
    Relief,
    Shore,
    Raised,
    Ramp,
    Facade,
    Ledge,
}

impl CellCoverageKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Building => "building",
            Self::Tree => "tree",
            Self::Water => "water",
            Self::Plane => "plane",
            Self::Waterfall => "waterfall",
            Self::Cutout => "cutout",
            Self::Relief => "relief",
            Self::Shore => "shore",
            Self::Raised => "raised",
            Self::Ramp => "ramp",
            Self::Facade => "facade",
            Self::Ledge => "ledge",
        }
    }
}
// The reference volume renderer slightly darkens flat structure tops so the
// folded native cliff courses read as vertical faces instead of one continuous
// sheet of equally lit pixels.
const BANK_TOP_SHADE: [f32; 4] = [0.85, 0.85, 0.85, 1.0];
const SOURCE_TILE_PIXELS: usize = SOURCE_TILE_HEIGHT as usize;
const TRADITIONAL_ROOF_FASCIA_PIXELS: usize = 2;

#[derive(Clone, Debug, Default)]
pub struct TerrainImageSamples {
    pixels: HashMap<AssetId<Image>, TileImageSample>,
}

#[derive(Clone, Debug)]
enum TileImageSample {
    Rgba(Vec<u8>),
    Invalid,
}

impl TerrainImageSamples {
    pub fn capture(frame: &VisualWorldFrame, images: &Assets<Image>) -> Self {
        let mut samples = Self::default();
        for tile in &frame.tiles {
            samples.pixels.entry(tile.texture.id()).or_insert_with(|| {
                let Some(image) = images.get(&tile.texture) else {
                    return TileImageSample::Invalid;
                };
                let size = image.texture_descriptor.size;
                let expected_len = SOURCE_TILE_PIXELS * SOURCE_TILE_PIXELS * 4;
                if size.width as usize != SOURCE_TILE_PIXELS
                    || size.height as usize != SOURCE_TILE_PIXELS
                    || size.depth_or_array_layers != 1
                    || image.data.len() != expected_len
                {
                    TileImageSample::Invalid
                } else {
                    TileImageSample::Rgba(image.data.clone())
                }
            });
        }
        samples
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl SurfaceMeshData {
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }

    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }
}

/// Two material domains are intentional. `textured` contains only native
/// top-facing cells and one-tile-high authored facade bands. `solid` contains
/// generated thickness, so source artwork can never be stretched down a wall.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerrainMeshData {
    pub textured: SurfaceMeshData,
    pub solid: SurfaceMeshData,
    pub footing_heights: Vec<f32>,
}

impl TerrainMeshData {
    pub fn into_meshes(self) -> (Mesh, Mesh) {
        (self.textured.into_mesh(), self.solid.into_mesh())
    }
}

/// Builds a combined textured surface mesh and a separate untextured solid
/// mesh from explicitly addressed cells and clean-room source profiles.
pub fn build_terrain_mesh(frame: &VisualWorldFrame) -> Result<TerrainMeshData, TerrainMeshError> {
    build_terrain_mesh_internal(frame, None)
}

/// Runtime variant that removes the authored ground pixels from upright tree
/// and prop bands. Buildings remain complete opaque source bands.
pub fn build_terrain_mesh_with_images(
    frame: &VisualWorldFrame,
    images: &Assets<Image>,
) -> Result<TerrainMeshData, TerrainMeshError> {
    let samples = TerrainImageSamples::capture(frame, images);
    build_terrain_mesh_with_samples(frame, &samples)
}

pub fn build_terrain_mesh_with_samples(
    frame: &VisualWorldFrame,
    samples: &TerrainImageSamples,
) -> Result<TerrainMeshData, TerrainMeshError> {
    build_terrain_mesh_internal(frame, Some(samples))
}

fn build_terrain_mesh_internal(
    frame: &VisualWorldFrame,
    images: Option<&TerrainImageSamples>,
) -> Result<TerrainMeshData, TerrainMeshError> {
    frame
        .validate()
        .map_err(TerrainMeshError::InvalidVisualFrame)?;
    if !frame.active {
        return Err(TerrainMeshError::InactiveFrame);
    }

    let width = usize::try_from(frame.grid_size.x).map_err(|_| TerrainMeshError::GridTooLarge)?;
    let height = usize::try_from(frame.grid_size.y).map_err(|_| TerrainMeshError::GridTooLarge)?;
    let cell_count = width
        .checked_mul(height)
        .ok_or(TerrainMeshError::GridTooLarge)?;
    let mut cells = vec![None; cell_count];
    for tile in &frame.tiles {
        let column = usize::try_from(tile.column).map_err(|_| TerrainMeshError::GridTooLarge)?;
        let row = usize::try_from(tile.row).map_err(|_| TerrainMeshError::GridTooLarge)?;
        let index = row
            .checked_mul(width)
            .and_then(|base| base.checked_add(column))
            .ok_or(TerrainMeshError::GridTooLarge)?;
        if cells[index].replace(tile).is_some() {
            return Err(TerrainMeshError::DuplicateTile {
                column: tile.column,
                row: tile.row,
            });
        }
    }
    if let Some(index) = cells.iter().position(Option::is_none) {
        return Err(TerrainMeshError::MissingTile {
            column: (index % width) as u32,
            row: (index / width) as u32,
        });
    }

    let cells: Vec<&VisualTile> = cells
        .into_iter()
        .map(|tile| tile.expect("complete tile grid was checked before meshing"))
        .collect();
    let mut shapes: Vec<_> = cells
        .iter()
        .map(|tile| shape_for_source_on_map(frame.map_id.as_ref(), &tile.source))
        .collect();
    resolve_rock_platform_tiers(&cells, &mut shapes, width);
    // Upright profiles require an exact live background cell for both the
    // vacated floor and pixel mask. A clipped viewport may not carry that
    // evidence; resolve only that shape back to the documented flat baseline
    // instead of failing the complete renderer or guessing from collision.
    let available_flat_tiles: std::collections::HashSet<_> = cells
        .iter()
        .zip(&shapes)
        .filter_map(|(tile, shape)| {
            matches!(shape, CellShape::Flat).then_some(tile.source.tile_index)
        })
        .collect();
    let available_relief_base_tiles: std::collections::HashSet<_> = cells
        .iter()
        .zip(&shapes)
        .filter_map(|(tile, shape)| {
            matches!(
                shape,
                CellShape::Flat
                    | CellShape::Water
                    | CellShape::RaisedTop {
                        solid: SolidKind::Bank,
                        ..
                    }
            )
            .then_some(tile.source.tile_index)
        })
        .collect();
    for shape in &mut shapes {
        if let CellShape::FacadeBand {
            ground_tile_index, ..
        } = *shape
            && !available_flat_tiles.contains(&ground_tile_index)
        {
            *shape = CellShape::Flat;
        }
        if let CellShape::Cutout {
            ground_tile_index, ..
        } = *shape
            && !available_flat_tiles.contains(&ground_tile_index)
        {
            *shape = CellShape::Flat;
        }
        if let CellShape::Relief {
            ground_tile_index, ..
        } = *shape
            && !available_relief_base_tiles.contains(&ground_tile_index)
        {
            *shape = CellShape::Flat;
        }
    }

    resolve_authored_mountain_tiers(&mut shapes, width, height);
    resolve_jump_ledge_ground(&mut shapes, width, height);

    let grid_width = frame.tile_size.x * frame.grid_size.x as f32;
    let grid_height = frame.tile_size.y * frame.grid_size.y as f32;
    let geometry = GridGeometry {
        width,
        height,
        tile_width: frame.tile_size.x,
        tile_height: frame.tile_size.y,
        origin_x: -grid_width * 0.5,
        origin_z: -grid_height * 0.5,
    };
    let mut mesh = TerrainMeshData::default();
    mesh.footing_heights = cells
        .iter()
        .zip(&shapes)
        .map(|(tile, shape)| match shape {
            CellShape::RaisedTop {
                solid: SolidKind::Bank,
                ..
            }
            | CellShape::LedgeBand { .. } => shape.surface_height(frame.tile_size.y),
            CellShape::RampNorth {
                north_height,
                south_height,
            } => (north_height + south_height) * 0.5 * frame.tile_size.y / SOURCE_TILE_HEIGHT,
            _ => support_height(&tile.source, frame.tile_size.y),
        })
        .collect();
    let mut claimed_by_building = vec![false; cell_count];
    let mut claimed_by_tree = vec![false; cell_count];
    let mut claimed_by_casino_stool = vec![false; cell_count];
    if let Some(images) = images {
        let placements = outdoor_building_placements(&cells, &geometry);
        for placement in &placements {
            let result = append_pixel_building(
                &mut mesh,
                images,
                &cells,
                &geometry,
                *placement,
                &mut claimed_by_building,
            );
            if let Err(error) = result {
                if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    // A clipped render halo may contain the complete drawing
                    // but not its explicitly profiled ground texel. Keep only
                    // that object as faithful flat art; one incomplete object
                    // must not retire the entire optional renderer.
                    for row in placement.row..placement.row + placement.height {
                        for column in placement.column..placement.column + placement.width {
                            claimed_by_building[row * geometry.width + column] = false;
                        }
                    }
                    continue;
                }
                return Err(error);
            }
        }
        // A partial template at the viewport edge is not enough evidence to
        // invent half a building. Preserve the faithful flat drawing until
        // the complete authored placement is available.
        for (index, shape) in shapes.iter_mut().enumerate() {
            if shape.solid_kind() == SolidKind::Building && !claimed_by_building[index] {
                *shape = CellShape::Flat;
            }
        }

        for placement in complete_tree_placements(&cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    // The expanded render halo can end through an otherwise
                    // complete tree drawing while its authored ground sample
                    // lies just beyond the published cells. Keep that object
                    // as faithful flat art instead of retiring the complete
                    // optional renderer.
                    for row in placement.row..placement.row + placement.height {
                        for column in placement.column..placement.column + placement.width {
                            claimed_by_tree[row * geometry.width + column] = false;
                        }
                    }
                    continue;
                }
                return Err(error);
            }
        }
        if frame.map_id.as_ref() == "CeladonGym" {
            for placement in celadon_hedge_placements(&cells, &geometry) {
                append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                )?;
            }
        }
        if crate::azalea_gym::supports_display_map(frame.map_id.as_ref()) {
            for placement in
                elite_four_gym_card_placements(frame.map_id.as_ref(), &cells, &geometry)
            {
                if let Err(error) = append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                ) {
                    if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                        for row in placement.row..placement.row + placement.height {
                            for column in placement.column..placement.column + placement.width {
                                claimed_by_tree[row * geometry.width + column] = false;
                            }
                        }
                        continue;
                    }
                    return Err(error);
                }
            }
        }
        if frame.map_id.as_ref() == "HallOfFame" {
            for placement in hall_of_fame_console_placements(&cells, &geometry) {
                append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                )?;
            }
        }
        if matches!(frame.map_id.as_ref(), "VioletGym" | "MahoganyGym") {
            for placement in violet_gym_card_placements(frame.map_id.as_ref(), &cells, &geometry) {
                if let Err(error) = append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                ) {
                    if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                        for row in placement.row..placement.row + placement.height {
                            for column in placement.column..placement.column + placement.width {
                                claimed_by_tree[row * geometry.width + column] = false;
                            }
                        }
                        continue;
                    }
                    return Err(error);
                }
            }
        }
        if frame.map_id.as_ref() == "CeruleanGym" {
            for placement in cerulean_statue_placements(&cells, &geometry) {
                append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                )?;
            }
        }
        if frame.map_id.as_ref() == "OlivineGym" {
            for placement in olivine_gym_boulder_placements(&cells, &geometry) {
                if let Err(error) = append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                ) {
                    if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                        for row in placement.row..placement.row + placement.height {
                            for column in placement.column..placement.column + placement.width {
                                claimed_by_tree[row * geometry.width + column] = false;
                            }
                        }
                        continue;
                    }
                    return Err(error);
                }
            }
        }
        if frame.map_id.as_ref() == "SaffronGym" {
            for placement in saffron_gym_planter_placements(&cells, &geometry) {
                if let Err(error) = append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                ) {
                    if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                        for row in placement.row..placement.row + placement.height {
                            for column in placement.column..placement.column + placement.width {
                                claimed_by_tree[row * geometry.width + column] = false;
                            }
                        }
                        continue;
                    }
                    return Err(error);
                }
            }
        }
        if crate::elite_four_room::supports_boulder_map(frame.map_id.as_ref()) {
            for placement in elite_four_room_boulder_placements(&cells, &geometry) {
                if let Err(error) = append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                ) {
                    if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                        for row in placement.row..placement.row + placement.height {
                            for column in placement.column..placement.column + placement.width {
                                claimed_by_tree[row * geometry.width + column] = false;
                            }
                        }
                        continue;
                    }
                    return Err(error);
                }
            }
        }
        for placement in ice_path_boulder_placements(&cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    for row in placement.row..placement.row + placement.height {
                        for column in placement.column..placement.column + placement.width {
                            claimed_by_tree[row * geometry.width + column] = false;
                        }
                    }
                    continue;
                }
                return Err(error);
            }
        }
        if crate::casino::is_game_corner_map(frame.map_id.as_ref()) {
            for placement in casino_stool_placements(&cells, &geometry) {
                append_casino_stool(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_casino_stool,
                )?;
            }
        }
    }
    let bank_runs = bank_column_runs(&shapes, &geometry);

    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
            if claimed_by_building[index]
                || claimed_by_tree[index]
                || claimed_by_casino_stool[index]
            {
                continue;
            }
            append_textured_cell(
                &mut mesh, &geometry, &cells, &shapes, &bank_runs, column, row, index, images,
            )?;
        }
    }

    for placement in waterfall_placements(&cells, width, height) {
        append_waterfall(&mut mesh.textured, &geometry, &cells, placement);
    }

    for placement in fountain_placements(&cells, width, height) {
        append_park_fountain(&mut mesh, &geometry, placement);
    }

    for row in 0..height {
        for column in 0..width {
            if claimed_by_building[row * width + column]
                || claimed_by_tree[row * width + column]
                || claimed_by_casino_stool[row * width + column]
            {
                continue;
            }
            append_exposed_sides(
                &mut mesh, &geometry, &cells, &shapes, &bank_runs, column, row,
            );
        }
    }

    Ok(mesh)
}

fn celadon_hedge_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::celadon_gym::GROUND_TILE,
        false,
        |source| {
            crate::celadon_gym::hedge_group(source).map(|group| {
                (
                    group.local_column,
                    group.local_row,
                    group.width,
                    group.height,
                )
            })
        },
    )
}

fn elite_four_gym_card_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let ground_tile_index = crate::azalea_gym::ground_tile(map_id)
        .expect("caller must restrict Elite Four Gym card maps");
    let mut placements = Vec::new();
    if map_id == "AzaleaGym" {
        placements.extend(grouped_flat_card_placements(
            cells,
            geometry,
            crate::azalea_gym::GROUND_TILE,
            true,
            |source| {
                crate::azalea_gym::central_tree(source).map(|group| {
                    (
                        group.local_column,
                        group.local_row,
                        group.width,
                        group.height,
                    )
                })
            },
        ));
    }
    placements.extend(grouped_flat_card_placements(
        cells,
        geometry,
        ground_tile_index,
        false,
        |source| {
            crate::azalea_gym::display_box(source).map(|group| {
                (
                    group.local_column,
                    group.local_row,
                    group.width,
                    group.height,
                )
            })
        },
    ));
    placements
}

fn hall_of_fame_console_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::hall_of_fame::FLOOR_TILE,
        false,
        |source| {
            crate::hall_of_fame::console_local(source).map(|(column, row)| (column, row, 2, 3))
        },
    )
}

fn violet_gym_card_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::violet_gym::GROUND_TILE,
        true,
        |source| {
            crate::violet_gym::card_group(map_id, source).map(|group| {
                (
                    group.local_column,
                    group.local_row,
                    group.width,
                    group.height,
                )
            })
        },
    )
}

fn cerulean_statue_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::cerulean_gym::DECK_TILE,
        true,
        |source| crate::cerulean_gym::statue_local(source).map(|(column, row)| (column, row, 2, 4)),
    )
}

fn olivine_gym_boulder_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::olivine_gym::GROUND_TILE,
        true,
        |source| crate::olivine_gym::boulder_local(source).map(|(column, row)| (column, row, 2, 2)),
    )
}

fn saffron_gym_planter_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::saffron_gym::FLOOR_TILE,
        true,
        |source| crate::saffron_gym::planter_local(source).map(|(column, row)| (column, row, 2, 4)),
    )
}

fn elite_four_room_boulder_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::elite_four_room::FLOOR_TILE,
        true,
        |source| {
            crate::elite_four_room::boulder_local(source).map(|(column, row)| (column, row, 2, 2))
        },
    )
}

fn ice_path_boulder_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = grouped_flat_card_placements(
        cells,
        geometry,
        crate::ice_path::CAVE_GROUND_TILE,
        true,
        |source| {
            crate::ice_path::boulder_local(source, crate::ice_path::BoulderBase::CaveGround)
                .map(|(column, row)| (column, row, 2, 2))
        },
    );
    placements.extend(grouped_flat_card_placements(
        cells,
        geometry,
        crate::ice_path::SMOOTH_ICE_TILE,
        true,
        |source| {
            crate::ice_path::boulder_local(source, crate::ice_path::BoulderBase::SmoothIce)
                .map(|(column, row)| (column, row, 2, 2))
        },
    ));
    placements
}

fn grouped_flat_card_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    ground_tile_index: u16,
    outline_mask: bool,
    classify: impl Fn(&VisualTileSource) -> Option<(u8, u8, usize, usize)>,
) -> Vec<TreePlacement> {
    let mut placements = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tile in cells {
        let Some((local_column, local_row, width, height)) = classify(&tile.source) else {
            continue;
        };
        let origin_column = tile.column as isize - local_column as isize;
        let origin_row = tile.row as isize - local_row as isize;
        if origin_column < 0
            || origin_row < 0
            || origin_column as usize + width > geometry.width
            || origin_row as usize + height > geometry.height
            || !seen.insert((origin_column, origin_row, width, height))
        {
            continue;
        }
        let complete = (0..height).all(|row| {
            (0..width).all(|column| {
                let cell = cells[(origin_row as usize + row) * geometry.width
                    + origin_column as usize
                    + column];
                classify(&cell.source) == Some((column as u8, row as u8, width, height))
            })
        });
        if complete {
            placements.push(TreePlacement {
                column: origin_column as usize,
                row: origin_row as usize,
                width,
                height,
                ground_tile_index,
                base_height: 0.0,
                rounded: false,
                outline_mask,
            });
        }
    }
    placements.sort_by_key(|placement| (placement.row, placement.column));
    placements
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BuildingPlacement {
    column: usize,
    row: usize,
    width: usize,
    height: usize,
    roof_rows: usize,
    ground_tile_index: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TreePlacement {
    column: usize,
    row: usize,
    width: usize,
    height: usize,
    ground_tile_index: u16,
    base_height: f32,
    rounded: bool,
    outline_mask: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CasinoStoolPlacement {
    column: usize,
    row: usize,
}

/// Classify a complete row-major source grid with the same group ownership
/// rules used by terrain meshing. This lets tooling find genuine flat
/// fallthroughs without misreporting cells consumed by buildings or trees.
pub fn audit_cell_coverage(
    tiles: &[VisualTile],
    width: usize,
    height: usize,
) -> Result<Vec<CellCoverageKind>, TerrainMeshError> {
    audit_cell_coverage_on_map("", tiles, width, height)
}

pub fn audit_cell_coverage_on_map(
    map_id: &str,
    tiles: &[VisualTile],
    width: usize,
    height: usize,
) -> Result<Vec<CellCoverageKind>, TerrainMeshError> {
    let cell_count = width
        .checked_mul(height)
        .ok_or(TerrainMeshError::GridTooLarge)?;
    if tiles.len() != cell_count {
        let index = tiles.len().min(cell_count.saturating_sub(1));
        return Err(TerrainMeshError::MissingTile {
            column: (index % width.max(1)) as u32,
            row: (index / width.max(1)) as u32,
        });
    }
    let mut ordered = vec![None; cell_count];
    for tile in tiles {
        let column = usize::try_from(tile.column).map_err(|_| TerrainMeshError::GridTooLarge)?;
        let row = usize::try_from(tile.row).map_err(|_| TerrainMeshError::GridTooLarge)?;
        if column >= width || row >= height {
            return Err(TerrainMeshError::MissingTile {
                column: tile.column,
                row: tile.row,
            });
        }
        let index = row * width + column;
        if ordered[index].replace(tile).is_some() {
            return Err(TerrainMeshError::DuplicateTile {
                column: tile.column,
                row: tile.row,
            });
        }
    }
    let cells: Vec<_> = ordered
        .into_iter()
        .enumerate()
        .map(|(index, tile)| {
            tile.ok_or(TerrainMeshError::MissingTile {
                column: (index % width.max(1)) as u32,
                row: (index / width.max(1)) as u32,
            })
        })
        .collect::<Result<_, _>>()?;
    let geometry = GridGeometry {
        width,
        height,
        tile_width: SOURCE_TILE_HEIGHT,
        tile_height: SOURCE_TILE_HEIGHT,
        origin_x: 0.0,
        origin_z: 0.0,
    };
    let mut coverage: Vec<_> = cells
        .iter()
        .map(|tile| match shape_for_source_on_map(map_id, &tile.source) {
            CellShape::Flat => CellCoverageKind::Flat,
            CellShape::Water => CellCoverageKind::Water,
            CellShape::PlaneAt { .. } => CellCoverageKind::Plane,
            CellShape::Waterfall => CellCoverageKind::Waterfall,
            CellShape::Cutout { .. } => CellCoverageKind::Cutout,
            CellShape::Relief { .. } => CellCoverageKind::Relief,
            CellShape::ShoreBand => CellCoverageKind::Shore,
            CellShape::RaisedTop { .. } => CellCoverageKind::Raised,
            CellShape::RampNorth { .. } => CellCoverageKind::Ramp,
            CellShape::FacadeBand {
                solid: SolidKind::Tree,
                ..
            } => CellCoverageKind::Tree,
            CellShape::FacadeBand { .. } => CellCoverageKind::Facade,
            CellShape::LedgeBand { .. } => CellCoverageKind::Ledge,
        })
        .collect();
    for placement in outdoor_building_placements(&cells, &geometry) {
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                coverage[row * width + column] = CellCoverageKind::Building;
            }
        }
    }
    for placement in complete_tree_placements(&cells, &geometry) {
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                coverage[row * width + column] = CellCoverageKind::Tree;
            }
        }
    }
    let tree_cards = if map_id == "CeladonGym" {
        celadon_hedge_placements(&cells, &geometry)
    } else {
        Vec::new()
    };
    apply_grouped_card_coverage(&mut coverage, width, tree_cards, CellCoverageKind::Tree);

    let mut prop_cards = match map_id {
        "AzaleaGym" | "GoldenrodGym" => elite_four_gym_card_placements(map_id, &cells, &geometry),
        "HallOfFame" => hall_of_fame_console_placements(&cells, &geometry),
        "CeruleanGym" => cerulean_statue_placements(&cells, &geometry),
        "VioletGym" | "MahoganyGym" => violet_gym_card_placements(map_id, &cells, &geometry),
        "OlivineGym" => olivine_gym_boulder_placements(&cells, &geometry),
        "SaffronGym" => saffron_gym_planter_placements(&cells, &geometry),
        map_id if crate::elite_four_room::supports_boulder_map(map_id) => {
            elite_four_room_boulder_placements(&cells, &geometry)
        }
        _ => Vec::new(),
    };
    prop_cards.extend(ice_path_boulder_placements(&cells, &geometry));
    apply_grouped_card_coverage(&mut coverage, width, prop_cards, CellCoverageKind::Cutout);
    Ok(coverage)
}

fn apply_grouped_card_coverage(
    coverage: &mut [CellCoverageKind],
    width: usize,
    placements: impl IntoIterator<Item = TreePlacement>,
    kind: CellCoverageKind,
) {
    for placement in placements {
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                coverage[row * width + column] = kind;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BankColumnRun {
    north: usize,
    front: usize,
}

fn is_goldenrod_game_corner(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: BuildingPlacement,
) -> bool {
    placement.width == 12
        && placement.height == 8
        && placement.roof_rows == 4
        && cells[placement.row * geometry.width + placement.column]
            .source
            .tileset_id
            .as_ref()
            == "johto_modern"
        && cells[(placement.row + 4) * geometry.width + placement.column + 4]
            .source
            .metatile_id
            == 0x17
}

fn bank_column_runs(shapes: &[CellShape], geometry: &GridGeometry) -> Vec<Option<BankColumnRun>> {
    let mut runs = vec![None; shapes.len()];
    for column in 0..geometry.width {
        let mut row = 0;
        while row < geometry.height {
            let index = row * geometry.width + column;
            let is_bank = !matches!(shapes[index], CellShape::RampNorth { .. })
                && shapes[index].solid_kind() == SolidKind::Bank
                && shapes[index].surface_height(geometry.tile_height) > 0.0;
            if !is_bank {
                row += 1;
                continue;
            }
            let north = row;
            let run_height = shapes[index].surface_height(geometry.tile_height);
            while row + 1 < geometry.height {
                let next = (row + 1) * geometry.width + column;
                if shapes[next].solid_kind() != SolidKind::Bank
                    || shapes[next].surface_height(geometry.tile_height) <= 0.0
                    || (shapes[next].surface_height(geometry.tile_height) - run_height).abs()
                        > f32::EPSILON
                {
                    break;
                }
                row += 1;
            }
            let run = BankColumnRun { north, front: row };
            for run_row in north..=row {
                runs[run_row * geometry.width + column] = Some(run);
            }
            row += 1;
        }
    }
    runs
}

fn complete_tree_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    let mut placements = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tile in cells {
        let (local_column, local_row, width, height, ground_tile_index, base_height, rounded) =
            match (tile.source.tileset_id.as_ref(), tile.source.metatile_id) {
                ("johto" | "johto_modern", 0x05) => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row,
                    2,
                    4,
                    if tile.source.tileset_id.as_ref() == "johto_modern" {
                        0x06
                    } else {
                        0x05
                    },
                    0.0,
                    false,
                ),
                ("battle_tower_outside", _) => {
                    let Some(group) = battle_tower_tree_group(&tile.source) else {
                        continue;
                    };
                    (
                        group.local_column,
                        group.local_row,
                        group.width,
                        group.height,
                        group.ground_tile_index,
                        0.0,
                        false,
                    )
                }
                ("johto_modern", 0x3d) if tile.source.subtile_row < 2 => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row,
                    2,
                    2,
                    0x06,
                    0.0,
                    false,
                ),
                ("johto_modern", 0x2f) if tile.source.subtile_row < 2 => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row,
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("johto", 0x60) if tile.source.subtile_column < 2 => (
                    tile.source.subtile_column,
                    tile.source.subtile_row % 2,
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("johto", 0x2a | 0x2c | 0x2d) if tile.source.subtile_row < 2 => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row,
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("johto", 0x62) if tile.source.subtile_column >= 2 => (
                    tile.source.subtile_column - 2,
                    tile.source.subtile_row % 2,
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("johto", 0x65) if tile.source.subtile_row >= 2 => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row - 2,
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("forest", 0x05) => (
                    tile.source.subtile_column,
                    tile.source.subtile_row,
                    4,
                    4,
                    0x05,
                    0.0,
                    false,
                ),
                ("forest", _) if matches!(tile.source.tile_index, 0x26 | 0x27 | 0x36 | 0x37) => (
                    u8::from(matches!(tile.source.tile_index, 0x27 | 0x37)),
                    u8::from(matches!(tile.source.tile_index, 0x36 | 0x37)),
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("kanto", _) if shape_for_source(&tile.source).solid_kind() == SolidKind::Tree => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row % 2,
                    2,
                    2,
                    KANTO_GROUND_TILE_INDEX,
                    0.0,
                    false,
                ),
                ("tower", 0x32..=0x36)
                    if matches!(tile.source.tile_index, 0x04 | 0x05 | 0x14 | 0x15) =>
                {
                    (
                        u8::from(matches!(tile.source.tile_index, 0x05 | 0x15)),
                        u8::from(matches!(tile.source.tile_index, 0x14 | 0x15)),
                        2,
                        2,
                        crate::tower::TOWER_FLOOR_TILE,
                        0.0,
                        false,
                    )
                }
                ("tower", 0x26)
                    if tile.source.subtile_row >= 2
                        && matches!(tile.source.tile_index, 0x07..=0x09 | 0x17..=0x19) =>
                {
                    (
                        tile.source.subtile_column % 2,
                        tile.source.subtile_row - 2,
                        2,
                        2,
                        crate::tower::TOWER_FLOOR_TILE,
                        0.0,
                        false,
                    )
                }
                _ => continue,
            };
        let origin_column = tile.column as isize - local_column as isize;
        let origin_row = tile.row as isize - local_row as isize;
        if origin_column < 0
            || origin_row < 0
            || origin_column as usize + width > geometry.width
            || origin_row as usize + height > geometry.height
            || !seen.insert((origin_column, origin_row, width, height))
        {
            continue;
        }
        let complete = (0..height).all(|row| {
            (0..width).all(|column| {
                let cell = cells[(origin_row as usize + row) * geometry.width
                    + origin_column as usize
                    + column];
                cell.source.tileset_id == tile.source.tileset_id
                    && (shape_for_source(&cell.source).solid_kind() == SolidKind::Tree
                        || (cell.source.tileset_id.as_ref() == "johto"
                            && cell.source.metatile_id == 0x6a
                            && cell.source.subtile_column < 2)
                        || (cell.source.tileset_id.as_ref() == "johto"
                            && cell.source.metatile_id == 0x6c
                            && cell.source.subtile_column < 2
                            && cell.source.subtile_row < 2))
            })
        });
        if complete {
            placements.push(TreePlacement {
                column: origin_column as usize,
                row: origin_row as usize,
                width,
                height,
                ground_tile_index,
                base_height,
                rounded,
                outline_mask: false,
            });
        }
    }
    placements.sort_by_key(|placement| (placement.row, placement.column));
    placements
}

fn outdoor_building_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<BuildingPlacement> {
    let mut metatiles = HashMap::new();
    for tile in cells {
        if !matches!(
            tile.source.tileset_id.as_ref(),
            "johto" | "johto_modern" | "kanto" | "forest" | "battle_tower_outside"
        ) {
            continue;
        }
        let origin = (
            tile.column as isize - tile.source.subtile_column as isize,
            tile.row as isize - tile.source.subtile_row as isize,
        );
        metatiles
            .entry(origin)
            .or_insert((tile.source.tileset_id.as_ref(), tile.source.metatile_id));
    }

    let mut placements = Vec::new();
    for (&(origin_x, origin_y), &(tileset_id, metatile_id)) in &metatiles {
        for template in BUILDING_TEMPLATES {
            if template.tileset != tileset_id || template.rows[0][0] != metatile_id {
                continue;
            }
            let matches = template.rows.iter().enumerate().all(|(template_y, row)| {
                row.iter().enumerate().all(|(template_x, expected)| {
                    metatiles
                        .get(&(
                            origin_x + (template_x * 4) as isize,
                            origin_y + (template_y * 4) as isize,
                        ))
                        .is_some_and(|(candidate_tileset, candidate)| {
                            *candidate_tileset == template.tileset && candidate == expected
                        })
                })
            });
            let width = template.rows[0].len() * 4 - template.skip_left_source_columns;
            let height = template.rows.len() * 4 - template.skip_top_source_rows;
            if matches
                && origin_x >= 0
                && origin_y >= 0
                && origin_x as usize + template.skip_left_source_columns + width <= geometry.width
                && origin_y as usize + template.skip_top_source_rows + height <= geometry.height
            {
                placements.push(BuildingPlacement {
                    column: origin_x as usize + template.skip_left_source_columns,
                    row: origin_y as usize + template.skip_top_source_rows,
                    width,
                    height,
                    roof_rows: template.roof_rows,
                    ground_tile_index: template.ground_tile,
                });
                // The catalog is most-specific first. Once a complete drawing
                // matches, do not also stamp a shorter prefix as a second
                // building inside the same landmark.
                break;
            }
        }
    }

    append_kanto_building_placements(&metatiles, geometry, &mut placements);
    // A facade row can itself resemble a shorter catalog template. Game
    // Corner is the clearest case: its $10/$17/$11 lower row was stamped a
    // second time as a shallow building, producing an L-shaped roof across
    // the front. Give the largest complete drawing ownership first and reject
    // every later placement that overlaps one of its source cells.
    placements.sort_by_key(|placement| {
        (
            std::cmp::Reverse(placement.width * placement.height),
            placement.row,
            placement.column,
        )
    });
    let mut occupied = vec![false; geometry.width * geometry.height];
    placements.retain(|placement| {
        let overlaps = (placement.row..placement.row + placement.height).any(|row| {
            (placement.column..placement.column + placement.width)
                .any(|column| occupied[row * geometry.width + column])
        });
        if overlaps {
            return false;
        }
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                occupied[row * geometry.width + column] = true;
            }
        }
        true
    });
    placements.sort_by_key(|placement| (placement.row, placement.column));
    placements.dedup();
    placements
}

fn append_kanto_building_placements(
    metatiles: &HashMap<(isize, isize), (&str, u16)>,
    geometry: &GridGeometry,
    placements: &mut Vec<BuildingPlacement>,
) {
    let at = |x: isize, y: isize| {
        metatiles
            .get(&(x, y))
            .and_then(|(tileset, id)| (*tileset == "kanto").then_some(*id))
    };
    let fits = |x: isize, y: isize, width: usize, height: usize| {
        x >= 0
            && y >= 0
            && x as usize + width * 4 <= geometry.width
            && y as usize + height * 4 <= geometry.height
    };
    let mut add = |x: isize, y: isize, width_blocks: usize, height_blocks: usize, roof_rows| {
        if fits(x, y, width_blocks, height_blocks) {
            placements.push(BuildingPlacement {
                column: x as usize,
                row: y as usize,
                width: width_blocks * 4,
                height: height_blocks * 4,
                roof_rows,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
            });
        }
    };

    for (&(x, y), &(tileset, first)) in metatiles {
        if tileset != "kanto" {
            continue;
        }

        // Kanto's large buildings use a roof-cap grammar: $20 begins the
        // roof, zero or more $54 spans extend it, and $21 closes it. The
        // next metatile row is the matching facade. This is the same
        // connected-run idea used by the reference renderer, expressed in
        // Crystal's stable metatile identities rather than collision.
        if first == 0x20 {
            for width_blocks in 2..=5 {
                let last_x = x + ((width_blocks - 1) * 4) as isize;
                if at(last_x, y) != Some(0x21)
                    || (1..width_blocks - 1)
                        .any(|column| at(x + (column * 4) as isize, y) != Some(0x54))
                {
                    continue;
                }
                let facade_y = y + 4;
                let facade_start = at(x, facade_y);
                let facade_end = at(last_x, facade_y);
                if matches!(facade_start, Some(0x37 | 0x7c))
                    && matches!(facade_end, Some(0x7e | 0x72 | 0x73))
                    && (1..width_blocks - 1).all(|column| {
                        matches!(
                            at(x + (column * 4) as isize, facade_y),
                            Some(0x3a | 0x7d | 0x7f)
                        )
                    })
                {
                    add(x, y, width_blocks, 2, 4);
                    break;
                }
            }
        }

        // Rounded-cap buildings include both ordinary Centers and tall city
        // landmarks. $68/$7f*/$69 may repeat for several complete storeys;
        // consume the whole vertical run before folding the final facade.
        // This is the reference renderer's longest-template-first rule and
        // prevents Silph Co.'s upper windows from remaining painted flat.
        if first == 0x68 && at(x, y - 4) != Some(0x68) {
            for width_blocks in 2..=5 {
                let last_x = x + ((width_blocks - 1) * 4) as isize;
                if at(last_x, y) != Some(0x69)
                    || (1..width_blocks - 1)
                        .any(|column| at(x + (column * 4) as isize, y) != Some(0x7f))
                {
                    continue;
                }
                let cap_row_matches = |candidate_y: isize| {
                    at(x, candidate_y) == Some(0x68)
                        && at(last_x, candidate_y) == Some(0x69)
                        && (1..width_blocks - 1)
                            .all(|column| at(x + (column * 4) as isize, candidate_y) == Some(0x7f))
                };
                let mut cap_rows = 1;
                while cap_row_matches(y + (cap_rows * 4) as isize) {
                    cap_rows += 1;
                }
                let facade_y = y + (cap_rows * 4) as isize;
                if at(x, facade_y) == Some(0x37)
                    && matches!(at(last_x, facade_y), Some(0x73 | 0x7e))
                    && (1..width_blocks - 1).all(|column| {
                        matches!(at(x + (column * 4) as isize, facade_y), Some(0x3a | 0x7d))
                    })
                {
                    add(x, y, width_blocks, cap_rows + 1, 4);
                    break;
                }
            }
        }

        // Pewter Museum is another authored variable-width cap: its stepped
        // $75/$71*/$76 roof sits over the standard $37/$7d*/$7e facade.
        if first == 0x75 {
            for width_blocks in 2..=5 {
                let last_x = x + ((width_blocks - 1) * 4) as isize;
                if at(last_x, y) != Some(0x76)
                    || (1..width_blocks - 1)
                        .any(|column| at(x + (column * 4) as isize, y) != Some(0x71))
                {
                    continue;
                }
                let facade_y = y + 4;
                if at(x, facade_y) == Some(0x37)
                    && at(last_x, facade_y) == Some(0x7e)
                    && (1..width_blocks - 1)
                        .all(|column| at(x + (column * 4) as isize, facade_y) == Some(0x7d))
                {
                    add(x, y, width_blocks, 2, 4);
                    break;
                }
            }
        }

        // Gyms and civic buildings use explicit cap/middle/end courses.
        if first == 0x0c {
            for width_blocks in 2..=6 {
                let last_x = x + ((width_blocks - 1) * 4) as isize;
                if at(last_x, y) == Some(0x0e)
                    && (1..width_blocks - 1)
                        .all(|column| at(x + (column * 4) as isize, y) == Some(0x0d))
                    && at(x, y + 4) == Some(0x10)
                    && at(last_x, y + 4) == Some(0x12)
                    && (1..width_blocks - 1)
                        .all(|column| at(x + (column * 4) as isize, y + 4) == Some(0x11))
                {
                    add(x, y, width_blocks, 2, 4);
                    break;
                }
            }
        }

        // Route 2's Diglett's Cave mound is one 4x2-block drawing. Its first
        // six source-tile rows are the top-down plateau and its final two are
        // the cave mouth/front course. Claim it as one pixel structure so the
        // diagonal shoulder and inset notch come from the drawing's actual
        // silhouette instead of filling the complete metatile rectangle.
        if first == 0x3e
            && at(x + 4, y) == Some(0x3f)
            && at(x + 8, y) == Some(0x3f)
            && at(x + 12, y) == Some(0x3b)
            && at(x, y + 4) == Some(0x24)
            && at(x + 4, y + 4) == Some(0x06)
            && at(x + 8, y + 4) == Some(0x57)
            && at(x + 12, y + 4) == Some(0x25)
        {
            add(x, y, 4, 2, 6);
        }

        // Compact Kanto houses are complete one-metatile-high drawings,
        // but are authored as left/right blocks. Their upper two tile rows
        // are roof art and their lower two rows are the facade.
        if matches!(first, 0x02 | 0x30) && at(x + 4, y) == Some(0x03) {
            add(x, y, 2, 1, 2);
        }
        if first == 0x38
            && at(x + 4, y) == Some(0x39)
            && at(x, y + 4) == Some(0x3c)
            && at(x + 4, y + 4) == Some(0x3d)
        {
            add(x, y, 2, 2, 4);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_grouped_tree(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        placement.ground_tile_index,
        placement.base_height,
        geometry.tile_height,
    )
    .ok_or(TerrainMeshError::MissingGroundSample {
        column: placement.column as u32,
        row: placement.row as u32,
        tile_index: placement.ground_tile_index,
    })?;
    let pixel_width = placement.width * SOURCE_TILE_PIXELS;
    let pixel_height = placement.height * SOURCE_TILE_PIXELS;
    let ground = tile_rgba(images, cells[ground_index])?;
    let mut equals_ground = vec![false; pixel_width * pixel_height];
    let mut drawing = vec![[0_u8; 4]; pixel_width * pixel_height];

    // Segment the complete drawing once. Flooding each 8px source tile in
    // isolation cuts a connected canopy at every tile seam; the reference
    // object path instead treats the whole 16/32px drawing as one silhouette.
    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let index =
                (placement.row + local_row) * geometry.width + placement.column + local_column;
            let source = tile_rgba(images, cells[index])?;
            for pixel_y in 0..SOURCE_TILE_PIXELS {
                for pixel_x in 0..SOURCE_TILE_PIXELS {
                    let x = local_column * SOURCE_TILE_PIXELS + pixel_x;
                    let y = local_row * SOURCE_TILE_PIXELS + pixel_y;
                    let source_offset = (pixel_y * SOURCE_TILE_PIXELS + pixel_x) * 4;
                    drawing[y * pixel_width + x]
                        .copy_from_slice(&source[source_offset..source_offset + 4]);
                    equals_ground[y * pixel_width + x] =
                        pixels_equal(source, ground, pixel_x, pixel_y);
                }
            }
        }
    }
    // Palette attributes can make the background painted inside a mixed
    // metatile differ from the nearby plain-ground sample. The reference
    // object path floods the complete drawing's boundary background. Admit
    // the four corner colors as background candidates as well as the exact
    // authored ground, then flood only boundary-connected matches so enclosed
    // highlights and trunk holes remain part of the tree.
    let corner_colors = [
        drawing[0],
        drawing[pixel_width - 1],
        drawing[(pixel_height - 1) * pixel_width],
        drawing[pixel_height * pixel_width - 1],
    ];
    let background_candidates: Vec<bool> = if placement.outline_mask {
        // Black-outline figures can reuse intermediate palette colors in
        // both their body and their painted floor. Preserve the darker
        // connected outline and everything it encloses; flood only the
        // lighter pixels reachable from the complete drawing boundary.
        let rgba = drawing.iter().flat_map(|pixel| *pixel).collect::<Vec<_>>();
        darker_palette_mask(&rgba, 2)
            .into_iter()
            .map(|dark| !dark)
            .collect()
    } else {
        drawing
            .iter()
            .enumerate()
            .map(|(index, pixel)| equals_ground[index] || corner_colors.contains(pixel))
            .collect()
    };
    let removable_ground =
        boundary_connected_mask(pixel_width, pixel_height, &background_candidates);
    let solid_pixels: Vec<_> = removable_ground.into_iter().map(|ground| !ground).collect();

    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            let index = row * geometry.width + column;
            claimed[index] = true;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                placement.base_height,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
        }
    }

    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    let plane_z =
        geometry.origin_z + (placement.row + placement.height) as f32 * geometry.tile_height;
    let object_height = placement.height as f32 * geometry.tile_height;
    let crown_height = placement.base_height + object_height;
    if placement.rounded {
        append_rounded_tree_hull(
            mesh,
            geometry,
            placement,
            &solid_pixels,
            pixel_width,
            pixel_height,
            crown_height,
        );
        return Ok(());
    }
    // Trees deliberately remain exact 2D art in the 2.5D scene. The rounded
    // canopy volume invented depth the Game Boy drawing never supplied and
    // produced huge lumpy masses. Stand the complete, background-masked
    // drawing upright at its feet instead: one source pixel maps to one world
    // pixel and the terrain/depth buffer still occludes the card naturally.
    for pixel_y in 0..pixel_height {
        for pixel_x in 0..pixel_width {
            if !solid_pixels[pixel_y * pixel_width + pixel_x] {
                continue;
            }
            let world_x0 = x0 + pixel_x as f32 * (x1 - x0) / pixel_width as f32;
            let world_x1 = x0 + (pixel_x + 1) as f32 * (x1 - x0) / pixel_width as f32;
            // Face the flat drawing toward the 45-degree camera around its
            // authored foot line. Leaving it world-vertical foreshortens a
            // two-tile tree until it reads as a one-tile shrub. This preserves
            // one source pixel per displayed pixel without stretching it.
            let card_cos = std::f32::consts::FRAC_1_SQRT_2;
            let card_sin = std::f32::consts::FRAC_1_SQRT_2;
            let local_top = object_height - pixel_y as f32 * object_height / pixel_height as f32;
            let local_bottom =
                object_height - (pixel_y + 1) as f32 * object_height / pixel_height as f32;
            let world_y1 = placement.base_height + local_top * card_cos;
            let world_y0 = placement.base_height + local_bottom * card_cos;
            let world_z1 = plane_z - local_top * card_sin;
            let world_z0 = plane_z - local_bottom * card_sin;
            let cell_column = placement.column + pixel_x / SOURCE_TILE_PIXELS;
            let cell_row = placement.row + pixel_y / SOURCE_TILE_PIXELS;
            let (u0, u1, v0, v1) = geometry.uv(cell_column, cell_row);
            let local_x = pixel_x % SOURCE_TILE_PIXELS;
            let local_y = pixel_y % SOURCE_TILE_PIXELS;
            let pu0 = lerp_pixel(u0, u1, local_x);
            let pu1 = lerp_pixel(u0, u1, local_x + 1);
            let pv0 = lerp_pixel(v0, v1, local_y);
            let pv1 = lerp_pixel(v0, v1, local_y + 1);
            append_quad(
                &mut mesh.textured,
                [
                    [world_x1, world_y0, world_z0],
                    [world_x1, world_y1, world_z1],
                    [world_x0, world_y1, world_z1],
                    [world_x0, world_y0, world_z0],
                ],
                [0.0, card_sin, card_cos],
                [[pu1, pv1], [pu1, pv0], [pu0, pv0], [pu0, pv1]],
                TEXTURED_SHADE,
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_rounded_tree_hull(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    placement: TreePlacement,
    solid_pixels: &[bool],
    pixel_width: usize,
    pixel_height: usize,
    crown_height: f32,
) {
    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let pixel_x_size = placement.width as f32 * geometry.tile_width / pixel_width as f32;
    let pixel_y_size = placement.height as f32 * geometry.tile_height / pixel_height as f32;
    let center_z = geometry.origin_z
        + (placement.row as f32 + placement.height as f32 * 0.5) * geometry.tile_height;
    let pixel_z_size = pixel_x_size;

    let mut front_depth = vec![0.0_f32; pixel_width * pixel_height];
    for py in 0..pixel_height {
        let Some(left) = (0..pixel_width).find(|&px| solid_pixels[py * pixel_width + px]) else {
            continue;
        };
        let right = (0..pixel_width)
            .rfind(|&px| solid_pixels[py * pixel_width + px])
            .expect("a row with a left silhouette pixel has a right pixel");
        let center = (left + right + 1) as f32 * 0.5;
        let radius = ((right - left + 1) as f32 * 0.5).max(0.5);
        for px in left..=right {
            if !solid_pixels[py * pixel_width + px] {
                continue;
            }
            let dx = (px as f32 + 0.5 - center) / radius;
            front_depth[py * pixel_width + px] =
                (radius * (1.0 - dx * dx).max(0.0).sqrt()).max(0.5) * pixel_z_size;
        }
    }

    let on = |x: isize, y: isize| {
        x >= 0
            && y >= 0
            && x < pixel_width as isize
            && y < pixel_height as isize
            && solid_pixels[y as usize * pixel_width + x as usize]
    };
    for py in 0..pixel_height {
        for px in 0..pixel_width {
            let index = py * pixel_width + px;
            if !solid_pixels[index] {
                continue;
            }
            let wx0 = x0 + px as f32 * pixel_x_size;
            let wx1 = wx0 + pixel_x_size;
            let wy1 = crown_height - py as f32 * pixel_y_size;
            let wy0 = wy1 - pixel_y_size;
            let depth = front_depth[index];
            let front = center_z + depth;
            let back = center_z - depth;
            let cell_column = placement.column + px / SOURCE_TILE_PIXELS;
            let cell_row = placement.row + py / SOURCE_TILE_PIXELS;
            let (u0, u1, v0, v1) = geometry.uv(cell_column, cell_row);
            let local_x = px % SOURCE_TILE_PIXELS;
            let local_y = py % SOURCE_TILE_PIXELS;
            let pu0 = lerp_pixel(u0, u1, local_x);
            let pu1 = lerp_pixel(u0, u1, local_x + 1);
            let pv0 = lerp_pixel(v0, v1, local_y);
            let pv1 = lerp_pixel(v0, v1, local_y + 1);
            append_quad(
                &mut mesh.textured,
                [
                    [wx1, wy0, front],
                    [wx1, wy1, front],
                    [wx0, wy1, front],
                    [wx0, wy0, front],
                ],
                [0.0, 0.0, 1.0],
                [[pu1, pv1], [pu1, pv0], [pu0, pv0], [pu0, pv1]],
                TEXTURED_SHADE,
            );
            append_quad(
                &mut mesh.textured,
                [
                    [wx0, wy0, back],
                    [wx0, wy1, back],
                    [wx1, wy1, back],
                    [wx1, wy0, back],
                ],
                [0.0, 0.0, -1.0],
                [[pu0, pv1], [pu0, pv0], [pu1, pv0], [pu1, pv1]],
                [0.68, 0.68, 0.68, 1.0],
            );
            let shade = |direction| solid_color(SolidKind::Tree, direction);
            if !on(px as isize - 1, py as isize) {
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        [wx0, wy0, front],
                        [wx0, wy1, front],
                        [wx0, wy1, back],
                        [wx0, wy0, back],
                    ],
                    [-1.0, 0.0, 0.0],
                    shade(Direction::West),
                );
            }
            if !on(px as isize + 1, py as isize) {
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        [wx1, wy0, back],
                        [wx1, wy1, back],
                        [wx1, wy1, front],
                        [wx1, wy0, front],
                    ],
                    [1.0, 0.0, 0.0],
                    shade(Direction::East),
                );
            }
            if !on(px as isize, py as isize - 1) {
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        [wx0, wy1, back],
                        [wx1, wy1, back],
                        [wx1, wy1, front],
                        [wx0, wy1, front],
                    ],
                    [0.0, 1.0, 0.0],
                    shade(Direction::South),
                );
            }
            if !on(px as isize, py as isize + 1) {
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        [wx0, wy0, front],
                        [wx1, wy0, front],
                        [wx1, wy0, back],
                        [wx0, wy0, back],
                    ],
                    [0.0, -1.0, 0.0],
                    shade(Direction::North),
                );
            }
        }
    }
}

fn append_pixel_building(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: BuildingPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let source_shapes = cells
        .iter()
        .map(|tile| shape_for_source(&tile.source))
        .collect::<Vec<_>>();
    let ground_index = authored_ground_cell(cells, &source_shapes, placement.ground_tile_index)
        .or_else(|| common_flat_ground_outside(cells, &source_shapes, geometry, placement))
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: placement.ground_tile_index,
        })?;
    let pixel_width = placement.width * SOURCE_TILE_PIXELS;
    let pixel_height = placement.height * SOURCE_TILE_PIXELS;
    let roof_pixels = placement.roof_rows * SOURCE_TILE_PIXELS;
    let mut luminance = vec![0_u16; pixel_width * pixel_height];

    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let index =
                (placement.row + local_row) * geometry.width + placement.column + local_column;
            claimed[index] = true;
            let source = tile_rgba(images, cells[index])?;
            for pixel_y in 0..SOURCE_TILE_PIXELS {
                for pixel_x in 0..SOURCE_TILE_PIXELS {
                    let x = local_column * SOURCE_TILE_PIXELS + pixel_x;
                    let y = local_row * SOURCE_TILE_PIXELS + pixel_y;
                    let offset = (pixel_y * SOURCE_TILE_PIXELS + pixel_x) * 4;
                    luminance[y * pixel_width + x] = u16::from(source[offset]) * 3
                        + u16::from(source[offset + 1]) * 6
                        + u16::from(source[offset + 2]);
                }
            }
            let (x0, x1, z0, z1) =
                geometry.bounds(placement.column + local_column, placement.row + local_row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
        }
    }

    let mut shades = luminance.clone();
    shades.sort_unstable();
    shades.dedup();
    let light_threshold = shades[shades.len().saturating_sub(2)];
    let light_pixels: Vec<_> = luminance
        .iter()
        .copied()
        .map(|luminance| luminance >= light_threshold)
        .collect();
    let outside = boundary_connected_mask(pixel_width, pixel_height, &light_pixels);
    let inside: Vec<_> = outside.into_iter().map(|outside| !outside).collect();
    let pixel_x_size = geometry.tile_width / SOURCE_TILE_PIXELS as f32;
    let pixel_z_size = geometry.tile_height / SOURCE_TILE_PIXELS as f32;
    let mut wall_height = (pixel_height - roof_pixels) as f32 * pixel_z_size;
    let (building_x0, _, building_z0, _) = geometry.bounds(placement.column, placement.row);
    let tall_lighthouse = placement.width == 8
        && placement.height == 28
        && placement.roof_rows == 8
        && cells[placement.row * geometry.width + placement.column]
            .source
            .metatile_id
            == 0x08;
    let kanto_cliff_mound = placement.width == 16
        && placement.height == 8
        && placement.roof_rows == 6
        && cells[placement.row * geometry.width + placement.column]
            .source
            .tileset_id
            .as_ref()
            == "kanto"
        && cells[placement.row * geometry.width + placement.column]
            .source
            .metatile_id
            == 0x3e;
    let burned_tower_roof = burned_tower_roof_style(
        placement.width,
        placement.height,
        placement.roof_rows,
        &cells[placement.row * geometry.width + placement.column].source,
    );
    let first_building_source = &cells[placement.row * geometry.width + placement.column].source;
    let game_corner_box = is_goldenrod_game_corner(cells, geometry, placement);
    let kanto_plan_roof = first_building_source.tileset_id.as_ref() == "kanto"
        && matches!(first_building_source.metatile_id, 0x0c | 0x20);
    let battle_tower_landmark = first_building_source.tileset_id.as_ref() == "battle_tower_outside";
    // The repeated four-row city houses are compact boxes, not landmark
    // facade cards. Keep their individual facade pixels, inset walls and
    // textured side courses so they visibly occupy 3D space. Large modern
    // landmarks retain the simpler straight facade treatment.
    let compact_modern_box = first_building_source.tileset_id.as_ref() == "johto_modern"
        && placement.height == 4
        && placement.roof_rows == 2
        && placement.width == 4;
    let straight_modern_facade =
        first_building_source.tileset_id.as_ref() == "johto_modern" && !compact_modern_box;
    let traditional_gable =
        uses_center_ridge_roof(placement.height, placement.roof_rows, first_building_source);
    let gabled_roof = traditional_gable;
    if kanto_cliff_mound {
        wall_height += 8.0;
    }
    // The visible doorway must remain on the drawing's original south edge:
    // gameplay warps and actor feet are authored against that exact edge.
    // Folding at the roof/facade seam moved every door north by the facade
    // height, so a player standing on the entrance tile appeared several
    // cells away in 2.5D. Grow all building depth northward from the original
    // south edge, exactly as the reference building placement does.
    let facade_z = building_facade_plane_z(building_z0, pixel_height, pixel_z_size);
    // Keep the facade/door on its original source seam and grow the building
    // northward behind it. This adds honest footprint depth without moving
    // warps, actors, or the visible doorway south across the map.
    let roof_depth_pixels = if let Some(style) = burned_tower_roof {
        style.depth_pixels
    } else {
        // Keep the footprint compact enough for Crystal's dense town layout:
        // use the authored roof band as depth. Full drawing-height footprints
        // caused nearby buildings to dominate the camera and obscure streets.
        roof_pixels
    };
    let roof_back_z = facade_z - roof_depth_pixels as f32 * pixel_z_size;
    let facade_height_scale = if kanto_cliff_mound { 1.5 } else { 1.0 };

    // The drawing's outer silhouette is the roof shape. This is the key
    // distinction between a voxelized building and a rectangular box wearing
    // roof pixels: tapered ends become gables while the authored roof band
    // remains top-facing. A small horizontal median rejects isolated antenna,
    // highlight, and damaged-roof pixels without flattening the whole profile.
    let roof_top = if game_corner_box || kanto_plan_roof {
        // The original sprite bakes fake perspective into its outer columns.
        // Game Corner is intentionally the same clean box grammar as the gym:
        // one rectangular roof slab above one straight facade and closed sides.
        vec![0; pixel_width]
    } else {
        measured_roof_profile(&inside, pixel_width, roof_pixels)
    };
    let darkest = shades[0];
    let facade_columns: Vec<_> = (0..pixel_width)
        .filter(|&x| (roof_pixels..pixel_height).any(|y| inside[y * pixel_width + x]))
        .collect();
    let facade_left = facade_columns.first().copied().unwrap_or(0);
    let facade_right = facade_columns
        .last()
        .copied()
        .unwrap_or(pixel_width.saturating_sub(1));
    let inset_house_walls = !tall_lighthouse
        && !kanto_cliff_mound
        && burned_tower_roof.is_none()
        && !straight_modern_facade;
    let body_left = if inset_house_walls {
        facade_left
            .max(HOUSE_WALL_INSET_PIXELS)
            .min(pixel_width.saturating_sub(1))
    } else {
        facade_left
    };
    let body_right = if inset_house_walls {
        facade_right
            .min(pixel_width.saturating_sub(HOUSE_WALL_INSET_PIXELS + 1))
            .max(body_left)
    } else {
        facade_right
    };
    let building_x1 = building_x0 + pixel_width as f32 * pixel_x_size;
    let wall_x0 = if inset_house_walls {
        building_x0 + body_left as f32 * pixel_x_size
    } else {
        building_x0
    };
    let wall_x1 = if inset_house_walls {
        building_x0 + (body_right + 1) as f32 * pixel_x_size
    } else {
        building_x1
    };
    if straight_modern_facade {
        // The side shell shares the facade's front edge. Pull the single
        // planar card forward by a sub-pixel epsilon so depth testing cannot
        // alternate between side texels and the front along that seam.
        let front_z = facade_z + pixel_z_size * 0.05;
        let (mut u0, _, v0, _) = geometry.uv(placement.column, placement.row + placement.roof_rows);
        let (_, mut u1, _, v1) = geometry.uv(
            placement.column + placement.width - 1,
            placement.row + placement.height - 1,
        );
        if game_corner_box {
            // Discard the six source-pixel perspective gutters and expand the
            // real facade across the rectangular box. The roof and side shell
            // remain full width, matching the gym-style building grammar.
            let crop = (u1 - u0) * 6.0 / pixel_width as f32;
            u0 += crop;
            u1 -= crop;
        }
        append_quad(
            &mut mesh.textured,
            [
                [building_x1, 0.0, front_z],
                [building_x1, wall_height, front_z],
                [building_x0, wall_height, front_z],
                [building_x0, 0.0, front_z],
            ],
            [0.0, 0.0, 1.0],
            [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
            TEXTURED_SHADE,
        );
    } else {
        let recessed = facade_recess_mask(
            &inside,
            &luminance,
            pixel_width,
            pixel_height,
            roof_pixels,
            darkest,
        );
        let recess_depth = pixel_z_size;
        for y in roof_pixels..pixel_height {
            for x in 0..pixel_width {
                // Background flooding is valid for discovering a roof's outer
                // silhouette, but not for a facade: Crystal commonly paints pale
                // siding with the same palette entry as the surrounding ground.
                // Dropping those pixels hollowed the Game Corner and reduced the
                // repeated $12/$13 houses to roof-like fragments. A catalogued
                // building owns its complete facade rectangle; only explicitly
                // framed panes are recessed below that plane.
                if inset_house_walls && !(body_left..=body_right).contains(&x) {
                    continue;
                }
                let x0 = building_x0 + x as f32 * pixel_x_size;
                let x1 = x0 + pixel_x_size;
                let top = (pixel_height - y) as f32 * pixel_z_size * facade_height_scale;
                let bottom = top - pixel_z_size * facade_height_scale;
                let front_z = facade_z
                    - recessed[y * pixel_width + x]
                        .then_some(recess_depth)
                        .unwrap_or(0.0);
                append_quad(
                    &mut mesh.textured,
                    [
                        [x1, bottom, front_z],
                        [x1, top, front_z],
                        [x0, top, front_z],
                        [x0, bottom, front_z],
                    ],
                    [0.0, 0.0, 1.0],
                    source_pixel_uv(geometry, placement, x, y, true),
                    TEXTURED_SHADE,
                );
                if recessed[y * pixel_width + x] {
                    let open = |nx: isize, ny: isize| {
                        nx < 0
                            || ny < roof_pixels as isize
                            || nx >= pixel_width as isize
                            || ny >= pixel_height as isize
                            || !recessed[ny as usize * pixel_width + nx as usize]
                    };
                    if open(x as isize - 1, y as isize) {
                        append_solid_quad(
                            &mut mesh.solid,
                            [
                                [x0, bottom, facade_z],
                                [x0, top, facade_z],
                                [x0, top, front_z],
                                [x0, bottom, front_z],
                            ],
                            [1.0, 0.0, 0.0],
                            solid_color(SolidKind::Building, Direction::West),
                        );
                    }
                    if open(x as isize + 1, y as isize) {
                        append_solid_quad(
                            &mut mesh.solid,
                            [
                                [x1, bottom, front_z],
                                [x1, top, front_z],
                                [x1, top, facade_z],
                                [x1, bottom, facade_z],
                            ],
                            [-1.0, 0.0, 0.0],
                            solid_color(SolidKind::Building, Direction::East),
                        );
                    }
                    if open(x as isize, y as isize - 1) {
                        append_solid_quad(
                            &mut mesh.solid,
                            [
                                [x0, top, front_z],
                                [x1, top, front_z],
                                [x1, top, facade_z],
                                [x0, top, facade_z],
                            ],
                            [0.0, -1.0, 0.0],
                            solid_color(SolidKind::Building, Direction::North),
                        );
                    }
                    if open(x as isize, y as isize + 1) {
                        append_solid_quad(
                            &mut mesh.solid,
                            [
                                [x0, bottom, facade_z],
                                [x1, bottom, facade_z],
                                [x1, bottom, front_z],
                                [x0, bottom, front_z],
                            ],
                            [0.0, 1.0, 0.0],
                            solid_color(SolidKind::Building, Direction::South),
                        );
                    }
                }
            }
        }
    }

    // The roof drawing is not a generic front-to-back gable. Following the
    // reference building voxelizer, its silhouette controls elevation across
    // X while the authored roof rows map over depth. A shallow constant-
    // thickness slab preserves the drawn eave instead of turning every house
    // into the same triangular prism.
    let roof_slab_pixels = if let Some(style) = burned_tower_roof {
        style.slab_pixels
    } else if kanto_cliff_mound {
        0
    } else {
        roof_pixels.min(4)
    };
    let roof_height_at =
        |x: usize| roof_slab_height(wall_height, roof_slab_pixels, roof_top[x], pixel_z_size);
    let roof_rise = roof_pixels as f32 * pixel_z_size;
    let pitched_height_at_depth =
        |depth: f32| gabled_roof_height(wall_height, roof_rise, depth, roof_depth_pixels as f32);
    if kanto_cliff_mound {
        append_kanto_cliff_cap(
            mesh,
            geometry,
            placement,
            &inside,
            roof_pixels,
            pixel_width,
            building_x0,
            roof_back_z,
            wall_height,
            pixel_x_size,
            pixel_z_size,
        );
    } else {
        if kanto_plan_roof {
            append_plan_shaped_roof(
                &mut mesh.textured,
                &mut mesh.solid,
                &inside,
                geometry,
                placement,
                roof_pixels,
                roof_depth_pixels,
                building_x0,
                roof_back_z,
                wall_height,
                pixel_x_size,
                pixel_z_size,
            );
        }
        for depth_pixel in 0..roof_depth_pixels {
            if kanto_plan_roof {
                continue;
            }
            let z0 = roof_back_z + depth_pixel as f32 * pixel_z_size;
            let z1 = z0 + pixel_z_size;
            for x in 0..pixel_width {
                if roof_top[x] == roof_pixels {
                    continue;
                }
                let x0 = building_x0 + x as f32 * pixel_x_size;
                let x1 = x0 + pixel_x_size;
                let north_height = if gabled_roof {
                    pitched_height_at_depth(depth_pixel as f32)
                } else {
                    roof_height_at(x)
                };
                let south_height = if gabled_roof {
                    pitched_height_at_depth((depth_pixel + 1) as f32)
                } else {
                    north_height
                };
                let source_y = roof_source_row(depth_pixel, roof_depth_pixels, roof_pixels)
                    .max(roof_top[x])
                    .min(roof_pixels - 1);
                if kanto_cliff_mound && !inside[source_y * pixel_width + x] {
                    continue;
                }
                if battle_tower_landmark && !inside[source_y * pixel_width + x] {
                    // The tower sprite contains background between overlapping
                    // roof courses. Once those courses are separated in 3D,
                    // that transparent interval exposes the synthesized lawn
                    // below. Close it with untextured implied roof material;
                    // never paint the source background onto the landmark.
                    append_solid_quad(
                        &mut mesh.solid,
                        [
                            [x0, north_height, z0],
                            [x0, south_height, z1],
                            [x1, south_height, z1],
                            [x1, north_height, z0],
                        ],
                        [0.0, 1.0, 0.0],
                        [0.46, 0.42, 0.38, 1.0],
                    );
                    continue;
                }
                append_quad(
                    &mut mesh.textured,
                    [
                        [x0, north_height, z0],
                        [x0, south_height, z1],
                        [x1, south_height, z1],
                        [x1, north_height, z0],
                    ],
                    if gabled_roof {
                        Vec3::new(0.0, pixel_z_size, north_height - south_height)
                            .normalize()
                            .to_array()
                    } else {
                        [0.0, 1.0, 0.0]
                    },
                    source_pixel_uv(geometry, placement, x, source_y, false),
                    [0.95, 0.95, 0.95, 1.0],
                );
            }
        }

        // Fold the drawing's own edge rows around the slab. Every fascia texel
        // remains one world pixel high; no black outline or roof stripe is
        // stretched down an arbitrary sidewall.
        for x in 0..pixel_width {
            if kanto_plan_roof {
                continue;
            }
            if roof_top[x] == roof_pixels {
                continue;
            }
            let x0 = building_x0 + x as f32 * pixel_x_size;
            let x1 = x0 + pixel_x_size;
            let levels = if gabled_roof {
                TRADITIONAL_ROOF_FASCIA_PIXELS.min(roof_pixels)
            } else {
                ((roof_height_at(x) - wall_height) / pixel_z_size).round() as usize
            };
            for level in 0..levels {
                let bottom = wall_height + level as f32 * pixel_z_size;
                let top = bottom + pixel_z_size;
                let south_source_y = roof_pixels - levels + level;
                let north_source_y = (levels - 1 - level).min(roof_pixels - 1);
                append_quad(
                    &mut mesh.textured,
                    [
                        [x1, bottom, facade_z],
                        [x1, top, facade_z],
                        [x0, top, facade_z],
                        [x0, bottom, facade_z],
                    ],
                    [0.0, 0.0, 1.0],
                    source_pixel_uv(geometry, placement, x, south_source_y, true),
                    TEXTURED_SHADE,
                );
                append_quad(
                    &mut mesh.textured,
                    [
                        [x0, bottom, roof_back_z],
                        [x0, top, roof_back_z],
                        [x1, top, roof_back_z],
                        [x1, bottom, roof_back_z],
                    ],
                    [0.0, 0.0, -1.0],
                    source_pixel_uv(geometry, placement, x, north_source_y, true),
                    [0.68, 0.68, 0.68, 1.0],
                );
            }
        }
    }

    if !kanto_cliff_mound {
        // Close the underside of the eaves created by the inset house body.
        // Without these two narrow soffits the roof overhang exposes the sky
        // between its outer fascia and the side wall—the visible corner gap
        // that made adjacent houses look disconnected.
        if inset_house_walls {
            if wall_x0 > building_x0 {
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        [building_x0, wall_height, roof_back_z],
                        [building_x0, wall_height, facade_z],
                        [wall_x0, wall_height, facade_z],
                        [wall_x0, wall_height, roof_back_z],
                    ],
                    [0.0, -1.0, 0.0],
                    solid_color(SolidKind::Building, Direction::North),
                );
            }
            if wall_x1 < building_x1 {
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        [wall_x1, wall_height, roof_back_z],
                        [wall_x1, wall_height, facade_z],
                        [building_x1, wall_height, facade_z],
                        [building_x1, wall_height, roof_back_z],
                    ],
                    [0.0, -1.0, 0.0],
                    solid_color(SolidKind::Building, Direction::North),
                );
            }
        }

        if straight_modern_facade {
            append_solid_quad(
                &mut mesh.solid,
                [
                    [wall_x0, 0.0, facade_z],
                    [wall_x0, wall_height, facade_z],
                    [wall_x0, wall_height, roof_back_z],
                    [wall_x0, 0.0, roof_back_z],
                ],
                [-1.0, 0.0, 0.0],
                solid_color(SolidKind::Building, Direction::West),
            );
            append_solid_quad(
                &mut mesh.solid,
                [
                    [wall_x1, 0.0, roof_back_z],
                    [wall_x1, wall_height, roof_back_z],
                    [wall_x1, wall_height, facade_z],
                    [wall_x1, 0.0, facade_z],
                ],
                [1.0, 0.0, 0.0],
                solid_color(SolidKind::Building, Direction::East),
            );
        } else {
            for source_y in roof_pixels..pixel_height {
                let west_source_x = facade_side_course_x(
                    &inside,
                    &luminance,
                    pixel_width,
                    source_y,
                    darkest,
                    false,
                );
                let east_source_x =
                    facade_side_course_x(&inside, &luminance, pixel_width, source_y, darkest, true);
                let y_top = (pixel_height - source_y) as f32 * pixel_z_size;
                let y_bottom = y_top - pixel_z_size;
                for depth_pixel in 0..roof_depth_pixels {
                    let z0 = roof_back_z + depth_pixel as f32 * pixel_z_size;
                    let z1 = z0 + pixel_z_size;
                    // A facade side is the drawing's visible edge carried backward
                    // through the building depth. Sweeping across the entire source
                    // tile here repeatedly sampled its black window/outline pixels,
                    // producing nearly black sidewalls. The reference building fold
                    // clamps to the measured edge course; preserve that exact course
                    // at every depth pixel instead of inventing new side artwork.
                    append_quad(
                        &mut mesh.textured,
                        [
                            [wall_x0, y_bottom, z1],
                            [wall_x0, y_top, z1],
                            [wall_x0, y_top, z0],
                            [wall_x0, y_bottom, z0],
                        ],
                        [-1.0, 0.0, 0.0],
                        source_pixel_uv(geometry, placement, west_source_x, source_y, true),
                        [0.78, 0.78, 0.78, 1.0],
                    );
                    append_quad(
                        &mut mesh.textured,
                        [
                            [wall_x1, y_bottom, z0],
                            [wall_x1, y_top, z0],
                            [wall_x1, y_top, z1],
                            [wall_x1, y_bottom, z1],
                        ],
                        [1.0, 0.0, 0.0],
                        source_pixel_uv(geometry, placement, east_source_x, source_y, true),
                        [0.86, 0.86, 0.86, 1.0],
                    );
                }
            }
        }

        // The source drawing supplies a front stack, but the building is a
        // closed volume. Carry that stack onto the rear plane at reduced
        // light, as the reference mesher does for non-front faces. Omitting
        // this plane exposed the ground through Ecruteak's rear eaves and
        // between adjoining traditional roof sections.
        for source_y in roof_pixels..pixel_height {
            let y_top = (pixel_height - source_y) as f32 * pixel_z_size;
            let y_bottom = y_top - pixel_z_size;
            for x in body_left..=body_right {
                let x0 = building_x0 + x as f32 * pixel_x_size;
                let x1 = x0 + pixel_x_size;
                append_quad(
                    &mut mesh.textured,
                    [
                        [x0, y_bottom, roof_back_z],
                        [x0, y_top, roof_back_z],
                        [x1, y_top, roof_back_z],
                        [x1, y_bottom, roof_back_z],
                    ],
                    [0.0, 0.0, -1.0],
                    source_pixel_uv(geometry, placement, x, source_y, true),
                    [0.68, 0.68, 0.68, 1.0],
                );
            }
        }
    }

    if !kanto_cliff_mound && !kanto_plan_roof {
        for (x, source_tile_x, normal, shade, reverse) in [
            (building_x0, 0, [-1.0, 0.0, 0.0], 0.78, false),
            (
                building_x1,
                pixel_width.saturating_sub(SOURCE_TILE_PIXELS),
                [1.0, 0.0, 0.0],
                0.86,
                true,
            ),
        ] {
            let edge_x = if reverse {
                pixel_width - 1
            } else {
                source_tile_x
            };
            if roof_top[edge_x] == roof_pixels {
                continue;
            }
            // Physical depth and source roof rows are normally equal, but
            // large landmarks deliberately repeat their native roof courses
            // over a bounded footprint. Side iteration must follow geometry
            // depth; `roof_source_row` maps that depth back into source art.
            let side_depth_pixels = roof_depth_pixels;
            for depth_pixel in 0..side_depth_pixels {
                let z0 = facade_z - side_depth_pixels as f32 * pixel_z_size
                    + depth_pixel as f32 * pixel_z_size;
                let z1 = z0 + pixel_z_size;
                let top = if gabled_roof {
                    pitched_height_at_depth(depth_pixel as f32)
                } else {
                    roof_height_at(edge_x)
                };
                let levels = ((top - wall_height) / pixel_z_size).round() as usize;
                // A generated side carries the drawing's outermost edge
                // backward through depth. Sweeping through all eight source
                // columns repeats windows, roof stripes, and highlights down
                // the side (most visibly on Kanto's cyan caps), making a flat
                // rectangular roof look stepped. The top surface already owns
                // the complete roof drawing; the side owns only this edge.
                let course_x = edge_x;
                for level in 0..levels {
                    let bottom = wall_height + level as f32 * pixel_z_size;
                    let band_top = bottom + pixel_z_size;
                    let positions = if normal[0] < 0.0 {
                        [
                            [x, bottom, z1],
                            [x, band_top, z1],
                            [x, band_top, z0],
                            [x, bottom, z0],
                        ]
                    } else {
                        [
                            [x, bottom, z0],
                            [x, band_top, z0],
                            [x, band_top, z1],
                            [x, bottom, z1],
                        ]
                    };
                    append_quad(
                        &mut mesh.textured,
                        positions,
                        normal,
                        source_pixel_uv(
                            geometry,
                            placement,
                            course_x,
                            roof_source_row(depth_pixel, roof_depth_pixels, roof_pixels),
                            true,
                        ),
                        [shade, shade, shade, 1.0],
                    );
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_kanto_cliff_cap(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    placement: BuildingPlacement,
    _inside: &[bool],
    depth: usize,
    width: usize,
    origin_x: f32,
    origin_z: f32,
    height: f32,
    pixel_x_size: f32,
    pixel_z_size: f32,
) {
    // This authored mound is a continuous plateau. Dark pixels in its source
    // image describe the cave mouth and talus, so image-luminance flood fill
    // cannot be used as a transparency/topology mask here.
    // The two outer 16-pixel columns contain Crystal's directional slope
    // paint, but they are still part of one continuous rectangular drawing.
    // Removing them from the footprint opens false corner gaps. Keep the cap
    // continuous; the battered wall below supplies the physical slope.
    let slope_pixels = SOURCE_TILE_PIXELS * 2;
    let bounds = |_row: usize| Some((0, width));
    // Rock banks in the reference renderer are battered rather than vertical:
    // the base projects beyond the plateau edge, producing a real angled wall
    // without inventing staircase tiers or stretching a source tile.
    let batter = pixel_x_size * slope_pixels as f32;
    let uv = |x: usize, y: usize| {
        [
            (placement.column * SOURCE_TILE_PIXELS + x) as f32
                / (geometry.width * SOURCE_TILE_PIXELS) as f32,
            (placement.row * SOURCE_TILE_PIXELS + y) as f32
                / (geometry.height * SOURCE_TILE_PIXELS) as f32,
        ]
    };

    for row in 0..depth {
        let Some((left0, right0)) = bounds(row) else {
            continue;
        };
        let (left1, right1) = bounds((row + 1).min(depth - 1)).unwrap_or((left0, right0));
        let z0 = origin_z + row as f32 * pixel_z_size;
        let z1 = z0 + pixel_z_size;
        let lx0 = origin_x + left0 as f32 * pixel_x_size;
        let lx1 = origin_x + left1 as f32 * pixel_x_size;
        let rx0 = origin_x + right0 as f32 * pixel_x_size;
        let rx1 = origin_x + right1 as f32 * pixel_x_size;

        append_quad(
            &mut mesh.textured,
            [
                [lx0, height, z0],
                [lx1, height, z1],
                [rx1, height, z1],
                [rx0, height, z0],
            ],
            [0.0, 1.0, 0.0],
            [
                uv(left0, row),
                uv(left1, row + 1),
                uv(right1, row + 1),
                uv(right0, row),
            ],
            BANK_TOP_SHADE,
        );
        let west_normal = Vec3::new(-height, batter, 0.0).normalize().to_array();
        let east_normal = Vec3::new(height, batter, 0.0).normalize().to_array();
        // Fold Crystal's two 16px directional slope strips onto the battered
        // walls. Depth advances through the source drawing one pixel row at
        // a time, while height crosses the complete 16px strip. This is the
        // reference mesher's native-band rule applied to the Gen 2 artwork:
        // no plateau tile is repeated or stretched down the cliff.
        append_quad(
            &mut mesh.textured,
            [
                [lx0 - batter, 0.0, z0],
                [lx0, height, z0],
                [lx1, height, z1],
                [lx1 - batter, 0.0, z1],
            ],
            west_normal,
            [
                uv(0, row),
                uv(slope_pixels, row),
                uv(slope_pixels, row + 1),
                uv(0, row + 1),
            ],
            [0.78, 0.78, 0.78, 1.0],
        );
        append_quad(
            &mut mesh.textured,
            [
                [rx0 + batter, 0.0, z0],
                [rx1 + batter, 0.0, z1],
                [rx1, height, z1],
                [rx0, height, z0],
            ],
            east_normal,
            [
                uv(width, row),
                uv(width, row + 1),
                uv(width - slope_pixels, row + 1),
                uv(width - slope_pixels, row),
            ],
            [0.86, 0.86, 0.86, 1.0],
        );

        if row == 0 {
            append_solid_quad(
                &mut mesh.solid,
                [
                    [rx0 + batter, 0.0, z0 - batter],
                    [rx0, height, z0],
                    [lx0, height, z0],
                    [lx0 - batter, 0.0, z0 - batter],
                ],
                [0.0, 0.0, -1.0],
                solid_color(SolidKind::Bank, Direction::North),
            );
        }
    }
}

/// Chooses one authored facade texel to carry through a building side.
///
/// The silhouette's nearest occupied pixel is normally its black outline.
/// Extruding that single pixel through the complete footprint makes the side
/// an unreadable black sheet. Use the most common non-background course in
/// this source row, then choose its nearest occurrence from the requested
/// edge. Horizontal siding, brick, eaves, and base bands therefore remain
/// continuous without copying windows or doors around the corner.
fn facade_side_course_x(
    inside: &[bool],
    luminance: &[u16],
    width: usize,
    row: usize,
    darkest: u16,
    from_east: bool,
) -> usize {
    let start = row * width;
    let mut counts = HashMap::<u16, usize>::new();
    for x in 0..width {
        let value = luminance[start + x];
        if inside[start + x] && value > darkest {
            *counts.entry(value).or_default() += 1;
        }
    }
    let dominant = counts
        .into_iter()
        .max_by_key(|(value, count)| (*count, *value))
        .map(|(value, _)| value);
    let fallback = if from_east { width - 1 } else { 0 };
    let Some(dominant) = dominant else {
        return fallback;
    };
    if from_east {
        (0..width)
            .rev()
            .find(|&x| inside[start + x] && luminance[start + x] == dominant)
            .unwrap_or(fallback)
    } else {
        (0..width)
            .find(|&x| inside[start + x] && luminance[start + x] == dominant)
            .unwrap_or(fallback)
    }
}

fn common_flat_ground_outside(
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: BuildingPlacement,
) -> Option<usize> {
    let mut votes: HashMap<u16, (usize, usize)> = HashMap::new();
    for (index, (tile, shape)) in cells.iter().zip(shapes).enumerate() {
        let column = index % geometry.width;
        let row = index / geometry.width;
        let inside_building = column >= placement.column
            && column < placement.column + placement.width
            && row >= placement.row
            && row < placement.row + placement.height;
        if inside_building || !matches!(shape, CellShape::Flat) {
            continue;
        }
        let vote = votes.entry(tile.source.tile_index).or_insert((0, index));
        vote.0 += 1;
        if (tile.row, tile.column) < (cells[vote.1].row, cells[vote.1].column) {
            vote.1 = index;
        }
    }
    votes
        .into_iter()
        .max_by_key(|(tile_index, (count, _))| (*count, std::cmp::Reverse(*tile_index)))
        .map(|(_, (_, index))| index)
}

fn facade_recess_mask(
    inside: &[bool],
    luminance: &[u16],
    width: usize,
    height: usize,
    facade_start: usize,
    darkest: u16,
) -> Vec<bool> {
    const MAX_PANE_EXTENT: usize = 24;
    let mut recessed = vec![false; width * height];
    let mut seen = vec![false; width * height];
    for start_y in facade_start..height {
        for start_x in 0..width {
            let start = start_y * width + start_x;
            if seen[start] || !inside[start] || luminance[start] == darkest {
                continue;
            }
            let mut queue = VecDeque::from([start]);
            let mut component = Vec::new();
            let (mut min_x, mut max_x, mut min_y, mut max_y) = (start_x, start_x, start_y, start_y);
            seen[start] = true;
            while let Some(index) = queue.pop_front() {
                component.push(index);
                let x = index % width;
                let y = index / width;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                for (nx, ny) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if nx >= width || ny < facade_start || ny >= height {
                        continue;
                    }
                    let neighbor = ny * width + nx;
                    if !seen[neighbor] && inside[neighbor] && luminance[neighbor] != darkest {
                        seen[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
            if max_x - min_x < MAX_PANE_EXTENT && max_y - min_y < MAX_PANE_EXTENT {
                for index in component {
                    recessed[index] = true;
                }
            }
        }
    }
    recessed
}

fn roof_slab_height(
    wall_height: f32,
    slab_pixels: usize,
    silhouette_inset: usize,
    pixel_size: f32,
) -> f32 {
    wall_height + slab_pixels.saturating_sub(silhouette_inset) as f32 * pixel_size
}

fn building_facade_plane_z(building_z0: f32, pixel_height: usize, pixel_z_size: f32) -> f32 {
    building_z0 + pixel_height as f32 * pixel_z_size
}

#[allow(clippy::too_many_arguments)]
fn append_plan_shaped_roof(
    textured: &mut SurfaceMeshData,
    solid: &mut SurfaceMeshData,
    inside: &[bool],
    geometry: &GridGeometry,
    placement: BuildingPlacement,
    roof_pixels: usize,
    roof_depth_pixels: usize,
    building_x0: f32,
    roof_back_z: f32,
    wall_height: f32,
    pixel_x_size: f32,
    pixel_z_size: f32,
) {
    let width = placement.width * SOURCE_TILE_PIXELS;
    if width == 0 || roof_pixels == 0 || roof_depth_pixels == 0 {
        return;
    }

    // The checkerboard end caps in Kanto's drawing are lateral roof slopes,
    // not a footprint that narrows toward the back. Measure the silhouette
    // across X, as the reference voxelizer does, then interpolate adjacent
    // pixel columns into continuous sloped quads. This preserves every roof
    // texel while avoiding both a staircase profile and a stretched side.
    let profile = measured_roof_profile(inside, width, roof_pixels);
    let cell_height: Vec<_> = profile
        .iter()
        .map(|&inset| roof_slab_height(wall_height, roof_pixels.min(4), inset, pixel_z_size))
        .collect();
    let vertex_height = |x: usize| {
        if x == 0 {
            cell_height[0]
        } else if x == width {
            cell_height[width - 1]
        } else {
            (cell_height[x - 1] + cell_height[x]) * 0.5
        }
    };

    for depth in 0..roof_depth_pixels {
        let source_row = roof_source_row(depth, roof_depth_pixels, roof_pixels);
        let z0 = roof_back_z + depth as f32 * pixel_z_size;
        let z1 = z0 + pixel_z_size;
        for x in 0..width {
            if profile[x] >= roof_pixels {
                continue;
            }
            let x0 = building_x0 + x as f32 * pixel_x_size;
            let x1 = x0 + pixel_x_size;
            let left_height = vertex_height(x);
            let right_height = vertex_height(x + 1);
            append_quad(
                textured,
                [
                    [x0, left_height, z0],
                    [x0, left_height, z1],
                    [x1, right_height, z1],
                    [x1, right_height, z0],
                ],
                Vec3::new(left_height - right_height, pixel_x_size, 0.0)
                    .normalize_or_zero()
                    .to_array(),
                source_pixel_uv(geometry, placement, x, source_row, false),
                [0.95, 0.95, 0.95, 1.0],
            );
        }
    }

    for (z, direction, normal) in [
        (roof_back_z, Direction::North, [0.0, 0.0, -1.0]),
        (
            roof_back_z + roof_depth_pixels as f32 * pixel_z_size,
            Direction::South,
            [0.0, 0.0, 1.0],
        ),
    ] {
        for x in 0..width {
            if profile[x] >= roof_pixels {
                continue;
            }
            let x0 = building_x0 + x as f32 * pixel_x_size;
            let x1 = x0 + pixel_x_size;
            let positions = if matches!(direction, Direction::South) {
                [
                    [x1, wall_height, z],
                    [x1, vertex_height(x + 1), z],
                    [x0, vertex_height(x), z],
                    [x0, wall_height, z],
                ]
            } else {
                [
                    [x0, wall_height, z],
                    [x0, vertex_height(x), z],
                    [x1, vertex_height(x + 1), z],
                    [x1, wall_height, z],
                ]
            };
            append_solid_quad(
                solid,
                positions,
                normal,
                solid_color(SolidKind::Building, direction),
            );
        }
    }

    let west_height = vertex_height(0);
    let east_height = vertex_height(width);
    append_solid_quad(
        solid,
        [
            [building_x0, wall_height, roof_back_z],
            [
                building_x0,
                wall_height,
                roof_back_z + roof_depth_pixels as f32 * pixel_z_size,
            ],
            [
                building_x0,
                west_height,
                roof_back_z + roof_depth_pixels as f32 * pixel_z_size,
            ],
            [building_x0, west_height, roof_back_z],
        ],
        [-1.0, 0.0, 0.0],
        solid_color(SolidKind::Building, Direction::West),
    );
    let east_x = building_x0 + width as f32 * pixel_x_size;
    append_solid_quad(
        solid,
        [
            [east_x, wall_height, roof_back_z],
            [east_x, east_height, roof_back_z],
            [
                east_x,
                east_height,
                roof_back_z + roof_depth_pixels as f32 * pixel_z_size,
            ],
            [
                east_x,
                wall_height,
                roof_back_z + roof_depth_pixels as f32 * pixel_z_size,
            ],
        ],
        [1.0, 0.0, 0.0],
        solid_color(SolidKind::Building, Direction::East),
    );
}

fn measured_roof_profile(inside: &[bool], width: usize, roof_rows: usize) -> Vec<usize> {
    let raw: Vec<_> = (0..width)
        .map(|x| {
            (0..roof_rows)
                .find(|&y| inside[y * width + x])
                .unwrap_or(roof_rows)
        })
        .collect();
    let mut filtered = raw.clone();
    for (x, top) in filtered.iter_mut().enumerate() {
        let start = x.saturating_sub(2);
        let end = (x + 3).min(width);
        let mut neighborhood: Vec<_> = raw[start..end]
            .iter()
            .copied()
            .filter(|&value| value < roof_rows)
            .collect();
        if neighborhood.len() >= 3 {
            neighborhood.sort_unstable();
            *top = neighborhood[neighborhood.len() / 2];
        }
    }
    filtered
}

fn gabled_roof_height(wall_height: f32, rise: f32, depth: f32, total_depth: f32) -> f32 {
    let half_depth = (total_depth * 0.5).max(f32::EPSILON);
    let distance_from_ridge = (depth - half_depth).abs();
    wall_height + rise * (1.0 - distance_from_ridge / half_depth).clamp(0.0, 1.0)
}

fn roof_source_row(depth: usize, total_depth: usize, source_rows: usize) -> usize {
    debug_assert!(source_rows > 0);
    let rim = 4.min(source_rows / 2);
    let from_front = total_depth - 1 - depth;
    if depth < rim {
        return depth;
    }
    if from_front < rim {
        return source_rows - 1 - from_front;
    }
    let cycle_start = rim;
    let cycle_len = source_rows.saturating_sub(rim * 2).max(1);
    cycle_start + (depth - rim) % cycle_len
}

fn source_pixel_uv(
    geometry: &GridGeometry,
    placement: BuildingPlacement,
    pixel_x: usize,
    pixel_y: usize,
    upright: bool,
) -> [[f32; 2]; 4] {
    let column = placement.column + pixel_x / SOURCE_TILE_PIXELS;
    let row = placement.row + pixel_y / SOURCE_TILE_PIXELS;
    let local_x = pixel_x % SOURCE_TILE_PIXELS;
    let local_y = pixel_y % SOURCE_TILE_PIXELS;
    let (tile_u0, tile_u1, tile_v0, tile_v1) = geometry.uv(column, row);
    let u0 = lerp_pixel(tile_u0, tile_u1, local_x);
    let u1 = lerp_pixel(tile_u0, tile_u1, local_x + 1);
    let v0 = lerp_pixel(tile_v0, tile_v1, local_y);
    let v1 = lerp_pixel(tile_v0, tile_v1, local_y + 1);
    if upright {
        [[u1, v1], [u1, v0], [u0, v0], [u0, v1]]
    } else {
        [[u0, v0], [u0, v1], [u1, v1], [u1, v0]]
    }
}

#[derive(Clone, Copy)]
struct GridGeometry {
    width: usize,
    height: usize,
    tile_width: f32,
    tile_height: f32,
    origin_x: f32,
    origin_z: f32,
}

impl GridGeometry {
    fn bounds(self, column: usize, row: usize) -> (f32, f32, f32, f32) {
        let x0 = self.origin_x + column as f32 * self.tile_width;
        let z0 = self.origin_z + row as f32 * self.tile_height;
        (x0, x0 + self.tile_width, z0, z0 + self.tile_height)
    }

    fn uv(self, column: usize, row: usize) -> (f32, f32, f32, f32) {
        (
            column as f32 / self.width as f32,
            (column + 1) as f32 / self.width as f32,
            row as f32 / self.height as f32,
            (row + 1) as f32 / self.height as f32,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn append_textured_cell(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    bank_runs: &[Option<BankColumnRun>],
    column: usize,
    row: usize,
    index: usize,
    images: Option<&TerrainImageSamples>,
) -> Result<(), TerrainMeshError> {
    let (x0, x1, z0, z1) = geometry.bounds(column, row);
    let bank_bounds = [x0, x1, z0, z1];
    if let Some(run) = bank_runs[index] {
        if let CellShape::LedgeBand { top_tile_index, .. } = shapes[index] {
            let surface_height = shapes[index].surface_height(geometry.tile_height);
            let source = authored_metatile_cell(cells, index, top_tile_index)
                .or_else(|| {
                    authored_surface_cell(
                        cells,
                        shapes,
                        top_tile_index,
                        surface_height,
                        geometry.tile_height,
                    )
                })
                .unwrap_or(index);
            append_top_shaded(
                &mut mesh.textured,
                bank_bounds,
                surface_height,
                geometry.uv(source % geometry.width, source / geometry.width),
                BANK_TOP_SHADE,
            );
            return Ok(());
        }
        // The connected Johto mountain family already classifies every
        // authored edge cell as a directional LedgeBand above. What remains
        // is genuine plateau art and must stay at its original grid cell.
        // Recycling the first two rows of the bank run paints cliff openings
        // repeatedly across the cap.
        if cells[index].source.tileset_id.as_ref() == "johto"
            && (0x68..=0x73).contains(&cells[index].source.metatile_id)
        {
            append_top_shaded(
                &mut mesh.textured,
                bank_bounds,
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(column, row),
                BANK_TOP_SHADE,
            );
            return Ok(());
        }
        // `$0a` is one complete 4x4 rock-platform drawing. Preserve every
        // authored cap cell exactly once; the generic bank-top path repeats a
        // two-row course and turns this drawing into stripes.
        if matches!(
            cells[index].source.tileset_id.as_ref(),
            "johto" | "johto_modern"
        ) && cells[index].source.metatile_id == 0x0a
        {
            let corner_clips = rock_platform_corner_clips(cells, shapes, geometry, column, row);
            append_rock_platform_top(
                &mut mesh.textured,
                bank_bounds,
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(column, row),
                cells[index].source.subtile_column,
                cells[index].source.subtile_row,
                corner_clips,
                BANK_TOP_SHADE,
            );
            return Ok(());
        }
        // Kanto's cave-mound cap mixes plateau, directional slope, and talus
        // art in the same six blocks. Only $01/$11 depict the plateau. Keep
        // those exact cells where available and use a local $11 cell for a
        // folded slope's vacated cap; never cycle the front courses northward
        // across the whole mound.
        if cells[index].source.tileset_id.as_ref() == "kanto"
            && matches!(
                cells[index].source.metatile_id,
                0x3e | 0x3f | 0x3b | 0x24 | 0x06 | 0x25
            )
            && shapes[index].surface_height(geometry.tile_height) >= 16.0
        {
            let source = if matches!(cells[index].source.tile_index, 0x01 | 0x11) {
                index
            } else {
                authored_metatile_cell(cells, index, 0x11).unwrap_or(index)
            };
            append_top_shaded(
                &mut mesh.textured,
                bank_bounds,
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(source % geometry.width, source / geometry.width),
                BANK_TOP_SHADE,
            );
            return Ok(());
        }
        let extent = run.front - run.north + 1;
        let repeat = extent.min(2);
        let source_row = run.north + (row - run.north) % repeat;
        let source = source_row * geometry.width + column;
        append_top_shaded(
            &mut mesh.textured,
            bank_bounds,
            shapes[index].surface_height(geometry.tile_height),
            geometry.uv(source % geometry.width, source / geometry.width),
            BANK_TOP_SHADE,
        );
        return Ok(());
    }
    match shapes[index] {
        CellShape::RaisedTop {
            solid: SolidKind::Bank,
            ..
        } => {
            append_top(
                &mut mesh.textured,
                bank_bounds,
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(column, row),
            );
        }
        CellShape::Flat
        | CellShape::Water
        | CellShape::PlaneAt { .. }
        | CellShape::RaisedTop { .. } => {
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(column, row),
            );
        }
        CellShape::RampNorth {
            north_height,
            south_height,
        } => {
            let scale = geometry.tile_height / SOURCE_TILE_HEIGHT;
            let north_height = north_height * scale;
            let south_height = south_height * scale;
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            let normal = Vec3::new(0.0, geometry.tile_height, north_height - south_height)
                .normalize()
                .to_array();
            append_quad(
                &mut mesh.textured,
                [
                    [x0, south_height, z1],
                    [x0, north_height, z0],
                    [x1, north_height, z0],
                    [x1, south_height, z1],
                ],
                normal,
                [[u0, v1], [u0, v0], [u1, v0], [u1, v1]],
                BANK_TOP_SHADE,
            );
        }
        CellShape::Waterfall => {
            let replacement = authored_water_cell(cells, shapes).ok_or(
                TerrainMeshError::MissingGroundSample {
                    column: column as u32,
                    row: row as u32,
                    tile_index: 0x14,
                },
            )?;
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(replacement % geometry.width, replacement / geometry.width),
            );
        }
        CellShape::ShoreBand => {
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(column, row),
            );
        }
        CellShape::Cutout {
            ground_tile_index,
            solid,
        } => {
            let replacement = authored_ground_cell(cells, shapes, ground_tile_index).ok_or(
                TerrainMeshError::MissingGroundSample {
                    column: column as u32,
                    row: row as u32,
                    tile_index: ground_tile_index,
                },
            )?;
            let base_height = shapes[replacement].surface_height(geometry.tile_height);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                base_height,
                geometry.uv(replacement % geometry.width, replacement / geometry.width),
            );
            if let Some(images) = images {
                let removable =
                    decorative_cutout_mask(images, cells[index], cells[replacement], solid)?;
                let depth = upright_depth(solid);
                let middle = upright_plane_z(solid, z0, z1);
                let (u0, u1, v0, v1) = geometry.uv(column, row);
                append_masked_upright_hull(
                    &mut mesh.textured,
                    &mut mesh.solid,
                    &removable,
                    [
                        x0,
                        x1,
                        base_height,
                        base_height + geometry.tile_height,
                        middle + depth * 0.5,
                    ],
                    [u0, u1, v0, v1],
                    solid,
                )?;
            }
        }
        CellShape::Relief {
            height,
            ground_tile_index,
            ..
        } => {
            let replacement = authored_relief_base_cell(cells, shapes, ground_tile_index).ok_or(
                TerrainMeshError::MissingGroundSample {
                    column: column as u32,
                    row: row as u32,
                    tile_index: ground_tile_index,
                },
            )?;
            let base_height = shapes[replacement].surface_height(geometry.tile_height);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                base_height,
                geometry.uv(replacement % geometry.width, replacement / geometry.width),
            );
            if let Some(images) = images {
                append_pixel_relief(
                    mesh,
                    images,
                    cells[index],
                    cells[replacement],
                    [x0, x1, z0, z1],
                    geometry.uv(column, row),
                    base_height,
                    height * geometry.tile_height / SOURCE_TILE_HEIGHT,
                )?;
            }
        }
        CellShape::FacadeBand {
            plane_subtile_row,
            band_from_top,
            band_count,
            ground_tile_index,
            solid,
        } => {
            let replacement = authored_ground_cell(cells, shapes, ground_tile_index).ok_or(
                TerrainMeshError::MissingGroundSample {
                    column: column as u32,
                    row: row as u32,
                    tile_index: ground_tile_index,
                },
            )?;
            let base_height = shapes[replacement].surface_height(geometry.tile_height);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                base_height,
                geometry.uv(replacement % geometry.width, replacement / geometry.width),
            );

            let tile = cells[index];
            // The viewport may clip the metatile origin, so this is signed.
            // A facade plane just outside the viewport is still valid geometry.
            let metatile_origin_row = row as isize - tile.source.subtile_row as isize;
            let plane_row = metatile_origin_row + plane_subtile_row as isize;
            let plane_z = geometry.origin_z + plane_row as f32 * geometry.tile_height;
            let band_top = base_height + (band_count - band_from_top) as f32 * geometry.tile_height;
            let band_bottom = band_top - geometry.tile_height;
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            if matches!(
                solid,
                SolidKind::Tree
                    | SolidKind::Rock
                    | SolidKind::FlatCard
                    | SolidKind::Prop
                    | SolidKind::CutTree
                    | SolidKind::Fence
            ) && let Some(images) = images
            {
                let removable_ground = grouped_boundary_ground_mask(
                    images,
                    cells,
                    shapes,
                    geometry,
                    column,
                    row,
                    index,
                    cells[replacement],
                )?;
                append_masked_upright_hull(
                    &mut mesh.textured,
                    &mut mesh.solid,
                    &removable_ground,
                    [x0, x1, band_bottom, band_top, plane_z],
                    [u0, u1, v0, v1],
                    solid,
                )?;
            } else {
                append_quad(
                    &mut mesh.textured,
                    [
                        [x1, band_bottom, plane_z],
                        [x1, band_top, plane_z],
                        [x0, band_top, plane_z],
                        [x0, band_bottom, plane_z],
                    ],
                    [0.0, 0.0, 1.0],
                    [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                    TEXTURED_SHADE,
                );
            }
        }
        CellShape::LedgeBand { .. } => unreachable!("bank runs are emitted before cell matching"),
    }
    Ok(())
}

fn authored_metatile_cell(cells: &[&VisualTile], index: usize, tile_index: u16) -> Option<usize> {
    let source = &cells[index].source;
    let origin_column = cells[index].column as i32 - i32::from(source.subtile_column);
    let origin_row = cells[index].row as i32 - i32::from(source.subtile_row);
    cells.iter().position(|candidate| {
        candidate.source.tileset_id == source.tileset_id
            && candidate.source.metatile_id == source.metatile_id
            && candidate.source.tile_index == tile_index
            && candidate.column as i32 - i32::from(candidate.source.subtile_column) == origin_column
            && candidate.row as i32 - i32::from(candidate.source.subtile_row) == origin_row
    })
}

fn casino_stool_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<CasinoStoolPlacement> {
    let mut placements = Vec::new();
    for (index, tile) in cells.iter().enumerate() {
        let source = &tile.source;
        let is_origin = source.tileset_id.as_ref() == "game_corner"
            && ((source.metatile_id == 0x05 && source.subtile_column == 2)
                || (source.metatile_id == 0x06 && source.subtile_column == 0))
            && matches!(source.subtile_row, 0 | 2);
        if !is_origin {
            continue;
        }
        let column = index % geometry.width;
        let row = index / geometry.width;
        if column + 1 >= geometry.width || row + 1 >= geometry.height {
            continue;
        }
        let expected = [[0x0a, 0x0b], [0x1a, 0x1b]];
        let complete = (0..2).all(|dy| {
            (0..2).all(|dx| {
                let candidate = cells[(row + dy) * geometry.width + column + dx];
                candidate.source.tileset_id.as_ref() == "game_corner"
                    && candidate.source.metatile_id == source.metatile_id
                    && candidate.source.tile_index == expected[dy][dx]
            })
        });
        if complete {
            placements.push(CasinoStoolPlacement { column, row });
        }
    }
    placements
}

#[allow(clippy::too_many_arguments)]
fn append_casino_stool(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: CasinoStoolPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    const CROP_LEFT: usize = 2;
    const CROP_RIGHT: usize = 14;
    const TOP_FIRST_ROW: usize = 4;
    const TOP_LAST_ROW: usize = 10;
    const FRONT_FIRST_ROW: usize = 10;
    const FRONT_LAST_ROW: usize = 15;
    const SEAT_DEPTH: f32 = 11.0;
    const SEAT_HEIGHT: f32 = 5.0;

    let replacement =
        authored_ground_cell(cells, shapes, 0x01).ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: 0x01,
        })?;
    for dy in 0..2 {
        for dx in 0..2 {
            let column = placement.column + dx;
            let row = placement.row + dy;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(replacement % geometry.width, replacement / geometry.width),
            );
            claimed[row * geometry.width + column] = true;
        }
    }

    let (origin_x, _, origin_z, _) = geometry.bounds(placement.column, placement.row);
    let pixel_width = geometry.tile_width / SOURCE_TILE_PIXELS as f32;
    let pixel_height = geometry.tile_height / SOURCE_TILE_PIXELS as f32;
    let x_at = |pixel: usize| origin_x + pixel as f32 * pixel_width;
    let top_z_at = |pixel: usize| {
        origin_z
            + (pixel - TOP_FIRST_ROW) as f32 * SEAT_DEPTH * pixel_height
                / (TOP_LAST_ROW - TOP_FIRST_ROW) as f32
    };
    let front_z = origin_z + SEAT_DEPTH * pixel_height;
    let scaled_height = SEAT_HEIGHT * pixel_height;

    for source_y in TOP_FIRST_ROW..TOP_LAST_ROW {
        for source_x in CROP_LEFT..CROP_RIGHT {
            let cell_column = placement.column + source_x / SOURCE_TILE_PIXELS;
            let cell_row = placement.row + source_y / SOURCE_TILE_PIXELS;
            let (u0, u1, v0, v1) = geometry.uv(cell_column, cell_row);
            let local_x = source_x % SOURCE_TILE_PIXELS;
            let local_y = source_y % SOURCE_TILE_PIXELS;
            let pu0 = lerp_pixel(u0, u1, local_x);
            let pu1 = lerp_pixel(u0, u1, local_x + 1);
            let pv0 = lerp_pixel(v0, v1, local_y);
            let pv1 = lerp_pixel(v0, v1, local_y + 1);
            append_quad(
                &mut mesh.textured,
                [
                    [x_at(source_x), scaled_height, top_z_at(source_y)],
                    [x_at(source_x), scaled_height, top_z_at(source_y + 1)],
                    [x_at(source_x + 1), scaled_height, top_z_at(source_y + 1)],
                    [x_at(source_x + 1), scaled_height, top_z_at(source_y)],
                ],
                [0.0, 1.0, 0.0],
                [[pu0, pv0], [pu0, pv1], [pu1, pv1], [pu1, pv0]],
                TEXTURED_SHADE,
            );
        }
    }
    for source_y in FRONT_FIRST_ROW..FRONT_LAST_ROW {
        for source_x in CROP_LEFT..CROP_RIGHT {
            let cell_column = placement.column + source_x / SOURCE_TILE_PIXELS;
            let cell_row = placement.row + source_y / SOURCE_TILE_PIXELS;
            let tile = cells[cell_row * geometry.width + cell_column];
            let _ = tile_rgba(images, tile)?;
            let (u0, u1, v0, v1) = geometry.uv(cell_column, cell_row);
            let local_x = source_x % SOURCE_TILE_PIXELS;
            let local_y = source_y % SOURCE_TILE_PIXELS;
            let pu0 = lerp_pixel(u0, u1, local_x);
            let pu1 = lerp_pixel(u0, u1, local_x + 1);
            let pv0 = lerp_pixel(v0, v1, local_y);
            let pv1 = lerp_pixel(v0, v1, local_y + 1);
            let y_top = (FRONT_LAST_ROW - source_y) as f32 * pixel_height;
            let y_bottom = y_top - pixel_height;
            append_quad(
                &mut mesh.textured,
                [
                    [x_at(source_x + 1), y_bottom, front_z],
                    [x_at(source_x + 1), y_top, front_z],
                    [x_at(source_x), y_top, front_z],
                    [x_at(source_x), y_bottom, front_z],
                ],
                [0.0, 0.0, 1.0],
                [[pu1, pv1], [pu1, pv0], [pu0, pv0], [pu0, pv1]],
                TEXTURED_SHADE,
            );
        }
    }
    let x0 = x_at(CROP_LEFT);
    let x1 = x_at(CROP_RIGHT);
    for (positions, direction) in [
        (
            [
                [x0, 0.0, origin_z],
                [x0, scaled_height, origin_z],
                [x0, scaled_height, front_z],
                [x0, 0.0, front_z],
            ],
            Direction::West,
        ),
        (
            [
                [x1, 0.0, front_z],
                [x1, scaled_height, front_z],
                [x1, scaled_height, origin_z],
                [x1, 0.0, origin_z],
            ],
            Direction::East,
        ),
    ] {
        append_solid_quad(
            &mut mesh.solid,
            positions,
            match direction {
                Direction::West => [-1.0, 0.0, 0.0],
                Direction::East => [1.0, 0.0, 0.0],
                _ => unreachable!(),
            },
            solid_color(SolidKind::Prop, direction),
        );
    }
    Ok(())
}

fn authored_ground_cell(
    cells: &[&VisualTile],
    shapes: &[CellShape],
    ground_tile_index: u16,
) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            matches!(
                shape,
                CellShape::Flat
                    | CellShape::RaisedTop {
                        solid: SolidKind::Bank,
                        ..
                    }
            ) && cells[*index].source.tile_index == ground_tile_index
        })
        // Coordinate order, not DTO order, makes the authored source stable.
        .min_by_key(|(index, _)| (cells[*index].row, cells[*index].column))
        .map(|(index, _)| index)
}

fn authored_water_cell(cells: &[&VisualTile], shapes: &[CellShape]) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            cells[*index].source.tile_index == 0x14
                && (matches!(shape, CellShape::Water)
                    || (matches!(shape, CellShape::Flat)
                        && matches!(
                            cells[*index].source.tileset_id.as_ref(),
                            "cave" | "dark_cave"
                        )))
        })
        .min_by_key(|(index, _)| (cells[*index].row, cells[*index].column))
        .map(|(index, _)| index)
}

fn append_park_fountain(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    placement: FountainPlacement,
) {
    const SEGMENTS: usize = 24;
    const RADIUS_X_TILES: f32 = 0.74;
    const RADIUS_Z_TILES: f32 = 0.50;
    const BASIN_HEIGHT_PIXELS: f32 = 5.0;

    let (origin_x, _, origin_z, _) = geometry.bounds(placement.column, placement.row);
    // `$80/$90` are animated fountain cells at the metatile's east edge;
    // the authored oval is centered on that boundary, not at (2, 2).
    let center_x = origin_x + geometry.tile_width * 4.0;
    let center_z = origin_z + geometry.tile_height * 2.15;
    let radius_x = geometry.tile_width * RADIUS_X_TILES;
    let radius_z = geometry.tile_height * RADIUS_Z_TILES;
    let base_height = crate::profile::WATER_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let top_height = base_height + BASIN_HEIGHT_PIXELS * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let uv_at = |x: f32, z: f32| {
        [
            (x - geometry.origin_x) / (geometry.width as f32 * geometry.tile_width),
            (z - geometry.origin_z) / (geometry.height as f32 * geometry.tile_height),
        ]
    };

    let mut top = Vec::with_capacity(SEGMENTS + 2);
    top.push(([center_x, top_height, center_z], uv_at(center_x, center_z)));
    for segment in 0..=SEGMENTS {
        // Clockwise in X/Z yields an upward-facing triangle fan.
        let angle = -(segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU);
        let x = center_x + angle.cos() * radius_x;
        let z = center_z + angle.sin() * radius_z;
        top.push(([x, top_height, z], uv_at(x, z)));
    }
    append_polygon(&mut mesh.textured, &top, [0.0, 1.0, 0.0], TEXTURED_SHADE);

    for segment in 0..SEGMENTS {
        let angle0 = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let angle1 = (segment + 1) as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let x0 = center_x + angle0.cos() * radius_x;
        let z0 = center_z + angle0.sin() * radius_z;
        let x1 = center_x + angle1.cos() * radius_x;
        let z1 = center_z + angle1.sin() * radius_z;
        let normal = Vec3::new(
            (angle0.cos() + angle1.cos()) * 0.5,
            0.0,
            (angle0.sin() + angle1.sin()) * 0.5,
        )
        .normalize_or_zero()
        .to_array();
        append_solid_quad(
            &mut mesh.solid,
            [
                [x0, base_height, z0],
                [x0, top_height, z0],
                [x1, top_height, z1],
                [x1, base_height, z1],
            ],
            normal,
            [0.25, 0.35, 0.72, 1.0],
        );
    }
}

fn append_waterfall(
    mesh: &mut SurfaceMeshData,
    geometry: &GridGeometry,
    cells: &[&VisualTile],
    placement: WaterfallPlacement,
) {
    let plane_row = placement.row + placement.height;
    let plane_z = geometry.origin_z + plane_row as f32 * geometry.tile_height;
    let base = crate::profile::GROUND_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;
    for source_row in 0..placement.height {
        let band_top = base + (placement.height - source_row) as f32 * geometry.tile_height;
        let band_bottom = band_top - geometry.tile_height;
        for offset_column in 0..placement.width {
            let column = placement.column + offset_column;
            let row = placement.row + source_row;
            let index = row * geometry.width + column;
            let (x0, x1, _, _) = geometry.bounds(column, row);
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            append_quad(
                mesh,
                [
                    [x1, band_bottom, plane_z],
                    [x1, band_top, plane_z],
                    [x0, band_top, plane_z],
                    [x0, band_bottom, plane_z],
                ],
                [0.0, 0.0, 1.0],
                [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                TEXTURED_SHADE,
            );
            debug_assert_eq!(cells[index].source.tile_index, 0x40);
        }
    }
}

fn authored_relief_base_cell(
    cells: &[&VisualTile],
    shapes: &[CellShape],
    base_tile_index: u16,
) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            matches!(
                shape,
                CellShape::Flat
                    | CellShape::Water
                    | CellShape::RaisedTop {
                        solid: SolidKind::Bank,
                        ..
                    }
            ) && cells[*index].source.tile_index == base_tile_index
        })
        .min_by_key(|(index, _)| (cells[*index].row, cells[*index].column))
        .map(|(index, _)| index)
}

fn authored_surface_cell(
    cells: &[&VisualTile],
    shapes: &[CellShape],
    tile_index: u16,
    height: f32,
    tile_height: f32,
) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            cells[*index].source.tile_index == tile_index
                && (shape.surface_height(tile_height) - height).abs() < f32::EPSILON
        })
        .min_by_key(|(index, _)| (cells[*index].row, cells[*index].column))
        .map(|(index, _)| index)
}

fn append_masked_upright_hull(
    textured: &mut SurfaceMeshData,
    solid_mesh: &mut SurfaceMeshData,
    removable_ground: &[bool; 64],
    bounds: [f32; 5],
    uv: [f32; 4],
    solid: SolidKind,
) -> Result<(), TerrainMeshError> {
    let [x0, x1, band_bottom, band_top, plane_z] = bounds;
    let [u0, u1, v0, v1] = uv;

    for pixel_y in 0..SOURCE_TILE_PIXELS {
        let mut pixel_x = 0;
        while pixel_x < SOURCE_TILE_PIXELS {
            while pixel_x < SOURCE_TILE_PIXELS
                && removable_ground[pixel_y * SOURCE_TILE_PIXELS + pixel_x]
            {
                pixel_x += 1;
            }
            let run_start = pixel_x;
            while pixel_x < SOURCE_TILE_PIXELS
                && !removable_ground[pixel_y * SOURCE_TILE_PIXELS + pixel_x]
            {
                pixel_x += 1;
            }
            if run_start == pixel_x {
                continue;
            }

            let x_start = lerp_pixel(x0, x1, run_start);
            let x_end = lerp_pixel(x0, x1, pixel_x);
            let y_top = lerp_pixel(band_top, band_bottom, pixel_y);
            let y_bottom = lerp_pixel(band_top, band_bottom, pixel_y + 1);
            let run_u0 = lerp_pixel(u0, u1, run_start);
            let run_u1 = lerp_pixel(u0, u1, pixel_x);
            let run_v0 = lerp_pixel(v0, v1, pixel_y);
            let run_v1 = lerp_pixel(v0, v1, pixel_y + 1);
            append_quad(
                textured,
                [
                    [x_end, y_bottom, plane_z],
                    [x_end, y_top, plane_z],
                    [x_start, y_top, plane_z],
                    [x_start, y_bottom, plane_z],
                ],
                [0.0, 0.0, 1.0],
                [
                    [run_u1, run_v1],
                    [run_u1, run_v0],
                    [run_u0, run_v0],
                    [run_u0, run_v1],
                ],
                TEXTURED_SHADE,
            );
            if solid == SolidKind::Tree {
                continue;
            }
            let depth = upright_depth(solid);
            append_quad(
                textured,
                [
                    [x_start, y_bottom, plane_z - depth],
                    [x_start, y_top, plane_z - depth],
                    [x_end, y_top, plane_z - depth],
                    [x_end, y_bottom, plane_z - depth],
                ],
                [0.0, 0.0, -1.0],
                [
                    [run_u0, run_v1],
                    [run_u0, run_v0],
                    [run_u1, run_v0],
                    [run_u1, run_v1],
                ],
                [0.72, 0.72, 0.72, 1.0],
            );
        }
    }

    if solid == SolidKind::Tree {
        return Ok(());
    }

    let depth = upright_depth(solid);
    let back_z = plane_z - depth;
    for pixel_y in 0..SOURCE_TILE_PIXELS {
        for pixel_x in 0..SOURCE_TILE_PIXELS {
            let index = pixel_y * SOURCE_TILE_PIXELS + pixel_x;
            if removable_ground[index] {
                continue;
            }
            let x_start = lerp_pixel(x0, x1, pixel_x);
            let x_end = lerp_pixel(x0, x1, pixel_x + 1);
            let y_top = lerp_pixel(band_top, band_bottom, pixel_y);
            let y_bottom = lerp_pixel(band_top, band_bottom, pixel_y + 1);
            let open = |x: isize, y: isize| {
                x < 0
                    || y < 0
                    || x >= SOURCE_TILE_PIXELS as isize
                    || y >= SOURCE_TILE_PIXELS as isize
                    || removable_ground[y as usize * SOURCE_TILE_PIXELS + x as usize]
            };
            if open(pixel_x as isize - 1, pixel_y as isize) {
                append_solid_quad(
                    solid_mesh,
                    [
                        [x_start, y_bottom, plane_z],
                        [x_start, y_top, plane_z],
                        [x_start, y_top, back_z],
                        [x_start, y_bottom, back_z],
                    ],
                    [-1.0, 0.0, 0.0],
                    solid_color(solid, Direction::West),
                );
            }
            if open(pixel_x as isize + 1, pixel_y as isize) {
                append_solid_quad(
                    solid_mesh,
                    [
                        [x_end, y_bottom, back_z],
                        [x_end, y_top, back_z],
                        [x_end, y_top, plane_z],
                        [x_end, y_bottom, plane_z],
                    ],
                    [1.0, 0.0, 0.0],
                    solid_color(solid, Direction::East),
                );
            }
            if open(pixel_x as isize, pixel_y as isize - 1) {
                append_solid_quad(
                    solid_mesh,
                    [
                        [x_start, y_top, back_z],
                        [x_end, y_top, back_z],
                        [x_end, y_top, plane_z],
                        [x_start, y_top, plane_z],
                    ],
                    [0.0, 1.0, 0.0],
                    solid_color(solid, Direction::South),
                );
            }
            if open(pixel_x as isize, pixel_y as isize + 1) {
                append_solid_quad(
                    solid_mesh,
                    [
                        [x_start, y_bottom, plane_z],
                        [x_end, y_bottom, plane_z],
                        [x_end, y_bottom, back_z],
                        [x_start, y_bottom, back_z],
                    ],
                    [0.0, -1.0, 0.0],
                    solid_color(solid, Direction::North),
                );
            }
        }
    }
    Ok(())
}

fn upright_depth(solid: SolidKind) -> f32 {
    match solid {
        SolidKind::Tree => 0.0,
        SolidKind::Rock => 8.0,
        SolidKind::Ship => 1.0,
        SolidKind::FlatCard => 0.0,
        SolidKind::Prop => 2.0,
        SolidKind::CutTree => 5.0,
        SolidKind::Flower => 1.0,
        SolidKind::Grass => 2.0,
        SolidKind::Fence => 6.0,
        _ => 1.0,
    }
}

/// Ground decoration stands on the far/north edge of its source cell so an
/// actor whose feet occupy that cell is always closer to the camera. Other
/// cutouts remain centered because their depth participates in real world
/// occlusion (trees, rocks, signs, and cuttable shrubs).
fn upright_plane_z(solid: SolidKind, north_z: f32, south_z: f32) -> f32 {
    if solid == SolidKind::Flower {
        north_z
    } else {
        (north_z + south_z) * 0.5
    }
}

fn decorative_cutout_mask(
    images: &TerrainImageSamples,
    tile: &VisualTile,
    ground_tile: &VisualTile,
    solid: SolidKind,
) -> Result<[bool; 64], TerrainMeshError> {
    let rgba = tile_rgba(images, tile)?;
    let mut removable = [false; 64];
    match solid {
        SolidKind::Grass => {
            let _ = ground_tile;
            // The grass slot is a complete tuft drawing, not the plain-ground
            // tile with a few changed pixels. Keep the darker two Game Boy
            // paint levels as blades and discard the lighter background.
            // This mirrors the reference grass template's tone threshold and
            // avoids erecting an opaque 8x8 lawn card.
            let dark = darker_palette_mask(rgba, 2);
            for index in 0..64 {
                removable[index] = !dark[index];
            }
        }
        SolidKind::Flower => {
            let _ = ground_tile;
            // Flowers keep their dark outline plus every lighter pixel that
            // outline encloses. The source slot animates, so texture alpha
            // still changes while this stable silhouette remains a thin slab.
            let dark = darker_palette_mask(rgba, 2);
            let traversable: Vec<_> = dark.iter().map(|dark| !dark).collect();
            let outside = boundary_connected_mask(8, 8, &traversable);
            for index in 0..64 {
                removable[index] = outside[index];
            }
        }
        _ => unreachable!("only decorative cutouts request a decorative mask"),
    }
    Ok(removable)
}

fn darker_palette_mask(rgba: &[u8], shade_count: usize) -> Vec<bool> {
    let luminance: Vec<_> = rgba
        .chunks_exact(4)
        .map(|pixel| u16::from(pixel[0]) * 3 + u16::from(pixel[1]) * 6 + u16::from(pixel[2]))
        .collect();
    let mut shades = luminance.clone();
    shades.sort_unstable();
    shades.dedup();
    let darkest_half_end = (shades.len() - 1) / 2;
    let threshold = shades[shade_count.saturating_sub(1).min(darkest_half_end)];
    luminance
        .into_iter()
        .map(|value| value <= threshold)
        .collect()
}

fn append_pixel_relief(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    tile: &VisualTile,
    ground_tile: &VisualTile,
    bounds: [f32; 4],
    uv: (f32, f32, f32, f32),
    base_height: f32,
    height: f32,
) -> Result<(), TerrainMeshError> {
    let rgba = tile_rgba(images, tile)?;
    let ground = tile_rgba(images, ground_tile)?;
    let equals_ground: Vec<_> = rgba
        .chunks_exact(4)
        .zip(ground.chunks_exact(4))
        .map(|(source, base)| source == base)
        .collect();
    let outside = boundary_connected_mask(8, 8, &equals_ground);
    let on = |x: isize, y: isize| {
        x >= 0 && y >= 0 && x < 8 && y < 8 && !outside[y as usize * 8 + x as usize]
    };
    let [x0, x1, z0, z1] = bounds;
    let (u0, u1, v0, v1) = uv;
    for py in 0_usize..8 {
        for px in 0_usize..8 {
            if !on(px as isize, py as isize) {
                continue;
            }
            let px0 = lerp_pixel(x0, x1, px as usize);
            let px1 = lerp_pixel(x0, x1, px as usize + 1);
            let pz0 = lerp_pixel(z0, z1, py as usize);
            let pz1 = lerp_pixel(z0, z1, py as usize + 1);
            let pu0 = lerp_pixel(u0, u1, px as usize);
            let pu1 = lerp_pixel(u0, u1, px as usize + 1);
            let pv0 = lerp_pixel(v0, v1, py as usize);
            let pv1 = lerp_pixel(v0, v1, py as usize + 1);
            append_quad(
                &mut mesh.textured,
                [
                    [px0, base_height + height, pz0],
                    [px0, base_height + height, pz1],
                    [px1, base_height + height, pz1],
                    [px1, base_height + height, pz0],
                ],
                [0.0, 1.0, 0.0],
                [[pu0, pv0], [pu0, pv1], [pu1, pv1], [pu1, pv0]],
                TEXTURED_SHADE,
            );
            for (nx, ny, direction) in [
                (px as isize - 1, py as isize, Direction::West),
                (px as isize + 1, py as isize, Direction::East),
                (px as isize, py as isize - 1, Direction::North),
                (px as isize, py as isize + 1, Direction::South),
            ] {
                if on(nx, ny) {
                    continue;
                }
                let positions = match direction {
                    Direction::West => [
                        [px0, base_height, pz0],
                        [px0, base_height + height, pz0],
                        [px0, base_height + height, pz1],
                        [px0, base_height, pz1],
                    ],
                    Direction::East => [
                        [px1, base_height, pz1],
                        [px1, base_height + height, pz1],
                        [px1, base_height + height, pz0],
                        [px1, base_height, pz0],
                    ],
                    Direction::North => [
                        [px1, base_height, pz0],
                        [px1, base_height + height, pz0],
                        [px0, base_height + height, pz0],
                        [px0, base_height, pz0],
                    ],
                    Direction::South => [
                        [px0, base_height, pz1],
                        [px0, base_height + height, pz1],
                        [px1, base_height + height, pz1],
                        [px1, base_height, pz1],
                    ],
                };
                append_solid_quad(
                    &mut mesh.solid,
                    positions,
                    match direction {
                        Direction::West => [-1.0, 0.0, 0.0],
                        Direction::East => [1.0, 0.0, 0.0],
                        Direction::North => [0.0, 0.0, -1.0],
                        Direction::South => [0.0, 0.0, 1.0],
                    },
                    solid_color(SolidKind::Prop, direction),
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn grouped_boundary_ground_mask(
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    column: usize,
    row: usize,
    index: usize,
    ground_tile: &VisualTile,
) -> Result<[bool; 64], TerrainMeshError> {
    let CellShape::FacadeBand {
        plane_subtile_row,
        band_from_top,
        band_count,
        ground_tile_index,
        solid,
    } = shapes[index]
    else {
        return Ok([false; 64]);
    };
    let target_plane =
        row as isize - cells[index].source.subtile_row as isize + plane_subtile_row as isize;
    let mut bands_by_column = vec![vec![None; usize::from(band_count)]; geometry.width];
    for (candidate_index, shape) in shapes.iter().copied().enumerate() {
        let CellShape::FacadeBand {
            plane_subtile_row: candidate_plane_row,
            band_from_top: candidate_band,
            band_count: candidate_count,
            ground_tile_index: candidate_ground,
            solid: candidate_solid,
        } = shape
        else {
            continue;
        };
        if candidate_count != band_count
            || candidate_ground != ground_tile_index
            || candidate_solid != solid
        {
            continue;
        }
        let candidate_row = candidate_index / geometry.width;
        let candidate_column = candidate_index % geometry.width;
        let candidate_plane = candidate_row as isize
            - cells[candidate_index].source.subtile_row as isize
            + candidate_plane_row as isize;
        if candidate_plane == target_plane && candidate_band < band_count {
            bands_by_column[candidate_column][usize::from(candidate_band)] = Some(candidate_index);
        }
    }
    let complete = |candidate_column: usize| {
        bands_by_column[candidate_column]
            .iter()
            .all(Option::is_some)
    };
    // A group clipped by the viewport has no trustworthy outer silhouette.
    // Keep the visible fragment opaque instead of treating its clipped edge
    // as transparent background (or disabling the optional renderer).
    if !complete(column) {
        return Ok([false; 64]);
    }
    let mut first_column = column;
    while first_column > 0 && complete(first_column - 1) {
        first_column -= 1;
    }
    let mut last_column = column;
    while last_column + 1 < geometry.width && complete(last_column + 1) {
        last_column += 1;
    }

    let group_columns = last_column - first_column + 1;
    let pixel_width = group_columns * SOURCE_TILE_PIXELS;
    let pixel_height = usize::from(band_count) * SOURCE_TILE_PIXELS;
    let ground = tile_rgba(images, ground_tile)?;
    let mut equals_ground = vec![false; pixel_width * pixel_height];
    let mut group_rgba = vec![0_u8; pixel_width * pixel_height * 4];
    for group_column in first_column..=last_column {
        for band in 0..usize::from(band_count) {
            let source_index = bands_by_column[group_column][band]
                .expect("complete facade group column was checked");
            let source = tile_rgba(images, cells[source_index])?;
            for pixel_y in 0..SOURCE_TILE_PIXELS {
                for pixel_x in 0..SOURCE_TILE_PIXELS {
                    let group_x = (group_column - first_column) * SOURCE_TILE_PIXELS + pixel_x;
                    let group_y = band * SOURCE_TILE_PIXELS + pixel_y;
                    equals_ground[group_y * pixel_width + group_x] =
                        pixels_equal(source, ground, pixel_x, pixel_y);
                    let source_offset = (pixel_y * SOURCE_TILE_PIXELS + pixel_x) * 4;
                    let group_offset = (group_y * pixel_width + group_x) * 4;
                    group_rgba[group_offset..group_offset + 4]
                        .copy_from_slice(&source[source_offset..source_offset + 4]);
                }
            }
        }
    }
    let removable_group = if solid == SolidKind::Fence {
        // Fence art deliberately uses its middle tones as structure.
        // Preserve the three darker Game Boy levels and flood only the
        // brightest boundary-connected paint, matching the reference
        // renderer's opaque post path instead of reducing rails to their
        // darkest outline.
        let dark = darker_palette_mask(&group_rgba, 3);
        let traversable: Vec<_> = dark.iter().map(|dark| !dark).collect();
        boundary_connected_mask(pixel_width, pixel_height, &traversable)
    } else if solid == SolidKind::Prop && cells[index].source.tileset_id.as_ref() == "kanto" {
        // Kanto signs and vertical fence posts are black-outline drawings.
        // Their surrounding paint is not always byte-identical to one ground
        // tile after palette assignment. Preserve the darker outline plus
        // enclosed face pixels, and flood the remaining background away.
        let dark = darker_palette_mask(&group_rgba, 2);
        let traversable: Vec<_> = dark.iter().map(|dark| !dark).collect();
        boundary_connected_mask(pixel_width, pixel_height, &traversable)
    } else {
        boundary_connected_mask(pixel_width, pixel_height, &equals_ground)
    };
    let mut removable_cell = [false; 64];
    let cell_x = (column - first_column) * SOURCE_TILE_PIXELS;
    let cell_y = usize::from(band_from_top) * SOURCE_TILE_PIXELS;
    for pixel_y in 0..SOURCE_TILE_PIXELS {
        for pixel_x in 0..SOURCE_TILE_PIXELS {
            removable_cell[pixel_y * SOURCE_TILE_PIXELS + pixel_x] =
                removable_group[(cell_y + pixel_y) * pixel_width + cell_x + pixel_x];
        }
    }
    Ok(removable_cell)
}

fn boundary_connected_mask(width: usize, height: usize, equals_ground: &[bool]) -> Vec<bool> {
    debug_assert_eq!(equals_ground.len(), width * height);
    let mut removable = vec![false; equals_ground.len()];
    let mut pending = VecDeque::new();
    for y in 0..height {
        for x in 0..width {
            if x != 0 && y != 0 && x + 1 != width && y + 1 != height {
                continue;
            }
            let index = y * width + x;
            if equals_ground[index] {
                if !removable[index] {
                    removable[index] = true;
                    pending.push_back((x, y));
                }
            }
        }
    }
    while let Some((x, y)) = pending.pop_front() {
        for (next_x, next_y) in [
            x.checked_sub(1).map(|next| (next, y)),
            (x + 1 < width).then_some((x + 1, y)),
            y.checked_sub(1).map(|next| (x, next)),
            (y + 1 < height).then_some((x, y + 1)),
        ]
        .into_iter()
        .flatten()
        {
            let index = next_y * width + next_x;
            if !removable[index] && equals_ground[index] {
                removable[index] = true;
                pending.push_back((next_x, next_y));
            }
        }
    }
    removable
}

fn tile_rgba<'a>(
    images: &'a TerrainImageSamples,
    tile: &VisualTile,
) -> Result<&'a [u8], TerrainMeshError> {
    let sample =
        images
            .pixels
            .get(&tile.texture.id())
            .ok_or(TerrainMeshError::MissingMaskImage {
                column: tile.column,
                row: tile.row,
            })?;
    match sample {
        TileImageSample::Rgba(pixels) => Ok(pixels),
        TileImageSample::Invalid => Err(TerrainMeshError::InvalidMaskImage {
            column: tile.column,
            row: tile.row,
        }),
    }
}

fn pixels_equal(source: &[u8], ground: &[u8], x: usize, y: usize) -> bool {
    let offset = (y * SOURCE_TILE_PIXELS + x) * 4;
    source[offset..offset + 4] == ground[offset..offset + 4]
}

fn lerp_pixel(start: f32, end: f32, pixel: usize) -> f32 {
    start + (end - start) * pixel as f32 / SOURCE_TILE_PIXELS as f32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    North,
    South,
    West,
    East,
}

impl Direction {
    const ALL: [Self; 4] = [Self::North, Self::South, Self::West, Self::East];

    fn offset(self) -> (isize, isize) {
        match self {
            Self::North => (0, -1),
            Self::South => (0, 1),
            Self::West => (-1, 0),
            Self::East => (1, 0),
        }
    }

    fn lateral_offsets(self) -> ((isize, isize), (isize, isize)) {
        match self {
            Self::East => ((0, 1), (0, -1)),
            Self::West => ((0, -1), (0, 1)),
            Self::South => ((-1, 0), (1, 0)),
            Self::North => ((1, 0), (-1, 0)),
        }
    }
}

fn append_ramp_sidewalls(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    shapes: &[CellShape],
    column: usize,
    row: usize,
    north_height: f32,
    south_height: f32,
) {
    let (x0, x1, z0, z1) = geometry.bounds(column, row);
    for (direction, neighbor_column) in [
        (Direction::West, column.checked_sub(1)),
        (
            Direction::East,
            (column + 1 < geometry.width).then_some(column + 1),
        ),
    ] {
        let Some(neighbor_column) = neighbor_column else {
            continue;
        };
        let neighbor =
            shapes[row * geometry.width + neighbor_column].surface_height(geometry.tile_height);
        if neighbor <= north_height.min(south_height) {
            continue;
        }
        let color = solid_color(SolidKind::Bank, direction);
        match direction {
            Direction::West => append_solid_quad(
                &mut mesh.solid,
                [
                    [x0, south_height, z1],
                    [x0, north_height, z0],
                    [x0, neighbor, z0],
                    [x0, neighbor, z1],
                ],
                [-1.0, 0.0, 0.0],
                color,
            ),
            Direction::East => append_solid_quad(
                &mut mesh.solid,
                [
                    [x1, south_height, z1],
                    [x1, neighbor, z1],
                    [x1, neighbor, z0],
                    [x1, north_height, z0],
                ],
                [1.0, 0.0, 0.0],
                color,
            ),
            Direction::North | Direction::South => unreachable!(),
        }
    }
}

fn append_exposed_sides(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    bank_runs: &[Option<BankColumnRun>],
    column: usize,
    row: usize,
) {
    let index = row * geometry.width + column;
    if let CellShape::RampNorth {
        north_height,
        south_height,
    } = shapes[index]
    {
        append_ramp_sidewalls(
            mesh,
            geometry,
            shapes,
            column,
            row,
            north_height * geometry.tile_height / SOURCE_TILE_HEIGHT,
            south_height * geometry.tile_height / SOURCE_TILE_HEIGHT,
        );
        return;
    }
    let top = shapes[index].surface_height(geometry.tile_height);
    let (x0, x1, z0, z1) = geometry.bounds(column, row);

    for direction in Direction::ALL {
        let (dx, dz) = direction.offset();
        let neighbor_column = column as isize + dx;
        let neighbor_row = row as isize + dz;
        // A viewport boundary is unknown continuation, not a diorama cliff.
        if neighbor_column < 0
            || neighbor_row < 0
            || neighbor_column >= geometry.width as isize
            || neighbor_row >= geometry.height as isize
        {
            continue;
        }
        let neighbor_column = neighbor_column as usize;
        let neighbor_row = neighbor_row as usize;
        let neighbor_index = neighbor_row * geometry.width + neighbor_column;
        let bottom = shapes[neighbor_index].surface_height(geometry.tile_height);
        if bottom >= top {
            continue;
        }
        if matches!(direction, Direction::South)
            && facade_covers_south_edge(
                cells[neighbor_index],
                shapes[neighbor_index],
                neighbor_row,
                top,
                geometry.tile_height,
            )
        {
            continue;
        }
        if top == 0.0 && matches!(shapes[neighbor_index], CellShape::Water) {
            append_textured_shoreline(
                &mut mesh.textured,
                geometry,
                column,
                row,
                direction,
                [x0, x1, z0, z1],
                bottom,
                top,
            );
            continue;
        }

        if shapes[index].solid_kind() == SolidKind::Ship {
            append_textured_ship_side(
                &mut mesh.textured,
                geometry,
                column,
                row,
                direction,
                [x0, x1, z0, z1],
                bottom,
                top,
            );
            continue;
        }

        // Game Corner $0b is the authored dark south end of a 16px machine
        // bank. Its final two native tile rows are the two cabinet courses;
        // map each once instead of covering the bank with a blank prop cap.
        if direction == Direction::South
            && cells[index].source.tileset_id.as_ref() == "game_corner"
            && cells[index].source.metatile_id == 0x0b
            && cells[index].source.subtile_row == 3
            && row > 0
        {
            let middle = bottom + (top - bottom) * 0.5;
            for (source_index, y0, y1) in [
                (index - geometry.width, middle, top),
                (index, bottom, middle),
            ] {
                let (u0, u1, v0, v1) =
                    geometry.uv(source_index % geometry.width, source_index / geometry.width);
                append_quad(
                    &mut mesh.textured,
                    [[x1, y0, z1], [x1, y1, z1], [x0, y1, z1], [x0, y0, z1]],
                    [0.0, 0.0, 1.0],
                    [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                    TEXTURED_SHADE,
                );
            }
            continue;
        }

        if let Some(run) = bank_runs[index] {
            let height_at = |offset: (isize, isize)| {
                let lateral_column = column as isize + offset.0;
                let lateral_row = row as isize + offset.1;
                if lateral_column < 0
                    || lateral_row < 0
                    || lateral_column >= geometry.width as isize
                    || lateral_row >= geometry.height as isize
                {
                    0.0
                } else {
                    shapes[lateral_row as usize * geometry.width + lateral_column as usize]
                        .surface_height(geometry.tile_height)
                }
            };
            let (left, right) = direction.lateral_offsets();
            append_bank_run_side(
                &mut mesh.textured,
                &mut mesh.solid,
                geometry,
                cells,
                shapes,
                run,
                column,
                direction,
                [x0, x1, z0, z1],
                bottom,
                top,
                [height_at(left), height_at(right)],
                bank_taper(&cells[index].source),
                cells[index].source.subtile_column,
                cells[index].source.subtile_row,
            );
            continue;
        }

        let solid = if matches!(shapes[index], CellShape::Water)
            || matches!(shapes[neighbor_index], CellShape::Water)
        {
            SolidKind::Bank
        } else {
            shapes[index].solid_kind()
        };
        let color = solid_color(solid, direction);
        match direction {
            Direction::North => append_solid_quad(
                &mut mesh.solid,
                [
                    [x0, bottom, z0],
                    [x0, top, z0],
                    [x1, top, z0],
                    [x1, bottom, z0],
                ],
                [0.0, 0.0, -1.0],
                color,
            ),
            Direction::South => append_solid_quad(
                &mut mesh.solid,
                [
                    [x1, bottom, z1],
                    [x1, top, z1],
                    [x0, top, z1],
                    [x0, bottom, z1],
                ],
                [0.0, 0.0, 1.0],
                color,
            ),
            Direction::West => append_solid_quad(
                &mut mesh.solid,
                [
                    [x0, bottom, z1],
                    [x0, top, z1],
                    [x0, top, z0],
                    [x0, bottom, z0],
                ],
                [-1.0, 0.0, 0.0],
                color,
            ),
            Direction::East => append_solid_quad(
                &mut mesh.solid,
                [
                    [x1, bottom, z0],
                    [x1, top, z0],
                    [x1, top, z1],
                    [x1, bottom, z1],
                ],
                [1.0, 0.0, 0.0],
                color,
            ),
        }
    }
}

fn bank_taper(source: &VisualTileSource) -> Option<[f32; 2]> {
    if matches!(source.tileset_id.as_ref(), "johto" | "johto_modern") && source.metatile_id == 0x0a
    {
        return Some([10.0, 4.0]);
    }
    if source.tileset_id.as_ref() == "kanto"
        && matches!(
            source.metatile_id,
            0x3e | 0x3f | 0x3b | 0x24 | 0x06 | 0x57 | 0x25
        )
    {
        return Some([16.0, 8.0]);
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn append_textured_ship_side(
    mesh: &mut SurfaceMeshData,
    geometry: &GridGeometry,
    column: usize,
    row: usize,
    direction: Direction,
    bounds: [f32; 4],
    bottom: f32,
    top: f32,
) {
    let [x0, x1, z0, z1] = bounds;
    let (u0, u1, v0, v1) = geometry.uv(column, row);
    let first_band = (bottom / geometry.tile_height).floor() as i32;
    let last_band = (top / geometry.tile_height).ceil() as i32;
    for band in first_band..last_band {
        let band_floor = band as f32 * geometry.tile_height;
        let band_ceiling = band_floor + geometry.tile_height;
        let band_bottom = bottom.max(band_floor);
        let band_top = top.min(band_ceiling);
        if band_top <= band_bottom {
            continue;
        }
        let crop_top = (band_ceiling - band_top) / geometry.tile_height;
        let crop_bottom = (band_ceiling - band_bottom) / geometry.tile_height;
        let cv0 = v0 + (v1 - v0) * crop_top;
        let cv1 = v0 + (v1 - v0) * crop_bottom;
        let shade = match direction {
            Direction::North => 0.68,
            Direction::South => 1.0,
            Direction::West => 0.76,
            Direction::East => 0.86,
        };
        let color = [shade, shade, shade, 1.0];
        let (positions, normal, uvs) = match direction {
            Direction::North => (
                [
                    [x0, band_bottom, z0],
                    [x0, band_top, z0],
                    [x1, band_top, z0],
                    [x1, band_bottom, z0],
                ],
                [0.0, 0.0, -1.0],
                [[u0, cv1], [u0, cv0], [u1, cv0], [u1, cv1]],
            ),
            Direction::South => (
                [
                    [x1, band_bottom, z1],
                    [x1, band_top, z1],
                    [x0, band_top, z1],
                    [x0, band_bottom, z1],
                ],
                [0.0, 0.0, 1.0],
                [[u1, cv1], [u1, cv0], [u0, cv0], [u0, cv1]],
            ),
            Direction::West => (
                [
                    [x0, band_bottom, z1],
                    [x0, band_top, z1],
                    [x0, band_top, z0],
                    [x0, band_bottom, z0],
                ],
                [-1.0, 0.0, 0.0],
                [[u1, cv1], [u1, cv0], [u0, cv0], [u0, cv1]],
            ),
            Direction::East => (
                [
                    [x1, band_bottom, z0],
                    [x1, band_top, z0],
                    [x1, band_top, z1],
                    [x1, band_bottom, z1],
                ],
                [1.0, 0.0, 0.0],
                [[u0, cv1], [u0, cv0], [u1, cv0], [u1, cv1]],
            ),
        };
        append_quad(mesh, positions, normal, uvs, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn append_bank_run_side(
    textured_mesh: &mut SurfaceMeshData,
    solid_mesh: &mut SurfaceMeshData,
    geometry: &GridGeometry,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    run: BankColumnRun,
    column: usize,
    direction: Direction,
    bounds: [f32; 4],
    bottom: f32,
    top: f32,
    lateral_heights: [f32; 2],
    taper: Option<[f32; 2]>,
    subtile_column: u8,
    subtile_row: u8,
) {
    let [x0, x1, z0, z1] = bounds;
    let height_span = (top - bottom).max(f32::EPSILON);
    // Authored rock platforms and the Kanto cave mound have narrow caps over
    // outward-spreading feet. Their exact source bands remain native-sized;
    // only the geometric side plane tilts to form the drawn trapezoid.
    let [bevel, corner_clip] = taper.unwrap_or([0.0, 0.0]);
    let shade = match direction {
        Direction::South => 0.90,
        Direction::West => 0.72,
        Direction::East => 0.84,
        Direction::North => 0.68,
    };
    let first_band = (bottom / geometry.tile_height).floor().max(0.0) as usize;
    let last_band = (top / geometry.tile_height).ceil().max(0.0) as usize;
    for band in first_band..last_band {
        let local_band = band - first_band;
        let band_floor = band as f32 * geometry.tile_height;
        let band_ceiling = band_floor + geometry.tile_height;
        let band_bottom = bottom.max(band_floor);
        let band_top = top.min(band_ceiling);
        let bottom_inset = -bevel * (1.0 - (band_bottom - bottom) / height_span);
        let top_inset = -bevel * (1.0 - (band_top - bottom) / height_span);
        let left_open = lateral_heights[0] < top;
        let right_open = lateral_heights[1] < top;
        let fallback_row = match direction {
            Direction::North => (run.north + local_band).min(run.front),
            Direction::South | Direction::West | Direction::East => {
                run.front.saturating_sub(local_band).max(run.north)
            }
        };
        let source =
            authored_bank_face_cell(cells, shapes, geometry, run, column, direction, local_band)
                .unwrap_or(fallback_row * geometry.width + column);
        let (u0, u1, v0, v1) = geometry.uv(source % geometry.width, source / geometry.width);
        let crop_top = (band_ceiling - band_top) / geometry.tile_height;
        let crop_bottom = (band_ceiling - band_bottom) / geometry.tile_height;
        let cropped_v0 = v0 + (v1 - v0) * crop_top;
        let cropped_v1 = v0 + (v1 - v0) * crop_bottom;
        let ao = side_ao_shades(
            lateral_heights[0],
            lateral_heights[1],
            band_bottom,
            band_top,
            band_bottom <= bottom + f32::EPSILON,
            shade,
        );
        let colors = [
            [ao[1], ao[1], ao[1], 1.0],
            [ao[2], ao[2], ao[2], 1.0],
            [ao[3], ao[3], ao[3], 1.0],
            [ao[0], ao[0], ao[0], 1.0],
        ];
        let (positions, normal, uvs) = match direction {
            Direction::South => (
                [
                    [
                        x1 - right_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                        band_bottom,
                        z1 - bottom_inset,
                    ],
                    [
                        x1 - right_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                        band_top,
                        z1 - top_inset,
                    ],
                    [
                        x0 + left_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                        band_top,
                        z1 - top_inset,
                    ],
                    [
                        x0 + left_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                        band_bottom,
                        z1 - bottom_inset,
                    ],
                ],
                Vec3::new(0.0, bevel, height_span).normalize().to_array(),
                [
                    [u1, cropped_v1],
                    [u1, cropped_v0],
                    [u0, cropped_v0],
                    [u0, cropped_v1],
                ],
            ),
            Direction::West => (
                [
                    [
                        x0 + bottom_inset,
                        band_bottom,
                        z1 - right_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                    ],
                    [
                        x0 + top_inset,
                        band_top,
                        z1 - right_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                    ],
                    [
                        x0 + top_inset,
                        band_top,
                        z0 + left_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                    ],
                    [
                        x0 + bottom_inset,
                        band_bottom,
                        z0 + left_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                    ],
                ],
                Vec3::new(-height_span, bevel, 0.0).normalize().to_array(),
                [
                    [u1, cropped_v1],
                    [u1, cropped_v0],
                    [u0, cropped_v0],
                    [u0, cropped_v1],
                ],
            ),
            Direction::East => (
                [
                    [
                        x1 - bottom_inset,
                        band_bottom,
                        z0 + right_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                    ],
                    [
                        x1 - top_inset,
                        band_top,
                        z0 + right_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                    ],
                    [
                        x1 - top_inset,
                        band_top,
                        z1 - left_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                    ],
                    [
                        x1 - bottom_inset,
                        band_bottom,
                        z1 - left_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                    ],
                ],
                Vec3::new(height_span, bevel, 0.0).normalize().to_array(),
                [
                    [u0, cropped_v1],
                    [u0, cropped_v0],
                    [u1, cropped_v0],
                    [u1, cropped_v1],
                ],
            ),
            Direction::North => (
                [
                    [
                        x0 + right_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                        band_bottom,
                        z0 + bottom_inset,
                    ],
                    [
                        x0 + right_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                        band_top,
                        z0 + top_inset,
                    ],
                    [
                        x1 - left_open.then_some(corner_clip + top_inset).unwrap_or(0.0),
                        band_top,
                        z0 + top_inset,
                    ],
                    [
                        x1 - left_open
                            .then_some(corner_clip + bottom_inset)
                            .unwrap_or(0.0),
                        band_bottom,
                        z0 + bottom_inset,
                    ],
                ],
                Vec3::new(0.0, bevel, -height_span).normalize().to_array(),
                [
                    [u0, cropped_v1],
                    [u0, cropped_v0],
                    [u1, cropped_v0],
                    [u1, cropped_v1],
                ],
            ),
        };
        append_quad_colors(textured_mesh, positions, normal, uvs, colors);
        if taper.is_some() && matches!(direction, Direction::North | Direction::South) {
            append_rock_platform_corner_faces(
                solid_mesh,
                direction,
                [x0, x1, z0, z1],
                band_bottom,
                band_top,
                bottom_inset,
                top_inset,
                corner_clip,
                left_open,
                right_open,
                shade,
                subtile_column,
                subtile_row,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_rock_platform_corner_faces(
    mesh: &mut SurfaceMeshData,
    direction: Direction,
    bounds: [f32; 4],
    band_bottom: f32,
    band_top: f32,
    bottom_inset: f32,
    top_inset: f32,
    clip: f32,
    left_open: bool,
    right_open: bool,
    _shade: f32,
    subtile_column: u8,
    subtile_row: u8,
) {
    let [x0, x1, z0, z1] = bounds;
    let color = solid_color(SolidKind::Bank, direction);
    let mut corner = |positions: [[f32; 3]; 4]| {
        let edge_a = Vec3::from_array(positions[1]) - Vec3::from_array(positions[0]);
        let edge_b = Vec3::from_array(positions[2]) - Vec3::from_array(positions[0]);
        let normal = edge_a.cross(edge_b).normalize().to_array();
        append_solid_quad(mesh, positions, normal, color);
    };
    match direction {
        Direction::North => {
            if right_open && subtile_column == 0 && subtile_row == 0 {
                corner([
                    [x0 + bottom_inset, band_bottom, z0 + clip + bottom_inset],
                    [x0 + top_inset, band_top, z0 + clip + top_inset],
                    [x0 + clip + top_inset, band_top, z0 + top_inset],
                    [x0 + clip + bottom_inset, band_bottom, z0 + bottom_inset],
                ]);
            }
            if left_open && subtile_column == 3 && subtile_row == 0 {
                corner([
                    [x1 - clip - bottom_inset, band_bottom, z0 + bottom_inset],
                    [x1 - clip - top_inset, band_top, z0 + top_inset],
                    [x1 - top_inset, band_top, z0 + clip + top_inset],
                    [x1 - bottom_inset, band_bottom, z0 + clip + bottom_inset],
                ]);
            }
        }
        Direction::South => {
            if left_open && subtile_column == 0 && subtile_row == 3 {
                corner([
                    [x0 + clip + bottom_inset, band_bottom, z1 - bottom_inset],
                    [x0 + clip + top_inset, band_top, z1 - top_inset],
                    [x0 + top_inset, band_top, z1 - clip - top_inset],
                    [x0 + bottom_inset, band_bottom, z1 - clip - bottom_inset],
                ]);
            }
            if right_open && subtile_column == 3 && subtile_row == 3 {
                corner([
                    [x1 - bottom_inset, band_bottom, z1 - clip - bottom_inset],
                    [x1 - top_inset, band_top, z1 - clip - top_inset],
                    [x1 - clip - top_inset, band_top, z1 - top_inset],
                    [x1 - clip - bottom_inset, band_bottom, z1 - bottom_inset],
                ]);
            }
        }
        Direction::West | Direction::East => {}
    }
}

/// Selects only the authored upright courses for a bank face. A mountain
/// run also contains horizontal plateau cells; walking backward through the
/// complete run paints those ground tiles onto tall cliff walls. The source
/// drawing is instead folded bottom-to-top and repeated for additional
/// elevation tiers, matching the reference renderer's authored-upright rule.
fn authored_bank_face_cell(
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    run: BankColumnRun,
    column: usize,
    direction: Direction,
    local_band: usize,
) -> Option<usize> {
    let authored_face = match direction {
        Direction::South | Direction::North => LedgeFace::South,
        Direction::West => LedgeFace::West,
        Direction::East => LedgeFace::East,
    };
    let mut courses = Vec::new();
    for row in run.north..=run.front {
        let index = row * geometry.width + column;
        let CellShape::LedgeBand {
            face,
            band_from_top,
            band_count,
            ..
        } = shapes[index]
        else {
            continue;
        };
        if face == authored_face {
            courses.push((band_from_top, band_count, index));
        }
    }
    let band_count = courses.iter().map(|(_, count, _)| *count).max()?;
    let wanted_from_top = band_count - 1 - (local_band as u8 % band_count);
    let repeats_above_authored_course = local_band >= band_count as usize;
    let is_doorway =
        |index: usize| matches!(cells[index].source.tile_index, 0x46 | 0x47 | 0x56 | 0x57);
    if let Some(index) = courses.iter().find_map(|(band, _, index)| {
        (*band == wanted_from_top && (!repeats_above_authored_course || !is_doorway(*index)))
            .then_some(*index)
    }) {
        return Some(index);
    }
    // A doorway may occupy the only matching course in this column. For an
    // additional elevation tier, borrow the same authored rock band from a
    // neighboring column rather than stamping a second entrance.
    if repeats_above_authored_course {
        for row in run.north..=run.front {
            for source_column in 0..geometry.width {
                let index = row * geometry.width + source_column;
                let CellShape::LedgeBand {
                    face,
                    band_from_top,
                    ..
                } = shapes[index]
                else {
                    continue;
                };
                if face == authored_face && band_from_top == wanted_from_top && !is_doorway(index) {
                    return Some(index);
                }
            }
        }
    }
    None
}

fn side_ao_shades(
    left_height: f32,
    right_height: f32,
    bottom: f32,
    top: f32,
    crease: bool,
    shade: f32,
) -> [f32; 4] {
    const AO_EDGE: f32 = 0.664;
    const AO_FLOOR: f32 = 0.25;
    let corner = (AO_EDGE * AO_EDGE).max(AO_FLOOR);
    let base = if crease { AO_EDGE } else { 1.0 };
    [
        shade
            * if left_height > bottom {
                if crease { corner } else { AO_EDGE }
            } else {
                base
            },
        shade
            * if right_height > bottom {
                if crease { corner } else { AO_EDGE }
            } else {
                base
            },
        shade * if right_height > top { AO_EDGE } else { 1.0 },
        shade * if left_height > top { AO_EDGE } else { 1.0 },
    ]
}

#[allow(clippy::too_many_arguments)]
fn append_textured_shoreline(
    mesh: &mut SurfaceMeshData,
    geometry: &GridGeometry,
    column: usize,
    row: usize,
    direction: Direction,
    bounds: [f32; 4],
    bottom: f32,
    top: f32,
) {
    let [x0, x1, z0, z1] = bounds;
    let (u0, u1, v0, v1) = geometry.uv(column, row);
    let source_fraction = ((top - bottom) / geometry.tile_height).clamp(0.0, 1.0);
    let cropped_v0 = v1 - (v1 - v0) * source_fraction;
    let shade = match direction {
        Direction::North => 0.68,
        Direction::South => 1.0,
        Direction::West => 0.76,
        Direction::East => 0.86,
    };
    let color = [shade, shade, shade, 1.0];
    let (positions, normal, uvs) = match direction {
        Direction::North => (
            [
                [x0, bottom, z0],
                [x0, top, z0],
                [x1, top, z0],
                [x1, bottom, z0],
            ],
            [0.0, 0.0, -1.0],
            [[u0, v1], [u0, cropped_v0], [u1, cropped_v0], [u1, v1]],
        ),
        Direction::South => (
            [
                [x1, bottom, z1],
                [x1, top, z1],
                [x0, top, z1],
                [x0, bottom, z1],
            ],
            [0.0, 0.0, 1.0],
            [[u1, v1], [u1, cropped_v0], [u0, cropped_v0], [u0, v1]],
        ),
        Direction::West => (
            [
                [x0, bottom, z1],
                [x0, top, z1],
                [x0, top, z0],
                [x0, bottom, z0],
            ],
            [-1.0, 0.0, 0.0],
            [[u1, v1], [u1, cropped_v0], [u0, cropped_v0], [u0, v1]],
        ),
        Direction::East => (
            [
                [x1, bottom, z0],
                [x1, top, z0],
                [x1, top, z1],
                [x1, bottom, z1],
            ],
            [1.0, 0.0, 0.0],
            [[u0, v1], [u0, cropped_v0], [u1, cropped_v0], [u1, v1]],
        ),
    };
    append_quad(mesh, positions, normal, uvs, color);
}

fn facade_covers_south_edge(
    tile: &VisualTile,
    shape: CellShape,
    row: usize,
    raised_height: f32,
    tile_height: f32,
) -> bool {
    let (plane_subtile_row, band_from_top, band_count) = match shape {
        CellShape::FacadeBand {
            plane_subtile_row,
            band_from_top,
            band_count,
            ..
        }
        | CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: plane_subtile_row,
            band_from_top,
            band_count,
            ..
        } => (plane_subtile_row, band_from_top, band_count),
        _ => return false,
    };
    if band_from_top != 0 || band_count as f32 * tile_height < raised_height {
        return false;
    }
    row as isize - tile.source.subtile_row as isize + plane_subtile_row as isize == row as isize
}

fn append_top(mesh: &mut SurfaceMeshData, bounds: [f32; 4], height: f32, uv: (f32, f32, f32, f32)) {
    append_top_shaded(mesh, bounds, height, uv, TEXTURED_SHADE);
}

fn rock_platform_corner_clips(
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    column: usize,
    row: usize,
) -> [bool; 4] {
    let height = shapes[row * geometry.width + column].surface_height(geometry.tile_height);
    let continues = |column: isize, row: isize| {
        if column < 0
            || row < 0
            || column >= geometry.width as isize
            || row >= geometry.height as isize
        {
            // The published viewport edge is unknown continuation, never an
            // authored outer corner.
            return true;
        }
        let source = &cells[row as usize * geometry.width + column as usize].source;
        let index = row as usize * geometry.width + column as usize;
        matches!(source.tileset_id.as_ref(), "johto" | "johto_modern")
            && source.metatile_id == 0x0a
            && (shapes[index].surface_height(geometry.tile_height) - height).abs() < f32::EPSILON
    };
    let north = continues(column as isize, row as isize - 1);
    let south = continues(column as isize, row as isize + 1);
    let west = continues(column as isize - 1, row as isize);
    let east = continues(column as isize + 1, row as isize);
    [
        !north && !west,
        !north && !east,
        !south && !west,
        !south && !east,
    ]
}

fn append_rock_platform_top(
    mesh: &mut SurfaceMeshData,
    bounds: [f32; 4],
    height: f32,
    uv: (f32, f32, f32, f32),
    subtile_column: u8,
    subtile_row: u8,
    corner_clips: [bool; 4],
    shade: [f32; 4],
) {
    const CLIP: f32 = 4.0;
    let [x0, x1, z0, z1] = bounds;
    let (u0, u1, v0, v1) = uv;
    let um = (u0 + u1) * 0.5;
    let vm = (v0 + v1) * 0.5;
    let vertices: Vec<([f32; 3], [f32; 2])> = match (subtile_column, subtile_row) {
        (0, 0) if corner_clips[0] => vec![
            ([x0 + CLIP, height, z0], [um, v0]),
            ([x0, height, z0 + CLIP], [u0, vm]),
            ([x0, height, z1], [u0, v1]),
            ([x1, height, z1], [u1, v1]),
            ([x1, height, z0], [u1, v0]),
        ],
        (3, 0) if corner_clips[1] => vec![
            ([x0, height, z0], [u0, v0]),
            ([x0, height, z1], [u0, v1]),
            ([x1, height, z1], [u1, v1]),
            ([x1, height, z0 + CLIP], [u1, vm]),
            ([x1 - CLIP, height, z0], [um, v0]),
        ],
        (0, 3) if corner_clips[2] => vec![
            ([x0, height, z0], [u0, v0]),
            ([x0, height, z1 - CLIP], [u0, vm]),
            ([x0 + CLIP, height, z1], [um, v1]),
            ([x1, height, z1], [u1, v1]),
            ([x1, height, z0], [u1, v0]),
        ],
        (3, 3) if corner_clips[3] => vec![
            ([x0, height, z0], [u0, v0]),
            ([x0, height, z1], [u0, v1]),
            ([x1 - CLIP, height, z1], [um, v1]),
            ([x1, height, z1 - CLIP], [u1, vm]),
            ([x1, height, z0], [u1, v0]),
        ],
        _ => {
            append_top_shaded(mesh, bounds, height, uv, shade);
            return;
        }
    };
    append_polygon(mesh, &vertices, [0.0, 1.0, 0.0], shade);
}

fn append_top_shaded(
    mesh: &mut SurfaceMeshData,
    bounds: [f32; 4],
    height: f32,
    uv: (f32, f32, f32, f32),
    shade: [f32; 4],
) {
    let [x0, x1, z0, z1] = bounds;
    let (u0, u1, v0, v1) = uv;
    append_quad(
        mesh,
        [
            [x0, height, z0],
            [x0, height, z1],
            [x1, height, z1],
            [x1, height, z0],
        ],
        [0.0, 1.0, 0.0],
        [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
        shade,
    );
}

fn append_solid_quad(
    mesh: &mut SurfaceMeshData,
    positions: [[f32; 3]; 4],
    normal: [f32; 3],
    color: [f32; 4],
) {
    append_quad(mesh, positions, normal, [[0.0, 0.0]; 4], color);
}

fn append_polygon(
    mesh: &mut SurfaceMeshData,
    vertices: &[([f32; 3], [f32; 2])],
    normal: [f32; 3],
    color: [f32; 4],
) {
    debug_assert!(vertices.len() >= 3);
    for index in 1..vertices.len() - 1 {
        let [origin, current, next] = [vertices[0], vertices[index], vertices[index + 1]];
        // Terrain consumers process faces in four-vertex groups. Encode each
        // fan triangle as a quad whose second triangle is degenerate so the
        // chamfer does not disturb that contract.
        append_quad(
            mesh,
            [origin.0, current.0, next.0, origin.0],
            normal,
            [origin.1, current.1, next.1, origin.1],
            color,
        );
    }
}

fn append_quad(
    mesh: &mut SurfaceMeshData,
    positions: [[f32; 3]; 4],
    normal: [f32; 3],
    uvs: [[f32; 2]; 4],
    color: [f32; 4],
) {
    append_quad_colors(mesh, positions, normal, uvs, [color; 4]);
}

fn append_quad_colors(
    mesh: &mut SurfaceMeshData,
    positions: [[f32; 3]; 4],
    normal: [f32; 3],
    uvs: [[f32; 2]; 4],
    colors: [[f32; 4]; 4],
) {
    let base = u32::try_from(mesh.positions.len()).expect("MVP terrain vertex count fits u32");
    mesh.positions.extend(positions);
    mesh.normals.extend([normal; 4]);
    mesh.uvs.extend(uvs);
    mesh.colors.extend(colors);
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn solid_color(kind: SolidKind, direction: Direction) -> [f32; 4] {
    let base = match kind {
        SolidKind::Building => [0.43, 0.36, 0.27],
        SolidKind::Tree => [0.20, 0.32, 0.18],
        SolidKind::Rock => [0.32, 0.27, 0.22],
        SolidKind::Ship => [0.30, 0.38, 0.43],
        SolidKind::FlatCard => [0.38, 0.34, 0.27],
        SolidKind::Prop => [0.38, 0.34, 0.27],
        SolidKind::CutTree => [0.20, 0.32, 0.14],
        SolidKind::Bank => [0.30, 0.25, 0.18],
        SolidKind::Flower => [0.34, 0.42, 0.20],
        SolidKind::Grass => [0.20, 0.38, 0.16],
        SolidKind::Fence => [0.34, 0.34, 0.30],
    };
    let shade = match direction {
        Direction::North => 0.68,
        Direction::South => 0.96,
        Direction::West => 0.76,
        Direction::East => 0.86,
    };
    [base[0] * shade, base[1] * shade, base[2] * shade, 1.0]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainMeshError {
    InvalidVisualFrame(crystal_render_api::VisualWorldFrameError),
    InactiveFrame,
    GridTooLarge,
    DuplicateTile {
        column: u32,
        row: u32,
    },
    MissingTile {
        column: u32,
        row: u32,
    },
    MissingGroundSample {
        column: u32,
        row: u32,
        tile_index: u16,
    },
    MissingMaskImage {
        column: u32,
        row: u32,
    },
    InvalidMaskImage {
        column: u32,
        row: u32,
    },
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::prelude::{Handle, Image, UVec2, Vec2};
    use crystal_render_api::{VisualTile, VisualTileSource, VisualWorldFrame};

    use super::*;

    fn source(metatile_id: u16, subtile_column: u8, subtile_row: u8) -> VisualTileSource {
        source_with_tile(metatile_id, subtile_column, subtile_row, 0x06)
    }

    fn source_with_tile(
        metatile_id: u16,
        subtile_column: u8,
        subtile_row: u8,
        tile_index: u16,
    ) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("johto"),
            metatile_id,
            subtile_column,
            subtile_row,
            tile_index,
        }
    }

    fn source_for_tileset(
        tileset_id: &str,
        metatile_id: u16,
        subtile_column: u8,
        subtile_row: u8,
        tile_index: u16,
    ) -> VisualTileSource {
        let mut source = source_with_tile(metatile_id, subtile_column, subtile_row, tile_index);
        source.tileset_id = Arc::from(tileset_id);
        source
    }

    fn frame(width: u32, height: u32, sources: Vec<VisualTileSource>) -> VisualWorldFrame {
        assert_eq!(sources.len(), (width * height) as usize);
        VisualWorldFrame {
            active: true,
            map_id: Arc::from("NewBarkTown"),
            terrain_revision: 1,
            map_texture: Handle::<Image>::weak_from_u128(1),
            center: Vec2::ZERO,
            viewport_size: Vec2::new(width as f32 * 8.0, height as f32 * 8.0),
            tile_size: Vec2::splat(8.0),
            grid_size: UVec2::new(width, height),
            tiles: sources
                .into_iter()
                .enumerate()
                .map(|(index, source)| VisualTile {
                    column: index as u32 % width,
                    row: index as u32 / width,
                    source,
                    texture: Handle::<Image>::weak_from_u128(index as u128 + 10),
                    priority: false,
                })
                .collect(),
            actors: Vec::new(),
        }
    }

    fn flat_source() -> VisualTileSource {
        source(0x01, 0, 0)
    }

    #[test]
    fn viewport_edge_has_no_generated_skirt() {
        let mesh = build_terrain_mesh(&frame(1, 1, vec![flat_source()]))
            .expect("one unknown cell should remain a flat surface");
        assert_eq!(mesh.textured.quad_count(), 1);
        assert_eq!(mesh.solid.quad_count(), 0);
    }

    #[test]
    fn game_corner_stool_is_recognized_only_as_a_complete_two_by_two_drawing() {
        let sources = vec![
            source_for_tileset("game_corner", 0x05, 2, 0, 0x0a),
            source_for_tileset("game_corner", 0x05, 3, 0, 0x0b),
            source_for_tileset("game_corner", 0x05, 2, 1, 0x1a),
            source_for_tileset("game_corner", 0x05, 3, 1, 0x1b),
        ];
        let complete = frame(2, 2, sources.clone());
        let cells: Vec<_> = complete.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 2,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            casino_stool_placements(&cells, &geometry),
            vec![CasinoStoolPlacement { column: 0, row: 0 }]
        );

        let mut incomplete_sources = sources;
        incomplete_sources[3].tile_index = 0x01;
        let incomplete = frame(2, 2, incomplete_sources);
        let incomplete_cells: Vec<_> = incomplete.tiles.iter().collect();
        assert!(casino_stool_placements(&incomplete_cells, &geometry).is_empty());
    }

    #[test]
    fn coverage_auditor_reports_ice_path_boulders_as_props_not_trees() {
        let sources = vec![
            source_for_tileset("ice_path", 0x1a, 2, 0, 0x82),
            source_for_tileset("ice_path", 0x1a, 3, 0, 0x83),
            source_for_tileset("ice_path", 0x1a, 2, 1, 0x92),
            source_for_tileset("ice_path", 0x1a, 3, 1, 0x93),
        ];
        let frame = frame(2, 2, sources);
        assert_eq!(
            audit_cell_coverage_on_map("IcePath1F", &frame.tiles, 2, 2)
                .expect("complete Ice Path boulder should audit"),
            vec![CellCoverageKind::Cutout; 4]
        );
    }

    #[test]
    fn grouped_park_fountain_is_one_continuous_shell_not_pixel_spikes() {
        let geometry = GridGeometry {
            width: 8,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -32.0,
            origin_z: -32.0,
        };
        let mut mesh = TerrainMeshData::default();
        append_park_fountain(
            &mut mesh,
            &geometry,
            FountainPlacement { column: 2, row: 2 },
        );

        assert_eq!(mesh.solid.quad_count(), 24, "one side per oval segment");
        assert_eq!(
            mesh.textured.quad_count(),
            24,
            "one triangle-fan segment per top"
        );
        let distinct_top_heights = mesh
            .textured
            .positions
            .iter()
            .map(|position| position[1].to_bits())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(distinct_top_heights.len(), 1);
        assert!(
            mesh.solid
                .positions
                .chunks_exact(4)
                .all(|quad| quad[0][1] == quad[3][1] && quad[1][1] == quad[2][1])
        );
    }

    #[test]
    fn shoreline_drop_uses_cropped_ground_art_instead_of_a_solid_wall() {
        let ground = source_for_tileset("kanto", 0x01, 0, 0, 0x2c);
        let water = source_for_tileset("kanto", 0x15, 0, 0, 0x14);
        let mesh = build_terrain_mesh(&frame(2, 1, vec![ground, water]))
            .expect("authored water edge should mesh");

        assert_eq!(mesh.textured.quad_count(), 3);
        assert_eq!(mesh.solid.quad_count(), 0);
        let shoreline_uvs = &mesh.textured.uvs[8..12];
        let min_v = shoreline_uvs
            .iter()
            .map(|uv| uv[1])
            .fold(f32::INFINITY, f32::min);
        let max_v = shoreline_uvs
            .iter()
            .map(|uv| uv[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_v - min_v - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn waterfall_uses_each_source_row_once_on_one_vertical_plane() {
        let mut sources = Vec::new();
        for row in 0..2 {
            for column in 0..3 {
                sources.push(source_for_tileset(
                    "cave",
                    if column < 2 { 0x2c } else { 0x01 },
                    column as u8,
                    row as u8,
                    if column < 2 { 0x40 } else { 0x14 },
                ));
            }
        }
        let mesh = build_terrain_mesh(&frame(3, 2, sources))
            .expect("waterfall and its authored water replacement should mesh");

        assert_eq!(mesh.textured.quad_count(), 10);
        let waterfall_vertices = &mesh.textured.positions[6 * 4..10 * 4];
        let plane_z = waterfall_vertices[0][2];
        for quad in waterfall_vertices.chunks_exact(4) {
            assert!(
                quad.iter()
                    .all(|position| (position[2] - plane_z).abs() < f32::EPSILON)
            );
            let min_y = quad
                .iter()
                .map(|position| position[1])
                .fold(f32::INFINITY, f32::min);
            let max_y = quad
                .iter()
                .map(|position| position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!((max_y - min_y - 8.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn cianwood_buoy_relief_keeps_its_base_on_recessed_water() {
        let buoy = source_with_tile(0x34, 0, 0, 0x58);
        let water = source_with_tile(0x34, 2, 0, 0x14);
        let mesh = build_terrain_mesh(&frame(2, 1, vec![buoy, water]))
            .expect("buoy relief should resolve its authored water base");

        assert_eq!(mesh.textured.quad_count(), 2);
        assert!(
            mesh.textured.positions.iter().all(|position| {
                (position[1] - crate::profile::WATER_HEIGHT).abs() < f32::EPSILON
            })
        );
        assert_eq!(mesh.solid.quad_count(), 0);
    }

    #[test]
    fn shore_transition_keeps_its_exact_rock_cap_art() {
        let ground = flat_source();
        let shore = source_with_tile(0x54, 0, 0, 0x4c);
        let water = source_with_tile(0x54, 1, 0, 0x14);
        let mesh = build_terrain_mesh(&frame(3, 1, vec![ground, shore, water]))
            .expect("authored shoreline should mesh");
        let shore_top = &mesh.textured.uvs[4..8];
        let min_u = shore_top
            .iter()
            .map(|uv| uv[0])
            .fold(f32::INFINITY, f32::min);
        let max_u = shore_top
            .iter()
            .map(|uv| uv[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_u - 1.0 / 3.0).abs() < f32::EPSILON);
        assert!((max_u - 2.0 / 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn port_ship_keeps_water_holes_and_emits_only_native_height_side_bands() {
        let ship = source_for_tileset("port", 0x18, 0, 2, 0x2b);
        let water = source_for_tileset("port", 0x18, 1, 2, 0x14);
        let mesh = build_terrain_mesh(&frame(2, 1, vec![ship, water]))
            .expect("ship edge beside open port water should mesh");
        assert!(
            mesh.textured.positions[0..4]
                .iter()
                .all(|position| position[1] == crate::port::SHIP_HEIGHT)
        );
        assert!(
            mesh.textured.positions[4..8]
                .iter()
                .all(|position| position[1] == crate::profile::WATER_HEIGHT)
        );
        let side_quads: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normal)| normal[0] != [0.0, 1.0, 0.0])
            .map(|(positions, _)| positions)
            .collect();
        assert!(!side_quads.is_empty());
        assert!(side_quads.iter().all(|quad| {
            let min = quad
                .iter()
                .map(|position| position[1])
                .fold(f32::INFINITY, f32::min);
            let max = quad
                .iter()
                .map(|position| position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            max - min <= SOURCE_TILE_HEIGHT
        }));
    }

    #[test]
    fn flower_mask_subtracts_dark_paletted_ground_instead_of_standing_a_full_card() {
        let flower_source = source_with_tile(0x03, 0, 0, 0x03);
        let ground_source = source_with_tile(0x01, 0, 0, 0x05);
        let frame = frame(2, 1, vec![flower_source, ground_source]);
        let flower = &frame.tiles[0];
        let ground = &frame.tiles[1];
        let ground_pixel = [16, 80, 96, 255];
        let mut ground_rgba = ground_pixel.repeat(64);
        let mut flower_rgba = ground_rgba.clone();
        for (x, y) in [(3, 3), (4, 3), (3, 4), (4, 4)] {
            let offset = (y * 8 + x) * 4;
            flower_rgba[offset..offset + 4].copy_from_slice(&[112, 0, 96, 255]);
        }
        let mut images = TerrainImageSamples::default();
        images
            .pixels
            .insert(flower.texture.id(), TileImageSample::Rgba(flower_rgba));
        images.pixels.insert(
            ground.texture.id(),
            TileImageSample::Rgba(std::mem::take(&mut ground_rgba)),
        );

        let removable = decorative_cutout_mask(&images, flower, ground, SolidKind::Flower)
            .expect("paletted flower mask should resolve");
        assert_eq!(removable.iter().filter(|pixel| !**pixel).count(), 4);
        assert!(!removable[3 * 8 + 3]);
        assert!(removable[0]);
    }

    #[test]
    fn flowers_stand_behind_actors_on_the_same_ground_cell() {
        assert_eq!(upright_plane_z(SolidKind::Flower, 8.0, 16.0), 8.0);
        assert_eq!(upright_plane_z(SolidKind::CutTree, 8.0, 16.0), 12.0);
        assert_eq!(upright_plane_z(SolidKind::Prop, 8.0, 16.0), 12.0);
    }

    #[test]
    fn fence_posts_are_deeper_than_sign_plates() {
        assert_eq!(upright_depth(SolidKind::Prop), 2.0);
        assert_eq!(upright_depth(SolidKind::Fence), 6.0);
    }

    #[test]
    fn grass_tile_rows_stand_on_distinct_depth_planes() {
        let mut textured = SurfaceMeshData::default();
        let mut solid = SurfaceMeshData::default();
        let mut removable = [true; 64];
        removable[3 * 8 + 3] = false;
        for plane_z in [4.0, 12.0] {
            append_masked_upright_hull(
                &mut textured,
                &mut solid,
                &removable,
                [0.0, 8.0, 0.0, 8.0, plane_z],
                [0.0, 1.0, 0.0, 1.0],
                SolidKind::Grass,
            )
            .expect("grass tuft should mesh");
        }
        assert!(
            textured
                .positions
                .iter()
                .any(|position| (position[2] - 4.0).abs() < f32::EPSILON)
        );
        assert!(
            textured
                .positions
                .iter()
                .any(|position| (position[2] - 12.0).abs() < f32::EPSILON)
        );
        assert_eq!(upright_depth(SolidKind::Grass), 2.0);
    }

    #[test]
    fn only_authored_rock_platform_and_kanto_mound_families_taper() {
        assert_eq!(
            bank_taper(&source_with_tile(0x0a, 0, 0, 0)),
            Some([10.0, 4.0])
        );
        let mut mound = source_with_tile(0x3e, 0, 0, 0);
        mound.tileset_id = Arc::from("kanto");
        assert_eq!(bank_taper(&mound), Some([16.0, 8.0]));
        mound.metatile_id = 0x01;
        assert_eq!(bank_taper(&mound), None);
    }

    #[test]
    fn johto_transition_drawing_folds_only_authored_south_face_bands() {
        let mut sources = Vec::new();
        for row in 0..5 {
            for column in 0..4 {
                sources.push(if row < 4 {
                    source_with_tile(
                        0x72,
                        column as u8,
                        row as u8,
                        if row < 2 { 0x3c } else { 0x4c },
                    )
                } else {
                    flat_source()
                });
            }
        }
        let mesh =
            build_terrain_mesh(&frame(4, 5, sources)).expect("authored mountain ledge should mesh");
        let south_faces: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .zip(mesh.textured.uvs.chunks_exact(4))
            .filter(|((_, normal), _)| normal[0] == [0.0, 0.0, 1.0])
            .map(|((positions, _), uvs)| (positions, uvs))
            .collect();
        assert!(!south_faces.is_empty());
        assert!(south_faces.iter().all(|(positions, _)| {
            let min = positions
                .iter()
                .map(|position| position[1])
                .fold(f32::INFINITY, f32::min);
            let max = positions
                .iter()
                .map(|position| position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            max - min <= SOURCE_TILE_HEIGHT
        }));
    }

    #[test]
    fn blackthorn_transition_corner_folds_connected_south_and_east_faces() {
        let mut sources = Vec::new();
        for row in 0..5 {
            for column in 0..5 {
                sources.push(if row < 4 && column < 4 {
                    source_with_tile(
                        0x6d,
                        column as u8,
                        row as u8,
                        match (column >= 2, row >= 2) {
                            (_, true) => 0x4c,
                            (true, false) => 0x3d,
                            _ => 0x3c,
                        },
                    )
                } else {
                    flat_source()
                });
            }
        }
        let mesh = build_terrain_mesh(&frame(5, 5, sources))
            .expect("authored mountain corner should mesh");
        let south_faces = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|normal| normal[0] == [0.0, 0.0, 1.0])
            .count();
        let east_faces = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|normal| normal[0] == [1.0, 0.0, 0.0])
            .count();
        assert!(south_faces > 0);
        assert!(east_faces > 0);
    }

    #[test]
    fn transition_run_uses_authored_directional_wall_courses() {
        let sources = vec![
            source_with_tile(0x69, 2, 0, 0x3d),
            source_with_tile(0x69, 3, 0, 0x3d),
            flat_source(),
            source_with_tile(0x70, 0, 0, 0x3c),
            flat_source(),
        ];
        let mesh = build_terrain_mesh(&frame(5, 1, sources))
            .expect("connected plateau side should use authored wall art");
        let textured_east_courses = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(positions, normals)| {
                normals[0] == [1.0, 0.0, 0.0]
                    && positions.iter().all(|position| position[0] == 12.0)
            })
            .count();
        let solid_east_faces = mesh
            .solid
            .positions
            .chunks_exact(4)
            .zip(mesh.solid.normals.chunks_exact(4))
            .filter(|(positions, normals)| {
                normals[0] == [1.0, 0.0, 0.0]
                    && positions.iter().all(|position| position[0] == 12.0)
            })
            .count();
        assert_eq!(textured_east_courses, 4);
        assert_eq!(solid_east_faces, 0);
    }

    #[test]
    fn transition_bank_run_folds_native_front_bands() {
        let sources = vec![
            source_with_tile(0x70, 0, 0, 0x3c),
            flat_source(),
            source_with_tile(0x72, 0, 1, 0x4b),
            flat_source(),
            source_with_tile(0x72, 0, 2, 0x4c),
            flat_source(),
            flat_source(),
            flat_source(),
        ];
        let mesh = build_terrain_mesh(&frame(2, 4, sources))
            .expect("bank column should fold its own source courses");
        let mut v_ranges: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .zip(mesh.textured.uvs.chunks_exact(4))
            .filter(|((positions, normals), _)| {
                normals[0] == [1.0, 0.0, 0.0] && positions.iter().all(|position| position[0] == 0.0)
            })
            .map(|(_, uvs)| {
                let min = uvs.iter().map(|uv| uv[1]).fold(f32::INFINITY, f32::min);
                let max = uvs.iter().map(|uv| uv[1]).fold(f32::NEG_INFINITY, f32::max);
                ((min * 100.0).round() as i32, (max * 100.0).round() as i32)
            })
            .collect();
        v_ranges.sort_unstable();
        v_ranges.dedup();
        assert!(!v_ranges.is_empty());
        assert!(v_ranges.iter().all(|(min, max)| max - min <= 25));
    }

    #[test]
    fn johto_rock_platform_is_one_raised_tapered_hull() {
        let tile_indices = [
            0x2b, 0x2c, 0x2c, 0x2d, 0x3b, 0x3c, 0x3c, 0x3d, 0x3b, 0x3c, 0x3c, 0x3d, 0x4b, 0x4c,
            0x4c, 0x4d,
        ];
        let mut sources = vec![flat_source(); 36];
        for (index, tile_index) in tile_indices.into_iter().enumerate() {
            let local_column = index % 4;
            let local_row = index / 4;
            sources[(local_row + 1) * 6 + local_column + 1] =
                source_with_tile(0x0a, local_column as u8, local_row as u8, tile_index);
        }
        let mesh = build_terrain_mesh(&frame(6, 6, sources)).expect("rock platform meshes");
        let raised_tops = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(positions, normals)| {
                normals[0] == [0.0, 1.0, 0.0]
                    && positions.iter().all(|position| {
                        (position[1] - crate::profile::MOUNTAIN_LEDGE_HEIGHT).abs() < f32::EPSILON
                    })
            })
            .count();
        assert_eq!(
            raised_tops, 24,
            "twelve square cells and four three-triangle corner fans rise together"
        );

        let mut raised_top_source_rows: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .zip(mesh.textured.uvs.chunks_exact(4))
            .filter(|((positions, normals), _)| {
                normals[0] == [0.0, 1.0, 0.0]
                    && positions.iter().all(|position| {
                        (position[1] - crate::profile::MOUNTAIN_LEDGE_HEIGHT).abs() < f32::EPSILON
                    })
            })
            .map(|(_, uvs)| {
                (uvs.iter().map(|uv| uv[1]).fold(f32::INFINITY, f32::min) * 6.0).round() as i32
            })
            .collect();
        raised_top_source_rows.sort_unstable();
        assert_eq!(
            raised_top_source_rows,
            vec![
                1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4,
            ],
            "the cap preserves the complete authored 4x4 drawing"
        );

        let tapered_faces = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|normals| normals[0][1] > 0.0 && normals[0][1] < 1.0)
            .count();
        assert_eq!(
            tapered_faces, 32,
            "four textured sides use two 8px tapered courses"
        );
        let chamfered_faces = mesh
            .solid
            .normals
            .chunks_exact(4)
            .filter(|normals| {
                normals[0][1] > 0.0
                    && normals[0][1] < 1.0
                    && normals[0][0] != 0.0
                    && normals[0][2] != 0.0
            })
            .count();
        assert_eq!(
            chamfered_faces, 8,
            "four neutral chamfer faces use two 8px courses"
        );

        let tapered_vertices: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normals)| normals[0][1] > 0.0 && normals[0][1] < 1.0)
            .flat_map(|(positions, _)| positions.iter().copied())
            .collect();
        let base_min_x = tapered_vertices
            .iter()
            .filter(|position| position[1].abs() < f32::EPSILON)
            .map(|position| position[0])
            .fold(f32::INFINITY, f32::min);
        let cap_min_x = tapered_vertices
            .iter()
            .filter(|position| {
                (position[1] - crate::profile::MOUNTAIN_LEDGE_HEIGHT).abs() < f32::EPSILON
            })
            .map(|position| position[0])
            .fold(f32::INFINITY, f32::min);
        assert_eq!(
            cap_min_x - base_min_x,
            10.0,
            "the base spreads beyond the cap"
        );
    }

    #[test]
    fn johto_modern_uses_the_same_authored_rock_platform_hull() {
        let tile_indices = [
            0x2b, 0x2c, 0x2c, 0x2d, 0x3b, 0x3c, 0x3c, 0x3d, 0x3b, 0x3c, 0x3c, 0x3d, 0x4b, 0x4c,
            0x4c, 0x4d,
        ];
        let mut sources = vec![flat_source(); 36];
        for (index, tile_index) in tile_indices.into_iter().enumerate() {
            let local_column = index % 4;
            let local_row = index / 4;
            let mut source =
                source_with_tile(0x0a, local_column as u8, local_row as u8, tile_index);
            source.tileset_id = Arc::from("johto_modern");
            sources[(local_row + 1) * 6 + local_column + 1] = source;
        }
        let mesh =
            build_terrain_mesh(&frame(6, 6, sources)).expect("Johto Modern rock platform meshes");

        let raised_caps = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(positions, normals)| {
                normals[0] == [0.0, 1.0, 0.0]
                    && positions.iter().all(|position| {
                        (position[1] - crate::profile::MOUNTAIN_LEDGE_HEIGHT).abs() < f32::EPSILON
                    })
            })
            .count();
        assert_eq!(raised_caps, 24);
        assert!(
            mesh.textured
                .normals
                .chunks_exact(4)
                .any(|normals| normals[0][1] > 0.0 && normals[0][1] < 1.0)
        );
    }

    #[test]
    fn adjacent_rock_platforms_remain_one_universal_height() {
        let tile_indices = [
            0x2b, 0x2c, 0x2c, 0x2d, 0x3b, 0x3c, 0x3c, 0x3d, 0x3b, 0x3c, 0x3c, 0x3d, 0x4b, 0x4c,
            0x4c, 0x4d,
        ];
        let mut sources = vec![flat_source(); 60];
        for platform_row in 0..2 {
            for (index, tile_index) in tile_indices.into_iter().enumerate() {
                let local_column = index % 4;
                let local_row = index / 4;
                let row = 1 + platform_row * 4 + local_row;
                sources[row * 6 + local_column + 1] =
                    source_with_tile(0x0a, local_column as u8, local_row as u8, tile_index);
            }
        }
        let mesh = build_terrain_mesh(&frame(6, 10, sources)).expect("joined rocks mesh");
        let cap_faces_at = |height: f32| {
            mesh.textured
                .positions
                .chunks_exact(4)
                .zip(mesh.textured.normals.chunks_exact(4))
                .filter(|(positions, normals)| {
                    normals[0] == [0.0, 1.0, 0.0]
                        && positions
                            .iter()
                            .all(|position| (position[1] - height).abs() < f32::EPSILON)
                })
                .count()
        };
        assert_eq!(
            cap_faces_at(crate::profile::MOUNTAIN_LEDGE_HEIGHT),
            40,
            "the shared seam removes the two touching outer-corner fans"
        );
        assert_eq!(cap_faces_at(crate::profile::MOUNTAIN_LEDGE_HEIGHT * 2.0), 0);
        assert_eq!(
            mesh.footing_heights[2 * 6 + 2],
            crate::profile::MOUNTAIN_LEDGE_HEIGHT,
            "the northern copy remains on the universal platform course"
        );
        assert_eq!(
            mesh.footing_heights[6 * 6 + 2],
            crate::profile::MOUNTAIN_LEDGE_HEIGHT,
            "the southern copy remains on the same platform course"
        );
    }

    #[test]
    fn cave_doorway_art_is_not_repeated_on_additional_cliff_tiers() {
        let frame = frame(
            2,
            2,
            vec![
                source_with_tile(0x73, 0, 0, 0x57),
                source_with_tile(0x72, 0, 0, 0x4c),
                source_with_tile(0x73, 0, 1, 0x3c),
                source_with_tile(0x72, 0, 1, 0x3c),
            ],
        );
        let cells: Vec<_> = frame.tiles.iter().collect();
        let band = |band_from_top| CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 2,
            band_from_top,
            band_count: 2,
            top_tile_index: 0x3c,
            height: crate::profile::MOUNTAIN_CLIFF_HEIGHT * 2.0,
        };
        let shapes = vec![band(1), band(1), band(0), band(0)];
        let geometry = GridGeometry {
            width: 2,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let run = BankColumnRun { north: 0, front: 1 };
        assert_eq!(
            authored_bank_face_cell(&cells, &shapes, &geometry, run, 0, Direction::South, 0,),
            Some(0),
            "the authored lowest tier keeps its one doorway"
        );
        assert_eq!(
            authored_bank_face_cell(&cells, &shapes, &geometry, run, 0, Direction::South, 2,),
            Some(1),
            "the repeated upper tier borrows ordinary rock art"
        );
    }

    #[test]
    fn bank_side_ao_darkens_the_ground_crease_and_inside_corner() {
        let shades = side_ao_shades(16.0, 0.0, 0.0, 8.0, true, 1.0);
        assert!(shades[0] < shades[1]);
        assert!(shades[0] < shades[3]);
        assert!(shades[3] < 1.0);
        assert_eq!(shades[2], 1.0);
    }

    #[test]
    fn traditional_roof_rises_to_middle_ridge_and_closes_at_both_eaves() {
        assert_eq!(gabled_roof_height(32.0, 16.0, 0.0, 24.0), 32.0);
        assert_eq!(gabled_roof_height(32.0, 16.0, 12.0, 24.0), 48.0);
        assert_eq!(gabled_roof_height(32.0, 16.0, 24.0, 24.0), 32.0);
    }

    #[test]
    fn explicit_ledge_face_is_not_covered_by_a_generic_solid_side() {
        let sources = vec![
            source_with_tile(0x69, 2, 0, 0x3c),
            source_with_tile(0x69, 3, 0, 0x3d),
            flat_source(),
        ];
        let mesh =
            build_terrain_mesh(&frame(3, 1, sources)).expect("authored east ledge should mesh");
        let generic_faces = mesh
            .solid
            .positions
            .chunks_exact(4)
            .zip(mesh.solid.normals.chunks_exact(4))
            .filter(|(positions, normals)| {
                normals[0] == [1.0, 0.0, 0.0] && positions.iter().all(|position| position[0] == 4.0)
            })
            .count();
        assert_eq!(generic_faces, 0);
    }

    #[test]
    fn compact_facade_bands_are_native_height_and_share_a_plane() {
        let mut sources = Vec::new();
        for row in 0..4 {
            sources.push(source(0x14, 0, row));
            sources.push(flat_source());
        }
        let mesh = build_terrain_mesh(&frame(2, 4, sources)).expect("compact house should mesh");

        let vertical_faces: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normals)| normals[0][1] == 0.0)
            .map(|(positions, _)| positions)
            .collect();
        assert_eq!(vertical_faces.len(), 2);
        for face in &vertical_faces {
            let min_y = face
                .iter()
                .map(|vertex| vertex[1])
                .fold(f32::INFINITY, f32::min);
            let max_y = face
                .iter()
                .map(|vertex| vertex[1])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(max_y - min_y <= 8.0, "source art was stretched vertically");
        }
        assert_eq!(vertical_faces[0][0][2], vertical_faces[1][0][2]);
    }

    #[test]
    fn generated_sides_are_not_in_the_textured_material_domain() {
        let mesh = build_terrain_mesh(&frame(2, 1, vec![source(0x18, 0, 0), flat_source()]))
            .expect("raised roof beside ground should mesh");

        assert_eq!(
            mesh.textured
                .normals
                .chunks_exact(4)
                .filter(|face| face[0][1] == 0.0)
                .count(),
            0
        );
        assert!(mesh.solid.quad_count() > 0);
        assert!(mesh.solid.uvs.iter().all(|uv| *uv == [0.0, 0.0]));
    }

    #[test]
    fn complete_large_house_is_detected_as_one_authored_placement() {
        let metatiles = [[0x18, 0x19], [0x16, 0x1e]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..8 {
                sources.push(source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                ));
            }
        }
        let frame = frame(8, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -32.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 8,
                height: 8,
                roof_rows: 4,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn forest_entrance_is_one_complete_three_by_three_block_structure() {
        for lower_left in [0x24, 0x25] {
            let metatiles = [
                [0x1d, 0x1e, 0x1f],
                [0x21, 0x22, 0x23],
                [lower_left, 0x26, 0x27],
            ];
            let mut sources = Vec::new();
            for row in 0..12 {
                for column in 0..12 {
                    sources.push(source_for_tileset(
                        "forest",
                        metatiles[row / 4][column / 4],
                        (column % 4) as u8,
                        (row % 4) as u8,
                        0x20,
                    ));
                }
            }
            let frame = frame(12, 12, sources);
            let cells: Vec<_> = frame.tiles.iter().collect();
            let geometry = GridGeometry {
                width: 12,
                height: 12,
                tile_width: 8.0,
                tile_height: 8.0,
                origin_x: -48.0,
                origin_z: -48.0,
            };
            assert_eq!(
                outdoor_building_placements(&cells, &geometry),
                vec![BuildingPlacement {
                    column: 0,
                    row: 0,
                    width: 12,
                    height: 12,
                    roof_rows: 4,
                    ground_tile_index: 0x05,
                }]
            );
        }
    }

    #[test]
    fn traditional_three_block_house_is_one_authored_placement() {
        let metatiles = [[0x2c, 0x2a, 0x2d], [0x26, 0x27, 0x2f]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..12 {
                sources.push(source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                ));
            }
        }
        let frame = frame(12, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 12,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -48.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 2,
                width: 12,
                height: 6,
                roof_rows: 2,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn burned_tower_exterior_is_one_authored_roof_and_facade() {
        let metatiles = [[0x20, 0x21], [0x37, 0x3b]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..8 {
                sources.push(source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                ));
            }
        }
        let frame = frame(8, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -32.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 8,
                height: 8,
                roof_rows: 4,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn complete_modern_city_building_is_detected_outside_new_bark() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..8 {
                let mut tile = source(
                    if column < 4 { 0x12 } else { 0x13 },
                    (column % 4) as u8,
                    row as u8,
                );
                tile.tileset_id = Arc::from("johto_modern");
                sources.push(tile);
            }
        }
        let mut frame = frame(8, 4, sources);
        frame.map_id = Arc::from("GoldenrodCity");
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -32.0,
            origin_z: -16.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![
                BuildingPlacement {
                    column: 0,
                    row: 0,
                    width: 4,
                    height: 4,
                    roof_rows: 2,
                    ground_tile_index: 0x06,
                },
                BuildingPlacement {
                    column: 4,
                    row: 0,
                    width: 4,
                    height: 4,
                    roof_rows: 2,
                    ground_tile_index: 0x06,
                },
            ]
        );
    }

    #[test]
    fn goldenrod_department_store_claims_every_storey_as_one_building() {
        let metatiles = [
            [0x18, 0x1f, 0x19],
            [0x27, 0x23, 0x28],
            [0x27, 0x23, 0x28],
            [0x10, 0x17, 0x33],
        ];
        let mut sources = Vec::new();
        for row in 0..16 {
            for column in 0..12 {
                let mut tile = source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                );
                tile.tileset_id = Arc::from("johto_modern");
                sources.push(tile);
            }
        }
        let frame = frame(12, 16, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 12,
            height: 16,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 12,
                height: 16,
                roof_rows: 4,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn goldenrod_game_corner_facade_is_not_stamped_as_a_second_building() {
        let metatiles = [[0x18, 0x1f, 0x19], [0x10, 0x17, 0x11]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..12 {
                let mut tile = source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                );
                tile.tileset_id = Arc::from("johto_modern");
                sources.push(tile);
            }
        }
        let frame = frame(12, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 12,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 12,
                height: 8,
                roof_rows: 4,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn goldenrod_radio_tower_claims_its_complete_landmark_drawing() {
        let metatiles = [[0x25, 0x26], [0x29, 0x2a], [0x2d, 0x2e]];
        let mut sources = Vec::new();
        for row in 0..12 {
            for column in 0..8 {
                let mut tile = source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                );
                tile.tileset_id = Arc::from("johto_modern");
                sources.push(tile);
            }
        }
        let frame = frame(8, 12, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 12,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 8,
                height: 12,
                roof_rows: 2,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn route34_daycare_is_one_complete_modern_building() {
        let metatiles = [[0x18, 0x1f, 0x19], [0x1a, 0x2c, 0x11]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..12 {
                let mut tile = source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                );
                tile.tileset_id = Arc::from("johto_modern");
                sources.push(tile);
            }
        }
        let mut frame = frame(12, 8, sources);
        frame.map_id = Arc::from("Route34");
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 12,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -48.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 12,
                height: 8,
                roof_rows: 4,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn olivine_lighthouse_is_detected_as_one_tall_door_anchored_tower() {
        let block_rows: [[u16; 2]; 7] = [
            [0x08, 0x09],
            [0x7e, 0x7f],
            [0x13, 0x0f],
            [0x13, 0x0f],
            [0x13, 0x0f],
            [0x13, 0x0f],
            [0x1a, 0x11],
        ];
        let mut sources = Vec::new();
        for block_row in block_rows {
            for subtile_row in 0..4 {
                for block in block_row {
                    for subtile_column in 0..4 {
                        sources.push(source(block, subtile_column, subtile_row));
                    }
                }
            }
        }
        let frame = frame(8, 28, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 28,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 8,
                height: 28,
                roof_rows: 8,
                ground_tile_index: 0x06,
            }]
        );
    }

    #[test]
    fn kanto_connected_roof_and_facade_courses_form_one_building() {
        let metatiles = [[0x20, 0x54, 0x21], [0x37, 0x3a, 0x7e]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..12 {
                sources.push(source_for_tileset(
                    "kanto",
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                    if column == 11 && row == 7 { 0x2c } else { 0x30 },
                ));
            }
        }
        let frame = frame(12, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 12,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -48.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 12,
                height: 8,
                roof_rows: 4,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
            }]
        );
    }

    #[test]
    fn route_2_cliff_mound_claims_all_four_blocks_without_a_center_hole() {
        let metatiles = [[0x3e, 0x3f, 0x3f, 0x3b], [0x24, 0x06, 0x57, 0x25]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..16 {
                sources.push(source_for_tileset(
                    "kanto",
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                    0x11,
                ));
            }
        }
        let frame = frame(16, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 16,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -64.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 16,
                height: 8,
                roof_rows: 6,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
            }]
        );
    }

    #[test]
    fn kanto_rounded_center_cap_and_facade_form_one_building() {
        let metatiles = [[0x68, 0x7f, 0x7f, 0x69], [0x37, 0x3a, 0x3a, 0x73]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..16 {
                sources.push(source_for_tileset(
                    "kanto",
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                    if column == 15 && row == 7 { 0x2c } else { 0x30 },
                ));
            }
        }
        let frame = frame(16, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 16,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -64.0,
            origin_z: -32.0,
        };

        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 16,
                height: 8,
                roof_rows: 4,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
            }]
        );
    }

    #[test]
    fn saffron_silph_claims_repeated_storeys_before_its_facade() {
        let metatiles = [
            [0x68, 0x7f, 0x7f, 0x69],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x37, 0x3a, 0x7d, 0x7e],
        ];
        let mut sources = Vec::new();
        for row in 0..20 {
            for column in 0..16 {
                sources.push(source_for_tileset(
                    "kanto",
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                    0x30,
                ));
            }
        }
        let frame = frame(16, 20, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 16,
            height: 20,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 16,
                height: 20,
                roof_rows: 4,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
            }]
        );
    }

    #[test]
    fn kanto_unknown_adjacency_does_not_invent_a_building() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..8 {
                sources.push(source_for_tileset(
                    "kanto",
                    if column < 4 { 0x20 } else { 0x22 },
                    (column % 4) as u8,
                    row as u8,
                    0x30,
                ));
            }
        }
        let frame = frame(8, 4, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -32.0,
            origin_z: -16.0,
        };
        assert!(outdoor_building_placements(&cells, &geometry).is_empty());
    }

    #[test]
    fn recognized_building_without_ground_evidence_stays_faithfully_flat() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..8 {
                sources.push(source_for_tileset(
                    "kanto",
                    if column < 4 { 0x02 } else { 0x03 },
                    (column % 4) as u8,
                    row as u8,
                    0x30,
                ));
            }
        }
        let frame = frame(8, 4, sources);
        let mut samples = TerrainImageSamples::default();
        for tile in &frame.tiles {
            samples.pixels.insert(
                tile.texture.id(),
                TileImageSample::Rgba([90, 80, 70, 255].repeat(64)),
            );
        }
        let mesh = build_terrain_mesh_with_samples(&frame, &samples)
            .expect("one incomplete object must not disable the optional renderer");
        assert_eq!(mesh.textured.quad_count(), 32);
        assert_eq!(mesh.solid.quad_count(), 0);
        assert!(
            mesh.textured
                .positions
                .iter()
                .all(|position| position[1] == 0.0)
        );
    }

    #[test]
    fn roof_slab_uses_the_drawn_column_silhouette_instead_of_a_generic_gable() {
        assert_eq!(roof_slab_height(32.0, 4, 0, 1.0), 36.0);
        assert_eq!(roof_slab_height(32.0, 4, 2, 1.0), 34.0);
        assert_eq!(roof_slab_height(32.0, 4, 4, 1.0), 32.0);
        assert_eq!(roof_slab_height(32.0, 4, 12, 1.0), 32.0);
    }

    #[test]
    fn building_facade_stays_on_the_original_south_door_edge() {
        assert_eq!(building_facade_plane_z(-72.0, 8, 1.0), -64.0);
        assert_eq!(building_facade_plane_z(-72.0, 32, 1.0), -40.0);
    }

    #[test]
    fn building_roof_profile_keeps_a_gable_but_rejects_one_pixel_noise() {
        let width = 9;
        let rows = 5;
        let mut inside = vec![false; width * rows];
        for (x, top) in [4, 3, 2, 1, 0, 1, 2, 3, 4].into_iter().enumerate() {
            for y in top..rows {
                inside[y * width + x] = true;
            }
        }
        // A decorative pixel above the otherwise two-pixel-inset shoulder
        // must not become a physical chimney/spike.
        inside[2] = true;
        assert_eq!(
            measured_roof_profile(&inside, width, rows),
            vec![3, 3, 1, 1, 1, 1, 2, 3, 3]
        );
    }

    #[test]
    fn deeper_roof_keeps_unique_rims_and_repeats_only_middle_courses() {
        let rows: Vec<_> = (0..48)
            .map(|depth| roof_source_row(depth, 48, 32))
            .collect();
        assert_eq!(&rows[..4], &[0, 1, 2, 3]);
        assert_eq!(&rows[44..], &[28, 29, 30, 31]);
        assert!(rows[4..44].iter().all(|row| (4..28).contains(row)));
        assert_eq!(rows[4], rows[28]);
    }

    #[test]
    fn small_enclosed_facade_pane_recesses_but_siding_stays_flush() {
        let width = 32;
        let height = 16;
        let mut inside = vec![true; width * height];
        let mut luminance = vec![300_u16; width * height];
        // A black frame encloses a small 6x6 bright pane.
        for y in 3..11 {
            for x in 3..11 {
                if x == 3 || x == 10 || y == 3 || y == 10 {
                    luminance[y * width + x] = 0;
                }
            }
        }
        // Outside art is not eligible even if it has a non-black shade.
        inside[15 * width + 31] = false;
        let recessed = facade_recess_mask(&inside, &luminance, width, height, 0, 0);

        assert!(recessed[6 * width + 6]);
        assert!(
            !recessed[3 * width + 3],
            "the proud black frame stays flush"
        );
        assert!(
            !recessed[1 * width + 20],
            "the broad connected siding course must not become one deep panel"
        );
        assert!(!recessed[15 * width + 31]);
    }

    #[test]
    fn facade_side_course_ignores_outline_and_uses_dominant_row_paint() {
        let inside = vec![true; 8];
        // Black outline at both edges, one window pixel, and a repeated
        // siding course across the rest of the authored row.
        let luminance = vec![10, 80, 80, 200, 80, 80, 80, 10];
        assert_eq!(
            facade_side_course_x(&inside, &luminance, 8, 0, 10, false),
            1
        );
        assert_eq!(facade_side_course_x(&inside, &luminance, 8, 0, 10, true), 6);
    }

    #[test]
    fn tree_art_is_one_flat_upright_card_instead_of_a_voxel_hull() {
        let mut sources = Vec::new();
        for row in 0..4 {
            sources.push(source_with_tile(0x05, 0, row, 0x1e + row as u16 * 0x10));
            sources.push(source_with_tile(0x01, 0, 0, 0x05));
        }
        let frame = frame(2, 4, sources);
        let mut samples = TerrainImageSamples::default();
        for tile in &frame.tiles {
            let rgba = if tile.source.metatile_id == 0x05 {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            };
            samples
                .pixels
                .insert(tile.texture.id(), TileImageSample::Rgba(rgba.repeat(64)));
        }
        let mesh = build_terrain_mesh_with_samples(&frame, &samples)
            .expect("tree group should mesh as an upright 2D card");
        let upright = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|face| face[0] == [0.0, 0.0, 1.0])
            .count();
        let backs = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|face| face[0] == [0.0, 0.0, -1.0])
            .count();
        assert!(upright > 0);
        assert_eq!(backs, 0, "flat tree cards do not invent a back volume");
        assert_eq!(mesh.solid.quad_count(), 0);
        let mut front_depths: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normals)| normals[0] == [0.0, 0.0, 1.0])
            .map(|(positions, _)| (positions[0][2] * 1000.0).round() as i32)
            .collect();
        front_depths.sort_unstable();
        front_depths.dedup();
        assert_eq!(front_depths.len(), 1);
    }

    #[test]
    fn repeated_tree_metatile_becomes_two_complete_two_tile_tall_sprites() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..5 {
                sources.push(if column < 4 {
                    source_with_tile(0x05, column as u8, row as u8, 0x20 + row as u16)
                } else {
                    source_with_tile(0x01, 0, 0, 0x05)
                });
            }
        }
        let frame = frame(5, 4, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 5,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -20.0,
            origin_z: -16.0,
        };
        assert_eq!(
            complete_tree_placements(&cells, &geometry),
            vec![
                TreePlacement {
                    column: 0,
                    row: 0,
                    width: 2,
                    height: 4,
                    ground_tile_index: 0x05,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                },
                TreePlacement {
                    column: 2,
                    row: 0,
                    width: 2,
                    height: 4,
                    ground_tile_index: 0x05,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                },
            ]
        );

        let mut samples = TerrainImageSamples::default();
        for tile in &frame.tiles {
            let rgba = if tile.source.metatile_id == 0x05 {
                let mut rgba = [0, 80, 0, 255].repeat(64);
                let at_left = tile.source.subtile_column % 2 == 0;
                let at_right = tile.source.subtile_column % 2 == 1;
                let at_top = tile.source.subtile_row == 0;
                let at_bottom = tile.source.subtile_row == 3;
                for (x, y) in [
                    (0, 0, at_left && at_top),
                    (7, 0, at_right && at_top),
                    (0, 7, at_left && at_bottom),
                    (7, 7, at_right && at_bottom),
                ]
                .into_iter()
                .filter_map(|(x, y, outer_corner)| outer_corner.then_some((x, y)))
                {
                    let offset = (y * SOURCE_TILE_PIXELS + x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
                rgba
            } else {
                [255, 255, 255, 255].repeat(64)
            };
            samples
                .pixels
                .insert(tile.texture.id(), TileImageSample::Rgba(rgba));
        }
        let mesh = build_terrain_mesh_with_samples(&frame, &samples)
            .expect("complete repeated tree drawing should mesh as upright sprites");
        let (min_y, max_y) = mesh.textured.positions.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY),
            |(min, max), position| (min.min(position[1]), max.max(position[1])),
        );
        let expected_world_y_span = 32.0 * std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (max_y - min_y - expected_world_y_span).abs() < 0.001,
            "headbutt trees are two map tiles tall"
        );
        let (min_z, max_z) = mesh
            .textured
            .positions
            .iter()
            .filter(|position| position[1] > 0.0)
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min, max), position| (min.min(position[2]), max.max(position[2])),
            );
        assert!(max_z > min_z, "isolated canopies must have real depth");
    }

    #[test]
    fn battle_tower_border_metatile_becomes_two_complete_upright_trees() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                let mut source = source_with_tile(
                    0x05,
                    column as u8,
                    row as u8,
                    [0x1e, 0x13, 0x13, 0x3e][row] + (column % 2) as u16,
                );
                source.tileset_id = Arc::from("battle_tower_outside");
                sources.push(source);
            }
        }
        let frame = frame(4, 4, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -16.0,
            origin_z: -16.0,
        };
        assert_eq!(
            complete_tree_placements(&cells, &geometry),
            vec![
                TreePlacement {
                    column: 0,
                    row: 0,
                    width: 2,
                    height: 4,
                    ground_tile_index: 0x06,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                },
                TreePlacement {
                    column: 2,
                    row: 0,
                    width: 2,
                    height: 4,
                    ground_tile_index: 0x06,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                },
            ]
        );
    }

    #[test]
    fn forest_sources_reuse_grouped_tree_placements() {
        let forest_source = |metatile_id, column, row, tile_index| {
            let mut source = source_with_tile(metatile_id, column, row, tile_index);
            source.tileset_id = Arc::from("forest");
            source
        };
        let scattered = frame(
            2,
            2,
            vec![
                forest_source(0x08, 0, 0, 0x26),
                forest_source(0x08, 1, 0, 0x27),
                forest_source(0x08, 0, 1, 0x36),
                forest_source(0x08, 1, 1, 0x37),
            ],
        );
        let scattered_cells: Vec<_> = scattered.tiles.iter().collect();
        let scattered_geometry = GridGeometry {
            width: 2,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -8.0,
            origin_z: -8.0,
        };
        assert_eq!(
            complete_tree_placements(&scattered_cells, &scattered_geometry),
            vec![TreePlacement {
                column: 0,
                row: 0,
                width: 2,
                height: 2,
                ground_tile_index: 0x05,
                base_height: 0.0,
                rounded: false,
                outline_mask: false,
            }]
        );

        let mut border_sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                border_sources.push(forest_source(
                    0x05,
                    column,
                    row,
                    0x0c + u16::from(column) + u16::from(row) * 0x10,
                ));
            }
        }
        let border = frame(4, 4, border_sources);
        let border_cells: Vec<_> = border.tiles.iter().collect();
        let border_geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -16.0,
            origin_z: -16.0,
        };
        assert_eq!(
            complete_tree_placements(&border_cells, &border_geometry),
            vec![TreePlacement {
                column: 0,
                row: 0,
                width: 4,
                height: 4,
                ground_tile_index: 0x05,
                base_height: 0.0,
                rounded: false,
                outline_mask: false,
            }]
        );
    }

    #[test]
    fn mountain_corner_art_is_not_misclassified_as_a_tree() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                sources.push(source_with_tile(
                    0x6a,
                    column as u8,
                    row as u8,
                    if column < 2 { 0x20 + row as u16 } else { 0x3c },
                ));
            }
        }
        let frame = frame(4, 4, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -16.0,
            origin_z: -16.0,
        };
        assert!(complete_tree_placements(&cells, &geometry).is_empty());
    }

    #[test]
    fn clipped_profile_without_ground_evidence_stays_flat() {
        let mut tree = source_with_tile(0x32, 0, 0, 0x40);
        tree.tileset_id = Arc::from("kanto");
        let mesh = build_terrain_mesh(&frame(1, 1, vec![tree]))
            .expect("missing profile evidence should preserve the flat baseline");

        assert_eq!(mesh.textured.quad_count(), 1);
        assert_eq!(mesh.solid.quad_count(), 0);
        assert!(
            mesh.textured
                .positions
                .iter()
                .all(|position| position[1] == 0.0)
        );
    }

    #[test]
    fn grouped_mask_does_not_treat_internal_tile_seams_as_an_outer_boundary() {
        let width = SOURCE_TILE_PIXELS * 2;
        let height = SOURCE_TILE_PIXELS * 2;
        let mut equals_ground = vec![true; width * height];
        for y in 3..=12 {
            for x in 3..=12 {
                if x == 3 || x == 12 || y == 3 || y == 12 {
                    equals_ground[y * width + x] = false;
                }
            }
        }

        let removable = boundary_connected_mask(width, height, &equals_ground);
        assert!(removable[0], "open background must be removed");
        assert!(
            !removable[8 * width + 8],
            "enclosed face crossing the x=8/y=8 tile seams must remain"
        );
        assert!(!removable[3 * width + 3], "outline must remain");
    }

    #[test]
    fn shuffled_explicit_coordinates_preserve_source_cell_uvs() {
        let mut frame = frame(2, 1, vec![flat_source(), flat_source()]);
        frame.tiles.swap(0, 1);

        let mesh = build_terrain_mesh(&frame).expect("tile vector order is not spatial order");
        assert_eq!(mesh.textured.uvs[0], [0.0, 0.0]);
        assert_eq!(mesh.textured.uvs[4], [0.5, 0.0]);
    }
}
