use serde::{Deserialize, Serialize};

use crate::state::{EventFlagError, GameState, is_engine_flag_name};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptFlagCommand {
    pub command: String,
    pub flag_id: String,
    pub source_script: String,
    pub command_index: usize,
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
pub enum ScriptFlagError {
    UnknownCommand { command: String },
    Flag { error: EventFlagError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptFlagCommandIssue {
    UnknownCommand,
    EmptyFlagId,
    InvalidFlagId,
}

impl From<EventFlagError> for ScriptFlagError {
    fn from(error: EventFlagError) -> Self {
        Self::Flag { error }
    }
}

pub const SCRIPT_FLAG_MUTATION_COMMANDS: &[&str] =
    &["setevent", "clearevent", "setflag", "clearflag"];
pub const SCRIPT_FLAG_CHECK_COMMANDS: &[&str] = &["checkevent", "checkflag"];

pub fn is_known_script_flag_command(command: &str) -> bool {
    SCRIPT_FLAG_MUTATION_COMMANDS.contains(&command)
        || SCRIPT_FLAG_CHECK_COMMANDS.contains(&command)
}

pub fn script_flag_command_issues(command: &ScriptFlagCommand) -> Vec<ScriptFlagCommandIssue> {
    let mut issues = Vec::new();
    if !is_known_script_flag_command(&command.command) {
        issues.push(ScriptFlagCommandIssue::UnknownCommand);
    }
    if command.flag_id.is_empty() {
        issues.push(ScriptFlagCommandIssue::EmptyFlagId);
    } else if command.flag_id.trim() != command.flag_id {
        issues.push(ScriptFlagCommandIssue::InvalidFlagId);
    }
    issues
}

pub fn apply_script_flag_mutation(
    state: &mut GameState,
    command: ScriptFlagCommand,
) -> Result<ScriptFlagMutationOutcome, ScriptFlagError> {
    let value = match command.command.as_str() {
        "setevent" | "setflag" => true,
        "clearevent" | "clearflag" => false,
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
    match command.command.as_str() {
        "checkevent" | "checkflag" => {}
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
        "setflag" | "clearflag" | "checkflag"
    ) || is_engine_flag_name(&command.flag_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EventFlagError;

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
        assert!(SCRIPT_FLAG_CHECK_COMMANDS.contains(&"checkevent"));
        assert!(SCRIPT_FLAG_CHECK_COMMANDS.contains(&"checkflag"));
        assert!(is_known_script_flag_command("setflag"));
        assert!(!is_known_script_flag_command("SetFlag"));
        assert!(!is_known_script_flag_command("toggleevent"));
    }

    #[test]
    fn script_flag_issue_collector_reports_exact_pack_shape_errors() {
        assert_eq!(
            script_flag_command_issues(&command("SetEvent", "")),
            vec![
                ScriptFlagCommandIssue::UnknownCommand,
                ScriptFlagCommandIssue::EmptyFlagId,
            ]
        );
        assert_eq!(
            script_flag_command_issues(&command("setevent", " EVENT_ROUTE_29_POTION")),
            vec![ScriptFlagCommandIssue::InvalidFlagId]
        );
        assert_eq!(
            script_flag_command_issues(&command("setevent", "EVENT_ROUTE_29_POTION")),
            Vec::<ScriptFlagCommandIssue>::new()
        );
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
    }

    #[test]
    fn rejects_empty_flags_and_unknown_commands() {
        let mut state = GameState::default();
        assert_eq!(
            apply_script_flag_mutation(&mut state, command("setevent", "")),
            Err(ScriptFlagError::Flag {
                error: EventFlagError::EmptyFlagName
            })
        );
        assert_eq!(
            check_script_flag(&state, command("setevent", "EVENT_ROUTE_29_POTION")),
            Err(ScriptFlagError::UnknownCommand {
                command: "setevent".to_string()
            })
        );
    }
}
