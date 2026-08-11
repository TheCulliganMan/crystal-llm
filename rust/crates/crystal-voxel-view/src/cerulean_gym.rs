//! Complete entrance-statue cards for Cerulean Gym.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const DECK_TILE: u16 = 0x05;

pub(crate) fn statue_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    (source.tileset_id.as_ref() == "port"
        && source.metatile_id == 0x32
        && source.subtile_column < 2)
        .then_some((source.subtile_column, source.subtile_row))
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id != "CeruleanGym" {
        return None;
    }
    let (_, row) = statue_local(source)?;
    Some(CellShape::FacadeBand {
        plane_subtile_row: 4,
        band_from_top: row,
        band_count: 4,
        ground_tile_index: DECK_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("port"),
            metatile_id: 0x32,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn statue_keeps_all_four_native_rows_on_one_flat_card() {
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(statue_local(&source(column, row)), Some((column, row)));
                assert!(matches!(
                    shape("CeruleanGym", &source(column, row)),
                    Some(CellShape::FacadeBand {
                        solid: SolidKind::FlatCard,
                        band_count: 4,
                        ..
                    })
                ));
            }
        }
    }

    #[test]
    fn shared_port_atlas_is_not_promoted_elsewhere() {
        assert_eq!(shape("OlivinePort", &source(0, 0)), None);
    }
}
