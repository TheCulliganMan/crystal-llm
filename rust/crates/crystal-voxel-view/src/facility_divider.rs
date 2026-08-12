//! Closed divider-wall profiles for Team Rocket's facility interiors.
//!
//! Facility metatile `$36` contains two unrelated drawings: its upper two
//! source rows are the brown divider, while its lower rows are furniture and
//! floor. Keep that distinction explicit so the wall never consumes or
//! raises the blue chairs beside it.

use crystal_render_api::VisualTileSource;

#[cfg(test)]
const METATILE: u16 = 0x36;
pub(crate) const FLOOR_TILE: u16 = 0x01;
pub(crate) const WALL_HEIGHT: f32 = 16.0;
/// Preserve the authored one-subtile footprint exactly.
pub(crate) const WALL_DEPTH: f32 = 8.0;

pub(crate) fn supports_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "PowerPlant" | "TeamRocketBaseB2F" | "TeamRocketBaseB3F" | "TrainerHouseB1F"
    )
}

pub(crate) fn is_horizontal_top(map_id: &str, source: &VisualTileSource) -> bool {
    supports_map(map_id)
        && source.tileset_id.as_ref() == "facility"
        && matches!(source.tile_index, 0x40..=0x42)
}

pub(crate) fn is_horizontal_face(map_id: &str, source: &VisualTileSource) -> bool {
    supports_map(map_id)
        && source.tileset_id.as_ref() == "facility"
        && matches!(source.tile_index, 0x4c..=0x4e)
}

pub(crate) fn horizontal_pair(top: &VisualTileSource, face: &VisualTileSource) -> bool {
    matches!(
        (top.tile_index, face.tile_index),
        (0x40, 0x4c) | (0x41, 0x4d) | (0x42, 0x4e)
    )
}

pub(crate) fn is_vertical_left(map_id: &str, source: &VisualTileSource) -> bool {
    supports_map(map_id) && source.tileset_id.as_ref() == "facility" && source.tile_index == 0x50
}

pub(crate) fn is_vertical_right(map_id: &str, source: &VisualTileSource) -> bool {
    supports_map(map_id) && source.tileset_id.as_ref() == "facility" && source.tile_index == 0x52
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("facility"),
            metatile_id: METATILE,
            subtile_column: 0,
            subtile_row: row,
            tile_index: if row == 0 { 0x41 } else { 0x4d },
        }
    }

    #[test]
    fn only_the_two_divider_rows_are_claimed() {
        let mut top = source(0);
        top.tile_index = 0x41;
        let mut face = source(1);
        face.tile_index = 0x4d;
        assert!(is_horizontal_top("TeamRocketBaseB2F", &top));
        assert!(is_horizontal_face("TeamRocketBaseB2F", &face));
        assert!(horizontal_pair(&top, &face));
        assert!(is_horizontal_top("PowerPlant", &top));
    }

    #[test]
    fn divider_is_one_gameplay_tile_high_and_has_real_depth() {
        assert_eq!(WALL_HEIGHT, 2.0 * 8.0);
        assert_eq!(WALL_DEPTH, 8.0);
        assert!(WALL_DEPTH > 1.0);
    }

    #[test]
    fn vertical_runs_claim_only_authored_maze_wall_halves() {
        let mut vertical = source(0);
        vertical.metatile_id = 0x22;
        vertical.tile_index = 0x50;
        assert!(is_vertical_left("TeamRocketBaseB2F", &vertical));
        vertical.tile_index = 0x52;
        assert!(is_vertical_right("TeamRocketBaseB2F", &vertical));
        assert!(is_vertical_right("PowerPlant", &vertical));
    }

    #[test]
    fn power_plant_reuses_the_exact_facility_wall_vocabulary() {
        let mut top = source(0);
        top.metatile_id = 0x34;
        top.tile_index = 0x41;
        let mut face = source(1);
        face.metatile_id = 0x34;
        face.tile_index = 0x4d;

        assert!(is_horizontal_top("PowerPlant", &top));
        assert!(is_horizontal_face("PowerPlant", &face));
        assert!(horizontal_pair(&top, &face));
        assert!(!is_horizontal_top("GoldenrodGameCorner", &top));
    }
}
