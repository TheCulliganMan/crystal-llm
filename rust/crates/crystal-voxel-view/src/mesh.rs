//! Pure authored-profile mesh construction for the optional voxel renderer.

use std::collections::{HashMap, VecDeque};

use bevy::{
    asset::AssetId,
    prelude::{Assets, Image, Mesh},
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use crystal_render_api::{VisualTile, VisualWorldFrame};

use crate::profile::{CellShape, SOURCE_TILE_HEIGHT, SolidKind, shape_for_source};

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
    if let Some(images) = images {
        let placements = new_bark_building_placements(frame, &cells, &geometry);
        for placement in &placements {
            append_pixel_building(
                &mut mesh,
                images,
                &cells,
                &geometry,
                *placement,
                &mut claimed_by_building,
            )?;
        }
        // A partial template at the viewport edge is not enough evidence to
        // invent half a building. Preserve the faithful flat drawing until
        // the complete authored placement is available.
        for (index, shape) in shapes.iter_mut().enumerate() {
            if shape.solid_kind() == SolidKind::Building && !claimed_by_building[index] {
                *shape = CellShape::Flat;
            }
        }
    }

    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
            if claimed_by_building[index] {
                continue;
            }
            append_textured_cell(
                &mut mesh.textured,
                &geometry,
                &cells,
                &shapes,
                column,
                row,
                index,
                images,
            )?;
        }
    }

    for row in 0..height {
        for column in 0..width {
            if claimed_by_building[row * width + column] {
                continue;
            }
            append_exposed_sides(&mut mesh.solid, &geometry, &cells, &shapes, column, row);
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

fn new_bark_building_placements(
    frame: &VisualWorldFrame,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<BuildingPlacement> {
    if frame.map_id.as_ref() != "NewBarkTown" {
        return Vec::new();
    }
    let mut metatiles = HashMap::new();
    for tile in cells {
        if tile.source.tileset_id.as_ref() != "johto" {
            continue;
        }
        let origin = (
            tile.column as isize - tile.source.subtile_column as isize,
            tile.row as isize - tile.source.subtile_row as isize,
        );
        metatiles.entry(origin).or_insert(tile.source.metatile_id);
    }

    const TEMPLATES: &[(&[&[u16]], usize)] = &[
        (&[&[0x18, 0x1f, 0x19], &[0x1c, 0x77, 0x1e]], 4),
        (&[&[0x18, 0x19], &[0x16, 0x1e]], 4),
        (&[&[0x14, 0x15]], 2),
    ];
    let mut placements = Vec::new();
    for (&(origin_x, origin_y), &metatile_id) in &metatiles {
        for &(rows, roof_rows) in TEMPLATES {
            if rows[0][0] != metatile_id {
                continue;
            }
            let matches = rows.iter().enumerate().all(|(template_y, row)| {
                row.iter().enumerate().all(|(template_x, expected)| {
                    metatiles.get(&(
                        origin_x + (template_x * 4) as isize,
                        origin_y + (template_y * 4) as isize,
                    )) == Some(expected)
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
                    ground_tile_index: 0x06,
                });
            }
        }
    }
    placements.sort_by_key(|placement| (placement.row, placement.column));
    placements
}

fn append_pixel_building(
    mesh: &mut TerrainMeshData,
    images: &TerrainImageSamples,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    placement: BuildingPlacement,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    let ground_index = authored_ground_cell(
        cells,
        &cells
            .iter()
            .map(|tile| shape_for_source(&tile.source))
            .collect::<Vec<_>>(),
        placement.ground_tile_index,
    )
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
        .into_iter()
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
    let eave_row = roof_top
        .iter()
        .copied()
        .filter(|&row| row < roof_pixels)
        .max()
        .unwrap_or(roof_pixels);

    for y in roof_pixels..pixel_height {
        for x in 0..pixel_width {
            if !inside[y * pixel_width + x] {
                continue;
            }
            let x0 = building_x0 + x as f32 * pixel_x_size;
            let x1 = x0 + pixel_x_size;
            let top = (pixel_height - y) as f32 * pixel_z_size;
            let bottom = top - pixel_z_size;
            append_quad(
                &mut mesh.textured,
                [
                    [x1, bottom, facade_z],
                    [x1, top, facade_z],
                    [x0, top, facade_z],
                    [x0, bottom, facade_z],
                ],
                [0.0, 0.0, 1.0],
                source_pixel_uv(geometry, placement, x, y, true),
                TEXTURED_SHADE,
            );
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
            let roof_height =
                measured_roof_height(wall_height, eave_row, roof_top[x], pixel_z_size);
            append_quad(
                &mut mesh.textured,
                [
                    [x0, roof_height, z0],
                    [x0, roof_height, z1],
                    [x1, roof_height, z1],
                    [x1, roof_height, z0],
                ],
                [0.0, 1.0, 0.0],
                source_pixel_uv(geometry, placement, x, y, false),
                TEXTURED_SHADE,
            );
        }
    }

    for (x, &top_row) in roof_top.iter().enumerate() {
        if top_row >= roof_pixels {
            continue;
        }
        let roof_height = measured_roof_height(wall_height, eave_row, top_row, pixel_z_size);
        if roof_height <= wall_height {
            continue;
        }
        let x0 = building_x0 + x as f32 * pixel_x_size;
        let x1 = x0 + pixel_x_size;
        append_solid_quad(
            &mut mesh.solid,
            [
                [x1, wall_height, facade_z],
                [x1, roof_height, facade_z],
                [x0, roof_height, facade_z],
                [x0, wall_height, facade_z],
            ],
            [0.0, 0.0, 1.0],
            solid_color(SolidKind::Building, Direction::South),
        );
    }

    let x1 = building_x0 + pixel_width as f32 * pixel_x_size;
    append_solid_quad(
        &mut mesh.solid,
        [
            [building_x0, 0.0, facade_z],
            [building_x0, wall_height, facade_z],
            [building_x0, wall_height, building_z0],
            [building_x0, 0.0, building_z0],
        ],
        [-1.0, 0.0, 0.0],
        solid_color(SolidKind::Building, Direction::West),
    );
    append_solid_quad(
        &mut mesh.solid,
        [
            [x1, 0.0, building_z0],
            [x1, wall_height, building_z0],
            [x1, wall_height, facade_z],
            [x1, 0.0, facade_z],
        ],
        [1.0, 0.0, 0.0],
        solid_color(SolidKind::Building, Direction::East),
    );
    Ok(())
}

fn measured_roof_height(
    wall_height: f32,
    eave_row: usize,
    silhouette_top_row: usize,
    pixel_height: f32,
) -> f32 {
    wall_height + eave_row.saturating_sub(silhouette_top_row) as f32 * pixel_height
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
    mesh: &mut SurfaceMeshData,
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
                mesh,
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
                mesh,
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
                append_masked_upright_band(
                    mesh,
                    &removable_ground,
                    [x0, x1, band_bottom, band_top, plane_z],
                    [u0, u1, v0, v1],
                )?;
            } else {
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
            }
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

fn append_masked_upright_band(
    mesh: &mut SurfaceMeshData,
    removable_ground: &[bool; 64],
    bounds: [f32; 5],
    uv: [f32; 4],
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
                mesh,
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
    mesh: &mut SurfaceMeshData,
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
                mesh,
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
                mesh,
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
                mesh,
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
                mesh,
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

fn facade_covers_south_edge(
    tile: &VisualTile,
    shape: CellShape,
    row: usize,
    raised_height: f32,
    tile_height: f32,
) -> bool {
    let CellShape::FacadeBand {
        plane_subtile_row,
        band_from_top,
        band_count,
        ..
    } = shape
    else {
        return false;
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
            new_bark_building_placements(&frame, &cells, &geometry),
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
    fn measured_roof_anchors_the_eave_to_the_wall() {
        assert_eq!(measured_roof_height(32.0, 12, 12, 1.0), 32.0);
        assert_eq!(measured_roof_height(32.0, 12, 4, 1.0), 40.0);
    }

    #[test]
    fn tree_art_is_grouped_upright_instead_of_face_up() {
        let mut sources = Vec::new();
        for row in 0..4 {
            sources.push(source_with_tile(0x05, 0, row, 0x1e + row as u16 * 0x10));
            sources.push(source_with_tile(0x01, 0, 0, 0x05));
        }
        let mesh = build_terrain_mesh(&frame(2, 4, sources)).expect("tree group should mesh");
        let upright = mesh
            .textured
            .normals
            .chunks_exact(4)
            .filter(|face| face[0] == [0.0, 0.0, 1.0])
            .count();
        assert_eq!(upright, 4);
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
