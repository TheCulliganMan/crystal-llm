//! Exact free-standing boulder groups shared by Crystal's Ice Path maps.

use crystal_render_api::VisualTileSource;

pub(crate) const CAVE_GROUND_TILE: u16 = 0x19;
pub(crate) const SMOOTH_ICE_TILE: u16 = 0xc6;

/// Block `$19` is one unique 32x32 ice-rock mass. The topology resolver uses
/// these exact coordinates to promote only a complete drawing into one
/// trapezoid; clipped fragments stay faithful flat art.
pub(crate) fn rock_mass_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "ice_path" {
        return None;
    }
    let drawing = match source.metatile_id {
        0x19 => [
            [0x84, 0x85, 0x86, 0x87],
            [0x94, 0x95, 0x96, 0x97],
            [0xa4, 0xa5, 0xa6, 0xa7],
            [0xb4, 0xb5, 0xb6, 0xb7],
        ],
        0x3b => [
            [0x88, 0x89, 0x8c, 0x8d],
            [0x98, 0x99, 0x9c, 0x9d],
            [0xb8, 0xb9, 0xbc, 0xbd],
            [0xc8, 0xc9, 0xcc, 0xcd],
        ],
        _ => return None,
    };
    let column = usize::from(source.subtile_column);
    let row = usize::from(source.subtile_row);
    (column < 4 && row < 4 && source.tile_index == drawing[row][column])
        .then_some((source.subtile_column, source.subtile_row))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoulderBase {
    CaveGround,
    SmoothIce,
    UpperRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeRockKind {
    Left,
    Right,
    Single,
}

pub(crate) fn boulder_local(source: &VisualTileSource, base: BoulderBase) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "ice_path" {
        return None;
    }
    let valid_block = match base {
        BoulderBase::CaveGround => matches!(source.metatile_id, 0x1a | 0x21),
        BoulderBase::SmoothIce => matches!(source.metatile_id, 0x2c..=0x2f),
        BoulderBase::UpperRight => source.metatile_id == 0x22,
    };
    if !valid_block {
        return None;
    }
    match base {
        BoulderBase::CaveGround | BoulderBase::SmoothIce => match source.tile_index {
            0x82 => Some((0, 0)),
            0x83 => Some((1, 0)),
            0x92 => Some((0, 1)),
            0x93 => Some((1, 1)),
            _ => None,
        },
        BoulderBase::UpperRight => match source.tile_index {
            0x06 => Some((0, 0)),
            0x07 => Some((1, 0)),
            0x16 => Some((0, 1)),
            0x17 => Some((1, 1)),
            _ => None,
        },
    }
}

/// Ice Path's perimeter and ladder frames reuse three complete 16x16 rock
/// drawings. Each drawing is a face-on object, not a terrain-height token:
/// group its four cells once and stand the live art upright without raising
/// the ice/floor underneath it.
pub(crate) fn edge_rock_local(source: &VisualTileSource) -> Option<(EdgeRockKind, u8, u8)> {
    if source.tileset_id.as_ref() != "ice_path" {
        return None;
    }
    match source.tile_index {
        0xc0 => Some((EdgeRockKind::Left, 0, 0)),
        0xc1 => Some((EdgeRockKind::Left, 1, 0)),
        0xd0 => Some((EdgeRockKind::Left, 0, 1)),
        0xd1 => Some((EdgeRockKind::Left, 1, 1)),
        0xc2 => Some((EdgeRockKind::Right, 0, 0)),
        0xc3 => Some((EdgeRockKind::Right, 1, 0)),
        0xd2 => Some((EdgeRockKind::Right, 0, 1)),
        0xd3 => Some((EdgeRockKind::Right, 1, 1)),
        0xc4 => Some((EdgeRockKind::Single, 0, 0)),
        0xc5 => Some((EdgeRockKind::Single, 1, 0)),
        0xd4 => Some((EdgeRockKind::Single, 0, 1)),
        0xd5 => Some((EdgeRockKind::Single, 1, 1)),
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
        for (tile, local) in [
            (0x06, (0, 0)),
            (0x07, (1, 0)),
            (0x16, (0, 1)),
            (0x17, (1, 1)),
        ] {
            assert_eq!(
                boulder_local(&source(0x22, tile), BoulderBase::UpperRight),
                Some(local)
            );
        }
        assert_eq!(
            boulder_local(&source(0x21, 0x06), BoulderBase::UpperRight),
            None
        );
    }

    #[test]
    fn block_19_is_one_exact_four_by_four_rock_mass() {
        for row in 0..4 {
            for column in 0..4 {
                let mut source = source(0x19, 0x84 + u16::from(row) * 0x10 + u16::from(column));
                source.subtile_column = column;
                source.subtile_row = row;
                assert_eq!(rock_mass_local(&source), Some((column, row)));
            }
        }
        assert_eq!(rock_mass_local(&source(0x18, 0x84)), None);
    }

    #[test]
    fn block_3b_is_one_exact_four_by_four_closed_rock_mass() {
        let drawing = [
            [0x88, 0x89, 0x8c, 0x8d],
            [0x98, 0x99, 0x9c, 0x9d],
            [0xb8, 0xb9, 0xbc, 0xbd],
            [0xc8, 0xc9, 0xcc, 0xcd],
        ];
        for row in 0..4 {
            for column in 0..4 {
                let mut source = source(0x3b, drawing[row as usize][column as usize]);
                source.subtile_column = column;
                source.subtile_row = row;
                assert_eq!(rock_mass_local(&source), Some((column, row)));
            }
        }
    }

    #[test]
    fn every_edge_rock_drawing_is_one_complete_two_by_two_object() {
        for (kind, drawing) in [
            (EdgeRockKind::Left, [0xc0, 0xc1, 0xd0, 0xd1]),
            (EdgeRockKind::Right, [0xc2, 0xc3, 0xd2, 0xd3]),
            (EdgeRockKind::Single, [0xc4, 0xc5, 0xd4, 0xd5]),
        ] {
            for (tile, local) in drawing.into_iter().zip([(0, 0), (1, 0), (0, 1), (1, 1)]) {
                assert_eq!(
                    edge_rock_local(&source(0x14, tile)),
                    Some((kind, local.0, local.1))
                );
            }
        }
        assert_eq!(edge_rock_local(&source(0x14, CAVE_GROUND_TILE)), None);
        let mut other_tileset = source(0x14, 0xc4);
        other_tileset.tileset_id = Arc::from("cave");
        assert_eq!(edge_rock_local(&other_tileset), None);
    }
}
