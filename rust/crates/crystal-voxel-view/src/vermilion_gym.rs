//! Exact grouped entrance-statue drawings for Vermilion Gym.

use crystal_render_api::VisualTileSource;

pub(crate) const FLOOR_TILE: u16 = 0x01;

pub(crate) fn statue_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != "VermilionGym" || source.tileset_id.as_ref() != "game_corner" {
        return None;
    }
    let local_column = match source.metatile_id {
        0x1d if source.subtile_column >= 2 => source.subtile_column - 2,
        0x1e if source.subtile_column < 2 => source.subtile_column,
        _ => return None,
    };
    let expected = [[0x42, 0x43], [0x52, 0x53], [0x44, 0x45], [0x54, 0x55]]
        [usize::from(source.subtile_row)][usize::from(local_column)];
    (source.tile_index == expected).then_some((local_column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("game_corner"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn split_blocks_resolve_complete_two_by_four_statues() {
        let drawing = [[0x42, 0x43], [0x52, 0x53], [0x44, 0x45], [0x54, 0x55]];
        for (block, offset) in [(0x1d, 2), (0x1e, 0)] {
            for row in 0..4 {
                for column in 0..2 {
                    let cell = source(
                        block,
                        column + offset,
                        row,
                        drawing[row as usize][column as usize],
                    );
                    assert_eq!(statue_local("VermilionGym", &cell), Some((column, row)));
                    assert_eq!(statue_local("GoldenrodGameCorner", &cell), None);
                }
            }
        }
    }
}
