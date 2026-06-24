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
    use crate::models::{ItemPocket, MAX_ITEM_STACK};

    fn item(id: &str, pocket: ItemPocket) -> Item {
        Item {
            name: id.replace('_', " "),
            description: String::new(),
            effect: "NONE".to_string(),
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket,
            field_menu: String::new(),
            battle_menu: String::new(),
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
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
        let items = catalog(vec![item("POTION", ItemPocket::Item)]);

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
        let items = catalog(vec![item("TM_MUD_SLAP", ItemPocket::TmHm)]);

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
        let items = catalog(vec![item("POTION", ItemPocket::Item)]);

        let error = grant_script_item(&mut state, &items, grant("POTION", 0))
            .expect_err("zero quantity is invalid");

        assert_eq!(error, ScriptItemGrantError::InvalidQuantity);
        assert_eq!(state.bag.quantity(&items["POTION"]), 0);
    }

    #[test]
    fn reports_bag_full_without_overfilling_stack() {
        let mut state = GameState::default();
        let items = catalog(vec![item("POTION", ItemPocket::Item)]);
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
        let mut tm = item("TM_MUD_SLAP", ItemPocket::TmHm);
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
        let items = catalog(vec![item("PASS", ItemPocket::KeyItem)]);
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
        let items = catalog(vec![item("BERRY", ItemPocket::Item)]);
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
        let mut tm = item("TM_MUD_SLAP", ItemPocket::TmHm);
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
