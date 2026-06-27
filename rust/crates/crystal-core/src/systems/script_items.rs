use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::Item;
use crate::state::GameState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemGrant {
    pub item_id: String,
    pub quantity: u16,
    pub source_script: String,
    pub command_index: usize,
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemAccess {
    pub item_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
pub enum ScriptItemGrantError {
    UnknownItem { item_id: String },
    InvalidQuantity,
    Bag { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptItemAccessError {
    UnknownItem { item_id: String },
    Bag { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptItemGrantIssue {
    InvalidItem { item_id: String },
    UnknownItem { item_id: String },
    InvalidQuantity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptItemAccessIssue {
    InvalidItem { item_id: String },
    UnknownItem { item_id: String },
}

pub const SCRIPT_ITEM_FROM_MEMORY_ID: &str = "ITEM_FROM_MEM";

pub fn script_item_grant_issues(
    grant: &ScriptItemGrant,
    item_catalog: &BTreeMap<String, Item>,
) -> Vec<ScriptItemGrantIssue> {
    let mut issues = Vec::new();
    if grant.quantity == 0 {
        issues.push(ScriptItemGrantIssue::InvalidQuantity);
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
    if !is_exact_script_item_token(&access.item_id) {
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
}

pub fn grant_script_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    grant: ScriptItemGrant,
) -> Result<ScriptItemGrantOutcome, ScriptItemGrantError> {
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
            item_id: item_id.to_string(),
            quantity,
            source_script: "GiftScript".to_string(),
            command_index: 3,
            verbose: true,
        }
    }

    fn access(item_id: &str) -> ScriptItemAccess {
        ScriptItemAccess {
            item_id: item_id.to_string(),
            source_script: "GateScript".to_string(),
            command_index: 7,
        }
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
                verbose: true,
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
                verbose: true,
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
                verbose: true,
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
