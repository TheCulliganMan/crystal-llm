//! Reusable authored wall courses for Crystal's ordinary house interiors.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

pub(crate) const HOUSE_FLOOR_TILE: u16 = 0x01;
pub(crate) const TRADITIONAL_HOUSE_FLOOR_TILE: u16 = 0x50;

/// Traditional gift shops use block `$02` as a complete merchandise shelf:
/// two source rows seen from above, followed by two native front rows.
pub(crate) fn traditional_gift_shop_shelf_local(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<(u8, u8)> {
    (matches!(map_id, "MahoganyMart1F" | "MountMoonGiftShop")
        && source.tileset_id.as_ref() == "traditional_house"
        && source.metatile_id == 0x02)
        .then_some((source.subtile_column, source.subtile_row))
}

/// Blocks `$30/$31` pack four independent 16x16 low display/planter tables.
/// Return coordinates within one drawing, not within the whole metatile, so
/// adjacent tables keep four separate footprints and never merge.
pub(crate) fn display_table_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "house" {
        return None;
    }
    let (origin_column, origin_row) = match source.metatile_id {
        0x30 if source.subtile_column < 2 => (0, source.subtile_row / 2 * 2),
        0x31 if source.subtile_column >= 2 => (2, source.subtile_row / 2 * 2),
        _ => return None,
    };
    let local_column = source.subtile_column - origin_column;
    let local_row = source.subtile_row - origin_row;
    let expected = [[0x2a, 0x2b], [0x5e, 0x5f]];
    (source.tile_index == expected[usize::from(local_row)][usize::from(local_column)])
        .then_some((local_column, local_row))
}

/// Trainer House block `$25` contains the complete 16x16 stairwell at the
/// only warp from 1F down to B1F. The west half is the authored drawing and
/// the east half is ordinary floor. Keep the four stair cells together and
/// leave the remaining twelve cells flat.
pub(crate) fn stair_local(
    source: &VisualTileSource,
) -> Option<(u8, u8, crate::players_house::StairKind)> {
    if source.tileset_id.as_ref() != "house"
        || source.metatile_id != 0x25
        || source.subtile_column >= 2
        || source.subtile_row >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x4c, 0x4d], [0x5c, 0x5d]];
    (source.tile_index
        == DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)])
    .then_some((
        source.subtile_column,
        source.subtile_row,
        crate::players_house::StairKind::DownWest,
    ))
}

/// The west warp in the Wise Trios room uses the same complete four-cell
/// descending stair drawing as the ordinary-house stairwell, but in the
/// traditional-house tileset. Keep this map-scoped: block `$39` is a mixed
/// transition block elsewhere and its collision label alone is not geometry.
pub(crate) fn wise_trios_stair_local(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<(u8, u8, crate::players_house::StairKind)> {
    if map_id != "WiseTriosRoom"
        || source.tileset_id.as_ref() != "traditional_house"
        || source.metatile_id != 0x39
        || source.subtile_column < 2
        || source.subtile_row >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x4c, 0x4d], [0x5c, 0x5d]];
    let local_column = source.subtile_column - 2;
    let local_row = source.subtile_row;
    (source.tile_index == DRAWING[usize::from(local_row)][usize::from(local_column)]).then_some((
        local_column,
        local_row,
        crate::players_house::StairKind::DownWest,
    ))
}

pub(crate) fn stair_shape(source: &VisualTileSource) -> Option<CellShape> {
    let (column, _, kind) = stair_local(source)?;
    let (west_height, east_height) = match (kind, column) {
        (crate::players_house::StairKind::DownWest, 0) => (-16.0, -8.0),
        (crate::players_house::StairKind::DownWest, 1) => (-8.0, 0.0),
        _ => unreachable!(),
    };
    Some(CellShape::RampEast {
        west_height,
        east_height,
    })
}

