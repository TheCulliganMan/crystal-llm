use serde::{Deserialize, Serialize};

use super::map::{
    Direction, METATILE_WIDTH, OverworldMapData, TilePosition, determine_quadrant_index,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Terrain {
    Land,
    Water,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum PlayerTraversalState {
    Walk,
    Surf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollisionAttributes {
    pub value: u8,
    pub terrain: Terrain,
    pub talk: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetatileCollision {
    pub collision: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TilesetCollision {
    pub metatiles: Vec<MetatileCollision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollisionSample {
    pub permission: u8,
    pub metatile_id: u16,
    pub quadrant: usize,
    pub tile: TilePosition,
}

pub mod permissions {
    pub const FLOOR: u8 = 0x00;
    pub const WALL: u8 = 0x07;
    pub const CUT_GRASS: u8 = 0x08;
    pub const TALL_GRASS_10: u8 = 0x10;
    pub const LONG_GRASS: u8 = 0x14;
    pub const HEADBUTT_TREE: u8 = 0x15;
    pub const HEADBUTT_TREE_1D: u8 = 0x1d;
    pub const TALL_GRASS: u8 = 0x18;
    pub const LONG_GRASS_1C: u8 = 0x1c;
    pub const CUT_GRASS_28: u8 = 0x28;
    pub const GRASS_48: u8 = 0x48;
    pub const GRASS_49: u8 = 0x49;
    pub const GRASS_4A: u8 = 0x4a;
    pub const GRASS_4B: u8 = 0x4b;
    pub const GRASS_4C: u8 = 0x4c;
    pub const PIT: u8 = 0x60;
    pub const PIT_68: u8 = 0x68;
    pub const WARP_CARPET_DOWN: u8 = 0x70;
    pub const DOOR: u8 = 0x71;
    pub const LADDER: u8 = 0x72;
    pub const STAIRCASE_73: u8 = 0x73;
    pub const CAVE_74: u8 = 0x74;
    pub const DOOR_75: u8 = 0x75;
    pub const WARP_CARPET_LEFT: u8 = 0x76;
    pub const WARP_77: u8 = 0x77;
    pub const WARP_CARPET_UP: u8 = 0x78;
    pub const DOOR_79: u8 = 0x79;
    pub const STAIRCASE: u8 = 0x7a;
    pub const CAVE: u8 = 0x7b;
    pub const WARP_PANEL: u8 = 0x7c;
    pub const DOOR_7D: u8 = 0x7d;
    pub const WARP_CARPET_RIGHT: u8 = 0x7e;
    pub const WARP_7F: u8 = 0x7f;
    pub const COUNTER: u8 = 0x90;
    pub const BOOKSHELF: u8 = 0x91;
    pub const PC: u8 = 0x93;
    pub const RADIO: u8 = 0x94;
    pub const TOWN_MAP: u8 = 0x95;
    pub const MART_SHELF: u8 = 0x96;
    pub const TV: u8 = 0x97;
    pub const COUNTER_98: u8 = 0x98;
    pub const WINDOW: u8 = 0x9d;
    pub const INCENSE_BURNER: u8 = 0x9f;
    pub const WATER: u8 = 0x29;
    pub const WHIRLPOOL: u8 = 0x24;
    pub const WHIRLPOOL_2C: u8 = 0x2c;
    pub const WATERFALL_RIGHT: u8 = 0x30;
    pub const WATERFALL_LEFT: u8 = 0x31;
    pub const WATERFALL_UP: u8 = 0x32;
    pub const WATERFALL: u8 = 0x33;
    pub const CURRENT_DOWN: u8 = 0x3b;
    pub const CURRENT_RIGHT: u8 = 0x38;
    pub const CURRENT_LEFT: u8 = 0x39;
    pub const CURRENT_UP: u8 = 0x3a;
    pub const ICE: u8 = 0x23;
    pub const ICE_2B: u8 = 0x2b;
    pub const WALK_RIGHT: u8 = 0x41;
    pub const WALK_LEFT: u8 = 0x42;
    pub const WALK_UP: u8 = 0x43;
    pub const WALK_DOWN: u8 = 0x44;
    pub const WALK_RIGHT_ALT: u8 = 0x50;
    pub const WALK_LEFT_ALT: u8 = 0x51;
    pub const WALK_UP_ALT: u8 = 0x52;
    pub const WALK_DOWN_ALT: u8 = 0x53;
    pub const HOP_RIGHT: u8 = 0xa0;
    pub const HOP_LEFT: u8 = 0xa1;
    pub const HOP_UP: u8 = 0xa2;
    pub const HOP_DOWN: u8 = 0xa3;
    pub const HOP_DOWN_RIGHT: u8 = 0xa4;
    pub const HOP_DOWN_LEFT: u8 = 0xa5;
    pub const HOP_UP_RIGHT: u8 = 0xa6;
    pub const HOP_UP_LEFT: u8 = 0xa7;
    pub const RIGHT_WALL: u8 = 0xb0;
    pub const LEFT_WALL: u8 = 0xb1;
    pub const UP_WALL: u8 = 0xb2;
    pub const DOWN_WALL: u8 = 0xb3;
    pub const DOWN_RIGHT_WALL: u8 = 0xb4;
    pub const DOWN_LEFT_WALL: u8 = 0xb5;
    pub const UP_RIGHT_WALL: u8 = 0xb6;
    pub const UP_LEFT_WALL: u8 = 0xb7;
    pub const RIGHT_BUOY: u8 = 0xc0;
    pub const LEFT_BUOY: u8 = 0xc1;
    pub const UP_BUOY: u8 = 0xc2;
    pub const DOWN_BUOY: u8 = 0xc3;
}

pub const fn is_grass_encounter_permission(permission: u8) -> bool {
    // Exact CheckGrassCollision.blocks entries except COLL_WATER, which the
    // encounter-surface resolver handles first as the water table.
    matches!(
        permission,
        permissions::CUT_GRASS
            | permissions::LONG_GRASS
            | permissions::TALL_GRASS
            | permissions::CUT_GRASS_28
            | permissions::GRASS_48
            | permissions::GRASS_49
            | permissions::GRASS_4A
            | permissions::GRASS_4B
            | permissions::GRASS_4C
    )
}

pub const fn spawns_shaking_grass_object(permission: u8) -> bool {
    matches!(
        permission,
        permissions::LONG_GRASS | permissions::LONG_GRASS_1C
    ) || (matches!(permission & 0xf0, 0x10 | 0x20) && permission & 0x07 == 0)
}

pub fn is_warp_permission(permission: u8) -> bool {
    permission == permissions::PIT
        || permission == permissions::PIT_68
        || (permission & 0xf0) == permissions::WARP_CARPET_DOWN
}

pub const fn directional_warp_facing(permission: u8) -> Option<Direction> {
    match permission {
        permissions::WARP_CARPET_DOWN => Some(Direction::Down),
        permissions::WARP_CARPET_UP => Some(Direction::Up),
        permissions::WARP_CARPET_LEFT => Some(Direction::Left),
        permissions::WARP_CARPET_RIGHT => Some(Direction::Right),
        _ => None,
    }
}

/// Return the standard script dispatched by Crystal when A is pressed while
/// facing an interactive collision tile.  These are not map background events:
/// they are defined globally in `data/collision/collision_stdscripts.asm`.
pub fn standard_interaction_script(permission: u8) -> Option<&'static str> {
    match permission {
        permissions::BOOKSHELF => Some("MagazineBookshelfScript"),
        permissions::PC => Some("PCScript"),
        permissions::RADIO => Some("Radio1Script"),
        permissions::TOWN_MAP => Some("TownMapScript"),
        permissions::MART_SHELF => Some("MerchandiseShelfScript"),
        permissions::TV => Some("TVScript"),
        permissions::WINDOW => Some("WindowScript"),
        permissions::INCENSE_BURNER => Some("IncenseBurnerScript"),
        _ => None,
    }
}

pub fn is_standard_interaction_script(script: &str) -> bool {
    [
        "MagazineBookshelfScript",
        "PCScript",
        "Radio1Script",
        "TownMapScript",
        "MerchandiseShelfScript",
        "TVScript",
        "WindowScript",
        "IncenseBurnerScript",
    ]
    .contains(&script)
}

pub fn describe_collision(permission: u8) -> CollisionAttributes {
    CollisionAttributes {
        value: permission,
        terrain: match permission {
            // `data/collision/collision_permissions.asm` is the authority.
            // Side walls deliberately remain land: their direction masks,
            // not their terrain class, decide which approaches are blocked.
            0x07
            | 0x0f
            | 0x12
            | 0x15
            | 0x1a
            | 0x1d
            | 0x27
            | 0x2f
            | 0x62
            | 0x6a
            | 0x80..=0x84
            | 0x88..=0x8c
            | 0x90..=0x9f
            | 0xff => Terrain::Wall,
            0x20..=0x22 | 0x24..=0x26 | 0x28..=0x2a | 0x2c..=0x2e | 0x30..=0x3f | 0xc0..=0xcf => {
                Terrain::Water
            }
            _ => Terrain::Land,
        },
        talk: matches!(
            permission,
            0x12 | 0x15 | 0x1a | 0x1d | 0x22 | 0x24 | 0x2a | 0x2c
        ),
    }
}

pub fn sample_collision(
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    tile: TilePosition,
) -> Option<CollisionSample> {
    if tile.x < 0 || tile.y < 0 {
        return None;
    }
    let metatile_x = tile.x / METATILE_WIDTH;
    let metatile_y = tile.y / METATILE_WIDTH;
    let metatile_id = map.metatile_at(metatile_x, metatile_y)?;
    let metatile = tileset.metatiles.get(metatile_id as usize)?;
    let quadrant = determine_quadrant_index(tile.x, tile.y)?;
    Some(CollisionSample {
        permission: metatile.collision[quadrant],
        metatile_id,
        quadrant,
        tile,
    })
}

pub fn is_direction_blocked(permission: u8, facing: Direction) -> bool {
    let hi = permission & 0xf0;
    if hi != (permissions::RIGHT_WALL & 0xf0) && hi != (permissions::RIGHT_BUOY & 0xf0) {
        return false;
    }
    let low = permission & 0x07;
    match facing {
        Direction::Down => [
            permissions::UP_WALL,
            permissions::UP_RIGHT_WALL,
            permissions::UP_LEFT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
        Direction::Up => [
            permissions::DOWN_WALL,
            permissions::DOWN_RIGHT_WALL,
            permissions::DOWN_LEFT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
        Direction::Left => [
            permissions::RIGHT_WALL,
            permissions::DOWN_RIGHT_WALL,
            permissions::UP_RIGHT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
        Direction::Right => [
            permissions::LEFT_WALL,
            permissions::DOWN_LEFT_WALL,
            permissions::UP_LEFT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
    }
}

/// Decode the departure half of Crystal's side-wall and side-buoy masks.
/// `GetMovementPermissions` applies this to the player, and
/// `CanObjectLeaveTile` applies the same directional relationship to NPCs.
pub fn is_direction_blocked_leaving(permission: u8, facing: Direction) -> bool {
    let hi = permission & 0xf0;
    if hi != (permissions::RIGHT_WALL & 0xf0) && hi != (permissions::RIGHT_BUOY & 0xf0) {
        return false;
    }
    let low = permission & 0x07;
    match facing {
        Direction::Down => [
            permissions::DOWN_WALL,
            permissions::DOWN_RIGHT_WALL,
            permissions::DOWN_LEFT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
        Direction::Up => [
            permissions::UP_WALL,
            permissions::UP_RIGHT_WALL,
            permissions::UP_LEFT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
        Direction::Left => [
            permissions::LEFT_WALL,
            permissions::DOWN_LEFT_WALL,
            permissions::UP_LEFT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
        Direction::Right => [
            permissions::RIGHT_WALL,
            permissions::DOWN_RIGHT_WALL,
            permissions::UP_RIGHT_WALL,
        ]
        .iter()
        .any(|value| (*value & 0x07) == low),
    }
}

pub fn allows_ledge_direction(permission: u8, facing: Direction) -> bool {
    if (permission & 0xf0) != (permissions::HOP_DOWN & 0xf0) {
        return false;
    }
    let low = permission & 0x0f;
    match facing {
        Direction::Down => [
            permissions::HOP_DOWN,
            permissions::HOP_DOWN_RIGHT,
            permissions::HOP_DOWN_LEFT,
        ]
        .iter()
        .any(|value| (*value & 0x0f) == low),
        Direction::Up => [
            permissions::HOP_UP,
            permissions::HOP_UP_RIGHT,
            permissions::HOP_UP_LEFT,
        ]
        .iter()
        .any(|value| (*value & 0x0f) == low),
        Direction::Left => [
            permissions::HOP_LEFT,
            permissions::HOP_DOWN_LEFT,
            permissions::HOP_UP_LEFT,
        ]
        .iter()
        .any(|value| (*value & 0x0f) == low),
        Direction::Right => [
            permissions::HOP_RIGHT,
            permissions::HOP_DOWN_RIGHT,
            permissions::HOP_UP_RIGHT,
        ]
        .iter()
        .any(|value| (*value & 0x0f) == low),
    }
}

pub fn ledge_complement_quadrant(quadrant: usize, facing: Direction) -> Option<usize> {
    match (facing, quadrant) {
        (Direction::Down, 2) => Some(0),
        (Direction::Down, 3) => Some(1),
        (Direction::Up, 0) => Some(2),
        (Direction::Up, 1) => Some(3),
        (Direction::Left, 0) => Some(1),
        (Direction::Left, 2) => Some(3),
        (Direction::Right, 1) => Some(0),
        (Direction::Right, 3) => Some(2),
        _ => None,
    }
}

pub fn collect_collision_samples(
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    tile: TilePosition,
    stride: i16,
) -> Vec<CollisionSample> {
    if stride <= 0 {
        return Vec::new();
    }
    let (width, height) = map.tile_bounds();
    let mut samples = Vec::with_capacity((stride * stride) as usize);
    for dx in 0..stride {
        for dy in 0..stride {
            let Some(sample_x) = tile.x.checked_sub(dx) else {
                return Vec::new();
            };
            let Some(sample_y) = tile.y.checked_sub(dy) else {
                return Vec::new();
            };
            let sample_tile = TilePosition {
                x: sample_x,
                y: sample_y,
            };
            if sample_tile.x < 0
                || sample_tile.y < 0
                || i32::from(sample_tile.x) >= i32::from(width)
                || i32::from(sample_tile.y) >= i32::from(height)
            {
                return Vec::new();
            }
            let Some(sample) = sample_collision(map, tileset, sample_tile) else {
                return Vec::new();
            };
            samples.push(sample);
        }
    }
    samples
}

pub fn front_face_samples(samples: &[CollisionSample], facing: Direction) -> Vec<CollisionSample> {
    let Some(first) = samples.first().copied() else {
        return Vec::new();
    };
    let front_coord = match facing {
        Direction::Down => samples
            .iter()
            .map(|sample| sample.tile.y)
            .max()
            .unwrap_or(first.tile.y),
        Direction::Up => samples
            .iter()
            .map(|sample| sample.tile.y)
            .min()
            .unwrap_or(first.tile.y),
        Direction::Right => samples
            .iter()
            .map(|sample| sample.tile.x)
            .max()
            .unwrap_or(first.tile.x),
        Direction::Left => samples
            .iter()
            .map(|sample| sample.tile.x)
            .min()
            .unwrap_or(first.tile.x),
    };
    samples
        .iter()
        .copied()
        .filter(|sample| match facing {
            Direction::Down | Direction::Up => sample.tile.y == front_coord,
            Direction::Left | Direction::Right => sample.tile.x == front_coord,
        })
        .collect()
}

pub fn is_ledge_face(
    samples: &[CollisionSample],
    facing: Direction,
    tileset: &TilesetCollision,
) -> bool {
    let front = front_face_samples(samples, facing);
    !front.is_empty()
        && front
            .iter()
            .all(|sample| sample_supports_ledge(*sample, facing, tileset))
}

pub fn can_jump_ledge(
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    tile: TilePosition,
    facing: Direction,
    stride: i16,
) -> bool {
    let samples = collect_collision_samples(map, tileset, tile, stride);
    is_ledge_face(&samples, facing, tileset)
}

fn sample_supports_ledge(
    sample: CollisionSample,
    facing: Direction,
    tileset: &TilesetCollision,
) -> bool {
    if allows_ledge_direction(sample.permission, facing) {
        return true;
    }
    if sample.permission != permissions::WALL {
        return false;
    }
    let Some(complement) = ledge_complement_quadrant(sample.quadrant, facing) else {
        return false;
    };
    tileset
        .metatiles
        .get(sample.metatile_id as usize)
        .and_then(|metatile| metatile.collision.get(complement))
        .copied()
        .map(|permission| allows_ledge_direction(permission, facing))
        .unwrap_or(false)
}

pub fn is_permission_passable(
    permission: u8,
    facing: Direction,
    traversal_state: PlayerTraversalState,
) -> bool {
    if is_warp_permission(permission) {
        return true;
    }
    if is_direction_blocked(permission, facing) {
        return false;
    }
    let attributes = describe_collision(permission);
    match traversal_state {
        PlayerTraversalState::Walk => attributes.terrain == Terrain::Land,
        // CheckSurfPerms consults CollisionPermissionTable after applying
        // only the independent directional wall mask. Every `$30..$3f`
        // current is WATER_TILE from every approach; CheckTile forces its
        // low-two-bit direction on the following overworld pass.
        PlayerTraversalState::Surf => attributes.terrain != Terrain::Wall,
    }
}

pub fn can_enter_tile(
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    tile: TilePosition,
    facing: Direction,
    traversal_state: PlayerTraversalState,
) -> bool {
    sample_collision(map, tileset, tile)
        .map(|sample| is_permission_passable(sample.permission, facing, traversal_state))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapAttributes;

    fn attributes_for_test(width: u16, height: u16) -> MapAttributes {
        MapAttributes {
            tileset_name: "test".to_string(),
            border_block: 0,
            width,
            height,
            connections: Vec::new(),
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: None,
            location: None,
            music: None,
            palette: None,
            fishing_group: None,
            map_constant: None,
            map_group_constant: None,
            blocks_label: None,
            map_scripts_label: None,
            map_events_label: None,
            connection_flags: None,
        }
    }

    fn test_map() -> OverworldMapData {
        OverworldMapData::from_attributes("test", &attributes_for_test(2, 1), vec![0, 1])
    }

    fn test_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [
                        permissions::FLOOR,
                        permissions::FLOOR,
                        permissions::WATER,
                        permissions::WALL,
                    ],
                },
                MetatileCollision {
                    collision: [
                        permissions::UP_WALL,
                        permissions::FLOOR,
                        permissions::WHIRLPOOL,
                        permissions::WATER,
                    ],
                },
            ],
        }
    }

    #[test]
    fn grass_encounter_permissions_match_check_grass_collision_table_exactly() {
        let source_table = [
            permissions::CUT_GRASS,
            permissions::TALL_GRASS,
            permissions::LONG_GRASS,
            permissions::CUT_GRASS_28,
            permissions::WATER,
            permissions::GRASS_48,
            permissions::GRASS_49,
            permissions::GRASS_4A,
            permissions::GRASS_4B,
            permissions::GRASS_4C,
        ];

        for permission in u8::MIN..=u8::MAX {
            // Water is present in CheckGrassCollision.blocks but is resolved
            // by the dedicated water branch before this grass predicate.
            let expected = permission != permissions::WATER && source_table.contains(&permission);
            assert_eq!(
                is_grass_encounter_permission(permission),
                expected,
                "collision {permission:#04x}"
            );
        }
    }

    #[test]
    fn samples_collision_by_metatile_and_quadrant() {
        let sample = sample_collision(&test_map(), &test_tileset(), TilePosition::new(0, 1))
            .expect("collision sample");
        assert_eq!(sample.metatile_id, 0);
        assert_eq!(sample.quadrant, 2);
        assert_eq!(sample.permission, permissions::WATER);

        let sample = sample_collision(&test_map(), &test_tileset(), TilePosition::new(2, 0))
            .expect("collision sample");
        assert_eq!(sample.metatile_id, 1);
        assert_eq!(sample.quadrant, 0);
        assert_eq!(sample.permission, permissions::UP_WALL);
    }

    #[test]
    fn passability_respects_walk_and_complete_surf_water_rules() {
        assert!(is_permission_passable(
            permissions::FLOOR,
            Direction::Down,
            PlayerTraversalState::Walk
        ));
        assert!(!is_permission_passable(
            permissions::WATER,
            Direction::Down,
            PlayerTraversalState::Walk
        ));
        assert!(is_permission_passable(
            permissions::WATER,
            Direction::Down,
            PlayerTraversalState::Surf
        ));
        assert!(is_permission_passable(
            permissions::WHIRLPOOL,
            Direction::Down,
            PlayerTraversalState::Surf
        ));
        assert!(is_permission_passable(
            permissions::WHIRLPOOL_2C,
            Direction::Left,
            PlayerTraversalState::Surf
        ));
        assert!(is_permission_passable(
            permissions::DOOR,
            Direction::Down,
            PlayerTraversalState::Walk
        ));
        assert!(is_permission_passable(
            permissions::WARP_CARPET_RIGHT,
            Direction::Right,
            PlayerTraversalState::Walk
        ));
    }

    #[test]
    fn every_current_permission_is_surfable_from_every_direction_before_forcing_motion() {
        for permission in 0x30..=0x3f {
            for direction in [
                Direction::Down,
                Direction::Up,
                Direction::Left,
                Direction::Right,
            ] {
                assert!(
                    is_permission_passable(permission, direction, PlayerTraversalState::Surf),
                    "current permission {permission:#04x} must admit {direction:?} before CheckTile forces its low-two-bit direction"
                );
            }
        }
    }

    #[test]
    fn all_collision_attributes_match_the_asm_permission_table() {
        for permission in u8::MIN..=u8::MAX {
            let expected_terrain = if matches!(
                permission,
                0x07
                    | 0x0f
                    | 0x12
                    | 0x15
                    | 0x1a
                    | 0x1d
                    | 0x27
                    | 0x2f
                    | 0x62
                    | 0x6a
                    | 0x80..=0x84
                    | 0x88..=0x8c
                    | 0x90..=0x9f
                    | 0xff
            ) {
                Terrain::Wall
            } else if matches!(
                permission,
                0x20..=0x22
                    | 0x24..=0x26
                    | 0x28..=0x2a
                    | 0x2c..=0x2e
                    | 0x30..=0x3f
                    | 0xc0..=0xcf
            ) {
                Terrain::Water
            } else {
                Terrain::Land
            };
            let expected_talk = matches!(
                permission,
                0x12 | 0x15 | 0x1a | 0x1d | 0x22 | 0x24 | 0x2a | 0x2c
            );

            assert_eq!(
                describe_collision(permission),
                CollisionAttributes {
                    value: permission,
                    terrain: expected_terrain,
                    talk: expected_talk,
                },
                "collision permission {permission:#04x}"
            );
        }
    }

    #[test]
    fn directional_walls_block_entry_from_matching_direction() {
        assert!(is_direction_blocked(permissions::UP_WALL, Direction::Down));
        assert!(!is_direction_blocked(permissions::UP_WALL, Direction::Up));
        assert!(is_direction_blocked(
            permissions::RIGHT_WALL,
            Direction::Left
        ));
    }

    #[test]
    fn every_side_wall_and_buoy_mask_blocks_the_matching_exit_directions() {
        for high_nibble in [0xb0, 0xc0] {
            for (low_nibble, blocked_directions) in [
                (0, &[Direction::Right][..]),
                (1, &[Direction::Left][..]),
                (2, &[Direction::Up][..]),
                (3, &[Direction::Down][..]),
                (4, &[Direction::Down, Direction::Right][..]),
                (5, &[Direction::Down, Direction::Left][..]),
                (6, &[Direction::Up, Direction::Right][..]),
                (7, &[Direction::Up, Direction::Left][..]),
            ] {
                let permission = high_nibble | low_nibble;
                for direction in [
                    Direction::Down,
                    Direction::Up,
                    Direction::Left,
                    Direction::Right,
                ] {
                    assert_eq!(
                        is_direction_blocked_leaving(permission, direction),
                        blocked_directions.contains(&direction),
                        "permission {permission:#04x}, direction {direction:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn can_enter_tile_samples_and_rejects_out_of_bounds() {
        assert!(can_enter_tile(
            &test_map(),
            &test_tileset(),
            TilePosition::new(0, 0),
            Direction::Down,
            PlayerTraversalState::Walk
        ));
        assert!(!can_enter_tile(
            &test_map(),
            &test_tileset(),
            TilePosition::new(1, 1),
            Direction::Down,
            PlayerTraversalState::Walk
        ));
        assert!(!can_enter_tile(
            &test_map(),
            &test_tileset(),
            TilePosition::new(99, 99),
            Direction::Down,
            PlayerTraversalState::Walk
        ));
    }

    #[test]
    fn ledge_direction_bits_match_collision_permissions() {
        assert!(allows_ledge_direction(
            permissions::HOP_DOWN_RIGHT,
            Direction::Down
        ));
        assert!(allows_ledge_direction(
            permissions::HOP_DOWN_RIGHT,
            Direction::Right
        ));
        assert!(!allows_ledge_direction(
            permissions::HOP_DOWN_RIGHT,
            Direction::Up
        ));
        assert!(!allows_ledge_direction(permissions::WALL, Direction::Down));
    }

    #[test]
    fn ledge_complements_match_metatile_quadrants() {
        assert_eq!(ledge_complement_quadrant(2, Direction::Down), Some(0));
        assert_eq!(ledge_complement_quadrant(1, Direction::Up), Some(3));
        assert_eq!(ledge_complement_quadrant(0, Direction::Left), Some(1));
        assert_eq!(ledge_complement_quadrant(3, Direction::Right), Some(2));
        assert_eq!(ledge_complement_quadrant(0, Direction::Down), None);
    }

    #[test]
    fn collects_collision_samples_for_stride_footprint() {
        let samples =
            collect_collision_samples(&test_map(), &test_tileset(), TilePosition::new(1, 1), 2);
        assert_eq!(samples.len(), 4);
        assert_eq!(
            samples.iter().map(|sample| sample.tile).collect::<Vec<_>>(),
            vec![
                TilePosition::new(1, 1),
                TilePosition::new(1, 0),
                TilePosition::new(0, 1),
                TilePosition::new(0, 0),
            ]
        );
        assert!(
            collect_collision_samples(&test_map(), &test_tileset(), TilePosition::new(0, 0), 2)
                .is_empty()
        );
        assert!(
            collect_collision_samples(
                &test_map(),
                &test_tileset(),
                TilePosition::new(i16::MIN, 1),
                2,
            )
            .is_empty()
        );
    }

    #[test]
    fn front_face_samples_select_forward_edge() {
        let samples =
            collect_collision_samples(&test_map(), &test_tileset(), TilePosition::new(1, 1), 2);

        assert_eq!(
            front_face_samples(&samples, Direction::Down)
                .iter()
                .map(|sample| sample.tile)
                .collect::<Vec<_>>(),
            vec![TilePosition::new(1, 1), TilePosition::new(0, 1)]
        );
        assert_eq!(
            front_face_samples(&samples, Direction::Right)
                .iter()
                .map(|sample| sample.tile)
                .collect::<Vec<_>>(),
            vec![TilePosition::new(1, 1), TilePosition::new(1, 0)]
        );
    }

    #[test]
    fn ledge_face_accepts_direct_and_wall_complement_permissions() {
        let map = OverworldMapData::from_attributes("ledge", &attributes_for_test(1, 1), vec![0]);
        let tileset = TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [
                    permissions::FLOOR,
                    permissions::HOP_DOWN,
                    permissions::HOP_DOWN,
                    permissions::WALL,
                ],
            }],
        };
        let samples = collect_collision_samples(&map, &tileset, TilePosition::new(1, 1), 2);

        assert!(is_ledge_face(&samples, Direction::Down, &tileset));
        assert!(!is_ledge_face(&samples, Direction::Up, &tileset));
        assert!(can_jump_ledge(
            &map,
            &tileset,
            TilePosition::new(1, 1),
            Direction::Down,
            2,
        ));
    }

    #[test]
    fn collision_discriminants_reject_legacy_alias_payloads() {
        let terrain_error = serde_json::from_value::<Terrain>(serde_json::json!({
            "water": {
                "fallback_terrain": "land"
            }
        }))
        .expect_err("terrain must not accept fallback object payloads")
        .to_string();
        assert!(
            terrain_error.contains("invalid type") || terrain_error.contains("unknown variant"),
            "{terrain_error}"
        );

        let traversal_error = serde_json::from_value::<PlayerTraversalState>(serde_json::json!({
            "Surf": {
                "legacy_state": "WALK"
            }
        }))
        .expect_err("traversal state must not accept legacy object payloads")
        .to_string();
        assert!(
            traversal_error.contains("invalid type") || traversal_error.contains("unknown variant"),
            "{traversal_error}"
        );
    }

    #[test]
    fn shaking_grass_permissions_match_both_source_checks_exactly() {
        let expected = [0x10, 0x14, 0x18, 0x1c, 0x20, 0x28];

        for permission in u8::MIN..=u8::MAX {
            assert_eq!(
                spawns_shaking_grass_object(permission),
                expected.contains(&permission),
                "collision {permission:#04x}"
            );
        }
    }
}
