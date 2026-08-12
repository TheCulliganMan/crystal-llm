//! Authored presentation geometry for Ecruteak's Dance Theater stage.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const STAGE_HEIGHT: f32 = 8.0;
const FLOOR_TILE: u16 = 0x50;

/// The map's first two metatile rows are the continuous performance stage.
/// Its third row supplies one native-height fascia band, followed by ordinary
/// audience-floor artwork. This is deliberately map-scoped because tile `$50`
/// is palette-reused for normal tatami throughout the same tileset.
pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "DanceTheater" || source.tileset_id.as_ref() != "traditional_house" {
        return None;
    }
    match source.metatile_id {
        0x2c | 0x2d => Some(CellShape::RaisedTop {
            height: STAGE_HEIGHT,
            solid: SolidKind::Prop,
        }),
        0x2e | 0x2f | 0x30 if source.subtile_row == 0 => Some(CellShape::RaisedTop {
            height: STAGE_HEIGHT,
            solid: SolidKind::Prop,
        }),
        0x2e | 0x2f | 0x30 if source.subtile_row == 1 => Some(CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 1,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: FLOOR_TILE,
            height: STAGE_HEIGHT,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("traditional_house"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: FLOOR_TILE,
        }
    }

    #[test]
    fn stage_has_two_top_courses_and_one_front_band() {
        assert!(matches!(
            shape("DanceTheater", &source(0x2d, 0, 0)),
            Some(CellShape::RaisedTop { height: 8.0, .. })
        ));
        assert!(matches!(
            shape("DanceTheater", &source(0x2c, 3, 3)),
            Some(CellShape::RaisedTop { height: 8.0, .. })
        ));
        assert!(matches!(
            shape("DanceTheater", &source(0x30, 2, 1)),
            Some(CellShape::LedgeBand { band_count: 1, .. })
        ));
        assert_eq!(shape("DanceTheater", &source(0x30, 2, 2)), None);
        assert_eq!(shape("WiseTriosRoom", &source(0x2c, 0, 0)), None);
    }
}
