//! Small authored overrides for building drawings whose sprite height and
//! physical footprint must not be conflated.

use crystal_render_api::VisualTileSource;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RoofStyle {
    pub depth_pixels: usize,
    pub slab_pixels: usize,
}

/// Ordinary house drawings need a visible half-tile eave at this projection.
/// The body closes four source pixels inward on both sides while the roof and
/// fascia keep their complete authored width. A matching soffit closes the
/// underside, so this produces an overhang rather than a gap.
pub(crate) const HOUSE_WALL_INSET_PIXELS: usize = 4;

/// Only Crystal's traditional Johto drawing encodes a center-ridge roof.
/// Blue striped city roofs are already top-facing art and remain level.
pub(crate) fn uses_center_ridge_roof(
    _height: usize,
    _roof_rows: usize,
    first: &VisualTileSource,
) -> bool {
    first.tileset_id.as_ref() == "johto" && matches!(first.metatile_id, 0x2a | 0x2c | 0x2d)
}

/// Burned Tower's exterior is an 8x8-tile drawing: four roof rows over four
/// facade rows. Its matched footprint is the four roof rows (32 pixels), not
/// the generic expanded depth used to give ordinary compact houses body.
pub(crate) fn burned_tower_roof_style(
    width: usize,
    height: usize,
    roof_rows: usize,
    first: &VisualTileSource,
) -> Option<RoofStyle> {
    (width == 8
        && height == 8
        && roof_rows == 4
        && first.tileset_id.as_ref() == "johto"
        && first.metatile_id == 0x20)
        .then_some(RoofStyle {
            depth_pixels: 32,
            slab_pixels: 3,
        })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tileset: &str, metatile_id: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset),
            metatile_id,
            subtile_column: 0,
            subtile_row: 0,
            tile_index: 0,
        }
    }

    #[test]
    fn burned_tower_uses_its_exact_matched_footprint() {
        assert_eq!(
            burned_tower_roof_style(8, 8, 4, &source("johto", 0x20)),
            Some(RoofStyle {
                depth_pixels: 32,
                slab_pixels: 3,
            })
        );
    }

    #[test]
    fn similar_dimensions_do_not_claim_an_unrelated_building() {
        assert_eq!(
            burned_tower_roof_style(8, 8, 4, &source("johto", 0x18)),
            None
        );
        assert_eq!(
            burned_tower_roof_style(8, 8, 4, &source("kanto", 0x20)),
            None
        );
    }

    #[test]
    fn only_traditional_johto_art_uses_a_center_ridge() {
        assert!(!uses_center_ridge_roof(4, 2, &source("johto_modern", 0x12)));
        assert!(!uses_center_ridge_roof(8, 4, &source("kanto", 0x20)));
        assert!(uses_center_ridge_roof(6, 2, &source("johto", 0x2c)));
        assert!(!uses_center_ridge_roof(
            16,
            4,
            &source("johto_modern", 0x18)
        ));
        assert!(!uses_center_ridge_roof(
            12,
            2,
            &source("johto_modern", 0x25)
        ));
        assert!(!uses_center_ridge_roof(8, 4, &source("johto", 0x20)));
    }
}