/// Traditional-house blocks `$03`, `$25`, and `$2d` are mixed north-edge
/// courses: two rows of face-on wall art followed by two rows of floor. Fold
/// only the authored wall half and preserve each room's actual floor material.
pub(crate) fn traditional_mixed_wall_shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "traditional_house" || source.subtile_row >= 2 {
        return None;
    }
    let ground_tile_index = match source.metatile_id {
        0x03 => 0x50,
        0x25 => 0x01,
        0x2d => 0x50,
        _ => return None,
    };
    Some(CellShape::FacadeBand {
        plane_subtile_row: 2,
        band_from_top: source.subtile_row,
        band_count: 2,
        ground_tile_index,
        solid: SolidKind::FlatCard,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FurnitureKind {
    Stool,
    Table,
}

impl FurnitureKind {
    pub(crate) const fn height(self) -> f32 {
        match self {
            Self::Stool => 5.0,
            Self::Table => 6.0,
        }
    }
}

/// Resolve the complete 2x2 stool/table drawings mirrored by blocks `$01`
/// and `$02`. Local coordinates let the mesher give each object one perimeter
/// instead of joining neighboring source cells into terrain.
pub(crate) fn furniture_local(source: &VisualTileSource) -> Option<(u8, u8, FurnitureKind)> {
    if source.tileset_id.as_ref() == "players_house" {
        let (origin_column, origin_row) = match (
            source.metatile_id,
            source.subtile_column,
            source.subtile_row,
        ) {
            (0x08 | 0x26, 0..=1, 2..=3) => (0, 2),
            (0x09, 2..=3, 2..=3) => (2, 2),
            (0x0c | 0x28, 0..=1, 0..=1) => (0, 0),
            (0x0d, 2..=3, 0..=1) => (2, 0),
            _ => return None,
        };
        const STOOL: [[u16; 2]; 2] = [[0x02, 0x03], [0x12, 0x13]];
        let local_column = source.subtile_column - origin_column;
        let local_row = source.subtile_row - origin_row;
        return (source.tile_index == STOOL[usize::from(local_row)][usize::from(local_column)])
            .then_some((local_column, local_row, FurnitureKind::Stool));
    }
    if source.tileset_id.as_ref() != "house" {
        return None;
    }
    let (origin_column, origin_row, drawing, kind) = match (
        source.metatile_id,
        source.subtile_column,
        source.subtile_row,
    ) {
        (0x01, 0..=1, 2..=3) => (0, 2, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
        (0x01, 2..=3, 2..=3) => (2, 2, [[0x26, 0x27], [0x36, 0x2f]], FurnitureKind::Table),
        (0x02, 0..=1, 2..=3) => (0, 2, [[0x27, 0x29], [0x2f, 0x39]], FurnitureKind::Table),
        (0x02, 2..=3, 2..=3) => (2, 2, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
        (0x0c, 0..=1, 0..=1) => (0, 0, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
        (0x0c, 2..=3, 0..=1) => (2, 0, [[0x05, 0x2f], [0x3c, 0x3a]], FurnitureKind::Table),
        (0x0d, 0..=1, 0..=1) => (0, 0, [[0x2f, 0x15], [0x3a, 0x3b]], FurnitureKind::Table),
        (0x0d, 2..=3, 0..=1) => (2, 0, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
        _ => return None,
    };
    let local_column = source.subtile_column - origin_column;
    let local_row = source.subtile_row - origin_row;
    (source.tile_index == drawing[usize::from(local_row)][usize::from(local_column)]).then_some((
        local_column,
        local_row,
        kind,
    ))
}

/// Resolve complete bookshelf, TV, and radio-cabinet drawings. These fixtures
/// are face-on artwork against the north wall, so they use the same thin-card
/// path as trees instead of pixel relief or a row of little boxes.
pub(crate) fn upright_fixture_local(source: &VisualTileSource) -> Option<(u8, u8, usize, usize)> {
    if source.tileset_id.as_ref() != "house" {
        return None;
    }
    let (origin_column, origin_row, width, height, expected) = match source.metatile_id {
        0x1d if source.subtile_column >= 2 && source.subtile_row >= 1 => {
            const DRAWING: [[u16; 2]; 3] = [[0x0c, 0x0d], [0x1c, 0x1d], [0x1e, 0x1f]];
            (
                2,
                1,
                2,
                3,
                DRAWING[usize::from(source.subtile_row - 1)]
                    [usize::from(source.subtile_column - 2)],
            )
        }
        0x1e if source.subtile_column < 2 && source.subtile_row >= 1 => {
            const DRAWING: [[u16; 2]; 3] = [[0x06, 0x07], [0x16, 0x17], [0x1e, 0x1f]];
            (
                0,
                1,
                2,
                3,
                DRAWING[usize::from(source.subtile_row - 1)][usize::from(source.subtile_column)],
            )
        }
        _ => return None,
    };
    (source.tile_index == expected).then_some((
        source.subtile_column - origin_column,
        source.subtile_row - origin_row,
        width,
        height,
    ))
}

/// Complete 16x32 ordinary-house bookcase drawings. Their first nine source
/// pixel rows are the shallow top and the remaining 23 rows are the facade.
pub(crate) fn bookcase_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "house" {
        return None;
    }
    let (origin, expected) = match source.metatile_id {
        0x04 => {
            const DRAWING: [[u16; 4]; 4] = [
                [0x32, 0x33, 0x32, 0x33],
                [0x30, 0x31, 0x0e, 0x0f],
                [0x0e, 0x0f, 0x0e, 0x0f],
                [0x3c, 0x3b, 0x1e, 0x1f],
            ];
            let origin = source.subtile_column / 2 * 2;
            (
                origin,
                DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)],
            )
        }
        0x09 if source.subtile_column >= 2 => {
            const DRAWING: [[u16; 2]; 4] = [[0x32, 0x33], [0x30, 0x31], [0x30, 0x31], [0x1e, 0x1f]];
            (
                2,
                DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column - 2)],
            )
        }
        0x1a => {
            const DRAWING: [[u16; 4]; 4] = [
                [0x32, 0x33, 0x32, 0x33],
                [0x30, 0x31, 0x30, 0x31],
                [0x30, 0x31, 0x0e, 0x0f],
                [0x1e, 0x1f, 0x1e, 0x1f],
            ];
            let origin = source.subtile_column / 2 * 2;
            (
                origin,
                DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)],
            )
        }
        _ => return None,
    };
    (source.tile_index == expected).then_some((source.subtile_column - origin, source.subtile_row))
}

