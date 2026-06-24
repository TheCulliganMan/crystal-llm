use serde::{Deserialize, Serialize};

use crate::state::{EventFlagError, GameState};
use crate::world::session::{OverworldFollowState, OverworldSession};
use crate::world::{
    map::{Direction, TilePosition},
    movement::move_by_stride,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptObjectCommand {
    pub command: String,
    pub object_id: Option<String>,
    pub target_object_id: Option<String>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub direction: Option<String>,
    pub movement: Option<String>,
    pub emote: Option<String>,
    pub duration: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovement {
    pub label: String,
    pub source_script: Option<String>,
    pub steps: Vec<ScriptMovementStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovementStep {
    pub command: String,
    pub direction: Option<String>,
    pub duration: Option<u16>,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptObjectMutationOutcome {
    pub command: String,
    pub object_id: String,
    pub event_flag: Option<String>,
    pub previous_x: Option<u16>,
    pub previous_y: Option<u16>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovementEffect {
    pub command: String,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovementOutcome {
    pub object_id: String,
    pub movement: String,
    pub previous_tile: TilePosition,
    pub tile: TilePosition,
    pub facing: Direction,
    pub effects: Vec<ScriptMovementEffect>,
    pub fixed_facing: bool,
    pub sliding: bool,
    pub steps_applied: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ScriptObjectCommandError {
    #[error("script object command '{command}' is not a state mutation")]
    NotObjectMutation { command: String },
    #[error("script object command '{command}' is missing an object id")]
    MissingObjectId { command: String },
    #[error("script object command '{command}' is missing a target object id")]
    MissingTargetObjectId { command: String },
    #[error("unknown script object '{object_id}'")]
    UnknownObject { object_id: String },
    #[error("object '{object_id}' has no initialized facing")]
    MissingObjectFacing { object_id: String },
    #[error("object '{object_id}' cannot be hidden or shown with event flag '{event_flag}'")]
    ObjectCannotToggle {
        object_id: String,
        event_flag: String,
    },
    #[error("moveobject for '{object_id}' is missing x/y coordinates")]
    MissingMoveCoordinates { object_id: String },
    #[error("script object command '{command}' is missing a direction")]
    MissingDirection { command: String },
    #[error("unknown script direction '{direction}'")]
    UnknownDirection { direction: String },
    #[error("applymovement for '{object_id}' is missing a movement label")]
    MissingMovement { object_id: String },
    #[error("applymovementlasttalked requires a last talked object")]
    MissingLastTalkedObject,
    #[error("movement '{movement}' is not the command movement '{expected}'")]
    WrongMovement { movement: String, expected: String },
    #[error("unsupported movement command '{command}' in movement '{movement}' at index {index}")]
    UnsupportedMovementCommand {
        movement: String,
        command: String,
        index: usize,
    },
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} is missing a direction"
    )]
    MovementMissingDirection {
        movement: String,
        command: String,
        index: usize,
    },
    #[error("movement for '{object_id}' moved outside unsigned object coordinates: ({x}, {y})")]
    ObjectPositionOutOfRange { object_id: String, x: i16, y: i16 },
    #[error("event flag error: {error}")]
    EventFlag { error: EventFlagError },
}

pub fn apply_script_object_mutation(
    state: &mut GameState,
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    match command.command.as_str() {
        "appear" => apply_visibility_command(state, session, command, false),
        "disappear" => apply_visibility_command(state, session, command, true),
        "moveobject" => apply_moveobject_command(session, command),
        "turnobject" => apply_turnobject_command(session, command),
        "faceobject" => apply_faceobject_command(session, command),
        "follow" => apply_follow_command(session, command),
        "stopfollow" => apply_stopfollow_command(session, command),
        command => Err(ScriptObjectCommandError::NotObjectMutation {
            command: command.to_string(),
        }),
    }
}

pub fn apply_script_movement(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
    movement: &ScriptMovement,
) -> Result<ScriptMovementOutcome, ScriptObjectCommandError> {
    if command.command != "applymovement" && command.command != "applymovementlasttalked" {
        return Err(ScriptObjectCommandError::NotObjectMutation {
            command: command.command.clone(),
        });
    }
    let object_id = movement_object_id(session, command)?;
    let expected =
        command
            .movement
            .clone()
            .ok_or_else(|| ScriptObjectCommandError::MissingMovement {
                object_id: object_id.clone(),
            })?;
    if movement.label != expected {
        return Err(ScriptObjectCommandError::WrongMovement {
            movement: movement.label.clone(),
            expected,
        });
    }

    let mut tile = object_tile(session, &object_id)?;
    let previous_tile = tile;
    let mut facing = object_facing(session, &object_id)?;
    let mut steps_applied = 0;
    let mut fixed_facing = false;
    let mut sliding = false;
    let mut effects = Vec::new();

    for step in &movement.steps {
        match step.command.as_str() {
            "step_end" => break,
            "fix_facing" => {
                fixed_facing = true;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "remove_fixed_facing" => {
                fixed_facing = false;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "set_sliding" => {
                sliding = true;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "remove_sliding" => {
                sliding = false;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "step_sleep" => {
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
                steps_applied += 1;
            }
            "step" | "slow_step" | "big_step" | "jump_step" | "fast_jump_step"
            | "slow_jump_step" => {
                let direction = movement_step_direction(movement, step)?;
                if !fixed_facing {
                    facing = direction;
                }
                tile = move_by_stride(tile, direction, 1);
                steps_applied += 1;
            }
            "turn_head" => {
                facing = movement_step_direction(movement, step)?;
                steps_applied += 1;
            }
            "teleport_from" | "skyfall_top" | "tree_shake" => {
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
                steps_applied += 1;
            }
            command => {
                return Err(ScriptObjectCommandError::UnsupportedMovementCommand {
                    movement: movement.label.clone(),
                    command: command.to_string(),
                    index: step.index,
                });
            }
        }
    }

    set_object_tile(session, &object_id, tile)?;
    set_object_facing(session, &object_id, facing)?;

    Ok(ScriptMovementOutcome {
        object_id,
        movement: movement.label.clone(),
        previous_tile,
        tile,
        facing,
        effects,
        fixed_facing,
        sliding,
        steps_applied,
    })
}

fn apply_visibility_command(
    state: &mut GameState,
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
    hidden: bool,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    if object_id == "PLAYER" {
        session.player_hidden = hidden;
        return Ok(ScriptObjectMutationOutcome {
            command: command.command.clone(),
            object_id,
            event_flag: None,
            previous_x: None,
            previous_y: None,
            x: None,
            y: None,
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
    }
    let object = session
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.clone(),
        })?;
    let event_flag = object.event_flag.clone();

    if event_flag == "-1" {
        if hidden {
            session.hidden_object_identifiers.insert(object_id.clone());
        } else {
            session.hidden_object_identifiers.remove(&object_id);
        }
    } else {
        validate_toggle_flag(&object_id, &event_flag)?;
        state
            .flags
            .set_event_flag(&event_flag, hidden)
            .map_err(|error| ScriptObjectCommandError::EventFlag { error })?;
        session.sync_event_flag_memory(&state.flags);
    }

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: (event_flag != "-1").then_some(event_flag),
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn apply_turnobject_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    let direction = command
        .direction
        .as_deref()
        .ok_or_else(|| ScriptObjectCommandError::MissingDirection {
            command: command.command.clone(),
        })
        .and_then(parse_script_direction)?;
    set_object_facing(session, &object_id, direction)?;

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: None,
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn apply_faceobject_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    let target_object_id = required_target_object_id(session, command)?;
    let from = object_tile(session, &object_id)?;
    let target = object_tile(session, &target_object_id)?;
    let direction = match direction_toward(from, target) {
        Some(direction) => direction,
        None => object_facing(session, &object_id)?,
    };
    set_object_facing(session, &object_id, direction)?;

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: None,
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn apply_follow_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let leader_object_id = required_object_id(session, command)?;
    let follower_object_id = required_target_object_id(session, command)?;
    validate_object_reference(session, &leader_object_id)?;
    validate_object_reference(session, &follower_object_id)?;
    session.following = Some(OverworldFollowState {
        leader_object_id: leader_object_id.clone(),
        follower_object_id: follower_object_id.clone(),
    });

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id: leader_object_id,
        event_flag: None,
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn apply_stopfollow_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    session.following = None;
    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id: "FOLLOW".to_string(),
        event_flag: None,
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn apply_moveobject_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    let x = command
        .x
        .ok_or_else(|| ScriptObjectCommandError::MissingMoveCoordinates {
            object_id: object_id.clone(),
        })?;
    let y = command
        .y
        .ok_or_else(|| ScriptObjectCommandError::MissingMoveCoordinates {
            object_id: object_id.clone(),
        })?;
    let object = session
        .objects
        .iter_mut()
        .find(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.clone(),
        })?;
    let previous_x = object.x;
    let previous_y = object.y;
    object.x = x;
    object.y = y;

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: None,
        previous_x: Some(previous_x),
        previous_y: Some(previous_y),
        x: Some(x),
        y: Some(y),
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn movement_object_id(
    session: &OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<String, ScriptObjectCommandError> {
    if command.command == "applymovementlasttalked" {
        return session
            .last_talked_object_identifier
            .clone()
            .ok_or(ScriptObjectCommandError::MissingLastTalkedObject);
    }
    required_object_id(session, command)
}

fn movement_step_direction(
    movement: &ScriptMovement,
    step: &ScriptMovementStep,
) -> Result<Direction, ScriptObjectCommandError> {
    step.direction
        .as_deref()
        .ok_or_else(|| ScriptObjectCommandError::MovementMissingDirection {
            movement: movement.label.clone(),
            command: step.command.clone(),
            index: step.index,
        })
        .and_then(parse_script_direction)
}

fn object_tile(
    session: &OverworldSession,
    object_id: &str,
) -> Result<TilePosition, ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        return Ok(session.player.tile);
    }
    session
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some(object_id))
        .map(|object| TilePosition::new(object.x as i16, object.y as i16))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        })
}

fn validate_object_reference(
    session: &OverworldSession,
    object_id: &str,
) -> Result<(), ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        return Ok(());
    }
    session
        .objects
        .iter()
        .any(|object| object.object_identifier.as_deref() == Some(object_id))
        .then_some(())
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        })
}

