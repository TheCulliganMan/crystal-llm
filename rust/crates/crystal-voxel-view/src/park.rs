//! Authored National Park fountain presentation.

use std::collections::BTreeSet;

use crystal_render_api::{VisualTile, VisualTileSource};

use crate::profile::CellShape;

const PARK_TILESET: &str = "park";
const FOUNTAIN_METATILE: u16 = 0x3f;
const PARK_WATER_TILE: u16 = 0x14;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FountainPlacement {
    pub column: usize,
    pub row: usize,
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
}
