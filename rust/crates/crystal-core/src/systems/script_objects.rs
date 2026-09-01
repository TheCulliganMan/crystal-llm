use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::{EventFlagError, GameState, ScriptRuntimeEmote};
use crate::systems::script_runtime::script_label_parent;
use crate::timing::wrapping_byte_counter_ticks;
use crate::world::session::{
    FollowQueuedStep, OverworldFollowState, OverworldSession,
    raw_event_tile_to_runtime_tile_checked, runtime_tile_to_raw_event_tile,
};
use crate::world::{
    map::{Direction, TilePosition},
    movement::{DEFAULT_RUNTIME_TILE_STRIDE, checked_move_by_stride},
};

// Script movement opcodes are authored in the same exact runtime tile
// coordinate space as objects, warps, and player movement.
pub const SCRIPT_MOVEMENT_EVENT_TILE_STRIDE: i16 = DEFAULT_RUNTIME_TILE_STRIDE;
pub const SCRIPT_MOVEMENT_JUMP_TILE_STRIDE: i16 = SCRIPT_MOVEMENT_EVENT_TILE_STRIDE * 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptObjectCommand {
    #[serde(deserialize_with = "required_script_object_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_script_object_token")]
    pub object_id: Option<String>,
    #[serde(deserialize_with = "required_nullable_script_object_token")]
    pub target_object_id: Option<String>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    #[serde(deserialize_with = "required_nullable_script_object_token")]
    pub direction: Option<String>,
    #[serde(deserialize_with = "required_nullable_script_object_token")]
    pub movement: Option<String>,
    #[serde(deserialize_with = "required_nullable_script_object_token")]
    pub emote: Option<String>,
    pub duration: Option<u16>,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptObjectCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptObjectCommand {
            #[serde(deserialize_with = "required_script_object_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_script_object_token")]
            object_id: Option<String>,
            #[serde(deserialize_with = "required_nullable_script_object_token")]
            target_object_id: Option<String>,
            x: Option<u16>,
            y: Option<u16>,
            #[serde(deserialize_with = "required_nullable_script_object_token")]
            direction: Option<String>,
            #[serde(deserialize_with = "required_nullable_script_object_token")]
            movement: Option<String>,
            #[serde(deserialize_with = "required_nullable_script_object_token")]
            emote: Option<String>,
            duration: Option<u16>,
            #[serde(deserialize_with = "required_script_label_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptObjectCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            object_id: raw.object_id,
            target_object_id: raw.target_object_id,
            x: raw.x,
            y: raw.y,
            direction: raw.direction,
            movement: raw.movement,
            emote: raw.emote,
            duration: raw.duration,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_object_command_shape(&command).map_err(D::Error::custom)?;
        Ok(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovement {
    #[serde(deserialize_with = "required_script_object_token")]
    pub label: String,
    #[serde(deserialize_with = "required_nullable_script_label_token")]
    pub source_script: Option<String>,
    pub steps: Vec<ScriptMovementStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovementStep {
    #[serde(deserialize_with = "required_script_object_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_script_object_token")]
    pub direction: Option<String>,
    pub duration: Option<u16>,
    pub index: usize,
}

impl<'de> Deserialize<'de> for ScriptMovementStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptMovementStep {
            #[serde(deserialize_with = "required_script_object_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_script_object_token")]
            direction: Option<String>,
            duration: Option<u16>,
            index: usize,
        }

        let raw = RawScriptMovementStep::deserialize(deserializer)?;
        let step = Self {
            command: raw.command,
            direction: raw.direction,
            duration: raw.duration,
            index: raw.index,
        };
        if let Some(issue) = script_movement_step_issues(&step).into_iter().next() {
            return Err(D::Error::custom(format!(
                "invalid movement step: {issue:?}"
            )));
        }
        Ok(step)
    }
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
    pub previous_facing: Direction,
    pub previous_hidden: bool,
    pub previous_follower: Option<ScriptMovementFollower>,
    pub tile: TilePosition,
    pub facing: Direction,
    pub executed_steps: Vec<ScriptMovementStep>,
    pub effects: Vec<ScriptMovementEffect>,
    pub fixed_facing: bool,
    pub sliding: bool,
    pub steps_applied: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMovementFollower {
    pub object_id: String,
    pub tile: TilePosition,
    pub facing: Direction,
    pub queued_step: Option<FollowQueuedStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptObjectCommandError {
    #[error("script object command '{command}' is not a state mutation")]
    NotObjectMutation { command: String },
    #[error("script object command '{command}' is missing an object id")]
    MissingObjectId { command: String },
    #[error("script object command '{command}' is missing a target object id")]
    MissingTargetObjectId { command: String },
    #[error("unknown script object '{object_id}'")]
    UnknownObject { object_id: String },
    #[error("script object id '{object_id}' is not an exact pack token")]
    InvalidObjectId { object_id: String },
    #[error("script object '{object_id}' has out-of-range raw event coordinate ({x}, {y})")]
    ObjectCoordinatesOutOfRange { object_id: String, x: u16, y: u16 },
    #[error("script object source script '{source_script}' is invalid")]
    InvalidSourceScript { source_script: String },
    #[error("object '{object_id}' has no initialized facing")]
    MissingObjectFacing { object_id: String },
    #[error("object '{object_id}' cannot be hidden or shown with event flag '{event_flag}'")]
    ObjectCannotToggle {
        object_id: String,
        event_flag: String,
    },
    #[error("moveobject for '{object_id}' is missing x/y coordinates")]
    MissingMoveCoordinates { object_id: String },
    #[error("moveobject for '{object_id}' has out-of-range raw event coordinate ({x}, {y})")]
    MoveCoordinatesOutOfRange { object_id: String, x: u16, y: u16 },
    #[error(
        "moveobject for '{object_id}' raw event coordinate resolves outside map {map_name} runtime tile bounds {width}x{height}: raw ({x}, {y})"
    )]
    MoveCoordinatesOutOfMap {
        object_id: String,
        map_name: String,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    #[error("script object command '{command}' is missing a direction")]
    MissingDirection { command: String },
    #[error("script object command '{command}' is missing emote payload")]
    MissingEmote { command: String },
    #[error("showemote duration {duration} does not fit the script byte")]
    EmoteDurationOutOfByteRange { duration: u16 },
    #[error("unknown script direction '{direction}'")]
    UnknownDirection { direction: String },
    #[error("applymovement for '{object_id}' is missing a movement label")]
    MissingMovement { object_id: String },
    #[error("movement label '{movement}' is not an exact pack token")]
    InvalidMovement { movement: String },
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
        "movement command '{command}' in movement '{movement}' at index {index} has no runtime stride"
    )]
    MovementMissingRuntimeStride {
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
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} has unexpected direction payload"
    )]
    MovementUnexpectedDirection {
        movement: String,
        command: String,
        index: usize,
    },
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} has unknown direction '{direction}'"
    )]
    MovementUnknownDirection {
        movement: String,
        command: String,
        index: usize,
        direction: String,
    },
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} is missing a duration"
    )]
    MovementMissingDuration {
        movement: String,
        command: String,
        index: usize,
    },
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} has unexpected duration payload"
    )]
    MovementUnexpectedDuration {
        movement: String,
        command: String,
        index: usize,
    },
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} has duration {duration}, which does not fit its byte parameter"
    )]
    MovementDurationOutOfByteRange {
        movement: String,
        command: String,
        index: usize,
        duration: u16,
    },
    #[error(
        "movement command 'step_sleep' in movement '{movement}' at index {index} cannot encode source duration zero"
    )]
    MovementZeroSleepDuration { movement: String, index: usize },
    #[error(
        "movement command '{command}' in movement '{movement}' at index {index} overflows supported runtime coordinates from ({x}, {y})"
    )]
    MovementRuntimeTileOverflow {
        movement: String,
        command: String,
        index: usize,
        x: i16,
        y: i16,
    },
    #[error("movement for '{object_id}' moved outside unsigned object coordinates: ({x}, {y})")]
    ObjectPositionOutOfRange { object_id: String, x: i16, y: i16 },
    #[error("movement for '{object_id}' ended on unsaveable object tile ({x}, {y})")]
    ObjectPositionUnsavable { object_id: String, x: i16, y: i16 },
    #[error("follow object '{object_id}' is missing from the overworld session")]
    FollowObjectMissing { object_id: String },
    #[error("follow object '{object_id}' cannot be moved to unsaveable runtime tile ({x}, {y})")]
    FollowPositionUnsavable { object_id: String, x: i16, y: i16 },
    #[error("map {map_name} runtime tile bounds overflow supported coordinates")]
    MapBoundsOverflow { map_name: String },
    #[error(
        "movement for '{object_id}' ended outside map {map_name} runtime bounds: ({x}, {y}) not within {width}x{height}"
    )]
    ObjectPositionOutOfMap {
        object_id: String,
        map_name: String,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
    #[error("event flag error: {error}")]
    EventFlag { error: EventFlagError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptMovementStepIssue {
    UnexpectedDirection,
    UnexpectedDuration,
    MissingDirection,
    MissingDuration,
    DurationOutOfByteRange { duration: u16 },
    ZeroSleepDuration,
    UnknownDirection { direction: String },
    UnsupportedCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptObjectCommandIssue {
    InvalidSourceScript {
        source_script: String,
        command_index: usize,
    },
    InvalidCommand {
        source_script: String,
        command_index: usize,
        command: String,
    },
    MissingObjectId {
        source_script: String,
        command_index: usize,
        command: String,
    },
    UnknownObjectId {
        source_script: String,
        command_index: usize,
        command: String,
        object_id: String,
    },
    InvalidObjectId {
        source_script: String,
        command_index: usize,
        command: String,
        object_id: String,
    },
    UnhideableObject {
        source_script: String,
        command_index: usize,
        command: String,
        object_id: String,
        event_flag: String,
    },
    MissingCoordinates {
        source_script: String,
        command_index: usize,
    },
    MoveCoordinatesOutOfRange {
        source_script: String,
        command_index: usize,
        x: u16,
        y: u16,
    },
    MissingDirection {
        source_script: String,
        command_index: usize,
    },
    UnknownDirection {
        source_script: String,
        command_index: usize,
        direction: String,
    },
    MissingTargetObjectId {
        source_script: String,
        command_index: usize,
        command: String,
    },
    UnknownTargetObjectId {
        source_script: String,
        command_index: usize,
        command: String,
        object_id: String,
    },
    InvalidTargetObjectId {
        source_script: String,
        command_index: usize,
        command: String,
        object_id: String,
    },
    MissingMovement {
        source_script: String,
        command_index: usize,
        command: String,
    },
    UnknownMovement {
        source_script: String,
        command_index: usize,
        command: String,
        movement: String,
    },
    InvalidMovement {
        source_script: String,
        command_index: usize,
        command: String,
        movement: String,
    },
    MissingEmote {
        source_script: String,
        command_index: usize,
    },
    EmoteDurationOutOfByteRange {
        source_script: String,
        command_index: usize,
        duration: u16,
    },
    UnknownCommand {
        source_script: String,
        command_index: usize,
        command: String,
    },
}

pub const SCRIPT_OBJECT_VISIBILITY_COMMANDS: &[&str] = &["appear", "disappear"];
pub const SCRIPT_OBJECT_COORDINATE_COMMANDS: &[&str] = &["moveobject"];
pub const SCRIPT_OBJECT_WRITE_COORDINATE_COMMANDS: &[&str] = &["writeobjectxy"];
pub const SCRIPT_OBJECT_DIRECTION_COMMANDS: &[&str] = &["turnobject"];
pub const SCRIPT_OBJECT_TARGET_COMMANDS: &[&str] = &["faceobject", "follow", "follownotexact"];
pub const SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS: &[&str] = &["applymovement"];
pub const SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS: &[&str] = &["applymovementlasttalked"];
pub const SCRIPT_OBJECT_MOVEMENT_COMMANDS: &[&str] = &["applymovement", "applymovementlasttalked"];
pub const SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS: &[&str] = &["faceplayer", "stopfollow"];
pub const SCRIPT_OBJECT_EMOTE_COMMANDS: &[&str] = &["showemote"];
pub const SCRIPT_MOVEMENT_PLAYER_FACING_DIRECTION: &str = "PLAYER_FACING";

pub const SCRIPT_MOVEMENT_DIRECTION_COMMANDS: &[&str] = &[
    "step",
    "slow_step",
    "big_step",
    "turn_step",
    "jump_step",
    "fast_jump_step",
    "slow_jump_step",
    "slide_step",
    "fast_slide_step",
    "slow_slide_step",
    "step_bump",
    "turn_head",
    "turn_away",
    "turn_in",
    "turn_waterfall",
];
pub const SCRIPT_MOVEMENT_REQUIRED_DURATION_COMMANDS: &[&str] = &[
    "step_sleep",
    "step_wait_end",
    "step_dig",
    "step_shake",
    "rock_smash",
    "return_dig",
];
pub const SCRIPT_MOVEMENT_NO_ARG_COMMANDS: &[&str] = &[
    "step_end",
    "step_loop",
    "step_stop",
    "fix_facing",
    "remove_fixed_facing",
    "set_sliding",
    "remove_sliding",
    "teleport_from",
    "teleport_to",
    "skyfall",
    "skyfall_top",
    "fish_got_bite",
    "fish_cast_rod",
    "hide_emote",
    "show_emote",
    "tree_shake",
    "remove_object",
    "hide_object",
    "show_object",
];
pub const SCRIPT_MOVEMENT_COMMANDS: &[&str] = &[
    "step",
    "slow_step",
    "big_step",
    "turn_step",
    "jump_step",
    "fast_jump_step",
    "slow_jump_step",
    "slide_step",
    "fast_slide_step",
    "slow_slide_step",
    "step_bump",
    "turn_head",
    "turn_away",
    "turn_in",
    "turn_waterfall",
    "step_sleep",
    "step_wait_end",
    "step_end",
    "step_loop",
    "step_stop",
    "fix_facing",
    "remove_fixed_facing",
    "set_sliding",
    "remove_sliding",
    "teleport_from",
    "teleport_to",
    "skyfall",
    "skyfall_top",
    "step_dig",
    "fish_got_bite",
    "fish_cast_rod",
    "hide_emote",
    "show_emote",
    "step_shake",
    "tree_shake",
    "rock_smash",
    "return_dig",
    "remove_object",
    "hide_object",
    "show_object",
];

pub fn is_known_script_object_command(command: &str) -> bool {
    SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_WRITE_COORDINATE_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command)
}

pub fn is_known_script_movement_command(command: &str) -> bool {
    SCRIPT_MOVEMENT_COMMANDS.contains(&command)
}

pub fn is_script_movement_terminator(command: &str) -> bool {
    matches!(
        command,
        "step_end" | "step_wait_end" | "remove_object" | "step_stop" | "step_loop"
    )
}

fn validate_script_object_command_shape(command: &ScriptObjectCommand) -> Result<(), String> {
    if !is_known_script_object_command(&command.command) {
        return Err(format!("unknown script object command {}", command.command));
    }
    match command.command.as_str() {
        command_name if SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command_name) => {
            reject_object_payload(command, command_name)?;
        }
        command_name if SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command_name) => {
            require_object_id_shape(command, command_name)?;
            reject_coordinates(command, command_name)?;
            reject_target_object_id(command, command_name)?;
            reject_direction(command, command_name)?;
            reject_movement(command, command_name)?;
            reject_emote(command, command_name)?;
        }
        command_name if SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command_name) => {
            require_object_id_shape(command, command_name)?;
            if command.x.is_none() || command.y.is_none() {
                return Err(format!(
                    "script object command {command_name} requires x and y"
                ));
            }
            if !moveobject_raw_coordinates_fit_runtime_tile(command) {
                return Err(format!(
                    "script object command {command_name} raw event coordinates overflow runtime tile space"
                ));
            }
            reject_target_object_id(command, command_name)?;
            reject_direction(command, command_name)?;
            reject_movement(command, command_name)?;
            reject_emote(command, command_name)?;
        }
        command_name if SCRIPT_OBJECT_WRITE_COORDINATE_COMMANDS.contains(&command_name) => {
            require_object_id_shape(command, command_name)?;
            reject_coordinates(command, command_name)?;
            reject_target_object_id(command, command_name)?;
            reject_direction(command, command_name)?;
            reject_movement(command, command_name)?;
            reject_emote(command, command_name)?;
        }
        command_name if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command_name) => {
            require_object_id_shape(command, command_name)?;
            let direction = command.direction.as_deref().ok_or_else(|| {
                format!("script object command {command_name} requires direction")
            })?;
            parse_script_direction(direction).map_err(|error| error.to_string())?;
            reject_coordinates(command, command_name)?;
            reject_target_object_id(command, command_name)?;
            reject_movement(command, command_name)?;
            reject_emote(command, command_name)?;
        }
        command_name if SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command_name) => {
            require_object_id_shape(command, command_name)?;
            require_target_object_id_shape(command, command_name)?;
            reject_coordinates(command, command_name)?;
            reject_direction(command, command_name)?;
            reject_movement(command, command_name)?;
            reject_emote(command, command_name)?;
        }
        "applymovement" => {
            require_object_id_shape(command, "applymovement")?;
            require_movement_shape(command, "applymovement")?;
            reject_coordinates(command, "applymovement")?;
            reject_target_object_id(command, "applymovement")?;
            reject_direction(command, "applymovement")?;
            reject_emote(command, "applymovement")?;
        }
        "applymovementlasttalked" => {
            require_movement_shape(command, "applymovementlasttalked")?;
            reject_object_id(command, "applymovementlasttalked")?;
            reject_coordinates(command, "applymovementlasttalked")?;
            reject_target_object_id(command, "applymovementlasttalked")?;
            reject_direction(command, "applymovementlasttalked")?;
            reject_emote(command, "applymovementlasttalked")?;
        }
        command_name if SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command_name) => {
            require_object_id_shape(command, command_name)?;
            if command.emote.is_none() || command.duration.is_none() {
                return Err(format!(
                    "script object command {command_name} requires emote and duration"
                ));
            }
            if command
                .duration
                .is_some_and(|duration| duration > u16::from(u8::MAX))
            {
                return Err(format!(
                    "script object command {command_name} duration does not fit a byte"
                ));
            }
            reject_coordinates(command, command_name)?;
            reject_target_object_id(command, command_name)?;
            reject_direction(command, command_name)?;
            reject_movement(command, command_name)?;
        }
        _ => unreachable!("known script object command was not handled"),
    }
    Ok(())
}

