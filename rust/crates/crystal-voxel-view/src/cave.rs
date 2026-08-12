//! Exact visual roles for Crystal's cave and dark-cave tile drawings.
//!
//! These identities come from the shipped metatile catalogs. They are not
//! inferred from runtime collision: unknown cave cells remain on the lower
//! datum and never become walls merely because movement is blocked.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const CAVE_SHELF_HEIGHT: f32 = 6.0;
pub(crate) const CAVE_ROCK_HEIGHT: f32 = 16.0;
/// One shared visual height for complete outdoor/cave trapezoidal mounds.
/// This is deliberately separate from the cave shelf datum: loose rocks do
/// not raise terrain, and every tileset reuses the same mound proportions.
pub(crate) const TRAPEZOID_MOUND_HEIGHT: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagonalCorner {
    SouthEast,
    SouthWest,
}

/// Crystal's small cave rock is one complete 2x2 drawing. The upper source
/// row is its cap and the lower source row is its front face; neither half is
/// meaningful as an independent voxel.
pub(crate) fn small_rock_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if !matches!(source.tileset_id.as_ref(), "cave" | "dark_cave") {
        return None;
    }
    let column = source.subtile_column % 2;
    let row = source.subtile_row % 2;
    let expected = [[0x0c, 0x0d], [0x1c, 0x1d]];
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// Crystal's four complete cave ladder/warp drawings. They are drawn from
/// directly above and occupy the southeast collision quadrant of their
/// metatile. Unlike the Gen 1 reference profile's staircase cells, these
/// pictures do not provide an authored rise direction, so the optional view
/// preserves them as explicit floor-parallel planes instead of inventing a
/// staircase or leaving them in the unclassified queue.
fn flat_ladder_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if !matches!(source.tileset_id.as_ref(), "cave" | "dark_cave")
        || source.subtile_column < 2
        || source.subtile_row < 2
    {
        return None;
    }
    let expected = match source.metatile_id {
        0x07 => [[0x2a, 0x2b], [0x3a, 0x3b]],
        0x0b => [[0x22, 0x23], [0x32, 0x33]],
        0x1b => [[0x28, 0x29], [0x38, 0x39]],
        0x1f => [[0x20, 0x21], [0x30, 0x31]],
        _ => return None,
    };
    let column = source.subtile_column - 2;
    let row = source.subtile_row - 2;
    (source.tile_index == expected[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// Returns the local cell and orientation of Crystal's complete diagonal
/// cave-corner drawings. `$10` owns the southeast collision quadrant and
/// `$11` mirrors it in the southwest quadrant. The exact 2x2 drawings are
/// grouped by the mesher; no individual `$0a/$0b` tile is promoted alone.
pub(crate) fn diagonal_corner_local(source: &VisualTileSource) -> Option<(DiagonalCorner, u8, u8)> {
    if !matches!(source.tileset_id.as_ref(), "cave" | "dark_cave") {
        return None;
    }
    let (corner, origin_column, expected) = match source.metatile_id {
        // `$35` reuses the complete `$10` southeast corner in its lower
        // half, underneath a separate loose boulder drawing. Keep those two
        // objects independent while sharing the exact corner primitive.
        0x10 | 0x35 => (DiagonalCorner::SouthEast, 2, [[0x0a, 0x26], [0x17, 0x0a]]),
        0x11 => (DiagonalCorner::SouthWest, 0, [[0x26, 0x0b], [0x0b, 0x15]]),
        _ => return None,
    };
    if source.subtile_column < origin_column || source.subtile_row < 2 {
        return None;
    }
    let column = source.subtile_column - origin_column;
    let row = source.subtile_row - 2;
    (column < 2 && row < 2 && source.tile_index == expected[usize::from(row)][usize::from(column)])
        .then_some((corner, column, row))
}

const SOUTH_SHELF_METATILES: [u16; 4] = [0x0c, 0x0d, 0x0e, 0x36];
const CEILING_MASS_METATILES: [u16; 3] = [0x2d, 0x2e, 0x2f];
const HOP_DOWN_SHELF_METATILES: [u16; 3] = [0x38, 0x39, 0x3a];

/// The southeast quadrant of `$13` and `$37` is one complete barred-rock
/// shelf: `$0e/$0f` over `$1e/$1f`.  Its black surround reaches every edge,
/// so it is a single solid 16px face rather than four props or floor art.
fn barred_rock_shelf_band(source: &VisualTileSource) -> Option<CellShape> {
    if !matches!(source.metatile_id, 0x13 | 0x37)
        || source.subtile_column < 2
        || source.subtile_row < 2
    {
        return None;
    }

    let local_column = usize::from(source.subtile_column - 2);
    let local_row = usize::from(source.subtile_row - 2);
    let drawing = [[0x0e, 0x0f], [0x1e, 0x1f]];
    (source.tile_index == drawing[local_row][local_column]).then_some(CellShape::LedgeBand {
        face: LedgeFace::South,
        plane_subtile: 4,
        band_from_top: source.subtile_row - 2,
        band_count: 2,
        // `$13` sits on ordinary cave floor and `$37` is embedded in the
        // pale ceiling course.  The top sample must follow that authored
        // context rather than using the face artwork as a horizontal cap.
        top_tile_index: if source.metatile_id == 0x13 {
            0x16
        } else {
            0x26
        },
        height: CAVE_ROCK_HEIGHT,
    })
}

pub(crate) fn cave_shape(source: &VisualTileSource) -> Option<CellShape> {
    if !matches!(source.tileset_id.as_ref(), "cave" | "dark_cave") {
        return None;
    }

    if let Some(shape) = barred_rock_shelf_band(source) {
        return Some(shape);
    }

    if flat_ladder_local(source).is_some() {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

    // `$23` mixes a real shallow south lip in its west half with only the
    // lower `$1a/$1b` row of another drawing in its northeast quadrant.
    // Crystal never supplies the matching `$0a/$0b` row inside this block,
    // and the quadrant is not a complete drawable staircase. Preserve those
    // two source cells on the cave datum; the west `$24` cells continue into
    // the authored shelf rule below. This is intentionally not the Gen 1
    // reference mod's complete four-tile stair profile.
    if source.metatile_id == 0x23
        && source.subtile_row == 0
        && source.subtile_column >= 2
        && matches!(source.tile_index, 0x1a | 0x1b)
    {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

    // `$34/$37` complete the same pale ceiling-rock mass as `$2d-$2f`,
    // but each mixes that mass with a different authored object.  `$34`
    // keeps two loose boulders in its north half and contributes only its
    // two southern `$26` rows.  `$37` contributes every `$26` cell around
    // the barred shelf handled above.  Classify the exact source cells so
    // neither mixed block becomes one invented sixteen-cell slab.
    if (source.metatile_id == 0x13
        && source.subtile_row >= 2
        && source.subtile_column < 2
        && source.tile_index == 0x26)
        || (source.metatile_id == 0x34 && source.subtile_row >= 2 && source.tile_index == 0x26)
        || (source.metatile_id == 0x37 && source.tile_index == 0x26)
    {
        return Some(CellShape::RaisedTop {
            height: CAVE_ROCK_HEIGHT,
            solid: SolidKind::Bank,
        });
    }

    // `$3d/$3e` are the paired cave-water corner blocks. Every collision
    // quadrant is water, and their `$0b/$15/$0a/$17` pixels are only the
    // drawn boundary around that water. The generic shelf-edge vocabulary
    // reuses `$15/$17`; applying it here creates a stray six-pixel ledge
    // beside a waterfall. Keep the complete authored block at the cave-water
    // datum instead. Cave pools are level with the lower floor, not recessed.
    if matches!(source.metatile_id, 0x3d | 0x3e) {
        return Some(CellShape::Flat);
    }

    // `$3b/$3c` are the complete west/east lateral hop edges. Their four
    // repeated `$15`/`$17` source cells depict one continuous shallow side
    // course, not four elevation bands. Fold each cell onto the same outside
    // plane at the normal six-pixel shelf datum.
    if source.metatile_id == 0x3b && source.subtile_column == 0 {
        return Some(CellShape::LedgeBand {
            face: LedgeFace::West,
            plane_subtile: 0,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        });
    }
    if source.metatile_id == 0x3c && source.subtile_column == 3 {
        return Some(CellShape::LedgeBand {
            face: LedgeFace::East,
            plane_subtile: 4,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        });
    }

    // `$12/$30` are a mirrored pair of quarter-rock transitions. In `$12`
    // the southwest quadrant is the horizontal cap and the southeast
    // `$36/$37` drawing is its two-course east wall. `$30` mirrors that
    // arrangement around the center seam. This is source-art topology, not
    // a collision fallback: only these exact blocks and quadrants participate.
    if matches!(source.metatile_id, 0x12 | 0x30) && source.subtile_row >= 2 {
        let west_cap = source.metatile_id == 0x12 && source.subtile_column < 2;
        let east_cap = source.metatile_id == 0x30 && source.subtile_column >= 2;
        if west_cap || east_cap {
            return Some(CellShape::RaisedTop {
                height: CAVE_ROCK_HEIGHT,
                solid: SolidKind::Bank,
            });
        }
        return Some(if source.metatile_id == 0x12 {
            CellShape::LedgeBand {
                face: LedgeFace::East,
                plane_subtile: 2,
                band_from_top: source.subtile_column - 2,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
            }
        } else {
            CellShape::LedgeBand {
                face: LedgeFace::West,
                plane_subtile: 2,
                band_from_top: 1 - source.subtile_column,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
            }
        });
    }

    // `$2d/$2e/$2f` are the left edge, interior, and right edge of one
    // repeating ceiling-rock mass. Their native `$25/$26/$27` art differs
    // only at the outside boundaries, so all three blocks share the normal
    // cave-rock datum and join through the generic bank topology pass. Do
    // not split them into sixteen little columns: adjacent RaisedTop cells
    // suppress their internal faces and expose only the authored perimeter.
    if CEILING_MASS_METATILES.contains(&source.metatile_id) {
        return Some(CellShape::RaisedTop {
            height: CAVE_ROCK_HEIGHT,
            solid: SolidKind::Bank,
        });
    }

    // `$31` is one closed lit shelf. Rows 0..2 already resolve as its north
    // cap, horizontal interior, and west/east edge courses. The final
    // `$25/$26/$27` row is the matching south lip; leaving it flat opens the
    // shelf at its foot and makes a blocked strip coplanar with the cave
    // floor. Fold that single authored row to the same 6px shelf datum.
    if source.metatile_id == 0x31 && source.subtile_row == 3 {
        return Some(CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        });
    }

    // `$38/$39/$3a` are the left, middle, and right blocks of one hop-down
    // shelf. Their first three rows are horizontal `$01` shelf art (plus the
    // authored west/east boundary); the final `$25/$26/$27` row is one front
    // course. Fold that row once at the normal six-pixel shelf datum. It is
    // not a second terrain tier and must never inherit the 16px rock height.
    if HOP_DOWN_SHELF_METATILES.contains(&source.metatile_id) && source.subtile_row == 3 {
        return Some(CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        });
    }

    // These blocks are complete shelf-to-floor transition drawings. Their
    // north half is the horizontal cap and their south half is two authored
    // rock courses. Treating $25-$27 as globally raised tiles turns every
    // course into an isolated box; fold the two rows once at the shared edge.
    if SOUTH_SHELF_METATILES.contains(&source.metatile_id) {
        return Some(if source.subtile_row >= 2 {
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: source.subtile_row - 2,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
            }
        } else if source.metatile_id == 0x0c && source.subtile_column < 2 {
            // `$0c` is the west end of the shelf. Its upper-left 2x2
            // drawing is the native west wall, not horizontal cap art.
            CellShape::LedgeBand {
                face: LedgeFace::West,
                plane_subtile: 0,
                band_from_top: 1 - source.subtile_column,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
            }
        } else if matches!(source.metatile_id, 0x0e | 0x36) && source.subtile_column >= 2 {
            // `$0e/$36` mirror that authored side drawing at the east end.
            CellShape::LedgeBand {
                face: LedgeFace::East,
                plane_subtile: 4,
                band_from_top: source.subtile_column - 2,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
            }
        } else {
            CellShape::RaisedTop {
                height: CAVE_ROCK_HEIGHT,
                solid: SolidKind::Bank,
            }
        });
    }

    // Block $27 is the one-course rocky lip above cave water. Preserve the
    // native face row; the water rows themselves stay on the cave datum.
    if source.metatile_id == 0x27 {
        return Some(if source.subtile_row == 0 {
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 1,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x16,
                height: CAVE_SHELF_HEIGHT,
            }
        } else {
            CellShape::Flat
        });
    }

    // $14/$15 carry Crystal's narrow ladder/stair drawings in their southeast
    // quadrant. They connect the 6px lit shelf at the north end to the cave
    // datum at the south end. Split that complete 16px flight across its two
    // native rows; standing the drawing upright creates a detached panel.
    if matches!(source.metatile_id, 0x14 | 0x15)
        && source.subtile_column >= 2
        && source.subtile_row >= 2
    {
        return Some(if source.subtile_row == 2 {
            CellShape::RampNorth {
                north_height: CAVE_SHELF_HEIGHT,
                south_height: CAVE_SHELF_HEIGHT * 0.5,
            }
        } else {
            CellShape::RampNorth {
                north_height: CAVE_SHELF_HEIGHT * 0.5,
                south_height: 0.0,
            }
        });
    }

    let shape = match source.tile_index {
        // The animated cave pool shares the lower cave datum. Unlike an
        // outdoor shoreline, it is not recessed below adjacent cave floor.
        0x14 | 0x16 => CellShape::Flat,

        // Lit shelf surface and its rounded corner caps.
        0x01 | 0x05 | 0x07 => CellShape::RaisedTop {
            height: CAVE_SHELF_HEIGHT,
            solid: SolidKind::Bank,
        },

        // One native course at each exposed edge of that shelf. The source
        // artwork itself supplies the face; the top is restored from $01.
        0x24 => CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: source.subtile_row.saturating_add(1),
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        },
        0x15 => CellShape::LedgeBand {
            face: LedgeFace::West,
            plane_subtile: source.subtile_column,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        },
        0x17 => CellShape::LedgeBand {
            face: LedgeFace::East,
            plane_subtile: source.subtile_column.saturating_add(1),
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x01,
            height: CAVE_SHELF_HEIGHT,
        },

        _ => return None,
    };
    Some(shape)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tileset: &str, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset),
            // Neutral floor context. Tests that exercise authored metatile
            // structure set their exact block explicitly.
            metatile_id: 0x01,
            subtile_column: 1,
            subtile_row: 2,
            tile_index,
        }
    }

    #[test]
    fn diagonal_corner_pair_requires_each_exact_two_by_two_drawing() {
        for (metatile, corner, origin, drawing) in [
            (
                0x10,
                DiagonalCorner::SouthEast,
                2,
                [[0x0a, 0x26], [0x17, 0x0a]],
            ),
            (
                0x35,
                DiagonalCorner::SouthEast,
                2,
                [[0x0a, 0x26], [0x17, 0x0a]],
            ),
            (
                0x11,
                DiagonalCorner::SouthWest,
                0,
                [[0x26, 0x0b], [0x0b, 0x15]],
            ),
        ] {
            for row in 0..2 {
                for column in 0..2 {
                    let mut cell = source("dark_cave", drawing[row][column]);
                    cell.metatile_id = metatile;
                    cell.subtile_column = origin + column as u8;
                    cell.subtile_row = 2 + row as u8;
                    assert_eq!(
                        diagonal_corner_local(&cell),
                        Some((corner, column as u8, row as u8))
                    );
                }
            }
        }

        let mut wrong = source("dark_cave", 0x16);
        wrong.metatile_id = 0x10;
        wrong.subtile_column = 2;
        wrong.subtile_row = 2;
        assert_eq!(diagonal_corner_local(&wrong), None);
    }

    #[test]
    fn complete_cave_ladder_drawings_remain_explicit_flat_planes() {
        for (metatile, drawing) in [
            (0x07, [[0x2a, 0x2b], [0x3a, 0x3b]]),
            (0x0b, [[0x22, 0x23], [0x32, 0x33]]),
            (0x1b, [[0x28, 0x29], [0x38, 0x39]]),
            (0x1f, [[0x20, 0x21], [0x30, 0x31]]),
        ] {
            for tileset in ["cave", "dark_cave"] {
                for row in 0..2 {
                    for column in 0..2 {
                        let mut cell = source(tileset, drawing[row][column]);
                        cell.metatile_id = metatile;
                        cell.subtile_column = column as u8 + 2;
                        cell.subtile_row = row as u8 + 2;
                        assert_eq!(flat_ladder_local(&cell), Some((column as u8, row as u8)));
                        assert_eq!(cave_shape(&cell), Some(CellShape::PlaneAt { height: 0.0 }));
                    }
                }
            }
        }

        let mut incomplete = source("cave", 0x20);
        incomplete.metatile_id = 0x1f;
        incomplete.subtile_column = 1;
        incomplete.subtile_row = 2;
        assert_eq!(flat_ladder_local(&incomplete), None);
    }

    #[test]
    fn block_23_keeps_incomplete_half_drawing_flat_beside_real_shelf_lip() {
        for (column, tile) in [(0, 0x24), (1, 0x24), (2, 0x1a), (3, 0x1b)] {
            let mut cell = source("cave", tile);
            cell.metatile_id = 0x23;
            cell.subtile_column = column;
            cell.subtile_row = 0;
            if column < 2 {
                assert!(matches!(
                    cave_shape(&cell),
                    Some(CellShape::LedgeBand { .. })
                ));
            } else {
                assert_eq!(cave_shape(&cell), Some(CellShape::PlaneAt { height: 0.0 }));
            }
        }
    }

    #[test]
    fn base_cave_reuses_the_same_exact_diagonal_drawing() {
        let mut source = source("cave", 0x26);
        source.metatile_id = 0x11;
        source.subtile_column = 0;
        source.subtile_row = 2;
        assert_eq!(
            diagonal_corner_local(&source),
            Some((DiagonalCorner::SouthWest, 0, 0))
        );
        source.tile_index = 0x16;
        assert_eq!(diagonal_corner_local(&source), None);
    }

    #[test]
    fn small_cave_rock_requires_its_exact_two_by_two_drawing_cells() {
        for (row, tiles) in [[0x0c, 0x0d], [0x1c, 0x1d]].into_iter().enumerate() {
            for (column, tile_index) in tiles.into_iter().enumerate() {
                let mut cell = source("cave", tile_index);
                cell.metatile_id = 0x18;
                cell.subtile_column = column as u8;
                cell.subtile_row = row as u8;
                assert_eq!(small_rock_local(&cell), Some((column as u8, row as u8)));
            }
        }
        assert_eq!(small_rock_local(&source("cave", 0x01)), None);
        assert_eq!(small_rock_local(&source("johto", 0x0c)), None);
    }

    #[test]
    fn cave_datum_shelf_and_rock_are_three_distinct_levels() {
        assert_eq!(cave_shape(&source("cave", 0x16)), Some(CellShape::Flat));
        assert_eq!(
            cave_shape(&source("cave", 0x01))
                .unwrap()
                .surface_height(8.0),
            CAVE_SHELF_HEIGHT
        );
        let mut transition = source("dark_cave", 0x16);
        transition.metatile_id = 0x0d;
        transition.subtile_row = 1;
        assert_eq!(
            cave_shape(&transition).unwrap().surface_height(8.0),
            CAVE_ROCK_HEIGHT
        );
    }

    #[test]
    fn cave_water_is_level_with_lower_floor_not_an_outdoor_trough() {
        assert_eq!(cave_shape(&source("cave", 0x14)), Some(CellShape::Flat));
    }

    #[test]
    fn cave_water_corners_do_not_inherit_reused_shelf_edges() {
        for (metatile, drawing) in [
            (
                0x3d,
                [
                    [0x26, 0x26, 0x26, 0x0b],
                    [0x14, 0x14, 0x14, 0x15],
                    [0x14, 0x14, 0x14, 0x15],
                    [0x14, 0x14, 0x14, 0x15],
                ],
            ),
            (
                0x3e,
                [
                    [0x0a, 0x26, 0x26, 0x26],
                    [0x17, 0x14, 0x14, 0x14],
                    [0x17, 0x14, 0x14, 0x14],
                    [0x17, 0x14, 0x14, 0x14],
                ],
            ),
        ] {
            for row in 0..4 {
                for column in 0..4 {
                    let mut cell = source("cave", drawing[row][column]);
                    cell.metatile_id = metatile;
                    cell.subtile_column = column as u8;
                    cell.subtile_row = row as u8;
                    assert_eq!(cave_shape(&cell), Some(CellShape::Flat));
                }
            }
        }

        let ordinary_shelf_edge = source("cave", 0x17);
        assert!(matches!(
            cave_shape(&ordinary_shelf_edge),
            Some(CellShape::LedgeBand {
                face: LedgeFace::East,
                ..
            })
        ));
    }

    #[test]
    fn lateral_hop_edges_are_one_shallow_course_not_four_levels() {
        for (metatile, column, tile, face, plane) in [
            (0x3b, 0, 0x15, LedgeFace::West, 0),
            (0x3c, 3, 0x17, LedgeFace::East, 4),
        ] {
            for row in 0..4 {
                let mut cell = source("dark_cave", tile);
                cell.metatile_id = metatile;
                cell.subtile_column = column;
                cell.subtile_row = row;
                assert_eq!(
                    cave_shape(&cell),
                    Some(CellShape::LedgeBand {
                        face,
                        plane_subtile: plane,
                        band_from_top: 0,
                        band_count: 1,
                        top_tile_index: 0x01,
                        height: CAVE_SHELF_HEIGHT,
                    })
                );
            }
        }

        let mut interior = source("cave", 0x01);
        interior.metatile_id = 0x3b;
        interior.subtile_column = 1;
        interior.subtile_row = 2;
        assert_eq!(
            cave_shape(&interior),
            Some(CellShape::RaisedTop {
                height: CAVE_SHELF_HEIGHT,
                solid: SolidKind::Bank,
            })
        );
    }

    #[test]
    fn ceiling_mass_edges_and_interior_share_one_rock_datum() {
        for tileset in ["cave", "dark_cave"] {
            for (metatile, expected_tiles) in [
                (0x2d, [0x25, 0x26, 0x26, 0x26]),
                (0x2e, [0x26, 0x26, 0x26, 0x26]),
                (0x2f, [0x26, 0x26, 0x26, 0x27]),
            ] {
                for (column, tile_index) in expected_tiles.into_iter().enumerate() {
                    let mut cell = source(tileset, tile_index);
                    cell.metatile_id = metatile;
                    cell.subtile_column = column as u8;
                    for row in 0..4 {
                        cell.subtile_row = row;
                        assert_eq!(
                            cave_shape(&cell).unwrap().surface_height(8.0),
                            CAVE_ROCK_HEIGHT,
                            "{tileset} ${metatile:02x} ({column}, {row})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn closed_shelf_folds_its_native_south_lip_to_the_shelf_datum() {
        for tileset in ["cave", "dark_cave"] {
            for (column, tile_index) in [0x25, 0x26, 0x26, 0x27].into_iter().enumerate() {
                let mut cell = source(tileset, tile_index);
                cell.metatile_id = 0x31;
                cell.subtile_column = column as u8;
                cell.subtile_row = 3;
                assert_eq!(
                    cave_shape(&cell),
                    Some(CellShape::LedgeBand {
                        face: LedgeFace::South,
                        plane_subtile: 4,
                        band_from_top: 0,
                        band_count: 1,
                        top_tile_index: 0x01,
                        height: CAVE_SHELF_HEIGHT,
                    })
                );
            }
        }
    }

    #[test]
    fn hop_down_family_has_one_six_pixel_front_course() {
        for tileset in ["cave", "dark_cave"] {
            for (metatile, expected) in [
                (0x38, [0x25, 0x26, 0x26, 0x26]),
                (0x39, [0x26, 0x26, 0x26, 0x26]),
                (0x3a, [0x26, 0x26, 0x26, 0x27]),
            ] {
                for (column, tile_index) in expected.into_iter().enumerate() {
                    let mut cell = source(tileset, tile_index);
                    cell.metatile_id = metatile;
                    cell.subtile_column = column as u8;
                    cell.subtile_row = 3;
                    assert_eq!(
                        cave_shape(&cell),
                        Some(CellShape::LedgeBand {
                            face: LedgeFace::South,
                            plane_subtile: 4,
                            band_from_top: 0,
                            band_count: 1,
                            top_tile_index: 0x01,
                            height: CAVE_SHELF_HEIGHT,
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn unrelated_cave_art_and_other_tilesets_are_not_profiled() {
        assert_eq!(cave_shape(&source("cave", 0x33)), None);
        assert_eq!(cave_shape(&source("johto", 0x26)), None);
    }

    #[test]
    fn south_shelf_folds_two_native_rows_instead_of_boxing_rock_tiles() {
        let mut top = source("cave", 0x16);
        top.metatile_id = 0x0d;
        top.subtile_row = 1;
        assert!(matches!(top_shape(&top), CellShape::RaisedTop { .. }));

        let mut upper_face = top.clone();
        upper_face.tile_index = 0x26;
        upper_face.subtile_row = 2;
        assert_eq!(
            top_shape(&upper_face),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
            }
        );

        let mut lower_face = upper_face.clone();
        lower_face.subtile_row = 3;
        assert!(matches!(
            top_shape(&lower_face),
            CellShape::LedgeBand {
                band_from_top: 1,
                ..
            }
        ));
    }

    #[test]
    fn shelf_end_wall_courses_do_not_lie_flat_on_the_cap() {
        for (metatile, column, face, band_from_top) in [
            (0x0c, 0, LedgeFace::West, 1),
            (0x0c, 1, LedgeFace::West, 0),
            (0x0e, 2, LedgeFace::East, 0),
            (0x0e, 3, LedgeFace::East, 1),
        ] {
            let mut wall = source("cave", if column < 2 { 0x15 } else { 0x17 });
            wall.metatile_id = metatile;
            wall.subtile_column = column;
            wall.subtile_row = 0;
            assert_eq!(
                cave_shape(&wall),
                Some(CellShape::LedgeBand {
                    face,
                    plane_subtile: if face == LedgeFace::West { 0 } else { 4 },
                    band_from_top,
                    band_count: 2,
                    top_tile_index: 0x16,
                    height: CAVE_ROCK_HEIGHT,
                })
            );
        }

        let mut cap = source("cave", 0x16);
        cap.metatile_id = 0x0c;
        cap.subtile_column = 2;
        cap.subtile_row = 0;
        assert!(matches!(
            cave_shape(&cap),
            Some(CellShape::RaisedTop { .. })
        ));
    }

    #[test]
    fn rock_face_tile_id_is_not_globally_promoted_to_a_box() {
        assert_eq!(cave_shape(&source("cave", 0x26)), None);
    }

    #[test]
    fn mixed_ceiling_blocks_raise_only_their_authored_mass_cells() {
        for row in 0..4 {
            for column in 0..4 {
                let mut block_34 = source(
                    "dark_cave",
                    if row < 2 {
                        [[0x0c, 0x0d], [0x1c, 0x1d]][usize::from(row)][usize::from(column % 2)]
                    } else {
                        0x26
                    },
                );
                block_34.metatile_id = 0x34;
                block_34.subtile_column = column;
                block_34.subtile_row = row;
                if row >= 2 {
                    assert_eq!(
                        cave_shape(&block_34),
                        Some(CellShape::RaisedTop {
                            height: CAVE_ROCK_HEIGHT,
                            solid: SolidKind::Bank,
                        })
                    );
                } else {
                    assert_eq!(cave_shape(&block_34), None);
                }

                let barred_shelf = row >= 2 && column >= 2;
                let mut block_37 = source(
                    "cave",
                    if barred_shelf {
                        [[0x0e, 0x0f], [0x1e, 0x1f]][usize::from(row - 2)][usize::from(column - 2)]
                    } else {
                        0x26
                    },
                );
                block_37.metatile_id = 0x37;
                block_37.subtile_column = column;
                block_37.subtile_row = row;
                assert!(matches!(
                    cave_shape(&block_37),
                    Some(CellShape::RaisedTop { .. }) | Some(CellShape::LedgeBand { .. })
                ));
            }
        }
    }

    #[test]
    fn barred_shelf_block_joins_its_left_ceiling_mass_without_claiming_floor() {
        for row in 0..4 {
            for column in 0..4 {
                let barred_shelf = row >= 2 && column >= 2;
                let mut cell = source(
                    "cave",
                    if barred_shelf {
                        [[0x0e, 0x0f], [0x1e, 0x1f]][usize::from(row - 2)][usize::from(column - 2)]
                    } else if row >= 2 {
                        0x26
                    } else {
                        0x16
                    },
                );
                cell.metatile_id = 0x13;
                cell.subtile_column = column;
                cell.subtile_row = row;
                let shape = cave_shape(&cell);
                if row < 2 {
                    assert_eq!(shape, Some(CellShape::Flat));
                } else if column < 2 {
                    assert!(matches!(shape, Some(CellShape::RaisedTop { .. })));
                } else {
                    assert!(matches!(shape, Some(CellShape::LedgeBand { .. })));
                }
            }
        }
    }

    #[test]
    fn barred_rock_shelf_folds_exactly_two_native_face_courses() {
        for metatile in [0x13, 0x37] {
            let drawing = [[0x0e, 0x0f], [0x1e, 0x1f]];
            for row in 0..2 {
                for column in 0..2 {
                    let mut cell = source("dark_cave", drawing[row][column]);
                    cell.metatile_id = metatile;
                    cell.subtile_column = 2 + column as u8;
                    cell.subtile_row = 2 + row as u8;
                    assert_eq!(
                        cave_shape(&cell),
                        Some(CellShape::LedgeBand {
                            face: LedgeFace::South,
                            plane_subtile: 4,
                            band_from_top: row as u8,
                            band_count: 2,
                            top_tile_index: if metatile == 0x13 { 0x16 } else { 0x26 },
                            height: CAVE_ROCK_HEIGHT,
                        })
                    );
                }
            }
        }

        let mut lookalike = source("cave", 0x0e);
        lookalike.metatile_id = 0x32;
        lookalike.subtile_column = 2;
        lookalike.subtile_row = 2;
        assert_eq!(cave_shape(&lookalike), None);
    }

    #[test]
    fn mirrored_quarter_rocks_keep_cap_and_sidewall_on_one_datum() {
        for metatile in [0x12, 0x30] {
            for row in 2..4 {
                for column in 0..4 {
                    let cap = (metatile == 0x12 && column < 2) || (metatile == 0x30 && column >= 2);
                    let mut cell = source(
                        "cave",
                        if cap {
                            0x26
                        } else if column % 2 == 0 {
                            0x36
                        } else {
                            0x37
                        },
                    );
                    cell.metatile_id = metatile;
                    cell.subtile_column = column;
                    cell.subtile_row = row;
                    if cap {
                        assert_eq!(
                            cave_shape(&cell),
                            Some(CellShape::RaisedTop {
                                height: CAVE_ROCK_HEIGHT,
                                solid: SolidKind::Bank,
                            })
                        );
                    } else {
                        let (face, band_from_top) = if metatile == 0x12 {
                            (LedgeFace::East, column - 2)
                        } else {
                            (LedgeFace::West, 1 - column)
                        };
                        assert_eq!(
                            cave_shape(&cell),
                            Some(CellShape::LedgeBand {
                                face,
                                plane_subtile: 2,
                                band_from_top,
                                band_count: 2,
                                top_tile_index: 0x16,
                                height: CAVE_ROCK_HEIGHT,
                            })
                        );
                    }
                }
            }
        }

        let mut reused_strip = source("cave", 0x36);
        reused_strip.metatile_id = 0x36;
        reused_strip.subtile_column = 0;
        reused_strip.subtile_row = 2;
        assert!(matches!(
            cave_shape(&reused_strip),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                ..
            })
        ));
    }

    #[test]
    fn cave_ladder_is_one_two_row_north_rising_flight() {
        for (row, expected) in [
            (
                2,
                CellShape::RampNorth {
                    north_height: CAVE_SHELF_HEIGHT,
                    south_height: CAVE_SHELF_HEIGHT * 0.5,
                },
            ),
            (
                3,
                CellShape::RampNorth {
                    north_height: CAVE_SHELF_HEIGHT * 0.5,
                    south_height: 0.0,
                },
            ),
        ] {
            let mut ladder = source("cave", if row == 2 { 0x2a } else { 0x3a });
            ladder.metatile_id = 0x14;
            ladder.subtile_column = 2;
            ladder.subtile_row = row;
            assert_eq!(cave_shape(&ladder), Some(expected));
        }
    }

    fn top_shape(source: &VisualTileSource) -> CellShape {
        cave_shape(source).expect("profiled cave transition")
    }
}
