//! Exact grouped statue drawings for the Ruins of Alph interior atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::CellShape;

const TILESET: &str = "ruins_of_alph";
pub(crate) const FLOOR_TILE: u16 = 0x02;

/// Blocks `$0e/$0f` are mirrored room-edge drawings: one half is the
/// repeated `$0a/$0b` border strip and the other half is checker floor.  The
/// strip follows the same floor plane in every puzzle and item room; it is
/// not a stack of four wall cells.  Classify the exact authored halves so the
/// coverage audit does not invite collision-derived extrusion.
pub(crate) fn boundary_plane(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != TILESET {
        return None;
    }

    let authored_strip = match source.metatile_id {
        0x0e => source.subtile_column < 2 && matches!(source.tile_index, 0x0a | 0x0b),
        0x0f => source.subtile_column >= 2 && matches!(source.tile_index, 0x0a | 0x0b),
        _ => false,
    };

    authored_strip.then_some(CellShape::PlaneAt { height: 0.0 })
}

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

    #[test]
    fn mirrored_room_edges_remain_on_the_authored_floor_plane() {
        for (metatile, columns) in [(0x0e, 0..2), (0x0f, 2..4)] {
            for row in 0..4 {
                for column in columns.clone() {
                    let mut cell = source(metatile, column, row);
                    cell.tile_index = if column % 2 == 0 { 0x0a } else { 0x0b };
                    assert_eq!(
                        boundary_plane(&cell),
                        Some(CellShape::PlaneAt { height: 0.0 })
                    );
                }
            }
        }

        let mut floor_half = source(0x0e, 2, 0);
        floor_half.tile_index = 0x02;
        assert_eq!(boundary_plane(&floor_half), None);

        let mut reused_art = source(0x0d, 0, 0);
        reused_art.tile_index = 0x0a;
        assert_eq!(boundary_plane(&reused_art), None);
    }
}
