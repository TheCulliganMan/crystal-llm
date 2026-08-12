//! Visible one-course maze walls for Viridian Gym.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x3d;

pub(crate) fn statue_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != "ViridianGym" || source.tileset_id.as_ref() != "train_station" {
        return None;
    }
    let local_column = match source.metatile_id {
        0x31 if source.subtile_column < 2 => source.subtile_column,
        0x32 if source.subtile_column >= 2 => source.subtile_column - 2,
        _ => return None,
    };
    let expected = [[0x48, 0x49], [0x58, 0x59], [0x4a, 0x4b], [0x5a, 0x5b]]
        [usize::from(source.subtile_row)][usize::from(local_column)];
    (source.tile_index == expected).then_some((local_column, source.subtile_row))
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "ViridianGym"
        || source.tileset_id.as_ref() != "train_station"
        || !(0x36..=0x3f).contains(&source.metatile_id)
        || source.tile_index == FLOOR_TILE
    {
        return None;
    }

    // Each 2x2 source quadrant is one visible 16px maze-wall drawing. Fold
    // its two rows once at that quadrant's south edge. Adjacent quadrants may
    // repeat, but never accumulate into the 48px fins warned about by the
    // reference profile.
    let quadrant_row = source.subtile_row / 2 * 2;
    Some(CellShape::FacadeBand {
        plane_subtile_row: quadrant_row + 2,
        band_from_top: source.subtile_row - quadrant_row,
        band_count: 2,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tile: u16, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("train_station"),
            metatile_id: 0x36,
            subtile_column: 0,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn each_visible_maze_cell_is_exactly_one_native_course() {
        assert_eq!(
            shape("ViridianGym", &source(0x02, 0)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: FLOOR_TILE,
                solid: SolidKind::FlatCard,
            })
        );
        assert_eq!(
            shape("ViridianGym", &source(0x02, 3)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: FLOOR_TILE,
                solid: SolidKind::FlatCard,
            })
        );
    }

    #[test]
    fn floor_and_shared_atlas_maps_are_not_raised() {
        assert_eq!(shape("ViridianGym", &source(FLOOR_TILE, 0)), None);
        assert_eq!(shape("CeladonGym", &source(0x02, 0)), None);
    }

    #[test]
    fn split_statue_blocks_are_two_complete_upright_cards() {
        let drawing = [[0x48, 0x49], [0x58, 0x59], [0x4a, 0x4b], [0x5a, 0x5b]];
        for (block, offset) in [(0x31, 0), (0x32, 2)] {
            for row in 0..4 {
                for column in 0..2 {
                    let mut cell = source(drawing[row as usize][column as usize], row);
                    cell.metatile_id = block;
                    cell.subtile_column = column + offset;
                    assert_eq!(statue_local("ViridianGym", &cell), Some((column, row)));
                    assert_eq!(statue_local("CeladonGym", &cell), None);
                }
            }
        }
    }
}
