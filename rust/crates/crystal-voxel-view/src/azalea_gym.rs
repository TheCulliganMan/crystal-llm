//! Map-scoped flat-card groupings for Azalea Gym.
//!
//! The Elite Four Room atlas is shared by unrelated interiors. These roles
//! therefore require both the exact map and the exact source drawing.

use crystal_render_api::VisualTileSource;

pub(crate) const GROUND_TILE: u16 = 0x1f;

pub(crate) fn ground_tile(map_id: &str) -> Option<u16> {
    match map_id {
        "AzaleaGym" => Some(GROUND_TILE),
        "GoldenrodGym" => Some(0x03),
        _ => None,
    }
}

pub(crate) fn supports_display_map(map_id: &str) -> bool {
    matches!(map_id, "AzaleaGym" | "GoldenrodGym")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CardGroup {
    pub local_column: u8,
    pub local_row: u8,
    pub width: usize,
    pub height: usize,
}

pub(crate) fn central_tree(source: &VisualTileSource) -> Option<CardGroup> {
    if source.tileset_id.as_ref() != "elite_four_room"
        || source.metatile_id != 0x21
        || source.subtile_column >= 3
        || source.subtile_row >= 3
    {
        return None;
    }
    Some(CardGroup {
        local_column: source.subtile_column,
        local_row: source.subtile_row,
        width: 3,
        height: 3,
    })
}

pub(crate) fn display_box(source: &VisualTileSource) -> Option<CardGroup> {
    if source.tileset_id.as_ref() != "elite_four_room"
        || !matches!(
            source.metatile_id,
            0x04 | 0x05
                | 0x06
                | 0x07
                | 0x08
                | 0x0a
                | 0x0b
                | 0x0c
                | 0x0d
                | 0x0e
                | 0x0f
                | 0x11
                | 0x13
        )
    {
        return None;
    }

    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row % 2;
    let valid = match local_row {
        0 => matches!(source.tile_index, 0x07 | 0x08 | 0x3e | 0x3f),
        1 => matches!(source.tile_index, 0x17 | 0x18),
        _ => false,
    };
    valid.then_some(CardGroup {
        local_column,
        local_row,
        width: 2,
        height: 2,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("elite_four_room"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn center_tree_is_one_complete_three_by_three_drawing() {
        for row in 0..3 {
            for column in 0..3 {
                assert_eq!(
                    central_tree(&source(0x21, column, row, 0)),
                    Some(CardGroup {
                        local_column: column,
                        local_row: row,
                        width: 3,
                        height: 3,
                    })
                );
            }
        }
        assert_eq!(central_tree(&source(0x21, 3, 0, GROUND_TILE)), None);
    }

    #[test]
    fn display_requires_native_top_and_base_rows() {
        assert_eq!(display_box(&source(0x04, 0, 0, 0x3e)).unwrap().local_row, 0);
        assert_eq!(display_box(&source(0x04, 0, 1, 0x17)).unwrap().local_row, 1);
        assert_eq!(display_box(&source(0x04, 2, 2, 0x03)), None);
        assert_eq!(display_box(&source(0x12, 0, 0, 0x3e)), None);
        assert!(display_box(&source(0x05, 0, 0, 0x07)).is_some());
        assert!(display_box(&source(0x13, 2, 2, 0x07)).is_some());
        assert!(supports_display_map("AzaleaGym"));
        assert!(supports_display_map("GoldenrodGym"));
        assert!(!supports_display_map("VioletGym"));
        assert_eq!(ground_tile("AzaleaGym"), Some(0x1f));
        assert_eq!(ground_tile("GoldenrodGym"), Some(0x03));
    }
}
