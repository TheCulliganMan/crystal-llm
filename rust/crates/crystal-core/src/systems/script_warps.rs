use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::state::{
    GameState, ScriptMapLoadRequest, ScriptMapRefreshRequest, ScriptMapRuntimeEvent,
    ScriptMapRuntimeKind, ScriptWarpRequest,
};
use crate::world::map::{Direction, TilePosition};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapCommand {
    pub command: String,
    pub target_map: Option<String>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub facing: Option<String>,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptMapAction {
    NoWarp {
        source_script: String,
        command_index: usize,
    },
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
pub enum ScriptMapCommandError {
    #[error("unknown script map command '{command}'")]
    UnknownCommand { command: String },
    #[error("script map command '{command}' is missing a target map")]
    MissingTargetMap { command: String },
    #[error("script map command '{command}' has invalid target map '{target_map}'")]
    InvalidTargetMap { command: String, target_map: String },
    #[error("script map command '{command}' references unknown map '{target_map}'")]
    UnknownTargetMap { command: String, target_map: String },
    #[error("script map command '{command}' has malformed no-warp sentinel")]
    MalformedNoWarpSentinel { command: String },
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
}

pub const SCRIPT_MAP_WARP_COMMANDS: &[&str] = &["warp"];
pub const SCRIPT_MAP_FACING_WARP_COMMANDS: &[&str] = &["warpfacing"];
pub const SCRIPT_MAP_WARP_CHECK_COMMANDS: &[&str] = &["warpcheck"];
pub const SCRIPT_MAP_NEW_LOAD_COMMANDS: &[&str] = &["newloadmap"];
pub const SCRIPT_MAP_RELOAD_COMMANDS: &[&str] =
    &["reloadmap", "reloadmappart", "reloadmapafterbattle"];
pub const SCRIPT_MAP_LOAD_COMMANDS: &[&str] = &[
    "newloadmap",
    "reloadmap",
    "reloadmappart",
    "reloadmapafterbattle",
];
pub const SCRIPT_MAP_SIMPLE_REFRESH_COMMANDS: &[&str] = &["refreshmap"];
pub const SCRIPT_MAP_REANCHOR_COMMANDS: &[&str] = &["reanchormap"];
pub const SCRIPT_MAP_REFRESH_COMMANDS: &[&str] = &["refreshmap", "reanchormap"];
pub const SCRIPT_MAP_NO_PAYLOAD_COMMANDS: &[&str] = &[
    "warpcheck",
    "reloadmap",
    "reloadmappart",
    "reloadmapafterbattle",
    "refreshmap",
];

pub fn is_known_script_map_command(command: &str) -> bool {
    SCRIPT_MAP_WARP_COMMANDS.contains(&command)
        || SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command)
        || SCRIPT_MAP_WARP_CHECK_COMMANDS.contains(&command)
        || SCRIPT_MAP_LOAD_COMMANDS.contains(&command)
        || SCRIPT_MAP_REFRESH_COMMANDS.contains(&command)
}

pub fn script_map_command_issues(
    command: &ScriptMapCommand,
    map_ids: &BTreeSet<String>,
) -> Vec<ScriptMapCommandError> {
    let mut issues = Vec::new();
    if SCRIPT_MAP_WARP_COMMANDS.contains(&command.command.as_str()) {
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
    match command.command.as_str() {
        "warp" => {
            reject_facing(&command)?;
            reject_map_setup(&command)?;
            if is_no_warp_sentinel(&command) {
                return Ok(ScriptMapAction::NoWarp {
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
        "reloadmap" | "reloadmappart" | "reloadmapafterbattle" => {
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

pub fn apply_script_map_action_to_state(state: &mut GameState, action: &ScriptMapAction) {
    match action {
        ScriptMapAction::NoWarp {
            source_script,
            command_index,
        } => {
            state.script_runtime.pending_script_warp = None;
            state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
                command: "warp".to_string(),
                kind: ScriptMapRuntimeKind::NoWarp,
                target_map: None,
                tile: None,
                facing: None,
                map_setup: None,
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
            state.script_runtime.warp_check_requested = true;
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
        ScriptMapAction::LoadMap {
            command,
            map_setup,
            source_script,
            command_index,
        } => {
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
        return Err(ScriptMapCommandError::MalformedNoWarpSentinel {
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

fn is_no_warp_sentinel(command: &ScriptMapCommand) -> bool {
    command.target_map.as_deref() == Some("NONE") && command.x == Some(0) && command.y == Some(0)
}

fn require_tile(command: &ScriptMapCommand) -> Result<TilePosition, ScriptMapCommandError> {
    let (Some(x), Some(y)) = (command.x, command.y) else {
        return Err(ScriptMapCommandError::MissingCoordinates {
            command: command.command.clone(),
        });
    };
    let x = i16::try_from(x).map_err(|_| ScriptMapCommandError::CoordinatesOutOfRange {
        command: command.command.clone(),
    })?;
    let y = i16::try_from(y).map_err(|_| ScriptMapCommandError::CoordinatesOutOfRange {
        command: command.command.clone(),
    })?;
    Ok(TilePosition::new(x, y))
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
        Some("NONE") if is_no_warp_sentinel(command) => {}
        Some("NONE") => issues.push(ScriptMapCommandError::MalformedNoWarpSentinel {
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
    } else if command.x.is_some_and(|x| i16::try_from(x).is_err())
        || command.y.is_some_and(|y| i16::try_from(y).is_err())
    {
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
    !value.is_empty() && value.trim() == value
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(SCRIPT_MAP_NO_PAYLOAD_COMMANDS.contains(&"reloadmappart"));
        assert!(is_known_script_map_command("warpfacing"));
        assert!(!is_known_script_map_command("Warp"));
        assert!(!is_known_script_map_command("loadmap"));
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
                ScriptMapCommandError::MalformedNoWarpSentinel {
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

        let mut padded = command("warpfacing");
        padded.target_map = Some(" EcruteakCity".to_string());
        padded.x = Some(3);
        padded.y = Some(4);
        padded.facing = Some(" UP".to_string());
        assert_eq!(
            script_map_command_issues(&padded, &maps),
            vec![
                ScriptMapCommandError::InvalidTargetMap {
                    command: "warpfacing".to_string(),
                    target_map: " EcruteakCity".to_string(),
                },
                ScriptMapCommandError::InvalidFacing {
                    facing: " UP".to_string(),
                },
            ]
        );

        let mut reanchor = command("reanchormap");
        reanchor.map_setup = Some(" MAPSETUP_TRAIN".to_string());
        assert_eq!(
            script_map_command_issues(&reanchor, &maps),
            vec![ScriptMapCommandError::InvalidMapSetup {
                command: "reanchormap".to_string(),
                map_setup: " MAPSETUP_TRAIN".to_string(),
            }]
        );
    }

    #[test]
    fn resolves_exact_warp_and_warpfacing_commands() {
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

        let mut warpfacing = command("warpfacing");
        warpfacing.target_map = Some("BattleTower1F".to_string());
        warpfacing.x = Some(7);
        warpfacing.y = Some(7);
        warpfacing.facing = Some(" UP".to_string());
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
    fn resolves_only_exact_no_warp_sentinel() {
        let mut sentinel = command("warp");
        sentinel.target_map = Some("NONE".to_string());
        sentinel.x = Some(0);
        sentinel.y = Some(0);
        assert_eq!(
            resolve_script_map_command(sentinel, &maps()).expect("sentinel"),
            ScriptMapAction::NoWarp {
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
            Err(ScriptMapCommandError::MalformedNoWarpSentinel { .. })
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
    fn no_warp_sentinel_clears_pending_script_warp() {
        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
            target_map: "EcruteakCity".to_string(),
            tile: TilePosition::new(6, 27),
            facing: None,
            source_script: "PreviousScript".to_string(),
            command_index: 1,
        });
        let mut sentinel = command("warp");
        sentinel.target_map = Some("NONE".to_string());
        sentinel.x = Some(0);
        sentinel.y = Some(0);

        apply_script_map_command(&mut state, sentinel, &maps()).expect("no warp");
        assert_eq!(state.script_runtime.pending_script_warp, None);
        assert_eq!(
            state.script_runtime.map_events[0].kind,
            ScriptMapRuntimeKind::NoWarp
        );
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
}
