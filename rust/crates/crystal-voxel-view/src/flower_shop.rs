//! Presentation roles unique to Crystal's Goldenrod Flower Shop.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const MAP_ID: &str = "GoldenrodFlowerShop";

/// The shop's four display-block variants arrange the same six top-down
/// flower-box tiles around their edges. Raise only those exact cells by the
/// drawn six-pixel base height; unrelated wall and floor cells in the mixed
/// metatiles stay untouched.
pub(crate) fn display_shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    (map_id == MAP_ID
        && source.tileset_id.as_ref() == "house"
        && matches!(source.metatile_id, 0x2c | 0x2f | 0x30 | 0x32)
        && matches!(source.tile_index, 0x2a | 0x2b | 0x54 | 0x55 | 0x5e | 0x5f))
    .then_some(CellShape::RaisedTop {
        height: 6.0,
        solid: SolidKind::Prop,
    })
}

/// The right half of $2e is the clerk's one 16x16 stool. Its native drawing
/// is a low seat seen from above, so retain it on a five-pixel raised top
/// rather than folding it into a sign or leaving it painted on the floor.
pub(crate) fn stool_shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != MAP_ID
        || source.tileset_id.as_ref() != "house"
        || source.metatile_id != 0x2e
        || source.subtile_column < 2
        || source.subtile_row >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x02, 0x03], [0x12, 0x13]];
    let local_column = source.subtile_column - 2;
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(local_column)])
        .then_some(CellShape::RaisedTop {
            height: 5.0,
            solid: SolidKind::Prop,
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
    fn flower_display_raises_only_exact_shop_tiles() {
        for metatile in [0x2c, 0x2f, 0x30, 0x32] {
            for tile in [0x2a, 0x2b, 0x54, 0x55, 0x5e, 0x5f] {
                assert_eq!(
                    display_shape(MAP_ID, &source("house", metatile, 0, 0, tile)),
                    Some(CellShape::RaisedTop {
                        height: 6.0,
                        solid: SolidKind::Prop,
                    })
                );
            }
        }
        assert_eq!(
            display_shape(MAP_ID, &source("house", 0x32, 0, 0, 0x00)),
            None
        );
        assert_eq!(
            display_shape("BillsFamilysHouse", &source("house", 0x2c, 0, 0, 0x2a)),
            None
        );
    }

    #[test]
    fn shop_stool_is_one_five_pixel_two_by_two_seat() {
        let drawing = [[0x02, 0x03], [0x12, 0x13]];
        for row in 0..2 {
            for column in 0..2 {
                assert_eq!(
                    stool_shape(
                        MAP_ID,
                        &source(
                            "house",
                            0x2e,
                            column + 2,
                            row,
                            drawing[usize::from(row)][usize::from(column)],
                        ),
                    ),
                    Some(CellShape::RaisedTop {
                        height: 5.0,
                        solid: SolidKind::Prop,
                    })
                );
            }
        }
        assert_eq!(
            stool_shape(MAP_ID, &source("house", 0x2e, 1, 0, 0x01)),
            None
        );
        assert_eq!(
            stool_shape(MAP_ID, &source("house", 0x2d, 2, 0, 0x02)),
            None
        );
    }
}
