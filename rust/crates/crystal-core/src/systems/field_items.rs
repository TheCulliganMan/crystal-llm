use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::Item;
use crate::state::{EventFlagError, GameState};
use crate::systems::script_objects::is_hideable_object_event_flag;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldItemPickup {
    pub item_id: String,
    pub quantity: u16,
    pub event_flag: String,
    pub source: FieldItemSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptFieldPickup {
    pub command: String,
    pub item_id: Option<String>,
    pub quantity: u16,
    pub event_flag: Option<String>,
    pub fruit_tree_id: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FruitTreeCatalog(pub BTreeMap<String, String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FruitTreeCatalogIssue {
    EmptyFruitTreeId {
        fruit_tree_id: String,
    },
    InvalidFruitTreeId {
        fruit_tree_id: String,
    },
    UnknownItem {
        fruit_tree_id: String,
        item_id: String,
    },
    InvalidItem {
        fruit_tree_id: String,
        item_id: String,
    },
}

pub fn fruit_tree_catalog_issues(
    catalog: &FruitTreeCatalog,
    items: &BTreeMap<String, Item>,
) -> Vec<FruitTreeCatalogIssue> {
    let mut issues = Vec::new();
    for (fruit_tree_id, item_id) in &catalog.0 {
        if fruit_tree_id.trim().is_empty() {
            issues.push(FruitTreeCatalogIssue::EmptyFruitTreeId {
                fruit_tree_id: fruit_tree_id.clone(),
            });
        } else if !is_exact_field_item_token(fruit_tree_id) {
            issues.push(FruitTreeCatalogIssue::InvalidFruitTreeId {
                fruit_tree_id: fruit_tree_id.clone(),
            });
        }
        if !is_exact_field_item_token(item_id) {
            issues.push(FruitTreeCatalogIssue::InvalidItem {
                fruit_tree_id: fruit_tree_id.clone(),
                item_id: item_id.clone(),
            });
        } else if !items.contains_key(item_id) {
            issues.push(FruitTreeCatalogIssue::UnknownItem {
                fruit_tree_id: fruit_tree_id.clone(),
                item_id: item_id.clone(),
            });
        }
    }
    issues
}

impl ScriptFieldPickup {
    pub fn to_field_item_pickup(&self) -> Result<FieldItemPickup, FieldItemError> {
        let source = match self.command.as_str() {
            "itemball" => FieldItemSource::ItemBall,
            "hiddenitem" => FieldItemSource::HiddenItem,
            "fruittree" => return Err(FieldItemError::FruitTreeRequiresCatalog),
            other if !is_exact_script_field_pickup_command_token(other) => {
                return Err(FieldItemError::InvalidScriptPickupCommand {
                    command: other.to_string(),
                });
            }
            other => {
                return Err(FieldItemError::UnknownScriptPickupCommand {
                    command: other.to_string(),
                });
            }
        };
        Ok(FieldItemPickup {
            item_id: self
                .item_id
                .clone()
                .ok_or_else(|| FieldItemError::MalformedScriptPickup {
                    command: self.command.clone(),
                    reason: "missing item_id".to_string(),
                })?,
            quantity: self.quantity,
            event_flag: self.event_flag.clone().ok_or_else(|| {
                FieldItemError::MalformedScriptPickup {
                    command: self.command.clone(),
                    reason: "missing event_flag".to_string(),
                }
            })?,
            source,
        })
    }

    pub fn to_fruit_tree_pickup(
        &self,
        fruit_trees: &FruitTreeCatalog,
    ) -> Result<FieldItemPickup, FieldItemError> {
        if self.command != "fruittree" {
            return Err(FieldItemError::UnexpectedFruitTreeCommand {
                command: self.command.clone(),
            });
        }
        if self.item_id.is_some() || self.event_flag.is_some() {
            return Err(FieldItemError::MalformedScriptPickup {
                command: self.command.clone(),
                reason: "fruittree must not inline item_id or event_flag".to_string(),
            });
        }
        let fruit_tree_id =
            self.fruit_tree_id
                .as_ref()
                .ok_or_else(|| FieldItemError::MalformedScriptPickup {
                    command: self.command.clone(),
                    reason: "missing fruit_tree_id".to_string(),
                })?;
        validate_field_item_token(fruit_tree_id).map_err(|_| FieldItemError::InvalidFruitTree {
            fruit_tree_id: fruit_tree_id.clone(),
        })?;
        let item_id = fruit_trees.0.get(fruit_tree_id).cloned().ok_or_else(|| {
            FieldItemError::UnknownFruitTree {
                fruit_tree_id: fruit_tree_id.clone(),
            }
        })?;
        Ok(FieldItemPickup {
            item_id,
            quantity: 1,
            event_flag: fruit_tree_collected_flag(fruit_tree_id),
            source: FieldItemSource::FruitTree,
        })
    }
}

pub const SCRIPT_FIELD_ITEMBALL_PICKUP_COMMANDS: &[&str] = &["itemball"];
pub const SCRIPT_FIELD_HIDDEN_ITEM_PICKUP_COMMANDS: &[&str] = &["hiddenitem"];
pub const SCRIPT_FIELD_ITEM_PICKUP_COMMANDS: &[&str] = &["itemball", "hiddenitem"];
pub const SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS: &[&str] = &["fruittree"];

pub fn is_known_script_field_pickup_command(command: &str) -> bool {
    SCRIPT_FIELD_ITEM_PICKUP_COMMANDS.contains(&command)
        || SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS.contains(&command)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptFieldPickupIssue {
    InvalidCommand,
    MissingItem,
    InvalidItem,
    UnknownItem,
    InvalidQuantity,
    MissingEvent,
    InvalidCollectibleFlag,
    MissingFruitTree,
    EmptyFruitTree,
    InvalidFruitTree,
    UnknownFruitTree,
    MalformedFruitTree,
    UnknownCommand,
}

pub fn script_field_pickup_issues(
    pickup: &ScriptFieldPickup,
    item_catalog: &BTreeMap<String, Item>,
    fruit_trees: &FruitTreeCatalog,
) -> Vec<ScriptFieldPickupIssue> {
    let mut issues = Vec::new();
    if SCRIPT_FIELD_ITEM_PICKUP_COMMANDS.contains(&pickup.command.as_str()) {
        match pickup.item_id.as_deref() {
            Some(item_id) if !is_exact_field_item_token(item_id) => {
                issues.push(ScriptFieldPickupIssue::InvalidItem);
            }
            Some(item_id) if item_catalog.contains_key(item_id) => {}
            Some(_) => issues.push(ScriptFieldPickupIssue::UnknownItem),
            None => issues.push(ScriptFieldPickupIssue::MissingItem),
        }
        if pickup.quantity == 0 {
            issues.push(ScriptFieldPickupIssue::InvalidQuantity);
        }
        match pickup.event_flag.as_deref() {
            Some(event_flag) if validate_collectible_flag(event_flag).is_ok() => {}
            Some(_) => issues.push(ScriptFieldPickupIssue::InvalidCollectibleFlag),
            None => issues.push(ScriptFieldPickupIssue::MissingEvent),
        }
    } else if SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS.contains(&pickup.command.as_str()) {
        match pickup.fruit_tree_id.as_deref() {
            Some(fruit_tree_id) if fruit_tree_id.trim().is_empty() => {
                issues.push(ScriptFieldPickupIssue::EmptyFruitTree);
            }
            Some(fruit_tree_id) if !is_exact_field_item_token(fruit_tree_id) => {
                issues.push(ScriptFieldPickupIssue::InvalidFruitTree);
            }
            Some(fruit_tree_id) if fruit_trees.0.contains_key(fruit_tree_id) => {}
            Some(_) => issues.push(ScriptFieldPickupIssue::UnknownFruitTree),
            None => issues.push(ScriptFieldPickupIssue::MissingFruitTree),
        }
        if pickup.item_id.is_some() || pickup.event_flag.is_some() {
            issues.push(ScriptFieldPickupIssue::MalformedFruitTree);
        }
    } else if !is_exact_script_field_pickup_command_token(&pickup.command) {
        issues.push(ScriptFieldPickupIssue::InvalidCommand);
    } else if !is_known_script_field_pickup_command(&pickup.command) {
        issues.push(ScriptFieldPickupIssue::UnknownCommand);
    }
    issues
}

fn is_exact_field_item_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_script_field_pickup_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldItemSource {
    ItemBall,
    HiddenItem,
    FruitTree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldItemPickupOutcome {
    Collected {
        item_id: String,
        quantity: u16,
        event_flag: String,
        source: FieldItemSource,
    },
    AlreadyCollected {
        event_flag: String,
        source: FieldItemSource,
    },
    BagFull {
        item_id: String,
        quantity: u16,
        event_flag: String,
        source: FieldItemSource,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldItemError {
    InvalidItem { item_id: String },
    InvalidFruitTree { fruit_tree_id: String },
    UnknownItem { item_id: String },
    UnknownFruitTree { fruit_tree_id: String },
    InvalidScriptPickupCommand { command: String },
    UnknownScriptPickupCommand { command: String },
    UnexpectedFruitTreeCommand { command: String },
    FruitTreeRequiresCatalog,
    MalformedScriptPickup { command: String, reason: String },
    InvalidQuantity,
    InvalidCollectibleFlag { event_flag: String },
    Flag { error: EventFlagError },
    Bag { error: String },
}

pub fn pickup_script_field_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    fruit_trees: &FruitTreeCatalog,
    pickup: ScriptFieldPickup,
) -> Result<FieldItemPickupOutcome, FieldItemError> {
    let pickup = if pickup.command == "fruittree" {
        pickup.to_fruit_tree_pickup(fruit_trees)?
    } else {
        pickup.to_field_item_pickup()?
    };
    pickup_field_item(state, item_catalog, pickup)
}

pub fn pickup_field_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    pickup: FieldItemPickup,
) -> Result<FieldItemPickupOutcome, FieldItemError> {
    if pickup.quantity == 0 {
        return Err(FieldItemError::InvalidQuantity);
    }
    validate_collectible_flag(&pickup.event_flag)?;
    validate_field_item_token(&pickup.item_id).map_err(|_| FieldItemError::InvalidItem {
        item_id: pickup.item_id.clone(),
    })?;
    if state
        .flags
        .is_event_flag_set(&pickup.event_flag)
        .map_err(|error| FieldItemError::Flag { error })?
    {
        return Ok(FieldItemPickupOutcome::AlreadyCollected {
            event_flag: pickup.event_flag,
            source: pickup.source,
        });
    }

    let item = item_catalog
        .get(&pickup.item_id)
        .ok_or_else(|| FieldItemError::UnknownItem {
            item_id: pickup.item_id.clone(),
        })?;
    let added = state
        .bag
        .add_item(item, pickup.quantity)
        .map_err(|error| FieldItemError::Bag { error })?;
    if !added {
        return Ok(FieldItemPickupOutcome::BagFull {
            item_id: pickup.item_id,
            quantity: pickup.quantity,
            event_flag: pickup.event_flag,
            source: pickup.source,
        });
    }

    state
        .flags
        .set_event_flag(&pickup.event_flag, true)
        .map_err(|error| FieldItemError::Flag { error })?;
    Ok(FieldItemPickupOutcome::Collected {
        item_id: pickup.item_id,
        quantity: pickup.quantity,
        event_flag: pickup.event_flag,
        source: pickup.source,
    })
}

pub fn fruit_tree_collected_flag(fruit_tree_id: &str) -> String {
    format!("{fruit_tree_id}_COLLECTED")
}

fn validate_collectible_flag(event_flag: &str) -> Result<(), FieldItemError> {
    if !is_hideable_object_event_flag(event_flag) {
        return Err(FieldItemError::InvalidCollectibleFlag {
            event_flag: event_flag.to_string(),
        });
    }
    Ok(())
}

fn validate_field_item_token(value: &str) -> Result<(), ()> {
    if is_exact_field_item_token(value) {
        Ok(())
    } else {
        Err(())
    }
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

    fn pickup(item_id: &str, event_flag: &str, source: FieldItemSource) -> FieldItemPickup {
        FieldItemPickup {
            item_id: item_id.to_string(),
            quantity: 1,
            event_flag: event_flag.to_string(),
            source,
        }
    }

    fn script_pickup(command: &str) -> ScriptFieldPickup {
        ScriptFieldPickup {
            command: command.to_string(),
            item_id: None,
            quantity: 1,
            event_flag: None,
            fruit_tree_id: None,
            source_script: "FieldScript".to_string(),
            command_index: 2,
        }
    }

    #[test]
    fn fruit_tree_catalog_issues_reject_empty_ids_invalid_items_and_unknown_exact_items() {
        let fruit_trees = FruitTreeCatalog(
            [
                ("".to_string(), "BERRY".to_string()),
                (" ROUTE_29_FRUIT_TREE".to_string(), "BERRY".to_string()),
                ("ROUTE 29_FRUIT_TREE".to_string(), "BERRY".to_string()),
                ("ROUTE_30_FRUIT_TREE".to_string(), "GOLD BERRY".to_string()),
                ("ROUTE_29_FRUIT_TREE".to_string(), "berry".to_string()),
            ]
            .into_iter()
            .collect(),
        );
        let items = catalog(vec![item("BERRY", item_pocket("ITEM"))]);

        assert_eq!(
            fruit_tree_catalog_issues(&fruit_trees, &items),
            vec![
                FruitTreeCatalogIssue::EmptyFruitTreeId {
                    fruit_tree_id: String::new(),
                },
                FruitTreeCatalogIssue::InvalidFruitTreeId {
                    fruit_tree_id: " ROUTE_29_FRUIT_TREE".to_string(),
                },
                FruitTreeCatalogIssue::InvalidFruitTreeId {
                    fruit_tree_id: "ROUTE 29_FRUIT_TREE".to_string(),
                },
                FruitTreeCatalogIssue::InvalidItem {
                    fruit_tree_id: "ROUTE_30_FRUIT_TREE".to_string(),
                    item_id: "GOLD BERRY".to_string(),
                },
                FruitTreeCatalogIssue::UnknownItem {
                    fruit_tree_id: "ROUTE_29_FRUIT_TREE".to_string(),
                    item_id: "berry".to_string(),
                },
            ]
        );
    }

    #[test]
    fn exported_field_pickup_command_sets_are_exact() {
        assert!(SCRIPT_FIELD_ITEM_PICKUP_COMMANDS.contains(&"itemball"));
        assert!(SCRIPT_FIELD_ITEM_PICKUP_COMMANDS.contains(&"hiddenitem"));
        assert!(SCRIPT_FIELD_ITEMBALL_PICKUP_COMMANDS.contains(&"itemball"));
        assert!(SCRIPT_FIELD_HIDDEN_ITEM_PICKUP_COMMANDS.contains(&"hiddenitem"));
        assert!(SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS.contains(&"fruittree"));
        assert!(is_known_script_field_pickup_command("hiddenitem"));
        assert!(!is_known_script_field_pickup_command("HiddenItem"));
        assert!(!is_known_script_field_pickup_command("berrytree"));
    }

    #[test]
    fn itemball_pickup_adds_exact_item_and_sets_exact_event_flag() {
        let mut state = GameState::default();
        let items = catalog(vec![item("ANTIDOTE", item_pocket("ITEM"))]);

        let outcome = pickup_field_item(
            &mut state,
            &items,
            pickup(
                "ANTIDOTE",
                "EVENT_ROUTE_29_POTION",
                FieldItemSource::ItemBall,
            ),
        )
        .expect("pickup succeeds");

        assert_eq!(
            outcome,
            FieldItemPickupOutcome::Collected {
                item_id: "ANTIDOTE".to_string(),
                quantity: 1,
                event_flag: "EVENT_ROUTE_29_POTION".to_string(),
                source: FieldItemSource::ItemBall,
            }
        );
        assert_eq!(state.bag.items["ANTIDOTE"], 1);
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_ROUTE_29_POTION"),
            Ok(true)
        );
    }

    #[test]
    fn hidden_item_pickup_does_not_add_item_twice() {
        let mut state = GameState::default();
        state
            .flags
            .set_event_flag("EVENT_GOT_HIDDEN_ANTIDOTE", true)
            .expect("set preexisting flag");
        let items = catalog(vec![item("ANTIDOTE", item_pocket("ITEM"))]);

        let outcome = pickup_field_item(
            &mut state,
            &items,
            pickup(
                "ANTIDOTE",
                "EVENT_GOT_HIDDEN_ANTIDOTE",
                FieldItemSource::HiddenItem,
            ),
        )
        .expect("already collected");

        assert_eq!(
            outcome,
            FieldItemPickupOutcome::AlreadyCollected {
                event_flag: "EVENT_GOT_HIDDEN_ANTIDOTE".to_string(),
                source: FieldItemSource::HiddenItem,
            }
        );
        assert!(state.bag.items.is_empty());
    }

    #[test]
    fn full_bag_does_not_set_collection_flag() {
        let mut state = GameState::default();
        let antidote = item("ANTIDOTE", item_pocket("ITEM"));
        state
            .bag
            .add_item(&antidote, MAX_ITEM_STACK)
            .expect("fill stack");
        let items = catalog(vec![antidote]);

        let outcome = pickup_field_item(
            &mut state,
            &items,
            pickup(
                "ANTIDOTE",
                "EVENT_GOT_HIDDEN_ANTIDOTE",
                FieldItemSource::HiddenItem,
            ),
        )
        .expect("bag full");

        assert_eq!(
            outcome,
            FieldItemPickupOutcome::BagFull {
                item_id: "ANTIDOTE".to_string(),
                quantity: 1,
                event_flag: "EVENT_GOT_HIDDEN_ANTIDOTE".to_string(),
                source: FieldItemSource::HiddenItem,
            }
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_GOT_HIDDEN_ANTIDOTE"),
            Ok(false)
        );
    }

    #[test]
    fn pickup_requires_exact_catalog_item_id_without_case_coercion() {
        let mut state = GameState::default();
        let items = catalog(vec![item("ANTIDOTE", item_pocket("ITEM"))]);

        assert_eq!(
            pickup_field_item(
                &mut state,
                &items,
                pickup(
                    "antidote",
                    "EVENT_GOT_HIDDEN_ANTIDOTE",
                    FieldItemSource::HiddenItem,
                ),
            ),
            Err(FieldItemError::UnknownItem {
                item_id: "antidote".to_string(),
            })
        );
    }

    #[test]
    fn pickup_rejects_malformed_item_id_before_unknown_lookup() {
        let mut state = GameState::default();
        let items = catalog(vec![item("RARE_CANDY", item_pocket("ITEM"))]);

        assert_eq!(
            pickup_field_item(
                &mut state,
                &items,
                pickup(
                    "RARE CANDY",
                    "EVENT_GOT_HIDDEN_ANTIDOTE",
                    FieldItemSource::HiddenItem,
                ),
            ),
            Err(FieldItemError::InvalidItem {
                item_id: "RARE CANDY".to_string(),
            })
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_GOT_HIDDEN_ANTIDOTE"),
            Ok(false)
        );
        assert!(state.bag.items.is_empty());
    }

    #[test]
    fn pickup_rejects_unhideable_event_flags() {
        let mut state = GameState::default();
        let items = catalog(vec![item("ANTIDOTE", item_pocket("ITEM"))]);

        assert_eq!(
            pickup_field_item(
                &mut state,
                &items,
                pickup("ANTIDOTE", "-1", FieldItemSource::ItemBall),
            ),
            Err(FieldItemError::InvalidCollectibleFlag {
                event_flag: "-1".to_string(),
            })
        );
    }

    #[test]
    fn script_pickup_conversion_errors_instead_of_dropping_unknown_commands() {
        let pickup = script_pickup("giveitem");

        assert_eq!(
            pickup.to_field_item_pickup(),
            Err(FieldItemError::UnknownScriptPickupCommand {
                command: "giveitem".to_string(),
            })
        );
        assert_eq!(
            script_field_pickup_issues(&pickup, &BTreeMap::new(), &FruitTreeCatalog::default()),
            vec![ScriptFieldPickupIssue::UnknownCommand]
        );

        let pickup = script_pickup("ItemBall");
        assert_eq!(
            pickup.to_field_item_pickup(),
            Err(FieldItemError::InvalidScriptPickupCommand {
                command: "ItemBall".to_string(),
            })
        );
        assert_eq!(
            script_field_pickup_issues(&pickup, &BTreeMap::new(), &FruitTreeCatalog::default()),
            vec![ScriptFieldPickupIssue::InvalidCommand]
        );
        let pickup = script_pickup("item ball");
        assert_eq!(
            script_field_pickup_issues(&pickup, &BTreeMap::new(), &FruitTreeCatalog::default()),
            vec![ScriptFieldPickupIssue::InvalidCommand]
        );
    }

    #[test]
    fn script_pickup_issue_collector_reports_exact_pack_shape_errors() {
        let items = catalog(vec![item("BERRY", item_pocket("ITEM"))]);
        let fruit_trees = FruitTreeCatalog(
            [("FRUITTREE_ROUTE_29".to_string(), "BERRY".to_string())]
                .into_iter()
                .collect(),
        );

        let mut bad_item = script_pickup("itemball");
        bad_item.quantity = 0;
        bad_item.event_flag = Some("-1".to_string());
        assert_eq!(
            script_field_pickup_issues(&bad_item, &items, &fruit_trees),
            vec![
                ScriptFieldPickupIssue::MissingItem,
                ScriptFieldPickupIssue::InvalidQuantity,
                ScriptFieldPickupIssue::InvalidCollectibleFlag,
            ]
        );

        bad_item.item_id = Some("RARE CANDY".to_string());
        assert_eq!(
            script_field_pickup_issues(&bad_item, &items, &fruit_trees),
            vec![
                ScriptFieldPickupIssue::InvalidItem,
                ScriptFieldPickupIssue::InvalidQuantity,
                ScriptFieldPickupIssue::InvalidCollectibleFlag,
            ]
        );

        let mut bad_fruit = script_pickup("fruittree");
        bad_fruit.fruit_tree_id = Some(String::new());
        bad_fruit.item_id = Some("BERRY".to_string());
        assert_eq!(
            script_field_pickup_issues(&bad_fruit, &items, &fruit_trees),
            vec![
                ScriptFieldPickupIssue::EmptyFruitTree,
                ScriptFieldPickupIssue::MalformedFruitTree,
            ]
        );

        bad_fruit.fruit_tree_id = Some(" FRUITTREE_ROUTE_29".to_string());
        assert_eq!(
            script_field_pickup_issues(&bad_fruit, &items, &fruit_trees),
            vec![
                ScriptFieldPickupIssue::InvalidFruitTree,
                ScriptFieldPickupIssue::MalformedFruitTree,
            ]
        );
        bad_fruit.fruit_tree_id = Some("FRUITTREE ROUTE_29".to_string());
        assert_eq!(
            script_field_pickup_issues(&bad_fruit, &items, &fruit_trees),
            vec![
                ScriptFieldPickupIssue::InvalidFruitTree,
                ScriptFieldPickupIssue::MalformedFruitTree,
            ]
        );
    }

    #[test]
    fn fruit_tree_pickup_uses_exact_catalog_item_and_collected_flag() {
        let mut state = GameState::default();
        let items = catalog(vec![item("BERRY", item_pocket("ITEM"))]);
        let fruit_trees = FruitTreeCatalog(
            [("FRUITTREE_ROUTE_29".to_string(), "BERRY".to_string())]
                .into_iter()
                .collect(),
        );
        let mut pickup = script_pickup("fruittree");
        pickup.fruit_tree_id = Some("FRUITTREE_ROUTE_29".to_string());

        let outcome = pickup_script_field_item(&mut state, &items, &fruit_trees, pickup)
            .expect("fruit tree pickup");

        assert_eq!(
            outcome,
            FieldItemPickupOutcome::Collected {
                item_id: "BERRY".to_string(),
                quantity: 1,
                event_flag: "FRUITTREE_ROUTE_29_COLLECTED".to_string(),
                source: FieldItemSource::FruitTree,
            }
        );
        assert_eq!(state.bag.items["BERRY"], 1);
        assert_eq!(
            state
                .flags
                .is_event_flag_set("FRUITTREE_ROUTE_29_COLLECTED"),
            Ok(true)
        );
    }

    #[test]
    fn fruit_tree_pickup_rejects_unknown_or_case_changed_tree_id_without_item_fallback() {
        let mut state = GameState::default();
        let items = catalog(vec![item("BERRY", item_pocket("ITEM"))]);
        let fruit_trees = FruitTreeCatalog(
            [("FRUITTREE_ROUTE_29".to_string(), "BERRY".to_string())]
                .into_iter()
                .collect(),
        );
        let mut pickup = script_pickup("fruittree");
        pickup.fruit_tree_id = Some("fruittree_route_29".to_string());

        assert_eq!(
            pickup_script_field_item(&mut state, &items, &fruit_trees, pickup),
            Err(FieldItemError::UnknownFruitTree {
                fruit_tree_id: "fruittree_route_29".to_string(),
            })
        );
        assert!(state.bag.items.is_empty());
        assert_eq!(
            state
                .flags
                .is_event_flag_set("fruittree_route_29_COLLECTED"),
            Ok(false)
        );
    }

    #[test]
    fn fruit_tree_pickup_rejects_malformed_tree_id_before_unknown_lookup() {
        let mut state = GameState::default();
        let items = catalog(vec![item("BERRY", item_pocket("ITEM"))]);
        let fruit_trees = FruitTreeCatalog(
            [("FRUITTREE_ROUTE_29".to_string(), "BERRY".to_string())]
                .into_iter()
                .collect(),
        );
        let mut pickup = script_pickup("fruittree");
        pickup.fruit_tree_id = Some("FRUITTREE ROUTE_29".to_string());

        assert_eq!(
            pickup_script_field_item(&mut state, &items, &fruit_trees, pickup),
            Err(FieldItemError::InvalidFruitTree {
                fruit_tree_id: "FRUITTREE ROUTE_29".to_string(),
            })
        );
        assert!(state.bag.items.is_empty());
        assert_eq!(
            state
                .flags
                .is_event_flag_set("FRUITTREE_ROUTE_29_COLLECTED"),
            Ok(false)
        );
    }
}
