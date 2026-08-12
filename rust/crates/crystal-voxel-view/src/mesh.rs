//! Pure authored-profile mesh construction for the optional voxel renderer.

#[path = "mesh/background.rs"]
mod background;
#[path = "mesh/ordinary_house.rs"]
mod ordinary_house;
#[path = "mesh/players_house.rs"]
mod players_house;
#[path = "mesh/traditional_house.rs"]
mod traditional_house;

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
    HOUSE_WALL_INSET_PIXELS, burned_tower_roof_style, lighthouse_depth_pixels, tin_tower_storeys,
    tin_tower_upper_source_x, uses_center_ridge_roof,
};
use crate::elevation::resolve_authored_mountain_tiers;
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
    // Blackthorn's closed `[6a 70 6b; 6c 72 6d]` drawing is a free-standing
    // mound, not part of the surrounding mountain datum graph. Claim the
    // complete drawing only; the same individual metatiles remain valid
    // cliff pieces everywhere else.
    for (column, row) in johto_closed_mound_origins(&cells, width, height) {
        for local_row in 0..8 {
            for local_column in 0..12 {
                shapes[(row + local_row) * width + column + local_column] = CellShape::Flat;
            }
        }
    }
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
        }
        | CellShape::FloatingRelief {
            ground_tile_index, ..
        } = *shape
            && !available_relief_base_tiles.contains(&ground_tile_index)
        {
            *shape = CellShape::Flat;
        }
    }

    resolve_authored_mountain_tiers(&mut shapes, width, height);

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
    if let Some(tileset) = cells.first().map(|tile| tile.source.tileset_id.as_ref()) {
        if crate::interior::has_back_wall(tileset) {
            append_room_back_wall(&mut mesh, &geometry, tileset);
        } else {
            background::append_repeating_background_apron(&mut mesh, &geometry, &cells, &shapes);
        }
    }
    // Cave faces are folds cut from the same continuous map drawing. Keep a
    // faithful copy of that drawing just below the modeled shelves so a fold,
    // diagonal corner, or clipped neighbor can never punch the clear color
    // through the cave floor. This is the connected-volume rule used by the
    // rock/cliff mesher: authored face art changes elevation, not topology.
    if cells
        .iter()
        .all(|tile| matches!(tile.source.tileset_id.as_ref(), "cave" | "dark_cave"))
    {
        append_top(
            &mut mesh.textured,
            [
                geometry.origin_x,
                geometry.origin_x + grid_width,
                geometry.origin_z,
                geometry.origin_z + grid_height,
            ],
            -0.02,
            (0.0, 1.0, 0.0, 1.0),
        );
    }
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
            CellShape::RampEast {
                west_height,
                east_height,
            } => (west_height + east_height) * 0.5 * frame.tile_size.y / SOURCE_TILE_HEIGHT,
            _ => support_height(&tile.source, frame.tile_size.y),
        })
        .collect();
    let mut claimed_by_building = vec![false; cell_count];
    let mut claimed_by_tree = vec![false; cell_count];
    let mut claimed_by_casino_stool = vec![false; cell_count];
    let mut claimed_by_house_furniture = vec![false; cell_count];
    traditional_house::append_north_wall_courses(
        &mut mesh,
        &cells,
        &geometry,
        &mut claimed_by_tree,
    )?;
    ordinary_house::append_north_wall_courses(&mut mesh, &cells, &geometry, &mut claimed_by_tree)?;
    players_house::append_north_wall_courses(&mut mesh, &cells, &geometry, &mut claimed_by_tree)?;
    append_house_stairs(
        &mut mesh,
        frame.map_id.as_ref(),
        &cells,
        &geometry,
        &mut claimed_by_tree,
    );
    append_facility_divider_network(
        &mut mesh,
        frame.map_id.as_ref(),
        &cells,
        &geometry,
        &mut claimed_by_tree,
    )?;
    append_rocket_base_wall_network(
        &mut mesh,
        frame.map_id.as_ref(),
        &cells,
        &geometry,
        &mut claimed_by_tree,
    );
    for (column, row) in ice_path_closed_rock_mass_origins(&cells, geometry.width, geometry.height)
    {
        append_ice_path_closed_rock_mass(
            &mut mesh,
            &cells,
            &geometry,
            column,
            row,
            &mut claimed_by_building,
        )?;
    }
    if let Some(images) = images {
        let placements = outdoor_building_placements(&cells, &geometry);
        for placement in &placements {
            let result = append_pixel_building(
                &mut mesh,
                images,
                &cells,
                &geometry,
                frame.map_id.as_ref(),
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
            if is_trapezoid_mound_placement(&cells, &geometry, *placement) {
                let height =
                    crate::cave::TRAPEZOID_MOUND_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;
                for row in placement.row..placement.row + placement.roof_rows {
                    for column in placement.column..placement.column + placement.width {
                        mesh.footing_heights[row * geometry.width + column] = height;
                    }
                }
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
        for placement in park_bench_placements(&cells, &geometry) {
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
        for placement in wise_trios_divider_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in kanto_round_barrier_placements(&cells, &geometry)
            .into_iter()
            .chain(kanto_round_path_barrier_placements(&cells, &geometry))
        {
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
        for placement in kanto_shoreline_round_barrier_placements(&cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                // A complete shoreline rock may enter the source halo before
                // a separate animated-water sample does. Leave just that
                // drawing on the faithful map plane; an optional prop must
                // never make the whole 2.5D frame disappear while scrolling.
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
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
        if frame.map_id.as_ref() == "VermilionGym" {
            for placement in vermilion_statue_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        if frame.map_id.as_ref() == "ViridianGym" {
            for placement in viridian_statue_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in pokecom_workstation_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in pokecom_plant_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in pokecom_chair_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in ship_stool_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in ship_rack_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in ship_barrel_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in train_station_seat_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in train_station_planter_placements(frame.map_id.as_ref(), &cells, &geometry)
        {
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
        for placement in train_station_gate_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        for placement in warehouse_crate_placements(frame.map_id.as_ref(), &cells, &geometry) {
            append_warehouse_crate(
                &mut mesh,
                &cells,
                &geometry,
                placement,
                &mut claimed_by_tree,
            )?;
        }
        for placement in house_plant_placements(&cells, &geometry) {
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
        for placement in house_upright_fixture_placements(&cells, &geometry) {
            if (placement.row..placement.row + placement.height).any(|row| {
                (placement.column..placement.column + placement.width)
                    .any(|column| claimed_by_tree[row * geometry.width + column])
            }) {
                continue;
            }
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
        for placement in house_bookcase_placements(&cells, &geometry) {
            append_house_bookcase(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            )?;
        }
        for placement in
            traditional_gift_shop_shelf_placements(frame.map_id.as_ref(), &cells, &geometry)
        {
            append_traditional_gift_shop_shelf(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            )?;
        }
        for placement in players_house_bookcase_placements(&cells, &geometry) {
            append_house_bookcase_with_ground(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
                crate::players_house::FLOOR_TILE,
            )?;
        }
        for placement in players_house_upright_fixture_placements(&cells, &geometry) {
            if (placement.row..placement.row + placement.height).any(|row| {
                (placement.column..placement.column + placement.width)
                    .any(|column| claimed_by_tree[row * geometry.width + column])
            }) {
                continue;
            }
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
        for placement in players_house_tv_placements(&cells, &geometry) {
            if (placement.row..placement.row + placement.height).any(|row| {
                (placement.column..placement.column + placement.width)
                    .any(|column| claimed_by_tree[row * geometry.width + column])
            }) {
                continue;
            }
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
        for placement in players_house_console_placements(&cells, &geometry) {
            if let Err(error) = append_shallow_top_group(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
                crate::players_house::FLOOR_TILE,
                3.0,
            ) && !matches!(error, TerrainMeshError::MissingGroundSample { .. })
            {
                return Err(error);
            }
        }
        for placement in players_house_bed_placements(&cells, &geometry) {
            append_player_bed_card(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            )?;
        }
        for placement in traditional_house_radio_placements(&cells, &geometry) {
            if (placement.row..placement.row + placement.height).any(|row| {
                (placement.column..placement.column + placement.width)
                    .any(|column| claimed_by_tree[row * geometry.width + column])
            }) {
                continue;
            }
            let mut placement = placement;
            placement.base_height = 4.0 * geometry.tile_height / SOURCE_TILE_HEIGHT;
            let mut overlay_claimed = vec![false; cells.len()];
            append_grouped_tree_overlay(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut overlay_claimed,
            )?;
        }
        for placement in traditional_house_cushion_placements(&cells, &geometry) {
            if let Err(error) = append_shallow_top_group(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
                crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
                2.0,
            ) && !matches!(error, TerrainMeshError::MissingGroundSample { .. })
            {
                return Err(error);
            }
        }
        if frame.map_id.as_ref() == "SoulHouse" {
            for placement in soul_house_bench_placements(&cells, &geometry) {
                if let Err(error) = append_shallow_top_group(
                    &mut mesh,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                    crate::house::HOUSE_FLOOR_TILE,
                    4.0,
                ) && !matches!(error, TerrainMeshError::MissingGroundSample { .. })
                {
                    return Err(error);
                }
            }
        }
        for placement in house_furniture_placements(&cells, &geometry) {
            append_house_furniture(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_house_furniture,
            )?;
        }
        for placement in house_table_placements(&cells, &geometry) {
            append_house_table(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_house_furniture,
            )?;
        }
        for mut placement in house_open_book_placements(&cells, &geometry) {
            placement.base_height = crate::house::FurnitureKind::Table.height()
                * geometry.tile_height
                / SOURCE_TILE_HEIGHT;
            let mut overlay_claimed = vec![false; cells.len()];
            append_grouped_tree_overlay(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut overlay_claimed,
            )?;
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
        if matches!(
            frame.map_id.as_ref(),
            "VioletGym" | "MahoganyGym" | "BlackthornGym1F"
        ) {
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
        if frame.map_id.as_ref() == "FuchsiaGym" {
            for placement in fuchsia_statue_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
        if frame.map_id.as_ref() == "CeladonGym" {
            for placement in celadon_statue_placements(frame.map_id.as_ref(), &cells, &geometry) {
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
            for placement in olivine_gym_statue_placements(frame.map_id.as_ref(), &cells, &geometry)
            {
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
        for placement in tower_statue_placements(&cells, &geometry) {
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
        if frame.map_id.as_ref() == "PewterGym" {
            for placement in tower_boulder_placements(&cells, &geometry) {
                if let Err(error) = append_grouped_tree(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                ) {
                    if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                        return Err(error);
                    }
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
            if let Err(error) = append_ice_path_small_boulder(
                &mut mesh,
                &cells,
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
        for placement in ice_path_edge_rock_placements(&cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
            }
        }
        for placement in ruins_statue_placements(&cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
            }
        }
        for placement in power_plant_plant_placements(frame.map_id.as_ref(), &cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
            }
        }
        for placement in rocket_base_plant_placements(frame.map_id.as_ref(), &cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
            }
        }
        if crate::casino::is_game_corner_map(frame.map_id.as_ref())
            || crate::cafe::is_cafe_map(frame.map_id.as_ref())
        {
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
        if crate::casino::is_game_corner_map(frame.map_id.as_ref()) {
            for placement in casino_slot_machine_placements(&cells, &geometry) {
                append_grouped_tree_scaled(
                    &mut mesh,
                    images,
                    &cells,
                    &shapes,
                    &geometry,
                    placement,
                    &mut claimed_by_tree,
                    1.6,
                )?;
            }
            for placement in casino_terminal_placements(&cells, &geometry) {
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
            for placement in casino_plant_placements(&cells, &geometry) {
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
        for placement in player_room_fixture_placements(&cells, &geometry) {
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
        for placement in player_room_pc_monitor_placements(&cells, &geometry) {
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
        for placement in player_room_pc_keyboard_placements(&cells, &geometry) {
            append_shallow_top_group(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
                0x02,
                3.0,
            )?;
        }
        for placement in player_bed_placements(&cells, &geometry) {
            append_player_bed_card(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            )?;
        }
        for placement in
            mr_pokemon_work_counter_placements(frame.map_id.as_ref(), &cells, &geometry)
        {
            if let Err(error) = append_shallow_top_group(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
                0x26,
                4.0,
            ) && !matches!(error, TerrainMeshError::MissingGroundSample { .. })
            {
                return Err(error);
            }
        }
        for placement in house_display_table_placements(&cells, &geometry) {
            if let Err(error) = append_shallow_top_group(
                &mut mesh,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
                crate::house::HOUSE_FLOOR_TILE,
                5.0,
            ) && !matches!(error, TerrainMeshError::MissingGroundSample { .. })
            {
                return Err(error);
            }
        }
        if cells
            .first()
            .is_some_and(|tile| tile.source.tileset_id.as_ref() == "players_room")
        {
            append_player_room_wall(&mut mesh, &cells, &shapes, &geometry, &mut claimed_by_tree)?;
        }
        for placement in pokecenter_pc_placements(frame.map_id.as_ref(), &cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
            }
        }
        for placement in mart_display_rack_placements(frame.map_id.as_ref(), &cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                // The expanded source halo can include a complete rack while
                // excluding the separate authored floor texel. Keep only
                // that rack as faithful flat art; an optional object must not
                // retire the entire 2.5D frame and produce movement flicker.
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
            }
        }
    }
    for placement in diagonal_cave_corner_placements(&cells, &geometry) {
        if let Err(error) = append_diagonal_cave_corner(
            &mut mesh,
            &cells,
            &shapes,
            &geometry,
            placement,
            &mut claimed_by_tree,
        ) {
            if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                return Err(error);
            }
        }
    }
    if let Some(images) = images {
        for placement in cave_small_rock_placements(&cells, &geometry) {
            if let Err(error) = append_grouped_tree(
                &mut mesh,
                images,
                &cells,
                &shapes,
                &geometry,
                placement,
                &mut claimed_by_tree,
            ) {
                if !matches!(error, TerrainMeshError::MissingGroundSample { .. }) {
                    return Err(error);
                }
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
                || claimed_by_house_furniture[index]
            {
                continue;
            }
            append_textured_cell(
                &mut mesh, &geometry, &cells, &shapes, &bank_runs, column, row, index, images,
            )?;
        }
    }

    append_cafe_table_pedestals(&mut mesh.solid, &cells, &geometry, frame.map_id.as_ref());

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
                || claimed_by_house_furniture[row * width + column]
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

fn wise_trios_divider_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::wise_trios::FLOOR_TILE,
        false,
        |source| crate::wise_trios::divider_local(map_id, source),
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

fn fuchsia_statue_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::fuchsia_gym::FLOOR_TILE,
        true,
        |source| {
            crate::fuchsia_gym::statue_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn celadon_statue_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::celadon_gym::STATUE_FLOOR_TILE,
        true,
        |source| {
            crate::celadon_gym::statue_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn vermilion_statue_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::vermilion_gym::FLOOR_TILE,
        true,
        |source| {
            crate::vermilion_gym::statue_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn viridian_statue_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::viridian_gym::FLOOR_TILE,
        true,
        |source| {
            crate::viridian_gym::statue_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn olivine_gym_boulder_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    rounded_object_placements(grouped_flat_card_placements(
        cells,
        geometry,
        crate::olivine_gym::GROUND_TILE,
        true,
        |source| crate::olivine_gym::boulder_local(source).map(|(column, row)| (column, row, 2, 2)),
    ))
}

fn olivine_gym_statue_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::olivine_gym::GROUND_TILE,
        true,
        |source| {
            crate::olivine_gym::statue_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn tower_boulder_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    rounded_object_placements(grouped_flat_card_placements(
        cells,
        geometry,
        crate::tower::TOWER_FLOOR_TILE,
        true,
        |source| crate::tower::boulder_local(source).map(|(column, row)| (column, row, 2, 2)),
    ))
}

fn tower_statue_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::tower::TOWER_FLOOR_TILE,
        true,
        |source| crate::tower::statue_local(source).map(|(column, row)| (column, row, 2, 2)),
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
    placements.extend(grouped_flat_card_placements(
        cells,
        geometry,
        crate::ice_path::CAVE_GROUND_TILE,
        true,
        |source| {
            crate::ice_path::boulder_local(source, crate::ice_path::BoulderBase::UpperRight)
                .map(|(column, row)| (column, row, 2, 2))
        },
    ));
    // Ice Path boulders are a face-on 16x16 drawing. Keep them as the same
    // cut-out standing cards used for cave props; a generated rounded hull
    // turns the four source tiles into a spiky basket and invents pixels the
    // map never supplied.
    placements
}

fn ruins_statue_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::ruins_of_alph::FLOOR_TILE,
        true,
        |source| {
            crate::ruins_of_alph::statue_local(source).map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn power_plant_plant_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::power_plant::FLOOR_TILE,
        true,
        |source| {
            crate::power_plant::plant_local(map_id, source).map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn rocket_base_plant_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::rocket_base::FLOOR_TILE,
        true,
        |source| {
            crate::rocket_base::plant_local(map_id, source).map(|(column, row)| (column, row, 2, 3))
        },
    )
}

fn rounded_object_placements(mut placements: Vec<TreePlacement>) -> Vec<TreePlacement> {
    for placement in &mut placements {
        placement.rounded = true;
    }
    placements
}

fn ice_path_edge_rock_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = Vec::new();
    for kind in [
        crate::ice_path::EdgeRockKind::Left,
        crate::ice_path::EdgeRockKind::Right,
        crate::ice_path::EdgeRockKind::Single,
    ] {
        placements.extend(grouped_flat_card_placements(
            cells,
            geometry,
            crate::ice_path::CAVE_GROUND_TILE,
            true,
            |source| {
                crate::ice_path::edge_rock_local(source).and_then(|(source_kind, column, row)| {
                    (source_kind == kind).then_some((column, row, 2, 2))
                })
            },
        ));
    }
    placements.sort_by_key(|placement| (placement.row, placement.column));
    placements
}

fn cave_small_rock_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, false, |source| {
        crate::cave::small_rock_local(source).map(|(column, row)| (column, row, 2, 2))
    })
    .into_iter()
    .map(|mut placement| {
        placement.base_height = crate::cave::CAVE_SHELF_HEIGHT;
        placement
    })
    .collect()
}

fn diagonal_cave_corner_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<DiagonalCaveCornerPlacement> {
    let mut placements = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let Some((corner, local_column, local_row)) =
            crate::cave::diagonal_corner_local(&cell.source)
        else {
            continue;
        };
        if local_column != 0 || local_row != 0 {
            continue;
        }
        let column = index % geometry.width;
        let row = index / geometry.width;
        if column + 2 > geometry.width || row + 2 > geometry.height {
            continue;
        }
        let complete = (0..2).all(|dy| {
            (0..2).all(|dx| {
                let source = &cells[(row + dy) * geometry.width + column + dx].source;
                crate::cave::diagonal_corner_local(source) == Some((corner, dx as u8, dy as u8))
            })
        });
        if complete {
            placements.push(DiagonalCaveCornerPlacement {
                column,
                row,
                corner,
            });
        }
    }
    placements
}

fn append_diagonal_cave_corner(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: DiagonalCaveCornerPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground_index = authored_surface_cell(cells, shapes, 0x16, None, 0.0, geometry.tile_height)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: 0x16,
        })?;
    for dy in 0..2 {
        for dx in 0..2 {
            let column = placement.column + dx;
            let row = placement.row + dy;
            claimed[row * geometry.width + column] = true;
            append_top(
                &mut mesh.textured,
                geometry.bounds(column, row).into(),
                0.0,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
        }
    }

    let (x0, _, z0, _) = geometry.bounds(placement.column, placement.row);
    let (_, x1, _, z1) = geometry.bounds(placement.column + 1, placement.row + 1);
    let height = crate::cave::CAVE_ROCK_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let (u0, _, v0, _) = geometry.uv(placement.column, placement.row);
    let (_, u1, _, v1) = geometry.uv(placement.column + 1, placement.row + 1);

    let bevel = geometry.tile_width * 0.5;
    let (cap, face, south, side, side_normal) = match placement.corner {
        crate::cave::DiagonalCorner::SouthEast => (
            [
                ([x1, height, z0], [u1, v0]),
                ([x1, height, z1], [u1, v1]),
                ([x0, height, z1], [u0, v1]),
            ],
            [
                [x1 - bevel, 0.0, z0 - bevel],
                [x1, height, z0],
                [x0, height, z1],
                [x0 - bevel, 0.0, z1 - bevel],
            ],
            [
                [x0 - bevel, 0.0, z1 - bevel],
                [x0, height, z1],
                [x1, height, z1],
                [x1, 0.0, z1],
            ],
            [
                [x1, 0.0, z1],
                [x1, height, z1],
                [x1, height, z0],
                [x1 - bevel, 0.0, z0 - bevel],
            ],
            [1.0, 0.0, 0.0],
        ),
        crate::cave::DiagonalCorner::SouthWest => (
            [
                ([x0, height, z0], [u0, v0]),
                ([x1, height, z1], [u1, v1]),
                ([x0, height, z1], [u0, v1]),
            ],
            [
                [x0 + bevel, 0.0, z0 - bevel],
                [x1 + bevel, 0.0, z1 - bevel],
                [x1, height, z1],
                [x0, height, z0],
            ],
            [
                [x0, 0.0, z1],
                [x0, height, z1],
                [x1, height, z1],
                [x1 + bevel, 0.0, z1 - bevel],
            ],
            [
                [x0 + bevel, 0.0, z0 - bevel],
                [x0, height, z0],
                [x0, height, z1],
                [x0, 0.0, z1],
            ],
            [-1.0, 0.0, 0.0],
        ),
    };
    append_polygon(&mut mesh.textured, &cap, [0.0, 1.0, 0.0], BANK_TOP_SHADE);
    let edge_a = Vec3::from_array(face[1]) - Vec3::from_array(face[0]);
    let edge_b = Vec3::from_array(face[2]) - Vec3::from_array(face[0]);
    append_quad(
        &mut mesh.textured,
        face,
        edge_a.cross(edge_b).normalize().to_array(),
        [[u0, v1], [u0, v0], [u1, v0], [u1, v1]],
        TEXTURED_SHADE,
    );
    // Close the diagonal with one-pixel strips from the live authored art.
    // A fixed solid color ignored the active cave palette and appeared as a
    // tan flyaway triangle at exposed corners.
    let pixel_u = (u1 - u0) / (2 * SOURCE_TILE_PIXELS) as f32;
    let pixel_v = (v1 - v0) / (2 * SOURCE_TILE_PIXELS) as f32;
    append_quad(
        &mut mesh.textured,
        south,
        [0.0, 0.0, 1.0],
        [[u0, v1], [u0, v1 - pixel_v], [u1, v1 - pixel_v], [u1, v1]],
        TEXTURED_SHADE,
    );
    let side_uv = match placement.corner {
        crate::cave::DiagonalCorner::SouthEast => {
            [[u1 - pixel_u, v1], [u1 - pixel_u, v0], [u1, v0], [u1, v1]]
        }
        crate::cave::DiagonalCorner::SouthWest => {
            [[u0 + pixel_u, v0], [u0, v0], [u0, v1], [u0 + pixel_u, v1]]
        }
    };
    append_quad(
        &mut mesh.textured,
        side,
        side_normal,
        side_uv,
        TEXTURED_SHADE,
    );
    Ok(())
}

fn casino_terminal_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, true, |source| {
        crate::casino::terminal_local(source).map(|(column, row)| (column, row, 2, 2))
    })
}

fn casino_slot_machine_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, false, |source| {
        crate::casino::slot_machine_local(source).map(|(column, row)| (column, row, 2, 2))
    })
    .into_iter()
    .map(|mut placement| {
        placement.remove_all_ground = true;
        placement
    })
    .collect()
}

fn casino_plant_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, true, |source| {
        crate::casino::plant_local(source).map(|(column, row)| (column, row, 2, 4))
    })
}

fn player_room_fixture_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x02, true, |source| {
        crate::interior::player_room_fixture_group(source).map(|(column, row, width, height)| {
            (column, row, usize::from(width), usize::from(height))
        })
    })
}

fn player_room_pc_monitor_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x02, true, |source| {
        crate::interior::player_room_pc_monitor_local(source)
            .map(|(column, row)| (column, row, 2, 2))
    })
}

fn player_room_pc_keyboard_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x02, false, |source| {
        crate::interior::player_room_pc_keyboard_local(source)
            .map(|(column, row)| (column, row, 2, 1))
    })
}

fn player_bed_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, false, |source| {
        crate::interior::player_bed_local(source).map(|(column, row)| (column, row, 2, 4))
    })
}

fn mr_pokemon_work_counter_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x26, false, |source| {
        crate::interior::mr_pokemon_work_counter_local(map_id, source)
            .map(|(column, row)| (column, row, 4, 2))
    })
}

fn house_display_table_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::HOUSE_FLOOR_TILE,
        false,
        |source| crate::house::display_table_local(source).map(|(column, row)| (column, row, 2, 2)),
    )
}

fn append_player_bed_card(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    if mesh.footing_heights.len() < cells.len() {
        mesh.footing_heights.resize(cells.len(), 0.0);
    }
    let ground_index = authored_surface_cell(cells, shapes, 0x01, None, 0.0, geometry.tile_height)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: 0x01,
        })?;
    // Bedroom beds are drawn from above.  Keep the complete mattress art on
    // one shallow horizontal surface; pitching the four source rows made the
    // bed read as a loose board leaning out of the room.
    let bed_height = 7.0 * geometry.tile_height / SOURCE_TILE_HEIGHT;
    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            let index = row * geometry.width + column;
            claimed[index] = true;
            mesh.footing_heights[index] = bed_height;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                bed_height,
                geometry.uv(column, row),
            );
        }
    }

    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    let z0 = geometry.origin_z + placement.row as f32 * geometry.tile_height;
    let z1 = z0 + placement.height as f32 * geometry.tile_height;
    for (vertices, normal, direction) in [
        (
            [
                [x0, 0.0, z1],
                [x0, bed_height, z1],
                [x0, bed_height, z0],
                [x0, 0.0, z0],
            ],
            [-1.0, 0.0, 0.0],
            Direction::West,
        ),
        (
            [
                [x1, 0.0, z0],
                [x1, bed_height, z0],
                [x1, bed_height, z1],
                [x1, 0.0, z1],
            ],
            [1.0, 0.0, 0.0],
            Direction::East,
        ),
        (
            [
                [x1, 0.0, z0],
                [x0, 0.0, z0],
                [x0, bed_height, z0],
                [x1, bed_height, z0],
            ],
            [0.0, 0.0, -1.0],
            Direction::North,
        ),
    ] {
        append_solid_quad(
            &mut mesh.solid,
            vertices,
            normal,
            solid_color(SolidKind::Prop, direction),
        );
    }

    // A bed's visible foot is the lower lip of the source drawing, not a
    // generated neutral wall. Crop the last seven source-pixel rows from the
    // southern source cells and stand those rows up exactly once.
    let source_row = placement.row + placement.height - 1;
    for local_column in 0..placement.width {
        let column = placement.column + local_column;
        let cell_x0 = geometry.origin_x + column as f32 * geometry.tile_width;
        let cell_x1 = cell_x0 + geometry.tile_width;
        let (u0, u1, v0, v1) = geometry.uv(column, source_row);
        let cropped_v0 = v0 + (v1 - v0) / SOURCE_TILE_PIXELS as f32;
        append_quad(
            &mut mesh.textured,
            [
                [cell_x0, 0.0, z1],
                [cell_x1, 0.0, z1],
                [cell_x1, bed_height, z1],
                [cell_x0, bed_height, z1],
            ],
            [0.0, 0.0, 1.0],
            [[u0, v1], [u1, v1], [u1, cropped_v0], [u0, cropped_v0]],
            TEXTURED_SHADE,
        );
    }
    Ok(())
}

