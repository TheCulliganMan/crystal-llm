use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::{ITEM_POCKET_TM_HM, Item, LearnedMove, Move, Pokemon};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TmHmLearnOutcome {
    pub item_id: String,
    pub tmhm_index: usize,
    pub learned_move: String,
    pub replaced_slot: Option<usize>,
    pub replaced_move: Option<String>,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum TmHmLearnError {
    #[error("invalid TM/HM item id '{item_id}'")]
    InvalidItemId { item_id: String },
    #[error("TM/HM item '{item_id}' has invalid move id '{move_id}'")]
    InvalidMoveId { item_id: String, move_id: String },
    #[error("item '{item_id}' is not a TM/HM")]
    NotTmHm { item_id: String },
    #[error("TM/HM item '{item_id}' is missing tmhm_index")]
    MissingTmHmIndex { item_id: String },
    #[error("TM/HM item '{item_id}' is missing tmhm_move")]
    MissingTmHmMove { item_id: String },
    #[error("TM/HM item '{item_id}' teaches unknown move '{move_id}'")]
    UnknownMove { item_id: String, move_id: String },
    #[error("species '{species_id}' cannot learn '{move_id}' from TM/HM")]
    CannotLearn { species_id: String, move_id: String },
    #[error("Pokemon already knows '{move_id}'")]
    AlreadyKnows { move_id: String },
    #[error("Pokemon has four moves and no replacement slot was selected")]
    MoveListFull,
    #[error("replacement slot {slot} is outside the Pokemon move list")]
    InvalidReplacementSlot { slot: usize },
    #[error("saved bag.tm_hm has {slots} slots, compiled TM/HM max index is {max_index}")]
    SavedTmHmSlotsExceedCompiledMax { slots: usize, max_index: usize },
    #[error("saved bag.tm_hm has {slots} slots, fewer than compiled TM/HM max index {max_index}")]
    SavedTmHmSlotsBelowCompiledMax { slots: usize, max_index: usize },
    #[error("saved bag.tm_hm has {slots} slots, but compiled pack has no TM/HM items")]
    SavedTmHmSlotsWithoutCompiledItems { slots: usize },
    #[error("saved bag.tm_hm[{index}] has no compiled TM/HM item with matching tmhm_index")]
    SavedTmHmMissingCompiledItem { index: usize },
    #[error(
        "saved bag.tm_hm[{index}] matches {matches} compiled TM/HM items; tmhm_index must be unique"
    )]
    SavedTmHmDuplicateCompiledItems { index: usize, matches: usize },
}

pub fn teach_tmhm_move(
    pokemon: &mut Pokemon,
    item: &Item,
    moves: &BTreeMap<String, Move>,
    replace_slot: Option<usize>,
    consumed: bool,
) -> Result<TmHmLearnOutcome, TmHmLearnError> {
    validate_tmhm_item_id(&item.script_name)?;
    if item.pocket != ITEM_POCKET_TM_HM {
        return Err(TmHmLearnError::NotTmHm {
            item_id: item.script_name.clone(),
        });
    }
    let tmhm_index = item
        .tmhm_index
        .ok_or_else(|| TmHmLearnError::MissingTmHmIndex {
            item_id: item.script_name.clone(),
        })?;
    let move_id = item
        .tmhm_move
        .as_ref()
        .ok_or_else(|| TmHmLearnError::MissingTmHmMove {
            item_id: item.script_name.clone(),
        })?;
    validate_tmhm_move_id(&item.script_name, move_id)?;
    let move_data = moves
        .get(move_id)
        .ok_or_else(|| TmHmLearnError::UnknownMove {
            item_id: item.script_name.clone(),
            move_id: move_id.clone(),
        })?;
    if !pokemon
        .species
        .tmhm_learnset
        .iter()
        .any(|learnable| learnable == move_id)
    {
        return Err(TmHmLearnError::CannotLearn {
            species_id: pokemon.species.id.clone(),
            move_id: move_id.clone(),
        });
    }
    if pokemon.moves.iter().any(|known| known.name == *move_id) {
        return Err(TmHmLearnError::AlreadyKnows {
            move_id: move_id.clone(),
        });
    }

    let learned = LearnedMove {
        name: move_id.clone(),
        current_pp: move_data.pp,
        pp_ups: 0,
    };
    let (replaced_slot, replaced_move) = if pokemon.moves.len() < 4 {
        pokemon.moves.push(learned);
        (None, None)
    } else {
        let slot = replace_slot.ok_or(TmHmLearnError::MoveListFull)?;
        let existing = pokemon
            .moves
            .get_mut(slot)
            .ok_or(TmHmLearnError::InvalidReplacementSlot { slot })?;
        let replaced = std::mem::replace(existing, learned).name;
        (Some(slot), Some(replaced))
    };

    Ok(TmHmLearnOutcome {
        item_id: item.script_name.clone(),
        tmhm_index,
        learned_move: move_id.clone(),
        replaced_slot,
        replaced_move,
        consumed,
    })
}

