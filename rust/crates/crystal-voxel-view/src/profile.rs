//! Clean-room, presentation-only shape profiles for known Crystal artwork.
//!
//! Profiles are keyed only by stable visual source identity. They deliberately
//! do not inspect movement permissions or collision classes: unknown artwork
//! remains a flat tile and cannot turn into an invented wall.

use crystal_render_api::{VisualTileSource, VisualWorldFrame};

pub const GROUND_HEIGHT: f32 = 0.0;
pub const WATER_HEIGHT: f32 = -2.0;
pub const COMPACT_BUILDING_HEIGHT: f32 = 16.0;
pub const LARGE_BUILDING_HEIGHT: f32 = 32.0;
pub const MAX_PROFILE_HEIGHT: f32 = LARGE_BUILDING_HEIGHT;
pub const MIN_PROFILE_HEIGHT: f32 = WATER_HEIGHT;
pub const SOURCE_TILE_HEIGHT: f32 = 8.0;

const JOHTO_TILESET: &str = "johto";
const KANTO_TILESET: &str = "kanto";
const CAVE_TILESET: &str = "cave";
const LIGHTHOUSE_TILESET: &str = "lighthouse";
const AUTHORED_WATER_TILESETS: [&str; 3] = ["johto_modern", "kanto", "cave"];
const AUTHORED_WATER_TILE_INDEX: u16 = 0x14;
const KANTO_TREE_TILE_INDICES: [u16; 8] = [0x2d, 0x2e, 0x3d, 0x3e, 0x40, 0x41, 0x50, 0x51];
// Tree metatiles carry $2c in their open cells; using that exact background
// keeps the mask palette-aware and guarantees the sample travels with the art.
const KANTO_GROUND_TILE_INDEX: u16 = 0x2c;
const CAVE_GROUND_TILE_INDEX: u16 = 0x01;
const CAVE_ROCK_TOP_TILE_INDICES: [u16; 2] = [0x0c, 0x0d];
const CAVE_ROCK_BOTTOM_TILE_INDICES: [u16; 2] = [0x1c, 0x1d];
const LIGHTHOUSE_WALL_METATILES: [u16; 3] = [0x3c, 0x3d, 0x3e];
const LIGHTHOUSE_FLOOR_TILE_INDEX: u16 = 0x2e;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolidKind {
    Building,
    Tree,
    Prop,
    Bank,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellShape {
    Flat,
    Water,
    RaisedTop {
        height: f32,
        solid: SolidKind,
    },
    /// One native-size source band folded onto a shared front plane.
    FacadeBand {
        plane_subtile_row: u8,
        band_from_top: u8,
        band_count: u8,
        ground_tile_index: u16,
        solid: SolidKind,
    },
}

impl CellShape {
    pub fn surface_height(self, tile_height: f32) -> f32 {
        let source_height = match self {
            Self::Flat | Self::FacadeBand { .. } => GROUND_HEIGHT,
            Self::Water => WATER_HEIGHT,
            Self::RaisedTop { height, .. } => height,
        };
        source_height * tile_height / SOURCE_TILE_HEIGHT
    }

    pub fn solid_kind(self) -> SolidKind {
        match self {
            Self::RaisedTop { solid, .. } | Self::FacadeBand { solid, .. } => solid,
            Self::Flat => SolidKind::Building,
            Self::Water => SolidKind::Bank,
        }
    }
}

/// Whether the frame can enter the optional renderer.
///
/// Every validated visual source has a safe flat baseline. Authored rules add
/// relief only for identities understood by this crate, so enabling 2.5D can
/// be inspected across the complete map catalog without inventing collision
/// geometry for unprofiled art.
pub fn supports_frame_profile(frame: &VisualWorldFrame) -> bool {
    !frame.map_id.is_empty() && !frame.tiles.is_empty()
}

pub fn shape_for_source(source: &VisualTileSource) -> CellShape {
    // Cave's $0c/$0d/$1c/$1d cells are one exact two-row rock drawing. The
    // metatile catalog places that same authored 16x16 prop at multiple
    // quadrants. Fold each source row once at its local two-row origin; never
    // promote the surrounding cave collision contour into a generic wall.
    if source.tileset_id.as_ref() == CAVE_TILESET {
        if CAVE_ROCK_TOP_TILE_INDICES.contains(&source.tile_index) {
            return CellShape::FacadeBand {
                plane_subtile_row: source.subtile_row,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            };
        }
        if CAVE_ROCK_BOTTOM_TILE_INDICES.contains(&source.tile_index) {
            return CellShape::FacadeBand {
                plane_subtile_row: source.subtile_row.saturating_sub(1),
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            };
        }
    }

    // The lighthouse perimeter blocks are complete four-row wall drawings in
    // Crystal's lighthouse metatile catalog. Fold those authored rows once
    // onto their south edge. Interior floor, stairs, furniture, and the 6F
    // chamber remain flat until they receive their own explicit profiles.
    if source.tileset_id.as_ref() == LIGHTHOUSE_TILESET
        && LIGHTHOUSE_WALL_METATILES.contains(&source.metatile_id)
    {
        return CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: LIGHTHOUSE_FLOOR_TILE_INDEX,
            solid: SolidKind::Bank,
        };
    }

    // These three Crystal tilesets use tile $14 as their animated open-water
    // cell, including inside shoreline metatiles. Matching the source cell
    // rather than the whole metatile recesses water without lowering banks,
    // rocks, or other artwork in that same 4x4 block.
    if AUTHORED_WATER_TILESETS.contains(&source.tileset_id.as_ref())
        && source.tile_index == AUTHORED_WATER_TILE_INDEX
    {
        return CellShape::Water;
    }

    // Kanto's tree drawings are composed from stable 2x2 source-cell sets.
    // Fold each set once at its own south edge instead of treating the whole
    // metatile as a raised block or repeating a cell down a wall.
    if source.tileset_id.as_ref() == KANTO_TILESET
        && KANTO_TREE_TILE_INDICES.contains(&source.tile_index)
    {
        let group_row = source.subtile_row / 2;
        return CellShape::FacadeBand {
            plane_subtile_row: (group_row + 1) * 2,
            band_from_top: source.subtile_row % 2,
            band_count: 2,
            ground_tile_index: KANTO_GROUND_TILE_INDEX,
            solid: SolidKind::Tree,
        };
    }

    if source.tileset_id.as_ref() != JOHTO_TILESET {
        return CellShape::Flat;
    }

    match source.metatile_id {
        // The compact house uses two source rows as a roof surface. Its two
        // lower rows become separate 8px facade bands on one shared seam.
        0x14 | 0x15 if source.subtile_row < 2 => CellShape::RaisedTop {
            height: COMPACT_BUILDING_HEIGHT,
            solid: SolidKind::Building,
        },
        0x14 | 0x15 => CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x06,
            solid: SolidKind::Building,
        },

        // Upper blocks in New Bark's large house and lab are top-facing roof
        // art. The matching lower block row is folded into four native bands.
        0x18 | 0x19 | 0x1f => CellShape::RaisedTop {
            height: LARGE_BUILDING_HEIGHT,
            solid: SolidKind::Building,
        },
        0x16 | 0x1c | 0x1e | 0x77 => CellShape::FacadeBand {
            plane_subtile_row: 0,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: 0x06,
            solid: SolidKind::Building,
        },

        // Connected tree artwork is upright art, never a face-up slab. Dense
        // 0x05 foliage is one four-band hedge card. The partial edge blocks
        // contain independent two-band groups and flat background halves.
        0x05 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        0x62 if source.subtile_column >= 2 && source.subtile_row < 2 => CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        0x62 if source.subtile_column >= 2 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        0x65 if source.subtile_row >= 2 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },

        // Two independently used 2x2 sign patterns. Like building facades,
        // each source row is folded exactly once at the group's south edge.
        0x45 if source.subtile_column < 2 && source.subtile_row < 2 => CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: 0x06,
            solid: SolidKind::Prop,
        },
        0x47 if source.subtile_column >= 2 && source.subtile_row >= 2 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Prop,
        },

        // Authored shallow water presentation; no gameplay collision is read.
        0x54 | 0x58 => CellShape::Water,
        _ => CellShape::Flat,
    }
}

