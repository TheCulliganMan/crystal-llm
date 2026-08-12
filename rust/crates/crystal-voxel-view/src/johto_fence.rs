//! Reusable fence drawings from Crystal's base Johto outdoor atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const TILESET: &str = "johto";
const GROUND_TILE: u16 = 0x06;

/// Block $49 is two rows of ground followed by the authored upper/lower
/// courses of one horizontal fence. Fold only those two exact source rows;
/// the surrounding ground stays at the normal map plane.
pub(crate) fn johto_fence_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != TILESET || source.metatile_id != 0x49 {
        return None;
    }
    let band_from_top = match (source.subtile_row, source.tile_index) {
        (2, 0x5a) => 0,
        (3, 0x59) => 1,
        _ => return None,
    };
    Some(CellShape::FacadeBand {
        plane_subtile_row: 4,
        band_from_top,
        band_count: 2,
        ground_tile_index: GROUND_TILE,
        solid: SolidKind::Fence,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile: u16, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(TILESET),
            metatile_id: metatile,
            subtile_column: 0,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn block_49_folds_only_its_two_native_fence_courses() {
        assert_eq!(
            johto_fence_shape(&source(0x49, 2, 0x5a)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: GROUND_TILE,
                solid: SolidKind::Fence,
            })
        );
        assert_eq!(
            johto_fence_shape(&source(0x49, 3, 0x59)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: GROUND_TILE,
                solid: SolidKind::Fence,
            })
        );
        assert_eq!(johto_fence_shape(&source(0x49, 1, 0x06)), None);
        assert_eq!(johto_fence_shape(&source(0x48, 2, 0x5a)), None);
    }
}
