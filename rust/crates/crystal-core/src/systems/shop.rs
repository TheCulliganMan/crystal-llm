use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::{
    BALL_POCKET_CAPACITY, ITEM_POCKET_BALL, ITEM_POCKET_CAPACITY, ITEM_POCKET_ITEM,
    ITEM_POCKET_KEY_ITEM, ITEM_POCKET_TM_HM, Item, ItemPocket, KEY_ITEM_POCKET_CAPACITY,
    MAX_ITEM_STACK,
};
use crate::state::{GameState, ScriptShopRequest, ScriptShopRuntimeEvent};
use crate::systems::economy::CurrencyCatalog;

const PRICE_DIGITS: usize = 6;
pub const CANCEL_ITEM_ID: &str = "CANCEL";
pub const SCRIPT_SHOP_COMMANDS: &[&str] = &["pokemart"];
pub const SCRIPT_SHOP_STANDARD_MART_TYPES: &[&str] =
    &["MARTTYPE_STANDARD", "MARTTYPE_PHARMACY", "MARTTYPE_BITTER"];
pub const SCRIPT_SHOP_ZERO_MART_TYPES: &[&str] = &["MARTTYPE_BARGAIN", "MARTTYPE_ROOFTOP"];

pub fn is_known_script_shop_command(command: &str) -> bool {
    SCRIPT_SHOP_COMMANDS.contains(&command)
}

