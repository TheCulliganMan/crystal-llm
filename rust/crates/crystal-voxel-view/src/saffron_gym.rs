//! Exact Saffron Gym wall and planter presentation roles.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x10;

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "SaffronGym"
        || source.tileset_id.as_ref() != "underground"
        || source.metatile_id != 0x09
        || source.subtile_row >= 2
    {
        return None;
    }
    Some(CellShape::FacadeBand {
        plane_subtile_row: 2,
        band_from_top: source.subtile_row,
        band_count: 2,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

pub(crate) fn planter_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "underground"
        || source.metatile_id != 0x36
        || source.subtile_column >= 2
    {
        return None;
    }
    Some((source.subtile_column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("underground"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn straight_wall_is_exactly_two_native_rows() {
        for row in 0..2 {
            assert!(matches!(
                shape("SaffronGym", &source(0x09, 0, row)),
                Some(CellShape::FacadeBand {
                    band_count: 2,
                    solid: SolidKind::FlatCard,
                    ..
                })
            ));
        }
        assert_eq!(shape("SaffronGym", &source(0x09, 0, 2)), None);
        assert_eq!(shape("SilphCo1F", &source(0x09, 0, 0)), None);
    }

    #[test]
    fn planter_keeps_one_complete_two_by_four_drawing() {
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(
                    planter_local(&source(0x36, column, row)),
                    Some((column, row))
                );
            }
        }
        assert_eq!(planter_local(&source(0x36, 2, 0)), None);
    }
}
