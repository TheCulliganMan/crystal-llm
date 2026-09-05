use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::models::pokemon::CaughtData;
use crate::models::{
    CaptureStorageLocation, Dv, Item, MAX_PC_BOXES, Move, Pokemon, PokemonBuildError,
    PokemonSpecies, PokemonStorage, create_pokemon_from_known_dvs,
};
use crate::state::GameState;
use crate::systems::experience::GrowthRateCatalog;
use crate::systems::learnsets::SpeciesLearnsets;

pub const NO_ITEM: &str = "NO_ITEM";
pub const EGG_NICKNAME: &str = "EGG";
pub const SCRIPT_GIFT_POKEMON_COMMANDS: &[&str] = &["givepoke", "giveegg"];

pub fn is_known_script_gift_pokemon_command(command: &str) -> bool {
    SCRIPT_GIFT_POKEMON_COMMANDS.contains(&command)
}

pub fn is_script_gift_egg_command(command: &str) -> bool {
    command == "giveegg"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GiftPokemonScript {
    #[serde(deserialize_with = "required_gift_token")]
    pub species_id: String,
    #[serde(deserialize_with = "required_gift_value_token")]
    pub level_token: String,
    pub level: u8,
    #[serde(deserialize_with = "required_nullable_gift_token")]
    pub held_item_id: Option<String>,
    #[serde(deserialize_with = "required_nullable_gift_token")]
    pub nickname_label: Option<String>,
    #[serde(deserialize_with = "required_nullable_gift_token")]
    pub ot_label: Option<String>,
    #[serde(deserialize_with = "required_gift_label_token")]
    pub source_script: String,
    pub command_index: usize,
    pub egg: bool,
}

impl<'de> Deserialize<'de> for GiftPokemonScript {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawGiftPokemonScript {
            #[serde(deserialize_with = "required_gift_token")]
            species_id: String,
            #[serde(deserialize_with = "required_gift_value_token")]
            level_token: String,
            level: u8,
            #[serde(deserialize_with = "required_nullable_gift_token")]
            held_item_id: Option<String>,
            #[serde(deserialize_with = "required_nullable_gift_token")]
            nickname_label: Option<String>,
            #[serde(deserialize_with = "required_nullable_gift_token")]
            ot_label: Option<String>,
            #[serde(deserialize_with = "required_gift_label_token")]
            source_script: String,
            command_index: usize,
            egg: bool,
        }

        let raw = RawGiftPokemonScript::deserialize(deserializer)?;
        let script = Self {
            species_id: raw.species_id,
            level_token: raw.level_token,
            level: raw.level,
            held_item_id: raw.held_item_id,
            nickname_label: raw.nickname_label,
            ot_label: raw.ot_label,
            source_script: raw.source_script,
            command_index: raw.command_index,
            egg: raw.egg,
        };
        validate_gift_level(script.level).map_err(D::Error::custom)?;
        Ok(script)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GiftPokemonRequest {
    #[serde(deserialize_with = "required_gift_token")]
    pub species_id: String,
    pub level: u8,
    #[serde(deserialize_with = "required_nullable_gift_token")]
    pub held_item_id: Option<String>,
    pub nickname: Option<String>,
    pub original_trainer_name: String,
    pub original_trainer_id: u16,
    pub caught_data: Option<CaughtData>,
    #[serde(deserialize_with = "required_gift_label_token")]
    pub source_script: String,
    pub command_index: usize,
    pub egg: bool,
    pub dvs: Dv,
}

impl<'de> Deserialize<'de> for GiftPokemonRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawGiftPokemonRequest {
            #[serde(deserialize_with = "required_gift_token")]
            species_id: String,
            level: u8,
            #[serde(deserialize_with = "required_nullable_gift_token")]
            held_item_id: Option<String>,
            nickname: Option<String>,
            original_trainer_name: String,
            original_trainer_id: u16,
            caught_data: Option<CaughtData>,
            #[serde(deserialize_with = "required_gift_label_token")]
            source_script: String,
            command_index: usize,
            egg: bool,
            dvs: Dv,
        }

        let raw = RawGiftPokemonRequest::deserialize(deserializer)?;
        validate_gift_level(raw.level).map_err(D::Error::custom)?;
        Ok(Self {
            species_id: raw.species_id,
            level: raw.level,
            held_item_id: raw.held_item_id,
            nickname: raw.nickname,
            original_trainer_name: raw.original_trainer_name,
            original_trainer_id: raw.original_trainer_id,
            caught_data: raw.caught_data,
            source_script: raw.source_script,
            command_index: raw.command_index,
            egg: raw.egg,
            dvs: raw.dvs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiftPokemonOutcome {
    pub species_id: String,
    pub level: u8,
    pub location: Option<CaptureStorageLocation>,
    /// The exact value left in wScriptVar by Script_givepoke or Script_giveegg.
    pub script_value: u8,
    pub pokemon: Pokemon,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum GiftPokemonError {
    InvalidSpeciesId { species_id: String },
    UnknownSpecies { species_id: String },
    InvalidHeldItemId { item_id: String },
    UnknownHeldItem { item_id: String },
    InvalidSourceScript { source_script: String },
    InvalidLevel { level: u8 },
    InvalidCurrentPcBox { current_pc_box: usize },
    PokemonBuild { error: PokemonBuildError },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GiftPokemonScriptIssue {
    InvalidSpeciesId { species_id: String },
    UnknownSpecies { species_id: String },
    InvalidHeldItemId { item_id: String },
    UnknownHeldItem { item_id: String },
    InvalidSourceScript { source_script: String },
    EmptyLabel { field: &'static str },
    InvalidLabel { field: &'static str, label: String },
    UnknownLabel { field: &'static str, label: String },
}

pub fn gift_pokemon_script_issues(
    gift: &GiftPokemonScript,
    species: &BTreeMap<String, PokemonSpecies>,
    items: &BTreeMap<String, Item>,
    script_labels: &BTreeSet<String>,
) -> Vec<GiftPokemonScriptIssue> {
    let mut issues = Vec::new();
    if !is_exact_gift_label_token(&gift.source_script) {
        issues.push(GiftPokemonScriptIssue::InvalidSourceScript {
            source_script: gift.source_script.clone(),
        });
    }
    if !is_exact_gift_token(&gift.species_id) {
        issues.push(GiftPokemonScriptIssue::InvalidSpeciesId {
            species_id: gift.species_id.clone(),
        });
    } else if !species.contains_key(&gift.species_id) {
        issues.push(GiftPokemonScriptIssue::UnknownSpecies {
            species_id: gift.species_id.clone(),
        });
    }
    if let Some(item_id) = gift.held_item_id.as_deref() {
        if !is_exact_gift_token(item_id) {
            issues.push(GiftPokemonScriptIssue::InvalidHeldItemId {
                item_id: item_id.to_string(),
            });
        } else if !items.contains_key(item_id) {
            issues.push(GiftPokemonScriptIssue::UnknownHeldItem {
                item_id: item_id.to_string(),
            });
        }
    }
    push_label_issue(
        "nickname",
        gift.nickname_label.as_deref(),
        script_labels,
        &mut issues,
    );
    push_label_issue(
        "original trainer",
        gift.ot_label.as_deref(),
        script_labels,
        &mut issues,
    );
    issues
}

fn push_label_issue(
    field: &'static str,
    label: Option<&str>,
    script_labels: &BTreeSet<String>,
    issues: &mut Vec<GiftPokemonScriptIssue>,
) {
    let Some(label) = label else {
        return;
    };
    if label.is_empty() {
        issues.push(GiftPokemonScriptIssue::EmptyLabel { field });
    } else if !is_exact_gift_token(label) {
        issues.push(GiftPokemonScriptIssue::InvalidLabel {
            field,
            label: label.to_string(),
        });
    } else if !script_labels.contains(label) {
        issues.push(GiftPokemonScriptIssue::UnknownLabel {
            field,
            label: label.to_string(),
        });
    }
}

fn is_exact_gift_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_gift_value_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn is_exact_gift_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn validate_gift_level(level: u8) -> Result<(), String> {
    if level == 0 {
        Err("gift Pokemon level must be positive".to_string())
    } else {
        Ok(())
    }
}

fn required_gift_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_gift_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "gift Pokemon token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_nullable_gift_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_gift_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "gift Pokemon token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_gift_value_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_gift_value_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "gift Pokemon value token must be exact visible ASCII, found {value:?}"
        )))
    }
}

fn required_gift_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_gift_label_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "gift Pokemon source script must be exact visible ASCII, found {value:?}"
        )))
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

pub fn give_gift_pokemon(
    storage: &mut PokemonStorage,
    current_pc_box: usize,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    items: &BTreeMap<String, Item>,
    request: GiftPokemonRequest,
) -> Result<GiftPokemonOutcome, GiftPokemonError> {
    if request.level == 0 {
        return Err(GiftPokemonError::InvalidLevel {
            level: request.level,
        });
    }
    if !is_exact_gift_label_token(&request.source_script) {
        return Err(GiftPokemonError::InvalidSourceScript {
            source_script: request.source_script.clone(),
        });
    }
    if !is_exact_gift_token(&request.species_id) {
        return Err(GiftPokemonError::InvalidSpeciesId {
            species_id: request.species_id.clone(),
        });
    }
    let species_data =
        species
            .get(&request.species_id)
            .ok_or_else(|| GiftPokemonError::UnknownSpecies {
                species_id: request.species_id.clone(),
            })?;
    if let Some(item_id) = request.held_item_id.as_deref() {
        if !is_exact_gift_token(item_id) {
            return Err(GiftPokemonError::InvalidHeldItemId {
                item_id: item_id.to_string(),
            });
        }
        if !items.contains_key(item_id) {
            return Err(GiftPokemonError::UnknownHeldItem {
                item_id: item_id.to_string(),
            });
        }
    }

    let mut pokemon = create_pokemon_from_known_dvs(
        species_data,
        request.level,
        request.dvs,
        learnsets,
        moves,
        growth_rates,
    )
    .map_err(|error| GiftPokemonError::PokemonBuild { error })?;
    pokemon.original_trainer_name = request.original_trainer_name.clone();
    pokemon.original_trainer_id = request.original_trainer_id;
    pokemon.caught_data = request.caught_data.clone();
    pokemon.item = request.held_item_id.clone();
    if let Some(nickname) = request.nickname.as_deref() {
        pokemon.nickname = nickname.to_string();
    }
    if request.egg {
        pokemon.is_egg = true;
        pokemon.nickname = EGG_NICKNAME.to_string();
        pokemon.happiness = species_data.step_cycles_to_hatch;
        pokemon.hp = 0;
        pokemon.status = None;
    } else {
        pokemon.hp = pokemon.max_hp;
        pokemon.status = None;
        pokemon.sleep_turns = 0;
        pokemon.flinching = false;
        pokemon.confusion_turns = 0;
        pokemon.rampage_turns = 0;
    }

    let (location, script_value) = if request.egg {
        let location = storage.party.next_open_slot().and_then(|slot| {
            storage
                .party
                .add_pokemon(pokemon.clone())
                .then_some(CaptureStorageLocation::Party { slot })
        });
        let script_value = if location.is_some() { 2 } else { 0 };
        (location, script_value)
    } else {
        if current_pc_box >= MAX_PC_BOXES {
            return Err(GiftPokemonError::InvalidCurrentPcBox { current_pc_box });
        }
        match storage.register_capture_in_box(current_pc_box, pokemon.clone()) {
            Ok(location) => {
                let script_value = match location {
                    CaptureStorageLocation::Party { .. } => 0,
                    CaptureStorageLocation::Pc { .. } => 1,
                };
                (Some(location), script_value)
            }
            // GivePoke treats a full selected box as an ordinary B=2 result.
            // register_capture_in_box has no other failure for a validated index.
            Err(_) => (None, 2),
        }
    };
    Ok(GiftPokemonOutcome {
        species_id: request.species_id,
        level: request.level,
        location,
        script_value,
        pokemon,
        source_script: request.source_script,
        command_index: request.command_index,
    })
}

pub fn grant_gift_pokemon_to_state(
    state: &mut GameState,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    items: &BTreeMap<String, Item>,
    request: GiftPokemonRequest,
) -> Result<GiftPokemonOutcome, GiftPokemonError> {
    let egg = request.egg;
    let current_pc_box = state.current_pc_box;
    let outcome = give_gift_pokemon(
        &mut state.storage,
        current_pc_box,
        species,
        learnsets,
        moves,
        growth_rates,
        items,
        request,
    )?;
    if !egg && outcome.location.is_some() {
        state.pokedex.record_caught_pokemon(&outcome.pokemon);
    }
    let script_value = outcome.script_value.to_string();
    state.script_runtime.script_value = Some(script_value.clone());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), script_value.clone());
    state
        .script_runtime
        .memory
        .insert("wScriptVar".to_string(), script_value);
    state.sync_party_from_storage();
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, MAX_BOX_MONS, PcBox, growth_rate, item_pocket, pokemon_type};
    use crate::systems::experience::{GrowthRateCatalog, crystal_growth_rate_catalog_for_tests};

    fn growth_rates() -> GrowthRateCatalog {
        crystal_growth_rate_catalog_for_tests()
    }

    fn species(id: &str) -> PokemonSpecies {
        PokemonSpecies {
            id: id.to_string(),
            int_id: 1,
            base_stats: BaseStats::new(45, 49, 65, 45, 49, 65),
            type1: pokemon_type("GRASS"),
            type2: pokemon_type("GRASS"),
            catch_rate: 45,
            base_exp: 64,
            item1: None,
            item2: None,
            gender_ratio: 127,
            unknown1: 0,
            unknown2: 0,
            growth_rate: growth_rate("GROWTH_MEDIUM_SLOW"),
            egg_group1: crate::models::egg_group("EGG_MONSTER"),
            egg_group2: crate::models::egg_group("EGG_MONSTER"),
            tmhm_learnset: Vec::new(),
            ability: crate::models::ability("NONE"),
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            weight: 0,
            step_cycles_to_hatch: 20,
        }
    }

    fn item(id: &str) -> Item {
        Item {
            name: id.to_string(),
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
            confusion_heal: None,
            repel_steps: None,
            escape_rope_mode: None,
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("ITEM"),
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

    fn request(species_id: &str, level: u8) -> GiftPokemonRequest {
        GiftPokemonRequest {
            species_id: species_id.to_string(),
            level,
            held_item_id: None,
            nickname: None,
            original_trainer_name: "PLAYER".to_string(),
            original_trainer_id: 1234,
            caught_data: None,
            source_script: "GiftScript".to_string(),
            command_index: 4,
            egg: false,
            dvs: Dv::from_non_hp(10, 10, 10, 10),
        }
    }

    fn learnsets(species_id: &str) -> SpeciesLearnsets {
        [(species_id.to_string(), Vec::new())].into_iter().collect()
    }

    fn pokemon(id: &str) -> Pokemon {
        Pokemon::new_for_tests(species(id), 5, Dv::from_non_hp(10, 10, 10, 10))
    }

    fn fill_party(storage: &mut PokemonStorage) {
        for index in 0..6 {
            assert!(
                storage
                    .party
                    .add_pokemon(pokemon(&format!("PARTY_{index}")))
            );
        }
    }

    fn full_box(index: usize) -> PcBox {
        let mut pc_box = PcBox::new(index);
        for slot in 0..MAX_BOX_MONS {
            assert!(pc_box.add_pokemon(pokemon(&format!("BOX_{index}_{slot}"))));
        }
        pc_box
    }

    #[test]
    fn exported_script_gift_command_set_is_exact() {
        assert!(SCRIPT_GIFT_POKEMON_COMMANDS.contains(&"givepoke"));
        assert!(SCRIPT_GIFT_POKEMON_COMMANDS.contains(&"giveegg"));
        assert!(is_known_script_gift_pokemon_command("givepoke"));
        assert!(is_known_script_gift_pokemon_command("giveegg"));
        assert!(is_script_gift_egg_command("giveegg"));
        assert!(!is_script_gift_egg_command("givepoke"));
        assert!(!is_known_script_gift_pokemon_command("GivePoke"));
        assert!(!is_known_script_gift_pokemon_command("fallback_givepoke"));
    }

    #[test]
    fn gift_pokemon_script_issues_validate_exact_ids_and_labels() {
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);
        let labels = ["GiftNicknameText".to_string()].into_iter().collect();
        let gift = GiftPokemonScript {
            species_id: "CYNDA QUIL".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: Some("BERRY JUICE".to_string()),
            nickname_label: Some("Gift NicknameText".to_string()),
            ot_label: Some(" GiftOtText".to_string()),
            source_script: "fallback_script".to_string(),
            command_index: 4,
            egg: false,
        };

        assert_eq!(
            gift_pokemon_script_issues(&gift, &species_map, &items, &labels),
            vec![
                GiftPokemonScriptIssue::InvalidSourceScript {
                    source_script: "fallback_script".to_string(),
                },
                GiftPokemonScriptIssue::InvalidSpeciesId {
                    species_id: "CYNDA QUIL".to_string(),
                },
                GiftPokemonScriptIssue::InvalidHeldItemId {
                    item_id: "BERRY JUICE".to_string(),
                },
                GiftPokemonScriptIssue::InvalidLabel {
                    field: "nickname",
                    label: "Gift NicknameText".to_string(),
                },
                GiftPokemonScriptIssue::InvalidLabel {
                    field: "original trainer",
                    label: " GiftOtText".to_string(),
                },
            ]
        );
    }

    #[test]
    fn gift_pokemon_script_issues_reject_reserved_pack_prefix_tokens() {
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);
        let labels = ["GiftNicknameText".to_string()].into_iter().collect();
        let gift = GiftPokemonScript {
            species_id: "fallback_cyndaquil".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: Some("legacy_berry".to_string()),
            nickname_label: Some("fallback_nickname".to_string()),
            ot_label: Some("legacy_ot".to_string()),
            source_script: "GiftScript".to_string(),
            command_index: 4,
            egg: false,
        };

        assert_eq!(
            gift_pokemon_script_issues(&gift, &species_map, &items, &labels),
            vec![
                GiftPokemonScriptIssue::InvalidSpeciesId {
                    species_id: "fallback_cyndaquil".to_string(),
                },
                GiftPokemonScriptIssue::InvalidHeldItemId {
                    item_id: "legacy_berry".to_string(),
                },
                GiftPokemonScriptIssue::InvalidLabel {
                    field: "nickname",
                    label: "fallback_nickname".to_string(),
                },
                GiftPokemonScriptIssue::InvalidLabel {
                    field: "original trainer",
                    label: "legacy_ot".to_string(),
                },
            ]
        );
    }

    #[test]
    fn gift_pokemon_script_json_rejects_reserved_pack_tokens() {
        for (field, value) in [
            ("species_id", serde_json::json!("fallback_cyndaquil")),
            ("level_token", serde_json::json!("legacy_level")),
            ("held_item_id", serde_json::json!("legacy_berry")),
            ("nickname_label", serde_json::json!("fallback_nickname")),
            ("ot_label", serde_json::json!("legacy_ot")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "species_id": "CYNDAQUIL",
                "level_token": "5",
                "level": 5,
                "held_item_id": "BERRY",
                "nickname_label": "GiftNicknameText",
                "ot_label": "GiftOtText",
                "source_script": "GiftScript",
                "command_index": 4,
                "egg": false
            });
            payload[field] = value;

            let error = serde_json::from_value::<GiftPokemonScript>(payload)
                .expect_err("reserved gift Pokemon script tokens must fail during JSON load")
                .to_string();
            assert!(
                error.contains("gift Pokemon") || error.contains("gift Pokemon"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn gift_pokemon_error_json_rejects_unknown_fallback_fields() {
        let species_error = serde_json::from_value::<GiftPokemonError>(serde_json::json!({
            "UnknownSpecies": {
                "species_id": "MODMON",
                "fallback_species_id": "PIKACHU"
            }
        }))
        .expect_err("fallback species id must be rejected")
        .to_string();
        assert!(
            species_error.contains("unknown field `fallback_species_id`"),
            "{species_error}"
        );

        let item_error = serde_json::from_value::<GiftPokemonError>(serde_json::json!({
            "UnknownHeldItem": {
                "item_id": "MOD_ITEM",
                "legacy_item_id": "NO_ITEM"
            }
        }))
        .expect_err("legacy item id must be rejected")
        .to_string();
        assert!(
            item_error.contains("unknown field `legacy_item_id`"),
            "{item_error}"
        );
    }

    #[test]
    fn gives_exact_species_to_party_with_exact_held_item() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);
        let mut request = request("CYNDAQUIL", 5);
        request.held_item_id = Some("BERRY".to_string());

        let outcome = give_gift_pokemon(
            &mut storage,
            0,
            &species_map,
            &learnsets("CYNDAQUIL"),
            &BTreeMap::new(),
            &growth_rates(),
            &items,
            request,
        )
        .expect("gift pokemon");

        assert_eq!(
            outcome.location,
            Some(CaptureStorageLocation::Party { slot: 0 })
        );
        assert_eq!(outcome.pokemon.species.id, "CYNDAQUIL");
        assert_eq!(outcome.pokemon.item.as_deref(), Some("BERRY"));
        assert_eq!(storage.party.filled_slots(), 1);
    }

    #[test]
    fn rejects_case_changed_species_and_item_ids() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);
        let bad_species = request("cyndaquil", 5);
        let mut bad_item = request("CYNDAQUIL", 5);
        bad_item.held_item_id = Some("berry".to_string());

        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                bad_species,
            ),
            Err(GiftPokemonError::UnknownSpecies {
                species_id: "cyndaquil".to_string(),
            })
        );
        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                bad_item,
            ),
            Err(GiftPokemonError::UnknownHeldItem {
                item_id: "berry".to_string(),
            })
        );
    }

    #[test]
    fn rejects_malformed_gift_request_ids_before_unknown_lookup() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);

        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                request("CYNDA QUIL", 5),
            ),
            Err(GiftPokemonError::InvalidSpeciesId {
                species_id: "CYNDA QUIL".to_string(),
            })
        );

        let mut bad_item = request("CYNDAQUIL", 5);
        bad_item.held_item_id = Some("BERRY JUICE".to_string());
        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                bad_item,
            ),
            Err(GiftPokemonError::InvalidHeldItemId {
                item_id: "BERRY JUICE".to_string(),
            })
        );

        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                request("fallback_cyndaquil", 5),
            ),
            Err(GiftPokemonError::InvalidSpeciesId {
                species_id: "fallback_cyndaquil".to_string(),
            })
        );

        let mut reserved_item = request("CYNDAQUIL", 5);
        reserved_item.held_item_id = Some("legacy_berry".to_string());
        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                reserved_item,
            ),
            Err(GiftPokemonError::InvalidHeldItemId {
                item_id: "legacy_berry".to_string(),
            })
        );

        let mut reserved_source = request("CYNDAQUIL", 5);
        reserved_source.source_script = "fallback_script".to_string();
        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &growth_rates(),
                &items,
                reserved_source,
            ),
            Err(GiftPokemonError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );
        assert_eq!(storage.party.filled_slots(), 0);
    }

    #[test]
    fn gives_egg_to_party_with_egg_metadata() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("TOGEPI".to_string(), species("TOGEPI"))]);
        let mut request = request("TOGEPI", 5);
        request.egg = true;

        let outcome = give_gift_pokemon(
            &mut storage,
            0,
            &species_map,
            &learnsets("TOGEPI"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            request,
        )
        .expect("gift egg");

        assert_eq!(outcome.pokemon.nickname, EGG_NICKNAME);
        assert_eq!(outcome.pokemon.status, None);
        assert!(outcome.pokemon.is_egg);
        assert_eq!(outcome.pokemon.hp, 0);
        assert_eq!(outcome.pokemon.happiness, 20);
    }

    #[test]
    fn grant_gift_pokemon_to_state_syncs_party_and_records_only_non_eggs() {
        let mut state = GameState::default();
        let species_map = BTreeMap::from([
            ("CYNDAQUIL".to_string(), species("CYNDAQUIL")),
            ("TOGEPI".to_string(), species("TOGEPI")),
        ]);

        let outcome = grant_gift_pokemon_to_state(
            &mut state,
            &species_map,
            &learnsets("CYNDAQUIL"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            request("CYNDAQUIL", 5),
        )
        .expect("gift pokemon");

        assert_eq!(
            outcome.location,
            Some(CaptureStorageLocation::Party { slot: 0 })
        );
        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().species.id,
            "CYNDAQUIL"
        );
        assert_eq!(
            state.party.pokemon[0].as_ref().unwrap().species,
            "CYNDAQUIL"
        );
        assert!(state.pokedex.has_seen("CYNDAQUIL"));
        assert!(state.pokedex.has_caught("CYNDAQUIL"));

        let mut egg_request = request("TOGEPI", 5);
        egg_request.egg = true;
        grant_gift_pokemon_to_state(
            &mut state,
            &species_map,
            &learnsets("TOGEPI"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            egg_request,
        )
        .expect("gift egg");

        assert_eq!(
            state.storage.party.pokemon[1].as_ref().unwrap().species.id,
            "TOGEPI"
        );
        assert_eq!(state.party.pokemon[1].as_ref().unwrap().species, "TOGEPI");
        assert!(!state.pokedex.has_seen("TOGEPI"));
        assert!(!state.pokedex.has_caught("TOGEPI"));
    }

    #[test]
    fn givepoke_uses_only_selected_box_and_reports_full_without_fatal_error() {
        let mut state = GameState::default();
        fill_party(&mut state.storage);
        state.storage.pc_boxes = vec![full_box(0), PcBox::new(1)];
        state.current_pc_box = 0;
        state.sync_party_from_storage();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);

        let outcome = grant_gift_pokemon_to_state(
            &mut state,
            &species_map,
            &learnsets("CYNDAQUIL"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            request("CYNDAQUIL", 5),
        )
        .expect("full selected box is a normal GivePoke result");

        assert_eq!(outcome.location, None);
        assert_eq!(outcome.script_value, 2);
        assert_eq!(state.storage.pc_boxes[0].filled_slots(), MAX_BOX_MONS);
        assert_eq!(state.storage.pc_boxes[1].filled_slots(), 0);
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wScriptVar")
                .map(String::as_str),
            Some("2")
        );
        assert!(!state.pokedex.has_caught("CYNDAQUIL"));
    }

    #[test]
    fn giveegg_is_party_only_and_reports_full_without_pc_fallback() {
        let mut state = GameState::default();
        fill_party(&mut state.storage);
        state.storage.pc_boxes = vec![PcBox::new(0), PcBox::new(1)];
        state.current_pc_box = 0;
        state.sync_party_from_storage();
        let species_map = BTreeMap::from([("TOGEPI".to_string(), species("TOGEPI"))]);
        let mut egg_request = request("TOGEPI", 5);
        egg_request.egg = true;

        let outcome = grant_gift_pokemon_to_state(
            &mut state,
            &species_map,
            &learnsets("TOGEPI"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            egg_request,
        )
        .expect("full party is a normal GiveEgg result");

        assert_eq!(outcome.location, None);
        assert_eq!(outcome.script_value, 0);
        assert_eq!(state.storage.pc_boxes[0].filled_slots(), 0);
        assert_eq!(state.storage.pc_boxes[1].filled_slots(), 0);
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wScriptVar")
                .map(String::as_str),
            Some("0")
        );
        assert!(!state.pokedex.has_seen("TOGEPI"));
        assert!(!state.pokedex.has_caught("TOGEPI"));
    }

    #[test]
    fn gift_script_accumulators_match_party_box_and_egg_success() {
        let species_map = BTreeMap::from([
            ("CYNDAQUIL".to_string(), species("CYNDAQUIL")),
            ("TOGEPI".to_string(), species("TOGEPI")),
        ]);

        let mut party_state = GameState::default();
        let party_outcome = grant_gift_pokemon_to_state(
            &mut party_state,
            &species_map,
            &learnsets("CYNDAQUIL"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            request("CYNDAQUIL", 5),
        )
        .expect("party gift");
        assert_eq!(party_outcome.script_value, 0);
        assert_eq!(
            party_state.script_runtime.script_value.as_deref(),
            Some("0")
        );

        let mut box_state = GameState::default();
        fill_party(&mut box_state.storage);
        box_state.current_pc_box = 3;
        let box_outcome = grant_gift_pokemon_to_state(
            &mut box_state,
            &species_map,
            &learnsets("CYNDAQUIL"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            request("CYNDAQUIL", 5),
        )
        .expect("box gift");
        assert_eq!(box_outcome.script_value, 1);
        assert_eq!(box_state.script_runtime.script_value.as_deref(), Some("1"));
        assert_eq!(box_state.storage.pc_boxes[3].filled_slots(), 1);

        let mut egg_state = GameState::default();
        let mut egg_request = request("TOGEPI", 5);
        egg_request.egg = true;
        let egg_outcome = grant_gift_pokemon_to_state(
            &mut egg_state,
            &species_map,
            &learnsets("TOGEPI"),
            &BTreeMap::new(),
            &growth_rates(),
            &BTreeMap::new(),
            egg_request,
        )
        .expect("egg gift");
        assert_eq!(egg_outcome.script_value, 2);
        assert_eq!(egg_state.script_runtime.script_value.as_deref(), Some("2"));
    }

    #[test]
    fn rejects_missing_learnset_moves_without_creating_zero_pp_gift() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let learnsets = [(
            "CYNDAQUIL".to_string(),
            vec![crate::systems::learnsets::LearnsetEntry(
                1,
                "TACKLE".to_string(),
            )],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                0,
                &species_map,
                &learnsets,
                &BTreeMap::new(),
                &growth_rates(),
                &BTreeMap::new(),
                request("CYNDAQUIL", 5),
            ),
            Err(GiftPokemonError::PokemonBuild {
                error: PokemonBuildError::UnknownLearnsetMove {
                    species_id: "CYNDAQUIL".to_string(),
                    move_name: "TACKLE".to_string(),
                },
            })
        );
        assert_eq!(storage.party.filled_slots(), 0);
    }
}
