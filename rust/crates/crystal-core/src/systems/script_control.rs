use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::{
    GameState, ScriptControlRuntimeEvent, ScriptControlRuntimeKind, ScriptEndState,
    ScriptLocation, ScriptReturnFrame,
};

const RUNNING_TRAINER_BATTLE_SCRIPT_MEMORY: &str = "wRunningTrainerBattleScript";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptControlCommand {
    #[serde(deserialize_with = "required_control_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_compare_token")]
    pub compare_value: Option<String>,
    #[serde(deserialize_with = "required_nullable_control_label_token")]
    pub target_label: Option<String>,
    #[serde(deserialize_with = "required_nullable_control_label_token")]
    pub resolved_target_script: Option<String>,
    #[serde(deserialize_with = "required_control_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptControlCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptControlCommand {
            #[serde(deserialize_with = "required_control_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_compare_token")]
            compare_value: Option<String>,
            #[serde(deserialize_with = "required_nullable_control_label_token")]
            target_label: Option<String>,
            #[serde(deserialize_with = "required_nullable_control_label_token")]
            resolved_target_script: Option<String>,
            #[serde(deserialize_with = "required_control_label_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptControlCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            compare_value: raw.compare_value,
            target_label: raw.target_label,
            resolved_target_script: raw.resolved_target_script,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_control_command(&command).map_err(D::Error::custom)?;
        Ok(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub enum ScriptControlCommandError {
    #[error("script control command name is empty")]
    EmptyCommand,
    #[error("script control command name is whitespace-padded '{command}'")]
    PaddedCommand { command: String },
    #[error("unknown script control command '{command}'")]
    UnknownCommand { command: String },
    #[error("script control command '{command}' is missing target label")]
    MissingTarget { command: String },
    #[error("script control command '{command}' has unexpected target label")]
    UnexpectedTarget { command: String },
    #[error("script control command '{command}' references empty target label")]
    EmptyTarget { command: String },
    #[error("script control command '{command}' references invalid target label '{target}'")]
    InvalidTarget { command: String, target: String },
    #[error("script control command '{command}' is missing compare value")]
    MissingCompareValue { command: String },
    #[error("script control command '{command}' has unexpected compare value")]
    UnexpectedCompareValue { command: String },
    #[error("script control command '{command}' references empty compare value")]
    EmptyCompareValue { command: String },
    #[error("script control command '{command}' references invalid compare value '{value}'")]
    InvalidCompareValue { command: String, value: String },
    #[error("script control command '{command}' is missing resolved target script")]
    MissingResolvedTarget { command: String },
    #[error(
        "script control command '{command}' references invalid resolved target script '{target_script}'"
    )]
    InvalidResolvedTarget {
        command: String,
        target_script: String,
    },
    #[error("script control source script '{source_script}' is invalid")]
    InvalidSourceScript { source_script: String },
    #[error("script accumulator is unset for '{command}'")]
    UnsetAccumulator { command: String },
    #[error("script accumulator value '{value}' is not an exact TRUE/FALSE/1/0 token")]
    UnknownBoolean { value: String },
    #[error("numeric script token '{token}' is not exact pack syntax")]
    InvalidNumericToken { token: String },
    #[error("cannot resolve numeric script token '{token}'")]
    UnknownNumericToken { token: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptControlCommandIssue {
    InvalidCommand { error: ScriptControlCommandError },
    InvalidTargetScript { target_script: String },
    UnknownTargetScript { target_script: String },
}

pub fn script_control_command_issues(
    command: &ScriptControlCommand,
    script_labels: &BTreeSet<String>,
) -> Vec<ScriptControlCommandIssue> {
    let mut issues = Vec::new();
    if let Err(error) = validate_script_control_command(command) {
        issues.push(ScriptControlCommandIssue::InvalidCommand { error });
        return issues;
    }
    if command.command != "jumpstd" {
        if let Some(target_script) = command.resolved_target_script.as_deref() {
            if !is_exact_nonempty_token(target_script) {
                issues.push(ScriptControlCommandIssue::InvalidTargetScript {
                    target_script: target_script.to_string(),
                });
            } else if !script_labels.contains(target_script) {
                issues.push(ScriptControlCommandIssue::UnknownTargetScript {
                    target_script: target_script.to_string(),
                });
            }
        }
    }
    issues
}

pub fn resolve_script_control_command(
    state: &GameState,
    command: ScriptControlCommand,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<ScriptControlAction, ScriptControlCommandError> {
    validate_script_control_command(&command)?;
    match command.command.as_str() {
        "ifequal" => {
            let (left, right) = comparison_bytes(state, &command, numeric_constants)?;
            branch(state, &command, left == right)
        }
        "ifnotequal" => {
            let (left, right) = comparison_bytes(state, &command, numeric_constants)?;
            branch(state, &command, left != right)
        }
        "iftrue" => branch(
            state,
            &command,
            require_boolean(state, &command, numeric_constants)?,
        ),
        "iffalse" => branch(
            state,
            &command,
            !require_boolean(state, &command, numeric_constants)?,
        ),
        "ifgreater" => {
            let (left, right) = comparison_bytes(state, &command, numeric_constants)?;
            branch(state, &command, left > right)
        }
        "ifless" => {
            let (left, right) = comparison_bytes(state, &command, numeric_constants)?;
            branch(state, &command, left < right)
        }
        "sjump" | "jump" | "farsjump" | "scall" | "farscall" | "sdefer" | "jumpstd" => {
            Ok(jump_action(command)?)
        }
        "end" | "endcallback" => Ok(ScriptControlAction::End {
            callback: command.command == "endcallback",
            just_battled_guard: false,
            source_script: command.source_script,
            command_index: command.command_index,
        }),
        "endifjustbattled" => {
            if running_trainer_battle_script(state, numeric_constants)? {
                Ok(ScriptControlAction::End {
                    callback: false,
                    just_battled_guard: true,
                    source_script: command.source_script,
                    command_index: command.command_index,
                })
            } else {
                Ok(ScriptControlAction::Continue {
                    source_script: command.source_script,
                    command_index: command.command_index,
                })
            }
        }
        other => Err(ScriptControlCommandError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn apply_script_control_command(
    state: &mut GameState,
    origin_map_name: &str,
    command: ScriptControlCommand,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<ScriptControlAction, ScriptControlCommandError> {
    let action = resolve_script_control_command(state, command, numeric_constants)?;
    apply_script_control_action_to_state(state, origin_map_name, &action);
    Ok(action)
}

pub fn apply_script_control_action_to_state(
    state: &mut GameState,
    origin_map_name: &str,
    action: &ScriptControlAction,
) {
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
                    .push(ScriptLocation {
                        origin_map_name: origin_map_name.to_string(),
                        script: target_script.clone(),
                    });
                ScriptControlRuntimeKind::Defer
            } else if *call {
                state.script_runtime.call_stack.push(ScriptReturnFrame {
                    origin_map_name: origin_map_name.to_string(),
                    source_script: source_script.clone(),
                    next_command_index: command_index + 1,
                });
                state.script_runtime.next_script = Some(ScriptLocation {
                    origin_map_name: origin_map_name.to_string(),
                    script: target_script.clone(),
                });
                ScriptControlRuntimeKind::Call
            } else if *standard {
                state.script_runtime.next_script = Some(ScriptLocation {
                    origin_map_name: origin_map_name.to_string(),
                    script: target_script.clone(),
                });
                ScriptControlRuntimeKind::StandardJump
            } else {
                state.script_runtime.next_script = Some(ScriptLocation {
                    origin_map_name: origin_map_name.to_string(),
                    script: target_script.clone(),
                });
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
    reject_invalid_source_script(command)?;
    if command.command.is_empty() {
        return Err(ScriptControlCommandError::EmptyCommand);
    }
    if !is_exact_nonempty_token(&command.command) {
        return Err(ScriptControlCommandError::PaddedCommand {
            command: command.command.clone(),
        });
    }
    match command.command.as_str() {
        "ifequal" | "ifnotequal" | "ifgreater" | "ifless" => {
            require_compare_value(command)?;
            require_target(command)?;
            require_resolved_target(command)?;
        }
        "iftrue" | "iffalse" | "sjump" | "jump" | "farsjump" | "scall" | "farscall"
        | "sdefer" => {
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

fn reject_invalid_source_script(
    command: &ScriptControlCommand,
) -> Result<(), ScriptControlCommandError> {
    if is_exact_nonempty_token(&command.source_script) {
        Ok(())
    } else {
        Err(ScriptControlCommandError::InvalidSourceScript {
            source_script: command.source_script.clone(),
        })
    }
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
        call: matches!(command.command.as_str(), "scall" | "farscall"),
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

fn comparison_bytes(
    state: &GameState,
    command: &ScriptControlCommand,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<(u8, u8), ScriptControlCommandError> {
    let left = parse_script_byte(accumulator(state, command)?, numeric_constants)?;
    let right = parse_script_byte(require_compare_value(command)?, numeric_constants)?;
    Ok((left, right))
}

fn parse_script_byte(
    token: &str,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<u8, ScriptControlCommandError> {
    // Every comparison operand is emitted by the pret macros with `db` and
    // compared against the one-byte wScriptVar register. Preserve that byte
    // representation for signed literals and constant expressions as well.
    Ok(parse_numeric_token(token, numeric_constants)?.rem_euclid(256) as u8)
}

fn running_trainer_battle_script(
    state: &GameState,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<bool, ScriptControlCommandError> {
    // TalkToTrainer clears this WRAM byte before a manual interaction, while
    // StartBattleWithMapTrainerScript writes -1 immediately before dispatching
    // the trainer's after-battle script. Script_endifjustbattled only ends for
    // that nonzero post-battle dispatch.
    state
        .script_runtime
        .memory
        .get(RUNNING_TRAINER_BATTLE_SCRIPT_MEMORY)
        .map(|value| parse_script_byte(value, numeric_constants).map(|value| value != 0))
        .unwrap_or(Ok(false))
}

fn require_boolean(
    state: &GameState,
    command: &ScriptControlCommand,
    numeric_constants: &BTreeMap<String, i32>,
) -> Result<bool, ScriptControlCommandError> {
    match accumulator(state, command)? {
        "TRUE" | "1" => Ok(true),
        "FALSE" | "0" => Ok(false),
        value => parse_script_byte(value, numeric_constants)
            .map(|value| value != 0)
            .map_err(|_| ScriptControlCommandError::UnknownBoolean {
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
    if value.trim() != value {
        return Err(ScriptControlCommandError::InvalidCompareValue {
            command: command.command.clone(),
            value: value.to_string(),
        });
    }
    if has_reserved_pack_prefix(value) {
        return Err(ScriptControlCommandError::InvalidCompareValue {
            command: command.command.clone(),
            value: value.to_string(),
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
    if !is_exact_nonempty_token(target) {
        return Err(ScriptControlCommandError::InvalidTarget {
            command: command.command.clone(),
            target: target.to_string(),
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
    let target_script = command.resolved_target_script.as_deref().ok_or_else(|| {
        ScriptControlCommandError::MissingResolvedTarget {
            command: command.command.clone(),
        }
    })?;
    if !is_exact_nonempty_token(target_script) {
        return Err(ScriptControlCommandError::InvalidResolvedTarget {
            command: command.command.clone(),
            target_script: target_script.to_string(),
        });
    }
    Ok(target_script)
}

fn parse_numeric_token(
    token: &str,
    constants: &BTreeMap<String, i32>,
) -> Result<i32, ScriptControlCommandError> {
    if token.is_empty() || token.trim() != token || token.contains('\t') {
        return Err(ScriptControlCommandError::InvalidNumericToken {
            token: token.to_string(),
        });
    }
    let parts: Vec<&str> = token.split(' ').collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(ScriptControlCommandError::InvalidNumericToken {
            token: token.to_string(),
        });
    }
    match parts.as_slice() {
        [single] => parse_numeric_atom(single, constants),
        [left, op @ ("-" | "+"), right] => {
            if format!("{left} {op} {right}") != token {
                return Err(ScriptControlCommandError::InvalidNumericToken {
                    token: token.to_string(),
                });
            }
            match *op {
                "-" => Ok(
                    parse_numeric_atom(left, constants)? - parse_numeric_atom(right, constants)?
                ),
                "+" => Ok(
                    parse_numeric_atom(left, constants)? + parse_numeric_atom(right, constants)?
                ),
                _ => unreachable!(),
            }
        }
        _ => Err(ScriptControlCommandError::InvalidNumericToken {
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
        if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ScriptControlCommandError::InvalidNumericToken {
                token: token.to_string(),
            });
        }
        return i32::from_str_radix(hex, 16).map_err(|_| {
            ScriptControlCommandError::UnknownNumericToken {
                token: token.to_string(),
            }
        });
    }
    token.parse::<i32>().map_err(|_| {
        if is_exact_numeric_symbol(token) {
            ScriptControlCommandError::UnknownNumericToken {
                token: token.to_string(),
            }
        } else {
            ScriptControlCommandError::InvalidNumericToken {
                token: token.to_string(),
            }
        }
    })
}

fn is_exact_numeric_symbol(value: &str) -> bool {
    let Some(first) = value.bytes().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn is_exact_nonempty_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn is_exact_control_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
        && !has_reserved_pack_prefix(value)
}

fn is_exact_compare_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && !has_reserved_pack_prefix(value)
}

fn required_control_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_control_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script control command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_control_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script control label must be exact ASCII label syntax, found {value:?}"
        )))
    }
}

fn required_nullable_control_label_token<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_nonempty_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script control label must be exact ASCII label syntax, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_nullable_compare_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_compare_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script control compare value must be exact visible ASCII, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script_control_command(
        name: &str,
        compare: Option<&str>,
        target: Option<&str>,
    ) -> ScriptControlCommand {
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
    fn script_control_serialized_variants_reject_unknown_fallback_fields() {
        let action_error = serde_json::from_value::<ScriptControlAction>(serde_json::json!({
            "jump": {
                "target_script": ".Done@Script",
                "call": false,
                "deferred": false,
                "standard": false,
                "source_script": "Script",
                "command_index": 6,
                "fallback_target_script": "DefaultScript"
            }
        }))
        .expect_err("control actions must not accept fallback targets");
        assert!(
            action_error
                .to_string()
                .contains("unknown field `fallback_target_script`"),
            "{action_error}"
        );

        let error_error = serde_json::from_value::<ScriptControlCommandError>(serde_json::json!({
            "UnknownCommand": {
                "command": "if_true",
                "normalized_command": "iftrue"
            }
        }))
        .expect_err("control errors must not accept normalized command aliases");
        assert!(
            error_error
                .to_string()
                .contains("unknown field `normalized_command`"),
            "{error_error}"
        );

        let issue_error = serde_json::from_value::<ScriptControlCommandIssue>(serde_json::json!({
            "unknown_target_script": {
                "target_script": ".Done@Script",
                "legacy_target_label": ".Done"
            }
        }))
        .expect_err("control issues must not accept legacy target labels");
        assert!(
            issue_error
                .to_string()
                .contains("unknown field `legacy_target_label`"),
            "{issue_error}"
        );
    }

    #[test]
    fn resolves_exact_accumulator_branches_and_jumps() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("6".to_string());
        let constants = BTreeMap::from([("SATURDAY".to_string(), 6)]);
        assert_eq!(
            resolve_script_control_command(
                &state,
                script_control_command("ifequal", Some("SATURDAY"), Some(".Done")),
                &constants,
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
                script_control_command("ifnotequal", Some("SATURDAY"), Some(".Done")),
                &constants,
            ),
            Ok(ScriptControlAction::Continue { .. })
        ));
    }

    #[test]
    fn resolves_comparison_operands_to_canonical_script_bytes() {
        let constants =
            BTreeMap::from([("PARTY_LENGTH".to_string(), 6), ("TUESDAY".to_string(), 2)]);
        let mut state = GameState::default();

        for (accumulator, compare) in [
            ("6", "PARTY_LENGTH"),
            ("2", "TUESDAY"),
            ("0", "$0"),
            ("TUESDAY", "$2"),
            ("-1", "$ff"),
        ] {
            state.script_runtime.script_value = Some(accumulator.to_string());
            assert!(matches!(
                resolve_script_control_command(
                    &state,
                    script_control_command("ifequal", Some(compare), Some(".Equal")),
                    &constants,
                ),
                Ok(ScriptControlAction::Jump { .. })
            ));
            assert!(matches!(
                resolve_script_control_command(
                    &state,
                    script_control_command("ifnotequal", Some(compare), Some(".Equal")),
                    &constants,
                ),
                Ok(ScriptControlAction::Continue { .. })
            ));
        }

        state.script_runtime.script_value = Some("-1".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("$0"), Some(".Greater")),
                &constants,
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));
    }

    #[test]
    fn resolves_boolean_numeric_and_standard_jumps() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("TRUE".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iftrue", None, Some(".Yes")),
                &BTreeMap::new(),
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));
        state.script_runtime.script_value = Some("1".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iftrue", None, Some(".One")),
                &BTreeMap::new(),
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));
        state.script_runtime.script_value = Some("0".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iffalse", None, Some(".Zero")),
                &BTreeMap::new(),
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));

        state.script_runtime.script_value = Some("8".to_string());
        let constants = BTreeMap::from([("NUM_JOHTO_BADGES".to_string(), 8)]);
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command(
                    "ifgreater",
                    Some("NUM_JOHTO_BADGES - 1"),
                    Some(".AllEight")
                ),
                &constants,
            ),
            Ok(ScriptControlAction::Jump { .. })
        ));

        let mut jumpstd = script_control_command("jumpstd", None, Some("PokecenterSignScript"));
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
    fn rejects_malformed_numeric_tokens_before_unknown_constants() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("8".to_string());
        let constants = BTreeMap::from([("NUM_JOHTO_BADGES".to_string(), 8)]);

        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("NUM_JOHTO_BADGES  -  1"), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::InvalidNumericToken { token })
                if token == "NUM_JOHTO_BADGES  -  1"
        ));
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("$GG"), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::InvalidNumericToken { token }) if token == "$GG"
        ));
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("MISSING_CONSTANT"), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::UnknownNumericToken { token })
                if token == "MISSING_CONSTANT"
        ));

        state.script_runtime.script_value = Some("NUM JOHTO BADGES".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("1"), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::InvalidNumericToken { token })
                if token == "NUM JOHTO BADGES"
        ));

        state.script_runtime.script_value = Some(" 8".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("7"), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::InvalidNumericToken { token }) if token == " 8"
        ));

        state.script_runtime.script_value = Some("8".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("7 "), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::InvalidCompareValue { value, .. }) if value == "7 "
        ));

        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("ifgreater", Some("NUM_JOHTO_BADGES\t-\t1"), Some(".AllEight")),
                &constants,
            ),
            Err(ScriptControlCommandError::InvalidNumericToken { token })
                if token == "NUM_JOHTO_BADGES\t-\t1"
        ));
    }

    #[test]
    fn command_issues_validate_structure_and_same_map_targets_without_fallbacks() {
        let labels = BTreeSet::from([".Done@Script".to_string()]);

        assert_eq!(
            script_control_command_issues(
                &script_control_command("iftrue", Some("TRUE"), Some(".Done")),
                &labels
            ),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::UnexpectedCompareValue {
                    command: "iftrue".to_string()
                }
            }]
        );
        assert_eq!(
            script_control_command_issues(
                &script_control_command("ifequal", Some("TRUE"), Some(".missing")),
                &labels
            ),
            vec![ScriptControlCommandIssue::UnknownTargetScript {
                target_script: ".missing@Script".to_string()
            }]
        );
        assert_eq!(
            script_control_command_issues(
                &script_control_command("ifequal", Some(" TRUE"), Some(".Done")),
                &labels
            ),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::InvalidCompareValue {
                    command: "ifequal".to_string(),
                    value: " TRUE".to_string(),
                }
            }]
        );

        let mut invalid_resolved = script_control_command("ifequal", Some("TRUE"), Some(".Done"));
        invalid_resolved.resolved_target_script = Some(" .Done@Script".to_string());
        assert_eq!(
            script_control_command_issues(&invalid_resolved, &labels),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::InvalidResolvedTarget {
                    command: "ifequal".to_string(),
                    target_script: " .Done@Script".to_string(),
                }
            }]
        );

        let mut jumpstd = script_control_command("jumpstd", None, Some("PokecenterSignScript"));
        jumpstd.resolved_target_script = None;
        assert_eq!(script_control_command_issues(&jumpstd, &labels), []);
    }

    #[test]
    fn control_commands_reject_reserved_pack_prefixes() {
        let labels = BTreeSet::from([".Done@Script".to_string()]);

        assert_eq!(
            script_control_command_issues(
                &script_control_command("fallbackjump", None, Some(".Done")),
                &labels,
            ),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::PaddedCommand {
                    command: "fallbackjump".to_string(),
                }
            }]
        );
        assert_eq!(
            script_control_command_issues(
                &script_control_command("ifequal", Some("legacy_value"), Some(".Done")),
                &labels,
            ),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::InvalidCompareValue {
                    command: "ifequal".to_string(),
                    value: "legacy_value".to_string(),
                }
            }]
        );
        assert_eq!(
            script_control_command_issues(
                &script_control_command("iftrue", None, Some("fallback_target")),
                &labels,
            ),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::InvalidTarget {
                    command: "iftrue".to_string(),
                    target: "fallback_target".to_string(),
                }
            }]
        );

        let mut invalid_resolved = script_control_command("iftrue", None, Some(".Done"));
        invalid_resolved.resolved_target_script = Some("legacy_target@Script".to_string());
        assert_eq!(
            script_control_command_issues(&invalid_resolved, &labels),
            vec![ScriptControlCommandIssue::InvalidCommand {
                error: ScriptControlCommandError::InvalidResolvedTarget {
                    command: "iftrue".to_string(),
                    target_script: "legacy_target@Script".to_string(),
                }
            }]
        );

        for (field, value) in [
            ("command", serde_json::json!("fallbackjump")),
            ("compare_value", serde_json::json!("legacy_value")),
            ("target_label", serde_json::json!("fallback_target")),
            (
                "resolved_target_script",
                serde_json::json!("legacy_target@Script"),
            ),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "ifequal",
                "compare_value": "TRUE",
                "target_label": ".Done",
                "resolved_target_script": ".Done@Script",
                "source_script": "Script",
                "command_index": 6
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptControlCommand>(payload)
                .expect_err("reserved script control command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script control"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn rejects_unset_or_case_changed_boolean_accumulators() {
        let state = GameState::default();
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::UnsetAccumulator { .. })
        ));

        let mut state = GameState::default();
        state.script_runtime.script_value = Some("true".to_string());
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::UnknownBoolean { .. })
        ));
    }

    #[test]
    fn rejects_padded_control_targets_without_normalization() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("TRUE".to_string());

        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::EmptyCommand)
        ));
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command(" iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::PaddedCommand { .. })
        ));
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("if true", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::PaddedCommand { .. })
        ));

        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iftrue", None, Some(" .Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::InvalidTarget { .. })
        ));
        assert!(matches!(
            resolve_script_control_command(
                &state,
                script_control_command("iftrue", None, Some(".Do ne")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::InvalidTarget { .. })
        ));

        let mut resolved_command = script_control_command("iftrue", None, Some(".Done"));
        resolved_command.resolved_target_script = Some(" .Done@Script".to_string());
        assert!(matches!(
            resolve_script_control_command(&state, resolved_command, &BTreeMap::new()),
            Err(ScriptControlCommandError::InvalidResolvedTarget { .. })
        ));
        let mut resolved_command = script_control_command("iftrue", None, Some(".Done"));
        resolved_command.resolved_target_script = Some(".Done @Script".to_string());
        assert!(matches!(
            resolve_script_control_command(&state, resolved_command, &BTreeMap::new()),
            Err(ScriptControlCommandError::InvalidResolvedTarget { .. })
        ));
    }

    #[test]
    fn applies_jump_call_defer_and_standard_control_actions() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("TRUE".to_string());
        apply_script_control_command(
            &mut state,
            "TestMap",
            script_control_command("iftrue", None, Some(".Yes")),
            &BTreeMap::new(),
        )
        .expect("jump");
        assert_eq!(
            state.script_runtime.next_script.as_ref().map(|location| location.script.as_str()),
            Some(".Yes@Script")
        );
        assert_eq!(
            state.script_runtime.control_events[0].kind,
            ScriptControlRuntimeKind::Jump
        );

        apply_script_control_command(
            &mut state,
            "TestMap",
            script_control_command("scall", None, Some(".Call")),
            &BTreeMap::new(),
        )
        .expect("call");
        assert_eq!(
            state.script_runtime.call_stack,
            vec![ScriptReturnFrame {
                origin_map_name: "TestMap".to_string(),
                source_script: "Script".to_string(),
                next_command_index: 7,
            }]
        );
        assert_eq!(
            state.script_runtime.next_script.as_ref().map(|location| location.script.as_str()),
            Some(".Call@Script")
        );

        apply_script_control_command(
            &mut state,
            "TestMap",
            script_control_command("sdefer", None, Some(".Deferred")),
            &BTreeMap::new(),
        )
        .expect("defer");
        assert_eq!(
            state.script_runtime.deferred_scripts,
            vec![ScriptLocation {
                origin_map_name: "TestMap".to_string(),
                script: ".Deferred@Script".to_string(),
            }]
        );

        let mut jumpstd = script_control_command("jumpstd", None, Some("PokecenterSignScript"));
        jumpstd.resolved_target_script = None;
        apply_script_control_command(&mut state, "TestMap", jumpstd, &BTreeMap::new())
            .expect("jumpstd");
        assert_eq!(
            state.script_runtime.next_script.as_ref().map(|location| location.script.as_str()),
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
        state.script_runtime.script_value = Some("6".to_string());
        let constants = BTreeMap::from([("SATURDAY".to_string(), 6)]);
        apply_script_control_command(
            &mut state,
            "TestMap",
            script_control_command("ifnotequal", Some("SATURDAY"), Some(".Done")),
            &constants,
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
    fn endifjustbattled_continues_during_manual_trainer_talk() {
        let mut state = GameState::default();
        state.script_runtime.next_script = Some(ScriptLocation {
            origin_map_name: "TestMap".to_string(),
            script: "PendingScript".to_string(),
        });
        state.script_runtime.memory.insert(
            RUNNING_TRAINER_BATTLE_SCRIPT_MEMORY.to_string(),
            "0".to_string(),
        );
        let action = apply_script_control_command(
            &mut state,
            "TestMap",
            script_control_command("endifjustbattled", None, None),
            &BTreeMap::new(),
        )
        .expect("manual trainer talk continues");

        assert!(matches!(action, ScriptControlAction::Continue { .. }));
        assert_eq!(
            state.script_runtime.next_script,
            Some(ScriptLocation {
                origin_map_name: "TestMap".to_string(),
                script: "PendingScript".to_string(),
            })
        );
        assert_eq!(state.script_runtime.script_ended, None);
        assert_eq!(
            state
                .script_runtime
                .control_events
                .last()
                .map(|event| event.kind),
            Some(ScriptControlRuntimeKind::Continue)
        );
    }

    #[test]
    fn endifjustbattled_ends_post_battle_trainer_dispatch() {
        let mut state = GameState::default();
        state.script_runtime.next_script = Some(ScriptLocation {
            origin_map_name: "TestMap".to_string(),
            script: "PendingScript".to_string(),
        });
        state.script_runtime.memory.insert(
            RUNNING_TRAINER_BATTLE_SCRIPT_MEMORY.to_string(),
            "-1".to_string(),
        );
        let action = apply_script_control_command(
            &mut state,
            "TestMap",
            script_control_command("endifjustbattled", None, None),
            &BTreeMap::new(),
        )
        .expect("post-battle trainer dispatch ends");

        assert!(matches!(action, ScriptControlAction::End { .. }));
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
                "TestMap",
                script_control_command("iftrue", None, Some(".Done")),
                &BTreeMap::new(),
            ),
            Err(ScriptControlCommandError::UnknownBoolean { .. })
        ));

        assert!(state.script_runtime.control_events.is_empty());
        assert_eq!(state.script_runtime.next_script, None);
        assert!(state.script_runtime.call_stack.is_empty());
        assert!(state.script_runtime.deferred_scripts.is_empty());
    }

    #[test]
    fn invalid_control_source_script_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("TRUE".to_string());
        state.script_runtime.next_script = Some(ScriptLocation {
            origin_map_name: "TestMap".to_string(),
            script: "PendingScript".to_string(),
        });
        let mut bad_source = script_control_command("scall", None, Some(".Done"));
        bad_source.source_script = "fallback_script".to_string();

        assert_eq!(
            apply_script_control_command(&mut state, "TestMap", bad_source, &BTreeMap::new()),
            Err(ScriptControlCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );

        assert!(state.script_runtime.control_events.is_empty());
        assert_eq!(
            state.script_runtime.next_script.as_ref().map(|location| location.script.as_str()),
            Some("PendingScript")
        );
        assert!(state.script_runtime.call_stack.is_empty());
        assert!(state.script_runtime.deferred_scripts.is_empty());
    }
}
