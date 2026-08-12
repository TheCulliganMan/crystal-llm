//! Authored front-facing fixtures from Crystal's shared Mart atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x01;
pub(crate) const STANDARD_MART_FLOOR_TILE: u16 = 0x48;

fn is_department_store(map_id: &str) -> bool {
    let Some(floor) = map_id
        .strip_prefix("CeladonDeptStore")
        .or_else(|| map_id.strip_prefix("GoldenrodDeptStore"))
    else {
        return false;
    };
    matches!(floor, "1F" | "2F" | "3F" | "4F" | "5F" | "6F")
}

fn is_department_store_elevator(map_id: &str) -> bool {
    matches!(
        map_id,
        "CeladonDeptStoreElevator" | "GoldenrodDeptStoreElevator"
    )
}

pub(crate) fn ground_tile_for_map(map_id: &str) -> u16 {
    if is_department_store(map_id) {
        FLOOR_TILE
    } else {
        STANDARD_MART_FLOOR_TILE
    }
}

/// Fold the department-store backdrop onto one upright plane. Crystal draws
/// these source rows flat in its 2D metatile, but together they are one back
/// wall, not shallow floor objects. Rows below the wall remain unclaimed so
/// escalators and other fixtures retain their own presentation.
pub(crate) fn department_store_wall_shape(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "mart" {
        return None;
    }
    let wall_rows = if is_department_store(map_id) {
        match source.metatile_id {
            0x05 => 3,
            0x01 | 0x02 | 0x03 | 0x08 | 0x09 | 0x0a | 0x17 | 0x18 | 0x19 | 0x2f => 2,
            _ => return None,
        }
    } else if is_department_store_elevator(map_id) && matches!(source.metatile_id, 0x03 | 0x26) {
        // The shared 2x2 elevator map puts the complete cabin wall in the
        // north pair of blocks. Its first two source rows are one 16px wall;
        // the lower rows and southern blocks are the exact walkable cabin
        // floor, door mat, and stair/door mechanics and must remain flat.
        2
    } else {
        return None;
    };
    if source.subtile_row >= wall_rows {
        return None;
    }
    Some(CellShape::FacadeBand {
        plane_subtile_row: wall_rows,
        band_from_top: source.subtile_row,
        band_count: wall_rows,
        ground_tile_index: FLOOR_TILE,
        // This backdrop is not part of an outdoor building template. Mark it
        // as an independent zero-depth card so the mesher keeps the authored
        // wall bands instead of rejecting them as an incomplete building.
        solid: SolidKind::FlatCard,
    })
}

