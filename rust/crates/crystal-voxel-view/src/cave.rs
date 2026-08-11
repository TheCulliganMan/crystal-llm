//! Exact visual roles for Crystal's cave and dark-cave tile drawings.
//!
//! These identities come from the shipped metatile catalogs. They are not
//! inferred from runtime collision: unknown cave cells remain on the lower
//! datum and never become walls merely because movement is blocked.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const CAVE_SHELF_HEIGHT: f32 = 6.0;
pub(crate) const CAVE_ROCK_HEIGHT: f32 = 16.0;

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
        return Some(if source.subtile_row < 2 {
            CellShape::RaisedTop {
                height: CAVE_ROCK_HEIGHT,
                solid: SolidKind::Bank,
            }
        } else {
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: source.subtile_row - 2,
                band_count: 2,
                top_tile_index: 0x16,
                height: CAVE_ROCK_HEIGHT,
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

    // $14/$15 carry Crystal's narrow two-by-two ladder drawings in their
    // southeast quadrant. They are props, not flights of stairs: group both
    // source rows on the shared south plane and let the facade mesher flood
    // away the surrounding $16 cave floor.
    if matches!(source.metatile_id, 0x14 | 0x15)
        && source.subtile_column >= 2
        && source.subtile_row >= 2
    {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x16,
            solid: SolidKind::Prop,
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
    fn rock_face_tile_id_is_not_globally_promoted_to_a_box() {
        assert_eq!(cave_shape(&source("cave", 0x26)), None);
    }

    #[test]
    fn cave_ladder_is_a_two_band_upright_prop_not_a_staircase() {
        for (row, band) in [(2, 0), (3, 1)] {
            let mut ladder = source("cave", if row == 2 { 0x2a } else { 0x3a });
            ladder.metatile_id = 0x14;
            ladder.subtile_column = 2;
            ladder.subtile_row = row;
            assert_eq!(
                cave_shape(&ladder),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 4,
                    band_from_top: band,
                    band_count: 2,
                    ground_tile_index: 0x16,
                    solid: SolidKind::Prop,
                })
            );
        }
    }

    fn top_shape(source: &VisualTileSource) -> CellShape {
        cave_shape(source).expect("profiled cave transition")
    }
}
