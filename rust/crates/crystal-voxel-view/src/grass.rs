//! Authored tall-grass presentation identities.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

#[derive(Clone, Copy)]
struct GrassSource {
    tileset: &'static str,
    metatile: u16,
    tile: u16,
    ground: u16,
}

const GRASS_SOURCES: [GrassSource; 8] = [
    GrassSource::new("johto", 0x03, 0x04, 0x05),
    GrassSource::new("johto_modern", 0x03, 0x04, 0x05),
    GrassSource::new("battle_tower_outside", 0x03, 0x04, 0x05),
    GrassSource::new("park", 0x03, 0x04, 0x01),
    GrassSource::new("cave", 0x03, 0x04, 0x01),
    GrassSource::new("dark_cave", 0x03, 0x04, 0x01),
    GrassSource::new("kanto", 0x0b, 0x52, 0x2c),
    GrassSource::new("unused_johto", 0x03, 0x04, 0x05),
];

const LONG_GRASS_SOURCES: [GrassSource; 4] = [
    // The complete repeated 16×16 long-grass drawing has upper $3a/$3b
    // blades and lower $4a/$4b blades.  Keeping only the upper source row
    // made National Park tufts visibly stop halfway down.
    GrassSource::new("park", 0x13, 0x3a, 0x01),
    GrassSource::new("park", 0x13, 0x3b, 0x01),
    GrassSource::new("park", 0x13, 0x4a, 0x01),
    GrassSource::new("park", 0x13, 0x4b, 0x01),
];

impl GrassSource {
    const fn new(tileset: &'static str, metatile: u16, tile: u16, ground: u16) -> Self {
        Self {
            tileset,
            metatile,
            tile,
            ground,
        }
    }
}

pub(crate) fn grass_shape(source: &VisualTileSource) -> Option<CellShape> {
    let profile = GRASS_SOURCES
        .iter()
        .chain(LONG_GRASS_SOURCES.iter())
        .find(|profile| {
            source.tileset_id.as_ref() == profile.tileset
                && source.metatile_id == profile.metatile
                && source.tile_index == profile.tile
        })?;
    Some(CellShape::Cutout {
        ground_tile_index: profile.ground,
        solid: SolidKind::Grass,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn every_crystal_tall_grass_tileset_uses_its_authored_underlay() {
        for profile in GRASS_SOURCES.into_iter().chain(LONG_GRASS_SOURCES) {
            let source = VisualTileSource {
                tileset_id: Arc::from(profile.tileset),
                metatile_id: profile.metatile,
                subtile_column: 0,
                subtile_row: 0,
                tile_index: profile.tile,
            };
            assert_eq!(
                grass_shape(&source),
                Some(CellShape::Cutout {
                    ground_tile_index: profile.ground,
                    solid: SolidKind::Grass,
                })
            );
        }
    }

    #[test]
    fn decorative_reuse_of_a_grass_tile_does_not_sprout_geometry() {
        let source = VisualTileSource {
            tileset_id: Arc::from("johto"),
            metatile_id: 0x04,
            subtile_column: 1,
            subtile_row: 1,
            tile_index: 0x04,
        };
        assert_eq!(grass_shape(&source), None);
    }
}
