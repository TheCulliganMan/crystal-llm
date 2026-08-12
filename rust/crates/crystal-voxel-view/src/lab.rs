//! Authored presentation roles for Crystal's active laboratory maps.
//!
//! The lab atlas mixes floor, tables, shelves, framed wall panels, and one
//! complete four-row machine inside the same collision vocabulary.  Match
//! exact drawing cells; collision is never used to invent laboratory walls.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const LAB_GROUND_TILE: u16 = 0x10;

pub(crate) fn lab_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "lab" {
        return None;
    }

    match source.metatile_id {
        // The repeated framed panel: its upper two source rows fold once
        // onto their south seam. The lower half remains ordinary lab floor.
        0x05 if source.subtile_row < 2 => Some(CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: LAB_GROUND_TILE,
            solid: SolidKind::Building,
        }),
        // The complete northwest equipment bank is a tall, one-cell-deep
        // drawing. Every native row appears once on the same upright plane;
        // it is not a four-cell-deep floor patch or a generic voxel block.
        0x08 => Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: LAB_GROUND_TILE,
            solid: SolidKind::Building,
        }),
        // Oak's north window/panel course uses the same two-band wall fold.
        0x20 if source.subtile_row < 2 => Some(CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: LAB_GROUND_TILE,
            solid: SolidKind::Building,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("lab"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn framed_panel_folds_only_its_two_authored_wall_rows() {
        for row in 0..2 {
            assert!(matches!(
                lab_shape(&source(0x05, 1, row)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 2,
                    band_from_top,
                    band_count: 2,
                    ..
                }) if band_from_top == row
            ));
        }
        assert_eq!(lab_shape(&source(0x05, 1, 2)), None);
    }

    #[test]
    fn complete_machine_uses_all_four_rows_once() {
        for row in 0..4 {
            assert!(matches!(
                lab_shape(&source(0x08, 2, row)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top,
                    band_count: 4,
                    ..
                }) if band_from_top == row
            ));
        }
    }

    #[test]
    fn lab_profile_does_not_escape_its_tileset_or_claim_floor() {
        let mut unrelated = source(0x08, 0, 0);
        unrelated.tileset_id = Arc::from("facility");
        assert_eq!(lab_shape(&unrelated), None);
        assert_eq!(lab_shape(&source(0x01, 0, 0)), None);
    }
}