fn require_object_id_shape(
    command: &ScriptObjectCommand,
    command_name: &str,
) -> Result<(), String> {
    command
        .object_id
        .as_deref()
        .ok_or_else(|| format!("script object command {command_name} requires object_id"))?;
    Ok(())
}

fn require_target_object_id_shape(
    command: &ScriptObjectCommand,
    command_name: &str,
) -> Result<(), String> {
    command
        .target_object_id
        .as_deref()
        .ok_or_else(|| format!("script object command {command_name} requires target_object_id"))?;
    Ok(())
}

fn require_movement_shape(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    command
        .movement
        .as_deref()
        .ok_or_else(|| format!("script object command {command_name} requires movement"))?;
    Ok(())
}

fn reject_object_payload(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    reject_object_id(command, command_name)?;
    reject_coordinates(command, command_name)?;
    reject_target_object_id(command, command_name)?;
    reject_direction(command, command_name)?;
    reject_movement(command, command_name)?;
    reject_emote(command, command_name)
}

fn reject_object_id(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    if command.object_id.is_some() {
        Err(format!(
            "script object command {command_name} must not declare object_id"
        ))
    } else {
        Ok(())
    }
}

fn reject_target_object_id(
    command: &ScriptObjectCommand,
    command_name: &str,
) -> Result<(), String> {
    if command.target_object_id.is_some() {
        Err(format!(
            "script object command {command_name} must not declare target_object_id"
        ))
    } else {
        Ok(())
    }
}

fn reject_coordinates(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    if command.x.is_some() || command.y.is_some() {
        Err(format!(
            "script object command {command_name} must not declare coordinates"
        ))
    } else {
        Ok(())
    }
}

fn reject_direction(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    if command.direction.is_some() {
        Err(format!(
            "script object command {command_name} must not declare direction"
        ))
    } else {
        Ok(())
    }
}

fn reject_movement(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    if command.movement.is_some() {
        Err(format!(
            "script object command {command_name} must not declare movement"
        ))
    } else {
        Ok(())
    }
}

fn reject_emote(command: &ScriptObjectCommand, command_name: &str) -> Result<(), String> {
    if command.emote.is_some() || command.duration.is_some() {
        Err(format!(
            "script object command {command_name} must not declare emote payload"
        ))
    } else {
        Ok(())
    }
}

pub fn script_object_command_issues(
    command: &ScriptObjectCommand,
    object_event_flags: &BTreeMap<String, String>,
    hideable_event_flags: &BTreeSet<String>,
    movements: &BTreeSet<(String, Option<String>)>,
) -> Vec<ScriptObjectCommandIssue> {
    let mut issues = Vec::new();
    if !is_exact_script_label_token(&command.source_script) {
        issues.push(ScriptObjectCommandIssue::InvalidSourceScript {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
    }
    if !is_exact_script_object_command_token(&command.command) {
        issues.push(ScriptObjectCommandIssue::InvalidCommand {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
            command: command.command.clone(),
        });
    } else if SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command.command.as_str()) {
    } else if SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command.command.as_str()) {
        let Some(object_id) = command.object_id.as_deref() else {
            issues.push(missing_object_id(command));
            return issues;
        };
        if object_id == "LAST_TALKED" || object_id == "PLAYER" {
            return issues;
        }
        if !is_exact_script_object_token(object_id) {
            issues.push(invalid_object_id(command, object_id));
            return issues;
        }
        let Some(event_flag) = object_event_flags.get(object_id) else {
            issues.push(unknown_object_id(command, object_id));
            return issues;
        };
        if event_flag != "-1" && !hideable_event_flags.contains(event_flag) {
            issues.push(ScriptObjectCommandIssue::UnhideableObject {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
                command: command.command.clone(),
                object_id: object_id.to_string(),
                event_flag: event_flag.clone(),
            });
        }
    } else if SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command.command.as_str()) {
        collect_required_object_id_issue(command, object_event_flags, false, &mut issues);
        if command.x.is_none() || command.y.is_none() {
            issues.push(ScriptObjectCommandIssue::MissingCoordinates {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            });
        } else if !moveobject_raw_coordinates_fit_runtime_tile(command)
            && let (Some(x), Some(y)) = (command.x, command.y)
        {
            issues.push(ScriptObjectCommandIssue::MoveCoordinatesOutOfRange {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
                x,
                y,
            });
        }
    } else if SCRIPT_OBJECT_WRITE_COORDINATE_COMMANDS.contains(&command.command.as_str()) {
        collect_required_object_id_issue(command, object_event_flags, true, &mut issues);
    } else if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command.command.as_str())
    {
        collect_required_object_id_issue(command, object_event_flags, true, &mut issues);
        if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command.command.as_str()) {
            collect_direction_issue(command, &mut issues);
        }
        if SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command.command.as_str()) {
            collect_required_target_object_id_issue(command, object_event_flags, true, &mut issues);
        }
    } else if SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()) {
        if SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()) {
            collect_required_object_id_issue(command, object_event_flags, true, &mut issues);
        }
        let Some(movement) = command.movement.as_deref() else {
            issues.push(ScriptObjectCommandIssue::MissingMovement {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
                command: command.command.clone(),
            });
            return issues;
        };
        if !is_exact_script_object_token(movement) {
            issues.push(ScriptObjectCommandIssue::InvalidMovement {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
                command: command.command.clone(),
                movement: movement.to_string(),
            });
            return issues;
        }
        let movement_source = script_label_parent(&command.source_script);
        if !movements.contains(&(movement.to_string(), Some(movement_source.to_string()))) {
            issues.push(ScriptObjectCommandIssue::UnknownMovement {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
                command: command.command.clone(),
                movement: movement.to_string(),
            });
        }
    } else if SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command.command.as_str()) {
        collect_required_object_id_issue(command, object_event_flags, true, &mut issues);
        if command.emote.is_none() || command.duration.is_none() {
            issues.push(ScriptObjectCommandIssue::MissingEmote {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            });
        } else if let Some(duration) = command
            .duration
            .filter(|duration| *duration > u16::from(u8::MAX))
        {
            issues.push(ScriptObjectCommandIssue::EmoteDurationOutOfByteRange {
                source_script: command.source_script.clone(),
                command_index: command.command_index,
                duration,
            });
        }
    } else if !is_known_script_object_command(&command.command) {
        issues.push(ScriptObjectCommandIssue::UnknownCommand {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
            command: command.command.clone(),
        });
    }
    issues
}

fn collect_required_object_id_issue(
    command: &ScriptObjectCommand,
    object_event_flags: &BTreeMap<String, String>,
    allow_player: bool,
    issues: &mut Vec<ScriptObjectCommandIssue>,
) {
    let Some(object_id) = command.object_id.as_deref() else {
        issues.push(missing_object_id(command));
        return;
    };
    if (allow_player && object_id == "PLAYER") || object_id == "LAST_TALKED" {
        return;
    }
    if !is_exact_script_object_token(object_id) {
        issues.push(invalid_object_id(command, object_id));
        return;
    }
    if !object_event_flags.contains_key(object_id) {
        issues.push(unknown_object_id(command, object_id));
    }
}

fn collect_required_target_object_id_issue(
    command: &ScriptObjectCommand,
    object_event_flags: &BTreeMap<String, String>,
    allow_player: bool,
    issues: &mut Vec<ScriptObjectCommandIssue>,
) {
    let Some(object_id) = command.target_object_id.as_deref() else {
        issues.push(ScriptObjectCommandIssue::MissingTargetObjectId {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
            command: command.command.clone(),
        });
        return;
    };
    if (allow_player && object_id == "PLAYER") || object_id == "LAST_TALKED" {
        return;
    }
    if !is_exact_script_object_token(object_id) {
        issues.push(ScriptObjectCommandIssue::InvalidTargetObjectId {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
            command: command.command.clone(),
            object_id: object_id.to_string(),
        });
        return;
    }
    if !object_event_flags.contains_key(object_id) {
        issues.push(ScriptObjectCommandIssue::UnknownTargetObjectId {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
            command: command.command.clone(),
            object_id: object_id.to_string(),
        });
    }
}

fn collect_direction_issue(
    command: &ScriptObjectCommand,
    issues: &mut Vec<ScriptObjectCommandIssue>,
) {
    let Some(direction) = command.direction.as_deref() else {
        issues.push(ScriptObjectCommandIssue::MissingDirection {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
        return;
    };
    if parse_script_direction(direction).is_err() {
        issues.push(ScriptObjectCommandIssue::UnknownDirection {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
            direction: direction.to_string(),
        });
    }
}

fn missing_object_id(command: &ScriptObjectCommand) -> ScriptObjectCommandIssue {
    ScriptObjectCommandIssue::MissingObjectId {
        source_script: command.source_script.clone(),
        command_index: command.command_index,
        command: command.command.clone(),
    }
}

fn unknown_object_id(command: &ScriptObjectCommand, object_id: &str) -> ScriptObjectCommandIssue {
    ScriptObjectCommandIssue::UnknownObjectId {
        source_script: command.source_script.clone(),
        command_index: command.command_index,
        command: command.command.clone(),
        object_id: object_id.to_string(),
    }
}

fn invalid_object_id(command: &ScriptObjectCommand, object_id: &str) -> ScriptObjectCommandIssue {
    ScriptObjectCommandIssue::InvalidObjectId {
        source_script: command.source_script.clone(),
        command_index: command.command_index,
        command: command.command.clone(),
        object_id: object_id.to_string(),
    }
}

fn is_exact_script_object_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_object_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !has_reserved_pack_prefix(value)
}

fn required_script_object_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_object_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script object command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_script_object_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_object_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script object token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_nullable_script_object_token<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_script_object_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script object token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_script_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_label_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script label token must be exact visible ASCII, found {value:?}"
        )))
    }
}

fn required_nullable_script_label_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_script_label_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script label token must be exact visible ASCII, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn validate_script_object_command_source(
    command: &ScriptObjectCommand,
) -> Result<(), ScriptObjectCommandError> {
    if is_exact_script_label_token(&command.source_script) {
        Ok(())
    } else {
        Err(ScriptObjectCommandError::InvalidSourceScript {
            source_script: command.source_script.clone(),
        })
    }
}

pub fn script_movement_step_issues(step: &ScriptMovementStep) -> Vec<ScriptMovementStepIssue> {
    let command = step.command.as_str();
    if SCRIPT_MOVEMENT_NO_ARG_COMMANDS.contains(&command) {
        let mut issues = Vec::new();
        if step.direction.is_some() {
            issues.push(ScriptMovementStepIssue::UnexpectedDirection);
        }
        if step.duration.is_some() {
            issues.push(ScriptMovementStepIssue::UnexpectedDuration);
        }
        issues
    } else if SCRIPT_MOVEMENT_REQUIRED_DURATION_COMMANDS.contains(&command) {
        let mut issues = Vec::new();
        if step.direction.is_some() {
            issues.push(ScriptMovementStepIssue::UnexpectedDirection);
        }
        match step.duration {
            None => issues.push(ScriptMovementStepIssue::MissingDuration),
            Some(duration) if duration > u16::from(u8::MAX) => {
                issues.push(ScriptMovementStepIssue::DurationOutOfByteRange { duration });
            }
            Some(0) if command == "step_sleep" => {
                issues.push(ScriptMovementStepIssue::ZeroSleepDuration);
            }
            Some(_) => {}
        }
        issues
    } else if SCRIPT_MOVEMENT_DIRECTION_COMMANDS.contains(&command) {
        let mut issues = match step.direction.as_deref() {
            Some(direction)
                if direction == SCRIPT_MOVEMENT_PLAYER_FACING_DIRECTION
                    || parse_script_direction(direction).is_ok() =>
            {
                Vec::new()
            }
            Some(direction) => vec![ScriptMovementStepIssue::UnknownDirection {
                direction: direction.to_string(),
            }],
            None => vec![ScriptMovementStepIssue::MissingDirection],
        };
        if step.duration.is_some() {
            issues.push(ScriptMovementStepIssue::UnexpectedDuration);
        }
        issues
    } else {
        vec![ScriptMovementStepIssue::UnsupportedCommand]
    }
}

