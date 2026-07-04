use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use super::item::{
    ITEM_POCKET_BALL, ITEM_POCKET_ITEM, ITEM_POCKET_KEY_ITEM, ITEM_POCKET_TM_HM, Item,
};

pub const MAX_ITEM_STACK: u16 = 99;
pub const ITEM_POCKET_CAPACITY: usize = 20;
pub const BALL_POCKET_CAPACITY: usize = 12;
pub const KEY_ITEM_POCKET_CAPACITY: usize = 25;
pub const PC_ITEM_CAPACITY: usize = 50;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bag {
    pub items: BTreeMap<String, u16>,
    pub pc_items: BTreeMap<String, u16>,
    pub balls: BTreeMap<String, u16>,
    pub key_items: BTreeMap<String, u16>,
    pub tm_hm: Vec<bool>,
    pub custom_pockets: BTreeMap<String, BTreeMap<String, u16>>,
}

impl<'de> Deserialize<'de> for Bag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBag {
            items: BTreeMap<String, u16>,
            pc_items: BTreeMap<String, u16>,
            balls: BTreeMap<String, u16>,
            key_items: BTreeMap<String, u16>,
            tm_hm: Vec<bool>,
            custom_pockets: BTreeMap<String, BTreeMap<String, u16>>,
        }

        let raw = RawBag::deserialize(deserializer)?;
        let bag = Self {
            items: raw.items,
            pc_items: raw.pc_items,
            balls: raw.balls,
            key_items: raw.key_items,
            tm_hm: raw.tm_hm,
            custom_pockets: raw.custom_pockets,
        };
        bag.validate().map_err(D::Error::custom)?;
        Ok(bag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum BagSaveError {
    #[error("saved {path} item {item_id} is missing from compiled pack items")]
    MissingItem { path: String, item_id: String },
    #[error("saved {path} item {item_id} does not match compiled item script_name {script_name}")]
    ItemScriptNameMismatch {
        path: String,
        item_id: String,
        script_name: String,
    },
    #[error(
        "saved {path} item {item_id} is in compiled pocket {actual_pocket}, expected {expected_pocket}"
    )]
    WrongPocket {
        path: String,
        item_id: String,
        actual_pocket: String,
        expected_pocket: String,
    },
}

impl Bag {
    pub fn add_item(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        match definition.pocket.as_str() {
            ITEM_POCKET_ITEM => add_to_inventory(
                &mut self.items,
                &definition.script_name,
                quantity,
                MAX_ITEM_STACK,
                Some(ITEM_POCKET_CAPACITY),
            ),
            ITEM_POCKET_BALL => add_to_inventory(
                &mut self.balls,
                &definition.script_name,
                quantity,
                MAX_ITEM_STACK,
                Some(BALL_POCKET_CAPACITY),
            ),
            ITEM_POCKET_KEY_ITEM => add_to_inventory(
                &mut self.key_items,
                &definition.script_name,
                quantity,
                1,
                Some(KEY_ITEM_POCKET_CAPACITY),
            ),
            ITEM_POCKET_TM_HM => self.add_tmhm(definition),
            other => add_to_custom_pocket(
                &mut self.custom_pockets,
                other,
                &definition.script_name,
                quantity,
            ),
        }
    }

    pub fn remove_item(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        match definition.pocket.as_str() {
            ITEM_POCKET_ITEM => {
                remove_from_inventory(&mut self.items, &definition.script_name, quantity)
            }
            ITEM_POCKET_BALL => {
                remove_from_inventory(&mut self.balls, &definition.script_name, quantity)
            }
            ITEM_POCKET_KEY_ITEM => {
                remove_from_inventory(&mut self.key_items, &definition.script_name, quantity)
            }
            ITEM_POCKET_TM_HM => self.remove_tmhm(definition),
            other => remove_from_custom_pocket(
                &mut self.custom_pockets,
                other,
                &definition.script_name,
                quantity,
            ),
        }
    }

    pub fn has_item(&self, definition: &Item) -> bool {
        self.quantity(definition) > 0
    }

    pub fn has_pc_item(&self, definition: &Item) -> bool {
        self.pc_item_quantity(definition) > 0
    }

    pub fn add_pc_item(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        if definition.pocket != ITEM_POCKET_ITEM {
            return Err(format!(
                "PC item '{}' is not in the ITEM pocket",
                definition.script_name
            ));
        }
        add_to_inventory(
            &mut self.pc_items,
            &definition.script_name,
            quantity,
            MAX_ITEM_STACK,
            Some(PC_ITEM_CAPACITY),
        )
    }

