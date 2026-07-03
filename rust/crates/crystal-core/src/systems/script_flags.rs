use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::{EventFlagError, GameState, is_engine_flag_name};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptFlagCommand {
    #[serde(deserialize_with = "required_script_flag_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_script_flag_token")]
    pub flag_id: String,
    #[serde(deserialize_with = "required_script_flag_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptFlagCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptFlagCommand {
            #[serde(deserialize_with = "required_script_flag_command_token")]
            command: String,
            #[serde(deserialize_with = "required_script_flag_token")]
            flag_id: String,
            #[serde(deserialize_with = "required_script_flag_source_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptFlagCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            flag_id: raw.flag_id,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_flag_command(&command)
            .map_err(|error| D::Error::custom(format!("{error:?}")))?;
        Ok(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptFlagMutationOutcome {
    pub command: String,
    pub flag_id: String,
    pub source_script: String,
    pub command_index: usize,
    pub value: bool,
    pub engine_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptFlagCheckOutcome {
    pub command: String,
    pub flag_id: String,
    pub source_script: String,
    pub command_index: usize,
    pub set: bool,
    pub engine_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ScriptFlagError {
    InvalidCommand { command: String },
    UnknownCommand { command: String },
    EmptyFlagId { command: String },
    InvalidFlagId { command: String, flag_id: String },
    Flag { error: EventFlagError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptFlagCommandIssue {
    InvalidCommand,
    UnknownCommand,
    EmptyFlagId,
    InvalidFlagId,
}

impl From<EventFlagError> for ScriptFlagError {
    fn from(error: EventFlagError) -> Self {
        Self::Flag { error }
    }
}

pub const SCRIPT_FLAG_MUTATION_COMMANDS: &[&str] = &[
    "setevent",
    "clearevent",
    "setflag",
    "set_flag",
    "setengineflag",
    "clearflag",
    "clear_flag",
];
pub const SCRIPT_FLAG_CHECK_COMMANDS: &[&str] = &["checkevent", "checkflag", "check_flag"];

pub fn is_known_script_flag_command(command: &str) -> bool {
    SCRIPT_FLAG_MUTATION_COMMANDS.contains(&command)
        || SCRIPT_FLAG_CHECK_COMMANDS.contains(&command)
}

pub fn script_flag_command_issues(command: &ScriptFlagCommand) -> Vec<ScriptFlagCommandIssue> {
    let mut issues = Vec::new();
    if !is_exact_script_flag_command_token(&command.command) {
        issues.push(ScriptFlagCommandIssue::InvalidCommand);
    } else if !is_known_script_flag_command(&command.command) {
        issues.push(ScriptFlagCommandIssue::UnknownCommand);
    }
    if command.flag_id.is_empty() {
        issues.push(ScriptFlagCommandIssue::EmptyFlagId);
    } else if !is_exact_script_flag_token(&command.flag_id) {
        issues.push(ScriptFlagCommandIssue::InvalidFlagId);
    }
    issues
}

pub fn validate_script_flag_command(command: &ScriptFlagCommand) -> Result<(), ScriptFlagError> {
    if !is_exact_script_flag_command_token(&command.command) {
        return Err(ScriptFlagError::InvalidCommand {
            command: command.command.clone(),
        });
    }
    if !is_known_script_flag_command(&command.command) {
        return Err(ScriptFlagError::UnknownCommand {
            command: command.command.clone(),
        });
    }
    if command.flag_id.is_empty() {
        return Err(ScriptFlagError::EmptyFlagId {
            command: command.command.clone(),
        });
    }
    if !is_exact_script_flag_token(&command.flag_id) {
        return Err(ScriptFlagError::InvalidFlagId {
            command: command.command.clone(),
            flag_id: command.flag_id.clone(),
        });
    }
    Ok(())
}

fn is_exact_script_flag_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_flag_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn is_exact_script_flag_source_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn required_script_flag_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_flag_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script flag command must be exact lowercase ASCII/underscore, found {value:?}"
        )))
    }
}

fn required_script_flag_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_flag_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script flag token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_script_flag_source_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_flag_source_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script flag source must be exact ASM label syntax, found {value:?}"
        )))
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