fn append_player_room_wall(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    const WALL_BANDS: usize = 4;
    // The visual grid includes a camera halo before map row zero. Find the
    // first authored room-block row instead of treating that halo boundary as
    // the wall seam. Player's Room's north wall is one complete 4x4 metatile
    // course. Its top collision quadrants are WALL and its bottom quadrants
    // are FLOOR, so only the first two source rows belong on the vertical
    // plane; folding all four rows visibly turns the striped floor upright.
    let wall_origin = (0..geometry.height).find_map(|row| {
        (0..=geometry.width.saturating_sub(16)).find_map(|column| {
            (0..16)
                .all(|offset| {
                    let source = &cells[row * geometry.width + column + offset].source;
                    source.metatile_id != 0
                        && usize::from(source.subtile_column) == offset % 4
                        && source.subtile_row == 0
                })
                .then_some((column, row))
        })
    });
    let (wall_start_column, wall_start_row) =
        wall_origin.ok_or(TerrainMeshError::MissingGroundSample {
            column: 0,
            row: 0,
            tile_index: 0x01,
        })?;
    let seam_row = (wall_start_row + 2).min(geometry.height);
    // Wall-mounted art and the staircase card terminate at this same authored
    // seam. Recess the architectural plane slightly so those independent
    // cards remain visible instead of z-fighting with or disappearing behind
    // the wall.
    let plane_z =
        geometry.origin_z + seam_row as f32 * geometry.tile_height - geometry.tile_height * 0.08;
    let clean_wall_samples = std::array::from_fn::<_, 4, _>(|subtile_column| {
        cells.iter().position(|tile| {
            tile.source.metatile_id == 0x04
                && usize::from(tile.source.subtile_column) == subtile_column
                && tile.source.subtile_row == 0
        })
    });
    for row in 0..wall_start_row {
        for column in 0..geometry.width {
            claimed[row * geometry.width + column] = true;
        }
    }
    let room_columns = wall_start_column..wall_start_column + 16;
    for column in room_columns {
        let x0 = geometry.origin_x + column as f32 * geometry.tile_width;
        let x1 = x0 + geometry.tile_width;
        for row in wall_start_row..seam_row {
            let index = row * geometry.width + column;
            let already_claimed = claimed[index];
            // Furniture cards already own their pixels. The remaining cells
            // are the actual authored wall artwork and must be lifted from the
            // floor, not replaced with a synthetic panel texture.
            if already_claimed || shapes[index] != CellShape::Flat {
                continue;
            }
            claimed[index] = true;
        }
        // Continue the authored wall pattern only to the same four-tile
        // architectural height used by the other house interiors. Repeating
        // this strip ten times produced an 80px tower behind the player's
        // bedroom even though Crystal draws the same compact room course as
        // the neighboring fixtures.
        for band in 0..WALL_BANDS {
            let Some(sample_index) = clean_wall_samples[column % 4] else {
                continue;
            };
            let source_column = sample_index % geometry.width;
            let source_row = sample_index / geometry.width;
            let y0 = band as f32 * geometry.tile_height;
            let y1 = y0 + geometry.tile_height;
            let (u0, u1, v0, v1) = geometry.uv(source_column, source_row);
            append_quad(
                &mut mesh.textured,
                [
                    [x1, y0, plane_z],
                    [x1, y1, plane_z],
                    [x0, y1, plane_z],
                    [x0, y0, plane_z],
                ],
                [0.0, 0.0, 1.0],
                [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                TEXTURED_SHADE,
            );
        }
    }
    Ok(())
}

fn pokecom_workstation_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::pokecom::WORKSTATION_FLOOR_TILE,
        true,
        |source| {
            crate::pokecom::workstation_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 3))
        },
    )
}

fn pokecom_plant_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::pokecom::PLANT_FLOOR_TILE,
        true,
        |source| {
            crate::pokecom::plant_local(map_id, source).map(|(column, row)| (column, row, 2, 3))
        },
    )
}

fn pokecom_chair_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::pokecom::PLANT_FLOOR_TILE,
        true,
        |source| {
            crate::pokecom::chair_local(map_id, source).map(|(column, row)| (column, row, 2, 2))
        },
    )
}

fn train_station_seat_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::train_station::FLOOR_TILE,
        true,
        |source| {
            crate::train_station::seat_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 2))
        },
    )
}

fn ship_stool_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::ship::CABIN_FLOOR_TILE,
        true,
        |source| crate::ship::stool_local(map_id, source).map(|(column, row)| (column, row, 2, 2)),
    )
}

fn ship_rack_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::ship::CABIN_FLOOR_TILE,
        false,
        |source| crate::ship::rack_local(map_id, source).map(|(column, row)| (column, row, 2, 4)),
    )
}

fn park_bench_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::park::PARK_PLAZA_GROUND_TILE,
        false,
        |source| crate::park::bench_local(source).map(|(column, row)| (column, row, 4, 2)),
    )
}

fn kanto_round_barrier_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x2c, true, |source| {
        crate::kanto_cliff::round_barrier_local(source).map(|(column, row)| (column, row, 2, 2))
    })
}

fn kanto_round_path_barrier_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x11, true, |source| {
        crate::kanto_cliff::round_path_barrier_local(source)
            .map(|(column, row)| (column, row, 2, 2))
    })
}

fn kanto_shoreline_round_barrier_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x14, true, |source| {
        crate::kanto_cliff::shoreline_round_barrier_local(source)
            .map(|(column, row)| (column, row, 2, 2))
    })
    .into_iter()
    .map(|mut placement| {
        placement.base_height = crate::profile::WATER_HEIGHT;
        placement
    })
    .collect()
}

fn ship_barrel_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::ship::CABIN_FLOOR_TILE,
        true,
        |source| crate::ship::barrel_local(map_id, source).map(|(column, row)| (column, row, 2, 2)),
    )
}

fn train_station_planter_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::train_station::FLOOR_TILE,
        true,
        |source| {
            crate::train_station::planter_local(map_id, source)
                .map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn train_station_gate_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x3e, true, |source| {
        crate::train_station::gate_local(map_id, source).map(|(column, row)| (column, row, 2, 4))
    })
}

fn warehouse_crate_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let Some(floor_tile) = crate::warehouse::floor_tile(map_id) else {
        return Vec::new();
    };
    grouped_flat_card_placements(cells, geometry, floor_tile, false, |source| {
        crate::warehouse::crate_local(map_id, source).map(|(column, row)| (column, row, 2, 2))
    })
}

fn append_warehouse_crate(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground = cells
        .iter()
        .position(|tile| tile.source.tile_index == placement.ground_tile_index)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: placement.ground_tile_index,
        })?;
    let ground_uv = geometry.uv(ground % geometry.width, ground / geometry.width);
    let first = geometry.bounds(placement.column, placement.row);
    let last = geometry.bounds(placement.column + 1, placement.row);
    let x0 = first.0;
    let x1 = last.1;
    let rear_z = first.2;
    let front_z = first.3;
    let height = crate::warehouse::CRATE_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;

    for row in placement.row..placement.row + 2 {
        for column in placement.column..placement.column + 2 {
            let index = row * geometry.width + column;
            claimed[index] = true;
            let bounds = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [bounds.0, bounds.1, bounds.2, bounds.3],
                0.0,
                ground_uv,
            );
        }
    }
    for local_column in 0..2 {
        let column = placement.column + local_column;
        let bounds = geometry.bounds(column, placement.row);
        append_top_shaded(
            &mut mesh.textured,
            [bounds.0, bounds.1, rear_z, front_z],
            height,
            geometry.uv(column, placement.row),
            [0.94, 0.94, 0.94, 1.0],
        );
        let face_uv = geometry.uv(column, placement.row + 1);
        append_quad(
            &mut mesh.textured,
            [
                [bounds.0, 0.0, front_z],
                [bounds.1, 0.0, front_z],
                [bounds.1, height, front_z],
                [bounds.0, height, front_z],
            ],
            [0.0, 0.0, 1.0],
            [
                [face_uv.0, face_uv.3],
                [face_uv.1, face_uv.3],
                [face_uv.1, face_uv.2],
                [face_uv.0, face_uv.2],
            ],
            TEXTURED_SHADE,
        );
    }
    let left_uv = geometry.uv(placement.column, placement.row + 1);
    let right_uv = geometry.uv(placement.column + 1, placement.row + 1);
    append_quad(
        &mut mesh.textured,
        [
            [x0, 0.0, rear_z],
            [x0, 0.0, front_z],
            [x0, height, front_z],
            [x0, height, rear_z],
        ],
        [-1.0, 0.0, 0.0],
        [
            [left_uv.0, left_uv.3],
            [left_uv.1, left_uv.3],
            [left_uv.1, left_uv.2],
            [left_uv.0, left_uv.2],
        ],
        [0.78, 0.78, 0.78, 1.0],
    );
    append_quad(
        &mut mesh.textured,
        [
            [x1, 0.0, front_z],
            [x1, 0.0, rear_z],
            [x1, height, rear_z],
            [x1, height, front_z],
        ],
        [1.0, 0.0, 0.0],
        [
            [right_uv.0, right_uv.3],
            [right_uv.1, right_uv.3],
            [right_uv.1, right_uv.2],
            [right_uv.0, right_uv.2],
        ],
        [0.84, 0.84, 0.84, 1.0],
    );
    Ok(())
}

fn house_plant_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::HOUSE_FLOOR_TILE,
        true,
        |source| crate::house::plant_local(source).map(|(column, row)| (column, row, 2, 4)),
    )
}

fn house_upright_fixture_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::HOUSE_FLOOR_TILE,
        false,
        crate::house::upright_fixture_local,
    );
    // These fixtures occupy their complete rectangular drawings. Remove only
    // exact floor-colored pixels; boundary-color flooding would eat cabinet
    // corners that legitimately share a palette shade with the room.
    for placement in &mut placements {
        placement.remove_all_ground = true;
        placement.card_thickness = geometry.tile_height * 0.125;
    }
    placements
}

fn house_bookcase_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::HOUSE_FLOOR_TILE,
        false,
        |source| crate::house::bookcase_local(source).map(|(column, row)| (column, row, 2, 4)),
    )
}

fn traditional_gift_shop_shelf_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
        false,
        |source| {
            crate::house::traditional_gift_shop_shelf_local(map_id, source)
                .map(|(column, row)| (column, row, 4, 4))
        },
    )
}

fn players_house_upright_fixture_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = grouped_flat_card_placements(
        cells,
        geometry,
        0x11,
        false,
        crate::players_house::upright_fixture_local,
    );
    for placement in &mut placements {
        placement.remove_all_ground = true;
        placement.card_thickness = geometry.tile_height * 0.125;
    }
    placements
}

fn players_house_bookcase_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::players_house::FLOOR_TILE,
        false,
        |source| {
            crate::players_house::bookcase_local(source).map(|(column, row)| (column, row, 2, 4))
        },
    )
}

fn players_house_tv_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = grouped_flat_card_placements(cells, geometry, 0x01, false, |source| {
        crate::players_house::tv_local(source).map(|(column, row)| (column, row, 2, 2))
    });
    for placement in &mut placements {
        placement.remove_all_ground = true;
        placement.card_thickness = geometry.tile_height * 0.125;
    }
    placements
}

fn players_house_console_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, false, |source| {
        crate::players_house::console_local(source).map(|(column, row)| (column, row, 2, 2))
    })
}

fn players_house_bed_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(cells, geometry, 0x01, false, |source| {
        crate::players_house::bed_local(source).map(|(column, row)| (column, row, 2, 4))
    })
}

fn traditional_house_radio_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
        false,
        |source| {
            crate::house::traditional_radio_local(source).map(|(column, row)| (column, row, 2, 2))
        },
    );
    for placement in &mut placements {
        placement.card_thickness = geometry.tile_height * 0.125;
    }
    placements
}

fn traditional_house_cushion_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
        false,
        |source| {
            crate::house::traditional_cushion_local(source).map(|(column, row)| (column, row, 2, 2))
        },
    )
}

fn soul_house_bench_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::HOUSE_FLOOR_TILE,
        false,
        |source| {
            crate::house::soul_house_bench_local(source).map(|(column, row)| (column, row, 2, 2))
        },
    )
}

fn house_open_book_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::house::HOUSE_FLOOR_TILE,
        false,
        |source| crate::house::open_book_local(source).map(|(column, row)| (column, row, 2, 2)),
    )
}

