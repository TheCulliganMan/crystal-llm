//! Small interior fixtures that read better as shallow pixel relief.
//!
//! These are deliberately source-art identities, rather than collision
//! classes: a counter in one atlas must not make an unrelated blocked tile
//! rise in another.  The low relief retains Crystal's compact top-down rooms
//! while giving desks, beds, shelves, appliances, and displays a visible lip.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, GROUND_HEIGHT, SolidKind};

const FIXTURE_HEIGHT: f32 = 3.0;

/// The player's bedroom block `$02` carries the upstairs landing in its
/// north-east 2x2 quadrant. Crystal's warp at `(7,0)` uses this exact drawing
/// to descend to 1F; the other twelve cells in the block are wall/floor.
pub(crate) fn player_room_stair_local(
    source: &VisualTileSource,
) -> Option<(u8, u8, crate::players_house::StairKind)> {
    if source.tileset_id.as_ref() != "players_room"
        || source.metatile_id != 0x02
        || source.subtile_column < 2
        || source.subtile_row >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x40, 0x41], [0x50, 0x51]];
    let column = source.subtile_column - 2;
    let row = source.subtile_row;
    (source.tile_index == DRAWING[usize::from(row)][usize::from(column)]).then_some((
        column,
        row,
        crate::players_house::StairKind::DownWest,
    ))
}

pub(crate) fn player_room_stair_shape(source: &VisualTileSource) -> Option<CellShape> {
    let (column, _, _) = player_room_stair_local(source)?;
    let (west_height, east_height) = if column == 0 {
        (-16.0, -8.0)
    } else {
        (-8.0, 0.0)
    };
    Some(CellShape::RampEast {
        west_height,
        east_height,
    })
}

/// Trainer House B1F's only warp sits on this northeast 2x2 stair drawing.
/// Its authored treads rise east; the rest of block `$04` is not staircase.
pub(crate) fn trainer_house_b1f_stair_local(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<(u8, u8)> {
    if map_id != "TrainerHouseB1F"
        || source.tileset_id.as_ref() != "facility"
        || source.metatile_id != 0x04
        || source.subtile_column < 2
        || source.subtile_row >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x10, 0x11], [0x20, 0x21]];
    let column = source.subtile_column - 2;
    (source.tile_index == DRAWING[usize::from(source.subtile_row)][usize::from(column)])
        .then_some((column, source.subtile_row))
}

pub(crate) fn trainer_house_b1f_stair_shape(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<CellShape> {
    let (column, _) = trainer_house_b1f_stair_local(map_id, source)?;
    let (west_height, east_height) = if column == 0 { (0.0, 8.0) } else { (8.0, 16.0) };
    Some(CellShape::RampEast {
        west_height,
        east_height,
    })
}

/// Tilesets whose maps are authored as enclosed rooms. Their northern map
/// edge receives a continuous architectural wall behind all tile artwork.
pub(crate) fn has_back_wall(tileset: &str) -> bool {
    matches!(
        tileset,
        "players_room"
            | "players_house"
            | "house"
            | "traditional_house"
            | "lab"
            | "mart"
            | "pokecenter"
            | "facility"
            | "game_corner"
            | "radio_tower"
            | "lighthouse"
            | "pokecom_center"
            | "mansion"
            | "gate"
            | "train_station"
            | "champions_room"
            | "battle_tower_inside"
            | "tower"
            | "underground"
            | "warehouse"
            | "ship"
    )
}

/// Each decorated bed variant is one 16x32 drawing in the left half of blocks
/// $1b-$1e. Keep the whole outline together so the 2.5D renderer can stand the
/// original sprite at its foot line instead of raising eight little tiles.
pub(crate) fn player_bed_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_room"
        || !matches!(source.metatile_id, 0x1b..=0x1e)
        || source.subtile_column >= 2
    {
        return None;
    }
    let middle = match source.metatile_id {
        0x1b => [[0x13, 0x14], [0x23, 0x24]],
        0x1c => [[0x09, 0x0a], [0x19, 0x1a]],
        0x1d => [[0x29, 0x2a], [0x39, 0x3a]],
        0x1e => [[0x49, 0x4a], [0x59, 0x5a]],
        _ => unreachable!(),
    };
    let expected = match source.subtile_row {
        0 => [0x03, 0x04],
        1 => middle[0],
        2 => middle[1],
        3 => [0x33, 0x34],
        _ => return None,
    };
    (source.tile_index == expected[usize::from(source.subtile_column)])
        .then_some((source.subtile_column, source.subtile_row))
}

