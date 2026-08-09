//! Pure authored-profile mesh construction for the optional voxel renderer.

use bevy::{
    prelude::{Assets, Image, Mesh},
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use crystal_render_api::{VisualTile, VisualWorldFrame};

use crate::profile::{CellShape, SOURCE_TILE_HEIGHT, SolidKind, shape_for_source};

const TEXTURED_SHADE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SOURCE_TILE_PIXELS: usize = SOURCE_TILE_HEIGHT as usize;

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
    build_terrain_mesh_internal(frame, Some(images))
}

fn build_terrain_mesh_internal(
    frame: &VisualWorldFrame,
    images: Option<&Assets<Image>>,
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

    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
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
            append_exposed_sides(&mut mesh.solid, &geometry, &cells, &shapes, column, row);
        }
    }

    Ok(mesh)
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
    images: Option<&Assets<Image>>,
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
                append_masked_upright_band(
                    mesh,
                    images,
                    tile,
                    cells[replacement],
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
            matches!(shape, CellShape::Flat)
                && cells[*index].source.tile_index == ground_tile_index
        })
        // Coordinate order, not DTO order, makes the authored source stable.
        .min_by_key(|(index, _)| (cells[*index].row, cells[*index].column))
        .map(|(index, _)| index)
}

fn append_masked_upright_band(
    mesh: &mut SurfaceMeshData,
    images: &Assets<Image>,
    source_tile: &VisualTile,
    ground_tile: &VisualTile,
    bounds: [f32; 5],
    uv: [f32; 4],
) -> Result<(), TerrainMeshError> {
    let source = tile_rgba(images, source_tile)?;
    let ground = tile_rgba(images, ground_tile)?;
    let [x0, x1, band_bottom, band_top, plane_z] = bounds;
    let [u0, u1, v0, v1] = uv;

    for pixel_y in 0..SOURCE_TILE_PIXELS {
        let mut pixel_x = 0;
        while pixel_x < SOURCE_TILE_PIXELS {
            while pixel_x < SOURCE_TILE_PIXELS && pixels_equal(source, ground, pixel_x, pixel_y) {
                pixel_x += 1;
            }
            let run_start = pixel_x;
            while pixel_x < SOURCE_TILE_PIXELS && !pixels_equal(source, ground, pixel_x, pixel_y) {
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

fn tile_rgba<'a>(
    images: &'a Assets<Image>,
    tile: &VisualTile,
) -> Result<&'a [u8], TerrainMeshError> {
    let image = images
        .get(&tile.texture)
        .ok_or(TerrainMeshError::MissingMaskImage {
            column: tile.column,
            row: tile.row,
        })?;
    let size = image.texture_descriptor.size;
    let expected_len = SOURCE_TILE_PIXELS * SOURCE_TILE_PIXELS * 4;
    if size.width as usize != SOURCE_TILE_PIXELS
        || size.height as usize != SOURCE_TILE_PIXELS
        || size.depth_or_array_layers != 1
        || image.data.len() != expected_len
    {
        return Err(TerrainMeshError::InvalidMaskImage {
            column: tile.column,
            row: tile.row,
        });
    }
    Ok(&image.data)
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
        assert!(mesh
            .textured
            .positions
            .iter()
            .all(|position| position[1] == 0.0));
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
