//! Continuous authored north walls for the four player-house layouts.

use super::*;

const PLAYERS_HOUSE_1F_COURSE: &[u16] = &[0x07, 0x0f, 0x11, 0x05, 0x0a];
const ELMS_HOUSE_COURSE: &[u16] = &[0x10, 0x01, 0x11, 0x1b];
const REDS_HOUSE_1F_COURSE: &[u16] = &[0x1b, 0x11, 0x01, 0x0a];
const COPYCATS_HOUSE_1F_COURSE: &[u16] = &[0x1b, 0x23, 0x11, 0x01];
const REDS_HOUSE_2F_COURSE: &[u16] = &[0x10, 0x20, 0x1b, 0x0b];
const COPYCATS_HOUSE_2F_COURSE: &[u16] = &[0x1b, 0x0b, 0x02, 0x20, 0x04];
const COURSE_VARIANTS: [&[u16]; 6] = [
    PLAYERS_HOUSE_1F_COURSE,
    ELMS_HOUSE_COURSE,
    REDS_HOUSE_1F_COURSE,
    COPYCATS_HOUSE_1F_COURSE,
    REDS_HOUSE_2F_COURSE,
    COPYCATS_HOUSE_2F_COURSE,
];
const COURSE_HEIGHT: usize = 4;

pub(super) fn append_north_wall_courses(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    for (column, row, width) in course_origins(cells, geometry) {
        append_course(mesh, cells, geometry, claimed, column, row, width)?;
    }
    Ok(())
}

pub(super) fn course_origins(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<(usize, usize, usize)> {
    let mut origins = Vec::new();
    for row in 0..geometry.height.saturating_sub(COURSE_HEIGHT - 1) {
        for column in 0..geometry.width {
            let matched_width = COURSE_VARIANTS.iter().find_map(|variant| {
                let width = variant.len() * 4;
                if column + width > geometry.width {
                    return None;
                }
                variant
                    .iter()
                    .enumerate()
                    .all(|(block, expected)| {
                        (0..4).all(|local_row| {
                            (0..4).all(|local_column| {
                                let tile = cells[(row + local_row) * geometry.width
                                    + column
                                    + block * 4
                                    + local_column];
                                tile.source.tileset_id.as_ref() == "players_house"
                                    && tile.source.metatile_id == *expected
                                    && usize::from(tile.source.subtile_column) == local_column
                                    && usize::from(tile.source.subtile_row) == local_row
                            })
                        })
                    })
                    .then_some(width)
            });
            if let Some(width) = matched_width {
                origins.push((column, row, width));
            }
        }
    }
    origins
}

fn append_course(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
    column: usize,
    row: usize,
    width: usize,
) -> Result<(), TerrainMeshError> {
    let ground_index = cells
        .iter()
        .position(|tile| {
            tile.source.tileset_id.as_ref() == "players_house" && tile.source.tile_index == 0x01
        })
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: column as u32,
            row: row as u32,
            tile_index: 0x01,
        })?;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    let plane_z = geometry.origin_z + (row + COURSE_HEIGHT) as f32 * geometry.tile_height;
    for local_row in 0..COURSE_HEIGHT {
        let band_bottom = (COURSE_HEIGHT - local_row - 1) as f32 * geometry.tile_height;
        let band_top = band_bottom + geometry.tile_height;
        for local_column in 0..width {
            let source_column = column + local_column;
            let source_row = row + local_row;
            let index = source_row * geometry.width + source_column;
            // Block `$0f` carries the upstairs flight inside the otherwise
            // continuous north-wall course. Those four cells are traversable
            // staircase art, not wall bands; leave them unclaimed so the
            // authored ramp profile can emit the flight at its map position.
            if crate::players_house::stair_shape(&cells[index].source).is_some() {
                continue;
            }
            claimed[index] = true;
            let (x0, x1, z0, z1) = geometry.bounds(source_column, source_row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
            let (u0, u1, v0, v1) = geometry.uv(source_column, source_row);
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Handle;
    use std::sync::Arc;

    fn tile(column: u32, row: u32) -> VisualTile {
        VisualTile {
            column,
            row,
            source: VisualTileSource {
                tileset_id: Arc::from("players_house"),
                metatile_id: 0,
                subtile_column: (column % 4) as u8,
                subtile_row: (row % 4) as u8,
                tile_index: 0x11,
            },
            texture: Handle::default(),
            priority: false,
        }
    }

    #[test]
    fn player_house_courses_include_their_complete_wall_widths() {
        assert_eq!(COURSE_VARIANTS.len(), 6);
        assert_eq!(PLAYERS_HOUSE_1F_COURSE, &[0x07, 0x0f, 0x11, 0x05, 0x0a]);
        assert_eq!(PLAYERS_HOUSE_1F_COURSE.len() * 4, 20);
        assert_eq!(COPYCATS_HOUSE_2F_COURSE.len() * 4, 20);
        assert_eq!(REDS_HOUSE_2F_COURSE.len() * 4, 16);
    }

    #[test]
    fn player_house_course_does_not_claim_the_embedded_stair_flight() {
        let mut tiles = (0..4)
            .flat_map(|row| (0..20).map(move |column| tile(column, row)))
            .collect::<Vec<_>>();
        let blocks = PLAYERS_HOUSE_1F_COURSE;
        for (block_column, block) in blocks.iter().enumerate() {
            for local_row in 0..4 {
                for local_column in 0..4 {
                    let tile = &mut tiles[local_row * 20 + block_column * 4 + local_column];
                    tile.source.tileset_id = Arc::from("players_house");
                    tile.source.metatile_id = *block;
                    tile.source.subtile_column = local_column as u8;
                    tile.source.subtile_row = local_row as u8;
                }
            }
        }
        for (local_row, drawing) in [[0x0a, 0x0b], [0x1a, 0x1b]].into_iter().enumerate() {
            for (local_column, tile_index) in drawing.into_iter().enumerate() {
                tiles[(local_row + 1) * 20 + 4 + local_column]
                    .source
                    .tile_index = tile_index;
            }
        }
        tiles[19].source.tile_index = 0x01;
        let cells = tiles.iter().collect::<Vec<_>>();
        let geometry = GridGeometry {
            width: 20,
            height: 4,
            tile_width: 8.0,
            tile_height: 8.0,
            origin_x: 0.0,
            origin_z: 0.0,
        };
        let mut mesh = TerrainMeshData::default();
        let mut claimed = vec![false; 80];
        append_north_wall_courses(&mut mesh, &cells, &geometry, &mut claimed).expect("wall course");
        for row in 1..=2 {
            for column in 4..=5 {
                assert!(!claimed[row * 20 + column]);
            }
        }
        assert!(claimed[0]);
        assert!(claimed[3 * 20 + 19]);
    }
}
