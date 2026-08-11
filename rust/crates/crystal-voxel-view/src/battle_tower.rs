//! Presentation profiles for the Battle Tower Outside atlas.

use crystal_render_api::VisualTileSource;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TreeGroup {
    pub(crate) local_column: u8,
    pub(crate) local_row: u8,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) ground_tile_index: u16,
}

/// Resolve every complete tree drawing embedded in Battle Tower Outside's
/// metatiles, including compact and mixed-wall variants.
pub(crate) fn tree_group(source: &VisualTileSource) -> Option<TreeGroup> {
    if source.tileset_id.as_ref() != "battle_tower_outside" {
        return None;
    }
    let (local_column, local_row, height) = match source.metatile_id {
        0x05 => (source.subtile_column % 2, source.subtile_row, 4),
        0x0d if source.subtile_column >= 2 => (source.subtile_column - 2, source.subtile_row, 4),
        0x1c if source.subtile_row < 2 => (source.subtile_column % 2, source.subtile_row, 2),
        0x1d if source.subtile_row >= 2 => (source.subtile_column % 2, source.subtile_row - 2, 2),
        0x1f => (source.subtile_column % 2, source.subtile_row % 2, 2),
        0x2d | 0x2e | 0x30 if source.subtile_column < 2 => {
            (source.subtile_column, source.subtile_row, 4)
        }
        0x32 if source.subtile_column >= 2 => {
            (source.subtile_column - 2, source.subtile_row % 2, 2)
        }
        0x33 if source.subtile_column >= 2 => (source.subtile_column - 2, source.subtile_row, 4),
        _ => return None,
    };
    Some(TreeGroup {
        local_column,
        local_row,
        width: 2,
        height,
        ground_tile_index: 0x06,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("battle_tower_outside"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn every_full_and_compact_tree_variant_has_an_exact_group() {
        for (metatile, column) in [
            (0x05, 0),
            (0x0d, 2),
            (0x2d, 0),
            (0x2e, 0),
            (0x30, 0),
            (0x33, 2),
        ] {
            assert_eq!(tree_group(&source(metatile, column, 0)).unwrap().height, 4);
        }
        for (metatile, column, row) in [(0x1c, 0, 0), (0x1d, 0, 2), (0x1f, 0, 0), (0x32, 2, 0)] {
            assert_eq!(
                tree_group(&source(metatile, column, row)).unwrap().height,
                2
            );
        }
    }

    #[test]
    fn non_tree_quadrants_stay_unclaimed() {
        assert!(tree_group(&source(0x0d, 0, 0)).is_none());
        assert!(tree_group(&source(0x1c, 0, 2)).is_none());
        assert!(tree_group(&source(0x2d, 2, 0)).is_none());
        assert!(tree_group(&source(0x33, 0, 0)).is_none());
    }
}
