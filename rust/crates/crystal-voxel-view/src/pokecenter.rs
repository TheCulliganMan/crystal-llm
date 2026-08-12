//! Authored wall and equipment cards for Pokémon Center 1F lobbies.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x11;

fn is_lobby(map_id: &str) -> bool {
    map_id.ends_with("Pokecenter1F")
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if !is_lobby(map_id) || source.tileset_id.as_ref() != "pokecenter" {
        return None;
    }

    // $01 is the complete wall-height healing-machine drawing.
    if source.metatile_id == 0x01 {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
    }

    // These north-course blocks carry two wall rows followed by floor or a
    // separately grouped PC. Fold only the wall rows; never claim the object
    // or walkable rows below them.
    if matches!(source.metatile_id, 0x02 | 0x03 | 0x08 | 0x13) && source.subtile_row < 2 {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: FLOOR_TILE,
            solid: SolidKind::FlatCard,
        });
    }

    // The northeast PC is consumed by the grouped cutout-card mesher.
    if source.metatile_id == 0x08 && source.subtile_column >= 2 && source.subtile_row >= 2 {
        return Some(CellShape::Flat);
    }

    // $05/$06/$07 form the uninterrupted west service counter. The first
    // source row is its top surface, the second its south-facing front, and
    // the remaining rows are ordinary lobby floor. This avoids the per-pixel
    // relief spikes produced when the native counter art is extruded.
    if matches!(source.metatile_id, 0x05 | 0x06 | 0x07) {
        if source.subtile_row == 0 {
            return Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            });
        }
        if source.subtile_row == 1 {
            let top_tile_index = match source.metatile_id {
                0x05 => 0x34,
                0x06 => [0x0c, 0x0c, 0x34, 0x34][usize::from(source.subtile_column)],
                0x07 => [0x34, 0x0c, 0x13, 0x35][usize::from(source.subtile_column)],
                _ => unreachable!("counter metatile was checked"),
            };
            return Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 2,
                band_from_top: 0,
                band_count: 1,
                top_tile_index,
                height: 8.0,
            });
        }
    }

    // $2e/$2f each contribute one complete 16x16 lounge seat. They are
    // top-down cushions with a dark native base, so keep the art horizontal
    // on one seat-high platform rather than applying noisy per-pixel relief.
    let lounge_seat =
        (source.metatile_id == 0x2e && source.subtile_column >= 2 && source.subtile_row >= 2)
            || (source.metatile_id == 0x2f && source.subtile_column < 2 && source.subtile_row >= 2);
    if lounge_seat {
        return Some(CellShape::RaisedTop {
            height: 5.0,
            solid: SolidKind::Prop,
        });
    }
    None
}

pub(crate) fn pc_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !is_lobby(map_id)
        || source.tileset_id.as_ref() != "pokecenter"
        || source.metatile_id != 0x08
        || source.subtile_column < 2
        || source.subtile_row < 2
    {
        return None;
    }
    let local_column = source.subtile_column - 2;
    let local_row = source.subtile_row - 2;
    let expected = [[0x30, 0x31], [0x40, 0x41]];
    (source.tile_index == expected[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("pokecenter"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn machine_and_wall_rows_fold_without_claiming_floor() {
        assert!(matches!(
            shape("PewterPokecenter1F", &source(0x01, 0, 3, 0x4c)),
            Some(CellShape::FacadeBand { band_count: 4, .. })
        ));
        assert!(matches!(
            shape("PewterPokecenter1F", &source(0x03, 0, 1, 0x02)),
            Some(CellShape::FacadeBand { band_count: 2, .. })
        ));
        assert_eq!(shape("PewterPokecenter1F", &source(0x03, 0, 2, 0x01)), None);
    }

    #[test]
    fn pc_is_one_exact_two_by_two_card_and_is_map_scoped() {
        for (column, row, tile) in [(2, 2, 0x30), (3, 2, 0x31), (2, 3, 0x40), (3, 3, 0x41)] {
            assert_eq!(
                pc_local("PewterPokecenter1F", &source(0x08, column, row, tile)),
                Some((column - 2, row - 2))
            );
        }
        assert_eq!(pc_local("CeladonHotel1F", &source(0x08, 2, 2, 0x30)), None);
    }

    #[test]
    fn service_counter_has_one_top_and_one_front_row() {
        assert_eq!(
            shape("PewterPokecenter1F", &source(0x06, 1, 0, 0x0c)),
            Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            })
        );
        assert_eq!(
            shape("PewterPokecenter1F", &source(0x06, 1, 1, 0x24)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 2,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x0c,
                height: 8.0,
            })
        );
        assert_eq!(shape("PewterPokecenter1F", &source(0x06, 1, 2, 0x01)), None);
    }

    #[test]
    fn paired_lounge_blocks_make_independent_seat_high_platforms() {
        for (block, columns) in [(0x2e, 2..4), (0x2f, 0..2)] {
            for row in 2..4 {
                for column in columns.clone() {
                    assert_eq!(
                        shape("PewterPokecenter1F", &source(block, column, row, 0x48)),
                        Some(CellShape::RaisedTop {
                            height: 5.0,
                            solid: SolidKind::Prop,
                        })
                    );
                }
            }
        }
        assert_eq!(shape("PewterPokecenter1F", &source(0x2e, 1, 2, 0x11)), None);
    }
}
