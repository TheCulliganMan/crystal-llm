//! Flat visual continuation beyond the host's real connected-map halo.
//!
//! The host publishes actual neighboring map data around the viewport. A
//! pitched camera can still see beyond that finite grid, however. This apron
//! repeats only the outermost authored metatile course underneath the real
//! mesh. It is presentation-only: it creates no walls, footing, collision, or
//! inferred topology.

use super::*;

// The real host halo already contributes twelve tiles on every side. Keep a
// much larger presentation-only apron beyond it so perspective views and
// future camera yaw cannot reach the clear color in any direction.
const BACKGROUND_APRON_TILES: isize = 96;
const METATILE_PERIOD: usize = 4;
const BACKGROUND_HEIGHT: f32 = -0.04;

pub(super) fn append_repeating_background_apron(
    mesh: &mut TerrainMeshData,
    geometry: &GridGeometry,
    cells: &[&VisualTile],
    shapes: &[CellShape],
) {
    // Small synthetic grids are used throughout the mesher's exact-geometry
    // tests. The real optional host frame includes its connected-map halo and
    // is at least 40x38 cells.
    if geometry.width < 40 || geometry.height < 38 {
        return;
    }
    let width = geometry.width as isize;
    let height = geometry.height as isize;
    let Some((background_column, background_row)) =
        dominant_flat_source(geometry.width, geometry.height, cells, shapes)
    else {
        return;
    };

    for row in -BACKGROUND_APRON_TILES..height + BACKGROUND_APRON_TILES {
        for column in -BACKGROUND_APRON_TILES..width + BACKGROUND_APRON_TILES {
            if (0..width).contains(&column) && (0..height).contains(&row) {
                continue;
            }

            let x0 = geometry.origin_x + column as f32 * geometry.tile_width;
            let z0 = geometry.origin_z + row as f32 * geometry.tile_height;
            append_top(
                &mut mesh.textured,
                [x0, x0 + geometry.tile_width, z0, z0 + geometry.tile_height],
                BACKGROUND_HEIGHT,
                geometry.uv(background_column, background_row),
            );
        }
    }
}

fn dominant_flat_source(
    width: usize,
    height: usize,
    cells: &[&VisualTile],
    shapes: &[CellShape],
) -> Option<(usize, usize)> {
    let mut counts: HashMap<(u16, u8, u8, u16), (usize, usize, usize)> = HashMap::new();
    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
            if cells[index].priority
                || !matches!(
                    shapes[index],
                    CellShape::Flat | CellShape::PlaneAt { height: 0.0 }
                )
            {
                continue;
            }
            let source = &cells[index].source;
            let key = (
                source.metatile_id,
                source.subtile_column,
                source.subtile_row,
                source.tile_index,
            );
            let entry = counts.entry(key).or_insert((0, column, row));
            entry.0 += 1;
        }
    }
    counts
        .into_values()
        .max_by_key(|(count, _, _)| *count)
        .map(|(_, column, row)| (column, row))
}

fn nearest_flat_source(
    nominal_column: usize,
    nominal_row: usize,
    width: usize,
    height: usize,
    cells: &[&VisualTile],
    shapes: &[CellShape],
) -> Option<(usize, usize)> {
    let is_flat = |column: usize, row: usize| {
        let index = row * width + column;
        !cells[index].priority
            && matches!(
                shapes[index],
                CellShape::Flat | CellShape::PlaneAt { height: 0.0 }
            )
    };
    if is_flat(nominal_column, nominal_row) {
        return Some((nominal_column, nominal_row));
    }

    // Search outward for actual ground art. This deliberately refuses tree,
    // building, rock, water, ledge, and cutout artwork rather than flattening
    // an object into the distant floor.
    for radius in 1..width.max(height) {
        let min_column = nominal_column.saturating_sub(radius);
        let max_column = (nominal_column + radius).min(width - 1);
        let min_row = nominal_row.saturating_sub(radius);
        let max_row = (nominal_row + radius).min(height - 1);
        for column in min_column..=max_column {
            for row in [min_row, max_row] {
                if is_flat(column, row) {
                    return Some((column, row));
                }
            }
        }
        for row in min_row.saturating_add(1)..max_row {
            for column in [min_column, max_column] {
                if is_flat(column, row) {
                    return Some((column, row));
                }
            }
        }
    }
    None
}

