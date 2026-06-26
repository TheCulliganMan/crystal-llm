use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::state::GameState;

pub const MAX_PHONE_CONTACTS: usize = 10;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneContactCatalog(pub BTreeMap<String, PhoneContactRecord>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPhoneCommand {
    pub command: String,
    pub contact_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPhoneInputs {
    pub accepted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum PhoneRegistrationResult {
    Registered,
    AlreadyRegistered,
    ContactsFull,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ScriptPhoneError {
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
    #[error("script phone command 'askforphonenumber' requires an explicit accepted/refused input")]
    MissingPhoneChoice,
    #[error("saved phone number '{contact_id}' is not present in the modpack phone catalog")]
    UnknownSavedContact { contact_id: String },
    #[error("permanent phone number '{contact_id}' is not present in the modpack phone catalog")]
    UnknownPermanentContact { contact_id: String },
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
    UnknownMapConstant {
        contact_id: String,
        map_constant: String,
    },
    UnknownPermanentContact {
        contact_id: String,
    },
}

pub fn phone_contact_catalog_issues(
    catalog: &PhoneContactCatalog,
    permanent_phone_numbers: &[String],
    map_constants: &BTreeMap<String, String>,
) -> Vec<PhoneContactCatalogIssue> {
    let mut issues = Vec::new();
    for (contact_id, record) in &catalog.0 {
        if contact_id.trim().is_empty() {
            issues.push(PhoneContactCatalogIssue::EmptyContactId {
                contact_id: contact_id.clone(),
            });
        } else if contact_id.trim() != contact_id {
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
            } else if !map_constants.contains_key(map_constant) {
                issues.push(PhoneContactCatalogIssue::UnknownMapConstant {
                    contact_id: contact_id.clone(),
                    map_constant: map_constant.to_string(),
                });
            }
        }
    }
    for contact_id in permanent_phone_numbers {
        if !catalog.0.contains_key(contact_id) {
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
    permanent_phone_numbers: &[String],
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
    permanent_phone_numbers: &[String],
) -> Result<Vec<String>, ScriptPhoneError> {
    validate_permanent_phone_numbers(permanent_phone_numbers, catalog)?;
    if permanent_phone_numbers.len() > MAX_PHONE_CONTACTS {
        return Err(ScriptPhoneError::PermanentContactsExceedCapacity {
            capacity: MAX_PHONE_CONTACTS,
        });
    }
    let mut inserted = Vec::new();
    for contact_id in permanent_phone_numbers {
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
    permanent_phone_numbers: &[String],
) -> Result<PhoneRegistrationResult, ScriptPhoneError> {
    validate_contact_id("askforphonenumber", contact_id, catalog)?;
    validate_saved_phone_numbers(phone_numbers, catalog)?;
    validate_permanent_phone_numbers(permanent_phone_numbers, catalog)?;

    if phone_numbers.contains(contact_id)
        || permanent_phone_numbers.iter().any(|id| id == contact_id)
    {
        return Ok(PhoneRegistrationResult::AlreadyRegistered);
    }

    let missing_permanent = permanent_phone_numbers
        .iter()
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
                    error @ (ScriptPhoneError::UnknownCommand { .. }
                    | ScriptPhoneError::UnknownContact { .. }
                    | ScriptPhoneError::EmptyContact { .. }
                    | ScriptPhoneError::PaddedContact { .. }),
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

fn has_phone_number(
    state: &GameState,
    permanent_phone_numbers: &[String],
    contact_id: &str,
) -> bool {
    state.script_runtime.phone_numbers.contains(contact_id)
        || permanent_phone_numbers.iter().any(|id| id == contact_id)
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
    if contact_id.trim() != contact_id {
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

fn validate_saved_phone_numbers(
    phone_numbers: &BTreeSet<String>,
    catalog: &PhoneContactCatalog,
) -> Result<(), ScriptPhoneError> {
    for contact_id in phone_numbers {
        if !catalog.0.contains_key(contact_id) {
            return Err(ScriptPhoneError::UnknownSavedContact {
                contact_id: contact_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_permanent_phone_numbers(
    permanent_phone_numbers: &[String],
    catalog: &PhoneContactCatalog,
) -> Result<(), ScriptPhoneError> {
    for contact_id in permanent_phone_numbers {
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

    #[test]
    fn phone_contact_catalog_issues_validate_exact_ids_and_map_constants() {
        let mut mismatch = record("PHONE_MISMATCH_RECORD");
        mismatch.primary_label = "PHONE_MISMATCH".to_string();
        mismatch.lines = vec!["PHONE_MISMATCH:".to_string()];
        let mut bad_lines = record("PHONE_BAD_LINES");
        bad_lines.lines = vec![String::new()];
        let mut label_mismatch = record("PHONE_LABEL");
        label_mismatch.primary_label = "OTHER".to_string();
        label_mismatch.lines = vec!["PHONE_LABEL:".to_string()];
        let mut empty_map = record("PHONE_EMPTY_MAP");
        empty_map.map_constant = Some(String::new());
        let mut unknown_map = record("PHONE_UNKNOWN_MAP");
        unknown_map.map_constant = Some("elms_lab".to_string());
        let catalog = PhoneContactCatalog(BTreeMap::from([
            ("".to_string(), record("")),
            (" PHONE_PADDED".to_string(), record(" PHONE_PADDED")),
            ("PHONE_MISMATCH".to_string(), mismatch),
            ("PHONE_BAD_LINES".to_string(), bad_lines),
            ("PHONE_LABEL".to_string(), label_mismatch),
            ("PHONE_EMPTY_MAP".to_string(), empty_map),
            ("PHONE_UNKNOWN_MAP".to_string(), unknown_map),
        ]));
        let permanent = vec!["phone_mom".to_string()];
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
                PhoneContactCatalogIssue::InvalidLines {
                    contact_id: "PHONE_BAD_LINES".to_string(),
                },
                PhoneContactCatalogIssue::EmptyMapConstant {
                    contact_id: "PHONE_EMPTY_MAP".to_string(),
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
                PhoneContactCatalogIssue::UnknownPermanentContact {
                    contact_id: "phone_mom".to_string(),
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
            command("checkcellnum", "phone_mom"),
            command("askforphonenumber", ""),
            command("askforphonenumber", " PHONE_MOM"),
        ];

        assert_eq!(
            script_phone_command_issues(&commands, &catalog()),
            vec![
                ScriptPhoneCommandIssue {
                    source_script: "PhoneScript".to_string(),
                    command_index: 8,
                    contact_id: "PHONE_MOM".to_string(),
                    error: ScriptPhoneError::UnknownCommand {
                        command: "CheckCellNum".to_string(),
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
            ]
        );
    }

    #[test]
    fn checkcellnum_sets_exact_numeric_script_value() {
        let mut state = GameState::default();
        let permanent = vec!["PHONE_MOM".to_string()];
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
    }

    #[test]
    fn askforphonenumber_requires_explicit_input_and_registers_exact_contact() {
        let mut state = GameState::default();
        assert!(matches!(
            apply_script_phone_command(
                &mut state,
                command("askforphonenumber", "PHONE_JOEY"),
                &catalog(),
                &[],
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::MissingPhoneChoice)
        ));

        let outcome = apply_script_phone_command(
            &mut state,
            command("askforphonenumber", "PHONE_JOEY"),
            &catalog(),
            &[],
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
            &[],
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
        let permanent = vec!["PHONE_MOM".to_string(), "PHONE_ELM".to_string()];
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
            &["PHONE_MOM".to_string(), "PHONE_ELM".to_string()],
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
                &[],
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
                &[],
                ScriptPhoneInputs::default(),
            ),
            Err(ScriptPhoneError::UnknownSavedContact { .. })
        ));
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
    }
}
