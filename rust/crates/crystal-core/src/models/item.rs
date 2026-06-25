use serde::{Deserialize, Serialize};

pub type ItemPocket = String;

pub const ITEM_POCKET_ITEM: &str = "ITEM";
pub const ITEM_POCKET_BALL: &str = "BALL";
pub const ITEM_POCKET_KEY_ITEM: &str = "KEY_ITEM";
pub const ITEM_POCKET_TM_HM: &str = "TM_HM";

pub fn item_pocket(id: &str) -> ItemPocket {
    id.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub effect: String,
    pub status_heals: Vec<String>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub revive_hp_percent: Option<u8>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub party_revive_hp_percent: Option<u8>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub pp_restore_scope: Option<String>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub pp_restore_points: Option<u8>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub pp_up_stages: Option<u8>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub vitamin_stat: Option<String>,
    #[serde(deserialize_with = "required_nullable_u16")]
    pub vitamin_stat_exp: Option<u16>,
    #[serde(deserialize_with = "required_nullable_u16")]
    pub vitamin_max_stat_exp: Option<u16>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub rare_candy_level_gain: Option<u8>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub battle_stat_boost_stat: Option<String>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub battle_stat_boost_stages: Option<u8>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub battle_escape_mode: Option<String>,
    #[serde(deserialize_with = "required_nullable_bool")]
    pub battle_focus_energy: Option<bool>,
    #[serde(deserialize_with = "required_nullable_bool")]
    pub battle_stat_drop_guard: Option<bool>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub battle_stat_drop_guard_turns: Option<u8>,
    #[serde(deserialize_with = "required_nullable_bool")]
    pub confusion_heal: Option<bool>,
    #[serde(deserialize_with = "required_nullable_u16")]
    pub repel_steps: Option<u16>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub escape_rope_mode: Option<String>,
    pub price: u16,
    pub held_effect: String,
    pub parameter: i16,
    pub property: String,
    pub pocket: ItemPocket,
    pub field_menu: String,
    pub field_usable: bool,
    pub battle_menu: String,
    pub battle_usable: bool,
    pub script_name: String,
    pub consumable: bool,
    #[serde(deserialize_with = "required_nullable_usize")]
    pub tmhm_index: Option<usize>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub tmhm_move: Option<String>,
}

fn required_nullable_u8<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<u8>::deserialize(deserializer)
}

fn required_nullable_u16<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<u16>::deserialize(deserializer)
}

fn required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

fn required_nullable_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer)
}

