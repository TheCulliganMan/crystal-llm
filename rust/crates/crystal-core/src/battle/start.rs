use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::{
    Dv, Move, Pokemon, PokemonBuildError, PokemonSpecies, Trainer, TrainerCatalog,
    TrainerPartyPokemon, create_pokemon_from_known_dvs,
};
use crate::battle::turn::{BattleSide, switch_battle_combat_pokemon};
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
    pub battle_music: String,
    pub encounter: WildEncounterRoll,
    pub enemy_pokemon: Pokemon,
    pub enemy_party: Vec<Pokemon>,
    pub rng_seed_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticWildBattleRequest {
    pub battle_type: String,
    pub battle_music: String,
    pub species: String,
    pub level: u8,
    pub source_script: String,
}

impl StaticWildBattleRequest {
    pub fn new(species: impl Into<String>, level: u8) -> Self {
        Self {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: String::new(),
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
    pub battle_music: String,
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
            battle_music: start.battle_music.clone(),
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
            battle_music: start.battle_music.clone(),
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
    #[error("trainer request has invalid trainer_id '{trainer_id}'")]
    InvalidTrainerId { trainer_id: String },
    #[error("trainer request is missing exact trainer_class")]
    MissingTrainerClass,
    #[error("trainer request has invalid trainer_class '{trainer_class}'")]
    InvalidTrainerClass { trainer_class: String },
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
    #[error("trainer '{trainer_id}' party slot {slot} has invalid species '{species}'")]
    InvalidPartySpecies {
        trainer_id: String,
        slot: usize,
        species: String,
    },
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
#[serde(deny_unknown_fields)]
pub enum ActiveBattlePartyError {
    #[error("active battle party index is not set")]
    MissingActivePartyIndex,
    #[error("active battle party index {index} is outside the party")]
    PartyIndexOutOfRange { index: usize },
    #[error("active battle party index {index} has no Pokemon")]
    EmptyPartySlot { index: usize },
    #[error("active battle party index {index} is fainted")]
    FaintedPartySlot { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ActiveBattleEnemyError {
    #[error("active enemy party index is not set")]
    MissingActiveEnemyPartyIndex,
    #[error("active enemy party index {index} is outside battle enemy party")]
    EnemyPartyIndexOutOfRange { index: usize },
    #[error("cannot update inactive battle enemy")]
    InactiveBattle,
    #[error("cannot advance trainer battle without an active trainer battle")]
    MissingActiveTrainerBattle,
    #[error("trainer battle enemy party index {index} rewards have not been claimed")]
    RewardsUnclaimed { index: usize },
    #[error("trainer battle enemy party index {index} rewards already claimed")]
    RewardsAlreadyClaimed { index: usize },
    #[error("cannot advance trainer battle before active enemy fainted")]
    ActiveEnemyNotFainted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerBattleAdvanceOutcome {
    pub next_enemy: Option<Pokemon>,
    pub trainer_defeated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum BattleStateItemError {
    #[error("cannot use battle state item without an active battle")]
    InactiveBattle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStatDropGuardOutcome {
    pub turns_before: u8,
    pub turns_after: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum StaticWildBattleError {
    #[error("static wild battle request is missing exact species id")]
    MissingSpecies,
    #[error("static wild battle request is missing exact battle music id")]
    MissingBattleMusic,
    #[error("static wild battle request has invalid battle music id '{battle_music}'")]
    InvalidBattleMusic { battle_music: String },
    #[error("static wild battle request has invalid species id '{species}'")]
    InvalidSpecies { species: String },
    #[error("unknown static wild species '{species}'")]
    UnknownSpecies { species: String },
    #[error("static wild battle level cannot be zero for species '{species}'")]
    ZeroLevel { species: String },
    #[error("static wild Pokemon build error: {0}")]
    PokemonBuild(#[from] PokemonBuildError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum WildBattleStartError {
    #[error("wild battle cannot start from unresolved encounter roll on map '{map_name}'")]
    UnresolvedEncounter { map_name: String },
    #[error("wild battle request is missing exact battle music id")]
    MissingBattleMusic,
    #[error("wild battle request has invalid battle music id '{battle_music}'")]
    InvalidBattleMusic { battle_music: String },
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
    battle_music: String,
    species: &PokemonSpecies,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    growth_rates: &GrowthRateCatalog,
    rng: &mut Random,
) -> Result<WildBattleStart, WildBattleStartError> {
    if battle_music.is_empty() {
        return Err(WildBattleStartError::MissingBattleMusic);
    }
    validate_battle_start_token(&battle_music).map_err(|_| {
        WildBattleStartError::InvalidBattleMusic {
            battle_music: battle_music.clone(),
        }
    })?;
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
        battle_music,
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
    if request.battle_music.is_empty() {
        return Err(StaticWildBattleError::MissingBattleMusic);
    }
    validate_battle_start_token(&request.battle_music).map_err(|_| {
        StaticWildBattleError::InvalidBattleMusic {
            battle_music: request.battle_music.clone(),
        }
    })?;
    if request.species.is_empty() {
        return Err(StaticWildBattleError::MissingSpecies);
    }
    validate_battle_start_token(&request.species).map_err(|_| {
        StaticWildBattleError::InvalidSpecies {
            species: request.species.clone(),
        }
    })?;
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
        battle_music: request.battle_music,
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
    validate_battle_start_token(&request.trainer_id).map_err(|_| {
        TrainerBattleError::InvalidTrainerId {
            trainer_id: request.trainer_id.clone(),
        }
    })?;
    if request.trainer_class.is_empty() {
        return Err(TrainerBattleError::MissingTrainerClass);
    }
    validate_battle_start_token(&request.trainer_class).map_err(|_| {
        TrainerBattleError::InvalidTrainerClass {
            trainer_class: request.trainer_class.clone(),
        }
    })?;
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
    validate_battle_start_token(&party_mon.species).map_err(|_| {
        TrainerBattleError::InvalidPartySpecies {
            trainer_id: trainer.trainer_id.clone(),
            slot,
            species: party_mon.species.clone(),
        }
    })?;
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

fn validate_battle_start_token(value: &str) -> Result<(), ()> {
    if !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        Ok(())
    } else {
        Err(())
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

pub fn complete_trainer_battle(
    state: &mut GameState,
    currency_constants: &CurrencyCatalog,
    completion: &TrainerBattleCompletion,
) -> Result<TrainerBattleCompletionOutcome, TrainerBattleError> {
    let continued_after_battle = completion.won || completion.can_lose;
    state.battle_result = if completion.won { 0 } else { 1 };
    let mut prize_money = 0;
    if completion.won {
        let max_money = trainer_battle_money_cap(currency_constants)?;
        let mut reward_unit = trainer_prize_money_from_active_battle(state, completion)?;
        if state.battle_amulet_coin_active {
            reward_unit = double_asm_battle_money(reward_unit);
        }
        let mom_shares = usize::from(state.mom_saving_some_money && state.moms_money < max_money);
        let wallet_shares = 4usize.saturating_sub(mom_shares);
        for _ in 0..mom_shares {
            state.moms_money = state.moms_money.saturating_add(reward_unit).min(max_money);
        }
        for _ in 0..wallet_shares {
            state.money = state.money.saturating_add(reward_unit).min(max_money);
        }
        prize_money = double_asm_battle_money(double_asm_battle_money(reward_unit));

        let mut pay_day_money = state.battle_pay_day_money.min(ASM_MAX_BATTLE_MONEY);
        if state.battle_amulet_coin_active {
            pay_day_money = double_asm_battle_money(pay_day_money);
        }
        state.money = state.money.saturating_add(pay_day_money).min(max_money);
    }
    if continued_after_battle && !completion.event_flag.is_empty() {
        state.flags.set_event_flag(&completion.event_flag, true)?;
    }
    if continued_after_battle {
        if completion.won {
            state.spread_pokerus_after_battle();
        }
        deactivate_battle(state);
    }
    Ok(TrainerBattleCompletionOutcome {
        continued_after_battle,
        prize_money,
        money_after: state.money,
    })
}

pub fn activate_wild_battle_start(state: &mut GameState, start: &WildBattleStart) {
    state.battle = BattleMemory::from(start);
    state.pokedex.record_seen_pokemon(&start.enemy_pokemon);
    reset_active_battle_slots(state);
}

pub fn activate_static_wild_battle_start(state: &mut GameState, start: &StaticWildBattleStart) {
    state.battle = BattleMemory::from(start);
    state.pokedex.record_seen_pokemon(&start.enemy_pokemon);
    reset_active_battle_slots(state);
}

pub fn activate_trainer_battle_start_status(
    state: &mut GameState,
    start: &TrainerBattleStartStatus,
) {
    if let TrainerBattleStartStatus::Started(started) = start {
        state.battle = BattleMemory::from(started);
        state.pokedex.record_seen_pokemon(&started.enemy_pokemon);
        reset_active_battle_slots(state);
    }
}

pub fn first_available_battle_party_index(state: &GameState) -> Option<usize> {
    state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .find_map(|(index, pokemon)| {
            let pokemon = pokemon.as_ref()?;
            (pokemon.hp > 0).then_some(index)
        })
}

pub fn require_active_battle_party_index(
    state: &GameState,
) -> Result<usize, ActiveBattlePartyError> {
    let index = state
        .battle_active_party_index
        .ok_or(ActiveBattlePartyError::MissingActivePartyIndex)?;
    validate_active_battle_party_index(state, index)?;
    Ok(index)
}

pub fn switch_active_battle_party_index(
    state: &mut GameState,
    index: usize,
) -> Result<usize, ActiveBattlePartyError> {
    validate_active_battle_party_index(state, index)?;
    state.battle_active_party_index = Some(index);
    mark_active_party_participant(state);
    activate_amulet_coin_for_active_party(state);
    Ok(index)
}

pub fn validate_active_battle_party_index(
    state: &GameState,
    index: usize,
) -> Result<(), ActiveBattlePartyError> {
    if index >= state.storage.party.pokemon.len() {
        return Err(ActiveBattlePartyError::PartyIndexOutOfRange { index });
    }
    let pokemon = state.storage.party.pokemon[index]
        .as_ref()
        .ok_or(ActiveBattlePartyError::EmptyPartySlot { index })?;
    if pokemon.hp == 0 {
        return Err(ActiveBattlePartyError::FaintedPartySlot { index });
    }
    Ok(())
}

pub fn require_active_battle_enemy_party_index(
    state: &GameState,
) -> Result<usize, ActiveBattleEnemyError> {
    state
        .battle_active_enemy_party_index
        .ok_or(ActiveBattleEnemyError::MissingActiveEnemyPartyIndex)
}

pub fn update_active_battle_enemy(
    state: &mut GameState,
    enemy_pokemon: Pokemon,
) -> Result<(), ActiveBattleEnemyError> {
    let active_enemy_index = require_active_battle_enemy_party_index(state)?;
    match &mut state.battle {
        BattleMemory::Wild {
            enemy_pokemon: active,
            enemy_party,
            ..
        }
        | BattleMemory::StaticWild {
            enemy_pokemon: active,
            enemy_party,
            ..
        }
        | BattleMemory::Trainer {
            enemy_pokemon: active,
            enemy_party,
            ..
        } => {
            let Some(party_entry) = enemy_party.get_mut(active_enemy_index) else {
                return Err(ActiveBattleEnemyError::EnemyPartyIndexOutOfRange {
                    index: active_enemy_index,
                });
            };
            *active = enemy_pokemon.clone();
            *party_entry = enemy_pokemon;
            Ok(())
        }
        BattleMemory::Inactive => Err(ActiveBattleEnemyError::InactiveBattle),
    }
}

pub fn advance_active_trainer_battle(
    state: &mut GameState,
) -> Result<TrainerBattleAdvanceOutcome, ActiveBattleEnemyError> {
    let current_enemy_index = require_active_battle_enemy_party_index(state)?;
    if !state
        .battle_rewarded_enemy_party_indices
        .contains(&current_enemy_index)
    {
        return Err(ActiveBattleEnemyError::RewardsUnclaimed {
            index: current_enemy_index,
        });
    }
    let BattleMemory::Trainer {
        enemy_pokemon,
        enemy_party,
        ..
    } = &mut state.battle
    else {
        return Err(ActiveBattleEnemyError::MissingActiveTrainerBattle);
    };
    if enemy_pokemon.hp != 0 {
        return Err(ActiveBattleEnemyError::ActiveEnemyNotFainted);
    }
    if current_enemy_index >= enemy_party.len() {
        return Err(ActiveBattleEnemyError::EnemyPartyIndexOutOfRange {
            index: current_enemy_index,
        });
    }
    enemy_party[current_enemy_index] = enemy_pokemon.clone();
    let next = enemy_party
        .iter()
        .enumerate()
        .find_map(|(index, pokemon)| {
            (index != current_enemy_index && pokemon.hp > 0)
                .then_some((index, pokemon.clone()))
        });
    if let Some((index, pokemon)) = next {
        *enemy_pokemon = pokemon.clone();
        state.battle_active_enemy_party_index = Some(index);
        if let Some(combat) = state.script_runtime.active_battle_combat.as_mut() {
            switch_battle_combat_pokemon(combat, BattleSide::Enemy, index).map_err(|_| {
                ActiveBattleEnemyError::EnemyPartyIndexOutOfRange { index }
            })?;
        }
        state.pokedex.record_seen_pokemon(&pokemon);
        Ok(TrainerBattleAdvanceOutcome {
            next_enemy: Some(pokemon),
            trainer_defeated: false,
        })
    } else {
        Ok(TrainerBattleAdvanceOutcome {
            next_enemy: None,
            trainer_defeated: true,
        })
    }
}

pub fn claim_active_trainer_battle_reward_index(
    state: &mut GameState,
) -> Result<usize, ActiveBattleEnemyError> {
    let enemy_index = require_active_battle_enemy_party_index(state)?;
    if !matches!(state.battle, BattleMemory::Trainer { .. }) {
        return Err(ActiveBattleEnemyError::MissingActiveTrainerBattle);
    }
    if state
        .battle_rewarded_enemy_party_indices
        .contains(&enemy_index)
    {
        return Err(ActiveBattleEnemyError::RewardsAlreadyClaimed { index: enemy_index });
    }
    state
        .battle_rewarded_enemy_party_indices
        .insert(enemy_index);
    Ok(enemy_index)
}

pub fn apply_battle_stat_drop_guard_turns(
    state: &mut GameState,
    turns: u8,
) -> Result<BattleStatDropGuardOutcome, BattleStateItemError> {
    require_active_battle_for_state_item(state)?;
    let turns_before = state.battle_player_stat_drop_guard_turns;
    state.battle_player_stat_drop_guard_turns = turns;
    Ok(BattleStatDropGuardOutcome {
        turns_before,
        turns_after: turns,
    })
}

pub fn require_active_battle_for_state_item(state: &GameState) -> Result<(), BattleStateItemError> {
    match state.battle {
        BattleMemory::Wild { .. }
        | BattleMemory::StaticWild { .. }
        | BattleMemory::Trainer { .. } => {}
        BattleMemory::Inactive => return Err(BattleStateItemError::InactiveBattle),
    }
    Ok(())
}

fn reset_active_battle_slots(state: &mut GameState) {
    state.battle_active_party_index = first_available_battle_party_index(state);
    state.battle_active_enemy_party_index = Some(0);
    state.battle_rewarded_enemy_party_indices.clear();
    state.battle_escape_attempts = 0;
    state.battle_player_stat_drop_guard_turns = 0;
    state.battle_pay_day_money = 0;
    state.battle_amulet_coin_active = false;
    state.script_runtime.active_battle_combat = None;
    mark_active_party_participant(state);
    activate_amulet_coin_for_active_party(state);
}

pub fn deactivate_battle(state: &mut GameState) {
    state.battle = BattleMemory::Inactive;
    state.script_runtime.active_battle_combat = None;
    clear_persistent_party_battle_state(state);
    state.sync_party_from_storage();
    clear_battle_participant_markers(state);
    clear_active_battle_slots(state);
}

/// BattleMon counters live in the transient WRAM battle structure in Crystal.
/// Only persistent status/HP/etc. survive a battle boundary; never leave
/// confusion, sleep countdowns, rampage, stat stages, or flinch state on the
/// saveable party records after a flee, victory, or loss.
fn clear_persistent_party_battle_state(state: &mut GameState) {
    for pokemon in state.storage.party.pokemon.iter_mut().flatten() {
        pokemon.sleep_turns = 0;
        pokemon.flinching = false;
        pokemon.rampage_turns = 0;
        pokemon.confusion_turns = 0;
        pokemon.perish_song_turns = 0;
        pokemon.focus_energy = false;
        pokemon.stat_boosts = crate::models::pokemon::default_stat_boosts();
    }
}

fn mark_active_party_participant(state: &mut GameState) {
    let Some(index) = state.battle_active_party_index else {
        return;
    };
    if let Some(pokemon) = state.storage.party.pokemon[index].as_mut() {
        pokemon.turns_in_battle = pokemon.turns_in_battle.saturating_add(1).max(1);
    }
}

pub(crate) fn activate_amulet_coin_for_active_party(state: &mut GameState) {
    if state.battle_amulet_coin_active {
        return;
    }
    let Some(index) = state.battle_active_party_index else {
        return;
    };
    if state
        .storage
        .party
        .pokemon
        .get(index)
        .and_then(Option::as_ref)
        .and_then(|pokemon| pokemon.item.as_deref())
        == Some("AMULET_COIN")
    {
        state.battle_amulet_coin_active = true;
    }
}

fn clear_battle_participant_markers(state: &mut GameState) {
    for pokemon in state.storage.party.pokemon.iter_mut().flatten() {
        pokemon.turns_in_battle = 0;
    }
    for pc_box in &mut state.storage.pc_boxes {
        for pokemon in pc_box.pokemon.iter_mut().flatten() {
            pokemon.turns_in_battle = 0;
        }
    }
}

pub fn clear_active_battle_slots(state: &mut GameState) {
    state.battle_active_party_index = None;
    state.battle_active_enemy_party_index = None;
    state.battle_rewarded_enemy_party_indices.clear();
    state.battle_escape_attempts = 0;
    state.battle_player_stat_drop_guard_turns = 0;
    state.battle_pay_day_money = 0;
    state.battle_amulet_coin_active = false;
}

const ASM_MAX_BATTLE_MONEY: u32 = 0x00ff_ffff;

fn double_asm_battle_money(amount: u32) -> u32 {
    amount.min(ASM_MAX_BATTLE_MONEY)
        .saturating_mul(2)
        .min(ASM_MAX_BATTLE_MONEY)
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
    Ok(reward
        .saturating_mul(u32::from(level))
        .min(ASM_MAX_BATTLE_MONEY))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Stat, growth_rate};
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
    fn battle_start_serialized_variants_reject_unknown_fallback_fields() {
        let status_error = serde_json::from_value::<TrainerBattleStartStatus>(serde_json::json!({
            "already_defeated": {
                "event_flag": "EVENT_BEAT_FALKNER",
                "callback": ".AfterBattle",
                "fallback_callback": ".Default"
            }
        }))
        .expect_err("trainer start status must not accept fallback callbacks");
        assert!(
            status_error
                .to_string()
                .contains("unknown field `fallback_callback`"),
            "{status_error}"
        );

        let static_error = serde_json::from_value::<StaticWildBattleError>(serde_json::json!({
            "UnknownSpecies": {
                "species": "PIKACHU",
                "fallback_species": "RATTATA"
            }
        }))
        .expect_err("static wild errors must not accept fallback species");
        assert!(
            static_error
                .to_string()
                .contains("unknown field `fallback_species`"),
            "{static_error}"
        );

        let wild_error = serde_json::from_value::<WildBattleStartError>(serde_json::json!({
            "UnresolvedEncounter": {
                "map_name": "Route29",
                "fallback_level": 1
            }
        }))
        .expect_err("wild battle errors must not accept fallback encounter levels");
        assert!(
            wild_error
                .to_string()
                .contains("unknown field `fallback_level`"),
            "{wild_error}"
        );
    }

    #[test]
    fn wild_battle_start_uses_rng_dvs_and_wild_ot() {
        let mut rng = Random::new(1);
        let start = wild_battle_start_from_encounter(
            encounter(),
            "MUSIC_JOHTO_WILD_BATTLE".to_string(),
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
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: start.enemy_pokemon.clone(),
                enemy_party: start.enemy_party.clone(),
            }
        );
    }

    #[test]
    fn activating_battle_start_sets_authoritative_runtime_battle_state() {
        let mut state = GameState::default();
        let mut fainted = Pokemon::new_for_tests(species(), 2, Dv::from_non_hp(0, 0, 0, 0));
        fainted.hp = 0;
        state
            .storage
            .register_capture(fainted)
            .expect("store fainted lead");
        state
            .storage
            .register_capture(Pokemon::new_for_tests(
                species(),
                3,
                Dv::from_non_hp(1, 1, 1, 1),
            ))
            .expect("store active party mon");
        let mut rng = Random::new(1);
        let start = wild_battle_start_from_encounter(
            encounter(),
            "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect("wild start");
        state.battle_rewarded_enemy_party_indices.insert(9);
        state.battle_escape_attempts = 3;
        state.battle_player_stat_drop_guard_turns = 4;
        state.battle_pay_day_money = 55;

        activate_wild_battle_start(&mut state, &start);

        assert_eq!(state.battle, BattleMemory::from(&start));
        assert!(state.pokedex.has_seen("PIDGEY"));
        assert_eq!(state.battle_active_party_index, Some(1));
        assert_eq!(state.battle_active_enemy_party_index, Some(0));
        assert!(state.battle_rewarded_enemy_party_indices.is_empty());
        assert_eq!(state.battle_escape_attempts, 0);
        assert_eq!(state.battle_player_stat_drop_guard_turns, 0);
        assert_eq!(state.battle_pay_day_money, 0);
    }

    #[test]
    fn deactivate_battle_clears_all_runtime_battle_bookkeeping() {
        let mut state = GameState::default();
        state
            .storage
            .register_capture(Pokemon::new_for_tests(
                species(),
                5,
                Dv::from_non_hp(1, 1, 1, 1),
            ))
            .expect("store active party mon");
        let mut rng = Random::new(1);
        let start = wild_battle_start_from_encounter(
            encounter(),
            "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect("wild start");
        activate_wild_battle_start(&mut state, &start);
        let active_index = state.battle_active_party_index.expect("active party");
        assert_eq!(
            state.storage.party.pokemon[active_index]
                .as_ref()
                .expect("active Pokemon")
                .turns_in_battle,
            1
        );
        state.battle_rewarded_enemy_party_indices.insert(0);
        state.battle_escape_attempts = 2;
        state.battle_player_stat_drop_guard_turns = 3;
        state.battle_pay_day_money = 40;
        {
            let pokemon = state.storage.party.pokemon[active_index]
                .as_mut()
                .expect("active Pokemon");
            pokemon.sleep_turns = 3;
            pokemon.flinching = true;
            pokemon.rampage_turns = 2;
            pokemon.confusion_turns = 4;
            pokemon.perish_song_turns = 2;
            pokemon.focus_energy = true;
            pokemon.stat_boosts.insert(Stat::Attack, 2);
        }

        deactivate_battle(&mut state);

        assert_eq!(state.battle, BattleMemory::Inactive);
        assert_eq!(state.battle_active_party_index, None);
        assert_eq!(state.battle_active_enemy_party_index, None);
        assert!(state.battle_rewarded_enemy_party_indices.is_empty());
        assert_eq!(state.battle_escape_attempts, 0);
        assert_eq!(state.battle_player_stat_drop_guard_turns, 0);
        assert_eq!(state.battle_pay_day_money, 0);
        assert!(
            state
                .storage
                .party
                .pokemon
                .iter()
                .flatten()
                .all(|pokemon| pokemon.turns_in_battle == 0)
        );
        let pokemon = state.storage.party.pokemon[active_index]
            .as_ref()
            .expect("active Pokemon after cleanup");
        assert_eq!(pokemon.sleep_turns, 0);
        assert!(!pokemon.flinching);
        assert_eq!(pokemon.rampage_turns, 0);
        assert_eq!(pokemon.confusion_turns, 0);
        assert_eq!(pokemon.perish_song_turns, 0);
        assert!(!pokemon.focus_energy);
        assert_eq!(
            pokemon.stat_boosts,
            crate::models::pokemon::default_stat_boosts()
        );
    }

    #[test]
    fn active_battle_party_switch_validates_target_before_mutation() {
        let mut state = GameState::default();
        let first = Pokemon::new_for_tests(species(), 2, Dv::from_non_hp(0, 0, 0, 0));
        state.storage.register_capture(first).expect("store first");
        let mut fainted = Pokemon::new_for_tests(species(), 3, Dv::from_non_hp(1, 1, 1, 1));
        fainted.hp = 0;
        state
            .storage
            .register_capture(fainted)
            .expect("store second");
        state.battle_active_party_index = Some(0);

        assert_eq!(
            switch_active_battle_party_index(&mut state, 2),
            Err(ActiveBattlePartyError::EmptyPartySlot { index: 2 })
        );
        assert_eq!(state.battle_active_party_index, Some(0));
        assert_eq!(
            switch_active_battle_party_index(&mut state, 1),
            Err(ActiveBattlePartyError::FaintedPartySlot { index: 1 })
        );
        assert_eq!(state.battle_active_party_index, Some(0));

        state
            .storage
            .register_capture(Pokemon::new_for_tests(
                species(),
                4,
                Dv::from_non_hp(2, 2, 2, 2),
            ))
            .expect("store third");

        assert_eq!(switch_active_battle_party_index(&mut state, 2), Ok(2));
        assert_eq!(require_active_battle_party_index(&state), Ok(2));
    }

    #[test]
    fn active_battle_index_helpers_reject_missing_indices() {
        let state = GameState::default();

        assert_eq!(
            require_active_battle_party_index(&state),
            Err(ActiveBattlePartyError::MissingActivePartyIndex)
        );
        assert_eq!(
            require_active_battle_enemy_party_index(&state),
            Err(ActiveBattleEnemyError::MissingActiveEnemyPartyIndex)
        );
    }

    #[test]
    fn active_enemy_update_rewrites_battle_memory_and_party_slot() {
        let mut state = GameState::default();
        let mut rng = Random::new(1);
        let start = wild_battle_start_from_encounter(
            encounter(),
            "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect("wild start");
        activate_wild_battle_start(&mut state, &start);
        let mut updated = start.enemy_pokemon.clone();
        updated.hp = updated.hp.saturating_sub(1);

        update_active_battle_enemy(&mut state, updated.clone()).expect("enemy updates");

        let BattleMemory::Wild {
            enemy_pokemon,
            enemy_party,
            ..
        } = &state.battle
        else {
            panic!("expected wild battle");
        };
        assert_eq!(enemy_pokemon, &updated);
        assert_eq!(enemy_party.first(), Some(&updated));

        state.battle_active_enemy_party_index = Some(9);
        let before = state.battle.clone();
        assert_eq!(
            update_active_battle_enemy(&mut state, updated),
            Err(ActiveBattleEnemyError::EnemyPartyIndexOutOfRange { index: 9 })
        );
        assert_eq!(state.battle, before);
    }

    #[test]
    fn trainer_battle_advance_requires_rewards_and_promotes_next_enemy() {
        let first = Pokemon::new_for_tests(species(), 7, Dv::from_non_hp(0, 0, 0, 0));
        let second = Pokemon::new_for_tests(species(), 9, Dv::from_non_hp(1, 1, 1, 1));
        let start = TrainerBattleStart {
            battle_type: "BATTLETYPE_TRAINER".to_string(),
            trainer_class: "FALKNER".to_string(),
            trainer_id: "FALKNER1".to_string(),
            trainer_name: "FALKNER@".to_string(),
            event_flag: "EVENT_BEAT_FALKNER".to_string(),
            seen_text: String::new(),
            win_text: String::new(),
            loss_text: String::new(),
            callback: String::new(),
            source_script: "BattleScript".to_string(),
            enemy_pokemon: first.clone(),
            enemy_party: vec![first.clone(), second.clone()],
            reward: 50,
            encounter_music: "MUSIC_HIKER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };
        let mut state = GameState::default();
        state.battle = BattleMemory::from(&start);
        state.battle_active_enemy_party_index = Some(0);
        let player = Pokemon::new_for_tests(species(), 10, Dv::from_non_hp(2, 2, 2, 2));
        state.script_runtime.active_battle_combat = Some(
            crate::battle::turn::BattleCombatState::new(
                player.clone(),
                first.clone(),
                state.rng_seed,
            )
                .with_parties(vec![player], vec![first.clone(), second.clone()])
                .with_party_indices(0, 0),
        );

        assert_eq!(
            advance_active_trainer_battle(&mut state),
            Err(ActiveBattleEnemyError::RewardsUnclaimed { index: 0 })
        );

        let BattleMemory::Trainer { enemy_pokemon, .. } = &mut state.battle else {
            panic!("expected trainer battle");
        };
        enemy_pokemon.hp = 0;
        let combat = state
            .script_runtime
            .active_battle_combat
            .as_mut()
            .expect("active trainer combat");
        combat.enemy.hp = 0;
        combat.enemy_party[0].hp = 0;
        state.battle_rewarded_enemy_party_indices.insert(0);

        let outcome = advance_active_trainer_battle(&mut state).expect("advance trainer battle");

        assert_eq!(
            outcome,
            TrainerBattleAdvanceOutcome {
                next_enemy: Some(second.clone()),
                trainer_defeated: false,
            }
        );
        assert_eq!(state.battle_active_enemy_party_index, Some(1));
        assert!(state.pokedex.has_seen("PIDGEY"));
        let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &state.battle
        else {
            panic!("expected trainer battle");
        };
        assert_eq!(enemy_pokemon, &second);
        assert_eq!(enemy_party[0].hp, 0);
        let combat = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .expect("advanced trainer combat");
        assert_eq!(combat.enemy.species.id, second.species.id);
        assert_eq!(combat.enemy.hp, second.hp);
        assert_eq!(combat.enemy.turns_in_battle, 1);
        assert_eq!(combat.enemy_party_index, 1);

        // Trainer AI can switch forward before a knockout. Replacement must
        // still find a living earlier slot instead of treating the trainer as
        // defeated merely because no later index is usable.
        let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &mut state.battle
        else {
            panic!("expected trainer battle");
        };
        *enemy_pokemon = second.clone();
        enemy_pokemon.hp = 0;
        enemy_party[0] = first.clone();
        enemy_party[1] = enemy_pokemon.clone();
        let combat = state
            .script_runtime
            .active_battle_combat
            .as_mut()
            .expect("active trainer combat");
        combat.enemy.hp = 0;
        combat.enemy_party[0] = first.clone();
        combat.enemy_party[1].hp = 0;
        state.battle_rewarded_enemy_party_indices.insert(1);

        let reordered =
            advance_active_trainer_battle(&mut state).expect("find earlier living trainer slot");
        assert_eq!(reordered.next_enemy, Some(first));
        assert!(!reordered.trainer_defeated);
        assert_eq!(state.battle_active_enemy_party_index, Some(0));
    }

    #[test]
    fn trainer_reward_index_claim_is_core_owned_and_duplicate_checked() {
        let start = TrainerBattleStart {
            battle_type: "BATTLETYPE_TRAINER".to_string(),
            trainer_class: "FALKNER".to_string(),
            trainer_id: "FALKNER1".to_string(),
            trainer_name: "FALKNER@".to_string(),
            event_flag: "EVENT_BEAT_FALKNER".to_string(),
            seen_text: String::new(),
            win_text: String::new(),
            loss_text: String::new(),
            callback: String::new(),
            source_script: "BattleScript".to_string(),
            enemy_pokemon: Pokemon::new_for_tests(species(), 7, Dv::from_non_hp(0, 0, 0, 0)),
            enemy_party: vec![Pokemon::new_for_tests(
                species(),
                7,
                Dv::from_non_hp(0, 0, 0, 0),
            )],
            reward: 50,
            encounter_music: "MUSIC_HIKER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };
        let mut state = GameState::default();
        state.battle = BattleMemory::from(&start);
        state.battle_active_enemy_party_index = Some(0);

        assert_eq!(claim_active_trainer_battle_reward_index(&mut state), Ok(0));
        assert!(state.battle_rewarded_enemy_party_indices.contains(&0));
        assert_eq!(
            claim_active_trainer_battle_reward_index(&mut state),
            Err(ActiveBattleEnemyError::RewardsAlreadyClaimed { index: 0 })
        );
    }

    #[test]
    fn battle_stat_drop_guard_requires_active_battle_and_records_before_after_turns() {
        let mut inactive = GameState::default();
        assert_eq!(
            apply_battle_stat_drop_guard_turns(&mut inactive, 5),
            Err(BattleStateItemError::InactiveBattle)
        );
        assert_eq!(inactive.battle_player_stat_drop_guard_turns, 0);

        let mut state = GameState::default();
        let mut rng = Random::new(1);
        let start = wild_battle_start_from_encounter(
            encounter(),
            "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect("wild start");
        activate_wild_battle_start(&mut state, &start);
        state.battle_player_stat_drop_guard_turns = 2;

        let guard = apply_battle_stat_drop_guard_turns(&mut state, 5).expect("guard turns apply");

        assert_eq!(
            guard,
            BattleStatDropGuardOutcome {
                turns_before: 2,
                turns_after: 5,
            }
        );
        assert_eq!(state.battle_player_stat_drop_guard_turns, 5);
    }

    #[test]
    fn wild_battle_start_rejects_unresolved_encounter_without_level_fallback() {
        let mut rng = Random::new(1);
        let mut encounter = encounter();
        encounter.resolved = None;

        let error = wild_battle_start_from_encounter(
            encounter,
            "MUSIC_JOHTO_WILD_BATTLE".to_string(),
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
    fn wild_battle_start_rejects_missing_or_malformed_battle_music() {
        let mut rng = Random::new(1);
        let missing = wild_battle_start_from_encounter(
            encounter(),
            String::new(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect_err("wild battle music must be explicit");
        assert_eq!(missing, WildBattleStartError::MissingBattleMusic);

        let malformed = wild_battle_start_from_encounter(
            encounter(),
            "MUSIC JOHTO WILD BATTLE".to_string(),
            &species(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            &mut rng,
        )
        .expect_err("wild battle music must be an exact token");
        assert_eq!(
            malformed,
            WildBattleStartError::InvalidBattleMusic {
                battle_music: "MUSIC JOHTO WILD BATTLE".to_string(),
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
        request.battle_music = "MUSIC_JOHTO_WILD_BATTLE".to_string();
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
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
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
        request.battle_music = "MUSIC_JOHTO_WILD_BATTLE".to_string();

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
            {
                let mut request = StaticWildBattleRequest::new("pidgey", 30);
                request.battle_music = "MUSIC_JOHTO_WILD_BATTLE".to_string();
                request
            },
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
    fn static_wild_battle_start_rejects_malformed_species_before_unknown_lookup() {
        let mut rng = Random::new(1);
        let error = static_wild_battle_start(
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            {
                let mut request = StaticWildBattleRequest::new("PID GEY", 30);
                request.battle_music = "MUSIC_JOHTO_WILD_BATTLE".to_string();
                request
            },
            &mut rng,
        )
        .expect_err("malformed species ids are invalid pack input");

        assert_eq!(
            error,
            StaticWildBattleError::InvalidSpecies {
                species: "PID GEY".to_string(),
            }
        );
    }

    #[test]
    fn static_wild_battle_start_rejects_missing_or_malformed_battle_music() {
        let mut rng = Random::new(1);
        let missing = static_wild_battle_start(
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            StaticWildBattleRequest::new("PIDGEY", 30),
            &mut rng,
        )
        .expect_err("static wild battle music must be explicit");
        assert_eq!(missing, StaticWildBattleError::MissingBattleMusic);

        let mut request = StaticWildBattleRequest::new("PIDGEY", 30);
        request.battle_music = "MUSIC JOHTO WILD BATTLE".to_string();
        let malformed = static_wild_battle_start(
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            request,
            &mut rng,
        )
        .expect_err("static wild battle music must be an exact token");
        assert_eq!(
            malformed,
            StaticWildBattleError::InvalidBattleMusic {
                battle_music: "MUSIC JOHTO WILD BATTLE".to_string(),
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
    fn trainer_battle_start_rejects_malformed_identity_before_defeated_lookup() {
        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let mut state = GameState::default();
        state
            .flags
            .set_event_flag("EVENT_BEAT_FALKNER", true)
            .expect("flag sets");

        let error = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALK NER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect_err("malformed class must not be hidden by defeated event");
        assert_eq!(
            error,
            TrainerBattleError::InvalidTrainerClass {
                trainer_class: "FALK NER".to_string(),
            }
        );

        let error = trainer_battle_start(
            &state,
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("FALKNER", "FALK NER1", "EVENT_BEAT_FALKNER"),
        )
        .expect_err("malformed trainer id must not be hidden by defeated event");
        assert_eq!(
            error,
            TrainerBattleError::InvalidTrainerId {
                trainer_id: "FALK NER1".to_string(),
            }
        );
    }

    #[test]
    fn trainer_party_materialization_rejects_malformed_species_before_unknown_lookup() {
        let mut trainer = trainer();
        trainer.party[0].species = "PID GEY".to_string();

        let error = materialize_trainer_party(
            &trainer,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
        )
        .expect_err("malformed trainer party species are invalid pack input");

        assert_eq!(
            error,
            TrainerBattleError::InvalidPartySpecies {
                trainer_id: "FALKNER1".to_string(),
                slot: 0,
                species: "PID GEY".to_string(),
            }
        );
    }

    #[test]
    fn battle_start_rejects_reserved_pack_prefix_tokens() {
        let mut rng = Random::new(1);
        let static_error = static_wild_battle_start(
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            {
                let mut request = StaticWildBattleRequest::new("fallbackPIDGEY", 30);
                request.battle_music = "MUSIC_JOHTO_WILD_BATTLE".to_string();
                request
            },
            &mut rng,
        )
        .expect_err("reserved static species ids are invalid pack input");
        assert_eq!(
            static_error,
            StaticWildBattleError::InvalidSpecies {
                species: "fallbackPIDGEY".to_string(),
            }
        );

        let mut catalog = TrainerCatalog::default();
        catalog.insert(trainer()).expect("trainer inserts");
        let trainer_error = trainer_battle_start(
            &GameState::default(),
            &catalog,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
            TrainerBattleRequest::new("legacyFALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect_err("reserved trainer classes are invalid pack input");
        assert_eq!(
            trainer_error,
            TrainerBattleError::InvalidTrainerClass {
                trainer_class: "legacyFALKNER".to_string(),
            }
        );

        let mut trainer = trainer();
        trainer.party[0].species = "fallbackPIDGEY".to_string();
        let party_error = materialize_trainer_party(
            &trainer,
            &species_table(),
            &learnsets(),
            &BTreeMap::new(),
            &growth_rates(),
        )
        .expect_err("reserved trainer party species are invalid pack input");
        assert_eq!(
            party_error,
            TrainerBattleError::InvalidPartySpecies {
                trainer_id: "FALKNER1".to_string(),
                slot: 0,
                species: "fallbackPIDGEY".to_string(),
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
        assert_eq!(outcome.prize_money, 1400);
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