pub fn apply_script_flag_mutation(
    state: &mut GameState,
    command: ScriptFlagCommand,
) -> Result<ScriptFlagMutationOutcome, ScriptFlagError> {
    validate_script_flag_command(&command)?;
    let value = match command.command.as_str() {
        "setevent" | "setflag" | "set_flag" | "setengineflag" => true,
        "clearevent" | "clearflag" | "clear_flag" => false,
        other => {
            return Err(ScriptFlagError::UnknownCommand {
                command: other.to_string(),
            });
        }
    };
    let engine_flag = is_engine_command(&command);
    if engine_flag {
        state.flags.set_engine_flag(&command.flag_id, value)?;
    } else {
        state.flags.set_event_flag(&command.flag_id, value)?;
    }
    Ok(ScriptFlagMutationOutcome {
        command: command.command,
        flag_id: command.flag_id,
        source_script: command.source_script,
        command_index: command.command_index,
        value,
        engine_flag,
    })
}

pub fn check_script_flag(
    state: &GameState,
    command: ScriptFlagCommand,
) -> Result<ScriptFlagCheckOutcome, ScriptFlagError> {
    validate_script_flag_command(&command)?;
    match command.command.as_str() {
        "checkevent" | "checkflag" | "check_flag" => {}
        other => {
            return Err(ScriptFlagError::UnknownCommand {
                command: other.to_string(),
            });
        }
    }
    let engine_flag = is_engine_command(&command);
    let set = if engine_flag {
        state.flags.is_engine_flag_set(&command.flag_id)?
    } else {
        state.flags.is_event_flag_set(&command.flag_id)?
    };
    Ok(ScriptFlagCheckOutcome {
        command: command.command,
        flag_id: command.flag_id,
        source_script: command.source_script,
        command_index: command.command_index,
        set,
        engine_flag,
    })
}

