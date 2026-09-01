use serde::{Deserialize, Serialize};

use super::collision::{
    PlayerTraversalState, Terrain, TilesetCollision, can_enter_tile, can_jump_ledge,
    describe_collision, is_direction_blocked_leaving, sample_collision,
};
use super::map::{Direction, OverworldMapData, TilePosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MovementMode {
    Normal,
    Bike,
    Skate,
    Surf,
    SurfPika,
}

impl MovementMode {
    pub const fn traversal_state(self) -> PlayerTraversalState {
        match self {
            Self::Normal | Self::Bike | Self::Skate => PlayerTraversalState::Walk,
            Self::Surf | Self::SurfPika => PlayerTraversalState::Surf,
        }
    }

    pub const fn speed_multiplier(self) -> u8 {
        match self {
            Self::Normal | Self::Surf | Self::SurfPika => 1,
            Self::Bike | Self::Skate => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerMovementState {
    pub tile: TilePosition,
    pub facing: Direction,
    pub mode: MovementMode,
}

impl PlayerMovementState {
    pub const fn new(tile: TilePosition) -> Self {
        Self {
            tile,
            facing: Direction::Down,
            mode: MovementMode::Normal,
        }
    }

    pub const fn with_mode(mut self, mode: MovementMode) -> Self {
        self.mode = mode;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StepOutcome {
    Turned {
        facing: Direction,
    },
    Moved {
        from: TilePosition,
        to: TilePosition,
        speed_multiplier: u8,
    },
    Blocked {
        at: TilePosition,
        facing: Direction,
    },
    BlockedByObject {
        at: TilePosition,
        facing: Direction,
        object_identifier: Option<String>,
    },
    RuntimeTileOverflow {
        from: TilePosition,
        facing: Direction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LedgeJumpOutcome {
    Jumped {
        from: TilePosition,
        over: TilePosition,
        to: TilePosition,
        speed_multiplier: u8,
    },
    NotLedge {
        at: TilePosition,
        facing: Direction,
    },
    BlockedLanding {
        at: TilePosition,
        facing: Direction,
    },
    BlockedByObject {
        at: TilePosition,
        facing: Direction,
        object_identifier: Option<String>,
    },
    RuntimeTileOverflow {
        from: TilePosition,
        facing: Direction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccupiedTile {
    pub tile: TilePosition,
    pub object_identifier: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepOptions {
    pub force_step_after_turn: bool,
    pub stride_tiles: i16,
}

pub const DEFAULT_RUNTIME_TILE_STRIDE: i16 = 1;

impl Default for StepOptions {
    fn default() -> Self {
        Self {
            force_step_after_turn: false,
            stride_tiles: DEFAULT_RUNTIME_TILE_STRIDE,
        }
    }
}

pub fn attempt_step(
    state: &mut PlayerMovementState,
    direction: Direction,
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    options: StepOptions,
) -> StepOutcome {
    attempt_step_with_occupied_tiles(state, direction, map, tileset, options, &[])
}

pub fn attempt_step_with_occupied_tiles(
    state: &mut PlayerMovementState,
    direction: Direction,
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    options: StepOptions,
    occupied_tiles: &[OccupiedTile],
) -> StepOutcome {
    if direction != state.facing && !options.force_step_after_turn {
        state.facing = direction;
        return StepOutcome::Turned { facing: direction };
    }

    state.facing = direction;
    let target = match checked_move_by_stride(state.tile, direction, options.stride_tiles) {
        Some(target) => target,
        None => {
            return StepOutcome::RuntimeTileOverflow {
                from: state.tile,
                facing: direction,
            };
        }
    };
    if sample_collision(map, tileset, state.tile)
        .is_some_and(|sample| is_direction_blocked_leaving(sample.permission, direction))
    {
        return StepOutcome::Blocked {
            at: target,
            facing: direction,
        };
    }
    if let Some(occupied) = occupied_tile_at(occupied_tiles, target) {
        return StepOutcome::BlockedByObject {
            at: target,
            facing: direction,
            object_identifier: occupied.object_identifier.clone(),
        };
    }
    if !can_enter_tile(
        map,
        tileset,
        target,
        direction,
        state.mode.traversal_state(),
    ) && !is_declared_connection_step(map, target, direction)
    {
        return StepOutcome::Blocked {
            at: target,
            facing: direction,
        };
    }

    let from = state.tile;
    if matches!(state.mode, MovementMode::Surf | MovementMode::SurfPika)
        && sample_collision(map, tileset, target)
            .is_some_and(|sample| describe_collision(sample.permission).terrain == Terrain::Land)
    {
        state.mode = MovementMode::Normal;
    }
    state.tile = target;
    StepOutcome::Moved {
        from,
        to: target,
        speed_multiplier: state.mode.speed_multiplier(),
    }
}

pub fn attempt_ledge_jump(
    state: &mut PlayerMovementState,
    direction: Direction,
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    options: StepOptions,
) -> LedgeJumpOutcome {
    attempt_ledge_jump_with_occupied_tiles(state, direction, map, tileset, options, &[])
}

pub fn attempt_ledge_jump_with_occupied_tiles(
    state: &mut PlayerMovementState,
    direction: Direction,
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    options: StepOptions,
    occupied_tiles: &[OccupiedTile],
) -> LedgeJumpOutcome {
    state.facing = direction;
    let stride = options.stride_tiles;
    let ledge = match checked_move_by_stride(state.tile, direction, stride) {
        Some(ledge) => ledge,
        None => {
            return LedgeJumpOutcome::RuntimeTileOverflow {
                from: state.tile,
                facing: direction,
            };
        }
    };
    // ASM TryJump reads the permission at the player's current collision
    // position. The tile in front is the course jumped over, not the tile
    // whose HOP_* direction bits authorize the jump.
    if !can_jump_ledge(map, tileset, state.tile, direction, stride) {
        return LedgeJumpOutcome::NotLedge {
            at: ledge,
            facing: direction,
        };
    }

    let landing_stride = match stride.checked_mul(2) {
        Some(stride) => stride,
        None => {
            return LedgeJumpOutcome::RuntimeTileOverflow {
                from: state.tile,
                facing: direction,
            };
        }
    };
    let landing = match checked_move_by_stride(state.tile, direction, landing_stride) {
        Some(landing) => landing,
        None => {
            return LedgeJumpOutcome::RuntimeTileOverflow {
                from: state.tile,
                facing: direction,
            };
        }
    };
    if let Some(occupied) = occupied_tile_at(occupied_tiles, landing) {
        return LedgeJumpOutcome::BlockedByObject {
            at: landing,
            facing: direction,
            object_identifier: occupied.object_identifier.clone(),
        };
    }
    if !can_enter_tile(
        map,
        tileset,
        landing,
        direction,
        state.mode.traversal_state(),
    ) && !is_declared_connection_step(map, landing, direction)
    {
        return LedgeJumpOutcome::BlockedLanding {
            at: landing,
            facing: direction,
        };
    }

    let from = state.tile;
    state.tile = landing;
    LedgeJumpOutcome::Jumped {
        from,
        over: ledge,
        to: landing,
        // STEP_LEDGE has its own fixed sixteen-frame movement function. Bike
        // and skate mode do not select STEP_BIKE for either half of the jump.
        speed_multiplier: 1,
    }
}

fn occupied_tile_at(occupied_tiles: &[OccupiedTile], tile: TilePosition) -> Option<&OccupiedTile> {
    occupied_tiles.iter().find(|occupied| occupied.tile == tile)
}

pub fn is_declared_connection_step(
    map: &OverworldMapData,
    target: TilePosition,
    direction: Direction,
) -> bool {
    let (width, height) = map.tile_bounds();
    let width = i32::from(width);
    let height = i32::from(height);
    let target_x = i32::from(target.x);
    let target_y = i32::from(target.y);
    let required_direction = match direction {
        Direction::Left if target_x < 0 => "west",
        Direction::Right if target_x >= width => "east",
        Direction::Up if target_y < 0 => "north",
        Direction::Down if target_y >= height => "south",
        _ => return false,
    };
    map.connections()
        .iter()
        .any(|connection| connection.direction == required_direction)
}

pub const fn move_by_stride(
    start: TilePosition,
    direction: Direction,
    stride_tiles: i16,
) -> TilePosition {
    let (dx, dy) = direction.delta();
    TilePosition {
        x: start.x + dx * stride_tiles,
        y: start.y + dy * stride_tiles,
    }
}

pub fn checked_move_by_stride(
    start: TilePosition,
    direction: Direction,
    stride_tiles: i16,
) -> Option<TilePosition> {
    let (dx, dy) = direction.delta();
    Some(TilePosition {
        x: start.x.checked_add(dx.checked_mul(stride_tiles)?)?,
        y: start.y.checked_add(dy.checked_mul(stride_tiles)?)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{MapAttributes, MapConnection};
    use crate::world::collision::{MetatileCollision, permissions};

    fn attributes(width: u16, height: u16) -> MapAttributes {
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

    fn map() -> OverworldMapData {
        OverworldMapData::from_attributes("test", &attributes(3, 2), vec![0, 0, 0, 0, 1, 0])
    }

    fn map_with_east_connection() -> OverworldMapData {
        let mut attributes = attributes(3, 2);
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        OverworldMapData::from_attributes("test", &attributes, vec![0, 0, 0, 0, 0, 0])
    }

    fn tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
                MetatileCollision {
                    collision: [permissions::WALL; 4],
                },
                MetatileCollision {
                    collision: [permissions::WATER; 4],
                },
            ],
        }
    }

    fn ledge_tileset(landing_permission: u8) -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
                MetatileCollision {
                    collision: [permissions::HOP_DOWN; 4],
                },
                MetatileCollision {
                    collision: [landing_permission; 4],
                },
            ],
        }
    }

    fn ledge_map() -> OverworldMapData {
        OverworldMapData::from_attributes("ledge", &attributes(2, 3), vec![0, 0, 1, 1, 2, 2])
    }

    #[test]
    fn first_direction_change_turns_without_moving() {
        let mut state = PlayerMovementState::new(TilePosition::new(0, 0));
        let outcome = attempt_step(
            &mut state,
            Direction::Right,
            &map(),
            &tileset(),
            StepOptions::default(),
        );
        assert_eq!(
            outcome,
            StepOutcome::Turned {
                facing: Direction::Right
            }
        );
        assert_eq!(state.tile, TilePosition::new(0, 0));
        assert_eq!(state.facing, Direction::Right);
    }

    #[test]
    fn movement_discriminants_reject_legacy_alias_payloads() {
        let mode_error = serde_json::from_str::<MovementMode>(r#"{"bike":{"legacy_speed":2}}"#)
            .expect_err("movement modes must not accept object-shaped aliases")
            .to_string();
        assert!(
            mode_error.contains("invalid type")
                || mode_error.contains("unknown field `legacy_speed`"),
            "{mode_error}"
        );

        let step_error = serde_json::from_str::<StepOutcome>(
            r#"{"moved":{"from":{"x":0,"y":0},"to":{"x":2,"y":0},"speed_multiplier":1,"fallback_tile":{"x":1,"y":0}}}"#,
        )
        .expect_err("step outcomes must not accept fallback movement fields")
        .to_string();
        assert!(
            step_error.contains("unknown field `fallback_tile`"),
            "{step_error}"
        );

        let ledge_error = serde_json::from_str::<LedgeJumpOutcome>(
            r#"{"jumped":{"from":{"x":1,"y":1},"over":{"x":1,"y":3},"to":{"x":1,"y":5},"speed_multiplier":1,"normalized_landing":{"x":1,"y":4}}}"#,
        )
        .expect_err("ledge outcomes must not accept normalized landing fields")
        .to_string();
        assert!(
            ledge_error.contains("unknown field `normalized_landing`"),
            "{ledge_error}"
        );
    }

    #[test]
    fn forced_step_after_turn_moves_by_default_runtime_stride() {
        let mut state = PlayerMovementState::new(TilePosition::new(0, 0));
        let outcome = attempt_step(
            &mut state,
            Direction::Right,
            &map(),
            &tileset(),
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );
        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: TilePosition::new(0, 0),
                to: TilePosition::new(1, 0),
                speed_multiplier: 1,
            }
        );
        assert_eq!(state.tile, TilePosition::new(1, 0));
    }

    #[test]
    fn occupied_tile_blocks_step_without_moving() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(0, 0),
            facing: Direction::Right,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_step_with_occupied_tiles(
            &mut state,
            Direction::Right,
            &map(),
            &tileset(),
            StepOptions::default(),
            &[OccupiedTile {
                tile: TilePosition::new(1, 0),
                object_identifier: Some("ROUTE29_TEACHER1".to_string()),
            }],
        );

        assert_eq!(
            outcome,
            StepOutcome::BlockedByObject {
                at: TilePosition::new(1, 0),
                facing: Direction::Right,
                object_identifier: Some("ROUTE29_TEACHER1".to_string()),
            }
        );
        assert_eq!(state.tile, TilePosition::new(0, 0));
    }

    #[test]
    fn player_cannot_leave_through_directional_wall_or_buoy_edges() {
        for (direction, source, target, wall, buoy) in [
            (
                Direction::Down,
                TilePosition::new(0, 0),
                TilePosition::new(0, 1),
                permissions::DOWN_WALL,
                permissions::DOWN_BUOY,
            ),
            (
                Direction::Up,
                TilePosition::new(0, 1),
                TilePosition::new(0, 0),
                permissions::UP_WALL,
                permissions::UP_BUOY,
            ),
            (
                Direction::Left,
                TilePosition::new(1, 0),
                TilePosition::new(0, 0),
                permissions::LEFT_WALL,
                permissions::LEFT_BUOY,
            ),
            (
                Direction::Right,
                TilePosition::new(0, 0),
                TilePosition::new(1, 0),
                permissions::RIGHT_WALL,
                permissions::RIGHT_BUOY,
            ),
        ] {
            for (source_permission, destination_permission, mode) in [
                (wall, permissions::FLOOR, MovementMode::Normal),
                (buoy, permissions::WATER, MovementMode::Surf),
            ] {
                let mut collision = [destination_permission; 4];
                let source_quadrant =
                    usize::from(source.y as u16 % 2) * 2 + usize::from(source.x as u16 % 2);
                collision[source_quadrant] = source_permission;
                let map = OverworldMapData::from_attributes(
                    "directional edge",
                    &attributes(1, 1),
                    vec![0],
                );
                let tileset = TilesetCollision {
                    metatiles: vec![MetatileCollision { collision }],
                };
                let mut state = PlayerMovementState {
                    tile: source,
                    facing: direction,
                    mode,
                };

                assert_eq!(
                    attempt_step(
                        &mut state,
                        direction,
                        &map,
                        &tileset,
                        StepOptions::default(),
                    ),
                    StepOutcome::Blocked {
                        at: target,
                        facing: direction,
                    },
                    "source permission {source_permission:#04x} toward {direction:?}",
                );
                assert_eq!(state.tile, source);
            }
        }
    }

    #[test]
    fn blocked_step_keeps_position() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(2, 2),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_step(
            &mut state,
            Direction::Down,
            &map(),
            &tileset(),
            StepOptions::default(),
        );
        assert_eq!(
            outcome,
            StepOutcome::Blocked {
                at: TilePosition::new(2, 3),
                facing: Direction::Down,
            }
        );
        assert_eq!(state.tile, TilePosition::new(2, 2));
    }

    #[test]
    fn declared_connection_step_can_move_beyond_map_edge() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(5, 0),
            facing: Direction::Right,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_step(
            &mut state,
            Direction::Right,
            &map_with_east_connection(),
            &tileset(),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: TilePosition::new(5, 0),
                to: TilePosition::new(6, 0),
                speed_multiplier: 1,
            }
        );
        assert_eq!(state.tile, TilePosition::new(6, 0));
    }

    #[test]
    fn undeclared_connection_step_still_blocks_at_map_edge() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(5, 0),
            facing: Direction::Right,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_step(
            &mut state,
            Direction::Right,
            &map(),
            &tileset(),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            StepOutcome::Blocked {
                at: TilePosition::new(6, 0),
                facing: Direction::Right,
            }
        );
        assert_eq!(state.tile, TilePosition::new(5, 0));
    }

    #[test]
    fn wide_map_connection_checks_do_not_narrow_tile_bounds() {
        let mut attributes = attributes(20_000, 1);
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        let map = OverworldMapData::from_attributes("wide", &attributes, vec![0; 20_000]);

        assert!(!is_declared_connection_step(
            &map,
            TilePosition::new(100, 0),
            Direction::Right
        ));
        assert!(!is_declared_connection_step(
            &map,
            TilePosition::new(i16::MAX, 0),
            Direction::Right
        ));
    }

    #[test]
    fn checked_move_by_stride_rejects_coordinate_overflow() {
        assert_eq!(
            checked_move_by_stride(TilePosition::new(0, 0), Direction::Right, 2),
            Some(TilePosition::new(2, 0))
        );
        assert_eq!(
            checked_move_by_stride(TilePosition::new(i16::MAX, 0), Direction::Right, 2),
            None
        );
        assert_eq!(
            checked_move_by_stride(TilePosition::new(i16::MIN, 0), Direction::Left, 2),
            None
        );
    }

    #[test]
    fn step_reports_runtime_tile_overflow_without_moving() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(i16::MAX, 0),
            facing: Direction::Right,
            mode: MovementMode::Normal,
        };

        let outcome = attempt_step(
            &mut state,
            Direction::Right,
            &map(),
            &tileset(),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            StepOutcome::RuntimeTileOverflow {
                from: TilePosition::new(i16::MAX, 0),
                facing: Direction::Right,
            }
        );
        assert_eq!(state.tile, TilePosition::new(i16::MAX, 0));
        assert_eq!(state.facing, Direction::Right);
    }

    #[test]
    fn ledge_jump_reports_runtime_tile_overflow_without_moving() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(0, i16::MAX),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };

        let outcome = attempt_ledge_jump(
            &mut state,
            Direction::Down,
            &ledge_map(),
            &ledge_tileset(permissions::FLOOR),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::RuntimeTileOverflow {
                from: TilePosition::new(0, i16::MAX),
                facing: Direction::Down,
            }
        );
        assert_eq!(state.tile, TilePosition::new(0, i16::MAX));
        assert_eq!(state.facing, Direction::Down);
    }

    #[test]
    fn surf_mode_can_enter_water_when_walk_cannot() {
        let water_map = OverworldMapData::from_attributes("water", &attributes(2, 1), vec![0, 2]);
        let mut walker = PlayerMovementState {
            tile: TilePosition::new(1, 0),
            facing: Direction::Right,
            mode: MovementMode::Normal,
        };
        let mut surfer = PlayerMovementState {
            mode: MovementMode::Surf,
            ..walker
        };

        assert!(matches!(
            attempt_step(
                &mut walker,
                Direction::Right,
                &water_map,
                &tileset(),
                StepOptions::default()
            ),
            StepOutcome::Blocked { .. }
        ));
        assert!(matches!(
            attempt_step(
                &mut surfer,
                Direction::Right,
                &water_map,
                &tileset(),
                StepOptions::default()
            ),
            StepOutcome::Moved { .. }
        ));
    }

    #[test]
    fn bike_and_skate_report_faster_step_speed() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(0, 0),
            facing: Direction::Right,
            mode: MovementMode::Bike,
        };
        let outcome = attempt_step(
            &mut state,
            Direction::Right,
            &map(),
            &tileset(),
            StepOptions::default(),
        );
        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: TilePosition::new(0, 0),
                to: TilePosition::new(1, 0),
                speed_multiplier: 2,
            }
        );
    }

    #[test]
    fn ledge_jump_moves_two_strides_over_valid_ledge() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(2, 2),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_ledge_jump(
            &mut state,
            Direction::Down,
            &ledge_map(),
            &ledge_tileset(permissions::FLOOR),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::Jumped {
                from: TilePosition::new(2, 2),
                over: TilePosition::new(2, 3),
                to: TilePosition::new(2, 4),
                speed_multiplier: 1,
            }
        );
        assert_eq!(state.tile, TilePosition::new(2, 4));
        assert_eq!(state.facing, Direction::Down);
    }

    #[test]
    fn ledge_jump_reads_directional_permission_under_player() {
        let map = OverworldMapData::from_attributes("one_way_ledge", &attributes(1, 2), vec![0, 1]);
        let tileset = TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [
                        permissions::FLOOR,
                        permissions::FLOOR,
                        permissions::HOP_DOWN,
                        permissions::HOP_DOWN,
                    ],
                },
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
            ],
        };
        let mut state = PlayerMovementState {
            tile: TilePosition::new(0, 1),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };

        let outcome = attempt_ledge_jump(
            &mut state,
            Direction::Down,
            &map,
            &tileset,
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::Jumped {
                from: TilePosition::new(0, 1),
                over: TilePosition::new(0, 2),
                to: TilePosition::new(0, 3),
                speed_multiplier: 1,
            }
        );
        assert_eq!(state.tile, TilePosition::new(0, 3));
    }

    #[test]
    fn bike_ledge_jump_keeps_fixed_ledge_speed() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(2, 2),
            facing: Direction::Down,
            mode: MovementMode::Bike,
        };
        let outcome = attempt_ledge_jump(
            &mut state,
            Direction::Down,
            &ledge_map(),
            &ledge_tileset(permissions::FLOOR),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::Jumped {
                from: TilePosition::new(2, 2),
                over: TilePosition::new(2, 3),
                to: TilePosition::new(2, 4),
                speed_multiplier: 1,
            }
        );
    }

    #[test]
    fn ledge_jump_rejects_wrong_direction_without_normalization() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(2, 3),
            facing: Direction::Up,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_ledge_jump(
            &mut state,
            Direction::Up,
            &ledge_map(),
            &ledge_tileset(permissions::FLOOR),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::NotLedge {
                at: TilePosition::new(2, 2),
                facing: Direction::Up,
            }
        );
        assert_eq!(state.tile, TilePosition::new(2, 3));
    }

    #[test]
    fn ledge_jump_rejects_blocked_landing() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(2, 2),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_ledge_jump(
            &mut state,
            Direction::Down,
            &ledge_map(),
            &ledge_tileset(permissions::WALL),
            StepOptions::default(),
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::BlockedLanding {
                at: TilePosition::new(2, 4),
                facing: Direction::Down,
            }
        );
        assert_eq!(state.tile, TilePosition::new(2, 2));
    }

    #[test]
    fn occupied_tile_blocks_ledge_landing_without_moving() {
        let mut state = PlayerMovementState {
            tile: TilePosition::new(2, 2),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let outcome = attempt_ledge_jump_with_occupied_tiles(
            &mut state,
            Direction::Down,
            &ledge_map(),
            &ledge_tileset(permissions::FLOOR),
            StepOptions::default(),
            &[OccupiedTile {
                tile: TilePosition::new(2, 4),
                object_identifier: Some("LANDING_NPC".to_string()),
            }],
        );

        assert_eq!(
            outcome,
            LedgeJumpOutcome::BlockedByObject {
                at: TilePosition::new(2, 4),
                facing: Direction::Down,
                object_identifier: Some("LANDING_NPC".to_string()),
            }
        );
        assert_eq!(state.tile, TilePosition::new(2, 2));
    }
}
