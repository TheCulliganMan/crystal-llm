//! Reusable grouped props from Crystal's Elite Four Room atlas.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, LedgeFace, SolidKind};

pub(crate) const FLOOR_TILE: u16 = 0x01;
pub(crate) const WALL_HEIGHT: f32 = 16.0;

pub(crate) fn supports_boulder_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "BlackthornGym1F" | "GoldenrodUndergroundSwitchRoomEntrances"
    )
}

pub(crate) fn supports_wall_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "GoldenrodUndergroundSwitchRoomEntrances"
            | "WillsRoom"
            | "KogasRoom"
            | "BrunosRoom"
            | "KarensRoom"
    )
}

/// Block $2a is the repeated facility wall module: two horizontal cap rows
/// over two native front courses. Keep the cap top-facing and fold each front
/// row once onto the shared south plane.
pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if !supports_wall_map(map_id)
        || source.tileset_id.as_ref() != "elite_four_room"
        || source.metatile_id != 0x2a
    {
        return None;
    }
    Some(if source.subtile_row < 2 {
        CellShape::RaisedTop {
            height: WALL_HEIGHT,
            solid: SolidKind::Bank,
        }
    } else {
        CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x25,
            height: WALL_HEIGHT,
        }
    })
}

/// Block $29 contains one complete 16x16 round boulder in its southeast
/// quadrant. The other twelve source cells are floor and must not be claimed.
pub(crate) fn boulder_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "elite_four_room" || source.metatile_id != 0x29 {
        return None;
    }
    match source.tile_index {
        0x2c => Some((0, 0)),
        0x2d => Some((1, 0)),
        0x3c => Some((0, 1)),
        0x3d => Some((1, 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("elite_four_room"),
            metatile_id: 0x29,
            subtile_column: 0,
            subtile_row: 0,
            tile_index,
        }
    }

    #[test]
    fn block_29_claims_only_the_complete_southeast_boulder() {
        for (tile, local) in [
            (0x2c, (0, 0)),
            (0x2d, (1, 0)),
            (0x3c, (0, 1)),
            (0x3d, (1, 1)),
        ] {
            assert_eq!(boulder_local(&source(tile)), Some(local));
        }
        assert_eq!(boulder_local(&source(FLOOR_TILE)), None);
    }

    #[test]
    fn boulder_scope_matches_only_the_two_maps_that_place_it() {
        assert!(supports_boulder_map("BlackthornGym1F"));
        assert!(supports_boulder_map(
            "GoldenrodUndergroundSwitchRoomEntrances"
        ));
        assert!(!supports_boulder_map("AzaleaGym"));
    }

    #[test]
    fn facility_wall_keeps_two_cap_rows_and_two_native_front_courses() {
        for row in 0..4 {
            let mut cell = source(if row < 2 { 0x25 } else { 0x10 });
            cell.metatile_id = 0x2a;
            cell.subtile_row = row;
            let shape = shape("GoldenrodUndergroundSwitchRoomEntrances", &cell)
                .expect("authored facility wall row");
            if row < 2 {
                assert!(matches!(shape, CellShape::RaisedTop { .. }));
            } else {
                assert!(matches!(
                    shape,
                    CellShape::LedgeBand {
                        face: LedgeFace::South,
                        band_from_top,
                        band_count: 2,
                        ..
                    } if band_from_top == row - 2
                ));
            }
        }
        let mut unrelated = source(0x25);
        unrelated.metatile_id = 0x2a;
        assert_eq!(shape("AzaleaGym", &unrelated), None);
    }
}
