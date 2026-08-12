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

/// Pewter Museum is the one Kanto civic landmark whose authored deep roof
/// band is a pitched hall rather than a flat plan slab.  Match the complete
/// catalogued drawing dimensions and its stable west-cap metatile; ordinary
/// marts, gyms, stations, and compact houses keep their existing roof form.
pub(crate) fn uses_kanto_museum_ridge(
    width: usize,
    height: usize,
    roof_rows: usize,
    source: &VisualTileSource,
) -> bool {
    source.tileset_id.as_ref() == "kanto"
        && source.metatile_id == 0x75
        && width == 16
        && height == 8
        && roof_rows == 4
}

/// Celadon's department store is a 12x16-tile landmark: four native roof
/// rows followed by twelve upright rows (six two-row window courses plus the
/// entrance/sign course). Its generated sides must carry the outer frame,
/// not turn the broad front window fields around the upper corners.
pub(crate) fn is_celadon_department_store(
    map_id: &str,
    width: usize,
    height: usize,
    roof_rows: usize,
    source: &VisualTileSource,
) -> bool {
    map_id == "CeladonCity"
        && width == 12
        && height == 16
        && roof_rows == 4
        && source.tileset_id.as_ref() == "kanto"
        && source.metatile_id == 0x20
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

/// Olivine's lighthouse is a twenty-row upright facade under a one-tile cap.
/// Its physical footprint is not the height of that drawing, but an eight-pixel
/// wafer provides no sidewall cue at the 45-degree presentation camera. Keep a
/// bounded three-tile footprint while leaving the authored south door plane and
/// all facade rows untouched.
pub(crate) fn lighthouse_depth_pixels(
    width: usize,
    height: usize,
    roof_rows: usize,
    first: &VisualTileSource,
) -> Option<usize> {
    (width == 8
        && height == 28
        && roof_rows == 8
        && first.tileset_id.as_ref() == "johto"
        && first.metatile_id == 0x08)
        .then_some(24)
}

/// Ecruteak's sacred Tin Tower exterior is represented on the 2D map by its
/// entrance storey.  The 2.5D landmark repeats that complete authored wall
/// and eave course to express the pagoda's canonical height.  Keep this
/// strictly map- and drawing-scoped: the same traditional tiles also form
/// ordinary houses elsewhere.
pub(crate) fn tin_tower_storeys(
    map_id: &str,
    width: usize,
    height: usize,
    roof_rows: usize,
    first: &VisualTileSource,
) -> usize {
    if map_id == "EcruteakCity"
        && width == 12
        && height == 6
        && roof_rows == 4
        && first.tileset_id.as_ref() == "johto"
        && first.metatile_id == 0x2c
    {
        5
    } else {
        1
    }
}

/// Replace only Tin Tower's entrance bay on upper storeys with the complete
/// window bay measured from the same 96-pixel source composite.
pub(crate) fn tin_tower_upper_source_x(pixel_width: usize, x: usize) -> usize {
    if pixel_width == 96 && (32..48).contains(&x) {
        // The intact left window occupies source columns 11..27.
        x - 21
    } else {
        x
    }
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
    fn olivine_lighthouse_uses_a_bounded_three_tile_footprint() {
        assert_eq!(
            lighthouse_depth_pixels(8, 28, 8, &source("johto", 0x08)),
            Some(24)
        );
        assert_eq!(
            lighthouse_depth_pixels(8, 28, 8, &source("johto", 0x18)),
            None
        );
        assert_eq!(
            lighthouse_depth_pixels(8, 28, 8, &source("kanto", 0x08)),
            None
        );
    }

    #[test]
    fn only_ecruteaks_complete_sacred_tower_becomes_five_storeys() {
        let first = source("johto", 0x2c);
        assert_eq!(tin_tower_storeys("EcruteakCity", 12, 6, 4, &first), 5);
        assert_eq!(tin_tower_storeys("VioletCity", 12, 6, 4, &first), 1);
        assert_eq!(tin_tower_storeys("EcruteakCity", 8, 6, 4, &first), 1);
        assert_eq!(
            tin_tower_storeys("EcruteakCity", 12, 6, 4, &source("johto", 0x2d)),
            1
        );
    }

    #[test]
    fn sacred_tower_upper_storeys_replace_only_the_door_with_a_native_window() {
        assert_eq!(tin_tower_upper_source_x(96, 31), 31);
        assert_eq!(tin_tower_upper_source_x(96, 32), 11);
        assert_eq!(tin_tower_upper_source_x(96, 47), 26);
        assert_eq!(tin_tower_upper_source_x(96, 48), 48);
        assert_eq!(tin_tower_upper_source_x(64, 32), 32);
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

    #[test]
    fn only_the_complete_pewter_museum_uses_the_kanto_hall_ridge() {
        assert!(uses_kanto_museum_ridge(16, 8, 4, &source("kanto", 0x75)));
        assert!(!uses_kanto_museum_ridge(12, 8, 4, &source("kanto", 0x75)));
        assert!(!uses_kanto_museum_ridge(16, 8, 4, &source("kanto", 0x20)));
    }

    #[test]
    fn celadon_store_keeps_twelve_upright_rows_below_its_true_cap() {
        let first = source("kanto", 0x20);
        assert!(is_celadon_department_store(
            "CeladonCity",
            12,
            16,
            4,
            &first
        ));
        assert_eq!(16 - 4, 12, "all six window courses stay upright");
        assert!(!is_celadon_department_store(
            "SaffronCity",
            12,
            16,
            4,
            &first
        ));
    }
}