fn required_nullable_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<usize>::deserialize(deserializer)
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
		              "status_heals":[],
		              "revive_hp_percent":null,
		              "party_revive_hp_percent":null,
		              "pp_restore_scope":null,
		              "pp_restore_points":null,
		              "pp_up_stages":null,
		              "vitamin_stat":null,
		              "vitamin_stat_exp":null,
		              "vitamin_max_stat_exp":null,
		              "rare_candy_level_gain":null,
		              "battle_stat_boost_stat":null,
		              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
			              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect("modded item effect ids are data, not schema errors");
        assert_eq!(item.effect, "MODDED_FLASH_STEP");
        assert!(item.status_heals.is_empty());
        assert_eq!(item.revive_hp_percent, None);
        assert_eq!(item.party_revive_hp_percent, None);
        assert_eq!(item.pp_restore_scope, None);
        assert_eq!(item.pp_restore_points, None);
        assert_eq!(item.pp_up_stages, None);
        assert_eq!(item.vitamin_stat, None);
        assert_eq!(item.vitamin_stat_exp, None);
        assert_eq!(item.vitamin_max_stat_exp, None);
        assert_eq!(item.rare_candy_level_gain, None);
        assert_eq!(item.battle_stat_boost_stat, None);
        assert_eq!(item.battle_stat_boost_stages, None);
        assert_eq!(item.battle_stat_drop_guard, None);

        let serialized = serde_json::to_value(&item).expect("serialize item");
        assert_eq!(serialized["effect"], "MODDED_FLASH_STEP");
        assert_eq!(serialized["status_heals"], serde_json::json!([]));
        assert_eq!(serialized["revive_hp_percent"], serde_json::json!(null));
        assert_eq!(
            serialized["party_revive_hp_percent"],
            serde_json::json!(null)
        );
        assert_eq!(serialized["pp_restore_scope"], serde_json::json!(null));
        assert_eq!(serialized["pp_restore_points"], serde_json::json!(null));
        assert_eq!(serialized["pp_up_stages"], serde_json::json!(null));
        assert_eq!(serialized["vitamin_stat"], serde_json::json!(null));
        assert_eq!(serialized["vitamin_stat_exp"], serde_json::json!(null));
        assert_eq!(serialized["vitamin_max_stat_exp"], serde_json::json!(null));
        assert_eq!(serialized["rare_candy_level_gain"], serde_json::json!(null));
        assert_eq!(
            serialized["battle_stat_boost_stat"],
            serde_json::json!(null)
        );
        assert_eq!(
            serialized["battle_stat_boost_stages"],
            serde_json::json!(null)
        );
        assert_eq!(
            serialized["battle_stat_drop_guard"],
            serde_json::json!(null)
        );
    }

    #[test]
    fn item_pocket_ids_are_modpack_owned_strings_not_core_enums() {
        let item: Item = serde_json::from_str(
            r#"{
              "name":"Battle Pass",
              "description":"A modded pocket item.",
              "effect":"NONE",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "rare_candy_level_gain":null,
              "battle_stat_boost_stat":null,
              "battle_stat_boost_stages":null,
              "battle_escape_mode":null,
              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
              "battle_stat_drop_guard_turns":null,
              "confusion_heal":null,
              "repel_steps":null,
              "escape_rope_mode":null,
              "price":0,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"BATTLE_PASS",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"BATTLE_PASS",
              "consumable":false,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect("modded item pocket ids are exact data");

        assert_eq!(item.pocket, item_pocket("BATTLE_PASS"));
        let serialized = serde_json::to_value(&item).expect("serialize item");
        assert_eq!(serialized["pocket"], serde_json::json!("BATTLE_PASS"));
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
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing item effect must not default to NONE")
        .to_string();

        assert!(error.contains("missing field `effect`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_status_heals() {
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
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing status_heals must not default to an empty list")
        .to_string();

        assert!(error.contains("missing field `status_heals`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_revive_hp_percent() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing revive_hp_percent must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `revive_hp_percent`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_pp_restore_scope() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "rare_candy_level_gain":null,
              "battle_stat_boost_stat":null,
              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing pp_restore_scope must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `pp_restore_scope`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_party_revive_hp_percent() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "rare_candy_level_gain":null,
              "battle_stat_boost_stat":null,
              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing party_revive_hp_percent must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `party_revive_hp_percent`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_pp_restore_points() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing pp_restore_points must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `pp_restore_points`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_pp_up_stages() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing pp_up_stages must not deserialize as None")
        .to_string();

        assert!(error.contains("missing field `pp_up_stages`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_script_name() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
	              "description":"A malformed modded item.",
	              "effect":"MODDED_FLASH_STEP",
	              "status_heals":[],
	              "revive_hp_percent":null,
		              "party_revive_hp_percent":null,
		              "pp_restore_scope":null,
		              "pp_restore_points":null,
		              "pp_up_stages":null,
		              "vitamin_stat":null,
		              "vitamin_stat_exp":null,
		              "vitamin_max_stat_exp":null,
		              "rare_candy_level_gain":null,
		              "battle_stat_boost_stat":null,
		              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
		              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing item script_name must not default from display name")
        .to_string();

        assert!(error.contains("missing field `script_name`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_vitamin_stat() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing vitamin_stat must not deserialize as None")
        .to_string();

        assert!(error.contains("missing field `vitamin_stat`"), "{error}");
    }

    #[test]
    fn serialized_items_require_explicit_vitamin_stat_exp() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_max_stat_exp":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing vitamin_stat_exp must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `vitamin_stat_exp`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_vitamin_max_stat_exp() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing vitamin_max_stat_exp must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `vitamin_max_stat_exp`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_rare_candy_level_gain() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing rare_candy_level_gain must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `rare_candy_level_gain`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_battle_stat_boost_stat() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "rare_candy_level_gain":null,
              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing battle_stat_boost_stat must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `battle_stat_boost_stat`"),
            "{error}"
        );
    }

    #[test]
    fn serialized_items_require_explicit_battle_stat_boost_stages() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"Flash Step Charm",
              "description":"A malformed modded item.",
              "effect":"MODDED_FLASH_STEP",
              "status_heals":[],
              "revive_hp_percent":null,
              "party_revive_hp_percent":null,
              "pp_restore_scope":null,
              "pp_restore_points":null,
              "pp_up_stages":null,
              "vitamin_stat":null,
              "vitamin_stat_exp":null,
              "vitamin_max_stat_exp":null,
              "rare_candy_level_gain":null,
              "battle_stat_boost_stat":null,
              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null
            }"#,
        )
        .expect_err("missing battle_stat_boost_stages must not deserialize as None")
        .to_string();

        assert!(
            error.contains("missing field `battle_stat_boost_stages`"),
            "{error}"
        );
    }

    #[test]
    fn tmhm_data_is_explicit_modpack_data_not_name_parsing() {
        let item: Item = serde_json::from_str(
            r#"{
              "name":"TM Mud-Slap",
	              "description":"Teaches Mud-Slap.",
	              "effect":"NONE",
	              "status_heals":[],
	              "revive_hp_percent":null,
		              "party_revive_hp_percent":null,
		              "pp_restore_scope":null,
		              "pp_restore_points":null,
		              "pp_up_stages":null,
		              "vitamin_stat":null,
		              "vitamin_stat_exp":null,
		              "vitamin_max_stat_exp":null,
		              "rare_candy_level_gain":null,
		              "battle_stat_boost_stat":null,
		              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
		              "price":3000,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"CANT_SELECT",
              "pocket":"TM_HM",
              "field_menu":"ITEMMENU_PARTY",
              "field_usable":true,
              "battle_menu":"ITEMMENU_NOUSE",
              "battle_usable":false,
              "script_name":"TM_MUD_SLAP",
              "consumable":true,
              "tmhm_index":30,
              "tmhm_move":"MUD_SLAP"
            }"#,
        )
        .expect("parse explicit symbolic TM");

        assert_eq!(item.script_name, "TM_MUD_SLAP");
        assert_eq!(item.tmhm_index, Some(30));
        assert_eq!(item.tmhm_move.as_deref(), Some("MUD_SLAP"));
    }

    #[test]
    fn serialized_items_require_explicit_nullable_tmhm_index() {
        let error = serde_json::from_str::<Item>(
            r#"{
              "name":"FLASH STEP CHARM",
	              "description":"A malformed modded item.",
	              "effect":"MODDED_FLASH_STEP",
	              "status_heals":[],
	              "revive_hp_percent":null,
		              "party_revive_hp_percent":null,
		              "pp_restore_scope":null,
		              "pp_restore_points":null,
		              "pp_up_stages":null,
		              "vitamin_stat":null,
		              "vitamin_stat_exp":null,
		              "vitamin_max_stat_exp":null,
		              "rare_candy_level_gain":null,
		              "battle_stat_boost_stat":null,
		              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
		              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
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
	              "status_heals":[],
	              "revive_hp_percent":null,
		              "party_revive_hp_percent":null,
		              "pp_restore_scope":null,
		              "pp_restore_points":null,
		              "pp_up_stages":null,
		              "vitamin_stat":null,
		              "vitamin_stat_exp":null,
		              "vitamin_max_stat_exp":null,
		              "rare_candy_level_gain":null,
		              "battle_stat_boost_stat":null,
		              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
		              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "tmhm_index":null,
              "tmhm_move":null
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
	              "status_heals":[],
	              "revive_hp_percent":null,
		              "party_revive_hp_percent":null,
		              "pp_restore_scope":null,
		              "pp_restore_points":null,
		              "pp_up_stages":null,
		              "vitamin_stat":null,
		              "vitamin_stat_exp":null,
		              "vitamin_max_stat_exp":null,
		              "rare_candy_level_gain":null,
		              "battle_stat_boost_stat":null,
		              "battle_stat_boost_stages":null,
		              "battle_escape_mode":null,
		              "battle_focus_energy":null,
              "battle_stat_drop_guard":null,
		              "battle_stat_drop_guard_turns":null,
		              "confusion_heal":null,
		              "repel_steps":null,
		              "escape_rope_mode":null,
		              "price":100,
              "held_effect":"HELD_NONE",
              "parameter":0,
              "property":"",
              "pocket":"ITEM",
              "field_menu":"",
              "field_usable":true,
              "battle_menu":"",
              "battle_usable":true,
              "script_name":"FLASH_STEP_CHARM",
              "consumable":true,
              "tmhm_index":null,
              "tmhm_move":null,
              "effect_enum":"NONE"
            }"#,
        )
        .expect_err("legacy effect enum fields must not be accepted")
        .to_string();

        assert!(error.contains("unknown field `effect_enum`"), "{error}");
    }
}
