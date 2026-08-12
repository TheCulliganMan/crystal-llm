//! Exact grouped entrance-statue drawings for Fuchsia Gym.
//!
//! The `lab` atlas splits each 16x32 statue between the left or right half
//! of blocks `$1a/$1b`.  It is one face-on prop, like a two-tile-wide tree,
//! rather than four rows of floor or four independently raised cells.

use crystal_render_api::VisualTileSource;

pub(crate) const FLOOR_TILE: u16 = 0x10;

pub(crate) fn statue_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != "FuchsiaGym" || source.tileset_id.as_ref() != "lab" {
        return None;
    }

    let local_column = match source.metatile_id {
        0x1a if source.subtile_column >= 2 => source.subtile_column - 2,
        0x1b if source.subtile_column < 2 => source.subtile_column,
        _ => return None,
    };
    let expected = [[0x4c, 0x4d], [0x5c, 0x5d], [0x4e, 0x4f], [0x5e, 0x5f]]
        [usize::from(source.subtile_row)][usize::from(local_column)];

    (source.tile_index == expected).then_some((local_column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("lab"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn both_split_blocks_resolve_one_two_by_four_statue() {
        let drawing = [[0x4c, 0x4d], [0x5c, 0x5d], [0x4e, 0x4f], [0x5e, 0x5f]];
        for (block, column_offset) in [(0x1a, 2), (0x1b, 0)] {
            for row in 0..4 {
                for column in 0..2 {
                    assert_eq!(
                        statue_local(
                            "FuchsiaGym",
                            &source(
                                block,
                                column + column_offset,
                                row,
                                drawing[row as usize][column as usize]
                            ),
                        ),
                        Some((column, row))
                    );
                }
            }
        }
    }

    #[test]
    fn shared_lab_art_does_not_promote_other_maps_or_floor() {
        let statue = source(0x1a, 2, 0, 0x4c);
        assert_eq!(statue_local("ElmsLab", &statue), None);
        assert_eq!(statue_local("FuchsiaGym", &source(0x1a, 0, 0, 0x10)), None);
    }
}
