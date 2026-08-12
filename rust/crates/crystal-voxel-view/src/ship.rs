//! Authored bulkhead faces for the Fast Ship interior.
//!
//! The lighthouse atlas draws the main corridor wall as a cap course followed
//! by exactly two face courses. Only those two face rows fold upright; deck
//! planking, void, and the cap remain in their native top-facing orientation.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const DECK_TILE: u16 = 0x04;
const VOID_TILE: u16 = 0x01;
pub(crate) const CABIN_FLOOR_TILE: u16 = 0x0d;

fn is_cabin_map(map_id: &str) -> bool {
    map_id.starts_with("FastShipCabins")
}

fn is_fast_ship_map(map_id: &str) -> bool {
    map_id.starts_with("FastShip")
}

/// Block `$07` carries one complete 16x16 round stool in its southeast
/// quadrant. Return its local coordinate so the mesher can mask the carpet
/// around the whole drawing and stand it as one zero-depth card.
pub(crate) fn stool_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !is_cabin_map(map_id)
        || source.tileset_id.as_ref() != "lighthouse"
        || source.metatile_id != 0x07
        || source.subtile_column < 2
        || source.subtile_row < 2
    {
        return None;
    }
    let column = source.subtile_column - 2;
    let row = source.subtile_row - 2;
    let expected = [[0x07, 0x08], [0x17, 0x18]];
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// Block `$2f` carries the ship's 16x32 storage rack in its east half. The
/// cap, repeated shelf ranks, and base are one face-on drawing.
pub(crate) fn rack_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !is_cabin_map(map_id)
        || source.tileset_id.as_ref() != "lighthouse"
        || source.metatile_id != 0x2f
        || source.subtile_column < 2
    {
        return None;
    }
    let column = source.subtile_column - 2;
    let row = source.subtile_row;
    let expected = [[0x13, 0x3e], [0x36, 0x2b], [0x36, 0x2b], [0x3b, 0x3c]];
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// The same four-tile barrel drawing appears in opposite halves of blocks
/// `$2f` and `$35`. Keep it as a masked, zero-depth card: the source supplies
/// no side texture and cans are deliberately not voxelized by this mod.
pub(crate) fn barrel_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !is_fast_ship_map(map_id) || source.tileset_id.as_ref() != "lighthouse" {
        return None;
    }
    let (column, row) = match source.metatile_id {
        0x2f if source.subtile_column < 2 && source.subtile_row >= 2 => {
            (source.subtile_column, source.subtile_row - 2)
        }
        0x35 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
            (source.subtile_column - 2, source.subtile_row - 2)
        }
        _ => return None,
    };
    let expected = [[0x48, 0x49], [0x58, 0x59]];
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// Fast Ship blocks `$36` and `$38` place the same 16x16 bunk drawing in
/// different vertical halves. The mattress is top-down artwork, so it remains
/// horizontal and receives only a low bed-height lift.
pub(crate) fn bunk_shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if !is_fast_ship_map(map_id) || source.tileset_id.as_ref() != "lighthouse" {
        return None;
    }
    let (column, row) = match source.metatile_id {
        0x36 if source.subtile_column >= 2 && source.subtile_row < 2 => {
            (source.subtile_column - 2, source.subtile_row)
        }
        0x38 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
            (source.subtile_column - 2, source.subtile_row - 2)
        }
        _ => return None,
    };
    let expected = [[0x46, 0x47], [0x56, 0x57]];
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some(
        CellShape::RaisedTop {
            height: 7.0,
            solid: SolidKind::Prop,
        },
    )
}

fn b1f_vertical_divider_cell(source: &VisualTileSource) -> bool {
    match source.metatile_id {
        0x20 => {
            source.subtile_column < 2
                && source.tile_index == 0x15 + u16::from(source.subtile_column)
        }
        0x21 => {
            source.subtile_column >= 2
                && source.tile_index == 0x15 + u16::from(source.subtile_column - 2)
        }
        0x22 if source.subtile_row < 2 => {
            source.subtile_column < 2
                && source.tile_index == 0x15 + u16::from(source.subtile_column)
        }
        0x22 => {
            source.subtile_column == 0
                && source.tile_index == if source.subtile_row == 2 { 0x15 } else { 0x32 }
        }
        0x23 if source.subtile_row < 2 => {
            source.subtile_column >= 2
                && source.tile_index == 0x15 + u16::from(source.subtile_column - 2)
        }
        0x23 => {
            source.subtile_column == 3
                && source.tile_index == if source.subtile_row == 2 { 0x16 } else { 0x22 }
        }
        _ => false,
    }
}

