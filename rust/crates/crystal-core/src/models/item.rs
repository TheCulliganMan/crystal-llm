use serde::{Deserialize, Deserializer, Serialize};

pub type ItemPocket = String;

pub const ITEM_POCKET_ITEM: &str = "ITEM";
pub const ITEM_POCKET_BALL: &str = "BALL";
pub const ITEM_POCKET_KEY_ITEM: &str = "KEY_ITEM";
pub const ITEM_POCKET_TM_HM: &str = "TM_HM";
pub const MAIL_ITEM_IDS: &[&str] = &[
    "FLOWER_MAIL",
    "SURF_MAIL",
    "LITEBLUEMAIL",
    "PORTRAITMAIL",
    "LOVELY_MAIL",
    "EON_MAIL",
    "MORPH_MAIL",
    "BLUESKY_MAIL",
    "MUSIC_MAIL",
    "MIRAGE_MAIL",
];

pub fn is_mail_item_id(item_id: &str) -> bool {
    MAIL_ITEM_IDS.contains(&item_id)
}

pub fn item_pocket(id: &str) -> ItemPocket {
    id.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub name: String,
    pub description: String,
    #[serde(deserialize_with = "required_item_token")]
    pub effect: String,
    #[serde(deserialize_with = "required_status_token_vec")]
    pub status_heals: Vec<String>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub revive_hp_percent: Option<u8>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub party_revive_hp_percent: Option<u8>,
    #[serde(deserialize_with = "required_nullable_item_token")]
    pub pp_restore_scope: Option<String>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub pp_restore_points: Option<u8>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub pp_up_stages: Option<u8>,
    #[serde(deserialize_with = "required_nullable_item_token")]
    pub vitamin_stat: Option<String>,
    #[serde(deserialize_with = "required_nullable_u16")]
    pub vitamin_stat_exp: Option<u16>,
    #[serde(deserialize_with = "required_nullable_u16")]
    pub vitamin_max_stat_exp: Option<u16>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub rare_candy_level_gain: Option<u8>,
    #[serde(deserialize_with = "required_nullable_item_token")]
    pub battle_stat_boost_stat: Option<String>,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub battle_stat_boost_stages: Option<u8>,
    #[serde(deserialize_with = "required_nullable_item_token")]
    pub battle_escape_mode: Option<String>,
    #[serde(deserialize_with = "required_nullable_bool")]
    pub battle_capture_ball: Option<bool>,
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
    #[serde(deserialize_with = "required_nullable_item_token")]
    pub escape_rope_mode: Option<String>,
    pub price: u16,
    #[serde(deserialize_with = "required_item_token")]
    pub held_effect: String,
    pub parameter: i16,
    #[serde(deserialize_with = "required_empty_or_item_property_expression")]
    pub property: String,
    #[serde(deserialize_with = "required_item_token")]
    pub pocket: ItemPocket,
    #[serde(deserialize_with = "required_empty_or_item_token")]
    pub field_menu: String,
    pub field_usable: bool,
    #[serde(deserialize_with = "required_empty_or_item_token")]
    pub battle_menu: String,
    pub battle_usable: bool,
    #[serde(deserialize_with = "required_item_token")]
    pub script_name: String,
    pub consumable: bool,
    #[serde(deserialize_with = "required_nullable_usize")]
    pub tmhm_index: Option<usize>,
    #[serde(deserialize_with = "required_nullable_item_token")]
    pub tmhm_move: Option<String>,
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawItem {
            name: String,
            description: String,
            #[serde(deserialize_with = "required_item_token")]
            effect: String,
            #[serde(deserialize_with = "required_status_token_vec")]
            status_heals: Vec<String>,
            #[serde(deserialize_with = "required_nullable_u8")]
            revive_hp_percent: Option<u8>,
            #[serde(deserialize_with = "required_nullable_u8")]
            party_revive_hp_percent: Option<u8>,
            #[serde(deserialize_with = "required_nullable_item_token")]
            pp_restore_scope: Option<String>,
            #[serde(deserialize_with = "required_nullable_u8")]
            pp_restore_points: Option<u8>,
            #[serde(deserialize_with = "required_nullable_u8")]
            pp_up_stages: Option<u8>,
            #[serde(deserialize_with = "required_nullable_item_token")]
            vitamin_stat: Option<String>,
            #[serde(deserialize_with = "required_nullable_u16")]
            vitamin_stat_exp: Option<u16>,
            #[serde(deserialize_with = "required_nullable_u16")]
            vitamin_max_stat_exp: Option<u16>,
            #[serde(deserialize_with = "required_nullable_u8")]
            rare_candy_level_gain: Option<u8>,
            #[serde(deserialize_with = "required_nullable_item_token")]
            battle_stat_boost_stat: Option<String>,
            #[serde(deserialize_with = "required_nullable_u8")]
            battle_stat_boost_stages: Option<u8>,
            #[serde(deserialize_with = "required_nullable_item_token")]
            battle_escape_mode: Option<String>,
            #[serde(deserialize_with = "required_nullable_bool")]
            battle_capture_ball: Option<bool>,
            #[serde(deserialize_with = "required_nullable_bool")]
            battle_focus_energy: Option<bool>,
            #[serde(deserialize_with = "required_nullable_bool")]
            battle_stat_drop_guard: Option<bool>,
            #[serde(deserialize_with = "required_nullable_u8")]
            battle_stat_drop_guard_turns: Option<u8>,
            #[serde(deserialize_with = "required_nullable_bool")]
            confusion_heal: Option<bool>,
            #[serde(deserialize_with = "required_nullable_u16")]
            repel_steps: Option<u16>,
            #[serde(deserialize_with = "required_nullable_item_token")]
            escape_rope_mode: Option<String>,
            price: u16,
            #[serde(deserialize_with = "required_item_token")]
            held_effect: String,
            parameter: i16,
            #[serde(deserialize_with = "required_empty_or_item_property_expression")]
            property: String,
            #[serde(deserialize_with = "required_item_token")]
            pocket: ItemPocket,
            #[serde(deserialize_with = "required_empty_or_item_token")]
            field_menu: String,
            field_usable: bool,
            #[serde(deserialize_with = "required_empty_or_item_token")]
            battle_menu: String,
            battle_usable: bool,
            #[serde(deserialize_with = "required_item_token")]
            script_name: String,
            consumable: bool,
            #[serde(deserialize_with = "required_nullable_usize")]
            tmhm_index: Option<usize>,
            #[serde(deserialize_with = "required_nullable_item_token")]
            tmhm_move: Option<String>,
        }

        let raw = RawItem::deserialize(deserializer)?;
        let item = Self {
            name: raw.name,
            description: raw.description,
            effect: raw.effect,
            status_heals: raw.status_heals,
            revive_hp_percent: raw.revive_hp_percent,
            party_revive_hp_percent: raw.party_revive_hp_percent,
            pp_restore_scope: raw.pp_restore_scope,
            pp_restore_points: raw.pp_restore_points,
            pp_up_stages: raw.pp_up_stages,
            vitamin_stat: raw.vitamin_stat,
            vitamin_stat_exp: raw.vitamin_stat_exp,
            vitamin_max_stat_exp: raw.vitamin_max_stat_exp,
            rare_candy_level_gain: raw.rare_candy_level_gain,
            battle_stat_boost_stat: raw.battle_stat_boost_stat,
            battle_stat_boost_stages: raw.battle_stat_boost_stages,
            battle_escape_mode: raw.battle_escape_mode,
            battle_capture_ball: raw.battle_capture_ball,
            battle_focus_energy: raw.battle_focus_energy,
            battle_stat_drop_guard: raw.battle_stat_drop_guard,
            battle_stat_drop_guard_turns: raw.battle_stat_drop_guard_turns,
            confusion_heal: raw.confusion_heal,
            repel_steps: raw.repel_steps,
            escape_rope_mode: raw.escape_rope_mode,
            price: raw.price,
            held_effect: raw.held_effect,
            parameter: raw.parameter,
            property: raw.property,
            pocket: raw.pocket,
            field_menu: raw.field_menu,
            field_usable: raw.field_usable,
            battle_menu: raw.battle_menu,
            battle_usable: raw.battle_usable,
            script_name: raw.script_name,
            consumable: raw.consumable,
            tmhm_index: raw.tmhm_index,
            tmhm_move: raw.tmhm_move,
        };
        validate_item_payload(&item).map_err(serde::de::Error::custom)?;
        Ok(item)
    }
}

