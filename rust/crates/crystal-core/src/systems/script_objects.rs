use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::{EventFlagError, GameState};
use crate::systems::script_runtime::script_label_parent;
use crate::world::session::{OverworldFollowState, OverworldSession};
use crate::world::{
    map::{Direction, TilePosition},
    movement::move_by_stride,
};

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
            #[serde(default, deserialize_with = "required_script_object_command_token")]
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
        if !command.command.is_empty() {
            validate_script_object_command_shape(&command).map_err(D::Error::custom)?;
        }
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
            #[serde(default, deserialize_with = "required_script_object_command_token")]
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
        if !step.command.is_empty()
            && let Some(issue) = script_movement_step_issues(&step).into_iter().next()
        {
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
    pub tile: TilePosition,
    pub facing: Direction,
    pub effects: Vec<ScriptMovementEffect>,
    pub fixed_facing: bool,
    pub sliding: bool,
    pub steps_applied: usize,
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
    #[error("script object command '{command}' is missing a direction")]
    MissingDirection { command: String },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptMovementStepIssue {
    UnexpectedDirection,
    MissingDirection,
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
    UnknownCommand {
        source_script: String,
        command_index: usize,
        command: String,
    },
}

pub const SCRIPT_OBJECT_VISIBILITY_COMMANDS: &[&str] = &["appear", "disappear"];
pub const SCRIPT_OBJECT_COORDINATE_COMMANDS: &[&str] = &["moveobject"];
pub const SCRIPT_OBJECT_DIRECTION_COMMANDS: &[&str] = &["turnobject"];
pub const SCRIPT_OBJECT_TARGET_COMMANDS: &[&str] = &["faceobject", "follow"];
pub const SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS: &[&str] = &["applymovement"];
pub const SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS: &[&str] = &["applymovementlasttalked"];
pub const SCRIPT_OBJECT_MOVEMENT_COMMANDS: &[&str] = &["applymovement", "applymovementlasttalked"];
pub const SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS: &[&str] = &["faceplayer", "stopfollow"];
pub const SCRIPT_OBJECT_EMOTE_COMMANDS: &[&str] = &["showemote"];

pub const SCRIPT_MOVEMENT_DIRECTION_COMMANDS: &[&str] = &[
    "step",
    "slow_step",
    "fast_step",
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
];
pub const SCRIPT_MOVEMENT_OPTIONAL_DURATION_COMMANDS: &[&str] = &["step_sleep"];
pub const SCRIPT_MOVEMENT_NO_ARG_COMMANDS: &[&str] = &[
    "step_end",
    "fix_facing",
    "remove_fixed_facing",
    "set_sliding",
    "remove_sliding",
    "teleport_from",
    "teleport_to",
    "skyfall_top",
    "tree_shake",
    "hide_object",
    "show_object",
];
pub const SCRIPT_MOVEMENT_COMMANDS: &[&str] = &[
    "step",
    "slow_step",
    "fast_step",
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
    "step_sleep",
    "step_end",
    "fix_facing",
    "remove_fixed_facing",
    "set_sliding",
    "remove_sliding",
    "teleport_from",
    "teleport_to",
    "skyfall_top",
    "tree_shake",
    "hide_object",
    "show_object",
];

pub fn is_known_script_object_command(command: &str) -> bool {
    SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command)
        || SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command)
}

