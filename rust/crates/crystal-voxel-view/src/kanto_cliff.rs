//! Authored Kanto overworld cliff-mound profiles.
//!
//! These rules describe Crystal's stable visual source identities only. They
//! never inspect collision or movement permissions. Unknown Kanto artwork
//! remains on the faithful flat baseline.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, JUMP_LEDGE_HEIGHT, LedgeFace, SolidKind};

pub(crate) const KANTO_CLIFF_HEIGHT: f32 = 16.0;

/// The same 16x16 round barrier drawing is packed into several Kanto land
/// metatiles. Return coordinates within one aligned 2x2 drawing so the mesher
/// can stand each rock independently instead of joining a repeated run into a
/// wall or extruding it as a box. Shore/water containers are deliberately
/// excluded because they need their own authored underlay and base datum.
pub(crate) fn round_barrier_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    const LAND_CONTAINERS: [u16; 15] = [
        0x13, 0x1c, 0x40, 0x41, 0x46, 0x4a, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x61, 0x62, 0x63,
    ];
    if source.tileset_id.as_ref() != "kanto" || !LAND_CONTAINERS.contains(&source.metatile_id) {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row % 2;
    const DRAWING: [[u16; 2]; 2] = [[0x2a, 0x2b], [0x3a, 0x3b]];
    (source.tile_index == DRAWING[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

/// The same rock drawing is also packed into Kanto's water-edge blocks. Keep
/// it separate from the land family: these rocks stand at the recessed water
/// datum and the cells they consume must be repainted with live `$14` water,
/// not lawn. Mixed `$67/$6a` corner blocks are safe because the exact 2x2
/// drawing is still required before a group can be claimed.
pub(crate) fn shoreline_round_barrier_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    const SHORE_CONTAINERS: [u16; 7] = [0x14, 0x15, 0x18, 0x19, 0x67, 0x6a, 0x6b];
    if source.tileset_id.as_ref() != "kanto" || !SHORE_CONTAINERS.contains(&source.metatile_id) {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row % 2;
    const DRAWING: [[u16; 2]; 2] = [[0x2a, 0x2b], [0x3a, 0x3b]];
    (source.tile_index == DRAWING[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

/// Block `$29` stores the same round barrier as four flipped copies of tile
/// `$24`, two drawings in its right half. It sits on pale paving rather than
/// grass, so it keeps a separate classifier and authored underlay.
pub(crate) fn round_path_barrier_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "kanto"
        || source.metatile_id != 0x29
        || source.subtile_column < 2
        || source.tile_index != 0x24
    {
        return None;
    }
    Some((source.subtile_column - 2, source.subtile_row % 2))
}

/// Resolves the six-block Diglett's Cave mound family used by Kanto towns and
/// routes. Crystal draws one plateau row over one mixed plateau/front row:
/// `[3e 3f 3b; 24 06 25]`. The lower block row's final two source rows are
/// the actual rock/cave-mouth courses and fold once onto the south edge.
pub(crate) fn kanto_cliff_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "kanto" {
        return None;
    }

    match source.metatile_id {
        // `$2a/$2b` are Crystal's mirrored southwest/southeast cliff-corner
        // transitions. Twelve `$11` cells carry plateau paint and one lower
        // 2x2 corner carries directional boundary ink. Each block is one
        // continuous level: those corner pixels describe the exposed edge,
        // not a separate four-cell obstacle. Raising the complete authored
        // block lets the neighbour mesher close only the exposed L edge and
        // prevents either corner drawing from becoming a vertical fin.
        0x2a | 0x2b => Some(CellShape::RaisedTop {
            height: KANTO_CLIFF_HEIGHT,
            solid: SolidKind::Bank,
        }),
        // Kanto block `$07` is the region's canonical south-facing jump
        // ledge: three rows of the same `$2c` upper surface and one native
        // `$37` front course. It uses the same universal ledge elevation as
        // Johto; the extra top rows are depth, not stacked height levels.
        0x07 if source.subtile_row < 3 && source.tile_index == 0x2c => Some(CellShape::RaisedTop {
            height: JUMP_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        }),
        // `$2f` is the authored half-width termination of `$07`: its west
        // two columns carry the same three-row cap and one-row south face,
        // while its east two columns are ordinary `$2c/$04` ground. Keep
        // the transition half-width instead of extending the ledge through
        // the walkable half of the metatile.
        0x2f if source.subtile_column < 2
            && source.subtile_row < 3
            && source.tile_index == 0x2c =>
        {
            Some(CellShape::RaisedTop {
                height: JUMP_LEDGE_HEIGHT,
                solid: SolidKind::Bank,
            })
        }
        0x1a if source.subtile_row < 3 && source.tile_index == 0x39 => Some(CellShape::RaisedTop {
            height: JUMP_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        }),
        0x07 | 0x1a if source.subtile_row == 3 && source.tile_index == 0x37 => {
            let top_tile_index = if source.metatile_id == 0x07 {
                0x2c
            } else {
                0x39
            };
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index,
                height: JUMP_LEDGE_HEIGHT,
            })
        }
        0x2f if source.subtile_column < 2
            && source.subtile_row == 3
            && matches!(source.tile_index, 0x37 | 0x34) =>
        {
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x2c,
                height: JUMP_LEDGE_HEIGHT,
            })
        }
        // `$28` is a four-row-deep west boundary: two repeated `$27`
        // columns depict the directional rock face and the two `$11`
        // columns east of it are the raised plateau. Depth rows extend the
        // same level north/south; they must never become four height tiers.
        0x28 if source.subtile_column < 2 && source.tile_index == 0x27 => {
            Some(side_band(LedgeFace::West, 0, 1 - source.subtile_column))
        }
        0x28 if source.subtile_column >= 2 && source.tile_index == 0x11 => {
            Some(CellShape::RaisedTop {
                height: KANTO_CLIFF_HEIGHT,
                solid: SolidKind::Bank,
            })
        }
        0x3e if source.subtile_column < 2 => {
            Some(side_band(LedgeFace::West, 0, 1 - source.subtile_column))
        }
        0x3b if source.subtile_column >= 2 => {
            Some(side_band(LedgeFace::East, 4, source.subtile_column - 2))
        }
        0x3e | 0x3f | 0x3b => Some(CellShape::RaisedTop {
            height: KANTO_CLIFF_HEIGHT,
            solid: SolidKind::Bank,
        }),
        0x24 if source.subtile_row < 2 && source.subtile_column < 2 => {
            Some(side_band(LedgeFace::West, 0, 1 - source.subtile_column))
        }
        0x25 if source.subtile_row < 2 && source.subtile_column >= 2 => {
            Some(side_band(LedgeFace::East, 4, source.subtile_column - 2))
        }
        0x24 | 0x06 | 0x57 | 0x25 if source.subtile_row < 2 => Some(CellShape::RaisedTop {
            height: KANTO_CLIFF_HEIGHT,
            solid: SolidKind::Bank,
        }),
        0x24 | 0x06 | 0x57 | 0x25 => Some(CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x11,
            height: KANTO_CLIFF_HEIGHT,
        }),
        _ => None,
    }
}

