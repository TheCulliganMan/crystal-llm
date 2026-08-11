//! Authored Kanto overworld cliff-mound profiles.
//!
//! These rules describe Crystal's stable visual source identities only. They
//! never inspect collision or movement permissions. Unknown Kanto artwork
//! remains on the faithful flat baseline.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const KANTO_CLIFF_HEIGHT: f32 = 16.0;

/// Resolves the six-block Diglett's Cave mound family used by Kanto towns and
/// routes. Crystal draws one plateau row over one mixed plateau/front row:
/// `[3e 3f 3b; 24 06 25]`. The lower block row's final two source rows are
/// the actual rock/cave-mouth courses and fold once onto the south edge.
pub(crate) fn kanto_cliff_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "kanto" {
        return None;
    }

    match source.metatile_id {
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
}
