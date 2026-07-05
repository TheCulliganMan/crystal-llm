use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use thiserror::Error;

use crate::battle::start::{
    ActiveBattleEnemyError, ActiveBattlePartyError, claim_active_trainer_battle_reward_index,
    deactivate_battle, require_active_battle_party_index, update_active_battle_enemy,
};
use crate::models::pokemon::StatExperience;
use crate::models::{LearnedMove, Move, Pokemon, PokemonSpecies, calculate_stats};
use crate::state::{BattleMemory, GameState, PendingMoveLearn};
use crate::systems::evolution::{
    EvolutionError, EvolutionReport, EvolutionTable, check_and_evolve,
};
use crate::systems::experience::{ExperienceError, GrowthRateCatalog, calculate_experience};
use crate::systems::learnsets::{LearnsetError, SpeciesLearnsets, level_up_moves_for_species};
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleRewardRules {
    pub max_level: u8,
    pub wild_exp_divisor: i32,
    pub trainer_exp_numerator: i32,
    pub trainer_exp_denominator: i32,
}

impl<'de> Deserialize<'de> for BattleRewardRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleRewardRules {
            max_level: u8,
            wild_exp_divisor: i32,
            trainer_exp_numerator: i32,
            trainer_exp_denominator: i32,
        }

        let raw = RawBattleRewardRules::deserialize(deserializer)?;
        let rules = Self {
            max_level: raw.max_level,
            wild_exp_divisor: raw.wild_exp_divisor,
            trainer_exp_numerator: raw.trainer_exp_numerator,
            trainer_exp_denominator: raw.trainer_exp_denominator,
        };
        rules.validate_shape().map_err(D::Error::custom)?;
        Ok(rules)
    }
}

impl Default for BattleRewardRules {
    fn default() -> Self {
        Self {
            max_level: 0,
            wild_exp_divisor: 0,
            trainer_exp_numerator: 0,
            trainer_exp_denominator: 0,
        }
    }
}

