use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::{
    Dv, Move, Pokemon, PokemonBuildError, PokemonSpecies, Trainer, TrainerCatalog,
    TrainerPartyPokemon, create_pokemon_from_known_dvs,
};
use crate::random::Random;
use crate::state::{BattleMemory, EventFlagError, GameState};
use crate::systems::economy::CurrencyCatalog;
use crate::systems::experience::GrowthRateCatalog;
use crate::systems::learnsets::SpeciesLearnsets;
use crate::world::session::WildEncounterRoll;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WildBattleStart {
    pub battle_type: String,
    pub encounter: WildEncounterRoll,
    pub enemy_pokemon: Pokemon,
    pub enemy_party: Vec<Pokemon>,
    pub rng_seed_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticWildBattleRequest {
    pub battle_type: String,
    pub species: String,
    pub level: u8,
    pub source_script: String,
}

impl StaticWildBattleRequest {
    pub fn new(species: impl Into<String>, level: u8) -> Self {
        Self {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            species: species.into(),
            level,
            source_script: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticWildBattleStart {
    pub battle_type: String,
    pub species: String,
    pub level: u8,
    pub source_script: String,
    pub enemy_pokemon: Pokemon,
    pub enemy_party: Vec<Pokemon>,
    pub rng_seed_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerBattleRequest {
    pub battle_type: String,
    pub trainer_class: String,
    pub trainer_id: String,
    pub event_flag: String,
    pub seen_text: String,
    pub win_text: String,
    pub loss_text: String,
    pub callback: String,
    pub source_script: String,
}

impl TrainerBattleRequest {
    pub fn new(
        trainer_class: impl Into<String>,
        trainer_id: impl Into<String>,
        event_flag: impl Into<String>,
    ) -> Self {
        Self {
            battle_type: "BATTLETYPE_TRAINER".to_string(),
            trainer_class: trainer_class.into(),
            trainer_id: trainer_id.into(),
            event_flag: event_flag.into(),
            seen_text: String::new(),
            win_text: String::new(),
            loss_text: String::new(),
            callback: String::new(),
            source_script: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerBattleStart {
    pub battle_type: String,
    pub trainer_class: String,
    pub trainer_id: String,
    pub trainer_name: String,
    pub event_flag: String,
    pub seen_text: String,
    pub win_text: String,
    pub loss_text: String,
    pub callback: String,
    pub source_script: String,
    pub enemy_pokemon: Pokemon,
    pub enemy_party: Vec<Pokemon>,
    pub reward: u32,
    pub encounter_music: String,
    pub ai_move_flags: u32,
    pub ai_item_switch_flags: u32,
    pub ai_layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainerBattleStartStatus {
    Started(TrainerBattleStart),
    AlreadyDefeated {
        event_flag: String,
        callback: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerBattleCompletion {
    pub trainer_id: String,
    pub trainer_class: String,
    pub event_flag: String,
    pub won: bool,
    pub can_lose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerBattleCompletionOutcome {
    pub continued_after_battle: bool,
    pub prize_money: u32,
    pub money_after: u32,
}

impl From<&WildBattleStart> for BattleMemory {
    fn from(start: &WildBattleStart) -> Self {
        Self::Wild {
            battle_type: start.battle_type.clone(),
            map_name: start.encounter.map_name.clone(),
            enemy_pokemon: start.enemy_pokemon.clone(),
            enemy_party: start.enemy_party.clone(),
        }
    }
}

impl From<&StaticWildBattleStart> for BattleMemory {
    fn from(start: &StaticWildBattleStart) -> Self {
        Self::StaticWild {
            battle_type: start.battle_type.clone(),
            species: start.species.clone(),
            level: start.level,
            source_script: start.source_script.clone(),
            enemy_pokemon: start.enemy_pokemon.clone(),
            enemy_party: start.enemy_party.clone(),
        }
    }
}

impl From<&TrainerBattleStart> for BattleMemory {
    fn from(start: &TrainerBattleStart) -> Self {
        Self::Trainer {
            battle_type: start.battle_type.clone(),
            trainer_class: start.trainer_class.clone(),
            trainer_id: start.trainer_id.clone(),
            trainer_name: start.trainer_name.clone(),
            event_flag: start.event_flag.clone(),
            seen_text: start.seen_text.clone(),
            win_text: start.win_text.clone(),
            loss_text: start.loss_text.clone(),
            callback: start.callback.clone(),
            source_script: start.source_script.clone(),
            enemy_pokemon: start.enemy_pokemon.clone(),
            enemy_party: start.enemy_party.clone(),
            reward: start.reward,
            encounter_music: start.encounter_music.clone(),
            ai_move_flags: start.ai_move_flags,
            ai_item_switch_flags: start.ai_item_switch_flags,
            ai_layers: start.ai_layers.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrainerBattleError {
    #[error("trainer request is missing exact trainer_id")]
    MissingTrainerId,
    #[error("trainer request is missing exact trainer_class")]
    MissingTrainerClass,
    #[error("unknown trainer '{trainer_id}'")]
    UnknownTrainer { trainer_id: String },
    #[error("trainer '{trainer_id}' class mismatch: request '{requested}', pack '{actual}'")]
    TrainerClassMismatch {
        trainer_id: String,
        requested: String,
        actual: String,
    },
    #[error("trainer '{trainer_id}' has empty party")]
    EmptyParty { trainer_id: String },
    #[error("trainer '{trainer_id}' party slot {slot} is missing species")]
    MissingPartySpecies { trainer_id: String, slot: usize },
    #[error("trainer '{trainer_id}' party slot {slot} references unknown species '{species}'")]
    UnknownPartySpecies {
        trainer_id: String,
        slot: usize,
        species: String,
    },
    #[error("trainer defeated flag error: {0:?}")]
    EventFlag(#[from] EventFlagError),
    #[error("trainer Pokemon build error: {0}")]
    PokemonBuild(#[from] PokemonBuildError),
    #[error("trainer battle completion requires an active trainer battle")]
    MissingActiveTrainerBattle,
    #[error(
        "trainer battle completion mismatch: active {active_class}/{active_id}, completion {completion_class}/{completion_id}"
    )]
    CompletionTrainerMismatch {
        active_class: String,
        active_id: String,
        completion_class: String,
        completion_id: String,
    },
    #[error("active trainer battle '{trainer_id}' has empty enemy party")]
    EmptyActiveTrainerParty { trainer_id: String },
    #[error("active trainer battle '{trainer_id}' still has unfainted party slot {slot}")]
    ActiveTrainerPartyNotDefeated { trainer_id: String, slot: usize },
    #[error("active trainer battle '{trainer_id}' party slot {slot} rewards were not claimed")]
    ActiveTrainerPartyRewardsUnclaimed { trainer_id: String, slot: usize },
    #[error("trainer prize money overflow for reward {reward} and level {level}")]
    PrizeMoneyOverflow { reward: u32, level: u8 },
    #[error("trainer battle completion requires currency constant '{constant}'")]
    MissingCurrencyLimit { constant: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum StaticWildBattleError {
    #[error("static wild battle request is missing exact species id")]
    MissingSpecies,
    #[error("unknown static wild species '{species}'")]
    UnknownSpecies { species: String },
    #[error("static wild battle level cannot be zero for species '{species}'")]
    ZeroLevel { species: String },
    #[error("static wild Pokemon build error: {0}")]
    PokemonBuild(#[from] PokemonBuildError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum WildBattleStartError {
    #[error("wild battle cannot start from unresolved encounter roll on map '{map_name}'")]
    UnresolvedEncounter { map_name: String },
    #[error("wild Pokemon build error: {0}")]
    PokemonBuild(#[from] PokemonBuildError),
}

pub fn wild_dvs_from_rng(rng: &mut Random) -> Dv {
    Dv::from_non_hp(
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
    )
}

pub fn wild_battle_start_from_encounter(
    encounter: WildEncounterRoll,
    species: &PokemonSpecies,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    rng: &mut Random,
) -> Result<WildBattleStart, WildBattleStartError> {
    let resolved =
        encounter
            .resolved
            .as_ref()
            .ok_or_else(|| WildBattleStartError::UnresolvedEncounter {
                map_name: encounter.map_name.clone(),
            })?;
    let level = resolved.level;
    let dvs = wild_dvs_from_rng(rng);
    let mut enemy_pokemon =
        create_pokemon_from_known_dvs(species, level, dvs, learnsets, moves, growth_rates)?;
    enemy_pokemon.original_trainer_name = "WILD".to_string();
    enemy_pokemon.original_trainer_id = 0;

    Ok(WildBattleStart {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        encounter,
        enemy_party: vec![enemy_pokemon.clone()],
        enemy_pokemon,
        rng_seed_after: rng.seed(),
    })
}

pub fn static_wild_battle_start(
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    request: StaticWildBattleRequest,
    rng: &mut Random,
) -> Result<StaticWildBattleStart, StaticWildBattleError> {
    if request.species.is_empty() {
        return Err(StaticWildBattleError::MissingSpecies);
    }
    if request.level == 0 {
        return Err(StaticWildBattleError::ZeroLevel {
            species: request.species,
        });
    }
    let species_data =
        species
            .get(&request.species)
            .ok_or_else(|| StaticWildBattleError::UnknownSpecies {
                species: request.species.clone(),
            })?;
    let dvs = if request.battle_type == "BATTLETYPE_FORCESHINY" {
        Dv::from_non_hp(14, 10, 10, 10)
    } else {
        wild_dvs_from_rng(rng)
    };
    let mut enemy_pokemon = create_pokemon_from_known_dvs(
        species_data,
        request.level,
        dvs,
        learnsets,
        moves,
        growth_rates,
    )?;
    enemy_pokemon.original_trainer_name = "WILD".to_string();
    enemy_pokemon.original_trainer_id = 0;
    if request.battle_type == "BATTLETYPE_FORCEITEM" {
        enemy_pokemon.item = species_data
            .item1
            .clone()
            .or_else(|| species_data.item2.clone());
    }

    Ok(StaticWildBattleStart {
        battle_type: request.battle_type,
        species: request.species,
        level: request.level,
        source_script: request.source_script,
        enemy_party: vec![enemy_pokemon.clone()],
        enemy_pokemon,
        rng_seed_after: rng.seed(),
    })
}

pub fn trainer_battle_start(
    state: &GameState,
    catalog: &TrainerCatalog,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    request: TrainerBattleRequest,
) -> Result<TrainerBattleStartStatus, TrainerBattleError> {
    if request.trainer_id.is_empty() {
        return Err(TrainerBattleError::MissingTrainerId);
    }
    if request.trainer_class.is_empty() {
        return Err(TrainerBattleError::MissingTrainerClass);
    }
    if !request.event_flag.is_empty() && state.flags.is_event_flag_set(&request.event_flag)? {
        return Ok(TrainerBattleStartStatus::AlreadyDefeated {
            event_flag: request.event_flag,
            callback: request.callback,
        });
    }

    let trainer =
        catalog
            .get(&request.trainer_id)
            .ok_or_else(|| TrainerBattleError::UnknownTrainer {
                trainer_id: request.trainer_id.clone(),
            })?;
    trainer_battle_start_from_trainer(trainer, species, learnsets, moves, growth_rates, request)
        .map(TrainerBattleStartStatus::Started)
}

pub fn trainer_battle_start_from_trainer(
    trainer: &Trainer,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    request: TrainerBattleRequest,
) -> Result<TrainerBattleStart, TrainerBattleError> {
    if trainer.trainer_class != request.trainer_class {
        return Err(TrainerBattleError::TrainerClassMismatch {
            trainer_id: request.trainer_id,
            requested: request.trainer_class,
            actual: trainer.trainer_class.clone(),
        });
    }
    if trainer.party.is_empty() {
        return Err(TrainerBattleError::EmptyParty {
            trainer_id: trainer.trainer_id.clone(),
        });
    };
    let enemy_party = materialize_trainer_party(trainer, species, learnsets, moves, growth_rates)?;
    let enemy_pokemon = enemy_party[0].clone();

    Ok(TrainerBattleStart {
        battle_type: request.battle_type,
        trainer_class: trainer.trainer_class.clone(),
        trainer_id: trainer.trainer_id.clone(),
        trainer_name: trainer.name.clone(),
        event_flag: request.event_flag,
        seen_text: request.seen_text,
        win_text: request.win_text,
        loss_text: request.loss_text,
        callback: request.callback,
        source_script: request.source_script,
        enemy_party,
        enemy_pokemon,
        reward: trainer.base_reward,
        encounter_music: trainer.encounter_music.clone(),
        ai_move_flags: trainer.ai_move_flags,
        ai_item_switch_flags: trainer.ai_item_switch_flags,
        ai_layers: trainer.ai_layers.clone(),
    })
}

pub fn materialize_trainer_party(
    trainer: &Trainer,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
) -> Result<Vec<Pokemon>, TrainerBattleError> {
    trainer
        .party
        .iter()
        .enumerate()
        .map(|(slot, party_mon)| {
            materialize_trainer_pokemon(
                trainer,
                slot,
                party_mon,
                species,
                learnsets,
                moves,
                growth_rates,
            )
        })
        .collect()
}

fn materialize_trainer_pokemon(
    trainer: &Trainer,
    slot: usize,
    party_mon: &TrainerPartyPokemon,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
) -> Result<Pokemon, TrainerBattleError> {
    if party_mon.species.is_empty() {
        return Err(TrainerBattleError::MissingPartySpecies {
            trainer_id: trainer.trainer_id.clone(),
            slot,
        });
    }
    let species_data =
        species
            .get(&party_mon.species)
            .ok_or_else(|| TrainerBattleError::UnknownPartySpecies {
                trainer_id: trainer.trainer_id.clone(),
                slot,
                species: party_mon.species.clone(),
            })?;
    let mut pokemon = create_pokemon_from_known_dvs(
        species_data,
        party_mon.level,
        party_mon.dvs,
        learnsets,
        moves,
        growth_rates,
    )?;
    pokemon.item = party_mon.item.clone();
    if !party_mon.moves.is_empty() {
        pokemon.moves = party_mon.moves.clone();
    }
    pokemon.original_trainer_name = trainer.name.clone();
    pokemon.original_trainer_id = 0;
    Ok(pokemon)
}

pub fn complete_trainer_battle(
    state: &mut GameState,
    currency_constants: &CurrencyCatalog,
    completion: &TrainerBattleCompletion,
) -> Result<TrainerBattleCompletionOutcome, TrainerBattleError> {
    let continued_after_battle = completion.won || completion.can_lose;
    let mut prize_money = 0;
    if completion.won {
        prize_money = trainer_prize_money_from_active_battle(state, completion)?;
        let max_money = trainer_battle_money_cap(currency_constants)?;
        state.money = state.money.saturating_add(prize_money).min(max_money);
    }
    if continued_after_battle && !completion.event_flag.is_empty() {
        state.flags.set_event_flag(&completion.event_flag, true)?;
    }
    if continued_after_battle {
        state.battle = BattleMemory::Inactive;
        state.battle_active_party_index = None;
        state.battle_active_enemy_party_index = None;
        state.battle_rewarded_enemy_party_indices.clear();
        state.battle_escape_attempts = 0;
        state.battle_player_stat_drop_guard_turns = 0;
    }
    Ok(TrainerBattleCompletionOutcome {
        continued_after_battle,
        prize_money,
        money_after: state.money,
    })
}

fn trainer_battle_money_cap(
    currency_constants: &CurrencyCatalog,
) -> Result<u32, TrainerBattleError> {
    currency_constants
        .get("MAX_MONEY")
        .ok_or_else(|| TrainerBattleError::MissingCurrencyLimit {
            constant: "MAX_MONEY".to_string(),
        })
}

fn trainer_prize_money_from_active_battle(
    state: &GameState,
    completion: &TrainerBattleCompletion,
) -> Result<u32, TrainerBattleError> {
    let BattleMemory::Trainer {
        trainer_class,
        trainer_id,
        enemy_party,
        reward,
        ..
    } = &state.battle
    else {
        return Err(TrainerBattleError::MissingActiveTrainerBattle);
    };
    if trainer_class != &completion.trainer_class || trainer_id != &completion.trainer_id {
        return Err(TrainerBattleError::CompletionTrainerMismatch {
            active_class: trainer_class.clone(),
            active_id: trainer_id.clone(),
            completion_class: completion.trainer_class.clone(),
            completion_id: completion.trainer_id.clone(),
        });
    }
    let level = enemy_party
        .last()
        .ok_or_else(|| TrainerBattleError::EmptyActiveTrainerParty {
            trainer_id: trainer_id.clone(),
        })?
        .level;
    if let Some((slot, _)) = enemy_party
        .iter()
        .enumerate()
        .find(|(_, pokemon)| pokemon.hp > 0)
    {
        return Err(TrainerBattleError::ActiveTrainerPartyNotDefeated {
            trainer_id: trainer_id.clone(),
            slot,
        });
    }
    if let Some(slot) = (0..enemy_party.len())
        .find(|slot| !state.battle_rewarded_enemy_party_indices.contains(slot))
    {
        return Err(TrainerBattleError::ActiveTrainerPartyRewardsUnclaimed {
            trainer_id: trainer_id.clone(),
            slot,
        });
    }
    reward
        .checked_mul(u32::from(level))
        .ok_or(TrainerBattleError::PrizeMoneyOverflow {
            reward: *reward,
            level,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, growth_rate};
    use crate::systems::experience::{GrowthRateCatalog, crystal_growth_rate_catalog_for_tests};
    use crate::world::encounters::{
        EncounterSurface, ResolvedWildEncounter, TimeOfDay, WildEncounter,
    };
    use crate::world::map::TilePosition;

    fn species() -> PokemonSpecies {
        PokemonSpecies {
            growth_rate: growth_rate("GROWTH_MEDIUM_FAST"),
            ..PokemonSpecies::new_for_tests("PIDGEY", BaseStats::new(40, 45, 40, 56, 35, 35))
        }
    }

    fn growth_rates() -> GrowthRateCatalog {
        crystal_growth_rate_catalog_for_tests()
    }

    fn trainer() -> Trainer {
        Trainer {
            name: "FALKNER@".to_string(),
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            party: vec![TrainerPartyPokemon {
                species: "PIDGEY".to_string(),
                level: 7,
                item: None,
                moves: Vec::new(),
                dvs: Dv::from_non_hp(0, 0, 0, 0),
            }],
            win_quote: "FALKNER: I won!".to_string(),
            lose_quote: "FALKNER: I lost!".to_string(),
            items: vec![Some("POTION".to_string())],
            base_reward: 50,
            ai_move_flags: 3,
            ai_item_switch_flags: 7,
            encounter_music: "MUSIC_HIKER_ENCOUNTER".to_string(),
            ai_layers: vec!["AI_BASIC".to_string()],
        }
    }

    fn species_table() -> BTreeMap<String, PokemonSpecies> {
        BTreeMap::from([("PIDGEY".to_string(), species())])
    }

    fn learnsets() -> SpeciesLearnsets {
        [("PIDGEY".to_string(), Vec::new())].into_iter().collect()
    }

    fn currency_constants(max_money: u32) -> CurrencyCatalog {
        CurrencyCatalog([("MAX_MONEY".to_string(), max_money)].into_iter().collect())
    }

    fn species_with_item() -> PokemonSpecies {
        PokemonSpecies {
            item1: Some("SILVER_WING".to_string()),
            ..species()
        }
    }

    fn encounter() -> WildEncounterRoll {
        WildEncounterRoll {
            map_name: "Route29".to_string(),
            tile: TilePosition::new(1, 1),
            surface: EncounterSurface::Grass,
            time: TimeOfDay::Day,
            threshold: 25,
            encounter_roll: 12,
            slot_percent_roll: Some(1),
            level_roll: Some(0),
            resolved: Some(ResolvedWildEncounter {
                encounter: WildEncounter {
                    level: 2,
                    species: "PIDGEY".to_string(),
                },
                slot: 0,
                level: 2,
            }),
            repelled_by: None,
            rng_seed_after: 123,
        }
    }

    #[test]
    fn wild_battle_start_uses_rng_dvs_and_wild_ot() {
        let mut rng = Random::new(1);
        let start = wild_battle_start_from_encounter(
            encounter(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect("resolved encounter starts battle");

        assert_eq!(start.battle_type, "BATTLETYPE_NORMAL");
        assert_eq!(start.enemy_pokemon.species.id, "PIDGEY");
        assert_eq!(start.enemy_pokemon.level, 2);
        assert_eq!(start.enemy_pokemon.original_trainer_name, "WILD");
        assert_eq!(start.enemy_party, vec![start.enemy_pokemon.clone()]);
        assert_eq!(start.enemy_pokemon.dvs, Dv::from_non_hp(4, 8, 5, 15));
        assert_eq!(start.rng_seed_after, rng.seed());
        assert_eq!(
            BattleMemory::from(&start),
            BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: start.enemy_pokemon.clone(),
                enemy_party: start.enemy_party.clone(),
            }
        );
    }

    #[test]
    fn wild_battle_start_rejects_unresolved_encounter_without_level_fallback() {
        let mut rng = Random::new(1);
        let mut encounter = encounter();
        encounter.resolved = None;

        let error = wild_battle_start_from_encounter(
            encounter,
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect_err("unresolved encounter must not start as level 1");

        assert_eq!(
            error,
            WildBattleStartError::UnresolvedEncounter {
                map_name: "Route29".to_string(),
            }
        );
    }

    #[test]
    fn trainer_battle_start_uses_exact_catalog_identity() {
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let mut request = TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER");
        request.seen_text = "FalknerSeenText".to_string();
        request.win_text = "FalknerBeatenText".to_string();
        request.callback = ".Script".to_string();

        let start = trainer_battle_start(
            &GameState::default(),
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            request,
        )
        .expect("battle start resolves");

        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should not be defeated");
        };
        assert_eq!(start.battle_type, "BATTLETYPE_TRAINER");
        assert_eq!(start.trainer_class, "FALKNER");
        assert_eq!(start.trainer_id, "FALKNER1");
        assert_eq!(start.event_flag, "EVENT_BEAT_FALKNER");
        assert_eq!(start.enemy_party.len(), 1);
        assert_eq!(start.enemy_pokemon.species.id, "PIDGEY");
        assert_eq!(start.reward, 50);
        assert_eq!(start.ai_layers, vec!["AI_BASIC"]);
        assert_eq!(
            BattleMemory::from(&start),
            BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "FALKNER".to_string(),
                trainer_id: "FALKNER1".to_string(),
                trainer_name: "FALKNER@".to_string(),
                event_flag: "EVENT_BEAT_FALKNER".to_string(),
                seen_text: "FalknerSeenText".to_string(),
                win_text: "FalknerBeatenText".to_string(),
                loss_text: String::new(),
                callback: ".Script".to_string(),
                source_script: String::new(),
                enemy_pokemon: start.enemy_pokemon.clone(),
                enemy_party: start.enemy_party.clone(),
                reward: 50,
                encounter_music: "MUSIC_HIKER_ENCOUNTER".to_string(),
                ai_move_flags: 3,
                ai_item_switch_flags: 7,
                ai_layers: vec!["AI_BASIC".to_string()],
            }
        );
    }

    #[test]
    fn static_wild_battle_start_uses_forced_shiny_dvs_without_species_coercion() {
        let mut rng = Random::new(1);
        let mut request = StaticWildBattleRequest::new("PIDGEY", 30);
        request.battle_type = "BATTLETYPE_FORCESHINY".to_string();
        request.source_script = "LakeOfRageRedGyarados".to_string();

        let start = static_wild_battle_start(
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            request,
            &mut rng,
        )
        .expect("static wild battle starts");

        assert_eq!(start.battle_type, "BATTLETYPE_FORCESHINY");
        assert_eq!(start.enemy_pokemon.species.id, "PIDGEY");
        assert_eq!(start.enemy_pokemon.level, 30);
        assert_eq!(start.enemy_pokemon.dvs, Dv::from_non_hp(14, 10, 10, 10));
        assert_eq!(start.enemy_pokemon.original_trainer_name, "WILD");
        assert_eq!(
            BattleMemory::from(&start),
            BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_FORCESHINY".to_string(),
                species: "PIDGEY".to_string(),
                level: 30,
                source_script: "LakeOfRageRedGyarados".to_string(),
                enemy_pokemon: start.enemy_pokemon.clone(),
                enemy_party: start.enemy_party.clone(),
            }
        );
    }

    #[test]
    fn static_wild_battle_start_forceitem_uses_exact_species_item() {
        let mut rng = Random::new(1);
        let mut request = StaticWildBattleRequest::new("PIDGEY", 60);
        request.battle_type = "BATTLETYPE_FORCEITEM".to_string();

        let start = static_wild_battle_start(
            &BTreeMap::from([("PIDGEY".to_string(), species_with_item())]),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            request,
            &mut rng,
        )
        .expect("static wild battle starts");

        assert_eq!(start.enemy_pokemon.item.as_deref(), Some("SILVER_WING"));
        assert_eq!(start.enemy_party, vec![start.enemy_pokemon.clone()]);
    }

    #[test]
    fn static_wild_battle_start_rejects_case_changed_species() {
        let mut rng = Random::new(1);
        let error = static_wild_battle_start(
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            StaticWildBattleRequest::new("pidgey", 30),
            &mut rng,
        )
        .expect_err("species ids must match exactly");

        assert_eq!(
            error,
            StaticWildBattleError::UnknownSpecies {
                species: "pidgey".to_string(),
            }
        );
    }

    #[test]
    fn trainer_battle_start_rejects_class_aliases() {
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");

        let error = trainer_battle_start(
            &GameState::default(),
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("falkner", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect_err("class must match exactly");

        assert_eq!(
            error,
            TrainerBattleError::TrainerClassMismatch {
                trainer_id: "FALKNER1".to_string(),
                requested: "falkner".to_string(),
                actual: "FALKNER".to_string(),
            }
        );
    }

    #[test]
    fn defeated_trainer_does_not_start_battle() {
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let mut state = GameState::default();
        state
            .flags
            .set_event_flag("EVENT_BEAT_FALKNER", true)
            .expect("flag sets");

        let start = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect("defeated status resolves");

        assert_eq!(
            start,
            TrainerBattleStartStatus::AlreadyDefeated {
                event_flag: "EVENT_BEAT_FALKNER".to_string(),
                callback: String::new(),
            }
        );
    }

    #[test]
    fn completing_trainer_battle_sets_exact_defeated_flag_only_after_continuing() {
        let mut state = GameState::default();
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let start = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect("battle start resolves");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should start");
        };
        state.battle = BattleMemory::from(&start);
        if let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &mut state.battle
        {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        state.battle_rewarded_enemy_party_indices.insert(0);
        let completion = TrainerBattleCompletion {
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            event_flag: "EVENT_BEAT_FALKNER".to_string(),
            won: true,
            can_lose: false,
        };

        let max_money = 999_999;
        let currency_constants = currency_constants(max_money);
        state.money = max_money - 100;
        let outcome = complete_trainer_battle(&mut state, &currency_constants, &completion)
            .expect("completion resolves");
        assert!(outcome.continued_after_battle);
        assert_eq!(outcome.prize_money, 350);
        assert_eq!(outcome.money_after, max_money);
        assert!(
            state
                .flags
                .is_event_flag_set("EVENT_BEAT_FALKNER")
                .expect("flag reads")
        );
        assert_eq!(state.battle, BattleMemory::Inactive);

        state.battle = BattleMemory::from(&start);
        let loss = TrainerBattleCompletion {
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            event_flag: "EVENT_BEAT_FALKNER_LOSS".to_string(),
            won: false,
            can_lose: false,
        };

        let loss_outcome =
            complete_trainer_battle(&mut state, &currency_constants, &loss).expect("loss resolves");
        assert!(!loss_outcome.continued_after_battle);
        assert_eq!(loss_outcome.prize_money, 0);
        assert_eq!(loss_outcome.money_after, state.money);
        assert_eq!(state.battle, BattleMemory::from(&start));
    }

    #[test]
    fn trainer_victory_completion_requires_pack_max_money_without_reward_fallback() {
        let mut state = GameState::default();
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let start = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect("battle start resolves");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should start");
        };
        state.battle = BattleMemory::from(&start);
        if let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &mut state.battle
        {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        state.battle_rewarded_enemy_party_indices.insert(0);
        let active_battle = state.battle.clone();
        let completion = TrainerBattleCompletion {
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            event_flag: "EVENT_BEAT_FALKNER".to_string(),
            won: true,
            can_lose: false,
        };

        assert_eq!(
            complete_trainer_battle(&mut state, &CurrencyCatalog::default(), &completion),
            Err(TrainerBattleError::MissingCurrencyLimit {
                constant: "MAX_MONEY".to_string(),
            })
        );
        assert_eq!(state.money, 0);
        assert_eq!(state.battle, active_battle);
        assert!(!state.battle_rewarded_enemy_party_indices.is_empty());
    }

    #[test]
    fn trainer_completion_rejects_missing_or_mismatched_active_trainer_battle() {
        let mut state = GameState::default();
        let completion = TrainerBattleCompletion {
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            event_flag: "EVENT_BEAT_FALKNER".to_string(),
            won: true,
            can_lose: false,
        };

        assert_eq!(
            complete_trainer_battle(&mut state, &currency_constants(999_999), &completion),
            Err(TrainerBattleError::MissingActiveTrainerBattle)
        );

        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let start = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect("battle start resolves");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should start");
        };
        state.battle = BattleMemory::from(&start);
        let mismatch = TrainerBattleCompletion {
            trainer_id: "BUG_CATCHER1".to_string(),
            trainer_class: "BUG_CATCHER".to_string(),
            event_flag: "EVENT_BEAT_BUG_CATCHER".to_string(),
            won: true,
            can_lose: false,
        };

        assert!(matches!(
            complete_trainer_battle(&mut state, &currency_constants(999_999), &mismatch),
            Err(TrainerBattleError::CompletionTrainerMismatch { .. })
        ));
    }

    #[test]
    fn trainer_victory_completion_requires_defeated_compiled_party() {
        let mut state = GameState::default();
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let start = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect("battle start resolves");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should start");
        };
        state.battle = BattleMemory::from(&start);
        let completion = TrainerBattleCompletion {
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            event_flag: "EVENT_BEAT_FALKNER".to_string(),
            won: true,
            can_lose: false,
        };

        assert_eq!(
            complete_trainer_battle(&mut state, &currency_constants(999_999), &completion),
            Err(TrainerBattleError::ActiveTrainerPartyNotDefeated {
                trainer_id: "FALKNER1".to_string(),
                slot: 0
            })
        );
        assert_eq!(state.money, 0);
        assert_eq!(state.battle, BattleMemory::from(&start));

        if let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &mut state.battle
        {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        assert_eq!(
            complete_trainer_battle(&mut state, &currency_constants(999_999), &completion),
            Err(TrainerBattleError::ActiveTrainerPartyRewardsUnclaimed {
                trainer_id: "FALKNER1".to_string(),
                slot: 0
            })
        );
        assert_eq!(state.money, 0);
    }
}