fn validate_item_payload(item: &Item) -> Result<(), String> {
    validate_exact_item_text("item.name", &item.name)?;
    validate_exact_optional_item_text("item.description", &item.description)?;
    if item.pp_up_stages.is_some_and(|stages| stages > 3) {
        return Err("item.pp_up_stages must be in 0..=3".to_string());
    }
    if item.revive_hp_percent.is_some_and(|percent| percent > 100)
        || item
            .party_revive_hp_percent
            .is_some_and(|percent| percent > 100)
    {
        return Err("item revive HP percent fields must be in 0..=100".to_string());
    }
    if item
        .battle_stat_boost_stages
        .is_some_and(|stages| stages == 0)
    {
        return Err("item.battle_stat_boost_stages must be positive".to_string());
    }
    if item.tmhm_index.is_some() != item.tmhm_move.is_some() {
        return Err("item TM/HM index and move must be declared together".to_string());
    }
    Ok(())
}

fn validate_exact_item_text(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(format!("{field} must be exact non-empty text"));
    }
    Ok(())
}

fn validate_exact_optional_item_text(field: &str, value: &str) -> Result<(), String> {
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err(format!("{field} must be exact text"));
    }
    Ok(())
}

fn required_item_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !is_exact_item_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "item token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_item_token(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_empty_or_item_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        return Ok(value);
    }
    if !is_exact_item_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "item token must be empty or exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_item_token(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_empty_or_item_property_expression<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        return Ok(value);
    }
    if !is_exact_item_property_expression(&value) {
        return Err(serde::de::Error::custom(format!(
            "item property must be empty or exact ASCII token/pipe expression, found {value:?}"
        )));
    }
    for token in value.split('|').map(str::trim) {
        validate_no_reserved_item_token(token).map_err(serde::de::Error::custom)?;
    }
    Ok(value)
}

