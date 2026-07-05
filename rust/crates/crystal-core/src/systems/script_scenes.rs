use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::map::MapSceneTable;
use crate::state::{GameState, SceneError, SceneStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSceneCommand {
    #[serde(deserialize_with = "required_script_scene_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_script_scene_token")]
    pub map_id: Option<String>,
    #[serde(deserialize_with = "required_nullable_script_scene_token")]
    pub scene_id: Option<String>,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptSceneCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptSceneCommand {
            #[serde(deserialize_with = "required_script_scene_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_script_scene_token")]
            map_id: Option<String>,
            #[serde(deserialize_with = "required_nullable_script_scene_token")]
            scene_id: Option<String>,
            #[serde(deserialize_with = "required_script_label_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptSceneCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            map_id: raw.map_id,
            scene_id: raw.scene_id,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        if let Some(issue) = script_scene_command_issues(&command).into_iter().next() {
            return Err(D::Error::custom(format!(
                "invalid script scene command: {issue:?}"
            )));
        }
        Ok(command)
    }
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
#[serde(deny_unknown_fields)]
pub enum ScriptSceneError {
    InvalidCommand { command: String },
    UnknownCommand { command: String },
    MissingCurrentMap,
    MissingTargetMap { command: String },
    InvalidTargetMap { command: String, map_id: String },
    UnexpectedTargetMap { command: String },
    InvalidSourceScript { source_script: String },
    MissingSceneId { command: String },
    InvalidSceneId { command: String, scene_id: String },
    UnexpectedSceneId { command: String },
    UnknownSceneToken { map_name: String, scene_id: String },
    Scene { error: SceneError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptSceneCommandIssue {
    InvalidCommand,
    UnknownCommand,
    MissingTargetMap,
    InvalidTargetMap,
    UnexpectedTargetMap,
    InvalidSourceScript,
    MissingSceneId,
    InvalidSceneId,
    UnexpectedSceneId,
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

pub fn script_scene_command_issues(command: &ScriptSceneCommand) -> Vec<ScriptSceneCommandIssue> {
    let mut issues = Vec::new();
    if !is_exact_script_label_token(&command.source_script) {
        issues.push(ScriptSceneCommandIssue::InvalidSourceScript);
    }
    if !is_exact_script_scene_command_token(&command.command) {
        issues.push(ScriptSceneCommandIssue::InvalidCommand);
    } else if SCRIPT_SCENE_CHECK_COMMANDS.contains(&command.command.as_str()) {
        if command.map_id.is_some() {
            issues.push(ScriptSceneCommandIssue::UnexpectedTargetMap);
        }
        if command.scene_id.is_some() {
            issues.push(ScriptSceneCommandIssue::UnexpectedSceneId);
        }
    } else if SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&command.command.as_str()) {
        if command.map_id.is_some() {
            issues.push(ScriptSceneCommandIssue::UnexpectedTargetMap);
        }
        match command.scene_id.as_deref() {
            Some(scene_id) if !is_exact_script_scene_token(scene_id) => {
                issues.push(ScriptSceneCommandIssue::InvalidSceneId);
            }
            Some(_) => {}
            None => issues.push(ScriptSceneCommandIssue::MissingSceneId),
        }
    } else if SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&command.command.as_str()) {
        match command.map_id.as_deref() {
            Some(map_id) if !is_exact_script_scene_token(map_id) => {
                issues.push(ScriptSceneCommandIssue::InvalidTargetMap);
            }
            Some(_) => {}
            None => issues.push(ScriptSceneCommandIssue::MissingTargetMap),
        }
        match command.scene_id.as_deref() {
            Some(scene_id) if !is_exact_script_scene_token(scene_id) => {
                issues.push(ScriptSceneCommandIssue::InvalidSceneId);
            }
            Some(_) => {}
            None => issues.push(ScriptSceneCommandIssue::MissingSceneId),
        }
    } else {
        issues.push(ScriptSceneCommandIssue::UnknownCommand);
    }
    issues
}

pub fn apply_script_scene_command(
    state: &mut GameState,
    current_map_name: &str,
    resolved_target_map_name: Option<&str>,
    table: &MapSceneTable,
    command: ScriptSceneCommand,
) -> Result<ScriptSceneOutcome, ScriptSceneError> {
    if !is_exact_script_label_token(&command.source_script) {
        return Err(ScriptSceneError::InvalidSourceScript {
            source_script: command.source_script,
        });
    }
    if !is_exact_script_scene_command_token(&command.command) {
        return Err(ScriptSceneError::InvalidCommand {
            command: command.command,
        });
    }
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
            require_target_map(&command)?;
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
    let scene_id = command
        .scene_id
        .as_deref()
        .ok_or_else(|| ScriptSceneError::MissingSceneId {
            command: command.command.clone(),
        })?;
    if !is_exact_script_scene_token(scene_id) {
        return Err(ScriptSceneError::InvalidSceneId {
            command: command.command.clone(),
            scene_id: scene_id.to_string(),
        });
    }
    Ok(scene_id)
}

fn require_target_map(command: &ScriptSceneCommand) -> Result<&str, ScriptSceneError> {
    let map_id = command
        .map_id
        .as_deref()
        .ok_or_else(|| ScriptSceneError::MissingTargetMap {
            command: command.command.clone(),
        })?;
    if !is_exact_script_scene_token(map_id) {
        return Err(ScriptSceneError::InvalidTargetMap {
            command: command.command.clone(),
            map_id: map_id.to_string(),
        });
    }
    Ok(map_id)
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
    if let Ok(index) = scene_token.parse::<usize>() {
        if table.scenes.is_empty() || table.scenes.get(index).is_some() {
            return Ok(scene_token.to_string());
        }
    }
    Err(ScriptSceneError::UnknownSceneToken {
        map_name: map_name.to_string(),
        scene_id: scene_token.to_string(),
    })
}

fn is_exact_script_scene_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_scene_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !has_reserved_pack_prefix(value)
}

fn required_script_scene_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_scene_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script scene command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_nullable_script_scene_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_script_scene_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script scene token must be exact ASCII alphanumeric/underscore, found {token:?}"
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
    fn script_scene_issue_collector_reports_exact_pack_shape_errors() {
        assert_eq!(
            script_scene_command_issues(&command(
                "checkscene",
                Some("ROUTE_43"),
                Some("SCENE_ROUTE43GATE_NOOP")
            )),
            vec![
                ScriptSceneCommandIssue::UnexpectedTargetMap,
                ScriptSceneCommandIssue::UnexpectedSceneId,
            ]
        );
        assert_eq!(
            script_scene_command_issues(&command("setscene", Some("ROUTE_43"), None)),
            vec![
                ScriptSceneCommandIssue::UnexpectedTargetMap,
                ScriptSceneCommandIssue::MissingSceneId,
            ]
        );
        assert_eq!(
            script_scene_command_issues(&command("setmapscene", None, None)),
            vec![
                ScriptSceneCommandIssue::MissingTargetMap,
                ScriptSceneCommandIssue::MissingSceneId,
            ]
        );
        assert_eq!(
            script_scene_command_issues(&command(
                "setmapscene",
                Some(" ROUTE_43"),
                Some(" SCENE_ROUTE_43_OPEN"),
            )),
            vec![
                ScriptSceneCommandIssue::InvalidTargetMap,
                ScriptSceneCommandIssue::InvalidSceneId,
            ]
        );
        assert_eq!(
            script_scene_command_issues(&command(
                "setmapscene",
                Some("ROUTE 43"),
                Some("SCENE ROUTE_43_OPEN"),
            )),
            vec![
                ScriptSceneCommandIssue::InvalidTargetMap,
                ScriptSceneCommandIssue::InvalidSceneId,
            ]
        );
        assert_eq!(
            script_scene_command_issues(&command("setscene", None, Some(" SCENE_START_OPEN"))),
            vec![ScriptSceneCommandIssue::InvalidSceneId]
        );
        assert_eq!(
            script_scene_command_issues(&command("SetScene", None, Some("SCENE_START_OPEN"))),
            vec![ScriptSceneCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_scene_command_issues(&command("resetscene", None, None)),
            vec![ScriptSceneCommandIssue::UnknownCommand]
        );
    }

    #[test]
    fn script_scene_commands_reject_reserved_pack_prefixes() {
        assert_eq!(
            script_scene_command_issues(&command(
                "fallbackscene",
                Some("ROUTE_43"),
                Some("SCENE_ROUTE43GATE_NOOP"),
            )),
            vec![ScriptSceneCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_scene_command_issues(&command(
                "setmapscene",
                Some("legacy_route"),
                Some("fallback_scene"),
            )),
            vec![
                ScriptSceneCommandIssue::InvalidTargetMap,
                ScriptSceneCommandIssue::InvalidSceneId,
            ]
        );

        for (field, value) in [
            ("command", serde_json::json!("fallbackscene")),
            ("map_id", serde_json::json!("legacy_route")),
            ("scene_id", serde_json::json!("fallback_scene")),
            ("source_script", serde_json::json!("legacy_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "setmapscene",
                "map_id": "ROUTE_43",
                "scene_id": "SCENE_ROUTE43GATE_NOOP",
                "source_script": ".branch@GateScript",
                "command_index": 4
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptSceneCommand>(payload)
                .expect_err("reserved script scene command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script scene") || error.contains("script label"),
                "{field} produced unexpected error: {error}"
            );
        }
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
    fn setmapscene_accepts_declared_numeric_scene_ids_without_index_resolution() {
        let table = MapSceneTable {
            scenes: vec![
                MapScene {
                    scene_id: "0".to_string(),
                    script_name: Some("RocketScene".to_string()),
                },
                MapScene {
                    scene_id: "1".to_string(),
                    script_name: None,
                },
            ],
        };
        let mut state = GameState::default();
        let outcome = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            Some("Route43"),
            &table,
            command("setmapscene", Some("ROUTE_43"), Some("1")),
        )
        .expect("set target map scene by declared numeric id");

        assert_eq!(outcome.map_name, "Route43");
        assert_eq!(outcome.scene_id, "1");
        assert_eq!(state.scenes.map_scenes["Route43"], "1");
    }

    #[test]
    fn rejects_undeclared_numeric_scene_tokens_without_index_fallback() {
        let mut state = GameState::default();
        let error = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            Some("Route43"),
            &table(),
            command("setmapscene", Some("ROUTE_43"), Some("1")),
        )
        .expect_err("numeric scene token must be a declared scene id");

        assert_eq!(
            error,
            ScriptSceneError::Scene {
                error: SceneError::UnknownScene {
                    map_name: "Route43".to_string(),
                    scene_name: "1".to_string(),
                },
            }
        );
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

    #[test]
    fn rejects_malformed_scene_tokens_at_runtime() {
        let mut state = GameState::default();
        state
            .scenes
            .enter_map("Route43Gate", &table())
            .expect("enter map");

        assert_eq!(
            apply_script_scene_command(
                &mut state,
                "Route43Gate",
                None,
                &table(),
                command("setscene", None, Some("SCENE ROUTE43GATE_NOOP")),
            ),
            Err(ScriptSceneError::InvalidSceneId {
                command: "setscene".to_string(),
                scene_id: "SCENE ROUTE43GATE_NOOP".to_string(),
            })
        );
        assert_eq!(
            apply_script_scene_command(
                &mut state,
                "Route43Gate",
                Some("Route43"),
                &table(),
                command("setmapscene", Some("ROUTE 43"), Some("1")),
            ),
            Err(ScriptSceneError::InvalidTargetMap {
                command: "setmapscene".to_string(),
                map_id: "ROUTE 43".to_string(),
            })
        );
        assert_eq!(
            apply_script_scene_command(
                &mut state,
                "Route43Gate",
                None,
                &table(),
                command("SetScene", None, Some("SCENE_ROUTE43GATE_NOOP")),
            ),
            Err(ScriptSceneError::InvalidCommand {
                command: "SetScene".to_string(),
            })
        );
    }

    #[test]
    fn rejects_invalid_scene_source_script_before_scene_mutation() {
        let mut state = GameState::default();
        state
            .scenes
            .enter_map("Route43Gate", &table())
            .expect("enter map");
        let mut bad_source = command("setscene", None, Some("SCENE_ROUTE43GATE_NOOP"));
        bad_source.source_script = "legacy_script".to_string();

        assert_eq!(
            script_scene_command_issues(&bad_source),
            vec![ScriptSceneCommandIssue::InvalidSourceScript]
        );
        assert_eq!(
            apply_script_scene_command(&mut state, "Route43Gate", None, &table(), bad_source,),
            Err(ScriptSceneError::InvalidSourceScript {
                source_script: "legacy_script".to_string(),
            })
        );
        assert_eq!(state.scenes.scene_name, "SCENE_ROUTE43GATE_ROCKETS");
        assert_eq!(
            state.scenes.map_scenes["Route43Gate"],
            "SCENE_ROUTE43GATE_ROCKETS"
        );
    }

    #[test]
    fn script_scene_serialized_variants_reject_unknown_fallback_fields() {
        let error = serde_json::from_value::<ScriptSceneError>(serde_json::json!({
            "UnknownSceneToken": {
                "map_name": "Route43Gate",
                "scene_id": "SCENE_ROUTE43GATE_NOOP",
                "fallback_scene_id": "0"
            }
        }))
        .expect_err("fallback scene id must be rejected")
        .to_string();
        assert!(
            error.contains("unknown field `fallback_scene_id`"),
            "{error}"
        );
    }
}