/// Presentation footing never puts actors on roofs, trees, sign cards, or the
/// recessed water surface. Gameplay remains two-dimensional and authoritative.
pub fn support_height(_source: &VisualTileSource, _tile_height: f32) -> f32 {
    GROUND_HEIGHT
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::prelude::{Handle, Image};
    use crystal_render_api::VisualTile;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        source_for_tileset(JOHTO_TILESET, metatile_id, column, row, 0)
    }

    fn source_for_tileset(
        tileset_id: &str,
        metatile_id: u16,
        column: u8,
        row: u8,
        tile_index: u16,
    ) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset_id),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index,
        }
    }

    fn profile_frame(map_id: &str, tileset_id: &str) -> VisualWorldFrame {
        let mut tile_source = source(0x01, 0, 0);
        tile_source.tileset_id = Arc::from(tileset_id);
        VisualWorldFrame {
            map_id: Arc::from(map_id),
            tiles: vec![VisualTile {
                column: 0,
                row: 0,
                source: tile_source,
                texture: Handle::<Image>::weak_from_u128(1),
                priority: false,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn every_named_map_has_a_safe_flat_profile_baseline() {
        assert!(supports_frame_profile(&profile_frame(
            "NewBarkTown",
            JOHTO_TILESET
        )));
        assert!(supports_frame_profile(&profile_frame(
            "CherrygroveCity",
            JOHTO_TILESET,
        )));
        assert!(supports_frame_profile(&profile_frame(
            "CeladonCity",
            "kanto",
        )));
        assert!(!supports_frame_profile(&profile_frame("", JOHTO_TILESET)));
    }

    #[test]
    fn compact_house_facade_rows_share_one_plane() {
        let upper = shape_for_source(&source(0x14, 0, 2));
        let lower = shape_for_source(&source(0x14, 0, 3));
        assert_eq!(
            upper,
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Building,
            }
        );
        assert_eq!(
            lower,
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Building,
            }
        );
    }

    #[test]
    fn unknown_art_is_flat_even_when_its_tile_index_looks_familiar() {
        let mut unknown = source(0xbeef, 0, 0);
        unknown.tile_index = 0x1e;
        assert_eq!(shape_for_source(&unknown), CellShape::Flat);
    }

    #[test]
    fn authored_water_identity_recesses_only_the_water_cell() {
        for tileset in AUTHORED_WATER_TILESETS {
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x43, 1, 2, 0x14)),
                CellShape::Water
            );
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x43, 1, 2, 0x15)),
                CellShape::Flat
            );
        }
        assert_eq!(
            shape_for_source(&source_for_tileset("house", 0x14, 1, 2, 0x14)),
            CellShape::Flat
        );
    }

    #[test]
    fn kanto_tree_cells_form_independent_two_band_cards() {
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x32, 0, 0, 0x40)),
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
                solid: SolidKind::Tree,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x32, 0, 3, 0x51)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
                solid: SolidKind::Tree,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset("johto_modern", 0x32, 0, 0, 0x40)),
            CellShape::Flat
        );
    }

    #[test]
    fn cave_rock_rows_fold_once_without_raising_support() {
        let top = source_for_tileset(CAVE_TILESET, 0x19, 0, 0, 0x0c);
        let bottom = source_for_tileset(CAVE_TILESET, 0x19, 0, 1, 0x1c);

        assert_eq!(
            shape_for_source(&top),
            CellShape::FacadeBand {
                plane_subtile_row: 0,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            }
        );
        assert_eq!(
            shape_for_source(&bottom),
            CellShape::FacadeBand {
                plane_subtile_row: 0,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            }
        );
        assert_eq!(support_height(&top, SOURCE_TILE_HEIGHT), GROUND_HEIGHT);
        assert_eq!(support_height(&bottom, SOURCE_TILE_HEIGHT), GROUND_HEIGHT);
    }

    #[test]
    fn lighthouse_perimeter_uses_native_wall_bands_only() {
        for metatile_id in LIGHTHOUSE_WALL_METATILES {
            for row in 0..4 {
                assert_eq!(
                    shape_for_source(&source_for_tileset(
                        LIGHTHOUSE_TILESET,
                        metatile_id,
                        1,
                        row,
                        0x5e,
                    )),
                    CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row,
                        band_count: 4,
                        ground_tile_index: LIGHTHOUSE_FLOOR_TILE_INDEX,
                        solid: SolidKind::Bank,
                    }
                );
            }
        }

        assert_eq!(
            shape_for_source(&source_for_tileset(
                LIGHTHOUSE_TILESET,
                0x27,
                0,
                0,
                LIGHTHOUSE_FLOOR_TILE_INDEX,
            )),
            CellShape::Flat
        );
    }

    #[test]
    fn building_and_tree_art_do_not_raise_actor_support() {
        assert_eq!(support_height(&source(0x18, 0, 0), 32.0), GROUND_HEIGHT);
        assert_eq!(support_height(&source(0x05, 0, 0), 32.0), GROUND_HEIGHT);
        assert_eq!(support_height(&source(0x54, 0, 0), 32.0), GROUND_HEIGHT);
    }
}
