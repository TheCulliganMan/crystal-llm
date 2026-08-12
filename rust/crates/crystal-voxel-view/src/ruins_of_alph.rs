//! Exact grouped statue drawings for the Ruins of Alph interior atlas.

use crystal_render_api::VisualTileSource;

const TILESET: &str = "ruins_of_alph";
pub(crate) const FLOOR_TILE: u16 = 0x02;

/// The statue is one 16x32 face-on drawing. Blocks `$1b/$1c` contain a whole
/// statue beside floor; `$1d..$20` split the same drawing across two adjacent
/// metatile rows. Resolve by the exact four source bands so both layouts group
/// into one card and no cell is independently extruded.
pub(crate) fn statue_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != TILESET || !matches!(source.metatile_id, 0x1b..=0x20) {
        return None;
    }

    match source.tile_index {
        0x0e => Some((0, 0)),
        0x0f => Some((1, 0)),
        0x1e => Some((0, 1)),
        0x1f => Some((1, 1)),
        0x2e => Some((0, 2)),
        0x2f => Some((1, 2)),
        0x3e => Some((0, 3)),
        0x3f => Some((1, 3)),
        _ => return None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(TILESET),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0x0e,
        }
    }

    #[test]
    fn complete_statue_has_two_columns_and_four_exact_bands() {
        for (tile, local) in [
            (0x0e, (0, 0)),
            (0x0f, (1, 0)),
            (0x1e, (0, 1)),
            (0x1f, (1, 1)),
            (0x2e, (0, 2)),
            (0x2f, (1, 2)),
            (0x3e, (0, 3)),
            (0x3f, (1, 3)),
        ] {
            let mut source = source(0x1b, local.0, local.1);
            source.tile_index = tile;
            assert_eq!(statue_local(&source), Some(local));
        }
    }

    #[test]
    fn only_authored_statue_blocks_claim_the_drawing() {
        let mut split_top = source(0x1d, 0, 2);
        split_top.tile_index = 0x0e;
        assert_eq!(statue_local(&split_top), Some((0, 0)));

        let mut split_bottom = source(0x1f, 0, 0);
        split_bottom.tile_index = 0x2e;
        assert_eq!(statue_local(&split_bottom), Some((0, 2)));

        let mut wrong_block = source(0x0d, 0, 0);
        wrong_block.tile_index = 0x0e;
        assert_eq!(statue_local(&wrong_block), None);
    }
}
