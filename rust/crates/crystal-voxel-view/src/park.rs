//! Authored National Park fountain presentation.

use std::collections::BTreeSet;

use crystal_render_api::{VisualTile, VisualTileSource};

use crate::profile::CellShape;

const PARK_TILESET: &str = "park";
const FOUNTAIN_METATILE: u16 = 0x3f;
const PARK_WATER_TILE: u16 = 0x14;
// The inward tree transition block `$27` supplies the green ground underneath
// the park's outer `$06` tree mass. Selecting a tile from plaza block `$01`
// uses the same graphics under a pale palette and creates visible holes.
pub(crate) const PARK_TREE_GROUND_METATILE: u16 = 0x27;
pub(crate) const PARK_TREE_GROUND_TILE: u16 = 0x01;
const LARGE_TREE_METATILE: u16 = 0x06;
const BENCH_METATILE: u16 = 0x0e;
pub(crate) const PARK_PLAZA_GROUND_TILE: u16 = 0x00;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FountainPlacement {
    pub column: usize,
    pub row: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HedgeKind {
    Light,
    Dark,
}

pub(crate) fn fountain_placements(
    cells: &[&VisualTile],
    width: usize,
    height: usize,
) -> Vec<FountainPlacement> {
    let mut origins = BTreeSet::new();
    for tile in cells {
        if tile.source.tileset_id.as_ref() != PARK_TILESET
            || tile.source.metatile_id != FOUNTAIN_METATILE
        {
            continue;
        }
        let column = tile.column as isize - tile.source.subtile_column as isize;
        let row = tile.row as isize - tile.source.subtile_row as isize;
        if column < 0 || row < 0 || column as usize + 4 > width || row as usize + 4 > height {
            continue;
        }
        let complete = (0..4).all(|local_row| {
            (0..4).all(|local_column| {
                let cell =
                    cells[(row as usize + local_row) * width + column as usize + local_column];
                cell.source.tileset_id.as_ref() == PARK_TILESET
                    && cell.source.metatile_id == FOUNTAIN_METATILE
                    && cell.source.subtile_column as usize == local_column
                    && cell.source.subtile_row as usize == local_row
            })
        });
        if complete {
            origins.insert(FountainPlacement {
                column: column as usize,
                row: row as usize,
            });
        }
    }
    origins.into_iter().collect()
}

/// Park uses the same animated `$14` water identity as the outdoor tilesets,
/// but owns a separate atlas. Recess that exact identity and leave every
/// non-water cell unchanged; the animated fountain requires a grouped mask
/// and must not be guessed by the generic per-tile relief path.
pub(crate) fn park_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != PARK_TILESET {
        return None;
    }
    (source.tile_index == PARK_WATER_TILE).then_some(CellShape::Water)
}

/// Block `$06` is one complete 32x32 tree drawing. Keep it as a single
/// masked upright card: splitting it into sixteen cells leaves the whole tree
/// painted flat on the ground, while giving it a hull invents volume that the
/// source art does not describe.
pub(crate) fn large_tree_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != PARK_TILESET || source.metatile_id != LARGE_TREE_METATILE {
        return None;
    }
    let expected = u16::from(source.subtile_row) * 0x10 + 0x0c + u16::from(source.subtile_column);
    (source.tile_index == expected).then_some((source.subtile_column, source.subtile_row))
}

/// National Park's clipped tree borders contain two self-contained 16x16
/// hedge drawings. They repeat inside several transition metatiles, but each
/// pair is still one face-on silhouette. Group only the exact four-tile
/// drawings; the remaining transition pixels stay flat until their complete
/// topology is understood.
pub(crate) fn hedge_local(source: &VisualTileSource) -> Option<(HedgeKind, u8, u8)> {
    if source.tileset_id.as_ref() != PARK_TILESET {
        return None;
    }
    match source.tile_index {
        0x25 => Some((HedgeKind::Light, 0, 0)),
        0x26 => Some((HedgeKind::Light, 1, 0)),
        0x35 => Some((HedgeKind::Light, 0, 1)),
        0x36 => Some((HedgeKind::Light, 1, 1)),
        0x23 => Some((HedgeKind::Dark, 0, 0)),
        0x24 => Some((HedgeKind::Dark, 1, 0)),
        0x33 => Some((HedgeKind::Dark, 0, 1)),
        0x34 => Some((HedgeKind::Dark, 1, 1)),
        _ => None,
    }
}

