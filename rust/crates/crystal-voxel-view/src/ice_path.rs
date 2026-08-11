//! Exact free-standing boulder groups shared by Crystal's Ice Path maps.

use crystal_render_api::VisualTileSource;

pub(crate) const CAVE_GROUND_TILE: u16 = 0x19;
pub(crate) const SMOOTH_ICE_TILE: u16 = 0xc6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoulderBase {
    CaveGround,
    SmoothIce,
}

pub(crate) fn boulder_local(source: &VisualTileSource, base: BoulderBase) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "ice_path" {
        return None;
    }
    let valid_block = match base {
        BoulderBase::CaveGround => matches!(source.metatile_id, 0x1a | 0x21),
        BoulderBase::SmoothIce => matches!(source.metatile_id, 0x2c..=0x2f),
    };
    if !valid_block {
        return None;
    }
    match source.tile_index {
        0x82 => Some((0, 0)),
        0x83 => Some((1, 0)),
        0x92 => Some((0, 1)),
        0x93 => Some((1, 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: u16, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("ice_path"),
            metatile_id: block,
            subtile_column: 0,
            subtile_row: 0,
            tile_index: tile,
        }
    }

    #[test]
    fn each_base_family_keeps_the_complete_two_by_two_drawing() {
        for (tile, local) in [
            (0x82, (0, 0)),
            (0x83, (1, 0)),
            (0x92, (0, 1)),
            (0x93, (1, 1)),
        ] {
            assert_eq!(
                boulder_local(&source(0x21, tile), BoulderBase::CaveGround),
                Some(local)
            );
            assert_eq!(
                boulder_local(&source(0x2f, tile), BoulderBase::SmoothIce),
                Some(local)
            );
        }
        assert_eq!(
            boulder_local(&source(0x21, 0x19), BoulderBase::CaveGround),
            None
        );
        assert_eq!(
            boulder_local(&source(0x21, 0x82), BoulderBase::SmoothIce),
            None
        );
    }
}
