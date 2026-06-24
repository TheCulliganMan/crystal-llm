use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::item::{Item, ItemPocket};

pub const MAX_ITEM_STACK: u16 = 99;
pub const ITEM_POCKET_CAPACITY: usize = 20;
pub const BALL_POCKET_CAPACITY: usize = 12;
pub const KEY_ITEM_POCKET_CAPACITY: usize = 25;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bag {
    pub items: BTreeMap<String, u16>,
    pub balls: BTreeMap<String, u16>,
    pub key_items: BTreeMap<String, u16>,
    pub tm_hm: Vec<bool>,
}

impl Bag {
    pub fn add_item(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        match definition.pocket {
            ItemPocket::Item => add_to_inventory(
                &mut self.items,
                &definition.script_name,
                quantity,
                MAX_ITEM_STACK,
                Some(ITEM_POCKET_CAPACITY),
            ),
            ItemPocket::Ball => add_to_inventory(
                &mut self.balls,
                &definition.script_name,
                quantity,
                MAX_ITEM_STACK,
                Some(BALL_POCKET_CAPACITY),
            ),
            ItemPocket::KeyItem => add_to_inventory(
                &mut self.key_items,
                &definition.script_name,
                quantity,
                1,
                Some(KEY_ITEM_POCKET_CAPACITY),
            ),
            ItemPocket::TmHm => self.add_tmhm(definition),
        }
    }

    pub fn remove_item(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        match definition.pocket {
            ItemPocket::Item => Ok(remove_from_inventory(
                &mut self.items,
                &definition.script_name,
                quantity,
            )),
            ItemPocket::Ball => Ok(remove_from_inventory(
                &mut self.balls,
                &definition.script_name,
                quantity,
            )),
            ItemPocket::KeyItem => Ok(remove_from_inventory(
                &mut self.key_items,
                &definition.script_name,
                quantity,
            )),
            ItemPocket::TmHm => self.remove_tmhm(definition),
        }
    }

    pub fn has_item(&self, definition: &Item) -> bool {
        self.quantity(definition) > 0
    }

    pub fn quantity(&self, definition: &Item) -> u16 {
        match definition.pocket {
            ItemPocket::Item => self
                .items
                .get(&definition.script_name)
                .copied()
                .unwrap_or(0),
            ItemPocket::Ball => self
                .balls
                .get(&definition.script_name)
                .copied()
                .unwrap_or(0),
            ItemPocket::KeyItem => self
                .key_items
                .get(&definition.script_name)
                .copied()
                .unwrap_or(0),
            ItemPocket::TmHm => definition
                .tmhm_index
                .and_then(|index| self.tm_hm.get(index).copied())
                .map(u16::from)
                .unwrap_or(0),
        }
    }

    pub fn consume_ball(&mut self, definition: &Item) -> Result<bool, String> {
        if definition.pocket != ItemPocket::Ball {
            return Err(format!(
                "item '{}' is not in the BALL pocket",
                definition.script_name
            ));
        }
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
        validate_inventory(&self.balls, MAX_ITEM_STACK, BALL_POCKET_CAPACITY, "balls")?;
        validate_inventory(&self.key_items, 1, KEY_ITEM_POCKET_CAPACITY, "key_items")?;
        Ok(())
    }
}

fn add_to_inventory(
    inventory: &mut BTreeMap<String, u16>,
    item_id: &str,
    quantity: u16,
    stack_limit: u16,
    capacity: Option<usize>,
) -> Result<bool, String> {
    if item_id.trim().is_empty() {
        return Err("item id is required".to_string());
    }
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
) -> bool {
    let Some(current) = inventory.get(item_id).copied() else {
        return false;
    };
    if current < quantity {
        return false;
    }
    let next = current - quantity;
    if next == 0 {
        inventory.remove(item_id);
    } else {
        inventory.insert(item_id.to_string(), next);
    }
    true
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
        if item_id.trim().is_empty() {
            return Err(format!("{label} contains an empty item id"));
        }
        if *quantity > stack_limit {
            return Err(format!(
                "{label}.{item_id} quantity {quantity} exceeds stack limit {stack_limit}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn bag_adds_and_consumes_exact_ball_ids() {
        let poke_ball = item("POKE_BALL", ItemPocket::Ball);
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
    fn bag_uses_pocket_capacities_without_item_id_coercion() {
        let mut bag = Bag::default();
        for index in 0..BALL_POCKET_CAPACITY {
            let ball = item(&format!("MOD_BALL_{index}"), ItemPocket::Ball);
            assert!(bag.add_item(&ball, 1).expect("add ball"));
        }
        let extra = item("mod_ball_0", ItemPocket::Ball);
        assert!(!bag.add_item(&extra, 1).expect("ball pocket full"));
        assert_eq!(bag.quantity(&extra), 0);
        bag.validate().expect("valid bag");
    }

    #[test]
    fn key_items_do_not_stack_and_tmhm_flags_are_exact() {
        let bicycle = item("BICYCLE", ItemPocket::KeyItem);
        let mut tm_mud_slap = item("TM_MUD_SLAP", ItemPocket::TmHm);
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
        let tm = item("TM_MUD_SLAP", ItemPocket::TmHm);
        let mut bag = Bag::default();

        let error = bag.add_item(&tm, 1).expect_err("missing tmhm index");

        assert_eq!(error, "TM/HM item id 'TM_MUD_SLAP' is not indexed");
    }

    #[test]
    fn bag_json_rejects_unknown_inventory_fields_without_legacy_fallbacks() {
        let error = serde_json::from_value::<Bag>(serde_json::json!({
            "items": {},
            "balls": {},
            "key_items": {},
            "tm_hm": [],
            "legacy_pc_items": {}
        }))
        .expect_err("bag saves must not accept legacy inventory fields")
        .to_string();

        assert!(error.contains("unknown field `legacy_pc_items`"), "{error}");
    }
}
