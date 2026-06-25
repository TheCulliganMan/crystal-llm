use serde::{Deserialize, Serialize};

use crate::map::MapSceneTable;
use crate::state::{GameState, SceneError, SceneStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSceneCommand {
    pub command: String,
    pub map_id: Option<String>,
    pub scene_id: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSceneOutcome {
    pub command: String,
    pub map_name: String,
    pub scene_id: String,
    pub scene_index: usize,
    pub script_name: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptSceneError {
    UnknownCommand { command: String },
    MissingCurrentMap,
    MissingTargetMap { command: String },
    UnexpectedTargetMap { command: String },
    MissingSceneId { command: String },
    UnexpectedSceneId { command: String },
    UnknownSceneToken { map_name: String, scene_id: String },
    Scene { error: SceneError },
}

impl From<SceneError> for ScriptSceneError {
    fn from(error: SceneError) -> Self {
        Self::Scene { error }
    }
}

pub const SCRIPT_SCENE_CHECK_COMMANDS: &[&str] = &["checkscene"];
pub const SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS: &[&str] = &["setscene"];
pub const SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS: &[&str] = &["setmapscene"];

pub fn is_known_script_scene_command(command: &str) -> bool {
    SCRIPT_SCENE_CHECK_COMMANDS.contains(&command)
        || SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&command)
        || SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&command)
}

pub fn apply_script_scene_command(
    state: &mut GameState,
    current_map_name: &str,
    resolved_target_map_name: Option<&str>,
    table: &MapSceneTable,
    command: ScriptSceneCommand,
) -> Result<ScriptSceneOutcome, ScriptSceneError> {
    let status = match command.command.as_str() {
        "checkscene" => {
            reject_target_map(&command)?;
            reject_scene_id(&command)?;
            let map_name = require_current_map(current_map_name)?;
            state.scenes.check_scene(map_name, table)?
        }
        "setscene" => {
            reject_target_map(&command)?;
            let map_name = require_current_map(current_map_name)?;
            let scene_token = require_scene_id(&command)?;
            let scene_id = resolve_scene_token(map_name, scene_token, table)?;
            state.scenes.set_map_scene(map_name, &scene_id, table)?
        }
        "setmapscene" => {
            if command.map_id.is_none() {
                return Err(ScriptSceneError::MissingTargetMap {
                    command: command.command.clone(),
                });
            }
            let map_name =
                resolved_target_map_name.ok_or_else(|| ScriptSceneError::MissingTargetMap {
                    command: command.command.clone(),
                })?;
            let scene_token = require_scene_id(&command)?;
            let scene_id = resolve_scene_token(map_name, scene_token, table)?;
            state.scenes.set_map_scene(map_name, &scene_id, table)?
        }
        other => {
            return Err(ScriptSceneError::UnknownCommand {
                command: other.to_string(),
            });
        }
    };
    Ok(outcome(command, status))
}

fn outcome(command: ScriptSceneCommand, status: SceneStatus) -> ScriptSceneOutcome {
    ScriptSceneOutcome {
        command: command.command,
        map_name: status.map_name,
        scene_id: status.scene_name,
        scene_index: status.scene_index,
        script_name: status.script_name,
        source_script: command.source_script,
        command_index: command.command_index,
    }
}

fn require_current_map(current_map_name: &str) -> Result<&str, ScriptSceneError> {
    if current_map_name.is_empty() {
        Err(ScriptSceneError::MissingCurrentMap)
    } else {
        Ok(current_map_name)
    }
}

fn require_scene_id(command: &ScriptSceneCommand) -> Result<&str, ScriptSceneError> {
    command
        .scene_id
        .as_deref()
        .ok_or_else(|| ScriptSceneError::MissingSceneId {
            command: command.command.clone(),
        })
}

fn reject_scene_id(command: &ScriptSceneCommand) -> Result<(), ScriptSceneError> {
    if command.scene_id.is_some() {
        Err(ScriptSceneError::UnexpectedSceneId {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn reject_target_map(command: &ScriptSceneCommand) -> Result<(), ScriptSceneError> {
    if command.map_id.is_some() {
        Err(ScriptSceneError::UnexpectedTargetMap {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn resolve_scene_token(
    map_name: &str,
    scene_token: &str,
    table: &MapSceneTable,
) -> Result<String, ScriptSceneError> {
    if table
        .scenes
        .iter()
        .any(|scene| scene.scene_id == scene_token)
    {
        return Ok(scene_token.to_string());
    }
    if let Ok(index) = scene_token.parse::<usize>()
        && let Some(scene) = table.scenes.get(index)
    {
        return Ok(scene.scene_id.clone());
    }
    Err(ScriptSceneError::UnknownSceneToken {
        map_name: map_name.to_string(),
        scene_id: scene_token.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapScene;

    fn table() -> MapSceneTable {
        MapSceneTable {
            scenes: vec![
                MapScene {
                    scene_id: "SCENE_ROUTE43GATE_ROCKETS".to_string(),
                    script_name: Some("RocketScene".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_ROUTE43GATE_NOOP".to_string(),
                    script_name: None,
                },
            ],
        }
    }

    fn command(name: &str, map_id: Option<&str>, scene_id: Option<&str>) -> ScriptSceneCommand {
        ScriptSceneCommand {
            command: name.to_string(),
            map_id: map_id.map(str::to_string),
            scene_id: scene_id.map(str::to_string),
            source_script: "GateScript".to_string(),
            command_index: 4,
        }
    }

    #[test]
    fn exported_scene_command_sets_are_exact() {
        assert!(SCRIPT_SCENE_CHECK_COMMANDS.contains(&"checkscene"));
        assert!(SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&"setscene"));
        assert!(SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&"setmapscene"));
        assert!(is_known_script_scene_command("setscene"));
        assert!(!is_known_script_scene_command("SetScene"));
        assert!(!is_known_script_scene_command("resetscene"));
    }

    #[test]
    fn setscene_and_checkscene_use_exact_scene_ids() {
        let mut state = GameState::default();
        state
            .scenes
            .enter_map("Route43Gate", &table())
            .expect("enter map");

        let outcome = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            None,
            &table(),
            command("setscene", None, Some("SCENE_ROUTE43GATE_NOOP")),
        )
        .expect("set scene");
        assert_eq!(outcome.scene_index, 1);
        assert_eq!(state.scenes.scene_name, "SCENE_ROUTE43GATE_NOOP");

        let check = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            None,
            &table(),
            command("checkscene", None, None),
        )
        .expect("check scene");
        assert_eq!(check.scene_id, "SCENE_ROUTE43GATE_NOOP");
    }

    #[test]
    fn setmapscene_resolves_numeric_scene_tokens_against_supplied_table() {
        let mut state = GameState::default();
        let outcome = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            Some("Route43"),
            &table(),
            command("setmapscene", Some("ROUTE_43"), Some("1")),
        )
        .expect("set target map scene by numeric token");

        assert_eq!(outcome.map_name, "Route43");
        assert_eq!(outcome.scene_id, "SCENE_ROUTE43GATE_NOOP");
        assert_eq!(state.scenes.map_scenes["Route43"], "SCENE_ROUTE43GATE_NOOP");
    }

    #[test]
    fn rejects_case_changed_or_unknown_scene_tokens() {
        let mut state = GameState::default();
        let error = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            None,
            &table(),
            command("setscene", None, Some("scene_route43gate_noop")),
        )
        .expect_err("case changed scene id must not resolve");

        assert_eq!(
            error,
            ScriptSceneError::UnknownSceneToken {
                map_name: "Route43Gate".to_string(),
                scene_id: "scene_route43gate_noop".to_string(),
            }
        );
    }
}