fn required_nullable_item_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_item_token(&token) => {
            validate_no_reserved_item_token(&token).map_err(serde::de::Error::custom)?;
            Ok(Some(token))
        }
        Some(token) => Err(serde::de::Error::custom(format!(
            "item token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_status_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    for token in &values {
        if !is_exact_status_token(token) {
            return Err(serde::de::Error::custom(format!(
                "item token must be exact ASCII uppercase/underscore syntax, found {token:?}"
            )));
        }
        validate_no_reserved_item_token(token).map_err(serde::de::Error::custom)?;
    }
    Ok(values)
}

fn is_exact_status_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn is_exact_item_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'|'))
}

fn is_exact_item_property_expression(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.split('|').all(|token| {
            let token = token.trim();
            !token.is_empty()
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
}

fn validate_no_reserved_item_token(value: &str) -> Result<(), String> {
    let lowered = value.to_ascii_lowercase();
    if lowered.starts_with("fallback") || lowered.starts_with("legacy") {
        return Err(format!(
            "item token '{value}' uses reserved modpack payload prefix"
        ));
    }
    Ok(())
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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
              "battle_capture_ball":null,
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

    #[test]
    fn item_effect_field_rejects_enum_object_values() {
        let error = serde_json::from_value::<Item>(serde_json::json!({
            "name": "Flash Step Charm",
            "description": "A modded effect item.",
            "effect": { "kind": "MODDED_FLASH_STEP" },
            "status_heals": [],
            "revive_hp_percent": null,
            "party_revive_hp_percent": null,
            "pp_restore_scope": null,
            "pp_restore_points": null,
            "pp_up_stages": null,
            "vitamin_stat": null,
            "vitamin_stat_exp": null,
            "vitamin_max_stat_exp": null,
            "rare_candy_level_gain": null,
            "battle_stat_boost_stat": null,
            "battle_stat_boost_stages": null,
            "battle_escape_mode": null,
            "battle_focus_energy": null,
            "battle_capture_ball": null,
            "battle_stat_drop_guard": null,
            "battle_stat_drop_guard_turns": null,
            "confusion_heal": null,
            "repel_steps": null,
            "escape_rope_mode": null,
            "price": 100,
            "held_effect": "HELD_NONE",
            "parameter": 0,
            "property": "",
            "pocket": "ITEM",
            "field_menu": "",
            "field_usable": true,
            "battle_menu": "",
            "battle_usable": true,
            "script_name": "FLASH_STEP_CHARM",
            "consumable": true,
            "tmhm_index": null,
            "tmhm_move": null
        }))
        .expect_err("effect must be the exact modpack string, not an enum object")
        .to_string();

        assert!(
            error.contains("invalid type: map")
                || error.contains("invalid type: enum")
                || error.contains("expected a string"),
            "{error}"
        );
    }

    #[test]
    fn item_identifier_fields_reject_malformed_tokens_at_deserialization() {
        for (field, value) in [
            ("effect", serde_json::json!(" MODDED_FLASH_STEP")),
            ("effect", serde_json::json!("fallback_EFFECT")),
            ("status_heals", serde_json::json!(["SLP", "BAD POISON"])),
            ("status_heals", serde_json::json!(["SLP", "legacy_POISON"])),
            ("pp_restore_scope", serde_json::json!("PARTY PP")),
            ("vitamin_stat", serde_json::json!("SPECIAL ATTACK")),
            (
                "battle_stat_boost_stat",
                serde_json::json!("SPECIAL DEFENSE"),
            ),
            ("battle_escape_mode", serde_json::json!("BATTLE ESCAPE")),
            ("held_effect", serde_json::json!("HELD NONE")),
            ("property", serde_json::json!("CANT SELECT")),
            ("pocket", serde_json::json!("KEY ITEM")),
            ("field_menu", serde_json::json!("ITEMMENU PARTY")),
            ("battle_menu", serde_json::json!("ITEMMENU NOUSE")),
            ("escape_rope_mode", serde_json::json!("FIELD ESCAPE")),
            ("script_name", serde_json::json!("FLASH STEP CHARM")),
            ("tmhm_move", serde_json::json!("MUD SLAP")),
        ] {
            let mut item = valid_item_json();
            item[field] = value;

            let error = serde_json::from_value::<Item>(item)
                .expect_err("malformed item identifier fields must fail before runtime use")
                .to_string();

            assert!(
                error.contains("item token must be")
                    || error.contains("item property must be")
                    || error.contains("uses reserved modpack payload prefix"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn serialized_items_require_explicit_menu_and_battle_guard_fields() {
        for field in [
            "battle_stat_drop_guard",
            "battle_stat_drop_guard_turns",
            "battle_capture_ball",
            "field_menu",
            "field_usable",
            "battle_menu",
            "battle_usable",
        ] {
            let mut item = valid_item_json();
            item.as_object_mut()
                .expect("valid item json is an object")
                .remove(field);

            let error = serde_json::from_value::<Item>(item)
                .expect_err("item pack data must not infer omitted menu or guard fields")
                .to_string();

            assert!(
                error.contains(&format!("missing field `{field}`")),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    fn valid_item_json() -> serde_json::Value {
        serde_json::json!({
            "name": "Flash Step Charm",
            "description": "A modded effect item.",
            "effect": "MODDED_FLASH_STEP",
            "status_heals": [],
            "revive_hp_percent": null,
            "party_revive_hp_percent": null,
            "pp_restore_scope": null,
            "pp_restore_points": null,
            "pp_up_stages": null,
            "vitamin_stat": null,
            "vitamin_stat_exp": null,
            "vitamin_max_stat_exp": null,
            "rare_candy_level_gain": null,
            "battle_stat_boost_stat": null,
            "battle_stat_boost_stages": null,
            "battle_escape_mode": null,
            "battle_focus_energy": null,
            "battle_capture_ball": null,
            "battle_stat_drop_guard": null,
            "battle_stat_drop_guard_turns": null,
            "confusion_heal": null,
            "repel_steps": null,
            "escape_rope_mode": null,
            "price": 100,
            "held_effect": "HELD_NONE",
            "parameter": 0,
            "property": "",
            "pocket": "ITEM",
            "field_menu": "",
            "field_usable": true,
            "battle_menu": "",
            "battle_usable": true,
            "script_name": "FLASH_STEP_CHARM",
            "consumable": true,
            "tmhm_index": null,
            "tmhm_move": null
        })
    }
}