impl BattleRewardRules {
    fn validate_shape(&self) -> Result<(), String> {
        if let Some(issue) = battle_reward_rules_issues(self).into_iter().next() {
            return Err(format!("invalid battle reward rules: {issue:?}"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleRewardRulesField {
    MaxLevel,
    WildExpDivisor,
    TrainerExpNumerator,
    TrainerExpDenominator,
}

impl BattleRewardRulesField {
    pub const fn subject(self) -> &'static str {
        match self {
            Self::MaxLevel => "battle_reward_rules:max_level",
            Self::WildExpDivisor => "battle_reward_rules:wild_exp_divisor",
            Self::TrainerExpNumerator => "battle_reward_rules:trainer_exp_numerator",
            Self::TrainerExpDenominator => "battle_reward_rules:trainer_exp_denominator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleRewardRulesIssue {
    MissingMaxLevel,
    InvalidWildExpDivisor { value: i32 },
    InvalidTrainerExpNumerator { value: i32 },
    InvalidTrainerExpDenominator { value: i32 },
}

impl BattleRewardRulesIssue {
    pub const fn field(&self) -> BattleRewardRulesField {
        match self {
            Self::MissingMaxLevel => BattleRewardRulesField::MaxLevel,
            Self::InvalidWildExpDivisor { .. } => BattleRewardRulesField::WildExpDivisor,
            Self::InvalidTrainerExpNumerator { .. } => BattleRewardRulesField::TrainerExpNumerator,
            Self::InvalidTrainerExpDenominator { .. } => {
                BattleRewardRulesField::TrainerExpDenominator
            }
        }
    }
}

pub fn battle_reward_rules_issues(rules: &BattleRewardRules) -> Vec<BattleRewardRulesIssue> {
    let mut issues = Vec::new();
    if rules.max_level == 0 {
        issues.push(BattleRewardRulesIssue::MissingMaxLevel);
    }
    if rules.wild_exp_divisor <= 0 {
        issues.push(BattleRewardRulesIssue::InvalidWildExpDivisor {
            value: rules.wild_exp_divisor,
        });
    }
    if rules.trainer_exp_numerator <= 0 {
        issues.push(BattleRewardRulesIssue::InvalidTrainerExpNumerator {
            value: rules.trainer_exp_numerator,
        });
    }
    if rules.trainer_exp_denominator <= 0 {
        issues.push(BattleRewardRulesIssue::InvalidTrainerExpDenominator {
            value: rules.trainer_exp_denominator,
        });
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleRewardOutcome {
    pub defeated_species: String,
    pub experience_awarded: i32,
    pub level_before: u8,
    pub level_after: u8,
    pub learned_moves: Vec<String>,
    pub pending_move_learns: Vec<LearnedMove>,
    pub deferred_level_evolution: bool,
    pub evolution: EvolutionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokemonLevelUpOutcome {
    pub level_before: u8,
    pub level_after: u8,
    pub experience_before: i32,
    pub experience_after: i32,
    pub learned_moves: Vec<String>,
    pub pending_move_learns: Vec<LearnedMove>,
    pub deferred_level_evolution: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BattleRewardError {
    #[error("battle rewards require defeated Pokemon to be fainted")]
    DefeatedPokemonNotFainted,
    #[error("missing level-up learnset for species {species_id}")]
    MissingLearnset { species_id: String },
    #[error("missing move data for level-up move {move_id}")]
    MissingMoveData { move_id: String },
    #[error("evolution reward failed: {0}")]
    Evolution(#[from] EvolutionError),
    #[error("experience table error: {0}")]
    Experience(#[from] ExperienceError),
    #[error("battle reward rules field {field} must be nonzero")]
    InvalidRule { field: String },
    #[error("battle reward rules are missing")]
    MissingRules,
    #[error("pending move learn is missing")]
    MissingPendingMoveLearn,
    #[error("pending move learn already exists for party index {party_index}")]
    PendingMoveLearnExists { party_index: usize },
    #[error("pending move learn requires a full move list for party index {party_index}")]
    PendingMoveLearnRequiresFullMoveList { party_index: usize },
    #[error("pending move learn party index {party_index} is empty")]
    PendingMoveLearnEmptyPartySlot { party_index: usize },
    #[error(
        "pending move learn replacement slot {move_slot} is outside party index {party_index} move list"
    )]
    InvalidPendingMoveLearnReplacement {
        party_index: usize,
        move_slot: usize,
    },
    #[error("pending move learn species {species_id} does not match party index {party_index}")]
    PendingMoveLearnSpeciesMismatch {
        party_index: usize,
        species_id: String,
    },
    #[error("pending move learn level {level} does not match party index {party_index}")]
    PendingMoveLearnLevelMismatch { party_index: usize, level: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActiveWildBattleRewardError {
    #[error("wild battle rewards require an active wild battle")]
    MissingActiveWildBattle,
    #[error("trainer battle {trainer_id} rewards require trainer-completion sequencing")]
    ActiveTrainerBattle { trainer_id: String },
    #[error("active battle party error: {0:?}")]
    ActiveParty(#[from] ActiveBattlePartyError),
    #[error("battle reward error: {0:?}")]
    Reward(#[from] BattleRewardError),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActiveTrainerBattleRewardError {
    #[error("trainer battle rewards require an active trainer battle")]
    MissingActiveTrainerBattle,
    #[error("wild battle rewards require wild reward flow")]
    ActiveWildBattle,
    #[error("active battle party error: {0:?}")]
    ActiveParty(#[from] ActiveBattlePartyError),
    #[error("active battle enemy error: {0:?}")]
    ActiveEnemy(#[from] ActiveBattleEnemyError),
    #[error("battle reward error: {0:?}")]
    Reward(#[from] BattleRewardError),
}

pub fn claim_active_trainer_battle_rewards(
    state: &mut GameState,
    rules: &BattleRewardRules,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
) -> Result<BattleRewardOutcome, ActiveTrainerBattleRewardError> {
    let enemy = match &state.battle {
        BattleMemory::Trainer { enemy_pokemon, .. } => enemy_pokemon.clone(),
        BattleMemory::Wild { .. } | BattleMemory::StaticWild { .. } => {
            return Err(ActiveTrainerBattleRewardError::ActiveWildBattle);
        }
        BattleMemory::Inactive => {
            return Err(ActiveTrainerBattleRewardError::MissingActiveTrainerBattle);
        }
    };
    let active_index = require_active_battle_party_index(state)?;
    let player = state.storage.party.pokemon[active_index].as_mut().ok_or(
        ActiveBattlePartyError::EmptyPartySlot {
            index: active_index,
        },
    )?;
    let outcome = apply_trainer_battle_rewards(
        rules,
        player,
        &enemy,
        species,
        moves,
        learnsets,
        growth_rates,
        evolutions,
        time_of_day,
    )?;
    queue_pending_move_learn(state, active_index, &outcome)?;
    state.sync_party_from_storage();
    update_active_battle_enemy(state, enemy)?;
    claim_active_trainer_battle_reward_index(state)?;
    Ok(outcome)
}

pub fn claim_active_wild_battle_rewards(
    state: &mut GameState,
    rules: &BattleRewardRules,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
) -> Result<BattleRewardOutcome, ActiveWildBattleRewardError> {
    let enemy = match &state.battle {
        BattleMemory::Wild { enemy_pokemon, .. }
        | BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
        BattleMemory::Trainer { trainer_id, .. } => {
            return Err(ActiveWildBattleRewardError::ActiveTrainerBattle {
                trainer_id: trainer_id.clone(),
            });
        }
        BattleMemory::Inactive => return Err(ActiveWildBattleRewardError::MissingActiveWildBattle),
    };
    let active_index = require_active_battle_party_index(state)?;
    let player = state.storage.party.pokemon[active_index].as_mut().ok_or(
        ActiveBattlePartyError::EmptyPartySlot {
            index: active_index,
        },
    )?;
    let outcome = apply_wild_battle_rewards(
        rules,
        player,
        &enemy,
        species,
        moves,
        learnsets,
        growth_rates,
        evolutions,
        time_of_day,
    )?;
    queue_pending_move_learn(state, active_index, &outcome)?;
    deactivate_battle(state);
    state.sync_party_from_storage();
    Ok(outcome)
}

fn queue_pending_move_learn(
    state: &mut GameState,
    party_index: usize,
    outcome: &BattleRewardOutcome,
) -> Result<(), BattleRewardError> {
    let Some(learned_move) = outcome.pending_move_learns.first() else {
        return Ok(());
    };
    if state.pending_move_learn.is_some() {
        return Err(BattleRewardError::PendingMoveLearnExists { party_index });
    }
    let pokemon = state.storage.party.pokemon[party_index]
        .as_ref()
        .ok_or(BattleRewardError::PendingMoveLearnEmptyPartySlot { party_index })?;
    if pokemon.moves.len() < 4 {
        return Err(BattleRewardError::PendingMoveLearnRequiresFullMoveList { party_index });
    }
    if pokemon
        .moves
        .iter()
        .any(|known| known.name == learned_move.name)
    {
        return Ok(());
    }
    state.pending_move_learn = Some(PendingMoveLearn {
        party_index,
        species_id: pokemon.species.id.clone(),
        level: pokemon.level,
        learned_move: learned_move.clone(),
        defer_level_evolution: outcome.deferred_level_evolution,
    });
    Ok(())
}

pub fn apply_wild_battle_rewards(
    rules: &BattleRewardRules,
    player: &mut Pokemon,
    defeated: &Pokemon,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
) -> Result<BattleRewardOutcome, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    apply_battle_rewards_with_experience(
        rules,
        player,
        defeated,
        species,
        moves,
        learnsets,
        growth_rates,
        evolutions,
        time_of_day,
        wild_experience_award(rules, defeated)?,
    )
}

pub fn apply_trainer_battle_rewards(
    rules: &BattleRewardRules,
    player: &mut Pokemon,
    defeated: &Pokemon,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
) -> Result<BattleRewardOutcome, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    apply_battle_rewards_with_experience(
        rules,
        player,
        defeated,
        species,
        moves,
        learnsets,
        growth_rates,
        evolutions,
        time_of_day,
        trainer_experience_award(rules, defeated)?,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingMoveLearnResolution {
    pub party_index: usize,
    pub learned_move: String,
    pub replaced_slot: Option<usize>,
    pub replaced_move: Option<String>,
    pub defer_level_evolution: bool,
}

pub fn replace_pending_move_learn(
    state: &mut GameState,
    move_slot: usize,
) -> Result<PendingMoveLearnResolution, BattleRewardError> {
    let pending = state
        .pending_move_learn
        .clone()
        .ok_or(BattleRewardError::MissingPendingMoveLearn)?;
    let pokemon = require_pending_move_learn_party_pokemon(state, &pending)?;
    let existing = pokemon.moves.get_mut(move_slot).ok_or(
        BattleRewardError::InvalidPendingMoveLearnReplacement {
            party_index: pending.party_index,
            move_slot,
        },
    )?;
    let replaced_move = std::mem::replace(existing, pending.learned_move.clone()).name;
    state.pending_move_learn = None;
    state.sync_party_from_storage();
    Ok(PendingMoveLearnResolution {
        party_index: pending.party_index,
        learned_move: pending.learned_move.name,
        replaced_slot: Some(move_slot),
        replaced_move: Some(replaced_move),
        defer_level_evolution: pending.defer_level_evolution,
    })
}

pub fn decline_pending_move_learn(
    state: &mut GameState,
) -> Result<PendingMoveLearnResolution, BattleRewardError> {
    let pending = state
        .pending_move_learn
        .clone()
        .ok_or(BattleRewardError::MissingPendingMoveLearn)?;
    require_pending_move_learn_party_pokemon(state, &pending)?;
    state.pending_move_learn = None;
    Ok(PendingMoveLearnResolution {
        party_index: pending.party_index,
        learned_move: pending.learned_move.name,
        replaced_slot: None,
        replaced_move: None,
        defer_level_evolution: pending.defer_level_evolution,
    })
}

fn require_pending_move_learn_party_pokemon<'a>(
    state: &'a mut GameState,
    pending: &PendingMoveLearn,
) -> Result<&'a mut Pokemon, BattleRewardError> {
    let pokemon = state
        .storage
        .party
        .pokemon
        .get_mut(pending.party_index)
        .and_then(Option::as_mut)
        .ok_or(BattleRewardError::PendingMoveLearnEmptyPartySlot {
            party_index: pending.party_index,
        })?;
    if pokemon.species.id != pending.species_id {
        return Err(BattleRewardError::PendingMoveLearnSpeciesMismatch {
            party_index: pending.party_index,
            species_id: pokemon.species.id.clone(),
        });
    }
    if pokemon.level != pending.level {
        return Err(BattleRewardError::PendingMoveLearnLevelMismatch {
            party_index: pending.party_index,
            level: pokemon.level,
        });
    }
    Ok(pokemon)
}

fn apply_battle_rewards_with_experience(
    rules: &BattleRewardRules,
    player: &mut Pokemon,
    defeated: &Pokemon,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
    experience_awarded: i32,
) -> Result<BattleRewardOutcome, BattleRewardError> {
    if defeated.hp != 0 {
        return Err(BattleRewardError::DefeatedPokemonNotFainted);
    }
    let mut rewarded = player.clone();
    let level_before = rewarded.level;
    rewarded.experience = rewarded.experience.saturating_add(experience_awarded);
    add_stat_experience(&mut rewarded, defeated.species.base_stats);
    refresh_level_stats(&mut rewarded);
    let level_up =
        apply_experience_level_ups(&mut rewarded, moves, learnsets, growth_rates, rules)?;
    let evolution_context = crate::systems::evolution::EvolutionContext {
        species,
        moves,
        learnsets,
        time_of_day,
        current_item: None,
        force_evolution: false,
        link_mode: crate::systems::evolution::LinkMode::None,
    };
    let mut pending_move_learns = level_up.pending_move_learns;
    let deferred_level_evolution = !pending_move_learns.is_empty();
    let evolution = if deferred_level_evolution {
        EvolutionReport::default()
    } else {
        let evolution = check_and_evolve(&mut rewarded, evolutions, &evolution_context, true)?;
        pending_move_learns.extend(evolution.pending_move_learns.clone());
        evolution
    };
    *player = rewarded;
    Ok(BattleRewardOutcome {
        defeated_species: defeated.species.id.clone(),
        experience_awarded,
        level_before,
        level_after: player.level,
        learned_moves: level_up.learned_moves,
        pending_move_learns,
        deferred_level_evolution,
        evolution,
    })
}

pub fn wild_experience_award(
    rules: &BattleRewardRules,
    defeated: &Pokemon,
) -> Result<i32, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    require_positive_i32(rules.wild_exp_divisor, "wild_exp_divisor")?;
    Ok((i32::from(defeated.species.base_exp) * i32::from(defeated.level)) / rules.wild_exp_divisor)
}

pub fn trainer_experience_award(
    rules: &BattleRewardRules,
    defeated: &Pokemon,
) -> Result<i32, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    require_positive_i32(rules.trainer_exp_numerator, "trainer_exp_numerator")?;
    require_positive_i32(rules.trainer_exp_denominator, "trainer_exp_denominator")?;
    Ok(
        (wild_experience_award(rules, defeated)? * rules.trainer_exp_numerator)
            / rules.trainer_exp_denominator,
    )
}

fn add_stat_experience(player: &mut Pokemon, base_stats: crate::models::BaseStats) {
    player.hp_exp = player.hp_exp.saturating_add(base_stats.hp);
    player.attack_exp = player.attack_exp.saturating_add(base_stats.attack);
    player.defense_exp = player.defense_exp.saturating_add(base_stats.defense);
    player.speed_exp = player.speed_exp.saturating_add(base_stats.speed);
    player.special_exp = player.special_exp.saturating_add(base_stats.special_attack);
}

pub fn apply_experience_level_ups(
    player: &mut Pokemon,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    rules: &BattleRewardRules,
) -> Result<PokemonLevelUpOutcome, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    require_positive_u8(rules.max_level, "max_level")?;
    let level_before = player.level;
    let experience_before = player.experience;
    let mut learned_moves = Vec::new();
    let mut pending_move_learns = Vec::new();
    while player.level < rules.max_level {
        let next_level_experience =
            calculate_experience(growth_rates, &player.species.growth_rate, player.level + 1)?;
        if player.experience < next_level_experience {
            break;
        }
        player.level += 1;
        refresh_level_stats(player);
        let level_moves = learn_moves_for_current_level(player, moves, learnsets)?;
        for learned in level_moves.learned {
            learned_moves.push(learned.name);
        }
        pending_move_learns.extend(level_moves.pending);
        if !pending_move_learns.is_empty() {
            break;
        }
    }
    let deferred_level_evolution = !pending_move_learns.is_empty();
    Ok(PokemonLevelUpOutcome {
        level_before,
        level_after: player.level,
        experience_before,
        experience_after: player.experience,
        learned_moves,
        pending_move_learns,
        deferred_level_evolution,
    })
}

pub fn apply_direct_level_gain(
    player: &mut Pokemon,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    rules: &BattleRewardRules,
    level_gain: u8,
) -> Result<PokemonLevelUpOutcome, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    require_positive_u8(rules.max_level, "max_level")?;
    let mut leveled = player.clone();
    let level_before = leveled.level;
    let experience_before = leveled.experience;
    let target_level = player.level.saturating_add(level_gain).min(rules.max_level);
    let mut learned_moves = Vec::new();
    let mut pending_move_learns = Vec::new();
    while leveled.level < target_level {
        leveled.level += 1;
        leveled.experience = leveled.experience.max(calculate_experience(
            growth_rates,
            &leveled.species.growth_rate,
            leveled.level,
        )?);
        refresh_level_stats(&mut leveled);
        let level_moves = learn_moves_for_current_level(&mut leveled, moves, learnsets)?;
        for learned in level_moves.learned {
            learned_moves.push(learned.name);
        }
        pending_move_learns.extend(level_moves.pending);
        if !pending_move_learns.is_empty() {
            break;
        }
    }
    *player = leveled;
    let deferred_level_evolution = !pending_move_learns.is_empty();
    Ok(PokemonLevelUpOutcome {
        level_before,
        level_after: player.level,
        experience_before,
        experience_after: player.experience,
        learned_moves,
        pending_move_learns,
        deferred_level_evolution,
    })
}

fn require_positive_i32(value: i32, field: &str) -> Result<(), BattleRewardError> {
    if value <= 0 {
        return Err(BattleRewardError::InvalidRule {
            field: field.to_string(),
        });
    }
    Ok(())
}

fn require_positive_u8(value: u8, field: &str) -> Result<(), BattleRewardError> {
    if value == 0 {
        return Err(BattleRewardError::InvalidRule {
            field: field.to_string(),
        });
    }
    Ok(())
}

fn require_battle_reward_rules(rules: &BattleRewardRules) -> Result<(), BattleRewardError> {
    if rules == &BattleRewardRules::default() {
        return Err(BattleRewardError::MissingRules);
    }
    if let Some(issue) = battle_reward_rules_issues(rules).into_iter().next() {
        return Err(BattleRewardError::InvalidRule {
            field: issue.field().subject().to_string(),
        });
    }
    Ok(())
}

fn refresh_level_stats(player: &mut Pokemon) {
    let old_max_hp = player.max_hp;
    let old_hp = player.hp;
    let stats = calculate_stats(
        &player.species,
        player.level,
        player.dvs,
        StatExperience {
            hp: player.hp_exp,
            attack: player.attack_exp,
            defense: player.defense_exp,
            speed: player.speed_exp,
            special: player.special_exp,
        },
    );
    player.max_hp = stats.max_hp;
    player.attack = stats.attack;
    player.defense = stats.defense;
    player.speed = stats.speed;
    player.special_attack = stats.special_attack;
    player.special_defense = stats.special_defense;
    let hp_delta = i32::from(stats.max_hp) - i32::from(old_max_hp);
    player.hp = (i32::from(old_hp) + hp_delta).clamp(0, i32::from(stats.max_hp)) as u16;
}

struct LevelMoveLearnResult {
    learned: Vec<LearnedMove>,
    pending: Vec<LearnedMove>,
}

fn learn_moves_for_current_level(
    player: &mut Pokemon,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
) -> Result<LevelMoveLearnResult, BattleRewardError> {
    let entries =
        level_up_moves_for_species(learnsets, &player.species.id).map_err(|error| match error {
            LearnsetError::InvalidSpecies { species_id }
            | LearnsetError::MissingSpecies { species_id } => {
                BattleRewardError::MissingLearnset { species_id }
            }
            LearnsetError::InvalidMove { move_id, .. } => {
                BattleRewardError::MissingMoveData { move_id }
            }
        })?;
    let mut learned = Vec::new();
    let mut pending = Vec::new();
    for crate::systems::learnsets::LearnsetEntry(level, move_name) in entries {
        if *level != player.level || player.moves.iter().any(|known| known.name == *move_name) {
            continue;
        }
        let move_data = moves
            .get(move_name)
            .ok_or_else(|| BattleRewardError::MissingMoveData {
                move_id: move_name.clone(),
            })?;
        let entry = LearnedMove {
            name: move_name.clone(),
            current_pp: move_data.pp,
            pp_ups: 0,
        };
        if player.moves.len() >= 4 {
            pending.push(entry);
        } else {
            player.moves.push(entry.clone());
            learned.push(entry);
        }
    }
    Ok(LevelMoveLearnResult { learned, pending })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, GrowthRate, growth_rate, pokemon_type};
    use crate::systems::evolution::EvolutionEntry;
    use crate::systems::experience::crystal_growth_rate_catalog_for_tests;
    use crate::systems::learnsets::LearnsetEntry;

    fn species(id: &str, base_exp: u16, growth_rate: GrowthRate) -> PokemonSpecies {
        let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65));
        species.base_exp = base_exp;
        species.growth_rate = growth_rate;
        species.type1 = pokemon_type("NORMAL");
        species.type2 = pokemon_type("NORMAL");
        species
    }

    fn move_data(name: &str, pp: u8) -> Move {
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

    fn reward_rules() -> BattleRewardRules {
        BattleRewardRules {
            max_level: 100,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
        }
    }

    fn pending_move_learn_state() -> GameState {
        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            16,
            Dv::default(),
        );
        pokemon.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 40,
                pp_ups: 0,
            },
            LearnedMove {
                name: "REFLECT".to_string(),
                current_pp: 20,
                pp_ups: 0,
            },
            LearnedMove {
                name: "POISONPOWDER".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
        ];
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();
        state.pending_move_learn = Some(PendingMoveLearn {
            party_index: 0,
            species_id: "CHIKORITA".to_string(),
            level: 16,
            learned_move: LearnedMove {
                name: "RAZOR_LEAF".to_string(),
                current_pp: 25,
                pp_ups: 0,
            },
            defer_level_evolution: true,
        });
        state
    }

    #[test]
    fn battle_reward_rules_issues_validate_declared_rules() {
        assert_eq!(
            battle_reward_rules_issues(&BattleRewardRules::default()),
            vec![
                BattleRewardRulesIssue::MissingMaxLevel,
                BattleRewardRulesIssue::InvalidWildExpDivisor { value: 0 },
                BattleRewardRulesIssue::InvalidTrainerExpNumerator { value: 0 },
                BattleRewardRulesIssue::InvalidTrainerExpDenominator { value: 0 },
            ]
        );

        let rules = BattleRewardRules {
            max_level: 0,
            wild_exp_divisor: 0,
            trainer_exp_numerator: -1,
            trainer_exp_denominator: 0,
        };
        assert_eq!(
            battle_reward_rules_issues(&rules),
            vec![
                BattleRewardRulesIssue::MissingMaxLevel,
                BattleRewardRulesIssue::InvalidWildExpDivisor { value: 0 },
                BattleRewardRulesIssue::InvalidTrainerExpNumerator { value: -1 },
                BattleRewardRulesIssue::InvalidTrainerExpDenominator { value: 0 },
            ],
        );
        assert_eq!(
            BattleRewardRulesIssue::InvalidTrainerExpDenominator { value: 0 }.field(),
            BattleRewardRulesField::TrainerExpDenominator,
        );
        assert_eq!(
            BattleRewardRulesField::TrainerExpDenominator.subject(),
            "battle_reward_rules:trainer_exp_denominator",
        );
    }

    #[test]
    fn reward_application_rejects_missing_rules_without_zero_reward_fallback() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        let player_before = player.clone();
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );

        assert_eq!(
            wild_experience_award(&BattleRewardRules::default(), &defeated),
            Err(BattleRewardError::MissingRules)
        );
        assert_eq!(
            trainer_experience_award(&BattleRewardRules::default(), &defeated),
            Err(BattleRewardError::MissingRules)
        );
        assert_eq!(
            apply_wild_battle_rewards(
                &BattleRewardRules::default(),
                &mut player,
                &defeated,
                &species,
                &BTreeMap::new(),
                &learnsets,
                &growth_rates,
                &evolutions,
                TimeOfDay::Day,
            ),
            Err(BattleRewardError::MissingRules)
        );
        assert_eq!(player, player_before);
    }

    #[test]
    fn reward_application_rejects_partial_invalid_rules_before_battle_state() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        let player_before = player.clone();
        let defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );
        let invalid_rules = BattleRewardRules {
            max_level: 0,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
        };

        assert_eq!(
            apply_wild_battle_rewards(
                &invalid_rules,
                &mut player,
                &defeated,
                &species,
                &BTreeMap::new(),
                &learnsets,
                &growth_rates,
                &evolutions,
                TimeOfDay::Day,
            ),
            Err(BattleRewardError::InvalidRule {
                field: "battle_reward_rules:max_level".to_string(),
            })
        );
        assert_eq!(player, player_before);
    }

    #[test]
    fn wild_battle_rewards_award_exp_stat_exp_level_moves_and_evolution() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience =
            calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let bayleef = species("BAYLEEF", 141, growth_rate("GROWTH_MEDIUM_FAST"));
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
            ("BAYLEEF".to_string(), bayleef),
        ]
        .into_iter()
        .collect();
        let moves = [
            ("TACKLE".to_string(), move_data("TACKLE", 35)),
            ("RAZOR_LEAF".to_string(), move_data("RAZOR_LEAF", 25)),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            (
                "CHIKORITA".to_string(),
                vec![LearnsetEntry(16, "RAZOR_LEAF".to_string())],
            ),
            ("BAYLEEF".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [
                (
                    "CHIKORITA".to_string(),
                    vec![EvolutionEntry::level("BAYLEEF", 16)],
                ),
                ("BAYLEEF".to_string(), Vec::new()),
                ("PIDGEY".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
        );

        let outcome = apply_wild_battle_rewards(
            &reward_rules(),
            &mut player,
            &defeated,
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            TimeOfDay::Day,
        )
        .expect("battle rewards");

        assert_eq!(outcome.experience_awarded, 65);
        assert_eq!(outcome.level_before, 15);
        assert_eq!(outcome.level_after, 16);
        assert_eq!(outcome.learned_moves, vec!["RAZOR_LEAF".to_string()]);
        assert_eq!(
            outcome.evolution.target_species,
            Some("BAYLEEF".to_string())
        );
        assert_eq!(player.species.id, "BAYLEEF");
        assert_eq!(player.hp_exp, 45);
        assert_eq!(player.attack_exp, 49);
        assert!(player.moves.iter().any(|known| known.name == "RAZOR_LEAF"));
    }

    #[test]
    fn active_wild_battle_rewards_commit_player_deactivate_and_sync_party() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let moves = [("TACKLE".to_string(), move_data("TACKLE", 35))]
            .into_iter()
            .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [
                ("CHIKORITA".to_string(), Vec::new()),
                ("PIDGEY".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
        );
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(player);
        state.battle_active_party_index = Some(0);
        state.battle = BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: defeated.clone(),
            enemy_party: vec![defeated],
        };

        let outcome = claim_active_wild_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            TimeOfDay::Day,
        )
        .expect("claim wild rewards");

        assert_eq!(outcome.defeated_species, "PIDGEY");
        assert_eq!(state.battle, BattleMemory::Inactive);
        assert_eq!(state.battle_active_party_index, None);
        assert_eq!(
            state.party.pokemon[0]
                .as_ref()
                .map(|pokemon| pokemon.species.as_str()),
            Some("CHIKORITA")
        );
        assert!(state.storage.party.pokemon[0].as_ref().unwrap().experience > 0);
    }

    #[test]
    fn active_trainer_battle_rewards_commit_player_enemy_and_reward_index() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let moves = [("TACKLE".to_string(), move_data("TACKLE", 35))]
            .into_iter()
            .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [
                ("CHIKORITA".to_string(), Vec::new()),
                ("PIDGEY".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
        );
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(player);
        state.battle_active_party_index = Some(0);
        state.battle_active_enemy_party_index = Some(0);
        state.battle = BattleMemory::Trainer {
            battle_type: "BATTLETYPE_TRAINER".to_string(),
            trainer_class: "YOUNGSTER".to_string(),
            trainer_id: "YOUNGSTER_JOEY".to_string(),
            trainer_name: "JOEY".to_string(),
            event_flag: "EVENT_BEAT_YOUNGSTER_JOEY".to_string(),
            seen_text: String::new(),
            win_text: String::new(),
            loss_text: String::new(),
            callback: String::new(),
            source_script: "TrainerScript".to_string(),
            enemy_pokemon: defeated.clone(),
            enemy_party: vec![defeated],
            reward: 64,
            encounter_music: "MUSIC_YOUNGSTER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };

        let outcome = claim_active_trainer_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            TimeOfDay::Day,
        )
        .expect("claim trainer rewards");

        assert_eq!(outcome.defeated_species, "PIDGEY");
        assert!(state.battle_rewarded_enemy_party_indices.contains(&0));
        assert_eq!(
            state.party.pokemon[0]
                .as_ref()
                .map(|pokemon| pokemon.species.as_str()),
            Some("CHIKORITA")
        );
        let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &state.battle
        else {
            panic!("expected trainer battle");
        };
        assert_eq!(enemy_pokemon.hp, 0);
        assert_eq!(enemy_party[0].hp, 0);
    }

    #[test]
    fn trainer_battle_rewards_use_trainer_exp_and_exact_level_tables() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience =
            calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let moves = [("RAZOR_LEAF".to_string(), move_data("RAZOR_LEAF", 25))]
            .into_iter()
            .collect();
        let learnsets = [
            (
                "CHIKORITA".to_string(),
                vec![LearnsetEntry(16, "RAZOR_LEAF".to_string())],
            ),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );

        let outcome = apply_trainer_battle_rewards(
            &reward_rules(),
            &mut player,
            &defeated,
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            TimeOfDay::Day,
        )
        .expect("trainer rewards");

        assert_eq!(wild_experience_award(&reward_rules(), &defeated), Ok(65));
        assert_eq!(trainer_experience_award(&reward_rules(), &defeated), Ok(97));
        assert_eq!(outcome.experience_awarded, 97);
        assert_eq!(outcome.level_after, 16);
        assert_eq!(outcome.learned_moves, vec!["RAZOR_LEAF".to_string()]);
        assert_eq!(player.hp_exp, 45);
    }

    #[test]
    fn rewards_refresh_stats_after_stat_exp_without_level_up() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("TYPHLOSION", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            60,
            Dv::from_non_hp(10, 10, 10, 10),
        );
        let level_before = player.level;
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("TYPHLOSION".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [("TYPHLOSION".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );

        let outcome = apply_trainer_battle_rewards(
            &reward_rules(),
            &mut player,
            &defeated,
            &species,
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            TimeOfDay::Day,
        )
        .expect("trainer rewards");

        assert_eq!(outcome.level_before, level_before);
        assert_eq!(outcome.level_after, level_before);
        assert!(player.hp_exp > 0);
        player
            .validate_saved_state()
            .expect("valid rewarded Pokemon");
        let expected = calculate_stats(
            &player.species,
            player.level,
            player.dvs,
            StatExperience {
                hp: player.hp_exp,
                attack: player.attack_exp,
                defense: player.defense_exp,
                speed: player.speed_exp,
                special: player.special_exp,
            },
        );
        assert_eq!(player.max_hp, expected.max_hp);
        assert_eq!(player.attack, expected.attack);
        assert_eq!(player.defense, expected.defense);
        assert_eq!(player.speed, expected.speed);
        assert_eq!(player.special_attack, expected.special_attack);
        assert_eq!(player.special_defense, expected.special_defense);
    }

    #[test]
    fn rewards_reject_unfainted_enemy_and_missing_exact_move_data() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience =
            calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
        ]
        .into_iter()
        .collect();
        let learnsets = [(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(16, "razor_leaf".to_string())],
        )]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );

        assert_eq!(
            apply_wild_battle_rewards(
                &reward_rules(),
                &mut player.clone(),
                &defeated,
                &species,
                &BTreeMap::new(),
                &learnsets,
                &growth_rates,
                &evolutions,
                TimeOfDay::Day,
            ),
            Err(BattleRewardError::DefeatedPokemonNotFainted)
        );

        defeated.hp = 0;
        let player_before_missing_move = player.clone();
        assert_eq!(
            apply_wild_battle_rewards(
                &reward_rules(),
                &mut player,
                &defeated,
                &species,
                &BTreeMap::new(),
                &learnsets,
                &growth_rates,
                &evolutions,
                TimeOfDay::Day,
            ),
            Err(BattleRewardError::MissingMoveData {
                move_id: "razor_leaf".to_string()
            })
        );
        assert_eq!(player, player_before_missing_move);
    }

    #[test]
    fn direct_level_gain_rejects_missing_move_without_partial_mutation() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience = calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 15).unwrap();
        let player_before = player.clone();
        let learnsets = [(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(16, "razor_leaf".to_string())],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            apply_direct_level_gain(
                &mut player,
                &BTreeMap::new(),
                &learnsets,
                &growth_rates,
                &reward_rules(),
                1,
            ),
            Err(BattleRewardError::MissingMoveData {
                move_id: "razor_leaf".to_string()
            })
        );
        assert_eq!(player, player_before);
    }

    #[test]
    fn pending_move_learn_replace_and_decline_validate_same_party_target() {
        let mut replace_state = pending_move_learn_state();
        replace_state.storage.party.pokemon[0] = None;
        assert_eq!(
            replace_pending_move_learn(&mut replace_state, 0),
            Err(BattleRewardError::PendingMoveLearnEmptyPartySlot { party_index: 0 })
        );
        assert!(replace_state.pending_move_learn.is_some());

        let mut decline_state = pending_move_learn_state();
        decline_state.storage.party.pokemon[0] = None;
        assert_eq!(
            decline_pending_move_learn(&mut decline_state),
            Err(BattleRewardError::PendingMoveLearnEmptyPartySlot { party_index: 0 })
        );
        assert!(decline_state.pending_move_learn.is_some());

        let mut replace_state = pending_move_learn_state();
        replace_state.storage.party.pokemon[0] = Some(Pokemon::new_for_tests(
            species("BAYLEEF", 141, growth_rate("GROWTH_MEDIUM_FAST")),
            16,
            Dv::default(),
        ));
        assert_eq!(
            replace_pending_move_learn(&mut replace_state, 0),
            Err(BattleRewardError::PendingMoveLearnSpeciesMismatch {
                party_index: 0,
                species_id: "BAYLEEF".to_string(),
            })
        );
        assert!(replace_state.pending_move_learn.is_some());

        let mut decline_state = pending_move_learn_state();
        decline_state.storage.party.pokemon[0] = Some(Pokemon::new_for_tests(
            species("BAYLEEF", 141, growth_rate("GROWTH_MEDIUM_FAST")),
            16,
            Dv::default(),
        ));
        assert_eq!(
            decline_pending_move_learn(&mut decline_state),
            Err(BattleRewardError::PendingMoveLearnSpeciesMismatch {
                party_index: 0,
                species_id: "BAYLEEF".to_string(),
            })
        );
        assert!(decline_state.pending_move_learn.is_some());

        let mut replace_state = pending_move_learn_state();
        replace_state.storage.party.pokemon[0]
            .as_mut()
            .expect("party Pokemon")
            .level = 17;
        assert_eq!(
            replace_pending_move_learn(&mut replace_state, 0),
            Err(BattleRewardError::PendingMoveLearnLevelMismatch {
                party_index: 0,
                level: 17,
            })
        );
        assert!(replace_state.pending_move_learn.is_some());

        let mut decline_state = pending_move_learn_state();
        decline_state.storage.party.pokemon[0]
            .as_mut()
            .expect("party Pokemon")
            .level = 17;
        assert_eq!(
            decline_pending_move_learn(&mut decline_state),
            Err(BattleRewardError::PendingMoveLearnLevelMismatch {
                party_index: 0,
                level: 17,
            })
        );
        assert!(decline_state.pending_move_learn.is_some());
    }

    #[test]
    fn pending_move_learn_decline_clears_only_after_valid_target() {
        let mut state = pending_move_learn_state();

        let resolution = decline_pending_move_learn(&mut state).expect("valid decline");

        assert_eq!(
            resolution,
            PendingMoveLearnResolution {
                party_index: 0,
                learned_move: "RAZOR_LEAF".to_string(),
                replaced_slot: None,
                replaced_move: None,
                defer_level_evolution: true,
            }
        );
        assert_eq!(state.pending_move_learn, None);
        assert_eq!(
            state.storage.party.pokemon[0]
                .as_ref()
                .expect("party Pokemon")
                .moves[0]
                .name,
            "TACKLE"
        );
    }

    #[test]
    fn battle_reward_issue_json_rejects_unknown_fallback_fields() {
        let error = serde_json::from_value::<BattleRewardRulesIssue>(serde_json::json!({
            "InvalidWildExpDivisor": {
                "value": 0,
                "default_divisor": 1
            }
        }))
        .expect_err("default divisor must be rejected")
        .to_string();
        assert!(error.contains("unknown field `default_divisor`"), "{error}");
    }
}
