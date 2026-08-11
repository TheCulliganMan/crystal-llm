//! Authored animated-flower presentation identities.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const FLOWER_GROUNDS: [(&str, u16); 5] = [
    ("johto", 0x05),
    ("johto_modern", 0x05),
    ("forest", 0x05),
    ("park", 0x01),
    ("kanto", 0x2c),
];

pub(crate) fn flower_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tile_index != 0x03 {
        return None;
    }
    let ground = FLOWER_GROUNDS.iter().find_map(|(tileset, ground)| {
        (source.tileset_id.as_ref() == *tileset).then_some(*ground)
    })?;
    Some(CellShape::Cutout {
        ground_tile_index: ground,
        solid: SolidKind::Flower,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn every_flower_animation_tileset_uses_its_authored_ground() {
        for (tileset, ground) in FLOWER_GROUNDS {
            let source = VisualTileSource {
                tileset_id: Arc::from(tileset),
                metatile_id: 0,
                subtile_column: 0,
                subtile_row: 0,
                tile_index: 0x03,
            };
            assert_eq!(
                flower_shape(&source),
                Some(CellShape::Cutout {
                    ground_tile_index: ground,
                    solid: SolidKind::Flower,
                })
            );
        }
    }

    #[test]
    fn non_animated_tileset_reusing_slot_three_stays_unprofiled() {
        let source = VisualTileSource {
            tileset_id: Arc::from("house"),
            metatile_id: 0,
            subtile_column: 0,
            subtile_row: 0,
            tile_index: 0x03,
        };
        assert_eq!(flower_shape(&source), None);
    }
}
