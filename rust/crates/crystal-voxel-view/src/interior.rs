//! Small interior fixtures that read better as shallow pixel relief.
//!
//! These are deliberately source-art identities, rather than collision
//! classes: a counter in one atlas must not make an unrelated blocked tile
//! rise in another.  The low relief retains Crystal's compact top-down rooms
//! while giving desks, beds, shelves, appliances, and displays a visible lip.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, GROUND_HEIGHT, SolidKind};

const FIXTURE_HEIGHT: f32 = 3.0;

fn fixture(ground_tile_index: u16) -> CellShape {
    CellShape::Relief {
        height: FIXTURE_HEIGHT,
        ground_tile_index,
        base_height: GROUND_HEIGHT,
    }
}

/// Returns relief only for complete, authored small-fixture source cells.
pub(crate) fn interior_fixture_shape(source: &VisualTileSource) -> Option<CellShape> {
    let tile = source.tile_index;
    let shape = match (source.tileset_id.as_ref(), source.metatile_id) {
        // The default bedroom decoration is the Town Map poster. Its two
        // authored rows are wall art, not a floor fixture; fold the complete
        // 16x16 drawing onto the north wall so it remains readable in 2.5D.
        ("players_room", 0x1f)
            if source.subtile_column < 2
                && source.subtile_row < 2
                && matches!(tile, 0x44 | 0x45 | 0x54 | 0x55) =>
        {
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: source.subtile_row,
                band_count: 2,
                ground_tile_index: 0x01,
                solid: SolidKind::Prop,
            }
        }
        // Player homes: PCs, TVs, radios, bookcases, the bedroom furniture,
        // and the Virtual Boy all use the same wood-floor datum.
        ("players_house", 0x10 | 0x18 | 0x1a | 0x1f)
            if matches!(
                tile,
                0x0c | 0x0d | 0x20 | 0x21 | 0x26 | 0x27 | 0x30 | 0x31 | 0x36 | 0x37
            ) =>
        {
            fixture(0x11)
        }
        ("players_house", 0x11 | 0x13 | 0x21 | 0x02) if matches!(tile, 0x06..=0x09 | 0x0e | 0x0f | 0x16..=0x19 | 0x1e | 0x1f | 0x2e | 0x2f | 0x3a | 0x3b) => {
            fixture(0x11)
        }
        ("players_house", 0x1b)
            if matches!(tile, 0x0e | 0x0f | 0x18 | 0x19 | 0x1e | 0x1f | 0x2e | 0x2f) =>
        {
            fixture(0x11)
        }
        ("players_house", 0x1e) if matches!(tile, 0x46 | 0x47 | 0x56 | 0x57) => fixture(0x11),
        ("players_room", 0x03)
            if matches!(
                tile,
                0x05 | 0x06
                    | 0x15
                    | 0x16
                    | 0x25
                    | 0x26
                    | 0x35
                    | 0x36
                    | 0x3b
                    | 0x3c
                    | 0x4b
                    | 0x4c
                    | 0x5b
                    | 0x5c
            ) =>
        {
            fixture(0x02)
        }
        // The bedroom-only metatiles carry the bed, desk, cabinet, wall
        // pictures, and rug-edge furniture.  Their source cells are kept as
        // low relief instead of incorrectly turning a bedroom into a box.
        ("players_room", 0x1b..=0x1e)
            if matches!(
                tile,
                0x03 | 0x04
                    | 0x09
                    | 0x0a
                    | 0x13
                    | 0x14
                    | 0x19
                    | 0x1a
                    | 0x23
                    | 0x24
                    | 0x29
                    | 0x2a
                    | 0x33
                    | 0x34
                    | 0x39
                    | 0x3a
                    | 0x49
                    | 0x4a
                    | 0x59
                    | 0x5a
            ) =>
        {
            fixture(0x01)
        }
        ("players_house", 0x07..=0x09 | 0x0c | 0x0d | 0x0f | 0x12 | 0x14..=0x17) if matches!(tile, 0x02 | 0x03 | 0x0a | 0x0b | 0x12 | 0x13 | 0x15 | 0x1a | 0x1b | 0x22..=0x25 | 0x2a | 0x2b | 0x32..=0x35 | 0x40 | 0x43 | 0x45 | 0x4a | 0x4b | 0x50..=0x53 | 0x5a | 0x5b) => {
            fixture(0x01)
        }

        // Generic homes and traditional houses: domestic appliances,
        // bookcases, town maps, incense tables, and shop displays.
        ("house", 0x10 | 0x1c) if matches!(tile, 0x20 | 0x21 | 0x30 | 0x31 | 0x40..=0x43) => {
            fixture(0x01)
        }
        ("house", 0x04 | 0x09 | 0x1a)
            if matches!(tile, 0x0e | 0x0f | 0x1e | 0x1f | 0x30..=0x33 | 0x3b | 0x3c) =>
        {
            fixture(0x01)
        }
        ("house", 0x1d | 0x1e)
            if matches!(
                tile,
                0x06 | 0x07 | 0x0c | 0x0d | 0x16 | 0x17 | 0x1c | 0x1d | 0x2d | 0x2e | 0x3d | 0x3e
            ) =>
        {
            fixture(0x01)
        }
        ("traditional_house", 0x12 | 0x13 | 0x1a | 0x23) if matches!(tile, 0x0a | 0x0b | 0x11 | 0x18 | 0x19 | 0x1a | 0x1b | 0x23 | 0x24 | 0x29 | 0x2a | 0x33 | 0x34 | 0x39 | 0x3a | 0x3e | 0x3f | 0x42..=0x46 | 0x52..=0x56) => {
            fixture(0x50)
        }
        ("traditional_house", 0x02)
            if matches!(
                tile,
                0x06 | 0x07 | 0x16 | 0x17 | 0x20 | 0x21 | 0x27 | 0x28 | 0x30 | 0x31 | 0x37 | 0x38
            ) =>
        {
            fixture(0x50)
        }
        ("traditional_house", 0x09..=0x19) if matches!(tile, 0x02 | 0x03 | 0x12 | 0x13 | 0x15 | 0x22..=0x24 | 0x25 | 0x26 | 0x29 | 0x2a | 0x2b | 0x2c | 0x32..=0x34 | 0x39 | 0x3a | 0x3b | 0x3c | 0x42 | 0x43 | 0x47..=0x5a) => {
            fixture(0x50)
        }

        // Public interiors: counters and merchandise are intentionally only
        // a few source pixels tall, so they remain readable gameplay spaces.
        ("lab", 0x04 | 0x14) if matches!(tile, 0x03..=0x05 | 0x07 | 0x13 | 0x14 | 0x35 | 0x36) => {
            fixture(0x10)
        }
        (
            "mart",
            0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x17 | 0x1a | 0x1b | 0x22 | 0x23 | 0x27
            | 0x28 | 0x2c | 0x2d | 0x2e,
        ) if matches!(tile, 0x0c | 0x0d | 0x18..=0x1d | 0x1e | 0x1f | 0x26..=0x2b | 0x2e | 0x2f | 0x3a | 0x3b | 0x3e | 0x3f | 0x40..=0x45 | 0x50..=0x5f) => {
            fixture(0x01)
        }
        ("pokecenter", 0x05..=0x08 | 0x21 | 0x30 | 0x32)
            if matches!(
                tile,
                0x0c | 0x0d
                    | 0x13
                    | 0x20
                    | 0x21
                    | 0x30
                    | 0x31
                    | 0x34
                    | 0x35
                    | 0x36
                    | 0x37
                    | 0x40
                    | 0x41
                    | 0x46
                    | 0x47
                    | 0x4a
                    | 0x4b
                    | 0x5a
                    | 0x5b
            ) =>
        {
            fixture(0x11)
        }
        ("facility", 0x06 | 0x1e | 0x31 | 0x3f) if matches!(tile, 0x1a | 0x1b | 0x28 | 0x29 | 0x2a | 0x2b | 0x40..=0x42 | 0x4c..=0x4e | 0x50 | 0x52) => {
            fixture(0x26)
        }
        // Remaining public interiors: counters, terminals, bookcases, and
        // displays get only a small lip rather than architectural volume.
        (
            "game_corner",
            0x07..=0x0b | 0x10 | 0x11 | 0x13 | 0x14 | 0x16..=0x18 | 0x1b | 0x1c | 0x27,
        ) if matches!(tile, 0x01 | 0x02 | 0x04 | 0x05 | 0x0e | 0x10..=0x12 | 0x14 | 0x15 | 0x1e | 0x1f | 0x34 | 0x35 | 0x3d | 0x40 | 0x41 | 0x4d | 0x51 | 0x80..=0xc4) => {
            fixture(0x02)
        }
        // The Goldenrod layout owns extra machine bays. They use later atlas
        // slots ($89-$9c and $a4-$b5) and must not stay painted into the
        // floor merely because Celadon's common cabinet stops at $a3.
        ("game_corner", 0x2b | 0x2f | 0x30)
            if matches!(tile, 0x89..=0x9c | 0xa4 | 0xa5 | 0xb4 | 0xb5) =>
        {
            fixture(0x02)
        }
        (
            "radio_tower",
            0x05
            | 0x06
            | 0x08..=0x0a
            | 0x0e
            | 0x0f
            | 0x10
            | 0x12
            | 0x17
            | 0x19
            | 0x1a
            | 0x1b
            | 0x21
            | 0x22
            | 0x24..=0x26
            | 0x2b
            | 0x2d
            | 0x2e
            | 0x38,
        ) if matches!(tile, 0x05..=0x09 | 0x0a | 0x0b | 0x15..=0x17 | 0x1a | 0x1b | 0x20..=0x26 | 0x29..=0x2f | 0x30..=0x33 | 0x35 | 0x36 | 0x3a..=0x3f | 0x42 | 0x43 | 0x4c..=0x4f | 0x50..=0x53 | 0x56..=0x5f | 0x8c) => {
            fixture(0x01)
        }
        ("lighthouse", 0x2f)
            if matches!(tile, 0x02 | 0x03 | 0x13 | 0x3e | 0x48 | 0x49 | 0x58 | 0x59) =>
        {
            fixture(0x01)
        }
        ("lighthouse", 0x34)
            if matches!(
                tile,
                0x02 | 0x03 | 0x0d | 0x10 | 0x12 | 0x1d | 0x2d | 0x3d | 0x52 | 0x53 | 0x5c | 0x5d
            ) =>
        {
            fixture(0x01)
        }
        ("pokecom_center", 0x07 | 0x09 | 0x0a)
            if matches!(
                tile,
                0x0c | 0x20
                    | 0x21
                    | 0x24
                    | 0x30
                    | 0x31
                    | 0x34
                    | 0x40
                    | 0x41
                    | 0x4a
                    | 0x4b
                    | 0x5a
                    | 0x5b
            ) =>
        {
            fixture(0x11)
        }
        ("mansion", 0x0f | 0x11 | 0x18) if matches!(tile, 0x03 | 0x13 | 0x22..=0x25 | 0x2a | 0x2b | 0x2e | 0x2f | 0x32..=0x36 | 0x38 | 0x3a | 0x3b | 0x5e | 0x5f) => {
            fixture(0x01)
        }
        (
            "gate",
            0x08
            | 0x09
            | 0x0b..=0x0f
            | 0x10
            | 0x13
            | 0x21
            | 0x28
            | 0x2b
            | 0x2c
            | 0x30
            | 0x32..=0x34
            | 0x3c
            | 0x3e
            | 0x3f,
        ) if matches!(tile, 0x07..=0x09 | 0x0e | 0x0f | 0x15 | 0x16 | 0x18 | 0x19 | 0x20 | 0x21 | 0x24 | 0x25 | 0x30..=0x34 | 0x38 | 0x39 | 0x45 | 0x48..=0x4b | 0x4e | 0x4f | 0x54..=0x57) => {
            fixture(0x01)
        }
        ("champions_room", 0x05..=0x07) if matches!(tile, 0x07..=0x09 | 0x11 | 0x17..=0x19 | 0x20 | 0x21 | 0x25..=0x27 | 0x30 | 0x31 | 0x35) => {
            fixture(0x11)
        }
        ("battle_tower_inside", 0x28) if matches!(tile, 0x0a | 0x1a | 0x1b | 0x2a | 0x2b) => {
            fixture(0x01)
        }
        ("underground", 0x3d) if matches!(tile, 0x1a..=0x1d | 0x40 | 0x42) => fixture(0x10),
        // Celadon and Viridian Gyms reuse the train-station atlas's compact
        // ornament row: pots/barrels, statues, railing ends, and wall-mounted
        // gym fixtures. Flooring remains flat under the raised art.
        ("train_station", 0x19..=0x23) if !matches!(tile, 0x30 | 0x31 | 0x3d | 0x56 | 0x57) => {
            fixture(0x3d)
        }
        ("train_station", 0x31..=0x3f) if matches!(tile, 0x44..=0x47 | 0x48..=0x5b | 0x80..=0x83) => {
            fixture(0x3d)
        }
        _ => return None,
    };
    Some(shape)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(tileset: &str, metatile: u16, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset),
            metatile_id: metatile,
            subtile_column: 0,
            subtile_row: 0,
            tile_index: tile,
        }
    }

    #[test]
    fn small_domestic_and_public_fixtures_become_shallow_relief() {
        for source in [
            source("players_house", 0x1e, 0x46),
            source("players_room", 0x03, 0x25),
            source("players_room", 0x1d, 0x39),
            source("players_house", 0x07, 0x43),
            source("house", 0x1d, 0x1c),
            source("traditional_house", 0x13, 0x3e),
            source("traditional_house", 0x11, 0x4b),
            source("lab", 0x14, 0x35),
            source("mart", 0x14, 0x58),
            source("pokecenter", 0x21, 0x4a),
            source("facility", 0x06, 0x28),
            source("game_corner", 0x10, 0x4d),
            source("game_corner", 0x0b, 0xc3),
            source("game_corner", 0x2f, 0x99),
            source("radio_tower", 0x24, 0x50),
            source("lighthouse", 0x34, 0x52),
            source("pokecom_center", 0x07, 0x20),
            source("mansion", 0x11, 0x5e),
            source("gate", 0x2c, 0x4a),
            source("champions_room", 0x05, 0x25),
            source("battle_tower_inside", 0x28, 0x2a),
            source("underground", 0x3d, 0x40),
            source("train_station", 0x1d, 0x4a),
        ] {
            assert!(matches!(
                interior_fixture_shape(&source),
                Some(CellShape::Relief {
                    height: FIXTURE_HEIGHT,
                    ..
                })
            ));
        }
    }

    #[test]
    fn shared_graphics_slots_do_not_promote_unrelated_metatiles() {
        assert_eq!(interior_fixture_shape(&source("house", 0x00, 0x20)), None);
        assert_eq!(interior_fixture_shape(&source("johto", 0x10, 0x20)), None);
    }

    #[test]
    fn bedroom_town_map_is_a_complete_wall_card() {
        for (column, row, tile) in [(0, 0, 0x44), (1, 0, 0x45), (0, 1, 0x54), (1, 1, 0x55)] {
            let mut poster = source("players_room", 0x1f, tile);
            poster.subtile_column = column;
            poster.subtile_row = row;
            assert_eq!(
                interior_fixture_shape(&poster),
                Some(CellShape::FacadeBand {
                    plane_subtile_row: 2,
                    band_from_top: row,
                    band_count: 2,
                    ground_tile_index: 0x01,
                    solid: SolidKind::Prop,
                })
            );
        }
    }
}
