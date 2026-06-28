use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::battle::start::{
    ActiveBattleEnemyError, ActiveBattlePartyError, claim_active_trainer_battle_reward_index,
    deactivate_battle, require_active_battle_party_index, update_active_battle_enemy,
};
use crate::models::pokemon::StatExperience;
use crate::models::{LearnedMove, Move, Pokemon, PokemonSpecies, calculate_stats};
use crate::state::{BattleMemory, GameState};
use crate::systems::evolution::{
    EvolutionError, EvolutionReport, EvolutionTable, check_and_evolve,
};
use crate::systems::experience::{ExperienceError, GrowthRateCatalog, calculate_experience};
use crate::systems::learnsets::{LearnsetError, SpeciesLearnsets, level_up_moves_for_species};
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleRewardRules {
    pub max_level: u8,
    pub wild_exp_divisor: i32,
    pub trainer_exp_numerator: i32,
    pub trainer_exp_denominator: i32,
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
    if rules == &BattleRewardRules::default() {
        return Vec::new();
    }

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
    pub evolution: EvolutionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokemonLevelUpOutcome {
    pub level_before: u8,
    pub level_after: u8,
    pub experience_before: i32,
    pub experience_after: i32,
    pub learned_moves: Vec<String>,
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
    deactivate_battle(state);
    state.sync_party_from_storage();
    Ok(outcome)
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
    let evolution = check_and_evolve(&mut rewarded, evolutions, &evolution_context, true)?;
    *player = rewarded;
    Ok(BattleRewardOutcome {
        defeated_species: defeated.species.id.clone(),
        experience_awarded,
        level_before,
        level_after: player.level,
        learned_moves: level_up.learned_moves,
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
    while player.level < rules.max_level {
        let next_level_experience =
            calculate_experience(growth_rates, &player.species.growth_rate, player.level + 1)?;
        if player.experience < next_level_experience {
            break;
        }
        player.level += 1;
        refresh_level_stats(player);
        for learned in learn_moves_for_current_level(player, moves, learnsets)? {
            learned_moves.push(learned.name);
        }
    }
    Ok(PokemonLevelUpOutcome {
        level_before,
        level_after: player.level,
        experience_before,
        experience_after: player.experience,
        learned_moves,
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
    while leveled.level < target_level {
        leveled.level += 1;
        leveled.experience = leveled.experience.max(calculate_experience(
            growth_rates,
            &leveled.species.growth_rate,
            leveled.level,
        )?);
        refresh_level_stats(&mut leveled);
        for learned in learn_moves_for_current_level(&mut leveled, moves, learnsets)? {
            learned_moves.push(learned.name);
        }
    }
    *player = leveled;
    Ok(PokemonLevelUpOutcome {
        level_before,
        level_after: player.level,
        experience_before,
        experience_after: player.experience,
        learned_moves,
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

fn learn_moves_for_current_level(
    player: &mut Pokemon,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
) -> Result<Vec<LearnedMove>, BattleRewardError> {
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
    for crate::systems::learnsets::LearnsetEntry(level, move_name) in entries {
        if *level != player.level
            || player.moves.iter().any(|known| known.name == *move_name)
            || player.moves.len() >= 4
        {
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
        player.moves.push(entry.clone());
        learned.push(entry);
    }
    Ok(learned)
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

    #[test]
    fn battle_reward_rules_issues_validate_declared_rules() {
        assert_eq!(
            battle_reward_rules_issues(&BattleRewardRules::default()),
            []
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