/// Resolve the complete player-room appliance drawings packed into blocks
/// $01 and $03. These are front-facing fixtures and belong on thin upright cards,
/// unlike the horizontal bed. The returned dimensions keep adjacent PC,
/// radio, TV, and bookshelf artwork from being fused together. Block $02's
/// north-east drawing is the authored stair landing and is intentionally not
/// returned here; it is consumed by `player_room_stair_local` instead.
pub(crate) fn player_room_fixture_group(
    source: &VisualTileSource,
) -> Option<(u8, u8, usize, usize)> {
    if source.tileset_id.as_ref() != "players_room" {
        return None;
    }
    let (origin_column, origin_row, width, height) = match source.metatile_id {
        0x01 if source.subtile_column >= 2 && source.subtile_row >= 2 => (2, 2, 2, 2),
        0x03 if source.subtile_column < 2 && source.subtile_row >= 1 => (0, 1, 2, 3),
        0x03 if source.subtile_column >= 2 => (2, 0, 2, 4),
        _ => return None,
    };
    Some((
        source.subtile_column - origin_column,
        source.subtile_row - origin_row,
        width,
        height,
    ))
}

/// The bedroom PC is drawn as a 16x16 monitor above a separate 16x8
/// keyboard/desk strip in block $01. Keeping these identities separate lets
/// the mesher stand the monitor on a shallow horizontal control surface
/// instead of leaning the entire 16x24 drawing against the wall.
pub(crate) fn player_room_pc_monitor_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_room"
        || source.metatile_id != 0x01
        || source.subtile_column >= 2
        || !(1..=2).contains(&source.subtile_row)
    {
        return None;
    }
    const DRAWING: [[u16; 2]; 2] = [[0x0b, 0x0c], [0x1b, 0x1c]];
    let row = source.subtile_row - 1;
    (source.tile_index == DRAWING[usize::from(row)][usize::from(source.subtile_column)])
        .then_some((source.subtile_column, row))
}

pub(crate) fn player_room_pc_keyboard_local(source: &VisualTileSource) -> Option<(u8, u8)> {
    if source.tileset_id.as_ref() != "players_room"
        || source.metatile_id != 0x01
        || source.subtile_column >= 2
        || source.subtile_row != 3
    {
        return None;
    }
    const DRAWING: [u16; 2] = [0x2b, 0x2c];
    (source.tile_index == DRAWING[usize::from(source.subtile_column)])
        .then_some((source.subtile_column, 0))
}