fn house_stair_local(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<(u8, u8, crate::players_house::StairKind)> {
    crate::interior::trainer_house_b1f_stair_local(map_id, source)
        .map(|(column, row)| (column, row, crate::players_house::StairKind::UpEast))
        .or_else(|| crate::house::wise_trios_stair_local(map_id, source))
        .or_else(|| crate::players_house::stair_local(source))
        .or_else(|| crate::house::stair_local(source))
        .or_else(|| crate::interior::player_room_stair_local(source))
}

fn append_house_stairs(
    mesh: &mut TerrainMeshData,
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
) {
    for row in 0..geometry.height.saturating_sub(1) {
        for column in 0..geometry.width.saturating_sub(1) {
            let kind = house_stair_local(map_id, &cells[row * geometry.width + column].source)
                .and_then(|(local_column, local_row, kind)| {
                    (local_column == 0 && local_row == 0).then_some(kind)
                });
            let Some(kind) = kind else {
                continue;
            };
            let complete = (0..2).all(|local_row| {
                (0..2).all(|local_column| {
                    house_stair_local(
                        map_id,
                        &cells[(row + local_row) * geometry.width + column + local_column].source,
                    ) == Some((local_column as u8, local_row as u8, kind))
                })
            });
            if !complete {
                continue;
            }
            // This facility flight is an above-floor stair, not a sunken
            // stairwell. Preserve the authored checker floor beneath its
            // four cells so perspective cannot expose a black rectangular
            // hole between/behind the rising treads.
            if map_id == "TrainerHouseB1F" {
                if let Some(ground_index) = cells.iter().position(|tile| {
                    tile.source.tileset_id.as_ref() == "facility" && tile.source.tile_index == 0x26
                }) {
                    let ground_uv =
                        geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
                    for local_row in 0..2 {
                        for local_column in 0..2 {
                            let (x0, x1, z0, z1) =
                                geometry.bounds(column + local_column, row + local_row);
                            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
                        }
                    }
                }
            }
            let step_count = if map_id == "TrainerHouseB1F" { 8 } else { 4 };
            let strips_per_tile = step_count / 2;
            let step_width = geometry.tile_width * 2.0 / step_count as f32;
            let z0 = geometry.origin_z + row as f32 * geometry.tile_height;
            for step in 0..step_count {
                let source_column = column + step / strips_per_tile;
                let source_strip = step % strips_per_tile;
                let x0 = geometry.origin_x
                    + column as f32 * geometry.tile_width
                    + step as f32 * step_width;
                let x1 = x0 + step_width;
                let height = match kind {
                    crate::players_house::StairKind::UpEast => {
                        (step + 1) as f32 * geometry.tile_height * 2.0 / step_count as f32
                    }
                    crate::players_house::StairKind::DownWest => {
                        (step as f32 - (step_count - 1) as f32) * geometry.tile_height * 2.0
                            / step_count as f32
                    }
                };
                for local_row in 0..2 {
                    let source_row = row + local_row;
                    let tile_z0 = z0 + local_row as f32 * geometry.tile_height;
                    let tile_z1 = tile_z0 + geometry.tile_height;
                    let (u0, u1, v0, v1) = geometry.uv(source_column, source_row);
                    let strip_u0 = u0 + (u1 - u0) * source_strip as f32 / strips_per_tile as f32;
                    let strip_u1 =
                        u0 + (u1 - u0) * (source_strip + 1) as f32 / strips_per_tile as f32;
                    let step_uv = (strip_u0, strip_u1, v0, v1);
                    append_top(
                        &mut mesh.textured,
                        [x0, x1, tile_z0, tile_z1],
                        height,
                        step_uv,
                    );
                    let previous_height = height - geometry.tile_height * 2.0 / step_count as f32;
                    append_solid_quad(
                        &mut mesh.solid,
                        [
                            [x0, previous_height, tile_z1],
                            [x0, height, tile_z1],
                            [x0, height, tile_z0],
                            [x0, previous_height, tile_z0],
                        ],
                        [-1.0, 0.0, 0.0],
                        solid_color(SolidKind::Bank, Direction::West),
                    );
                }
            }
            for local_row in 0..2 {
                for local_column in 0..2 {
                    claimed[(row + local_row) * geometry.width + column + local_column] = true;
                }
            }
        }
    }
}

fn house_furniture_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<HouseFurniturePlacement> {
    let mut placements = Vec::new();
    for (index, tile) in cells.iter().enumerate() {
        let Some((0, 0, kind @ crate::house::FurnitureKind::Stool)) =
            crate::house::furniture_local(&tile.source)
        else {
            continue;
        };
        let column = index % geometry.width;
        let row = index / geometry.width;
        if column + 1 >= geometry.width || row + 1 >= geometry.height {
            continue;
        }
        let complete = (0..2).all(|local_row| {
            (0..2).all(|local_column| {
                let candidate = cells[(row + local_row) * geometry.width + column + local_column];
                crate::house::furniture_local(&candidate.source)
                    == Some((local_column as u8, local_row as u8, kind))
            })
        });
        if complete {
            placements.push(HouseFurniturePlacement { column, row, kind });
        }
    }
    placements.sort_by_key(|placement| (placement.row, placement.column));
    placements
}

fn house_table_placements(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<HouseTablePlacement> {
    const HOUSE_DRAWING: [[u16; 4]; 4] = [
        [0x26, 0x27, 0x27, 0x29],
        [0x36, 0x2f, 0x2f, 0x39],
        [0x36, 0x2f, 0x2f, 0x39],
        [0x3c, 0x3a, 0x3a, 0x3b],
    ];
    const PLAYER_FAMILY_DRAWING: [[u16; 4]; 4] = [
        [0x23, 0x22, 0x22, 0x24],
        [0x25, 0x15, 0x15, 0x35],
        [0x25, 0x15, 0x15, 0x35],
        [0x33, 0x32, 0x32, 0x34],
    ];
    let mut placements = Vec::new();
    for row in 0..geometry.height.saturating_sub(3) {
        for column in 0..geometry.width.saturating_sub(3) {
            if (0..4).all(|local_row| {
                (0..4).all(|local_column| {
                    let tile = cells[(row + local_row) * geometry.width + column + local_column];
                    let drawing = match tile.source.tileset_id.as_ref() {
                        "house" => HOUSE_DRAWING,
                        "players_house" => PLAYER_FAMILY_DRAWING,
                        _ => return false,
                    };
                    tile.source.tile_index == drawing[local_row][local_column]
                })
            }) {
                placements.push(HouseTablePlacement {
                    column,
                    row,
                    ground_tile_index: crate::house::HOUSE_FLOOR_TILE,
                    height_pixels: crate::house::FurnitureKind::Table.height(),
                });
            }
        }
    }
    const TRADITIONAL_DRAWING: [[u16; 4]; 4] = [
        [0x23, 0x22, 0x22, 0x24],
        [0x42, 0x15, 0x15, 0x43],
        [0x42, 0x15, 0x15, 0x43],
        [0x33, 0x32, 0x32, 0x34],
    ];
    for row in 0..geometry.height.saturating_sub(3) {
        for column in 0..geometry.width.saturating_sub(3) {
            if (0..4).all(|local_row| {
                (0..4).all(|local_column| {
                    let tile = cells[(row + local_row) * geometry.width + column + local_column];
                    tile.source.tileset_id.as_ref() == "traditional_house"
                        && tile.source.tile_index == TRADITIONAL_DRAWING[local_row][local_column]
                })
            }) {
                placements.push(HouseTablePlacement {
                    column,
                    row,
                    ground_tile_index: crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
                    height_pixels: 4.0,
                });
            }
        }
    }
    placements
}

fn mart_display_rack_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    let mut placements = grouped_flat_card_placements(
        cells,
        geometry,
        crate::mart::ground_tile_for_map(map_id),
        false,
        |source| crate::mart::display_rack_local(source).map(|(column, row)| (column, row, 2, 4)),
    );
    for placement in &mut placements {
        placement.remove_all_ground = true;
    }
    placements
}

fn pokecenter_pc_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<TreePlacement> {
    grouped_flat_card_placements(
        cells,
        geometry,
        crate::pokecenter::FLOOR_TILE,
        true,
        |source| {
            crate::pokecenter::pc_local(map_id, source).map(|(column, row)| (column, row, 2, 2))
        },
    )
}

fn append_cafe_table_pedestals(
    mesh: &mut SurfaceMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    map_id: &str,
) {
    for tile in cells {
        let source = &tile.source;
        if source.subtile_column != 0
            || source.subtile_row != 0
            || !crate::cafe::is_table(map_id, source)
        {
            continue;
        }
        let column = tile.column as usize;
        let row = tile.row as usize;
        if column + 4 > geometry.width || row + 4 > geometry.height {
            continue;
        }
        let x0 = geometry.origin_x + (column as f32 + 1.5) * geometry.tile_width;
        let x1 = geometry.origin_x + (column as f32 + 2.5) * geometry.tile_width;
        let z0 = geometry.origin_z + (row as f32 + 1.5) * geometry.tile_height;
        let z1 = geometry.origin_z + (row as f32 + 2.5) * geometry.tile_height;
        let y1 = crate::cafe::TABLETOP_BOTTOM;
        for (positions, normal, direction) in [
            (
                [[x0, 0.0, z1], [x0, y1, z1], [x0, y1, z0], [x0, 0.0, z0]],
                [-1.0, 0.0, 0.0],
                Direction::West,
            ),
            (
                [[x1, 0.0, z0], [x1, y1, z0], [x1, y1, z1], [x1, 0.0, z1]],
                [1.0, 0.0, 0.0],
                Direction::East,
            ),
            (
                [[x0, 0.0, z0], [x0, y1, z0], [x1, y1, z0], [x1, 0.0, z0]],
                [0.0, 0.0, -1.0],
                Direction::North,
            ),
            (
                [[x1, 0.0, z1], [x1, y1, z1], [x0, y1, z1], [x0, 0.0, z1]],
                [0.0, 0.0, 1.0],
                Direction::South,
            ),
        ] {
            append_solid_quad(
                mesh,
                positions,
                normal,
                solid_color(SolidKind::Prop, direction),
            );
        }
    }
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
                ground_metatile_id: None,
                base_height: 0.0,
                rounded: false,
                outline_mask,
                remove_all_ground: false,
                card_thickness: 0.0,
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
    ground_metatile_id: Option<u16>,
    base_height: f32,
    rounded: bool,
    outline_mask: bool,
    remove_all_ground: bool,
    /// Shallow depth behind an authored upright drawing. Most cards stay
    /// paper-thin; house appliances opt into a small casing without changing
    /// or stretching their exact live front art.
    card_thickness: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CasinoStoolPlacement {
    column: usize,
    row: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HouseFurniturePlacement {
    column: usize,
    row: usize,
    kind: crate::house::FurnitureKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HouseTablePlacement {
    column: usize,
    row: usize,
    ground_tile_index: u16,
    height_pixels: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DiagonalCaveCornerPlacement {
    column: usize,
    row: usize,
    corner: crate::cave::DiagonalCorner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FacilityDividerPlacement {
    column: usize,
    row: usize,
    width: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FacilityVerticalDividerPlacement {
    column: usize,
    row: usize,
    height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FacilityDividerCapPlacement {
    column: usize,
    row: usize,
    width: usize,
}

fn facility_divider_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<FacilityDividerPlacement> {
    if !crate::facility_divider::supports_map(map_id) || geometry.height < 2 {
        return Vec::new();
    }
    let is_complete_wall_column = |column: usize, row: usize| {
        let top = cells[row * geometry.width + column];
        let face = cells[(row + 1) * geometry.width + column];
        crate::facility_divider::is_horizontal_top(map_id, &top.source)
            && crate::facility_divider::is_horizontal_face(map_id, &face.source)
            && crate::facility_divider::horizontal_pair(&top.source, &face.source)
    };
    let mut placements = Vec::new();
    for row in 0..geometry.height - 1 {
        let mut column = 0;
        while column < geometry.width {
            if !is_complete_wall_column(column, row) {
                column += 1;
                continue;
            }
            let start = column;
            while column < geometry.width && is_complete_wall_column(column, row) {
                column += 1;
            }
            placements.push(FacilityDividerPlacement {
                column: start,
                row,
                width: column - start,
            });
        }
    }
    placements
}

fn facility_vertical_divider_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<FacilityVerticalDividerPlacement> {
    if !crate::facility_divider::supports_map(map_id) || geometry.width < 2 {
        return Vec::new();
    }
    let is_complete_wall_row = |column: usize, row: usize| {
        let left = cells[row * geometry.width + column];
        let right = cells[row * geometry.width + column + 1];
        crate::facility_divider::is_vertical_left(map_id, &left.source)
            && crate::facility_divider::is_vertical_right(map_id, &right.source)
    };
    let mut placements = Vec::new();
    for column in 0..geometry.width - 1 {
        let mut row = 0;
        while row < geometry.height {
            if !is_complete_wall_row(column, row) {
                row += 1;
                continue;
            }
            let start = row;
            while row < geometry.height && is_complete_wall_row(column, row) {
                row += 1;
            }
            placements.push(FacilityVerticalDividerPlacement {
                column,
                row: start,
                height: row - start,
            });
        }
    }
    placements
}

fn facility_divider_cap_placements(
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<FacilityDividerCapPlacement> {
    if !crate::facility_divider::supports_map(map_id) || geometry.width < 2 {
        return Vec::new();
    }
    let is_cap_pair = |column: usize, row: usize| {
        let left = cells[row * geometry.width + column];
        let right = cells[row * geometry.width + column + 1];
        left.source.tile_index == 0x40
            && right.source.tile_index == 0x42
            && crate::facility_divider::is_horizontal_top(map_id, &left.source)
            && crate::facility_divider::is_horizontal_top(map_id, &right.source)
            && (row + 1 >= geometry.height
                || !crate::facility_divider::horizontal_pair(
                    &left.source,
                    &cells[(row + 1) * geometry.width + column].source,
                ))
    };
    let mut placements = Vec::new();
    for row in 0..geometry.height {
        let mut column = 0;
        while column + 1 < geometry.width {
            if !is_cap_pair(column, row) {
                column += 1;
                continue;
            }
            let start = column;
            column += 2;
            while column + 1 < geometry.width && is_cap_pair(column, row) {
                column += 2;
            }
            placements.push(FacilityDividerCapPlacement {
                column: start,
                row,
                width: column - start,
            });
        }
    }
    placements
}

fn facility_divider_stripe_uv(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    column: usize,
    row: usize,
) -> Result<[[f32; 2]; 4], TerrainMeshError> {
    let stripe_index = cells
        .iter()
        .position(|tile| tile.source.tile_index == 0x4d)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: column as u32,
            row: row as u32,
            tile_index: 0x4d,
        })?;
    let uv = geometry.uv(stripe_index % geometry.width, stripe_index / geometry.width);
    Ok([[uv.0, uv.3], [uv.1, uv.3], [uv.1, uv.2], [uv.0, uv.2]])
}

fn append_facility_divider_network(
    mesh: &mut TerrainMeshData,
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    if !crate::facility_divider::supports_map(map_id) {
        return Ok(());
    }
    let find_tile = |tile_index| {
        cells
            .iter()
            .position(|tile| tile.source.tile_index == tile_index)
            .ok_or(TerrainMeshError::MissingGroundSample {
                column: 0,
                row: 0,
                tile_index,
            })
    };
    let ground = find_tile(crate::facility_divider::FLOOR_TILE)?;
    let brown = find_tile(0x41)?;
    let ground_uv = geometry.uv(ground % geometry.width, ground / geometry.width);
    let brown_uv = geometry.uv(brown % geometry.width, brown / geometry.width);
    let stripe_uv = facility_divider_stripe_uv(cells, geometry, 0, 0)?;
    let mut occupied = vec![false; geometry.width * geometry.height];

    for placement in facility_divider_placements(map_id, cells, geometry) {
        for column in placement.column..placement.column + placement.width {
            occupied[placement.row * geometry.width + column] = true;
            occupied[(placement.row + 1) * geometry.width + column] = true;
        }
    }
    for placement in facility_vertical_divider_placements(map_id, cells, geometry) {
        for row in placement.row..placement.row + placement.height {
            occupied[row * geometry.width + placement.column] = true;
            occupied[row * geometry.width + placement.column + 1] = true;
        }
    }
    for placement in facility_divider_cap_placements(map_id, cells, geometry) {
        for column in placement.column..placement.column + placement.width {
            occupied[placement.row * geometry.width + column] = true;
        }
    }

    let wall_height = crate::facility_divider::WALL_HEIGHT;
    for row in 0..geometry.height {
        for column in 0..geometry.width {
            let index = row * geometry.width + column;
            if !occupied[index] {
                continue;
            }
            claimed[index] = true;
            let bounds = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [bounds.0, bounds.1, bounds.2, bounds.3],
                0.0,
                ground_uv,
            );
            let source = &cells[index].source;
            let top_uv = if matches!(source.tile_index, 0x40..=0x42) {
                geometry.uv(column, row)
            } else if matches!(source.tile_index, 0x4c..=0x4e)
                && row > 0
                && occupied[(row - 1) * geometry.width + column]
            {
                geometry.uv(column, row - 1)
            } else {
                brown_uv
            };
            append_top_shaded(
                &mut mesh.textured,
                [bounds.0, bounds.1, bounds.2, bounds.3],
                wall_height,
                top_uv,
                [0.9, 0.9, 0.9, 1.0],
            );

            let north_open = row == 0 || !occupied[(row - 1) * geometry.width + column];
            let south_open =
                row + 1 == geometry.height || !occupied[(row + 1) * geometry.width + column];
            let west_open = column == 0 || !occupied[row * geometry.width + column - 1];
            let east_open =
                column + 1 == geometry.width || !occupied[row * geometry.width + column + 1];
            for band in 0..2 {
                let y0 = band as f32 * geometry.tile_height;
                let y1 = y0 + geometry.tile_height;
                if north_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.1, y0, bounds.2],
                            [bounds.0, y0, bounds.2],
                            [bounds.0, y1, bounds.2],
                            [bounds.1, y1, bounds.2],
                        ],
                        [0.0, 0.0, -1.0],
                        stripe_uv,
                        [0.72, 0.72, 0.72, 1.0],
                    );
                }
                if south_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.0, y0, bounds.3],
                            [bounds.1, y0, bounds.3],
                            [bounds.1, y1, bounds.3],
                            [bounds.0, y1, bounds.3],
                        ],
                        [0.0, 0.0, 1.0],
                        stripe_uv,
                        TEXTURED_SHADE,
                    );
                }
                if west_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.0, y0, bounds.2],
                            [bounds.0, y0, bounds.3],
                            [bounds.0, y1, bounds.3],
                            [bounds.0, y1, bounds.2],
                        ],
                        [-1.0, 0.0, 0.0],
                        stripe_uv,
                        [0.8, 0.8, 0.8, 1.0],
                    );
                }
                if east_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.1, y0, bounds.3],
                            [bounds.1, y0, bounds.2],
                            [bounds.1, y1, bounds.2],
                            [bounds.1, y1, bounds.3],
                        ],
                        [1.0, 0.0, 0.0],
                        stripe_uv,
                        TEXTURED_SHADE,
                    );
                }
            }
        }
    }
    Ok(())
}

fn append_rocket_base_wall_network(
    mesh: &mut TerrainMeshData,
    map_id: &str,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
) {
    let occupied: Vec<_> = cells
        .iter()
        .map(|tile| crate::rocket_base::is_wall_cell(map_id, &tile.source))
        .collect();
    let closes_edge: Vec<_> = cells
        .iter()
        .zip(&occupied)
        .map(|(tile, occupied)| {
            *occupied || crate::rocket_base::closes_wall_edge(map_id, &tile.source)
        })
        .collect();
    if !occupied.iter().any(|occupied| *occupied) {
        return;
    }
    let find_tile = |tile_index| {
        cells
            .iter()
            .position(|tile| tile.source.tile_index == tile_index)
    };
    let (Some(floor), Some(upper), Some(lower)) = (
        find_tile(crate::rocket_base::FLOOR_TILE),
        find_tile(0x04),
        find_tile(0x14),
    ) else {
        // A clipped frame without the complete authored wall vocabulary is
        // insufficient evidence to fold it. Preserve the faithful flat map.
        return;
    };
    let floor_uv = geometry.uv(floor % geometry.width, floor / geometry.width);
    let default_upper_uv = geometry.uv(upper % geometry.width, upper / geometry.width);
    let default_lower_uv = geometry.uv(lower % geometry.width, lower / geometry.width);
    let face_uv = |(u0, u1, v0, v1)| [[u0, v1], [u1, v1], [u1, v0], [u0, v0]];
    let wall_height = crate::rocket_base::WALL_HEIGHT;

    for row in 0..geometry.height {
        for column in 0..geometry.width {
            let index = row * geometry.width + column;
            if !occupied[index] {
                continue;
            }
            claimed[index] = true;
            let bounds = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [bounds.0, bounds.1, bounds.2, bounds.3],
                0.0,
                floor_uv,
            );
            let source = &cells[index].source;
            let source_uv = geometry.uv(column, row);
            let top_uv = if crate::rocket_base::is_upper_face(source) {
                source_uv
            } else {
                default_upper_uv
            };
            append_top_shaded(
                &mut mesh.textured,
                [bounds.0, bounds.1, bounds.2, bounds.3],
                wall_height,
                top_uv,
                [0.88, 0.88, 0.88, 1.0],
            );

            let north_open = row == 0 || !closes_edge[(row - 1) * geometry.width + column];
            let south_open =
                row + 1 == geometry.height || !closes_edge[(row + 1) * geometry.width + column];
            let west_open = column == 0 || !closes_edge[row * geometry.width + column - 1];
            let east_open =
                column + 1 == geometry.width || !closes_edge[row * geometry.width + column + 1];
            for band in 0..2 {
                let y0 = band as f32 * geometry.tile_height;
                let y1 = y0 + geometry.tile_height;
                let horizontal_uv = if band == 0 {
                    if crate::rocket_base::is_lower_face(source) {
                        source_uv
                    } else {
                        default_lower_uv
                    }
                } else if crate::rocket_base::is_upper_face(source) {
                    source_uv
                } else {
                    default_upper_uv
                };
                let vertical_uv = if matches!(source.tile_index, 0x0c | 0x0d) {
                    source_uv
                } else {
                    horizontal_uv
                };
                let horizontal_uv = face_uv(horizontal_uv);
                let vertical_uv = face_uv(vertical_uv);
                if north_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.1, y0, bounds.2],
                            [bounds.0, y0, bounds.2],
                            [bounds.0, y1, bounds.2],
                            [bounds.1, y1, bounds.2],
                        ],
                        [0.0, 0.0, -1.0],
                        horizontal_uv,
                        [0.72, 0.72, 0.72, 1.0],
                    );
                }
                if south_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.0, y0, bounds.3],
                            [bounds.1, y0, bounds.3],
                            [bounds.1, y1, bounds.3],
                            [bounds.0, y1, bounds.3],
                        ],
                        [0.0, 0.0, 1.0],
                        horizontal_uv,
                        TEXTURED_SHADE,
                    );
                }
                if west_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.0, y0, bounds.2],
                            [bounds.0, y0, bounds.3],
                            [bounds.0, y1, bounds.3],
                            [bounds.0, y1, bounds.2],
                        ],
                        [-1.0, 0.0, 0.0],
                        vertical_uv,
                        [0.8, 0.8, 0.8, 1.0],
                    );
                }
                if east_open {
                    append_quad(
                        &mut mesh.textured,
                        [
                            [bounds.1, y0, bounds.3],
                            [bounds.1, y0, bounds.2],
                            [bounds.1, y1, bounds.2],
                            [bounds.1, y1, bounds.3],
                        ],
                        [1.0, 0.0, 0.0],
                        vertical_uv,
                        TEXTURED_SHADE,
                    );
                }
            }
        }
    }
}

fn append_facility_divider(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: FacilityDividerPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground = cells
        .iter()
        .position(|tile| tile.source.tile_index == crate::facility_divider::FLOOR_TILE)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: crate::facility_divider::FLOOR_TILE,
        })?;
    let ground_uv = geometry.uv(ground % geometry.width, ground / geometry.width);
    let first = geometry.bounds(placement.column, placement.row);
    let last = geometry.bounds(placement.column + placement.width - 1, placement.row);
    let x0 = first.0;
    let x1 = last.1;
    let front_z = first.2 + geometry.tile_height * 2.0;
    let rear_z = front_z - crate::facility_divider::WALL_DEPTH;
    let wall_height = crate::facility_divider::WALL_HEIGHT;
    let stripe_uv = facility_divider_stripe_uv(cells, geometry, placement.column, placement.row)?;

    for column in placement.column..placement.column + placement.width {
        let top_index = placement.row * geometry.width + column;
        let face_index = (placement.row + 1) * geometry.width + column;
        claimed[top_index] = true;
        claimed[face_index] = true;
        for row in [placement.row, placement.row + 1] {
            let (gx0, gx1, gz0, gz1) = geometry.bounds(column, row);
            append_top(&mut mesh.textured, [gx0, gx1, gz0, gz1], 0.0, ground_uv);
        }

        let (cx0, cx1, _, _) = geometry.bounds(column, placement.row);
        let top_uv = geometry.uv(column, placement.row);
        let depth_band_count =
            (crate::facility_divider::WALL_DEPTH / geometry.tile_height) as usize;
        for depth_band in 0..depth_band_count {
            let z0 = rear_z + depth_band as f32 * geometry.tile_height;
            append_top_shaded(
                &mut mesh.textured,
                [cx0, cx1, z0, z0 + geometry.tile_height],
                wall_height,
                top_uv,
                [0.9, 0.9, 0.9, 1.0],
            );
        }
        let band_count = (wall_height / geometry.tile_height) as usize;
        for band in 0..band_count {
            let y0 = band as f32 * geometry.tile_height;
            let y1 = y0 + geometry.tile_height;
            append_quad(
                &mut mesh.textured,
                [
                    [cx0, y0, front_z],
                    [cx1, y0, front_z],
                    [cx1, y1, front_z],
                    [cx0, y1, front_z],
                ],
                [0.0, 0.0, 1.0],
                stripe_uv,
                TEXTURED_SHADE,
            );
        }
    }

    for column in placement.column..placement.column + placement.width {
        let (cx0, cx1, _, _) = geometry.bounds(column, placement.row);
        let band_count = (wall_height / geometry.tile_height) as usize;
        for band in 0..band_count {
            let y0 = band as f32 * geometry.tile_height;
            let y1 = y0 + geometry.tile_height;
            append_quad(
                &mut mesh.textured,
                [
                    [cx1, y0, rear_z],
                    [cx0, y0, rear_z],
                    [cx0, y1, rear_z],
                    [cx1, y1, rear_z],
                ],
                [0.0, 0.0, -1.0],
                stripe_uv,
                [0.72, 0.72, 0.72, 1.0],
            );
        }
    }
    for (x, _column, normal) in [
        (x0, placement.column, [-1.0, 0.0, 0.0]),
        (x1, placement.column + placement.width - 1, [1.0, 0.0, 0.0]),
    ] {
        append_quad(
            &mut mesh.textured,
            [
                [x, 0.0, rear_z],
                [x, 0.0, front_z],
                [x, wall_height, front_z],
                [x, wall_height, rear_z],
            ],
            normal,
            stripe_uv,
            [0.8, 0.8, 0.8, 1.0],
        );
    }
    Ok(())
}

fn append_facility_vertical_divider(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: FacilityVerticalDividerPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground = cells
        .iter()
        .position(|tile| tile.source.tile_index == crate::facility_divider::FLOOR_TILE)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: crate::facility_divider::FLOOR_TILE,
        })?;
    let ground_uv = geometry.uv(ground % geometry.width, ground / geometry.width);
    let first = geometry.bounds(placement.column, placement.row);
    let last = geometry.bounds(placement.column, placement.row + placement.height - 1);
    let front_x = first.0 + geometry.tile_width * 2.0;
    let rear_x = front_x - crate::facility_divider::WALL_DEPTH;
    let z0 = first.2;
    let z1 = last.3;
    let wall_height = crate::facility_divider::WALL_HEIGHT;
    let stripe_uv = facility_divider_stripe_uv(cells, geometry, placement.column, placement.row)?;

    for row in placement.row..placement.row + placement.height {
        for column in placement.column..placement.column + 2 {
            let index = row * geometry.width + column;
            claimed[index] = true;
            let (cx0, cx1, cz0, cz1) = geometry.bounds(column, row);
            append_top(&mut mesh.textured, [cx0, cx1, cz0, cz1], 0.0, ground_uv);
        }
        let (_, _, row_z0, row_z1) = geometry.bounds(placement.column, row);
        let top_uv = geometry.uv(placement.column, row);
        let depth_band_count = (crate::facility_divider::WALL_DEPTH / geometry.tile_width) as usize;
        for depth_band in 0..depth_band_count {
            let x0 = rear_x + depth_band as f32 * geometry.tile_width;
            append_top_shaded(
                &mut mesh.textured,
                [x0, x0 + geometry.tile_width, row_z0, row_z1],
                wall_height,
                top_uv,
                [0.9, 0.9, 0.9, 1.0],
            );
        }
        for band in 0..2 {
            let y0 = band as f32 * wall_height * 0.5;
            let y1 = y0 + wall_height * 0.5;
            append_quad(
                &mut mesh.textured,
                [
                    [front_x, y0, row_z1],
                    [front_x, y0, row_z0],
                    [front_x, y1, row_z0],
                    [front_x, y1, row_z1],
                ],
                [1.0, 0.0, 0.0],
                stripe_uv,
                TEXTURED_SHADE,
            );
            append_quad(
                &mut mesh.textured,
                [
                    [rear_x, y0, row_z0],
                    [rear_x, y0, row_z1],
                    [rear_x, y1, row_z1],
                    [rear_x, y1, row_z0],
                ],
                [-1.0, 0.0, 0.0],
                stripe_uv,
                [0.72, 0.72, 0.72, 1.0],
            );
        }
    }

    for (z, _row, normal) in [
        (z0, placement.row, [0.0, 0.0, -1.0]),
        (z1, placement.row + placement.height - 1, [0.0, 0.0, 1.0]),
    ] {
        append_quad(
            &mut mesh.textured,
            [
                [rear_x, 0.0, z],
                [front_x, 0.0, z],
                [front_x, wall_height, z],
                [rear_x, wall_height, z],
            ],
            normal,
            stripe_uv,
            [0.8, 0.8, 0.8, 1.0],
        );
    }
    Ok(())
}

