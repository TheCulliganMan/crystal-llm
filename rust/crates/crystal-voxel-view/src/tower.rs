//! Authored shapes for the Crystal `tower` interior atlas.
//!
//! Pewter Gym's rock maze is drawn as repeated complete 2x2 boulders. The
//! surrounding metatile arrangement describes placement, not one continuous
//! wall, so each drawing is grouped independently and stood up like the
//! renderer's flat 2.5D trees. No inferred voxel volume is added.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const TOWER_FLOOR_TILE: u16 = 0x02;

pub(crate) fn tower_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "tower" {
        return None;
    }

    let boulder = matches!(source.metatile_id, 0x32..=0x36)
        && matches!(source.tile_index, 0x04 | 0x05 | 0x14 | 0x15);
    let statue = source.metatile_id == 0x26
        && source.subtile_row >= 2
        && matches!(source.tile_index, 0x07..=0x09 | 0x17..=0x19);
    if !boulder && !statue {
        let north_wall = matches!(source.metatile_id, 0x04..=0x06)
            && source.subtile_row < 2
            && matches!(source.tile_index, 0x11 | 0x20..=0x23 | 0x40);
        let partition = matches!(source.metatile_id, 0x0c..=0x0e | 0x16 | 0x17)
            && source.subtile_row >= 2
            && matches!(source.tile_index, 0x11 | 0x20 | 0x21 | 0x40);
        if north_wall || partition {
            let first_row = if north_wall { 0 } else { 2 };
            return Some(CellShape::FacadeBand {
                plane_subtile_row: first_row + 2,
                band_from_top: source.subtile_row - first_row,
                band_count: 2,
                ground_tile_index: TOWER_FLOOR_TILE,
                solid: SolidKind::Prop,
            });
        }
        return None;
    }

    Some(CellShape::FacadeBand {
        plane_subtile_row: source.subtile_row + 2,
        band_from_top: if boulder {
            u8::from(matches!(source.tile_index, 0x14 | 0x15))
        } else {
            source.subtile_row - 2
        },
        band_count: 2,
        ground_tile_index: TOWER_FLOOR_TILE,
        solid: SolidKind::Tree,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("tower"),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn pewter_boulder_drawing_is_grouped_but_floor_stays_flat() {
        for (tile, row) in [(0x04, 0), (0x05, 0), (0x14, 1), (0x15, 1)] {
            assert!(matches!(
                tower_shape(&source(
                    0x32,
                    u8::from(tile == 0x05 || tile == 0x15),
                    row,
                    tile
                )),
                Some(CellShape::FacadeBand {
                    solid: SolidKind::Tree,
                    ground_tile_index: TOWER_FLOOR_TILE,
                    ..
                })
            ));
        }
        assert_eq!(tower_shape(&source(0x32, 0, 2, TOWER_FLOOR_TILE)), None);
    }

    #[test]
    fn pewter_entrance_statues_are_grouped_flat_standees() {
        for (column, tile) in [(0, 0x07), (1, 0x08), (2, 0x08), (3, 0x09)] {
            assert!(matches!(
                tower_shape(&source(0x26, column, 2, tile)),
                Some(CellShape::FacadeBand {
                    solid: SolidKind::Tree,
                    ground_tile_index: TOWER_FLOOR_TILE,
                    ..
                })
            ));
        }
    }

    #[test]
    fn tower_horizontal_wall_courses_fold_once_but_invisible_maze_stays_flat() {
        assert!(matches!(
            tower_shape(&source(0x05, 0, 0, 0x11)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                solid: SolidKind::Prop,
                ..
            })
        ));
        assert!(matches!(
            tower_shape(&source(0x0d, 0, 3, 0x21)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                solid: SolidKind::Prop,
                ..
            })
        ));
        assert_eq!(tower_shape(&source(0x37, 0, 0, 0x01)), None);
    }
}
