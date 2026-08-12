//! Authored PokéCom Center admin-office presentation roles.
//!
//! This Japanese-mobile room owns a unique tileset. Keep its profiles
//! map-scoped so identically numbered tiles elsewhere cannot inherit them.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const ADMIN_OFFICE: &str = "PokecomCenterAdminOfficeMobile";
pub(crate) const WORKSTATION_FLOOR_TILE: u16 = 0x82;
pub(crate) const PLANT_FLOOR_TILE: u16 = 0x01;

/// The admin office's southern room begins with a continuous north window
/// course. Its first three source rows are one face-on 24px drawing; row 3
/// differs per block and remains the room-side base/floor transition.
pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if map_id == ADMIN_OFFICE
        && source.tileset_id.as_ref() == "pokecom_center"
        && matches!(source.metatile_id, 0x30..=0x32)
        && source.subtile_row < 3
    {
        return Some(CellShape::FacadeBand {
            plane_subtile_row: 3,
            band_from_top: source.subtile_row,
            band_count: 3,
            ground_tile_index: WORKSTATION_FLOOR_TILE,
            // This is a complete independent wall card, not a partial
            // building template subject to the mesher's completeness gate.
            solid: SolidKind::FlatCard,
        });
    }
    None
}

/// The three admin workstations are one 16x24 drawing split between the right
/// half of block $35 and the left half of block $37. Return object-local
/// coordinates only when the exact authored pixels occupy that source cell.
pub(crate) fn workstation_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != ADMIN_OFFICE || source.tileset_id.as_ref() != "pokecom_center" {
        return None;
    }
    let local_column = match source.metatile_id {
        0x35 if source.subtile_column >= 2 => source.subtile_column - 2,
        0x37 if source.subtile_column < 2 => source.subtile_column,
        _ => return None,
    };
    if source.subtile_row >= 3 {
        return None;
    }
    const DRAWING: [[u16; 2]; 3] = [[0x28, 0x29], [0x38, 0x39], [0x3a, 0x3b]];
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(local_column)])
        .then_some((local_column, source.subtile_row))
}

/// Block $34 contains two independent 16x24 potted-plant drawings. Each is
/// segmented as its own thin card so adjacent plants cannot fuse laterally.
pub(crate) fn plant_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != ADMIN_OFFICE
        || source.tileset_id.as_ref() != "pokecom_center"
        || source.metatile_id != 0x34
        || !(1..=3).contains(&source.subtile_row)
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row - 1;
    const DRAWING: [[u16; 2]; 3] = [[0xae, 0xaf], [0xbe, 0xbf], [0x5e, 0x5f]];
    (source.tile_index == DRAWING[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

/// Block $2e packs two independent 16x16 chair drawings into its upper half.
/// Keep each chair as a separate zero-depth card; joining the four columns
/// creates a bench, while extruding them creates the same can-like artifact as
/// the old casino machine geometry.
pub(crate) fn chair_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != ADMIN_OFFICE
        || source.tileset_id.as_ref() != "pokecom_center"
        || source.metatile_id != 0x2e
        || source.subtile_row >= 2
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    const DRAWING: [[u16; 2]; 2] = [[0x48, 0x49], [0x58, 0x59]];
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(local_column)])
        .then_some((local_column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("pokecom_center"),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index,
        }
    }

    #[test]
    fn both_metatile_halves_resolve_the_same_complete_workstation() {
        let drawing = [[0x28, 0x29], [0x38, 0x39], [0x3a, 0x3b]];
        for metatile in [0x35, 0x37] {
            for (row, source_row) in drawing.into_iter().enumerate() {
                for (column, tile) in source_row.into_iter().enumerate() {
                    let source_column = if metatile == 0x35 { column + 2 } else { column };
                    assert_eq!(
                        workstation_local(
                            ADMIN_OFFICE,
                            &source(metatile, source_column as u8, row as u8, tile)
                        ),
                        Some((column as u8, row as u8))
                    );
                }
            }
        }
    }

    #[test]
    fn window_course_folds_three_rows_and_preserves_the_base_row() {
        for block in 0x30..=0x32 {
            for row in 0..3 {
                assert_eq!(
                    shape(ADMIN_OFFICE, &source(block, 0, row, 0x8a)),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 3,
                        band_from_top: row,
                        band_count: 3,
                        ground_tile_index: WORKSTATION_FLOOR_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
            assert_eq!(shape(ADMIN_OFFICE, &source(block, 0, 3, 0x34)), None);
        }
        assert_eq!(
            shape("GoldenrodPokecenter1F", &source(0x30, 0, 0, 0x8a)),
            None
        );
    }

    #[test]
    fn workstation_identity_is_map_and_art_scoped() {
        let workstation = source(0x35, 2, 0, 0x28);
        assert_eq!(
            workstation_local("GoldenrodPokecenter1F", &workstation),
            None
        );
        assert_eq!(
            workstation_local(ADMIN_OFFICE, &source(0x35, 2, 0, 0x29)),
            None
        );
        assert_eq!(
            workstation_local(ADMIN_OFFICE, &source(0x35, 2, 3, WORKSTATION_FLOOR_TILE)),
            None
        );
    }

    #[test]
    fn block_34_resolves_two_separate_complete_plants() {
        let drawing = [[0xae, 0xaf], [0xbe, 0xbf], [0x5e, 0x5f]];
        for half in 0..2 {
            for (row, source_row) in drawing.into_iter().enumerate() {
                for (column, tile) in source_row.into_iter().enumerate() {
                    assert_eq!(
                        plant_local(
                            ADMIN_OFFICE,
                            &source(0x34, (half * 2 + column) as u8, (row + 1) as u8, tile)
                        ),
                        Some((column as u8, row as u8))
                    );
                }
            }
        }
        assert_eq!(
            plant_local(ADMIN_OFFICE, &source(0x34, 0, 0, PLANT_FLOOR_TILE)),
            None
        );
    }

    #[test]
    fn block_2e_resolves_two_separate_chairs_without_volume() {
        let drawing = [[0x48, 0x49], [0x58, 0x59]];
        for half in 0..2 {
            for (row, source_row) in drawing.into_iter().enumerate() {
                for (column, tile) in source_row.into_iter().enumerate() {
                    assert_eq!(
                        chair_local(
                            ADMIN_OFFICE,
                            &source(0x2e, (half * 2 + column) as u8, row as u8, tile)
                        ),
                        Some((column as u8, row as u8))
                    );
                }
            }
        }
        assert_eq!(chair_local(ADMIN_OFFICE, &source(0x2e, 0, 2, 0x01)), None);
    }
}