/// The upper half of block `$0e` is one complete 32x16 park bench. It has no
/// authored depth view, so preserve it as a single upright masked card rather
/// than fabricating a seat box from its front-facing pixels.
pub(crate) fn bench_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != PARK_TILESET
        || source.metatile_id != BENCH_METATILE
        || source.subtile_row >= 2
    {
        return None;
    }
    let expected = 0x07 + u16::from(source.subtile_row) * 0x10 + u16::from(source.subtile_column);
    (source.tile_index == expected).then_some((source.subtile_column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::prelude::{Handle, Image};
    use crystal_render_api::{VisualTile, VisualTileSource};

    use super::*;

    fn source(metatile_id: u16, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(PARK_TILESET),
            metatile_id,
            subtile_column: 0,
            subtile_row: 0,
            tile_index,
        }
    }

    #[test]
    fn fountain_water_recesses_without_extruding_animation_cells() {
        assert_eq!(
            park_shape(&source(0x3f, PARK_WATER_TILE)),
            Some(CellShape::Water)
        );
        assert_eq!(park_shape(&source(0x3f, 0x5f)), None);
        assert_eq!(park_shape(&source(0x3f, 0x80)), None);
    }

    #[test]
    fn ordinary_park_water_is_recessed_but_other_art_is_not_guessed() {
        assert_eq!(
            park_shape(&source(0x31, PARK_WATER_TILE)),
            Some(CellShape::Water)
        );
        assert_eq!(park_shape(&source(0x31, 0x41)), None);
    }

    #[test]
    fn complete_fountain_metatile_produces_one_grouped_placement() {
        let tiles = (0..16)
            .map(|index| VisualTile {
                column: (index % 4) as u32,
                row: (index / 4) as u32,
                source: VisualTileSource {
                    tileset_id: Arc::from(PARK_TILESET),
                    metatile_id: FOUNTAIN_METATILE,
                    subtile_column: (index % 4) as u8,
                    subtile_row: (index / 4) as u8,
                    tile_index: PARK_WATER_TILE,
                },
                texture: Handle::<Image>::weak_from_u128(index as u128 + 1),
                priority: false,
            })
            .collect::<Vec<_>>();
        let ordered = tiles.iter().collect::<Vec<_>>();
        assert_eq!(
            fountain_placements(&ordered, 4, 4),
            vec![FountainPlacement { column: 0, row: 0 }]
        );
        assert!(fountain_placements(&ordered[..12], 4, 3).is_empty());
    }

    #[test]
    fn large_park_tree_is_one_exact_four_by_four_drawing() {
        for row in 0..4 {
            for column in 0..4 {
                let mut tile = source(0x06, row * 0x10 + 0x0c + column);
                tile.subtile_column = column as u8;
                tile.subtile_row = row as u8;
                assert_eq!(large_tree_local(&tile), Some((column as u8, row as u8)));
            }
        }
        assert_eq!(large_tree_local(&source(0x06, 0x00)), None);
        assert_eq!(large_tree_local(&source(0x05, 0x0c)), None);
    }

    #[test]
    fn park_hedges_are_two_complete_two_by_two_drawings() {
        for (kind, drawing) in [
            (HedgeKind::Light, [0x25, 0x26, 0x35, 0x36]),
            (HedgeKind::Dark, [0x23, 0x24, 0x33, 0x34]),
        ] {
            for (tile, local) in drawing.into_iter().zip([(0, 0), (1, 0), (0, 1), (1, 1)]) {
                assert_eq!(
                    hedge_local(&source(0x11, tile)),
                    Some((kind, local.0, local.1))
                );
            }
        }
        assert_eq!(hedge_local(&source(0x11, 0x05)), None);
        let mut johto = source(0x11, 0x25);
        johto.tileset_id = Arc::from("johto");
        assert_eq!(hedge_local(&johto), None);
    }

    #[test]
    fn park_bench_is_one_exact_four_by_two_card() {
        for row in 0..2 {
            for column in 0..4 {
                let mut tile = source(0x0e, 0x07 + row * 0x10 + column);
                tile.subtile_column = column as u8;
                tile.subtile_row = row as u8;
                assert_eq!(bench_local(&tile), Some((column as u8, row as u8)));
            }
        }
        let mut floor = source(0x0e, 0x27);
        floor.subtile_row = 2;
        assert_eq!(bench_local(&floor), None);
        assert_eq!(bench_local(&source(0x0f, 0x07)), None);
    }
}