fn repeated_edge_coordinate(coordinate: isize, length: usize) -> usize {
    let period = length.min(METATILE_PERIOD);
    debug_assert!(period > 0);
    if coordinate < 0 {
        coordinate.rem_euclid(period as isize) as usize
    } else if coordinate >= length as isize {
        length - period + (coordinate - length as isize) as usize % period
    } else {
        coordinate as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_the_nearest_outer_metatile_course() {
        assert_eq!(repeated_edge_coordinate(-1, 44), 3);
        assert_eq!(repeated_edge_coordinate(-4, 44), 0);
        assert_eq!(repeated_edge_coordinate(44, 44), 40);
        assert_eq!(repeated_edge_coordinate(47, 44), 43);
        assert_eq!(repeated_edge_coordinate(48, 44), 40);
    }

    #[test]
    fn raised_objects_are_never_flattened_into_the_background() {
        let mut shapes = vec![CellShape::Flat; 16];
        shapes[0] = CellShape::Water;
        shapes[1] = CellShape::RaisedTop {
            height: 16.0,
            solid: SolidKind::Tree,
        };
        let frame = test_frame_with_grid(4, 4);
        let cells: Vec<_> = frame.tiles.iter().collect();

        assert_eq!(
            nearest_flat_source(0, 0, 4, 4, &cells, &shapes),
            Some((0, 1))
        );
        assert_ne!(
            nearest_flat_source(0, 0, 4, 4, &cells, &shapes),
            Some((1, 0))
        );
    }

    #[test]
    fn apron_is_flat_and_extends_every_grid_edge_without_solid_geometry() {
        let geometry = GridGeometry {
            width: 40,
            height: 40,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: -16.0,
            origin_z: -16.0,
        };
        let mut mesh = TerrainMeshData::default();
        let shapes = vec![CellShape::Flat; geometry.width * geometry.height];
        let frame = test_frame_with_grid(geometry.width, geometry.height);
        let cells: Vec<_> = frame.tiles.iter().collect();

        append_repeating_background_apron(&mut mesh, &geometry, &cells, &shapes);

        let extended_width = geometry.width + BACKGROUND_APRON_TILES as usize * 2;
        let extended_height = geometry.height + BACKGROUND_APRON_TILES as usize * 2;
        let expected_quads = extended_width * extended_height - geometry.width * geometry.height;
        assert_eq!(mesh.textured.positions.len(), expected_quads * 4);
        assert!(
            mesh.textured
                .positions
                .iter()
                .all(|position| position[1] == BACKGROUND_HEIGHT)
        );
        assert!(mesh.solid.positions.is_empty());

        let min_x = mesh
            .textured
            .positions
            .iter()
            .map(|position| position[0])
            .fold(f32::INFINITY, f32::min);
        let min_z = mesh
            .textured
            .positions
            .iter()
            .map(|position| position[2])
            .fold(f32::INFINITY, f32::min);
        assert_eq!(
            min_x,
            geometry.origin_x - BACKGROUND_APRON_TILES as f32 * geometry.tile_width
        );
        assert_eq!(
            min_z,
            geometry.origin_z - BACKGROUND_APRON_TILES as f32 * geometry.tile_height
        );
    }

    fn test_frame_with_grid(width: usize, height: usize) -> VisualWorldFrame {
        use std::sync::Arc;

        use bevy::prelude::{Handle, UVec2, Vec2};

        let mut frame = VisualWorldFrame {
            active: true,
            map_id: Arc::from("test"),
            map_texture: Handle::default(),
            center: Vec2::ZERO,
            viewport_size: Vec2::new(width as f32 * 8.0, height as f32 * 8.0),
            tile_size: Vec2::splat(8.0),
            grid_size: UVec2::new(width as u32, height as u32),
            tiles: Vec::new(),
            actors: Vec::new(),
            terrain_revision: 0,
        };
        frame.tiles = (0..height)
            .flat_map(|row| {
                (0..width).map(move |column| VisualTile {
                    column: column as u32,
                    row: row as u32,
                    source: VisualTileSource {
                        tileset_id: Arc::from("test"),
                        metatile_id: 0,
                        subtile_column: (column % 4) as u8,
                        subtile_row: (row % 4) as u8,
                        tile_index: 0,
                    },
                    texture: Handle::default(),
                    priority: false,
                })
            })
            .collect();
        frame
    }
}
