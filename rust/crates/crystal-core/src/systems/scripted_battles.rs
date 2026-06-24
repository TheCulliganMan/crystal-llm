use serde::{Deserialize, Serialize};

use crate::map::ObjectEvent;
use crate::state::{EventFlagError, GameState};
use crate::world::session::OverworldSession;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedBattleEffects {
    pub event_flags: Vec<String>,
    pub script_flags: Vec<String>,
    pub disappear_object_ids: Vec<String>,
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
pub struct DisappearedObject {
    pub object_identifier: String,
    pub event_flag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptedBattleEffectsError {
    InvalidEventFlag {
        event_flag: String,
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
    if event_flag.is_empty() || event_flag == "0" || event_flag == "-1" {
        return Err(ScriptedBattleEffectsError::InvalidEventFlag {
            event_flag: event_flag.to_string(),
        });
    }
    Ok(())
}

fn validate_disappear_flag(
    object_identifier: &str,
    event_flag: &str,
) -> Result<(), ScriptedBattleEffectsError> {
    if event_flag.is_empty() || event_flag == "0" || event_flag == "-1" {
        return Err(ScriptedBattleEffectsError::ObjectCannotDisappear {
            object_identifier: object_identifier.to_string(),
            event_flag: event_flag.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
