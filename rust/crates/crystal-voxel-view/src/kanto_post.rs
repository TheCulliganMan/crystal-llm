//! Reusable Kanto bollard/post identities from the authored metatile atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) fn kanto_post_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "kanto" {
        return None;
    }
    let local_row = match source.metatile_id {
        0x1b if source.subtile_column < 2 => source.subtile_row % 2,
        0x5f if source.subtile_column >= 2 => source.subtile_row % 2,
        0x56 if source.subtile_column < 2 && source.subtile_row >= 2 => source.subtile_row - 2,
        0x77 if source.subtile_row >= 2 => source.subtile_row - 2,
        _ => return None,
    };
    Some(CellShape::FacadeBand {
        plane_subtile_row: source.subtile_row - local_row + 2,
        band_from_top: local_row,
        band_count: 2,
        ground_tile_index: 0x23,
        solid: SolidKind::Fence,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("kanto"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn every_pewter_bollard_variant_folds_exactly_two_rows() {
        for (metatile, column, top_row) in [(0x1b, 0, 0), (0x5f, 2, 0), (0x56, 0, 2), (0x77, 0, 2)]
        {
            for row in 0..2 {
                assert_eq!(
                    kanto_post_shape(&source(metatile, column, top_row + row)),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: top_row + 2,
                        band_from_top: row,
                        band_count: 2,
                        ground_tile_index: 0x23,
                        solid: SolidKind::Fence,
                    })
                );
            }
        }
    }

    #[test]
    fn mixed_sign_and_ground_quadrants_are_not_posts() {
        assert!(kanto_post_shape(&source(0x56, 2, 2)).is_none());
        assert!(kanto_post_shape(&source(0x77, 0, 0)).is_none());
        assert!(kanto_post_shape(&source(0x1b, 2, 0)).is_none());
    }
}
