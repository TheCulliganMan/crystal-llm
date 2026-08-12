//! Grouped front-facing fixtures for the player-home tileset.

use crystal_render_api::VisualTileSource;

use crate::profile::CellShape;

pub(crate) const FLOOR_TILE: u16 = 0x01;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StairKind {
    UpEast,
    DownWest,
}

/// Return the exact 2x2 source coordinate and direction for each authored
/// player-house stair drawing. Blocks `$0a`/`$0b` are embedded in the north
/// wall course used by Red and Copycat's houses; block `$0f` is the equivalent
/// ground-floor flight in the player's house. These are art identities, not
/// collision guesses.
pub(crate) fn stair_local(source: &VisualTileSource) -> Option<(u8, u8, StairKind)> {
    if source.tileset_id.as_ref() != "players_house" {
        return None;
    }
    let (origin_column, origin_row, drawing, kind) = match source.metatile_id {
        0x0f => (0, 1, [[0x0a, 0x0b], [0x1a, 0x1b]], StairKind::UpEast),
        0x0a => (2, 0, [[0x4c, 0x4d], [0x5c, 0x5d]], StairKind::UpEast),
        0x0b => (2, 0, [[0x4e, 0x4f], [0x5e, 0x5f]], StairKind::DownWest),
        _ => return None,
    };
    if source.subtile_column < origin_column
        || source.subtile_column >= origin_column + 2
        || source.subtile_row < origin_row
        || source.subtile_row >= origin_row + 2
    {
        return None;
    }
    let column = source.subtile_column - origin_column;
    let row = source.subtile_row - origin_row;
    (source.tile_index == drawing[usize::from(row)][usize::from(column)])
        .then_some((column, row, kind))
}

pub(crate) fn stair_shape(source: &VisualTileSource) -> Option<CellShape> {
    let (column, _, kind) = stair_local(source)?;
    let (west_height, east_height) = match (kind, column) {
        (StairKind::UpEast, 0) => (0.0, 8.0),
        (StairKind::UpEast, 1) => (8.0, 16.0),
        (StairKind::DownWest, 0) => (-16.0, -8.0),
        (StairKind::DownWest, 1) => (-8.0, 0.0),
        _ => unreachable!(),
    };
    Some(CellShape::RampEast {
        west_height,
        east_height,
    })
}

/// The two cabinets in block $1b are independent 16x32 drawings. Their
/// complete source art is reconstructed as shallow bookcases by the mesher;
/// they must not be absorbed into the generic north-wall card path.
pub(crate) fn bookcase_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_house" || source.metatile_id != 0x1b {
        return None;
    }
    const DRAWING: [[u16; 4]; 4] = [
        [0x0e, 0x0f, 0x0e, 0x0f],
        [0x1e, 0x1f, 0x2e, 0x2f],
        [0x2e, 0x2f, 0x08, 0x09],
        [0x18, 0x19, 0x3a, 0x3b],
    ];
    let origin_column = source.subtile_column / 2 * 2;
    let local_column = source.subtile_column - origin_column;
    let local_row = source.subtile_row;
    (source.tile_index == DRAWING[usize::from(local_row)][usize::from(source.subtile_column)])
        .then_some((local_column, local_row))
}

pub(crate) fn upright_fixture_local(source: &VisualTileSource) -> Option<(u8, u8, usize, usize)> {
    if source.tileset_id.as_ref() != "players_house" {
        return None;
    }
    let (origin_column, origin_row, width, height, expected) = match source.metatile_id {
        0x07 if source.subtile_row >= 2 => {
            let origin = source.subtile_column / 2 * 2;
            const DRAWING: [[u16; 4]; 2] = [[0x50, 0x51, 0x43, 0x45], [0x52, 0x53, 0x18, 0x19]];
            (
                origin,
                2,
                2,
                2,
                DRAWING[usize::from(source.subtile_row - 2)][usize::from(source.subtile_column)],
            )
        }
        0x0f if source.subtile_column >= 2 => {
            const DRAWING: [[u16; 2]; 4] = [[0x25, 0x35], [0x25, 0x35], [0x25, 0x35], [0x25, 0x35]];
            (
                2,
                0,
                2,
                4,
                DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column - 2)],
            )
        }
        0x11 if source.subtile_row >= 1 => {
            let origin = source.subtile_column / 2 * 2;
            const DRAWING: [[u16; 4]; 3] = [
                [0x06, 0x07, 0x11, 0x11],
                [0x16, 0x17, 0x0e, 0x0f],
                [0x08, 0x09, 0x3a, 0x3b],
            ];
            (
                origin,
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

pub(crate) fn tv_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_house"
        || source.metatile_id != 0x13
        || source.subtile_column < 2
    {
        return None;
    }
    if source.subtile_row >= 2 {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x06, 0x07], [0x16, 0x17]];
    let column = source.subtile_column - 2;
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(column)])
        .then_some((column, source.subtile_row))
}

pub(crate) fn console_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_house"
        || source.metatile_id != 0x13
        || source.subtile_column < 2
        || source.subtile_row < 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x4a, 0x4b], [0x5a, 0x5b]];
    let column = source.subtile_column - 2;
    let row = source.subtile_row - 2;
    (source.tile_index == DRAWING[usize::from(row)][usize::from(column)]).then_some((column, row))
}