/// Traditional-house block `$01` contains one complete 16x16 radio drawing
/// beside its tatami ground sample. Keep the four live art cells together as
/// a single face-on fixture instead of leaving them pitched onto the floor.
pub(crate) fn traditional_radio_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "traditional_house"
        || source.metatile_id != 0x01
        || source.subtile_column >= 2
        || source.subtile_row < 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x0a, 0x0b], [0x1a, 0x1b]];
    let column = source.subtile_column;
    let row = source.subtile_row - 2;
    (source.tile_index == DRAWING[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// Block `$04` contains four separate 16x16 floor cushions. Each quadrant is
/// one shallow top-facing pad; grouping by quadrant prevents the four cushions
/// from becoming either sixteen little cubes or one 32px platform.
pub(crate) fn traditional_cushion_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "traditional_house" || source.metatile_id != 0x04 {
        return None;
    }
    const DRAWING: [[u16; 4]; 4] = [
        [0x44, 0x45, 0x45, 0x46],
        [0x54, 0x55, 0x55, 0x56],
        [0x45, 0x46, 0x44, 0x45],
        [0x55, 0x56, 0x54, 0x55],
    ];
    (source.tile_index
        == DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)])
    .then_some((source.subtile_column % 2, source.subtile_row % 2))
}

/// Blocks `$06/$07` pack the same complete 16x32 potted-plant drawing into
/// opposite halves. This is shared house art, so every exact occurrence is
/// one upright card rather than four floor cells.
pub(crate) fn plant_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "house" {
        return None;
    }
    let local_column = match source.metatile_id {
        0x06 if source.subtile_column < 2 => source.subtile_column,
        0x07 if source.subtile_column >= 2 => source.subtile_column - 2,
        _ => return None,
    };
    const DRAWING: [[u16; 2]; 4] = [[0x0a, 0x0b], [0x08, 0x09], [0x1a, 0x1b], [0x18, 0x19]];
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(local_column)])
        .then_some((local_column, source.subtile_row))
}

/// Trainer House's open book is one 16x16 drawing split across the south
/// edge of block `$14` and the north edge of block `$18`. Keep the four cells
/// together so the paper can stand on its table instead of remaining painted
/// across four pitched floor cells.
pub(crate) fn open_book_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "house" || source.subtile_column < 2 {
        return None;
    }
    let (row, expected) = match (source.metatile_id, source.subtile_row) {
        (0x14, 3) => (0, [0x46, 0x47]),
        (0x18, 0) => (1, [0x56, 0x57]),
        _ => return None,
    };
    let column = source.subtile_column - 2;
    (source.tile_index == expected[usize::from(column)]).then_some((column, row))
}

