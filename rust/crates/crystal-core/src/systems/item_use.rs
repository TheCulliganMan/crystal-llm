use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::Item;
use crate::state::{GameState, ItemUseRuntimeEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
    #[serde(deserialize_with = "required_item_use_id")]
    pub item_id: String,
    pub context: ItemUseContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemUseOutcome {
    pub item_id: String,
    pub context: ItemUseContext,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ItemUseError {
    InvalidItemId {
        item_id: String,
    },
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
    validate_item_use_id(&request.item_id)?;
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
    let usable = match request.context {
        ItemUseContext::Field => item.field_usable,
        ItemUseContext::Battle => item.battle_usable,
    };
    if !usable {
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
        context: request.context,
        consumed,
    };
    state
        .script_runtime
        .item_use_events
        .push(ItemUseRuntimeEvent {
            item_id: outcome.item_id.clone(),
            context: outcome.context.as_str().to_string(),
            consumed: outcome.consumed,
        });
    Ok(outcome)
}

fn validate_item_use_id(item_id: &str) -> Result<(), ItemUseError> {
    if item_id.is_empty()
        || item_id.trim() != item_id
        || !item_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || has_reserved_pack_prefix(item_id)
    {
        return Err(ItemUseError::InvalidItemId {
            item_id: item_id.to_string(),
        });
    }
    Ok(())
}

fn required_item_use_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty()
        || value.trim() != value
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || has_reserved_pack_prefix(&value)
    {
        Err(serde::de::Error::custom(format!(
            "item use id must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    } else {
        Ok(value)
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ItemPocket, item_pocket};

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
            parameter: 7,
            property: "NO_LIMITS".to_string(),
            pocket,
            field_menu: field_menu.to_string(),
            field_usable: field_menu != "ITEMMENU_NOUSE",
            battle_menu: battle_menu.to_string(),
            battle_usable: battle_menu != "ITEMMENU_NOUSE",
            script_name: id.to_string(),
            consumable,
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

    #[test]
    fn uses_exact_modpack_item_id_and_consumes_declared_consumables() {
        let items = catalog(vec![item(
            "POTION",
            item_pocket("ITEM"),
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

        assert_eq!(outcome.item_id, "POTION");
        assert!(outcome.consumed);
        assert_eq!(state.bag.quantity(&items["POTION"]), 1);
        assert_eq!(state.script_runtime.item_use_events.len(), 1);
        assert_eq!(state.script_runtime.item_use_events[0].item_id, "POTION");
        assert_eq!(state.script_runtime.item_use_events[0].context, "battle");
    }

    #[test]
    fn exact_key_items_record_item_id_without_consumption() {
        let items = catalog(vec![item(
            "ITEMFINDER",
            item_pocket("KEY_ITEM"),
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
            state.script_runtime.item_use_events[0].item_id,
            "ITEMFINDER"
        );
        assert_eq!(state.script_runtime.item_use_events[0].context, "field");
    }

    #[test]
    fn rejects_unknown_case_changed_and_context_unusable_items() {
        let items = catalog(vec![item(
            "POTION",
            item_pocket("ITEM"),
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
    fn rejects_malformed_item_use_ids_before_unknown_lookup() {
        let items = catalog(vec![item(
            "POKE_BALL",
            item_pocket("BALL"),
            "ITEMMENU_CLOSE",
            "ITEMMENU_NOUSE",
            true,
        )]);
        let mut state = GameState::default();
        state
            .bag
            .add_item(&items["POKE_BALL"], 1)
            .expect("add item");

        let spaced = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "POKE BALL".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect_err("space-separated item ids are invalid pack/runtime input");
        assert_eq!(
            spaced,
            ItemUseError::InvalidItemId {
                item_id: "POKE BALL".to_string(),
            }
        );

        let padded = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "POKE_BALL ".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect_err("padded item ids are invalid pack/runtime input");
        assert_eq!(
            padded,
            ItemUseError::InvalidItemId {
                item_id: "POKE_BALL ".to_string(),
            }
        );
        assert_eq!(state.bag.quantity(&items["POKE_BALL"]), 1);
        assert!(state.script_runtime.item_use_events.is_empty());
    }

    #[test]
    fn rejects_reserved_item_use_ids_before_unknown_lookup() {
        let items = catalog(vec![item(
            "POTION",
            item_pocket("ITEM"),
            "ITEMMENU_PARTY",
            "ITEMMENU_PARTY",
            true,
        )]);
        let mut state = GameState::default();

        let error = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "fallback_potion".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect_err("reserved item ids are invalid runtime input");
        assert_eq!(
            error,
            ItemUseError::InvalidItemId {
                item_id: "fallback_potion".to_string(),
            }
        );

        let serde_error = serde_json::from_value::<ItemUseRequest>(serde_json::json!({
            "item_id": "legacy_potion",
            "context": "field"
        }))
        .expect_err("reserved item use ids must fail during JSON load")
        .to_string();
        assert!(serde_error.contains("item use id must be"), "{serde_error}");
        assert!(state.script_runtime.item_use_events.is_empty());
    }

    #[test]
    fn accepts_modpack_item_menu_ids_without_core_whitelisting() {
        let items = catalog(vec![item(
            "MOD_MENU_ITEM",
            item_pocket("ITEM"),
            "ITEMMENU_MODDED",
            "ITEMMENU_NOUSE",
            true,
        )]);
        let mut state = GameState::default();
        state
            .bag
            .add_item(&items["MOD_MENU_ITEM"], 1)
            .expect("add item");

        let outcome = use_bag_item(
            &mut state,
            &items,
            ItemUseRequest {
                item_id: "MOD_MENU_ITEM".to_string(),
                context: ItemUseContext::Field,
            },
        )
        .expect("modpack menu ids are definitive data");

        assert_eq!(outcome.item_id, "MOD_MENU_ITEM");
        assert!(outcome.consumed);
        assert_eq!(state.bag.quantity(&items["MOD_MENU_ITEM"]), 0);
        assert_eq!(state.script_runtime.item_use_events.len(), 1);
        assert_eq!(
            state.script_runtime.item_use_events[0].item_id,
            "MOD_MENU_ITEM"
        );
    }

    #[test]
    fn not_held_items_do_not_record_or_consume() {
        let items = catalog(vec![item(
            "POTION",
            item_pocket("ITEM"),
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
            item_pocket("ITEM"),
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

    #[test]
    fn item_use_json_rejects_legacy_alias_payloads() {
        let context_error =
            serde_json::from_str::<ItemUseContext>(r#"{"field":{"legacy_context":"FIELD"}}"#)
                .expect_err("item-use contexts must not accept object-shaped aliases")
                .to_string();
        assert!(
            context_error.contains("invalid type")
                || context_error.contains("unknown field `legacy_context`"),
            "{context_error}"
        );

        let error_error = serde_json::from_value::<ItemUseError>(serde_json::json!({
            "UnusableInContext": {
                "item_id": "BICYCLE",
                "context": "battle",
                "fallback_context": "field"
            }
        }))
        .expect_err("item-use errors must not accept fallback contexts")
        .to_string();
        assert!(
            error_error.contains("unknown field `fallback_context`"),
            "{error_error}"
        );
    }
}
