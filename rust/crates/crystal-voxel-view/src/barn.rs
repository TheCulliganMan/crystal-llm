//! Authored stall partitions for Route 39 Barn.
//!
//! Block `$26` paints a two-row rail across the south half of an otherwise
//! plain floor metatile. It is a face-on barrier, not a floor marking and not
//! a box, so the optional renderer folds the two source rows once onto a
//! zero-depth plane.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const FLOOR_TILE: u16 = 0x01;

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "Route39Barn"
        || source.tileset_id.as_ref() != "traditional_house"
        || source.metatile_id != 0x26
        || source.subtile_row < 2
        || !matches!(source.tile_index, 0x40 | 0x41)
    {
        return None;
    }
    Some(CellShape::FacadeBand {
        plane_subtile_row: 4,
        band_from_top: source.subtile_row - 2,
        band_count: 2,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("traditional_house"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index,
        }
    }

    #[test]
    fn barn_partition_folds_both_native_rows_once() {
        for column in 0..4 {
            assert_eq!(
                shape("Route39Barn", &source(0x26, column, 2, 0x40)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: 0,
                    band_count: 2,
                    ground_tile_index: FLOOR_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
            assert_eq!(
                shape("Route39Barn", &source(0x26, column, 3, 0x41)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: 1,
                    band_count: 2,
                    ground_tile_index: FLOOR_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
        }
    }

    #[test]
    fn shared_traditional_house_art_is_not_changed_elsewhere() {
        assert_eq!(shape("Route39Farmhouse", &source(0x26, 0, 2, 0x40)), None);
        assert_eq!(shape("Route39Barn", &source(0x20, 0, 2, 0x01)), None);
        assert_eq!(shape("Route39Barn", &source(0x26, 0, 0, 0x01)), None);
    }
}