/// Red's upstairs bed is the complete left-half drawing in block $22.
/// Its art is seen from above, so the mesher keeps it as one shallow sloped
/// surface instead of standing its four source rows upright.
pub(crate) fn bed_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_house"
        || source.metatile_id != 0x22
        || source.subtile_column >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 4] = [[0x28, 0x29], [0x38, 0x39], [0x48, 0x49], [0x58, 0x59]];
    let column = source.subtile_column;
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(column)])
        .then_some((column, source.subtile_row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn source(block: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("players_house"),
            metatile_id: block,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn kitchen_drawings_are_separate_complete_upright_objects() {
        assert_eq!(
            upright_fixture_local(&source(0x07, 0, 2, 0x50)),
            Some((0, 0, 2, 2))
        );
        assert_eq!(
            upright_fixture_local(&source(0x07, 3, 3, 0x19)),
            Some((1, 1, 2, 2))
        );
        assert_eq!(
            upright_fixture_local(&source(0x0f, 2, 0, 0x25)),
            Some((0, 0, 2, 4))
        );
        assert_eq!(
            upright_fixture_local(&source(0x0f, 3, 3, 0x35)),
            Some((1, 3, 2, 4))
        );
        assert_eq!(
            upright_fixture_local(&source(0x11, 0, 1, 0x06)),
            Some((0, 0, 2, 3))
        );
        assert_eq!(
            upright_fixture_local(&source(0x11, 3, 3, 0x3b)),
            Some((1, 2, 2, 3))
        );
    }

    #[test]
    fn block_1b_is_two_separate_complete_bookcases() {
        let drawing = [
            [0x0e, 0x0f, 0x0e, 0x0f],
            [0x1e, 0x1f, 0x2e, 0x2f],
            [0x2e, 0x2f, 0x08, 0x09],
            [0x18, 0x19, 0x3a, 0x3b],
        ];
        for row in 0..4 {
            for column in 0..4 {
                assert_eq!(
                    bookcase_local(&source(
                        0x1b,
                        column,
                        row,
                        drawing[row as usize][column as usize]
                    )),
                    Some((column % 2, row))
                );
            }
        }
        assert_eq!(bookcase_local(&source(0x1b, 2, 1, 0x1f)), None);
    }

    #[test]
    fn red_tv_and_console_are_two_distinct_complete_drawings() {
        assert_eq!(tv_local(&source(0x13, 2, 0, 0x06)), Some((0, 0)));
        assert_eq!(tv_local(&source(0x13, 3, 1, 0x17)), Some((1, 1)));
        assert_eq!(console_local(&source(0x13, 2, 2, 0x4a)), Some((0, 0)));
        assert_eq!(console_local(&source(0x13, 3, 3, 0x5b)), Some((1, 1)));
        assert_eq!(tv_local(&source(0x13, 2, 2, 0x4a)), None);
        assert_eq!(console_local(&source(0x13, 0, 0, 0x01)), None);
    }

    #[test]
    fn red_bed_is_one_complete_top_facing_drawing() {
        assert_eq!(bed_local(&source(0x22, 0, 0, 0x28)), Some((0, 0)));
        assert_eq!(bed_local(&source(0x22, 1, 3, 0x59)), Some((1, 3)));
        assert_eq!(bed_local(&source(0x22, 2, 0, 0x01)), None);
    }

    #[test]
    fn floor_and_partial_lookalikes_are_not_claimed() {
        assert_eq!(upright_fixture_local(&source(0x07, 0, 1, 0x11)), None);
        assert_eq!(upright_fixture_local(&source(0x0f, 1, 0, 0x11)), None);
        assert_eq!(upright_fixture_local(&source(0x11, 0, 0, 0x11)), None);
    }

    #[test]
    fn all_three_house_stairs_use_their_exact_drawing_and_direction() {
        for (block, origin_column, origin_row, drawing, kind) in [
            (0x0f, 0, 1, [[0x0a, 0x0b], [0x1a, 0x1b]], StairKind::UpEast),
            (0x0a, 2, 0, [[0x4c, 0x4d], [0x5c, 0x5d]], StairKind::UpEast),
            (
                0x0b,
                2,
                0,
                [[0x4e, 0x4f], [0x5e, 0x5f]],
                StairKind::DownWest,
            ),
        ] {
            for row in 0..2 {
                for column in 0..2 {
                    let candidate = source(
                        block,
                        origin_column + column,
                        origin_row + row,
                        drawing[row as usize][column as usize],
                    );
                    assert_eq!(stair_local(&candidate), Some((column, row, kind)));
                    let expected = match (kind, column) {
                        (StairKind::UpEast, 0) => (0.0, 8.0),
                        (StairKind::UpEast, 1) => (8.0, 16.0),
                        (StairKind::DownWest, 0) => (-16.0, -8.0),
                        (StairKind::DownWest, 1) => (-8.0, 0.0),
                        _ => unreachable!(),
                    };
                    assert_eq!(
                        stair_shape(&candidate),
                        Some(CellShape::RampEast {
                            west_height: expected.0,
                            east_height: expected.1,
                        })
                    );
                }
            }
        }
        assert_eq!(stair_shape(&source(0x0f, 0, 0, 0x11)), None);
        assert_eq!(stair_shape(&source(0x0f, 2, 1, 0x25)), None);
        assert_eq!(stair_shape(&source(0x0f, 0, 1, 0x0b)), None);
    }
}