fn append_facility_divider_cap(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: FacilityDividerCapPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground = cells
        .iter()
        .position(|tile| tile.source.tile_index == crate::facility_divider::FLOOR_TILE)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: crate::facility_divider::FLOOR_TILE,
        })?;
    let ground_uv = geometry.uv(ground % geometry.width, ground / geometry.width);
    let first = geometry.bounds(placement.column, placement.row);
    let last = geometry.bounds(placement.column + placement.width - 1, placement.row);
    let x0 = first.0;
    let x1 = last.1;
    let front_z = first.3;
    let rear_z = front_z - crate::facility_divider::WALL_DEPTH;
    let wall_height = crate::facility_divider::WALL_HEIGHT;
    let stripe_uv = facility_divider_stripe_uv(cells, geometry, placement.column, placement.row)?;

    for column in placement.column..placement.column + placement.width {
        let index = placement.row * geometry.width + column;
        claimed[index] = true;
        let (cx0, cx1, cz0, cz1) = geometry.bounds(column, placement.row);
        append_top(&mut mesh.textured, [cx0, cx1, cz0, cz1], 0.0, ground_uv);
        let top_uv = geometry.uv(column, placement.row);
        let depth_band_count =
            (crate::facility_divider::WALL_DEPTH / geometry.tile_height) as usize;
        for depth_band in 0..depth_band_count {
            let z0 = rear_z + depth_band as f32 * geometry.tile_height;
            append_top_shaded(
                &mut mesh.textured,
                [cx0, cx1, z0, z0 + geometry.tile_height],
                wall_height,
                top_uv,
                [0.9, 0.9, 0.9, 1.0],
            );
        }
    }

    for column in placement.column..placement.column + placement.width {
        let (cx0, cx1, _, _) = geometry.bounds(column, placement.row);
        for band in 0..2 {
            let y0 = band as f32 * geometry.tile_height;
            let y1 = y0 + geometry.tile_height;
            append_quad(
                &mut mesh.textured,
                [
                    [cx0, y0, front_z],
                    [cx1, y0, front_z],
                    [cx1, y1, front_z],
                    [cx0, y1, front_z],
                ],
                [0.0, 0.0, 1.0],
                stripe_uv,
                TEXTURED_SHADE,
            );
            append_quad(
                &mut mesh.textured,
                [
                    [cx1, y0, rear_z],
                    [cx0, y0, rear_z],
                    [cx0, y1, rear_z],
                    [cx1, y1, rear_z],
                ],
                [0.0, 0.0, -1.0],
                stripe_uv,
                [0.72, 0.72, 0.72, 1.0],
            );
        }
    }
    for (x, _column, normal) in [
        (x0, placement.column, [-1.0, 0.0, 0.0]),
        (x1, placement.column + placement.width - 1, [1.0, 0.0, 0.0]),
    ] {
        for band in 0..2 {
            let y0 = band as f32 * geometry.tile_height;
            let y1 = y0 + geometry.tile_height;
            append_quad(
                &mut mesh.textured,
                [
                    [x, y0, rear_z],
                    [x, y0, front_z],
                    [x, y1, front_z],
                    [x, y1, rear_z],
                ],
                normal,
                stripe_uv,
                [0.8, 0.8, 0.8, 1.0],
            );
        }
    }
    Ok(())
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
    let mut audit_shapes: Vec<_> = cells
        .iter()
        .map(|tile| shape_for_source_on_map(map_id, &tile.source))
        .collect();
    resolve_rock_platform_tiers(&cells, &mut audit_shapes, width);
    let mut coverage: Vec<_> = audit_shapes
        .into_iter()
        .map(|shape| match shape {
            CellShape::Flat => CellCoverageKind::Flat,
            CellShape::Water => CellCoverageKind::Water,
            CellShape::PlaneAt { .. } => CellCoverageKind::Plane,
            CellShape::Waterfall => CellCoverageKind::Waterfall,
            CellShape::Cutout { .. } => CellCoverageKind::Cutout,
            CellShape::Relief { .. } | CellShape::FloatingRelief { .. } => CellCoverageKind::Relief,
            CellShape::ShoreBand => CellCoverageKind::Shore,
            CellShape::RaisedTop { .. } => CellCoverageKind::Raised,
            CellShape::RampNorth { .. } | CellShape::RampEast { .. } => CellCoverageKind::Ramp,
            CellShape::FacadeBand {
                solid: SolidKind::Tree,
                ..
            } => CellCoverageKind::Tree,
            CellShape::FacadeBand { .. } => CellCoverageKind::Facade,
            CellShape::LedgeBand { .. } => CellCoverageKind::Ledge,
        })
        .collect();
    for (index, tile) in cells.iter().enumerate() {
        if house_stair_local(map_id, &tile.source).is_some() {
            coverage[index] = CellCoverageKind::Ramp;
        }
    }
    for (index, tile) in cells.iter().enumerate() {
        if crate::rocket_base::is_wall_cell(map_id, &tile.source) {
            coverage[index] = CellCoverageKind::Raised;
        }
    }
    for (column, row, course_width) in players_house::course_origins(&cells, &geometry)
        .into_iter()
        .chain(ordinary_house::course_origins(&cells, &geometry))
    {
        for local_row in 0..4 {
            for local_column in 0..course_width {
                let index = (row + local_row) * width + column + local_column;
                if house_stair_local(map_id, &cells[index].source).is_none() {
                    coverage[index] = CellCoverageKind::Facade;
                }
            }
        }
    }
    for (column, row, course_width, _) in traditional_house::course_origins(&cells, &geometry) {
        for local_row in 0..4 {
            for local_column in 0..course_width {
                coverage[(row + local_row) * width + column + local_column] =
                    CellCoverageKind::Facade;
            }
        }
    }
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
    apply_grouped_card_coverage(
        &mut coverage,
        width,
        cave_small_rock_placements(&cells, &geometry),
        CellCoverageKind::Cutout,
    );
    for placement in diagonal_cave_corner_placements(&cells, &geometry) {
        for row in placement.row..placement.row + 2 {
            for column in placement.column..placement.column + 2 {
                coverage[row * width + column] = CellCoverageKind::Ledge;
            }
        }
    }

    let mut prop_cards = match map_id {
        "AzaleaGym" | "GoldenrodGym" => elite_four_gym_card_placements(map_id, &cells, &geometry),
        "HallOfFame" => hall_of_fame_console_placements(&cells, &geometry),
        "CeruleanGym" => cerulean_statue_placements(&cells, &geometry),
        "FuchsiaGym" => fuchsia_statue_placements(map_id, &cells, &geometry),
        "CeladonGym" => celadon_statue_placements(map_id, &cells, &geometry),
        "VermilionGym" => vermilion_statue_placements(map_id, &cells, &geometry),
        "ViridianGym" => viridian_statue_placements(map_id, &cells, &geometry),
        "VioletGym" | "MahoganyGym" | "BlackthornGym1F" => {
            violet_gym_card_placements(map_id, &cells, &geometry)
        }
        "OlivineGym" => {
            let mut placements = olivine_gym_boulder_placements(&cells, &geometry);
            placements.extend(olivine_gym_statue_placements(map_id, &cells, &geometry));
            placements
        }
        "SaffronGym" => saffron_gym_planter_placements(&cells, &geometry),
        map_id if crate::elite_four_room::supports_boulder_map(map_id) => {
            elite_four_room_boulder_placements(&cells, &geometry)
        }
        _ => Vec::new(),
    };
    prop_cards.extend(ice_path_boulder_placements(&cells, &geometry));
    prop_cards.extend(ice_path_edge_rock_placements(&cells, &geometry));
    prop_cards.extend(ruins_statue_placements(&cells, &geometry));
    prop_cards.extend(tower_statue_placements(&cells, &geometry));
    prop_cards.extend(power_plant_plant_placements(map_id, &cells, &geometry));
    prop_cards.extend(rocket_base_plant_placements(map_id, &cells, &geometry));
    prop_cards.extend(mart_display_rack_placements(map_id, &cells, &geometry));
    prop_cards.extend(pokecenter_pc_placements(map_id, &cells, &geometry));
    prop_cards.extend(pokecom_workstation_placements(map_id, &cells, &geometry));
    prop_cards.extend(pokecom_plant_placements(map_id, &cells, &geometry));
    prop_cards.extend(pokecom_chair_placements(map_id, &cells, &geometry));
    prop_cards.extend(park_bench_placements(&cells, &geometry));
    prop_cards.extend(kanto_round_barrier_placements(&cells, &geometry));
    prop_cards.extend(kanto_round_path_barrier_placements(&cells, &geometry));
    prop_cards.extend(kanto_shoreline_round_barrier_placements(&cells, &geometry));
    prop_cards.extend(ship_stool_placements(map_id, &cells, &geometry));
    prop_cards.extend(ship_rack_placements(map_id, &cells, &geometry));
    prop_cards.extend(ship_barrel_placements(map_id, &cells, &geometry));
    prop_cards.extend(train_station_seat_placements(map_id, &cells, &geometry));
    prop_cards.extend(train_station_planter_placements(map_id, &cells, &geometry));
    prop_cards.extend(train_station_gate_placements(map_id, &cells, &geometry));
    prop_cards.extend(house_plant_placements(&cells, &geometry));
    prop_cards.extend(house_upright_fixture_placements(&cells, &geometry));
    prop_cards.extend(house_bookcase_placements(&cells, &geometry));
    prop_cards.extend(traditional_gift_shop_shelf_placements(
        map_id, &cells, &geometry,
    ));
    prop_cards.extend(house_open_book_placements(&cells, &geometry));
    prop_cards.extend(players_house_bookcase_placements(&cells, &geometry));
    prop_cards.extend(players_house_upright_fixture_placements(&cells, &geometry));
    prop_cards.extend(players_house_tv_placements(&cells, &geometry));
    prop_cards.extend(players_house_console_placements(&cells, &geometry));
    prop_cards.extend(players_house_bed_placements(&cells, &geometry));
    prop_cards.extend(player_room_fixture_placements(&cells, &geometry));
    prop_cards.extend(player_bed_placements(&cells, &geometry));
    prop_cards.extend(traditional_house_radio_placements(&cells, &geometry));
    prop_cards.extend(traditional_house_cushion_placements(&cells, &geometry));
    prop_cards.extend(wise_trios_divider_placements(map_id, &cells, &geometry));
    if map_id == "SoulHouse" {
        prop_cards.extend(soul_house_bench_placements(&cells, &geometry));
    }
    apply_grouped_card_coverage(&mut coverage, width, prop_cards, CellCoverageKind::Cutout);
    for placement in mr_pokemon_work_counter_placements(map_id, &cells, &geometry) {
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                coverage[row * width + column] = CellCoverageKind::Raised;
            }
        }
    }
    for placement in house_display_table_placements(&cells, &geometry) {
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                coverage[row * width + column] = CellCoverageKind::Raised;
            }
        }
    }
    for placement in warehouse_crate_placements(map_id, &cells, &geometry) {
        for row in placement.row..placement.row + placement.height {
            for column in placement.column..placement.column + placement.width {
                coverage[row * width + column] = CellCoverageKind::Raised;
            }
        }
    }
    for placement in house_furniture_placements(&cells, &geometry) {
        for row in placement.row..placement.row + 2 {
            for column in placement.column..placement.column + 2 {
                coverage[row * width + column] = CellCoverageKind::Raised;
            }
        }
    }
    for placement in house_table_placements(&cells, &geometry) {
        for row in placement.row..placement.row + 4 {
            for column in placement.column..placement.column + 4 {
                coverage[row * width + column] = CellCoverageKind::Raised;
            }
        }
    }
    if crate::casino::is_game_corner_map(map_id) || crate::cafe::is_cafe_map(map_id) {
        for placement in casino_stool_placements(&cells, &geometry) {
            for row in placement.row..placement.row + 2 {
                for column in placement.column..placement.column + 2 {
                    coverage[row * width + column] = CellCoverageKind::Cutout;
                }
            }
        }
    }
    if crate::casino::is_game_corner_map(map_id) {
        apply_grouped_card_coverage(
            &mut coverage,
            width,
            casino_terminal_placements(&cells, &geometry),
            CellCoverageKind::Cutout,
        );
        apply_grouped_card_coverage(
            &mut coverage,
            width,
            casino_plant_placements(&cells, &geometry),
            CellCoverageKind::Cutout,
        );
    }
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
            let is_bank = !matches!(
                shapes[index],
                CellShape::RampNorth { .. } | CellShape::RampEast { .. }
            ) && shapes[index].solid_kind() == SolidKind::Bank
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
                ("johto", 0x61) => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row % 2,
                    2,
                    2,
                    0x05,
                    0.0,
                    false,
                ),
                ("johto", 0x5d) if tile.source.subtile_row < 2 => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row,
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
                ("park", _) => {
                    if let Some((local_column, local_row)) =
                        crate::park::large_tree_local(&tile.source)
                    {
                        (
                            local_column,
                            local_row,
                            4,
                            4,
                            crate::park::PARK_TREE_GROUND_TILE,
                            0.0,
                            false,
                        )
                    } else if let Some((_, local_column, local_row)) =
                        crate::park::hedge_local(&tile.source)
                    {
                        (
                            local_column,
                            local_row,
                            2,
                            2,
                            crate::park::PARK_TREE_GROUND_TILE,
                            0.0,
                            false,
                        )
                    } else {
                        continue;
                    }
                }
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
                        || crate::park::large_tree_local(&cell.source).is_some()
                        || crate::park::hedge_local(&tile.source).is_some_and(
                            |(expected_kind, _, _)| {
                                crate::park::hedge_local(&cell.source)
                                    .is_some_and(|(kind, _, _)| kind == expected_kind)
                            },
                        )
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
                ground_metatile_id: (tile.source.tileset_id.as_ref() == "park")
                    .then_some(crate::park::PARK_TREE_GROUND_METATILE),
                base_height,
                rounded,
                outline_mask: false,
                remove_all_ground: false,
                card_thickness: 0.0,
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
    for (column, row) in johto_closed_mound_origins(cells, geometry.width, geometry.height) {
        placements.push(BuildingPlacement {
            column,
            row,
            width: 12,
            height: 8,
            roof_rows: 6,
            ground_tile_index: 0x01,
        });
    }
    for (column, row, width) in ice_path_plateau_origins(cells, geometry.width, geometry.height) {
        placements.push(BuildingPlacement {
            column,
            row,
            width,
            height: 8,
            roof_rows: 6,
            ground_tile_index: 0x9a,
        });
    }
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

fn johto_closed_mound_origins(
    cells: &[&VisualTile],
    width: usize,
    height: usize,
) -> Vec<(usize, usize)> {
    let mut metatiles = HashMap::new();
    for tile in cells {
        if tile.source.tileset_id.as_ref() != "johto" {
            continue;
        }
        let origin_column = tile.column as isize - tile.source.subtile_column as isize;
        let origin_row = tile.row as isize - tile.source.subtile_row as isize;
        if origin_column >= 0 && origin_row >= 0 {
            metatiles
                .entry((origin_column as usize, origin_row as usize))
                .or_insert(tile.source.metatile_id);
        }
    }
    let expected = [[0x6a, 0x70, 0x6b], [0x6c, 0x72, 0x6d]];
    let mut origins = Vec::new();
    for (&(column, row), &first) in &metatiles {
        if first != expected[0][0] || column + 12 > width || row + 8 > height {
            continue;
        }
        let complete = expected.iter().enumerate().all(|(block_row, ids)| {
            ids.iter().enumerate().all(|(block_column, expected_id)| {
                metatiles.get(&(column + block_column * 4, row + block_row * 4))
                    == Some(expected_id)
            })
        });
        if complete {
            origins.push((column, row));
        }
    }
    origins.sort_unstable();
    origins.dedup();
    origins
}

/// Ice Path's rock islands and mainland shelves use one variable-width,
/// two-block-deep drawing. The north row is `$04 $05... $06`; the south row
/// carries exact front, corner, and opening variants. Six source-tile rows
/// form the cap and the final two form its front, matching the existing mound
/// primitive without classifying any block in isolation.
fn ice_path_plateau_origins(
    cells: &[&VisualTile],
    width: usize,
    height: usize,
) -> Vec<(usize, usize, usize)> {
    let mut metatiles = HashMap::new();
    for tile in cells {
        if tile.source.tileset_id.as_ref() != "ice_path" {
            continue;
        }
        let origin_column = tile.column as isize - tile.source.subtile_column as isize;
        let origin_row = tile.row as isize - tile.source.subtile_row as isize;
        if origin_column >= 0 && origin_row >= 0 {
            metatiles
                .entry((origin_column as usize, origin_row as usize))
                .or_insert(tile.source.metatile_id);
        }
    }
    let mut origins = Vec::new();
    for (&(column, row), &first) in &metatiles {
        if first != 0x04 || row + 8 > height {
            continue;
        }
        for block_width in 2..=(width.saturating_sub(column) / 4) {
            let last_column = column + (block_width - 1) * 4;
            if metatiles.get(&(last_column, row)) != Some(&0x06)
                || (1..block_width - 1)
                    .any(|block| metatiles.get(&(column + block * 4, row)) != Some(&0x05))
            {
                continue;
            }
            let south: Vec<_> = (0..block_width)
                .filter_map(|block| metatiles.get(&(column + block * 4, row + 4)).copied())
                .collect();
            let complete_south = south.len() == block_width
                && matches!(south.last(), Some(0x0e | 0x3a))
                && matches!(south.first(), Some(0x09 | 0x0c | 0x10))
                && south.iter().all(|id| {
                    matches!(
                        id,
                        0x09 | 0x0c | 0x0d | 0x0e | 0x10 | 0x11 | 0x12 | 0x3a | 0x3e
                    )
                });
            if complete_south {
                origins.push((column, row, block_width * 4));
                break;
            }
        }
    }
    origins.sort_unstable();
    origins.dedup();
    origins
}

fn ice_path_closed_rock_mass_origins(
    cells: &[&VisualTile],
    width: usize,
    height: usize,
) -> Vec<(usize, usize)> {
    let mut origins = Vec::new();
    for tile in cells {
        let Some((local_column, local_row)) = crate::ice_path::rock_mass_local(&tile.source) else {
            continue;
        };
        let origin_column = tile.column as isize - isize::from(local_column);
        let origin_row = tile.row as isize - isize::from(local_row);
        if origin_column < 0
            || origin_row < 0
            || origin_column as usize + 4 > width
            || origin_row as usize + 4 > height
        {
            continue;
        }
        let complete = (0..4).all(|row| {
            (0..4).all(|column| {
                let candidate =
                    cells[(origin_row as usize + row) * width + origin_column as usize + column];
                crate::ice_path::rock_mass_local(&candidate.source)
                    == Some((column as u8, row as u8))
                    && candidate.source.metatile_id == tile.source.metatile_id
            })
        });
        if complete {
            origins.push((origin_column as usize, origin_row as usize));
        }
    }
    origins.sort_unstable();
    origins.dedup();
    origins
}