fn side_band(face: LedgeFace, plane_subtile: u8, band_from_top: u8) -> CellShape {
    CellShape::LedgeBand {
        face,
        plane_subtile,
        band_from_top,
        band_count: 2,
        top_tile_index: 0x11,
        height: KANTO_CLIFF_HEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("kanto"),
            metatile_id,
            subtile_column: 0,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn mound_has_one_plateau_and_two_native_front_courses() {
        assert_eq!(
            kanto_cliff_shape(&source(0x3f, 3)),
            Some(CellShape::RaisedTop {
                height: KANTO_CLIFF_HEIGHT,
                solid: SolidKind::Bank,
            })
        );
        assert_eq!(
            kanto_cliff_shape(&source(0x06, 2)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 2,
                top_tile_index: 0x11,
                height: KANTO_CLIFF_HEIGHT,
            })
        );
        assert_eq!(
            kanto_cliff_shape(&source(0x06, 3)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 1,
                band_count: 2,
                top_tile_index: 0x11,
                height: KANTO_CLIFF_HEIGHT,
            })
        );
    }

    #[test]
    fn unrelated_kanto_art_is_not_invented_as_cliff() {
        assert_eq!(kanto_cliff_shape(&source(0x01, 0)), None);
    }

    #[test]
    fn mirrored_corner_blocks_are_each_one_continuous_level() {
        for (metatile, drawing) in [
            (
                0x2a,
                [
                    [0x11, 0x11, 0x11, 0x11],
                    [0x11, 0x11, 0x11, 0x11],
                    [0x37, 0x13, 0x11, 0x11],
                    [0x13, 0x27, 0x11, 0x11],
                ],
            ),
            (
                0x2b,
                [
                    [0x11, 0x11, 0x11, 0x11],
                    [0x11, 0x11, 0x11, 0x11],
                    [0x11, 0x11, 0x35, 0x37],
                    [0x11, 0x11, 0x24, 0x35],
                ],
            ),
        ] {
            for (row, tiles) in drawing.into_iter().enumerate() {
                for (column, tile_index) in tiles.into_iter().enumerate() {
                    let mut cell = source(metatile, row as u8);
                    cell.subtile_column = column as u8;
                    cell.tile_index = tile_index;
                    assert_eq!(
                        kanto_cliff_shape(&cell),
                        Some(CellShape::RaisedTop {
                            height: KANTO_CLIFF_HEIGHT,
                            solid: SolidKind::Bank,
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn mound_corner_slope_art_folds_onto_its_directional_side() {
        let mut west = source(0x3e, 1);
        west.subtile_column = 0;
        assert_eq!(
            kanto_cliff_shape(&west),
            Some(side_band(LedgeFace::West, 0, 1))
        );

        let mut east = source(0x3b, 1);
        east.subtile_column = 3;
        assert_eq!(
            kanto_cliff_shape(&east),
            Some(side_band(LedgeFace::East, 4, 1))
        );
    }

    #[test]
    fn kanto_jump_ledge_has_one_front_course_at_universal_height() {
        for row in 0..3 {
            let mut top = source(0x07, row);
            top.tile_index = 0x2c;
            assert_eq!(
                kanto_cliff_shape(&top),
                Some(CellShape::RaisedTop {
                    height: JUMP_LEDGE_HEIGHT,
                    solid: SolidKind::Bank,
                })
            );
        }
        let mut front = source(0x07, 3);
        front.tile_index = 0x37;
        assert_eq!(
            kanto_cliff_shape(&front),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x2c,
                height: JUMP_LEDGE_HEIGHT,
            })
        );
    }

    #[test]
    fn alternate_kanto_jump_ledge_uses_its_own_cap_art() {
        for row in 0..3 {
            let mut top = source(0x1a, row);
            top.subtile_column = 1;
            top.tile_index = 0x39;
            assert_eq!(
                kanto_cliff_shape(&top),
                Some(CellShape::RaisedTop {
                    height: JUMP_LEDGE_HEIGHT,
                    solid: SolidKind::Bank,
                })
            );
        }
        let mut front = source(0x1a, 3);
        front.subtile_column = 1;
        front.tile_index = 0x37;
        assert_eq!(
            kanto_cliff_shape(&front),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x39,
                height: JUMP_LEDGE_HEIGHT,
            })
        );
    }

    #[test]
    fn half_width_jump_ledge_preserves_its_flat_east_half() {
        for row in 0..3 {
            for column in 0..2 {
                let mut top = source(0x2f, row);
                top.subtile_column = column;
                top.tile_index = 0x2c;
                assert_eq!(
                    kanto_cliff_shape(&top),
                    Some(CellShape::RaisedTop {
                        height: JUMP_LEDGE_HEIGHT,
                        solid: SolidKind::Bank,
                    })
                );
            }
        }
        for (column, tile_index) in [(0, 0x37), (1, 0x34)] {
            let mut face = source(0x2f, 3);
            face.subtile_column = column;
            face.tile_index = tile_index;
            assert_eq!(
                kanto_cliff_shape(&face),
                Some(CellShape::LedgeBand {
                    face: LedgeFace::South,
                    plane_subtile: 4,
                    band_from_top: 0,
                    band_count: 1,
                    top_tile_index: 0x2c,
                    height: JUMP_LEDGE_HEIGHT,
                })
            );
        }
        for row in 0..4 {
            for column in 2..4 {
                let mut flat = source(0x2f, row);
                flat.subtile_column = column;
                flat.tile_index = if row == 3 { 0x04 } else { 0x2c };
                assert_eq!(kanto_cliff_shape(&flat), None);
            }
        }
    }

    #[test]
    fn repeated_west_boundary_is_one_plateau_level_not_four_steps() {
        for row in 0..4 {
            for column in 0..2 {
                let mut face = source(0x28, row);
                face.subtile_column = column;
                face.tile_index = 0x27;
                assert_eq!(
                    kanto_cliff_shape(&face),
                    Some(side_band(LedgeFace::West, 0, 1 - column))
                );
            }
            for column in 2..4 {
                let mut plateau = source(0x28, row);
                plateau.subtile_column = column;
                plateau.tile_index = 0x11;
                assert_eq!(
                    kanto_cliff_shape(&plateau),
                    Some(CellShape::RaisedTop {
                        height: KANTO_CLIFF_HEIGHT,
                        solid: SolidKind::Bank,
                    })
                );
            }
        }
    }

    #[test]
    fn every_round_barrier_container_resolves_independent_two_by_two_drawings() {
        for (metatile, origins) in [
            (0x4d, &[(2, 0), (2, 2)][..]),
            (0x4e, &[(0, 0), (0, 2)]),
            (0x50, &[(0, 0), (0, 2), (2, 2)]),
        ] {
            for &(origin_column, origin_row) in origins {
                for (row, source_row) in [[0x2a, 0x2b], [0x3a, 0x3b]].into_iter().enumerate() {
                    for (column, tile) in source_row.into_iter().enumerate() {
                        let mut source = source(metatile, (origin_row + row) as u8);
                        source.subtile_column = (origin_column + column) as u8;
                        source.tile_index = tile;
                        assert_eq!(
                            round_barrier_local(&source),
                            Some((column as u8, row as u8))
                        );
                    }
                }
            }
        }
        assert_eq!(round_barrier_local(&source(0x4c, 0)), None);
        let mut shoreline = source(0x6b, 2);
        shoreline.tile_index = 0x2a;
        assert_eq!(round_barrier_local(&shoreline), None);
    }

    #[test]
    fn flipped_tile_barriers_in_block_29_resolve_as_two_complete_drawings() {
        for origin_row in [0, 2] {
            for row in 0..2 {
                for column in 0..2 {
                    let mut source = source(0x29, origin_row + row);
                    source.subtile_column = 2 + column;
                    source.tile_index = 0x24;
                    assert_eq!(round_path_barrier_local(&source), Some((column, row)));
                }
            }
        }
        let mut ground = source(0x29, 0);
        ground.subtile_column = 0;
        ground.tile_index = 0x11;
        assert_eq!(round_path_barrier_local(&ground), None);
    }

    #[test]
    fn shoreline_barriers_are_classified_separately_from_land() {
        for (metatile, origins) in [
            (0x14, &[(0, 0), (0, 2), (2, 2)][..]),
            (0x19, &[(2, 0), (2, 2)]),
            (0x6b, &[(0, 2), (2, 2)]),
        ] {
            for &(origin_column, origin_row) in origins {
                for (row, source_row) in [[0x2a, 0x2b], [0x3a, 0x3b]].into_iter().enumerate() {
                    for (column, tile) in source_row.into_iter().enumerate() {
                        let mut source = source(metatile, (origin_row + row) as u8);
                        source.subtile_column = (origin_column + column) as u8;
                        source.tile_index = tile;
                        assert_eq!(
                            shoreline_round_barrier_local(&source),
                            Some((column as u8, row as u8))
                        );
                        assert_eq!(round_barrier_local(&source), None);
                    }
                }
            }
        }
        let mut water = source(0x14, 0);
        water.subtile_column = 2;
        water.tile_index = 0x14;
        assert_eq!(shoreline_round_barrier_local(&water), None);
    }
}
