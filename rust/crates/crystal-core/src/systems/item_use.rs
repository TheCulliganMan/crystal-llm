use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::Item;
use crate::state::{GameState, ItemUseRuntimeEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemUseContext {
    Field,
    Battle,
}

impl ItemUseContext {
    fn as_str(self) -> &'static str {
        match self {
            Self::Field => "field",
            Self::Battle => "battle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemUseRequest {
    pub item_id: String,
    pub context: ItemUseContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemUseOutcome {
    pub item_id: String,
    pub effect: String,
    pub held_effect: String,
    pub parameter: i16,
    pub property: String,
    pub field_menu: String,
    pub battle_menu: String,
    pub context: ItemUseContext,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemUseError {
    UnknownItem {
        item_id: String,
    },
    ItemNotHeld {
        item_id: String,
    },
    UnusableInContext {
        item_id: String,
        context: ItemUseContext,
    },
    Bag {
        error: String,
    },
}

pub fn use_bag_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    request: ItemUseRequest,
) -> Result<ItemUseOutcome, ItemUseError> {
    let item = item_catalog
        .get(&request.item_id)
        .ok_or_else(|| ItemUseError::UnknownItem {
            item_id: request.item_id.clone(),
        })?;
    if !state.bag.has_item(item) {
        return Err(ItemUseError::ItemNotHeld {
            item_id: request.item_id,
        });
    }
    let menu = match request.context {
        ItemUseContext::Field => &item.field_menu,
        ItemUseContext::Battle => &item.battle_menu,
    };
    if menu == "ITEMMENU_NOUSE" {
        return Err(ItemUseError::UnusableInContext {
            item_id: request.item_id,
            context: request.context,
        });
    }

    let consumed = if item.consumable {
        state
            .bag
            .remove_item(item, 1)
            .map_err(|error| ItemUseError::Bag { error })?
    } else {
        false
    };
    let outcome = ItemUseOutcome {
        item_id: request.item_id,
        effect: item.effect.clone(),
        held_effect: item.held_effect.clone(),
        parameter: item.parameter,
        property: item.property.clone(),
        field_menu: item.field_menu.clone(),
        battle_menu: item.battle_menu.clone(),
        context: request.context,
        consumed,
    };
    state
        .script_runtime
        .item_use_events
        .push(ItemUseRuntimeEvent {
            item_id: outcome.item_id.clone(),
            effect: outcome.effect.clone(),
            held_effect: outcome.held_effect.clone(),
            parameter: outcome.parameter,
            property: outcome.property.clone(),
            field_menu: outcome.field_menu.clone(),
            battle_menu: outcome.battle_menu.clone(),
            context: outcome.context.as_str().to_string(),
            consumed: outcome.consumed,
        });
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ItemPocket;

    fn item(
        id: &str,
        pocket: ItemPocket,
        field_menu: &str,
        battle_menu: &str,
        consumable: bool,
    ) -> Item {
        Item {
            name: id.replace('_', " "),
            description: String::new(),
            effect: format!("EFFECT_{id}"),
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 7,
            property: "NO_LIMITS".to_string(),
            pocket,
            field_menu: field_menu.to_string(),
            battle_menu: battle_menu.to_string(),
            script_name: id.to_string(),
            consumable,
            tmhm_index: None,
        }
    }

    fn catalog(items: Vec<Item>) -> BTreeMap<String, Item> {
        items
            .into_iter()
            .map(|item| (item.script_name.clone(), item))
            .collect()
    }

    #[test]
    fn uses_exact_modpack_item_effect_and_consumes_declared_consumables() {
        let items = catalog(vec![item(
            "POTION",
            ItemPocket::Item,
            "ITEMMENU_PARTY",
            "ITEMMENU_PARTY",
            true,
        )]);
        let mut state = GameState::default();
        state.bag.add_item(&items["POTION"], 2).expect("add item");

        let outcome = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "POTION".to_string(),
                context: ItemUseContext::Battle,
            },
        )
        .expect("use item");

        assert_eq!(outcome.effect, "EFFECT_POTION");
        assert!(outcome.consumed);
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);
        assert_eq!(state.script_runtime.item_use_events.len(), 1);
        assert_eq!(state.script_runtime.item_use_events[0].item_id, "POTION");
        assert_eq!(state.script_runtime.item_use_events[0].context, "battle");
    }

    #[test]
    fn exact_key_items_record_effect_without_consumption() {
        let items = catalog(vec![item(
            "ITEMFINDER",
            ItemPocket::KeyItem,
            "ITEMMENU_CLOSE",
            "ITEMMENU_NOUSE",
            false,
        )]);
        let mut state = GameState::default();
        state
            .bag
            .add_item(&items["ITEMFINDER"], 1)
            .expect("add key item");

        let outcome = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "ITEMFINDER".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect("use key item");

        assert!(!outcome.consumed);
        assert_eq!(state.bag.quantity(&items["ITEMFINDER"]), 1);
        assert_eq!(
            state.script_runtime.item_use_events[0].effect,
            "EFFECT_ITEMFINDER"
        );
    }

    #[test]
    fn rejects_unknown_case_changed_and_context_unusable_items() {
        let items = catalog(vec![item(
            "POTION",
            ItemPocket::Item,
            "ITEMMENU_PARTY",
            "ITEMMENU_NOUSE",
            true,
        )]);
        let mut state = GameState::default();
        state.bag.add_item(&items["POTION"], 1).expect("add item");

        let unknown = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "potion".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect_err("case changed id is unknown");
        assert_eq!(
            unknown,
            ItemUseError::UnknownItem {
                item_id: "potion".to_string(),
            }
        );

        let unusable = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "POTION".to_string(),
                context: ItemUseContext::Battle,
            },
        )
        .expect_err("battle menu forbids this item");
        assert_eq!(
            unusable,
            ItemUseError::UnusableInContext {
                item_id: "POTION".to_string(),
                context: ItemUseContext::Battle,
            }
        );
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);
    }

    #[test]
    fn not_held_items_do_not_record_or_consume() {
        let items = catalog(vec![item(
            "POTION",
            ItemPocket::Item,
            "ITEMMENU_PARTY",
            "ITEMMENU_PARTY",
            true,
        )]);
        let mut state = GameState::default();

        let error = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "POTION".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect_err("item not held");

        assert_eq!(
            error,
            ItemUseError::ItemNotHeld {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(state.script_runtime.item_use_events.len(), 0);
    }

    #[test]
    fn non_consumable_items_are_pack_declared_not_pocket_inferred() {
        let items = catalog(vec![item(
            "MODDED_CHARM",
            ItemPocket::Item,
            "ITEMMENU_CLOSE",
            "ITEMMENU_NOUSE",
            false,
        )]);
        let mut state = GameState::default();
        state
            .bag
            .add_item(&items["MODDED_CHARM"], 1)
            .expect("add item");

        let outcome = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "MODDED_CHARM".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect("use non-consumable item");

        assert!(!outcome.consumed);
        assert_eq!(state.bag.quantity(&items["MODDED_CHARM"]), 1);
    }
}
