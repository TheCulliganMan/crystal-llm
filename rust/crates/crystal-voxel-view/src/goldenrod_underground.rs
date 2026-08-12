//! Authored presentation roles for Goldenrod's main underground corridor.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const MAP_ID: &str = "GoldenrodUnderground";
const FLOOR_TILE: u16 = 0x01;

/// Goldenrod's end blocks extend the reusable `$02` wall with a fixture in
/// one half. The complete `$02` wall itself lives in `gate.rs`.
pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    let wall_cell = match source.metatile_id {
        // $0e keeps the wall in its right half and puts a separate fixture
        // drawing in its left half.
        0x0e => source.subtile_column >= 2 && source.subtile_row < 3,
        // $13 mirrors that composition.
        0x13 => source.subtile_column < 2 && source.subtile_row < 3,
        _ => false,
    };
    if map_id == MAP_ID && source.tileset_id.as_ref() == "gate" && wall_cell {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 3,
            band_from_top: source.subtile_row,
            band_count: 3,
            ground_tile_index: FLOOR_TILE,
            // This is an independent wall card, not a partial building
            // template. `Building` is completeness-gated by the mesher.
            solid: SolidKind::FlatCard,
        });
    }
    None
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
    fn wall_end_variants_claim_only_their_authored_half() {
        let mut right_wall = source(1, 0x10);
        right_wall.metatile_id = 0x0e;
        right_wall.subtile_column = 2;
        assert!(shape(MAP_ID, &right_wall).is_some());
        right_wall.subtile_column = 1;
        assert_eq!(shape(MAP_ID, &right_wall), None);

        let mut left_wall = source(2, 0x11);
        left_wall.metatile_id = 0x13;
        left_wall.subtile_column = 1;
        assert!(shape(MAP_ID, &left_wall).is_some());
        left_wall.subtile_column = 2;
        assert_eq!(shape(MAP_ID, &left_wall), None);
    }
}
