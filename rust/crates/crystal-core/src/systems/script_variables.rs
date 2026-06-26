use serde::{Deserialize, Serialize};

use crate::state::GameState;
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptVariableCommand {
    pub command: String,
    pub target: Option<String>,
    pub value_tokens: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
            let expected = parse_time_token(token)?;
            let active = time_of_day.is_some_and(|time_of_day| time_of_day == expected);
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
            parse_time_token(token)?;
        }
        other => {
            return Err(ScriptVariableCommandError::UnknownCommand {
                command: other.to_string(),
            });
        }
    }
    Ok(())
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
    if target.trim() != target {
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
        .find(|token| token.trim() != **token)
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

fn parse_time_token(token: &str) -> Result<TimeOfDay, ScriptVariableCommandError> {
    match token {
        "MORN" => Ok(TimeOfDay::Morning),
        "DAY" => Ok(TimeOfDay::Day),
        "NITE" => Ok(TimeOfDay::Night),
        other => Err(ScriptVariableCommandError::UnknownTimeToken {
            time_token: other.to_string(),
        }),
    }
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
    fn rejects_padded_targets_and_value_tokens_without_normalization() {
        assert!(matches!(
            validate_script_variable_command(&command("readvar", Some(" VAR_CALLERID"), &[])),
            Err(ScriptVariableCommandError::InvalidTarget { .. })
        ));
        assert!(matches!(
            validate_script_variable_command(&command("setval", None, &[" TRUE"])),
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
    fn script_variable_command_issues_preserve_exact_source_positions() {
        let commands = vec![
            command("checktime", None, &["night"]),
            command("readvar", Some(""), &[]),
            command("readmem", Some(" wVanceFightCount"), &[]),
            command("setval", None, &[" TRUE"]),
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
                    error: ScriptVariableCommandError::InvalidValueToken {
                        command: "setval".to_string(),
                        token: " TRUE".to_string(),
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
}
