use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::Item;
use crate::state::GameState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemGrant {
    #[serde(deserialize_with = "required_script_item_grant_command")]
    pub command: String,
    #[serde(deserialize_with = "required_script_item_token")]
    pub item_id: String,
    pub quantity: u16,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemAccess {
    #[serde(deserialize_with = "required_script_item_access_command")]
    pub command: String,
    #[serde(deserialize_with = "required_script_item_token")]
    pub item_id: String,
    #[serde(deserialize_with = "required_script_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptItemGrantOutcome {
    Granted {
        item_id: String,
        quantity: u16,
        source_script: String,
        command_index: usize,
        verbose: bool,
    },
    BagFull {
        item_id: String,
        quantity: u16,
        source_script: String,
        command_index: usize,
        verbose: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemCheckOutcome {
    pub item_id: String,
    pub source_script: String,
    pub command_index: usize,
    pub held: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemTakeOutcome {
    pub item_id: String,
    pub source_script: String,
    pub command_index: usize,
    pub removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ScriptItemGrantError {
    InvalidCommand { command: String },
    InvalidItem { item_id: String },
    InvalidSourceScript { source_script: String },
    UnknownItem { item_id: String },
    InvalidQuantity,
    Bag { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ScriptItemAccessError {
    InvalidCommand { command: String },
    InvalidItem { item_id: String },
    InvalidSourceScript { source_script: String },
    UnknownItem { item_id: String },
    Bag { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptItemGrantIssue {
    InvalidCommand { command: String },
    InvalidItem { item_id: String },
    UnknownItem { item_id: String },
    InvalidQuantity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptItemAccessIssue {
    InvalidCommand { command: String },
    InvalidItem { item_id: String },
    UnknownItem { item_id: String },
}

pub const SCRIPT_ITEM_FROM_MEMORY_ID: &str = "ITEM_FROM_MEM";
pub const SCRIPT_ITEM_GRANT_COMMANDS: &[&str] = &["giveitem", "verbosegiveitem"];
pub const SCRIPT_ITEM_CHECK_COMMANDS: &[&str] = &["checkitem"];
pub const SCRIPT_ITEM_TAKE_COMMANDS: &[&str] = &["takeitem"];

pub fn is_known_script_item_grant_command(command: &str) -> bool {
    SCRIPT_ITEM_GRANT_COMMANDS.contains(&command)
}

pub fn is_known_script_item_check_command(command: &str) -> bool {
    SCRIPT_ITEM_CHECK_COMMANDS.contains(&command)
}

pub fn is_known_script_item_take_command(command: &str) -> bool {
    SCRIPT_ITEM_TAKE_COMMANDS.contains(&command)
}

pub fn is_known_script_item_access_command(command: &str) -> bool {
    is_known_script_item_check_command(command) || is_known_script_item_take_command(command)
}

pub fn validate_script_item_grant_command(command: &str) -> Result<(), String> {
    validate_exact_script_item_command(command)?;
    if is_known_script_item_grant_command(command) {
        Ok(())
    } else {
        Err(format!("unknown script item grant command '{command}'"))
    }
}

pub fn validate_script_item_access_command(command: &str) -> Result<(), String> {
    validate_exact_script_item_command(command)?;
    if is_known_script_item_access_command(command) {
        Ok(())
    } else {
        Err(format!("unknown script item access command '{command}'"))
    }
}

pub fn script_item_grant_issues(
    grant: &ScriptItemGrant,
    item_catalog: &BTreeMap<String, Item>,
) -> Vec<ScriptItemGrantIssue> {
    let mut issues = Vec::new();
    if grant.quantity == 0 {
        issues.push(ScriptItemGrantIssue::InvalidQuantity);
    }
    if validate_script_item_grant_command(&grant.command).is_err() {
        issues.push(ScriptItemGrantIssue::InvalidCommand {
            command: grant.command.clone(),
        });
    }
    if !is_exact_script_item_token(&grant.item_id) {
        issues.push(ScriptItemGrantIssue::InvalidItem {
            item_id: grant.item_id.clone(),
        });
    } else if grant.item_id != SCRIPT_ITEM_FROM_MEMORY_ID
        && !item_catalog.contains_key(&grant.item_id)
    {
        issues.push(ScriptItemGrantIssue::UnknownItem {
            item_id: grant.item_id.clone(),
        });
    }
    issues
}

pub fn script_item_access_issues(
    access: &ScriptItemAccess,
    item_catalog: &BTreeMap<String, Item>,
) -> Vec<ScriptItemAccessIssue> {
    if validate_script_item_access_command(&access.command).is_err() {
        vec![ScriptItemAccessIssue::InvalidCommand {
            command: access.command.clone(),
        }]
    } else if !is_exact_script_item_token(&access.item_id) {
        vec![ScriptItemAccessIssue::InvalidItem {
            item_id: access.item_id.clone(),
        }]
    } else if item_catalog.contains_key(&access.item_id) {
        Vec::new()
    } else {
        vec![ScriptItemAccessIssue::UnknownItem {
            item_id: access.item_id.clone(),
        }]
    }
}

fn is_exact_script_item_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn validate_exact_script_item_command(command: &str) -> Result<(), String> {
    if command.is_empty()
        || command.trim() != command
        || !command.bytes().all(|byte| byte.is_ascii_lowercase())
        || has_reserved_pack_prefix(command)
    {
        Err(format!(
            "script item command must be exact lowercase ASCII, found {command:?}"
        ))
    } else {
        Ok(())
    }
}

fn is_exact_script_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !has_reserved_pack_prefix(value)
}

fn required_script_item_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_item_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script item token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_script_item_grant_command<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validate_script_item_grant_command(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_script_item_access_command<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validate_script_item_access_command(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_script_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_label_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script label token must be exact visible ASCII, found {value:?}"
        )))
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

pub fn grant_script_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    grant: ScriptItemGrant,
) -> Result<ScriptItemGrantOutcome, ScriptItemGrantError> {
    validate_script_item_grant_for_runtime(&grant)?;
    if grant.quantity == 0 {
        return Err(ScriptItemGrantError::InvalidQuantity);
    }
    let item =
        item_catalog
            .get(&grant.item_id)
            .ok_or_else(|| ScriptItemGrantError::UnknownItem {
                item_id: grant.item_id.clone(),
            })?;
    let added = state
        .bag
        .add_item(item, grant.quantity)
        .map_err(|error| ScriptItemGrantError::Bag { error })?;

    if !added {
        return Ok(ScriptItemGrantOutcome::BagFull {
            item_id: grant.item_id,
            quantity: grant.quantity,
            source_script: grant.source_script,
            command_index: grant.command_index,
            verbose: grant.verbose,
        });
    }

    Ok(ScriptItemGrantOutcome::Granted {
        item_id: grant.item_id,
        quantity: grant.quantity,
        source_script: grant.source_script,
        command_index: grant.command_index,
        verbose: grant.verbose,
    })
}

pub fn check_script_item(
    state: &GameState,
    item_catalog: &BTreeMap<String, Item>,
    access: ScriptItemAccess,
) -> Result<ScriptItemCheckOutcome, ScriptItemAccessError> {
    validate_script_item_access_for_runtime(&access)?;
    let item =
        item_catalog
            .get(&access.item_id)
            .ok_or_else(|| ScriptItemAccessError::UnknownItem {
                item_id: access.item_id.clone(),
            })?;
    Ok(ScriptItemCheckOutcome {
        item_id: access.item_id,
        source_script: access.source_script,
        command_index: access.command_index,
        held: state.bag.has_item(item),
    })
}

pub fn take_script_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    access: ScriptItemAccess,
) -> Result<ScriptItemTakeOutcome, ScriptItemAccessError> {
    validate_script_item_access_for_runtime(&access)?;
    let item =
        item_catalog
            .get(&access.item_id)
            .ok_or_else(|| ScriptItemAccessError::UnknownItem {
                item_id: access.item_id.clone(),
            })?;
    let removed = state
        .bag
        .remove_item(item, 1)
        .map_err(|error| ScriptItemAccessError::Bag { error })?;
    Ok(ScriptItemTakeOutcome {
        item_id: access.item_id,
        source_script: access.source_script,
        command_index: access.command_index,
        removed,
    })
}

fn validate_script_item_grant_for_runtime(
    grant: &ScriptItemGrant,
) -> Result<(), ScriptItemGrantError> {
    if validate_script_item_grant_command(&grant.command).is_err() {
        return Err(ScriptItemGrantError::InvalidCommand {
            command: grant.command.clone(),
        });
    }
    if !is_exact_script_item_token(&grant.item_id) {
        return Err(ScriptItemGrantError::InvalidItem {
            item_id: grant.item_id.clone(),
        });
    }
    if !is_exact_script_label_token(&grant.source_script) {
        return Err(ScriptItemGrantError::InvalidSourceScript {
            source_script: grant.source_script.clone(),
        });
    }
    Ok(())
}

fn validate_script_item_access_for_runtime(
    access: &ScriptItemAccess,
) -> Result<(), ScriptItemAccessError> {
    if validate_script_item_access_command(&access.command).is_err() {
        return Err(ScriptItemAccessError::InvalidCommand {
            command: access.command.clone(),
        });
    }
    if !is_exact_script_item_token(&access.item_id) {
        return Err(ScriptItemAccessError::InvalidItem {
            item_id: access.item_id.clone(),
        });
    }
    if !is_exact_script_label_token(&access.source_script) {
        return Err(ScriptItemAccessError::InvalidSourceScript {
            source_script: access.source_script.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ItemPocket, MAX_ITEM_STACK, item_pocket};

    fn item(id: &str, pocket: ItemPocket) -> Item {
        Item {
            name: id.replace('_', " "),
            description: String::new(),
            effect: "NONE".to_string(),
            status_heals: Vec::new(),
            revive_hp_percent: None,
            party_revive_hp_percent: None,
            pp_restore_scope: None,
            pp_restore_points: None,
            pp_up_stages: None,
            vitamin_stat: None,
            vitamin_stat_exp: None,
            vitamin_max_stat_exp: None,
            rare_candy_level_gain: None,
            battle_stat_boost_stat: None,
            battle_stat_boost_stages: None,
            battle_escape_mode: None,
            battle_capture_ball: None,
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
            confusion_heal: None,
            repel_steps: None,
            escape_rope_mode: None,
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket,
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn catalog(items: Vec<Item>) -> BTreeMap<String, Item> {
        items
            .into_iter()
            .map(|item| (item.script_name.clone(), item))
            .collect()
    }

    fn grant(item_id: &str, quantity: u16) -> ScriptItemGrant {
        ScriptItemGrant {
            command: "giveitem".to_string(),
            item_id: item_id.to_string(),
            quantity,
            source_script: "GiftScript".to_string(),
            command_index: 3,
            verbose: false,
        }
    }

    fn access(item_id: &str) -> ScriptItemAccess {
        ScriptItemAccess {
            command: "checkitem".to_string(),
            item_id: item_id.to_string(),
            source_script: "GateScript".to_string(),
            command_index: 7,
        }
    }

    #[test]
    fn exported_script_item_command_sets_are_exact() {
        assert!(SCRIPT_ITEM_GRANT_COMMANDS.contains(&"giveitem"));
        assert!(SCRIPT_ITEM_GRANT_COMMANDS.contains(&"verbosegiveitem"));
        assert!(SCRIPT_ITEM_CHECK_COMMANDS.contains(&"checkitem"));
        assert!(SCRIPT_ITEM_TAKE_COMMANDS.contains(&"takeitem"));
        assert!(is_known_script_item_grant_command("giveitem"));
        assert!(is_known_script_item_grant_command("verbosegiveitem"));
        assert!(is_known_script_item_check_command("checkitem"));
        assert!(is_known_script_item_take_command("takeitem"));
        assert!(is_known_script_item_access_command("checkitem"));
        assert!(is_known_script_item_access_command("takeitem"));
        assert!(validate_script_item_grant_command("giveitem").is_ok());
        assert!(validate_script_item_access_command("takeitem").is_ok());
        assert!(validate_script_item_grant_command("GiveItem").is_err());
        assert!(validate_script_item_access_command("fallback_takeitem").is_err());
        assert!(validate_script_item_access_command("giveitem").is_err());
    }

    #[test]
    fn grants_exact_script_item_id() {
        let mut state = GameState::default();
        let items = catalog(vec![item("POTION", item_pocket("ITEM"))]);

        let outcome =
            grant_script_item(&mut state, &items, grant("POTION", 1)).expect("grant item");

        assert_eq!(
            outcome,
            ScriptItemGrantOutcome::Granted {
                item_id: "POTION".to_string(),
                quantity: 1,
                source_script: "GiftScript".to_string(),
                command_index: 3,
                verbose: false,
            }
        );
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);
    }

    #[test]
    fn rejects_case_changed_item_id_without_mutating_bag() {
        let mut state = GameState::default();
        let items = catalog(vec![item("TM_MUD_SLAP", item_pocket("TM_HM"))]);

        let error = grant_script_item(&mut state, &items, grant("tm_mud_slap", 1))
            .expect_err("case changed id is unknown");

        assert_eq!(
            error,
            ScriptItemGrantError::UnknownItem {
                item_id: "tm_mud_slap".to_string(),
            }
        );
        assert_eq!(state.bag.quantity(&items["TM_MUD_SLAP"]), 0);
    }

    #[test]
    fn rejects_zero_quantity_without_mutating_bag() {
        let mut state = GameState::default();
        let items = catalog(vec![item("POTION", item_pocket("ITEM"))]);

        let error = grant_script_item(&mut state, &items, grant("POTION", 0))
            .expect_err("zero quantity is invalid");

        assert_eq!(error, ScriptItemGrantError::InvalidQuantity);
        assert_eq!(state.bag.quantity(&items["POTION"]), 0);
    }

    #[test]
    fn runtime_item_commands_reject_invalid_shape_before_bag_mutation() {
        let mut state = GameState::default();
        let items = catalog(vec![item("POTION", item_pocket("ITEM"))]);
        state.bag.add_item(&items["POTION"], 1).expect("seed item");

        let mut invalid_grant = grant("POTION", 1);
        invalid_grant.source_script = "legacy_script".to_string();
        assert_eq!(
            grant_script_item(&mut state, &items, invalid_grant),
            Err(ScriptItemGrantError::InvalidSourceScript {
                source_script: "legacy_script".to_string(),
            })
        );
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);

        let mut invalid_grant = grant("POTION", 1);
        invalid_grant.command = "fallbackgive".to_string();
        assert_eq!(
            grant_script_item(&mut state, &items, invalid_grant),
            Err(ScriptItemGrantError::InvalidCommand {
                command: "fallbackgive".to_string(),
            })
        );
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);

        let mut invalid_access = access("POTION");
        invalid_access.command = "takeitem".to_string();
        invalid_access.source_script = "fallback_script".to_string();
        assert_eq!(
            take_script_item(&mut state, &items, invalid_access),
            Err(ScriptItemAccessError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);
    }

    #[test]
    fn grant_issues_allow_memory_item_sentinel_but_reject_unknown_exact_ids() {
        let items = catalog(vec![item("POTION", item_pocket("ITEM"))]);

        assert_eq!(script_item_grant_issues(&grant("POTION", 1), &items), []);
        assert_eq!(
            script_item_grant_issues(&grant(SCRIPT_ITEM_FROM_MEMORY_ID, 1), &items),
            []
        );
        assert_eq!(
            script_item_grant_issues(&grant("potion", 0), &items),
            [
                ScriptItemGrantIssue::InvalidQuantity,
                ScriptItemGrantIssue::UnknownItem {
                    item_id: "potion".to_string()
                },
            ]
        );
        assert_eq!(
            script_item_grant_issues(&grant(" POTION", 1), &items),
            [ScriptItemGrantIssue::InvalidItem {
                item_id: " POTION".to_string()
            }]
        );
        assert_eq!(
            script_item_grant_issues(&grant("PO TION", 1), &items),
            [ScriptItemGrantIssue::InvalidItem {
                item_id: "PO TION".to_string()
            }]
        );
        assert_eq!(
            script_item_grant_issues(&grant(" ITEM_FROM_MEM", 1), &items),
            [ScriptItemGrantIssue::InvalidItem {
                item_id: " ITEM_FROM_MEM".to_string()
            }]
        );
        assert_eq!(
            script_item_grant_issues(&grant("ITEM FROM_MEM", 1), &items),
            [ScriptItemGrantIssue::InvalidItem {
                item_id: "ITEM FROM_MEM".to_string()
            }]
        );
    }

    #[test]
    fn script_item_grants_reject_reserved_pack_prefixes() {
        let items = catalog(vec![item("POTION", item_pocket("ITEM"))]);

        assert_eq!(
            script_item_grant_issues(&grant("fallback_potion", 1), &items),
            [ScriptItemGrantIssue::InvalidItem {
                item_id: "fallback_potion".to_string()
            }]
        );
        assert_eq!(
            script_item_access_issues(&access("legacy_pass"), &items),
            [ScriptItemAccessIssue::InvalidItem {
                item_id: "legacy_pass".to_string()
            }]
        );

        for (field, value) in [
            ("item_id", serde_json::json!("fallback_potion")),
            ("source_script", serde_json::json!("legacy_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "giveitem",
                "item_id": "POTION",
                "quantity": 1,
                "source_script": ".branch@GiftScript",
                "command_index": 3,
                "verbose": false
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptItemGrant>(payload)
                .expect_err("reserved script item grant tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script item") || error.contains("script label"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("item_id", serde_json::json!("legacy_pass")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "checkitem",
                "item_id": "PASS",
                "source_script": ".branch@GateScript",
                "command_index": 7
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptItemAccess>(payload)
                .expect_err("reserved script item access tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script item") || error.contains("script label"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn script_item_serialized_variants_reject_unknown_fallback_fields() {
        let grant_outcome_error =
            serde_json::from_value::<ScriptItemGrantOutcome>(serde_json::json!({
                "granted": {
                    "item_id": "POTION",
                    "quantity": 1,
                    "source_script": "GiftScript",
                    "command_index": 3,
                    "verbose": true,
                    "fallback_item_id": "BERRY"
                }
            }))
            .expect_err("grant outcomes must not accept fallback fields");
        assert!(
            grant_outcome_error
                .to_string()
                .contains("unknown field `fallback_item_id`"),
            "{grant_outcome_error}"
        );

        let access_error = serde_json::from_value::<ScriptItemAccessError>(serde_json::json!({
            "UnknownItem": {
                "item_id": "PASS",
                "legacy_item_id": "S_S_TICKET"
            }
        }))
        .expect_err("access errors must not accept legacy fields");
        assert!(
            access_error
                .to_string()
                .contains("unknown field `legacy_item_id`"),
            "{access_error}"
        );

        let issue_error = serde_json::from_value::<ScriptItemGrantIssue>(serde_json::json!({
            "unknown_item": {
                "item_id": "POTION",
                "normalized_item_id": "POTION"
            }
        }))
        .expect_err("grant issues must not accept normalized aliases");
        assert!(
            issue_error
                .to_string()
                .contains("unknown field `normalized_item_id`"),
            "{issue_error}"
        );
    }

    #[test]
    fn access_issues_reject_unknown_exact_ids_without_memory_sentinel() {
        let items = catalog(vec![item("PASS", item_pocket("KEY_ITEM"))]);

        assert_eq!(script_item_access_issues(&access("PASS"), &items), []);
        assert_eq!(
            script_item_access_issues(&access(SCRIPT_ITEM_FROM_MEMORY_ID), &items),
            [ScriptItemAccessIssue::UnknownItem {
                item_id: SCRIPT_ITEM_FROM_MEMORY_ID.to_string()
            }]
        );
        assert_eq!(
            script_item_access_issues(&access("pass"), &items),
            [ScriptItemAccessIssue::UnknownItem {
                item_id: "pass".to_string()
            }]
        );
        assert_eq!(
            script_item_access_issues(&access(" PASS"), &items),
            [ScriptItemAccessIssue::InvalidItem {
                item_id: " PASS".to_string()
            }]
        );
        assert_eq!(
            script_item_access_issues(&access("PA SS"), &items),
            [ScriptItemAccessIssue::InvalidItem {
                item_id: "PA SS".to_string()
            }]
        );
    }

    #[test]
    fn reports_bag_full_without_overfilling_stack() {
        let mut state = GameState::default();
        let items = catalog(vec![item("POTION", item_pocket("ITEM"))]);
        state.bag.items.insert("POTION".to_string(), MAX_ITEM_STACK);

        let outcome =
            grant_script_item(&mut state, &items, grant("POTION", 1)).expect("bag full outcome");

        assert_eq!(
            outcome,
            ScriptItemGrantOutcome::BagFull {
                item_id: "POTION".to_string(),
                quantity: 1,
                source_script: "GiftScript".to_string(),
                command_index: 3,
                verbose: false,
            }
        );
        assert_eq!(state.bag.items["POTION"], MAX_ITEM_STACK);
    }

    #[test]
    fn grants_symbolic_tm_when_pack_declares_explicit_tmhm_index() {
        let mut tm = item("TM_MUD_SLAP", item_pocket("TM_HM"));
        tm.tmhm_index = Some(30);
        let items = catalog(vec![tm]);
        let mut state = GameState::default();

        let outcome = grant_script_item(&mut state, &items, grant("TM_MUD_SLAP", 1))
            .expect("grant symbolic tm");

        assert_eq!(
            outcome,
            ScriptItemGrantOutcome::Granted {
                item_id: "TM_MUD_SLAP".to_string(),
                quantity: 1,
                source_script: "GiftScript".to_string(),
                command_index: 3,
                verbose: false,
            }
        );
        assert_eq!(state.bag.quantity(&items["TM_MUD_SLAP"]), 1);
    }

    #[test]
    fn checks_exact_item_without_case_coercion() {
        let mut state = GameState::default();
        let items = catalog(vec![item("PASS", item_pocket("KEY_ITEM"))]);
        state.bag.add_item(&items["PASS"], 1).expect("add pass");

        let held = check_script_item(&state, &items, access("PASS")).expect("check exact item");
        let missing = check_script_item(&state, &items, access("pass"))
            .expect_err("case changed item id rejected");

        assert_eq!(
            held,
            ScriptItemCheckOutcome {
                item_id: "PASS".to_string(),
                source_script: "GateScript".to_string(),
                command_index: 7,
                held: true,
            }
        );
        assert_eq!(
            missing,
            ScriptItemAccessError::UnknownItem {
                item_id: "pass".to_string()
            }
        );
    }

    #[test]
    fn takes_one_exact_item_without_removing_unknown_or_missing_items() {
        let mut state = GameState::default();
        let items = catalog(vec![item("BERRY", item_pocket("ITEM"))]);
        state.bag.add_item(&items["BERRY"], 2).expect("add berries");

        let first = take_script_item(&mut state, &items, access("BERRY")).expect("take berry");
        let second = take_script_item(&mut state, &items, access("BERRY")).expect("take berry");
        let third = take_script_item(&mut state, &items, access("BERRY")).expect("missing berry");

        assert!(first.removed);
        assert!(second.removed);
        assert!(!third.removed);
        assert_eq!(state.bag.quantity(&items["BERRY"]), 0);
    }

    #[test]
    fn takes_symbolic_tm_only_when_pack_declares_tmhm_index() {
        let mut tm = item("TM_MUD_SLAP", item_pocket("TM_HM"));
        tm.tmhm_index = Some(30);
        let items = catalog(vec![tm]);
        let mut state = GameState::default();
        state
            .bag
            .add_item(&items["TM_MUD_SLAP"], 1)
            .expect("add tm");

        let outcome =
            take_script_item(&mut state, &items, access("TM_MUD_SLAP")).expect("take symbolic tm");

        assert!(outcome.removed);
        assert_eq!(state.bag.quantity(&items["TM_MUD_SLAP"]), 0);
    }
}