/// Floors 5F in both department stores share one exact 8x4-tile blocked
/// fixture assembled from blocks $1c/$1d. The drawing resembles a U from
/// above, but every gameplay quadrant is WALL: its apparent centre is painted
/// into the fixture, not walkable floor. Keep the complete drawing horizontal
/// on one half-cell-high surface, matching the reference renderer's counter
/// treatment without turning four source rows into a 32px wall.
pub(crate) fn department_store_fifth_floor_fixture_shape(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<CellShape> {
    if !matches!(map_id, "CeladonDeptStore5F" | "GoldenrodDeptStore5F")
        || source.tileset_id.as_ref() != "mart"
        || !matches!(source.metatile_id, 0x1c | 0x1d)
    {
        return None;
    }
    Some(CellShape::RaisedTop {
        height: 8.0,
        solid: SolidKind::Prop,
    })
}

/// Goldenrod's roof uses four blocks for one continuous south parapet. Their
/// lower two source rows are the repeated native face courses; upper rows may
/// contain corner fittings or unrelated terrace objects and stay untouched.
pub(crate) fn goldenrod_roof_south_parapet_shape(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<CellShape> {
    if map_id != "GoldenrodDeptStoreRoof"
        || source.tileset_id.as_ref() != "mart"
        || !matches!(source.metatile_id, 0x33 | 0x34 | 0x35 | 0x3b)
        || source.subtile_row < 2
    {
        return None;
    }
    Some(CellShape::FacadeBand {
        plane_subtile_row: 4,
        band_from_top: source.subtile_row - 2,
        band_count: 2,
        ground_tile_index: 0x91,
        solid: SolidKind::FlatCard,
    })
}

/// Each roof block $3b contains one complete 32x16 top-view terrace display
/// above the shared parapet courses. It is a low fixture, not floor and not
/// another wall tier; preserve each source cell once on a four-pixel surface.
pub(crate) fn goldenrod_roof_display_shape(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<CellShape> {
    if map_id != "GoldenrodDeptStoreRoof"
        || source.tileset_id.as_ref() != "mart"
        || source.metatile_id != 0x3b
        || source.subtile_row >= 2
    {
        return None;
    }
    Some(CellShape::RaisedTop {
        height: 4.0,
        solid: SolidKind::Prop,
    })
}

/// Blocks $07 and $12 each contain one 16x32 glass display rack, on opposite
/// halves. Return local coordinates within the rack rather than the metatile.
pub(crate) fn display_rack_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "mart" {
        return None;
    }
    if source.metatile_id == 0x13 {
        let expected = [
            [0x40, 0x41, 0x42, 0x2b],
            [0x50, 0x51, 0x52, 0x45],
            [0x43, 0x44, 0x5c, 0x5d],
            [0x53, 0x54, 0x1f, 0x55],
        ];
        let row = expected.get(usize::from(source.subtile_row))?;
        if row[usize::from(source.subtile_column)] != source.tile_index {
            return None;
        }
        return Some((source.subtile_column % 2, source.subtile_row));
    }
    if source.metatile_id == 0x21 {
        let expected = [
            [0x0c, 0x0d, 0x0c, 0x0d],
            [0x4c, 0x4d, 0x4c, 0x4d],
            [0x4c, 0x4d, 0x4c, 0x4d],
            [0x4e, 0x4f, 0x4e, 0x4f],
        ];
        let row = expected.get(usize::from(source.subtile_row))?;
        if row[usize::from(source.subtile_column)] != source.tile_index {
            return None;
        }
        // One metatile contains two adjacent vending machines. Keep them as
        // two independent thin cards so their touching outlines cannot fuse
        // into a single deep cabinet.
        return Some((source.subtile_column % 2, source.subtile_row));
    }
    if source.metatile_id == 0x0b {
        let expected = [[0x4a, 0x4b], [0x08, 0x09], [0x89, 0x8a], [0xa7, 0xa8]];
        if source.subtile_column >= 2
            || expected[usize::from(source.subtile_row)][usize::from(source.subtile_column)]
                != source.tile_index
        {
            return None;
        }
        return Some((source.subtile_column, source.subtile_row));
    }
    if source.metatile_id == 0x20 {
        let expected = [[0x22, 0x23], [0x32, 0x33], [0x24, 0x25], [0x34, 0x35]];
        let local_column = source.subtile_column.checked_sub(2)?;
        if local_column >= 2
            || expected[usize::from(source.subtile_row)][usize::from(local_column)]
                != source.tile_index
        {
            return None;
        }
        return Some((local_column, source.subtile_row));
    }
    // 2F's final rack crosses a metatile boundary: the upper half occupies
    // the southeast of $2c and the lower half the northeast of $2d.
    if source.subtile_column >= 2 {
        let local_column = source.subtile_column - 2;
        let (local_row, expected) = match source.metatile_id {
            0x2c if source.subtile_row >= 2 => (
                source.subtile_row - 2,
                [[0x26, 0x27], [0x36, 0x37]][usize::from(source.subtile_row - 2)],
            ),
            0x2d if source.subtile_row < 2 => (
                source.subtile_row + 2,
                [[0x28, 0x29], [0x38, 0x39]][usize::from(source.subtile_row)],
            ),
            _ => (0, [u16::MAX; 2]),
        };
        if expected[usize::from(local_column)] == source.tile_index {
            return Some((local_column, local_row));
        }
    }
    let (first_column, expected) = match source.metatile_id {
        0x07 => (0, [[0x22, 0x23], [0x32, 0x33], [0x24, 0x25], [0x34, 0x35]]),
        0x12 => (2, [[0x26, 0x27], [0x36, 0x37], [0x28, 0x29], [0x38, 0x39]]),
        _ => return None,
    };
    let local_column = source.subtile_column.checked_sub(first_column)?;
    if local_column >= 2 {
        return None;
    }
    expected
        .get(usize::from(source.subtile_row))
        .filter(|row| row[usize::from(local_column)] == source.tile_index)
        .map(|_| (local_column, source.subtile_row))
}

/// The 1F checkout counter is one continuous eight-cell run assembled from
/// the right half of $0c, all of $0d, and the left half of $0e. Its upper
/// source row is the work surface and its lower row is the native front.
pub(crate) fn shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "mart" {
        return None;
    }
    // Every standard Mart places the same compact checkout end in block
    // `$22`: `$20/$21` are its top surface and `$30/$31` its native south
    // face. Keep the adjacent three quarters of the block as ordinary shop
    // floor and build this exact 16x16 drawing as one half-cell counter.
    if source.metatile_id == 0x22 && source.subtile_column < 2 && source.subtile_row < 2 {
        let expected = [[0x20, 0x21], [0x30, 0x31]];
        if source.tile_index
            == expected[usize::from(source.subtile_row)][usize::from(source.subtile_column)]
        {
            return if source.subtile_row == 0 {
                Some(CellShape::RaisedTop {
                    height: 8.0,
                    solid: SolidKind::Prop,
                })
            } else {
                Some(CellShape::LedgeBand {
                    face: LedgeFace::South,
                    plane_subtile: 2,
                    band_from_top: 0,
                    band_count: 1,
                    top_tile_index: expected[0][usize::from(source.subtile_column)],
                    height: 8.0,
                })
            };
        }
    }
    // These blocks are complete four-row merchandise shelves. Their source
    // drawing describes a tall front, not four shallow objects lying on the
    // floor. Fold every native row exactly once onto one zero-depth plane and
    // restore ordinary shop floor beneath the vacated drawing.
    if matches!(source.metatile_id, 0x14 | 0x15) {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
    }
    if source.subtile_row >= 2 {
        return None;
    }
    let in_counter = match source.metatile_id {
        0x0c => source.subtile_column >= 2,
        0x0d => true,
        0x0e => source.subtile_column < 2,
        _ => false,
    };
    if !in_counter {
        return None;
    }
    if source.subtile_row == 0 {
        return Some(CellShape::RaisedTop {
            height: 8.0,
            solid: SolidKind::Prop,
        });
    }
    let top_tile_index = match source.metatile_id {
        0x0c if source.subtile_column == 2 => 0x2e,
        0x0e if source.subtile_column == 1 => 0x2f,
        _ => 0x1e,
    };
    Some(CellShape::LedgeBand {
        face: LedgeFace::South,
        plane_subtile: 2,
        band_from_top: 0,
        band_count: 1,
        top_tile_index,
        height: 8.0,
    })
}

/// Exact counter-end quadrants used on department-store 1F and 3F. They are
/// blocked COUNTER cells whose art is viewed from above, so keep it on an
/// eight-pixel surface rather than leaving it painted on the shop floor.
pub(crate) fn department_store_counter_end_shape(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<CellShape> {
    if !matches!(
        map_id,
        "GoldenrodDeptStore1F"
            | "CeladonDeptStore1F"
            | "GoldenrodDeptStore3F"
            | "CeladonDeptStore3F"
    ) || source.tileset_id.as_ref() != "mart"
    {
        return None;
    }
    let exact = (source.metatile_id == 0x08
        && source.subtile_column >= 2
        && source.subtile_row >= 2
        && matches!(source.tile_index, 0x3e | 0x3f))
        || (source.metatile_id == 0x0a
            && source.subtile_column < 2
            && source.subtile_row >= 2
            && matches!(source.tile_index, 0x20 | 0x21 | 0x30 | 0x31));
    exact.then_some(CellShape::RaisedTop {
        height: 8.0,
        solid: SolidKind::Prop,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("mart"),
            metatile_id: 0x07,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn block_07_claims_only_the_complete_left_hand_rack() {
        let rows = [[0x22, 0x23], [0x32, 0x33], [0x24, 0x25], [0x34, 0x35]];
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(
                    display_rack_local(&source(
                        column,
                        row,
                        rows[usize::from(row)][usize::from(column)]
                    )),
                    Some((column, row))
                );
            }
        }
        assert_eq!(display_rack_local(&source(2, 0, FLOOR_TILE)), None);
        assert_eq!(display_rack_local(&source(0, 0, FLOOR_TILE)), None);
    }

    #[test]
    fn standard_mart_checkout_end_is_one_half_cell_counter() {
        for (column, tile) in [(0, 0x20), (1, 0x21)] {
            let mut top = source(column, 0, tile);
            top.metatile_id = 0x22;
            assert_eq!(
                shape(&top),
                Some(CellShape::RaisedTop {
                    height: 8.0,
                    solid: SolidKind::Prop,
                })
            );

            let mut front = source(column, 1, 0x30 + column as u16);
            front.metatile_id = 0x22;
            assert_eq!(
                shape(&front),
                Some(CellShape::LedgeBand {
                    face: LedgeFace::South,
                    plane_subtile: 2,
                    band_from_top: 0,
                    band_count: 1,
                    top_tile_index: tile,
                    height: 8.0,
                })
            );
        }
        let mut floor = source(2, 0, STANDARD_MART_FLOOR_TILE);
        floor.metatile_id = 0x22;
        assert_eq!(shape(&floor), None);
    }

    #[test]
    fn block_12_claims_only_the_complete_right_hand_rack() {
        let rows = [[0x26, 0x27], [0x36, 0x37], [0x28, 0x29], [0x38, 0x39]];
        for row in 0..4 {
            for column in 0..2 {
                let mut tile = source(2 + column, row, rows[usize::from(row)][usize::from(column)]);
                tile.metatile_id = 0x12;
                assert_eq!(display_rack_local(&tile), Some((column, row)));
            }
        }
        let mut floor = source(0, 0, FLOOR_TILE);
        floor.metatile_id = 0x12;
        assert_eq!(display_rack_local(&floor), None);
    }

    #[test]
    fn block_13_splits_its_two_merchandise_displays() {
        let rows = [
            [0x40, 0x41, 0x42, 0x2b],
            [0x50, 0x51, 0x52, 0x45],
            [0x43, 0x44, 0x5c, 0x5d],
            [0x53, 0x54, 0x1f, 0x55],
        ];
        for row in 0..4 {
            for column in 0..4 {
                let mut tile = source(column, row, rows[usize::from(row)][usize::from(column)]);
                tile.metatile_id = 0x13;
                assert_eq!(display_rack_local(&tile), Some((column % 2, row)));
            }
        }
    }

    #[test]
    fn block_21_splits_its_two_vending_machines() {
        let rows = [
            [0x0c, 0x0d, 0x0c, 0x0d],
            [0x4c, 0x4d, 0x4c, 0x4d],
            [0x4c, 0x4d, 0x4c, 0x4d],
            [0x4e, 0x4f, 0x4e, 0x4f],
        ];
        for row in 0..4 {
            for column in 0..4 {
                let mut tile = source(column, row, rows[usize::from(row)][usize::from(column)]);
                tile.metatile_id = 0x21;
                assert_eq!(display_rack_local(&tile), Some((column % 2, row)));
            }
        }
    }

    #[test]
    fn sixth_floor_machine_variants_resolve_their_exact_metatile_halves() {
        for (metatile_id, first_column, rows) in [
            (
                0x0b,
                0,
                [[0x4a, 0x4b], [0x08, 0x09], [0x89, 0x8a], [0xa7, 0xa8]],
            ),
            (
                0x20,
                2,
                [[0x22, 0x23], [0x32, 0x33], [0x24, 0x25], [0x34, 0x35]],
            ),
        ] {
            for row in 0..4 {
                for column in 0..2 {
                    let mut tile = source(
                        first_column + column,
                        row,
                        rows[usize::from(row)][usize::from(column)],
                    );
                    tile.metatile_id = metatile_id;
                    assert_eq!(display_rack_local(&tile), Some((column, row)));
                }
            }
        }
    }

    #[test]
    fn second_floor_cross_metatile_rack_remains_one_complete_drawing() {
        for (metatile_id, source_row, tiles, local_row) in [
            (0x2c, 2, [0x26, 0x27], 0),
            (0x2c, 3, [0x36, 0x37], 1),
            (0x2d, 0, [0x28, 0x29], 2),
            (0x2d, 1, [0x38, 0x39], 3),
        ] {
            for (column, tile_index) in tiles.into_iter().enumerate() {
                let mut tile = source(column as u8 + 2, source_row, tile_index);
                tile.metatile_id = metatile_id;
                assert_eq!(display_rack_local(&tile), Some((column as u8, local_row)));
            }
        }
    }

    #[test]
    fn first_and_third_floor_counter_ends_are_half_cell_top_surfaces() {
        for map_id in [
            "GoldenrodDeptStore1F",
            "CeladonDeptStore1F",
            "GoldenrodDeptStore3F",
            "CeladonDeptStore3F",
        ] {
            for (metatile_id, column, row, tile_index) in [
                (0x08, 2, 2, 0x3e),
                (0x08, 3, 3, 0x3f),
                (0x0a, 0, 2, 0x20),
                (0x0a, 1, 3, 0x31),
            ] {
                let mut tile = source(column, row, tile_index);
                tile.metatile_id = metatile_id;
                assert_eq!(
                    department_store_counter_end_shape(map_id, &tile),
                    Some(CellShape::RaisedTop {
                        height: 8.0,
                        solid: SolidKind::Prop,
                    })
                );
            }
        }
    }

    #[test]
    fn tall_shelf_blocks_fold_each_native_row_once_without_volume() {
        for metatile_id in [0x14, 0x15] {
            for row in 0..4 {
                let mut tile = source(0, row, 0);
                tile.metatile_id = metatile_id;
                assert_eq!(
                    shape(&tile),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row,
                        band_count: 4,
                        ground_tile_index: FLOOR_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
        }
    }

    #[test]
    fn checkout_counter_keeps_one_top_and_one_front_row() {
        let mut top = source(2, 0, 0x2e);
        top.metatile_id = 0x0c;
        assert_eq!(
            shape(&top),
            Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            })
        );

        let mut front = source(1, 1, 0x19);
        front.metatile_id = 0x0e;
        assert_eq!(
            shape(&front),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 2,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x2f,
                height: 8.0,
            })
        );

        let mut floor = source(2, 2, FLOOR_TILE);
        floor.metatile_id = 0x0c;
        assert_eq!(shape(&floor), None);
    }

    #[test]
    fn department_store_wall_folds_only_native_wall_rows() {
        let mut wall = source(0, 1, 0x1c);
        wall.metatile_id = 0x03;
        assert_eq!(
            department_store_wall_shape("CeladonDeptStore2F", &wall),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: FLOOR_TILE,
                solid: SolidKind::FlatCard,
            })
        );
        wall.subtile_row = 2;
        assert_eq!(
            department_store_wall_shape("CeladonDeptStore2F", &wall),
            None
        );
    }

    #[test]
    fn elevator_surround_keeps_its_third_wall_row() {
        let mut wall = source(2, 2, 0x1b);
        wall.metatile_id = 0x05;
        assert_eq!(
            department_store_wall_shape("GoldenrodDeptStore6F", &wall),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 3,
                band_from_top: 2,
                band_count: 3,
                ground_tile_index: FLOOR_TILE,
                solid: SolidKind::FlatCard,
            })
        );
    }

    #[test]
    fn department_store_elevator_folds_only_the_north_cabin_wall() {
        for map_id in ["GoldenrodDeptStoreElevator", "CeladonDeptStoreElevator"] {
            for metatile_id in [0x03, 0x26] {
                for row in 0..2 {
                    let mut wall = source(0, row, 0);
                    wall.metatile_id = metatile_id;
                    assert_eq!(
                        department_store_wall_shape(map_id, &wall),
                        Some(CellShape::FacadeBand {
                            plane_subtile_row: 2,
                            band_from_top: row,
                            band_count: 2,
                            ground_tile_index: FLOOR_TILE,
                            solid: SolidKind::FlatCard,
                        })
                    );
                }
                let mut floor = source(0, 2, FLOOR_TILE);
                floor.metatile_id = metatile_id;
                assert_eq!(department_store_wall_shape(map_id, &floor), None);
            }
        }

        let mut southern_door = source(0, 0, FLOOR_TILE);
        southern_door.metatile_id = 0x06;
        assert_eq!(
            department_store_wall_shape("GoldenrodDeptStoreElevator", &southern_door),
            None
        );
    }

    #[test]
    fn fifth_floor_fixture_is_one_half_cell_surface_not_a_four_row_wall() {
        for map_id in ["GoldenrodDeptStore5F", "CeladonDeptStore5F"] {
            for metatile_id in [0x1c, 0x1d] {
                for row in 0..4 {
                    for column in 0..4 {
                        let mut tile = source(column, row, 0);
                        tile.metatile_id = metatile_id;
                        assert_eq!(
                            department_store_fifth_floor_fixture_shape(map_id, &tile),
                            Some(CellShape::RaisedTop {
                                height: 8.0,
                                solid: SolidKind::Prop,
                            })
                        );
                    }
                }
            }
        }

        let mut same_art = source(0, 0, 0);
        same_art.metatile_id = 0x1c;
        assert_eq!(
            department_store_fifth_floor_fixture_shape("GoldenrodGameCorner", &same_art),
            None
        );
    }

    #[test]
    fn roof_south_parapet_folds_only_its_two_repeated_face_courses() {
        for metatile_id in [0x33, 0x34, 0x35, 0x3b] {
            for row in 0..4 {
                let mut tile = source(0, row, if row == 2 { 0xa1 } else { 0x96 });
                tile.metatile_id = metatile_id;
                let expected = if row >= 2 {
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row - 2,
                        band_count: 2,
                        ground_tile_index: 0x91,
                        solid: SolidKind::FlatCard,
                    })
                } else {
                    None
                };
                assert_eq!(
                    goldenrod_roof_south_parapet_shape("GoldenrodDeptStoreRoof", &tile),
                    expected
                );
            }
        }

        let mut shared_art = source(0, 2, 0xa1);
        shared_art.metatile_id = 0x34;
        assert_eq!(
            goldenrod_roof_south_parapet_shape("GoldenrodDeptStore6F", &shared_art),
            None
        );
    }

    #[test]
    fn each_roof_display_lifts_only_its_complete_upper_half() {
        for row in 0..4 {
            for column in 0..4 {
                let mut tile = source(column, row, 0);
                tile.metatile_id = 0x3b;
                assert_eq!(
                    goldenrod_roof_display_shape("GoldenrodDeptStoreRoof", &tile),
                    if row < 2 {
                        Some(CellShape::RaisedTop {
                            height: 4.0,
                            solid: SolidKind::Prop,
                        })
                    } else {
                        None
                    }
                );
            }
        }
    }

    #[test]
    fn wall_profile_is_scoped_and_does_not_capture_merchandise() {
        let mut shelf = source(0, 0, 0x40);
        shelf.metatile_id = 0x13;
        assert_eq!(
            department_store_wall_shape("CeladonDeptStore2F", &shelf),
            None
        );
        shelf.metatile_id = 0x03;
        assert_eq!(department_store_wall_shape("CeladonMart", &shelf), None);
    }

    #[test]
    fn ordinary_and_department_store_racks_use_their_native_floors() {
        assert_eq!(ground_tile_for_map("PewterMart"), STANDARD_MART_FLOOR_TILE);
        assert_eq!(ground_tile_for_map("CeladonDeptStore3F"), FLOOR_TILE);
    }
}
