//! Hall of Fame recording terminal as one complete thin console card.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x0c;

pub(crate) fn console_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    (source.tileset_id.as_ref() == "ice_path"
        && source.metatile_id == 0x3c
        && source.subtile_column < 2
        && source.subtile_row < 3)
        .then_some((source.subtile_column, source.subtile_row))
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "HallOfFame" {
        return None;
    }
    let (_, row) = console_local(source)?;
    Some(CellShape::FacadeBand {
        plane_subtile_row: 3,
        band_from_top: row,
        band_count: 3,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("ice_path"),
            metatile_id: 0x3c,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn console_is_one_two_by_three_zero_depth_drawing() {
        for row in 0..3 {
            for column in 0..2 {
                assert_eq!(console_local(&source(column, row)), Some((column, row)));
                assert!(matches!(
                    shape("HallOfFame", &source(column, row)),
                    Some(CellShape::FacadeBand {
                        solid: SolidKind::FlatCard,
                        band_count: 3,
                        ..
                    })
                ));
            }
        }
        assert_eq!(console_local(&source(2, 0)), None);
    }

    #[test]
    fn shared_ice_path_atlas_is_not_promoted_elsewhere() {
        assert_eq!(shape("IcePath1F", &source(0, 0)), None);
    }
}