/// Mr. Pokémon's right-hand work counter is one complete 32x16 top-view
/// drawing. It belongs on a single shallow horizontal surface rather than
/// four collision-shaped boxes.
pub(crate) fn mr_pokemon_work_counter_local(
    map_id: &str,
    source: &VisualTileSource,
) -> Option<(u8, u8)> {
    if map_id != "MrPokemonsHouse"
        || source.tileset_id.as_ref() != "facility"
        || source.metatile_id != 0x28
        || source.subtile_row >= 2
    {
        return None;
    }
    const DRAWING: [[u16; 4]; 2] = [[0x02, 0x03, 0x04, 0x05], [0x12, 0x13, 0x14, 0x15]];
    (source.tile_index
        == DRAWING[usize::from(source.subtile_row)][usize::from(source.subtile_column)])
    .then_some((source.subtile_column, source.subtile_row))
}

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
        ("players_room", 0x1f)
            if source.subtile_column >= 2
                && source.subtile_row < 2
                && matches!(tile, 0x40 | 0x41 | 0x50 | 0x51) =>
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
            if player_room_fixture_group(source).is_some() {
                CellShape::Flat
            } else {
                fixture(0x02)
            }
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
            if player_bed_local(source).is_some() {
                // A bed is viewed from above. Keep its complete source art on
                // one shallow plane parallel to the room floor so camera
                // perspective applies naturally, with no generated sides.
                CellShape::PlaneAt { height: 0.75 }
            } else {
                fixture(0x01)
            }
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
    fn mr_pokemon_work_counter_is_one_map_scoped_four_by_two_drawing() {
        let drawing = [[0x02, 0x03, 0x04, 0x05], [0x12, 0x13, 0x14, 0x15]];
        for row in 0..2_u8 {
            for column in 0..4_u8 {
                let mut cell = source(
                    "facility",
                    0x28,
                    drawing[usize::from(row)][usize::from(column)],
                );
                cell.subtile_column = column;
                cell.subtile_row = row;
                assert_eq!(
                    mr_pokemon_work_counter_local("MrPokemonsHouse", &cell),
                    Some((column, row))
                );
                assert_eq!(mr_pokemon_work_counter_local("ElmsLab", &cell), None);
            }
        }
    }

    #[test]
    fn trainer_house_basement_stair_is_exact_and_map_scoped() {
        let drawing = [[0x10, 0x11], [0x20, 0x21]];
        for row in 0..2_u8 {
            for column in 0..2_u8 {
                let mut cell = source(
                    "facility",
                    0x04,
                    drawing[usize::from(row)][usize::from(column)],
                );
                cell.subtile_column = column + 2;
                cell.subtile_row = row;
                assert_eq!(
                    trainer_house_b1f_stair_local("TrainerHouseB1F", &cell),
                    Some((column, row))
                );
                assert_eq!(
                    trainer_house_b1f_stair_local("MrPokemonsHouse", &cell),
                    None
                );
                assert!(matches!(
                    trainer_house_b1f_stair_shape("TrainerHouseB1F", &cell),
                    Some(CellShape::RampEast { .. })
                ));
            }
        }
    }

    #[test]
    fn player_bedroom_stairwell_is_the_exact_northeast_two_by_two_drawing() {
        let drawing = [[0x40, 0x41], [0x50, 0x51]];
        for row in 0..2 {
            for column in 0..2 {
                let mut cell = source("players_room", 0x02, drawing[row as usize][column as usize]);
                cell.subtile_column = column + 2;
                cell.subtile_row = row;
                assert_eq!(
                    player_room_stair_local(&cell),
                    Some((column, row, crate::players_house::StairKind::DownWest))
                );
                assert!(matches!(
                    player_room_stair_shape(&cell),
                    Some(CellShape::RampEast { .. })
                ));
                assert_eq!(
                    player_room_fixture_group(&cell),
                    None,
                    "the stair landing must not also become an upright appliance"
                );
            }
        }
        assert_eq!(
            player_room_stair_local(&source("players_room", 0x02, 0x02)),
            None
        );
        assert_eq!(
            player_room_stair_local(&source("players_room", 0x01, 0x40)),
            None
        );
    }

    #[test]
    fn player_room_pc_separates_monitor_from_keyboard_surface() {
        let monitor = [[0x0b, 0x0c], [0x1b, 0x1c]];
        for row in 0..2 {
            for column in 0..2 {
                let mut cell = source("players_room", 0x01, monitor[row][column]);
                cell.subtile_column = column as u8;
                cell.subtile_row = row as u8 + 1;
                assert_eq!(
                    player_room_pc_monitor_local(&cell),
                    Some((column as u8, row as u8))
                );
                assert_eq!(player_room_fixture_group(&cell), None);
            }
        }
        for column in 0..2 {
            let mut cell = source("players_room", 0x01, [0x2b, 0x2c][column]);
            cell.subtile_column = column as u8;
            cell.subtile_row = 3;
            assert_eq!(
                player_room_pc_keyboard_local(&cell),
                Some((column as u8, 0))
            );
            assert_eq!(player_room_fixture_group(&cell), None);
        }
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

    #[test]
    fn every_decorated_bed_variant_is_one_two_by_four_card() {
        let middles = [
            (0x1b, [[0x13, 0x14], [0x23, 0x24]]),
            (0x1c, [[0x09, 0x0a], [0x19, 0x1a]]),
            (0x1d, [[0x29, 0x2a], [0x39, 0x3a]]),
            (0x1e, [[0x49, 0x4a], [0x59, 0x5a]]),
        ];
        for (metatile, middle) in middles {
            let rows = [[0x03, 0x04], middle[0], middle[1], [0x33, 0x34]];
            for row in 0..4 {
                for column in 0..2 {
                    let cell = source("players_room", metatile, rows[row][column]);
                    let cell = VisualTileSource {
                        subtile_column: column as u8,
                        subtile_row: row as u8,
                        ..cell
                    };
                    assert_eq!(player_bed_local(&cell), Some((column as u8, row as u8)));
                }
            }
        }
    }

    #[test]
    fn enclosed_room_tilesets_receive_architectural_back_walls() {
        for tileset in [
            "players_room",
            "house",
            "mart",
            "pokecenter",
            "game_corner",
            "train_station",
            "warehouse",
            "ship",
        ] {
            assert!(has_back_wall(tileset), "{tileset}");
        }
        for tileset in ["johto", "kanto", "park", "forest"] {
            assert!(!has_back_wall(tileset), "{tileset}");
        }
    }
}
