use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::{
    GameState, ScriptMapLoadRequest, ScriptMapRefreshRequest, ScriptMapRuntimeEvent,
    ScriptMapRuntimeKind, ScriptWarpRequest,
};
use crate::world::map::{Direction, TilePosition};
use crate::world::movement::PlayerMovementState;
use crate::world::session::raw_event_tile_to_runtime_tile_checked;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapCommand {
    #[serde(deserialize_with = "required_script_map_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_script_map_token")]
    pub target_map: Option<String>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    #[serde(deserialize_with = "required_nullable_script_map_token")]
    pub facing: Option<String>,
    #[serde(deserialize_with = "required_nullable_script_map_token")]
    pub map_setup: Option<String>,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptMapCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptMapCommand {
            #[serde(deserialize_with = "required_script_map_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_script_map_token")]
            target_map: Option<String>,
            x: Option<u16>,
            y: Option<u16>,
            #[serde(deserialize_with = "required_nullable_script_map_token")]
            facing: Option<String>,
            #[serde(deserialize_with = "required_nullable_script_map_token")]
            map_setup: Option<String>,
            #[serde(deserialize_with = "required_script_label_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptMapCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            target_map: raw.target_map,
            x: raw.x,
            y: raw.y,
            facing: raw.facing,
            map_setup: raw.map_setup,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_map_command_shape(&command).map_err(D::Error::custom)?;
        Ok(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptMapAction {
    Warp {
        target_map: String,
        tile: TilePosition,
        facing: Option<Direction>,
        source_script: String,
        command_index: usize,
    },
    WarpCheck {
        source_script: String,
        command_index: usize,
    },
    BattleWhiteout {
        source_script: String,
        command_index: usize,
    },
    LoadMap {
        command: String,
        map_setup: Option<String>,
        source_script: String,
        command_index: usize,
    },
    RefreshMap {
        command: String,
        map_setup: Option<String>,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptMapCommandError {
    #[error("script map command '{command}' is not exact pack syntax")]
    InvalidCommand { command: String },
    #[error("script map command source script '{source_script}' is invalid")]
    InvalidSourceScript { source_script: String },
    #[error("unknown script map command '{command}'")]
    UnknownCommand { command: String },
    #[error("script map command '{command}' is missing a target map")]
    MissingTargetMap { command: String },
    #[error("script map command '{command}' has invalid target map '{target_map}'")]
    InvalidTargetMap { command: String, target_map: String },
    #[error("script map command '{command}' references unknown map '{target_map}'")]
    UnknownTargetMap { command: String, target_map: String },
    #[error("script map command '{command}' has malformed bad-warp sentinel")]
    MalformedBadWarpSentinel { command: String },
    #[error("script map command '{command}' is missing warp coordinates")]
    MissingCoordinates { command: String },
    #[error("script map command '{command}' has out-of-range warp coordinates")]
    CoordinatesOutOfRange { command: String },
    #[error("script map command '{command}' has unexpected target map or coordinates")]
    UnexpectedWarpDestination { command: String },
    #[error("script map command '{command}' is missing a facing direction")]
    MissingFacing { command: String },
    #[error("script map command '{command}' has unexpected facing direction")]
    UnexpectedFacing { command: String },
    #[error("invalid script facing direction '{facing}'")]
    InvalidFacing { facing: String },
    #[error("unknown script facing direction '{facing}'")]
    UnknownFacing { facing: String },
    #[error("script map command '{command}' is missing a map setup")]
    MissingMapSetup { command: String },
    #[error("script map command '{command}' has invalid map setup '{map_setup}'")]
    InvalidMapSetup { command: String, map_setup: String },
    #[error("script map command '{command}' has unexpected map setup")]
    UnexpectedMapSetup { command: String },
    #[error("cannot complete script warp without a pending script warp")]
    MissingPendingScriptWarp,
    #[error("completed script warp does not match pending script warp")]
    PendingScriptWarpMismatch,
}

pub const SCRIPT_MAP_WARP_COMMANDS: &[&str] = &["warp"];
pub const SCRIPT_MAP_FACING_WARP_COMMANDS: &[&str] = &["warpfacing"];
pub const SCRIPT_MAP_WARP_CHECK_COMMANDS: &[&str] = &["warpcheck"];
pub const SCRIPT_MAP_NEW_LOAD_COMMANDS: &[&str] = &["newloadmap"];
pub const SCRIPT_MAP_RELOAD_COMMANDS: &[&str] = &["reloadmap", "reloadmapafterbattle"];
pub const SCRIPT_MAP_LOAD_COMMANDS: &[&str] = &["newloadmap", "reloadmap", "reloadmapafterbattle"];
pub const SCRIPT_MAP_SIMPLE_REFRESH_COMMANDS: &[&str] = &["refreshmap"];
pub const SCRIPT_MAP_REANCHOR_COMMANDS: &[&str] = &["reanchormap"];
pub const SCRIPT_MAP_REFRESH_COMMANDS: &[&str] = &["refreshmap", "reanchormap"];
pub const SCRIPT_MAP_NO_PAYLOAD_COMMANDS: &[&str] = &[
    "warpcheck",
    "reloadmap",
    "reloadmapafterbattle",
    "refreshmap",
];

pub const MAP_CALLBACK_NEWMAP: &str = "MAPCALLBACK_NEWMAP";
pub const MAP_CALLBACK_TILES: &str = "MAPCALLBACK_TILES";
pub const MAP_CALLBACK_OBJECTS: &str = "MAPCALLBACK_OBJECTS";
pub const MAP_CALLBACK_CMDQUEUE: &str = "MAPCALLBACK_CMDQUEUE";
pub const MAP_CALLBACK_SPRITES: &str = "MAPCALLBACK_SPRITES";

const MAP_SETUP_WARP_CALLBACKS: &[&str] = &[
    MAP_CALLBACK_NEWMAP,
    MAP_CALLBACK_CMDQUEUE,
    MAP_CALLBACK_TILES,
    MAP_CALLBACK_SPRITES,
    MAP_CALLBACK_OBJECTS,
];
const MAP_SETUP_CONNECTION_CALLBACKS: &[&str] = &[
    MAP_CALLBACK_NEWMAP,
    MAP_CALLBACK_CMDQUEUE,
    MAP_CALLBACK_TILES,
    MAP_CALLBACK_OBJECTS,
];
const MAP_SETUP_RELOAD_CALLBACKS: &[&str] = &[MAP_CALLBACK_TILES, MAP_CALLBACK_SPRITES];
const MAP_SETUP_LINK_RETURN_CALLBACKS: &[&str] = &[
    MAP_CALLBACK_NEWMAP,
    MAP_CALLBACK_CMDQUEUE,
    MAP_CALLBACK_TILES,
    MAP_CALLBACK_SPRITES,
];
const MAP_SETUP_CONTINUE_CALLBACKS: &[&str] = &[
    MAP_CALLBACK_CMDQUEUE,
    MAP_CALLBACK_TILES,
    MAP_CALLBACK_SPRITES,
];
const MAP_SETUP_SUBMENU_CALLBACKS: &[&str] = &[MAP_CALLBACK_TILES];

/// Map callbacks reached by each exact `MapSetupScript_*`, in execution order.
pub fn map_setup_callback_kinds(map_setup: &str) -> Option<&'static [&'static str]> {
    match map_setup {
        "MAPSETUP_WARP" | "MAPSETUP_TELEPORT" | "MAPSETUP_DOOR" | "MAPSETUP_FALL"
        | "MAPSETUP_TRAIN" | "MAPSETUP_BADWARP" | "MAPSETUP_FLY" => Some(MAP_SETUP_WARP_CALLBACKS),
        "MAPSETUP_CONNECTION" => Some(MAP_SETUP_CONNECTION_CALLBACKS),
        "MAPSETUP_RELOADMAP" => Some(MAP_SETUP_RELOAD_CALLBACKS),
        "MAPSETUP_LINKRETURN" => Some(MAP_SETUP_LINK_RETURN_CALLBACKS),
        "MAPSETUP_CONTINUE" => Some(MAP_SETUP_CONTINUE_CALLBACKS),
        "MAPSETUP_SUBMENU" => Some(MAP_SETUP_SUBMENU_CALLBACKS),
        _ => None,
    }
}

pub fn is_known_script_map_command(command: &str) -> bool {
    SCRIPT_MAP_WARP_COMMANDS.contains(&command)
        || SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command)
        || SCRIPT_MAP_WARP_CHECK_COMMANDS.contains(&command)
        || SCRIPT_MAP_LOAD_COMMANDS.contains(&command)
        || SCRIPT_MAP_REFRESH_COMMANDS.contains(&command)
}

fn validate_script_map_command_shape(command: &ScriptMapCommand) -> Result<(), String> {
    if !is_known_script_map_command(&command.command) {
        return Err(format!("unknown script map command {}", command.command));
    }
    match command.command.as_str() {
        "warp" => {
            if is_bad_warp_sentinel(command) {
                reject_facing(command).map_err(|error| error.to_string())?;
                reject_map_setup(command).map_err(|error| error.to_string())?;
                return Ok(());
            }
            require_warp_destination_shape(command)?;
            reject_facing(command).map_err(|error| error.to_string())?;
            reject_map_setup(command).map_err(|error| error.to_string())?;
        }
        "warpfacing" => {
            require_warp_destination_shape(command)?;
            let facing = command.facing.as_deref().ok_or_else(|| {
                ScriptMapCommandError::MissingFacing {
                    command: command.command.clone(),
                }
                .to_string()
            })?;
            parse_script_warp_facing(facing).map_err(|error| error.to_string())?;
            reject_map_setup(command).map_err(|error| error.to_string())?;
        }
        "warpcheck" | "reloadmap" | "reloadmapafterbattle" | "refreshmap" => {
            reject_warp_destination(command).map_err(|error| error.to_string())?;
            reject_facing(command).map_err(|error| error.to_string())?;
            reject_map_setup(command).map_err(|error| error.to_string())?;
        }
        "newloadmap" => {
            reject_warp_destination(command).map_err(|error| error.to_string())?;
            reject_facing(command).map_err(|error| error.to_string())?;
            require_map_setup(command).map_err(|error| error.to_string())?;
        }
        "reanchormap" => {
            reject_warp_destination(command).map_err(|error| error.to_string())?;
            reject_facing(command).map_err(|error| error.to_string())?;
            if let Some(map_setup) = command
                .map_setup
                .as_deref()
                .filter(|map_setup| !is_exact_nonempty_token(map_setup))
            {
                return Err(format!(
                    "script map command reanchormap has invalid map setup {map_setup}"
                ));
            }
        }
        _ => unreachable!("known script map command was not handled"),
    }
    Ok(())
}

fn require_warp_destination_shape(command: &ScriptMapCommand) -> Result<(), String> {
    let Some(target_map) = command.target_map.as_deref() else {
        return Err(ScriptMapCommandError::MissingTargetMap {
            command: command.command.clone(),
        }
        .to_string());
    };
    if target_map == "NONE" {
        return Err(ScriptMapCommandError::MalformedBadWarpSentinel {
            command: command.command.clone(),
        }
        .to_string());
    }
    if command.x.is_none() || command.y.is_none() {
        return Err(ScriptMapCommandError::MissingCoordinates {
            command: command.command.clone(),
        }
        .to_string());
    }
    if command_runtime_tile(command).is_err() {
        return Err(ScriptMapCommandError::CoordinatesOutOfRange {
            command: command.command.clone(),
        }
        .to_string());
    }
    Ok(())
}

pub fn script_map_command_issues(
    command: &ScriptMapCommand,
    map_ids: &BTreeSet<String>,
) -> Vec<ScriptMapCommandError> {
    let mut issues = Vec::new();
    if !is_exact_script_label_token(&command.source_script) {
        issues.push(ScriptMapCommandError::InvalidSourceScript {
            source_script: command.source_script.clone(),
        });
    }
    if !is_exact_script_map_command_token(&command.command) {
        issues.push(ScriptMapCommandError::InvalidCommand {
            command: command.command.clone(),
        });
    } else if SCRIPT_MAP_WARP_COMMANDS.contains(&command.command.as_str()) {
        push_warp_destination_issues(command, map_ids, &mut issues);
        push_unexpected_facing(command, &mut issues);
        push_unexpected_map_setup(command, &mut issues);
    } else if SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command.command.as_str()) {
        push_warp_destination_issues(command, map_ids, &mut issues);
        match command.facing.as_deref() {
            Some(facing) if parse_script_warp_facing(facing).is_ok() => {}
            Some(facing) if !is_exact_nonempty_token(facing) => {
                issues.push(ScriptMapCommandError::InvalidFacing {
                    facing: facing.to_string(),
                });
            }
            Some(facing) => issues.push(ScriptMapCommandError::UnknownFacing {
                facing: facing.to_string(),
            }),
            None => issues.push(ScriptMapCommandError::MissingFacing {
                command: command.command.clone(),
            }),
        }
        push_unexpected_map_setup(command, &mut issues);
    } else if SCRIPT_MAP_NO_PAYLOAD_COMMANDS.contains(&command.command.as_str()) {
        push_unexpected_warp_destination(command, &mut issues);
        push_unexpected_facing(command, &mut issues);
        push_unexpected_map_setup(command, &mut issues);
    } else if SCRIPT_MAP_REANCHOR_COMMANDS.contains(&command.command.as_str()) {
        push_unexpected_warp_destination(command, &mut issues);
        push_unexpected_facing(command, &mut issues);
        push_invalid_map_setup(command, &mut issues);
    } else if SCRIPT_MAP_NEW_LOAD_COMMANDS.contains(&command.command.as_str()) {
        push_unexpected_warp_destination(command, &mut issues);
        push_unexpected_facing(command, &mut issues);
        if command.map_setup.is_none() {
            issues.push(ScriptMapCommandError::MissingMapSetup {
                command: command.command.clone(),
            });
        } else {
            push_invalid_map_setup(command, &mut issues);
        }
    } else {
        issues.push(ScriptMapCommandError::UnknownCommand {
            command: command.command.clone(),
        });
    }
    issues
}

pub fn resolve_script_map_command(
    command: ScriptMapCommand,
    map_ids: &BTreeSet<String>,
) -> Result<ScriptMapAction, ScriptMapCommandError> {
    if !is_exact_script_label_token(&command.source_script) {
        return Err(ScriptMapCommandError::InvalidSourceScript {
            source_script: command.source_script,
        });
    }
    if !is_exact_script_map_command_token(&command.command) {
        return Err(ScriptMapCommandError::InvalidCommand {
            command: command.command,
        });
    }
    match command.command.as_str() {
        "warp" => {
            reject_facing(&command)?;
            reject_map_setup(&command)?;
            if is_bad_warp_sentinel(&command) {
                return Ok(ScriptMapAction::LoadMap {
                    command: command.command,
                    map_setup: Some("MAPSETUP_BADWARP".to_string()),
                    source_script: command.source_script,
                    command_index: command.command_index,
                });
            }
            let target_map = require_known_target_map(&command, map_ids)?.to_string();
            let tile = require_tile(&command)?;
            Ok(ScriptMapAction::Warp {
                target_map,
                tile,
                facing: None,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "warpfacing" => {
            reject_map_setup(&command)?;
            let target_map = require_known_target_map(&command, map_ids)?.to_string();
            let tile = require_tile(&command)?;
            let facing = parse_script_warp_facing(command.facing.as_deref().ok_or_else(|| {
                ScriptMapCommandError::MissingFacing {
                    command: command.command.clone(),
                }
            })?)?;
            Ok(ScriptMapAction::Warp {
                target_map,
                tile,
                facing: Some(facing),
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "warpcheck" => {
            reject_warp_destination(&command)?;
            reject_facing(&command)?;
            reject_map_setup(&command)?;
            Ok(ScriptMapAction::WarpCheck {
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "newloadmap" => {
            reject_warp_destination(&command)?;
            reject_facing(&command)?;
            let map_setup = require_map_setup(&command)?;
            Ok(ScriptMapAction::LoadMap {
                command: command.command,
                map_setup: Some(map_setup),
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "reloadmap" | "reloadmapafterbattle" => {
            reject_warp_destination(&command)?;
            reject_facing(&command)?;
            reject_map_setup(&command)?;
            Ok(ScriptMapAction::LoadMap {
                command: command.command,
                map_setup: None,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "refreshmap" => {
            reject_warp_destination(&command)?;
            reject_facing(&command)?;
            reject_map_setup(&command)?;
            Ok(ScriptMapAction::RefreshMap {
                command: command.command,
                map_setup: None,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "reanchormap" => {
            reject_warp_destination(&command)?;
            reject_facing(&command)?;
            if let Some(map_setup) = command
                .map_setup
                .as_deref()
                .filter(|map_setup| !is_exact_nonempty_token(map_setup))
            {
                return Err(ScriptMapCommandError::InvalidMapSetup {
                    command: command.command.clone(),
                    map_setup: map_setup.to_string(),
                });
            }
            Ok(ScriptMapAction::RefreshMap {
                command: command.command,
                map_setup: command.map_setup,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        other => Err(ScriptMapCommandError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn apply_script_map_command(
    state: &mut GameState,
    command: ScriptMapCommand,
    map_ids: &BTreeSet<String>,
) -> Result<ScriptMapAction, ScriptMapCommandError> {
    let action = resolve_script_map_command(command, map_ids)?;
    apply_script_map_action_to_state(state, &action);
    Ok(action)
}

pub fn complete_pending_script_warp(
    state: &mut GameState,
    request: &ScriptWarpRequest,
) -> Result<ScriptWarpRequest, ScriptMapCommandError> {
    let pending = state
        .script_runtime
        .pending_script_warp
        .as_ref()
        .ok_or(ScriptMapCommandError::MissingPendingScriptWarp)?;
    if pending != request {
        return Err(ScriptMapCommandError::PendingScriptWarpMismatch);
    }
    state
        .script_runtime
        .pending_script_warp
        .take()
        .ok_or(ScriptMapCommandError::MissingPendingScriptWarp)
}

pub fn apply_script_warp_arrival_facing(
    player: &mut PlayerMovementState,
    request: &ScriptWarpRequest,
) -> Option<Direction> {
    let facing = request.facing?;
    player.facing = facing;
    Some(facing)
}

pub fn apply_script_map_action_to_state(state: &mut GameState, action: &ScriptMapAction) {
    match action {
        ScriptMapAction::LoadMap {
            command,
            map_setup,
            source_script,
            command_index,
        } => {
            if command != "newloadmap" {
                state.script_runtime.pending_script_warp = None;
            }
            state.script_runtime.pending_map_load = Some(ScriptMapLoadRequest {
                command: command.clone(),
                map_setup: map_setup.clone(),
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
                command: command.clone(),
                kind: ScriptMapRuntimeKind::LoadMap,
                target_map: None,
                tile: None,
                facing: None,
                map_setup: map_setup.clone(),
                source_script: source_script.clone(),
                command_index: *command_index,
            });
        }
        ScriptMapAction::Warp {
            target_map,
            tile,
            facing,
            source_script,
            command_index,
        } => {
            state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
                target_map: target_map.clone(),
                tile: *tile,
                facing: *facing,
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
                command: if facing.is_some() {
                    "warpfacing".to_string()
                } else {
                    "warp".to_string()
                },
                kind: ScriptMapRuntimeKind::Warp,
                target_map: Some(target_map.clone()),
                tile: Some(*tile),
                facing: *facing,
                map_setup: None,
                source_script: source_script.clone(),
                command_index: *command_index,
            });
        }
        ScriptMapAction::WarpCheck {
            source_script,
            command_index,
        } => {
            state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
                command: "warpcheck".to_string(),
                kind: ScriptMapRuntimeKind::WarpCheck,
                target_map: None,
                tile: None,
                facing: None,
                map_setup: None,
                source_script: source_script.clone(),
                command_index: *command_index,
            });
        }
        ScriptMapAction::BattleWhiteout { .. } => {}
        ScriptMapAction::RefreshMap {
            command,
            map_setup,
            source_script,
            command_index,
        } => {
            state.script_runtime.pending_map_refresh = Some(ScriptMapRefreshRequest {
                command: command.clone(),
                map_setup: map_setup.clone(),
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
                command: command.clone(),
                kind: ScriptMapRuntimeKind::RefreshMap,
                target_map: None,
                tile: None,
                facing: None,
                map_setup: map_setup.clone(),
                source_script: source_script.clone(),
                command_index: *command_index,
            });
        }
    }
}

pub fn parse_script_warp_facing(facing: &str) -> Result<Direction, ScriptMapCommandError> {
    if !is_exact_nonempty_token(facing) {
        return Err(ScriptMapCommandError::InvalidFacing {
            facing: facing.to_string(),
        });
    }
    match facing {
        "DOWN" => Ok(Direction::Down),
        "UP" => Ok(Direction::Up),
        "LEFT" => Ok(Direction::Left),
        "RIGHT" => Ok(Direction::Right),
        other => Err(ScriptMapCommandError::UnknownFacing {
            facing: other.to_string(),
        }),
    }
}

fn require_known_target_map<'a>(
    command: &'a ScriptMapCommand,
    map_ids: &BTreeSet<String>,
) -> Result<&'a str, ScriptMapCommandError> {
    let target_map =
        command
            .target_map
            .as_deref()
            .ok_or_else(|| ScriptMapCommandError::MissingTargetMap {
                command: command.command.clone(),
            })?;
    if !is_exact_nonempty_token(target_map) {
        return Err(ScriptMapCommandError::InvalidTargetMap {
            command: command.command.clone(),
            target_map: target_map.to_string(),
        });
    }
    if target_map == "NONE" {
        return Err(ScriptMapCommandError::MalformedBadWarpSentinel {
            command: command.command.clone(),
        });
    }
    if !map_ids.contains(target_map) {
        return Err(ScriptMapCommandError::UnknownTargetMap {
            command: command.command.clone(),
            target_map: target_map.to_string(),
        });
    }
    Ok(target_map)
}

fn is_bad_warp_sentinel(command: &ScriptMapCommand) -> bool {
    command.target_map.as_deref() == Some("NONE") && command.x == Some(0) && command.y == Some(0)
}

fn require_tile(command: &ScriptMapCommand) -> Result<TilePosition, ScriptMapCommandError> {
    command_runtime_tile(command)
}

fn command_runtime_tile(command: &ScriptMapCommand) -> Result<TilePosition, ScriptMapCommandError> {
    let (Some(x), Some(y)) = (command.x, command.y) else {
        return Err(ScriptMapCommandError::MissingCoordinates {
            command: command.command.clone(),
        });
    };
    raw_event_tile_to_runtime_tile_checked(x, y).ok_or_else(|| {
        ScriptMapCommandError::CoordinatesOutOfRange {
            command: command.command.clone(),
        }
    })
}

fn reject_warp_destination(command: &ScriptMapCommand) -> Result<(), ScriptMapCommandError> {
    if command.target_map.is_some() || command.x.is_some() || command.y.is_some() {
        Err(ScriptMapCommandError::UnexpectedWarpDestination {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn reject_facing(command: &ScriptMapCommand) -> Result<(), ScriptMapCommandError> {
    if command.facing.is_some() {
        Err(ScriptMapCommandError::UnexpectedFacing {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn reject_map_setup(command: &ScriptMapCommand) -> Result<(), ScriptMapCommandError> {
    if command.map_setup.is_some() {
        Err(ScriptMapCommandError::UnexpectedMapSetup {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn push_warp_destination_issues(
    command: &ScriptMapCommand,
    map_ids: &BTreeSet<String>,
    issues: &mut Vec<ScriptMapCommandError>,
) {
    match command.target_map.as_deref() {
        Some("NONE") if is_bad_warp_sentinel(command) => {}
        Some("NONE") => issues.push(ScriptMapCommandError::MalformedBadWarpSentinel {
            command: command.command.clone(),
        }),
        Some(target_map) if !is_exact_nonempty_token(target_map) => {
            issues.push(ScriptMapCommandError::InvalidTargetMap {
                command: command.command.clone(),
                target_map: target_map.to_string(),
            });
        }
        Some(target_map) if map_ids.contains(target_map) => {}
        Some(target_map) => issues.push(ScriptMapCommandError::UnknownTargetMap {
            command: command.command.clone(),
            target_map: target_map.to_string(),
        }),
        None => issues.push(ScriptMapCommandError::MissingTargetMap {
            command: command.command.clone(),
        }),
    }
    if command.x.is_none() || command.y.is_none() {
        issues.push(ScriptMapCommandError::MissingCoordinates {
            command: command.command.clone(),
        });
    } else if command_runtime_tile(command).is_err() {
        issues.push(ScriptMapCommandError::CoordinatesOutOfRange {
            command: command.command.clone(),
        });
    }
}

fn push_unexpected_warp_destination(
    command: &ScriptMapCommand,
    issues: &mut Vec<ScriptMapCommandError>,
) {
    if command.target_map.is_some() || command.x.is_some() || command.y.is_some() {
        issues.push(ScriptMapCommandError::UnexpectedWarpDestination {
            command: command.command.clone(),
        });
    }
}

fn push_unexpected_facing(command: &ScriptMapCommand, issues: &mut Vec<ScriptMapCommandError>) {
    if command.facing.is_some() {
        issues.push(ScriptMapCommandError::UnexpectedFacing {
            command: command.command.clone(),
        });
    }
}

fn push_unexpected_map_setup(command: &ScriptMapCommand, issues: &mut Vec<ScriptMapCommandError>) {
    if command.map_setup.is_some() {
        issues.push(ScriptMapCommandError::UnexpectedMapSetup {
            command: command.command.clone(),
        });
    }
}

fn push_invalid_map_setup(command: &ScriptMapCommand, issues: &mut Vec<ScriptMapCommandError>) {
    if let Some(map_setup) = command
        .map_setup
        .as_deref()
        .filter(|map_setup| !is_exact_nonempty_token(map_setup))
    {
        issues.push(ScriptMapCommandError::InvalidMapSetup {
            command: command.command.clone(),
            map_setup: map_setup.to_string(),
        });
    }
}

fn require_map_setup(command: &ScriptMapCommand) -> Result<String, ScriptMapCommandError> {
    let map_setup =
        command
            .map_setup
            .clone()
            .ok_or_else(|| ScriptMapCommandError::MissingMapSetup {
                command: command.command.clone(),
            })?;
    if !is_exact_nonempty_token(&map_setup) {
        return Err(ScriptMapCommandError::InvalidMapSetup {
            command: command.command.clone(),
            map_setup,
        });
    }
    Ok(map_setup)
}

fn is_exact_nonempty_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'))
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_map_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !has_reserved_pack_prefix(value)
}

fn required_script_map_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_map_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script map command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_nullable_script_map_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_nonempty_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script map token must be exact ASCII alphanumeric/underscore, found {token:?}"
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

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_setup_callbacks_follow_the_asm_command_paths() {
        assert_eq!(
            map_setup_callback_kinds("MAPSETUP_WARP"),
            Some(
                [
                    MAP_CALLBACK_NEWMAP,
                    MAP_CALLBACK_CMDQUEUE,
                    MAP_CALLBACK_TILES,
                    MAP_CALLBACK_SPRITES,
                    MAP_CALLBACK_OBJECTS,
                ]
                .as_slice()
            )
        );
        assert_eq!(
            map_setup_callback_kinds("MAPSETUP_CONNECTION"),
            Some(
                [
                    MAP_CALLBACK_NEWMAP,
                    MAP_CALLBACK_CMDQUEUE,
                    MAP_CALLBACK_TILES,
                    MAP_CALLBACK_OBJECTS,
                ]
                .as_slice()
            )
        );
        assert_eq!(
            map_setup_callback_kinds("MAPSETUP_RELOADMAP"),
            Some([MAP_CALLBACK_TILES, MAP_CALLBACK_SPRITES].as_slice())
        );
        assert_eq!(
            map_setup_callback_kinds("MAPSETUP_LINKRETURN"),
            Some(
                [
                    MAP_CALLBACK_NEWMAP,
                    MAP_CALLBACK_CMDQUEUE,
                    MAP_CALLBACK_TILES,
                    MAP_CALLBACK_SPRITES,
                ]
                .as_slice()
            )
        );
        assert_eq!(
            map_setup_callback_kinds("MAPSETUP_CONTINUE"),
            Some(
                [
                    MAP_CALLBACK_CMDQUEUE,
                    MAP_CALLBACK_TILES,
                    MAP_CALLBACK_SPRITES,
                ]
                .as_slice()
            )
        );
        assert_eq!(
            map_setup_callback_kinds("MAPSETUP_SUBMENU"),
            Some([MAP_CALLBACK_TILES].as_slice())
        );
        assert_eq!(map_setup_callback_kinds("MAPSETUP_NOT_SOURCE"), None);
    }

    fn maps() -> BTreeSet<String> {
        BTreeSet::from(["BattleTower1F".to_string(), "EcruteakCity".to_string()])
    }

    fn command(name: &str) -> ScriptMapCommand {
        ScriptMapCommand {
            command: name.to_string(),
            target_map: None,
            x: None,
            y: None,
            facing: None,
            map_setup: None,
            source_script: "WarpScript".to_string(),
            command_index: 4,
        }
    }

    #[test]
    fn exported_map_command_sets_are_exact() {
        assert!(SCRIPT_MAP_WARP_COMMANDS.contains(&"warp"));
        assert!(SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&"warpfacing"));
        assert!(SCRIPT_MAP_WARP_CHECK_COMMANDS.contains(&"warpcheck"));
        assert!(SCRIPT_MAP_NEW_LOAD_COMMANDS.contains(&"newloadmap"));
        assert!(SCRIPT_MAP_RELOAD_COMMANDS.contains(&"reloadmap"));
        assert!(SCRIPT_MAP_LOAD_COMMANDS.contains(&"newloadmap"));
        assert!(SCRIPT_MAP_LOAD_COMMANDS.contains(&"reloadmapafterbattle"));
        assert!(SCRIPT_MAP_SIMPLE_REFRESH_COMMANDS.contains(&"refreshmap"));
        assert!(SCRIPT_MAP_REANCHOR_COMMANDS.contains(&"reanchormap"));
        assert!(SCRIPT_MAP_REFRESH_COMMANDS.contains(&"refreshmap"));
        assert!(SCRIPT_MAP_REFRESH_COMMANDS.contains(&"reanchormap"));
        assert!(!SCRIPT_MAP_NO_PAYLOAD_COMMANDS.contains(&"reloadmappart"));
        assert!(!is_known_script_map_command("reloadmappart"));
        assert!(is_known_script_map_command("warpfacing"));
        assert!(!is_known_script_map_command("Warp"));
        assert!(!is_known_script_map_command("loadmap"));
    }

    #[test]
    fn rejects_legacy_reloadmappart_alias_in_favor_of_source_refreshmap() {
        assert!(matches!(
            resolve_script_map_command(command("reloadmappart"), &maps()),
            Err(ScriptMapCommandError::UnknownCommand { command })
                if command == "reloadmappart"
        ));
    }

    #[test]
    fn script_map_serialized_variants_reject_unknown_fallback_fields() {
        let action_error = serde_json::from_value::<ScriptMapAction>(serde_json::json!({
            "warp": {
                "target_map": "EcruteakCity",
                "tile": { "x": 4, "y": 8 },
                "facing": "down",
                "source_script": "WarpScript",
                "command_index": 4,
                "fallback_target_map": "NewBarkTown"
            }
        }))
        .expect_err("map actions must not accept fallback targets");
        assert!(
            action_error
                .to_string()
                .contains("unknown field `fallback_target_map`"),
            "{action_error}"
        );

        let command_error = serde_json::from_value::<ScriptMapCommandError>(serde_json::json!({
            "UnknownTargetMap": {
                "command": "warp",
                "target_map": "EcruteakCity",
                "legacy_target_map": "ECRUTEAK_CITY"
            }
        }))
        .expect_err("map command errors must not accept legacy target fields");
        assert!(
            command_error
                .to_string()
                .contains("unknown field `legacy_target_map`"),
            "{command_error}"
        );
    }

    #[test]
    fn script_map_issue_collector_reports_exact_pack_shape_errors() {
        let maps = maps();
        let mut warp = command("warp");
        warp.target_map = Some("NONE".to_string());
        warp.x = Some(1);
        warp.y = Some(0);
        warp.facing = Some("DOWN".to_string());
        assert_eq!(
            script_map_command_issues(&warp, &maps),
            vec![
                ScriptMapCommandError::MalformedBadWarpSentinel {
                    command: "warp".to_string(),
                },
                ScriptMapCommandError::UnexpectedFacing {
                    command: "warp".to_string(),
                },
            ]
        );

        let mut facing = command("warpfacing");
        facing.target_map = Some("ROUTE_29".to_string());
        facing.x = Some(3);
        facing.y = Some(4);
        facing.facing = Some("down".to_string());
        assert_eq!(
            script_map_command_issues(&facing, &maps),
            vec![
                ScriptMapCommandError::UnknownTargetMap {
                    command: "warpfacing".to_string(),
                    target_map: "ROUTE_29".to_string(),
                },
                ScriptMapCommandError::UnknownFacing {
                    facing: "down".to_string(),
                },
            ]
        );

        let mut load = command("newloadmap");
        load.target_map = Some("EcruteakCity".to_string());
        assert_eq!(
            script_map_command_issues(&load, &maps),
            vec![
                ScriptMapCommandError::UnexpectedWarpDestination {
                    command: "newloadmap".to_string(),
                },
                ScriptMapCommandError::MissingMapSetup {
                    command: "newloadmap".to_string(),
                },
            ]
        );

        assert_eq!(
            script_map_command_issues(&command("loadmap"), &maps),
            vec![ScriptMapCommandError::UnknownCommand {
                command: "loadmap".to_string(),
            }]
        );
        assert_eq!(
            script_map_command_issues(&command("Warp"), &maps),
            vec![ScriptMapCommandError::InvalidCommand {
                command: "Warp".to_string(),
            }]
        );
        assert_eq!(
            script_map_command_issues(&command("warp facing"), &maps),
            vec![ScriptMapCommandError::InvalidCommand {
                command: "warp facing".to_string(),
            }]
        );

        let mut padded = command("warpfacing");
        padded.target_map = Some("Ecruteak City".to_string());
        padded.x = Some(3);
        padded.y = Some(4);
        padded.facing = Some("U P".to_string());
        assert_eq!(
            script_map_command_issues(&padded, &maps),
            vec![
                ScriptMapCommandError::InvalidTargetMap {
                    command: "warpfacing".to_string(),
                    target_map: "Ecruteak City".to_string(),
                },
                ScriptMapCommandError::InvalidFacing {
                    facing: "U P".to_string(),
                },
            ]
        );

        let mut reanchor = command("reanchormap");
        reanchor.map_setup = Some("MAPSETUP TRAIN".to_string());
        assert_eq!(
            script_map_command_issues(&reanchor, &maps),
            vec![ScriptMapCommandError::InvalidMapSetup {
                command: "reanchormap".to_string(),
                map_setup: "MAPSETUP TRAIN".to_string(),
            }]
        );
    }

    #[test]
    fn script_map_commands_reject_reserved_pack_prefixes() {
        let maps = maps();
        assert_eq!(
            script_map_command_issues(&command("fallbackwarp"), &maps),
            vec![ScriptMapCommandError::InvalidCommand {
                command: "fallbackwarp".to_string(),
            }]
        );

        let mut warp = command("warp");
        warp.target_map = Some("legacy_map".to_string());
        warp.x = Some(1);
        warp.y = Some(1);
        assert!(script_map_command_issues(&warp, &maps).contains(
            &ScriptMapCommandError::InvalidTargetMap {
                command: "warp".to_string(),
                target_map: "legacy_map".to_string(),
            }
        ));

        let mut reanchor = command("reanchormap");
        reanchor.map_setup = Some("fallback_setup".to_string());
        assert_eq!(
            script_map_command_issues(&reanchor, &maps),
            vec![ScriptMapCommandError::InvalidMapSetup {
                command: "reanchormap".to_string(),
                map_setup: "fallback_setup".to_string(),
            }]
        );

        for (field, value) in [
            ("command", serde_json::json!("fallbackwarp")),
            ("target_map", serde_json::json!("legacy_map")),
            ("facing", serde_json::json!("fallback_down")),
            ("map_setup", serde_json::json!("legacy_setup")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "warpfacing",
                "target_map": "EcruteakCity",
                "x": 1,
                "y": 1,
                "facing": "DOWN",
                "map_setup": null,
                "source_script": ".branch@WarpScript",
                "command_index": 4
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptMapCommand>(payload)
                .expect_err("reserved script map command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script map") || error.contains("script label"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn resolves_raw_warp_and_warpfacing_commands() {
        let mut warp = command("warp");
        warp.target_map = Some("EcruteakCity".to_string());
        warp.x = Some(6);
        warp.y = Some(27);
        assert_eq!(
            resolve_script_map_command(warp, &maps()).expect("warp"),
            ScriptMapAction::Warp {
                target_map: "EcruteakCity".to_string(),
                tile: TilePosition::new(6, 27),
                facing: None,
                source_script: "WarpScript".to_string(),
                command_index: 4,
            }
        );

        let mut warpfacing = command("warpfacing");
        warpfacing.target_map = Some("BattleTower1F".to_string());
        warpfacing.x = Some(7);
        warpfacing.y = Some(7);
        warpfacing.facing = Some("UP".to_string());
        assert_eq!(
            resolve_script_map_command(warpfacing, &maps()).expect("warpfacing"),
            ScriptMapAction::Warp {
                target_map: "BattleTower1F".to_string(),
                tile: TilePosition::new(7, 7),
                facing: Some(Direction::Up),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            }
        );
    }

    #[test]
    fn script_warp_coordinates_are_raw_event_tiles() {
        let mut warp = command("warpfacing");
        warp.target_map = Some("EcruteakCity".to_string());
        warp.x = Some(27);
        warp.y = Some(1);
        warp.facing = Some("RIGHT".to_string());

        assert_eq!(
            resolve_script_map_command(warp, &maps()).expect("raw tile warpfacing"),
            ScriptMapAction::Warp {
                target_map: "EcruteakCity".to_string(),
                tile: TilePosition::new(27, 1),
                facing: Some(Direction::Right),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            }
        );
    }

    #[test]
    fn rejects_script_warp_coordinates_that_overflow_runtime_tile_space() {
        let mut warp = command("warp");
        warp.target_map = Some("EcruteakCity".to_string());
        warp.x = Some(40_000);
        warp.y = Some(0);

        assert_eq!(
            resolve_script_map_command(warp.clone(), &maps()),
            Err(ScriptMapCommandError::CoordinatesOutOfRange {
                command: "warp".to_string(),
            })
        );
        assert_eq!(
            script_map_command_issues(&warp, &maps()),
            vec![ScriptMapCommandError::CoordinatesOutOfRange {
                command: "warp".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_case_changed_map_ids_and_facing_tokens() {
        let mut warp = command("warp");
        warp.target_map = Some("ecruteakcity".to_string());
        warp.x = Some(6);
        warp.y = Some(27);
        assert!(matches!(
            resolve_script_map_command(warp, &maps()),
            Err(ScriptMapCommandError::UnknownTargetMap { .. })
        ));

        let mut warpfacing = command("warpfacing");
        warpfacing.target_map = Some("BattleTower1F".to_string());
        warpfacing.x = Some(7);
        warpfacing.y = Some(7);
        warpfacing.facing = Some("up".to_string());
        assert!(matches!(
            resolve_script_map_command(warpfacing, &maps()),
            Err(ScriptMapCommandError::UnknownFacing { .. })
        ));
    }

    #[test]
    fn rejects_padded_map_facing_and_setup_tokens_without_normalization() {
        let mut warp = command("warp");
        warp.target_map = Some(" EcruteakCity".to_string());
        warp.x = Some(6);
        warp.y = Some(27);
        assert!(matches!(
            resolve_script_map_command(warp, &maps()),
            Err(ScriptMapCommandError::InvalidTargetMap { .. })
        ));
        let mut warp = command("warp");
        warp.target_map = Some("Ecruteak City".to_string());
        warp.x = Some(6);
        warp.y = Some(27);
        assert!(matches!(
            resolve_script_map_command(warp, &maps()),
            Err(ScriptMapCommandError::InvalidTargetMap { .. })
        ));

        let mut warpfacing = command("warpfacing");
        warpfacing.target_map = Some("BattleTower1F".to_string());
        warpfacing.x = Some(7);
        warpfacing.y = Some(7);
        warpfacing.facing = Some(" UP".to_string());
        assert!(matches!(
            resolve_script_map_command(warpfacing, &maps()),
            Err(ScriptMapCommandError::InvalidFacing { .. })
        ));
        let mut warpfacing = command("warpfacing");
        warpfacing.target_map = Some("BattleTower1F".to_string());
        warpfacing.x = Some(7);
        warpfacing.y = Some(7);
        warpfacing.facing = Some("U P".to_string());
        assert!(matches!(
            resolve_script_map_command(warpfacing, &maps()),
            Err(ScriptMapCommandError::InvalidFacing { .. })
        ));

        let mut newloadmap = command("newloadmap");
        newloadmap.map_setup = Some(" MAPSETUP_TRAIN".to_string());
        assert!(matches!(
            resolve_script_map_command(newloadmap, &maps()),
            Err(ScriptMapCommandError::InvalidMapSetup { .. })
        ));
        let mut newloadmap = command("newloadmap");
        newloadmap.map_setup = Some("MAPSETUP TRAIN".to_string());
        assert!(matches!(
            resolve_script_map_command(newloadmap, &maps()),
            Err(ScriptMapCommandError::InvalidMapSetup { .. })
        ));
    }

    #[test]
    fn rejects_malformed_map_commands_before_unknown_command_handling() {
        assert_eq!(
            resolve_script_map_command(command("Warp"), &maps()),
            Err(ScriptMapCommandError::InvalidCommand {
                command: "Warp".to_string(),
            })
        );
        assert_eq!(
            resolve_script_map_command(command("warp facing"), &maps()),
            Err(ScriptMapCommandError::InvalidCommand {
                command: "warp facing".to_string(),
            })
        );
        assert_eq!(
            resolve_script_map_command(command("loadmap"), &maps()),
            Err(ScriptMapCommandError::UnknownCommand {
                command: "loadmap".to_string(),
            })
        );
    }

    #[test]
    fn resolves_map_lifecycle_commands_without_implicit_destinations() {
        let warpcheck = resolve_script_map_command(command("warpcheck"), &maps()).expect("check");
        assert!(matches!(warpcheck, ScriptMapAction::WarpCheck { .. }));

        let mut newloadmap = command("newloadmap");
        newloadmap.map_setup = Some("MAPSETUP_TRAIN".to_string());
        assert_eq!(
            resolve_script_map_command(newloadmap, &maps()).expect("new load"),
            ScriptMapAction::LoadMap {
                command: "newloadmap".to_string(),
                map_setup: Some("MAPSETUP_TRAIN".to_string()),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            }
        );

        assert!(matches!(
            resolve_script_map_command(command("reloadmap"), &maps()).expect("reload"),
            ScriptMapAction::LoadMap {
                command,
                map_setup: None,
                ..
            } if command == "reloadmap"
        ));
    }

    #[test]
    fn warp_none_uses_the_source_bad_warp_map_setup() {
        let mut sentinel = command("warp");
        sentinel.target_map = Some("NONE".to_string());
        sentinel.x = Some(0);
        sentinel.y = Some(0);
        assert_eq!(
            resolve_script_map_command(sentinel, &maps()).expect("bad warp"),
            ScriptMapAction::LoadMap {
                command: "warp".to_string(),
                map_setup: Some("MAPSETUP_BADWARP".to_string()),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            }
        );

        let mut malformed = command("warp");
        malformed.target_map = Some("NONE".to_string());
        malformed.x = Some(1);
        malformed.y = Some(0);
        assert!(matches!(
            resolve_script_map_command(malformed, &maps()),
            Err(ScriptMapCommandError::MalformedBadWarpSentinel { .. })
        ));
    }

    #[test]
    fn applies_exact_script_warp_requests_to_runtime_state() {
        let mut state = GameState::default();
        let mut warp = command("warpfacing");
        warp.target_map = Some("BattleTower1F".to_string());
        warp.x = Some(7);
        warp.y = Some(7);
        warp.facing = Some("UP".to_string());

        let action = apply_script_map_command(&mut state, warp, &maps()).expect("apply warpfacing");
        assert!(matches!(
            action,
            ScriptMapAction::Warp {
                facing: Some(Direction::Up),
                ..
            }
        ));
        assert_eq!(
            state.script_runtime.pending_script_warp,
            Some(ScriptWarpRequest {
                target_map: "BattleTower1F".to_string(),
                tile: TilePosition::new(7, 7),
                facing: Some(Direction::Up),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            })
        );
        assert_eq!(state.script_runtime.map_events.len(), 1);
        assert_eq!(
            state.script_runtime.map_events[0].kind,
            ScriptMapRuntimeKind::Warp
        );
    }

    #[test]
    fn applies_map_lifecycle_requests_without_inventing_destinations() {
        let mut state = GameState::default();
        let mut newloadmap = command("newloadmap");
        newloadmap.map_setup = Some("MAPSETUP_TRAIN".to_string());
        apply_script_map_command(&mut state, newloadmap, &maps()).expect("newloadmap");
        apply_script_map_command(&mut state, command("refreshmap"), &maps()).expect("refresh");

        assert_eq!(
            state.script_runtime.pending_map_load,
            Some(ScriptMapLoadRequest {
                command: "newloadmap".to_string(),
                map_setup: Some("MAPSETUP_TRAIN".to_string()),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            })
        );
        assert_eq!(
            state.script_runtime.pending_map_refresh,
            Some(ScriptMapRefreshRequest {
                command: "refreshmap".to_string(),
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 4,
            })
        );
        assert!(
            state
                .script_runtime
                .map_events
                .iter()
                .all(|event| event.target_map.is_none() && event.tile.is_none())
        );
    }

    #[test]
    fn newloadmap_preserves_the_destination_staged_by_the_preceding_command() {
        let mut state = GameState::default();
        let staged_warp = ScriptWarpRequest {
            target_map: "EcruteakCity".to_string(),
            tile: TilePosition::new(12, 54),
            facing: None,
            source_script: "WarpScript".to_string(),
            command_index: 3,
        };
        state.script_runtime.pending_script_warp = Some(staged_warp.clone());
        let mut newloadmap = command("newloadmap");
        newloadmap.map_setup = Some("MAPSETUP_WARP".to_string());

        apply_script_map_command(&mut state, newloadmap, &maps()).expect("newloadmap");

        assert_eq!(state.script_runtime.pending_script_warp, Some(staged_warp));
        assert!(matches!(
            state.script_runtime.pending_map_load,
            Some(ScriptMapLoadRequest { ref command, .. }) if command == "newloadmap"
        ));
    }

    #[test]
    fn bad_warp_replaces_a_pending_warp_with_a_map_load() {
        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
            target_map: "EcruteakCity".to_string(),
            tile: TilePosition::new(12, 54),
            facing: None,
            source_script: "PreviousScript".to_string(),
            command_index: 1,
        });
        let mut sentinel = command("warp");
        sentinel.target_map = Some("NONE".to_string());
        sentinel.x = Some(0);
        sentinel.y = Some(0);

        apply_script_map_command(&mut state, sentinel, &maps()).expect("bad warp");
        assert_eq!(state.script_runtime.pending_script_warp, None);
        assert_eq!(
            state.script_runtime.pending_map_load,
            Some(ScriptMapLoadRequest {
                command: "warp".to_string(),
                map_setup: Some("MAPSETUP_BADWARP".to_string()),
                source_script: "WarpScript".to_string(),
                command_index: 4,
            })
        );
        assert_eq!(
            state.script_runtime.map_events[0].kind,
            ScriptMapRuntimeKind::LoadMap
        );
        assert_eq!(state.script_runtime.validate(), Ok(()));
    }

    #[test]
    fn completing_pending_script_warp_clears_exact_request_only() {
        let request = ScriptWarpRequest {
            target_map: "EcruteakCity".to_string(),
            tile: TilePosition::new(12, 54),
            facing: Some(Direction::Down),
            source_script: "WarpScript".to_string(),
            command_index: 2,
        };
        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(request.clone());

        assert_eq!(
            complete_pending_script_warp(&mut state, &request),
            Ok(request)
        );
        assert_eq!(state.script_runtime.pending_script_warp, None);
    }

    #[test]
    fn completing_pending_script_warp_rejects_missing_or_changed_request() {
        let request = ScriptWarpRequest {
            target_map: "EcruteakCity".to_string(),
            tile: TilePosition::new(12, 54),
            facing: None,
            source_script: "WarpScript".to_string(),
            command_index: 2,
        };
        let mut state = GameState::default();

        assert_eq!(
            complete_pending_script_warp(&mut state, &request),
            Err(ScriptMapCommandError::MissingPendingScriptWarp)
        );

        let pending = ScriptWarpRequest {
            target_map: "GoldenrodCity".to_string(),
            ..request.clone()
        };
        state.script_runtime.pending_script_warp = Some(pending.clone());
        assert_eq!(
            complete_pending_script_warp(&mut state, &request),
            Err(ScriptMapCommandError::PendingScriptWarpMismatch)
        );
        assert_eq!(state.script_runtime.pending_script_warp, Some(pending));
    }

    #[test]
    fn script_warp_arrival_facing_applies_only_explicit_direction() {
        let mut player = PlayerMovementState::new(TilePosition::new(12, 54));
        player.facing = Direction::Left;
        let request = ScriptWarpRequest {
            target_map: "EcruteakCity".to_string(),
            tile: TilePosition::new(12, 54),
            facing: Some(Direction::Up),
            source_script: "WarpScript".to_string(),
            command_index: 2,
        };

        assert_eq!(
            apply_script_warp_arrival_facing(&mut player, &request),
            Some(Direction::Up)
        );
        assert_eq!(player.facing, Direction::Up);

        let no_facing = ScriptWarpRequest {
            facing: None,
            ..request
        };
        assert_eq!(
            apply_script_warp_arrival_facing(&mut player, &no_facing),
            None
        );
        assert_eq!(player.facing, Direction::Up);
    }

    #[test]
    fn invalid_script_map_command_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        let mut warp = command("warp");
        warp.target_map = Some("ecruteakcity".to_string());
        warp.x = Some(6);
        warp.y = Some(27);

        assert!(matches!(
            apply_script_map_command(&mut state, warp, &maps()),
            Err(ScriptMapCommandError::UnknownTargetMap { .. })
        ));
        assert!(state.script_runtime.map_events.is_empty());
        assert_eq!(state.script_runtime.pending_script_warp, None);
    }

    #[test]
    fn invalid_script_map_source_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        state.script_runtime.pending_map_refresh = Some(ScriptMapRefreshRequest {
            command: "refreshmap".to_string(),
            map_setup: None,
            source_script: "PreviousScript".to_string(),
            command_index: 1,
        });
        let mut refresh = command("refreshmap");
        refresh.source_script = "fallback_script".to_string();

        assert_eq!(
            script_map_command_issues(&refresh, &maps()),
            vec![ScriptMapCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            }]
        );
        assert_eq!(
            apply_script_map_command(&mut state, refresh, &maps()),
            Err(ScriptMapCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );

        assert!(state.script_runtime.map_events.is_empty());
        assert_eq!(state.script_runtime.pending_script_warp, None);
        assert_eq!(
            state.script_runtime.pending_map_refresh,
            Some(ScriptMapRefreshRequest {
                command: "refreshmap".to_string(),
                map_setup: None,
                source_script: "PreviousScript".to_string(),
                command_index: 1,
            })
        );
    }
}
