use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::state::{
    GameState, ScriptTextRuntimeEvent, ScriptTextRuntimeKind, ScriptTextWait, ScriptYesNoPrompt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextCommand {
    pub command: String,
    pub text_label: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextBody {
    pub label: String,
    pub commands: Vec<ScriptTextBodyCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextBodyCommand {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMenuDefinition {
    pub label: String,
    pub commands: Vec<ScriptMenuCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMenuCommand {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptTextAction {
    Open {
        source_script: String,
        command_index: usize,
    },
    Close {
        source_script: String,
        command_index: usize,
    },
    WaitButton {
        command: String,
        source_script: String,
        command_index: usize,
    },
    YesNo {
        source_script: String,
        command_index: usize,
    },
    Write {
        command: String,
        text_label: String,
        face_player: bool,
        closes_text: bool,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ScriptTextCommandError {
    #[error("unknown script text command '{command}'")]
    UnknownCommand { command: String },
    #[error("script text command '{command}' is missing a text label")]
    MissingTextLabel { command: String },
    #[error("script text command '{command}' references unknown text label '{text_label}'")]
    UnknownTextLabel { command: String, text_label: String },
    #[error("script text command '{command}' has unexpected text label")]
    UnexpectedTextLabel { command: String },
}

pub fn resolve_script_text_command(
    command: ScriptTextCommand,
    text_labels: &BTreeSet<String>,
) -> Result<ScriptTextAction, ScriptTextCommandError> {
    match command.command.as_str() {
        "opentext" => {
            reject_text_label(&command)?;
            Ok(ScriptTextAction::Open {
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "closetext" => {
            reject_text_label(&command)?;
            Ok(ScriptTextAction::Close {
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "promptbutton" | "waitbutton" => {
            reject_text_label(&command)?;
            Ok(ScriptTextAction::WaitButton {
                command: command.command,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "yesorno" => {
            reject_text_label(&command)?;
            Ok(ScriptTextAction::YesNo {
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "writetext" | "jumptext" | "jumptextfaceplayer" => {
            let text_label = require_known_text_label(&command, text_labels)?.to_string();
            Ok(ScriptTextAction::Write {
                command: command.command.clone(),
                text_label,
                face_player: command.command == "jumptextfaceplayer",
                closes_text: command.command == "jumptext"
                    || command.command == "jumptextfaceplayer",
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        other => Err(ScriptTextCommandError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn apply_script_text_command(
    state: &mut GameState,
    command: ScriptTextCommand,
    text_labels: &BTreeSet<String>,
) -> Result<ScriptTextAction, ScriptTextCommandError> {
    let action = resolve_script_text_command(command, text_labels)?;
    apply_script_text_action_to_state(state, &action);
    Ok(action)
}

pub fn apply_script_text_action_to_state(state: &mut GameState, action: &ScriptTextAction) {
    match action {
        ScriptTextAction::Open {
            source_script,
            command_index,
        } => {
            state.script_runtime.text_window_open = true;
            state
                .script_runtime
                .text_events
                .push(ScriptTextRuntimeEvent {
                    command: "opentext".to_string(),
                    kind: ScriptTextRuntimeKind::Open,
                    text_label: None,
                    face_player: false,
                    closes_text: false,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptTextAction::Close {
            source_script,
            command_index,
        } => {
            state.script_runtime.text_window_open = false;
            state.script_runtime.pending_text_label = None;
            state.script_runtime.pending_text_wait = None;
            state.script_runtime.pending_yes_no = None;
            state
                .script_runtime
                .text_events
                .push(ScriptTextRuntimeEvent {
                    command: "closetext".to_string(),
                    kind: ScriptTextRuntimeKind::Close,
                    text_label: None,
                    face_player: false,
                    closes_text: false,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptTextAction::WaitButton {
            command,
            source_script,
            command_index,
        } => {
            state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                command: command.clone(),
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state
                .script_runtime
                .text_events
                .push(ScriptTextRuntimeEvent {
                    command: command.clone(),
                    kind: ScriptTextRuntimeKind::WaitButton,
                    text_label: None,
                    face_player: false,
                    closes_text: false,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptTextAction::YesNo {
            source_script,
            command_index,
        } => {
            state.script_runtime.pending_yes_no = Some(ScriptYesNoPrompt {
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state
                .script_runtime
                .text_events
                .push(ScriptTextRuntimeEvent {
                    command: "yesorno".to_string(),
                    kind: ScriptTextRuntimeKind::YesNo,
                    text_label: None,
                    face_player: false,
                    closes_text: false,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptTextAction::Write {
            command,
            text_label,
            face_player,
            closes_text,
            source_script,
            command_index,
        } => {
            state.script_runtime.text_window_open = true;
            state.script_runtime.pending_text_label = Some(text_label.clone());
            if *closes_text {
                state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                    command: command.clone(),
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
            }
            state
                .script_runtime
                .text_events
                .push(ScriptTextRuntimeEvent {
                    command: command.clone(),
                    kind: ScriptTextRuntimeKind::Write,
                    text_label: Some(text_label.clone()),
                    face_player: *face_player,
                    closes_text: *closes_text,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
    }
}

fn require_known_text_label<'a>(
    command: &'a ScriptTextCommand,
    text_labels: &BTreeSet<String>,
) -> Result<&'a str, ScriptTextCommandError> {
    let text_label =
        command
            .text_label
            .as_deref()
            .ok_or_else(|| ScriptTextCommandError::MissingTextLabel {
                command: command.command.clone(),
            })?;
    if !text_labels.contains(text_label) {
        return Err(ScriptTextCommandError::UnknownTextLabel {
            command: command.command.clone(),
            text_label: text_label.to_string(),
        });
    }
    Ok(text_label)
}

fn reject_text_label(command: &ScriptTextCommand) -> Result<(), ScriptTextCommandError> {
    if command.text_label.is_some() {
        Err(ScriptTextCommandError::UnexpectedTextLabel {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> BTreeSet<String> {
        BTreeSet::from(["GreetingText".to_string(), "SignText".to_string()])
    }

    fn command(name: &str, text_label: Option<&str>) -> ScriptTextCommand {
        ScriptTextCommand {
            command: name.to_string(),
            text_label: text_label.map(str::to_string),
            source_script: "TextScript".to_string(),
            command_index: 3,
        }
    }

    #[test]
    fn resolves_dialog_flow_and_text_writes() {
        assert_eq!(
            resolve_script_text_command(command("opentext", None), &labels()).expect("open"),
            ScriptTextAction::Open {
                source_script: "TextScript".to_string(),
                command_index: 3,
            }
        );
        assert_eq!(
            resolve_script_text_command(command("writetext", Some("GreetingText")), &labels())
                .expect("write"),
            ScriptTextAction::Write {
                command: "writetext".to_string(),
                text_label: "GreetingText".to_string(),
                face_player: false,
                closes_text: false,
                source_script: "TextScript".to_string(),
                command_index: 3,
            }
        );
        assert_eq!(
            resolve_script_text_command(command("jumptextfaceplayer", Some("SignText")), &labels())
                .expect("jump face"),
            ScriptTextAction::Write {
                command: "jumptextfaceplayer".to_string(),
                text_label: "SignText".to_string(),
                face_player: true,
                closes_text: true,
                source_script: "TextScript".to_string(),
                command_index: 3,
            }
        );
    }

    #[test]
    fn rejects_case_changed_or_unexpected_text_labels() {
        assert!(matches!(
            resolve_script_text_command(command("writetext", Some("greetingtext")), &labels()),
            Err(ScriptTextCommandError::UnknownTextLabel { .. })
        ));
        assert!(matches!(
            resolve_script_text_command(command("waitbutton", Some("GreetingText")), &labels()),
            Err(ScriptTextCommandError::UnexpectedTextLabel { .. })
        ));
    }

    #[test]
    fn applies_text_flow_to_runtime_state() {
        let mut state = GameState::default();
        apply_script_text_command(&mut state, command("opentext", None), &labels()).expect("open");
        apply_script_text_command(
            &mut state,
            command("writetext", Some("GreetingText")),
            &labels(),
        )
        .expect("write");
        apply_script_text_command(&mut state, command("waitbutton", None), &labels())
            .expect("wait");

        assert!(state.script_runtime.text_window_open);
        assert_eq!(
            state.script_runtime.pending_text_label.as_deref(),
            Some("GreetingText")
        );
        assert_eq!(
            state.script_runtime.pending_text_wait,
            Some(ScriptTextWait {
                command: "waitbutton".to_string(),
                source_script: "TextScript".to_string(),
                command_index: 3,
            })
        );
        assert_eq!(state.script_runtime.text_events.len(), 3);
        assert_eq!(
            state.script_runtime.text_events[1].kind,
            ScriptTextRuntimeKind::Write
        );
    }

    #[test]
    fn applies_jumptextfaceplayer_as_write_that_closes_text() {
        let mut state = GameState::default();
        apply_script_text_command(
            &mut state,
            command("jumptextfaceplayer", Some("SignText")),
            &labels(),
        )
        .expect("jump text");

        assert!(state.script_runtime.text_window_open);
        assert_eq!(
            state.script_runtime.pending_text_label.as_deref(),
            Some("SignText")
        );
        assert_eq!(
            state.script_runtime.pending_text_wait,
            Some(ScriptTextWait {
                command: "jumptextfaceplayer".to_string(),
                source_script: "TextScript".to_string(),
                command_index: 3,
            })
        );
        assert!(state.script_runtime.text_events[0].face_player);
        assert!(state.script_runtime.text_events[0].closes_text);
    }

    #[test]
    fn applies_yesorno_without_implicit_answer() {
        let mut state = GameState::default();
        apply_script_text_command(&mut state, command("yesorno", None), &labels()).expect("yes no");

        assert_eq!(
            state.script_runtime.pending_yes_no,
            Some(ScriptYesNoPrompt {
                source_script: "TextScript".to_string(),
                command_index: 3,
            })
        );
        assert_eq!(
            state.script_runtime.text_events[0].kind,
            ScriptTextRuntimeKind::YesNo
        );
        assert_eq!(state.script_runtime.script_value, None);
    }

    #[test]
    fn close_text_clears_pending_text_state() {
        let mut state = GameState::default();
        apply_script_text_command(
            &mut state,
            command("writetext", Some("GreetingText")),
            &labels(),
        )
        .expect("write");
        apply_script_text_command(&mut state, command("yesorno", None), &labels()).expect("yes no");
        apply_script_text_command(&mut state, command("closetext", None), &labels())
            .expect("close");

        assert!(!state.script_runtime.text_window_open);
        assert_eq!(state.script_runtime.pending_text_label, None);
        assert_eq!(state.script_runtime.pending_text_wait, None);
        assert_eq!(state.script_runtime.pending_yes_no, None);
    }

    #[test]
    fn invalid_text_command_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        let error = apply_script_text_command(
            &mut state,
            command("writetext", Some("greetingtext")),
            &labels(),
        )
        .expect_err("case-changed label rejected");

        assert!(matches!(
            error,
            ScriptTextCommandError::UnknownTextLabel { .. }
        ));
        assert!(state.script_runtime.text_events.is_empty());
        assert!(!state.script_runtime.text_window_open);
        assert_eq!(state.script_runtime.pending_text_label, None);
    }

    #[test]
    fn script_text_json_commands_require_explicit_args() {
        let error = serde_json::from_str::<ScriptTextBodyCommand>(
            r#"{"command":"text","command_index":0}"#,
        )
        .expect_err("missing body command args must not default to empty")
        .to_string();
        assert!(error.contains("missing field `args`"), "{error}");

        let error = serde_json::from_str::<ScriptMenuCommand>(
            r#"{"command":"verticalmenu","command_index":0}"#,
        )
        .expect_err("missing menu command args must not default to empty")
        .to_string();
        assert!(error.contains("missing field `args`"), "{error}");
    }
}
