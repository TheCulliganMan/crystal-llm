use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::GameState;

pub const MAX_PHONE_CONTACTS: usize = 10;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct PhoneContactCatalog(pub BTreeMap<String, PhoneContactRecord>);

impl<'de> Deserialize<'de> for PhoneContactCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let contacts = BTreeMap::<String, PhoneContactRecord>::deserialize(deserializer)?;
        for (contact_id, record) in &contacts {
            if !is_exact_phone_token(contact_id) {
                return Err(serde::de::Error::custom(format!(
                    "phone contact catalog entry id '{contact_id}' must be exact ASCII alphanumeric or underscore"
                )));
            }
            if record.contact_id != *contact_id {
                return Err(serde::de::Error::custom(format!(
                    "phone contact catalog entry id '{contact_id}' must match record contactId '{}'",
                    record.contact_id
                )));
            }
        }
        Ok(Self(contacts))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermanentPhoneNumberRule {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhoneContactRecord {
    pub contact_id: String,
    pub trainer_class: Option<String>,
    pub trainer_label: Option<String>,
    pub lines: Vec<String>,
    pub primary_label: String,
    pub map_constant: Option<String>,
    pub callee_time_mask: u8,
    pub callee_script: Option<String>,
    pub caller_time_mask: u8,
    pub caller_script: Option<String>,
}

impl<'de> Deserialize<'de> for PhoneContactRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawRecord {
            #[serde(deserialize_with = "required_phone_contact_id")]
            contact_id: String,
            #[serde(deserialize_with = "optional_phone_trainer_class")]
            trainer_class: Option<String>,
            #[serde(deserialize_with = "optional_phone_trainer_label")]
            trainer_label: Option<String>,
            lines: Vec<String>,
            primary_label: String,
            #[serde(deserialize_with = "optional_phone_map_constant")]
            map_constant: Option<String>,
            callee_time_mask: u8,
            #[serde(deserialize_with = "optional_phone_callee_script")]
            callee_script: Option<String>,
            caller_time_mask: u8,
            #[serde(deserialize_with = "optional_phone_caller_script")]
            caller_script: Option<String>,
        }

        let raw = RawRecord::deserialize(deserializer)?;
        if raw.lines.is_empty() || raw.lines.iter().any(|line| line.trim().is_empty()) {
            return Err(serde::de::Error::custom(format!(
                "phone contact {} must declare nonempty dialogue lines",
                raw.contact_id
            )));
        }
        if raw.primary_label.trim().is_empty() {
            return Err(serde::de::Error::custom(format!(
                "phone contact {} must declare a nonempty primaryLabel",
                raw.contact_id
            )));
        }
        let expected_primary = raw
            .lines
            .first()
            .expect("checked nonempty lines")
            .trim_end_matches(':')
            .trim();
        if expected_primary != raw.primary_label {
            return Err(serde::de::Error::custom(format!(
                "phone contact {} primaryLabel {:?} does not match first line {:?}",
                raw.contact_id,
                raw.primary_label,
                raw.lines.first().expect("checked nonempty lines")
            )));
        }

        Ok(Self {
            contact_id: raw.contact_id,
            trainer_class: raw.trainer_class,
            trainer_label: raw.trainer_label,
            lines: raw.lines,
            primary_label: raw.primary_label,
            map_constant: raw.map_constant,
            callee_time_mask: raw.callee_time_mask,
            callee_script: raw.callee_script,
            caller_time_mask: raw.caller_time_mask,
            caller_script: raw.caller_script,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPhoneCommand {
    #[serde(deserialize_with = "required_script_phone_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_phone_contact_id")]
    pub contact_id: String,
    #[serde(deserialize_with = "required_phone_source_script")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptPhoneCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptPhoneCommand {
            #[serde(deserialize_with = "required_script_phone_command_token")]
            command: String,
            #[serde(deserialize_with = "required_phone_contact_id")]
            contact_id: String,
            #[serde(deserialize_with = "required_phone_source_script")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptPhoneCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            contact_id: raw.contact_id,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_phone_command_shape(&command).map_err(D::Error::custom)?;
        Ok(command)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPhoneInputs {
    pub accepted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptPhoneOutcome {
    CheckCellNum {
        contact_id: String,
        registered: bool,
        script_value: String,
        source_script: String,
        command_index: usize,
    },
    AskForPhoneNumber {
        contact_id: String,
        result: PhoneRegistrationResult,
        script_value: String,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PhoneRegistrationResult {
    Registered,
    AlreadyRegistered,
    ContactsFull,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptPhoneError {
    #[error("invalid script phone command '{command}'")]
    InvalidCommand { command: String },
    #[error("unknown script phone command '{command}'")]
    UnknownCommand { command: String },
    #[error("script phone command '{command}' references unknown contact '{contact_id}'")]
    UnknownContact { command: String, contact_id: String },
    #[error("script phone command '{command}' references empty contact id")]
    EmptyContact { command: String },
    #[error(
        "script phone command '{command}' references whitespace-padded contact id '{contact_id}'"
    )]
    PaddedContact { command: String, contact_id: String },
    #[error("script phone source script '{source_script}' is invalid")]
    InvalidSourceScript { source_script: String },
    #[error("script phone command 'askforphonenumber' requires an explicit accepted/refused input")]
    MissingPhoneChoice,
    #[error("saved phone number '{contact_id}' is not present in the modpack phone catalog")]
    UnknownSavedContact { contact_id: String },
    #[error("saved phone number '{contact_id}' is not an exact modpack phone contact id")]
    InvalidSavedContact { contact_id: String },
    #[error("permanent phone number '{contact_id}' is not present in the modpack phone catalog")]
    UnknownPermanentContact { contact_id: String },
    #[error("permanent phone number '{contact_id}' is not an exact modpack phone contact id")]
    InvalidPermanentContact { contact_id: String },
    #[error("permanent phone numbers exceed exact phone contact capacity {capacity}")]
    PermanentContactsExceedCapacity { capacity: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptPhoneCommandIssue {
    pub source_script: String,
    pub command_index: usize,
    pub contact_id: String,
    pub error: ScriptPhoneError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneContactCatalogIssue {
    EmptyContactId {
        contact_id: String,
    },
    InvalidContactId {
        contact_id: String,
    },
    ContactIdMismatch {
        contact_id: String,
        record_contact_id: String,
    },
    InvalidTrainerClass {
        contact_id: String,
        trainer_class: String,
    },
    InvalidTrainerLabel {
        contact_id: String,
        trainer_label: String,
    },
    EmptyPrimaryLabel {
        contact_id: String,
    },
    InvalidLines {
        contact_id: String,
    },
    PrimaryLabelMismatch {
        contact_id: String,
        primary_label: String,
        first_line: String,
    },
    EmptyMapConstant {
        contact_id: String,
    },
    InvalidMapConstant {
        contact_id: String,
        map_constant: String,
    },
    UnknownMapConstant {
        contact_id: String,
        map_constant: String,
    },
    InvalidCalleeScript {
        contact_id: String,
        callee_script: String,
    },
    InvalidCallerScript {
        contact_id: String,
        caller_script: String,
    },
    UnknownPermanentContact {
        contact_id: String,
    },
    InvalidPermanentContact {
        contact_id: String,
    },
}

pub fn phone_contact_catalog_issues(
    catalog: &PhoneContactCatalog,
    permanent_phone_numbers: &BTreeMap<String, PermanentPhoneNumberRule>,
    map_constants: &BTreeMap<String, String>,
) -> Vec<PhoneContactCatalogIssue> {
    let mut issues = Vec::new();
    for (contact_id, record) in &catalog.0 {
        if contact_id.trim().is_empty() {
            issues.push(PhoneContactCatalogIssue::EmptyContactId {
                contact_id: contact_id.clone(),
            });
        } else if !is_exact_phone_token(contact_id) {
            issues.push(PhoneContactCatalogIssue::InvalidContactId {
                contact_id: contact_id.clone(),
            });
        }
        if record.contact_id != *contact_id {
            issues.push(PhoneContactCatalogIssue::ContactIdMismatch {
                contact_id: contact_id.clone(),
                record_contact_id: record.contact_id.clone(),
            });
        }
        if let Some(trainer_class) = record.trainer_class.as_deref() {
            if !is_exact_phone_token(trainer_class) {
                issues.push(PhoneContactCatalogIssue::InvalidTrainerClass {
                    contact_id: contact_id.clone(),
                    trainer_class: trainer_class.to_string(),
                });
            }
        }
        if let Some(trainer_label) = record.trainer_label.as_deref() {
            if !is_exact_phone_token(trainer_label) {
                issues.push(PhoneContactCatalogIssue::InvalidTrainerLabel {
                    contact_id: contact_id.clone(),
                    trainer_label: trainer_label.to_string(),
                });
            }
        }
        if record.primary_label.trim().is_empty() {
            issues.push(PhoneContactCatalogIssue::EmptyPrimaryLabel {
                contact_id: contact_id.clone(),
            });
        }
        if record.lines.is_empty() || record.lines.iter().any(|line| line.trim().is_empty()) {
            issues.push(PhoneContactCatalogIssue::InvalidLines {
                contact_id: contact_id.clone(),
            });
        } else if let Some(first_line) = record.lines.first() {
            let expected_primary = first_line.trim_end_matches(':').trim();
            if expected_primary != record.primary_label {
                issues.push(PhoneContactCatalogIssue::PrimaryLabelMismatch {
                    contact_id: contact_id.clone(),
                    primary_label: record.primary_label.clone(),
                    first_line: first_line.clone(),
                });
            }
        }
        if let Some(map_constant) = record.map_constant.as_deref() {
            if map_constant.trim().is_empty() {
                issues.push(PhoneContactCatalogIssue::EmptyMapConstant {
                    contact_id: contact_id.clone(),
                });
            } else if !is_exact_phone_token(map_constant) {
                issues.push(PhoneContactCatalogIssue::InvalidMapConstant {
                    contact_id: contact_id.clone(),
                    map_constant: map_constant.to_string(),
                });
            } else if !map_constants.contains_key(map_constant) {
                issues.push(PhoneContactCatalogIssue::UnknownMapConstant {
                    contact_id: contact_id.clone(),
                    map_constant: map_constant.to_string(),
                });
            }
        }
        if let Some(callee_script) = record.callee_script.as_deref() {
            if !is_exact_phone_token(callee_script) {
                issues.push(PhoneContactCatalogIssue::InvalidCalleeScript {
                    contact_id: contact_id.clone(),
                    callee_script: callee_script.to_string(),
                });
            }
        }
        if let Some(caller_script) = record.caller_script.as_deref() {
            if !is_exact_phone_token(caller_script) {
                issues.push(PhoneContactCatalogIssue::InvalidCallerScript {
                    contact_id: contact_id.clone(),
                    caller_script: caller_script.to_string(),
                });
            }
        }
    }
    for contact_id in permanent_phone_numbers.keys() {
        if !is_exact_phone_token(contact_id) {
            issues.push(PhoneContactCatalogIssue::InvalidPermanentContact {
                contact_id: contact_id.clone(),
            });
        } else if !catalog.0.contains_key(contact_id) {
            issues.push(PhoneContactCatalogIssue::UnknownPermanentContact {
                contact_id: contact_id.clone(),
            });
        }
    }
    issues
}

pub const SCRIPT_PHONE_REGISTRATION_COMMANDS: &[&str] = &["askforphonenumber"];
pub const SCRIPT_PHONE_CHECK_COMMANDS: &[&str] = &["checkcellnum"];

pub fn is_known_script_phone_command(command: &str) -> bool {
    SCRIPT_PHONE_REGISTRATION_COMMANDS.contains(&command)
        || SCRIPT_PHONE_CHECK_COMMANDS.contains(&command)
}

pub fn apply_script_phone_command(
    state: &mut GameState,
    command: ScriptPhoneCommand,
    catalog: &PhoneContactCatalog,
    permanent_phone_numbers: &BTreeMap<String, PermanentPhoneNumberRule>,
    inputs: ScriptPhoneInputs,
) -> Result<ScriptPhoneOutcome, ScriptPhoneError> {
    validate_script_phone_command(&command, catalog)?;
    validate_saved_phone_numbers(&state.script_runtime.phone_numbers, catalog)?;
    validate_permanent_phone_numbers(permanent_phone_numbers, catalog)?;

    match command.command.as_str() {
        "checkcellnum" => {
            let registered =
                has_phone_number(state, permanent_phone_numbers, command.contact_id.as_str());
            let script_value = if registered { "1" } else { "0" }.to_string();
            state.script_runtime.script_value = Some(script_value.clone());
            Ok(ScriptPhoneOutcome::CheckCellNum {
                contact_id: command.contact_id,
                registered,
                script_value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "askforphonenumber" => {
            let accepted = inputs
                .accepted
                .ok_or(ScriptPhoneError::MissingPhoneChoice)?;
            let (result, script_value) = if !accepted {
                (PhoneRegistrationResult::Refused, "2".to_string())
            } else {
                match register_phone_number(
                    &mut state.script_runtime.phone_numbers,
                    command.contact_id.as_str(),
                    catalog,
                    permanent_phone_numbers,
                )? {
                    PhoneRegistrationResult::Registered => {
                        (PhoneRegistrationResult::Registered, "0".to_string())
                    }
                    PhoneRegistrationResult::AlreadyRegistered => {
                        (PhoneRegistrationResult::AlreadyRegistered, "1".to_string())
                    }
                    PhoneRegistrationResult::ContactsFull => {
                        (PhoneRegistrationResult::ContactsFull, "1".to_string())
                    }
                    PhoneRegistrationResult::Refused => {
                        unreachable!("accepted input cannot refuse")
                    }
                }
            };
            state.script_runtime.script_value = Some(script_value.clone());
            Ok(ScriptPhoneOutcome::AskForPhoneNumber {
                contact_id: command.contact_id,
                result,
                script_value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        other => Err(ScriptPhoneError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn initialize_permanent_phone_numbers(
    state: &mut GameState,
    catalog: &PhoneContactCatalog,
    permanent_phone_numbers: &BTreeMap<String, PermanentPhoneNumberRule>,
) -> Result<Vec<String>, ScriptPhoneError> {
    validate_permanent_phone_numbers(permanent_phone_numbers, catalog)?;
    if permanent_phone_numbers.len() > MAX_PHONE_CONTACTS {
        return Err(ScriptPhoneError::PermanentContactsExceedCapacity {
            capacity: MAX_PHONE_CONTACTS,
        });
    }
    let mut inserted = Vec::new();
    for contact_id in permanent_phone_numbers.keys() {
        if state
            .script_runtime
            .phone_numbers
            .insert(contact_id.clone())
        {
            inserted.push(contact_id.clone());
        }
    }
    Ok(inserted)
}

pub fn register_phone_number(
    phone_numbers: &mut BTreeSet<String>,
    contact_id: &str,
    catalog: &PhoneContactCatalog,
    permanent_phone_numbers: &BTreeMap<String, PermanentPhoneNumberRule>,
) -> Result<PhoneRegistrationResult, ScriptPhoneError> {
    validate_contact_id("askforphonenumber", contact_id, catalog)?;
    validate_saved_phone_numbers(phone_numbers, catalog)?;
    validate_permanent_phone_numbers(permanent_phone_numbers, catalog)?;

    if phone_numbers.contains(contact_id) || permanent_phone_numbers.contains_key(contact_id) {
        return Ok(PhoneRegistrationResult::AlreadyRegistered);
    }

    let missing_permanent = permanent_phone_numbers
        .keys()
        .filter(|permanent| permanent.as_str() != contact_id)
        .filter(|permanent| !phone_numbers.contains(*permanent))
        .count();
    let capacity = MAX_PHONE_CONTACTS.checked_sub(missing_permanent).ok_or(
        ScriptPhoneError::PermanentContactsExceedCapacity {
            capacity: MAX_PHONE_CONTACTS,
        },
    )?;
    if phone_numbers.len() >= capacity {
        return Ok(PhoneRegistrationResult::ContactsFull);
    }

    phone_numbers.insert(contact_id.to_string());
    Ok(PhoneRegistrationResult::Registered)
}

pub fn validate_script_phone_command(
    command: &ScriptPhoneCommand,
    catalog: &PhoneContactCatalog,
) -> Result<(), ScriptPhoneError> {
    validate_script_phone_command_token(&command.command)?;
    validate_phone_source_script(&command.source_script)?;
    match command.command.as_str() {
        "askforphonenumber" | "checkcellnum" => {
            validate_contact_id(&command.command, &command.contact_id, catalog)
        }
        other => Err(ScriptPhoneError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn script_phone_command_issues(
    commands: &[ScriptPhoneCommand],
    catalog: &PhoneContactCatalog,
) -> Vec<ScriptPhoneCommandIssue> {
    commands
        .iter()
        .filter_map(
            |command| match validate_script_phone_command(command, catalog) {
                Err(
                    error @ (ScriptPhoneError::InvalidCommand { .. }
                    | ScriptPhoneError::UnknownCommand { .. }
                    | ScriptPhoneError::UnknownContact { .. }
                    | ScriptPhoneError::EmptyContact { .. }
                    | ScriptPhoneError::PaddedContact { .. }
                    | ScriptPhoneError::InvalidSourceScript { .. }),
                ) => Some(ScriptPhoneCommandIssue {
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                    contact_id: command.contact_id.clone(),
                    error,
                }),
                _ => None,
            },
        )
        .collect()
}

fn validate_script_phone_command_shape(command: &ScriptPhoneCommand) -> Result<(), String> {
    validate_script_phone_command_token(&command.command).map_err(|error| error.to_string())?;
    validate_phone_source_script(&command.source_script).map_err(|error| error.to_string())?;
    if command.contact_id.is_empty() {
        return Err(format!(
            "script phone command {} references empty contact id",
            command.command
        ));
    }
    if !is_exact_phone_token(&command.contact_id) {
        return Err(format!(
            "script phone command {} references invalid contact id {}",
            command.command, command.contact_id
        ));
    }
    Ok(())
}

fn validate_script_phone_command_token(command: &str) -> Result<(), ScriptPhoneError> {
    if !is_exact_script_phone_command_token(command) {
        Err(ScriptPhoneError::InvalidCommand {
            command: command.to_string(),
        })
    } else if is_known_script_phone_command(command) {
        Ok(())
    } else {
        Err(ScriptPhoneError::UnknownCommand {
            command: command.to_string(),
        })
    }
}

fn is_exact_script_phone_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
}

fn has_phone_number(
    state: &GameState,
    permanent_phone_numbers: &BTreeMap<String, PermanentPhoneNumberRule>,
    contact_id: &str,
) -> bool {
    state.script_runtime.phone_numbers.contains(contact_id)
        || permanent_phone_numbers.contains_key(contact_id)
}

fn validate_contact_id(
    command: &str,
    contact_id: &str,
    catalog: &PhoneContactCatalog,
) -> Result<(), ScriptPhoneError> {
    if contact_id.is_empty() {
        return Err(ScriptPhoneError::EmptyContact {
            command: command.to_string(),
        });
    }
    if !is_exact_phone_token(contact_id) {
        return Err(ScriptPhoneError::PaddedContact {
            command: command.to_string(),
            contact_id: contact_id.to_string(),
        });
    }
    if !catalog.0.contains_key(contact_id) {
        return Err(ScriptPhoneError::UnknownContact {
            command: command.to_string(),
            contact_id: contact_id.to_string(),
        });
    }
    Ok(())
}

fn is_exact_phone_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_phone_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn validate_phone_source_script(source_script: &str) -> Result<(), ScriptPhoneError> {
    if is_exact_phone_label_token(source_script) {
        Ok(())
    } else {
        Err(ScriptPhoneError::InvalidSourceScript {
            source_script: source_script.to_string(),
        })
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn required_script_phone_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validate_script_phone_command_token(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_phone_source_script<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validate_phone_source_script(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_phone_contact_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_phone_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "phone contact record contactId '{value}' must be exact ASCII alphanumeric or underscore"
        )))
    }
}

fn optional_phone_trainer_class<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_phone_token(deserializer, "phone contact trainerClass")
}

fn optional_phone_trainer_label<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_phone_token(deserializer, "phone contact trainerLabel")
}

fn optional_phone_map_constant<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_phone_token(deserializer, "phone contact mapConstant")
}

fn optional_phone_callee_script<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_phone_token(deserializer, "phone contact calleeScript")
}

fn optional_phone_caller_script<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_phone_token(deserializer, "phone contact callerScript")
}

fn optional_phone_token<'de, D>(deserializer: D, label: &str) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if !is_exact_phone_token(&token) => Err(serde::de::Error::custom(format!(
            "{label} '{token}' must be exact ASCII alphanumeric or underscore"
        ))),
        other => Ok(other),
    }
}

fn validate_saved_phone_numbers(
    phone_numbers: &BTreeSet<String>,
    catalog: &PhoneContactCatalog,
) -> Result<(), ScriptPhoneError> {
    for contact_id in phone_numbers {
        if !is_exact_phone_token(contact_id) {
            return Err(ScriptPhoneError::InvalidSavedContact {
                contact_id: contact_id.clone(),
            });
        }
        if !catalog.0.contains_key(contact_id) {
            return Err(ScriptPhoneError::UnknownSavedContact {
                contact_id: contact_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_permanent_phone_numbers(
    permanent_phone_numbers: &BTreeMap<String, PermanentPhoneNumberRule>,
    catalog: &PhoneContactCatalog,
) -> Result<(), ScriptPhoneError> {
    for contact_id in permanent_phone_numbers.keys() {
        if !is_exact_phone_token(contact_id) {
            return Err(ScriptPhoneError::InvalidPermanentContact {
                contact_id: contact_id.clone(),
            });
        }
        if !catalog.0.contains_key(contact_id) {
            return Err(ScriptPhoneError::UnknownPermanentContact {
                contact_id: contact_id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> PhoneContactCatalog {
        PhoneContactCatalog(BTreeMap::from([
            ("PHONE_MOM".to_string(), record("PHONE_MOM")),
            ("PHONE_ELM".to_string(), record("PHONE_ELM")),
            ("PHONE_JOEY".to_string(), record("PHONE_JOEY")),
            ("PHONE_BILL".to_string(), record("PHONE_BILL")),
            ("PHONE_BUENA".to_string(), record("PHONE_BUENA")),
            ("PHONE_KENJI".to_string(), record("PHONE_KENJI")),
            ("PHONE_REENA".to_string(), record("PHONE_REENA")),
            ("PHONE_LIZ".to_string(), record("PHONE_LIZ")),
            ("PHONE_ANTHONY".to_string(), record("PHONE_ANTHONY")),
            ("PHONE_BIKE_SHOP".to_string(), record("PHONE_BIKE_SHOP")),
            ("PHONE_EXTRA".to_string(), record("PHONE_EXTRA")),
        ]))
    }

    fn record(contact_id: &str) -> PhoneContactRecord {
        PhoneContactRecord {
            contact_id: contact_id.to_string(),
            trainer_class: None,
            trainer_label: None,
            lines: vec![format!("{contact_id}:")],
            primary_label: contact_id.to_string(),
            map_constant: None,
            callee_time_mask: 0,
            callee_script: None,
            caller_time_mask: 0,
            caller_script: None,
        }
    }

    fn command(name: &str, contact_id: &str) -> ScriptPhoneCommand {
        ScriptPhoneCommand {
            command: name.to_string(),
            contact_id: contact_id.to_string(),
            source_script: "PhoneScript".to_string(),
            command_index: 8,
        }
    }

    fn permanent_numbers<const N: usize>(
        ids: [&str; N],
    ) -> BTreeMap<String, PermanentPhoneNumberRule> {
        ids.into_iter()
            .map(|id| (id.to_string(), PermanentPhoneNumberRule::default()))
            .collect()
    }

    #[test]
    fn phone_contact_catalog_issues_validate_exact_ids_and_map_constants() {
        let mut mismatch = record("PHONE_MISMATCH_RECORD");
        mismatch.primary_label = "PHONE_MISMATCH".to_string();
        mismatch.lines = vec!["PHONE_MISMATCH:".to_string()];
        let mut bad_lines = record("PHONE_BAD_LINES");
        bad_lines.lines = vec![String::new()];
        let mut bad_optionals = record("PHONE_BAD_OPTIONALS");
        bad_optionals.trainer_class = Some("TRAINER NONE".to_string());
        bad_optionals.trainer_label = Some("PHONECONTACT ELM".to_string());
        bad_optionals.callee_script = Some("Elm Phone Calee".to_string());
        bad_optionals.caller_script = Some("Unused Phone Script".to_string());
        let mut label_mismatch = record("PHONE_LABEL");
        label_mismatch.primary_label = "OTHER".to_string();
        label_mismatch.lines = vec!["PHONE_LABEL:".to_string()];
        let mut empty_map = record("PHONE_EMPTY_MAP");
        empty_map.map_constant = Some(String::new());
        let mut unknown_map = record("PHONE_UNKNOWN_MAP");
        unknown_map.map_constant = Some("elms_lab".to_string());
        let mut invalid_map = record("PHONE_INVALID_MAP");
        invalid_map.map_constant = Some("ELMS LAB".to_string());
        let catalog = PhoneContactCatalog(BTreeMap::from([
            ("".to_string(), record("")),
            (" PHONE_PADDED".to_string(), record(" PHONE_PADDED")),
            ("PHONE PADDED".to_string(), record("PHONE PADDED")),
            ("PHONE_MISMATCH".to_string(), mismatch),
            ("PHONE_BAD_LINES".to_string(), bad_lines),
            ("PHONE_BAD_OPTIONALS".to_string(), bad_optionals),
            ("PHONE_LABEL".to_string(), label_mismatch),
            ("PHONE_EMPTY_MAP".to_string(), empty_map),
            ("PHONE_INVALID_MAP".to_string(), invalid_map),
            ("PHONE_UNKNOWN_MAP".to_string(), unknown_map),
        ]));
        let permanent = permanent_numbers(["PHONE MOM", "phone_mom"]);
        let map_constants = BTreeMap::from([("ELMS_LAB".to_string(), "ElmsLab".to_string())]);

        assert_eq!(
            phone_contact_catalog_issues(&catalog, &permanent, &map_constants),
            vec![
                PhoneContactCatalogIssue::EmptyContactId {
                    contact_id: String::new(),
                },
                PhoneContactCatalogIssue::EmptyPrimaryLabel {
                    contact_id: String::new(),
                },
                PhoneContactCatalogIssue::InvalidContactId {
                    contact_id: " PHONE_PADDED".to_string(),
                },
                PhoneContactCatalogIssue::PrimaryLabelMismatch {
                    contact_id: " PHONE_PADDED".to_string(),
                    primary_label: " PHONE_PADDED".to_string(),
                    first_line: " PHONE_PADDED:".to_string(),
                },
                PhoneContactCatalogIssue::InvalidContactId {
                    contact_id: "PHONE PADDED".to_string(),
                },
                PhoneContactCatalogIssue::InvalidLines {
                    contact_id: "PHONE_BAD_LINES".to_string(),
                },
                PhoneContactCatalogIssue::InvalidTrainerClass {
                    contact_id: "PHONE_BAD_OPTIONALS".to_string(),
                    trainer_class: "TRAINER NONE".to_string(),
                },
                PhoneContactCatalogIssue::InvalidTrainerLabel {
                    contact_id: "PHONE_BAD_OPTIONALS".to_string(),
                    trainer_label: "PHONECONTACT ELM".to_string(),
                },
                PhoneContactCatalogIssue::InvalidCalleeScript {
                    contact_id: "PHONE_BAD_OPTIONALS".to_string(),
                    callee_script: "Elm Phone Calee".to_string(),
                },
                PhoneContactCatalogIssue::InvalidCallerScript {
                    contact_id: "PHONE_BAD_OPTIONALS".to_string(),
                    caller_script: "Unused Phone Script".to_string(),
                },
                PhoneContactCatalogIssue::EmptyMapConstant {
                    contact_id: "PHONE_EMPTY_MAP".to_string(),
                },
                PhoneContactCatalogIssue::InvalidMapConstant {
                    contact_id: "PHONE_INVALID_MAP".to_string(),
                    map_constant: "ELMS LAB".to_string(),
                },
                PhoneContactCatalogIssue::PrimaryLabelMismatch {
                    contact_id: "PHONE_LABEL".to_string(),
                    primary_label: "OTHER".to_string(),
                    first_line: "PHONE_LABEL:".to_string(),
                },
                PhoneContactCatalogIssue::ContactIdMismatch {
                    contact_id: "PHONE_MISMATCH".to_string(),
                    record_contact_id: "PHONE_MISMATCH_RECORD".to_string(),
                },
                PhoneContactCatalogIssue::UnknownMapConstant {
                    contact_id: "PHONE_UNKNOWN_MAP".to_string(),
                    map_constant: "elms_lab".to_string(),
                },
                PhoneContactCatalogIssue::InvalidPermanentContact {
                    contact_id: "PHONE MOM".to_string(),
                },
                PhoneContactCatalogIssue::UnknownPermanentContact {
                    contact_id: "phone_mom".to_string(),
                },
            ]
        );
    }

    #[test]
    fn phone_contact_catalog_issues_reject_reserved_pack_prefix_tokens() {
        let mut reserved = record("fallback_phone_mom");
        reserved.trainer_class = Some("legacy_trainer".to_string());
        reserved.trainer_label = Some("fallback_label".to_string());
        reserved.map_constant = Some("legacy_map".to_string());
        reserved.callee_script = Some("fallback_callee".to_string());
        reserved.caller_script = Some("legacy_caller".to_string());
        let catalog = PhoneContactCatalog(BTreeMap::from([(
            "fallback_phone_mom".to_string(),
            reserved,
        )]));
        let permanent = permanent_numbers(["legacy_phone_mom"]);

        assert_eq!(
            phone_contact_catalog_issues(&catalog, &permanent, &BTreeMap::new()),
            vec![
                PhoneContactCatalogIssue::InvalidContactId {
                    contact_id: "fallback_phone_mom".to_string(),
                },
                PhoneContactCatalogIssue::InvalidTrainerClass {
                    contact_id: "fallback_phone_mom".to_string(),
                    trainer_class: "legacy_trainer".to_string(),
                },
                PhoneContactCatalogIssue::InvalidTrainerLabel {
                    contact_id: "fallback_phone_mom".to_string(),
                    trainer_label: "fallback_label".to_string(),
                },
                PhoneContactCatalogIssue::InvalidMapConstant {
                    contact_id: "fallback_phone_mom".to_string(),
                    map_constant: "legacy_map".to_string(),
                },
                PhoneContactCatalogIssue::InvalidCalleeScript {
                    contact_id: "fallback_phone_mom".to_string(),
                    callee_script: "fallback_callee".to_string(),
                },
                PhoneContactCatalogIssue::InvalidCallerScript {
                    contact_id: "fallback_phone_mom".to_string(),
                    caller_script: "legacy_caller".to_string(),
                },
                PhoneContactCatalogIssue::InvalidPermanentContact {
                    contact_id: "legacy_phone_mom".to_string(),
                },
            ]
        );
    }

    #[test]
    fn exported_phone_command_sets_are_exact() {
        assert!(SCRIPT_PHONE_REGISTRATION_COMMANDS.contains(&"askforphonenumber"));
        assert!(SCRIPT_PHONE_CHECK_COMMANDS.contains(&"checkcellnum"));
        assert!(is_known_script_phone_command("checkcellnum"));
        assert!(!is_known_script_phone_command("CheckCellNum"));
        assert!(!is_known_script_phone_command("deletecellnum"));
    }

    #[test]
    fn script_phone_command_issues_preserve_exact_source_positions() {
        let commands = vec![
            command("checkcellnum", "PHONE_MOM"),
            command("CheckCellNum", "PHONE_MOM"),
            command("deletecellnum", "PHONE_MOM"),
            command("checkcellnum", "phone_mom"),
            command("askforphonenumber", ""),
            command("askforphonenumber", " PHONE_MOM"),
            command("askforphonenumber", "PHONE MOM"),
            command("fallbackphone", "PHONE_MOM"),
            command("askforphonenumber", "legacy_phone_mom"),
            {
                let mut command = command("checkcellnum", "PHONE_MOM");
                command.source_script = "fallback_script".to_string();
                command
            },
        ];

        assert_eq!(
            script_phone_command_issues(&commands, &catalog()),
            vec![
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "PHONE_MOM".to_string(),
                    error: ScriptPhoneError::InvalidCommand {
                        command: "CheckCellNum".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "PHONE_MOM".to_string(),
                    error: ScriptPhoneError::UnknownCommand {
                        command: "deletecellnum".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "phone_mom".to_string(),
                    error: ScriptPhoneError::UnknownContact {
                        command: "checkcellnum".to_string(),
                        contact_id: "phone_mom".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: String::new(),
                    error: ScriptPhoneError::EmptyContact {
                        command: "askforphonenumber".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: " PHONE_MOM".to_string(),
                    error: ScriptPhoneError::PaddedContact {
                        command: "askforphonenumber".to_string(),
                        contact_id: " PHONE_MOM".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "PHONE MOM".to_string(),
                    error: ScriptPhoneError::PaddedContact {
                        command: "askforphonenumber".to_string(),
                        contact_id: "PHONE MOM".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "PHONE_MOM".to_string(),
                    error: ScriptPhoneError::InvalidCommand {
                        command: "fallbackphone".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "legacy_phone_mom".to_string(),
                    error: ScriptPhoneError::PaddedContact {
                        command: "askforphonenumber".to_string(),
                        contact_id: "legacy_phone_mom".to_string(),
                    },
                },
                ScriptPhoneCommandIssue {
                    source_script: "fallback_script".to_string(),
                    command_index: 8,
                    contact_id: "PHONE_MOM".to_string(),
                    error: ScriptPhoneError::InvalidSourceScript {
                        source_script: "fallback_script".to_string(),
                    },
                },
            ]
        );
    }

    #[test]
    fn checkcellnum_sets_exact_numeric_script_value() {
        let mut state = GameState::default();
        let permanent = permanent_numbers(["PHONE_MOM"]);
        let outcome = apply_script_phone_command(
            &mut state,
            command("checkcellnum", "PHONE_MOM"),
            &catalog(),
            &permanent,
            ScriptPhoneInputs::default(),
        )
        .expect("check permanent");
        assert_eq!(
            outcome,
            ScriptPhoneOutcome::CheckCellNum {
                contact_id: "PHONE_MOM".to_string(),
                registered: true,
                script_value: "1".to_string(),
                source_script: "PhoneScript".to_string(),
                command_index: 8,
            }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

        apply_script_phone_command(
            &mut state,
            command("checkcellnum", "PHONE_JOEY"),
            &catalog(),
            &permanent,
            ScriptPhoneInputs::default(),
        )
        .expect("check missing");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

        let mut invalid_source = command("checkcellnum", "PHONE_MOM");
        invalid_source.source_script = "legacy_script".to_string();
        assert_eq!(
            apply_script_phone_command(
                &mut state,
                invalid_source,
                &catalog(),
                &permanent,
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::InvalidSourceScript {
                source_script: "legacy_script".to_string(),
            })
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    }

    #[test]
    fn askforphonenumber_requires_explicit_input_and_registers_exact_contact() {
        let mut state = GameState::default();
        assert!(matches!(
            apply_script_phone_command(
                &mut state,
                command("askforphonenumber", "PHONE_JOEY"),
                &catalog(),
                &BTreeMap::new(),
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::MissingPhoneChoice)
        ));

        let outcome = apply_script_phone_command(
            &mut state,
            command("askforphonenumber", "PHONE_JOEY"),
            &catalog(),
            &BTreeMap::new(),
            ScriptPhoneInputs {
                accepted: Some(true),
            },
        )
        .expect("register");
        assert_eq!(
            outcome,
            ScriptPhoneOutcome::AskForPhoneNumber {
                contact_id: "PHONE_JOEY".to_string(),
                result: PhoneRegistrationResult::Registered,
                script_value: "0".to_string(),
                source_script: "PhoneScript".to_string(),
                command_index: 8,
            }
        );
        assert!(state.script_runtime.phone_numbers.contains("PHONE_JOEY"));
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

        apply_script_phone_command(
            &mut state,
            command("askforphonenumber", "PHONE_ELM"),
            &catalog(),
            &BTreeMap::new(),
            ScriptPhoneInputs {
                accepted: Some(false),
            },
        )
        .expect("refuse");
        assert!(!state.script_runtime.phone_numbers.contains("PHONE_ELM"));
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
    }

    #[test]
    fn phone_registration_reserves_missing_permanent_slots() {
        let mut state = GameState::default();
        for contact_id in [
            "PHONE_JOEY",
            "PHONE_BILL",
            "PHONE_BUENA",
            "PHONE_KENJI",
            "PHONE_REENA",
            "PHONE_LIZ",
            "PHONE_ANTHONY",
            "PHONE_BIKE_SHOP",
        ] {
            state
                .script_runtime
                .phone_numbers
                .insert(contact_id.to_string());
        }
        let permanent = permanent_numbers(["PHONE_MOM", "PHONE_ELM"]);
        let outcome = apply_script_phone_command(
            &mut state,
            command("askforphonenumber", "PHONE_EXTRA"),
            &catalog(),
            &permanent,
            ScriptPhoneInputs {
                accepted: Some(true),
            },
        )
        .expect("full");
        assert_eq!(
            outcome,
            ScriptPhoneOutcome::AskForPhoneNumber {
                contact_id: "PHONE_EXTRA".to_string(),
                result: PhoneRegistrationResult::ContactsFull,
                script_value: "1".to_string(),
                source_script: "PhoneScript".to_string(),
                command_index: 8,
            }
        );
        assert!(!state.script_runtime.phone_numbers.contains("PHONE_EXTRA"));
    }

    #[test]
    fn initializes_permanent_phone_numbers_from_pack() {
        let mut state = GameState::default();
        let inserted = initialize_permanent_phone_numbers(
            &mut state,
            &catalog(),
            &permanent_numbers(["PHONE_MOM", "PHONE_ELM"]),
        )
        .expect("initialize");
        assert_eq!(inserted, vec!["PHONE_MOM", "PHONE_ELM"]);
        assert!(state.script_runtime.phone_numbers.contains("PHONE_MOM"));
        assert!(state.script_runtime.phone_numbers.contains("PHONE_ELM"));
    }

    #[test]
    fn phone_commands_reject_unknown_or_case_changed_contacts() {
        let mut state = GameState::default();
        assert!(matches!(
            apply_script_phone_command(
                &mut state,
                command("checkcellnum", "phone_mom"),
                &catalog(),
                &BTreeMap::new(),
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::UnknownContact { .. })
        ));
        state
            .script_runtime
            .phone_numbers
            .insert("PHONE_UNKNOWN".to_string());
        assert!(matches!(
            apply_script_phone_command(
                &mut state,
                command("checkcellnum", "PHONE_MOM"),
                &catalog(),
                &BTreeMap::new(),
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::UnknownSavedContact { .. })
        ));
        state.script_runtime.phone_numbers.clear();
        state
            .script_runtime
            .phone_numbers
            .insert("PHONE UNKNOWN".to_string());
        assert!(matches!(
            apply_script_phone_command(
                &mut state,
                command("checkcellnum", "PHONE_MOM"),
                &catalog(),
                &BTreeMap::new(),
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::InvalidSavedContact { .. })
        ));
        assert!(matches!(
            initialize_permanent_phone_numbers(
                &mut GameState::default(),
                &catalog(),
                &permanent_numbers(["PHONE MOM"]),
            ),
            Err(ScriptPhoneError::InvalidPermanentContact { .. })
        ));
    }

    #[test]
    fn script_phone_command_json_rejects_reserved_pack_tokens() {
        for (field, value) in [
            ("command", serde_json::json!("fallbackphone")),
            ("contact_id", serde_json::json!("legacy_phone_mom")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "checkcellnum",
                "contact_id": "PHONE_MOM",
                "source_script": "PhoneScript",
                "command_index": 8
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptPhoneCommand>(payload)
                .expect_err("reserved phone command tokens must fail during JSON load")
                .to_string();
            assert!(
                error.contains("phone"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn phone_contact_json_requires_explicit_lines() {
        let error = serde_json::from_str::<PhoneContactRecord>(
            r#"{
              "contactId":"PHONE_MOM",
              "trainerClass":"TRAINER_NONE",
              "trainerLabel":"PHONECONTACT_MOM",
              "primaryLabel":"MOM",
              "mapConstant":"PLAYERS_HOUSE_1F",
              "calleeTimeMask":7,
              "calleeScript":"MomPhoneCalleeScript",
              "callerTimeMask":0,
              "callerScript":"UnusedPhoneScript"
            }"#,
        )
        .expect_err("missing phone contact lines must not default to empty")
        .to_string();

        assert!(error.contains("missing field `lines`"), "{error}");

        let catalog_error = serde_json::from_str::<PhoneContactCatalog>(
            r#"{"contacts":{"PHONE_MOM":{"contactId":"PHONE_MOM","trainerClass":null,"trainerLabel":null,"lines":["Mom:"],"primaryLabel":"MOM","mapConstant":null,"calleeTimeMask":0,"calleeScript":null,"callerTimeMask":0,"callerScript":null}},"fallback_contact":"PHONE_ELM"}"#,
        )
        .expect_err("phone contact catalogs must be the compiler-emitted contact map")
        .to_string();
        assert!(
            catalog_error.contains("invalid type")
                || catalog_error.contains("invalid value")
                || catalog_error.contains("unknown field"),
            "{catalog_error}"
        );

        let key_error = serde_json::from_str::<PhoneContactCatalog>(
            r#"{"Phone Elm":{"contactId":"PhoneElm","trainerClass":null,"trainerLabel":null,"lines":["PhoneElm:"],"primaryLabel":"PhoneElm","mapConstant":null,"calleeTimeMask":0,"calleeScript":null,"callerTimeMask":0,"callerScript":null}}"#,
        )
        .expect_err("phone contact catalog keys must be exact during JSON load")
        .to_string();
        assert!(
            key_error.contains(
                "phone contact catalog entry id 'Phone Elm' must be exact ASCII alphanumeric or underscore"
            ),
            "{key_error}"
        );

        let field_error = serde_json::from_str::<PhoneContactRecord>(
            r#"{"contactId":"PhoneElm","trainerClass":"TRAINER NONE","trainerLabel":null,"lines":["PhoneElm:"],"primaryLabel":"PhoneElm","mapConstant":null,"calleeTimeMask":0,"calleeScript":null,"callerTimeMask":0,"callerScript":null}"#,
        )
        .expect_err("phone contact token fields must be exact during JSON load")
        .to_string();
        assert!(
            field_error.contains(
                "phone contact trainerClass 'TRAINER NONE' must be exact ASCII alphanumeric or underscore"
            ),
            "{field_error}"
        );
    }

    #[test]
    fn script_phone_serialized_variants_reject_unknown_fallback_fields() {
        let outcome_error = serde_json::from_value::<ScriptPhoneOutcome>(serde_json::json!({
            "check_cell_num": {
                "contact_id": "PHONE_MOM",
                "registered": true,
                "script_value": "TRUE",
                "source_script": "MomScript",
                "command_index": 2,
                "fallback_contact_id": "PHONE_DEFAULT"
            }
        }))
        .expect_err("fallback contact id must be rejected")
        .to_string();
        assert!(
            outcome_error.contains("unknown field `fallback_contact_id`"),
            "{outcome_error}"
        );

        let command_error = serde_json::from_value::<ScriptPhoneCommand>(serde_json::json!({
            "command": "checkcellnum",
            "contact_id": "PHONE_MOM",
            "source_script": "MomScript",
            "command_index": 2,
            "normalized_command": "checkcellnum"
        }))
        .expect_err("normalized command must be rejected")
        .to_string();
        assert!(
            command_error.contains("unknown field `normalized_command`"),
            "{command_error}"
        );
    }
}