pub fn is_known_script_movement_command(command: &str) -> bool {
    SCRIPT_MOVEMENT_COMMANDS.contains(&command)
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
        }
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
        if !movements.contains(&(movement.to_string(), None))
            && !movements.contains(&(movement.to_string(), Some(movement_source.to_string())))
        {
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
    if SCRIPT_MOVEMENT_NO_ARG_COMMANDS.contains(&command)
        || SCRIPT_MOVEMENT_OPTIONAL_DURATION_COMMANDS.contains(&command)
    {
        if step.direction.is_some() {
            vec![ScriptMovementStepIssue::UnexpectedDirection]
        } else {
            Vec::new()
        }
    } else if SCRIPT_MOVEMENT_DIRECTION_COMMANDS.contains(&command) {
        match step.direction.as_deref() {
            Some(direction) if parse_script_direction(direction).is_ok() => Vec::new(),
            Some(direction) => vec![ScriptMovementStepIssue::UnknownDirection {
                direction: direction.to_string(),
            }],
            None => vec![ScriptMovementStepIssue::MissingDirection],
        }
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
            command if movement_step_moves_object(command) => {
                let direction = movement_step_direction(movement, step)?;
                if !fixed_facing {
                    facing = direction;
                }
                tile = move_by_stride(tile, direction, 1);
                steps_applied += 1;
            }
            command if movement_step_turns_without_moving(command) => {
                let direction = movement_step_direction(movement, step)?;
                facing = match command {
                    "turn_away" => opposite_direction(direction),
                    _ => direction,
                };
                steps_applied += 1;
            }
            "teleport_from" | "teleport_to" | "skyfall_top" | "tree_shake" | "hide_object"
            | "show_object" => {
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

fn movement_step_moves_object(command: &str) -> bool {
    matches!(
        command,
        "step"
            | "slow_step"
            | "fast_step"
            | "big_step"
            | "turn_step"
            | "jump_step"
            | "fast_jump_step"
            | "slow_jump_step"
            | "slide_step"
            | "fast_slide_step"
            | "slow_slide_step"
    )
}

fn movement_step_turns_without_moving(command: &str) -> bool {
    matches!(command, "turn_head" | "turn_away" | "step_bump")
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
    if object_id != "LAST_TALKED"
        && object_id != "PLAYER"
        && !is_exact_script_object_token(object_id)
    {
        return Err(ScriptObjectCommandError::InvalidObjectId {
            object_id: object_id.to_string(),
        });
    }
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
    fn exported_object_command_sets_are_exact() {
        assert!(SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&"appear"));
        assert!(SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&"disappear"));
        assert!(SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&"moveobject"));
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
        assert!(!is_known_script_object_command("MoveObject"));
        assert!(!is_known_script_object_command("hideobject"));
        assert_eq!(
            SCRIPT_MOVEMENT_DIRECTION_COMMANDS,
            &[
                "step",
                "slow_step",
                "fast_step",
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
                "turn_away"
            ]
        );
        assert_eq!(SCRIPT_MOVEMENT_OPTIONAL_DURATION_COMMANDS, &["step_sleep"]);
        assert_eq!(
            SCRIPT_MOVEMENT_NO_ARG_COMMANDS,
            &[
                "step_end",
                "fix_facing",
                "remove_fixed_facing",
                "set_sliding",
                "remove_sliding",
                "teleport_from",
                "skyfall_top",
                "tree_shake",
                "hide_object",
                "show_object"
            ]
        );
        assert!(is_known_script_movement_command("turn_head"));
        assert!(is_known_script_movement_command("fast_slide_step"));
        assert!(is_known_script_movement_command("hide_object"));
        assert!(!is_known_script_movement_command("spin_forever"));
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
                command: "turn_head".to_string(),
                direction: Some("LEFT".to_string()),
                duration: None,
                index: 4,
            }),
            Vec::<ScriptMovementStepIssue>::new()
        );
    }

    #[test]
    fn object_and_movement_commands_reject_reserved_pack_prefixes() {
        let object_event_flags =
            BTreeMap::from([("NPC".to_string(), "EVENT_HIDE_NPC".to_string())]);
        let hideable_event_flags = BTreeSet::from(["EVENT_HIDE_NPC".to_string()]);
        let movements = BTreeSet::from([("Walk".to_string(), None)]);

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
    fn applymovement_rejects_malformed_matching_label_before_mutating_session() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(1, 1);
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
        assert_eq!(session.player.tile, TilePosition::new(1, 1));
    }

    #[test]
    fn applymovement_rejects_invalid_source_before_mutating_session() {
        let mut session = session(Vec::new());
        session.player.tile = TilePosition::new(1, 1);
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
        assert_eq!(session.player.tile, TilePosition::new(1, 1));

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
        assert_eq!(session.player.tile, TilePosition::new(1, 1));
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
                    command: "tree_shake".to_string(),
                    direction: None,
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
                    command: "teleport_to".to_string(),
                    index: 1,
                },
                ScriptMovementEffect {
                    command: "skyfall_top".to_string(),
                    index: 2,
                },
                ScriptMovementEffect {
                    command: "tree_shake".to_string(),
                    index: 3,
                },
            ]
        );
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
                    command: "fast_step".to_string(),
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
                    command: "hide_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 6,
                },
                ScriptMovementStep {
                    command: "show_object".to_string(),
                    direction: None,
                    duration: None,
                    index: 7,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 8,
                },
            ],
        };

        let outcome = apply_script_movement(&mut session, &command, &movement)
            .expect("extended movement opcodes apply");

        assert_eq!(outcome.previous_tile, TilePosition::new(2, 2));
        assert_eq!(outcome.tile, TilePosition::new(2, 2));
        assert_eq!(outcome.facing, Direction::Down);
        assert_eq!(outcome.steps_applied, 8);
        assert_eq!(
            outcome.effects,
            vec![
                ScriptMovementEffect {
                    command: "hide_object".to_string(),
                    index: 6,
                },
                ScriptMovementEffect {
                    command: "show_object".to_string(),
                    index: 7,
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
            ("GlobalWalk".to_string(), None),
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
