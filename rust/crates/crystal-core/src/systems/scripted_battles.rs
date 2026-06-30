use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::battle::start::{StaticWildBattleStart, activate_static_wild_battle_start};
use crate::map::ObjectEvent;
use crate::state::{EventFlagError, GameState, RngSeedCommit};
use crate::world::session::OverworldSession;

pub const SCRIPT_TRAINER_TABLE_COMMANDS: &[&str] = &["trainer"];
pub const SCRIPT_TRAINER_BATTLE_COMMANDS: &[&str] = &["winlosstext", "loadtrainer", "startbattle"];
pub const SCRIPT_WILD_BATTLE_COMMANDS: &[&str] = &["loadwildmon", "startbattle"];

pub fn is_known_script_trainer_table_command(command: &str) -> bool {
    SCRIPT_TRAINER_TABLE_COMMANDS.contains(&command)
}

pub fn is_known_script_trainer_battle_command(command: &str) -> bool {
    SCRIPT_TRAINER_BATTLE_COMMANDS.contains(&command)
}

pub fn is_known_script_wild_battle_command(command: &str) -> bool {
    SCRIPT_WILD_BATTLE_COMMANDS.contains(&command)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedBattleEffects {
    #[serde(deserialize_with = "required_scripted_battle_token_vec")]
    pub event_flags: Vec<String>,
    #[serde(deserialize_with = "required_scripted_battle_token_vec")]
    pub script_flags: Vec<String>,
    #[serde(deserialize_with = "required_scripted_battle_token_vec")]
    pub disappear_object_ids: Vec<String>,
}

impl<'de> Deserialize<'de> for ScriptedBattleEffects {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptedBattleEffects {
            #[serde(deserialize_with = "required_scripted_battle_token_vec")]
            event_flags: Vec<String>,
            #[serde(deserialize_with = "required_scripted_battle_token_vec")]
            script_flags: Vec<String>,
            #[serde(deserialize_with = "required_scripted_battle_token_vec")]
            disappear_object_ids: Vec<String>,
        }

        let raw = RawScriptedBattleEffects::deserialize(deserializer)?;
        let effects = Self {
            event_flags: raw.event_flags,
            script_flags: raw.script_flags,
            disappear_object_ids: raw.disappear_object_ids,
        };
        effects.validate_shape().map_err(D::Error::custom)?;
        Ok(effects)
    }
}