    pub fn remove_pc_item(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        if definition.pocket != ITEM_POCKET_ITEM {
            return Err(format!(
                "PC item '{}' is not in the ITEM pocket",
                definition.script_name
            ));
        }
        remove_from_inventory(&mut self.pc_items, &definition.script_name, quantity)
    }

    pub fn quantity(&self, definition: &Item) -> u16 {
        match definition.pocket.as_str() {
            ITEM_POCKET_ITEM => self
                .items
                .get(&definition.script_name)
                .copied()
                .unwrap_or(0),
            ITEM_POCKET_BALL => self
                .balls
                .get(&definition.script_name)
                .copied()
                .unwrap_or(0),
            ITEM_POCKET_KEY_ITEM => self
                .key_items
                .get(&definition.script_name)
                .copied()
                .unwrap_or(0),
            ITEM_POCKET_TM_HM => definition
                .tmhm_index
                .and_then(|index| self.tm_hm.get(index).copied())
                .map(u16::from)
                .unwrap_or(0),
            other => self
                .custom_pockets
                .get(other)
                .and_then(|inventory| inventory.get(&definition.script_name).copied())
                .unwrap_or(0),
        }
    }

    pub fn pc_item_quantity(&self, definition: &Item) -> u16 {
        if definition.pocket != ITEM_POCKET_ITEM {
            return 0;
        }
        self.pc_items
            .get(&definition.script_name)
            .copied()
            .unwrap_or(0)
    }

    pub fn consume_ball(&mut self, definition: &Item) -> Result<bool, String> {
        self.remove_item(definition, 1)
    }

    fn add_tmhm(&mut self, definition: &Item) -> Result<bool, String> {
        let Some(index) = definition.tmhm_index else {
            return Err(format!(
                "TM/HM item id '{}' is not indexed",
                definition.script_name
            ));
        };
        if self.tm_hm.len() <= index {
            self.tm_hm.resize(index + 1, false);
        }
        if self.tm_hm[index] {
            return Ok(false);
        }
        self.tm_hm[index] = true;
        Ok(true)
    }

    fn remove_tmhm(&mut self, definition: &Item) -> Result<bool, String> {
        let Some(index) = definition.tmhm_index else {
            return Err(format!(
                "TM/HM item id '{}' is not indexed",
                definition.script_name
            ));
        };
        let Some(flag) = self.tm_hm.get_mut(index) else {
            return Ok(false);
        };
        if !*flag {
            return Ok(false);
        }
        *flag = false;
        Ok(true)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_inventory(&self.items, MAX_ITEM_STACK, ITEM_POCKET_CAPACITY, "items")?;
        validate_inventory(&self.pc_items, MAX_ITEM_STACK, PC_ITEM_CAPACITY, "pc_items")?;
        validate_inventory(&self.balls, MAX_ITEM_STACK, BALL_POCKET_CAPACITY, "balls")?;
        validate_inventory(&self.key_items, 1, KEY_ITEM_POCKET_CAPACITY, "key_items")?;
        validate_custom_pockets(&self.custom_pockets)?;
        Ok(())
    }
}

fn add_to_custom_pocket(
    custom_pockets: &mut BTreeMap<String, BTreeMap<String, u16>>,
    pocket_id: &str,
    item_id: &str,
    quantity: u16,
) -> Result<bool, String> {
    validate_pocket_id(pocket_id)?;
    let inventory = custom_pockets.entry(pocket_id.to_string()).or_default();
    add_to_inventory(inventory, item_id, quantity, MAX_ITEM_STACK, None)
}

fn remove_from_custom_pocket(
    custom_pockets: &mut BTreeMap<String, BTreeMap<String, u16>>,
    pocket_id: &str,
    item_id: &str,
    quantity: u16,
) -> Result<bool, String> {
    validate_pocket_id(pocket_id)?;
    let Some(inventory) = custom_pockets.get_mut(pocket_id) else {
        return Ok(false);
    };
    remove_from_inventory(inventory, item_id, quantity)
}

pub fn validate_saved_bag_pocket_references(
    items: &BTreeMap<String, Item>,
    path: &str,
    inventory: &BTreeMap<String, u16>,
    expected_pocket: &str,
) -> Result<(), BagSaveError> {
    for item_id in inventory.keys() {
        let item = items
            .get(item_id)
            .ok_or_else(|| BagSaveError::MissingItem {
                path: path.to_string(),
                item_id: item_id.clone(),
            })?;
        if item.script_name.as_str() != item_id {
            return Err(BagSaveError::ItemScriptNameMismatch {
                path: path.to_string(),
                item_id: item_id.clone(),
                script_name: item.script_name.clone(),
            });
        }
        if item.pocket != expected_pocket {
            return Err(BagSaveError::WrongPocket {
                path: path.to_string(),
                item_id: item_id.clone(),
                actual_pocket: item.pocket.clone(),
                expected_pocket: expected_pocket.to_string(),
            });
        }
    }
    Ok(())
}

