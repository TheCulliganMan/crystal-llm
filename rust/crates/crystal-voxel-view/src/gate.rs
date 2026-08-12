//! Reusable route-gate wall geometry from Crystal's exact gate atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const FLOOR_TILE: u16 = 0x01;

/// Gate block `$02` is one complete wall drawing: header, panel and skirting
/// occupy source rows 0-2, while row 3 is ordinary walkable floor.  Fold the
/// three authored courses once at their shared floor seam.  This is keyed to
/// the full source identity and does not infer shape from collision.
pub(crate) fn shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "gate" || source.metatile_id != 0x02 || source.subtile_row >= 3
    {
        return None;
    }

    Some(CellShape::FacadeBand {
        plane_subtile_row: 3,
        band_from_top: source.subtile_row,
        band_count: 3,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(row: u8, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("gate"),
            metatile_id: 0x02,
            subtile_column: 0,
            subtile_row: row,
            tile_index,
        }
    }

    #[test]
    fn block_two_folds_each_native_wall_course_once() {
        for (row, tile) in [(0, 0x5c), (1, 0x10), (2, 0x11)] {
            assert_eq!(
                shape(&source(row, tile)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 3,
                    band_from_top: row,
                    band_count: 3,
                    ground_tile_index: FLOOR_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
        }
    }

    #[test]
    fn floor_and_similar_unscoped_art_stay_flat() {
        assert_eq!(shape(&source(3, FLOOR_TILE)), None);

        let mut other_block = source(0, 0x5c);
        other_block.metatile_id = 0x03;
        assert_eq!(shape(&other_block), None);

        let mut other_tileset = source(0, 0x5c);
        other_tileset.tileset_id = Arc::from("facility");
        assert_eq!(shape(&other_tileset), None);
    }
}