pub fn is_known_script_mart_type(mart_type: &str) -> bool {
    SCRIPT_SHOP_STANDARD_MART_TYPES.contains(&mart_type)
        || SCRIPT_SHOP_ZERO_MART_TYPES.contains(&mart_type)
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MartCatalogIssue {
    EmptyMartId { mart_id: String },
    InvalidMartId { mart_id: String },
    UnknownItem { mart_id: String, item_id: String },
}

pub fn mart_catalog_issues(
    catalog: &MartCatalog,
    items: &BTreeMap<String, Item>,
) -> Vec<MartCatalogIssue> {
    let mut issues = Vec::new();
    for (mart_id, item_ids) in &catalog.0 {
        if mart_id.trim().is_empty() {
            issues.push(MartCatalogIssue::EmptyMartId {
                mart_id: mart_id.clone(),
            });
        } else if mart_id.trim() != mart_id {
            issues.push(MartCatalogIssue::InvalidMartId {
                mart_id: mart_id.clone(),
            });
        }
        for item_id in item_ids {
            if !items.contains_key(item_id) {
                issues.push(MartCatalogIssue::UnknownItem {
                    mart_id: mart_id.clone(),
                    item_id: item_id.clone(),
                });
            }
        }
    }
    issues
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
    #[error("invalid script mart type '{mart_type}'")]
    InvalidMartType { mart_type: String },
    #[error("invalid mart id '{mart_id}'")]
    InvalidMartId { mart_id: String },
    #[error("mart type '{mart_type}' cannot use explicit mart id 0")]
    InvalidZeroMart { mart_type: String },
    #[error("shop money mutation requires currency constant '{constant}'")]
    MissingCurrencyLimit { constant: String },
    #[error("{message}")]
    Bag { message: String },
    #[error("unsupported item pocket '{pocket}'")]
    UnsupportedPocket { pocket: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptShopCommandIssue {
    pub source_script: String,
    pub command_index: usize,
    pub error: ShopError,
}

pub fn script_shop_command_issues(
    catalog: &MartCatalog,
    commands: &[ScriptShopCommand],
) -> Vec<ScriptShopCommandIssue> {
    commands
        .iter()
        .filter_map(
            |command| match validate_script_shop_command(catalog, command) {
                Err(
                    error @ (ShopError::InvalidMartType { .. }
                    | ShopError::UnknownMartType { .. }
                    | ShopError::InvalidMartId { .. }
                    | ShopError::InvalidZeroMart { .. }
                    | ShopError::UnknownMart { .. }),
                ) => Some(ScriptShopCommandIssue {
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                    error,
                }),
                _ => None,
            },
        )
        .collect()
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

pub fn validate_script_shop_command(
    catalog: &MartCatalog,
    command: &ScriptShopCommand,
) -> Result<(), ShopError> {
    validate_script_mart_type(&command.mart_type)?;
    if command.mart_id == "0" {
        validate_zero_mart(&command.mart_type)
    } else if command.mart_id.trim().is_empty() || command.mart_id.trim() != command.mart_id {
        Err(ShopError::InvalidMartId {
            mart_id: command.mart_id.clone(),
        })
    } else {
        catalog.inventory_ids(&command.mart_id).map(|_| ())
    }
}

pub fn format_price(value: u32) -> String {
    format!("¥{:0>width$}", value, width = PRICE_DIGITS)
}

fn validate_script_mart_type(mart_type: &str) -> Result<(), ShopError> {
    if mart_type.trim().is_empty() || mart_type.trim() != mart_type {
        Err(ShopError::InvalidMartType {
            mart_type: mart_type.to_string(),
        })
    } else if is_known_script_mart_type(mart_type) {
        Ok(())
    } else {
        Err(ShopError::UnknownMartType {
            mart_type: mart_type.to_string(),
        })
    }
}

fn validate_zero_mart(mart_type: &str) -> Result<(), ShopError> {
    if SCRIPT_SHOP_ZERO_MART_TYPES.contains(&mart_type) {
        Ok(())
    } else {
        Err(ShopError::InvalidZeroMart {
            mart_type: mart_type.to_string(),
        })
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
    if owned == 0 && pocket_is_full(state, &item.pocket) {
        return 0;
    }
    let stack_limit = match item.pocket.as_str() {
        ITEM_POCKET_KEY_ITEM | ITEM_POCKET_TM_HM => 1,
        ITEM_POCKET_ITEM | ITEM_POCKET_BALL => MAX_ITEM_STACK,
        _ => 0,
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
    currency_constants: &CurrencyCatalog,
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
    let max_money = shop_money_cap(currency_constants)?;
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
    state.money = state.money.saturating_add(payout).min(max_money);
    Ok(ShopResult {
        success: true,
        message: format_price(payout),
        credited: state.money - starting_money,
    })
}

fn shop_money_cap(currency_constants: &CurrencyCatalog) -> Result<u32, ShopError> {
    currency_constants
        .get("MAX_MONEY")
        .ok_or_else(|| ShopError::MissingCurrencyLimit {
            constant: "MAX_MONEY".to_string(),
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

fn pocket_is_full(state: &GameState, pocket: &ItemPocket) -> bool {
    match pocket.as_str() {
        ITEM_POCKET_ITEM => active_slots(&state.bag.items) >= ITEM_POCKET_CAPACITY,
        ITEM_POCKET_BALL => active_slots(&state.bag.balls) >= BALL_POCKET_CAPACITY,
        ITEM_POCKET_KEY_ITEM => active_slots(&state.bag.key_items) >= KEY_ITEM_POCKET_CAPACITY,
        ITEM_POCKET_TM_HM => false,
        _ => true,
    }
}

fn active_slots(items: &BTreeMap<String, u16>) -> usize {
    items.values().filter(|quantity| **quantity > 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::item_pocket;

    fn item(id: &str, price: u16, pocket: ItemPocket) -> Item {
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
            price,
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

    fn items() -> BTreeMap<String, Item> {
        [
            (
                "POKE_BALL".to_string(),
                item("POKE_BALL", 200, item_pocket("BALL")),
            ),
            (
                "POTION".to_string(),
                item("POTION", 300, item_pocket("ITEM")),
            ),
            (
                "RARE_CANDY".to_string(),
                item("RARE_CANDY", 1000, item_pocket("ITEM")),
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

    fn currency_constants(max_money: u32) -> CurrencyCatalog {
        CurrencyCatalog([("MAX_MONEY".to_string(), max_money)].into_iter().collect())
    }

    #[test]
    fn mart_catalog_issues_reject_empty_ids_and_unknown_exact_items() {
        let catalog = MartCatalog(
            [
                ("".to_string(), vec!["POTION".to_string()]),
                (" CHERRYGROVE_MART".to_string(), vec!["POTION".to_string()]),
                (
                    "CHERRYGROVE_MART".to_string(),
                    vec!["POTION".to_string(), "potion".to_string()],
                ),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            mart_catalog_issues(&catalog, &items()),
            vec![
                MartCatalogIssue::EmptyMartId {
                    mart_id: String::new(),
                },
                MartCatalogIssue::InvalidMartId {
                    mart_id: " CHERRYGROVE_MART".to_string(),
                },
                MartCatalogIssue::UnknownItem {
                    mart_id: "CHERRYGROVE_MART".to_string(),
                    item_id: "potion".to_string(),
                },
            ]
        );
    }

    #[test]
    fn exported_shop_command_and_mart_type_sets_are_exact() {
        assert!(SCRIPT_SHOP_COMMANDS.contains(&"pokemart"));
        assert!(SCRIPT_SHOP_STANDARD_MART_TYPES.contains(&"MARTTYPE_STANDARD"));
        assert!(SCRIPT_SHOP_STANDARD_MART_TYPES.contains(&"MARTTYPE_BITTER"));
        assert!(SCRIPT_SHOP_ZERO_MART_TYPES.contains(&"MARTTYPE_BARGAIN"));
        assert!(SCRIPT_SHOP_ZERO_MART_TYPES.contains(&"MARTTYPE_ROOFTOP"));
        assert!(is_known_script_shop_command("pokemart"));
        assert!(is_known_script_mart_type("MARTTYPE_PHARMACY"));
        assert!(!is_known_script_shop_command("PokeMart"));
        assert!(!is_known_script_mart_type("marttype_standard"));
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
    fn validates_script_shop_command_without_mutating_runtime_state() {
        let catalog = MartCatalog(
            [("MART_CHERRYGROVE".to_string(), vec!["POTION".to_string()])]
                .into_iter()
                .collect(),
        );

        validate_script_shop_command(
            &catalog,
            &shop_command("MARTTYPE_STANDARD", "MART_CHERRYGROVE"),
        )
        .expect("known mart is valid");
        validate_script_shop_command(&catalog, &shop_command("MARTTYPE_ROOFTOP", "0"))
            .expect("zero rooftop mart is valid");

        assert_eq!(
            validate_script_shop_command(&catalog, &shop_command("marttype_standard", "MART")),
            Err(ShopError::UnknownMartType {
                mart_type: "marttype_standard".to_string()
            })
        );
        assert_eq!(
            validate_script_shop_command(&catalog, &shop_command(" MARTTYPE_STANDARD", "MART")),
            Err(ShopError::InvalidMartType {
                mart_type: " MARTTYPE_STANDARD".to_string()
            })
        );
        assert_eq!(
            validate_script_shop_command(&catalog, &shop_command("MARTTYPE_STANDARD", "0")),
            Err(ShopError::InvalidZeroMart {
                mart_type: "MARTTYPE_STANDARD".to_string()
            })
        );
        assert_eq!(
            validate_script_shop_command(&catalog, &shop_command("MARTTYPE_STANDARD", "mart")),
            Err(ShopError::UnknownMart {
                mart_id: "mart".to_string()
            })
        );
        assert_eq!(
            validate_script_shop_command(&catalog, &shop_command("MARTTYPE_STANDARD", " MART")),
            Err(ShopError::InvalidMartId {
                mart_id: " MART".to_string()
            })
        );
    }

    #[test]
    fn script_shop_command_issues_preserve_exact_source_positions() {
        let catalog = MartCatalog(
            [("MART_CHERRYGROVE".to_string(), vec!["POTION".to_string()])]
                .into_iter()
                .collect(),
        );
        let commands = vec![
            shop_command("MARTTYPE_STANDARD", "MART_CHERRYGROVE"),
            shop_command("marttype_standard", "MART_CHERRYGROVE"),
            shop_command(" MARTTYPE_STANDARD", "MART_CHERRYGROVE"),
            shop_command("MARTTYPE_STANDARD", "0"),
            shop_command("MARTTYPE_STANDARD", "mart_cherrygrove"),
            shop_command("MARTTYPE_STANDARD", " MART_CHERRYGROVE"),
        ];

        assert_eq!(
            script_shop_command_issues(&catalog, &commands),
            vec![
                ScriptShopCommandIssue {
                    source_script: "ShopScript".to_string(),
                    command_index: 11,
                    error: ShopError::UnknownMartType {
                        mart_type: "marttype_standard".to_string(),
                    },
                },
                ScriptShopCommandIssue {
                    source_script: "ShopScript".to_string(),
                    command_index: 11,
                    error: ShopError::InvalidMartType {
                        mart_type: " MARTTYPE_STANDARD".to_string(),
                    },
                },
                ScriptShopCommandIssue {
                    source_script: "ShopScript".to_string(),
                    command_index: 11,
                    error: ShopError::InvalidZeroMart {
                        mart_type: "MARTTYPE_STANDARD".to_string(),
                    },
                },
                ScriptShopCommandIssue {
                    source_script: "ShopScript".to_string(),
                    command_index: 11,
                    error: ShopError::UnknownMart {
                        mart_id: "mart_cherrygrove".to_string(),
                    },
                },
                ScriptShopCommandIssue {
                    source_script: "ShopScript".to_string(),
                    command_index: 11,
                    error: ShopError::InvalidMartId {
                        mart_id: " MART_CHERRYGROVE".to_string(),
                    },
                },
            ]
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
        let potion = item("POTION", 10, item_pocket("ITEM"));
        for index in 0..ITEM_POCKET_CAPACITY {
            let filler = item(&format!("DUMMY_ITEM_{index}"), 1, item_pocket("ITEM"));
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
        let max_money = 999_999;
        let currency_constants = currency_constants(max_money);
        let mut state = GameState {
            money: max_money - 100,
            ..GameState::default()
        };
        let items = items();
        state
            .bag
            .add_item(&items["RARE_CANDY"], 1)
            .expect("add item");

        let result =
            sell_item(&mut state, &items, &currency_constants, "RARE_CANDY", 1).expect("sell");

        assert_eq!(state.money, max_money);
        assert_eq!(result.credited, 100);
        assert_eq!(result.message, "¥000500");
        assert_eq!(state.bag.quantity(&items["RARE_CANDY"]), 0);
    }

    #[test]
    fn selling_requires_pack_max_money_without_removing_item() {
        let mut state = GameState {
            money: 500,
            ..GameState::default()
        };
        let items = items();
        state
            .bag
            .add_item(&items["RARE_CANDY"], 1)
            .expect("add item");

        assert_eq!(
            sell_item(
                &mut state,
                &items,
                &CurrencyCatalog::default(),
                "RARE_CANDY",
                1
            ),
            Err(ShopError::MissingCurrencyLimit {
                constant: "MAX_MONEY".to_string(),
            })
        );
        assert_eq!(state.money, 500);
        assert_eq!(state.bag.quantity(&items["RARE_CANDY"]), 1);
    }

    #[test]
    fn selling_rejects_unowned_or_valueless_items_without_state_change() {
        let currency_constants = currency_constants(999_999);
        let mut state = GameState {
            money: 500,
            ..GameState::default()
        };
        let mut items = items();
        items.insert(
            "FREEBIE".to_string(),
            item("FREEBIE", 1, item_pocket("ITEM")),
        );

        let unowned =
            sell_item(&mut state, &items, &currency_constants, "POTION", 1).expect("sell unowned");
        assert!(!unowned.success);
        assert_eq!(state.money, 500);

        state
            .bag
            .add_item(&items["FREEBIE"], 1)
            .expect("add freebie");
        let valueless = sell_item(&mut state, &items, &currency_constants, "FREEBIE", 1)
            .expect("sell valueless");
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
