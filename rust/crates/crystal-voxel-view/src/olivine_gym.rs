//! Exact grouped boulder and entrance-statue drawings for Olivine Gym.

use crystal_render_api::VisualTileSource;

pub(crate) const GROUND_TILE: u16 = 0x53;

pub(crate) fn statue_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != "OlivineGym" || source.tileset_id.as_ref() != "champions_room" {
        return None;
    }
    let local_column = match source.metatile_id {
        0x1a if source.subtile_column < 2 => source.subtile_column,
        0x1b if source.subtile_column >= 2 => source.subtile_column - 2,
        _ => return None,
    };
    let expected = [[0x2e, 0x2f], [0x3e, 0x3f], [0x4e, 0x4f], [0x5e, 0x5f]]
        [usize::from(source.subtile_row)][usize::from(local_column)];
    (source.tile_index == expected).then_some((local_column, source.subtile_row))
}

pub(crate) fn boulder_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "champions_room"
        || !matches!(
            source.metatile_id,
            0x11 | 0x13 | 0x16 | 0x18 | 0x19 | 0x25 | 0x26 | 0x2a
        )
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row % 2;
    let expected = match (local_column, local_row) {
        (0, 0) => 0x46,
        (1, 0) => 0x47,
        (0, 1) => 0x56,
        (1, 1) => 0x57,
        _ => unreachable!(),
    };
    (source.tile_index == expected).then_some((local_column, local_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("champions_room"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn only_complete_native_boulder_cells_are_grouped() {
        assert_eq!(boulder_local(&source(0x11, 0, 0, 0x46)), Some((0, 0)));
        assert_eq!(boulder_local(&source(0x11, 1, 0, 0x47)), Some((1, 0)));
        assert_eq!(boulder_local(&source(0x11, 0, 1, 0x56)), Some((0, 1)));
        assert_eq!(boulder_local(&source(0x11, 1, 1, 0x57)), Some((1, 1)));
        assert_eq!(boulder_local(&source(0x11, 0, 0, 0x53)), None);
        assert_eq!(boulder_local(&source(0x10, 0, 0, 0x46)), None);
    }

    #[test]
    fn split_entrance_blocks_resolve_complete_two_by_four_statues() {
        let drawing = [[0x2e, 0x2f], [0x3e, 0x3f], [0x4e, 0x4f], [0x5e, 0x5f]];
        for (block, offset) in [(0x1a, 0), (0x1b, 2)] {
            for row in 0..4 {
                for column in 0..2 {
                    let cell = source(
                        block,
                        column + offset,
                        row,
                        drawing[row as usize][column as usize],
                    );
                    assert_eq!(statue_local("OlivineGym", &cell), Some((column, row)));
                    assert_eq!(statue_local("ChampionsRoom", &cell), None);
                }
            }
        }
    }
}
