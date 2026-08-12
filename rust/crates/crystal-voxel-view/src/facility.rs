//! Authored wall equipment from Crystal's shared `facility` atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const FLOOR_TILE: u16 = 0x01;

/// Block `$02` is one north-wall cabinet drawing. All four source rows form
/// one face-on picture; none supplies authored depth. Fold the complete image
/// once at its south seam instead of manufacturing a cap behind it.
pub(crate) fn shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "facility" || source.metatile_id != 0x02 {
        return None;
    }
    let expected = match source.subtile_row {
        0 => [0x40, 0x42, 0x40, 0x42],
        1 | 2 => [0x0a, 0x0b, 0x0a, 0x0b],
        3 => [0x1a, 0x1b, 0x1a, 0x1b],
        _ => unreachable!(),
    };
    if source.tile_index != expected[usize::from(source.subtile_column)] {
        return None;
    }
    Some(CellShape::FacadeBand {
        plane_subtile_row: 4,
        band_from_top: source.subtile_row,
        band_count: 4,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("facility"),
            metatile_id: 0x02,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn block_two_folds_as_one_four_course_equipment_face() {
        let drawing = [
            [0x40, 0x42, 0x40, 0x42],
            [0x0a, 0x0b, 0x0a, 0x0b],
            [0x0a, 0x0b, 0x0a, 0x0b],
            [0x1a, 0x1b, 0x1a, 0x1b],
        ];
        for (row, tiles) in drawing.into_iter().enumerate() {
            for (column, tile) in tiles.into_iter().enumerate() {
                assert_eq!(
                    shape(&source(column as u8, row as u8, tile)),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row as u8,
                        band_count: 4,
                        ground_tile_index: FLOOR_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
        }
    }

    #[test]
    fn source_identity_is_exact() {
        let mut wrong_tile = source(0, 0, 0x41);
        assert_eq!(shape(&wrong_tile), None);
        wrong_tile.tile_index = 0x40;
        wrong_tile.metatile_id = 0x03;
        assert_eq!(shape(&wrong_tile), None);
        wrong_tile.metatile_id = 0x02;
        wrong_tile.tileset_id = Arc::from("lab");
        assert_eq!(shape(&wrong_tile), None);
    }
}