fn set_object_tile(
    session: &mut OverworldSession,
    object_id: &str,
    tile: TilePosition,
) -> Result<(), ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        session.player.tile = tile;
        return Ok(());
    }
    let object = session
        .objects
        .iter_mut()
        .find(|object| object.object_identifier.as_deref() == Some(object_id))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        })?;
    object.x =
        tile.x
            .try_into()
            .map_err(|_| ScriptObjectCommandError::ObjectPositionOutOfRange {
                object_id: object_id.to_string(),
                x: tile.x,
                y: tile.y,
            })?;
    object.y =
        tile.y
            .try_into()
            .map_err(|_| ScriptObjectCommandError::ObjectPositionOutOfRange {
                object_id: object_id.to_string(),
                x: tile.x,
                y: tile.y,
            })?;
    Ok(())
}

fn object_facing(
    session: &OverworldSession,
    object_id: &str,
) -> Result<Direction, ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        return Ok(session.player.facing);
    }
    if !session
        .objects
        .iter()
        .any(|object| object.object_identifier.as_deref() == Some(object_id))
    {
        return Err(ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        });
    }
    Ok(session
        .object_facings
        .get(object_id)
        .copied()
        .ok_or_else(|| ScriptObjectCommandError::MissingObjectFacing {
            object_id: object_id.to_string(),
        })?)
}

