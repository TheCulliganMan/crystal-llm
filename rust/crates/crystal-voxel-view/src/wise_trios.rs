//! Authored grouped divider drawings in Ecruteak's Wise Trio room.

use crystal_render_api::VisualTileSource;

pub(crate) const FLOOR_TILE: u16 = 0x50;

/// Return the source-local position and extent of one complete divider card.
///
/// Block `$37` contains two distinct drawings. Keeping them as separate groups
/// preserves the authored opening instead of inventing one solid wall between
/// the north course and its southeast return.
pub(crate) fn divider_local(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<(u8, u8, usize, usize)> {
    if map_id != "WiseTriosRoom" || source.tileset_id.as_ref() != "traditional_house" {
        return None;
    }

    match source.metatile_id {
        0x28 if source.subtile_row < 2 => expected_course_tile(source).then_some((
            source.subtile_column,
            source.subtile_row,
            4,
            2,
        )),
        0x37 if source.subtile_row < 2 => expected_course_tile(source).then_some((
            source.subtile_column,
            source.subtile_row,
            4,
            2,
        )),
        0x37 | 0x38 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
            expected_course_tile(source).then_some((
                source.subtile_column - 2,
                source.subtile_row - 2,
                2,
                2,
            ))
        }
        _ => None,
    }
}

fn expected_course_tile(source: &VisualTileSource) -> bool {
    source.tile_index
        == if source.subtile_row % 2 == 0 {
            0x40
        } else {
            0x41
        }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("traditional_house"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn block_37_keeps_two_complete_divider_drawings_separate() {
        assert_eq!(
            divider_local("WiseTriosRoom", &source(0x37, 3, 1, 0x41)),
            Some((3, 1, 4, 2))
        );
        assert_eq!(
            divider_local("WiseTriosRoom", &source(0x37, 3, 3, 0x41)),
            Some((1, 1, 2, 2))
        );
        assert_eq!(
            divider_local("WiseTriosRoom", &source(0x37, 0, 2, 0x50)),
            None
        );
    }

    #[test]
    fn divider_identity_is_map_and_art_scoped() {
        assert_eq!(
            divider_local("DanceTheater", &source(0x28, 0, 0, 0x40)),
            None
        );
        assert_eq!(
            divider_local("WiseTriosRoom", &source(0x28, 0, 0, 0x41)),
            None
        );
    }
}
