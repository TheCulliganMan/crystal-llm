//! Authored shapes for the Crystal `tower` interior atlas.
//!
//! Pewter Gym's rock maze is drawn as repeated complete 2x2 boulders. The
//! surrounding metatile arrangement describes placement, not one continuous
//! wall, so each complete drawing is grouped independently by the mesher.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const TOWER_FLOOR_TILE: u16 = 0x02;

/// Block `$26` contains two independent 16x16 statues in its lower half.
/// Return coordinates local to each two-by-two drawing so the mesher does not
/// fuse the pair into one four-tile-wide facade.
pub(crate) fn statue_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "tower" || source.metatile_id != 0x26 || source.subtile_row < 2
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row - 2;
    let expected = if source.subtile_column < 2 {
        [[0x07, 0x08], [0x17, 0x18]]
    } else {
        [[0x08, 0x09], [0x18, 0x19]]
    }[usize::from(local_row)][usize::from(local_column)];
    (source.tile_index == expected).then_some((local_column, local_row))
}

pub(crate) fn boulder_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "tower" || !matches!(source.metatile_id, 0x32..=0x36) {
        return None;
    }
    let local = match source.tile_index {
        0x04 => (0, 0),
        0x05 => (1, 0),
        0x14 => (0, 1),
        0x15 => (1, 1),
        _ => return None,
    };
    (source.subtile_column % 2 == local.0 && source.subtile_row % 2 == local.1).then_some(local)
}

pub(crate) fn tower_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "tower" {
        return None;
    }

    // `$0b` is the repeated dark board/void field around the authored tower
    // walkways. Crystal marks it impassable, but the drawing is deliberately
    // parallel to the floor; turning that collision into a wall would cover
    // the paths with a forest of striped columns. Pin the complete exact
    // metatile as a presentation plane and let the coverage auditor distinguish
    // intentional flat background from an unsupported object.
    if source.metatile_id == 0x0b
        && matches!(
            (source.subtile_column, source.tile_index),
            (0, 0x1a) | (1..=3, 0x1b)
        )
    {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

    // These exact tower fields are authored horizontal depth/background,
    // despite containing blocked quadrants. They were verified as planes in
    // paired 2D/2.5D tower views; collision must never turn them into walls.
    // Keep the source matrices explicit so similarly numbered object art is
    // not flattened by tile id alone.
    let intentional_plane = match source.metatile_id {
        0x04 => {
            source.subtile_row >= 2
                && source.subtile_column < 2
                && matches!(source.tile_index, 0x41 | 0x31)
        }
        0x06 => {
            source.subtile_row >= 2
                && source.subtile_column >= 2
                && matches!(source.tile_index, 0x41 | 0x31)
        }
        0x07 => matches!(
            (source.subtile_column, source.subtile_row, source.tile_index),
            (0, 0, 0x0a) | (1..=3, 0, 0x0b) | (0, 1..=3, 0x1a) | (1..=3, 1..=3, 0x1b)
        ),
        0x08 => source.subtile_column < 2 && matches!(source.tile_index, 0x41 | 0x31),
        0x0a => source.subtile_column >= 2 && matches!(source.tile_index, 0x41 | 0x31),
        0x0f => matches!(
            (source.subtile_row, source.tile_index),
            (0, 0x0b) | (1..=3, 0x1b)
        ),
        0x1c => matches!(
            (source.subtile_column, source.tile_index),
            (0, 0x16) | (1, 0x24) | (2, 0x16) | (3, 0x06)
        ),
        0x23 => matches!(
            (source.subtile_column, source.tile_index),
            (0, 0x25) | (1, 0x35) | (2, 0x34) | (3, 0x35)
        ),
        0x2b => matches!(
            (source.subtile_column, source.subtile_row, source.tile_index),
            (0, 0, 0x1b)
                | (1, 0, 0x46)
                | (2..=3, 0, 0x02)
                | (0, 1, 0x1b)
                | (1, 1, 0x56)
                | (2..=3, 1, 0x02)
                | (0, 2..=3, 0x1b)
                | (1..=3, 2, 0x0b)
                | (1..=3, 3, 0x1b)
        ),
        0x2c => matches!(
            (source.subtile_column, source.subtile_row, source.tile_index),
            (0..=1, 0..=1, 0x02)
                | (2, 0, 0x47)
                | (3, 0, 0x1a)
                | (2, 1, 0x57)
                | (3, 1, 0x1a)
                | (0..=2, 2, 0x0b)
                | (3, 2, 0x1b)
                | (0..=3, 3, 0x1b)
        ),
        _ => false,
    };
    if intentional_plane {
        return Some(CellShape::PlaneAt { height: 0.0 });
    }

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
    None
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
    fn pewter_boulder_drawing_resolves_one_exact_two_by_two_group() {
        for (tile, row) in [(0x04, 0), (0x05, 0), (0x14, 1), (0x15, 1)] {
            let column = u8::from(tile == 0x05 || tile == 0x15);
            let source = source(0x32, column, row, tile);
            assert_eq!(boulder_local(&source), Some((column, row)));
            assert_eq!(tower_shape(&source), None);
        }
        assert_eq!(boulder_local(&source(0x32, 0, 2, TOWER_FLOOR_TILE)), None);
    }

    #[test]
    fn entrance_statues_are_two_independent_two_by_two_cards() {
        for (column, tile, local) in [
            (0, 0x07, (0, 0)),
            (1, 0x08, (1, 0)),
            (2, 0x08, (0, 0)),
            (3, 0x09, (1, 0)),
        ] {
            let cell = source(0x26, column, 2, tile);
            assert_eq!(statue_local(&cell), Some(local));
            assert_eq!(tower_shape(&cell), None);
        }
        assert_eq!(statue_local(&source(0x26, 0, 3, 0x17)), Some((0, 1)));
        assert_eq!(statue_local(&source(0x26, 3, 3, 0x19)), Some((1, 1)));
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

    #[test]
    fn dark_walkway_background_is_an_explicit_flat_plane_not_a_collision_wall() {
        for row in 0..4 {
            assert_eq!(
                tower_shape(&source(0x0b, 0, row, 0x1a)),
                Some(CellShape::PlaneAt { height: 0.0 })
            );
            for column in 1..4 {
                assert_eq!(
                    tower_shape(&source(0x0b, column, row, 0x1b)),
                    Some(CellShape::PlaneAt { height: 0.0 })
                );
            }
        }
        assert_eq!(tower_shape(&source(0x0b, 0, 0, 0x1b)), None);
    }

    #[test]
    fn verified_depth_fields_are_explicit_planes() {
        for cell in [
            source(0x04, 0, 2, 0x41),
            source(0x06, 3, 3, 0x31),
            source(0x07, 0, 0, 0x0a),
            source(0x08, 1, 3, 0x31),
            source(0x0a, 2, 1, 0x41),
            source(0x0f, 3, 3, 0x1b),
            source(0x1c, 1, 2, 0x24),
            source(0x23, 2, 1, 0x34),
            source(0x2b, 1, 1, 0x56),
            source(0x2c, 2, 1, 0x57),
        ] {
            assert_eq!(tower_shape(&cell), Some(CellShape::PlaneAt { height: 0.0 }));
        }
        assert_eq!(tower_shape(&source(0x08, 2, 0, TOWER_FLOOR_TILE)), None);
        assert_eq!(tower_shape(&source(0x23, 2, 0, 0x35)), None);
    }
}
