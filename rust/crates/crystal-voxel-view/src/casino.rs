//! Authored geometry for the Game Corner interior atlas.
//!
//! Casino furniture is not generic collision geometry. Long north/south
//! drawings are ranks of upright cabinets, not carpets or deep boxes.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, GROUND_HEIGHT, LedgeFace, SolidKind};

// $01 is present in the repeated checker-floor block and remains a true flat
// sample. $02 also appears in north-wall block $02, so using it as replacement
// ground makes the safety resolver correctly reject every wall facade.
const FLOOR_TILE: u16 = 0x01;

pub(crate) fn is_game_corner_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "GoldenrodGameCorner" | "CeladonGameCorner" | "CeladonGameCornerPrizeRoom"
    )
}

/// The Prize Room's northern course is one front-facing drawing. Keep its
/// picture, vendor windows, and green counter bands together on a single
/// opaque plane; splitting the lower rows into a box lets the counter top
/// occlude the windows under the pitched camera.
pub(crate) fn casino_shape_on_map(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id == "CeladonGameCornerPrizeRoom"
        && source.tileset_id.as_ref() == "game_corner"
        && matches!(source.metatile_id, 0x10 | 0x11)
    {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::Building,
        });
    }
    casino_shape(source)
}

/// Block $08 packs two independent 16x16 front-facing terminals. Return the
/// local coordinate within either terminal so the grouped card mesher never
/// fuses both drawings into one wide object.
pub(crate) fn terminal_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "game_corner" || source.metatile_id != 0x08 {
        return None;
    }
    match source.tile_index {
        0x80 | 0x82 => Some((0, 0)),
        0x81 | 0x83 => Some((1, 0)),
        0x90 | 0x92 => Some((0, 1)),
        0x91 | 0x93 => Some((1, 1)),
        _ => None,
    }
}

/// Each $07/$0b bank contains four independent 16x16 slot cabinets in a 2x2
/// arrangement. Resolve the local coordinate within one cabinet so each
/// source drawing becomes its own tall flat card instead of either lying on
/// the floor or joining into a repeated 16x32 billboard.
pub(crate) fn slot_machine_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "game_corner" || !matches!(source.metatile_id, 0x07 | 0x0b) {
        return None;
    }
    Some((source.subtile_column % 2, source.subtile_row % 2))
}

/// Block $12 contains two identical 16x32 potted-plant drawings; $2a contains
/// the same drawing in its left half. Resolve each complete plant so both maps
/// use the same whole-object mask instead of folding $2a as a solid rectangle.
pub(crate) fn plant_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "game_corner" || !matches!(source.metatile_id, 0x12 | 0x2a) {
        return None;
    }
    let local_column = match source.metatile_id {
        0x12 => source.subtile_column % 2,
        0x2a if source.subtile_column < 2 => source.subtile_column,
        _ => return None,
    };
    let expected = [[0x19, 0x2c], [0x29, 0x3c], [0x39, 0x0f], [0x24, 0x25]];
    (expected[usize::from(source.subtile_row)][usize::from(local_column)] == source.tile_index)
        .then_some((local_column, source.subtile_row))
}

