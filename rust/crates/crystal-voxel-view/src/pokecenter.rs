//! Authored wall and equipment cards for Pokémon Center 1F lobbies.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x11;

fn is_lobby(map_id: &str) -> bool {
    map_id.ends_with("Pokecenter1F")
}

fn is_link_floor(map_id: &str) -> bool {
    map_id == "Pokecenter2F" || map_id.ends_with("Pokecenter2FBeta")
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "pokecenter" {
        return None;
    }

    // Grouped Center PCs are emitted atomically by the mesher. Keep their
    // source cells on the room plane so the generic fixture profile cannot
    // turn the screen and desk into four unrelated relief columns.
    if pc_local(map_id, source).is_some() {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }
    if healing_console_local(map_id, source).is_some() {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

    // Grouped link-floor seats are emitted atomically by the mesher. Keep
    // their source cells on the room plane here so adjacent seats cannot
    // merge into one generic relief slab.
    if link_floor_seat_local(map_id, source).is_some() {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

    // The Cable Club's blue north/south dividers are continuous room-height
    // partitions. Their six atlas cells are reused through several blocks;
    // on the link floor they must form one-cell-wide wall volumes instead of
    // lying on the floor as pixel relief.
    if is_link_floor(map_id) && matches!(source.tile_index, 0x16 | 0x17 | 0x36 | 0x37 | 0x46 | 0x47)
    {
        return Some(CellShape::RaisedTop {
            height: 16.0,
            solid: SolidKind::Building,
        });
    }

    // The northwest Cable Club counter uses the same one-top/one-front
    // construction as the lobby service counter.
    if is_link_floor(map_id) && source.metatile_id == 0x05 {
        if source.subtile_row == 0 && source.tile_index == 0x34 {
            return Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            });
        }
        if source.subtile_row == 1 && source.tile_index == 0x24 {
            return Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 2,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x34,
                height: 8.0,
            });
        }
    }

    // `$38` is the link-floor's compact west counter end: one row of
    // top-view surface over one south-facing front row. The east half and
    // lower rows are ordinary floor and must remain unclaimed.
    if is_link_floor(map_id) && source.metatile_id == 0x38 && source.subtile_column < 2 {
        if source.subtile_row == 0 && source.tile_index == 0x34 {
            return Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            });
        }
        if source.subtile_row == 1 && source.tile_index == 0x24 {
            return Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 2,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x34,
                height: 8.0,
            });
        }
    }

    if !is_lobby(map_id) {
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

    // `$03` finishes the north wall beside the healing machine with one
    // narrow counter return. `$0f` is its top-down surface and `$25` the
    // single native front course immediately south of it. The neighboring
    // `$03` source pixel belongs to the machine/wall drawing and stays on the
    // lobby plane; widening this rule would manufacture a second counter.
    if source.metatile_id == 0x03 && source.subtile_column == 3 {
        if source.subtile_row == 2 && source.tile_index == 0x0f {
            return Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            });
        }
        if source.subtile_row == 3 && source.tile_index == 0x25 {
            return Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x0f,
                height: 8.0,
            });
        }
    }
    if source.metatile_id == 0x03
        && source.subtile_column == 2
        && source.subtile_row == 3
        && source.tile_index == 0x03
    {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

    // The northeast PC is consumed by the grouped cutout-card mesher.
    if source.metatile_id == 0x08 && source.subtile_column >= 2 && source.subtile_row >= 2 {
        return Some(CellShape::Flat);
    }

    // $12 is the authored southwest stair/landing drawing beside the 2F
    // warp. It describes a horizontal surface in the original projection;
    // it is not a stack of wall-height steps. Keep the complete block on the
    // lobby plane so the optional renderer preserves its topology instead of
    // inventing staircase geometry from the dark edge pixels.
    if source.metatile_id == 0x12 {
        return Some(CellShape::PlaneAt { height: 0.0 });
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
    if source.tileset_id.as_ref() != "pokecenter" {
        return None;
    }
    let (local_column, local_row) = if is_lobby(map_id)
        && source.metatile_id == 0x08
        && source.subtile_column >= 2
        && source.subtile_row >= 2
    {
        (source.subtile_column - 2, source.subtile_row - 2)
    } else if is_link_floor(map_id)
        && source.metatile_id == 0x32
        && source.subtile_column < 2
        && source.subtile_row < 2
    {
        (source.subtile_column, source.subtile_row)
    } else {
        return None;
    };
    let expected = [[0x30, 0x31], [0x40, 0x41]];
    (source.tile_index == expected[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

/// Block `$02` contains the 16x16 console/flank drawing immediately below
/// the Center's north wall. It is one face-on piece of equipment, not four
/// independent relief cells and not a wall-height collision box.
pub(crate) fn healing_console_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !is_lobby(map_id)
        || source.tileset_id.as_ref() != "pokecenter"
        || source.metatile_id != 0x02
        || source.subtile_column >= 2
        || source.subtile_row < 2
    {
        return None;
    }
    let column = source.subtile_column;
    let row = source.subtile_row - 2;
    let expected = [[0x0a, 0x0b], [0x1a, 0x1b]];
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// `$29` contains one complete 16x16 link-floor seat; `$2d` contains two.
/// Return coordinates local to each independent drawing so adjacent seats
/// remain separate objects rather than one long platform.
pub(crate) fn link_floor_seat_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !is_link_floor(map_id)
        || source.tileset_id.as_ref() != "pokecenter"
        || !matches!(source.metatile_id, 0x29 | 0x2d)
        || source.subtile_row >= 2
        || (source.metatile_id == 0x29 && source.subtile_column >= 2)
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row;
    let expected = [[0x48, 0x49], [0x58, 0x59]];
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
    fn healing_console_is_one_exact_two_by_two_lobby_card() {
        for (column, row, tile) in [(0, 2, 0x0a), (1, 2, 0x0b), (0, 3, 0x1a), (1, 3, 0x1b)] {
            assert_eq!(
                healing_console_local("PewterPokecenter1F", &source(0x02, column, row, tile)),
                Some((column, row - 2))
            );
            assert_eq!(
                shape("PewterPokecenter1F", &source(0x02, column, row, tile)),
                Some(CellShape::PlaneAt { height: 0.0 })
            );
        }
        assert_eq!(
            healing_console_local("Pokecenter2F", &source(0x02, 0, 2, 0x0a)),
            None
        );
        assert_eq!(
            healing_console_local("PewterPokecenter1F", &source(0x02, 2, 2, 0x01)),
            None
        );
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
    fn northeast_counter_return_uses_only_its_exact_top_and_front_pair() {
        assert_eq!(
            shape("PewterPokecenter1F", &source(0x03, 3, 2, 0x0f)),
            Some(CellShape::RaisedTop {
                height: 8.0,
                solid: SolidKind::Prop,
            })
        );
        assert_eq!(
            shape("PewterPokecenter1F", &source(0x03, 3, 3, 0x25)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x0f,
                height: 8.0,
            })
        );
        assert_eq!(
            shape("PewterPokecenter1F", &source(0x03, 2, 3, 0x03)),
            Some(CellShape::PlaneAt { height: 0.0 })
        );
        assert_eq!(shape("Pokecenter2F", &source(0x03, 3, 2, 0x0f)), None);
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

    #[test]
    fn southwest_stair_drawing_stays_on_the_authored_lobby_plane() {
        let drawing = [
            [0x11, 0x11, 0x11, 0x11],
            [0x11, 0x11, 0x11, 0x11],
            [0x44, 0x45, 0x11, 0x11],
            [0x54, 0x55, 0x11, 0x11],
        ];
        for (row, tiles) in drawing.into_iter().enumerate() {
            for (column, tile) in tiles.into_iter().enumerate() {
                assert_eq!(
                    shape(
                        "PewterPokecenter1F",
                        &source(0x12, column as u8, row as u8, tile),
                    ),
                    Some(CellShape::PlaneAt { height: 0.0 })
                );
            }
        }
        assert_eq!(shape("CeladonHotel1F", &source(0x12, 0, 2, 0x44)), None);
    }

    #[test]
    fn link_floor_seats_are_complete_seat_height_platforms() {
        for (block, columns) in [(0x29, 0..2), (0x2d, 0..4)] {
            for row in 0..2 {
                for column in columns.clone() {
                    let tile =
                        [[0x48, 0x49], [0x58, 0x59]][usize::from(row)][usize::from(column % 2)];
                    assert_eq!(
                        link_floor_seat_local(
                            "PewterPokecenter2FBeta",
                            &source(block, column, row, tile),
                        ),
                        Some((column % 2, row))
                    );
                    assert_eq!(
                        shape("Pokecenter2F", &source(block, column, row, tile)),
                        Some(CellShape::PlaneAt { height: 0.0 })
                    );
                }
            }
        }
        assert_eq!(
            link_floor_seat_local("Pokecenter2F", &source(0x29, 2, 0, 0x11)),
            None
        );
        assert_eq!(shape("PewterPokecenter1F", &source(0x29, 0, 0, 0x48)), None);
    }

    #[test]
    fn link_floor_counter_has_one_top_and_one_front_row() {
        for column in 0..2 {
            assert_eq!(
                shape("Pokecenter2F", &source(0x38, column, 0, 0x34)),
                Some(CellShape::RaisedTop {
                    height: 8.0,
                    solid: SolidKind::Prop,
                })
            );
            assert_eq!(
                shape("Pokecenter2F", &source(0x38, column, 1, 0x24)),
                Some(CellShape::LedgeBand {
                    face: LedgeFace::South,
                    plane_subtile: 2,
                    band_from_top: 0,
                    band_count: 1,
                    top_tile_index: 0x34,
                    height: 8.0,
                })
            );
        }
        assert_eq!(shape("Pokecenter2F", &source(0x38, 2, 0, 0x11)), None);
        assert_eq!(shape("PewterPokecenter1F", &source(0x38, 0, 0, 0x34)), None);
    }
}
