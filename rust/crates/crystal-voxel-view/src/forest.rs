//! Presentation-only shape identities for Crystal's Forest tileset.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, JUMP_LEDGE_HEIGHT, LedgeFace, SolidKind};

const FOREST_TILESET: &str = "forest";
const FOREST_GROUND_TILE: u16 = 0x05;

/// Resolve the connected `$18-$1c` Forest jump-ledge drawing.
///
/// The U-shaped drawing has authored west/east side courses and one south
/// lip. Cells enclosed by those courses are its raised cap. This is visual
/// source classification only; gameplay collision and hop permissions remain
/// authoritative in crystal-core.
pub(crate) fn forest_ledge_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != FOREST_TILESET {
        return None;
    }
    let raised = || CellShape::RaisedTop {
        height: JUMP_LEDGE_HEIGHT,
        solid: SolidKind::Bank,
    };
    let band = |face, plane_subtile| CellShape::LedgeBand {
        face,
        plane_subtile,
        band_from_top: 0,
        band_count: 1,
        top_tile_index: FOREST_GROUND_TILE,
        height: JUMP_LEDGE_HEIGHT,
    };
    match source.metatile_id {
        0x18 => Some(if source.subtile_column == 0 {
            band(LedgeFace::West, 0)
        } else {
            raised()
        }),
        0x19 => Some(if source.subtile_row == 3 {
            band(LedgeFace::South, 4)
        } else if source.subtile_column == 0 {
            band(LedgeFace::West, 0)
        } else {
            raised()
        }),
        0x1a => Some(if source.subtile_row == 3 {
            band(LedgeFace::South, 4)
        } else {
            raised()
        }),
        0x1b => Some(if source.subtile_row == 3 {
            band(LedgeFace::South, 4)
        } else if source.subtile_column == 3 {
            band(LedgeFace::East, 4)
        } else {
            raised()
        }),
        0x1c if source.subtile_column < 2 => Some(if source.subtile_row == 3 {
            band(LedgeFace::South, 4)
        } else {
            raised()
        }),
        0x1c if source.subtile_row < 2 => Some(raised()),
        0x1c => {
            // The opening occupies only the south half of this metatile.
            // Its north half is the upper plateau; two 8px source rows make
            // the short transition down to the lower path.
            let step = JUMP_LEDGE_HEIGHT / 2.0;
            let ramp_row = source.subtile_row - 2;
            let north_height = JUMP_LEDGE_HEIGHT - f32::from(ramp_row) * step;
            Some(CellShape::RampNorth {
                north_height,
                south_height: north_height - step,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(FOREST_TILESET),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: FOREST_GROUND_TILE,
        }
    }

    #[test]
    fn forest_u_ledge_separates_cap_and_directional_courses() {
        assert!(matches!(
            forest_ledge_shape(&source(0x19, 1, 1)),
            Some(CellShape::RaisedTop { height, .. }) if height == JUMP_LEDGE_HEIGHT
        ));
        assert!(matches!(
            forest_ledge_shape(&source(0x19, 0, 1)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::West,
                ..
            })
        ));
        assert!(matches!(
            forest_ledge_shape(&source(0x1a, 2, 3)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::South,
                ..
            })
        ));
        assert!(matches!(
            forest_ledge_shape(&source(0x1b, 3, 1)),
            Some(CellShape::LedgeBand {
                face: LedgeFace::East,
                ..
            })
        ));
        assert_eq!(
            forest_ledge_shape(&source(0x1c, 2, 1)),
            Some(CellShape::RaisedTop {
                height: JUMP_LEDGE_HEIGHT,
                solid: SolidKind::Bank,
            })
        );
        assert_eq!(
            forest_ledge_shape(&source(0x1c, 2, 2)),
            Some(CellShape::RampNorth {
                north_height: JUMP_LEDGE_HEIGHT,
                south_height: JUMP_LEDGE_HEIGHT / 2.0,
            })
        );
    }
}