impl ScriptedBattleEffects {
    fn validate_shape(&self) -> Result<(), String> {
        for event_flag in self.event_flags.iter().chain(self.script_flags.iter()) {
            if event_flag == "0" || event_flag == "-1" {
                return Err(format!(
                    "scripted battle effect flag {event_flag} is not a real flag"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedBattleEffectsOutcome {
    pub event_flags_set: Vec<String>,
    pub script_flags_set: Vec<String>,
    pub disappeared_objects: Vec<DisappearedObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedWildBattleStartOutcome {
    pub pre_battle_event_flags_set: Vec<String>,
    pub rng_seed: RngSeedCommit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisappearedObject {
    pub object_identifier: String,
    pub event_flag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ScriptedBattleEffectsError {
    InvalidEventFlag {
        event_flag: String,
    },
    InvalidDisappearObject {
        object_identifier: String,
    },
    UnknownDisappearObject {
        object_identifier: String,
    },
    ObjectCannotDisappear {
        object_identifier: String,
        event_flag: String,
    },
    EventFlag {
        error: EventFlagError,
    },
}

pub fn apply_scripted_wild_battle_start(
    state: &mut GameState,
    pre_battle_event_flags: &[String],
    start: &StaticWildBattleStart,
) -> Result<ScriptedWildBattleStartOutcome, ScriptedBattleEffectsError> {
    let mut pre_battle_event_flags_set = Vec::new();
    for event_flag in pre_battle_event_flags {
        validate_effect_flag(event_flag)?;
        state
            .flags
            .set_event_flag(event_flag, true)
            .map_err(|error| ScriptedBattleEffectsError::EventFlag { error })?;
        pre_battle_event_flags_set.push(event_flag.clone());
    }
    let rng_seed = state.commit_rng_seed(start.rng_seed_after);
    activate_static_wild_battle_start(state, start);
    Ok(ScriptedWildBattleStartOutcome {
        pre_battle_event_flags_set,
        rng_seed,
    })
}

pub fn apply_scripted_battle_effects(
    state: &mut GameState,
    objects: &[ObjectEvent],
    effects: &ScriptedBattleEffects,
) -> Result<ScriptedBattleEffectsOutcome, ScriptedBattleEffectsError> {
    let mut event_flags_set = Vec::new();
    for event_flag in &effects.event_flags {
        validate_effect_flag(event_flag)?;
        state
            .flags
            .set_event_flag(event_flag, true)
            .map_err(|error| ScriptedBattleEffectsError::EventFlag { error })?;
        event_flags_set.push(event_flag.clone());
    }

    let mut script_flags_set = Vec::new();
    for script_flag in &effects.script_flags {
        validate_effect_flag(script_flag)?;
        state
            .flags
            .set_script_flag(script_flag, true)
            .map_err(|error| ScriptedBattleEffectsError::EventFlag { error })?;
        script_flags_set.push(script_flag.clone());
    }

    let mut disappeared_objects = Vec::new();
    for object_identifier in &effects.disappear_object_ids {
        validate_disappear_object_identifier(object_identifier)?;
        let object = objects
            .iter()
            .find(|object| object.object_identifier.as_ref() == Some(object_identifier))
            .ok_or_else(|| ScriptedBattleEffectsError::UnknownDisappearObject {
                object_identifier: object_identifier.clone(),
            })?;
        validate_disappear_flag(object_identifier, &object.event_flag)?;
        state
            .flags
            .set_event_flag(&object.event_flag, true)
            .map_err(|error| ScriptedBattleEffectsError::EventFlag { error })?;
        disappeared_objects.push(DisappearedObject {
            object_identifier: object_identifier.clone(),
            event_flag: object.event_flag.clone(),
        });
    }

    Ok(ScriptedBattleEffectsOutcome {
        event_flags_set,
        script_flags_set,
        disappeared_objects,
    })
}

pub fn apply_scripted_battle_effects_to_session(
    state: &mut GameState,
    session: &mut OverworldSession,
    effects: &ScriptedBattleEffects,
) -> Result<ScriptedBattleEffectsOutcome, ScriptedBattleEffectsError> {
    let outcome = apply_scripted_battle_effects(state, &session.objects, effects)?;
    session.sync_event_flag_memory(&state.flags);
    Ok(outcome)
}

fn validate_effect_flag(event_flag: &str) -> Result<(), ScriptedBattleEffectsError> {
    if event_flag == "0" || event_flag == "-1" || !is_exact_scripted_battle_token(event_flag) {
        return Err(ScriptedBattleEffectsError::InvalidEventFlag {
            event_flag: event_flag.to_string(),
        });
    }
    Ok(())
}

fn validate_disappear_object_identifier(
    object_identifier: &str,
) -> Result<(), ScriptedBattleEffectsError> {
    if !is_exact_scripted_battle_token(object_identifier) {
        return Err(ScriptedBattleEffectsError::InvalidDisappearObject {
            object_identifier: object_identifier.to_string(),
        });
    }
    Ok(())
}

fn validate_disappear_flag(
    object_identifier: &str,
    event_flag: &str,
) -> Result<(), ScriptedBattleEffectsError> {
    if event_flag == "0" || event_flag == "-1" || !is_exact_scripted_battle_token(event_flag) {
        return Err(ScriptedBattleEffectsError::ObjectCannotDisappear {
            object_identifier: object_identifier.to_string(),
            event_flag: event_flag.to_string(),
        });
    }
    Ok(())
}

fn is_exact_scripted_battle_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn required_scripted_battle_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if let Some(token) = values
        .iter()
        .find(|token| !is_exact_scripted_battle_token(token))
    {
        Err(serde::de::Error::custom(format!(
            "scripted battle token must be exact ASCII alphanumeric/underscore, found {token:?}"
        )))
    } else {
        Ok(values)
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, Pokemon, PokemonSpecies};

    fn test_pokemon() -> Pokemon {
        let species =
            PokemonSpecies::new_for_tests("SNORLAX", BaseStats::new(160, 110, 65, 30, 65, 110));
        Pokemon::new_for_tests(species, 30, Dv::default())
    }

    fn object(object_identifier: &str, event_flag: &str) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x: 1,
            y: 1,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "StaticMon".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(object_identifier.to_string()),
            sightline_direction_override: None,
        }
    }

    #[test]
    fn exported_scripted_battle_command_sets_are_exact() {
        assert!(SCRIPT_TRAINER_TABLE_COMMANDS.contains(&"trainer"));
        assert!(SCRIPT_TRAINER_BATTLE_COMMANDS.contains(&"winlosstext"));
        assert!(SCRIPT_TRAINER_BATTLE_COMMANDS.contains(&"loadtrainer"));
        assert!(SCRIPT_TRAINER_BATTLE_COMMANDS.contains(&"startbattle"));
        assert!(SCRIPT_WILD_BATTLE_COMMANDS.contains(&"loadwildmon"));
        assert!(SCRIPT_WILD_BATTLE_COMMANDS.contains(&"startbattle"));
        assert!(is_known_script_trainer_table_command("trainer"));
        assert!(is_known_script_trainer_battle_command("loadtrainer"));
        assert!(is_known_script_wild_battle_command("loadwildmon"));
        assert!(!is_known_script_trainer_table_command("Trainer"));
        assert!(!is_known_script_trainer_battle_command(
            "fallback_loadtrainer"
        ));
        assert!(!is_known_script_wild_battle_command("legacy_loadwildmon"));
    }

    #[test]
    fn scripted_wild_battle_start_sets_flags_rng_and_active_battle() {
        let pokemon = test_pokemon();
        let start = StaticWildBattleStart {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            species: "SNORLAX".to_string(),
            level: 30,
            source_script: "Route12SnorlaxScript".to_string(),
            enemy_pokemon: pokemon.clone(),
            enemy_party: vec![pokemon],
            rng_seed_after: 0xfeed_beef,
        };
        let mut state = GameState {
            rng_seed: 0x1234_5678,
            ..GameState::default()
        };

        let outcome = apply_scripted_wild_battle_start(
            &mut state,
            &["EVENT_FOUGHT_SNORLAX".to_string()],
            &start,
        )
        .expect("scripted wild battle start applies");

        assert_eq!(
            outcome.pre_battle_event_flags_set,
            vec!["EVENT_FOUGHT_SNORLAX".to_string()]
        );
        assert_eq!(outcome.rng_seed.rng_seed_before, 0x1234_5678);
        assert_eq!(outcome.rng_seed.rng_seed_after, 0xfeed_beef);
        assert_eq!(state.rng_seed, 0xfeed_beef);
        assert_eq!(state.battle_active_party_index, None);
        assert_eq!(state.battle_active_enemy_party_index, Some(0));
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_FOUGHT_SNORLAX"),
            Ok(true)
        );
        assert!(matches!(
            state.battle,
            crate::state::BattleMemory::StaticWild {
                ref species,
                level: 30,
                ..
            } if species == "SNORLAX"
        ));
    }

    #[test]
    fn scripted_wild_battle_start_rejects_invalid_pre_battle_flag_without_mutation() {
        let pokemon = test_pokemon();
        let start = StaticWildBattleStart {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            species: "SNORLAX".to_string(),
            level: 30,
            source_script: "Route12SnorlaxScript".to_string(),
            enemy_pokemon: pokemon.clone(),
            enemy_party: vec![pokemon],
            rng_seed_after: 0xfeed_beef,
        };
        let mut state = GameState {
            rng_seed: 0x1234_5678,
            ..GameState::default()
        };

        let error =
            apply_scripted_wild_battle_start(&mut state, &["fallback_flag".to_string()], &start)
                .expect_err("invalid pre-battle flag rejected");

        assert_eq!(
            error,
            ScriptedBattleEffectsError::InvalidEventFlag {
                event_flag: "fallback_flag".to_string(),
            }
        );
        assert_eq!(state.rng_seed, 0x1234_5678);
        assert_eq!(state.battle, crate::state::BattleMemory::Inactive);
    }

    #[test]
    fn scripted_battle_effects_set_exact_flags_and_disappear_object_flags() {
        let mut state = GameState::default();
        let effects = ScriptedBattleEffects {
            event_flags: vec!["EVENT_FOUGHT_SNORLAX".to_string()],
            script_flags: vec!["ENGINE_FLYPOINT_VERMILION".to_string()],
            disappear_object_ids: vec!["VERMILIONCITY_BIG_SNORLAX".to_string()],
        };

        let outcome = apply_scripted_battle_effects(
            &mut state,
            &[object(
                "VERMILIONCITY_BIG_SNORLAX",
                "EVENT_VERMILION_CITY_SNORLAX",
            )],
            &effects,
        )
        .expect("effects apply");

        assert_eq!(
            outcome.event_flags_set,
            vec!["EVENT_FOUGHT_SNORLAX".to_string()]
        );
        assert_eq!(
            outcome.script_flags_set,
            vec!["ENGINE_FLYPOINT_VERMILION".to_string()]
        );
        assert_eq!(
            outcome.disappeared_objects,
            vec![DisappearedObject {
                object_identifier: "VERMILIONCITY_BIG_SNORLAX".to_string(),
                event_flag: "EVENT_VERMILION_CITY_SNORLAX".to_string(),
            }]
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_FOUGHT_SNORLAX"),
            Ok(true)
        );
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_VERMILION_CITY_SNORLAX"),
            Ok(true)
        );
        assert_eq!(
            state.flags.is_engine_flag_set("ENGINE_FLYPOINT_VERMILION"),
            Ok(true)
        );
    }

    #[test]
    fn disappear_object_requires_exact_object_identifier() {
        let mut state = GameState::default();
        let effects = ScriptedBattleEffects {
            event_flags: Vec::new(),
            script_flags: Vec::new(),
            disappear_object_ids: vec!["vermilioncity_big_snorlax".to_string()],
        };

        let error = apply_scripted_battle_effects(
            &mut state,
            &[object(
                "VERMILIONCITY_BIG_SNORLAX",
                "EVENT_VERMILION_CITY_SNORLAX",
            )],
            &effects,
        )
        .expect_err("object id must match exactly");

        assert_eq!(
            error,
            ScriptedBattleEffectsError::UnknownDisappearObject {
                object_identifier: "vermilioncity_big_snorlax".to_string(),
            }
        );
    }

    #[test]
    fn scripted_battle_effects_reject_malformed_flags_and_object_ids() {
        let mut state = GameState::default();
        let malformed_flag = ScriptedBattleEffects {
            event_flags: vec![" EVENT_FOUGHT_SNORLAX".to_string()],
            script_flags: Vec::new(),
            disappear_object_ids: Vec::new(),
        };
        let error = apply_scripted_battle_effects(&mut state, &[], &malformed_flag)
            .expect_err("padded event flags are invalid pack data");
        assert_eq!(
            error,
            ScriptedBattleEffectsError::InvalidEventFlag {
                event_flag: " EVENT_FOUGHT_SNORLAX".to_string(),
            }
        );

        let malformed_object = ScriptedBattleEffects {
            event_flags: Vec::new(),
            script_flags: Vec::new(),
            disappear_object_ids: vec!["VERMILIONCITY BIG SNORLAX".to_string()],
        };
        let error = apply_scripted_battle_effects(
            &mut state,
            &[object(
                "VERMILIONCITY_BIG_SNORLAX",
                "EVENT_VERMILION_CITY_SNORLAX",
            )],
            &malformed_object,
        )
        .expect_err("space-separated object ids are invalid pack data");
        assert_eq!(
            error,
            ScriptedBattleEffectsError::InvalidDisappearObject {
                object_identifier: "VERMILIONCITY BIG SNORLAX".to_string(),
            }
        );

        let malformed_object_flag = ScriptedBattleEffects {
            event_flags: Vec::new(),
            script_flags: Vec::new(),
            disappear_object_ids: vec!["VERMILIONCITY_BIG_SNORLAX".to_string()],
        };
        let error = apply_scripted_battle_effects(
            &mut state,
            &[object(
                "VERMILIONCITY_BIG_SNORLAX",
                "EVENT VERMILION CITY SNORLAX",
            )],
            &malformed_object_flag,
        )
        .expect_err("object event flags must be exact pack tokens");
        assert_eq!(
            error,
            ScriptedBattleEffectsError::ObjectCannotDisappear {
                object_identifier: "VERMILIONCITY_BIG_SNORLAX".to_string(),
                event_flag: "EVENT VERMILION CITY SNORLAX".to_string(),
            }
        );
    }

    #[test]
    fn scripted_battle_effects_reject_reserved_pack_prefixes() {
        let mut state = GameState::default();
        let reserved_flag = ScriptedBattleEffects {
            event_flags: vec!["fallback_event".to_string()],
            script_flags: Vec::new(),
            disappear_object_ids: Vec::new(),
        };
        let error = apply_scripted_battle_effects(&mut state, &[], &reserved_flag)
            .expect_err("reserved event flags are invalid pack data");
        assert_eq!(
            error,
            ScriptedBattleEffectsError::InvalidEventFlag {
                event_flag: "fallback_event".to_string(),
            }
        );

        let reserved_object = ScriptedBattleEffects {
            event_flags: Vec::new(),
            script_flags: Vec::new(),
            disappear_object_ids: vec!["legacy_object".to_string()],
        };
        let error = apply_scripted_battle_effects(&mut state, &[], &reserved_object)
            .expect_err("reserved object ids are invalid pack data");
        assert_eq!(
            error,
            ScriptedBattleEffectsError::InvalidDisappearObject {
                object_identifier: "legacy_object".to_string(),
            }
        );

        for (field, value) in [
            ("event_flags", serde_json::json!(["fallback_event"])),
            ("script_flags", serde_json::json!(["legacy_script_flag"])),
            (
                "disappear_object_ids",
                serde_json::json!(["fallback_object"]),
            ),
        ] {
            let mut payload = serde_json::json!({
                "event_flags": ["EVENT_FOUGHT_SNORLAX"],
                "script_flags": ["ENGINE_FLYPOINT_VERMILION"],
                "disappear_object_ids": ["VERMILIONCITY_BIG_SNORLAX"]
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptedBattleEffects>(payload)
                .expect_err("reserved scripted battle tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("scripted battle token must be"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn disappear_rejects_unhideable_object_flags() {
        let mut state = GameState::default();
        let effects = ScriptedBattleEffects {
            event_flags: Vec::new(),
            script_flags: Vec::new(),
            disappear_object_ids: vec!["ALWAYS_VISIBLE".to_string()],
        };

        let error =
            apply_scripted_battle_effects(&mut state, &[object("ALWAYS_VISIBLE", "-1")], &effects)
                .expect_err("unhideable object cannot disappear");

        assert_eq!(
            error,
            ScriptedBattleEffectsError::ObjectCannotDisappear {
                object_identifier: "ALWAYS_VISIBLE".to_string(),
                event_flag: "-1".to_string(),
            }
        );
    }

    #[test]
    fn scripted_battle_effect_error_json_rejects_unknown_fallback_fields() {
        let error = serde_json::from_value::<ScriptedBattleEffectsError>(serde_json::json!({
            "UnknownDisappearObject": {
                "object_identifier": "MOD_SNORLAX",
                "fallback_object_identifier": "VERMILIONCITY_BIG_SNORLAX"
            }
        }))
        .expect_err("fallback object identifier must be rejected")
        .to_string();
        assert!(
            error.contains("unknown field `fallback_object_identifier`"),
            "{error}"
        );
    }
}