fn b1f_horizontal_divider_cell(source: &VisualTileSource) -> bool {
    if source.subtile_row < 2 {
        return false;
    }
    let expected = match source.metatile_id {
        0x25 => match source.subtile_row {
            2 => 0x33,
            3 => 0x11,
            _ => return false,
        },
        0x22 if source.subtile_column >= 1 => match source.subtile_row {
            2 => [0x26, 0x33, 0x33][usize::from(source.subtile_column - 1)],
            3 => 0x11,
            _ => return false,
        },
        0x23 if source.subtile_column <= 2 => match source.subtile_row {
            2 => [0x33, 0x33, 0x25][usize::from(source.subtile_column)],
            3 => 0x11,
            _ => return false,
        },
        _ => return false,
    };
    source.tile_index == expected
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if let Some(shape) = bunk_shape(map_id, source) {
        return Some(shape);
    }
    if source.tileset_id.as_ref() != "lighthouse" {
        return None;
    }
    if map_id == "FastShipB1F" && b1f_vertical_divider_cell(source) {
        return Some(CellShape::RaisedTop {
            height: 16.0,
            solid: SolidKind::Prop,
        });
    }

    // Block $1a's west half is the dark flight used at both 1F deck warps.
    // Its treads descend toward the west, so the continuous surface rises
    // east across the two authored columns. Both native rows share the same
    // slope; they are depth, not a second elevation tier.
    if source.metatile_id == 0x1a
        && source.subtile_column < 2
        && source.subtile_row < 2
        && matches!(source.tile_index, 0x27 | 0x28 | 0x37 | 0x38)
    {
        let west_height = f32::from(source.subtile_column) * 8.0;
        return Some(CellShape::RampEast {
            west_height,
            east_height: west_height + 8.0,
        });
    }

    let ground_tile_index =
        if map_id == "FastShip1F" && matches!(source.metatile_id, 0x05 | 0x0f | 0x13 | 0x19) {
            DECK_TILE
        } else if map_id == "FastShipB1F" && source.metatile_id == 0x05 {
            VOID_TILE
        } else if map_id == "FastShipB1F" && b1f_horizontal_divider_cell(source) {
            CABIN_FLOOR_TILE
        } else {
            return None;
        };
    if source.subtile_row < 2 {
        return None;
    }

    Some(CellShape::FacadeBand {
        plane_subtile_row: 4,
        band_from_top: source.subtile_row - 2,
        band_count: 2,
        ground_tile_index,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("lighthouse"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn corridor_bulkhead_folds_exactly_two_face_rows() {
        for block in [0x05, 0x0f, 0x13, 0x19] {
            assert_eq!(shape("FastShip1F", &source(block, 0, 0, 0x01)), None);
            assert_eq!(shape("FastShip1F", &source(block, 0, 1, 0x11)), None);
            for row in 2..4 {
                assert_eq!(
                    shape("FastShip1F", &source(block, 0, row, 0x10)),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row - 2,
                        band_count: 2,
                        ground_tile_index: DECK_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
        }
    }

    #[test]
    fn b1f_repeated_north_bulkhead_folds_over_outside_hull_void() {
        assert_eq!(
            shape("OlivineLighthouse1F", &source(0x05, 0, 2, 0x02)),
            None
        );
        for row in 0..2 {
            assert_eq!(shape("FastShipB1F", &source(0x05, 0, row, 0x01)), None);
        }
        for row in 2..4 {
            assert_eq!(
                shape("FastShipB1F", &source(0x05, 0, row, 0x10)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: row - 2,
                    band_count: 2,
                    ground_tile_index: VOID_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
        }
        assert_eq!(shape("FastShipB1F", &source(0x0f, 0, 2, 0x10)), None);
    }

    #[test]
    fn b1f_straight_room_divider_folds_only_its_two_face_rows() {
        for row in 0..2 {
            assert_eq!(shape("FastShipB1F", &source(0x25, 0, row, 0x0d)), None);
        }
        for row in 2..4 {
            let tile = if row == 2 { 0x33 } else { 0x11 };
            assert_eq!(
                shape("FastShipB1F", &source(0x25, 0, row, tile)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: row - 2,
                    band_count: 2,
                    ground_tile_index: CABIN_FLOOR_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
        }
        assert!(matches!(
            shape("FastShipB1F", &source(0x20, 0, 2, 0x15)),
            Some(CellShape::RaisedTop { height: 16.0, .. })
        ));
    }

    #[test]
    fn b1f_vertical_dividers_claim_only_the_authored_two_cell_strips() {
        for row in 0..4 {
            assert!(matches!(
                shape("FastShipB1F", &source(0x20, 0, row, 0x15)),
                Some(CellShape::RaisedTop { height: 16.0, .. })
            ));
            assert!(matches!(
                shape("FastShipB1F", &source(0x21, 3, row, 0x16)),
                Some(CellShape::RaisedTop { height: 16.0, .. })
            ));
        }
        for block in [0x22, 0x23] {
            let column = if block == 0x22 { 0 } else { 2 };
            assert!(b1f_vertical_divider_cell(&source(block, column, 0, 0x15)));
        }
        assert!(b1f_vertical_divider_cell(&source(0x22, 0, 2, 0x15)));
        assert!(b1f_vertical_divider_cell(&source(0x22, 0, 3, 0x32)));
        assert!(b1f_vertical_divider_cell(&source(0x23, 3, 2, 0x16)));
        assert!(b1f_vertical_divider_cell(&source(0x23, 3, 3, 0x22)));
        assert_eq!(
            shape("FastShipB1F", &source(0x20, 2, 0, CABIN_FLOOR_TILE)),
            None
        );
    }

    #[test]
    fn b1f_corner_arms_fold_without_consuming_the_vertical_corner_column() {
        for (block, row_two, columns) in [
            (0x22, [0x26, 0x33, 0x33], 1..4),
            (0x23, [0x33, 0x33, 0x25], 0..3),
        ] {
            for (offset, column) in columns.enumerate() {
                for (row, tile) in [(2, row_two[offset]), (3, 0x11)] {
                    assert!(matches!(
                        shape("FastShipB1F", &source(block, column, row, tile)),
                        Some(CellShape::FacadeBand {
                            ground_tile_index: CABIN_FLOOR_TILE,
                            ..
                        })
                    ));
                }
            }
        }
        assert_eq!(shape("FastShipB1F", &source(0x22, 1, 2, 0x33)), None);
        assert!(matches!(
            shape("FastShipB1F", &source(0x22, 0, 3, 0x32)),
            Some(CellShape::RaisedTop { height: 16.0, .. })
        ));
    }

    #[test]
    fn both_rows_of_the_dark_flight_form_one_east_rising_ramp() {
        let expected = [[0x27, 0x28], [0x37, 0x38]];
        for row in 0..2 {
            for column in 0..2 {
                assert_eq!(
                    shape(
                        "FastShip1F",
                        &source(
                            0x1a,
                            column,
                            row,
                            expected[usize::from(row)][usize::from(column)]
                        )
                    ),
                    Some(CellShape::RampEast {
                        west_height: f32::from(column) * 8.0,
                        east_height: f32::from(column + 1) * 8.0,
                    })
                );
            }
        }
        assert_eq!(shape("FastShip1F", &source(0x1a, 2, 0, 0x04)), None);
    }

    #[test]
    fn cabin_stool_requires_one_complete_southeast_drawing() {
        let expected = [[0x07, 0x08], [0x17, 0x18]];
        for row in 0..2 {
            for column in 0..2 {
                assert_eq!(
                    stool_local(
                        "FastShipCabins_SE_SSE_CaptainsCabin",
                        &source(
                            0x07,
                            column + 2,
                            row + 2,
                            expected[usize::from(row)][usize::from(column)]
                        )
                    ),
                    Some((column, row))
                );
            }
        }
        assert_eq!(stool_local("FastShip1F", &source(0x07, 2, 2, 0x07)), None);
    }

    #[test]
    fn cabin_rack_keeps_cap_shelves_and_base_in_one_group() {
        let expected = [[0x13, 0x3e], [0x36, 0x2b], [0x36, 0x2b], [0x3b, 0x3c]];
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(
                    rack_local(
                        "FastShipCabins_NNW_NNE_NE",
                        &source(
                            0x2f,
                            column + 2,
                            row,
                            expected[usize::from(row)][usize::from(column)]
                        )
                    ),
                    Some((column, row))
                );
            }
        }
        assert_eq!(
            rack_local("FastShipCabins_NNW_NNE_NE", &source(0x2f, 1, 0, 0x03)),
            None
        );
    }

    #[test]
    fn both_barrel_blocks_resolve_the_same_complete_card() {
        let expected = [[0x48, 0x49], [0x58, 0x59]];
        for block in [0x2f, 0x35] {
            let column_offset = if block == 0x2f { 0 } else { 2 };
            for row in 0..2 {
                for column in 0..2 {
                    assert_eq!(
                        barrel_local(
                            "FastShipCabins_SE_SSE_CaptainsCabin",
                            &source(
                                block,
                                column + column_offset,
                                row + 2,
                                expected[usize::from(row)][usize::from(column)]
                            )
                        ),
                        Some((column, row))
                    );
                }
            }
        }
        assert_eq!(
            barrel_local("OlivineLighthouse1F", &source(0x35, 2, 2, 0x48)),
            None
        );
    }

    #[test]
    fn both_fast_ship_bunk_variants_raise_only_the_mattress() {
        let expected = [[0x46, 0x47], [0x56, 0x57]];
        for block in [0x36, 0x38] {
            let row_offset = if block == 0x36 { 0 } else { 2 };
            for row in 0..2 {
                for column in 0..2 {
                    assert_eq!(
                        bunk_shape(
                            "FastShipB1F",
                            &source(
                                block,
                                column + 2,
                                row + row_offset,
                                expected[usize::from(row)][usize::from(column)]
                            )
                        ),
                        Some(CellShape::RaisedTop {
                            height: 7.0,
                            solid: SolidKind::Prop,
                        })
                    );
                }
            }
        }
        assert_eq!(bunk_shape("FastShipB1F", &source(0x36, 0, 0, 0x0d)), None);
        assert_eq!(
            bunk_shape("OlivineLighthouse1F", &source(0x36, 2, 0, 0x46)),
            None
        );
    }
}
