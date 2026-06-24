use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::state::{
    GameState, ScriptControlRuntimeEvent, ScriptControlRuntimeKind, ScriptEndState,
    ScriptReturnFrame,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptControlCommand {
    pub command: String,
    pub compare_value: Option<String>,
    pub target_label: Option<String>,
    pub resolved_target_script: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptControlAction {
    Continue {
        source_script: String,
        command_index: usize,
    },
    Jump {
        target_script: String,
        call: bool,
        deferred: bool,
        standard: bool,
        source_script: String,
        command_index: usize,
    },
    End {
        callback: bool,
        just_battled_guard: bool,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ScriptControlCommandError {
    #[error("unknown script control command '{command}'")]
    UnknownCommand { command: String },
    #[error("script control command '{command}' is missing target label")]
    MissingTarget { command: String },
    #[error("script control command '{command}' has unexpected target label")]
    UnexpectedTarget { command: String },
    #[error("script control command '{command}' references empty target label")]
    EmptyTarget { command: String },
    #[error("script control command '{command}' is missing compare value")]
    MissingCompareValue { command: String },
    #[error("script control command '{command}' has unexpected compare value")]
    UnexpectedCompareValue { command: String },
    #[error("script control command '{command}' references empty compare value")]
    EmptyCompareValue { command: String },
    #[error("script control command '{command}' is missing resolved target script")]
    MissingResolvedTarget { command: String },
    #[error("script accumulator is unset for '{command}'")]
    UnsetAccumulator { command: String },
    #[error("script accumulator value '{value}' is not an exact TRUE/FALSE token")]
    UnknownBoolean { value: String },
    #[error("cannot resolve numeric script token '{token}'")]
    UnknownNumericToken { token: String },
}

pub fn resolve_script_control_command(
    state: &GameState,
    command: ScriptControlCommand,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<ScriptControlAction, ScriptControlCommandError> {
    validate_script_control_command(&command)?;
    match command.command.as_str() {
        "ifequal" => branch(
            state,
            &command,
            accumulator(state, &command)? == require_compare_value(&command)?,
        ),
        "ifnotequal" => branch(
            state,
            &command,
            accumulator(state, &command)? != require_compare_value(&command)?,
        ),
        "iftrue" => branch(state, &command, require_boolean(state, &command)?),
        "iffalse" => branch(state, &command, !require_boolean(state, &command)?),
        "ifgreater" => {
            let left = parse_numeric_token(accumulator(state, &command)?, numeric_constants)?;
            let right = parse_numeric_token(require_compare_value(&command)?, numeric_constants)?;
            branch(state, &command, left > right)
        }
        "ifless" => {
            let left = parse_numeric_token(accumulator(state, &command)?, numeric_constants)?;
            let right = parse_numeric_token(require_compare_value(&command)?, numeric_constants)?;
            branch(state, &command, left < right)
        }
        "sjump" | "jump" | "scall" | "sdefer" | "jumpstd" => Ok(jump_action(command)?),
        "end" | "endcallback" | "endifjustbattled" => Ok(ScriptControlAction::End {
            callback: command.command == "endcallback",
            just_battled_guard: command.command == "endifjustbattled",
            source_script: command.source_script,
            command_index: command.command_index,
        }),
        other => Err(ScriptControlCommandError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn apply_script_control_command(
    state: &mut GameState,
    command: ScriptControlCommand,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<ScriptControlAction, ScriptControlCommandError> {
    let action = resolve_script_control_command(state, command, numeric_constants)?;
    apply_script_control_action_to_state(state, &action);
    Ok(action)
}

pub fn apply_script_control_action_to_state(state: &mut GameState, action: &ScriptControlAction) {
    match action {
        ScriptControlAction::Continue {
            source_script,
            command_index,
        } => {
            state
                .script_runtime
                .control_events
                .push(ScriptControlRuntimeEvent {
                    kind: ScriptControlRuntimeKind::Continue,
                    target_script: None,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptControlAction::Jump {
            target_script,
            call,
            deferred,
            standard,
            source_script,
            command_index,
        } => {
            let kind = if *deferred {
                state
                    .script_runtime
                    .deferred_scripts
                    .push(target_script.clone());
                ScriptControlRuntimeKind::Defer
            } else if *call {
                state.script_runtime.call_stack.push(ScriptReturnFrame {
                    source_script: source_script.clone(),
                    next_command_index: command_index + 1,
                });
                state.script_runtime.next_script = Some(target_script.clone());
                ScriptControlRuntimeKind::Call
            } else if *standard {
                state.script_runtime.next_script = Some(target_script.clone());
                ScriptControlRuntimeKind::StandardJump
            } else {
                state.script_runtime.next_script = Some(target_script.clone());
                ScriptControlRuntimeKind::Jump
            };
            state
                .script_runtime
                .control_events
                .push(ScriptControlRuntimeEvent {
                    kind,
                    target_script: Some(target_script.clone()),
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptControlAction::End {
            callback,
            just_battled_guard,
            source_script,
            command_index,
        } => {
            state.script_runtime.script_ended = Some(ScriptEndState {
                callback: *callback,
                just_battled_guard: *just_battled_guard,
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state.script_runtime.next_script = None;
            state
                .script_runtime
                .control_events
                .push(ScriptControlRuntimeEvent {
                    kind: ScriptControlRuntimeKind::End,
                    target_script: None,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
    }
}

pub fn validate_script_control_command(
    command: &ScriptControlCommand,
) -> Result<(), ScriptControlCommandError> {
    match command.command.as_str() {
        "ifequal" | "ifnotequal" | "ifgreater" | "ifless" => {
            require_compare_value(command)?;
            require_target(command)?;
            require_resolved_target(command)?;
        }
        "iftrue" | "iffalse" | "sjump" | "jump" | "scall" | "sdefer" => {
            reject_compare_value(command)?;
            require_target(command)?;
            require_resolved_target(command)?;
        }
        "jumpstd" => {
            reject_compare_value(command)?;
            require_target(command)?;
        }
        "end" | "endcallback" | "endifjustbattled" => {
            reject_compare_value(command)?;
            reject_target(command)?;
            if command.resolved_target_script.is_some() {
                return Err(ScriptControlCommandError::UnexpectedTarget {
                    command: command.command.clone(),
                });
            }
        }
        other => {
            return Err(ScriptControlCommandError::UnknownCommand {
                command: other.to_string(),
            });
        }
    }
    Ok(())
}

fn branch(
    _state: &GameState,
    command: &ScriptControlCommand,
    taken: bool,
) -> Result<ScriptControlAction, ScriptControlCommandError> {
    if taken {
        Ok(jump_action(command.clone())?)
    } else {
        Ok(ScriptControlAction::Continue {
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        })
    }
}

fn jump_action(
    command: ScriptControlCommand,
) -> Result<ScriptControlAction, ScriptControlCommandError> {
    let target_script = if command.command == "jumpstd" {
        require_target(&command)?.to_string()
    } else {
        require_resolved_target(&command)?.to_string()
    };
    Ok(ScriptControlAction::Jump {
        target_script,
        call: command.command == "scall",
        deferred: command.command == "sdefer",
        standard: command.command == "jumpstd",
        source_script: command.source_script,
        command_index: command.command_index,
    })
}

fn accumulator<'a>(
    state: &'a GameState,
    command: &ScriptControlCommand,
) -> Result<&'a str, ScriptControlCommandError> {
    state.script_runtime.script_value.as_deref().ok_or_else(|| {
        ScriptControlCommandError::UnsetAccumulator {
            command: command.command.clone(),
        }
    })
}

fn require_boolean(
    state: &GameState,
    command: &ScriptControlCommand,
) -> Result<bool, ScriptControlCommandError> {
    match accumulator(state, command)? {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        value => Err(ScriptControlCommandError::UnknownBoolean {
            value: value.to_string(),
        }),
    }
}

fn require_compare_value(
    command: &ScriptControlCommand,
) -> Result<&str, ScriptControlCommandError> {
    let value = command.compare_value.as_deref().ok_or_else(|| {
        ScriptControlCommandError::MissingCompareValue {
            command: command.command.clone(),
        }
    })?;
    if value.is_empty() {
        return Err(ScriptControlCommandError::EmptyCompareValue {
            command: command.command.clone(),
        });
    }
    Ok(value)
}

fn reject_compare_value(command: &ScriptControlCommand) -> Result<(), ScriptControlCommandError> {
    if command.compare_value.is_some() {
        Err(ScriptControlCommandError::UnexpectedCompareValue {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn require_target(command: &ScriptControlCommand) -> Result<&str, ScriptControlCommandError> {
    let target = command.target_label.as_deref().ok_or_else(|| {
        ScriptControlCommandError::MissingTarget {
            command: command.command.clone(),
        }
    })?;
    if target.is_empty() {
        return Err(ScriptControlCommandError::EmptyTarget {
            command: command.command.clone(),
        });
    }
    Ok(target)
}

fn reject_target(command: &ScriptControlCommand) -> Result<(), ScriptControlCommandError> {
    if command.target_label.is_some() {
        Err(ScriptControlCommandError::UnexpectedTarget {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn require_resolved_target(
    command: &ScriptControlCommand,
) -> Result<&str, ScriptControlCommandError> {
    command.resolved_target_script.as_deref().ok_or_else(|| {
        ScriptControlCommandError::MissingResolvedTarget {
            command: command.command.clone(),
        }
    })
}

fn parse_numeric_token(
    token: &str,
    constants: &BTreeMap<String, i32>,
) -> Result<i32, ScriptControlCommandError> {
    let parts: Vec<&str> = token.split_whitespace().collect();
    match parts.as_slice() {
        [single] => parse_numeric_atom(single, constants),
        [left, "-", right] => {
            Ok(parse_numeric_atom(left, constants)? - parse_numeric_atom(right, constants)?)
        }
        [left, "+", right] => {
            Ok(parse_numeric_atom(left, constants)? + parse_numeric_atom(right, constants)?)
        }
        _ => Err(ScriptControlCommandError::UnknownNumericToken {
            token: token.to_string(),
        }),
    }
}

fn parse_numeric_atom(
    token: &str,
    constants: &BTreeMap<String, i32>,
) -> Result<i32, ScriptControlCommandError> {
    if let Some(value) = constants.get(token) {
        return Ok(*value);
    }
    if let Some(hex) = token.strip_prefix('$') {
        return i32::from_str_radix(hex, 16).map_err(|_| {
            ScriptControlCommandError::UnknownNumericToken {
                token: token.to_string(),
            }
        });
    }
    token
        .parse::<i32>()
        .map_err(|_| ScriptControlCommandError::UnknownNumericToken {
            token: token.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(name: &str, compare: Option<&str>, target: Option<&str>) -> ScriptControlCommand {
        ScriptControlCommand {
            command: name.to_string(),
            compare_value: compare.map(str::to_string),
            target_label: target.map(str::to_string),
            resolved_target_script: target.map(|target| format!("{target}@Script")),
            source_script: "Script".to_string(),
            command_index: 6,
        }
    }

    #[test]
    fn resolves_exact_accumulator_branches_and_jumps() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("SATURDAY".to_string());
        assert_eq!(
            resolve_script_control_command(
                &state,
                command("ifequal", Some("SATURDAY"), Some(".Done")),
                &BTreeMap::new(),
            )
            .expect("branch"),
            ScriptControlAction::Jump {
                target_script: ".Done@Script".to_string(),
                call: false,
                deferred: false,
                standard: false,
                source_script: "Script".to_string(),
                command_index: 6,
            }
        );
        assert!(matches!(
            resolve_script_control_command(
                &state,
                command("ifnotequal", Some("SATURDAY"), Some(".Done")),
                &BTreeMap::new(),
            ),
            Ok(ScriptControlAction::Continue { .. })
        ));
    }

    #[test]
    fn resolves_boolean_numeric_and_standard_jumps() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("TRUE".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                command("iftrue", None, Some(".Yes")),
                &BTreeMap::new(),
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));

        state.script_runtime.script_value = Some("8".to_string());
        let constants = BTreeMap::from([("NUM_JOHTO_BADGES".to_string(), 8)]);
        assert!(matches!(
            resolve_script_control_command(
                &state,
                command("ifgreater", Some("NUM_JOHTO_BADGES - 1"), Some(".AllEight")),
                &constants,
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));

        let mut jumpstd = command("jumpstd", None, Some("PokecenterSignScript"));
        jumpstd.resolved_target_script = None;
        assert_eq!(
            resolve_script_control_command(&state, jumpstd, &constants).expect("jumpstd"),
            ScriptControlAction::Jump {
                target_script: "PokecenterSignScript".to_string(),
                call: false,
                deferred: false,
                standard: true,
                source_script: "Script".to_string(),
                command_index: 6,
            }
        );
    }

    #[test]
    fn rejects_unset_or_case_changed_boolean_accumulators() {
        let state = GameState::default();
        assert!(matches!(
            resolve_script_control_command(
                &state,
                command("iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::UnsetAccumulator { .. })
        ));

        let mut state = GameState::default();
        state.script_runtime.script_value = Some("true".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                command("iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::UnknownBoolean { .. })
        ));
    }

    #[test]
    fn applies_jump_call_defer_and_standard_control_actions() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("TRUE".to_string());
        apply_script_control_command(
            &mut state,
            command("iftrue", None, Some(".Yes")),
            &BTreeMap::new(),
        )
        .expect("jump");
        assert_eq!(
            state.script_runtime.next_script.as_deref(),
            Some(".Yes@Script")
        );
        assert_eq!(
            state.script_runtime.control_events[0].kind,
            ScriptControlRuntimeKind::Jump
        );

        apply_script_control_command(
            &mut state,
            command("scall", None, Some(".Call")),
            &BTreeMap::new(),
        )
        .expect("call");
        assert_eq!(
            state.script_runtime.call_stack,
            vec![ScriptReturnFrame {
                source_script: "Script".to_string(),
                next_command_index: 7,
            }]
        );
        assert_eq!(
            state.script_runtime.next_script.as_deref(),
            Some(".Call@Script")
        );

        apply_script_control_command(
            &mut state,
            command("sdefer", None, Some(".Deferred")),
            &BTreeMap::new(),
        )
        .expect("defer");
        assert_eq!(
            state.script_runtime.deferred_scripts,
            vec![".Deferred@Script"]
        );

        let mut jumpstd = command("jumpstd", None, Some("PokecenterSignScript"));
        jumpstd.resolved_target_script = None;
        apply_script_control_command(&mut state, jumpstd, &BTreeMap::new()).expect("jumpstd");
        assert_eq!(
            state.script_runtime.next_script.as_deref(),
            Some("PokecenterSignScript")
        );
        assert_eq!(
            state
                .script_runtime
                .control_events
                .last()
                .map(|event| event.kind),
            Some(ScriptControlRuntimeKind::StandardJump)
        );
    }

    #[test]
    fn continue_branch_records_no_target_or_jump_state() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("SATURDAY".to_string());
        apply_script_control_command(
            &mut state,
            command("ifnotequal", Some("SATURDAY"), Some(".Done")),
            &BTreeMap::new(),
        )
        .expect("continue");

        assert_eq!(state.script_runtime.next_script, None);
        assert!(state.script_runtime.call_stack.is_empty());
        assert_eq!(
            state.script_runtime.control_events,
            vec![ScriptControlRuntimeEvent {
                kind: ScriptControlRuntimeKind::Continue,
                target_script: None,
                source_script: "Script".to_string(),
                command_index: 6,
            }]
        );
    }

    #[test]
    fn end_records_exact_end_state_and_clears_next_script() {
        let mut state = GameState::default();
        state.script_runtime.next_script = Some("PendingScript".to_string());
        apply_script_control_command(
            &mut state,
            command("endifjustbattled", None, None),
            &BTreeMap::new(),
        )
        .expect("end guarded");

        assert_eq!(state.script_runtime.next_script, None);
        assert_eq!(
            state.script_runtime.script_ended,
            Some(ScriptEndState {
                callback: false,
                just_battled_guard: true,
                source_script: "Script".to_string(),
                command_index: 6,
            })
        );
    }

    #[test]
    fn invalid_control_command_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("true".to_string());
        assert!(matches!(
            apply_script_control_command(
                &mut state,
                command("iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::UnknownBoolean { .. })
        ));

        assert!(state.script_runtime.control_events.is_empty());
        assert_eq!(state.script_runtime.next_script, None);
        assert!(state.script_runtime.call_stack.is_empty());
        assert!(state.script_runtime.deferred_scripts.is_empty());
    }
}