fn add_to_inventory(
    inventory: &mut BTreeMap<String, u16>,
    item_id: &str,
    quantity: u16,
    stack_limit: u16,
    capacity: Option<usize>,
) -> Result<bool, String> {
    validate_item_id(item_id)?;
    let current = inventory.get(item_id).copied().unwrap_or(0);
    if current >= stack_limit {
        return Ok(false);
    }
    if current == 0
        && let Some(capacity) = capacity
        && inventory.values().filter(|quantity| **quantity > 0).count() >= capacity
    {
        return Ok(false);
    }
    let Some(next) = current.checked_add(quantity) else {
        return Ok(false);
    };
    if next > stack_limit {
        return Ok(false);
    }
    inventory.insert(item_id.to_string(), next);
    Ok(true)
}

fn remove_from_inventory(
    inventory: &mut BTreeMap<String, u16>,
    item_id: &str,
    quantity: u16,
) -> Result<bool, String> {
    validate_item_id(item_id)?;
    let Some(current) = inventory.get(item_id).copied() else {
        return Ok(false);
    };
    if current < quantity {
        return Ok(false);
    }
    let next = current - quantity;
    if next == 0 {
        inventory.remove(item_id);
    } else {
        inventory.insert(item_id.to_string(), next);
    }
    Ok(true)
}

fn validate_inventory(
    inventory: &BTreeMap<String, u16>,
    stack_limit: u16,
    capacity: usize,
    label: &str,
) -> Result<(), String> {
    let active = inventory.values().filter(|quantity| **quantity > 0).count();
    if active > capacity {
        return Err(format!(
            "{label} has {active} active slots, capacity is {capacity}"
        ));
    }
    for (item_id, quantity) in inventory {
        if item_id.is_empty() {
            return Err(format!("{label} contains an empty item id"));
        }
        if item_id.trim() != item_id {
            return Err(format!(
                "{label} contains item id '{item_id}' that must be exact and untrimmed"
            ));
        }
        if !is_exact_item_id(item_id) {
            return Err(format!(
                "{label} contains item id '{item_id}' that must contain only ASCII letters, numbers, or underscores"
            ));
        }
        if *quantity > stack_limit {
            return Err(format!(
                "{label}.{item_id} quantity {quantity} exceeds stack limit {stack_limit}"
            ));
        }
    }
    Ok(())
}

fn validate_custom_pockets(
    custom_pockets: &BTreeMap<String, BTreeMap<String, u16>>,
) -> Result<(), String> {
    for (pocket_id, inventory) in custom_pockets {
        validate_pocket_id(pocket_id)?;
        match pocket_id.as_str() {
            ITEM_POCKET_ITEM | ITEM_POCKET_BALL | ITEM_POCKET_KEY_ITEM | ITEM_POCKET_TM_HM => {
                return Err(format!(
                    "custom_pockets must not redefine built-in pocket {pocket_id}"
                ));
            }
            _ => {}
        }
        validate_inventory(
            inventory,
            MAX_ITEM_STACK,
            usize::MAX,
            &format!("custom_pockets.{pocket_id}"),
        )?;
    }
    Ok(())
}

fn is_exact_item_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn validate_item_id(item_id: &str) -> Result<(), String> {
    if item_id.is_empty() {
        return Err("item id is required".to_string());
    }
    if item_id.trim() != item_id {
        return Err(format!("item id '{item_id}' must be exact and untrimmed"));
    }
    if !is_exact_item_id(item_id) {
        return Err(format!(
            "item id '{item_id}' must contain only ASCII letters, numbers, or underscores"
        ));
    }
    Ok(())
}

