//! Exact connected wall vocabulary for Team Rocket Base B1F.

use crystal_render_api::VisualTileSource;

const MAP: &str = "TeamRocketBaseB1F";
const TILESET: &str = "underground";
pub(crate) const FLOOR_TILE: u16 = 0x10;
pub(crate) const WALL_HEIGHT: f32 = 16.0;

pub(crate) fn is_wall_cell(map_id: &str, source: &VisualTileSource) -> bool {
    map_id == MAP
        && source.tileset_id.as_ref() == TILESET
        && matches!(source.tile_index, 0x03..=0x05 | 0x0c | 0x0d | 0x13..=0x15)
}

pub(crate) fn is_upper_face(source: &VisualTileSource) -> bool {
    matches!(source.tile_index, 0x03..=0x05)
}

pub(crate) fn is_lower_face(source: &VisualTileSource) -> bool {
    matches!(source.tile_index, 0x13..=0x15)
}

/// Block `$04` draws one 16x32 warning lamp over the continuous horizontal
/// wall. Its upper half visually closes that course, but the lamp retains its
/// existing renderer; this predicate only suppresses false exposed wall ends.
pub(crate) fn closes_wall_edge(map_id: &str, source: &VisualTileSource) -> bool {
    map_id == MAP
        && source.tileset_id.as_ref() == TILESET
        && source.metatile_id == 0x04
        && source.subtile_column < 2
        && source.subtile_row < 2
        && matches!(source.tile_index, 0x45 | 0x46 | 0x55 | 0x56)
}

/// Blocks `$29/$2a` place the same 16x24 potted plant in opposite halves.
/// Resolve the complete drawing so its three bands stand as one thin prop.
pub(crate) fn plant_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != MAP || source.tileset_id.as_ref() != TILESET || source.subtile_row >= 3 {
        return None;
    }
    let local_column = match source.metatile_id {
        0x29 if source.subtile_column >= 2 => source.subtile_column - 2,
        0x2a if source.subtile_column < 2 => source.subtile_column,
        _ => return None,
    };
    const DRAWING: [[u16; 2]; 3] = [[0x1e, 0x1f], [0x2e, 0x2f], [0x3e, 0x3f]];
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(local_column)])
        .then_some((local_column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(TILESET),
            metatile_id: 0x09,
            subtile_column: 0,
            subtile_row: 0,
            tile_index,
        }
    }

    #[test]
    fn exact_horizontal_and_vertical_wall_tiles_join_one_network() {
        for tile in [0x03, 0x04, 0x05, 0x0c, 0x0d, 0x13, 0x14, 0x15] {
            assert!(is_wall_cell(MAP, &source(tile)));
        }
        assert!(!is_wall_cell(MAP, &source(FLOOR_TILE)));
        assert!(!is_wall_cell(
            "GoldenrodUndergroundWarehouse",
            &source(0x04)
        ));
    }

    #[test]
    fn horizontal_art_keeps_two_distinct_source_bands() {
        for tile in 0x03..=0x05 {
            assert!(is_upper_face(&source(tile)));
            assert!(!is_lower_face(&source(tile)));
        }
        for tile in 0x13..=0x15 {
            assert!(is_lower_face(&source(tile)));
            assert!(!is_upper_face(&source(tile)));
        }
    }

    #[test]
    fn warning_lamp_is_one_complete_card_over_a_continuous_wall() {
        let drawing = [[0x45, 0x46], [0x55, 0x56], [0x09, 0x19], [0x30, 0x31]];
        for row in 0..4 {
            for column in 0..2 {
                let mut cell = source(drawing[row][column]);
                cell.metatile_id = 0x04;
                cell.subtile_column = column as u8;
                cell.subtile_row = row as u8;
                assert_eq!(
                    closes_wall_edge(MAP, &cell),
                    row < 2,
                    "only the lamp's upper half closes the wall edge"
                );
                assert!(!is_wall_cell(MAP, &cell));
            }
        }
    }

    #[test]
    fn split_plant_blocks_resolve_one_two_by_three_drawing() {
        let drawing = [[0x1e, 0x1f], [0x2e, 0x2f], [0x3e, 0x3f]];
        for metatile in [0x29, 0x2a] {
            for row in 0..3 {
                for column in 0..2 {
                    let mut cell = source(drawing[row][column]);
                    cell.metatile_id = metatile;
                    cell.subtile_column = if metatile == 0x29 {
                        column as u8 + 2
                    } else {
                        column as u8
                    };
                    cell.subtile_row = row as u8;
                    assert_eq!(plant_local(MAP, &cell), Some((column as u8, row as u8)));
                }
            }
        }
    }
}