pub fn apply_script_object_mutation(
    state: &mut GameState,
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    validate_script_object_command_source(command)?;
    match command.command.as_str() {
        "appear" => apply_visibility_command(state, session, command, false),
        "disappear" => apply_visibility_command(state, session, command, true),
        "moveobject" => apply_moveobject_command(session, command),
        "writeobjectxy" => apply_writeobjectxy_command(session, command),
        "turnobject" => apply_turnobject_command(session, command),
        "faceobject" => apply_faceobject_command(session, command),
        "faceplayer" => apply_faceplayer_command(session, command),
        "follow" => apply_follow_command(session, command),
        "follownotexact" => apply_follownotexact_command(session, command),
        "stopfollow" => apply_stopfollow_command(session, command),
        "showemote" => apply_showemote_command(state, session, command),
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
    validate_script_object_command_source(command)?;
    if let Some(source_script) = movement
        .source_script
        .as_deref()
        .filter(|source_script| !is_exact_script_label_token(source_script))
    {
        return Err(ScriptObjectCommandError::InvalidSourceScript {
            source_script: source_script.to_string(),
        });
    }
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
    if !is_exact_script_object_token(&expected) {
        return Err(ScriptObjectCommandError::InvalidMovement { movement: expected });
    }
    if !is_exact_script_object_token(&movement.label) {
        return Err(ScriptObjectCommandError::InvalidMovement {
            movement: movement.label.clone(),
        });
    }
    if movement.label != expected {
        return Err(ScriptObjectCommandError::WrongMovement {
            movement: movement.label.clone(),
            expected,
        });
    }
    validate_script_movement_steps_for_runtime(movement)?;

    let mut tile = object_tile(session, &object_id)?;
    let previous_tile = tile;
    let mut facing = object_facing(session, &object_id)?;
    let previous_facing = facing;
    let previous_hidden = if object_id == "PLAYER" {
        session.player_hidden
    } else {
        session
            .objects
            .iter()
            .position(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .is_some_and(|index| !session.object_struct_is_visible(index))
    };
    if object_id != "PLAYER" {
        let object_index = session
            .objects
            .iter()
            .position(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
                object_id: object_id.clone(),
            })?;
        if !session.object_has_loaded_struct(object_index) {
            let fixed_facing = session.fixed_facing_object_identifiers.contains(&object_id);
            let sliding = session.sliding_object_identifiers.contains(&object_id);
            return Ok(ScriptMovementOutcome {
                object_id,
                movement: movement.label.clone(),
                previous_tile,
                previous_facing: facing,
                previous_hidden,
                previous_follower: None,
                tile,
                facing,
                executed_steps: Vec::new(),
                effects: Vec::new(),
                fixed_facing,
                sliding,
                steps_applied: 0,
            });
        }
    }
    // LoadMovementDataPointer installs STEP_TYPE_RESET before scripted
    // movement begins. The flags2 BOULDER_MOVING bit is independent and is
    // deliberately retained, but an in-flight Strength step no longer owns
    // its duration or previous-coordinate collision.
    session.reset_object_step_type_for_script_movement(&object_id);
    let follow_queue_active = session.following.as_ref().is_some_and(|following| {
        session
            .object_struct_slot(&object_id)
            .is_some_and(|object_struct_slot| {
                following.leader_slot == Some(object_struct_slot)
                    && following.follower_slot.is_some()
            })
            && session.normal_follow_object_ids().is_some()
    });
    let previous_follower = session
        .following
        .as_ref()
        .filter(|_| follow_queue_active)
        .and_then(|following| following.follower_slot)
        .and_then(|slot| session.object_id_for_struct_slot(slot))
        .map(|follower_object_id| {
            Ok(ScriptMovementFollower {
                tile: object_tile(session, &follower_object_id)?,
                facing: object_facing(session, &follower_object_id)?,
                object_id: follower_object_id,
                queued_step: session.following_queued_step,
            })
        })
        .transpose()?;
    let mut follower_tile = previous_follower.as_ref().map(|follower| follower.tile);
    let mut follower_facing = previous_follower.as_ref().map(|follower| follower.facing);
    let mut queued_follower_step = previous_follower
        .as_ref()
        .and_then(|follower| {
            follower.queued_step.map(|step| {
                (
                    step.direction,
                    step.stride,
                    step.duration,
                    step.jump,
                    step.standing_frame,
                )
            })
        })
        .or_else(|| {
            previous_follower.as_ref().and_then(|follower| {
                let direction = if previous_tile.x > follower.tile.x {
                    Direction::Right
                } else if previous_tile.x < follower.tile.x {
                    Direction::Left
                } else if previous_tile.y > follower.tile.y {
                    Direction::Down
                } else if previous_tile.y < follower.tile.y {
                    Direction::Up
                } else {
                    return None;
                };
                Some((direction, 1_i16, 8_u8, false, false))
            })
        });
    let mut steps_applied = 0;
    let mut fixed_facing = session.fixed_facing_object_identifiers.contains(&object_id);
    let mut sliding = session.sliding_object_identifiers.contains(&object_id);
    let mut executed_steps = Vec::new();
    let mut effects = Vec::new();
    let mut last_movement_step = None;
    let mut player_path = Vec::new();

    for step in &movement.steps {
        match step.command.as_str() {
            "fix_facing" => {
                executed_steps.push(step.clone());
                fixed_facing = true;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "remove_fixed_facing" => {
                executed_steps.push(step.clone());
                fixed_facing = false;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "set_sliding" => {
                executed_steps.push(step.clone());
                sliding = true;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            "remove_sliding" => {
                executed_steps.push(step.clone());
                sliding = false;
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
            }
            command if movement_step_sleeps(command) => {
                executed_steps.push(step.clone());
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
                steps_applied += movement_step_tick_count(movement, step)?;
            }
            command if movement_step_ends_sequence(command) => {
                break;
            }
            command if movement_step_moves_object(command) => {
                executed_steps.push(step.clone());
                let direction = movement_step_direction(movement, step, facing)?;
                if !fixed_facing {
                    facing = if command == "turn_away" {
                        opposite_direction(direction)
                    } else {
                        direction
                    };
                }
                let stride = movement_step_stride(movement, step)?;
                if let (
                    Some(current_follower_tile),
                    Some((queued_direction, queued_stride, _, _, _)),
                ) = (follower_tile, queued_follower_step)
                {
                    follower_tile = Some(
                        checked_move_by_stride(
                            current_follower_tile,
                            queued_direction,
                            queued_stride,
                        )
                        .ok_or_else(|| {
                            ScriptObjectCommandError::MovementRuntimeTileOverflow {
                                movement: movement.label.clone(),
                                command: step.command.clone(),
                                index: step.index,
                                x: current_follower_tile.x,
                                y: current_follower_tile.y,
                            }
                        })?,
                    );
                    follower_facing = Some(queued_direction);
                }
                if follow_queue_active {
                    queued_follower_step = Some((
                        direction,
                        stride,
                        follower_step_visible_duration(step.command.as_str()),
                        step.command.contains("jump_step"),
                        step.command.contains("jump_step") || step.command.contains("slide_step"),
                    ));
                }
                let from_tile = tile;
                tile = checked_move_by_stride(tile, direction, stride).ok_or_else(|| {
                    ScriptObjectCommandError::MovementRuntimeTileOverflow {
                        movement: movement.label.clone(),
                        command: step.command.clone(),
                        index: step.index,
                        x: tile.x,
                        y: tile.y,
                    }
                })?;
                if object_id == "PLAYER" {
                    player_path.push(tile);
                }
                last_movement_step = Some((from_tile, tile));
                steps_applied += 1;
            }
            command if movement_step_turns_without_moving(command) => {
                executed_steps.push(step.clone());
                let direction = movement_step_direction(movement, step, facing)?;
                facing = match command {
                    "turn_away" => opposite_direction(direction),
                    _ => direction,
                };
                steps_applied += 1;
            }
            command if movement_step_records_effect(command) => {
                executed_steps.push(step.clone());
                if matches!(command, "teleport_from" | "teleport_to") {
                    // Both teleport routines reset OBJECT_STEP_FRAME before
                    // their counterclockwise spin. Sixteen action frames make
                    // a complete cycle and leave OBJECT_DIRECTION at OW_DOWN.
                    facing = Direction::Down;
                }
                effects.push(ScriptMovementEffect {
                    command: step.command.clone(),
                    index: step.index,
                });
                steps_applied += movement_step_tick_count(movement, step)?;
                if matches!(command, "step_wait_end" | "remove_object") {
                    break;
                }
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

    if tile != previous_tile {
        validate_follow_after_script_movement(session, &object_id, previous_tile)?;
    }

    if object_id == "PLAYER" {
        session
            .advance_object_struct_roster_along_player_path(&player_path)
            .map_err(|error| match error {
                crate::world::session::OverworldObjectCoordinateError::OutOfRange {
                    object_id,
                    x,
                    y,
                } => ScriptObjectCommandError::ObjectCoordinatesOutOfRange { object_id, x, y },
            })?;
    }

    set_object_tile(session, &object_id, tile)?;
    set_object_facing(session, &object_id, facing)?;
    if fixed_facing {
        session
            .fixed_facing_object_identifiers
            .insert(object_id.clone());
    } else {
        session.fixed_facing_object_identifiers.remove(&object_id);
    }
    if sliding {
        session.sliding_object_identifiers.insert(object_id.clone());
    } else {
        session.sliding_object_identifiers.remove(&object_id);
    }
    if let (Some(follower), Some(tile), Some(facing)) =
        (previous_follower.as_ref(), follower_tile, follower_facing)
    {
        set_object_tile(session, &follower.object_id, tile)?;
        set_object_facing(session, &follower.object_id, facing)?;
        session.following_queued_step =
            queued_follower_step.map(|(direction, stride, duration, jump, standing_frame)| {
                FollowQueuedStep {
                    direction,
                    stride,
                    duration,
                    jump,
                    standing_frame,
                }
            });
    } else if follow_queue_active {
        session.following_queued_step =
            queued_follower_step.map(|(direction, stride, duration, jump, standing_frame)| {
                FollowQueuedStep {
                    direction,
                    stride,
                    duration,
                    jump,
                    standing_frame,
                }
            });
    } else if let Some((from, to)) = last_movement_step {
        session.update_follow_after_entity_move(&object_id, from, to);
    }
    for effect in &effects {
        apply_movement_step_effect(session, &object_id, effect.command.as_str());
    }

    Ok(ScriptMovementOutcome {
        object_id,
        movement: movement.label.clone(),
        previous_tile,
        previous_facing,
        previous_hidden,
        previous_follower,
        tile,
        facing,
        executed_steps,
        effects,
        fixed_facing,
        sliding,
        steps_applied,
    })
}

fn validate_script_movement_steps_for_runtime(
    movement: &ScriptMovement,
) -> Result<(), ScriptObjectCommandError> {
    for step in &movement.steps {
        if let Some(issue) = script_movement_step_issues(step).into_iter().next() {
            return Err(script_movement_step_issue_error(movement, step, issue));
        }
    }
    Ok(())
}

fn script_movement_step_issue_error(
    movement: &ScriptMovement,
    step: &ScriptMovementStep,
    issue: ScriptMovementStepIssue,
) -> ScriptObjectCommandError {
    match issue {
        ScriptMovementStepIssue::UnexpectedDirection => {
            ScriptObjectCommandError::MovementUnexpectedDirection {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
            }
        }
        ScriptMovementStepIssue::MissingDirection => {
            ScriptObjectCommandError::MovementMissingDirection {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
            }
        }
        ScriptMovementStepIssue::MissingDuration => {
            ScriptObjectCommandError::MovementMissingDuration {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
            }
        }
        ScriptMovementStepIssue::UnexpectedDuration => {
            ScriptObjectCommandError::MovementUnexpectedDuration {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
            }
        }
        ScriptMovementStepIssue::DurationOutOfByteRange { duration } => {
            ScriptObjectCommandError::MovementDurationOutOfByteRange {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
                duration,
            }
        }
        ScriptMovementStepIssue::ZeroSleepDuration => {
            ScriptObjectCommandError::MovementZeroSleepDuration {
                movement: movement.label.clone(),
                index: step.index,
            }
        }
        ScriptMovementStepIssue::UnknownDirection { direction } => {
            ScriptObjectCommandError::MovementUnknownDirection {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
                direction,
            }
        }
        ScriptMovementStepIssue::UnsupportedCommand => {
            ScriptObjectCommandError::UnsupportedMovementCommand {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
            }
        }
    }
}

fn apply_movement_step_effect(session: &mut OverworldSession, object_id: &str, command: &str) {
    match command {
        "remove_object" => {
            session.delete_loaded_object_struct_for_movement_remove(object_id);
        }
        "hide_object" => set_movement_object_hidden(session, object_id, true),
        "show_object" => set_movement_object_hidden(session, object_id, false),
        _ => {}
    }
}

fn set_movement_object_hidden(session: &mut OverworldSession, object_id: &str, hidden: bool) {
    if object_id == "PLAYER" {
        session.player_hidden = hidden;
    } else {
        session.set_loaded_object_struct_invisible(object_id, hidden);
    }
}

fn apply_visibility_command(
    state: &mut GameState,
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
    hidden: bool,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    if object_id == "PLAYER" {
        if hidden {
            session.stop_follow_for_deleted_object_struct("PLAYER");
        }
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
    session.clear_loaded_roster_visibility_override(&object_id);

    if event_flag == "-1" {
        if hidden {
            session.hidden_object_identifiers.insert(object_id.clone());
            session.shown_object_identifiers.remove(&object_id);
        } else {
            session.hidden_object_identifiers.remove(&object_id);
            session.shown_object_identifiers.insert(object_id.clone());
        }
    } else {
        validate_toggle_flag(&object_id, &event_flag)?;
        state
            .flags
            .set_event_flag(&event_flag, hidden)
            .map_err(|error| ScriptObjectCommandError::EventFlag { error })?;
        session.sync_event_flag_memory(&state.flags);
        session.clear_loaded_roster_visibility_override(&object_id);
        if hidden {
            session.hidden_object_identifiers.insert(object_id.clone());
            session.shown_object_identifiers.remove(&object_id);
        } else {
            session.hidden_object_identifiers.remove(&object_id);
            session.shown_object_identifiers.insert(object_id.clone());
        }
    }

    if hidden {
        session.delete_loaded_object_struct_for_disappear(&object_id);
    } else {
        session
            .copy_object_struct_for_appear(&object_id)
            .map_err(|error| match error {
                crate::world::session::OverworldObjectCoordinateError::OutOfRange {
                    x, y, ..
                } => ScriptObjectCommandError::ObjectCoordinatesOutOfRange {
                    object_id: object_id.clone(),
                    x,
                    y,
                },
            })?;
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
    if !object_is_visible(session, &object_id)?
        || !object_has_facings(session, &object_id)?
        || session.fixed_facing_object_identifiers.contains(&object_id)
    {
        return Ok(facing_noop_outcome(command, object_id));
    }
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
    if !object_is_visible(session, &object_id)?
        || !object_is_visible(session, &target_object_id)?
        || !object_has_facings(session, &object_id)?
        || session.fixed_facing_object_identifiers.contains(&object_id)
    {
        return Ok(facing_noop_outcome(command, object_id));
    }
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

fn apply_faceplayer_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let Some(object_id) = session.last_talked_object_identifier.clone() else {
        return Ok(facing_noop_outcome(command, "LAST_TALKED".to_string()));
    };
    if !object_is_visible(session, &object_id)?
        || session.player_hidden
        || !object_has_facings(session, &object_id)?
        || session.fixed_facing_object_identifiers.contains(&object_id)
    {
        return Ok(facing_noop_outcome(command, object_id));
    }
    let from = object_tile(session, &object_id)?;
    let target = object_tile(session, "PLAYER")?;
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

fn facing_noop_outcome(
    command: &ScriptObjectCommand,
    object_id: String,
) -> ScriptObjectMutationOutcome {
    ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: None,
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    }
}

fn apply_follow_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let leader_object_id = required_object_id(session, command)?;
    let follower_object_id = required_target_object_id(session, command)?;
    validate_object_reference(session, &leader_object_id)?;
    validate_object_reference(session, &follower_object_id)?;
    let Some(leader_slot) = session.object_struct_slot(&leader_object_id) else {
        return Ok(ScriptObjectMutationOutcome {
            command: command.command.clone(),
            object_id: leader_object_id,
            event_flag: None,
            previous_x: None,
            previous_y: None,
            x: None,
            y: None,
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
    };
    let follower_slot = session.object_struct_slot(&follower_object_id);
    session.reset_normal_follower_movement();
    session.following = Some(OverworldFollowState {
        leader_slot: Some(leader_slot),
        follower_slot,
    });
    if follower_slot.is_none() {
        session.following_queued_step = None;
        return Ok(ScriptObjectMutationOutcome {
            command: command.command.clone(),
            object_id: leader_object_id,
            event_flag: None,
            previous_x: None,
            previous_y: None,
            x: None,
            y: None,
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
    }
    session.set_normal_follower_movement(
        follower_slot.expect("loaded follower slot checked before queue initialization"),
    );
    let leader_tile = object_tile(session, &leader_object_id)?;
    let follower_tile = object_tile(session, &follower_object_id)?;
    let initial_direction = direction_toward(follower_tile, leader_tile);
    // Ensure an NPC follower has an explicit pre-step runtime coordinate.
    // Bevy's visible object-diff interpolator needs this origin to retain the
    // first eight-frame follower stride after `follow`; without it, the first
    // authoritative mutation appeared only in the post-step map and snapped.
    if follower_object_id != "PLAYER" {
        set_object_tile(session, &follower_object_id, follower_tile)?;
    }
    session.following_queued_step = initial_direction.map(|direction| FollowQueuedStep {
        direction,
        stride: SCRIPT_MOVEMENT_EVENT_TILE_STRIDE,
        duration: 8,
        jump: false,
        standing_frame: false,
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
    session.reset_normal_follower_movement();
    session.following = None;
    session.following_queued_step = None;
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

fn apply_follownotexact_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let leader_object_id = required_object_id(session, command)?;
    let follower_object_id = required_target_object_id(session, command)?;
    validate_object_reference(session, &leader_object_id)?;
    validate_object_reference(session, &follower_object_id)?;
    if !object_is_visible(session, &leader_object_id)?
        || !object_is_visible(session, &follower_object_id)?
    {
        return Ok(ScriptObjectMutationOutcome {
            command: command.command.clone(),
            object_id: follower_object_id,
            event_flag: None,
            previous_x: None,
            previous_y: None,
            x: None,
            y: None,
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
    }
    let leader_slot = session
        .object_struct_slot(&leader_object_id)
        .expect("a visible script object has an allocated object struct");
    let leader_tile = object_tile(session, &leader_object_id)?;
    let previous_follower_tile = object_tile(session, &follower_object_id)?;
    let follower_tile = direction_toward(previous_follower_tile, leader_tile)
        .and_then(|direction| {
            checked_move_by_stride(
                previous_follower_tile,
                direction,
                SCRIPT_MOVEMENT_EVENT_TILE_STRIDE,
            )
        })
        .unwrap_or(previous_follower_tile);
    set_object_tile(session, &follower_object_id, follower_tile)?;
    session
        .following_not_exact
        .insert(follower_object_id.clone(), leader_slot);
    session
        .normal_following_object_identifiers
        .remove(&follower_object_id);
    session.object_step_durations.remove(&follower_object_id);
    session
        .strength_moving_object_identifiers
        .remove(&follower_object_id);
    session
        .object_pending_random_wait
        .remove(&follower_object_id);
    session
        .initialized_fixed_spin_objects
        .remove(&follower_object_id);
    session
        .object_last_runtime_tiles
        .insert(follower_object_id.clone(), follower_tile);
    session
        .object_last_tiles_occupied_until_frame
        .remove(&follower_object_id);

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id: follower_object_id,
        event_flag: None,
        previous_x: u16::try_from(previous_follower_tile.x).ok(),
        previous_y: u16::try_from(previous_follower_tile.y).ok(),
        x: u16::try_from(follower_tile.x).ok(),
        y: u16::try_from(follower_tile.y).ok(),
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn apply_showemote_command(
    state: &mut GameState,
    session: &OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    validate_object_reference(session, &object_id)?;
    let emote = command
        .emote
        .clone()
        .ok_or_else(|| ScriptObjectCommandError::MissingEmote {
            command: command.command.clone(),
        })?;
    let duration = command
        .duration
        .ok_or_else(|| ScriptObjectCommandError::MissingEmote {
            command: command.command.clone(),
        })?;
    let duration_byte = u8::try_from(duration)
        .map_err(|_| ScriptObjectCommandError::EmoteDurationOutOfByteRange { duration })?;
    state
        .script_runtime
        .pending_emotes
        .push(ScriptRuntimeEmote {
            emote,
            object: object_id.clone(),
            duration,
            frames: crate::timing::wrapping_byte_counter_frames(duration_byte, 2),
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });

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
    let object_index = session
        .objects
        .iter()
        .position(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.clone(),
        })?;
    require_moveobject_runtime_tile(session, &object_id, x, y)?;
    let live_tile = session
        .object_runtime_tile_checked(object_index, &session.objects[object_index])
        .map_err(|error| match error {
            crate::world::session::OverworldObjectCoordinateError::OutOfRange { x, y, .. } => {
                ScriptObjectCommandError::ObjectCoordinatesOutOfRange {
                    object_id: object_id.clone(),
                    x,
                    y,
                }
            }
        })?;
    let previous_x = session.objects[object_index].x;
    let previous_y = session.objects[object_index].y;
    if session.object_has_loaded_struct(object_index) {
        session
            .object_runtime_tiles
            .insert(object_id.clone(), live_tile);
    } else {
        session.object_runtime_tiles.remove(&object_id);
    }
    session.objects[object_index].x = x;
    session.objects[object_index].y = y;

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

/// Applies Crystal's `WriteObjectXY` primitive for both interpreted scripts
/// and source-certified typed consumers.
pub fn apply_writeobjectxy_command(
    session: &mut OverworldSession,
    command: &ScriptObjectCommand,
) -> Result<ScriptObjectMutationOutcome, ScriptObjectCommandError> {
    let object_id = required_object_id(session, command)?;
    let (object_index, previous_coordinates) = if object_id == "PLAYER" {
        if session.player_hidden {
            return Ok(writeobjectxy_noop_outcome(command, object_id));
        }
        (None, None)
    } else {
        let object_index = session
            .objects
            .iter()
            .position(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
                object_id: object_id.clone(),
            })?;
        if !session.object_has_loaded_struct(object_index) {
            return Ok(writeobjectxy_noop_outcome(command, object_id));
        }
        (
            Some(object_index),
            Some((
                session.objects[object_index].x,
                session.objects[object_index].y,
            )),
        )
    };
    let tile = object_tile(session, &object_id)?;
    let raw_tile = runtime_tile_to_raw_event_tile(tile).ok_or_else(|| {
        ScriptObjectCommandError::ObjectPositionUnsavable {
            object_id: object_id.clone(),
            x: tile.x,
            y: tile.y,
        }
    })?;
    let x = u16::try_from(raw_tile.x).map_err(|_| {
        ScriptObjectCommandError::ObjectPositionUnsavable {
            object_id: object_id.clone(),
            x: tile.x,
            y: tile.y,
        }
    })?;
    let y = u16::try_from(raw_tile.y).map_err(|_| {
        ScriptObjectCommandError::ObjectPositionUnsavable {
            object_id: object_id.clone(),
            x: tile.x,
            y: tile.y,
        }
    })?;

    if let Some(object_index) = object_index {
        session.objects[object_index].x = x;
        session.objects[object_index].y = y;
    }

    Ok(ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: None,
        previous_x: previous_coordinates.map(|(x, _)| x),
        previous_y: previous_coordinates.map(|(_, y)| y),
        x: Some(x),
        y: Some(y),
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    })
}

fn writeobjectxy_noop_outcome(
    command: &ScriptObjectCommand,
    object_id: String,
) -> ScriptObjectMutationOutcome {
    ScriptObjectMutationOutcome {
        command: command.command.clone(),
        object_id,
        event_flag: None,
        previous_x: None,
        previous_y: None,
        x: None,
        y: None,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    }
}

fn moveobject_raw_coordinates_fit_runtime_tile(command: &ScriptObjectCommand) -> bool {
    let (Some(x), Some(y)) = (command.x, command.y) else {
        return false;
    };
    raw_event_tile_to_runtime_tile_checked(x, y).is_some()
}

fn require_moveobject_runtime_tile(
    session: &OverworldSession,
    object_id: &str,
    x: u16,
    y: u16,
) -> Result<TilePosition, ScriptObjectCommandError> {
    let tile = raw_event_tile_to_runtime_tile_checked(x, y).ok_or_else(|| {
        ScriptObjectCommandError::MoveCoordinatesOutOfRange {
            object_id: object_id.to_string(),
            x,
            y,
        }
    })?;
    let (width, height) = session.map.checked_tile_bounds().ok_or_else(|| {
        ScriptObjectCommandError::MapBoundsOverflow {
            map_name: session.map.name.clone(),
        }
    })?;
    if !runtime_tile_within_bounds(tile, width, height) {
        return Err(ScriptObjectCommandError::MoveCoordinatesOutOfMap {
            object_id: object_id.to_string(),
            map_name: session.map.name.clone(),
            x,
            y,
            width,
            height,
        });
    }
    Ok(tile)
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

fn validate_follow_after_script_movement(
    session: &OverworldSession,
    moved_object_id: &str,
    _previous_tile: TilePosition,
) -> Result<(), ScriptObjectCommandError> {
    let Some(following) = session.following.as_ref() else {
        return Ok(());
    };
    if session.object_struct_slot(moved_object_id) != following.leader_slot {
        return Ok(());
    }
    let Some(follower_slot) = following.follower_slot else {
        return Ok(());
    };
    let Some(follower_object_id) = session.object_id_for_struct_slot(follower_slot) else {
        return Ok(());
    };
    if !session
        .normal_following_object_identifiers
        .contains(&follower_object_id)
    {
        return Ok(());
    }
    if follower_object_id == "PLAYER" {
        return Ok(());
    }
    session
        .objects
        .iter()
        .any(|object| object.object_identifier.as_deref() == Some(follower_object_id.as_str()))
        .then_some(())
        .ok_or_else(|| ScriptObjectCommandError::FollowObjectMissing {
            object_id: follower_object_id,
        })
}

fn movement_step_direction(
    movement: &ScriptMovement,
    step: &ScriptMovementStep,
    current_facing: Direction,
) -> Result<Direction, ScriptObjectCommandError> {
    step.direction
        .as_deref()
        .ok_or_else(|| ScriptObjectCommandError::MovementMissingDirection {
            movement: movement.label.clone(),
            command: step.command.clone(),
            index: step.index,
        })
        .and_then(|direction| {
            if direction == SCRIPT_MOVEMENT_PLAYER_FACING_DIRECTION {
                Ok(current_facing)
            } else {
                parse_script_direction(direction)
            }
        })
}

fn movement_step_moves_object(command: &str) -> bool {
    matches!(
        command,
        "step"
            | "slow_step"
            | "big_step"
            | "turn_step"
            | "jump_step"
            | "fast_jump_step"
            | "slow_jump_step"
            | "slide_step"
            | "fast_slide_step"
            | "slow_slide_step"
            | "turn_away"
            | "turn_in"
            | "turn_waterfall"
    )
}

fn movement_step_stride(
    movement: &ScriptMovement,
    step: &ScriptMovementStep,
) -> Result<i16, ScriptObjectCommandError> {
    script_movement_step_runtime_stride(&step.command).ok_or_else(|| {
        ScriptObjectCommandError::MovementMissingRuntimeStride {
            movement: movement.label.clone(),
            command: step.command.clone(),
            index: step.index,
        }
    })
}

pub fn script_movement_step_runtime_stride(command: &str) -> Option<i16> {
    if !movement_step_moves_object(command) {
        return None;
    }
    match command {
        "jump_step" | "fast_jump_step" | "slow_jump_step" => Some(SCRIPT_MOVEMENT_JUMP_TILE_STRIDE),
        _ => Some(SCRIPT_MOVEMENT_EVENT_TILE_STRIDE),
    }
}

fn follower_step_visible_duration(command: &str) -> u8 {
    let base = if command.starts_with("slow_") {
        16
    } else if command.starts_with("fast_") || command == "big_step" {
        4
    } else {
        8
    };
    if command.contains("jump_step") {
        base * 2
    } else {
        base
    }
}

fn movement_step_tick_count(
    movement: &ScriptMovement,
    step: &ScriptMovementStep,
) -> Result<usize, ScriptObjectCommandError> {
    if let Some(duration) = exact_stationary_effect_duration(step.command.as_str()) {
        Ok(duration)
    } else if SCRIPT_MOVEMENT_REQUIRED_DURATION_COMMANDS.contains(&step.command.as_str()) {
        let duration =
            step.duration
                .ok_or_else(|| ScriptObjectCommandError::MovementMissingDuration {
                    movement: movement.label.clone(),
                    command: step.command.clone(),
                    index: step.index,
                })?;
        let duration = u8::try_from(duration).map_err(|_| {
            ScriptObjectCommandError::MovementDurationOutOfByteRange {
                movement: movement.label.clone(),
                command: step.command.clone(),
                index: step.index,
                duration,
            }
        })?;
        let counter = if step.command == "step_shake" {
            duration & 0x3f
        } else {
            duration
        };
        Ok(usize::from(wrapping_byte_counter_ticks(counter)))
    } else {
        Ok(step.duration.map(usize::from).unwrap_or(1))
    }
}

fn movement_step_sleeps(command: &str) -> bool {
    command == "step_sleep"
}

fn exact_stationary_effect_duration(command: &str) -> Option<usize> {
    match command {
        // map_objects.asm: TeleportFrom runs a 16-frame spin followed by a
        // 16-frame spin-rise. TeleportTo has a one-frame wait initializer,
        // then 16 wait, 16 descent, and 16 final-spin frames.
        "teleport_from" => Some(32),
        "teleport_to" => Some(49),
        "skyfall" => Some(32),
        "skyfall_top" => Some(16),
        "tree_shake" => Some(24),
        _ => None,
    }
}

fn movement_step_turns_without_moving(command: &str) -> bool {
    matches!(command, "turn_head" | "step_bump")
}

fn movement_step_ends_sequence(command: &str) -> bool {
    matches!(command, "step_end" | "step_stop" | "step_loop")
}

fn movement_step_records_effect(command: &str) -> bool {
    matches!(
        command,
        "step_wait_end"
            | "teleport_from"
            | "teleport_to"
            | "skyfall"
            | "skyfall_top"
            | "step_dig"
            | "fish_got_bite"
            | "fish_cast_rod"
            | "hide_emote"
            | "show_emote"
            | "step_shake"
            | "tree_shake"
            | "rock_smash"
            | "return_dig"
            | "remove_object"
            | "hide_object"
            | "show_object"
    )
}

#[cfg(test)]
fn movement_step_is_executable(command: &str) -> bool {
    movement_step_sleeps(command)
        || movement_step_ends_sequence(command)
        || movement_step_moves_object(command)
        || movement_step_turns_without_moving(command)
        || movement_step_records_effect(command)
        || matches!(
            command,
            "fix_facing" | "remove_fixed_facing" | "set_sliding" | "remove_sliding"
        )
}

fn opposite_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
}

fn object_tile(
    session: &OverworldSession,
    object_id: &str,
) -> Result<TilePosition, ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        return Ok(session.player.tile);
    }
    if let Some(tile) = session.object_runtime_tiles.get(object_id) {
        return Ok(*tile);
    }
    let (index, object) = session
        .objects
        .iter()
        .enumerate()
        .find(|(_, object)| object.object_identifier.as_deref() == Some(object_id))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        })?;
    session
        .object_runtime_tile_checked(index, object)
        .map_err(|_| ScriptObjectCommandError::ObjectCoordinatesOutOfRange {
            object_id: object_id.to_string(),
            x: object.x,
            y: object.y,
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
    if object_id != "PLAYER"
        && !session
            .objects
            .iter()
            .any(|object| object.object_identifier.as_deref() == Some(object_id))
    {
        return Err(ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        });
    }
    if object_id == "PLAYER" {
        let (width, height) = session.map.checked_tile_bounds().ok_or_else(|| {
            ScriptObjectCommandError::MapBoundsOverflow {
                map_name: session.map.name.clone(),
            }
        })?;
        if !runtime_tile_within_bounds(tile, width, height) {
            return Err(ScriptObjectCommandError::ObjectPositionOutOfMap {
                object_id: object_id.to_string(),
                map_name: session.map.name.clone(),
                x: tile.x,
                y: tile.y,
                width,
                height,
            });
        }
        session.player.tile = tile;
        return Ok(());
    }
    session
        .object_runtime_tiles
        .insert(object_id.to_string(), tile);
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

fn object_is_visible(
    session: &OverworldSession,
    object_id: &str,
) -> Result<bool, ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        return Ok(!session.player_hidden);
    }
    let (index, _) = session
        .objects
        .iter()
        .enumerate()
        .find(|(_, object)| object.object_identifier.as_deref() == Some(object_id))
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        })?;
    Ok(session.object_has_loaded_struct(index))
}

fn object_has_facings(
    session: &OverworldSession,
    object_id: &str,
) -> Result<bool, ScriptObjectCommandError> {
    if object_id == "PLAYER" {
        return Ok(true);
    }
    session
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some(object_id))
        .map(|object| object.sprite_has_facings)
        .ok_or_else(|| ScriptObjectCommandError::UnknownObject {
            object_id: object_id.to_string(),
        })
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
    if object_id != "LAST_TALKED"
        && object_id != "PLAYER"
        && !is_exact_script_object_token(object_id)
    {
        return Err(ScriptObjectCommandError::InvalidObjectId {
            object_id: object_id.to_string(),
        });
    }
    if object_id == "LAST_TALKED" {
        let resolved = session
            .last_talked_object_identifier
            .clone()
            .ok_or(ScriptObjectCommandError::MissingLastTalkedObject)?;
        if is_exact_script_object_token(&resolved) {
            return Ok(resolved);
        }
        return Err(ScriptObjectCommandError::InvalidObjectId {
            object_id: resolved,
        });
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

fn runtime_tile_within_bounds(tile: TilePosition, width: u16, height: u16) -> bool {
    tile.x >= 0
        && tile.y >= 0
        && i32::from(tile.x) < i32::from(width)
        && i32::from(tile.y) < i32::from(height)
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
            sprite_has_facings: true,
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
                    width: 32,
                    height: 32,
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
                vec![0; 1024],
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

    fn apply_test_movement(
        session: &mut OverworldSession,
        object_id: &str,
        movement_label: &str,
        directions: &[&str],
    ) {
        let mut command = command("applymovement", object_id);
        command.movement = Some(movement_label.to_string());
        let mut steps = directions
            .iter()
            .enumerate()
            .map(|(index, direction)| ScriptMovementStep {
                command: "step".to_string(),
                direction: Some((*direction).to_string()),
                duration: None,
                index,
            })
            .collect::<Vec<_>>();
        steps.push(ScriptMovementStep {
            command: "step_end".to_string(),
            direction: None,
            duration: None,
            index: steps.len(),
        });
        let movement = ScriptMovement {
            label: movement_label.to_string(),
            source_script: None,
            steps,
        };
        apply_script_movement(session, &command, &movement).expect("test movement applies");
    }

    #[test]
    fn applymovement_resets_an_active_strength_step_but_retains_the_push_flag() {
        let mut boulder = object("BOULDER", "-1", 4, 4);
        boulder.spritemovedata = "SPRITEMOVEDATA_STRENGTH_BOULDER".to_string();
        let mut session = session(vec![boulder]);
        session
            .object_runtime_tiles
            .insert("BOULDER".to_string(), TilePosition::new(5, 4));
        session
            .object_last_runtime_tiles
            .insert("BOULDER".to_string(), TilePosition::new(4, 4));
        session
            .object_last_tiles_occupied_until_frame
            .insert("BOULDER".to_string(), session.frame + 6);
        session
            .object_step_durations
            .insert("BOULDER".to_string(), 6);
        session
            .strength_moving_object_identifiers
            .insert("BOULDER".to_string());
        session
            .strength_boulder_push_directions
            .insert("BOULDER".to_string(), Direction::Right);

        apply_test_movement(&mut session, "BOULDER", "MoveBoulder", &["RIGHT"]);

        assert_eq!(
            session.object_runtime_tiles.get("BOULDER"),
            Some(&TilePosition::new(6, 4))
        );
        assert_eq!(
            session.object_last_runtime_tiles.get("BOULDER"),
            Some(&TilePosition::new(5, 4)),
            "STEP_TYPE_RESET copies the live coordinate before scripted movement"
        );
        assert!(!session.object_step_durations.contains_key("BOULDER"));
        assert!(
            !session
                .strength_moving_object_identifiers
                .contains("BOULDER")
        );
        assert!(
            !session
                .object_last_tiles_occupied_until_frame
                .contains_key("BOULDER")
        );
        assert_eq!(
            session.strength_boulder_push_directions.get("BOULDER"),
            Some(&Direction::Right),
            "LoadMovementDataPointer does not clear the flags2 BOULDER_MOVING bit"
        );
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
    fn exported_object_command_sets_are_exact() {
        assert!(SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&"appear"));
        assert!(SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&"disappear"));
        assert!(SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&"moveobject"));
        assert!(SCRIPT_OBJECT_WRITE_COORDINATE_COMMANDS.contains(&"writeobjectxy"));
        assert!(SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&"turnobject"));
        assert!(SCRIPT_OBJECT_TARGET_COMMANDS.contains(&"faceobject"));
        assert!(SCRIPT_OBJECT_TARGET_COMMANDS.contains(&"follow"));
        assert!(SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS.contains(&"applymovement"));
        assert!(SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS.contains(&"applymovementlasttalked"));
        assert!(SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&"applymovement"));
        assert!(SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&"applymovementlasttalked"));
        assert!(SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&"faceplayer"));
        assert!(SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&"stopfollow"));
        assert!(SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&"showemote"));
        assert!(is_known_script_object_command("moveobject"));
        assert!(is_known_script_object_command("writeobjectxy"));
        assert!(!is_known_script_object_command("MoveObject"));
        assert!(!is_known_script_object_command("hideobject"));
        assert_eq!(
            SCRIPT_MOVEMENT_DIRECTION_COMMANDS,
            &[
                "step",
                "slow_step",
                "big_step",
                "turn_step",
                "jump_step",
                "fast_jump_step",
                "slow_jump_step",
                "slide_step",
                "fast_slide_step",
                "slow_slide_step",
                "step_bump",
                "turn_head",
                "turn_away",
                "turn_in",
                "turn_waterfall"
            ]
        );
        assert_eq!(
            SCRIPT_MOVEMENT_REQUIRED_DURATION_COMMANDS,
            &[
                "step_sleep",
                "step_wait_end",
                "step_dig",
                "step_shake",
                "rock_smash",
                "return_dig"
            ]
        );
        assert_eq!(
            SCRIPT_MOVEMENT_NO_ARG_COMMANDS,
            &[
                "step_end",
                "step_loop",
                "step_stop",
                "fix_facing",
                "remove_fixed_facing",
                "set_sliding",
                "remove_sliding",
                "teleport_from",
                "teleport_to",
                "skyfall",
                "skyfall_top",
                "fish_got_bite",
                "fish_cast_rod",
                "hide_emote",
                "show_emote",
                "tree_shake",
                "remove_object",
                "hide_object",
                "show_object"
            ]
        );
        assert!(is_known_script_movement_command("turn_head"));
        assert!(is_known_script_movement_command("turn_waterfall"));
        assert!(is_known_script_movement_command("fast_slide_step"));
        assert!(is_known_script_movement_command("step_dig"));
        assert!(is_known_script_movement_command("rock_smash"));
        assert!(!is_known_script_movement_command("fast_step"));
        assert!(!is_known_script_movement_command("step_sleep_8"));
        assert!(is_known_script_movement_command("hide_object"));
        assert!(!is_known_script_movement_command("step_sleep_17"));
        assert!(!is_known_script_movement_command("spin_forever"));
    }

    #[test]
    fn writeobjectxy_copies_last_talked_runtime_coordinates_to_map_object_memory() {
        let mut state = GameState::default();
        let mut session = session(vec![object("ROUTE29_TRAINER", "-1", 2, 3)]);
        session.last_talked_object_identifier = Some("ROUTE29_TRAINER".to_string());
        session
            .object_runtime_tiles
            .insert("ROUTE29_TRAINER".to_string(), TilePosition::new(7, 9));
        let command = command("writeobjectxy", "LAST_TALKED");

        let outcome = apply_script_object_mutation(&mut state, &mut session, &command)
            .expect("writeobjectxy applies");

        assert_eq!(outcome.object_id, "ROUTE29_TRAINER");
        assert_eq!((outcome.previous_x, outcome.previous_y), (Some(2), Some(3)));
        assert_eq!((outcome.x, outcome.y), (Some(7), Some(9)));
        assert_eq!((session.objects[0].x, session.objects[0].y), (7, 9));
        assert_eq!(
            session.object_runtime_tiles.get("ROUTE29_TRAINER"),
            Some(&TilePosition::new(7, 9))
        );
    }

    #[test]
    fn writeobjectxy_returns_without_reading_coordinates_when_object_struct_is_unloaded() {
        let mut state = GameState::default();
        let mut session = session(vec![object("HIDDEN_TRAINER", "-1", 2, 3)]);
        session.delete_loaded_object_struct("HIDDEN_TRAINER");
        session
            .object_runtime_tiles
            .insert("HIDDEN_TRAINER".to_string(), TilePosition::new(-1, -1));
        let command = command("writeobjectxy", "HIDDEN_TRAINER");

        let outcome = apply_script_object_mutation(&mut state, &mut session, &command)
            .expect("unloaded writeobjectxy returns like CheckObjectVisibility carry");

        assert_eq!((outcome.previous_x, outcome.previous_y), (None, None));
        assert_eq!((outcome.x, outcome.y), (None, None));
        assert_eq!((session.objects[0].x, session.objects[0].y), (2, 3));
    }

    #[test]
    fn script_movement_stride_matches_runtime_player_stride() {
        assert_eq!(SCRIPT_MOVEMENT_EVENT_TILE_STRIDE, 1);
        assert_eq!(
            crate::world::movement::StepOptions::default().stride_tiles,
            SCRIPT_MOVEMENT_EVENT_TILE_STRIDE
        );
        assert_eq!(script_movement_step_runtime_stride("step"), Some(1));
        assert_eq!(script_movement_step_runtime_stride("jump_step"), Some(2));
        assert_eq!(script_movement_step_runtime_stride("turn_head"), None);
    }

    #[test]
    fn scripted_player_path_loads_each_newly_exposed_object_struct_edge() {
        let mut session = session(vec![object("ELM", "-1", 3, 4)]);
        session
            .advance_object_struct_roster_along_player_path(&[TilePosition::new(4, 11)])
            .expect("fixture object coordinates");
        assert!(!session.object_has_loaded_struct(0));
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("WalkUpToElm".to_string());
        let movement = ScriptMovement {
            label: "WalkUpToElm".to_string(),
            source_script: None,
            steps: (0..7)
                .map(|index| ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index,
                })
                .collect(),
        };

        apply_script_movement(&mut session, &command, &movement)
            .expect("scripted player path applies");

        assert_eq!(session.player.tile, TilePosition::new(4, 4));
        assert!(session.object_has_loaded_struct(0));
    }

    #[test]
    fn applymovement_returns_without_running_for_unloaded_map_object() {
        let mut session = session(vec![object("OFFSCREEN_NPC", "-1", 20, 0)]);
        assert!(!session.object_has_loaded_struct(0));
        let mut command = command("applymovement", "OFFSCREEN_NPC");
        command.movement = Some("OffscreenMovement".to_string());
        let movement = ScriptMovement {
            label: "OffscreenMovement".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "step".to_string(),
                direction: Some("LEFT".to_string()),
                duration: None,
                index: 0,
            }],
        };

        let outcome = apply_script_movement(&mut session, &command, &movement)
            .expect("GetMovementData carry returns without starting movement");

        assert_eq!(outcome.previous_tile, TilePosition::new(20, 0));
        assert_eq!(outcome.tile, TilePosition::new(20, 0));
        assert_eq!(outcome.steps_applied, 0);
        assert!(outcome.executed_steps.is_empty());
    }

    #[test]
    fn every_verified_script_movement_command_has_runtime_execution_bucket() {
        let missing = SCRIPT_MOVEMENT_COMMANDS
            .iter()
            .copied()
            .filter(|command| !movement_step_is_executable(command))
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "verified movement commands without Rust runtime execution buckets: {missing:?}"
        );
    }

    #[test]
    fn script_object_serialized_variants_reject_unknown_fallback_fields() {
        let command_error = serde_json::from_value::<ScriptObjectCommandError>(serde_json::json!({
            "UnknownObject": {
                "object_id": "LYRA",
                "fallback_object_id": "PLAYER"
            }
        }))
        .expect_err("object errors must not accept fallback object ids");
        assert!(
            command_error
                .to_string()
                .contains("unknown field `fallback_object_id`"),
            "{command_error}"
        );

        let movement_issue_error =
            serde_json::from_value::<ScriptMovementStepIssue>(serde_json::json!({
                "unknown_direction": {
                    "direction": "north",
                    "normalized_direction": "UP"
                }
            }))
            .expect_err("movement issues must not accept normalized direction aliases");
        assert!(
            movement_issue_error
                .to_string()
                .contains("unknown field `normalized_direction`"),
            "{movement_issue_error}"
        );
    }

    #[test]
    fn movement_step_issues_require_exact_payload_shapes() {
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }),
            vec![ScriptMovementStepIssue::MissingDirection]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step".to_string(),
                direction: Some("north".to_string()),
                duration: None,
                index: 1,
            }),
            vec![ScriptMovementStepIssue::UnknownDirection {
                direction: "north".to_string()
            }]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step_end".to_string(),
                direction: Some("DOWN".to_string()),
                duration: None,
                index: 2,
            }),
            vec![ScriptMovementStepIssue::UnexpectedDirection]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "spin_forever".to_string(),
                direction: None,
                duration: None,
                index: 3,
            }),
            vec![ScriptMovementStepIssue::UnsupportedCommand]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step_dig".to_string(),
                direction: None,
                duration: None,
                index: 5,
            }),
            vec![ScriptMovementStepIssue::MissingDuration]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step_sleep".to_string(),
                direction: None,
                duration: None,
                index: 6,
            }),
            vec![ScriptMovementStepIssue::MissingDuration]
        );
        for non_opcode in ["fast_step", "step_sleep_1", "step_sleep_8", "step_sleep_16"] {
            assert_eq!(
                script_movement_step_issues(&ScriptMovementStep {
                    command: non_opcode.to_string(),
                    direction: None,
                    duration: None,
                    index: 7,
                }),
                vec![ScriptMovementStepIssue::UnsupportedCommand]
            );
        }
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "rock_smash".to_string(),
                direction: None,
                duration: Some(10),
                index: 6,
            }),
            Vec::<ScriptMovementStepIssue>::new()
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "turn_head".to_string(),
                direction: Some("LEFT".to_string()),
                duration: None,
                index: 7,
            }),
            Vec::<ScriptMovementStepIssue>::new()
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: Some(1),
                index: 8,
            }),
            vec![ScriptMovementStepIssue::UnexpectedDuration]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step".to_string(),
                direction: Some("UP".to_string()),
                duration: Some(1),
                index: 9,
            }),
            vec![ScriptMovementStepIssue::UnexpectedDuration]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step_shake".to_string(),
                direction: None,
                duration: Some(256),
                index: 10,
            }),
            vec![ScriptMovementStepIssue::DurationOutOfByteRange { duration: 256 }]
        );
        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "step_sleep".to_string(),
                direction: None,
                duration: Some(0),
                index: 11,
            }),
            vec![ScriptMovementStepIssue::ZeroSleepDuration]
        );
    }

    #[test]
    fn object_and_movement_commands_reject_reserved_pack_prefixes() {
        let object_event_flags =
            BTreeMap::from([("NPC".to_string(), "EVENT_HIDE_NPC".to_string())]);
        let hideable_event_flags = BTreeSet::from(["EVENT_HIDE_NPC".to_string()]);
        let movements = BTreeSet::from([("Walk".to_string(), Some("Script".to_string()))]);

        assert_eq!(
            script_object_command_issues(
                &command("fallbackobject", "NPC"),
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::InvalidCommand {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "fallbackobject".to_string(),
            }]
        );

        let reserved_object = command("appear", "legacy_npc");
        assert_eq!(
            script_object_command_issues(
                &reserved_object,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::InvalidObjectId {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "appear".to_string(),
                object_id: "legacy_npc".to_string(),
            }]
        );

        let mut reserved_movement = command("applymovement", "NPC");
        reserved_movement.movement = Some("fallback_walk".to_string());
        assert_eq!(
            script_object_command_issues(
                &reserved_movement,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::InvalidMovement {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "applymovement".to_string(),
                movement: "fallback_walk".to_string(),
            }]
        );

        assert_eq!(
            script_movement_step_issues(&ScriptMovementStep {
                command: "legacystep".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }),
            vec![ScriptMovementStepIssue::UnsupportedCommand]
        );
    }

    #[test]
    fn object_and_movement_json_rejects_reserved_pack_prefixes() {
        for (field, value) in [
            ("command", serde_json::json!("fallbackobject")),
            ("object_id", serde_json::json!("legacy_npc")),
            ("target_object_id", serde_json::json!("fallback_target")),
            ("direction", serde_json::json!("legacy_down")),
            ("movement", serde_json::json!("fallback_walk")),
            ("emote", serde_json::json!("legacy_emote")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "turnobject",
                "object_id": "NPC",
                "target_object_id": null,
                "x": null,
                "y": null,
                "direction": "DOWN",
                "movement": null,
                "emote": null,
                "duration": null,
                "source_script": ".branch@Script",
                "command_index": 3
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptObjectCommand>(payload)
                .expect_err("reserved script object command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script object") || error.contains("script label"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("label", serde_json::json!("fallback_walk")),
            ("source_script", serde_json::json!("legacy_script")),
        ] {
            let mut payload = serde_json::json!({
                "label": "Walk",
                "source_script": ".branch@Script",
                "steps": []
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptMovement>(payload)
                .expect_err("reserved script movement tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script object") || error.contains("script label"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("command", serde_json::json!("legacystep")),
            ("direction", serde_json::json!("fallback_down")),
        ] {
            let mut payload = serde_json::json!({
                "command": "step",
                "direction": "DOWN",
                "duration": null,
                "index": 0
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptMovementStep>(payload)
                .expect_err("reserved script movement step tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script object"),
                "{field} produced unexpected error: {error}"
            );
        }
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
        assert!(session.object_has_loaded_struct(0));

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
        assert!(!session.object_has_loaded_struct(0));

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
        assert!(session.object_has_loaded_struct(0));
    }

    #[test]
    fn appear_copies_an_offscreen_map_event_into_an_available_object_struct() {
        let mut state = GameState::default();
        state
            .flags
            .set_event_flag("EVENT_OFFSCREEN_OBJECT", true)
            .expect("hide fixture map event");
        let mut session = session(vec![object(
            "OFFSCREEN_OBJECT",
            "EVENT_OFFSCREEN_OBJECT",
            20,
            0,
        )]);
        session.sync_event_flag_memory(&state.flags);
        assert!(!session.object_has_loaded_struct(0));

        apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("appear", "OFFSCREEN_OBJECT"),
        )
        .expect("appear copies map object regardless of viewport");

        assert!(session.is_object_visible(&session.objects[0]));
        assert!(session.object_has_loaded_struct(0));
    }

    #[test]
    fn object_mutation_rejects_invalid_source_before_state_or_session_changes() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "VERMILIONCITY_BIG_SNORLAX",
            "EVENT_VERMILION_CITY_SNORLAX",
            2,
            3,
        )]);
        let mut disappear = command("disappear", "VERMILIONCITY_BIG_SNORLAX");
        disappear.source_script = "fallback_script".to_string();

        assert_eq!(
            script_object_command_issues(
                &disappear,
                &BTreeMap::from([(
                    "VERMILIONCITY_BIG_SNORLAX".to_string(),
                    "EVENT_VERMILION_CITY_SNORLAX".to_string()
                )]),
                &BTreeSet::from(["EVENT_VERMILION_CITY_SNORLAX".to_string()]),
                &BTreeSet::new(),
            ),
            vec![ScriptObjectCommandIssue::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
                command_index: 3,
            }]
        );
        assert_eq!(
            apply_script_object_mutation(&mut state, &mut session, &disappear),
            Err(ScriptObjectCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );
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

        let error = apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("disappear", "VERMILION CITY_BIG_SNORLAX"),
        )
        .expect_err("malformed object ids are rejected before lookup");
        assert_eq!(
            error,
            ScriptObjectCommandError::InvalidObjectId {
                object_id: "VERMILION CITY_BIG_SNORLAX".to_string(),
            }
        );
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_VERMILION_CITY_SNORLAX"),
            Ok(false)
        );
        assert!(session.is_object_visible(&session.objects[0]));
    }

    #[test]
    fn moveobject_updates_map_memory_without_teleporting_loaded_object_struct() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "INDIGOPLATEAUPOKECENTER1F_RIVAL",
            "EVENT_INDIGO_PLATEAU_POKECENTER_RIVAL",
            1,
            1,
        )]);
        let mut moveobject = command("moveobject", "INDIGOPLATEAUPOKECENTER1F_RIVAL");
        moveobject.x = Some(7);
        moveobject.y = Some(5);

        let outcome = apply_script_object_mutation(&mut state, &mut session, &moveobject)
            .expect("moveobject applies");

        assert_eq!((outcome.previous_x, outcome.previous_y), (Some(1), Some(1)));
        assert_eq!((outcome.x, outcome.y), (Some(7), Some(5)));
        assert_eq!((session.objects[0].x, session.objects[0].y), (7, 5));
        assert_eq!(
            session
                .object_runtime_tiles
                .get("INDIGOPLATEAUPOKECENTER1F_RIVAL"),
            Some(&TilePosition::new(1, 1))
        );

        apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("disappear", "INDIGOPLATEAUPOKECENTER1F_RIVAL"),
        )
        .expect("disappear deletes the old live object struct");
        apply_script_object_mutation(
            &mut state,
            &mut session,
            &command("appear", "INDIGOPLATEAUPOKECENTER1F_RIVAL"),
        )
        .expect("appear copies the updated map-object coordinates");
        assert_eq!(
            session
                .object_runtime_tile_by_id("INDIGOPLATEAUPOKECENTER1F_RIVAL")
                .expect("reloaded live object coordinate"),
            TilePosition::new(7, 5)
        );
    }

    #[test]
    fn moveobject_rejects_coordinates_that_overflow_runtime_tile_space() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "INDIGOPLATEAUPOKECENTER1F_RIVAL",
            "EVENT_RIVAL",
            1,
            1,
        )]);
        let mut moveobject = command("moveobject", "INDIGOPLATEAUPOKECENTER1F_RIVAL");
        moveobject.x = Some(40_000);
        moveobject.y = Some(0);

        assert_eq!(
            apply_script_object_mutation(&mut state, &mut session, &moveobject),
            Err(ScriptObjectCommandError::MoveCoordinatesOutOfRange {
                object_id: "INDIGOPLATEAUPOKECENTER1F_RIVAL".to_string(),
                x: 40_000,
                y: 0,
            })
        );
        assert_eq!((session.objects[0].x, session.objects[0].y), (1, 1));

        let object_flags = BTreeMap::from([(
            "INDIGOPLATEAUPOKECENTER1F_RIVAL".to_string(),
            "EVENT_RIVAL".to_string(),
        )]);
        assert_eq!(
            script_object_command_issues(
                &moveobject,
                &object_flags,
                &BTreeSet::new(),
                &BTreeSet::new()
            ),
            vec![ScriptObjectCommandIssue::MoveCoordinatesOutOfRange {
                source_script: "Script".to_string(),
                command_index: 3,
                x: 40_000,
                y: 0,
            }]
        );
    }

    #[test]
    fn moveobject_json_rejects_coordinates_that_overflow_runtime_tile_space() {
        let payload = serde_json::json!({
            "command": "moveobject",
            "object_id": "INDIGOPLATEAUPOKECENTER1F_RIVAL",
            "target_object_id": null,
            "x": 40000,
            "y": 0,
            "direction": null,
            "movement": null,
            "emote": null,
            "duration": null,
            "source_script": "Script",
            "command_index": 0
        });

        let error = serde_json::from_value::<ScriptObjectCommand>(payload)
            .expect_err("overflowing moveobject coordinates must fail during JSON load")
            .to_string();

        assert!(
            error.contains(
                "script object command moveobject raw event coordinates overflow runtime tile space"
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn moveobject_rejects_coordinates_outside_current_map() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "INDIGOPLATEAUPOKECENTER1F_RIVAL",
            "EVENT_RIVAL",
            1,
            1,
        )]);
        let mut moveobject = command("moveobject", "INDIGOPLATEAUPOKECENTER1F_RIVAL");
        moveobject.x = Some(8);
        moveobject.y = Some(4);

        apply_script_object_mutation(&mut state, &mut session, &moveobject)
            .expect("raw moveobject coordinates inside current map must apply");
        assert_eq!((session.objects[0].x, session.objects[0].y), (8, 4));
        assert_eq!(
            session
                .object_runtime_tiles
                .get("INDIGOPLATEAUPOKECENTER1F_RIVAL"),
            Some(&TilePosition::new(1, 1))
        );
    }

    #[test]
    fn faceobject_rejects_existing_object_coordinates_that_overflow_runtime_tile_space() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("SOURCE_NPC", "-1", 40_000, 1),
            object("TARGET_NPC", "-1", 1, 1),
        ]);
        let mut faceobject = command("faceobject", "SOURCE_NPC");
        faceobject.target_object_id = Some("TARGET_NPC".to_string());

        assert_eq!(
            apply_script_object_mutation(&mut state, &mut session, &faceobject),
            Err(ScriptObjectCommandError::ObjectCoordinatesOutOfRange {
                object_id: "SOURCE_NPC".to_string(),
                x: 40_000,
                y: 1,
            })
        );
    }

    #[test]
    fn object_runtime_tile_updates_accept_negative_transient_positions() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            4,
            4,
        )]);

        set_object_tile(
            &mut session,
            "ECRUTEAKPOKECENTER1F_BILL",
            TilePosition::new(-1, 9),
        )
        .expect("scripted object positions may move offscreen");

        assert_eq!(
            session
                .object_runtime_tiles
                .get("ECRUTEAKPOKECENTER1F_BILL")
                .copied(),
            Some(TilePosition::new(-1, 9))
        );
        assert_eq!((session.objects[0].x, session.objects[0].y), (4, 4));
    }

    #[test]
    fn object_runtime_tile_updates_accept_positions_outside_current_map() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            4,
            4,
        )]);

        set_object_tile(
            &mut session,
            "ECRUTEAKPOKECENTER1F_BILL",
            TilePosition::new(8, 4),
        )
        .expect("scripted object positions may move outside the visible map");

        assert_eq!(
            session
                .object_runtime_tiles
                .get("ECRUTEAKPOKECENTER1F_BILL")
                .copied(),
            Some(TilePosition::new(8, 4))
        );
        assert_eq!((session.objects[0].x, session.objects[0].y), (4, 4));
    }

    #[test]
    fn applymovement_accepts_out_of_map_endpoint_as_transient_object_state() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            7,
            4,
        )]);
        session
            .copy_object_struct_for_appear("ECRUTEAKPOKECENTER1F_BILL")
            .expect("allocate movement fixture");
        let mut movement_command = command("applymovement", "ECRUTEAKPOKECENTER1F_BILL");
        movement_command.movement = Some("MovesOutOfMap".to_string());
        let movement = ScriptMovement {
            label: "MovesOutOfMap".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "hide_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 1,
                },
            ],
        };

        let outcome = apply_script_movement(&mut session, &movement_command, &movement)
            .expect("object movement may leave the visible map");

        assert_eq!(
            session
                .object_runtime_tiles
                .get("ECRUTEAKPOKECENTER1F_BILL")
                .copied(),
            Some(TilePosition::new(8, 4))
        );
        assert_eq!((session.objects[0].x, session.objects[0].y), (7, 4));
        assert_eq!(outcome.steps_applied, 2);
        assert!(!session.object_struct_is_visible(0));
    }

    #[test]
    fn movement_remove_object_deletes_only_the_object_struct_not_the_map_event() {
        let object = object("TRANSIENT_REMOVAL", "-1", 1, 1);
        let mut session = session(vec![object.clone()]);
        let mut movement_command = command("applymovement", "TRANSIENT_REMOVAL");
        movement_command.movement = Some("RemoveStruct".to_string());
        let movement = ScriptMovement {
            label: "RemoveStruct".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "remove_object".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        };

        apply_script_movement(&mut session, &movement_command, &movement)
            .expect("remove object movement applies");

        assert!(!session.object_has_loaded_struct(0));
        assert!(
            session.is_object_visible(&object),
            "Movement_remove_object leaves the map object unmasked"
        );
        assert!(
            session
                .copy_object_struct_for_appear("TRANSIENT_REMOVAL")
                .expect("the retained map object can be copied again")
        );
        assert!(session.object_has_loaded_struct(0));
    }

    #[test]
    fn deleting_an_object_struct_discards_transient_movement_flags() {
        let mut session = session(vec![object("TRANSIENT_FLAGS", "-1", 1, 1)]);
        let mut movement_command = command("applymovement", "TRANSIENT_FLAGS");
        movement_command.movement = Some("SetTransientFlags".to_string());
        let movement = ScriptMovement {
            label: "SetTransientFlags".to_string(),
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
                    command: "hide_object".to_string(),
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

        apply_script_movement(&mut session, &movement_command, &movement)
            .expect("transient flag movement applies");
        session
            .following_not_exact
            .insert("TRANSIENT_FLAGS".to_string(), 0);
        assert!(
            session
                .fixed_facing_object_identifiers
                .contains("TRANSIENT_FLAGS")
        );
        assert!(
            session
                .sliding_object_identifiers
                .contains("TRANSIENT_FLAGS")
        );
        assert!(!session.object_struct_is_visible(0));

        session.delete_loaded_object_struct("TRANSIENT_FLAGS");

        assert!(
            !session
                .fixed_facing_object_identifiers
                .contains("TRANSIENT_FLAGS")
        );
        assert!(
            !session
                .sliding_object_identifiers
                .contains("TRANSIENT_FLAGS")
        );
        assert!(
            !session
                .hidden_object_identifiers
                .contains("TRANSIENT_FLAGS")
        );
        assert!(!session.following_not_exact.contains_key("TRANSIENT_FLAGS"));
        assert!(
            session
                .copy_object_struct_for_appear("TRANSIENT_FLAGS")
                .expect("map object can be copied after transient struct deletion")
        );
        assert!(session.object_struct_is_visible(0));
    }

    #[test]
    fn applymovement_rejects_runtime_tile_overflow_without_partial_visibility_mutation() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            i16::MAX as u16,
            0,
        )]);
        session
            .copy_object_struct_for_appear("ECRUTEAKPOKECENTER1F_BILL")
            .expect("allocate movement fixture");
        let mut movement_command = command("applymovement", "ECRUTEAKPOKECENTER1F_BILL");
        movement_command.movement = Some("MovesPastRuntimeLimit".to_string());
        let movement = ScriptMovement {
            label: "MovesPastRuntimeLimit".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "hide_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 1,
                },
            ],
        };

        let error = apply_script_movement(&mut session, &movement_command, &movement)
            .expect_err("overflowing endpoint rejects");

        assert_eq!(
            error,
            ScriptObjectCommandError::MovementRuntimeTileOverflow {
                movement: "MovesPastRuntimeLimit".to_string(),
                command: "step".to_string(),
                index: 1,
                x: i16::MAX,
                y: 0,
            }
        );
        assert_eq!(
            (session.objects[0].x, session.objects[0].y),
            (i16::MAX as u16, 0)
        );
        assert!(
            !session
                .hidden_object_identifiers
                .contains("ECRUTEAKPOKECENTER1F_BILL")
        );
    }

    #[test]
    fn applymovement_with_empty_follower_slot_moves_leader_and_retains_the_queue() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            4,
            4,
        )]);
        session.following = Some(OverworldFollowState {
            leader_slot: Some(1),
            follower_slot: Some(12),
        });
        let mut movement_command = command("applymovement", "ECRUTEAKPOKECENTER1F_BILL");
        movement_command.movement = Some("MoveWithFollower".to_string());
        let movement = ScriptMovement {
            label: "MoveWithFollower".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "hide_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 1,
                },
            ],
        };

        apply_script_movement(&mut session, &movement_command, &movement)
            .expect("an empty referenced struct slot does not reject leader movement");

        assert_eq!((session.objects[0].x, session.objects[0].y), (4, 4));
        assert_eq!(
            object_tile(&session, "ECRUTEAKPOKECENTER1F_BILL").unwrap(),
            TilePosition::new(5, 4)
        );
        assert!(
            session
                .invisible_object_struct_identifiers
                .contains("ECRUTEAKPOKECENTER1F_BILL")
        );
        assert_eq!(
            session.following_queued_step.map(|step| step.direction),
            Some(Direction::Right)
        );
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
    fn showemote_queues_exact_object_emote() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "ROUTE43GATE_ROCKET1",
            "EVENT_ROUTE43GATE_ROCKETS",
            4,
            4,
        )]);
        let mut command = command("showemote", "ROUTE43GATE_ROCKET1");
        command.emote = Some("EMOTE_SHOCK".to_string());
        command.duration = Some(15);
        command.command_index = 8;

        let outcome =
            apply_script_object_mutation(&mut state, &mut session, &command).expect("showemote");

        assert_eq!(outcome.command, "showemote");
        assert_eq!(outcome.object_id, "ROUTE43GATE_ROCKET1");
        assert_eq!(state.script_runtime.pending_emotes.len(), 1);
        assert_eq!(state.script_runtime.pending_emotes[0].emote, "EMOTE_SHOCK");
        assert_eq!(
            state.script_runtime.pending_emotes[0].object,
            "ROUTE43GATE_ROCKET1"
        );
        assert_eq!(state.script_runtime.pending_emotes[0].duration, 15);
        assert_eq!(state.script_runtime.pending_emotes[0].frames, 30);
        assert_eq!(
            state.script_runtime.pending_emotes[0].source_script,
            "Script"
        );
        assert_eq!(state.script_runtime.pending_emotes[0].command_index, 8);
    }

    #[test]
    fn showemote_resolves_last_talked_without_object_id_fallback() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "POKECENTER_NURSE",
            "EVENT_POKECENTER_NURSE",
            2,
            2,
        )]);
        session.last_talked_object_identifier = Some("POKECENTER_NURSE".to_string());
        let command = ScriptObjectCommand {
            command: "showemote".to_string(),
            object_id: Some("LAST_TALKED".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: Some("EMOTE_HAPPY".to_string()),
            duration: Some(16),
            source_script: "Script".to_string(),
            command_index: 9,
        };

        apply_script_object_mutation(&mut state, &mut session, &command).expect("showemote");

        assert_eq!(
            state.script_runtime.pending_emotes[0].object,
            "POKECENTER_NURSE"
        );

        session.last_talked_object_identifier = Some("missing_nurse".to_string());
        let error = apply_script_object_mutation(&mut state, &mut session, &command)
            .expect_err("last talked must resolve to an exact object");
        assert_eq!(
            error,
            ScriptObjectCommandError::UnknownObject {
                object_id: "missing_nurse".to_string()
            }
        );

        session.last_talked_object_identifier = Some("MISSING_NURSE".to_string());
        let error = apply_script_object_mutation(&mut state, &mut session, &command)
            .expect_err("last talked must name an existing object");
        assert_eq!(
            error,
            ScriptObjectCommandError::UnknownObject {
                object_id: "MISSING_NURSE".to_string()
            }
        );
    }

    #[test]
    fn showemote_rejects_duration_outside_the_script_byte_without_mutation() {
        let mut state = GameState::default();
        let mut session = session(vec![object(
            "ROUTE43GATE_ROCKET1",
            "EVENT_ROUTE43GATE_ROCKETS",
            4,
            4,
        )]);
        let mut command = command("showemote", "ROUTE43GATE_ROCKET1");
        command.emote = Some("EMOTE_SHOCK".to_string());
        command.duration = Some(256);

        assert_eq!(
            apply_script_object_mutation(&mut state, &mut session, &command),
            Err(ScriptObjectCommandError::EmoteDurationOutOfByteRange { duration: 256 })
        );
        assert!(state.script_runtime.pending_emotes.is_empty());
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
                leader_slot: Some(1),
                follower_slot: Some(0),
            })
        );

        let mut face = command("faceobject", "PLAYER");
        face.target_object_id = Some("BATTLETOWERHALLWAY_RECEPTIONIST".to_string());
        apply_script_object_mutation(&mut state, &mut session, &face).expect("faceobject applies");
        assert_eq!(session.player.facing, Direction::Up);

        session.last_talked_object_identifier = Some("BATTLETOWER1F_RECEPTIONIST".to_string());
        let face_player = ScriptObjectCommand {
            command: "faceplayer".to_string(),
            object_id: None,
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "Script".to_string(),
            command_index: 8,
        };
        let face_player_outcome =
            apply_script_object_mutation(&mut state, &mut session, &face_player)
                .expect("faceplayer applies");
        assert_eq!(face_player_outcome.object_id, "BATTLETOWER1F_RECEPTIONIST");
        assert_eq!(
            session.object_facings.get("BATTLETOWER1F_RECEPTIONIST"),
            Some(&Direction::Down)
        );

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
    fn follow_preserves_partial_slot_writes_from_visibility_checks() {
        let mut state = GameState::default();
        let mut leader_missing = session(vec![object("LEADER", "-1", 4, 4)]);
        leader_missing.delete_loaded_object_struct("LEADER");
        leader_missing.following = Some(OverworldFollowState {
            leader_slot: Some(7),
            follower_slot: Some(0),
        });
        let mut follow = command("follow", "LEADER");
        follow.target_object_id = Some("PLAYER".to_string());
        apply_script_object_mutation(&mut state, &mut leader_missing, &follow)
            .expect("missing leader is a successful no-op");
        assert_eq!(
            leader_missing.following,
            Some(OverworldFollowState {
                leader_slot: Some(7),
                follower_slot: Some(0),
            })
        );

        let mut follower_missing = session(vec![
            object("LEADER", "-1", 4, 4),
            object("FOLLOWER", "-1", 5, 4),
        ]);
        follower_missing.delete_loaded_object_struct("FOLLOWER");
        let mut follow = command("follow", "LEADER");
        follow.target_object_id = Some("FOLLOWER".to_string());
        apply_script_object_mutation(&mut state, &mut follower_missing, &follow)
            .expect("missing follower retains the newly written leader slot");
        assert_eq!(
            follower_missing.following,
            Some(OverworldFollowState {
                leader_slot: Some(1),
                follower_slot: None,
            })
        );
    }

    #[test]
    fn follow_replaces_and_stopfollow_resets_the_follower_movement_phase() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("LEADER", "-1", 4, 4),
            object("OLD_FOLLOWER", "-1", 5, 4),
            object("NEW_FOLLOWER", "-1", 6, 4),
        ]);
        session.following = Some(OverworldFollowState {
            leader_slot: Some(1),
            follower_slot: Some(2),
        });
        session
            .normal_following_object_identifiers
            .insert("OLD_FOLLOWER".to_string());
        for object_id in ["OLD_FOLLOWER", "NEW_FOLLOWER"] {
            session
                .object_step_durations
                .insert(object_id.to_string(), 37);
            session
                .object_pending_random_wait
                .insert(object_id.to_string());
            session
                .initialized_fixed_spin_objects
                .insert(object_id.to_string());
            session.following_not_exact.insert(object_id.to_string(), 1);
        }
        session
            .object_runtime_tiles
            .insert("OLD_FOLLOWER".to_string(), TilePosition::new(5, 4));
        session
            .object_last_runtime_tiles
            .insert("OLD_FOLLOWER".to_string(), TilePosition::new(4, 4));
        session
            .object_last_tiles_occupied_until_frame
            .insert("OLD_FOLLOWER".to_string(), 8);
        session
            .object_runtime_tiles
            .insert("NEW_FOLLOWER".to_string(), TilePosition::new(6, 4));
        session
            .object_last_runtime_tiles
            .insert("NEW_FOLLOWER".to_string(), TilePosition::new(7, 4));
        session
            .object_last_tiles_occupied_until_frame
            .insert("NEW_FOLLOWER".to_string(), 8);

        let mut follow = command("follow", "LEADER");
        follow.target_object_id = Some("NEW_FOLLOWER".to_string());
        apply_script_object_mutation(&mut state, &mut session, &follow)
            .expect("replacement follow applies");

        for object_id in ["OLD_FOLLOWER", "NEW_FOLLOWER"] {
            assert!(!session.object_step_durations.contains_key(object_id));
            assert!(!session.object_pending_random_wait.contains(object_id));
            assert!(!session.initialized_fixed_spin_objects.contains(object_id));
            assert!(!session.following_not_exact.contains_key(object_id));
            assert_eq!(
                session.object_last_runtime_tiles.get(object_id),
                session.object_runtime_tiles.get(object_id),
                "STEP_TYPE_RESET copies the live coordinate into OBJECT_LAST_MAP_X/Y"
            );
            assert!(
                !session
                    .object_last_tiles_occupied_until_frame
                    .contains_key(object_id),
                "the pre-reset tile must stop owning collision"
            );
        }

        session
            .object_runtime_tiles
            .insert("NEW_FOLLOWER".to_string(), TilePosition::new(7, 4));
        session
            .object_last_runtime_tiles
            .insert("NEW_FOLLOWER".to_string(), TilePosition::new(6, 4));
        session
            .object_last_tiles_occupied_until_frame
            .insert("NEW_FOLLOWER".to_string(), 8);
        session
            .object_step_durations
            .insert("NEW_FOLLOWER".to_string(), 19);
        session
            .object_pending_random_wait
            .insert("NEW_FOLLOWER".to_string());
        let stop = command("stopfollow", "");
        apply_script_object_mutation(&mut state, &mut session, &stop).expect("stopfollow applies");

        assert!(!session.object_step_durations.contains_key("NEW_FOLLOWER"));
        assert!(!session.object_pending_random_wait.contains("NEW_FOLLOWER"));
        assert_eq!(
            session.object_last_runtime_tiles.get("NEW_FOLLOWER"),
            session.object_runtime_tiles.get("NEW_FOLLOWER")
        );
        assert!(
            !session
                .object_last_tiles_occupied_until_frame
                .contains_key("NEW_FOLLOWER")
        );
        assert_eq!(session.following, None);
        assert_eq!(session.following_queued_step, None);
    }

    #[test]
    fn stopfollow_resets_a_player_followers_last_coordinate() {
        let mut state = GameState::default();
        let mut session = session(vec![object("GUIDE", "-1", 4, 4)]);
        session.player.tile = TilePosition::new(5, 4);
        session.player_last_runtime_tile = Some(TilePosition::new(6, 4));
        session.player_last_tile_occupied_until_frame = 8;
        session.following = Some(OverworldFollowState {
            leader_slot: Some(1),
            follower_slot: Some(0),
        });
        session
            .normal_following_object_identifiers
            .insert("PLAYER".to_string());

        let stop = command("stopfollow", "");
        apply_script_object_mutation(&mut state, &mut session, &stop).expect("stopfollow applies");

        assert_eq!(session.player_last_runtime_tile, Some(session.player.tile));
        assert_eq!(session.player_last_tile_occupied_until_frame, session.frame);
        assert!(
            !session
                .normal_following_object_identifiers
                .contains("PLAYER")
        );
    }

    #[test]
    fn disappear_stops_follow_when_deleting_either_participating_object_struct() {
        for disappeared in ["FOLLOW_LEADER", "FOLLOWER"] {
            let mut state = GameState::default();
            let mut session = session(vec![
                object("FOLLOW_LEADER", "-1", 4, 4),
                object("FOLLOWER", "-1", 5, 4),
            ]);
            let mut follow = command("follow", "FOLLOW_LEADER");
            follow.target_object_id = Some("FOLLOWER".to_string());
            apply_script_object_mutation(&mut state, &mut session, &follow)
                .expect("follow applies");
            assert!(session.following.is_some());

            let disappear = command("disappear", disappeared);
            apply_script_object_mutation(&mut state, &mut session, &disappear)
                .expect("disappear applies");

            assert_eq!(session.following, None);
            assert_eq!(session.following_queued_step, None);
        }
    }

    #[test]
    fn raw_follower_slot_reuse_does_not_transfer_the_following_movement_type() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("FOLLOW_LEADER", "-1", 4, 4),
            object("OLD_FOLLOWER", "-1", 5, 4),
            object("NEW_FOLLOWER", "-1", 5, 4),
        ]);
        session.delete_loaded_object_struct("NEW_FOLLOWER");
        let mut follow = command("follow", "FOLLOW_LEADER");
        follow.target_object_id = Some("OLD_FOLLOWER".to_string());
        apply_script_object_mutation(&mut state, &mut session, &follow).expect("follow applies");
        let remove = ScriptMovement {
            label: "RemoveFollowerStruct".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "remove_object".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        };
        let mut remove_command = command("applymovement", "OLD_FOLLOWER");
        remove_command.movement = Some(remove.label.clone());
        apply_script_movement(&mut session, &remove_command, &remove)
            .expect("raw follower deletion applies");
        assert!(
            session
                .copy_object_struct_for_appear("NEW_FOLLOWER")
                .expect("replacement follower takes the freed slot")
        );
        assert_eq!(session.object_struct_slot("NEW_FOLLOWER"), Some(2));
        let replacement_before =
            object_tile(&session, "NEW_FOLLOWER").expect("replacement follower tile");
        let leader_step = ScriptMovement {
            label: "LeaderStep".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
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

        let mut leader_command = command("applymovement", "FOLLOW_LEADER");
        leader_command.movement = Some(leader_step.label.clone());
        apply_script_movement(&mut session, &leader_command, &leader_step)
            .expect("leader movement applies");

        assert_eq!(
            object_tile(&session, "NEW_FOLLOWER").expect("replacement follower remains autonomous"),
            replacement_before
        );
        assert_eq!(
            session.following_queued_step.map(|step| step.direction),
            Some(Direction::Right),
            "the raw queue byte still advances even though the replacement struct cannot consume it"
        );
    }

    #[test]
    fn movement_remove_object_compares_the_deleted_struct_slot_to_the_leader_slot() {
        let mut session = session(vec![
            object("UNRELATED", "-1", 3, 4),
            object("FOLLOWER", "-1", 5, 4),
            object("LEADER", "-1", 4, 4),
        ]);
        session.delete_loaded_object_struct("UNRELATED");
        session.delete_loaded_object_struct("LEADER");
        assert!(session.copy_object_struct_for_appear("LEADER").unwrap());
        assert!(session.copy_object_struct_for_appear("UNRELATED").unwrap());
        assert_eq!(session.object_struct_slot("LEADER"), Some(1));
        assert_eq!(session.object_struct_slot("UNRELATED"), Some(3));
        session.following = Some(OverworldFollowState {
            leader_slot: Some(1),
            follower_slot: Some(2),
        });
        let remove = ScriptMovement {
            label: "RemoveUnrelated".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "remove_object".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        };
        let mut remove_command = command("applymovement", "UNRELATED");
        remove_command.movement = Some(remove.label.clone());

        apply_script_movement(&mut session, &remove_command, &remove)
            .expect("unrelated raw deletion applies");

        assert_eq!(
            session.following,
            Some(OverworldFollowState {
                leader_slot: Some(1),
                follower_slot: Some(2),
            })
        );

        let mut remove_leader_command = command("applymovement", "LEADER");
        remove_leader_command.movement = Some(remove.label.clone());
        apply_script_movement(&mut session, &remove_leader_command, &remove)
            .expect("leader raw deletion applies");

        assert_eq!(
            session.following,
            Some(OverworldFollowState {
                leader_slot: None,
                follower_slot: Some(2),
            })
        );
    }

    #[test]
    fn follownotexact_moves_object1_toward_object2_and_tracks_its_last_coordinate() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("LOOSE_LEADER", "EVENT_LOOSE_LEADER", 4, 4),
            object("LOOSE_FOLLOWER", "EVENT_LOOSE_FOLLOWER", 6, 4),
        ]);
        let leader_tile = object_tile(&session, "LOOSE_LEADER").expect("leader tile");
        let follower_before = object_tile(&session, "LOOSE_FOLLOWER").expect("follower tile");
        let mut command = command("follownotexact", "LOOSE_LEADER");
        command.target_object_id = Some("LOOSE_FOLLOWER".to_string());

        let outcome = apply_script_object_mutation(&mut state, &mut session, &command)
            .expect("follownotexact applies");
        let follower_after = object_tile(&session, "LOOSE_FOLLOWER").expect("moved follower");
        assert_eq!(follower_after.x, follower_before.x - 1);
        assert_eq!(follower_after.y, follower_before.y);
        assert_eq!(outcome.object_id, "LOOSE_FOLLOWER");
        assert_eq!(session.following_not_exact.get("LOOSE_FOLLOWER"), Some(&1));

        session.update_follow_after_entity_move(
            "LOOSE_LEADER",
            leader_tile,
            TilePosition::new(leader_tile.x + 1, leader_tile.y),
        );
        let follower_second = object_tile(&session, "LOOSE_FOLLOWER").expect("tracked follower");
        assert_eq!(follower_second.x, follower_after.x - 1);
        assert_eq!(follower_second.y, follower_after.y);
        assert_eq!(
            session.object_step_durations.get("LOOSE_FOLLOWER"),
            Some(&8)
        );
    }

    #[test]
    fn follownotexact_returns_without_mutation_when_either_object_struct_is_unloaded() {
        for unloaded_object_id in ["LOOSE_LEADER", "LOOSE_FOLLOWER"] {
            let mut state = GameState::default();
            let mut session = session(vec![
                object("LOOSE_LEADER", "EVENT_LOOSE_LEADER", 4, 4),
                object("LOOSE_FOLLOWER", "EVENT_LOOSE_FOLLOWER", 6, 4),
            ]);
            session.delete_loaded_object_struct(unloaded_object_id);
            let follower_before = object_tile(&session, "LOOSE_FOLLOWER").expect("follower tile");
            let mut command = command("follownotexact", "LOOSE_LEADER");
            command.target_object_id = Some("LOOSE_FOLLOWER".to_string());

            let outcome = apply_script_object_mutation(&mut state, &mut session, &command)
                .expect("an unloaded object makes follownotexact a successful no-op");

            assert_eq!(outcome.object_id, "LOOSE_FOLLOWER");
            assert_eq!(
                object_tile(&session, "LOOSE_FOLLOWER").expect("follower tile after command"),
                follower_before
            );
            assert!(session.following_not_exact.is_empty());
        }
    }

    #[test]
    fn follownotexact_tracks_the_leader_struct_slot_after_allocator_reuse() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("OLD_LEADER", "EVENT_OLD_LEADER", 4, 4),
            object("LOOSE_FOLLOWER", "EVENT_LOOSE_FOLLOWER", 6, 4),
            object("NEW_LEADER", "EVENT_NEW_LEADER", 4, 4),
        ]);
        session.delete_loaded_object_struct("NEW_LEADER");
        let mut command = command("follownotexact", "OLD_LEADER");
        command.target_object_id = Some("LOOSE_FOLLOWER".to_string());
        apply_script_object_mutation(&mut state, &mut session, &command)
            .expect("follownotexact applies");
        assert_eq!(session.following_not_exact.get("LOOSE_FOLLOWER"), Some(&1));

        session.delete_loaded_object_struct("OLD_LEADER");
        assert!(
            session
                .copy_object_struct_for_appear("NEW_LEADER")
                .expect("replacement object can reuse the deleted leader's slot")
        );
        assert_eq!(session.object_struct_slot("NEW_LEADER"), Some(1));
        let follower_before = object_tile(&session, "LOOSE_FOLLOWER").expect("follower tile");
        session.update_follow_after_entity_move(
            "NEW_LEADER",
            TilePosition::new(4, 4),
            TilePosition::new(5, 4),
        );

        assert_ne!(
            object_tile(&session, "LOOSE_FOLLOWER").expect("tracked follower tile"),
            follower_before,
            "OBJECT_RANGE follows the new occupant of the stored struct slot"
        );
    }

    #[test]
    fn facing_commands_return_without_turning_unloaded_objects() {
        let mut state = GameState::default();
        let mut session = session(vec![
            object("HIDDEN_NPC", "EVENT_HIDDEN_NPC", 4, 4),
            object("VISIBLE_NPC", "EVENT_VISIBLE_NPC", 4, 6),
            object("STILL_OBJECT", "EVENT_STILL_OBJECT", 6, 6),
        ]);
        session.objects[2].sprite_has_facings = false;
        session.delete_loaded_object_struct("HIDDEN_NPC");
        session
            .object_facings
            .insert("HIDDEN_NPC".to_string(), Direction::Left);
        session
            .object_facings
            .insert("VISIBLE_NPC".to_string(), Direction::Right);

        let mut turn = command("turnobject", "HIDDEN_NPC");
        turn.direction = Some("DOWN".to_string());
        apply_script_object_mutation(&mut state, &mut session, &turn)
            .expect("turnobject returns like CheckObjectVisibility carry");
        assert_eq!(
            session.object_facings.get("HIDDEN_NPC"),
            Some(&Direction::Left)
        );

        let mut face = command("faceobject", "VISIBLE_NPC");
        face.target_object_id = Some("HIDDEN_NPC".to_string());
        apply_script_object_mutation(&mut state, &mut session, &face)
            .expect("faceobject returns when GetRelativeFacing cannot see its target");
        assert_eq!(
            session.object_facings.get("VISIBLE_NPC"),
            Some(&Direction::Right)
        );

        session
            .object_facings
            .insert("STILL_OBJECT".to_string(), Direction::Up);
        let mut turn_still = command("turnobject", "STILL_OBJECT");
        turn_still.direction = Some("RIGHT".to_string());
        apply_script_object_mutation(&mut state, &mut session, &turn_still)
            .expect("STILL_SPRITE turnobject returns without turning");
        assert_eq!(
            session.object_facings.get("STILL_OBJECT"),
            Some(&Direction::Up)
        );
    }

    #[test]
    fn faceplayer_returns_when_last_talked_is_the_no_object_sentinel() {
        let mut state = GameState::default();
        let mut session = session(Vec::new());
        let command = ScriptObjectCommand {
            command: "faceplayer".to_string(),
            object_id: None,
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "NoLastTalkedScript".to_string(),
            command_index: 0,
        };

        let outcome = apply_script_object_mutation(&mut state, &mut session, &command)
            .expect("hLastTalked == 0 returns without an error");
        assert_eq!(outcome.object_id, "LAST_TALKED");
        assert!(session.object_facings.is_empty());
    }

    #[test]
    fn fixed_facing_and_sliding_bits_survive_separate_movement_programs() {
        let object_id = "SCRIPT_NPC";
        let mut state = GameState::default();
        let mut session = session(vec![object(object_id, "EVENT_SCRIPT_NPC", 4, 4)]);
        session
            .object_facings
            .insert(object_id.to_string(), Direction::Left);

        let apply =
            |session: &mut OverworldSession, label: &str, steps: Vec<ScriptMovementStep>| {
                let mut command = command("applymovement", object_id);
                command.movement = Some(label.to_string());
                apply_script_movement(
                    session,
                    &command,
                    &ScriptMovement {
                        label: label.to_string(),
                        source_script: None,
                        steps,
                    },
                )
                .expect("movement applies")
            };
        let no_arg = |command: &str, index| ScriptMovementStep {
            command: command.to_string(),
            direction: None,
            duration: None,
            index,
        };

        let fixed = apply(
            &mut session,
            "FixFacing",
            vec![no_arg("fix_facing", 0), no_arg("set_sliding", 1)],
        );
        assert!(fixed.fixed_facing);
        assert!(fixed.sliding);

        let mut turn = command("turnobject", object_id);
        turn.direction = Some("DOWN".to_string());
        apply_script_object_mutation(&mut state, &mut session, &turn)
            .expect("fixed-facing turnobject returns without turning");
        assert_eq!(
            session.object_facings.get(object_id),
            Some(&Direction::Left)
        );

        let retained = apply(
            &mut session,
            "RetainFlags",
            vec![ScriptMovementStep {
                command: "step".to_string(),
                direction: Some("DOWN".to_string()),
                duration: None,
                index: 0,
            }],
        );
        assert_eq!(retained.tile, TilePosition::new(4, 5));
        assert_eq!(retained.facing, Direction::Left);
        assert!(retained.fixed_facing);
        assert!(retained.sliding);

        let released = apply(
            &mut session,
            "ReleaseFlags",
            vec![
                no_arg("remove_fixed_facing", 0),
                no_arg("remove_sliding", 1),
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 2,
                },
            ],
        );
        assert_eq!(released.facing, Direction::Down);
        assert!(!released.fixed_facing);
        assert!(!released.sliding);
        assert!(session.fixed_facing_object_identifiers.is_empty());
        assert!(session.sliding_object_identifiers.is_empty());
    }

    #[test]
    fn applymovement_moves_player_and_objects_by_exact_runtime_steps() {
        let mut session = session(vec![object(
            "ECRUTEAKPOKECENTER1F_BILL",
            "EVENT_BILL_IN_ECRUTEAK",
            4,
            4,
        )]);
        session.player.tile = TilePosition::new(2, 6);
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
        assert_eq!(player_outcome.previous_tile, TilePosition::new(2, 6));
        assert_eq!(player_outcome.tile, TilePosition::new(2, 5));
        assert_eq!(session.player.tile, TilePosition::new(2, 5));
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
        assert_eq!((session.objects[0].x, session.objects[0].y), (4, 4));
        assert_eq!(
            session
                .object_runtime_tiles
                .get("ECRUTEAKPOKECENTER1F_BILL")
                .copied(),
            Some(TilePosition::new(5, 5))
        );
        assert_eq!(
            session.object_facings.get("ECRUTEAKPOKECENTER1F_BILL"),
            Some(&Direction::Down)
        );
    }

    #[test]
    fn applymovement_follow_advances_per_script_step() {
        let mut session = session(vec![object("GUIDE", "-1", 10, 6)]);
        session.player.tile = TilePosition::new(9, 6);
        session
            .copy_object_struct_for_appear("GUIDE")
            .expect("allocate guide fixture");
        session.following = Some(OverworldFollowState {
            leader_slot: Some(1),
            follower_slot: Some(0),
        });
        session
            .normal_following_object_identifiers
            .insert("PLAYER".to_string());
        let mut command = command("applymovement", "GUIDE");
        command.movement = Some("GuideWalks".to_string());
        let movement = ScriptMovement {
            label: "GuideWalks".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("LEFT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("LEFT".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("LEFT".to_string()),
                    duration: None,
                    index: 3,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 4,
                },
            ],
        };

        let outcome = apply_script_movement(&mut session, &command, &movement)
            .expect("guide movement applies");

        assert_eq!(outcome.previous_tile, TilePosition::new(10, 6));
        assert_eq!(outcome.tile, TilePosition::new(7, 5));
        assert_eq!(session.player.tile, TilePosition::new(8, 5));
        assert_eq!(
            session.player.facing,
            Direction::Up,
            "the follower faces along the previously released queue entry, not the leader's still-buffered final step"
        );
    }

    #[test]
    fn applymovement_guide_tour_follow_path_ends_on_last_leader_step() {
        let mut session = session(vec![object("CHERRYGROVECITY_GRAMPS", "-1", 32, 6)]);
        session.player.tile = TilePosition::new(32, 7);
        session
            .copy_object_struct_for_appear("CHERRYGROVECITY_GRAMPS")
            .expect("allocate guide fixture");
        session.following = Some(OverworldFollowState {
            leader_slot: Some(1),
            follower_slot: Some(0),
        });
        session
            .normal_following_object_identifiers
            .insert("PLAYER".to_string());

        apply_test_movement(
            &mut session,
            "CHERRYGROVECITY_GRAMPS",
            "GuideGentMovement1",
            &["LEFT", "LEFT", "UP", "LEFT"],
        );
        apply_test_movement(
            &mut session,
            "CHERRYGROVECITY_GRAMPS",
            "GuideGentMovement2",
            &["LEFT", "LEFT", "LEFT", "LEFT", "LEFT", "LEFT"],
        );
        apply_test_movement(
            &mut session,
            "CHERRYGROVECITY_GRAMPS",
            "GuideGentMovement3",
            &["LEFT", "LEFT", "LEFT", "LEFT", "LEFT", "LEFT", "LEFT"],
        );
        apply_test_movement(
            &mut session,
            "CHERRYGROVECITY_GRAMPS",
            "GuideGentMovement4",
            &[
                "LEFT", "LEFT", "LEFT", "DOWN", "LEFT", "LEFT", "LEFT", "DOWN",
            ],
        );
        apply_test_movement(
            &mut session,
            "CHERRYGROVECITY_GRAMPS",
            "GuideGentMovement5",
            &[
                "DOWN", "DOWN", "RIGHT", "RIGHT", "RIGHT", "RIGHT", "RIGHT", "RIGHT", "RIGHT",
                "RIGHT", "RIGHT", "RIGHT", "DOWN", "DOWN", "RIGHT", "RIGHT", "RIGHT", "RIGHT",
                "RIGHT",
            ],
        );

        assert_eq!(session.player.tile, TilePosition::new(24, 11));
        assert_eq!(
            session.object_runtime_tiles.get("CHERRYGROVECITY_GRAMPS"),
            Some(&TilePosition::new(25, 11))
        );
    }

    #[test]
    fn applymovement_rejects_malformed_matching_label_before_mutating_session() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(2, 2);
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("Player Walks".to_string());
        let movement = ScriptMovement {
            label: "Player Walks".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "step".to_string(),
                direction: Some("UP".to_string()),
                duration: None,
                index: 0,
            }],
        };

        assert_eq!(
            apply_script_movement(&mut session, &command, &movement),
            Err(ScriptObjectCommandError::InvalidMovement {
                movement: "Player Walks".to_string(),
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(2, 2));
    }

    #[test]
    fn applymovement_rejects_invalid_source_before_mutating_session() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(2, 2);
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("PlayerWalks".to_string());
        let movement = ScriptMovement {
            label: "PlayerWalks".to_string(),
            source_script: Some("legacy_script".to_string()),
            steps: vec![ScriptMovementStep {
                command: "step".to_string(),
                direction: Some("UP".to_string()),
                duration: None,
                index: 0,
            }],
        };

        assert_eq!(
            apply_script_movement(&mut session, &command, &movement),
            Err(ScriptObjectCommandError::InvalidSourceScript {
                source_script: "legacy_script".to_string(),
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(2, 2));

        command.source_script = "fallback_script".to_string();
        let movement = ScriptMovement {
            source_script: None,
            ..movement
        };
        assert_eq!(
            apply_script_movement(&mut session, &command, &movement),
            Err(ScriptObjectCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(2, 2));
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
        assert_eq!((session.objects[0].x, session.objects[0].y), (5, 5));
        assert_eq!(
            session
                .object_runtime_tiles
                .get("POKECENTER2F_RECEPTIONIST")
                .copied(),
            Some(TilePosition::new(5, 4))
        );
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
                    command: "big_step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "jump_step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 3,
                },
                ScriptMovementStep {
                    command: "remove_sliding".to_string(),
                    direction: None,
                    duration: None,
                    index: 4,
                },
                ScriptMovementStep {
                    command: "remove_fixed_facing".to_string(),
                    direction: None,
                    duration: None,
                    index: 5,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 6,
                },
            ],
        };

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("slide applies");

        assert_eq!(outcome.previous_tile, TilePosition::new(0, 0));
        assert_eq!(outcome.tile, TilePosition::new(3, 0));
        assert_eq!(outcome.facing, Direction::Left);
        assert_eq!(
            outcome
                .executed_steps
                .iter()
                .map(|step| step.command.as_str())
                .collect::<Vec<_>>(),
            vec![
                "fix_facing",
                "set_sliding",
                "big_step",
                "jump_step",
                "remove_sliding",
                "remove_fixed_facing"
            ]
        );
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
                    index: 4,
                },
                ScriptMovementEffect {
                    command: "remove_fixed_facing".to_string(),
                    index: 5,
                },
            ]
        );
    }

    #[test]
    fn applymovement_uses_exact_steps_and_double_step_jump_commands() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(4, 4);
        session.player.facing = Direction::Down;
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("ExactStride".to_string());
        let movement = ScriptMovement {
            label: "ExactStride".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "slow_step".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "jump_step".to_string(),
                    direction: Some("LEFT".to_string()),
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
            apply_script_movement(&mut session, &command, &movement).expect("movement applies");

        assert_eq!(outcome.previous_tile, TilePosition::new(4, 4));
        assert_eq!(outcome.tile, TilePosition::new(3, 5));
        assert_eq!(session.player.tile, TilePosition::new(3, 5));
        assert_eq!(outcome.facing, Direction::Left);
        assert_eq!(outcome.steps_applied, 3);
        assert_eq!(
            outcome
                .executed_steps
                .iter()
                .map(|step| step.command.as_str())
                .collect::<Vec<_>>(),
            vec!["step", "slow_step", "jump_step"]
        );
    }

    #[test]
    fn dynamic_surf_step_uses_the_players_live_facing_direction() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(4, 4);
        session.player.facing = Direction::Right;
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("wMovementBuffer".to_string());
        let movement = ScriptMovement {
            label: "wMovementBuffer".to_string(),
            source_script: Some("UsedSurfScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "slow_step".to_string(),
                    direction: Some(SCRIPT_MOVEMENT_PLAYER_FACING_DIRECTION.to_string()),
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

        let outcome = apply_script_movement(&mut session, &command, &movement)
            .expect("dynamic Surf movement applies");

        assert_eq!(outcome.previous_tile, TilePosition::new(4, 4));
        assert_eq!(outcome.tile, TilePosition::new(5, 4));
        assert_eq!(session.player.tile, TilePosition::new(5, 4));
        assert_eq!(outcome.facing, Direction::Right);
        assert_eq!(outcome.steps_applied, 1);
    }

    #[test]
    fn applymovement_moves_objects_by_runtime_stride_and_saves_raw_event_coordinates() {
        let mut session = session(vec![object("ROUTE29_YOUNGSTER", "-1", 1, 1)]);
        let mut command = command("applymovement", "ROUTE29_YOUNGSTER");
        command.movement = Some("NpcRuntimeStride".to_string());
        let movement = ScriptMovement {
            label: "NpcRuntimeStride".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "jump_step".to_string(),
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

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("object moves");

        assert_eq!(outcome.previous_tile, TilePosition::new(1, 1));
        assert_eq!(outcome.tile, TilePosition::new(2, 3));
        assert_eq!((session.objects[0].x, session.objects[0].y), (1, 1));
        assert_eq!(
            session
                .object_runtime_tiles
                .get("ROUTE29_YOUNGSTER")
                .copied(),
            Some(TilePosition::new(2, 3))
        );
        assert_eq!(outcome.facing, Direction::Down);
    }

    #[test]
    fn applymovement_rejects_malformed_later_step_without_partial_movement() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(2, 2);
        session.player.facing = Direction::Down;
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("MalformedLaterStep".to_string());
        let movement = ScriptMovement {
            label: "MalformedLaterStep".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 1,
                },
            ],
        };

        let error = apply_script_movement(&mut session, &command, &movement)
            .expect_err("malformed movement rejects before mutation");

        assert_eq!(
            error,
            ScriptObjectCommandError::MovementUnexpectedDirection {
                movement: "MalformedLaterStep".to_string(),
                command: "step_end".to_string(),
                index: 1,
            }
        );
        assert_eq!(session.player.tile, TilePosition::new(2, 2));
        assert_eq!(session.player.facing, Direction::Down);
    }

    #[test]
    fn applymovement_accepts_odd_starting_runtime_tile_even_without_moving() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(1, 0);
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("TurnOnly".to_string());
        let movement = ScriptMovement {
            label: "TurnOnly".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "turn_head".to_string(),
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

        let outcome = apply_script_movement(&mut session, &command, &movement)
            .expect("odd player runtime tile is valid");

        assert_eq!(outcome.steps_applied, 1);
        assert_eq!(session.player.tile, TilePosition::new(1, 0));
        assert_eq!(session.player.facing, Direction::Up);
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
                    command: "teleport_to".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "skyfall_top".to_string(),
                    direction: None,
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "skyfall".to_string(),
                    direction: None,
                    duration: None,
                    index: 3,
                },
                ScriptMovementStep {
                    command: "step_dig".to_string(),
                    direction: None,
                    duration: Some(32),
                    index: 4,
                },
                ScriptMovementStep {
                    command: "fish_cast_rod".to_string(),
                    direction: None,
                    duration: None,
                    index: 5,
                },
                ScriptMovementStep {
                    command: "fish_got_bite".to_string(),
                    direction: None,
                    duration: None,
                    index: 6,
                },
                ScriptMovementStep {
                    command: "hide_emote".to_string(),
                    direction: None,
                    duration: None,
                    index: 7,
                },
                ScriptMovementStep {
                    command: "show_emote".to_string(),
                    direction: None,
                    duration: None,
                    index: 8,
                },
                ScriptMovementStep {
                    command: "step_shake".to_string(),
                    direction: None,
                    duration: Some(16),
                    index: 9,
                },
                ScriptMovementStep {
                    command: "tree_shake".to_string(),
                    direction: None,
                    duration: None,
                    index: 10,
                },
                ScriptMovementStep {
                    command: "rock_smash".to_string(),
                    direction: None,
                    duration: Some(10),
                    index: 11,
                },
                ScriptMovementStep {
                    command: "return_dig".to_string(),
                    direction: None,
                    duration: Some(32),
                    index: 12,
                },
                ScriptMovementStep {
                    command: "remove_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 13,
                },
                ScriptMovementStep {
                    command: "step_wait_end".to_string(),
                    direction: None,
                    duration: Some(4),
                    index: 14,
                },
                ScriptMovementStep {
                    command: "step_sleep".to_string(),
                    direction: None,
                    duration: Some(8),
                    index: 15,
                },
                ScriptMovementStep {
                    command: "step_stop".to_string(),
                    direction: None,
                    duration: None,
                    index: 16,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 17,
                },
            ],
        };

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("visuals apply");
        assert_eq!(outcome.tile, TilePosition::new(0, 0));
        assert_eq!(outcome.steps_applied, 248);
        assert_eq!(
            outcome
                .executed_steps
                .iter()
                .map(|step| (step.command.as_str(), step.duration))
                .collect::<Vec<_>>(),
            vec![
                ("teleport_from", None),
                ("teleport_to", None),
                ("skyfall_top", None),
                ("skyfall", None),
                ("step_dig", Some(32)),
                ("fish_cast_rod", None),
                ("fish_got_bite", None),
                ("hide_emote", None),
                ("show_emote", None),
                ("step_shake", Some(16)),
                ("tree_shake", None),
                ("rock_smash", Some(10)),
                ("return_dig", Some(32)),
                ("remove_object", None)
            ]
        );
        assert_eq!(
            outcome.effects,
            vec![
                ScriptMovementEffect {
                    command: "teleport_from".to_string(),
                    index: 0,
                },
                ScriptMovementEffect {
                    command: "teleport_to".to_string(),
                    index: 1,
                },
                ScriptMovementEffect {
                    command: "skyfall_top".to_string(),
                    index: 2,
                },
                ScriptMovementEffect {
                    command: "skyfall".to_string(),
                    index: 3,
                },
                ScriptMovementEffect {
                    command: "step_dig".to_string(),
                    index: 4,
                },
                ScriptMovementEffect {
                    command: "fish_cast_rod".to_string(),
                    index: 5,
                },
                ScriptMovementEffect {
                    command: "fish_got_bite".to_string(),
                    index: 6,
                },
                ScriptMovementEffect {
                    command: "hide_emote".to_string(),
                    index: 7,
                },
                ScriptMovementEffect {
                    command: "show_emote".to_string(),
                    index: 8,
                },
                ScriptMovementEffect {
                    command: "step_shake".to_string(),
                    index: 9,
                },
                ScriptMovementEffect {
                    command: "tree_shake".to_string(),
                    index: 10,
                },
                ScriptMovementEffect {
                    command: "rock_smash".to_string(),
                    index: 11,
                },
                ScriptMovementEffect {
                    command: "return_dig".to_string(),
                    index: 12,
                },
                ScriptMovementEffect {
                    command: "remove_object".to_string(),
                    index: 13,
                },
            ]
        );
    }

    #[test]
    fn applymovement_step_dig_and_return_dig_toggle_actor_visibility() {
        let mut session = session(Vec::new());
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("DigReturn".to_string());
        let movement = ScriptMovement {
            label: "DigReturn".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "step_dig".to_string(),
                    direction: None,
                    duration: Some(32),
                    index: 0,
                },
                ScriptMovementStep {
                    command: "return_dig".to_string(),
                    direction: None,
                    duration: Some(32),
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

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("dig movement applies");

        assert_eq!(
            outcome
                .effects
                .iter()
                .map(|effect| effect.command.as_str())
                .collect::<Vec<_>>(),
            vec!["step_dig", "return_dig"]
        );
        assert!(!session.player_hidden);
    }

    #[test]
    fn applymovement_rejects_duration_required_steps_without_runtime_default() {
        let mut session = session(Vec::new());
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("DigOut".to_string());
        let movement = ScriptMovement {
            label: "DigOut".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "step_dig".to_string(),
                direction: None,
                duration: None,
                index: 4,
            }],
        };

        assert_eq!(
            apply_script_movement(&mut session, &command, &movement),
            Err(ScriptObjectCommandError::MovementMissingDuration {
                movement: "DigOut".to_string(),
                command: "step_dig".to_string(),
                index: 4,
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
    }

    #[test]
    fn terminal_movement_effects_do_not_execute_following_bytes() {
        for (label, terminal) in [
            (
                "WaitEnd",
                ScriptMovementStep {
                    command: "step_wait_end".to_string(),
                    direction: None,
                    duration: Some(4),
                    index: 0,
                },
            ),
            (
                "Remove",
                ScriptMovementStep {
                    command: "remove_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
            ),
        ] {
            let mut session = session(Vec::new());
            let mut command = command("applymovement", "PLAYER");
            command.movement = Some(label.to_string());
            let movement = ScriptMovement {
                label: label.to_string(),
                source_script: None,
                steps: vec![
                    terminal,
                    ScriptMovementStep {
                        command: "step".to_string(),
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

            let outcome = apply_script_movement(&mut session, &command, &movement)
                .expect("terminal movement applies");
            assert_eq!(outcome.tile, TilePosition::new(0, 0));
            assert_eq!(outcome.executed_steps.len(), 1);
        }
    }

    #[test]
    fn applymovement_supports_slide_fast_bump_and_turn_away_opcodes() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(2, 2);
        session.player.facing = Direction::Down;
        let mut command = command("applymovement", "PLAYER");
        command.movement = Some("MoreCrystalMovement".to_string());
        let movement = ScriptMovement {
            label: "MoreCrystalMovement".to_string(),
            source_script: None,
            steps: vec![
                ScriptMovementStep {
                    command: "big_step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "slide_step".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "fast_slide_step".to_string(),
                    direction: Some("LEFT".to_string()),
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "slow_slide_step".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 3,
                },
                ScriptMovementStep {
                    command: "step_bump".to_string(),
                    direction: Some("LEFT".to_string()),
                    duration: None,
                    index: 4,
                },
                ScriptMovementStep {
                    command: "turn_away".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 5,
                },
                ScriptMovementStep {
                    command: "turn_in".to_string(),
                    direction: Some("LEFT".to_string()),
                    duration: None,
                    index: 6,
                },
                ScriptMovementStep {
                    command: "turn_waterfall".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 7,
                },
                ScriptMovementStep {
                    command: "hide_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 8,
                },
                ScriptMovementStep {
                    command: "show_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 9,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 10,
                },
            ],
        };

        let outcome = apply_script_movement(&mut session, &command, &movement)
            .expect("extended movement opcodes apply");

        assert_eq!(outcome.previous_tile, TilePosition::new(2, 2));
        assert_eq!(outcome.tile, TilePosition::new(1, 0));
        assert_eq!(outcome.facing, Direction::Up);
        assert_eq!(outcome.steps_applied, 10);
        assert_eq!(
            outcome.effects,
            vec![
                ScriptMovementEffect {
                    command: "hide_object".to_string(),
                    index: 8,
                },
                ScriptMovementEffect {
                    command: "show_object".to_string(),
                    index: 9,
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

    #[test]
    fn script_object_command_issues_validate_exact_object_payloads() {
        let object_event_flags = BTreeMap::from([
            ("NPC".to_string(), "EVENT_HIDE_NPC".to_string()),
            ("ROCK".to_string(), "-1".to_string()),
            ("STATUE".to_string(), "EVENT_STATIC_STATUE".to_string()),
        ]);
        let hideable_event_flags = BTreeSet::from(["EVENT_HIDE_NPC".to_string()]);
        let movements = BTreeSet::from([
            ("GlobalWalk".to_string(), Some("Script".to_string())),
            ("LocalWalk".to_string(), Some("SceneScript".to_string())),
        ]);

        let mut missing_visibility = command("appear", "NPC");
        missing_visibility.object_id = None;
        assert_eq!(
            script_object_command_issues(
                &missing_visibility,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::MissingObjectId {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "appear".to_string(),
            }]
        );

        let unhideable = command("disappear", "STATUE");
        assert_eq!(
            script_object_command_issues(
                &unhideable,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::UnhideableObject {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "disappear".to_string(),
                object_id: "STATUE".to_string(),
                event_flag: "EVENT_STATIC_STATUE".to_string(),
            }]
        );

        let mut turn = command("turnobject", "NPC");
        turn.direction = Some("sideways".to_string());
        assert!(
            script_object_command_issues(
                &turn,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            )
            .contains(&ScriptObjectCommandIssue::UnknownDirection {
                source_script: "Script".to_string(),
                command_index: 3,
                direction: "sideways".to_string(),
            })
        );

        let mut face = command("faceobject", "NPC");
        face.target_object_id = Some("MISSING".to_string());
        assert!(
            script_object_command_issues(
                &face,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            )
            .contains(&ScriptObjectCommandIssue::UnknownTargetObjectId {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "faceobject".to_string(),
                object_id: "MISSING".to_string(),
            })
        );

        face.target_object_id = Some("START PLAYER".to_string());
        assert!(
            script_object_command_issues(
                &face,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            )
            .contains(&ScriptObjectCommandIssue::InvalidTargetObjectId {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "faceobject".to_string(),
                object_id: "START PLAYER".to_string(),
            })
        );

        let mut local_movement = command("applymovement", "NPC");
        local_movement.source_script = ".branch@SceneScript".to_string();
        local_movement.movement = Some("LocalWalk".to_string());
        assert!(
            script_object_command_issues(
                &local_movement,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            )
            .is_empty()
        );

        let mut missing_movement = command("applymovement", "NPC");
        missing_movement.movement = Some("MissingWalk".to_string());
        assert_eq!(
            script_object_command_issues(
                &missing_movement,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::UnknownMovement {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "applymovement".to_string(),
                movement: "MissingWalk".to_string(),
            }]
        );

        missing_movement.movement = Some("Missing Walk".to_string());
        assert_eq!(
            script_object_command_issues(
                &missing_movement,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::InvalidMovement {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "applymovement".to_string(),
                movement: "Missing Walk".to_string(),
            }]
        );

        let malformed_object = command("disappear", "START RIVAL");
        assert_eq!(
            script_object_command_issues(
                &malformed_object,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::InvalidObjectId {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "disappear".to_string(),
                object_id: "START RIVAL".to_string(),
            }]
        );

        let unknown = command("spinobject", "NPC");
        assert_eq!(
            script_object_command_issues(
                &unknown,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::UnknownCommand {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "spinobject".to_string(),
            }]
        );

        let malformed_command = command("MoveObject", "NPC");
        assert_eq!(
            script_object_command_issues(
                &malformed_command,
                &object_event_flags,
                &hideable_event_flags,
                &movements,
            ),
            vec![ScriptObjectCommandIssue::InvalidCommand {
                source_script: "Script".to_string(),
                command_index: 3,
                command: "MoveObject".to_string(),
            }]
        );
    }
}