pub fn validate_saved_tmhm_references(
    items: &BTreeMap<String, Item>,
    tm_hm: &[bool],
) -> Result<(), TmHmLearnError> {
    let max_index = items
        .values()
        .filter(|item| item.pocket == ITEM_POCKET_TM_HM)
        .filter_map(|item| item.tmhm_index)
        .max();
    match max_index {
        Some(max_index) if tm_hm.len() > max_index + 1 => {
            return Err(TmHmLearnError::SavedTmHmSlotsExceedCompiledMax {
                slots: tm_hm.len(),
                max_index,
            });
        }
        Some(max_index) if tm_hm.len() < max_index + 1 => {
            return Err(TmHmLearnError::SavedTmHmSlotsBelowCompiledMax {
                slots: tm_hm.len(),
                max_index,
            });
        }
        None if !tm_hm.is_empty() => {
            return Err(TmHmLearnError::SavedTmHmSlotsWithoutCompiledItems { slots: tm_hm.len() });
        }
        _ => {}
    }
    for index in 0..tm_hm.len() {
        let matches = items
            .values()
            .filter(|item| item.pocket == ITEM_POCKET_TM_HM && item.tmhm_index == Some(index))
            .count();
        if tm_hm[index] && matches == 0 {
            return Err(TmHmLearnError::SavedTmHmMissingCompiledItem { index });
        }
        if matches > 1 {
            return Err(TmHmLearnError::SavedTmHmDuplicateCompiledItems { index, matches });
        }
    }
    Ok(())
}

fn validate_tmhm_item_id(item_id: &str) -> Result<(), TmHmLearnError> {
    if !is_exact_tmhm_token(item_id) {
        return Err(TmHmLearnError::InvalidItemId {
            item_id: item_id.to_string(),
        });
    }
    Ok(())
}

fn validate_tmhm_move_id(item_id: &str, move_id: &str) -> Result<(), TmHmLearnError> {
    if !is_exact_tmhm_token(move_id) {
        return Err(TmHmLearnError::InvalidMoveId {
            item_id: item_id.to_string(),
            move_id: move_id.to_string(),
        });
    }
    Ok(())
}

