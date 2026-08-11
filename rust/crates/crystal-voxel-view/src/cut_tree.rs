//! Presentation-only profiles for Crystal's field-move Cut trees.
//!
//! These source positions come from pokecrystal's Cut source/replacement
//! block pairs. Comparing each pair identifies the exact 2x2 drawing and its
//! authored ground without consulting collision at render time.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

#[derive(Clone, Copy)]
struct CutTreeBlock {
    tileset: &'static str,
    metatile: u16,
    origin_column: u8,
    origin_row: u8,
    ground_tile: u16,
}

const CUT_TREE_BLOCKS: [CutTreeBlock; 10] = [
    CutTreeBlock::new("johto", 0x5b, 2, 0, 0x05),
    CutTreeBlock::new("johto", 0x5f, 2, 2, 0x05),
    CutTreeBlock::new("johto", 0x63, 0, 2, 0x05),
    CutTreeBlock::new("johto", 0x67, 0, 0, 0x05),
    CutTreeBlock::new("kanto", 0x32, 2, 0, 0x2c),
    CutTreeBlock::new("kanto", 0x33, 2, 2, 0x2c),
    CutTreeBlock::new("kanto", 0x34, 0, 0, 0x2c),
    CutTreeBlock::new("kanto", 0x35, 2, 0, 0x2c),
    CutTreeBlock::new("kanto", 0x60, 0, 2, 0x2c),
    CutTreeBlock::new("forest", 0x0f, 0, 2, 0x05),
];

impl CutTreeBlock {
    const fn new(
        tileset: &'static str,
        metatile: u16,
        origin_column: u8,
        origin_row: u8,
        ground_tile: u16,
    ) -> Self {
        Self {
            tileset,
            metatile,
            origin_column,
            origin_row,
            ground_tile,
        }
    }
}

pub(crate) fn cut_tree_shape(source: &VisualTileSource) -> Option<CellShape> {
    let block = CUT_TREE_BLOCKS.iter().find(|block| {
        source.tileset_id.as_ref() == block.tileset
            && source.metatile_id == block.metatile
            && (block.origin_column..block.origin_column + 2).contains(&source.subtile_column)
            && (block.origin_row..block.origin_row + 2).contains(&source.subtile_row)
    })?;
    Some(CellShape::FacadeBand {
        plane_subtile_row: block.origin_row + 2,
        band_from_top: source.subtile_row - block.origin_row,
        band_count: 2,
        ground_tile_index: block.ground_tile,
        solid: SolidKind::CutTree,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(block: CutTreeBlock, column: u8, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(block.tileset),
            metatile_id: block.metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: 0,
        }
    }

    #[test]
    fn every_authoritative_cut_tree_block_resolves_exactly_one_two_by_two_card() {
        for block in CUT_TREE_BLOCKS {
            let mut resolved = 0;
            for row in 0..4 {
                for column in 0..4 {
                    let shape = cut_tree_shape(&source(block, column, row));
                    if (block.origin_column..block.origin_column + 2).contains(&column)
                        && (block.origin_row..block.origin_row + 2).contains(&row)
                    {
                        resolved += 1;
                        assert_eq!(
                            shape,
                            Some(CellShape::FacadeBand {
                                plane_subtile_row: block.origin_row + 2,
                                band_from_top: row - block.origin_row,
                                band_count: 2,
                                ground_tile_index: block.ground_tile,
                                solid: SolidKind::CutTree,
                            })
                        );
                    } else {
                        assert_eq!(shape, None);
                    }
                }
            }
            assert_eq!(
                resolved, 4,
                "{} block ${:02x}",
                block.tileset, block.metatile
            );
        }
    }
}
