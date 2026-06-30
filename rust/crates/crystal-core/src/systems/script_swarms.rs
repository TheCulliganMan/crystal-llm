use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::state::{GameState, SwarmMapTarget};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSwarmCommand {
    #[serde(deserialize_with = "required_script_swarm_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_script_swarm_token")]
    pub swarm_token: String,
    #[serde(deserialize_with = "required_script_swarm_token")]
    pub map_id: String,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSwarmOutcome {
    pub command: String,
    pub swarm_token: String,
    pub map_id: String,
    pub map_group: Option<u16>,
    pub map_number: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptSwarmError {
    #[error("invalid script swarm command '{command}'")]
    InvalidCommand { command: String },
    #[error("unknown script swarm command '{command}'")]
    UnknownCommand { command: String },
    #[error("script swarm command references unknown map '{map_id}'")]
    UnknownMap { map_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptSwarmCommandIssue {
    InvalidCommand,
    UnknownCommand,
    InvalidSwarmToken,
    InvalidMapId,
    UnknownMap,
}

pub const SCRIPT_SWARM_COMMANDS: &[&str] = &["swarm"];

pub fn is_known_script_swarm_command(command: &str) -> bool {
    SCRIPT_SWARM_COMMANDS.contains(&command)
}

pub fn script_swarm_command_issues(
    command: &ScriptSwarmCommand,
    map_ids: &BTreeSet<String>,
) -> Vec<ScriptSwarmCommandIssue> {
    let mut issues = Vec::new();
    if !is_exact_script_swarm_command_token(&command.command) {
        issues.push(ScriptSwarmCommandIssue::InvalidCommand);
    } else if !is_known_script_swarm_command(&command.command) {
        issues.push(ScriptSwarmCommandIssue::UnknownCommand);
    }
    if !is_exact_script_swarm_token(&command.swarm_token) {
        issues.push(ScriptSwarmCommandIssue::InvalidSwarmToken);
    }
    if !is_exact_script_swarm_token(&command.map_id) {
        issues.push(ScriptSwarmCommandIssue::InvalidMapId);
    } else if !map_ids.contains(&command.map_id) {
        issues.push(ScriptSwarmCommandIssue::UnknownMap);
    }
    issues
}

pub fn apply_script_swarm_command(
    state: &mut GameState,
    command: ScriptSwarmCommand,
    map_groups: &BTreeMap<String, (u16, u16)>,
) -> Result<ScriptSwarmOutcome, ScriptSwarmError> {
    if !is_exact_script_swarm_command_token(&command.command) {
        return Err(ScriptSwarmError::InvalidCommand {
            command: command.command,
        });
    }
    if !is_known_script_swarm_command(&command.command) {
        return Err(ScriptSwarmError::UnknownCommand {
            command: command.command,
        });
    }
    let Some((group, number)) = map_groups.get(&command.map_id).copied() else {
        return Err(ScriptSwarmError::UnknownMap {
            map_id: command.map_id,
        });
    };
    state.swarms.active.insert(
        command.swarm_token.clone(),
        SwarmMapTarget {
            map_id: command.map_id.clone(),
            map_group: Some(group),
            map_number: Some(number),
        },
    );
    let target = state
        .swarms
        .active
        .get(&command.swarm_token)
        .expect("inserted swarm target");
    Ok(ScriptSwarmOutcome {
        command: command.command,
        swarm_token: command.swarm_token,
        map_id: target.map_id.clone(),
        map_group: target.map_group,
        map_number: target.map_number,
        source_script: command.source_script,
        command_index: command.command_index,
    })
}

fn is_exact_script_swarm_command_token(value: &str) -> bool {
    value == "swarm"
}

fn is_exact_script_swarm_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn required_script_swarm_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_swarm_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script swarm command must be exactly 'swarm', found {value:?}"
        )))
    }
}

fn required_script_swarm_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_swarm_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script swarm token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_script_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    required_script_swarm_token(deserializer)
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(name: &str, swarm_token: &str, map_id: &str) -> ScriptSwarmCommand {
        ScriptSwarmCommand {
            command: name.to_string(),
            swarm_token: swarm_token.to_string(),
            map_id: map_id.to_string(),
            source_script: "SwarmScript".to_string(),
            command_index: 2,
        }
    }

    #[test]
    fn applies_exact_swarm_command_to_authoritative_state() {
        let mut state = GameState::default();
        let maps = BTreeMap::from([("ROUTE_35".to_string(), (3, 18))]);

        let outcome = apply_script_swarm_command(
            &mut state,
            command("swarm", "SWARM_YANMA", "ROUTE_35"),
            &maps,
        )
        .expect("swarm");

        assert_eq!(
            outcome,
            ScriptSwarmOutcome {
                command: "swarm".to_string(),
                swarm_token: "SWARM_YANMA".to_string(),
                map_id: "ROUTE_35".to_string(),
                map_group: Some(3),
                map_number: Some(18),
                source_script: "SwarmScript".to_string(),
                command_index: 2,
            }
        );
        assert_eq!(
            state.swarms.active.get("SWARM_YANMA"),
            Some(&SwarmMapTarget {
                map_id: "ROUTE_35".to_string(),
                map_group: Some(3),
                map_number: Some(18),
            })
        );
    }

    #[test]
    fn rejects_unknown_or_case_changed_swarm_payloads_without_resolution() {
        let mut state = GameState::default();
        let maps = BTreeMap::from([("ROUTE_35".to_string(), (3, 18))]);
        assert_eq!(
            apply_script_swarm_command(
                &mut state,
                command("Swarm", "SWARM_YANMA", "ROUTE_35"),
                &maps,
            ),
            Err(ScriptSwarmError::InvalidCommand {
                command: "Swarm".to_string(),
            })
        );
        assert_eq!(
            apply_script_swarm_command(
                &mut state,
                command("swarm", "SWARM_YANMA", "route_35"),
                &maps,
            ),
            Err(ScriptSwarmError::UnknownMap {
                map_id: "route_35".to_string(),
            })
        );
    }
}
