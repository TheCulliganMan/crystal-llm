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

/// Returns the local cell and orientation of Crystal's complete diagonal
/// cave-corner drawings. `$10` owns the southeast collision quadrant and
/// `$11` mirrors it in the southwest quadrant. The exact 2x2 drawings are
/// grouped by the mesher; no individual `$0a/$0b` tile is promoted alone.
pub(crate) fn diagonal_corner_local(source: &VisualTileSource) -> Option<(DiagonalCorner, u8, u8)> {
    if !matches!(source.tileset_id.as_ref(), "cave" | "dark_cave") {
        return None;
    }
    let (corner, origin_column, expected) = match source.metatile_id {
        0x10 => (DiagonalCorner::SouthEast, 2, [[0x0a, 0x26], [0x17, 0x0a]]),
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

pub(crate) fn cave_shape(source: &VisualTileSource) -> Option<CellShape> {
    if !matches!(source.tileset_id.as_ref(), "cave" | "dark_cave") {
        return None;
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
            metatile_id: 0x2e,
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
