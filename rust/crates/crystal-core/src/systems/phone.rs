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
