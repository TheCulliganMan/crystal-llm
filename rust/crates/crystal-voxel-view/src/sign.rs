//! Reusable outdoor sign profiles derived from each Crystal metatile atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) fn sign_shape(source: &VisualTileSource) -> Option<CellShape> {
    let (plane_subtile_row, band_from_top, ground_tile_index) = match source.tileset_id.as_ref() {
        "johto" => match source.metatile_id {
            // Town-route signboards share $18/$19 over the post pair $17.
            // These appear in many maps as mixed scenery blocks, not only in
            // the dedicated $45/$47 sign containers below.
            0x17 | 0x1b if matches!(source.tile_index, 0x18 | 0x19) => (4, 0, 0x07),
            0x17 | 0x1b if source.tile_index == 0x17 => (4, 1, 0x07),
            0x45 if source.subtile_column < 2 && source.subtile_row < 2 => {
                (2, source.subtile_row, 0x06)
            }
            0x47 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x05)
            }
            0x78 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x06)
            }
            _ => return None,
        },
        "johto_modern" => match source.metatile_id {
            0x1b | 0x33 if matches!(source.tile_index, 0x18 | 0x19) => (4, 0, 0x07),
            0x1b | 0x33 if source.tile_index == 0x17 => (4, 1, 0x07),
            0x45 if source.subtile_column < 2 && source.subtile_row < 2 => {
                (2, source.subtile_row, 0x06)
            }
            0x3c if source.subtile_column < 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x05)
            }
            0x3d | 0x65 if source.subtile_column < 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x06)
            }
            0x77 if source.subtile_column < 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x2f)
            }
            0x47 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x05)
            }
            0x78 if source.subtile_column >= 2 && source.subtile_row >= 2 => {
                (4, source.subtile_row - 2, 0x06)
            }
            _ => return None,
        },
        "battle_tower_outside"
            if source.metatile_id == 0x21
                && source.subtile_column < 2
                && source.subtile_row < 2 =>
        {
            (2, source.subtile_row, 0x06)
        }
        "kanto" if matches!(source.tile_index, 0x46 | 0x47 | 0x56 | 0x57) => {
            (4, source.subtile_row.saturating_sub(2), 0x39)
        }
        // Park signs appear in two placement variants, but keep the same
        // $45/$46 signboard and $55/$56 post artwork. Their source tile
        // identity is stable even where the surrounding block is mixed.
        "park"
            if matches!(source.metatile_id, 0x15 | 0x17)
                && matches!(source.tile_index, 0x45 | 0x46) =>
        {
            (source.subtile_row + 2, 0, 0x00)
        }
        "park"
            if matches!(source.metatile_id, 0x15 | 0x17)
                && matches!(source.tile_index, 0x55 | 0x56) =>
        {
            (source.subtile_row, 1, 0x00)
        }
        // Ilex Forest's narrow reading sign is the exact $01/$06 over
        // $15/$16 two-by-two drawing on ordinary forest floor.
        "forest" if source.metatile_id == 0x13 && matches!(source.tile_index, 0x01 | 0x06) => {
            (source.subtile_row + 2, 0, 0x05)
        }
        "forest" if source.metatile_id == 0x13 && matches!(source.tile_index, 0x15 | 0x16) => {
            (source.subtile_row, 1, 0x05)
        }
        // Radio Tower directory placards use the same two-row façade grammar
        // as outdoor signs, but attach to the interior's $01 floor sample.
        "radio_tower" if source.metatile_id == 0x02 && matches!(source.tile_index, 0x03 | 0x04) => {
            (2, 0, 0x01)
        }
        "radio_tower" if source.metatile_id == 0x02 && matches!(source.tile_index, 0x13 | 0x14) => {
            (2, 1, 0x01)
        }
        _ => return None,
    };
    Some(CellShape::FacadeBand {
        plane_subtile_row,
        band_from_top,
        band_count: 2,
        ground_tile_index,
        solid: SolidKind::Prop,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tileset: &str, metatile: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn every_atlas_sign_container_folds_its_complete_two_by_two_drawing() {
        for (tileset, metatile, column, ground) in [
            ("johto", 0x45, 0, 0x06),
            ("johto", 0x47, 2, 0x05),
            ("johto", 0x78, 2, 0x06),
            ("johto_modern", 0x3c, 0, 0x05),
            ("johto_modern", 0x3d, 0, 0x06),
            ("johto_modern", 0x45, 0, 0x06),
            ("johto_modern", 0x47, 2, 0x05),
            ("johto_modern", 0x65, 0, 0x06),
            ("johto_modern", 0x77, 0, 0x2f),
            ("johto_modern", 0x78, 2, 0x06),
            ("battle_tower_outside", 0x21, 0, 0x06),
        ] {
            let top_row = if metatile == 0x45 || tileset == "battle_tower_outside" {
                0
            } else {
                2
            };
            for local_row in 0..2 {
                assert_eq!(
                    sign_shape(&source(
                        tileset,
                        metatile,
                        column,
                        top_row + local_row,
                        if local_row == 0 { 0x4e } else { 0x5e },
                    )),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: if top_row == 0 { 2 } else { 4 },
                        band_from_top: local_row,
                        band_count: 2,
                        ground_tile_index: ground,
                        solid: SolidKind::Prop,
                    })
                );
            }
        }
    }

    #[test]
    fn mixed_scenery_and_public_signboards_are_not_left_flat() {
        for source in [
            source("johto", 0x17, 0, 2, 0x18),
            source("johto", 0x1b, 0, 3, 0x17),
            source("johto_modern", 0x33, 0, 2, 0x19),
            source("park", 0x15, 0, 0, 0x45),
            source("park", 0x17, 2, 3, 0x56),
            source("forest", 0x13, 2, 2, 0x01),
            source("forest", 0x13, 3, 3, 0x16),
            source("radio_tower", 0x02, 2, 0, 0x03),
            source("radio_tower", 0x02, 3, 1, 0x14),
        ] {
            assert!(matches!(
                sign_shape(&source),
                Some(CellShape::FacadeBand {
                    band_count: 2,
                    solid: SolidKind::Prop,
                    ..
                })
            ));
        }
    }
}