fn is_engine_command(command: &ScriptFlagCommand) -> bool {
    matches!(
        command.command.as_str(),
        "setflag"
            | "set_flag"
            | "setengineflag"
            | "clearflag"
            | "clear_flag"
            | "checkflag"
            | "check_flag"
    ) || is_engine_flag_name(&command.flag_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(name: &str, flag_id: &str) -> ScriptFlagCommand {
        ScriptFlagCommand {
            command: name.to_string(),
            flag_id: flag_id.to_string(),
            source_script: "RouteScript".to_string(),
            command_index: 3,
        }
    }

    #[test]
    fn exported_flag_command_sets_are_exact() {
        assert!(SCRIPT_FLAG_MUTATION_COMMANDS.contains(&"setevent"));
        assert!(SCRIPT_FLAG_MUTATION_COMMANDS.contains(&"clearflag"));
        assert!(SCRIPT_FLAG_MUTATION_COMMANDS.contains(&"set_flag"));
        assert!(SCRIPT_FLAG_MUTATION_COMMANDS.contains(&"clear_flag"));
        assert!(SCRIPT_FLAG_MUTATION_COMMANDS.contains(&"setengineflag"));
        assert!(SCRIPT_FLAG_CHECK_COMMANDS.contains(&"checkevent"));
        assert!(SCRIPT_FLAG_CHECK_COMMANDS.contains(&"checkflag"));
        assert!(SCRIPT_FLAG_CHECK_COMMANDS.contains(&"check_flag"));
        assert!(is_known_script_flag_command("setflag"));
        assert!(is_known_script_flag_command("set_flag"));
        assert!(!is_known_script_flag_command("SetFlag"));
        assert!(!is_known_script_flag_command("toggleevent"));
    }

    #[test]
    fn script_flag_issue_collector_reports_exact_pack_shape_errors() {
        assert_eq!(
            script_flag_command_issues(&command("SetEvent", "")),
            vec![
                ScriptFlagCommandIssue::InvalidCommand,
                ScriptFlagCommandIssue::EmptyFlagId,
            ]
        );
        assert_eq!(
            script_flag_command_issues(&command("set event", "EVENT_ROUTE_29_POTION")),
            vec![ScriptFlagCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_flag_command_issues(&command("toggleevent", "EVENT_ROUTE_29_POTION")),
            vec![ScriptFlagCommandIssue::UnknownCommand]
        );
        assert_eq!(
            script_flag_command_issues(&command("setevent", " EVENT_ROUTE_29_POTION")),
            vec![ScriptFlagCommandIssue::InvalidFlagId]
        );
        assert_eq!(
            script_flag_command_issues(&command("setevent", "EVENT ROUTE_29_POTION")),
            vec![ScriptFlagCommandIssue::InvalidFlagId]
        );
        assert_eq!(
            script_flag_command_issues(&command("setevent", "EVENT_ROUTE_29_POTION")),
            Vec::<ScriptFlagCommandIssue>::new()
        );
    }

    #[test]
    fn script_flag_commands_reject_reserved_pack_prefixes() {
        assert_eq!(
            script_flag_command_issues(&command("fallbackset", "EVENT_ROUTE_29_POTION")),
            vec![ScriptFlagCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_flag_command_issues(&command("setevent", "legacy_event")),
            vec![ScriptFlagCommandIssue::InvalidFlagId]
        );

        for (field, value) in [
            ("command", serde_json::json!("fallbackset")),
            ("flag_id", serde_json::json!("legacy_event")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "setevent",
                "flag_id": "EVENT_ROUTE_29_POTION",
                "source_script": "RouteScript",
                "command_index": 3
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptFlagCommand>(payload)
                .expect_err("reserved script flag command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script flag"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn event_flag_commands_mutate_exact_event_flags_without_case_coercion() {
        let mut state = GameState::default();
        let set =
            apply_script_flag_mutation(&mut state, command("setevent", "EVENT_ROUTE_29_POTION"))
                .expect("set exact event flag");

        assert_eq!(set.value, true);
        assert_eq!(set.engine_flag, false);
        assert_eq!(
            check_script_flag(&state, command("checkevent", "EVENT_ROUTE_29_POTION"))
                .expect("check exact event flag")
                .set,
            true
        );
        assert_eq!(
            check_script_flag(&state, command("checkevent", "event_route_29_potion"))
                .expect("case-changed id is a distinct flag")
                .set,
            false
        );

        apply_script_flag_mutation(&mut state, command("clearevent", "EVENT_ROUTE_29_POTION"))
            .expect("clear exact event flag");
        assert_eq!(
            check_script_flag(&state, command("checkevent", "EVENT_ROUTE_29_POTION"))
                .expect("check cleared event flag")
                .set,
            false
        );
    }

    #[test]
    fn flag_commands_use_engine_storage_exactly() {
        let mut state = GameState::default();
        let set = apply_script_flag_mutation(&mut state, command("setflag", "ENGINE_ZEPHYRBADGE"))
            .expect("set engine flag");

        assert_eq!(set.engine_flag, true);
        assert_eq!(
            state.flags.is_engine_flag_set("ENGINE_ZEPHYRBADGE"),
            Ok(true)
        );
        assert_eq!(
            state.flags.is_event_flag_set("ENGINE_ZEPHYRBADGE"),
            Ok(false)
        );
        assert_eq!(
            check_script_flag(&state, command("checkflag", "ENGINE_ZEPHYRBADGE"))
                .expect("check engine flag")
                .set,
            true
        );

        apply_script_flag_mutation(&mut state, command("clear_flag", "ENGINE_ZEPHYRBADGE"))
            .expect("clear exact underscore engine flag");
        assert_eq!(
            check_script_flag(&state, command("check_flag", "ENGINE_ZEPHYRBADGE"))
                .expect("check exact underscore engine flag")
                .set,
            false
        );

        let set_engine =
            apply_script_flag_mutation(&mut state, command("setengineflag", "ENGINE_FLYPOINT"))
                .expect("set explicit engine flag");
        assert_eq!(set_engine.command, "setengineflag");
        assert!(set_engine.engine_flag);
        assert_eq!(set_engine.value, true);
        assert_eq!(state.flags.is_engine_flag_set("ENGINE_FLYPOINT"), Ok(true));
    }

    #[test]
    fn rejects_empty_flags_and_unknown_commands() {
        let mut state = GameState::default();
        assert_eq!(
            apply_script_flag_mutation(&mut state, command("set event", "EVENT_ROUTE_29_POTION")),
            Err(ScriptFlagError::InvalidCommand {
                command: "set event".to_string()
            })
        );
        assert_eq!(
            apply_script_flag_mutation(&mut state, command("setevent", "")),
            Err(ScriptFlagError::EmptyFlagId {
                command: "setevent".to_string()
            })
        );
        assert_eq!(
            apply_script_flag_mutation(&mut state, command("setevent", "EVENT ROUTE_29_POTION")),
            Err(ScriptFlagError::InvalidFlagId {
                command: "setevent".to_string(),
                flag_id: "EVENT ROUTE_29_POTION".to_string(),
            })
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_ROUTE_29_POTION"),
            Ok(false)
        );
        assert_eq!(
            check_script_flag(&state, command("setevent", "EVENT_ROUTE_29_POTION")),
            Err(ScriptFlagError::UnknownCommand {
                command: "setevent".to_string()
            })
        );
    }

    #[test]
    fn script_flag_serialized_variants_reject_unknown_fallback_fields() {
        let error = serde_json::from_value::<ScriptFlagError>(serde_json::json!({
            "UnknownCommand": {
                "command": "set_event",
                "normalized_command": "setevent"
            }
        }))
        .expect_err("normalized command must be rejected")
        .to_string();
        assert!(
            error.contains("unknown field `normalized_command`"),
            "{error}"
        );
    }
}
