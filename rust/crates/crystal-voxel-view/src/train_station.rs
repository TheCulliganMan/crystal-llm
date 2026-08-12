//! Authored, presentation-only fixtures from Crystal's Magnet Train stations.

use crystal_render_api::VisualTileSource;

pub(crate) const FLOOR_TILE: u16 = 0x3d;

pub(crate) fn supports_map(map_id: &str) -> bool {
    matches!(
        map_id,
        "GoldenrodMagnetTrainStation" | "SaffronMagnetTrainStation"
    )
}

/// Metatile $01 contains two separate 16x16 waiting-room seats. Keep each
/// native drawing as an independent, zero-depth upright card; joining the
/// four columns makes a bench that Crystal never draws, while extrusion makes
/// the face-on cushion art into a box.
pub(crate) fn seat_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !supports_map(map_id)
        || source.tileset_id.as_ref() != "train_station"
        || source.metatile_id != 0x01
        || source.subtile_row >= 2
    {
        return None;
    }
    let local_column = source.subtile_column % 2;
    const DRAWING: [[u16; 2]; 2] = [[0x40, 0x41], [0x42, 0x43]];
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(local_column)])
        .then_some((local_column, source.subtile_row))
}

/// The left half of $0c is one 16x32 station planter. Its repeated middle
/// foliage is part of the single authored drawing, not evidence for stacked
/// cubes, so retain it as one masked upright card over the station floor.
pub(crate) fn planter_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !supports_map(map_id)
        || source.tileset_id.as_ref() != "train_station"
        || source.metatile_id != 0x0c
        || source.subtile_column >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 4] = [[0x50, 0x51], [0x52, 0x53], [0x52, 0x53], [0x54, 0x55]];
    (source.tile_index
        == DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)])
    .then_some((source.subtile_column, source.subtile_row))
}

/// Blocks $07 and $08 each carry the same narrow 16x32 station gate on their
/// left half. Preserve each occurrence as its own thin face-on fixture; the
/// right halves are unrelated floor/rail art and must not be claimed.
pub(crate) fn gate_local(map_id: &str, source: &VisualTileSource) -> Option<(u8, u8)> {
    if !supports_map(map_id)
        || source.tileset_id.as_ref() != "train_station"
        || !matches!(source.metatile_id, 0x07 | 0x08)
        || source.subtile_column >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 4] = [[0x35, 0x36], [0x37, 0x38], [0x39, 0x3a], [0x3b, 0x3c]];
    (source.tile_index
        == DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)])
    .then_some((source.subtile_column, source.subtile_row))
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
    fn each_half_of_block_one_is_one_complete_seat() {
        for half in 0..2 {
            for row in 0..2 {
                for column in 0..2 {
                    let drawing = [[0x40, 0x41], [0x42, 0x43]];
                    assert_eq!(
                        seat_local(
                            "GoldenrodMagnetTrainStation",
                            &source(
                                "train_station",
                                0x01,
                                half * 2 + column,
                                row,
                                drawing[usize::from(row)][usize::from(column)],
                            ),
                        ),
                        Some((column, row))
                    );
                }
            }
        }
    }

    #[test]
    fn seat_identity_is_map_tileset_and_art_scoped() {
        let seat = source("train_station", 0x01, 0, 0, 0x40);
        assert_eq!(seat_local("SaffronMagnetTrainStation", &seat), Some((0, 0)));
        assert_eq!(seat_local("GoldenrodCity", &seat), None);
        assert_eq!(
            seat_local(
                "GoldenrodMagnetTrainStation",
                &source("train_station", 0x03, 0, 0, 0x40),
            ),
            None
        );
        assert_eq!(
            seat_local(
                "GoldenrodMagnetTrainStation",
                &source("mart", 0x01, 0, 0, 0x40),
            ),
            None
        );
    }

    #[test]
    fn planter_keeps_one_complete_two_by_four_drawing() {
        let drawing = [[0x50, 0x51], [0x52, 0x53], [0x52, 0x53], [0x54, 0x55]];
        for row in 0..4 {
            for column in 0..2 {
                assert_eq!(
                    planter_local(
                        "GoldenrodMagnetTrainStation",
                        &source(
                            "train_station",
                            0x0c,
                            column,
                            row,
                            drawing[usize::from(row)][usize::from(column)],
                        ),
                    ),
                    Some((column, row))
                );
            }
        }
        assert_eq!(
            planter_local(
                "GoldenrodMagnetTrainStation",
                &source("train_station", 0x0c, 2, 0, 0x3d),
            ),
            None
        );
    }

    #[test]
    fn each_station_gate_is_one_scoped_two_by_four_card() {
        let drawing = [[0x35, 0x36], [0x37, 0x38], [0x39, 0x3a], [0x3b, 0x3c]];
        for metatile in [0x07, 0x08] {
            for row in 0..4 {
                for column in 0..2 {
                    assert_eq!(
                        gate_local(
                            "SaffronMagnetTrainStation",
                            &source(
                                "train_station",
                                metatile,
                                column,
                                row,
                                drawing[usize::from(row)][usize::from(column)],
                            ),
                        ),
                        Some((column, row))
                    );
                }
            }
        }
        assert_eq!(
            gate_local(
                "GoldenrodMagnetTrainStation",
                &source("train_station", 0x07, 2, 0, 0x3e),
            ),
            None
        );
    }
}