/// Returns a casino shape only where the Crystal source art has one
/// unambiguous orientation. Shared checkerboard and wall tiles stay generic.
pub(crate) fn casino_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "game_corner" {
        return None;
    }

    // Goldenrod's north course is a single four-row wall drawing with the
    // payout machine, display, pillars and doors painted into it. Keeping it
    // top-facing exposes the black interior void as a trench. Fold the exact
    // wall blocks onto their common south seam; no floor block is included.
    if matches!(
        source.metatile_id,
        0x02 | 0x04 | 0x0d | 0x0e | 0x13 | 0x1c | 0x28 | 0x2b | 0x2f | 0x30
    ) {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::Building,
        });
    }

    // The casino floor alternates $05 chair / $07 machine / $06 chair.
    // $07 and $0b are long perspective machine-rank drawings. Keep their
    // exact faithful map-plane presentation. Generated tops/sides make cans,
    // while lifting arbitrary subgroups destroys the continuous source art.
    if matches!(source.metatile_id, 0x07 | 0x0b) {
        return Some(CellShape::Flat);
    }

    // $09 is the straight prize/service counter; $14/$27/$1b are its corner
    // and display variants. All share two floor rows, one top course, then one
    // front-panel course. Preserve each variant's live top artwork while
    // keeping every arm at one continuous 8px height.
    let counter_block = matches!(source.metatile_id, 0x09 | 0x14 | 0x1b | 0x27);
    if counter_block && source.subtile_row == 2 {
        return Some(CellShape::RaisedTop {
            height: 8.0,
            solid: SolidKind::Prop,
        });
    }
    if counter_block && source.subtile_row == 3 {
        let top_tile_index = match source.metatile_id {
            0x09 => 0x0e,
            0x14 => [0x14, 0x84, 0x0e, 0x0e][usize::from(source.subtile_column)],
            0x1b => [0x0e, 0x0e, 0x94, 0x86][usize::from(source.subtile_column)],
            0x27 => [0x40, 0x41, 0x0e, 0x0e][usize::from(source.subtile_column)],
            _ => unreachable!("counter block was checked"),
        };
        return Some(CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: 0,
            band_count: 1,
            top_tile_index,
            height: 8.0,
        });
    }

    // $05 and $06 each contain two independent 2x2 stools beside the bank.
    // They are consumed by the grouped casino-stool mesher as one masked,
    // upright 16x16 card. Per-cell geometry leaves a square footprint, while
    // adding a seat top/front/side turns the small drawing into a can-like
    // voxel object that the source art never depicts.
    let chair_half = (source.metatile_id == 0x05 && source.subtile_column >= 2)
        || (source.metatile_id == 0x06 && source.subtile_column < 2);
    if chair_half {
        return Some(CellShape::Flat);
    }

    // $2a's left half is the repeated 2x4 west-aisle plant. It is consumed by
    // the same grouped, background-masked zero-depth card path as $12 below;
    // the right half remains ordinary wall/floor.
    if source.metatile_id == 0x2a && source.subtile_column < 2 {
        return Some(CellShape::Flat);
    }

    // Celadon's $12 is consumed as two complete grouped plant cards below.
    // Keeping the cells flat here prevents the generic facade path from
    // slicing each drawing into four separate panels.
    if source.metatile_id == 0x12 {
        return Some(CellShape::Flat);
    }

    // Service desks and display counters are waist-high. Their top-down work
    // surface gets one 8px lift, never a wall-height extrusion.
    if matches!(
        source.tile_index,
        0x09
            | 0x15
            | 0x19
            | 0x24..=0x27
            | 0x30
            | 0x31
            | 0x34
            | 0x35
            | 0x36
            | 0x38
            | 0x39
            | 0x46..=0x49
            | 0x55..=0x57
    ) {
        return Some(CellShape::Relief {
            height: 8.0,
            ground_tile_index: FLOOR_TILE,
            base_height: GROUND_HEIGHT,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("game_corner"),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn machine_bank_stays_faithful_flat_art() {
        for row in 0..4 {
            assert_eq!(
                casino_shape(&source(0x07, 2, row, 0x90 + u16::from(row))),
                Some(CellShape::Flat)
            );
        }
    }

    #[test]
    fn grouped_stools_stay_flat_in_the_per_cell_profile() {
        for row in 0..4 {
            assert_eq!(
                casino_shape(&source(0x05, 2, row, 0x0a)),
                Some(CellShape::Flat)
            );
        }
        assert_eq!(casino_shape(&source(0x05, 0, 0, 0x01)), None);
    }

    #[test]
    fn straight_counter_has_one_top_and_one_folded_front_course() {
        assert_eq!(
            casino_shape(&source(0x09, 1, 2, 0x0e)),
            Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            })
        );
        assert_eq!(
            casino_shape(&source(0x09, 1, 3, 0x1e)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x0e,
                height: 8.0,
            })
        );
        assert_eq!(casino_shape(&source(0x09, 1, 1, 0x12)), None);
    }

    #[test]
    fn counter_variants_keep_their_own_top_art_at_the_same_height() {
        for (metatile, column, top) in [(0x14, 1, 0x84), (0x1b, 2, 0x94), (0x27, 0, 0x40)] {
            assert_eq!(
                casino_shape(&source(metatile, column, 3, 0x1e)),
                Some(CellShape::LedgeBand {
                    face: LedgeFace::South,
                    plane_subtile: 4,
                    band_from_top: 0,
                    band_count: 1,
                    top_tile_index: top,
                    height: 8.0,
                })
            );
        }
    }

    #[test]
    fn west_aisle_plant_stays_flat_until_grouped_and_masks_only_the_left_half() {
        for row in 0..4 {
            assert_eq!(
                casino_shape(&source(0x2a, 0, row, 0x19)),
                Some(CellShape::Flat)
            );
        }
        assert_eq!(casino_shape(&source(0x2a, 2, 0, 0x01)), None);
    }

    #[test]
    fn celadon_double_plant_stays_flat_until_grouped() {
        for column in 0..4 {
            for row in 0..4 {
                assert_eq!(
                    casino_shape(&source(0x12, column, row, 0x19)),
                    Some(CellShape::Flat)
                );
            }
        }
    }

    #[test]
    fn block_12_resolves_two_independent_two_by_four_plants() {
        let rows = [[0x19, 0x2c], [0x29, 0x3c], [0x39, 0x0f], [0x24, 0x25]];
        for half in [0, 2] {
            for row in 0..4 {
                for column in 0..2 {
                    assert_eq!(
                        plant_local(&source(
                            0x12,
                            half + column,
                            row,
                            rows[usize::from(row)][usize::from(column)]
                        )),
                        Some((column, row))
                    );
                }
            }
        }
        assert_eq!(plant_local(&source(0x12, 0, 0, 0x01)), None);
    }

    #[test]
    fn block_2a_reuses_the_same_complete_masked_plant_drawing() {
        let expected = [[0x19, 0x2c], [0x29, 0x3c], [0x39, 0x0f], [0x24, 0x25]];
        for (row, tiles) in expected.into_iter().enumerate() {
            for (column, tile) in tiles.into_iter().enumerate() {
                let source = source(0x2a, column as u8, row as u8, tile);
                assert_eq!(plant_local(&source), Some((column as u8, row as u8)));
                assert_eq!(casino_shape(&source), Some(CellShape::Flat));
            }
        }
        assert_eq!(plant_local(&source(0x2a, 2, 0, 0x01)), None);
        assert_eq!(casino_shape(&source(0x2a, 2, 0, 0x01)), None);
    }

    #[test]
    fn north_wall_blocks_fold_to_one_common_seam() {
        for metatile in [0x02, 0x04, 0x0d, 0x0e, 0x13, 0x1c, 0x28, 0x2b, 0x2f, 0x30] {
            for row in 0..4 {
                assert_eq!(
                    casino_shape(&source(metatile, 0, row, 0x10)),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row,
                        band_count: 4,
                        ground_tile_index: FLOOR_TILE,
                        solid: SolidKind::Building,
                    })
                );
            }
        }
    }

    #[test]
    fn counters_are_not_full_walls() {
        assert_eq!(
            casino_shape(&source(0x10, 0, 0, 0x38)),
            Some(CellShape::Relief {
                height: 8.0,
                ground_tile_index: FLOOR_TILE,
                base_height: GROUND_HEIGHT,
            })
        );
    }

    #[test]
    fn shared_floor_art_is_not_promoted() {
        assert_eq!(casino_shape(&source(0x01, 0, 0, 0x37)), None);
    }

    #[test]
    fn prize_room_wall_stays_one_complete_opaque_drawing() {
        for metatile in [0x10, 0x11] {
            for row in 0..4 {
                assert_eq!(
                    casino_shape_on_map(
                        "CeladonGameCornerPrizeRoom",
                        &source(metatile, 0, row, 0x4d)
                    ),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row,
                        band_count: 4,
                        ground_tile_index: FLOOR_TILE,
                        solid: SolidKind::Building,
                    })
                );
            }
        }
        assert_eq!(
            casino_shape_on_map("GoldenrodGameCorner", &source(0x10, 0, 0, 0x4d)),
            None
        );
    }

    #[test]
    fn block_08_resolves_two_separate_terminal_cards() {
        for (tile, local) in [
            (0x80, (0, 0)),
            (0x81, (1, 0)),
            (0x90, (0, 1)),
            (0x91, (1, 1)),
            (0x82, (0, 0)),
            (0x83, (1, 0)),
            (0x92, (0, 1)),
            (0x93, (1, 1)),
        ] {
            assert_eq!(terminal_local(&source(0x08, 0, 0, tile)), Some(local));
        }
        assert_eq!(terminal_local(&source(0x08, 0, 0, 0x01)), None);
        assert_eq!(terminal_local(&source(0x07, 0, 0, 0x80)), None);
    }

    #[test]
    fn machine_banks_resolve_four_independent_two_by_two_cabinets() {
        for metatile in [0x07, 0x0b] {
            for row in 0..4 {
                for column in 0..4 {
                    assert_eq!(
                        slot_machine_local(&source(metatile, column, row, 0x90)),
                        Some((column % 2, row % 2))
                    );
                }
            }
        }
        assert_eq!(slot_machine_local(&source(0x08, 0, 0, 0x80)), None);
    }
}
