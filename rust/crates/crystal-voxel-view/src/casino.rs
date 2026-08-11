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
            solid: SolidKind::Prop,
        });
    }

    // The casino floor alternates $05 chair / $07 machine / $06 chair.
    // $07 is the body of a north/south machine bank and $0b its dark south
    // end. The art remains top-facing on a continuous 16px cabinet volume;
    // folding each course south incorrectly puts machines on the back wall.
    if matches!(source.metatile_id, 0x07 | 0x0b) {
        return Some(CellShape::RaisedTop {
            height: 16.0,
            solid: SolidKind::Prop,
        });
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
    // They are consumed by the grouped casino-stool mesher, which can crop
    // and unproject the complete 16x16 drawing. Per-cell geometry cannot
    // preserve the seat/front split without leaving a square footprint.
    let chair_half = (source.metatile_id == 0x05 && source.subtile_column >= 2)
        || (source.metatile_id == 0x06 && source.subtile_column < 2);
    if chair_half {
        return Some(CellShape::Flat);
    }

    // $2a's left half is the repeated 2x4 west-aisle plant (crown, stem and
    // pot); its right half is ordinary wall/floor. Preserve only that complete
    // grouped drawing as a thin upright prop.
    if source.metatile_id == 0x2a && source.subtile_column < 2 {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
    }

    // Celadon's $12 is two independent 2x4 potted plants packed side by
    // side. Both halves use the same crown/stem/pot drawing and stand at the
    // north wall. Leaving these cells planar projects green floor fragments
    // into the black interior void behind the wall.
    if source.metatile_id == 0x12 {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
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
    fn machine_bank_is_one_continuous_cabinet_volume() {
        for row in 0..4 {
            assert_eq!(
                casino_shape(&source(0x07, 2, row, 0x90 + u16::from(row))),
                Some(CellShape::RaisedTop {
                    height: 16.0,
                    solid: SolidKind::Prop,
                })
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
    fn west_aisle_plants_claim_only_the_complete_left_half() {
        for row in 0..4 {
            assert_eq!(
                casino_shape(&source(0x2a, 0, row, 0x19)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: row,
                    band_count: 4,
                    ground_tile_index: FLOOR_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
        }
        assert_eq!(casino_shape(&source(0x2a, 2, 0, 0x01)), None);
    }

    #[test]
    fn celadon_double_plant_groups_both_two_column_halves() {
        for column in 0..4 {
            for row in 0..4 {
                assert_eq!(
                    casino_shape(&source(0x12, column, row, 0x19)),
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
                        solid: SolidKind::Prop,
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
}
