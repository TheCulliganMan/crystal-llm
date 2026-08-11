//! Exact flat-card groups for Violet Gym's statues and plaques.

use crystal_render_api::VisualTileSource;

pub(crate) const GROUND_TILE: u16 = 0x01;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CardGroup {
    pub local_column: u8,
    pub local_row: u8,
    pub width: usize,
    pub height: usize,
}

pub(crate) fn card_group(map_id: &str, source: &VisualTileSource) -> Option<CardGroup> {
    if source.tileset_id.as_ref() != "elite_four_room" {
        return None;
    }
    match source.metatile_id {
        0x1d if source.subtile_column < 2 => Some(CardGroup {
            local_column: source.subtile_column,
            local_row: source.subtile_row,
            width: 2,
            height: 4,
        }),
        0x1e if source.subtile_column >= 2 => Some(CardGroup {
            local_column: source.subtile_column - 2,
            local_row: source.subtile_row,
            width: 2,
            height: 4,
        }),
        0x33 if map_id == "VioletGym" && source.subtile_row < 2 => Some(CardGroup {
            local_column: source.subtile_column % 2,
            local_row: source.subtile_row,
            width: 2,
            height: 2,
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
            tileset_id: Arc::from("elite_four_room"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn podiums_keep_all_four_native_rows() {
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(
                    card_group("VioletGym", &source(0x1d, column, row))
                        .unwrap()
                        .height,
                    4
                );
                assert_eq!(
                    card_group("MahoganyGym", &source(0x1e, column + 2, row))
                        .unwrap()
                        .height,
                    4
                );
            }
        }
    }

    #[test]
    fn plaque_block_contains_two_independent_complete_cards() {
        assert_eq!(
            card_group("VioletGym", &source(0x33, 0, 0))
                .unwrap()
                .local_column,
            0
        );
        assert_eq!(
            card_group("VioletGym", &source(0x33, 2, 0))
                .unwrap()
                .local_column,
            0
        );
        assert_eq!(card_group("VioletGym", &source(0x33, 0, 2)), None);
        assert_eq!(card_group("MahoganyGym", &source(0x33, 0, 0)), None);
    }
}