fn is_exact_tmhm_token(value: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies, item_pocket, pokemon_type};

    fn test_move(name: &str, pp: u8) -> Move {
        Move {
            name: name.to_string(),
            move_type: pokemon_type("NORMAL"),
            power: 40,
            accuracy: 100,
            pp,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    fn moves() -> BTreeMap<String, Move> {
        BTreeMap::from([
            ("HEADBUTT".to_string(), test_move("HEADBUTT", 15)),
            ("CUT".to_string(), test_move("CUT", 30)),
            ("TACKLE".to_string(), test_move("TACKLE", 35)),
            ("GROWL".to_string(), test_move("GROWL", 40)),
            ("TAIL_WHIP".to_string(), test_move("TAIL_WHIP", 30)),
            ("LEER".to_string(), test_move("LEER", 30)),
        ])
    }

    fn item(id: &str, index: usize, move_id: &str, consumable: bool) -> Item {
        Item {
            name: id.to_string(),
            description: String::new(),
            effect: "USE_TMHM".to_string(),
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
            pocket: item_pocket("TM_HM"),
            field_menu: "ITEMMENU_PARTY".to_string(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: id.to_string(),
            consumable,
            tmhm_index: Some(index),
            tmhm_move: Some(move_id.to_string()),
        }
    }

    fn pokemon(learnset: &[&str], known_moves: &[&str]) -> Pokemon {
        let mut species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        species.tmhm_learnset = learnset
            .iter()
            .map(|move_id| (*move_id).to_string())
            .collect();
        let move_catalog = moves();
        let mut pokemon = Pokemon::new_for_tests(species, 12, Dv::default());
        pokemon.moves = known_moves
            .iter()
            .map(|move_id| LearnedMove {
                name: (*move_id).to_string(),
                current_pp: move_catalog.get(*move_id).expect("move").pp,
                pp_ups: 0,
            })
            .collect();
        pokemon
    }

    #[test]
    fn tmhm_error_json_rejects_unknown_fallback_fields() {
        let move_error = serde_json::from_value::<TmHmLearnError>(serde_json::json!({
            "UnknownMove": {
                "item_id": "TM_HEADBUTT",
                "move_id": "MOD_MOVE",
                "fallback_move_id": "TACKLE"
            }
        }))
        .expect_err("TM/HM errors must not accept fallback move ids")
        .to_string();
        assert!(
            move_error.contains("unknown field `fallback_move_id`"),
            "{move_error}"
        );

        let slot_error = serde_json::from_value::<TmHmLearnError>(serde_json::json!({
            "InvalidReplacementSlot": {
                "slot": 9,
                "default_slot": 0
            }
        }))
        .expect_err("TM/HM errors must not accept default replacement slots")
        .to_string();
        assert!(
            slot_error.contains("unknown field `default_slot`"),
            "{slot_error}"
        );
    }

    #[test]
    fn teaches_tmhm_move_from_explicit_item_field() {
        let item = item("TM_HEADBUTT", 1, "HEADBUTT", true);
        let mut pokemon = pokemon(&["HEADBUTT"], &["TACKLE"]);

        let outcome =
            teach_tmhm_move(&mut pokemon, &item, &moves(), None, true).expect("teach TM move");

        assert_eq!(outcome.learned_move, "HEADBUTT");
        assert_eq!(outcome.tmhm_index, 1);
        assert!(outcome.consumed);
        assert_eq!(pokemon.moves.last().expect("learned").name, "HEADBUTT");
        assert_eq!(pokemon.moves.last().expect("learned").current_pp, 15);
    }

    #[test]
    fn rejects_species_without_exact_tmhm_learnset_entry() {
        let item = item("TM_HEADBUTT", 1, "HEADBUTT", true);
        let mut pokemon = pokemon(&["CUT"], &["TACKLE"]);

        let error =
            teach_tmhm_move(&mut pokemon, &item, &moves(), None, false).expect_err("cannot learn");

        assert!(matches!(error, TmHmLearnError::CannotLearn { .. }));
        assert_eq!(pokemon.moves.len(), 1);
    }

    #[test]
    fn rejects_already_known_move() {
        let item = item("TM_HEADBUTT", 1, "HEADBUTT", true);
        let mut pokemon = pokemon(&["HEADBUTT"], &["HEADBUTT"]);

        let error =
            teach_tmhm_move(&mut pokemon, &item, &moves(), None, false).expect_err("already knows");

        assert_eq!(
            error,
            TmHmLearnError::AlreadyKnows {
                move_id: "HEADBUTT".to_string()
            }
        );
    }

    #[test]
    fn full_move_list_requires_replacement_slot() {
        let item = item("TM_HEADBUTT", 1, "HEADBUTT", true);
        let mut pokemon = pokemon(&["HEADBUTT"], &["TACKLE", "GROWL", "TAIL_WHIP", "LEER"]);

        let error =
            teach_tmhm_move(&mut pokemon, &item, &moves(), None, false).expect_err("must replace");

        assert_eq!(error, TmHmLearnError::MoveListFull);
    }

    #[test]
    fn replaces_selected_slot_when_move_list_is_full() {
        let item = item("TM_HEADBUTT", 1, "HEADBUTT", true);
        let mut pokemon = pokemon(&["HEADBUTT"], &["TACKLE", "GROWL", "TAIL_WHIP", "LEER"]);

        let outcome =
            teach_tmhm_move(&mut pokemon, &item, &moves(), Some(2), true).expect("replace move");

        assert_eq!(outcome.replaced_slot, Some(2));
        assert_eq!(outcome.replaced_move.as_deref(), Some("TAIL_WHIP"));
        assert_eq!(pokemon.moves[2].name, "HEADBUTT");
    }

    #[test]
    fn rejects_missing_definitive_move_field() {
        let mut item = item("TM_HEADBUTT", 1, "HEADBUTT", true);
        item.tmhm_move = None;
        let mut pokemon = pokemon(&["HEADBUTT"], &["TACKLE"]);

        let error =
            teach_tmhm_move(&mut pokemon, &item, &moves(), None, false).expect_err("missing move");

        assert_eq!(
            error,
            TmHmLearnError::MissingTmHmMove {
                item_id: "TM_HEADBUTT".to_string()
            }
        );
    }

    #[test]
    fn rejects_malformed_tmhm_ids_before_unknown_or_missing_fallbacks() {
        let mut bad_item_id = item("TM HEADBUTT", 1, "HEADBUTT", true);
        let mut pokemon = pokemon(&["HEADBUTT"], &["TACKLE"]);
        let error = teach_tmhm_move(&mut pokemon, &bad_item_id, &moves(), None, false)
            .expect_err("malformed item ids are invalid pack data");
        assert_eq!(
            error,
            TmHmLearnError::InvalidItemId {
                item_id: "TM HEADBUTT".to_string()
            }
        );
        assert_eq!(pokemon.moves.len(), 1);

        bad_item_id.script_name = "TM_HEADBUTT".to_string();
        bad_item_id.tmhm_move = Some("HEAD BUTT".to_string());
        let error = teach_tmhm_move(&mut pokemon, &bad_item_id, &moves(), None, false)
            .expect_err("malformed move ids are invalid pack data");
        assert_eq!(
            error,
            TmHmLearnError::InvalidMoveId {
                item_id: "TM_HEADBUTT".to_string(),
                move_id: "HEAD BUTT".to_string(),
            }
        );
        assert_eq!(pokemon.moves.len(), 1);

        bad_item_id.script_name = "fallback_tm_headbutt".to_string();
        bad_item_id.tmhm_move = Some("HEADBUTT".to_string());
        let error = teach_tmhm_move(&mut pokemon, &bad_item_id, &moves(), None, false)
            .expect_err("reserved item ids are invalid pack data");
        assert_eq!(
            error,
            TmHmLearnError::InvalidItemId {
                item_id: "fallback_tm_headbutt".to_string()
            }
        );
        assert_eq!(pokemon.moves.len(), 1);

        bad_item_id.script_name = "TM_HEADBUTT".to_string();
        bad_item_id.tmhm_move = Some("legacy_headbutt".to_string());
        let error = teach_tmhm_move(&mut pokemon, &bad_item_id, &moves(), None, false)
            .expect_err("reserved move ids are invalid pack data");
        assert_eq!(
            error,
            TmHmLearnError::InvalidMoveId {
                item_id: "TM_HEADBUTT".to_string(),
                move_id: "legacy_headbutt".to_string(),
            }
        );
        assert_eq!(pokemon.moves.len(), 1);
    }
}
