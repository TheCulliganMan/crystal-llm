//! Exact grouped prop drawings for the Power Plant interior.

use crystal_render_api::VisualTileSource;

const MAP: &str = "PowerPlant";
const TILESET: &str = "facility";
pub(crate) const FLOOR_TILE: u16 = 0x01;

/// Blocks `$0d/$0e` place the same 16x32 potted plant on opposite sides of
/// their floor cells. Keep its four authored bands together as one upright
/// cutout instead of leaving eight independent tiles on the floor plane.
pub(crate) fn plant_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if map_id != MAP
        || source.tileset_id.as_ref() != TILESET
        || !matches!(source.metatile_id, 0x0d | 0x0e)
    {
        return None;
    }

    match source.tile_index {
        0x2c => Some((0, 0)),
        0x2d => Some((1, 0)),
        0x3c => Some((0, 1)),
        0x3d => Some((1, 1)),
        0x2e => Some((0, 2)),
        0x2f => Some((1, 2)),
        0x3e => Some((0, 3)),
        0x3f => Some((1, 3)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile_id: u16, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(TILESET),
            metatile_id,
            subtile_column: 0,
            subtile_row: 0,
            tile_index,
        }
    }

    #[test]
    fn plant_is_one_two_by_four_authored_drawing() {
        for (tile, local) in [
            (0x2c, (0, 0)),
            (0x2d, (1, 0)),
            (0x3c, (0, 1)),
            (0x3d, (1, 1)),
            (0x2e, (0, 2)),
            (0x2f, (1, 2)),
            (0x3e, (0, 3)),
            (0x3f, (1, 3)),
        ] {
            assert_eq!(plant_local(MAP, &source(0x0d, tile)), Some(local));
            assert_eq!(plant_local(MAP, &source(0x0e, tile)), Some(local));
        }
    }

    #[test]
    fn identical_tiles_elsewhere_are_not_claimed() {
        assert_eq!(plant_local("TeamRocketBaseB2F", &source(0x0d, 0x2c)), None);
        assert_eq!(plant_local(MAP, &source(0x12, 0x2c)), None);
    }
}
