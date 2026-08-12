//! Map-scoped round café tables from Crystal's shared Game Corner atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const FLOOR_TILE: u16 = 0x01;
pub(crate) const TABLETOP_BOTTOM: f32 = 14.0;
pub(crate) const TABLETOP_THICKNESS: f32 = 2.0;
pub(crate) const SERVICE_COUNTER_HEIGHT: f32 = 8.0;

pub(crate) fn is_cafe_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "CeladonCafe" | "OlivineCafe" | "SafariZoneMainOffice"
    )
}

/// The two cafés draw the same complete 32x32 octagonal table with different
/// upper decoration. Keep its plan-view art on top and lift only its authored
/// silhouette; shared Game Corner maps must not inherit this table reading.
pub(crate) fn is_table(map_id: &str, source: &VisualTileSource) -> bool {
    if source.tileset_id.as_ref() != "game_corner" {
        return false;
    }
    match map_id {
        "CeladonCafe" => source.metatile_id == 0x2e,
        "OlivineCafe" | "SafariZoneMainOffice" => source.metatile_id == 0x15,
        _ => false,
    }
}

/// Exact source cells forming the cafés' long service counter. The vertical
/// arm is depth visible in the original plan view, not a stack of wall bands;
/// the complete C-shaped footprint rises by one native tile course.
fn is_service_counter(map_id: &str, source: &VisualTileSource) -> bool {
    if !is_cafe_map(map_id) || source.tileset_id.as_ref() != "game_corner" {
        return false;
    }
    match source.metatile_id {
        // Rounded north end and the straight north-south arm.
        0x13 => source.subtile_column < 2 && source.subtile_row >= 1,
        0x16 => source.subtile_column < 2,
        // The turn continues south across the lower half of $17/$18.
        0x17 => source.subtile_column < 2 || source.subtile_row >= 2,
        0x18 => source.subtile_row >= 2,
        _ => false,
    }
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if is_table(map_id, source) {
        return Some(CellShape::FloatingRelief {
            height: TABLETOP_THICKNESS,
            ground_tile_index: FLOOR_TILE,
            base_height: TABLETOP_BOTTOM,
        });
    }
    is_service_counter(map_id, source).then_some(CellShape::RaisedTop {
        height: SERVICE_COUNTER_HEIGHT,
        solid: SolidKind::Prop,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source_at(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("game_corner"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0x1c,
        }
    }

    fn source(metatile_id: u16) -> VisualTileSource {
        source_at(metatile_id, 0, 0)
    }

    #[test]
    fn each_cafe_promotes_only_its_complete_round_table_block() {
        let expected = Some(CellShape::FloatingRelief {
            height: TABLETOP_THICKNESS,
            ground_tile_index: FLOOR_TILE,
            base_height: TABLETOP_BOTTOM,
        });
        assert_eq!(shape("CeladonCafe", &source(0x2e)), expected);
        assert_eq!(shape("OlivineCafe", &source(0x15)), expected);
        assert_eq!(shape("SafariZoneMainOffice", &source(0x15)), expected);
        assert_eq!(shape("CeladonCafe", &source(0x15)), None);
        assert_eq!(shape("OlivineCafe", &source(0x2e)), None);
    }

    #[test]
    fn shared_atlas_maps_do_not_inherit_cafe_tables() {
        assert!(is_cafe_map("CeladonCafe"));
        assert!(is_cafe_map("OlivineCafe"));
        assert!(is_cafe_map("SafariZoneMainOffice"));
        assert!(!is_cafe_map("GoldenrodGameCorner"));
        assert_eq!(shape("GoldenrodGameCorner", &source(0x2e)), None);
        assert_eq!(shape("VermilionGym", &source(0x15)), None);
    }

    #[test]
    fn service_counter_raises_only_its_authored_c_footprint() {
        let counter = Some(CellShape::RaisedTop {
            height: SERVICE_COUNTER_HEIGHT,
            solid: SolidKind::Prop,
        });
        for row in 0..4 {
            for column in 0..4 {
                assert_eq!(
                    shape("CeladonCafe", &source_at(0x16, column, row)),
                    (column < 2).then_some(counter.expect("copy shape"))
                );
                assert_eq!(
                    shape("CeladonCafe", &source_at(0x18, column, row)),
                    (row >= 2).then_some(counter.expect("copy shape"))
                );
            }
        }
        assert_eq!(shape("CeladonCafe", &source_at(0x13, 0, 0)), None);
        assert_eq!(shape("CeladonCafe", &source_at(0x13, 0, 1)), counter);
        assert_eq!(shape("CeladonCafe", &source_at(0x17, 3, 0)), None);
        assert_eq!(shape("CeladonCafe", &source_at(0x17, 3, 2)), counter);
    }

    #[test]
    fn other_shared_atlas_maps_do_not_inherit_the_cafe_counter() {
        let counter_cell = source_at(0x16, 0, 0);
        assert_eq!(shape("GoldenrodGameCorner", &counter_cell), None);
        assert_eq!(shape("VermilionGym", &counter_cell), None);
    }
}
