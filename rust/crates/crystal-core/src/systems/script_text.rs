use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::{
    GameState, ScriptTextRuntimeEvent, ScriptTextRuntimeKind, ScriptTextWait, ScriptYesNoPrompt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextCommand {
    #[serde(deserialize_with = "required_script_text_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_script_label_token")]
    pub text_label: Option<String>,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptTextCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptTextCommand {
            #[serde(deserialize_with = "required_script_text_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_script_label_token")]
            text_label: Option<String>,
            #[serde(deserialize_with = "required_script_label_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptTextCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            text_label: raw.text_label,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_text_command_shape(&command).map_err(D::Error::custom)?;
        Ok(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextBody {
    #[serde(deserialize_with = "required_script_label_token")]
    pub label: String,
    pub commands: Vec<ScriptTextBodyCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextBodyCommand {
    #[serde(deserialize_with = "required_script_text_command_token")]
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptTextBodyCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptTextBodyCommand {
            #[serde(deserialize_with = "required_script_text_command_token")]
            command: String,
            args: Vec<String>,
            command_index: usize,
        }

        let raw = RawScriptTextBodyCommand::deserialize(deserializer)?;
        validate_fixed_arg_command_shape(
            "script text body",
            &text_body_command_arg_counts(),
            &raw.command,
            raw.args.len(),
        )
        .map_err(D::Error::custom)?;
        Ok(Self {
            command: raw.command,
            args: raw.args,
            command_index: raw.command_index,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMenuDefinition {
    #[serde(deserialize_with = "required_script_label_token")]
    pub label: String,
    pub commands: Vec<ScriptMenuCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMenuCommand {
    #[serde(deserialize_with = "required_script_text_command_token")]
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptMenuCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptMenuCommand {
            #[serde(deserialize_with = "required_script_text_command_token")]
            command: String,
            args: Vec<String>,
            command_index: usize,
        }

        let raw = RawScriptMenuCommand::deserialize(deserializer)?;
        validate_variable_arg_command_shape(
            "script menu",
            &menu_definition_command_arg_counts(),
            &raw.command,
            raw.args.len(),
        )
        .map_err(D::Error::custom)?;
        Ok(Self {
            command: raw.command,
            args: raw.args,
            command_index: raw.command_index,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptTextBodyIssue {
    InvalidKey {
        key: String,
    },
    InvalidLabel {
        label: String,
    },
    LabelMismatch {
        key: String,
        label: String,
    },
    UnknownCommand {
        command_index: usize,
        command: String,
    },
    MalformedCommand {
        command_index: usize,
        command: String,
        expected: usize,
        actual: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptMenuDefinitionIssue {
    InvalidKey {
        key: String,
    },
    InvalidLabel {
        label: String,
    },
    LabelMismatch {
        key: String,
        label: String,
    },
    UnknownCommand {
        command_index: usize,
        command: String,
    },
    MalformedCommand {
        command_index: usize,
        command: String,
        expected: BTreeSet<usize>,
        actual: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmTextCatalogIssue {
    InvalidText { label: String },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct AsmTextTable(pub BTreeMap<String, String>);

impl<'de> Deserialize<'de> for AsmTextTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = BTreeMap::<String, String>::deserialize(deserializer)?;
        if values.is_empty() {
            return Err(D::Error::custom("ASM text table must not be empty"));
        }
        if let Some(issue) = asm_text_catalog_issues(&values).into_iter().next() {
            return Err(D::Error::custom(format!(
                "invalid ASM text table entry: {issue:?}"
            )));
        }
        Ok(Self(values))
    }
}

pub fn asm_text_catalog_issues(asm_text: &BTreeMap<String, String>) -> Vec<AsmTextCatalogIssue> {
    asm_text
        .iter()
        .filter(|(label, text)| !is_exact_nonempty_label(label) || !is_exact_nonempty_text(text))
        .map(|(label, _)| AsmTextCatalogIssue::InvalidText {
            label: label.clone(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub enum ScriptTextCommandError {
    #[error("script text command '{command}' is not exact pack syntax")]
    InvalidCommand { command: String },
    #[error("unknown script text command '{command}'")]
    UnknownCommand { command: String },
    #[error("script text command '{command}' is missing a text label")]
    MissingTextLabel { command: String },
    #[error("script text command '{command}' has invalid text label '{text_label}'")]
    InvalidTextLabel { command: String, text_label: String },
    #[error("script text command '{command}' references unknown text label '{text_label}'")]
    UnknownTextLabel { command: String, text_label: String },
    #[error("script text command '{command}' has unexpected text label")]
    UnexpectedTextLabel { command: String },
    #[error("script text source script '{source_script}' is not exact pack syntax")]
    InvalidSourceScript { source_script: String },
}

pub const SCRIPT_TEXT_NO_LABEL_COMMANDS: &[&str] = &[
    "opentext",
    "closetext",
    "promptbutton",
    "waitbutton",
    "yesorno",
];

pub const SCRIPT_TEXT_LABEL_COMMANDS: &[&str] = &[
    "writetext",
    "farwritetext",
    "jumptext",
    "jumptextfaceplayer",
    "farjumptext",
];

pub fn is_known_script_text_command(command: &str) -> bool {
    SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&command)
        || SCRIPT_TEXT_LABEL_COMMANDS.contains(&command)
}

fn validate_script_text_command_shape(command: &ScriptTextCommand) -> Result<(), String> {
    if !is_known_script_text_command(&command.command) {
        return Err(format!("unknown script text command {}", command.command));
    }
    if SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&command.command.as_str()) {
        if command.text_label.is_some() {
            return Err(format!(
                "script text command {} must not declare text_label",
                command.command
            ));
        }
    } else if command.text_label.is_none() {
        return Err(format!(
            "script text command {} requires text_label",
            command.command
        ));
    }
    Ok(())
}

pub fn script_text_command_issues(
    command: &ScriptTextCommand,
    text_labels: &BTreeSet<String>,
) -> Vec<ScriptTextCommandError> {
    let mut issues = Vec::new();
    if SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&command.command.as_str()) {
        if command.text_label.is_some() {
            issues.push(ScriptTextCommandError::UnexpectedTextLabel {
                command: command.command.clone(),
            });
        }
    } else if SCRIPT_TEXT_LABEL_COMMANDS.contains(&command.command.as_str()) {
        match command.text_label.as_deref() {
            Some(text_label) if !is_exact_nonempty_label(text_label) => {
                issues.push(ScriptTextCommandError::InvalidTextLabel {
                    command: command.command.clone(),
                    text_label: text_label.to_string(),
                });
            }
            Some(text_label) if text_labels.contains(text_label) => {}
            Some(text_label) => issues.push(ScriptTextCommandError::UnknownTextLabel {
                command: command.command.clone(),
                text_label: text_label.to_string(),
            }),
            None => issues.push(ScriptTextCommandError::MissingTextLabel {
                command: command.command.clone(),
            }),
        }
    } else if !is_exact_script_text_command_token(&command.command) {
        issues.push(ScriptTextCommandError::InvalidCommand {
            command: command.command.clone(),
        });
    } else {
        issues.push(ScriptTextCommandError::UnknownCommand {
            command: command.command.clone(),
        });
    }
    issues
}

pub fn script_text_body_issues(key: &str, body: &ScriptTextBody) -> Vec<ScriptTextBodyIssue> {
    let expected_arg_counts = text_body_command_arg_counts();
    let mut issues = Vec::new();
    if !is_exact_nonempty_label(key) {
        issues.push(ScriptTextBodyIssue::InvalidKey {
            key: key.to_string(),
        });
    }
    if !is_exact_nonempty_label(&body.label) {
        issues.push(ScriptTextBodyIssue::InvalidLabel {
            label: body.label.clone(),
        });
    }
    if body.label != key {
        issues.push(ScriptTextBodyIssue::LabelMismatch {
            key: key.to_string(),
            label: body.label.clone(),
        });
    }
    for command in &body.commands {
        let Some(expected) = expected_arg_counts.get(command.command.as_str()) else {
            issues.push(ScriptTextBodyIssue::UnknownCommand {
                command_index: command.command_index,
                command: command.command.clone(),
            });
            continue;
        };
        if command.args.len() != *expected {
            issues.push(ScriptTextBodyIssue::MalformedCommand {
                command_index: command.command_index,
                command: command.command.clone(),
                expected: *expected,
                actual: command.args.len(),
            });
        }
    }
    issues
}

pub fn script_menu_definition_issues(
    key: &str,
    menu: &ScriptMenuDefinition,
) -> Vec<ScriptMenuDefinitionIssue> {
    let expected_arg_counts = menu_definition_command_arg_counts();
    let mut issues = Vec::new();
    if !is_exact_nonempty_label(key) {
        issues.push(ScriptMenuDefinitionIssue::InvalidKey {
            key: key.to_string(),
        });
    }
    if !is_exact_nonempty_label(&menu.label) {
        issues.push(ScriptMenuDefinitionIssue::InvalidLabel {
            label: menu.label.clone(),
        });
    }
    if menu.label != key {
        issues.push(ScriptMenuDefinitionIssue::LabelMismatch {
            key: key.to_string(),
            label: menu.label.clone(),
        });
    }
    for command in &menu.commands {
        let Some(expected) = expected_arg_counts.get(command.command.as_str()) else {
            issues.push(ScriptMenuDefinitionIssue::UnknownCommand {
                command_index: command.command_index,
                command: command.command.clone(),
            });
            continue;
        };
        if !expected.contains(&command.args.len()) {
            issues.push(ScriptMenuDefinitionIssue::MalformedCommand {
                command_index: command.command_index,
                command: command.command.clone(),
                expected: expected.clone(),
                actual: command.args.len(),
            });
        }
    }
    issues
}

pub fn text_body_command_arg_counts() -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("text", 1),
        ("text_start", 0),
        ("text_block", 1),
        ("line", 1),
        ("para", 1),
        ("cont", 1),
        ("next", 1),
        ("done", 0),
        ("text_end", 0),
        ("prompt", 0),
        ("text_promptbutton", 0),
        ("text_ram", 1),
        ("text_decimal", 3),
        ("text_far", 1),
        ("sound_item", 0),
        ("sound_dex_fanfare_50_79", 0),
        ("sound_dex_fanfare_80_109", 0),
        ("sound_dex_fanfare_140_169", 0),
        ("sound_dex_fanfare_170_199", 0),
        ("sound_dex_fanfare_200_229", 0),
        ("sound_dex_fanfare_230_plus", 0),
    ])
}

pub fn menu_definition_command_arg_counts() -> BTreeMap<&'static str, BTreeSet<usize>> {
    BTreeMap::from([
        ("db", BTreeSet::from([1, 3])),
        ("dn", BTreeSet::from([2])),
        ("dba", BTreeSet::from([1])),
        ("dbw", BTreeSet::from([2])),
        ("menu_coords", BTreeSet::from([4])),
        ("dw", BTreeSet::from([1])),
    ])
}

fn validate_fixed_arg_command_shape(
    section: &str,
    counts: &BTreeMap<&'static str, usize>,
    command: &str,
    actual: usize,
) -> Result<(), String> {
    let Some(expected) = counts.get(command) else {
        return Err(format!(
            "{section} command {command:?} is not a Crystal command"
        ));
    };
    if *expected != actual {
        return Err(format!(
            "{section} command {command} has {actual} args, expected {expected}"
        ));
    }
    Ok(())
}

fn validate_variable_arg_command_shape(
    section: &str,
    counts: &BTreeMap<&'static str, BTreeSet<usize>>,
    command: &str,
    actual: usize,
) -> Result<(), String> {
    let Some(expected) = counts.get(command) else {
        return Err(format!(
            "{section} command {command:?} is not a Crystal command"
        ));
    };
    if !expected.contains(&actual) {
        return Err(format!(
            "{section} command {command} has {actual} args, expected {expected:?}"
        ));
    }
    Ok(())
}

pub fn resolve_script_text_command(
    command: ScriptTextCommand,
    text_labels: &BTreeSet<String>,
) -> Result<ScriptTextAction, ScriptTextCommandError> {
    if !is_exact_script_text_command_token(&command.command) {
        return Err(ScriptTextCommandError::InvalidCommand {
            command: command.command,
        });
    }
    if !is_exact_nonempty_label(&command.source_script) {
        return Err(ScriptTextCommandError::InvalidSourceScript {
            source_script: command.source_script,
        });
    }
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
        "writetext" | "farwritetext" | "jumptext" | "jumptextfaceplayer" | "farjumptext" => {
            let text_label = require_known_text_label(&command, text_labels)?.to_string();
            Ok(ScriptTextAction::Write {
                command: command.command.clone(),
                text_label,
                face_player: command.command == "jumptextfaceplayer",
                closes_text: command.command == "jumptext"
                    || command.command == "jumptextfaceplayer"
                    || command.command == "farjumptext",
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
            state.script_runtime.pending_text_label = None;
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
    if !is_exact_nonempty_label(text_label) {
        return Err(ScriptTextCommandError::InvalidTextLabel {
            command: command.command.clone(),
            text_label: text_label.to_string(),
        });
    }
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

fn is_exact_nonempty_label(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn is_exact_nonempty_text(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .chars()
            .all(|character| character == '\n' || character == '\r' || !character.is_control())
}

fn is_exact_script_text_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn required_script_text_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_text_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script text command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_script_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_label(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script text label must be exact ASCII label syntax, found {value:?}"
        )))
    }
}

fn required_nullable_script_label_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_nonempty_label(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script text label must be exact ASCII label syntax, found {token:?}"
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
    fn exported_text_command_sets_are_exact() {
        assert!(SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&"opentext"));
        assert!(SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&"yesorno"));
        assert!(SCRIPT_TEXT_LABEL_COMMANDS.contains(&"writetext"));
        assert!(SCRIPT_TEXT_LABEL_COMMANDS.contains(&"farwritetext"));
        assert!(SCRIPT_TEXT_LABEL_COMMANDS.contains(&"jumptextfaceplayer"));
        assert!(SCRIPT_TEXT_LABEL_COMMANDS.contains(&"farjumptext"));
        assert!(is_known_script_text_command("jumptext"));
        assert!(is_known_script_text_command("farwritetext"));
        assert!(is_known_script_text_command("farjumptext"));
        assert!(!is_known_script_text_command("JumpText"));
        assert!(!is_known_script_text_command("text"));
        assert_eq!(text_body_command_arg_counts()["text"], 1);
        assert_eq!(text_body_command_arg_counts()["text_start"], 0);
        assert_eq!(text_body_command_arg_counts()["text_block"], 1);
        assert_eq!(text_body_command_arg_counts()["text_promptbutton"], 0);
        assert_eq!(text_body_command_arg_counts()["text_end"], 0);
        assert_eq!(text_body_command_arg_counts()["next"], 1);
        assert_eq!(text_body_command_arg_counts()["text_decimal"], 3);
        assert_eq!(text_body_command_arg_counts()["sound_item"], 0);
        assert_eq!(
            text_body_command_arg_counts()["sound_dex_fanfare_230_plus"],
            0
        );
        assert!(!text_body_command_arg_counts().contains_key("text_default"));
        assert_eq!(
            menu_definition_command_arg_counts()["menu_coords"],
            BTreeSet::from([4])
        );
        assert_eq!(
            menu_definition_command_arg_counts()["db"],
            BTreeSet::from([1, 3])
        );
        assert!(!menu_definition_command_arg_counts().contains_key("verticalmenu"));
    }

    #[test]
    fn script_text_serialized_variants_reject_unknown_fallback_fields() {
        let action_error = serde_json::from_value::<ScriptTextAction>(serde_json::json!({
            "write": {
                "command": "writetext",
                "text_label": "GreetingText",
                "face_player": false,
                "closes_text": false,
                "source_script": "TextScript",
                "command_index": 3,
                "fallback_text_label": "DefaultText"
            }
        }))
        .expect_err("text actions must not accept fallback text labels");
        assert!(
            action_error
                .to_string()
                .contains("unknown field `fallback_text_label`"),
            "{action_error}"
        );

        let command_error = serde_json::from_value::<ScriptTextCommandError>(serde_json::json!({
            "UnknownTextLabel": {
                "command": "writetext",
                "text_label": "GreetingText",
                "legacy_text_label": "GREETING_TEXT"
            }
        }))
        .expect_err("text command errors must not accept legacy text labels");
        assert!(
            command_error
                .to_string()
                .contains("unknown field `legacy_text_label`"),
            "{command_error}"
        );
    }

    #[test]
    fn script_text_issue_collector_reports_exact_pack_shape_errors() {
        assert_eq!(
            script_text_command_issues(&command("waitbutton", Some("GreetingText")), &labels()),
            vec![ScriptTextCommandError::UnexpectedTextLabel {
                command: "waitbutton".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("jumptext", None), &labels()),
            vec![ScriptTextCommandError::MissingTextLabel {
                command: "jumptext".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("writetext", Some("greetingtext")), &labels()),
            vec![ScriptTextCommandError::UnknownTextLabel {
                command: "writetext".to_string(),
                text_label: "greetingtext".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("writetext", Some(" GreetingText")), &labels()),
            vec![ScriptTextCommandError::InvalidTextLabel {
                command: "writetext".to_string(),
                text_label: " GreetingText".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("writetext", Some("Greeting Text")), &labels()),
            vec![ScriptTextCommandError::InvalidTextLabel {
                command: "writetext".to_string(),
                text_label: "Greeting Text".to_string(),
            }]
        );
        let labels_with_invalid = BTreeSet::from(["Greeting Text".to_string()]);
        assert_eq!(
            script_text_command_issues(
                &command("writetext", Some("Greeting Text")),
                &labels_with_invalid,
            ),
            vec![ScriptTextCommandError::InvalidTextLabel {
                command: "writetext".to_string(),
                text_label: "Greeting Text".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("text", Some("GreetingText")), &labels()),
            vec![ScriptTextCommandError::UnknownCommand {
                command: "text".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("JumpText", Some("GreetingText")), &labels()),
            vec![ScriptTextCommandError::InvalidCommand {
                command: "JumpText".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("jump text", Some("GreetingText")), &labels()),
            vec![ScriptTextCommandError::InvalidCommand {
                command: "jump text".to_string(),
            }]
        );
    }

    #[test]
    fn script_text_commands_reject_reserved_pack_prefixes() {
        assert_eq!(
            script_text_command_issues(&command("fallbacktext", Some("GreetingText")), &labels()),
            vec![ScriptTextCommandError::InvalidCommand {
                command: "fallbacktext".to_string(),
            }]
        );
        assert_eq!(
            script_text_command_issues(&command("writetext", Some("legacy_text")), &labels()),
            vec![ScriptTextCommandError::InvalidTextLabel {
                command: "writetext".to_string(),
                text_label: "legacy_text".to_string(),
            }]
        );

        for (field, value) in [
            ("command", serde_json::json!("fallbacktext")),
            ("text_label", serde_json::json!("legacy_text")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "writetext",
                "text_label": "GreetingText",
                "source_script": ".branch@TextScript",
                "command_index": 3
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptTextCommand>(payload)
                .expect_err("reserved script text command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script text"),
                "{field} produced unexpected error: {error}"
            );
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
            resolve_script_text_command(command("farwritetext", Some("GreetingText")), &labels())
                .expect("far write"),
            ScriptTextAction::Write {
                command: "farwritetext".to_string(),
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
        assert_eq!(
            resolve_script_text_command(command("farjumptext", Some("SignText")), &labels())
                .expect("far jump text"),
            ScriptTextAction::Write {
                command: "farjumptext".to_string(),
                text_label: "SignText".to_string(),
                face_player: false,
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
            resolve_script_text_command(command("writetext", Some(" GreetingText")), &labels()),
            Err(ScriptTextCommandError::InvalidTextLabel { .. })
        ));
        assert!(matches!(
            resolve_script_text_command(command("writetext", Some("Greeting Text")), &labels()),
            Err(ScriptTextCommandError::InvalidTextLabel { .. })
        ));
        assert!(matches!(
            resolve_script_text_command(command("waitbutton", Some("GreetingText")), &labels()),
            Err(ScriptTextCommandError::UnexpectedTextLabel { .. })
        ));
        assert_eq!(
            resolve_script_text_command(command("JumpText", Some("GreetingText")), &labels()),
            Err(ScriptTextCommandError::InvalidCommand {
                command: "JumpText".to_string(),
            })
        );
        assert_eq!(
            resolve_script_text_command(command("text", Some("GreetingText")), &labels()),
            Err(ScriptTextCommandError::UnknownCommand {
                command: "text".to_string(),
            })
        );
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
    fn applies_jumptext_variants_as_writes_that_close_text() {
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

        apply_script_text_command(
            &mut state,
            command("farjumptext", Some("GreetingText")),
            &labels(),
        )
        .expect("far jump text");

        assert_eq!(
            state.script_runtime.pending_text_label.as_deref(),
            Some("GreetingText")
        );
        assert_eq!(
            state.script_runtime.pending_text_wait,
            Some(ScriptTextWait {
                command: "farjumptext".to_string(),
                source_script: "TextScript".to_string(),
                command_index: 3,
            })
        );
        assert!(!state.script_runtime.text_events[1].face_player);
        assert!(state.script_runtime.text_events[1].closes_text);
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
        assert_eq!(state.script_runtime.pending_text_label, None);
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
        assert_eq!(state.script_runtime.pending_text_label, None);
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

        let mut malformed_source = command("opentext", None);
        malformed_source.source_script = "fallback_script".to_string();
        let error = apply_script_text_command(&mut state, malformed_source, &labels())
            .expect_err("reserved source script rejected");
        assert_eq!(
            error,
            ScriptTextCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            }
        );
        assert!(state.script_runtime.text_events.is_empty());
        assert!(!state.script_runtime.text_window_open);
    }

    #[test]
    fn script_text_json_commands_require_explicit_args() {
        let error = serde_json::from_str::<ScriptTextCommand>(
            r#"{"text_label":null,"source_script":"Script","command_index":0}"#,
        )
        .expect_err("missing text command must not default to empty")
        .to_string();
        assert!(error.contains("missing field `command`"), "{error}");

        let error =
            serde_json::from_str::<ScriptTextBodyCommand>(r#"{"args":[],"command_index":0}"#)
                .expect_err("missing body command must not default to empty")
                .to_string();
        assert!(error.contains("missing field `command`"), "{error}");

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

    #[test]
    fn script_text_body_and_menu_json_reject_reserved_pack_prefixes() {
        for (field, value) in [
            ("label", serde_json::json!("fallback_text")),
            ("command", serde_json::json!("legacytext")),
        ] {
            let mut payload = serde_json::json!({
                "label": "GreetingText",
                "commands": [{
                    "command": "text",
                    "args": ["Hello there!"],
                    "command_index": 0
                }]
            });
            if field == "command" {
                payload["commands"][0]["command"] = value;
            } else {
                payload[field] = value;
            }

            let error = serde_json::from_value::<ScriptTextBody>(payload)
                .expect_err("reserved script text body tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script text"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("label", serde_json::json!("legacy_menu")),
            ("command", serde_json::json!("fallbackdb")),
        ] {
            let mut payload = serde_json::json!({
                "label": "ChoiceMenu",
                "commands": [{
                    "command": "db",
                    "args": ["MENU_BACKUP_TILES"],
                    "command_index": 0
                }]
            });
            if field == "command" {
                payload["commands"][0]["command"] = value;
            } else {
                payload[field] = value;
            }

            let error = serde_json::from_value::<ScriptMenuDefinition>(payload)
                .expect_err("reserved script menu tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script text"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn script_text_body_issues_validate_exact_label_and_command_arity() {
        let body = ScriptTextBody {
            label: "Other Text".to_string(),
            commands: vec![
                ScriptTextBodyCommand {
                    command: "text".to_string(),
                    args: Vec::new(),
                    command_index: 0,
                },
                ScriptTextBodyCommand {
                    command: "unknown_text_op".to_string(),
                    args: vec!["arg".to_string()],
                    command_index: 1,
                },
            ],
        };

        assert_eq!(
            script_text_body_issues("GreetingText", &body),
            vec![
                ScriptTextBodyIssue::InvalidLabel {
                    label: "Other Text".to_string(),
                },
                ScriptTextBodyIssue::LabelMismatch {
                    key: "GreetingText".to_string(),
                    label: "Other Text".to_string(),
                },
                ScriptTextBodyIssue::MalformedCommand {
                    command_index: 0,
                    command: "text".to_string(),
                    expected: 1,
                    actual: 0,
                },
                ScriptTextBodyIssue::UnknownCommand {
                    command_index: 1,
                    command: "unknown_text_op".to_string(),
                },
            ]
        );

        let exact_body = ScriptTextBody {
            label: "GreetingText".to_string(),
            commands: Vec::new(),
        };
        assert_eq!(
            script_text_body_issues(" GreetingText", &exact_body),
            vec![
                ScriptTextBodyIssue::InvalidKey {
                    key: " GreetingText".to_string(),
                },
                ScriptTextBodyIssue::LabelMismatch {
                    key: " GreetingText".to_string(),
                    label: "GreetingText".to_string(),
                },
            ]
        );
    }

    #[test]
    fn script_menu_definition_issues_validate_exact_label_and_command_arity() {
        let menu = ScriptMenuDefinition {
            label: "Other Menu".to_string(),
            commands: vec![
                ScriptMenuCommand {
                    command: "db".to_string(),
                    args: vec!["one".to_string(), "two".to_string()],
                    command_index: 0,
                },
                ScriptMenuCommand {
                    command: "verticalmenu".to_string(),
                    args: Vec::new(),
                    command_index: 1,
                },
            ],
        };

        assert_eq!(
            script_menu_definition_issues("ChoiceMenu", &menu),
            vec![
                ScriptMenuDefinitionIssue::InvalidLabel {
                    label: "Other Menu".to_string(),
                },
                ScriptMenuDefinitionIssue::LabelMismatch {
                    key: "ChoiceMenu".to_string(),
                    label: "Other Menu".to_string(),
                },
                ScriptMenuDefinitionIssue::MalformedCommand {
                    command_index: 0,
                    command: "db".to_string(),
                    expected: BTreeSet::from([1, 3]),
                    actual: 2,
                },
                ScriptMenuDefinitionIssue::UnknownCommand {
                    command_index: 1,
                    command: "verticalmenu".to_string(),
                },
            ]
        );

        let exact_menu = ScriptMenuDefinition {
            label: "ChoiceMenu".to_string(),
            commands: Vec::new(),
        };
        assert_eq!(
            script_menu_definition_issues(" ChoiceMenu", &exact_menu),
            vec![
                ScriptMenuDefinitionIssue::InvalidKey {
                    key: " ChoiceMenu".to_string(),
                },
                ScriptMenuDefinitionIssue::LabelMismatch {
                    key: " ChoiceMenu".to_string(),
                    label: "ChoiceMenu".to_string(),
                },
            ]
        );
    }

    #[test]
    fn asm_text_catalog_issues_require_nonempty_labels_and_text() {
        let asm_text = [
            ("".to_string(), "Hello!".to_string()),
            ("GreetingText".to_string(), "Hello!".to_string()),
            ("RouteSignText".to_string(), " ".to_string()),
            (" PaddedLabel".to_string(), "Hello!".to_string()),
            ("PaddedText".to_string(), " Hello!".to_string()),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            asm_text_catalog_issues(&asm_text),
            vec![
                AsmTextCatalogIssue::InvalidText {
                    label: String::new(),
                },
                AsmTextCatalogIssue::InvalidText {
                    label: " PaddedLabel".to_string(),
                },
                AsmTextCatalogIssue::InvalidText {
                    label: "PaddedText".to_string(),
                },
                AsmTextCatalogIssue::InvalidText {
                    label: "RouteSignText".to_string(),
                },
            ],
        );
    }
}
