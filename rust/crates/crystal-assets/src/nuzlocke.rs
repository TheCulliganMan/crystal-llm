use anyhow::{Result, ensure};
use crystal_core::models::Pokemon;
use crystal_core::state::{BattleMemory, GameState};
use serde::{Deserialize, Serialize};

pub const NUZLOCKE_MANIFEST_ID: &str = "nuzlocke";

const ENCOUNTER_PREFIX: &str = "NUZLOCKE_ENCOUNTER_";
const ACTIVE_ENCOUNTER_KEY: &str = "NUZLOCKE_ACTIVE_ENCOUNTER";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuzlockeRules {
    pub first_encounter_only: bool,
    pub permadeath: bool,
    pub require_capture_nickname: bool,
}

impl NuzlockeRules {
    pub const fn standard() -> Self {
        Self {
            first_encounter_only: true,
            permadeath: true,
            require_capture_nickname: true,
        }
    }

    pub const fn enabled(self) -> bool {
        self.first_encounter_only || self.permadeath || self.require_capture_nickname
    }

    pub const fn is_disabled(&self) -> bool {
        !self.first_encounter_only && !self.permadeath && !self.require_capture_nickname
    }
}

pub(crate) fn register_wild_encounter(
    rules: NuzlockeRules,
    state: &mut GameState,
    map_name: &str,
    battle_type: &str,
) -> bool {
    state.script_runtime.variables.remove(ACTIVE_ENCOUNTER_KEY);
    if !rules.first_encounter_only || !counts_for_first_encounter(battle_type) {
        return true;
    }

    let key = encounter_key(map_name);
    if state.script_runtime.variables.contains_key(&key) {
        return false;
    }
    state.script_runtime.variables.insert(key, "1".to_string());
    state
        .script_runtime
        .variables
        .insert(ACTIVE_ENCOUNTER_KEY.to_string(), map_name.to_string());
    true
}

pub(crate) fn ensure_active_capture_allowed(rules: NuzlockeRules, state: &GameState) -> Result<()> {
    if !rules.first_encounter_only {
        return Ok(());
    }
    let (map_name, battle_type) = match &state.battle {
        BattleMemory::Wild {
            map_name,
            battle_type,
            ..
        } => (map_name.as_str(), battle_type.as_str()),
        BattleMemory::StaticWild {
            origin_map_name,
            battle_type,
            ..
        } => (origin_map_name.as_str(), battle_type.as_str()),
        BattleMemory::Trainer { .. } | BattleMemory::Inactive => return Ok(()),
    };
    if !counts_for_first_encounter(battle_type) {
        return Ok(());
    }
    ensure!(
        state
            .script_runtime
            .variables
            .get(ACTIVE_ENCOUNTER_KEY)
            .is_some_and(|active| active == map_name),
        "Nuzlocke encounter for {map_name} has already been used"
    );
    Ok(())
}

pub(crate) fn ensure_capture_nickname(rules: NuzlockeRules, nickname: Option<&str>) -> Result<()> {
    if rules.require_capture_nickname {
        ensure!(
            nickname.is_some_and(|nickname| !nickname.is_empty()),
            "Nuzlocke captures must be nicknamed"
        );
    }
    Ok(())
}

pub(crate) fn ensure_can_restore_hp(rules: NuzlockeRules, pokemon: &Pokemon) -> Result<()> {
    if rules.permadeath {
        ensure!(
            pokemon.hp > 0,
            "Nuzlocke permadeath prevents restoring a fainted Pokemon"
        );
    }
    Ok(())
}

pub(crate) fn allows_party_storage_without_usable_replacement(
    rules: NuzlockeRules,
    pokemon: &Pokemon,
) -> bool {
    rules.permadeath && pokemon.hp == 0
}

pub(crate) fn run_is_over(rules: NuzlockeRules, state: &GameState) -> bool {
    rules.permadeath
        && !state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .chain(
                state
                    .storage
                    .pc_boxes
                    .iter()
                    .flat_map(|pc_box| pc_box.pokemon.iter().flatten()),
            )
            .any(|pokemon| !pokemon.is_egg && pokemon.hp > 0)
}

pub(crate) fn encounter_was_used(rules: NuzlockeRules, state: &GameState, map_name: &str) -> bool {
    rules.first_encounter_only
        && state
            .script_runtime
            .variables
            .contains_key(&encounter_key(map_name))
}

fn counts_for_first_encounter(battle_type: &str) -> bool {
    !matches!(battle_type, "BATTLETYPE_TUTORIAL" | "BATTLETYPE_CONTEST")
}

