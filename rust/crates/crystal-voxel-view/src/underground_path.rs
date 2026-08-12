//! Authored geometry for Crystal's north-south Underground Path.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SOURCE_TILE_HEIGHT, SolidKind};

const CORRIDOR_FLOOR_TILE: u16 = 0x10;

/// The right half of block $02 is the two-tile-wide stair at both surface
/// warps. The reference treatment reads this artwork as one continuous flight
/// rising east, not two discrete steps or a vertical facade.
pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "UndergroundPath" || source.tileset_id.as_ref() != "underground" {
        return None;
    }
    // $08/$09/$0a are the left corner, straight span and right corner of
    // the north enclosure. Their first two rows are one 16px face; the lower
    // two rows continue the corridor and must remain top-facing.
    if matches!(source.metatile_id, 0x08 | 0x09 | 0x0a) && source.subtile_row < 2 {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: CORRIDOR_FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
    }
    if source.metatile_id != 0x02 || source.subtile_column < 2 || source.subtile_row >= 2 {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x2a, 0x2b], [0x3a, 0x3b]];
    let local_column = source.subtile_column - 2;
    if source.tile_index != DRAWING[usize::from(source.subtile_row)][usize::from(local_column)] {
        return None;
    }
    let west_height = f32::from(local_column) * SOURCE_TILE_HEIGHT;
    Some(CellShape::RampEast {
        west_height,
        east_height: west_height + SOURCE_TILE_HEIGHT,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tileset: &str, metatile: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn block_two_is_one_continuous_east_rising_flight() {
        let drawing = [[0x2a, 0x2b], [0x3a, 0x3b]];
        for row in 0..2 {
            assert_eq!(
                shape(
                    "UndergroundPath",
                    &source("underground", 0x02, 2, row, drawing[usize::from(row)][0]),
                ),
                Some(CellShape::RampEast {
                    west_height: 0.0,
                    east_height: 8.0,
                })
            );
            assert_eq!(
                shape(
                    "UndergroundPath",
                    &source("underground", 0x02, 3, row, drawing[usize::from(row)][1]),
                ),
                Some(CellShape::RampEast {
                    west_height: 8.0,
                    east_height: 16.0,
                })
            );
        }
    }

    #[test]
    fn stair_is_map_tileset_art_and_half_scoped() {
        let stair = source("underground", 0x02, 2, 0, 0x2a);
        assert!(shape("UndergroundPath", &stair).is_some());
        assert_eq!(shape("GoldenrodDeptStoreB1F", &stair), None);
        assert_eq!(
            shape("UndergroundPath", &source("gate", 0x02, 2, 0, 0x2a),),
            None
        );
        assert_eq!(
            shape("UndergroundPath", &source("underground", 0x02, 1, 0, 0x2a),),
            None
        );
    }

    #[test]
    fn north_enclosure_folds_exactly_two_rows() {
        for metatile in [0x08, 0x09, 0x0a] {
            for row in 0..2 {
                assert_eq!(
                    shape(
                        "UndergroundPath",
                        &source("underground", metatile, 1, row, 0x04),
                    ),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 2,
                        band_from_top: row,
                        band_count: 2,
                        ground_tile_index: CORRIDOR_FLOOR_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
            assert_eq!(
                shape(
                    "UndergroundPath",
                    &source("underground", metatile, 1, 2, CORRIDOR_FLOOR_TILE),
                ),
                None
            );
        }
    }
}
