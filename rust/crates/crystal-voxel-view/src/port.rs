//! Exact presentation roles for Crystal's Vermilion Port tileset.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, GROUND_HEIGHT, SolidKind};

pub(crate) const SHIP_HEIGHT: f32 = 28.0;

pub(crate) fn port_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "port" {
        return None;
    }
    if source.tile_index == 0x14 {
        return Some(CellShape::Water);
    }
    // Cerulean Gym's Kanto water room reuses compact floating barrel/pot
    // drawings from this atlas. They are shallow props, never ship hulls.
    if matches!(source.metatile_id, 0x2e..=0x31 | 0x36..=0x39)
        && matches!(source.tile_index, 0x0a | 0x0b | 0x22)
    {
        return Some(CellShape::Relief {
            height: 3.0,
            ground_tile_index: 0x14,
            base_height: GROUND_HEIGHT,
        });
    }
    if (0x18..=0x1f).contains(&source.metatile_id) {
        return Some(CellShape::RaisedTop {
            height: SHIP_HEIGHT,
            solid: SolidKind::Ship,
        });
    }
    // Each repeated blue barrier is one 2x2 face-on drawing. Stand that exact
    // drawing at the south edge of its own cell with zero geometric depth,
    // matching the flat-card tree treatment. It must never become pixel
    // relief or a volume: both make the repeated art read as barrels/cages.
    if matches!(source.tile_index, 0x01 | 0x02 | 0x11 | 0x12) {
        let group_row = source.subtile_row / 2;
        return Some(CellShape::FacadeBand {
            plane_subtile_row: (group_row + 1) * 2,
            band_from_top: source.subtile_row % 2,
            band_count: 2,
            ground_tile_index: 0x14,
            solid: SolidKind::FlatCard,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("port"),
            metatile_id,
            subtile_column: 0,
            subtile_row: 0,
            tile_index,
        }
    }

    #[test]
    fn ship_blocks_preserve_water_holes_instead_of_becoming_a_rectangle() {
        assert_eq!(port_shape(&source(0x18, 0x14)), Some(CellShape::Water));
        assert_eq!(
            port_shape(&source(0x18, 0x2b)),
            Some(CellShape::RaisedTop {
                height: SHIP_HEIGHT,
                solid: SolidKind::Ship,
            })
        );
    }

    #[test]
    fn gangway_and_quay_use_the_safe_flat_baseline() {
        assert_eq!(port_shape(&source(0x11, 0x31)), None);
        assert_eq!(port_shape(&source(0x04, 0x05)), None);
    }

    #[test]
    fn repeated_quay_barrier_is_a_zero_depth_two_band_card() {
        assert_eq!(
            port_shape(&source(0x01, 0x01)),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: 0x14,
                solid: SolidKind::FlatCard,
            })
        );
        let mut bottom = source(0x01, 0x12);
        bottom.subtile_row = 1;
        assert_eq!(
            port_shape(&bottom),
            Some(CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x14,
                solid: SolidKind::FlatCard,
            })
        );
    }

    #[test]
    fn cerulean_water_room_barrels_are_small_relief_not_ship_geometry() {
        assert!(matches!(
            port_shape(&source(0x30, 0x0a)),
            Some(CellShape::Relief {
                ground_tile_index: 0x14,
                ..
            })
        ));
    }
}