fn encounter_key(map_name: &str) -> String {
    format!("{ENCOUNTER_PREFIX}{map_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_encounter_is_persisted_and_later_encounters_are_ineligible() {
        let rules = NuzlockeRules::standard();
        let mut state = GameState::default();

        assert!(register_wild_encounter(
            rules,
            &mut state,
            "Route29",
            "BATTLETYPE_NORMAL"
        ));
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("NUZLOCKE_ENCOUNTER_Route29")
                .map(String::as_str),
            Some("1")
        );
        let saved = serde_json::to_value(&state).expect("serialize Nuzlocke encounter state");
        assert_eq!(
            saved.pointer("/script_runtime/variables/NUZLOCKE_ENCOUNTER_Route29"),
            Some(&serde_json::Value::String("1".to_string()))
        );
        assert!(!register_wild_encounter(
            rules,
            &mut state,
            "Route29",
            "BATTLETYPE_NORMAL"
        ));
        assert!(
            !state
                .script_runtime
                .variables
                .contains_key(ACTIVE_ENCOUNTER_KEY)
        );
    }

    #[test]
    fn tutorial_and_contest_encounters_do_not_consume_an_area() {
        let rules = NuzlockeRules::standard();
        for battle_type in ["BATTLETYPE_TUTORIAL", "BATTLETYPE_CONTEST"] {
            let mut state = GameState::default();
            assert!(register_wild_encounter(
                rules,
                &mut state,
                "NationalPark",
                battle_type
            ));
            assert!(
                !state
                    .script_runtime
                    .variables
                    .contains_key("NUZLOCKE_ENCOUNTER_NationalPark")
            );
        }
    }

    #[test]
    fn standard_rules_require_names_and_reject_reviving_fainted_pokemon() {
        let rules = NuzlockeRules::standard();
        assert!(ensure_capture_nickname(rules, None).is_err());
        assert!(ensure_capture_nickname(rules, Some("SPROUT")).is_ok());

        let mut pokemon = crystal_core::models::Pokemon::new_for_tests(
            crystal_core::models::PokemonSpecies::new_for_tests(
                "BELLSPROUT",
                crystal_core::models::BaseStats::new(50, 75, 35, 40, 70, 30),
            ),
            5,
            crystal_core::models::Dv::default(),
        );
        pokemon.hp = 0;
        assert!(ensure_can_restore_hp(rules, &pokemon).is_err());
        assert!(allows_party_storage_without_usable_replacement(
            rules, &pokemon
        ));
    }

    #[test]
    fn full_party_healing_keeps_fainted_pokemon_dead() {
        let mut state = GameState::default();
        let mut pokemon = crystal_core::models::Pokemon::new_for_tests(
            crystal_core::models::PokemonSpecies::new_for_tests(
                "BELLSPROUT",
                crystal_core::models::BaseStats::new(50, 75, 35, 40, 70, 30),
            ),
            5,
            crystal_core::models::Dv::default(),
        );
        pokemon.hp = 0;
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();

        let outcome = crate::full_heal_party_slot(
            &mut state,
            &std::collections::BTreeMap::new(),
            0,
            NuzlockeRules::standard(),
        )
        .expect("Nuzlocke center heal");

        assert_eq!(outcome.hp_before, 0);
        assert_eq!(outcome.hp_after, 0);
        assert_eq!(state.storage.party.pokemon[0].as_ref().unwrap().hp, 0);
    }

    #[test]
    fn disabled_rules_preserve_core_pack_encoding_but_enabled_rules_are_identity_bound() {
        let core =
            serde_json::to_value(crate::GameDataSet::default()).expect("serialize core game data");
        assert!(core.get("nuzlocke_rules").is_none());

        let mut nuzlocke = crate::GameDataSet::default();
        nuzlocke.nuzlocke_rules = NuzlockeRules::standard();
        let encoded = serde_json::to_value(nuzlocke).expect("serialize Nuzlocke game data");
        assert_eq!(
            encoded.pointer("/nuzlocke_rules/permadeath"),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[test]
    fn run_over_status_includes_living_pc_reserves() {
        let rules = NuzlockeRules::standard();
        let mut state = GameState::default();
        assert!(run_is_over(rules, &state));

        let pokemon = crystal_core::models::Pokemon::new_for_tests(
            crystal_core::models::PokemonSpecies::new_for_tests(
                "BELLSPROUT",
                crystal_core::models::BaseStats::new(50, 75, 35, 40, 70, 30),
            ),
            5,
            crystal_core::models::Dv::default(),
        );
        assert!(state.storage.pc_boxes[0].add_pokemon(pokemon));
        assert!(!run_is_over(rules, &state));
    }
}
