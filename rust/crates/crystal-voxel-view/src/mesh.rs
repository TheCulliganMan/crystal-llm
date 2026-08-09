//! Pure authored-profile mesh construction for the optional voxel renderer.

use std::collections::{HashMap, VecDeque};

use bevy::{
    asset::AssetId,
    prelude::{Assets, Image, Mesh},
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use crystal_render_api::{VisualTile, VisualWorldFrame};

use crate::profile::{
    CellShape, KANTO_GROUND_TILE_INDEX, LedgeFace, SOURCE_TILE_HEIGHT, SolidKind, shape_for_source,
};

const TEXTURED_SHADE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SOURCE_TILE_PIXELS: usize = SOURCE_TILE_HEIGHT as usize;

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
        .map(|tile| shape_for_source(&tile.source))
        .collect();
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
    for shape in &mut shapes {
        if let CellShape::FacadeBand {
            ground_tile_index, ..
        } = *shape
            && !available_flat_tiles.contains(&ground_tile_index)
        {
            *shape = CellShape::Flat;
        }
    }

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
    let mut claimed_by_building = vec![false; cell_count];
    let mut claimed_by_tree = vec![false; cell_count];
    if let Some(images) = images {
        let placements = outdoor_building_placements(&cells, &geometry);
        for placement in &placements {
            if let Err(error) = append_pixel_building(
                &mut mesh,
                images,
                &cells,
                &geometry,
                *placement,
                &mut claimed_by_building,
            ) {
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

    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
            if claimed_by_building[index] || claimed_by_tree[index] {
                continue;
            }
            append_textured_cell(
                &mut mesh, &geometry, &cells, &shapes, column, row, index, images,
            )?;
        }
    }

    for row in 0..height {
        for column in 0..width {
            if claimed_by_building[row * width + column] || claimed_by_tree[row * width + column] {
                continue;
            }
            append_exposed_sides(&mut mesh, &geometry, &cells, &shapes, column, row);
        }
    }

    Ok(mesh)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TreePlacement {
    column: usize,
    row: usize,
    width: usize,
    height: usize,
    ground_tile_index: u16,
}

fn complete_tree_placements(cells: &[&VisualTile], geometry: &GridGeometry) -> Vec<TreePlacement> {
    let mut placements = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tile in cells {
        let (local_column, local_row, width, height, ground_tile_index) =
            match (tile.source.tileset_id.as_ref(), tile.source.metatile_id) {
                ("johto" | "johto_modern", 0x05) => (
                    tile.source.subtile_column,
                    tile.source.subtile_row,
                    4,
                    4,
                    if tile.source.tileset_id.as_ref() == "johto_modern" {
                        0x06
                    } else {
                        0x05
                    },
                ),
                ("johto", 0x62) if tile.source.subtile_column >= 2 => (
                    tile.source.subtile_column - 2,
                    tile.source.subtile_row % 2,
                    2,
                    2,
                    0x05,
                ),
                ("johto", 0x65) if tile.source.subtile_row >= 2 => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row - 2,
                    2,
                    2,
                    0x05,
                ),
                ("kanto", _) if shape_for_source(&tile.source).solid_kind() == SolidKind::Tree => (
                    tile.source.subtile_column % 2,
                    tile.source.subtile_row % 2,
                    2,
                    2,
                    KANTO_GROUND_TILE_INDEX,
                ),
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
                    && shape_for_source(&cell.source).solid_kind() == SolidKind::Tree
            })
        });
        if complete {
            placements.push(TreePlacement {
                column: origin_column as usize,
                row: origin_row as usize,
                width,
                height,
                ground_tile_index,
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
            "johto" | "johto_modern" | "kanto"
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

    const TEMPLATES: &[(&str, &[&[u16]], usize, u16)] = &[
        (
            "johto",
            &[&[0x18, 0x1f, 0x19], &[0x1c, 0x77, 0x1e]],
            4,
            0x06,
        ),
        ("johto", &[&[0x18, 0x19], &[0x16, 0x1e]], 4, 0x06),
        ("johto", &[&[0x14, 0x15]], 2, 0x06),
        (
            "johto_modern",
            &[&[0x18, 0x1f, 0x19], &[0x1c, 0x1d, 0x1e]],
            4,
            0x06,
        ),
        ("johto_modern", &[&[0x18, 0x19], &[0x16, 0x1e]], 4, 0x06),
        ("johto_modern", &[&[0x14, 0x15]], 2, 0x06),
        ("johto_modern", &[&[0x12, 0x13]], 2, 0x06),
        ("johto_modern", &[&[0x10, 0x17, 0x11]], 2, 0x06),
    ];
    let mut placements = Vec::new();
    for (&(origin_x, origin_y), &(tileset_id, metatile_id)) in &metatiles {
        for &(template_tileset, rows, roof_rows, ground_tile_index) in TEMPLATES {
            if template_tileset != tileset_id || rows[0][0] != metatile_id {
                continue;
            }
            let matches = rows.iter().enumerate().all(|(template_y, row)| {
                row.iter().enumerate().all(|(template_x, expected)| {
                    metatiles
                        .get(&(
                            origin_x + (template_x * 4) as isize,
                            origin_y + (template_y * 4) as isize,
                        ))
                        .is_some_and(|(candidate_tileset, candidate)| {
                            *candidate_tileset == template_tileset && candidate == expected
                        })
                })
            });
            let width = rows[0].len() * 4;
            let height = rows.len() * 4;
            if matches
                && origin_x >= 0
                && origin_y >= 0
                && origin_x as usize + width <= geometry.width
                && origin_y as usize + height <= geometry.height
            {
                placements.push(BuildingPlacement {
                    column: origin_x as usize,
                    row: origin_y as usize,
                    width,
                    height,
                    roof_rows,
                    ground_tile_index,
                });
            }
        }
    }

    append_kanto_building_placements(&metatiles, geometry, &mut placements);
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

        // Compact Kanto houses are complete one-metatile-high drawings,
        // but are authored as left/right blocks. Their upper two tile rows
        // are roof art and their lower two rows are the facade.
        if matches!(first, 0x02 | 0x30) && at(x + 4, y) == Some(0x03) {
            add(x, y, 2, 1, 2);
        }
        if first == 0x68 && at(x + 4, y) == Some(0x69) {
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
    let ground_index = authored_ground_cell(cells, shapes, placement.ground_tile_index).ok_or(
        TerrainMeshError::MissingGroundSample {
            column: placement.column as u32,
            row: placement.row as u32,
            tile_index: placement.ground_tile_index,
        },
    )?;
    let pixel_width = placement.width * SOURCE_TILE_PIXELS;
    let pixel_height = placement.height * SOURCE_TILE_PIXELS;
    let mut solid_pixels = vec![false; pixel_width * pixel_height];

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
                0.0,
                geometry.uv(ground_index % geometry.width, ground_index / geometry.width),
            );
            let removable = grouped_boundary_ground_mask(
                images,
                cells,
                shapes,
                geometry,
                column,
                row,
                index,
                cells[ground_index],
            )?;
            for pixel_y in 0..SOURCE_TILE_PIXELS {
                for pixel_x in 0..SOURCE_TILE_PIXELS {
                    solid_pixels[(local_row * SOURCE_TILE_PIXELS + pixel_y) * pixel_width
                        + local_column * SOURCE_TILE_PIXELS
                        + pixel_x] = !removable[pixel_y * SOURCE_TILE_PIXELS + pixel_x];
                }
            }
        }
    }

    let x0 = geometry.origin_x + placement.column as f32 * geometry.tile_width;
    let x1 = x0 + placement.width as f32 * geometry.tile_width;
    let plane_z =
        geometry.origin_z + (placement.row + placement.height) as f32 * geometry.tile_height;
    let crown_height = placement.height as f32 * geometry.tile_height;
    let max_depth = placement.width as f32 * geometry.tile_width;
    let center_z = plane_z - max_depth * 0.5;
    let mut chords = vec![None; pixel_width * pixel_height];
    for pixel_y in 0..pixel_height {
        let Some(left) = (0..pixel_width).find(|&x| solid_pixels[pixel_y * pixel_width + x]) else {
            continue;
        };
        let right = (left..pixel_width)
            .rev()
            .find(|&x| solid_pixels[pixel_y * pixel_width + x])
            .expect("left solid canopy pixel implies a right pixel");
        let center_x = (left + right + 1) as f32 * 0.5;
        let radius = (right - left + 1) as f32 * 0.5;
        for pixel_x in left..=right {
            if !solid_pixels[pixel_y * pixel_width + pixel_x] {
                continue;
            }
            let dx = pixel_x as f32 + 0.5 - center_x;
            let depth = max_depth * (1.0 - (dx / radius).powi(2)).max(0.0).sqrt();
            chords[pixel_y * pixel_width + pixel_x] =
                Some((center_z - depth * 0.5, center_z + depth * 0.5));
        }
    }
    let chord_at = |x: isize, y: isize| {
        (x >= 0 && y >= 0 && x < pixel_width as isize && y < pixel_height as isize)
            .then(|| chords[y as usize * pixel_width + x as usize])
            .flatten()
    };
    for pixel_y in 0..pixel_height {
        for pixel_x in 0..pixel_width {
            let Some((back_z, front_z)) = chord_at(pixel_x as isize, pixel_y as isize) else {
                continue;
            };
            let world_x0 = x0 + pixel_x as f32 * (x1 - x0) / pixel_width as f32;
            let world_x1 = x0 + (pixel_x + 1) as f32 * (x1 - x0) / pixel_width as f32;
            let world_y1 = crown_height - pixel_y as f32 * crown_height / pixel_height as f32;
            let world_y0 = crown_height - (pixel_y + 1) as f32 * crown_height / pixel_height as f32;
            let cell_column = placement.column + pixel_x / SOURCE_TILE_PIXELS;
            let cell_row = placement.row + pixel_y / SOURCE_TILE_PIXELS;
            let (u0, u1, v0, v1) = geometry.uv(cell_column, cell_row);
            let local_x = pixel_x % SOURCE_TILE_PIXELS;
            let local_y = pixel_y % SOURCE_TILE_PIXELS;
            let pu0 = lerp_pixel(u0, u1, local_x);
            let pu1 = lerp_pixel(u0, u1, local_x + 1);
            let pv0 = lerp_pixel(v0, v1, local_y);
            let pv1 = lerp_pixel(v0, v1, local_y + 1);
            for (z, normal, shade, reverse) in [
                (front_z, [0.0, 0.0, 1.0], TEXTURED_SHADE, false),
                (back_z, [0.0, 0.0, -1.0], [0.68, 0.68, 0.68, 1.0], true),
            ] {
                let (xa, xb, ua, ub) = if reverse {
                    (world_x0, world_x1, pu0, pu1)
                } else {
                    (world_x1, world_x0, pu1, pu0)
                };
                append_quad(
                    &mut mesh.textured,
                    [
                        [xa, world_y0, z],
                        [xa, world_y1, z],
                        [xb, world_y1, z],
                        [xb, world_y0, z],
                    ],
                    normal,
                    [[ua, pv1], [ua, pv0], [ub, pv0], [ub, pv1]],
                    shade,
                );
            }
            for (neighbor, x, normal) in [
                (
                    chord_at(pixel_x as isize - 1, pixel_y as isize),
                    world_x0,
                    [-1.0, 0.0, 0.0],
                ),
                (
                    chord_at(pixel_x as isize + 1, pixel_y as isize),
                    world_x1,
                    [1.0, 0.0, 0.0],
                ),
            ] {
                for (z0, z1) in exposed_chord_intervals(back_z, front_z, neighbor) {
                    append_solid_quad(
                        &mut mesh.solid,
                        [
                            [x, world_y0, z1],
                            [x, world_y1, z1],
                            [x, world_y1, z0],
                            [x, world_y0, z0],
                        ],
                        normal,
                        solid_color(SolidKind::Tree, Direction::West),
                    );
                }
            }
            for (neighbor, y, normal) in [
                (
                    chord_at(pixel_x as isize, pixel_y as isize - 1),
                    world_y1,
                    [0.0, 1.0, 0.0],
                ),
                (
                    chord_at(pixel_x as isize, pixel_y as isize + 1),
                    world_y0,
                    [0.0, -1.0, 0.0],
                ),
            ] {
                for (z0, z1) in exposed_chord_intervals(back_z, front_z, neighbor) {
                    append_solid_quad(
                        &mut mesh.solid,
                        [
                            [world_x0, y, z0],
                            [world_x0, y, z1],
                            [world_x1, y, z1],
                            [world_x1, y, z0],
                        ],
                        normal,
                        solid_color(SolidKind::Tree, Direction::South),
                    );
                }
            }
        }
    }
    Ok(())
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
    let wall_height = (pixel_height - roof_pixels) as f32 * pixel_z_size;
    let (building_x0, _, building_z0, _) = geometry.bounds(placement.column, placement.row);
    let facade_z = building_z0 + roof_pixels as f32 * pixel_z_size;

    let mut roof_top = vec![roof_pixels; pixel_width];
    for x in 0..pixel_width {
        if let Some(y) = (0..roof_pixels).find(|&y| inside[y * pixel_width + x]) {
            roof_top[x] = y;
        }
    }
    let darkest = shades[0];
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
            if !inside[y * pixel_width + x] {
                continue;
            }
            let x0 = building_x0 + x as f32 * pixel_x_size;
            let x1 = x0 + pixel_x_size;
            let top = (pixel_height - y) as f32 * pixel_z_size;
            let bottom = top - pixel_z_size;
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

    for y in 0..roof_pixels {
        for x in 0..pixel_width {
            if !inside[y * pixel_width + x] || roof_top[x] == roof_pixels {
                continue;
            }
            let x0 = building_x0 + x as f32 * pixel_x_size;
            let x1 = x0 + pixel_x_size;
            let z0 = building_z0 + y as f32 * pixel_z_size;
            let z1 = z0 + pixel_z_size;
            let south_height = gable_height(
                wall_height,
                roof_pixels as f32 * pixel_z_size,
                (y + 1) as f32 * pixel_z_size,
            );
            let north_height = gable_height(
                wall_height,
                roof_pixels as f32 * pixel_z_size,
                y as f32 * pixel_z_size,
            );
            append_quad(
                &mut mesh.textured,
                [
                    [x0, north_height, z0],
                    [x0, south_height, z1],
                    [x1, south_height, z1],
                    [x1, north_height, z0],
                ],
                roof_normal(north_height, south_height, pixel_z_size),
                source_pixel_uv(geometry, placement, x, y, false),
                TEXTURED_SHADE,
            );
        }
    }

    let building_x1 = building_x0 + pixel_width as f32 * pixel_x_size;
    for source_y in roof_pixels..pixel_height {
        let west_source_x = (0..pixel_width.min(4))
            .find(|&x| {
                inside[source_y * pixel_width + x]
                    && luminance[source_y * pixel_width + x] > darkest
            })
            .unwrap_or(0);
        let east_source_x = (pixel_width.saturating_sub(4)..pixel_width)
            .rev()
            .find(|&x| {
                inside[source_y * pixel_width + x]
                    && luminance[source_y * pixel_width + x] > darkest
            })
            .unwrap_or(pixel_width - 1);
        let y_top = (pixel_height - source_y) as f32 * pixel_z_size;
        let y_bottom = y_top - pixel_z_size;
        for depth_pixel in 0..roof_pixels {
            let z0 = building_z0 + depth_pixel as f32 * pixel_z_size;
            let z1 = z0 + pixel_z_size;
            append_quad(
                &mut mesh.textured,
                [
                    [building_x0, y_bottom, z0],
                    [building_x0, y_top, z0],
                    [building_x0, y_top, z1],
                    [building_x0, y_bottom, z1],
                ],
                [-1.0, 0.0, 0.0],
                source_pixel_uv(geometry, placement, west_source_x, source_y, true),
                [0.78, 0.78, 0.78, 1.0],
            );
            append_quad(
                &mut mesh.textured,
                [
                    [building_x1, y_bottom, z1],
                    [building_x1, y_top, z1],
                    [building_x1, y_top, z0],
                    [building_x1, y_bottom, z0],
                ],
                [1.0, 0.0, 0.0],
                source_pixel_uv(geometry, placement, east_source_x, source_y, true),
                [0.86, 0.86, 0.86, 1.0],
            );
        }
    }

    // Close the two gable ends with the roof's edge pixels. Each source
    // texel covers one source-sized quad; no roof row is stretched.
    for source_y in 0..roof_pixels {
        let z0 = building_z0 + source_y as f32 * pixel_z_size;
        let z1 = z0 + pixel_z_size;
        let north_height = gable_height(
            wall_height,
            roof_pixels as f32 * pixel_z_size,
            source_y as f32 * pixel_z_size,
        );
        let south_height = gable_height(
            wall_height,
            roof_pixels as f32 * pixel_z_size,
            (source_y + 1) as f32 * pixel_z_size,
        );
        let west_source_x = (0..pixel_width)
            .find(|&x| {
                inside[source_y * pixel_width + x]
                    && luminance[source_y * pixel_width + x] > darkest
            })
            .unwrap_or(0);
        let east_source_x = (0..pixel_width)
            .rev()
            .find(|&x| {
                inside[source_y * pixel_width + x]
                    && luminance[source_y * pixel_width + x] > darkest
            })
            .unwrap_or(pixel_width - 1);
        for (x, source_x, normal, shade) in [
            (building_x0, west_source_x, [-1.0, 0.0, 0.0], 0.78),
            (building_x1, east_source_x, [1.0, 0.0, 0.0], 0.86),
        ] {
            append_quad(
                &mut mesh.textured,
                [
                    [x, wall_height, z0],
                    [x, north_height, z0],
                    [x, south_height, z1],
                    [x, wall_height, z1],
                ],
                normal,
                source_pixel_uv(geometry, placement, source_x, source_y, true),
                [shade, shade, shade, 1.0],
            );
        }
    }
    Ok(())
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

fn gable_height(wall_height: f32, roof_depth: f32, depth_from_north: f32) -> f32 {
    if roof_depth <= f32::EPSILON {
        return wall_height;
    }
    let normalized = (depth_from_north / roof_depth).clamp(0.0, 1.0);
    let ridge = 1.0 - (normalized * 2.0 - 1.0).abs();
    wall_height + ridge * (roof_depth * 0.5).min(16.0)
}

fn roof_normal(north_height: f32, south_height: f32, depth: f32) -> [f32; 3] {
    let rise = south_height - north_height;
    let length = (depth * depth + rise * rise).sqrt().max(f32::EPSILON);
    [0.0, depth / length, -rise / length]
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
    column: usize,
    row: usize,
    index: usize,
    images: Option<&TerrainImageSamples>,
) -> Result<(), TerrainMeshError> {
    let (x0, x1, z0, z1) = geometry.bounds(column, row);
    match shapes[index] {
        CellShape::Flat | CellShape::Water | CellShape::RaisedTop { .. } => {
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                shapes[index].surface_height(geometry.tile_height),
                geometry.uv(column, row),
            );
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
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                0.0,
                geometry.uv(replacement % geometry.width, replacement / geometry.width),
            );

            let tile = cells[index];
            // The viewport may clip the metatile origin, so this is signed.
            // A facade plane just outside the viewport is still valid geometry.
            let metatile_origin_row = row as isize - tile.source.subtile_row as isize;
            let plane_row = metatile_origin_row + plane_subtile_row as isize;
            let plane_z = geometry.origin_z + plane_row as f32 * geometry.tile_height;
            let band_top = (band_count - band_from_top) as f32 * geometry.tile_height;
            let band_bottom = band_top - geometry.tile_height;
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            if matches!(solid, SolidKind::Tree | SolidKind::Prop)
                && let Some(images) = images
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
        CellShape::LedgeBand {
            face,
            plane_subtile,
            band_from_top,
            band_count,
            top_tile_index,
            height,
        } => {
            let top_source = cells
                .iter()
                .zip(shapes)
                .enumerate()
                .filter(|(_, (tile, shape))| {
                    tile.source.tile_index == top_tile_index
                        && shape.surface_height(geometry.tile_height)
                            == height * geometry.tile_height / SOURCE_TILE_HEIGHT
                })
                .min_by_key(|(_, (tile, _))| (tile.row, tile.column))
                .map(|(index, _)| index)
                .ok_or(TerrainMeshError::MissingGroundSample {
                    column: column as u32,
                    row: row as u32,
                    tile_index: top_tile_index,
                })?;
            let raised_height = height * geometry.tile_height / SOURCE_TILE_HEIGHT;
            append_top(
                &mut mesh.textured,
                [x0, x1, z0, z1],
                raised_height,
                geometry.uv(top_source % geometry.width, top_source / geometry.width),
            );
            let tile = cells[index];
            let band_top = raised_height - band_from_top as f32 * geometry.tile_height;
            let band_bottom = band_top - geometry.tile_height;
            let (u0, u1, v0, v1) = geometry.uv(column, row);
            let origin_column = column as isize - tile.source.subtile_column as isize;
            let origin_row = row as isize - tile.source.subtile_row as isize;
            let (positions, normal, uvs) = match face {
                LedgeFace::South => {
                    let plane_z = geometry.origin_z
                        + (origin_row + plane_subtile as isize) as f32 * geometry.tile_height;
                    (
                        [
                            [x1, band_bottom, plane_z],
                            [x1, band_top, plane_z],
                            [x0, band_top, plane_z],
                            [x0, band_bottom, plane_z],
                        ],
                        [0.0, 0.0, 1.0],
                        [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                    )
                }
                LedgeFace::West => {
                    let plane_x = geometry.origin_x
                        + (origin_column + plane_subtile as isize) as f32 * geometry.tile_width;
                    (
                        [
                            [plane_x, band_bottom, z0],
                            [plane_x, band_top, z0],
                            [plane_x, band_top, z1],
                            [plane_x, band_bottom, z1],
                        ],
                        [-1.0, 0.0, 0.0],
                        [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
                    )
                }
                LedgeFace::East => {
                    let plane_x = geometry.origin_x
                        + (origin_column + plane_subtile as isize) as f32 * geometry.tile_width;
                    (
                        [
                            [plane_x, band_bottom, z1],
                            [plane_x, band_top, z1],
                            [plane_x, band_top, z0],
                            [plane_x, band_bottom, z0],
                        ],
                        [1.0, 0.0, 0.0],
                        [[u1, v1], [u0, v1], [u0, v0], [u1, v0]],
                    )
                }
            };
            append_quad(&mut mesh.textured, positions, normal, uvs, TEXTURED_SHADE);
            debug_assert_eq!(band_count as f32 * geometry.tile_height, raised_height);
        }
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
            matches!(shape, CellShape::Flat) && cells[*index].source.tile_index == ground_tile_index
        })
        // Coordinate order, not DTO order, makes the authored source stable.
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
    if solid == SolidKind::Tree {
        append_round_tree_hull(textured, solid_mesh, removable_ground, bounds, uv);
        return Ok(());
    }

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

fn append_round_tree_hull(
    textured: &mut SurfaceMeshData,
    solid_mesh: &mut SurfaceMeshData,
    removable_ground: &[bool; 64],
    bounds: [f32; 5],
    uv: [f32; 4],
) {
    let [x0, x1, band_bottom, band_top, plane_z] = bounds;
    let [u0, u1, v0, v1] = uv;
    let max_depth = upright_depth(SolidKind::Tree);
    let center_z = plane_z - max_depth * 0.5;
    let mut chords = [None; SOURCE_TILE_PIXELS * SOURCE_TILE_PIXELS];

    for pixel_y in 0..SOURCE_TILE_PIXELS {
        let solid_columns: Vec<_> = (0..SOURCE_TILE_PIXELS)
            .filter(|&pixel_x| !removable_ground[pixel_y * SOURCE_TILE_PIXELS + pixel_x])
            .collect();
        let (Some(&left), Some(&right)) = (solid_columns.first(), solid_columns.last()) else {
            continue;
        };
        let center_x = (left + right + 1) as f32 * 0.5;
        let radius = (right - left + 1) as f32 * 0.5;
        for pixel_x in solid_columns {
            let dx = pixel_x as f32 + 0.5 - center_x;
            let normalized = (1.0 - (dx / radius).powi(2)).max(0.0).sqrt();
            let depth = (max_depth * normalized).max(max_depth / SOURCE_TILE_PIXELS as f32);
            chords[pixel_y * SOURCE_TILE_PIXELS + pixel_x] =
                Some((center_z - depth * 0.5, center_z + depth * 0.5));
        }
    }

    let chord_at = |x: isize, y: isize| {
        if x < 0 || y < 0 || x >= SOURCE_TILE_PIXELS as isize || y >= SOURCE_TILE_PIXELS as isize {
            None
        } else {
            chords[y as usize * SOURCE_TILE_PIXELS + x as usize]
        }
    };

    for pixel_y in 0..SOURCE_TILE_PIXELS {
        for pixel_x in 0..SOURCE_TILE_PIXELS {
            let Some((back_z, front_z)) = chord_at(pixel_x as isize, pixel_y as isize) else {
                continue;
            };
            let x_start = lerp_pixel(x0, x1, pixel_x);
            let x_end = lerp_pixel(x0, x1, pixel_x + 1);
            let y_top = lerp_pixel(band_top, band_bottom, pixel_y);
            let y_bottom = lerp_pixel(band_top, band_bottom, pixel_y + 1);
            let pixel_u0 = lerp_pixel(u0, u1, pixel_x);
            let pixel_u1 = lerp_pixel(u0, u1, pixel_x + 1);
            let pixel_v0 = lerp_pixel(v0, v1, pixel_y);
            let pixel_v1 = lerp_pixel(v0, v1, pixel_y + 1);

            append_quad(
                textured,
                [
                    [x_end, y_bottom, front_z],
                    [x_end, y_top, front_z],
                    [x_start, y_top, front_z],
                    [x_start, y_bottom, front_z],
                ],
                [0.0, 0.0, 1.0],
                [
                    [pixel_u1, pixel_v1],
                    [pixel_u1, pixel_v0],
                    [pixel_u0, pixel_v0],
                    [pixel_u0, pixel_v1],
                ],
                TEXTURED_SHADE,
            );
            append_quad(
                textured,
                [
                    [x_start, y_bottom, back_z],
                    [x_start, y_top, back_z],
                    [x_end, y_top, back_z],
                    [x_end, y_bottom, back_z],
                ],
                [0.0, 0.0, -1.0],
                [
                    [pixel_u0, pixel_v1],
                    [pixel_u0, pixel_v0],
                    [pixel_u1, pixel_v0],
                    [pixel_u1, pixel_v1],
                ],
                [0.68, 0.68, 0.68, 1.0],
            );

            for (neighbor, x, normal) in [
                (
                    chord_at(pixel_x as isize - 1, pixel_y as isize),
                    x_start,
                    [-1.0, 0.0, 0.0],
                ),
                (
                    chord_at(pixel_x as isize + 1, pixel_y as isize),
                    x_end,
                    [1.0, 0.0, 0.0],
                ),
            ] {
                for (z0, z1) in exposed_chord_intervals(back_z, front_z, neighbor) {
                    append_solid_quad(
                        solid_mesh,
                        [
                            [x, y_bottom, z1],
                            [x, y_top, z1],
                            [x, y_top, z0],
                            [x, y_bottom, z0],
                        ],
                        normal,
                        solid_color(SolidKind::Tree, Direction::West),
                    );
                }
            }

            for (neighbor, y, normal) in [
                (
                    chord_at(pixel_x as isize, pixel_y as isize - 1),
                    y_top,
                    [0.0, 1.0, 0.0],
                ),
                (
                    chord_at(pixel_x as isize, pixel_y as isize + 1),
                    y_bottom,
                    [0.0, -1.0, 0.0],
                ),
            ] {
                for (z0, z1) in exposed_chord_intervals(back_z, front_z, neighbor) {
                    append_solid_quad(
                        solid_mesh,
                        [
                            [x_start, y, z0],
                            [x_start, y, z1],
                            [x_end, y, z1],
                            [x_end, y, z0],
                        ],
                        normal,
                        solid_color(SolidKind::Tree, Direction::South),
                    );
                }
            }
        }
    }
}

fn exposed_chord_intervals(start: f32, end: f32, neighbor: Option<(f32, f32)>) -> Vec<(f32, f32)> {
    let Some((neighbor_start, neighbor_end)) = neighbor else {
        return vec![(start, end)];
    };
    let mut intervals = Vec::with_capacity(2);
    if neighbor_start > start {
        intervals.push((start, neighbor_start.min(end)));
    }
    if neighbor_end < end {
        intervals.push((neighbor_end.max(start), end));
    }
    intervals.retain(|(interval_start, interval_end)| interval_end > interval_start);
    intervals
}

fn upright_depth(solid: SolidKind) -> f32 {
    match solid {
        SolidKind::Tree => 8.0,
        SolidKind::Prop => 2.0,
        _ => 1.0,
    }
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
                }
            }
        }
    }
    let removable_group = boundary_connected_mask(pixel_width, pixel_height, &equals_ground);
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