fn append_ice_path_closed_rock_mass(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    column: usize,
    row: usize,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    const WIDTH: usize = 4;
    const HEIGHT: usize = 4;
    const CAP_ROWS: usize = 2;
    let world_height = 12.0 * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let batter = 2.0 * geometry.tile_width / SOURCE_TILE_HEIGHT;

    let ground_index = cells
        .iter()
        .position(|tile| tile.source.tile_index == crate::ice_path::CAVE_GROUND_TILE)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: column as u32,
            row: row as u32,
            tile_index: crate::ice_path::CAVE_GROUND_TILE,
        })?;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    for local_row in 0..HEIGHT {
        for local_column in 0..WIDTH {
            let index = (row + local_row) * geometry.width + column + local_column;
            claimed[index] = true;
            let (x0, x1, z0, z1) = geometry.bounds(column + local_column, row + local_row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
        }
    }

    // The northern two source rows are the authored top. Preserve every
    // source cell once on a single level plane; the southern two rows become
    // the front face below rather than remaining duplicated on the floor.
    for local_row in 0..CAP_ROWS {
        for local_column in 0..WIDTH {
            let (x0, x1, z0, z1) = geometry.bounds(column + local_column, row + local_row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                world_height,
                geometry.uv(column + local_column, row + local_row),
            );
        }
    }

    let first = geometry.bounds(column, row);
    let last = geometry.bounds(column + WIDTH - 1, row + CAP_ROWS - 1);
    let x0 = first.0;
    let x1 = last.1;
    let north_z = first.2;
    let front_z = last.3;
    let face_band_height = world_height / (HEIGHT - CAP_ROWS) as f32;
    for band in 0..HEIGHT - CAP_ROWS {
        let top = world_height - band as f32 * face_band_height;
        let bottom = top - face_band_height;
        let top_batter = batter * (1.0 - top / world_height);
        let bottom_batter = batter * (1.0 - bottom / world_height);
        for local_column in 0..WIDTH {
            let bounds = geometry.bounds(column + local_column, row + CAP_ROWS + band);
            let left_top = bounds.0 - if local_column == 0 { top_batter } else { 0.0 };
            let left_bottom = bounds.0
                - if local_column == 0 {
                    bottom_batter
                } else {
                    0.0
                };
            let right_top = bounds.1
                + if local_column + 1 == WIDTH {
                    top_batter
                } else {
                    0.0
                };
            let right_bottom = bounds.1
                + if local_column + 1 == WIDTH {
                    bottom_batter
                } else {
                    0.0
                };
            let (u0, u1, v0, v1) = geometry.uv(column + local_column, row + CAP_ROWS + band);
            append_quad(
                &mut mesh.textured,
                [
                    [right_bottom, bottom, front_z + bottom_batter],
                    [right_top, top, front_z + top_batter],
                    [left_top, top, front_z + top_batter],
                    [left_bottom, bottom, front_z + bottom_batter],
                ],
                Vec3::new(0.0, batter, world_height).normalize().to_array(),
                [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                TEXTURED_SHADE,
            );
        }
    }

    let side = solid_color(SolidKind::Rock, Direction::West);
    append_solid_quad(
        &mut mesh.solid,
        [
            [x0 - batter, 0.0, front_z + batter],
            [x0, world_height, front_z],
            [x0, world_height, north_z],
            [x0 - batter, 0.0, north_z - batter],
        ],
        Vec3::new(-world_height, batter, 0.0).normalize().to_array(),
        side,
    );
    append_solid_quad(
        &mut mesh.solid,
        [
            [x1 + batter, 0.0, north_z - batter],
            [x1, world_height, north_z],
            [x1, world_height, front_z],
            [x1 + batter, 0.0, front_z + batter],
        ],
        Vec3::new(world_height, batter, 0.0).normalize().to_array(),
        solid_color(SolidKind::Rock, Direction::East),
    );
    append_solid_quad(
        &mut mesh.solid,
        [
            [x0 - batter, 0.0, north_z - batter],
            [x0, world_height, north_z],
            [x1, world_height, north_z],
            [x1 + batter, 0.0, north_z - batter],
        ],
        Vec3::new(0.0, batter, -world_height).normalize().to_array(),
        solid_color(SolidKind::Rock, Direction::North),
    );
    Ok(())
}

fn append_ice_path_small_boulder(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    debug_assert_eq!((placement.width, placement.height), (2, 2));
    let world_height = geometry.tile_height;
    // Keep the flare inside one source pixel. A wider untextured wedge reads
    // as a separate black diamond beside the boulder at the pitched camera.
    let batter = 0.75 * geometry.tile_width / SOURCE_TILE_HEIGHT;
    let ground_index = cells
        .iter()
        .position(|tile| tile.source.tile_index == placement.ground_tile_index)
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: placement.ground_tile_index,
        })?;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    for local_row in 0..2 {
        for local_column in 0..2 {
            let index =
                (placement.row + local_row) * geometry.width + placement.column + local_column;
            claimed[index] = true;
            let (x0, x1, z0, z1) =
                geometry.bounds(placement.column + local_column, placement.row + local_row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
        }
    }

    // The upper native row is the boulder's cap; the lower native row is its
    // front. This is one low trapezoid, not four pixel columns and not a
    // zero-depth billboard.
    for local_column in 0..2 {
        let (x0, x1, z0, z1) = geometry.bounds(placement.column + local_column, placement.row);
        append_top(
            &mut mesh.textured,
            [x0, x1, z0, z1],
            world_height,
            geometry.uv(placement.column + local_column, placement.row),
        );
    }
    let first = geometry.bounds(placement.column, placement.row);
    let last = geometry.bounds(placement.column + 1, placement.row);
    let x0 = first.0;
    let x1 = last.1;
    let north_z = first.2;
    let front_z = first.3;
    for local_column in 0..2 {
        let bounds = geometry.bounds(placement.column + local_column, placement.row + 1);
        let left_bottom = bounds.0 - if local_column == 0 { batter } else { 0.0 };
        let right_bottom = bounds.1 + if local_column == 1 { batter } else { 0.0 };
        let (u0, u1, v0, v1) = geometry.uv(placement.column + local_column, placement.row + 1);
        append_quad(
            &mut mesh.textured,
            [
                [right_bottom, 0.0, front_z + batter],
                [bounds.1, world_height, front_z],
                [bounds.0, world_height, front_z],
                [left_bottom, 0.0, front_z + batter],
            ],
            Vec3::new(0.0, batter, world_height).normalize().to_array(),
            [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
            TEXTURED_SHADE,
        );
    }
    // Pull the side colors from the drawing's own outermost texel columns.
    // This is the same edge-strip principle used by the reference renderer:
    // recognizable artwork stays on the cap/front and narrow generated sides
    // inherit its palette instead of becoming unrelated solid-color fins.
    let west_uv = geometry.uv(placement.column, placement.row);
    let east_uv = geometry.uv(placement.column + 1, placement.row);
    let west_strip = (west_uv.1 - west_uv.0) / SOURCE_TILE_HEIGHT;
    let east_strip = (east_uv.1 - east_uv.0) / SOURCE_TILE_HEIGHT;
    append_quad(
        &mut mesh.textured,
        [
            [x0 - batter, 0.0, front_z + batter],
            [x0, world_height, front_z],
            [x0, world_height, north_z],
            [x0 - batter, 0.0, north_z - batter],
        ],
        Vec3::new(-world_height, batter, 0.0).normalize().to_array(),
        [
            [west_uv.0 + west_strip, west_uv.3],
            [west_uv.0 + west_strip, west_uv.2],
            [west_uv.0, west_uv.2],
            [west_uv.0, west_uv.3],
        ],
        TEXTURED_SHADE,
    );
    append_quad(
        &mut mesh.textured,
        [
            [x1 + batter, 0.0, north_z - batter],
            [x1, world_height, north_z],
            [x1, world_height, front_z],
            [x1 + batter, 0.0, front_z + batter],
        ],
        Vec3::new(world_height, batter, 0.0).normalize().to_array(),
        [
            [east_uv.1 - east_strip, east_uv.2],
            [east_uv.1 - east_strip, east_uv.2],
            [east_uv.1, east_uv.3],
            [east_uv.1, east_uv.3],
        ],
        TEXTURED_SHADE,
    );
    Ok(())
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

                // Celadon Department Store combines the ordinary
                // $20/$54*/$21 roof cap with repeated rounded window
                // courses before its $37/$3a*/$73 entrance course.  The
                // rounded-course detector below cannot own the cap above
                // it, which left that roof artwork painted flat behind the
                // tower.  Claim the complete landmark from its real cap and
                // keep only that first metatile row top-facing.
                let rounded_row_matches = |candidate_y: isize| {
                    at(x, candidate_y) == Some(0x68)
                        && at(last_x, candidate_y) == Some(0x69)
                        && (1..width_blocks - 1)
                            .all(|column| at(x + (column * 4) as isize, candidate_y) == Some(0x7f))
                };
                let mut rounded_rows = 0;
                while rounded_row_matches(y + ((rounded_rows + 1) * 4) as isize) {
                    rounded_rows += 1;
                }
                if rounded_rows == 0 {
                    continue;
                }
                let entrance_y = y + ((rounded_rows + 1) * 4) as isize;
                if at(x, entrance_y) == Some(0x37)
                    && matches!(at(last_x, entrance_y), Some(0x73 | 0x7e))
                    && (1..width_blocks - 1).all(|column| {
                        matches!(at(x + (column * 4) as isize, entrance_y), Some(0x3a | 0x7d))
                    })
                {
                    add(x, y, width_blocks, rounded_rows + 2, 4);
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
    append_grouped_tree_scaled(
        mesh, images, cells, shapes, geometry, placement, claimed, 1.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_grouped_tree_scaled(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
    height_scale: f32,
) -> Result<(), TerrainMeshError> {
    append_grouped_tree_scaled_inner(
        mesh,
        images,
        cells,
        shapes,
        geometry,
        placement,
        claimed,
        height_scale,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_grouped_tree_overlay(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    append_grouped_tree_scaled_inner(
        mesh, images, cells, shapes, geometry, placement, claimed, 1.0, false,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_grouped_tree_scaled_inner(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
    height_scale: f32,
    replace_source_surface: bool,
) -> Result<(), TerrainMeshError> {
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        placement.ground_tile_index,
        placement.ground_metatile_id,
        if replace_source_surface {
            placement.base_height
        } else {
            0.0
        },
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
    let removable_ground = if placement.remove_all_ground {
        equals_ground
    } else {
        boundary_connected_mask(pixel_width, pixel_height, &background_candidates)
    };
    let mut solid_pixels: Vec<_> = removable_ground.into_iter().map(|ground| !ground).collect();
    // Scaled Game Corner cabinets are painted over palette-shifted aisle
    // tiles, so equality with the standalone floor sample cannot remove the
    // grid reliably. The vivid pink/yellow/orange cabinet face provides an
    // exact authored crop; retain its bounding casing and discard everything
    // outside it before standing the sprite up.
    if height_scale > 1.0 {
        for cell_y in 0..placement.height {
            for cell_x in 0..placement.width {
                let px0 = cell_x * SOURCE_TILE_PIXELS;
                let py0 = cell_y * SOURCE_TILE_PIXELS;
                let mut bounds = None::<(usize, usize, usize, usize)>;
                for py in py0..py0 + SOURCE_TILE_PIXELS {
                    for px in px0..px0 + SOURCE_TILE_PIXELS {
                        let [r, g, b, _] = drawing[py * pixel_width + px];
                        let vivid = r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 72
                            && r.max(g).max(b) > 160;
                        if vivid {
                            bounds = Some(match bounds {
                                Some((min_x, max_x, min_y, max_y)) => {
                                    (min_x.min(px), max_x.max(px), min_y.min(py), max_y.max(py))
                                }
                                None => (px, px, py, py),
                            });
                        }
                    }
                }
                if let Some((min_x, max_x, min_y, max_y)) = bounds {
                    for py in py0..py0 + SOURCE_TILE_PIXELS {
                        for px in px0..px0 + SOURCE_TILE_PIXELS {
                            solid_pixels[py * pixel_width + px] =
                                px >= min_x && px <= max_x && py >= min_y && py <= max_y;
                        }
                    }
                }
            }
        }
    }

    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            let index = row * geometry.width + column;
            if replace_source_surface {
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
    }

    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    let plane_z =
        geometry.origin_z + (placement.row + placement.height) as f32 * geometry.tile_height;
    let object_height = placement.height as f32 * geometry.tile_height * height_scale;
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
    let card_cos = std::f32::consts::FRAC_1_SQRT_2;
    let card_sin = std::f32::consts::FRAC_1_SQRT_2;
    let solid_at = |x: isize, y: isize| {
        x >= 0
            && y >= 0
            && x < pixel_width as isize
            && y < pixel_height as isize
            && solid_pixels[y as usize * pixel_width + x as usize]
    };
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
            if placement.card_thickness > 0.0 {
                let dy = card_sin * placement.card_thickness;
                let dz = card_cos * placement.card_thickness;
                let back = |p: [f32; 3]| [p[0], p[1] - dy, p[2] - dz];
                let front_bl = [world_x0, world_y0, world_z0];
                let front_br = [world_x1, world_y0, world_z0];
                let front_tr = [world_x1, world_y1, world_z1];
                let front_tl = [world_x0, world_y1, world_z1];
                let shade = [0.52, 0.52, 0.52, 1.0];
                append_solid_quad(
                    &mut mesh.solid,
                    [
                        back(front_br),
                        back(front_bl),
                        back(front_tl),
                        back(front_tr),
                    ],
                    [0.0, -card_sin, -card_cos],
                    shade,
                );
                if !solid_at(pixel_x as isize - 1, pixel_y as isize) {
                    append_solid_quad(
                        &mut mesh.solid,
                        [front_bl, back(front_bl), back(front_tl), front_tl],
                        [-1.0, 0.0, 0.0],
                        shade,
                    );
                }
                if !solid_at(pixel_x as isize + 1, pixel_y as isize) {
                    append_solid_quad(
                        &mut mesh.solid,
                        [front_tr, back(front_tr), back(front_br), front_br],
                        [1.0, 0.0, 0.0],
                        shade,
                    );
                }
                if !solid_at(pixel_x as isize, pixel_y as isize - 1) {
                    append_solid_quad(
                        &mut mesh.solid,
                        [front_tl, back(front_tl), back(front_tr), front_tr],
                        [0.0, card_cos, -card_sin],
                        shade,
                    );
                }
                if !solid_at(pixel_x as isize, pixel_y as isize + 1) {
                    append_solid_quad(
                        &mut mesh.solid,
                        [front_br, back(front_br), back(front_bl), front_bl],
                        [0.0, -card_cos, card_sin],
                        shade,
                    );
                }
            }
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

fn is_trapezoid_mound_placement(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: BuildingPlacement,
) -> bool {
    let source = &cells[placement.row * geometry.width + placement.column].source;
    let kanto = placement.width == 16
        && placement.height == 8
        && placement.roof_rows == 6
        && source.tileset_id.as_ref() == "kanto"
        && source.metatile_id == 0x3e;
    let johto = placement.width == 12
        && placement.height == 8
        && placement.roof_rows == 6
        && source.tileset_id.as_ref() == "johto"
        && source.metatile_id == 0x6a;
    let ice_path = placement.width >= 12
        && placement.width % 4 == 0
        && placement.height == 8
        && placement.roof_rows == 6
        && source.tileset_id.as_ref() == "ice_path"
        && source.metatile_id == 0x04;
    kanto || johto || ice_path
}

fn append_pixel_building(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    map_id: &str,
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
    let wall_course_height = (pixel_height - roof_pixels) as f32 * pixel_z_size;
    let (building_x0, _, building_z0, _) = geometry.bounds(placement.column, placement.row);
    let lighthouse_depth = lighthouse_depth_pixels(
        placement.width,
        placement.height,
        placement.roof_rows,
        &cells[placement.row * geometry.width + placement.column].source,
    );
    let tall_lighthouse = lighthouse_depth.is_some();
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
    let johto_closed_mound = placement.width == 12
        && placement.height == 8
        && placement.roof_rows == 6
        && cells[placement.row * geometry.width + placement.column]
            .source
            .tileset_id
            .as_ref()
            == "johto"
        && cells[placement.row * geometry.width + placement.column]
            .source
            .metatile_id
            == 0x6a;
    let ice_path_plateau = placement.width >= 12
        && placement.width % 4 == 0
        && placement.height == 8
        && placement.roof_rows == 6
        && cells[placement.row * geometry.width + placement.column]
            .source
            .tileset_id
            .as_ref()
            == "ice_path"
        && cells[placement.row * geometry.width + placement.column]
            .source
            .metatile_id
            == 0x04;
    let cliff_mound = kanto_cliff_mound || johto_closed_mound || ice_path_plateau;
    let burned_tower_roof = burned_tower_roof_style(
        placement.width,
        placement.height,
        placement.roof_rows,
        &cells[placement.row * geometry.width + placement.column].source,
    );
    let first_building_source = &cells[placement.row * geometry.width + placement.column].source;
    let tower_storeys = tin_tower_storeys(
        map_id,
        placement.width,
        placement.height,
        placement.roof_rows,
        first_building_source,
    );
    let mut wall_height = wall_course_height * tower_storeys as f32;
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
    if cliff_mound {
        wall_height = crate::cave::TRAPEZOID_MOUND_HEIGHT * pixel_z_size;
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
    } else if let Some(depth_pixels) = lighthouse_depth {
        depth_pixels
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
    let inset_house_walls = tower_storeys == 1
        && !tall_lighthouse
        && !cliff_mound
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
        for storey in 0..tower_storeys {
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
                    let top = storey as f32 * wall_course_height
                        + (pixel_height - y) as f32 * pixel_z_size * facade_height_scale;
                    let bottom = top - pixel_z_size * facade_height_scale;
                    let source_x = if storey > 0 && tower_storeys > 1 {
                        tin_tower_upper_source_x(pixel_width, x)
                    } else {
                        x
                    };
                    let is_recessed = recessed[y * pixel_width + source_x];
                    let front_z = facade_z - is_recessed.then_some(recess_depth).unwrap_or(0.0);
                    append_quad(
                        &mut mesh.textured,
                        [
                            [x1, bottom, front_z],
                            [x1, top, front_z],
                            [x0, top, front_z],
                            [x0, bottom, front_z],
                        ],
                        [0.0, 0.0, 1.0],
                        source_pixel_uv(geometry, placement, source_x, y, true),
                        TEXTURED_SHADE,
                    );
                    if is_recessed {
                        let open = |nx: isize, ny: isize| {
                            nx < 0
                                || ny < roof_pixels as isize
                                || nx >= pixel_width as isize
                                || ny >= pixel_height as isize
                                || {
                                    let neighbor_x = if storey > 0 && tower_storeys > 1 {
                                        tin_tower_upper_source_x(pixel_width, nx as usize)
                                    } else {
                                        nx as usize
                                    };
                                    !recessed[ny as usize * pixel_width + neighbor_x]
                                }
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
    }

    // The roof drawing is not a generic front-to-back gable. Following the
    // reference building voxelizer, its silhouette controls elevation across
    // X while the authored roof rows map over depth. A shallow constant-
    // thickness slab preserves the drawn eave instead of turning every house
    // into the same triangular prism.
    let roof_slab_pixels = if let Some(style) = burned_tower_roof {
        style.slab_pixels
    } else if cliff_mound {
        0
    } else {
        roof_pixels.min(4)
    };
    let roof_height_at =
        |x: usize| roof_slab_height(wall_height, roof_slab_pixels, roof_top[x], pixel_z_size);
    let roof_rise = roof_pixels as f32 * pixel_z_size;
    let pitched_height_at_depth =
        |depth: f32| gabled_roof_height(wall_height, roof_rise, depth, roof_depth_pixels as f32);
    if cliff_mound {
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
                if cliff_mound && !inside[source_y * pixel_width + x] {
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

    if !cliff_mound {
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
            for storey in 0..tower_storeys {
                for source_y in roof_pixels..pixel_height {
                    let west_source_x = facade_side_course_x(
                        &inside,
                        &luminance,
                        pixel_width,
                        source_y,
                        darkest,
                        false,
                    );
                    let east_source_x = facade_side_course_x(
                        &inside,
                        &luminance,
                        pixel_width,
                        source_y,
                        darkest,
                        true,
                    );
                    let y_top = storey as f32 * wall_course_height
                        + (pixel_height - source_y) as f32 * pixel_z_size;
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
        }

        // The source drawing supplies a front stack, but the building is a
        // closed volume. Carry that stack onto the rear plane at reduced
        // light, as the reference mesher does for non-front faces. Omitting
        // this plane exposed the ground through Ecruteak's rear eaves and
        // between adjoining traditional roof sections.
        for storey in 0..tower_storeys {
            for source_y in roof_pixels..pixel_height {
                let y_top = storey as f32 * wall_course_height
                    + (pixel_height - source_y) as f32 * pixel_z_size;
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
    }

    if !cliff_mound && !kanto_plan_roof {
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
            append_quad(
                &mut mesh.textured,
                [
                    [rx0 + batter, 0.0, z0 - batter],
                    [rx0, height, z0],
                    [lx0, height, z0],
                    [lx0 - batter, 0.0, z0 - batter],
                ],
                [0.0, 0.0, -1.0],
                [
                    uv(width, slope_pixels),
                    uv(width, 0),
                    uv(0, 0),
                    uv(0, slope_pixels),
                ],
                [0.68, 0.68, 0.68, 1.0],
            );
        }
        if row + 1 == depth {
            // The authored south face occupies the cap-width rectangle.
            // Battering the west/east walls widens only their feet, leaving
            // one triangular opening at either end of that face unless the
            // widened base is explicitly joined back to the facade.
            let facade_bottom = placement.height * SOURCE_TILE_PIXELS;
            append_quad(
                &mut mesh.textured,
                [
                    [lx1, 0.0, z1],
                    [lx1, height, z1],
                    [lx1, height, z1],
                    [lx1 - batter, 0.0, z1],
                ],
                [0.0, 0.0, 1.0],
                [
                    uv(slope_pixels, facade_bottom),
                    uv(slope_pixels, depth),
                    uv(slope_pixels, depth),
                    uv(0, facade_bottom),
                ],
                [0.90, 0.90, 0.90, 1.0],
            );
            append_quad(
                &mut mesh.textured,
                [
                    [rx1 + batter, 0.0, z1],
                    [rx1, height, z1],
                    [rx1, height, z1],
                    [rx1, 0.0, z1],
                ],
                [0.0, 0.0, 1.0],
                [
                    uv(width, facade_bottom),
                    uv(width - slope_pixels, depth),
                    uv(width - slope_pixels, depth),
                    uv(width - slope_pixels, facade_bottom),
                ],
                [0.90, 0.90, 0.90, 1.0],
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

fn append_room_back_wall(mesh: &mut TerrainMeshData, geometry: &GridGeometry, tileset: &str) {
    let x0 = geometry.origin_x;
    let x1 = geometry.origin_x + geometry.width as f32 * geometry.tile_width;
    // Indoor metatiles reserve their northern four-subtile course for wall
    // and fixture artwork. Stand the architectural wall at that authored
    // floor seam, not at the outer render-grid edge behind the void.
    let seam_rows = if tileset == "players_room" { 12.0 } else { 8.0 };
    let wall_rows = 4.0;
    let z = geometry.origin_z + geometry.tile_height * seam_rows - geometry.tile_height * 0.08;
    let height = geometry.tile_height * wall_rows;
    let bottom = -geometry.tile_height * 1.5;
    append_solid_quad(
        &mut mesh.solid,
        [
            [x1, bottom, z],
            [x1, height, z],
            [x0, height, z],
            [x0, bottom, z],
        ],
        [0.0, 0.0, 1.0],
        [0.20, 0.18, 0.15, 1.0],
    );
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
                        None,
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
        CellShape::RampEast {
            west_height,
            east_height,
        } => {
            let scale = geometry.tile_height / SOURCE_TILE_HEIGHT;
            let west_height = west_height * scale;
            let east_height = east_height * scale;
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            let normal = Vec3::new(west_height - east_height, geometry.tile_width, 0.0)
                .normalize()
                .to_array();
            append_quad(
                &mut mesh.textured,
                [
                    [x0, west_height, z1],
                    [x1, east_height, z1],
                    [x1, east_height, z0],
                    [x0, west_height, z0],
                ],
                normal,
                [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
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
        CellShape::FloatingRelief {
            height,
            ground_tile_index,
            base_height,
        } => {
            let replacement = authored_relief_base_cell(cells, shapes, ground_tile_index).ok_or(
                TerrainMeshError::MissingGroundSample {
                    column: column as u32,
                    row: row as u32,
                    tile_index: ground_tile_index,
                },
            )?;
            let ground_height = shapes[replacement].surface_height(geometry.tile_height);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                ground_height,
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
                    base_height * geometry.tile_height / SOURCE_TILE_HEIGHT,
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
    append_grouped_tree(
        mesh,
        images,
        cells,
        shapes,
        geometry,
        TreePlacement {
            column: placement.column,
            row: placement.row,
            width: 2,
            height: 2,
            ground_tile_index: 0x01,
            ground_metatile_id: None,
            base_height: 0.0,
            rounded: false,
            outline_mask: true,
            remove_all_ground: false,
            card_thickness: 0.0,
        },
        claimed,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_house_furniture(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: HouseFurniturePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    if mesh.footing_heights.len() < cells.len() {
        mesh.footing_heights.resize(cells.len(), 0.0);
    }
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        crate::house::HOUSE_FLOOR_TILE,
        None,
        0.0,
        geometry.tile_height,
    )
    .ok_or(TerrainMeshError::MissingGroundSample {
        column: placement.column as u32,
        row: placement.row as u32,
        tile_index: crate::house::HOUSE_FLOOR_TILE,
    })?;
    let top_height = placement.kind.height() * geometry.tile_height / SOURCE_TILE_HEIGHT;
    for local_row in 0..2 {
        for local_column in 0..2 {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            let index = row * geometry.width + column;
            claimed[index] = true;
            mesh.footing_heights[index] = top_height;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
        }
    }

    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let z0 = geometry.origin_z + placement.row as f32 * geometry.tile_height;
    let pixel_x = geometry.tile_width / SOURCE_TILE_PIXELS as f32;
    let pixel_z = geometry.tile_height / SOURCE_TILE_PIXELS as f32;
    let ground = tile_rgba(images, cells[ground_index])?;
    let source_pixel =
        |x: usize, y: usize| -> Result<(&VisualTile, usize, usize), TerrainMeshError> {
            let column = placement.column + x / SOURCE_TILE_PIXELS;
            let row = placement.row + y / SOURCE_TILE_PIXELS;
            Ok((
                cells[row * geometry.width + column],
                x % SOURCE_TILE_PIXELS,
                y % SOURCE_TILE_PIXELS,
            ))
        };
    let uv_pixel = |x: usize, y: usize| {
        let column = placement.column + x / SOURCE_TILE_PIXELS;
        let row = placement.row + y / SOURCE_TILE_PIXELS;
        let (u0, u1, v0, v1) = geometry.uv(column, row);
        let px = x % SOURCE_TILE_PIXELS;
        let py = y % SOURCE_TILE_PIXELS;
        [
            lerp_pixel(u0, u1, px),
            lerp_pixel(u0, u1, px + 1),
            lerp_pixel(v0, v1, py),
            lerp_pixel(v0, v1, py + 1),
        ]
    };

    // The authored stool is a small round seat painted over room floor, not
    // a 16x16 solid cube. As in the reference mod, rows 5..10 form a shallow
    // lid and rows 11..15 fold once into its front/legs. Pixel-level ground
    // rejection keeps the gaps between the legs open.
    for depth_pixel in 0..11 {
        let source_y = 5 + depth_pixel * 6 / 11;
        for source_x in 2..14 {
            let (tile, px, py) = source_pixel(source_x, source_y)?;
            if pixels_equal(tile_rgba(images, tile)?, ground, px, py) {
                continue;
            }
            let [u0, u1, v0, v1] = uv_pixel(source_x, source_y);
            let wx0 = x0 + source_x as f32 * pixel_x;
            let wx1 = wx0 + pixel_x;
            let wz0 = z0 + (3 + depth_pixel) as f32 * pixel_z;
            let wz1 = wz0 + pixel_z;
            append_quad(
                &mut mesh.textured,
                [
                    [wx0, top_height, wz0],
                    [wx0, top_height, wz1],
                    [wx1, top_height, wz1],
                    [wx1, top_height, wz0],
                ],
                [0.0, 1.0, 0.0],
                [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
                TEXTURED_SHADE,
            );
        }
    }
    let front_z = z0 + 14.0 * pixel_z;
    for source_y in 11..16 {
        for source_x in 2..14 {
            let (tile, px, py) = source_pixel(source_x, source_y)?;
            if pixels_equal(tile_rgba(images, tile)?, ground, px, py) {
                continue;
            }
            let [u0, u1, v0, v1] = uv_pixel(source_x, source_y);
            let wx0 = x0 + source_x as f32 * pixel_x;
            let wx1 = wx0 + pixel_x;
            let wy1 = (16 - source_y) as f32 * pixel_z;
            let wy0 = wy1 - pixel_z;
            append_quad(
                &mut mesh.textured,
                [
                    [wx1, wy0, front_z],
                    [wx1, wy1, front_z],
                    [wx0, wy1, front_z],
                    [wx0, wy0, front_z],
                ],
                [0.0, 0.0, 1.0],
                [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                TEXTURED_SHADE,
            );
        }
    }
    Ok(())
}

fn append_house_table(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: HouseTablePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    if mesh.footing_heights.len() < cells.len() {
        mesh.footing_heights.resize(cells.len(), 0.0);
    }
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        placement.ground_tile_index,
        None,
        0.0,
        geometry.tile_height,
    )
    .or_else(|| {
        (cells[placement.row * geometry.width + placement.column]
            .source
            .tileset_id
            .as_ref()
            == "traditional_house")
            .then(|| {
                shapes
                    .iter()
                    .enumerate()
                    .filter(|(_, shape)| {
                        matches!(shape, CellShape::Flat)
                            && shape.surface_height(geometry.tile_height).abs() < f32::EPSILON
                    })
                    .min_by_key(|(index, _)| {
                        let column = *index % geometry.width;
                        let row = *index / geometry.width;
                        (
                            column.abs_diff(placement.column),
                            row.abs_diff(placement.row),
                        )
                    })
                    .map(|(index, _)| index)
            })
            .flatten()
    })
    .ok_or(TerrainMeshError::MissingGroundSample {
        column: placement.column as u32,
        row: placement.row as u32,
        tile_index: placement.ground_tile_index,
    })?;
    let height = placement.height_pixels * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    for local_row in 0..4 {
        for local_column in 0..4 {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            let index = row * geometry.width + column;
            claimed[index] = true;
            mesh.footing_heights[index] = height;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            if local_row < 3 {
                append_top(
                    &mut mesh.textured,
                    [x0, x1, z0, z1],
                    height,
                    (u0, u1, v0, v1),
                );
            } else {
                let top_v1 = lerp_pixel(v0, v1, 4);
                append_top(
                    &mut mesh.textured,
                    [x0, x1, z0, z0 + geometry.tile_height * 0.5],
                    height,
                    (u0, u1, v0, top_v1),
                );
            }
        }
    }

    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + 4.0 * geometry.tile_width;
    let z0 = geometry.origin_z + placement.row as f32 * geometry.tile_height;
    let front_z = z0 + 3.5 * geometry.tile_height;
    for local_column in 0..4 {
        let column = placement.column + local_column;
        let segment_x0 = x0 + local_column as f32 * geometry.tile_width;
        let segment_x1 = segment_x0 + geometry.tile_width;
        let (u0, u1, v0, v1) = geometry.uv(column, placement.row + 3);
        let facade_v0 = lerp_pixel(v0, v1, 4);
        append_quad(
            &mut mesh.textured,
            [
                [segment_x1, 0.0, front_z],
                [segment_x1, height, front_z],
                [segment_x0, height, front_z],
                [segment_x0, 0.0, front_z],
            ],
            [0.0, 0.0, 1.0],
            [[u1, v1], [u1, facade_v0], [u0, facade_v0], [u0, v1]],
            TEXTURED_SHADE,
        );
    }
    for local_row in 0..4 {
        let segment_z0 = z0 + local_row as f32 * geometry.tile_height;
        let segment_z1 = if local_row == 3 {
            front_z
        } else {
            segment_z0 + geometry.tile_height
        };
        let (west_u0, west_u1, v0, v1) = geometry.uv(placement.column, placement.row + local_row);
        let source_v1 = if local_row == 3 {
            lerp_pixel(v0, v1, 4)
        } else {
            v1
        };
        append_quad(
            &mut mesh.textured,
            [
                [x0, 0.0, segment_z1],
                [x0, height, segment_z1],
                [x0, height, segment_z0],
                [x0, 0.0, segment_z0],
            ],
            [-1.0, 0.0, 0.0],
            [
                [west_u0, source_v1],
                [west_u0, v0],
                [lerp_pixel(west_u0, west_u1, 1), v0],
                [lerp_pixel(west_u0, west_u1, 1), source_v1],
            ],
            TEXTURED_SHADE,
        );
        let (east_u0, east_u1, v0, v1) =
            geometry.uv(placement.column + 3, placement.row + local_row);
        let source_v1 = if local_row == 3 {
            lerp_pixel(v0, v1, 4)
        } else {
            v1
        };
        append_quad(
            &mut mesh.textured,
            [
                [x1, 0.0, segment_z0],
                [x1, height, segment_z0],
                [x1, height, segment_z1],
                [x1, 0.0, segment_z1],
            ],
            [1.0, 0.0, 0.0],
            [
                [east_u1, source_v1],
                [east_u1, v0],
                [lerp_pixel(east_u0, east_u1, SOURCE_TILE_PIXELS - 1), v0],
                [
                    lerp_pixel(east_u0, east_u1, SOURCE_TILE_PIXELS - 1),
                    source_v1,
                ],
            ],
            TEXTURED_SHADE,
        );
    }
    Ok(())
}

fn append_house_bookcase(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    append_house_bookcase_with_ground(
        mesh,
        images,
        cells,
        shapes,
        geometry,
        placement,
        claimed,
        crate::house::HOUSE_FLOOR_TILE,
    )
}

fn append_traditional_gift_shop_shelf(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    debug_assert_eq!((placement.width, placement.height), (4, 4));
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
        None,
        0.0,
        geometry.tile_height,
    )
    .ok_or(TerrainMeshError::MissingGroundSample {
        column: placement.column as u32,
        row: placement.row as u32,
        tile_index: crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
    })?;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    for local_row in 0..4 {
        for local_column in 0..4 {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            claimed[row * geometry.width + column] = true;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
        }
    }

    let height = 16.0 * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let north_z = geometry.origin_z + placement.row as f32 * geometry.tile_height;
    let front_z = north_z + 2.0 * geometry.tile_height;
    for local_row in 0..2 {
        for local_column in 0..4 {
            let column = placement.column + local_column;
            let (x0, x1, z0, z1) = geometry.bounds(column, placement.row + local_row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                height,
                geometry.uv(column, placement.row + local_row),
            );
        }
    }
    let pixel_width = placement.width * SOURCE_TILE_PIXELS;
    let pixel_height = 2 * SOURCE_TILE_PIXELS;
    let mut inside = vec![false; pixel_width * pixel_height];
    let mut luminance = vec![0_u16; pixel_width * pixel_height];
    for source_y in 0..pixel_height {
        for source_x in 0..pixel_width {
            let column = placement.column + source_x / SOURCE_TILE_PIXELS;
            let row = placement.row + 2 + source_y / SOURCE_TILE_PIXELS;
            let rgba = tile_rgba(images, cells[row * geometry.width + column])?;
            let pixel_x = source_x % SOURCE_TILE_PIXELS;
            let pixel_y = source_y % SOURCE_TILE_PIXELS;
            let offset = (pixel_y * SOURCE_TILE_PIXELS + pixel_x) * 4;
            inside[source_y * pixel_width + source_x] = rgba[offset + 3] != 0;
            luminance[source_y * pixel_width + source_x] = u16::from(rgba[offset]) * 3
                + u16::from(rgba[offset + 1]) * 6
                + u16::from(rgba[offset + 2]);
        }
    }
    let darkest = luminance
        .iter()
        .enumerate()
        .filter_map(|(index, value)| inside[index].then_some(*value))
        .min()
        .unwrap_or(0);
    let recessed = facade_recess_mask(&inside, &luminance, pixel_width, pixel_height, 0, darkest);
    let recess_depth = geometry.tile_height / SOURCE_TILE_HEIGHT;
    for band in 0..2 {
        let top = height - band as f32 * geometry.tile_height;
        for local_column in 0..4 {
            let column = placement.column + local_column;
            let x0 = geometry.origin_x + column as f32 * geometry.tile_width;
            let (u0, u1, v0, v1) = geometry.uv(column, placement.row + 2 + band);
            for pixel_y in 0..SOURCE_TILE_PIXELS {
                let world_top = top - geometry.tile_height * pixel_y as f32 / SOURCE_TILE_HEIGHT;
                let world_bottom =
                    top - geometry.tile_height * (pixel_y + 1) as f32 / SOURCE_TILE_HEIGHT;
                let pv0 = lerp_pixel(v0, v1, pixel_y);
                let pv1 = lerp_pixel(v0, v1, pixel_y + 1);
                for pixel_x in 0..SOURCE_TILE_PIXELS {
                    let source_x = local_column * SOURCE_TILE_PIXELS + pixel_x;
                    let source_y = band * SOURCE_TILE_PIXELS + pixel_y;
                    let world_x0 = x0 + geometry.tile_width * pixel_x as f32 / SOURCE_TILE_HEIGHT;
                    let world_x1 =
                        x0 + geometry.tile_width * (pixel_x + 1) as f32 / SOURCE_TILE_HEIGHT;
                    let pu0 = lerp_pixel(u0, u1, pixel_x);
                    let pu1 = lerp_pixel(u0, u1, pixel_x + 1);
                    let z = front_z
                        - if recessed[source_y * pixel_width + source_x] {
                            recess_depth
                        } else {
                            0.0
                        };
                    append_quad(
                        &mut mesh.textured,
                        [
                            [world_x1, world_bottom, z],
                            [world_x1, world_top, z],
                            [world_x0, world_top, z],
                            [world_x0, world_bottom, z],
                        ],
                        [0.0, 0.0, 1.0],
                        [[pu1, pv1], [pu1, pv0], [pu0, pv0], [pu0, pv1]],
                        TEXTURED_SHADE,
                    );
                }
            }
        }
    }
    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    append_solid_quad(
        &mut mesh.solid,
        [
            [x0, 0.0, front_z],
            [x0, height, front_z],
            [x0, height, north_z],
            [x0, 0.0, north_z],
        ],
        [-1.0, 0.0, 0.0],
        solid_color(SolidKind::Prop, Direction::West),
    );
    append_solid_quad(
        &mut mesh.solid,
        [
            [x1, 0.0, north_z],
            [x1, height, north_z],
            [x1, height, front_z],
            [x1, 0.0, front_z],
        ],
        [1.0, 0.0, 0.0],
        solid_color(SolidKind::Prop, Direction::East),
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_house_bookcase_with_ground(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
    ground_tile_index: u16,
) -> Result<(), TerrainMeshError> {
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        ground_tile_index,
        None,
        0.0,
        geometry.tile_height,
    )
    .ok_or(TerrainMeshError::MissingGroundSample {
        column: placement.column as u32,
        row: placement.row as u32,
        tile_index: ground_tile_index,
    })?;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            claimed[row * geometry.width + column] = true;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
        }
    }

    let plane_z =
        geometry.origin_z + (placement.row + placement.height) as f32 * geometry.tile_height;
    let depth = geometry.tile_height;
    let facade_height = 23.0 * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let pixel_width = placement.width * SOURCE_TILE_PIXELS;
    let pixel_height = placement.height * SOURCE_TILE_PIXELS;
    let mut inside = vec![false; pixel_width * pixel_height];
    let mut luminance = vec![0_u16; pixel_width * pixel_height];
    for source_y in 0..pixel_height {
        for source_x in 0..pixel_width {
            let column = placement.column + source_x / SOURCE_TILE_PIXELS;
            let row = placement.row + source_y / SOURCE_TILE_PIXELS;
            let tile = cells[row * geometry.width + column];
            let rgba = tile_rgba(images, tile)?;
            let pixel_x = source_x % SOURCE_TILE_PIXELS;
            let pixel_y = source_y % SOURCE_TILE_PIXELS;
            let offset = (pixel_y * SOURCE_TILE_PIXELS + pixel_x) * 4;
            inside[source_y * pixel_width + source_x] = rgba[offset + 3] != 0;
            luminance[source_y * pixel_width + source_x] = u16::from(rgba[offset]) * 3
                + u16::from(rgba[offset + 1]) * 6
                + u16::from(rgba[offset + 2]);
        }
    }
    let darkest = luminance
        .iter()
        .enumerate()
        .filter_map(|(index, value)| inside[index].then_some(*value))
        .min()
        .unwrap_or(0);
    // As in the reference renderer, only small non-outline components sealed
    // inside the facade are recessed. This leaves the cabinet frame proud
    // while shelf contents and door panels sit one source pixel behind it.
    let recessed = facade_recess_mask(&inside, &luminance, pixel_width, pixel_height, 9, darkest);
    let recess_depth = geometry.tile_height / SOURCE_TILE_HEIGHT;
    for local_column in 0..placement.width {
        let column = placement.column + local_column;
        let x0 = geometry.origin_x + column as f32 * geometry.tile_width;
        let x1 = x0 + geometry.tile_width;
        // Rows 0..=9 are the authored top/depth course. Row 9 is also the
        // cabinet's one-pixel front rim, so it participates in both planes at
        // the fold. Stopping at row 8 made every shared house bookcase one
        // pixel too shallow and erased that corner from the lid.
        for source_y in 0..10 {
            let source_row = placement.row + source_y / SOURCE_TILE_PIXELS;
            let pixel_y = source_y % SOURCE_TILE_PIXELS;
            let (u0, u1, v0, v1) = geometry.uv(column, source_row);
            let sv0 = lerp_pixel(v0, v1, pixel_y);
            let sv1 = lerp_pixel(v0, v1, pixel_y + 1);
            let z0 = plane_z - depth + depth * source_y as f32 / 10.0;
            let z1 = plane_z - depth + depth * (source_y + 1) as f32 / 10.0;
            append_quad(
                &mut mesh.textured,
                [
                    [x0, facade_height, z0],
                    [x0, facade_height, z1],
                    [x1, facade_height, z1],
                    [x1, facade_height, z0],
                ],
                [0.0, 1.0, 0.0],
                [[u0, sv0], [u0, sv1], [u1, sv1], [u1, sv0]],
                TEXTURED_SHADE,
            );
        }
        for source_y in 9..32 {
            let source_row = placement.row + source_y / SOURCE_TILE_PIXELS;
            let pixel_y = source_y % SOURCE_TILE_PIXELS;
            let (u0, u1, v0, v1) = geometry.uv(column, source_row);
            let sv0 = lerp_pixel(v0, v1, pixel_y);
            let sv1 = lerp_pixel(v0, v1, pixel_y + 1);
            let top = (32 - source_y) as f32 * geometry.tile_height / SOURCE_TILE_HEIGHT;
            let bottom = (31 - source_y) as f32 * geometry.tile_height / SOURCE_TILE_HEIGHT;
            let source_x = local_column * SOURCE_TILE_PIXELS;
            let recessed_row = &recessed[source_y * pixel_width + source_x
                ..source_y * pixel_width + source_x + SOURCE_TILE_PIXELS];
            if recessed_row.iter().all(|pixel| !pixel) {
                append_quad(
                    &mut mesh.textured,
                    [
                        [x1, bottom, plane_z],
                        [x1, top, plane_z],
                        [x0, top, plane_z],
                        [x0, bottom, plane_z],
                    ],
                    [0.0, 0.0, 1.0],
                    [[u1, sv1], [u1, sv0], [u0, sv0], [u0, sv1]],
                    TEXTURED_SHADE,
                );
                continue;
            }
            for pixel_x in 0..SOURCE_TILE_PIXELS {
                let world_x0 = x0 + geometry.tile_width * pixel_x as f32 / SOURCE_TILE_HEIGHT;
                let world_x1 = x0 + geometry.tile_width * (pixel_x + 1) as f32 / SOURCE_TILE_HEIGHT;
                let pu0 = lerp_pixel(u0, u1, pixel_x);
                let pu1 = lerp_pixel(u0, u1, pixel_x + 1);
                let is_recessed = recessed_row[pixel_x];
                let front_z = plane_z - if is_recessed { recess_depth } else { 0.0 };
                append_quad(
                    &mut mesh.textured,
                    [
                        [world_x1, bottom, front_z],
                        [world_x1, top, front_z],
                        [world_x0, top, front_z],
                        [world_x0, bottom, front_z],
                    ],
                    [0.0, 0.0, 1.0],
                    [[pu1, sv1], [pu1, sv0], [pu0, sv0], [pu0, sv1]],
                    TEXTURED_SHADE,
                );
            }
        }
    }
    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    for source_y in 9..32 {
        let source_row = placement.row + source_y / SOURCE_TILE_PIXELS;
        let pixel_y = source_y % SOURCE_TILE_PIXELS;
        let (west_u0, west_u1, west_v0, west_v1) = geometry.uv(placement.column, source_row);
        let (east_u0, east_u1, east_v0, east_v1) =
            geometry.uv(placement.column + placement.width - 1, source_row);
        let west_sv0 = lerp_pixel(west_v0, west_v1, pixel_y);
        let west_sv1 = lerp_pixel(west_v0, west_v1, pixel_y + 1);
        let east_sv0 = lerp_pixel(east_v0, east_v1, pixel_y);
        let east_sv1 = lerp_pixel(east_v0, east_v1, pixel_y + 1);
        let top = (32 - source_y) as f32 * geometry.tile_height / SOURCE_TILE_HEIGHT;
        let bottom = (31 - source_y) as f32 * geometry.tile_height / SOURCE_TILE_HEIGHT;
        append_quad(
            &mut mesh.textured,
            [
                [x0, bottom, plane_z],
                [x0, top, plane_z],
                [x0, top, plane_z - depth],
                [x0, bottom, plane_z - depth],
            ],
            [-1.0, 0.0, 0.0],
            [
                [west_u0, west_sv1],
                [west_u0, west_sv0],
                [lerp_pixel(west_u0, west_u1, 1), west_sv0],
                [lerp_pixel(west_u0, west_u1, 1), west_sv1],
            ],
            TEXTURED_SHADE,
        );
        append_quad(
            &mut mesh.textured,
            [
                [x1, bottom, plane_z - depth],
                [x1, top, plane_z - depth],
                [x1, top, plane_z],
                [x1, bottom, plane_z],
            ],
            [1.0, 0.0, 0.0],
            [
                [
                    lerp_pixel(east_u0, east_u1, SOURCE_TILE_PIXELS - 1),
                    east_sv1,
                ],
                [
                    lerp_pixel(east_u0, east_u1, SOURCE_TILE_PIXELS - 1),
                    east_sv0,
                ],
                [east_u1, east_sv0],
                [east_u1, east_sv1],
            ],
            TEXTURED_SHADE,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_shallow_top_group(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    geometry: &GridGeometry,
    placement: TreePlacement,
    claimed: &mut [bool],
    ground_tile_index: u16,
    source_height: f32,
) -> Result<(), TerrainMeshError> {
    if mesh.footing_heights.len() < cells.len() {
        mesh.footing_heights.resize(cells.len(), 0.0);
    }
    let ground_index = authored_surface_cell(
        cells,
        shapes,
        ground_tile_index,
        None,
        0.0,
        geometry.tile_height,
    )
    .ok_or(TerrainMeshError::MissingGroundSample {
        column: placement.column as u32,
        row: placement.row as u32,
        tile_index: ground_tile_index,
    })?;
    let height = source_height * geometry.tile_height / SOURCE_TILE_HEIGHT;
    for local_row in 0..placement.height {
        for local_column in 0..placement.width {
            let column = placement.column + local_column;
            let row = placement.row + local_row;
            let index = row * geometry.width + column;
            claimed[index] = true;
            mesh.footing_heights[index] = height;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                height,
                geometry.uv(column, row),
            );
        }
    }

    // A cushion's source art is a top view. Its sides are only the outermost
    // source-pixel strips, never a repeated or stretched recognizable tile.
    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    let z0 = geometry.origin_z + placement.row as f32 * geometry.tile_height;
    let z1 = z0 + placement.height as f32 * geometry.tile_height;
    for local_column in 0..placement.width {
        let segment_x0 = x0 + local_column as f32 * geometry.tile_width;
        let segment_x1 = segment_x0 + geometry.tile_width;
        let (u0, u1, north_v0, north_v1) =
            geometry.uv(placement.column + local_column, placement.row);
        append_quad(
            &mut mesh.textured,
            [
                [segment_x0, 0.0, z0],
                [segment_x0, height, z0],
                [segment_x1, height, z0],
                [segment_x1, 0.0, z0],
            ],
            [0.0, 0.0, -1.0],
            [
                [u0, lerp_pixel(north_v0, north_v1, 1)],
                [u0, north_v0],
                [u1, north_v0],
                [u1, lerp_pixel(north_v0, north_v1, 1)],
            ],
            TEXTURED_SHADE,
        );
        let (u0, u1, south_v0, south_v1) = geometry.uv(
            placement.column + local_column,
            placement.row + placement.height - 1,
        );
        append_quad(
            &mut mesh.textured,
            [
                [segment_x1, 0.0, z1],
                [segment_x1, height, z1],
                [segment_x0, height, z1],
                [segment_x0, 0.0, z1],
            ],
            [0.0, 0.0, 1.0],
            [
                [u1, south_v1],
                [u1, lerp_pixel(south_v0, south_v1, SOURCE_TILE_PIXELS - 1)],
                [u0, lerp_pixel(south_v0, south_v1, SOURCE_TILE_PIXELS - 1)],
                [u0, south_v1],
            ],
            TEXTURED_SHADE,
        );
    }
    for local_row in 0..placement.height {
        let segment_z0 = z0 + local_row as f32 * geometry.tile_height;
        let segment_z1 = segment_z0 + geometry.tile_height;
        let (west_u0, west_u1, v0, v1) = geometry.uv(placement.column, placement.row + local_row);
        append_quad(
            &mut mesh.textured,
            [
                [x0, 0.0, segment_z1],
                [x0, height, segment_z1],
                [x0, height, segment_z0],
                [x0, 0.0, segment_z0],
            ],
            [-1.0, 0.0, 0.0],
            [
                [west_u0, v1],
                [west_u0, v0],
                [lerp_pixel(west_u0, west_u1, 1), v0],
                [lerp_pixel(west_u0, west_u1, 1), v1],
            ],
            TEXTURED_SHADE,
        );
        let (east_u0, east_u1, v0, v1) = geometry.uv(
            placement.column + placement.width - 1,
            placement.row + local_row,
        );
        append_quad(
            &mut mesh.textured,
            [
                [x1, 0.0, segment_z0],
                [x1, height, segment_z0],
                [x1, height, segment_z1],
                [x1, 0.0, segment_z1],
            ],
            [1.0, 0.0, 0.0],
            [
                [east_u1, v1],
                [east_u1, v0],
                [lerp_pixel(east_u0, east_u1, SOURCE_TILE_PIXELS - 1), v0],
                [lerp_pixel(east_u0, east_u1, SOURCE_TILE_PIXELS - 1), v1],
            ],
            TEXTURED_SHADE,
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
    let base = crate::profile::GROUND_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;
    let drop = crate::cave::CAVE_ROCK_HEIGHT * geometry.tile_height / SOURCE_TILE_HEIGHT;
    for source_row in 0..placement.height {
        // The source rows describe the cascade's north-to-south footprint,
        // not independent eight-pixel elevation courses. Preserve each live
        // animated row once on one bounded slope spanning exactly one cave
        // level; folding an N-row waterfall upright made its height N*8.
        let north_fraction = 1.0 - source_row as f32 / placement.height as f32;
        let south_fraction = 1.0 - (source_row + 1) as f32 / placement.height as f32;
        let north_height = base + drop * north_fraction;
        let south_height = base + drop * south_fraction;
        for offset_column in 0..placement.width {
            let column = placement.column + offset_column;
            let row = placement.row + source_row;
            let index = row * geometry.width + column;
            let (x0, x1, z0, z1) = geometry.bounds(column, row);
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            let normal = Vec3::new(0.0, z1 - z0, north_height - south_height)
                .normalize()
                .to_array();
            append_quad(
                mesh,
                [
                    [x0, north_height, z0],
                    [x0, south_height, z1],
                    [x1, south_height, z1],
                    [x1, north_height, z0],
                ],
                normal,
                [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
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
    metatile_id: Option<u16>,
    height: f32,
    tile_height: f32,
) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            cells[*index].source.tile_index == tile_index
                && metatile_id.is_none_or(|id| cells[*index].source.metatile_id == id)
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

fn append_east_ramp_sidewalls(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    shapes: &[CellShape],
    column: usize,
    row: usize,
    west_height: f32,
    east_height: f32,
) {
    let (x0, x1, z0, z1) = geometry.bounds(column, row);
    let is_east_ramp = |column: usize, row: usize| {
        matches!(
            shapes[row * geometry.width + column],
            CellShape::RampEast { .. }
        )
    };
    if row + 1 >= geometry.height || !is_east_ramp(column, row + 1) {
        append_solid_quad(
            &mut mesh.solid,
            [
                [x0, 0.0, z1],
                [x1, 0.0, z1],
                [x1, east_height, z1],
                [x0, west_height, z1],
            ],
            [0.0, 0.0, 1.0],
            solid_color(SolidKind::Bank, Direction::South),
        );
    }
    if row == 0 || !is_east_ramp(column, row - 1) {
        append_solid_quad(
            &mut mesh.solid,
            [
                [x0, 0.0, z0],
                [x0, west_height, z0],
                [x1, east_height, z0],
                [x1, 0.0, z0],
            ],
            [0.0, 0.0, -1.0],
            solid_color(SolidKind::Bank, Direction::North),
        );
    }
    if column + 1 >= geometry.width || !is_east_ramp(column + 1, row) {
        append_solid_quad(
            &mut mesh.solid,
            [
                [x1, 0.0, z1],
                [x1, 0.0, z0],
                [x1, east_height, z0],
                [x1, east_height, z1],
            ],
            [1.0, 0.0, 0.0],
            solid_color(SolidKind::Bank, Direction::East),
        );
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
    if let CellShape::RampEast {
        west_height,
        east_height,
    } = shapes[index]
    {
        let scale = geometry.tile_height / SOURCE_TILE_HEIGHT;
        append_east_ramp_sidewalls(
            mesh,
            geometry,
            shapes,
            column,
            row,
            west_height * scale,
            east_height * scale,
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
                    // Outside the published grid is unknown continuation,
                    // never evidence for an exposed mound corner.
                    top
                } else {
                    shapes[lateral_row as usize * geometry.width + lateral_column as usize]
                        .surface_height(geometry.tile_height)
                }
            };
            let (left, right) = direction.lateral_offsets();
            append_bank_run_side(
                &mut mesh.textured,
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
                bank_taper(&cells[index].source, shapes[index]),
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

fn bank_taper(source: &VisualTileSource, shape: CellShape) -> Option<[f32; 2]> {
    // A profiled cliff is a geometric class, not a regional exception. Once
    // its exact authored cells resolve to cliff height, every exposed edge
    // uses the same narrow-cap/wide-foot silhouette. This does not promote
    // unknown art: unprofiled cells are still Flat and never enter this path.
    if shape.surface_height(SOURCE_TILE_HEIGHT) >= crate::profile::MOUNTAIN_CLIFF_HEIGHT {
        return Some([SOURCE_TILE_HEIGHT * 0.25, 0.0]);
    }
    // Exact source identities only answer "is this authored as a mound?".
    // Once recognized, Kanto, Johto, cave, and Ice Path all use the same
    // Route 2 trapezoid. No region-specific dimensions are permitted.
    let authored_mound = matches!(source.tileset_id.as_ref(), "cave" | "dark_cave")
        && (shape.surface_height(SOURCE_TILE_HEIGHT) - crate::cave::CAVE_ROCK_HEIGHT).abs()
            < f32::EPSILON
        || source.tileset_id.as_ref() == "ice_path" && source.metatile_id == 0x19
        || matches!(source.tileset_id.as_ref(), "johto" | "johto_modern")
            && source.metatile_id == 0x0a
        || source.tileset_id.as_ref() == "kanto"
            && matches!(
                source.metatile_id,
                0x3e | 0x3f | 0x3b | 0x24 | 0x06 | 0x57 | 0x25
            );
    if authored_mound {
        return Some([SOURCE_TILE_HEIGHT * 0.25, 0.0]);
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
        let authored_source =
            authored_bank_face_cell(cells, shapes, geometry, run, column, direction, local_band);
        let source = authored_source.unwrap_or(fallback_row * geometry.width + column);
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
        let corner_source = &cells[source].source;
        let uses_authored_corner_chamfer =
            matches!(corner_source.tileset_id.as_ref(), "johto" | "johto_modern")
                && corner_source.metatile_id == 0x0a
                || corner_source.tileset_id.as_ref() == "kanto"
                    && matches!(
                        corner_source.metatile_id,
                        0x3e | 0x3f | 0x3b | 0x24 | 0x06 | 0x57 | 0x25
                    );
        if taper.is_some()
            && uses_authored_corner_chamfer
            && matches!(direction, Direction::North | Direction::South)
        {
            append_rock_platform_corner_faces(
                textured_mesh,
                direction,
                [x0, x1, z0, z1],
                band_bottom,
                band_top,
                bottom_inset,
                top_inset,
                left_open,
                right_open,
                [u0, u1, cropped_v0, cropped_v1],
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
    left_open: bool,
    right_open: bool,
    source_uv: [f32; 4],
    subtile_column: u8,
    subtile_row: u8,
) {
    let [x0, x1, z0, z1] = bounds;
    let [u0, u1, v0, v1] = source_uv;
    let edge_u = u0 + (u1 - u0) / SOURCE_TILE_PIXELS as f32;
    let shade = match direction {
        Direction::North => 0.68,
        Direction::South => 0.90,
        Direction::West => 0.72,
        Direction::East => 0.84,
    };
    let mut corner = |mut positions: [[f32; 3]; 4]| {
        let face_normal = |points: &[[f32; 3]; 4]| {
            let edge_a = Vec3::from_array(points[1]) - Vec3::from_array(points[0]);
            let edge_b = Vec3::from_array(points[2]) - Vec3::from_array(points[0]);
            edge_a.cross(edge_b).normalize()
        };
        if positions[1] == positions[2] {
            positions = [positions[0], positions[1], positions[3], positions[3]];
        }
        let mut normal = face_normal(&positions);
        if normal.y < 0.0 {
            positions.reverse();
            normal = face_normal(&positions);
        }
        append_quad(
            mesh,
            positions,
            normal.to_array(),
            [[u0, v1], [u0, v0], [edge_u, v0], [edge_u, v1]],
            [shade, shade, shade, 1.0],
        );
    };
    match direction {
        Direction::North => {
            if right_open && subtile_column == 0 && subtile_row == 0 {
                corner([
                    [x0, band_bottom, z0 + bottom_inset],
                    [x0, band_top, z0 + top_inset],
                    [x0 + top_inset, band_top, z0],
                    [x0 + bottom_inset, band_bottom, z0],
                ]);
            }
            if left_open && subtile_column == 3 && subtile_row == 0 {
                corner([
                    [x1 - bottom_inset, band_bottom, z0],
                    [x1 - top_inset, band_top, z0],
                    [x1, band_top, z0 + top_inset],
                    [x1, band_bottom, z0 + bottom_inset],
                ]);
            }
        }
        Direction::South => {
            if left_open && subtile_column == 0 && subtile_row == 3 {
                corner([
                    [x0 + bottom_inset, band_bottom, z1],
                    [x0 + top_inset, band_top, z1],
                    [x0, band_top, z1 - top_inset],
                    [x0, band_bottom, z1 - bottom_inset],
                ]);
            }
            if right_open && subtile_column == 3 && subtile_row == 3 {
                corner([
                    [x1, band_bottom, z1 - bottom_inset],
                    [x1, band_top, z1 - top_inset],
                    [x1 - top_inset, band_top, z1],
                    [x1 - bottom_inset, band_bottom, z1],
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
    fn player_bedroom_wall_stays_at_compact_house_height() {
        let mut sources = Vec::new();
        for row in 0..2 {
            for column in 0..16 {
                sources.push(source_for_tileset(
                    "players_room",
                    0x04,
                    (column % 4) as u8,
                    row as u8,
                    0x01,
                ));
            }
        }
        let frame = frame(16, 2, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let shapes = vec![CellShape::Flat; cells.len()];
        let geometry = GridGeometry {
            width: 16,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let mut mesh = TerrainMeshData::default();
        let mut claimed = vec![false; cells.len()];
        append_player_room_wall(&mut mesh, &cells, &shapes, &geometry, &mut claimed)
            .expect("complete player-room wall course should mesh");

        let max_height = mesh
            .textured
            .positions
            .iter()
            .map(|position| position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(max_height, 32.0);
        assert_eq!(mesh.textured.quad_count(), 16 * 4);
    }

    #[test]
    fn traditional_gift_shop_shelf_uses_two_top_rows_and_two_front_rows() {
        let drawing = [
            [0x06, 0x07, 0x07, 0x20],
            [0x16, 0x17, 0x17, 0x30],
            [0x21, 0x27, 0x27, 0x28],
            [0x31, 0x37, 0x37, 0x38],
        ];
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                sources.push(source_for_tileset(
                    "traditional_house",
                    0x02,
                    column,
                    row,
                    drawing[usize::from(row)][usize::from(column)],
                ));
            }
        }
        // The dedicated mesher needs one faithful floor sample outside the
        // shelf drawing. Use a fifth column without changing shelf topology.
        let mut expanded = Vec::new();
        for row in 0..4 {
            expanded.extend(
                sources[usize::from(row) * 4..usize::from(row) * 4 + 4]
                    .iter()
                    .cloned(),
            );
            expanded.push(source_for_tileset("traditional_house", 0x20, 0, row, 0x50));
        }
        let shelf_frame = frame(5, 4, expanded);
        let cells = shelf_frame.tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 5,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let placement = traditional_gift_shop_shelf_placements("MahoganyMart1F", &cells, &geometry)
            .into_iter()
            .next()
            .expect("complete shelf placement");
        let mut mesh = TerrainMeshData::default();
        let mut claimed = vec![false; cells.len()];
        let mut samples = TerrainImageSamples::default();
        for tile in &shelf_frame.tiles {
            let mut rgba = [210, 210, 210, 255].repeat(64);
            if tile.source.subtile_row >= 2 {
                // A small framed display field exercises the same measured
                // relief path as the live shelf art.
                for y in 2..6 {
                    for x in 2..6 {
                        let offset = (y * 8 + x) * 4;
                        let value = if x == 2 || x == 5 || y == 2 || y == 5 {
                            0
                        } else {
                            120
                        };
                        rgba[offset..offset + 3].fill(value);
                    }
                }
            }
            samples
                .pixels
                .insert(tile.texture.id(), TileImageSample::Rgba(rgba));
        }
        append_traditional_gift_shop_shelf(
            &mut mesh,
            &samples,
            &cells,
            &vec![CellShape::Flat; cells.len()],
            &geometry,
            placement,
            &mut claimed,
        )
        .expect("gift-shop shelf should mesh");
        let max_height = mesh
            .textured
            .positions
            .iter()
            .map(|position| position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(max_height, 16.0);
        assert!(mesh.textured.quad_count() > 32);
        assert!(
            mesh.textured
                .positions
                .iter()
                .any(|position| position[2] < 16.0),
            "enclosed merchandise fields sit behind their proud frames"
        );
        assert_eq!(claimed.iter().filter(|claimed| **claimed).count(), 16);
    }

    #[test]
    fn cave_keeps_a_continuous_faithful_floor_below_raised_geometry() {
        let mesh = build_terrain_mesh(&frame(
            1,
            1,
            vec![source_for_tileset("cave", 0x01, 0, 0, 0x16)],
        ))
        .expect("cave floor should mesh");
        assert_eq!(
            mesh.textured.quad_count(),
            2,
            "one faithful cave underlay plus the visible authored cell"
        );
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
    fn adjacent_house_stool_is_not_mistaken_for_a_partial_table() {
        let sources = vec![
            source_for_tileset("house", 0x01, 0, 2, 0x02),
            source_for_tileset("house", 0x01, 1, 2, 0x03),
            source_for_tileset("house", 0x01, 2, 2, 0x26),
            source_for_tileset("house", 0x01, 3, 2, 0x27),
            source_for_tileset("house", 0x01, 0, 3, 0x12),
            source_for_tileset("house", 0x01, 1, 3, 0x13),
            source_for_tileset("house", 0x01, 2, 3, 0x36),
            source_for_tileset("house", 0x01, 3, 3, 0x2f),
        ];
        let frame = frame(4, 2, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 4,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            house_furniture_placements(&cells, &geometry),
            vec![HouseFurniturePlacement {
                column: 0,
                row: 0,
                kind: crate::house::FurnitureKind::Stool,
            }]
        );
        assert!(house_table_placements(&cells, &geometry).is_empty());
    }

    #[test]
    fn ordinary_house_table_is_one_complete_four_by_four_drawing() {
        let drawing = [
            [0x26, 0x27, 0x27, 0x29],
            [0x36, 0x2f, 0x2f, 0x39],
            [0x36, 0x2f, 0x2f, 0x39],
            [0x3c, 0x3a, 0x3a, 0x3b],
        ];
        let sources = drawing
            .into_iter()
            .enumerate()
            .flat_map(|(row, tiles)| {
                tiles.into_iter().enumerate().map(move |(column, tile)| {
                    source_for_tileset("house", 0x01, column as u8, row as u8, tile)
                })
            })
            .collect();
        let complete = frame(4, 4, sources);
        let cells: Vec<_> = complete.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            house_table_placements(&cells, &geometry),
            vec![HouseTablePlacement {
                column: 0,
                row: 0,
                ground_tile_index: crate::house::HOUSE_FLOOR_TILE,
                height_pixels: 6.0,
            }]
        );
    }

    #[test]
    fn player_family_table_is_one_complete_four_by_four_drawing() {
        let drawing = [
            [0x23, 0x22, 0x22, 0x24],
            [0x25, 0x15, 0x15, 0x35],
            [0x25, 0x15, 0x15, 0x35],
            [0x33, 0x32, 0x32, 0x34],
        ];
        let sources = drawing
            .into_iter()
            .enumerate()
            .flat_map(|(row, tiles)| {
                tiles.into_iter().enumerate().map(move |(column, tile)| {
                    source_for_tileset("players_house", 0x08, column as u8, row as u8, tile)
                })
            })
            .collect();
        let complete = frame(4, 4, sources);
        let cells: Vec<_> = complete.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            house_table_placements(&cells, &geometry),
            vec![HouseTablePlacement {
                column: 0,
                row: 0,
                ground_tile_index: crate::house::HOUSE_FLOOR_TILE,
                height_pixels: 6.0,
            }]
        );
    }

    #[test]
    fn traditional_low_table_is_one_complete_four_by_four_drawing() {
        let drawing = [
            [0x23, 0x22, 0x22, 0x24],
            [0x42, 0x15, 0x15, 0x43],
            [0x42, 0x15, 0x15, 0x43],
            [0x33, 0x32, 0x32, 0x34],
        ];
        let sources = drawing
            .into_iter()
            .enumerate()
            .flat_map(|(row, tiles)| {
                tiles.into_iter().enumerate().map(move |(column, tile)| {
                    source_for_tileset("traditional_house", 0x1c, column as u8, row as u8, tile)
                })
            })
            .collect();
        let complete = frame(4, 4, sources);
        let cells: Vec<_> = complete.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            house_table_placements(&cells, &geometry),
            vec![HouseTablePlacement {
                column: 0,
                row: 0,
                ground_tile_index: crate::house::TRADITIONAL_HOUSE_FLOOR_TILE,
                height_pixels: 4.0,
            }]
        );
    }

    #[test]
    fn dark_cave_diagonal_is_one_complete_corner_not_four_tile_boxes() {
        let sources = vec![
            source_for_tileset("dark_cave", 0x10, 2, 2, 0x0a),
            source_for_tileset("dark_cave", 0x10, 3, 2, 0x26),
            source_for_tileset("dark_cave", 0x10, 2, 3, 0x17),
            source_for_tileset("dark_cave", 0x10, 3, 3, 0x0a),
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
            diagonal_cave_corner_placements(&cells, &geometry),
            vec![DiagonalCaveCornerPlacement {
                column: 0,
                row: 0,
                corner: crate::cave::DiagonalCorner::SouthEast,
            }]
        );

        let mut incomplete_sources = sources;
        incomplete_sources[3].tile_index = 0x16;
        let incomplete = frame(2, 2, incomplete_sources);
        let incomplete_cells: Vec<_> = incomplete.tiles.iter().collect();
        assert!(diagonal_cave_corner_placements(&incomplete_cells, &geometry).is_empty());
    }

    #[test]
    fn cave_diagonal_corner_is_a_closed_rock_prism() {
        let complete = frame(
            3,
            2,
            vec![
                source_for_tileset("cave", 0x10, 2, 2, 0x0a),
                source_for_tileset("cave", 0x10, 3, 2, 0x26),
                source_for_tileset("cave", 0x01, 0, 0, 0x16),
                source_for_tileset("cave", 0x10, 2, 3, 0x17),
                source_for_tileset("cave", 0x10, 3, 3, 0x0a),
                source_for_tileset("cave", 0x01, 1, 0, 0x16),
            ],
        );
        let cells: Vec<_> = complete.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 3,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let placement = diagonal_cave_corner_placements(&cells, &geometry)[0];
        let mut mesh = TerrainMeshData::default();
        let shapes = vec![CellShape::Flat; cells.len()];
        let mut claimed = vec![false; cells.len()];

        append_diagonal_cave_corner(
            &mut mesh,
            &cells,
            &shapes,
            &geometry,
            placement,
            &mut claimed,
        )
        .expect("a complete cave corner with a ground sample should mesh");

        assert_eq!(
            mesh.solid.quad_count(),
            0,
            "diagonal closures must use live cave art, not solid-color fins"
        );
        assert_eq!(
            mesh.textured.indices.len() / 3,
            16,
            "ground, cap, diagonal face, and two live edge-strip closures"
        );
        assert_eq!(&claimed[..2], &[true, true]);
        assert_eq!(&claimed[3..5], &[true, true]);
    }

    #[test]
    fn coverage_auditor_reports_game_corner_stools_as_cutout_cards() {
        let sources = vec![
            source_for_tileset("game_corner", 0x05, 2, 0, 0x0a),
            source_for_tileset("game_corner", 0x05, 3, 0, 0x0b),
            source_for_tileset("game_corner", 0x05, 2, 1, 0x1a),
            source_for_tileset("game_corner", 0x05, 3, 1, 0x1b),
        ];
        let frame = frame(2, 2, sources);
        assert_eq!(
            audit_cell_coverage_on_map("GoldenrodGameCorner", &frame.tiles, 2, 2)
                .expect("complete casino stool should audit"),
            vec![CellCoverageKind::Cutout; 4]
        );
    }

    #[test]
    fn coverage_auditor_reports_player_house_bookcases_as_cutout_geometry() {
        let drawing = [
            [0x0e, 0x0f, 0x0e, 0x0f],
            [0x1e, 0x1f, 0x2e, 0x2f],
            [0x2e, 0x2f, 0x08, 0x09],
            [0x18, 0x19, 0x3a, 0x3b],
        ];
        let mut sources = Vec::new();
        for row in 0..4_u8 {
            for column in 0..4_u8 {
                sources.push(source_for_tileset(
                    "players_house",
                    0x1b,
                    column,
                    row,
                    drawing[usize::from(row)][usize::from(column)],
                ));
            }
        }
        let frame = frame(4, 4, sources);
        assert_eq!(
            audit_cell_coverage_on_map("CopycatsHouse1F", &frame.tiles, 4, 4)
                .expect("complete paired player-house bookcases should audit"),
            vec![CellCoverageKind::Cutout; 16]
        );
    }

    #[test]
    fn coverage_auditor_reports_player_bedroom_fixture_bank_as_cutout_geometry() {
        let drawing = [[0x05, 0x06], [0x15, 0x16], [0x25, 0x26], [0x35, 0x36]];
        let mut sources = Vec::new();
        for row in 0..4_u8 {
            for column in 0..2_u8 {
                sources.push(source_for_tileset(
                    "players_room",
                    0x03,
                    column + 2,
                    row,
                    drawing[usize::from(row)][usize::from(column)],
                ));
            }
        }
        let frame = frame(2, 4, sources);
        assert_eq!(
            audit_cell_coverage_on_map("PlayersHouse2F", &frame.tiles, 2, 4)
                .expect("complete player-room fixture bank should audit"),
            vec![CellCoverageKind::Cutout; 8]
        );
    }

    #[test]
    fn coverage_auditor_reports_complete_player_house_wall_course_as_facade() {
        let blocks = [0x07, 0x0f, 0x11, 0x05, 0x0a];
        let mut sources = Vec::new();
        for row in 0..4_u8 {
            for block in blocks {
                for column in 0..4_u8 {
                    let tile = if block == 0x0f && column < 2 && (1..3).contains(&row) {
                        [[0x0a, 0x0b], [0x1a, 0x1b]][usize::from(row - 1)][usize::from(column)]
                    } else {
                        0x11
                    };
                    sources.push(source_for_tileset(
                        "players_house",
                        block,
                        column,
                        row,
                        tile,
                    ));
                }
            }
        }
        let frame = frame(20, 4, sources);
        let coverage = audit_cell_coverage_on_map("PlayersHouse1F", &frame.tiles, 20, 4)
            .expect("complete player-house wall course should audit");
        for row in 0..4 {
            for column in 0..20 {
                let expected = if (4..6).contains(&column) && (1..3).contains(&row) {
                    CellCoverageKind::Ramp
                } else {
                    CellCoverageKind::Facade
                };
                assert_eq!(coverage[row * 20 + column], expected);
            }
        }
    }

    #[test]
    fn coverage_auditor_reports_wise_trio_divider_as_one_cutout_group() {
        let mut sources = Vec::new();
        for row in 0..2_u8 {
            for column in 0..4_u8 {
                sources.push(source_for_tileset(
                    "traditional_house",
                    0x28,
                    column,
                    row,
                    if row == 0 { 0x40 } else { 0x41 },
                ));
            }
        }
        let frame = frame(4, 2, sources);
        assert_eq!(
            audit_cell_coverage_on_map("WiseTriosRoom", &frame.tiles, 4, 2)
                .expect("complete Wise Trio divider should audit"),
            vec![CellCoverageKind::Cutout; 8]
        );
    }

    #[test]
    fn coverage_auditor_reports_mr_pokemon_work_counter_as_one_raised_surface() {
        let drawing = [[0x02, 0x03, 0x04, 0x05], [0x12, 0x13, 0x14, 0x15]];
        let mut sources = Vec::new();
        for row in 0..2_u8 {
            for column in 0..4_u8 {
                sources.push(source_for_tileset(
                    "facility",
                    0x28,
                    column,
                    row,
                    drawing[usize::from(row)][usize::from(column)],
                ));
            }
        }
        let frame = frame(4, 2, sources);
        assert_eq!(
            audit_cell_coverage_on_map("MrPokemonsHouse", &frame.tiles, 4, 2)
                .expect("complete work counter should audit"),
            vec![CellCoverageKind::Raised; 8]
        );
    }

    #[test]
    fn trainer_house_basement_stair_uses_eight_treads_over_faithful_floor() {
        let sources = vec![
            source_for_tileset("facility", 0x04, 2, 0, 0x10),
            source_for_tileset("facility", 0x04, 3, 0, 0x11),
            source_for_tileset("facility", 0x04, 2, 1, 0x20),
            source_for_tileset("facility", 0x04, 3, 1, 0x21),
            source_for_tileset("facility", 0x00, 0, 0, 0x26),
            source_for_tileset("facility", 0x00, 1, 0, 0x26),
        ];
        let frame = frame(2, 3, sources);
        let cells = frame.tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 2,
            height: 3,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let mut mesh = TerrainMeshData::default();
        let mut claimed = vec![false; 6];
        append_house_stairs(
            &mut mesh,
            "TrainerHouseB1F",
            &cells,
            &geometry,
            &mut claimed,
        );
        assert_eq!(&claimed[..4], &[true; 4]);
        assert_eq!(
            mesh.textured.quad_count(),
            20,
            "four faithful floor cells plus eight two-row stair treads"
        );
    }

    #[test]
    fn player_bed_is_one_level_mattress_not_a_sloped_card() {
        let drawing = [[0x03, 0x04], [0x13, 0x14], [0x23, 0x24], [0x33, 0x34]];
        let mut sources = Vec::new();
        for row in 0..4_u8 {
            for column in 0..2_u8 {
                sources.push(source_for_tileset(
                    "players_room",
                    0x1b,
                    column,
                    row,
                    drawing[usize::from(row)][usize::from(column)],
                ));
            }
            sources.push(source_for_tileset("players_room", 0x01, 3, row, 0x01));
        }
        let frame = frame(3, 4, sources);
        let cells = frame.tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 3,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let placement = player_bed_placements(&cells, &geometry)
            .into_iter()
            .next()
            .expect("complete bedroom bed placement");
        let mut mesh = TerrainMeshData::default();
        let mut claimed = vec![false; cells.len()];
        append_player_bed_card(
            &mut mesh,
            &cells,
            &vec![CellShape::Flat; cells.len()],
            &geometry,
            placement,
            &mut claimed,
        )
        .expect("bed should mesh");
        let raised_heights = mesh
            .textured
            .positions
            .iter()
            .map(|position| position[1])
            .filter(|height| *height > 0.0)
            .collect::<Vec<_>>();
        assert!(!raised_heights.is_empty());
        assert!(raised_heights.iter().all(|height| *height == 7.0));
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(mesh.footing_heights[row * 3 + column], 7.0);
            }
        }
        assert_eq!(mesh.textured.quad_count(), 18);
        assert_eq!(
            mesh.solid.quad_count(),
            3,
            "only the hidden head and narrow flanks use neutral structure; the visible foot is source art"
        );
        assert_eq!(claimed.iter().filter(|claimed| **claimed).count(), 8);
    }

    #[test]
    fn player_family_table_exports_its_six_pixel_visual_support() {
        let drawing = [
            [0x23, 0x22, 0x22, 0x24],
            [0x25, 0x15, 0x15, 0x35],
            [0x25, 0x15, 0x15, 0x35],
            [0x33, 0x32, 0x32, 0x34],
        ];
        let mut sources = Vec::new();
        for row in 0..4_u8 {
            for column in 0..4_u8 {
                sources.push(source_for_tileset(
                    "players_house",
                    0x25,
                    column,
                    row,
                    drawing[usize::from(row)][usize::from(column)],
                ));
            }
            sources.push(source_for_tileset("players_house", 0x03, 0, row, 0x01));
        }
        let frame = frame(5, 4, sources);
        let cells = frame.tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 5,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let placement = house_table_placements(&cells, &geometry)
            .into_iter()
            .next()
            .expect("complete player-family table placement");
        let mut mesh = TerrainMeshData::default();
        let mut claimed = vec![false; cells.len()];
        append_house_table(
            &mut mesh,
            &cells,
            &vec![CellShape::Flat; cells.len()],
            &geometry,
            placement,
            &mut claimed,
        )
        .expect("table should mesh");
        for row in 0..4 {
            for column in 0..4 {
                assert_eq!(mesh.footing_heights[row * 5 + column], 6.0);
            }
        }
        assert_eq!(mesh.footing_heights[4], 0.0);
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
    fn coverage_auditor_reports_ice_path_edge_rocks_as_complete_props() {
        let sources = vec![
            source_for_tileset("ice_path", 0x14, 0, 0, 0xc4),
            source_for_tileset("ice_path", 0x14, 1, 0, 0xc5),
            source_for_tileset("ice_path", 0x14, 0, 1, 0xd4),
            source_for_tileset("ice_path", 0x14, 1, 1, 0xd5),
        ];
        let frame = frame(2, 2, sources);
        assert_eq!(
            audit_cell_coverage_on_map("IcePathB2FBlackthornSide", &frame.tiles, 2, 2)
                .expect("complete Ice Path edge rock should audit"),
            vec![CellCoverageKind::Cutout; 4]
        );
    }

    #[test]
    fn coverage_auditor_reports_complete_ice_mass_as_one_raised_platform() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                sources.push(source_for_tileset(
                    "ice_path",
                    0x19,
                    column,
                    row,
                    0x84 + u16::from(row) * 0x10 + u16::from(column),
                ));
            }
        }
        let frame = frame(4, 4, sources);
        assert_eq!(
            audit_cell_coverage_on_map("IcePath1F", &frame.tiles, 4, 4)
                .expect("complete Ice Path mass should audit"),
            vec![CellCoverageKind::Raised; 16]
        );
    }

    #[test]
    fn coverage_auditor_reports_each_complete_cave_rock_as_a_prop() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                let drawing = [[0x0c, 0x0d], [0x1c, 0x1d]];
                sources.push(source_for_tileset(
                    "dark_cave",
                    0x1d,
                    column,
                    row,
                    drawing[usize::from(row % 2)][usize::from(column % 2)],
                ));
            }
        }
        let frame = frame(4, 4, sources);
        assert_eq!(
            audit_cell_coverage_on_map("DarkCaveBlackthornEntrance", &frame.tiles, 4, 4)
                .expect("four complete cave rocks should audit"),
            vec![CellCoverageKind::Cutout; 16]
        );
    }

    #[test]
    fn complete_cave_rock_is_a_flat_card_on_the_shelf_datum() {
        let complete = frame(
            3,
            2,
            vec![
                source_for_tileset("cave", 0x1d, 0, 0, 0x0c),
                source_for_tileset("cave", 0x1d, 1, 0, 0x0d),
                source_for_tileset("cave", 0x01, 0, 0, 0x01),
                source_for_tileset("cave", 0x1d, 0, 1, 0x1c),
                source_for_tileset("cave", 0x1d, 1, 1, 0x1d),
                source_for_tileset("cave", 0x01, 1, 0, 0x01),
            ],
        );
        let cells: Vec<_> = complete.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 3,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let placement = cave_small_rock_placements(&cells, &geometry)[0];
        assert_eq!(placement.width, 2);
        assert_eq!(placement.height, 2);
        assert_eq!(placement.base_height, crate::cave::CAVE_SHELF_HEIGHT);
        assert!(!placement.rounded, "cave rocks must not grow a voxel hull");
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
    fn waterfall_uses_each_source_row_once_on_one_bounded_cave_slope() {
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

        assert_eq!(mesh.textured.quad_count(), 11);
        let waterfall_vertices = &mesh.textured.positions[7 * 4..11 * 4];
        let min_y = waterfall_vertices
            .iter()
            .map(|position| position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = waterfall_vertices
            .iter()
            .map(|position| position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_z = waterfall_vertices
            .iter()
            .map(|position| position[2])
            .fold(f32::INFINITY, f32::min);
        let max_z = waterfall_vertices
            .iter()
            .map(|position| position[2])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_y - min_y - crate::cave::CAVE_ROCK_HEIGHT).abs() < f32::EPSILON);
        assert!((max_z - min_z - 16.0).abs() < f32::EPSILON);
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
    fn authored_rock_platform_cave_and_kanto_mound_families_taper() {
        assert_eq!(
            bank_taper(
                &source_with_tile(0x0a, 0, 0, 0),
                CellShape::RaisedTop {
                    height: crate::profile::MOUNTAIN_LEDGE_HEIGHT,
                    solid: SolidKind::Bank,
                },
            ),
            Some([2.0, 0.0])
        );
        let mut mound = source_with_tile(0x3e, 0, 0, 0);
        mound.tileset_id = Arc::from("kanto");
        assert_eq!(
            bank_taper(
                &mound,
                CellShape::RaisedTop {
                    height: crate::profile::MOUNTAIN_CLIFF_HEIGHT,
                    solid: SolidKind::Bank,
                },
            ),
            Some([2.0, 0.0])
        );
        mound.metatile_id = 0x01;
        assert_eq!(bank_taper(&mound, CellShape::Flat), None);
        let mut ice_mass = source_with_tile(0x19, 0, 0, 0x84);
        ice_mass.tileset_id = Arc::from("ice_path");
        assert_eq!(
            bank_taper(
                &ice_mass,
                CellShape::RaisedTop {
                    height: crate::profile::MOUNTAIN_LEDGE_HEIGHT,
                    solid: SolidKind::Bank,
                },
            ),
            Some([2.0, 0.0])
        );
        ice_mass.metatile_id = 0x18;
        assert_eq!(bank_taper(&ice_mass, CellShape::Flat), None);
        let mut cave_rock = source_with_tile(0x04, 0, 0, 0x01);
        cave_rock.tileset_id = Arc::from("cave");
        assert_eq!(
            bank_taper(
                &cave_rock,
                CellShape::RaisedTop {
                    height: crate::cave::CAVE_ROCK_HEIGHT,
                    solid: SolidKind::Bank,
                },
            ),
            Some([2.0, 0.0])
        );
        cave_rock.tileset_id = Arc::from("dark_cave");
        assert_eq!(
            bank_taper(
                &cave_rock,
                CellShape::RaisedTop {
                    height: crate::cave::CAVE_ROCK_HEIGHT,
                    solid: SolidKind::Bank,
                },
            ),
            Some([2.0, 0.0])
        );
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
            .filter(|((_, normal), _)| normal[0][2] > 0.5)
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
            .filter(|normal| normal[0][2] > 0.5)
            .count();
        let east_faces = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|normal| normal[0][0] > 0.5)
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
                normals[0][0] > 0.5 && positions.iter().all(|position| position[0] > 8.0)
            })
            .count();
        let solid_east_faces = mesh
            .solid
            .positions
            .chunks_exact(4)
            .zip(mesh.solid.normals.chunks_exact(4))
            .filter(|(positions, normals)| {
                normals[0][0] > 0.5 && positions.iter().all(|position| position[0] > 8.0)
            })
            .count();
        assert_eq!(textured_east_courses, 2);
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
            .filter(|((_, normals), _)| normals[0][0] > 0.5)
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
            tapered_faces, 36,
            "four meeting sides and their chamfers use native tapered courses"
        );
        let chamfered_faces = mesh
            .textured
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
            chamfered_faces, 4,
            "visible textured chamfers close the widened exposed corners"
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
            2.0,
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
    fn jump_ledge_does_not_invent_a_plateau_in_neighboring_flat_ground() {
        let mut sources = vec![flat_source(); 5];
        for row in 0..3 {
            sources[row + 1] = source_with_tile(0x57, 1, row as u8, 0x05);
        }
        sources[4] = source_with_tile(0x57, 1, 3, 0x4c);

        let mesh = build_terrain_mesh(&frame(1, 5, sources)).expect("exact jump ledge meshes");
        assert_eq!(
            mesh.footing_heights[0], 0.0,
            "ordinary ground north of the exact ledge drawing must remain flat"
        );
        assert_eq!(
            mesh.footing_heights[1],
            crate::profile::JUMP_LEDGE_HEIGHT,
            "the authored ledge cap retains its single level"
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
                roof_rows: 4,
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
                roof_rows: 2,
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
    fn blackthorn_closed_mound_is_one_object_not_six_cliff_blocks() {
        let metatiles = [[0x6a, 0x70, 0x6b], [0x6c, 0x72, 0x6d]];
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
            origin_x: 0.0,
            origin_z: 0.0,
        };

        assert_eq!(johto_closed_mound_origins(&cells, 12, 8), vec![(0, 0)]);
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 12,
                height: 8,
                roof_rows: 6,
                ground_tile_index: 0x01,
            }]
        );
    }

    #[test]
    fn ice_path_closed_plateau_reuses_the_complete_mound_object() {
        let metatiles = [[0x04, 0x05, 0x06], [0x0c, 0x0d, 0x0e]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..12 {
                let mut tile = source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                );
                tile.tileset_id = Arc::from("ice_path");
                tile.tile_index = 0x9a;
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

        assert_eq!(ice_path_plateau_origins(&cells, 12, 8), vec![(0, 0, 12)]);
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 12,
                height: 8,
                roof_rows: 6,
                ground_tile_index: 0x9a,
            }]
        );

        let mut wrong_sources = frame.tiles.clone();
        for row in 4..8 {
            for column in 8..12 {
                wrong_sources[row * 12 + column].source.metatile_id = 0x1f;
            }
        }
        let wrong_cells: Vec<_> = wrong_sources.iter().collect();
        assert!(ice_path_plateau_origins(&wrong_cells, 12, 8).is_empty());
    }

    #[test]
    fn ice_path_two_block_island_is_one_trapezoid() {
        let metatiles = [[0x04, 0x06], [0x10, 0x3a]];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..8 {
                let mut tile = source(
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                );
                tile.tileset_id = Arc::from("ice_path");
                tile.tile_index = 0x9a;
                sources.push(tile);
            }
        }
        let frame = frame(8, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 8,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };

        assert_eq!(ice_path_plateau_origins(&cells, 8, 8), vec![(0, 0, 8)]);
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 8,
                height: 8,
                roof_rows: 6,
                ground_tile_index: 0x9a,
            }]
        );
    }

    #[test]
    fn ice_path_long_plateau_keeps_transition_blocks_inside_one_object() {
        let north = [0x04, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x06];
        let south = [0x09, 0x10, 0x0d, 0x3e, 0x0d, 0x3e, 0x0d, 0x12, 0x0e];
        let mut sources = Vec::new();
        for row in 0..8 {
            for column in 0..36 {
                let metatile = if row < 4 {
                    north[column / 4]
                } else {
                    south[column / 4]
                };
                let mut tile = source(metatile, (column % 4) as u8, (row % 4) as u8);
                tile.tileset_id = Arc::from("ice_path");
                tile.tile_index = 0x9a;
                sources.push(tile);
            }
        }
        let frame = frame(36, 8, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 36,
            height: 8,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };

        assert_eq!(ice_path_plateau_origins(&cells, 36, 8), vec![(0, 0, 36)]);
        assert_eq!(
            outdoor_building_placements(&cells, &geometry),
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 36,
                height: 8,
                roof_rows: 6,
                ground_tile_index: 0x9a,
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
    fn celadon_department_store_claims_its_roof_and_every_window_course() {
        let metatiles = [
            [0x20, 0x54, 0x21],
            [0x68, 0x7f, 0x69],
            [0x68, 0x7f, 0x69],
            [0x37, 0x3a, 0x73],
        ];
        let mut sources = Vec::new();
        for row in 0..16 {
            for column in 0..12 {
                sources.push(source_for_tileset(
                    "kanto",
                    metatiles[row / 4][column / 4],
                    (column % 4) as u8,
                    (row % 4) as u8,
                    0x30,
                ));
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
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
            }]
        );
    }

    #[test]
    fn saffron_silph_claims_its_cap_before_all_four_window_courses() {
        let metatiles = [
            [0x20, 0x54, 0x54, 0x21],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x68, 0x7f, 0x7f, 0x69],
            [0x37, 0x3a, 0x7d, 0x7e],
        ];
        let mut sources = Vec::new();
        for row in 0..24 {
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
        let frame = frame(16, 24, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 16,
            height: 24,
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
                height: 24,
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
    fn house_bookcase_recess_starts_below_the_lid_and_keeps_frame_proud() {
        let width = 16;
        let height = 32;
        let mut inside = vec![true; width * height];
        let mut luminance = vec![300_u16; width * height];
        // A dark rectangular frame around one shelf opening. The opening is
        // deliberately below source row 9, where the authored lid folds.
        for y in 11..18 {
            for x in 3..13 {
                if x == 3 || x == 12 || y == 11 || y == 17 {
                    luminance[y * width + x] = 0;
                }
            }
        }
        // Transparent pixels are outside the drawing and cannot be panels.
        inside[31 * width] = false;

        let recessed = facade_recess_mask(&inside, &luminance, width, height, 9, 0);

        assert!(
            recessed[14 * width + 8],
            "the enclosed shelf field recesses"
        );
        assert!(
            !recessed[11 * width + 8],
            "the dark shelf frame stays proud"
        );
        assert!(
            recessed[..9 * width].iter().all(|pixel| !pixel),
            "the cabinet lid is outside the facade relief pass"
        );
        assert!(
            !recessed[31 * width],
            "transparent background never recesses"
        );
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
    fn house_appliance_cards_gain_shallow_casings_but_tree_cards_stay_flat() {
        let drawing = [
            [0x11, 0x11, 0x11, 0x11],
            [0x06, 0x07, 0x11, 0x11],
            [0x16, 0x17, 0x0e, 0x0f],
            [0x08, 0x09, 0x3a, 0x3b],
        ];
        let house_tiles = (0..4)
            .flat_map(|row| {
                (0..4).map(move |column| {
                    source_for_tileset(
                        "players_house",
                        0x11,
                        column as u8,
                        row as u8,
                        drawing[row][column],
                    )
                })
            })
            .collect::<Vec<_>>();
        let frame = frame(4, 4, house_tiles);
        let cells = frame.tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 4,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let fixtures = players_house_upright_fixture_placements(&cells, &geometry);
        assert_eq!(fixtures.len(), 2);
        assert!(fixtures.iter().all(|fixture| fixture.card_thickness == 1.0));

        let generic = grouped_flat_card_placements(
            &cells,
            &geometry,
            0x11,
            false,
            crate::players_house::upright_fixture_local,
        );
        assert!(generic.iter().all(|fixture| fixture.card_thickness == 0.0));
    }

    #[test]
    fn trainer_house_open_book_keeps_the_complete_page_drawing() {
        let book = [
            source_for_tileset("house", 0x14, 2, 3, 0x46),
            source_for_tileset("house", 0x14, 3, 3, 0x47),
            source_for_tileset("house", 0x18, 2, 0, 0x56),
            source_for_tileset("house", 0x18, 3, 0, 0x57),
        ];
        let frame = frame(2, 2, book.into());
        let cells = frame.tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 2,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };

        let placements = house_open_book_placements(&cells, &geometry);
        assert_eq!(placements.len(), 1);
        assert_eq!((placements[0].column, placements[0].row), (0, 0));
        assert_eq!((placements[0].width, placements[0].height), (2, 2));
        assert!(
            !placements[0].outline_mask,
            "the light pages touch the drawing boundary, so a dark-outline mask drops them"
        );
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
                    ground_metatile_id: None,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                    remove_all_ground: false,
                    card_thickness: 0.0,
                },
                TreePlacement {
                    column: 2,
                    row: 0,
                    width: 2,
                    height: 4,
                    ground_tile_index: 0x05,
                    ground_metatile_id: None,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                    remove_all_ground: false,
                    card_thickness: 0.0,
                },
            ]
        );

        let mut samples = TerrainImageSamples::default();
        for tile in &frame.tiles {
            let rgba = if tile.source.metatile_id == 0x05 {
                // Model the authored drawing: boundary-connected light
                // ground surrounds a darker canopy that continues through
                // all four source rows. Painting the corners canopy-colored
                // would correctly classify the entire synthetic image as
                // background and would not represent a real tree sprite.
                let mut rgba = [255, 255, 255, 255].repeat(64);
                for y in 0..SOURCE_TILE_PIXELS {
                    for x in 1..SOURCE_TILE_PIXELS - 1 {
                        let offset = (y * SOURCE_TILE_PIXELS + x) * 4;
                        rgba[offset..offset + 4].copy_from_slice(&[0, 80, 0, 255]);
                    }
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
        let is_tree_card_normal = |normal: [f32; 3]| {
            normal[0].abs() < 0.000_001
                && normal[1] > 0.0
                && normal[2] > 0.0
                && (normal[1] - normal[2]).abs() < 0.000_001
        };
        let (min_y, max_y) = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normals)| is_tree_card_normal(normals[0]))
            .flat_map(|(positions, _)| positions)
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min, max), position| (min.min(position[1]), max.max(position[1])),
            );
        let expected_world_y_span = 32.0 * std::f32::consts::FRAC_1_SQRT_2;
        let normal_samples: Vec<_> = mesh
            .textured
            .normals
            .chunks_exact(4)
            .map(|quad| quad[0])
            .collect();
        assert!(
            (max_y - min_y - expected_world_y_span).abs() < 0.001,
            "headbutt trees are two map tiles tall: measured {}..{} (span {}), expected {}; normals {:?}",
            min_y,
            max_y,
            max_y - min_y,
            expected_world_y_span,
            normal_samples,
        );
        assert!(!normal_samples.is_empty());
        let (min_z, max_z) = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normals)| is_tree_card_normal(normals[0]))
            .flat_map(|(positions, _)| positions)
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min, max), position| (min.min(position[2]), max.max(position[2])),
            );
        assert!(max_z > min_z, "isolated canopies must have real depth");
    }

    #[test]
    fn headbutt_tree_block_becomes_four_independent_two_by_two_cards() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..5 {
                sources.push(if column < 4 {
                    source_for_tileset(
                        "johto",
                        0x61,
                        column,
                        row,
                        if row % 2 == 0 {
                            if column % 2 == 0 { 0x1e } else { 0x1f }
                        } else if column % 2 == 0 {
                            0x3e
                        } else {
                            0x3f
                        },
                    )
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
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let placements = complete_tree_placements(&cells, &geometry);
        assert_eq!(placements.len(), 4);
        assert_eq!(
            placements
                .iter()
                .map(|placement| (
                    placement.column,
                    placement.row,
                    placement.width,
                    placement.height
                ))
                .collect::<Vec<_>>(),
            vec![(0, 0, 2, 2), (2, 0, 2, 2), (0, 2, 2, 2), (2, 2, 2, 2)]
        );
    }

    #[test]
    fn mixed_headbutt_block_claims_two_trees_but_not_its_ground_rows() {
        let mut sources = Vec::new();
        for row in 0..2 {
            for column in 0..5 {
                sources.push(if column < 4 {
                    source_for_tileset(
                        "johto",
                        0x5d,
                        column,
                        row,
                        if row == 0 {
                            if column % 2 == 0 { 0x1e } else { 0x1f }
                        } else if column % 2 == 0 {
                            0x3e
                        } else {
                            0x3f
                        },
                    )
                } else {
                    source_with_tile(0x01, 0, 0, 0x05)
                });
            }
        }
        let frame = frame(5, 2, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 5,
            height: 2,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        assert_eq!(
            complete_tree_placements(&cells, &geometry)
                .iter()
                .map(|placement| (
                    placement.column,
                    placement.row,
                    placement.width,
                    placement.height
                ))
                .collect::<Vec<_>>(),
            vec![(0, 0, 2, 2), (2, 0, 2, 2)]
        );
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
                    ground_metatile_id: None,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                    remove_all_ground: false,
                    card_thickness: 0.0,
                },
                TreePlacement {
                    column: 2,
                    row: 0,
                    width: 2,
                    height: 4,
                    ground_tile_index: 0x06,
                    ground_metatile_id: None,
                    base_height: 0.0,
                    rounded: false,
                    outline_mask: false,
                    remove_all_ground: false,
                    card_thickness: 0.0,
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
                ground_metatile_id: None,
                base_height: 0.0,
                rounded: false,
                outline_mask: false,
                remove_all_ground: false,
                card_thickness: 0.0,
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
                ground_metatile_id: None,
                base_height: 0.0,
                rounded: false,
                outline_mask: false,
                remove_all_ground: false,
                card_thickness: 0.0,
            }]
        );
    }

    #[test]
    fn saffron_gym_planter_stays_a_flat_masked_card() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..2 {
                let mut source = source_with_tile(0x36, column, row, 0x20);
                source.tileset_id = Arc::from("underground");
                sources.push(source);
            }
        }
        let frame = frame(2, 4, sources);
        let cells: Vec<_> = frame.tiles.iter().collect();
        let geometry = GridGeometry {
            width: 2,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -8.0,
            origin_z: -16.0,
        };

        let placements = saffron_gym_planter_placements(&cells, &geometry);
        assert_eq!(placements.len(), 1);
        assert_eq!((placements[0].width, placements[0].height), (2, 4));
        assert!(!placements[0].rounded);
        assert!(placements[0].outline_mask);
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
