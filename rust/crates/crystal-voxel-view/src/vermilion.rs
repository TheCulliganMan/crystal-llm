//! Map-scoped wall art for Vermilion Gym.
//!
//! The Gym shares the Game Corner atlas. Its trash cans deliberately remain
//! faithful floor art; this module only folds the north target/gate drawings.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x01;

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "VermilionGym" || source.tileset_id.as_ref() != "game_corner" {
        return None;
    }
    // $1d and $1e mirror the two-column trash-can bank across the room. Each
    // half contains two cans stacked in map rows. Keep every source cell on
    // the faithful floor plane: the drawing supplies no side or top art from
    // which the mod could legitimately construct volume.
    let trash_can = (source.metatile_id == 0x1d && source.subtile_column >= 2)
        || (source.metatile_id == 0x1e && source.subtile_column < 2);
    if trash_can {
        return Some(CellShape::Flat);
    }
    // The two $20/$21 pairs are the complete 32px target/gate drawings at
    // the north side of the puzzle. Fold each native row once onto the wall;
    // this is a zero-depth image plane, not an extruded cabinet.
    if matches!(source.metatile_id, 0x20 | 0x21) {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("game_corner"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn gym_cans_are_not_promoted_into_mod_geometry() {
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(
                    shape("VermilionGym", &source(0x1d, column + 2, row)),
                    Some(CellShape::Flat)
                );
                assert_eq!(
                    shape("VermilionGym", &source(0x1e, column, row)),
                    Some(CellShape::Flat)
                );
            }
        }
    }

    #[test]
    fn shared_game_corner_atlas_does_not_make_cans_outside_the_gym() {
        assert_eq!(shape("GoldenrodGameCorner", &source(0x1d, 2, 0)), None);
    }

    #[test]
    fn target_wall_uses_each_native_row_once_on_a_flat_plane() {
        for row in 0..4 {
            assert_eq!(
                shape("VermilionGym", &source(0x20, 0, row)),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: row,
                    band_count: 4,
                    ground_tile_index: FLOOR_TILE,
                    solid: SolidKind::FlatCard,
                })
            );
        }
    }
}
