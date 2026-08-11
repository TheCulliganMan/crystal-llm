//! Visible one-course maze walls for Viridian Gym.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const FLOOR_TILE: u16 = 0x3d;

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "ViridianGym"
        || source.tileset_id.as_ref() != "train_station"
        || !(0x36..=0x3f).contains(&source.metatile_id)
        || source.tile_index == FLOOR_TILE
    {
        return None;
    }

    // Each 2x2 source quadrant is one visible 16px maze-wall drawing. Fold
    // its two rows once at that quadrant's south edge. Adjacent quadrants may
    // repeat, but never accumulate into the 48px fins warned about by the
    // reference profile.
    let quadrant_row = source.subtile_row / 2 * 2;
    Some(CellShape::FacadeBand {
        plane_subtile_row: quadrant_row + 2,
        band_from_top: source.subtile_row - quadrant_row,
        band_count: 2,
        ground_tile_index: FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tile: u16, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("train_station"),
            metatile_id: 0x36,
            subtile_column: 0,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn each_visible_maze_cell_is_exactly_one_native_course() {
        assert_eq!(
            shape("ViridianGym", &source(0x02, 0)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: FLOOR_TILE,
                solid: SolidKind::FlatCard,
            })
        );
        assert_eq!(
            shape("ViridianGym", &source(0x02, 3)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: FLOOR_TILE,
                solid: SolidKind::FlatCard,
            })
        );
    }

    #[test]
    fn floor_and_shared_atlas_maps_are_not_raised() {
        assert_eq!(shape("ViridianGym", &source(FLOOR_TILE, 0)), None);
        assert_eq!(shape("CeladonGym", &source(0x02, 0)), None);
    }
}
