//! Flat upright hedge drawings for Celadon Gym.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const GROUND_TILE: u16 = 0x57;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HedgeGroup {
    pub local_column: u8,
    pub local_row: u8,
    pub width: usize,
    pub height: usize,
}

pub(crate) fn hedge_group(source: &VisualTileSource) -> Option<HedgeGroup> {
    if source.tileset_id.as_ref() != "train_station" {
        return None;
    }
    // $1a repeats four exact [4c 4d; 5c 5d] hedge crowns. $1b carries
    // the same two drawings only in its lower half. The blue $20-$23 art is
    // the animated flowerbed family and deliberately remains on that path.
    let (local_column, local_row) = match source.metatile_id {
        0x1a => (source.subtile_column % 2, source.subtile_row % 2),
        0x1b if source.subtile_row >= 2 => (source.subtile_column % 2, source.subtile_row - 2),
        _ => return None,
    };
    Some(HedgeGroup {
        local_column,
        local_row,
        width: 2,
        height: 2,
    })
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "CeladonGym" {
        return None;
    }
    let group = hedge_group(source)?;
    Some(CellShape::FacadeBand {
        plane_subtile_row: source.subtile_row - group.local_row + group.height as u8,
        band_from_top: group.local_row,
        band_count: group.height as u8,
        ground_tile_index: GROUND_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("train_station"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn exact_hedge_drawings_are_grouped_but_flowerbeds_are_not() {
        assert_eq!(hedge_group(&source(0x1a, 1, 3)).unwrap().height, 2);
        assert_eq!(hedge_group(&source(0x1b, 3, 3)).unwrap().local_row, 1);
        assert!(hedge_group(&source(0x1b, 0, 1)).is_none());
        assert!(hedge_group(&source(0x20, 0, 0)).is_none());
    }

    #[test]
    fn hedge_cards_are_scoped_to_celadon_gym() {
        assert!(matches!(
            shape("CeladonGym", &source(0x1a, 0, 0)),
            Some(CellShape::FacadeBand {
                solid: SolidKind::FlatCard,
                band_count: 2,
                ..
            })
        ));
        assert_eq!(shape("GoldenrodStation", &source(0x1a, 0, 0)), None);
    }
}