/// Soul House blocks `$28`, `$2a`, and `$2b` each pack two independent 16x16
/// memorial units into their north half. Keep each two-by-two drawing
/// separate: grouping the whole four-column block creates a false continuous
/// bench across the visible seams and destroys the room's aisle topology.
pub(crate) fn soul_house_bench_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "house" || source.subtile_row >= 2 {
        return None;
    }
    let drawing = match source.metatile_id {
        0x28 => [[0x37, 0x28, 0x37, 0x28], [0x38, 0x3f, 0x38, 0x4e]],
        0x2a => [[0x37, 0x28, 0x37, 0x28], [0x38, 0x4e, 0x38, 0x3f]],
        0x2b => [[0x37, 0x28, 0x37, 0x28], [0x38, 0x4e, 0x38, 0x4e]],
        _ => return None,
    };
    (source.tile_index
        == drawing[usize::from(source.subtile_row)][usize::from(source.subtile_column)])
    .then_some((source.subtile_column % 2, source.subtile_row))
}

/// Block `$05` is the common two-row north wall/window drawing followed by
/// two rows of ordinary floor. Fold only those face-on rows onto their shared
/// south seam; furniture and collision do not participate in this decision.
pub(crate) fn shape(source: &VisualTileSource) -> Option<CellShape> {
    if source.tileset_id.as_ref() != "house" {
        return None;
    }
    if source.subtile_row >= 2 {
        return None;
    }
    let expected = match source.metatile_id {
        0x05 => [
            [Some(0x00), Some(0x00), Some(0x24), Some(0x4a)],
            [Some(0x00), Some(0x00), Some(0x34), Some(0x2c)],
        ],
        // These mixed corner blocks contain wall courses beside furniture.
        // `None` cells remain owned by their furniture profile.
        0x1d => [
            [Some(0x00), Some(0x00), Some(0x00), Some(0x00)],
            [Some(0x00), Some(0x00), None, None],
        ],
        0x1e => [
            [Some(0x00), Some(0x00), Some(0x2d), Some(0x2e)],
            [None, None, Some(0x3d), Some(0x3e)],
        ],
        _ => return None,
    };
    if expected[usize::from(source.subtile_row)][usize::from(source.subtile_column)]
        != Some(source.tile_index)
    {
        return None;
    }
    Some(CellShape::FacadeBand {
        plane_subtile_row: 2,
        band_from_top: source.subtile_row,
        band_count: 2,
        ground_tile_index: HOUSE_FLOOR_TILE,
        solid: SolidKind::FlatCard,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(column: u8, row: u8, tile_index: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("house"),
            metatile_id: 0x05,
            subtile_column: column,
            subtile_row: row,
            tile_index,
        }
    }

    #[test]
    fn traditional_gift_shop_shelf_is_one_complete_map_scoped_block() {
        let mut cell = source(3, 2, 0x28);
        cell.tileset_id = Arc::from("traditional_house");
        cell.metatile_id = 0x02;
        assert_eq!(
            traditional_gift_shop_shelf_local("MahoganyMart1F", &cell),
            Some((3, 2))
        );
        assert_eq!(
            traditional_gift_shop_shelf_local("MountMoonGiftShop", &cell),
            Some((3, 2))
        );
        assert_eq!(traditional_gift_shop_shelf_local("KurtsHouse", &cell), None);
    }

    #[test]
    fn trainer_house_stairwell_claims_only_the_exact_west_two_by_two_drawing() {
        let drawing = [[0x4c, 0x4d], [0x5c, 0x5d]];
        for row in 0..2 {
            for column in 0..2 {
                let mut cell = source(column, row, drawing[row as usize][column as usize]);
                cell.metatile_id = 0x25;
                assert_eq!(
                    stair_local(&cell),
                    Some((column, row, crate::players_house::StairKind::DownWest))
                );
                let expected_heights = if column == 0 {
                    (-16.0, -8.0)
                } else {
                    (-8.0, 0.0)
                };
                assert_eq!(
                    stair_shape(&cell),
                    Some(CellShape::RampEast {
                        west_height: expected_heights.0,
                        east_height: expected_heights.1,
                    })
                );
            }
        }
        let mut floor = source(2, 0, 0x01);
        floor.metatile_id = 0x25;
        assert_eq!(stair_local(&floor), None);
        let mut lookalike = source(0, 0, 0x4c);
        lookalike.metatile_id = 0x24;
        assert_eq!(stair_local(&lookalike), None);
    }

    #[test]
    fn wise_trios_west_warp_uses_one_complete_traditional_stair_drawing() {
        let drawing = [[0x4c, 0x4d], [0x5c, 0x5d]];
        for row in 0..2 {
            for column in 0..2 {
                let mut cell = source(column + 2, row, drawing[row as usize][column as usize]);
                cell.tileset_id = Arc::from("traditional_house");
                cell.metatile_id = 0x39;
                assert_eq!(
                    wise_trios_stair_local("WiseTriosRoom", &cell),
                    Some((column, row, crate::players_house::StairKind::DownWest))
                );
                assert_eq!(wise_trios_stair_local("DanceTheater", &cell), None);
            }
        }
    }

    #[test]
    fn display_blocks_resolve_four_independent_two_by_two_tables() {
        let drawing = [[0x2a, 0x2b], [0x5e, 0x5f]];
        for (block, origin_column) in [(0x30, 0_u8), (0x31, 2_u8)] {
            for origin_row in [0_u8, 2_u8] {
                for row in 0..2_u8 {
                    for column in 0..2_u8 {
                        let mut cell = source(
                            origin_column + column,
                            origin_row + row,
                            drawing[usize::from(row)][usize::from(column)],
                        );
                        cell.metatile_id = block;
                        assert_eq!(display_table_local(&cell), Some((column, row)));
                    }
                }
            }
        }
        let mut floor = source(2, 0, 0x01);
        floor.metatile_id = 0x30;
        assert_eq!(display_table_local(&floor), None);
    }

    #[test]
    fn common_house_wall_folds_two_exact_rows_once() {
        let drawing = [[0x00, 0x00, 0x24, 0x4a], [0x00, 0x00, 0x34, 0x2c]];
        for row in 0..2 {
            for column in 0..4 {
                assert_eq!(
                    shape(&source(
                        column,
                        row,
                        drawing[usize::from(row)][usize::from(column)]
                    )),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 2,
                        band_from_top: row,
                        band_count: 2,
                        ground_tile_index: HOUSE_FLOOR_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
        }
    }

    #[test]
    fn mixed_traditional_courses_fold_only_their_wall_half() {
        for (metatile, ground) in [(0x03, 0x50), (0x25, 0x01), (0x2d, 0x50)] {
            for row in 0..2 {
                let mut cell = source(0, row, 0x11);
                cell.tileset_id = Arc::from("traditional_house");
                cell.metatile_id = metatile;
                assert_eq!(
                    traditional_mixed_wall_shape(&cell),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 2,
                        band_from_top: row,
                        band_count: 2,
                        ground_tile_index: ground,
                        solid: SolidKind::FlatCard,
                    })
                );
                cell.subtile_row = 2;
                assert_eq!(traditional_mixed_wall_shape(&cell), None);
            }
        }
    }

    #[test]
    fn mixed_corner_blocks_fold_wall_art_but_leave_furniture_owned_cells() {
        for (metatile, cells) in [
            (0x1d, [(0, 0, 0x00), (3, 0, 0x00), (0, 1, 0x00)]),
            (0x1e, [(0, 0, 0x00), (2, 0, 0x2d), (3, 1, 0x3e)]),
        ] {
            for (column, row, tile_index) in cells {
                let mut cell = source(column, row, tile_index);
                cell.metatile_id = metatile;
                assert_eq!(
                    shape(&cell),
                    Some(CellShape::FacadeBand {
                        plane_subtile_row: 2,
                        band_from_top: row,
                        band_count: 2,
                        ground_tile_index: HOUSE_FLOOR_TILE,
                        solid: SolidKind::FlatCard,
                    })
                );
            }
        }
        let mut furniture = source(2, 1, 0x0c);
        furniture.metatile_id = 0x1d;
        assert_eq!(shape(&furniture), None);
        furniture = source(0, 1, 0x06);
        furniture.metatile_id = 0x1e;
        assert_eq!(shape(&furniture), None);
    }

    #[test]
    fn ordinary_floor_and_similar_unscoped_art_remain_flat() {
        assert_eq!(shape(&source(0, 2, HOUSE_FLOOR_TILE)), None);
        let mut other = source(0, 0, 0x00);
        other.metatile_id = 0x06;
        assert_eq!(shape(&other), None);
    }

    #[test]
    fn shared_house_plant_is_one_complete_four_row_card() {
        let drawing = [[0x0a, 0x0b], [0x08, 0x09], [0x1a, 0x1b], [0x18, 0x19]];
        for metatile in [0x06, 0x07] {
            for row in 0..4 {
                for column in 0..2 {
                    let source_column = if metatile == 0x06 { column } else { column + 2 };
                    let mut cell =
                        source(source_column, row, drawing[row as usize][column as usize]);
                    cell.metatile_id = metatile;
                    assert_eq!(plant_local(&cell), Some((column, row)));
                }
            }
        }
    }

    #[test]
    fn plant_identity_does_not_escape_the_house_tileset_or_block_half() {
        let mut plant = source(0, 0, 0x0a);
        plant.metatile_id = 0x06;
        assert_eq!(plant_local(&plant), Some((0, 0)));
        plant.tileset_id = Arc::from("mart");
        assert_eq!(plant_local(&plant), None);
        plant.tileset_id = Arc::from("house");
        plant.subtile_column = 2;
        assert_eq!(plant_local(&plant), None);
    }

    #[test]
    fn soul_house_memorial_variants_are_two_separate_two_by_two_drawings() {
        for (block, drawing) in [
            (0x28, [[0x37, 0x28, 0x37, 0x28], [0x38, 0x3f, 0x38, 0x4e]]),
            (0x2a, [[0x37, 0x28, 0x37, 0x28], [0x38, 0x4e, 0x38, 0x3f]]),
            (0x2b, [[0x37, 0x28, 0x37, 0x28], [0x38, 0x4e, 0x38, 0x4e]]),
        ] {
            for row in 0..2 {
                for column in 0..4 {
                    let mut cell = source(column, row, drawing[row as usize][column as usize]);
                    cell.metatile_id = block;
                    assert_eq!(soul_house_bench_local(&cell), Some((column % 2, row)));
                }
            }
        }
        let mut other = source(0, 0, 0x37);
        other.metatile_id = 0x27;
        assert_eq!(soul_house_bench_local(&other), None);
    }

    #[test]
    fn trainer_house_open_book_is_one_cross_metatile_drawing() {
        for (metatile, source_row, tile_row, local_row) in
            [(0x14, 3, [0x46, 0x47], 0), (0x18, 0, [0x56, 0x57], 1)]
        {
            for (column, tile) in tile_row.into_iter().enumerate() {
                let mut cell = source(column as u8 + 2, source_row, tile);
                cell.metatile_id = metatile;
                assert_eq!(open_book_local(&cell), Some((column as u8, local_row)));
            }
        }

        let mut wrong = source(2, 3, 0x46);
        wrong.metatile_id = 0x18;
        assert_eq!(open_book_local(&wrong), None);
    }

    #[test]
    fn mirrored_furniture_resolves_as_four_separate_complete_objects() {
        for (metatile, origin_column, drawing, kind) in [
            (0x01, 0, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
            (0x01, 2, [[0x26, 0x27], [0x36, 0x2f]], FurnitureKind::Table),
            (0x02, 0, [[0x27, 0x29], [0x2f, 0x39]], FurnitureKind::Table),
            (0x02, 2, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
        ] {
            for row in 0..2 {
                for column in 0..2 {
                    let mut cell = source(
                        origin_column + column,
                        row + 2,
                        drawing[row as usize][column as usize],
                    );
                    cell.metatile_id = metatile;
                    assert_eq!(furniture_local(&cell), Some((column, row, kind)));
                }
            }
        }
    }

    #[test]
    fn second_mirrored_furniture_pair_uses_the_same_group_contract() {
        for (metatile, origin_column, drawing, kind) in [
            (0x0c, 0, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
            (0x0c, 2, [[0x05, 0x2f], [0x3c, 0x3a]], FurnitureKind::Table),
            (0x0d, 0, [[0x2f, 0x15], [0x3a, 0x3b]], FurnitureKind::Table),
            (0x0d, 2, [[0x02, 0x03], [0x12, 0x13]], FurnitureKind::Stool),
        ] {
            for row in 0..2 {
                for column in 0..2 {
                    let mut cell = source(
                        origin_column + column,
                        row,
                        drawing[row as usize][column as usize],
                    );
                    cell.metatile_id = metatile;
                    assert_eq!(furniture_local(&cell), Some((column, row, kind)));
                }
            }
        }
    }

    #[test]
    fn player_family_stools_share_the_complete_house_stool_profile() {
        for (metatile, origin_column, origin_row) in [
            (0x08, 0, 2),
            (0x09, 2, 2),
            (0x0c, 0, 0),
            (0x0d, 2, 0),
            (0x26, 0, 2),
            (0x28, 0, 0),
        ] {
            for (row, source_row) in [[0x02, 0x03], [0x12, 0x13]].into_iter().enumerate() {
                for (column, tile) in source_row.into_iter().enumerate() {
                    let mut cell =
                        source(origin_column + column as u8, origin_row + row as u8, tile);
                    cell.tileset_id = Arc::from("players_house");
                    cell.metatile_id = metatile;
                    assert_eq!(
                        furniture_local(&cell),
                        Some((column as u8, row as u8, FurnitureKind::Stool))
                    );
                }
            }
        }
    }

    #[test]
    fn furniture_group_rejects_wrong_art_and_other_tilesets() {
        let mut cell = source(2, 2, 0x01);
        cell.metatile_id = 0x01;
        assert_eq!(furniture_local(&cell), None);
        cell.tile_index = 0x26;
        cell.tileset_id = Arc::from("mart");
        assert_eq!(furniture_local(&cell), None);
    }

    #[test]
    fn bookcases_are_complete_shallow_cabinet_drawings() {
        for (metatile, column, row, tile, expected) in [
            (0x04, 0, 0, 0x32, (0, 0)),
            (0x04, 3, 3, 0x1f, (1, 3)),
            (0x09, 2, 0, 0x32, (0, 0)),
            (0x1a, 1, 2, 0x31, (1, 2)),
        ] {
            let mut cell = source(column, row, tile);
            cell.metatile_id = metatile;
            assert_eq!(bookcase_local(&cell), Some(expected));
            assert_eq!(upright_fixture_local(&cell), None);
        }
    }

    #[test]
    fn tv_and_radio_remain_complete_upright_drawings() {
        for (metatile, column, row, tile, expected) in [
            (0x1d, 2, 1, 0x0c, (0, 0, 2, 3)),
            (0x1d, 3, 3, 0x1f, (1, 2, 2, 3)),
            (0x1e, 0, 1, 0x06, (0, 0, 2, 3)),
            (0x1e, 1, 3, 0x1f, (1, 2, 2, 3)),
        ] {
            let mut cell = source(column, row, tile);
            cell.metatile_id = metatile;
            assert_eq!(upright_fixture_local(&cell), Some(expected));
        }
    }

    #[test]
    fn traditional_house_radio_is_one_exact_two_by_two_drawing() {
        for (tile, column, source_row, local_row) in [
            (0x0a, 0, 2, 0),
            (0x0b, 1, 2, 0),
            (0x1a, 0, 3, 1),
            (0x1b, 1, 3, 1),
        ] {
            let mut cell = source(column, source_row, tile);
            cell.tileset_id = Arc::from("traditional_house");
            cell.metatile_id = 0x01;
            assert_eq!(traditional_radio_local(&cell), Some((column, local_row)));
        }

        let mut wrong_block = source(0, 2, 0x0a);
        wrong_block.tileset_id = Arc::from("traditional_house");
        wrong_block.metatile_id = 0x02;
        assert_eq!(traditional_radio_local(&wrong_block), None);
    }

    #[test]
    fn traditional_cushions_are_four_independent_two_by_two_pads() {
        for (column, row, tile) in [(0, 0, 0x44), (1, 1, 0x55), (2, 0, 0x45), (3, 3, 0x55)] {
            let mut cell = source(column, row, tile);
            cell.tileset_id = Arc::from("traditional_house");
            cell.metatile_id = 0x04;
            assert_eq!(
                traditional_cushion_local(&cell),
                Some((column % 2, row % 2))
            );
        }
    }

    #[test]
    fn upright_fixture_rejects_partial_and_similar_art() {
        let mut cell = source(1, 0, 0x32);
        cell.metatile_id = 0x09;
        assert_eq!(upright_fixture_local(&cell), None);
        cell = source(2, 1, 0x0e);
        cell.metatile_id = 0x1d;
        assert_eq!(upright_fixture_local(&cell), None);
    }
}