fn validate_pocket_id(pocket_id: &str) -> Result<(), String> {
    if pocket_id.is_empty() {
        return Err("item pocket id is required".to_string());
    }
    if pocket_id.trim() != pocket_id {
        return Err(format!(
            "item pocket id '{pocket_id}' must be exact and untrimmed"
        ));
    }
    if !is_exact_item_id(pocket_id) {
        return Err(format!(
            "item pocket id '{pocket_id}' must contain only ASCII letters, numbers, or underscores"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ItemPocket, item_pocket};

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

    #[test]
    fn bag_adds_and_consumes_exact_ball_ids() {
        let poke_ball = item("POKE_BALL", item_pocket("BALL"));
        let mut bag = Bag::default();

        assert!(bag.add_item(&poke_ball, 2).expect("add balls"));
        assert_eq!(bag.quantity(&poke_ball), 2);
        assert!(bag.consume_ball(&poke_ball).expect("consume ball"));
        assert_eq!(bag.quantity(&poke_ball), 1);
        assert!(bag.consume_ball(&poke_ball).expect("consume ball"));
        assert_eq!(bag.quantity(&poke_ball), 0);
        assert!(!bag.consume_ball(&poke_ball).expect("no balls left"));
    }

    #[test]
    fn bag_consumes_capture_items_from_exact_custom_pockets() {
        let prism_ball = item("PRISM_BALL", item_pocket("PRISM_BALL"));
        let mut bag = Bag::default();

        assert!(bag.add_item(&prism_ball, 2).expect("add custom ball"));
        assert_eq!(bag.quantity(&prism_ball), 2);
        assert!(bag.consume_ball(&prism_ball).expect("consume custom ball"));
        assert_eq!(bag.quantity(&prism_ball), 1);
        assert_eq!(
            bag.custom_pockets
                .get("PRISM_BALL")
                .and_then(|pocket| pocket.get("PRISM_BALL"))
                .copied(),
            Some(1)
        );
    }

    #[test]
    fn bag_uses_pocket_capacities_without_item_id_coercion() {
        let mut bag = Bag::default();
        for index in 0..BALL_POCKET_CAPACITY {
            let ball = item(&format!("MOD_BALL_{index}"), item_pocket("BALL"));
            assert!(bag.add_item(&ball, 1).expect("add ball"));
        }
        let extra = item("mod_ball_0", item_pocket("BALL"));
        assert!(!bag.add_item(&extra, 1).expect("ball pocket full"));
        assert_eq!(bag.quantity(&extra), 0);
        bag.validate().expect("valid bag");
    }

    #[test]
    fn bag_rejects_malformed_item_ids_without_trimming() {
        let padded_ball = item(" POKE_BALL", item_pocket("BALL"));
        let mut bag = Bag::default();

        assert_eq!(
            bag.add_item(&padded_ball, 1),
            Err("item id ' POKE_BALL' must be exact and untrimmed".to_string()),
        );
        assert_eq!(bag.quantity(&padded_ball), 0);

        let spaced_ball = item("POKE BALL", item_pocket("BALL"));
        assert_eq!(
            bag.add_item(&spaced_ball, 1),
            Err(
                "item id 'POKE BALL' must contain only ASCII letters, numbers, or underscores"
                    .to_string()
            ),
        );
        assert_eq!(
            bag.remove_item(&spaced_ball, 1),
            Err(
                "item id 'POKE BALL' must contain only ASCII letters, numbers, or underscores"
                    .to_string()
            ),
        );

        bag.balls.insert("POKE_BALL ".to_string(), 1);
        assert_eq!(
            bag.validate(),
            Err("balls contains item id 'POKE_BALL ' that must be exact and untrimmed".to_string()),
        );

        bag.balls.clear();
        bag.balls.insert("POKE BALL".to_string(), 1);
        assert_eq!(
            bag.validate(),
            Err(
                "balls contains item id 'POKE BALL' that must contain only ASCII letters, numbers, or underscores"
                    .to_string()
            ),
        );
    }

    #[test]
    fn bag_rejects_reserved_pack_prefix_item_ids() {
        let fallback_ball = item("fallback_POKE_BALL", item_pocket("BALL"));
        let mut bag = Bag::default();

        assert_eq!(
            bag.add_item(&fallback_ball, 1),
            Err(
                "item id 'fallback_POKE_BALL' must contain only ASCII letters, numbers, or underscores"
                    .to_string()
            ),
        );

        bag.balls.insert("legacy_POKE_BALL".to_string(), 1);
        assert_eq!(
            bag.validate(),
            Err(
                "balls contains item id 'legacy_POKE_BALL' that must contain only ASCII letters, numbers, or underscores"
                    .to_string()
            ),
        );
    }

    #[test]
    fn key_items_do_not_stack_and_tmhm_flags_are_exact() {
        let bicycle = item("BICYCLE", item_pocket("KEY_ITEM"));
        let mut tm_mud_slap = item("TM_MUD_SLAP", item_pocket("TM_HM"));
        tm_mud_slap.tmhm_index = Some(30);
        let mut bag = Bag::default();

        assert!(bag.add_item(&bicycle, 1).expect("add key item"));
        assert!(!bag.add_item(&bicycle, 1).expect("key item already held"));
        assert!(bag.add_item(&tm_mud_slap, 1).expect("add tm"));
        assert_eq!(bag.quantity(&tm_mud_slap), 1);
        assert!(!bag.add_item(&tm_mud_slap, 1).expect("tm already held"));
        assert!(bag.remove_item(&tm_mud_slap, 1).expect("remove tm"));
        assert_eq!(bag.quantity(&tm_mud_slap), 0);
    }

    #[test]
    fn tmhm_items_require_explicit_index_data() {
        let tm = item("TM_MUD_SLAP", item_pocket("TM_HM"));
        let mut bag = Bag::default();

        let error = bag.add_item(&tm, 1).expect_err("missing tmhm index");

        assert_eq!(error, "TM/HM item id 'TM_MUD_SLAP' is not indexed");
    }

    #[test]
    fn custom_pack_pockets_store_exact_items_without_core_enum_support() {
        let pass = item("BATTLE_PASS", item_pocket("BATTLE_PASS"));
        let mut bag = Bag::default();

        assert!(bag.add_item(&pass, 2).expect("add custom pocket item"));
        assert_eq!(bag.quantity(&pass), 2);
        assert_eq!(bag.custom_pockets["BATTLE_PASS"]["BATTLE_PASS"], 2);
        assert!(
            bag.remove_item(&pass, 1)
                .expect("remove custom pocket item")
        );
        assert_eq!(bag.quantity(&pass), 1);
        bag.validate().expect("valid custom pocket");
    }

    #[test]
    fn custom_pack_pockets_reject_malformed_or_builtin_pocket_ids() {
        let pass = item("BATTLE_PASS", item_pocket("BATTLE PASS"));
        let mut bag = Bag::default();

        assert_eq!(
            bag.add_item(&pass, 1),
            Err(
                "item pocket id 'BATTLE PASS' must contain only ASCII letters, numbers, or underscores"
                    .to_string()
            )
        );

        bag.custom_pockets.insert(
            ITEM_POCKET_ITEM.to_string(),
            BTreeMap::from([("POTION".to_string(), 1)]),
        );
        assert_eq!(
            bag.validate(),
            Err("custom_pockets must not redefine built-in pocket ITEM".to_string())
        );
    }

    #[test]
    fn pc_items_use_exact_item_pocket_storage() {
        let potion = item("POTION", item_pocket("ITEM"));
        let ball = item("POKE_BALL", item_pocket("BALL"));
        let mut bag = Bag::default();

        assert!(bag.add_pc_item(&potion, 2).expect("add pc item"));
        assert_eq!(bag.pc_item_quantity(&potion), 2);
        assert!(bag.has_pc_item(&potion));
        assert_eq!(
            bag.add_pc_item(&ball, 1),
            Err("PC item 'POKE_BALL' is not in the ITEM pocket".to_string())
        );
        bag.validate().expect("valid pc items");
    }

    #[test]
    fn bag_json_rejects_unknown_inventory_fields_without_legacy_fallbacks() {
        let error = serde_json::from_value::<Bag>(serde_json::json!({
            "items": {},
            "pc_items": {},
            "balls": {},
            "key_items": {},
            "tm_hm": [],
            "custom_pockets": {},
            "legacy_pc_items": {}
        }))
        .expect_err("bag saves must not accept legacy inventory fields")
        .to_string();

        assert!(error.contains("unknown field `legacy_pc_items`"), "{error}");
    }

    #[test]
    fn bag_json_requires_all_inventory_pockets_without_empty_defaults() {
        let complete = serde_json::json!({
            "items": {},
            "pc_items": {},
            "balls": {},
            "key_items": {},
            "tm_hm": [],
            "custom_pockets": {}
        });
        serde_json::from_value::<Bag>(complete.clone())
            .expect("explicit empty bag pockets are valid");

        for field in [
            "items",
            "pc_items",
            "balls",
            "key_items",
            "tm_hm",
            "custom_pockets",
        ] {
            let mut missing = complete.clone();
            missing.as_object_mut().expect("bag object").remove(field);
            let error = serde_json::from_value::<Bag>(missing)
                .expect_err("missing bag pocket must not default to empty")
                .to_string();
            assert!(
                error.contains(&format!("missing field `{field}`")),
                "{error}"
            );
        }
    }
}
