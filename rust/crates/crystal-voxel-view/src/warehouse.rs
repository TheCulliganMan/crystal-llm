//! Authored warehouse fixtures from Crystal's Goldenrod underground rooms.

use crystal_render_api::VisualTileSource;

pub(crate) const CRATE_HEIGHT: f32 = 12.0;

pub(crate) fn supports_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "GoldenrodDeptStoreB1F" | "GoldenrodUndergroundWarehouse" | "TeamRocketBaseB1F"
    )
}

pub(crate) fn floor_tile(map_id: &str) -> Option<u16> {
    match map_id {
        "GoldenrodDeptStoreB1F" | "GoldenrodUndergroundWarehouse" => Some(0x01),
        "TeamRocketBaseB1F" => Some(0x10),
        _ => None,
    }
}

/// Underground block $0b packs four independent 16x16 crate-front drawings.
/// Return coordinates within one crate so the renderer cannot join the 4x4
/// metatile into a single cabinet or stretch face art over a generated lid.
pub(crate) fn crate_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !supports_map(map_id)
        || source.tileset_id.as_ref() != "underground"
        || source.metatile_id != 0x0b
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    let local_row = source.subtile_row % 2;
    const DRAWING: [[u16; 2]; 2] = [[0x43, 0x44], [0x53, 0x54]];
    (source.tile_index == DRAWING[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(
        map_tileset: &str,
        metatile: u16,
        column: u8,
        row: u8,
        tile: u16,
    ) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(map_tileset),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn block_b_resolves_four_separate_crates() {
        let drawing = [[0x43, 0x44], [0x53, 0x54]];
        for crate_row in 0..2 {
            for crate_column in 0..2 {
                for row in 0..2 {
                    for column in 0..2 {
                        assert_eq!(
                            crate_local(
                                "GoldenrodDeptStoreB1F",
                                &source(
                                    "underground",
                                    0x0b,
                                    crate_column * 2 + column,
                                    crate_row * 2 + row,
                                    drawing[usize::from(row)][usize::from(column)],
                                ),
                            ),
                            Some((column, row))
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn crate_identity_is_map_tileset_and_art_scoped() {
        let crate_tile = source("underground", 0x0b, 0, 0, 0x43);
        assert_eq!(
            crate_local("GoldenrodUndergroundWarehouse", &crate_tile),
            Some((0, 0))
        );
        assert_eq!(crate_local("TeamRocketBaseB1F", &crate_tile), Some((0, 0)));
        assert_eq!(crate_local("UndergroundPath", &crate_tile), None);
        assert_eq!(
            crate_local("GoldenrodDeptStoreB1F", &source("gate", 0x0b, 0, 0, 0x43),),
            None
        );
        assert_eq!(
            crate_local(
                "GoldenrodDeptStoreB1F",
                &source("underground", 0x0a, 0, 0, 0x43),
            ),
            None
        );
    }

    #[test]
    fn each_supported_layout_uses_its_authored_floor() {
        assert_eq!(floor_tile("GoldenrodDeptStoreB1F"), Some(0x01));
        assert_eq!(floor_tile("GoldenrodUndergroundWarehouse"), Some(0x01));
        assert_eq!(floor_tile("TeamRocketBaseB1F"), Some(0x10));
        assert_eq!(floor_tile("UndergroundPath"), None);
    }

    #[test]
    fn crate_is_a_shallow_authored_box_not_a_wall_or_card() {
        assert_eq!(CRATE_HEIGHT, 12.0);
        assert!(CRATE_HEIGHT < 16.0);
    }
}
