use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemPocket {
    Item,
    Ball,
    KeyItem,
    TmHm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub effect: String,
    pub price: u16,
    pub held_effect: String,
    pub parameter: i16,
    pub property: String,
    pub pocket: ItemPocket,
    pub field_menu: String,
    pub battle_menu: String,
    pub script_name: String,
    pub consumable: bool,
    #[serde(deserialize_with = "required_nullable_usize")]
    pub tmhm_index: Option<usize>,
}

fn required_nullable_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<usize>::deserialize(deserializer)
}

fn default_item_effect() -> String {
    "NONE".to_string()
}

impl Item {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            script_name: name.clone(),
            name,
            description: String::new(),
            effect: default_item_effect(),
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: ItemPocket::Item,
            field_menu: String::new(),
            battle_menu: String::new(),
            tmhm_index: None,
            consumable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_effect_ids_are_modpack_owned_strings_not_core_enums() {
        let item: Item = serde_json::from_str(
            r#"{
              "name":"Flash Step Charm",
              "description":"A modded effect item.",
              "effect":"MODDED_FLASH_STEP",
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "battle_menu":"",
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null
            }"#,
        )
        .expect("modded item effect ids are data, not schema errors");
        assert_eq!(item.effect, "MODDED_FLASH_STEP");

        let serialized = serde_json::to_value(&item).expect("serialize item");
        assert_eq!(serialized["effect"], "MODDED_FLASH_STEP");
    }

    #[test]
    fn serialized_items_require_explicit_effect_id() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "battle_menu":"",
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null
            }"#,
        )
        .expect_err("missing item effect must not default to NONE")
        .to_string();

        assert!(error.contains("missing field `effect`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_script_name() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "battle_menu":"",
              "consumable":true,
              "tmhm_index":null
            }"#,
        )
        .expect_err("missing item script_name must not default from display name")
        .to_string();

        assert!(error.contains("missing field `script_name`"), "{error}");
    }

    #[test]
    fn item_defaults_match_schema_defaults() {
        let item = Item::new("poke ball");
        assert_eq!(item.description, "");
        assert_eq!(item.effect, "NONE");
        assert_eq!(item.price, 0);
        assert_eq!(item.held_effect, "HELD_NONE");
        assert_eq!(item.parameter, 0);
        assert_eq!(item.pocket, ItemPocket::Item);
        assert_eq!(item.script_name, "poke ball");
        assert!(!item.consumable);
        assert_eq!(item.tmhm_index, None);
    }

    #[test]
    fn tmhm_index_is_explicit_modpack_data_not_name_parsing() {
        let item: Item = serde_json::from_str(
            r#"{
              "name":"TM Mud-Slap",
              "description":"Teaches Mud-Slap.",
              "effect":"NONE",
              "price":3000,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"CANT_SELECT",
              "pocket":"TM_HM",
              "field_menu":"ITEMMENU_PARTY",
              "battle_menu":"ITEMMENU_NOUSE",
              "script_name":"TM_MUD_SLAP",
              "consumable":true,
              "tmhm_index":30
            }"#,
        )
        .expect("parse explicit symbolic TM");

        assert_eq!(item.script_name, "TM_MUD_SLAP");
        assert_eq!(item.tmhm_index, Some(30));
    }

    #[test]
    fn serialized_items_require_explicit_nullable_tmhm_index() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"FLASH STEP CHARM",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "battle_menu":"",
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true
            }"#,
        )
        .expect_err("missing tmhm_index must not deserialize as None")
        .to_string();

        assert!(error.contains("missing field `tmhm_index`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_consumable_flag() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "battle_menu":"",
              "script_name":"FLASH_STEP_CHARM",
              "tmhm_index":null
            }"#,
        )
        .expect_err("missing consumable must not deserialize as a default")
        .to_string();

        assert!(error.contains("missing field `consumable`"), "{error}");
    }

    #[test]
    fn serialized_items_reject_unknown_modpack_fields() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A modded effect item.",
              "effect":"MODDED_FLASH_STEP",
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "battle_menu":"",
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "effect_enum":"NONE"
            }"#,
        )
        .expect_err("legacy effect enum fields must not be accepted")
        .to_string();

        assert!(error.contains("unknown field `effect_enum`"), "{error}");
    }
}
