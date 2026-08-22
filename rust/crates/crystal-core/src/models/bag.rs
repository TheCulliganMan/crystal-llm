use std::{collections::BTreeMap, ops::Index};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use super::item::{
    ITEM_POCKET_BALL, ITEM_POCKET_ITEM, ITEM_POCKET_KEY_ITEM, ITEM_POCKET_TM_HM, Item,
};

pub const MAX_ITEM_STACK: u16 = 99;
pub const ITEM_POCKET_CAPACITY: usize = 20;
pub const BALL_POCKET_CAPACITY: usize = 12;
pub const KEY_ITEM_POCKET_CAPACITY: usize = 25;
pub const PC_ITEM_CAPACITY: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PocketStack {
    pub item_id: String,
    pub quantity: u16,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PocketInventory(pub Vec<PocketStack>);

impl PocketInventory {
    pub fn get(&self, item_id: &str) -> Option<&u16> {
        self.0
            .iter()
            .find(|stack| stack.item_id == item_id)
            .map(|stack| &stack.quantity)
    }

    pub fn insert(&mut self, item_id: String, quantity: u16) -> Option<u16> {
        if let Some(stack) = self.0.iter_mut().find(|stack| stack.item_id == item_id) {
            return Some(std::mem::replace(&mut stack.quantity, quantity));
        }
        self.0.push(PocketStack { item_id, quantity });
        None
    }

    pub fn remove(&mut self, item_id: &str) -> Option<u16> {
        let index = self.0.iter().position(|stack| stack.item_id == item_id)?;
        Some(self.0.remove(index).quantity)
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &u16)> {
        self.0.iter().map(|stack| (&stack.item_id, &stack.quantity))
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.iter().map(|stack| &stack.item_id)
    }

    pub fn values(&self) -> impl Iterator<Item = &u16> {
        self.0.iter().map(|stack| &stack.quantity)
    }

    pub fn stacks(&self) -> &[PocketStack] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a PocketInventory {
    type Item = (&'a String, &'a u16);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, PocketStack>,
        fn(&'a PocketStack) -> (&'a String, &'a u16),
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn pair(stack: &PocketStack) -> (&String, &u16) {
            (&stack.item_id, &stack.quantity)
        }
        self.0.iter().map(pair)
    }
}

impl Index<&str> for PocketInventory {
    type Output = u16;

    fn index(&self, item_id: &str) -> &Self::Output {
        self.get(item_id)
            .unwrap_or_else(|| panic!("no pocket stack for item {item_id}"))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bag {
    pub items: PocketInventory,
    pub pc_items: PocketInventory,
    pub balls: PocketInventory,
    pub key_items: PocketInventory,
    /// TM/HM quantities, indexed by the compiled `tmhm_index`.
    pub tm_hm: Vec<u8>,
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
            items: PocketInventory,
            pc_items: PocketInventory,
            balls: PocketInventory,
            key_items: PocketInventory,
            tm_hm: Vec<u8>,
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
    pub fn switch_item_stacks(
        &mut self,
        pocket: &str,
        source_index: usize,
        target_index: usize,
    ) -> Result<usize, String> {
        match pocket {
            ITEM_POCKET_ITEM => {
                switch_inventory_stacks(&mut self.items, source_index, target_index, MAX_ITEM_STACK)
            }
            ITEM_POCKET_BALL => {
                switch_inventory_stacks(&mut self.balls, source_index, target_index, MAX_ITEM_STACK)
            }
            ITEM_POCKET_KEY_ITEM => {
                switch_inventory_stacks(&mut self.key_items, source_index, target_index, 1)
            }
            _ => Err(format!("item switching is unavailable for pocket {pocket}")),
        }
    }

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
                1,
                1,
                Some(KEY_ITEM_POCKET_CAPACITY),
            ),
            ITEM_POCKET_TM_HM => self.add_tmhm(definition, quantity),
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
                remove_from_inventory(&mut self.key_items, &definition.script_name, 1)
            }
            ITEM_POCKET_TM_HM => self.remove_tmhm(definition, quantity),
            other => remove_from_custom_pocket(
                &mut self.custom_pockets,
                other,
                &definition.script_name,
                quantity,
            ),
        }
    }

    pub fn remove_item_at(
        &mut self,
        definition: &Item,
        stack_index: usize,
        quantity: u16,
    ) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        match definition.pocket.as_str() {
            ITEM_POCKET_ITEM => remove_from_inventory_at(
                &mut self.items,
                &definition.script_name,
                stack_index,
                quantity,
            ),
            ITEM_POCKET_BALL => remove_from_inventory_at(
                &mut self.balls,
                &definition.script_name,
                stack_index,
                quantity,
            ),
            ITEM_POCKET_KEY_ITEM => remove_from_inventory_at(
                &mut self.key_items,
                &definition.script_name,
                stack_index,
                1,
            ),
            _ => Err(format!(
                "indexed removal is unavailable for pocket {}",
                definition.pocket
            )),
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
        remove_from_inventory(&mut self.pc_items, &definition.script_name, quantity)
    }

    pub fn remove_pc_item_at(
        &mut self,
        definition: &Item,
        stack_index: usize,
        quantity: u16,
    ) -> Result<bool, String> {
        if quantity == 0 {
            return Err("quantity must be positive".to_string());
        }
        remove_from_inventory_at(
            &mut self.pc_items,
            &definition.script_name,
            stack_index,
            quantity,
        )
    }

    pub fn quantity(&self, definition: &Item) -> u16 {
        match definition.pocket.as_str() {
            ITEM_POCKET_ITEM => inventory_quantity(&self.items, &definition.script_name),
            ITEM_POCKET_BALL => inventory_quantity(&self.balls, &definition.script_name),
            ITEM_POCKET_KEY_ITEM => inventory_quantity(&self.key_items, &definition.script_name),
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
        inventory_quantity(&self.pc_items, &definition.script_name)
    }

    pub fn consume_ball(&mut self, definition: &Item) -> Result<bool, String> {
        self.remove_item(definition, 1)
    }

    fn add_tmhm(&mut self, definition: &Item, quantity: u16) -> Result<bool, String> {
        let Some(index) = definition.tmhm_index else {
            return Err(format!(
                "TM/HM item id '{}' is not indexed",
                definition.script_name
            ));
        };
        if self.tm_hm.len() <= index {
            self.tm_hm.resize(index + 1, 0);
        }
        let Ok(quantity) = u8::try_from(quantity) else {
            return Ok(false);
        };
        let Some(next) = self.tm_hm[index].checked_add(quantity) else {
            return Ok(false);
        };
        if next > MAX_ITEM_STACK as u8 {
            return Ok(false);
        }
        self.tm_hm[index] = next;
        Ok(true)
    }

    fn remove_tmhm(&mut self, definition: &Item, amount: u16) -> Result<bool, String> {
        let Some(index) = definition.tmhm_index else {
            return Err(format!(
                "TM/HM item id '{}' is not indexed",
                definition.script_name
            ));
        };
        let Some(quantity) = self.tm_hm.get_mut(index) else {
            return Ok(false);
        };
        let Ok(amount) = u8::try_from(amount) else {
            return Ok(false);
        };
        if amount == 0 || *quantity < amount {
            return Ok(false);
        }
        *quantity -= amount;
        Ok(true)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_inventory(&self.items, MAX_ITEM_STACK, ITEM_POCKET_CAPACITY, "items")?;
        validate_inventory(&self.pc_items, MAX_ITEM_STACK, PC_ITEM_CAPACITY, "pc_items")?;
        validate_inventory(&self.balls, MAX_ITEM_STACK, BALL_POCKET_CAPACITY, "balls")?;
        validate_inventory(&self.key_items, 1, KEY_ITEM_POCKET_CAPACITY, "key_items")?;
        validate_custom_pockets(&self.custom_pockets)?;
        if let Some((index, quantity)) = self
            .tm_hm
            .iter()
            .enumerate()
            .find(|(_, quantity)| **quantity > MAX_ITEM_STACK as u8)
        {
            return Err(format!(
                "tm_hm[{index}] quantity {quantity} exceeds maximum {MAX_ITEM_STACK}"
            ));
        }
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
    add_to_map_inventory(inventory, item_id, quantity, MAX_ITEM_STACK)
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
    remove_from_map_inventory(inventory, item_id, quantity)
}

pub fn validate_saved_bag_pocket_references<'a>(
    items: &BTreeMap<String, Item>,
    path: &str,
    inventory: impl IntoIterator<Item = (&'a String, &'a u16)>,
    expected_pocket: &str,
) -> Result<(), BagSaveError> {
    for (item_id, _) in inventory {
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

pub fn validate_saved_pc_item_references<'a>(
    items: &BTreeMap<String, Item>,
    path: &str,
    inventory: impl IntoIterator<Item = (&'a String, &'a u16)>,
) -> Result<(), BagSaveError> {
    for (item_id, _) in inventory {
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
    }
    Ok(())
}

fn add_to_inventory(
    inventory: &mut PocketInventory,
    item_id: &str,
    quantity: u16,
    stack_limit: u16,
    capacity: Option<usize>,
) -> Result<bool, String> {
    validate_item_id(item_id)?;
    let capacity = capacity.unwrap_or(usize::MAX);
    let matching_space = inventory
        .0
        .iter()
        .filter(|stack| stack.item_id == item_id)
        .map(|stack| stack_limit.saturating_sub(stack.quantity))
        .sum::<u16>();
    let empty_slots = capacity.saturating_sub(inventory.len());
    let empty_space = u32::try_from(empty_slots)
        .unwrap_or(u32::MAX)
        .saturating_mul(u32::from(stack_limit));
    if u32::from(quantity) > u32::from(matching_space).saturating_add(empty_space) {
        return Ok(false);
    }

    let mut remaining = quantity;
    for stack in inventory
        .0
        .iter_mut()
        .filter(|stack| stack.item_id == item_id)
    {
        let added = remaining.min(stack_limit - stack.quantity);
        stack.quantity += added;
        remaining -= added;
        if remaining == 0 {
            return Ok(true);
        }
    }

    while remaining > 0 {
        let added = remaining.min(stack_limit);
        inventory.0.push(PocketStack {
            item_id: item_id.to_string(),
            quantity: added,
        });
        remaining -= added;
    }
    Ok(true)
}

fn remove_from_inventory(
    inventory: &mut PocketInventory,
    item_id: &str,
    quantity: u16,
) -> Result<bool, String> {
    validate_item_id(item_id)?;
    let Some(index) = inventory
        .0
        .iter()
        .position(|stack| stack.item_id == item_id)
    else {
        return Ok(false);
    };
    if inventory.0[index].quantity < quantity {
        return Ok(false);
    }
    let next = inventory.0[index].quantity - quantity;
    if next == 0 {
        inventory.0.remove(index);
    } else {
        inventory.0[index].quantity = next;
    }
    Ok(true)
}

fn remove_from_inventory_at(
    inventory: &mut PocketInventory,
    item_id: &str,
    stack_index: usize,
    quantity: u16,
) -> Result<bool, String> {
    validate_item_id(item_id)?;
    let Some(stack) = inventory.0.get(stack_index) else {
        return Ok(false);
    };
    if stack.item_id != item_id || stack.quantity < quantity {
        return Ok(false);
    }
    let next = stack.quantity - quantity;
    if next == 0 {
        inventory.0.remove(stack_index);
    } else {
        inventory.0[stack_index].quantity = next;
    }
    Ok(true)
}

fn inventory_quantity(inventory: &PocketInventory, item_id: &str) -> u16 {
    inventory
        .0
        .iter()
        .filter(|stack| stack.item_id == item_id)
        .map(|stack| stack.quantity)
        .sum()
}

fn switch_inventory_stacks(
    inventory: &mut PocketInventory,
    source_index: usize,
    target_index: usize,
    stack_limit: u16,
) -> Result<usize, String> {
    if source_index >= inventory.len() || target_index >= inventory.len() {
        return Err(format!(
            "item switch indices {source_index}->{target_index} are outside pocket length {}",
            inventory.len()
        ));
    }
    if source_index == target_index {
        return Ok(target_index);
    }

    let source = &inventory.0[source_index];
    let target = &inventory.0[target_index];
    if source.item_id == target.item_id
        && source.quantity < stack_limit
        && target.quantity < stack_limit
    {
        let total = source.quantity + target.quantity;
        if total <= stack_limit {
            inventory.0[target_index].quantity = total;
            inventory.0.remove(source_index);
            return Ok(if source_index < target_index {
                target_index - 1
            } else {
                target_index
            });
        }
        inventory.0[target_index].quantity = stack_limit;
        inventory.0[source_index].quantity = total - stack_limit;
        return Ok(target_index);
    }

    let stack = inventory.0.remove(source_index);
    inventory.0.insert(target_index, stack);
    Ok(target_index)
}

fn add_to_map_inventory(
    inventory: &mut BTreeMap<String, u16>,
    item_id: &str,
    quantity: u16,
    stack_limit: u16,
) -> Result<bool, String> {
    validate_item_id(item_id)?;
    let current = inventory.get(item_id).copied().unwrap_or(0);
    let Some(next) = current.checked_add(quantity) else {
        return Ok(false);
    };
    if next > stack_limit {
        return Ok(false);
    }
    inventory.insert(item_id.to_string(), next);
    Ok(true)
}

fn remove_from_map_inventory(
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
    inventory: &PocketInventory,
    stack_limit: u16,
    capacity: usize,
    label: &str,
) -> Result<(), String> {
    let active = inventory.len();
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
        if *quantity == 0 || *quantity > stack_limit {
            return Err(format!(
                "{label}.{item_id} quantity {quantity} is outside stack range 1..={stack_limit}"
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
        validate_map_inventory(
            inventory,
            MAX_ITEM_STACK,
            &format!("custom_pockets.{pocket_id}"),
        )?;
    }
    Ok(())
}

fn validate_map_inventory(
    inventory: &BTreeMap<String, u16>,
    stack_limit: u16,
    label: &str,
) -> Result<(), String> {
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
        if *quantity == 0 || *quantity > stack_limit {
            return Err(format!(
                "{label}.{item_id} quantity {quantity} is outside stack range 1..={stack_limit}"
            ));
        }
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
    fn key_items_append_distinct_slots_and_tmhm_quantities_are_exact() {
        let bicycle = item("BICYCLE", item_pocket("KEY_ITEM"));
        let mut tm_mud_slap = item("TM_MUD_SLAP", item_pocket("TM_HM"));
        tm_mud_slap.tmhm_index = Some(30);
        let mut bag = Bag::default();

        assert!(bag.add_item(&bicycle, 1).expect("add key item"));
        assert!(
            bag.add_item(&bicycle, 1)
                .expect("ASM ReceiveKeyItem appends duplicate key-item slots")
        );
        assert_eq!(bag.quantity(&bicycle), 2);
        assert!(
            bag.add_item(&bicycle, 99)
                .expect("key item quantity byte is ignored")
        );
        assert_eq!(bag.quantity(&bicycle), 3);
        assert!(
            bag.remove_item(&bicycle, 99)
                .expect("key item toss removes one entry")
        );
        assert_eq!(bag.quantity(&bicycle), 2);
        assert!(bag.add_item(&tm_mud_slap, 1).expect("add tm"));
        assert_eq!(bag.quantity(&tm_mud_slap), 1);
        assert!(
            bag.add_item(&tm_mud_slap, 1)
                .expect("tm quantity increments")
        );
        assert_eq!(bag.quantity(&tm_mud_slap), 2);
        assert!(
            !bag.add_item(&tm_mud_slap, u16::from(MAX_ITEM_STACK - 1))
                .expect("an overflowing TM quantity is rejected atomically")
        );
        assert_eq!(bag.quantity(&tm_mud_slap), 2);
        assert!(
            bag.add_item(&tm_mud_slap, u16::from(MAX_ITEM_STACK - 2))
                .expect("an exact TM stack fill succeeds")
        );
        assert_eq!(bag.quantity(&tm_mud_slap), u16::from(MAX_ITEM_STACK));
        assert!(!bag.add_item(&tm_mud_slap, 1).expect("TM stack is full"));
        assert!(bag.remove_item(&tm_mud_slap, 1).expect("remove tm"));
        assert_eq!(bag.quantity(&tm_mud_slap), u16::from(MAX_ITEM_STACK - 1));
        assert!(
            !bag.remove_item(&tm_mud_slap, u16::from(MAX_ITEM_STACK))
                .expect("an oversized TM removal is rejected atomically")
        );
        assert_eq!(bag.quantity(&tm_mud_slap), u16::from(MAX_ITEM_STACK - 1));
    }

    #[test]
    fn item_pockets_preserve_order_and_append_duplicate_stacks_like_asm() {
        let potion = item("POTION", item_pocket("ITEM"));
        let antidote = item("ANTIDOTE", item_pocket("ITEM"));
        let mut bag = Bag::default();

        assert!(bag.add_item(&potion, 90).expect("add first stack"));
        assert!(bag.add_item(&antidote, 1).expect("append another item"));
        assert!(bag.add_item(&potion, 20).expect("fill and append potion"));
        assert_eq!(
            bag.items.stacks(),
            [
                PocketStack {
                    item_id: "POTION".to_string(),
                    quantity: 99,
                },
                PocketStack {
                    item_id: "ANTIDOTE".to_string(),
                    quantity: 1,
                },
                PocketStack {
                    item_id: "POTION".to_string(),
                    quantity: 11,
                },
            ]
        );
        assert_eq!(bag.quantity(&potion), 110);

        assert!(
            bag.remove_item(&potion, 50)
                .expect("remove from first stack")
        );
        assert_eq!(bag.items.stacks()[0].quantity, 49);
        assert_eq!(bag.items.stacks()[2].quantity, 11);
        assert!(
            !bag.remove_item(&potion, 50)
                .expect("removal cannot spill across duplicate stacks")
        );
        assert_eq!(bag.quantity(&potion), 60);
        assert!(
            bag.remove_item_at(&potion, 2, 10)
                .expect("cursor-addressed removal selects the duplicate stack")
        );
        assert_eq!(bag.items.stacks()[2].quantity, 1);
    }

    #[test]
    fn switch_items_reorders_and_combines_stacks_like_asm() {
        let potion = item("POTION", item_pocket("ITEM"));
        let antidote = item("ANTIDOTE", item_pocket("ITEM"));
        let mut bag = Bag::default();
        assert!(bag.add_item(&potion, 99).unwrap());
        assert!(bag.add_item(&antidote, 1).unwrap());
        assert!(bag.add_item(&potion, 20).unwrap());

        assert_eq!(bag.switch_item_stacks(ITEM_POCKET_ITEM, 1, 0).unwrap(), 0);
        assert_eq!(bag.items.stacks()[0].item_id, "ANTIDOTE");
        bag.items.0[1].quantity = 80;
        assert_eq!(bag.switch_item_stacks(ITEM_POCKET_ITEM, 2, 1).unwrap(), 1);
        assert_eq!(bag.items.stacks()[1].quantity, 99);
        assert_eq!(bag.items.stacks()[2].quantity, 1);
        bag.items.0[1].quantity = 50;
        assert_eq!(bag.switch_item_stacks(ITEM_POCKET_ITEM, 2, 1).unwrap(), 1);
        assert_eq!(bag.items.stacks()[1].quantity, 51);
        assert_eq!(bag.items.len(), 2);
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
    fn pc_items_accept_ball_pocket_items_bought_by_mom() {
        let potion = item("POTION", item_pocket("ITEM"));
        let ball = item("POKE_BALL", item_pocket("BALL"));
        let mut bag = Bag::default();

        assert!(bag.add_pc_item(&potion, 2).expect("add pc item"));
        assert_eq!(bag.pc_item_quantity(&potion), 2);
        assert!(bag.has_pc_item(&potion));
        assert!(
            bag.add_pc_item(&ball, 1)
                .expect("Mom can put a ball in PC storage")
        );
        assert_eq!(bag.pc_item_quantity(&ball), 1);
        assert!(bag.remove_pc_item(&ball, 1).expect("withdraw Mom's ball"));
        bag.validate().expect("valid pc items");
    }

    #[test]
    fn bag_json_rejects_unknown_inventory_fields_without_legacy_fallbacks() {
        let error = serde_json::from_value::<Bag>(serde_json::json!({
            "items": [],
            "pc_items": [],
            "balls": [],
            "key_items": [],
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
            "items": [],
            "pc_items": [],
            "balls": [],
            "key_items": [],
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