fn set_object_facing(
    session: &mut OverworldSession,
    object_id: &str,
    direction: Direction,
) -> Result<(), ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        session.player.facing = direction;
        return Ok(());
    }
    if !session
        .objects
        .iter()
        .any(|object| object.object_identifier.as_deref() == Some(object_id))
    {
        return Err(ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        });
    }
    session
        .object_facings
        .insert(object_id.to_string(), direction);
    Ok(())
}

fn required_object_id(
    session: &OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<String, ScriptObjectCommandError> {
    let object_id =
        command
            .object_id
            .as_deref()
            .ok_or_else(|| ScriptObjectCommandError::MissingObjectId {
                command: command.command.clone(),
            })?;
    resolve_script_object_id(session, object_id)
}

fn required_target_object_id(
    session: &OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<String, ScriptObjectCommandError> {
    let object_id = command.target_object_id.as_deref().ok_or_else(|| {
        ScriptObjectCommandError::MissingTargetObjectId {
            command: command.command.clone(),
        }
    })?;
    resolve_script_object_id(session, object_id)
}

fn resolve_script_object_id(
    session: &OverworldSession,
    object_id: &str,
) -> Result<String, ScriptObjectCommandError> {
    if object_id == "LAST_TALKED" {
        return session
            .last_talked_object_identifier
            .clone()
            .ok_or(ScriptObjectCommandError::MissingLastTalkedObject);
    }
    Ok(object_id.to_string())
}

fn direction_toward(from: TilePosition, target: TilePosition) -> Option<Direction> {
    let dx = target.x - from.x;
    let dy = target.y - from.y;
    if dx.abs() >= dy.abs() && dx != 0 {
        return Some(if dx > 0 {
            Direction::Right
        } else {
            Direction::Left
        });
    }
    if dy != 0 {
        return Some(if dy > 0 {
            Direction::Down
        } else {
            Direction::Up
        });
    }
    None
}

pub fn parse_script_direction(direction: &str) -> Result<Direction, ScriptObjectCommandError> {
    match direction {
        "DOWN" => Ok(Direction::Down),
        "UP" => Ok(Direction::Up),
        "LEFT" => Ok(Direction::Left),
        "RIGHT" => Ok(Direction::Right),
        direction => Err(ScriptObjectCommandError::UnknownDirection {
            direction: direction.to_string(),
        }),
    }
}

pub fn is_hideable_object_event_flag(event_flag: &str) -> bool {
    !(event_flag.is_empty() || event_flag == "0" || event_flag == "-1")
}

fn validate_toggle_flag(object_id: &str, event_flag: &str) -> Result<(), ScriptObjectCommandError> {
    if !is_hideable_object_event_flag(event_flag) {
        return Err(ScriptObjectCommandError::ObjectCannotToggle {
            object_id: object_id.to_string(),
            event_flag: event_flag.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{MapAttributes, MapEvents, ObjectEvent};
    use crate::world::collision::{MetatileCollision, TilesetCollision, permissions};
    use crate::world::map::{OverworldMapData, TilePosition};

    fn object(object_id: &str, event_flag: &str, x: u16, y: u16) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x,
            y,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "ObjectScript".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(object_id.to_string()),
            sightline_direction_override: None,
        }
    }

    fn session(objects: Vec<ObjectEvent>) -> OverworldSession {
        OverworldSession::with_events_and_objects(
            OverworldMapData::from_attributes(
                "TestMap",
                &MapAttributes {
                    tileset_name: "test".to_string(),
                    border_block: 0,
                    width: 4,
                    height: 4,
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
                },
                vec![0; 16],
            ),
            MapEvents::default(),
            objects,
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                }],
            },
            TilePosition::new(0, 0),
        )
    }

    fn command(command: &str, object_id: &str) -> ScriptObjectCommand {
        ScriptObjectCommand {
            command: command.to_string(),
            object_id: Some(object_id.to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "Script".to_string(),
            command_index: 3,
        }
    }

    #[test]
    fn session_initializes_object_facing_from_exact_movement_data() {
        let mut down = object("DOWN_OBJECT", "EVENT_DOWN", 1, 1);
        down.spritemovedata = "SPRITEMOVEDATA_STANDING_DOWN".to_string();
        let mut left = object("LEFT_OBJECT", "EVENT_LEFT", 2, 1);
        left.spritemovedata = "SPRITEMOVEDATA_STANDING_LEFT".to_string();
        let mut unknown = object("UNKNOWN_OBJECT", "EVENT_UNKNOWN", 3, 1);
        unknown.spritemovedata = "spritemovedata_standing_down".to_string();

        let session = session(vec![down, left, unknown]);

        assert_eq!(
            session.object_facings.get("DOWN_OBJECT"),
            Some(&Direction::Down)
        );
        assert_eq!(
            session.object_facings.get("LEFT_OBJECT"),
            Some(&Direction::Left)
        );
        assert!(!session.object_facings.contains_key("UNKNOWN_OBJECT"));
    }

    #[test]
    fn applymovement_requires_initialized_object_facing_without_defaulting_down() {
        let mut unknown = object("UNKNOWN_OBJECT", "EVENT_UNKNOWN", 1, 1);
        unknown.spritemovedata = "spritemovedata_standing_down".to_string();
        let mut session = session(vec![unknown]);
        let mut movement_command = command("applymovement", "UNKNOWN_OBJECT");
        movement_command.movement = Some("Waits".to_string());
        let movement = ScriptMovement {
            label: "Waits".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        };

        assert_eq!(
            apply_script_movement(&mut session, &movement_command, &movement),
            Err(ScriptObjectCommandError::MissingObjectFacing {
                object_id: "UNKNOWN_OBJECT".to_string(),
            })
        );
    }

    #[test]
    fn disappear_and_appear_toggle_exact_event_flag() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "VERMILIONCITY_BIG_SNORLAX",
            "EVENT_VERMILION_CITY_SNORLAX",
            2,
            3,
        )]);

        let disappear = apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("disappear", "VERMILIONCITY_BIG_SNORLAX"),
        )
        .expect("disappear applies");
        assert_eq!(
            disappear.event_flag.as_deref(),
            Some("EVENT_VERMILION_CITY_SNORLAX")
        );
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_VERMILION_CITY_SNORLAX"),
            Ok(true)
        );
        assert!(!session.is_object_visible(&session.objects[0]));

        apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("appear", "VERMILIONCITY_BIG_SNORLAX"),
        )
        .expect("appear applies");
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_VERMILION_CITY_SNORLAX"),
            Ok(false)
        );
        assert!(session.is_object_visible(&session.objects[0]));
    }

    #[test]
    fn disappear_and_appear_mutate_temporary_object_visibility() {
        let mut state = GameState::default();
        let mut session = session(vec![object("BATTLETOWER1F_RECEPTIONIST", "-1", 2, 3)]);

        let disappear = apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("disappear", "BATTLETOWER1F_RECEPTIONIST"),
        )
        .expect("temporary disappear applies");
        assert_eq!(disappear.event_flag, None);
        assert!(state.flags.active_event_flags().next().is_none());
        assert!(!session.is_object_visible(&session.objects[0]));

        apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("appear", "BATTLETOWER1F_RECEPTIONIST"),
        )
        .expect("temporary appear applies");
        assert!(session.is_object_visible(&session.objects[0]));
    }

    #[test]
    fn object_mutation_resolves_last_talked_operand() {
        let mut state = GameState::default();
        let mut session = session(vec![object("CELADONGAMECORNER_FISHER", "-1", 2, 3)]);
        session.last_talked_object_identifier = Some("CELADONGAMECORNER_FISHER".to_string());

        let mut turn = command("turnobject", "LAST_TALKED");
        turn.direction = Some("LEFT".to_string());

        let outcome =
            apply_script_object_mutation(&mut state, &mut session, &turn).expect("turn applies");
        assert_eq!(outcome.object_id, "CELADONGAMECORNER_FISHER");
        assert_eq!(
            session.object_facings.get("CELADONGAMECORNER_FISHER"),
            Some(&Direction::Left)
        );
    }

    #[test]
    fn disappear_and_appear_player_mutate_player_visibility() {
        let mut state = GameState::default();
        let mut session = session(Vec::new());

        apply_script_object_mutation(&mut state, &mut session, &command("disappear", "PLAYER"))
            .expect("player disappear applies");
        assert!(session.player_hidden);

        apply_script_object_mutation(&mut state, &mut session, &command("appear", "PLAYER"))
            .expect("player appear applies");
        assert!(!session.player_hidden);
    }

    #[test]
    fn object_mutation_requires_exact_object_id() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "VERMILIONCITY_BIG_SNORLAX",
            "EVENT_VERMILION_CITY_SNORLAX",
            2,
            3,
        )]);

        let error = apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("disappear", "vermilioncity_big_snorlax"),
        )
        .expect_err("object ids are exact");

        assert_eq!(
            error,
            ScriptObjectCommandError::UnknownObject {
                object_id: "vermilioncity_big_snorlax".to_string()
            }
        );
    }

    #[test]
    fn moveobject_updates_exact_object_coordinates() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "INDIGOPLATEAUPOKECENTER1F_RIVAL",
            "EVENT_INDIGO_PLATEAU_POKECENTER_RIVAL",
            1,
            1,
        )]);
        let mut moveobject = command("moveobject", "INDIGOPLATEAUPOKECENTER1F_RIVAL");
        moveobject.x = Some(17);
        moveobject.y = Some(9);

        let outcome = apply_script_object_mutation(&mut state, &mut session, &moveobject)
            .expect("moveobject applies");

        assert_eq!((outcome.previous_x, outcome.previous_y), (Some(1), Some(1)));
        assert_eq!((outcome.x, outcome.y), (Some(17), Some(9)));
        assert_eq!((session.objects[0].x, session.objects[0].y), (17, 9));
    }

    #[test]
    fn turnobject_sets_exact_object_facing_without_direction_coercion() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "ROUTE43GATE_ROCKET1",
            "EVENT_ROUTE43GATE_ROCKETS",
            4,
            4,
        )]);
        let mut turn = command("turnobject", "ROUTE43GATE_ROCKET1");
        turn.direction = Some("UP".to_string());

        apply_script_object_mutation(&mut state, &mut session, &turn).expect("turnobject applies");
        assert_eq!(
            session.object_facings.get("ROUTE43GATE_ROCKET1"),
            Some(&Direction::Up)
        );

        turn.direction = Some("up".to_string());
        let error = apply_script_object_mutation(&mut state, &mut session, &turn)
            .expect_err("direction ids are exact");
        assert_eq!(
            error,
            ScriptObjectCommandError::UnknownDirection {
                direction: "up".to_string()
            }
        );
    }

    #[test]
    fn follow_stopfollow_and_faceobject_mutate_exact_session_state() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("BATTLETOWER1F_RECEPTIONIST", "EVENT_BT_RECEPTIONIST", 4, 4),
            object("BATTLETOWERHALLWAY_RECEPTIONIST", "EVENT_BT_HALLWAY", 4, 2),
        ]);
        session.player.tile = TilePosition::new(4, 6);

        let mut follow = command("follow", "BATTLETOWER1F_RECEPTIONIST");
        follow.target_object_id = Some("PLAYER".to_string());
        apply_script_object_mutation(&mut state, &mut session, &follow).expect("follow applies");
        assert_eq!(
            session.following,
            Some(OverworldFollowState {
                leader_object_id: "BATTLETOWER1F_RECEPTIONIST".to_string(),
                follower_object_id: "PLAYER".to_string(),
            })
        );

        let mut face = command("faceobject", "PLAYER");
        face.target_object_id = Some("BATTLETOWERHALLWAY_RECEPTIONIST".to_string());
        apply_script_object_mutation(&mut state, &mut session, &face).expect("faceobject applies");
        assert_eq!(session.player.facing, Direction::Up);

        let stop = ScriptObjectCommand {
            command: "stopfollow".to_string(),
            object_id: None,
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "Script".to_string(),
            command_index: 9,
        };
        apply_script_object_mutation(&mut state, &mut session, &stop).expect("stopfollow applies");
        assert_eq!(session.following, None);
    }

    #[test]
    fn applymovement_moves_player_and_objects_from_exact_steps() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            4,
            4,
        )]);
        session.player.tile = TilePosition::new(1, 1);
        let mut player_command = command("applymovement", "PLAYER");
        player_command.movement = Some("PlayerWalks".to_string());
        let player_movement = ScriptMovement {
            label: "PlayerWalks".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "turn_head".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 2,
                },
            ],
        };

        let player_outcome = apply_script_movement(&mut session, &player_command, &player_movement)
            .expect("player movement applies");
        assert_eq!(player_outcome.previous_tile, TilePosition::new(1, 1));
        assert_eq!(player_outcome.tile, TilePosition::new(1, 0));
        assert_eq!(session.player.tile, TilePosition::new(1, 0));
        assert_eq!(session.player.facing, Direction::Right);

        let mut bill_command = command("applymovement", "ECRUTEAKPOKECENTER1F_BILL");
        bill_command.movement = Some("BillWalks".to_string());
        let bill_movement = ScriptMovement {
            label: "BillWalks".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 2,
                },
            ],
        };

        let bill_outcome = apply_script_movement(&mut session, &bill_command, &bill_movement)
            .expect("object moves");
        assert_eq!(bill_outcome.previous_tile, TilePosition::new(4, 4));
        assert_eq!(bill_outcome.tile, TilePosition::new(5, 5));
        assert_eq!((session.objects[0].x, session.objects[0].y), (5, 5));
        assert_eq!(
            session.object_facings.get("ECRUTEAKPOKECENTER1F_BILL"),
            Some(&Direction::Down)
        );
    }

    #[test]
    fn applymovementlasttalked_uses_recorded_exact_object_identifier() {
        let mut session = session(vec![object(
            "POKECENTER2F_RECEPTIONIST",
            "EVENT_RECEPTIONIST",
            5,
            5,
        )]);
        session.last_talked_object_identifier = Some("POKECENTER2F_RECEPTIONIST".to_string());
        let command = ScriptObjectCommand {
            command: "applymovementlasttalked".to_string(),
            object_id: None,
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("ReceptionistWalks".to_string()),
            emote: None,
            duration: None,
            source_script: "Script".to_string(),
            command_index: 4,
        };
        let movement = ScriptMovement {
            label: "ReceptionistWalks".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
            ],
        };

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("last talked moves");
        assert_eq!(outcome.object_id, "POKECENTER2F_RECEPTIONIST");
        assert_eq!(outcome.previous_tile, TilePosition::new(5, 5));
        assert_eq!(outcome.tile, TilePosition::new(5, 4));
        assert_eq!((session.objects[0].x, session.objects[0].y), (5, 4));
    }

    #[test]
    fn applymovement_honors_fixed_facing_and_records_sliding_effects() {
        let mut session = session(Vec::new());
        session.player.facing = Direction::Left;
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("Slide".to_string());
        let movement = ScriptMovement {
            label: "Slide".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "fix_facing".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "set_sliding".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "slow_jump_step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "remove_sliding".to_string(),
                    direction: None,
                    duration: None,
                    index: 3,
                },
                ScriptMovementStep {
                    command: "remove_fixed_facing".to_string(),
                    direction: None,
                    duration: None,
                    index: 4,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 5,
                },
            ],
        };

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("slide applies");

        assert_eq!(outcome.previous_tile, TilePosition::new(0, 0));
        assert_eq!(outcome.tile, TilePosition::new(1, 0));
        assert_eq!(outcome.facing, Direction::Left);
        assert_eq!(session.player.facing, Direction::Left);
        assert!(!outcome.fixed_facing);
        assert!(!outcome.sliding);
        assert_eq!(
            outcome.effects,
            vec![
                ScriptMovementEffect {
                    command: "fix_facing".to_string(),
                    index: 0,
                },
                ScriptMovementEffect {
                    command: "set_sliding".to_string(),
                    index: 1,
                },
                ScriptMovementEffect {
                    command: "remove_sliding".to_string(),
                    index: 3,
                },
                ScriptMovementEffect {
                    command: "remove_fixed_facing".to_string(),
                    index: 4,
                },
            ]
        );
    }

    #[test]
    fn applymovement_records_visual_movement_opcodes_without_position_fallbacks() {
        let mut session = session(Vec::new());
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("Visuals".to_string());
        let movement = ScriptMovement {
            label: "Visuals".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "teleport_from".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "skyfall_top".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "tree_shake".to_string(),
                    direction: None,
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 3,
                },
            ],
        };

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("visuals apply");
        assert_eq!(outcome.tile, TilePosition::new(0, 0));
        assert_eq!(
            outcome.effects,
            vec![
                ScriptMovementEffect {
                    command: "teleport_from".to_string(),
                    index: 0,
                },
                ScriptMovementEffect {
                    command: "skyfall_top".to_string(),
                    index: 1,
                },
                ScriptMovementEffect {
                    command: "tree_shake".to_string(),
                    index: 2,
                },
            ]
        );
    }

    #[test]
    fn applymovement_rejects_unknown_movement_opcodes() {
        let mut session = session(Vec::new());
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("Unknown".to_string());
        let movement = ScriptMovement {
            label: "Unknown".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "spin_forever".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        };

        let error = apply_script_movement(&mut session, &command, &movement)
            .expect_err("unknown movement must be explicit");
        assert_eq!(
            error,
            ScriptObjectCommandError::UnsupportedMovementCommand {
                movement: "Unknown".to_string(),
                command: "spin_forever".to_string(),
                index: 0,
            }
        );
    }
}
