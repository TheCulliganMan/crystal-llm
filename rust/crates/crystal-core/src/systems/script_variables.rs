use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::GameState;
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptVariableCommand {
    #[serde(deserialize_with = "required_script_variable_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_script_variable_target_token")]
    pub target: Option<String>,
    #[serde(deserialize_with = "required_script_variable_value_token_vec")]
    pub value_tokens: Vec<String>,
    #[serde(deserialize_with = "required_script_variable_target_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptVariableCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptVariableCommand {
            #[serde(default, deserialize_with = "required_script_variable_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_script_variable_target_token")]
            target: Option<String>,
            #[serde(deserialize_with = "required_script_variable_value_token_vec")]
            value_tokens: Vec<String>,
            #[serde(deserialize_with = "required_script_variable_source_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptVariableCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            target: raw.target,
            value_tokens: raw.value_tokens,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        if !command.command.is_empty() {
            validate_script_variable_command(&command).map_err(D::Error::custom)?;
        }
        Ok(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptVariableOutcome {
    SetAccumulator {
        value: String,
        source_script: String,
        command_index: usize,
    },
    LoadVariable {
        variable: String,
        value: String,
        source_script: String,
        command_index: usize,
    },
    LoadMemory {
        memory: String,
        value: String,
        source_script: String,
        command_index: usize,
    },
    WriteMemory {
        memory: String,
        value: String,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptVariableCommandError {
    #[error("unknown script variable command '{command}'")]
    UnknownCommand { command: String },
    #[error("script variable command '{command}' is missing target")]
    MissingTarget { command: String },
    #[error("script variable command '{command}' has unexpected target")]
    UnexpectedTarget { command: String },
    #[error("script variable command '{command}' is missing value tokens")]
    MissingValue { command: String },
    #[error("script variable command '{command}' has unexpected value tokens")]
    UnexpectedValue { command: String },
    #[error("script variable command '{command}' references empty target")]
    EmptyTarget { command: String },
    #[error("script variable command '{command}' references invalid target '{target}'")]
    InvalidTarget { command: String, target: String },
    #[error("script variable command '{command}' has an empty value token")]
    EmptyValueToken { command: String },
    #[error("script variable command '{command}' has invalid value token '{token}'")]
    InvalidValueToken { command: String, token: String },
    #[error("script variable command source script '{source_script}' is invalid")]
    InvalidSourceScript { source_script: String },
    #[error("script variable '{variable}' is unset")]
    UnsetVariable { variable: String },
    #[error("script memory '{memory}' is unset")]
    UnsetMemory { memory: String },
    #[error("script accumulator is unset")]
    UnsetAccumulator,
    #[error("unknown script time token '{time_token}'")]
    UnknownTimeToken { time_token: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptVariableCommandIssue {
    pub source_script: String,
    pub command_index: usize,
    pub error: ScriptVariableCommandError,
}

pub fn script_variable_command_issues(
    commands: &[ScriptVariableCommand],
) -> Vec<ScriptVariableCommandIssue> {
    commands
        .iter()
        .filter_map(|command| {
            validate_script_variable_command(command)
                .err()
                .map(|error| ScriptVariableCommandIssue {
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                    error,
                })
        })
        .collect()
}

pub fn apply_script_variable_command(
    state: &mut GameState,
    command: ScriptVariableCommand,
    time_of_day: Option<TimeOfDay>,
) -> Result<ScriptVariableOutcome, ScriptVariableCommandError> {
    reject_invalid_source_script(&command)?;
    match command.command.as_str() {
        "setval" => {
            reject_target(&command)?;
            let value = require_joined_value(&command)?;
            state.script_runtime.script_value = Some(value.clone());
            Ok(ScriptVariableOutcome::SetAccumulator {
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "readvar" => {
            reject_value(&command)?;
            let variable = require_target(&command)?.to_string();
            let value = state
                .script_runtime
                .variables
                .get(&variable)
                .cloned()
                .ok_or_else(|| ScriptVariableCommandError::UnsetVariable {
                    variable: variable.clone(),
                })?;
            state.script_runtime.script_value = Some(value.clone());
            Ok(ScriptVariableOutcome::SetAccumulator {
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "loadvar" => {
            let variable = require_target(&command)?.to_string();
            let value = require_joined_value(&command)?;
            state
                .script_runtime
                .variables
                .insert(variable.clone(), value.clone());
            if variable == "VAR_BATTLETYPE" {
                state.pending_special_battle_type = Some(value.clone());
            }
            Ok(ScriptVariableOutcome::LoadVariable {
                variable,
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "readmem" => {
            reject_value(&command)?;
            let memory = require_target(&command)?.to_string();
            let value = state
                .script_runtime
                .memory
                .get(&memory)
                .cloned()
                .ok_or_else(|| ScriptVariableCommandError::UnsetMemory {
                    memory: memory.clone(),
                })?;
            state.script_runtime.script_value = Some(value.clone());
            Ok(ScriptVariableOutcome::SetAccumulator {
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "loadmem" => {
            let memory = require_target(&command)?.to_string();
            let value = require_joined_value(&command)?;
            state
                .script_runtime
                .memory
                .insert(memory.clone(), value.clone());
            Ok(ScriptVariableOutcome::LoadMemory {
                memory,
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "writemem" => {
            reject_value(&command)?;
            let memory = require_target(&command)?.to_string();
            let value = state
                .script_runtime
                .script_value
                .clone()
                .ok_or(ScriptVariableCommandError::UnsetAccumulator)?;
            state
                .script_runtime
                .memory
                .insert(memory.clone(), value.clone());
            Ok(ScriptVariableOutcome::WriteMemory {
                memory,
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "checktime" => {
            reject_target(&command)?;
            let token = require_single_value(&command)?;
            let expected = parse_time_mask(token)?;
            let active = time_of_day
                .is_some_and(|time_of_day| expected & time_of_day_mask(time_of_day) != 0);
            let value = if active { "TRUE" } else { "FALSE" }.to_string();
            state.script_runtime.script_value = Some(value.clone());
            Ok(ScriptVariableOutcome::SetAccumulator {
                value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        other => Err(ScriptVariableCommandError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn validate_script_variable_command(
    command: &ScriptVariableCommand,
) -> Result<(), ScriptVariableCommandError> {
    reject_invalid_source_script(command)?;
    if !is_exact_script_variable_command_token(&command.command) {
        return Err(ScriptVariableCommandError::UnknownCommand {
            command: command.command.clone(),
        });
    }
    match command.command.as_str() {
        "setval" => {
            reject_target(command)?;
            require_joined_value(command)?;
        }
        "readvar" | "readmem" | "writemem" => {
            require_target(command)?;
            reject_value(command)?;
        }
        "loadvar" | "loadmem" => {
            require_target(command)?;
            require_joined_value(command)?;
        }
        "checktime" => {
            reject_target(command)?;
            let token = require_single_value(command)?;
            parse_time_mask(token)?;
        }
        other => {
            return Err(ScriptVariableCommandError::UnknownCommand {
                command: other.to_string(),
            });
        }
    }
    Ok(())
}

fn reject_invalid_source_script(
    command: &ScriptVariableCommand,
) -> Result<(), ScriptVariableCommandError> {
    if is_exact_script_variable_source_token(&command.source_script) {
        Ok(())
    } else {
        Err(ScriptVariableCommandError::InvalidSourceScript {
            source_script: command.source_script.clone(),
        })
    }
}

fn require_target(command: &ScriptVariableCommand) -> Result<&str, ScriptVariableCommandError> {
    let target =
        command
            .target
            .as_deref()
            .ok_or_else(|| ScriptVariableCommandError::MissingTarget {
                command: command.command.clone(),
            })?;
    if target.is_empty() {
        return Err(ScriptVariableCommandError::EmptyTarget {
            command: command.command.clone(),
        });
    }
    if !is_exact_script_variable_target_token(target) {
        return Err(ScriptVariableCommandError::InvalidTarget {
            command: command.command.clone(),
            target: target.to_string(),
        });
    }
    Ok(target)
}

fn reject_target(command: &ScriptVariableCommand) -> Result<(), ScriptVariableCommandError> {
    if command.target.is_some() {
        Err(ScriptVariableCommandError::UnexpectedTarget {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn require_joined_value(
    command: &ScriptVariableCommand,
) -> Result<String, ScriptVariableCommandError> {
    if command.value_tokens.is_empty() {
        return Err(ScriptVariableCommandError::MissingValue {
            command: command.command.clone(),
        });
    }
    if command.value_tokens.iter().any(|token| token.is_empty()) {
        return Err(ScriptVariableCommandError::EmptyValueToken {
            command: command.command.clone(),
        });
    }
    if let Some(token) = command
        .value_tokens
        .iter()
        .find(|token| !is_exact_script_variable_value_token(token))
    {
        return Err(ScriptVariableCommandError::InvalidValueToken {
            command: command.command.clone(),
            token: token.clone(),
        });
    }
    Ok(command.value_tokens.join(" "))
}

fn require_single_value(
    command: &ScriptVariableCommand,
) -> Result<&str, ScriptVariableCommandError> {
    require_joined_value(command)?;
    if command.value_tokens.len() != 1 {
        return Err(ScriptVariableCommandError::UnexpectedValue {
            command: command.command.clone(),
        });
    }
    Ok(command.value_tokens[0].as_str())
}

fn reject_value(command: &ScriptVariableCommand) -> Result<(), ScriptVariableCommandError> {
    if command.value_tokens.is_empty() {
        Ok(())
    } else {
        Err(ScriptVariableCommandError::UnexpectedValue {
            command: command.command.clone(),
        })
    }
}

fn parse_time_mask(token: &str) -> Result<u8, ScriptVariableCommandError> {
    if token == "ANYTIME" {
        return Ok(time_of_day_mask(TimeOfDay::Morning)
            | time_of_day_mask(TimeOfDay::Day)
            | time_of_day_mask(TimeOfDay::Night));
    }
    let mut mask = 0;
    for part in token.split('|') {
        if part.is_empty() {
            return Err(ScriptVariableCommandError::UnknownTimeToken {
                time_token: token.to_string(),
            });
        }
        mask |= match part {
            "MORN" => time_of_day_mask(TimeOfDay::Morning),
            "DAY" => time_of_day_mask(TimeOfDay::Day),
            "NITE" => time_of_day_mask(TimeOfDay::Night),
            other => {
                return Err(ScriptVariableCommandError::UnknownTimeToken {
                    time_token: other.to_string(),
                });
            }
        };
    }
    Ok(mask)
}

fn time_of_day_mask(time_of_day: TimeOfDay) -> u8 {
    match time_of_day {
        TimeOfDay::Morning => 0b001,
        TimeOfDay::Day => 0b010,
        TimeOfDay::Night => 0b100,
    }
}

fn is_exact_script_variable_target_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_variable_source_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_variable_value_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_variable_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
        && !has_reserved_pack_prefix(value)
}

fn required_script_variable_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_variable_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script variable command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_script_variable_target_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_variable_target_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script variable target must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_script_variable_source_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_variable_source_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script variable source must be exact ASM label syntax, found {value:?}"
        )))
    }
}

fn required_nullable_script_variable_target_token<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_script_variable_target_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script variable target must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_script_variable_value_token_vec<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if let Some(token) = values
        .iter()
        .find(|token| !is_exact_script_variable_value_token(token))
    {
        Err(serde::de::Error::custom(format!(
            "script variable value token must be exact visible ASCII, found {token:?}"
        )))
    } else {
        Ok(values)
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(name: &str, target: Option<&str>, value_tokens: &[&str]) -> ScriptVariableCommand {
        ScriptVariableCommand {
            command: name.to_string(),
            target: target.map(str::to_string),
            value_tokens: value_tokens
                .iter()
                .map(|token| (*token).to_string())
                .collect(),
            source_script: "VarScript".to_string(),
            command_index: 5,
        }
    }

    #[test]
    fn set_read_and_load_variable_commands_use_exact_ids() {
        let mut state = GameState::default();
        let load = apply_script_variable_command(
            &mut state,
            command("loadvar", Some("VAR_CALLERID"), &["PHONE_BIRDKEEPER_VANCE"]),
            None,
        )
        .expect("load var");
        assert_eq!(
            load,
            ScriptVariableOutcome::LoadVariable {
                variable: "VAR_CALLERID".to_string(),
                value: "PHONE_BIRDKEEPER_VANCE".to_string(),
                source_script: "VarScript".to_string(),
                command_index: 5,
            }
        );

        apply_script_variable_command(
            &mut state,
            command("readvar", Some("VAR_CALLERID"), &[]),
            None,
        )
        .expect("read var");
        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some("PHONE_BIRDKEEPER_VANCE")
        );

        assert!(matches!(
            apply_script_variable_command(
                &mut state,
                command("readvar", Some("var_callerid"), &[]),
                None,
            ),
            Err(ScriptVariableCommandError::UnsetVariable { .. })
        ));
    }

    #[test]
    fn loadvar_battletype_sets_pending_battle_type_without_aliasing() {
        let mut state = GameState::default();
        let outcome = apply_script_variable_command(
            &mut state,
            command(
                "loadvar",
                Some("VAR_BATTLETYPE"),
                &["BATTLETYPE_FORCESHINY"],
            ),
            None,
        )
        .expect("load battle type");

        assert_eq!(
            outcome,
            ScriptVariableOutcome::LoadVariable {
                variable: "VAR_BATTLETYPE".to_string(),
                value: "BATTLETYPE_FORCESHINY".to_string(),
                source_script: "VarScript".to_string(),
                command_index: 5,
            }
        );
        assert_eq!(
            state.script_runtime.variables.get("VAR_BATTLETYPE"),
            Some(&"BATTLETYPE_FORCESHINY".to_string())
        );
        assert_eq!(
            state.pending_special_battle_type.as_deref(),
            Some("BATTLETYPE_FORCESHINY")
        );

        apply_script_variable_command(
            &mut state,
            command("loadvar", Some("var_battletype"), &["BATTLETYPE_NORMAL"]),
            None,
        )
        .expect("lowercase variable is just a distinct exact variable");
        assert_eq!(
            state.pending_special_battle_type.as_deref(),
            Some("BATTLETYPE_FORCESHINY")
        );
    }

    #[test]
    fn memory_commands_write_exact_labels_without_aliasing() {
        let mut state = GameState::default();
        apply_script_variable_command(
            &mut state,
            command("loadmem", Some("wVanceFightCount"), &["2"]),
            None,
        )
        .expect("load mem");
        apply_script_variable_command(
            &mut state,
            command("readmem", Some("wVanceFightCount"), &[]),
            None,
        )
        .expect("read mem");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));

        apply_script_variable_command(&mut state, command("setval", None, &["TRUE"]), None)
            .expect("set accumulator");
        apply_script_variable_command(
            &mut state,
            command("writemem", Some("wMooMooBerries"), &[]),
            None,
        )
        .expect("write mem");
        assert_eq!(state.script_runtime.memory["wMooMooBerries"], "TRUE");
    }

    #[test]
    fn checktime_uses_exact_time_tokens() {
        let mut state = GameState::default();
        apply_script_variable_command(
            &mut state,
            command("checktime", None, &["NITE"]),
            Some(TimeOfDay::Night),
        )
        .expect("night");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("TRUE"));

        apply_script_variable_command(
            &mut state,
            command("checktime", None, &["DAY"]),
            Some(TimeOfDay::Night),
        )
        .expect("day at night");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("FALSE"));

        assert!(matches!(
            validate_script_variable_command(&command("checktime", None, &["night"])),
            Err(ScriptVariableCommandError::UnknownTimeToken { .. })
        ));
        assert_eq!(
            apply_script_variable_command(&mut state, command("checktime", None, &["night"]), None),
            Err(ScriptVariableCommandError::UnknownTimeToken {
                time_token: "night".to_string(),
            })
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("FALSE"));
    }

    #[test]
    fn checktime_uses_exact_time_masks() {
        let mut state = GameState::default();
        apply_script_variable_command(
            &mut state,
            command("checktime", None, &["MORN|NITE"]),
            Some(TimeOfDay::Night),
        )
        .expect("night matches combined mask");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("TRUE"));

        apply_script_variable_command(
            &mut state,
            command("checktime", None, &["MORN|NITE"]),
            Some(TimeOfDay::Day),
        )
        .expect("day misses combined mask");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("FALSE"));

        apply_script_variable_command(
            &mut state,
            command("checktime", None, &["ANYTIME"]),
            Some(TimeOfDay::Day),
        )
        .expect("anytime matches exact symbolic mask");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("TRUE"));

        assert_eq!(
            validate_script_variable_command(&command("checktime", None, &["MORN||DAY"])),
            Err(ScriptVariableCommandError::UnknownTimeToken {
                time_token: "MORN||DAY".to_string(),
            })
        );
        assert_eq!(
            validate_script_variable_command(&command("checktime", None, &["MORN|late"])),
            Err(ScriptVariableCommandError::UnknownTimeToken {
                time_token: "late".to_string(),
            })
        );
        assert_eq!(
            validate_script_variable_command(&command("checktime", None, &["morn|nite"])),
            Err(ScriptVariableCommandError::UnknownTimeToken {
                time_token: "morn".to_string(),
            })
        );
    }

    #[test]
    fn rejects_padded_targets_and_value_tokens_without_normalization() {
        assert!(matches!(
            validate_script_variable_command(&command("readvar", Some(" VAR_CALLERID"), &[])),
            Err(ScriptVariableCommandError::InvalidTarget { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command("readvar", Some("VAR CALLERID"), &[])),
            Err(ScriptVariableCommandError::InvalidTarget { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command("setval", None, &[" TRUE"])),
            Err(ScriptVariableCommandError::InvalidValueToken { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command("setval", None, &["PHONE BIRDKEEPER_VANCE"])),
            Err(ScriptVariableCommandError::InvalidValueToken { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command("checktime", None, &[" NITE"])),
            Err(ScriptVariableCommandError::InvalidValueToken { .. })
        ));

        let mut state = GameState::default();
        assert!(matches!(
            apply_script_variable_command(
                &mut state,
                command("loadmem", Some(" wVanceFightCount"), &["2"]),
                None,
            ),
            Err(ScriptVariableCommandError::InvalidTarget { .. })
        ));
        assert!(state.script_runtime.memory.is_empty());
    }

    #[test]
    fn rejects_reserved_variable_tokens_without_pack_fallbacks() {
        assert!(matches!(
            validate_script_variable_command(&command(
                "loadvar",
                Some("fallback_variable"),
                &["PHONE_BIRDKEEPER_VANCE"]
            )),
            Err(ScriptVariableCommandError::InvalidTarget { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command(
                "loadvar",
                Some("VAR_CALLERID"),
                &["legacy_phone"]
            )),
            Err(ScriptVariableCommandError::InvalidValueToken { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command("fallbackset", None, &["TRUE"])),
            Err(ScriptVariableCommandError::UnknownCommand { .. })
        ));

        for (field, value) in [
            ("command", serde_json::json!("fallbackset")),
            ("target", serde_json::json!("legacy_target")),
            ("value_tokens", serde_json::json!(["fallback_value"])),
            ("source_script", serde_json::json!("legacy_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "loadvar",
                "target": "VAR_CALLERID",
                "value_tokens": ["PHONE_BIRDKEEPER_VANCE"],
                "source_script": "VarScript",
                "command_index": 5
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptVariableCommand>(payload)
                .expect_err("reserved script variable command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script variable"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn rejects_invalid_source_script_before_variable_state_mutation() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("UNCHANGED".to_string());
        let mut bad_source = command(
            "loadvar",
            Some("VAR_BATTLETYPE"),
            &["BATTLETYPE_FORCESHINY"],
        );
        bad_source.source_script = "fallback_script".to_string();

        assert_eq!(
            validate_script_variable_command(&bad_source),
            Err(ScriptVariableCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );
        assert_eq!(
            apply_script_variable_command(&mut state, bad_source, None),
            Err(ScriptVariableCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );

        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some("UNCHANGED")
        );
        assert!(state.script_runtime.variables.is_empty());
        assert!(state.script_runtime.memory.is_empty());
        assert!(state.pending_special_battle_type.is_none());
    }

    #[test]
    fn script_variable_command_issues_preserve_exact_source_positions() {
        let commands = vec![
            command("checktime", None, &["night"]),
            command("readvar", Some(""), &[]),
            command("readmem", Some(" wVanceFightCount"), &[]),
            command("loadvar", Some("VAR CALLERID"), &["PHONE_BIRDKEEPER_VANCE"]),
            command("setval", None, &[" TRUE"]),
            command("setval", None, &["PHONE BIRDKEEPER_VANCE"]),
            command("setval", Some("VAR_BADGES"), &["7"]),
            command("loadvar", Some("VAR_CALLERID"), &["PHONE_BIRDKEEPER_VANCE"]),
        ];

        assert_eq!(
            script_variable_command_issues(&commands),
            vec![
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::UnknownTimeToken {
                        time_token: "night".to_string(),
                    },
                },
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::EmptyTarget {
                        command: "readvar".to_string(),
                    },
                },
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::InvalidTarget {
                        command: "readmem".to_string(),
                        target: " wVanceFightCount".to_string(),
                    },
                },
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::InvalidTarget {
                        command: "loadvar".to_string(),
                        target: "VAR CALLERID".to_string(),
                    },
                },
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::InvalidValueToken {
                        command: "setval".to_string(),
                        token: " TRUE".to_string(),
                    },
                },
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::InvalidValueToken {
                        command: "setval".to_string(),
                        token: "PHONE BIRDKEEPER_VANCE".to_string(),
                    },
                },
                ScriptVariableCommandIssue {
                    source_script: "VarScript".to_string(),
                    command_index: 5,
                    error: ScriptVariableCommandError::UnexpectedTarget {
                        command: "setval".to_string(),
                    },
                },
            ]
        );
    }

    #[test]
    fn script_variable_serialized_variants_reject_unknown_fallback_fields() {
        let outcome_error = serde_json::from_value::<ScriptVariableOutcome>(serde_json::json!({
            "load_variable": {
                "variable": "VAR_CALLERID",
                "value": "PHONE_BIRDKEEPER_VANCE",
                "source_script": "VarScript",
                "command_index": 5,
                "fallback_value": "PHONE_NONE"
            }
        }))
        .expect_err("fallback variable value must be rejected")
        .to_string();
        assert!(
            outcome_error.contains("unknown field `fallback_value`"),
            "{outcome_error}"
        );

        let error_error = serde_json::from_value::<ScriptVariableCommandError>(serde_json::json!({
            "InvalidTarget": {
                "command": "loadvar",
                "target": "VAR CALLERID",
                "normalized_target": "VAR_CALLERID"
            }
        }))
        .expect_err("normalized target must be rejected")
        .to_string();
        assert!(
            error_error.contains("unknown field `normalized_target`"),
            "{error_error}"
        );
    }
}