#[derive(Clone, Copy)]
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
}

fn append_exposed_sides(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    cells: &[&VisualTile],
    shapes: &[CellShape],
    column: usize,
    row: usize,
) {
    let index = row * geometry.width + column;
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
        TEXTURED_SHADE,
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

fn append_quad(
    mesh: &mut SurfaceMeshData,
    positions: [[f32; 3]; 4],
    normal: [f32; 3],
    uvs: [[f32; 2]; 4],
    color: [f32; 4],
) {
    let base = u32::try_from(mesh.positions.len()).expect("MVP terrain vertex count fits u32");
    mesh.positions.extend(positions);
    mesh.normals.extend([normal; 4]);
    mesh.uvs.extend(uvs);
    mesh.colors.extend([color; 4]);
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn solid_color(kind: SolidKind, direction: Direction) -> [f32; 4] {
    let base = match kind {
        SolidKind::Building => [0.43, 0.36, 0.27],
        SolidKind::Tree => [0.20, 0.32, 0.18],
        SolidKind::Prop => [0.38, 0.34, 0.27],
        SolidKind::Bank => [0.30, 0.25, 0.18],
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
    fn blackthorn_south_ledge_uses_two_native_face_bands() {
        let mut sources = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                sources.push(source_with_tile(
                    0x72,
                    column as u8,
                    row as u8,
                    if row < 2 { 0x3c } else { 0x4c },
                ));
            }
        }
        let mesh =
            build_terrain_mesh(&frame(4, 4, sources)).expect("authored mountain ledge should mesh");
        let south_faces: Vec<_> = mesh
            .textured
            .positions
            .chunks_exact(4)
            .zip(mesh.textured.normals.chunks_exact(4))
            .filter(|(_, normal)| normal[0] == [0.0, 0.0, 1.0])
            .map(|(positions, _)| positions)
            .collect();
        assert_eq!(south_faces.len(), 8);
        assert!(south_faces.iter().all(|face| {
            let min = face
                .iter()
                .map(|vertex| vertex[1])
                .fold(f32::INFINITY, f32::min);
            let max = face
                .iter()
                .map(|vertex| vertex[1])
                .fold(f32::NEG_INFINITY, f32::max);
            max - min == 8.0
        }));
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
            vec![BuildingPlacement {
                column: 0,
                row: 0,
                width: 8,
                height: 4,
                roof_rows: 2,
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
    fn gable_roof_has_two_low_eaves_and_a_center_ridge() {
        assert_eq!(gable_height(32.0, 32.0, 0.0), 32.0);
        assert_eq!(gable_height(32.0, 32.0, 16.0), 48.0);
        assert_eq!(gable_height(32.0, 32.0, 32.0), 32.0);
        let south_slope = roof_normal(32.0, 40.0, 8.0);
        let north_slope = roof_normal(40.0, 32.0, 8.0);
        assert!(south_slope[2] < 0.0);
        assert!(north_slope[2] > 0.0);
        assert!((south_slope[1] - north_slope[1]).abs() < f32::EPSILON);
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
    fn tree_art_is_grouped_upright_instead_of_face_up() {
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
            .expect("tree group should mesh as a shallow silhouette hull");
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
        assert_eq!(backs, upright, "every tree front run needs a back run");
        assert!(
            mesh.solid.quad_count() > 0,
            "the grouped tree silhouette needs a closed shallow hull"
        );
    }

    #[test]
    fn rounded_tree_hull_has_multiple_front_depths() {
        let mut textured = SurfaceMeshData::default();
        let mut solid = SurfaceMeshData::default();
        let mut background = [true; 64];
        for y in 1..7 {
            for x in 1..7 {
                background[y * 8 + x] = false;
            }
        }
        append_round_tree_hull(
            &mut textured,
            &mut solid,
            &background,
            [0.0, 8.0, 0.0, 8.0, 0.0],
            [0.0, 1.0, 0.0, 1.0],
        );

        let mut front_depths: Vec<_> = textured
            .positions
            .chunks_exact(4)
            .zip(textured.normals.chunks_exact(4))
            .filter(|(_, normals)| normals[0] == [0.0, 0.0, 1.0])
            .map(|(positions, _)| (positions[0][2] * 1000.0).round() as i32)
            .collect();
        front_depths.sort_unstable();
        front_depths.dedup();
        assert!(front_depths.len() >= 3);
        assert!(solid.quad_count() > 0);
    }

    #[test]
    fn complete_tree_metatile_becomes_one_full_depth_canopy() {
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
            vec![TreePlacement {
                column: 0,
                row: 0,
                width: 4,
                height: 4,
                ground_tile_index: 0x05,
            }]
        );

        let mut samples = TerrainImageSamples::default();
        for tile in &frame.tiles {
            let rgba = if tile.source.metatile_id == 0x05 {
                [0, 80, 0, 255]
            } else {
                [255, 255, 255, 255]
            };
            samples
                .pixels
                .insert(tile.texture.id(), TileImageSample::Rgba(rgba.repeat(64)));
        }
        let mesh = build_terrain_mesh_with_samples(&frame, &samples)
            .expect("complete tree drawing should mesh as one canopy");
        let (min_z, max_z) = mesh
            .textured
            .positions
            .iter()
            .filter(|position| position[1] > 0.0)
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min, max), position| (min.min(position[2]), max.max(position[2])),
            );
        assert!(
            max_z - min_z > 24.0,
            "a 32px tree drawing must have a crown, not four shallow tile cards"
        );
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
