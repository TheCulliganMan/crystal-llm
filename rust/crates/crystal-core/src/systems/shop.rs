use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::{
    BALL_POCKET_CAPACITY, ITEM_POCKET_CAPACITY, Item, ItemPocket, KEY_ITEM_POCKET_CAPACITY,
    MAX_ITEM_STACK,
};
use crate::state::{GameState, ScriptShopRequest, ScriptShopRuntimeEvent};

pub const MAX_MONEY: u32 = 999_999;
const PRICE_DIGITS: usize = 6;
pub const CANCEL_ITEM_ID: &str = "CANCEL";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopCommand {
    pub mart_type: String,
    pub mart_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MartCatalog(pub BTreeMap<String, Vec<String>>);

impl MartCatalog {
    pub fn inventory_ids(&self, mart_id: &str) -> Result<&[String], ShopError> {
        self.0
            .get(mart_id)
            .map(Vec::as_slice)
            .ok_or_else(|| ShopError::UnknownMart {
                mart_id: mart_id.to_string(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MartItem {
    pub identifier: String,
    pub display_name: String,
    pub price: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShopResult {
    pub success: bool,
    pub message: String,
    pub credited: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopOutcome {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ShopError {
    #[error("mart '{mart_id}' was not loaded")]
    UnknownMart { mart_id: String },
    #[error("mart '{mart_id}' references missing item '{item_id}'")]
    UnknownMartItem { mart_id: String, item_id: String },
    #[error("item '{item_id}' was not loaded")]
    UnknownItem { item_id: String },
    #[error("quantity must be positive")]
    InvalidQuantity,
    #[error("shop quantity {quantity} exceeds runtime quantity limit")]
    QuantityTooLarge { quantity: u32 },
    #[error("unknown script mart type '{mart_type}'")]
    UnknownMartType { mart_type: String },
    #[error("mart type '{mart_type}' cannot use explicit mart id 0")]
    InvalidZeroMart { mart_type: String },
    #[error("{message}")]
    Bag { message: String },
}

pub fn apply_script_shop_command(
    state: &mut GameState,
    catalog: &MartCatalog,
    items: &BTreeMap<String, Item>,
    command: ScriptShopCommand,
) -> Result<ScriptShopOutcome, ShopError> {
    validate_script_mart_type(&command.mart_type)?;
    let inventory = if command.mart_id == "0" {
        validate_zero_mart(&command.mart_type)?;
        Vec::new()
    } else {
        load_inventory(catalog, items, &command.mart_id)?
            .into_iter()
            .map(|item| item.identifier)
            .collect()
    };
    state.script_runtime.pending_shop = Some(ScriptShopRequest {
        mart_type: command.mart_type.clone(),
        mart_id: command.mart_id.clone(),
        inventory: inventory.clone(),
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    });
    state
        .script_runtime
        .shop_events
        .push(ScriptShopRuntimeEvent {
            mart_type: command.mart_type.clone(),
            mart_id: command.mart_id.clone(),
            inventory: inventory.clone(),
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        });
    Ok(ScriptShopOutcome {
        mart_type: command.mart_type,
        mart_id: command.mart_id,
        inventory,
        source_script: command.source_script,
        command_index: command.command_index,
    })
}

pub fn format_price(value: u32) -> String {
    let clamped = value.min(MAX_MONEY);
    format!("¥{:0>width$}", clamped, width = PRICE_DIGITS)
}

fn validate_script_mart_type(mart_type: &str) -> Result<(), ShopError> {
    match mart_type {
        "MARTTYPE_STANDARD" | "MARTTYPE_PHARMACY" | "MARTTYPE_BITTER" | "MARTTYPE_BARGAIN"
        | "MARTTYPE_ROOFTOP" => Ok(()),
        other => Err(ShopError::UnknownMartType {
            mart_type: other.to_string(),
        }),
    }
}

fn validate_zero_mart(mart_type: &str) -> Result<(), ShopError> {
    match mart_type {
        "MARTTYPE_BARGAIN" | "MARTTYPE_ROOFTOP" => Ok(()),
        other => Err(ShopError::InvalidZeroMart {
            mart_type: other.to_string(),
        }),
    }
}

pub fn load_inventory(
    catalog: &MartCatalog,
    items: &BTreeMap<String, Item>,
    mart_id: &str,
) -> Result<Vec<MartItem>, ShopError> {
    let mut inventory = Vec::new();
    for item_id in catalog.inventory_ids(mart_id)? {
        let item = items
            .get(item_id)
            .ok_or_else(|| ShopError::UnknownMartItem {
                mart_id: mart_id.to_string(),
                item_id: item_id.clone(),
            })?;
        inventory.push(MartItem {
            identifier: item.script_name.clone(),
            display_name: item.name.clone(),
            price: u32::from(item.price),
        });
    }
    Ok(inventory)
}

pub fn build_buy_menu(
    catalog: &MartCatalog,
    items: &BTreeMap<String, Item>,
    mart_id: &str,
) -> Result<Vec<MartItem>, ShopError> {
    let mut inventory = load_inventory(catalog, items, mart_id)?;
    inventory.push(MartItem {
        identifier: CANCEL_ITEM_ID.to_string(),
        display_name: CANCEL_ITEM_ID.to_string(),
        price: 0,
    });
    Ok(inventory)
}

pub fn max_buy_quantity(state: &GameState, item: &Item) -> u16 {
    if item.price == 0 {
        return 0;
    }
    let owned = state.bag.quantity(item);
    if owned == 0 && pocket_is_full(state, item.pocket) {
        return 0;
    }
    let stack_limit = match item.pocket {
        ItemPocket::KeyItem | ItemPocket::TmHm => 1,
        ItemPocket::Item | ItemPocket::Ball => MAX_ITEM_STACK,
    };
    let capacity = stack_limit.saturating_sub(owned);
    let affordable = state.money / u32::from(item.price);
    capacity.min(affordable.min(u32::from(u16::MAX)) as u16)
}

pub fn buy_item(
    state: &mut GameState,
    items: &BTreeMap<String, Item>,
    item_id: &str,
    quantity: u16,
) -> Result<ShopResult, ShopError> {
    if quantity == 0 {
        return Err(ShopError::InvalidQuantity);
    }
    let item = items.get(item_id).ok_or_else(|| ShopError::UnknownItem {
        item_id: item_id.to_string(),
    })?;
    let total_cost = u32::from(item.price) * u32::from(quantity);
    if total_cost > state.money {
        return Ok(ShopResult {
            success: false,
            message: "You don't have enough money.".to_string(),
            credited: 0,
        });
    }
    let added = state
        .bag
        .add_item(item, quantity)
        .map_err(|message| ShopError::Bag { message })?;
    if !added {
        return Ok(ShopResult {
            success: false,
            message: "Your Pack is full.".to_string(),
            credited: 0,
        });
    }
    state.money -= total_cost;
    Ok(ShopResult {
        success: true,
        message: format_price(total_cost),
        credited: total_cost,
    })
}

pub fn sell_item(
    state: &mut GameState,
    items: &BTreeMap<String, Item>,
    item_id: &str,
    quantity: u16,
) -> Result<ShopResult, ShopError> {
    if quantity == 0 {
        return Err(ShopError::InvalidQuantity);
    }
    let item = items.get(item_id).ok_or_else(|| ShopError::UnknownItem {
        item_id: item_id.to_string(),
    })?;
    let sell_price = u32::from(item.price / 2);
    if sell_price == 0 {
        return Ok(ShopResult {
            success: false,
            message: "We can't offer anything for that item.".to_string(),
            credited: 0,
        });
    }
    if state.bag.quantity(item) < quantity {
        return Ok(ShopResult {
            success: false,
            message: "Looks like you don't have that many.".to_string(),
            credited: 0,
        });
    }
    let removed = state
        .bag
        .remove_item(item, quantity)
        .map_err(|message| ShopError::Bag { message })?;
    if !removed {
        return Ok(ShopResult {
            success: false,
            message: "Looks like you don't have that many.".to_string(),
            credited: 0,
        });
    }

    let payout = sell_price * u32::from(quantity);
    let starting_money = state.money;
    state.money = state.money.saturating_add(payout).min(MAX_MONEY);
    Ok(ShopResult {
        success: true,
        message: format_price(payout),
        credited: state.money - starting_money,
    })
}

pub fn paginate_selection(
    selection: usize,
    scroll: usize,
    total_items: usize,
    direction: SelectionDirection,
) -> (usize, usize) {
    const MART_MENU_PAGE_SIZE: usize = 4;
    if total_items == 0 {
        return (0, 0);
    }
    let selection = match direction {
        SelectionDirection::Up => selection.saturating_sub(1),
        SelectionDirection::Down => (selection + 1).min(total_items - 1),
    };
    let scroll = if selection < scroll {
        selection
    } else if selection >= scroll + MART_MENU_PAGE_SIZE {
        selection - MART_MENU_PAGE_SIZE + 1
    } else {
        scroll
    };
    (selection, scroll)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionDirection {
    Up,
    Down,
}

fn pocket_is_full(state: &GameState, pocket: ItemPocket) -> bool {
    match pocket {
        ItemPocket::Item => active_slots(&state.bag.items) >= ITEM_POCKET_CAPACITY,
        ItemPocket::Ball => active_slots(&state.bag.balls) >= BALL_POCKET_CAPACITY,
        ItemPocket::KeyItem => active_slots(&state.bag.key_items) >= KEY_ITEM_POCKET_CAPACITY,
        ItemPocket::TmHm => false,
    }
}

fn active_slots(items: &BTreeMap<String, u16>) -> usize {
    items.values().filter(|quantity| **quantity > 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, price: u16, pocket: ItemPocket) -> Item {
        Item {
            name: id.replace('_', " "),
            description: String::new(),
            effect: "NONE".to_string(),
            price,
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

    fn items() -> BTreeMap<String, Item> {
        [
            (
                "POKE_BALL".to_string(),
                item("POKE_BALL", 200, ItemPocket::Ball),
            ),
            ("POTION".to_string(), item("POTION", 300, ItemPocket::Item)),
            (
                "RARE_CANDY".to_string(),
                item("RARE_CANDY", 1000, ItemPocket::Item),
            ),
        ]
        .into_iter()
        .collect()
    }

    fn shop_command(mart_type: &str, mart_id: &str) -> ScriptShopCommand {
        ScriptShopCommand {
            mart_type: mart_type.to_string(),
            mart_id: mart_id.to_string(),
            source_script: "ShopScript".to_string(),
            command_index: 11,
        }
    }

    #[test]
    fn load_inventory_uses_exact_mart_and_item_ids_without_aliasing() {
        let catalog = MartCatalog(
            [(
                "MartCherrygroveDex".to_string(),
                vec!["POKE_BALL".to_string(), "POTION".to_string()],
            )]
            .into_iter()
            .collect(),
        );
        let items = items();

        let inventory = load_inventory(&catalog, &items, "MartCherrygroveDex").expect("inventory");
        assert_eq!(
            inventory
                .iter()
                .map(|item| item.identifier.as_str())
                .collect::<Vec<_>>(),
            vec!["POKE_BALL", "POTION"]
        );
        assert_eq!(
            load_inventory(&catalog, &items, "MART_CHERRYGROVE_DEX"),
            Err(ShopError::UnknownMart {
                mart_id: "MART_CHERRYGROVE_DEX".to_string(),
            })
        );
    }

    #[test]
    fn mart_references_to_missing_items_are_errors_not_zero_price_entries() {
        let catalog = MartCatalog(
            [("MartBroken".to_string(), vec!["potion".to_string()])]
                .into_iter()
                .collect(),
        );

        assert_eq!(
            load_inventory(&catalog, &items(), "MartBroken"),
            Err(ShopError::UnknownMartItem {
                mart_id: "MartBroken".to_string(),
                item_id: "potion".to_string(),
            })
        );
    }

    #[test]
    fn build_buy_menu_appends_cancel_command() {
        let catalog = MartCatalog(
            [("Mart".to_string(), vec!["POTION".to_string()])]
                .into_iter()
                .collect(),
        );

        let menu = build_buy_menu(&catalog, &items(), "Mart").expect("menu");

        assert_eq!(menu[1].identifier, CANCEL_ITEM_ID);
    }

    #[test]
    fn applies_script_shop_command_with_exact_mart_inventory() {
        let catalog = MartCatalog(
            [(
                "MartCherrygroveDex".to_string(),
                vec!["POKE_BALL".to_string(), "POTION".to_string()],
            )]
            .into_iter()
            .collect(),
        );
        let items = items();
        let mut state = GameState::default();

        let outcome = apply_script_shop_command(
            &mut state,
            &catalog,
            &items,
            shop_command("MARTTYPE_STANDARD", "MartCherrygroveDex"),
        )
        .expect("apply shop");

        assert_eq!(
            outcome,
            ScriptShopOutcome {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MartCherrygroveDex".to_string(),
                inventory: vec!["POKE_BALL".to_string(), "POTION".to_string()],
                source_script: "ShopScript".to_string(),
                command_index: 11,
            }
        );
        assert_eq!(
            state.script_runtime.pending_shop,
            Some(ScriptShopRequest {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MartCherrygroveDex".to_string(),
                inventory: vec!["POKE_BALL".to_string(), "POTION".to_string()],
                source_script: "ShopScript".to_string(),
                command_index: 11,
            })
        );
        assert_eq!(state.script_runtime.shop_events.len(), 1);
    }

    #[test]
    fn applies_zero_mart_only_for_exact_rooftop_or_bargain_types() {
        let catalog = MartCatalog::default();
        let items = items();
        let mut state = GameState::default();

        let rooftop = apply_script_shop_command(
            &mut state,
            &catalog,
            &items,
            shop_command("MARTTYPE_ROOFTOP", "0"),
        )
        .expect("rooftop zero mart");
        assert!(rooftop.inventory.is_empty());
        assert_eq!(
            state
                .script_runtime
                .pending_shop
                .as_ref()
                .map(|shop| shop.mart_id.as_str()),
            Some("0")
        );

        let error = apply_script_shop_command(
            &mut state,
            &catalog,
            &items,
            shop_command("MARTTYPE_STANDARD", "0"),
        )
        .expect_err("standard zero mart rejected");
        assert_eq!(
            error,
            ShopError::InvalidZeroMart {
                mart_type: "MARTTYPE_STANDARD".to_string(),
            }
        );
    }

    #[test]
    fn invalid_script_shop_command_does_not_mutate_runtime_state() {
        let catalog = MartCatalog(
            [("Mart".to_string(), vec!["POTION".to_string()])]
                .into_iter()
                .collect(),
        );
        let items = items();
        let mut state = GameState::default();

        assert_eq!(
            apply_script_shop_command(
                &mut state,
                &catalog,
                &items,
                shop_command("marttype_standard", "Mart"),
            ),
            Err(ShopError::UnknownMartType {
                mart_type: "marttype_standard".to_string(),
            })
        );
        assert_eq!(
            apply_script_shop_command(
                &mut state,
                &catalog,
                &items,
                shop_command("MARTTYPE_STANDARD", "mart"),
            ),
            Err(ShopError::UnknownMart {
                mart_id: "mart".to_string(),
            })
        );
        assert_eq!(state.script_runtime.pending_shop, None);
        assert!(state.script_runtime.shop_events.is_empty());
    }

    #[test]
    fn buying_items_spends_money_and_uses_bag_capacity() {
        let mut state = GameState {
            money: 1000,
            ..GameState::default()
        };
        let items = items();

        let result = buy_item(&mut state, &items, "POTION", 2).expect("buy");

        assert_eq!(
            result,
            ShopResult {
                success: true,
                message: "¥000600".to_string(),
                credited: 600,
            }
        );
        assert_eq!(state.money, 400);
        assert_eq!(state.bag.quantity(&items["POTION"]), 2);

        let denied = buy_item(&mut state, &items, "POTION", 2).expect("insufficient funds");
        assert!(!denied.success);
        assert_eq!(state.money, 400);
        assert_eq!(state.bag.quantity(&items["POTION"]), 2);
    }

    #[test]
    fn max_buy_quantity_respects_money_stack_and_full_pocket() {
        let mut state = GameState {
            money: 1000,
            ..GameState::default()
        };
        let potion = item("POTION", 10, ItemPocket::Item);
        for index in 0..ITEM_POCKET_CAPACITY {
            let filler = item(&format!("DUMMY_ITEM_{index}"), 1, ItemPocket::Item);
            state.bag.add_item(&filler, 1).expect("add filler");
        }
        assert_eq!(max_buy_quantity(&state, &potion), 0);

        let mut stocked = GameState {
            money: 1000,
            ..GameState::default()
        };
        stocked.bag.add_item(&potion, 98).expect("add potion");
        assert_eq!(max_buy_quantity(&stocked, &potion), 1);
    }

    #[test]
    fn selling_items_credits_half_price_and_caps_money() {
        let mut state = GameState {
            money: MAX_MONEY - 100,
            ..GameState::default()
        };
        let items = items();
        state
            .bag
            .add_item(&items["RARE_CANDY"], 1)
            .expect("add item");

        let result = sell_item(&mut state, &items, "RARE_CANDY", 1).expect("sell");

        assert_eq!(state.money, MAX_MONEY);
        assert_eq!(result.credited, 100);
        assert_eq!(result.message, "¥000500");
        assert_eq!(state.bag.quantity(&items["RARE_CANDY"]), 0);
    }

    #[test]
    fn selling_rejects_unowned_or_valueless_items_without_state_change() {
        let mut state = GameState {
            money: 500,
            ..GameState::default()
        };
        let mut items = items();
        items.insert("FREEBIE".to_string(), item("FREEBIE", 1, ItemPocket::Item));

        let unowned = sell_item(&mut state, &items, "POTION", 1).expect("sell unowned");
        assert!(!unowned.success);
        assert_eq!(state.money, 500);

        state
            .bag
            .add_item(&items["FREEBIE"], 1)
            .expect("add freebie");
        let valueless = sell_item(&mut state, &items, "FREEBIE", 1).expect("sell valueless");
        assert!(!valueless.success);
        assert_eq!(state.money, 500);
        assert_eq!(state.bag.quantity(&items["FREEBIE"]), 1);
    }

    #[test]
    fn pagination_matches_four_item_mart_window() {
        assert_eq!(
            paginate_selection(3, 0, 8, SelectionDirection::Down),
            (4, 1)
        );
        assert_eq!(paginate_selection(1, 2, 8, SelectionDirection::Up), (0, 0));
    }
}
