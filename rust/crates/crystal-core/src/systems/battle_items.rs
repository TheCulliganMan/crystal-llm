use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::models::pokemon::StatExperience;
use crate::models::{Item, Move, Party, Pokemon, PokemonSpecies, Stat, calculate_stats};
use crate::systems::battle_rewards::{
    BattleRewardError, BattleRewardRules, apply_direct_level_gain,
};
use crate::systems::evolution::{
    EvolutionContext, EvolutionError, EvolutionEvent, EvolutionTable, LinkMode, check_and_evolve,
};
use crate::systems::experience::GrowthRateCatalog;
use crate::systems::learnsets::SpeciesLearnsets;
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleItemPpChange {
    pub move_slot: usize,
    pub move_id: String,
    pub pp_before: u8,
    pub pp_after: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleItemStatChange {
    pub stat: String,
    pub stat_exp_before: u16,
    pub stat_exp_after: u16,
    pub stat_before: u16,
    pub stat_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleItemStageChange {
    pub stat: String,
    pub stage_before: i8,
    pub stage_after: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyItemReviveChange {
    pub party_index: usize,
    pub pokemon_id: String,
    pub hp_before: u16,
    pub hp_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyItemOutcome {
    pub item_id: String,
    pub revive_changes: Vec<PartyItemReviveChange>,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleItemOutcome {
    pub item_id: String,
    pub hp_before: u16,
    pub hp_after: u16,
    pub level_before: u8,
    pub level_after: u8,
    pub experience_before: i32,
    pub experience_after: i32,
    pub status_before: Option<String>,
    pub status_after: Option<String>,
    pub confusion_turns_before: u8,
    pub confusion_turns_after: u8,
    pub focus_energy_before: bool,
    pub focus_energy_after: bool,
    pub pp_changes: Vec<BattleItemPpChange>,
    pub stat_changes: Vec<BattleItemStatChange>,
    pub battle_stat_stage_changes: Vec<BattleItemStageChange>,
    pub learned_moves: Vec<String>,
    pub evolution_target: Option<String>,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum BattleItemError {
    #[error("battle item {item_id} declares no battle item payload")]
    MissingBattleItemPayload { item_id: String },
    #[error("battle item {item_id} has invalid heal amount {amount}")]
    InvalidHealAmount { item_id: String, amount: i16 },
    #[error("battle item {item_id} cannot heal a fainted Pokemon")]
    TargetFainted { item_id: String },
    #[error("battle item {item_id} declares STATUS_HEAL without status_heals")]
    MissingStatusHeals { item_id: String },
    #[error("battle item {item_id} declares REVIVE without revive_hp_percent")]
    MissingReviveHpPercent { item_id: String },
    #[error("battle item {item_id} has invalid revive HP percent {percent}")]
    InvalidReviveHpPercent { item_id: String, percent: u8 },
    #[error("battle item {item_id} declares party revive without party_revive_hp_percent")]
    MissingPartyReviveHpPercent { item_id: String },
    #[error("battle item {item_id} has invalid party revive HP percent {percent}")]
    InvalidPartyReviveHpPercent { item_id: String, percent: u8 },
    #[error("battle item {item_id} declares RESTORE_PP without pp_restore_scope")]
    MissingPpRestoreScope { item_id: String },
    #[error("battle item {item_id} has invalid PP restore scope {scope}")]
    InvalidPpRestoreScope { item_id: String, scope: String },
    #[error("battle item {item_id} requires a move slot for PP restore")]
    MissingMoveSlot { item_id: String },
    #[error("battle item {item_id} move slot {slot} is outside the target moves")]
    MoveSlotOutOfRange { item_id: String, slot: usize },
    #[error("battle item {item_id} references unknown move {move_id}")]
    UnknownMove { item_id: String, move_id: String },
    #[error("battle item {item_id} has invalid PP restore points {points}")]
    InvalidPpRestorePoints { item_id: String, points: u8 },
    #[error("battle item {item_id} declares PP_UP without pp_up_stages")]
    MissingPpUpStages { item_id: String },
    #[error("battle item {item_id} has invalid PP Up stages {stages}")]
    InvalidPpUpStages { item_id: String, stages: u8 },
    #[error("battle item {item_id} declares VITAMIN without vitamin_stat")]
    MissingVitaminStat { item_id: String },
    #[error("battle item {item_id} has invalid vitamin stat {stat}")]
    InvalidVitaminStat { item_id: String, stat: String },
    #[error("battle item {item_id} declares VITAMIN without vitamin_stat_exp")]
    MissingVitaminStatExp { item_id: String },
    #[error("battle item {item_id} has invalid vitamin stat exp {amount}")]
    InvalidVitaminStatExp { item_id: String, amount: u16 },
    #[error("battle item {item_id} declares VITAMIN without vitamin_max_stat_exp")]
    MissingVitaminMaxStatExp { item_id: String },
    #[error("battle item {item_id} has invalid vitamin max stat exp {max}")]
    InvalidVitaminMaxStatExp { item_id: String, max: u16 },
    #[error("battle item {item_id} declares RARE_CANDY without rare_candy_level_gain")]
    MissingRareCandyLevelGain { item_id: String },
    #[error("battle item {item_id} has invalid rare candy level gain {level_gain}")]
    InvalidRareCandyLevelGain { item_id: String, level_gain: u8 },
    #[error("battle item {item_id} rare candy level-up is missing learnset for {species_id}")]
    RareCandyMissingLearnset { item_id: String, species_id: String },
    #[error("battle item {item_id} rare candy level-up references unknown move {move_id}")]
    RareCandyMissingMoveData { item_id: String, move_id: String },
    #[error("battle item {item_id} rare candy evolution failed: {error}")]
    RareCandyEvolution { item_id: String, error: String },
    #[error("battle item {item_id} evolution stone failed: {error}")]
    EvolutionStone { item_id: String, error: String },
    #[error("battle item {item_id} declares battle stat boost without battle_stat_boost_stat")]
    MissingBattleStatBoostStat { item_id: String },
    #[error("battle item {item_id} has invalid battle stat boost stat {stat}")]
    InvalidBattleStatBoostStat { item_id: String, stat: String },
    #[error("battle item {item_id} declares battle stat boost without battle_stat_boost_stages")]
    MissingBattleStatBoostStages { item_id: String },
    #[error("battle item {item_id} has invalid battle stat boost stages {stages}")]
    InvalidBattleStatBoostStages { item_id: String, stages: u8 },
    #[error("battle item {item_id} declares DIRE_HIT without battle_focus_energy")]
    MissingBattleFocusEnergy { item_id: String },
    #[error("battle item {item_id} has invalid battle_focus_energy false")]
    InvalidBattleFocusEnergy { item_id: String },
    #[error("battle item {item_id} declares BITTER_BERRY without confusion_heal")]
    MissingConfusionHeal { item_id: String },
    #[error("battle item {item_id} has invalid confusion_heal false")]
    InvalidConfusionHeal { item_id: String },
    #[error("battle item {item_id} declares battle escape without battle_escape_mode")]
    MissingBattleEscapeMode { item_id: String },
    #[error("battle item {item_id} declares invalid battle_escape_mode {mode}")]
    InvalidBattleEscapeMode { item_id: String, mode: String },
    #[error("battle item {item_id} declares battle stat drop guard without battle_stat_drop_guard")]
    MissingBattleStatDropGuard { item_id: String },
    #[error("battle item {item_id} has invalid battle_stat_drop_guard false")]
    InvalidBattleStatDropGuard { item_id: String },
    #[error("battle item {item_id} declares battle stat drop guard without turns")]
    MissingBattleStatDropGuardTurns { item_id: String },
    #[error("battle item {item_id} has invalid battle stat drop guard turns {turns}")]
    InvalidBattleStatDropGuardTurns { item_id: String, turns: u8 },
    #[error("battle item {item_id} would not change the target")]
    NoTargetChange { item_id: String },
}

pub fn validate_battle_escape_item(item: &Item) -> Result<&str, BattleItemError> {
    let mode = item.battle_escape_mode.as_deref().ok_or_else(|| {
        BattleItemError::MissingBattleEscapeMode {
            item_id: item.script_name.clone(),
        }
    })?;
    if mode != "WILD_BATTLE" {
        return Err(BattleItemError::InvalidBattleEscapeMode {
            item_id: item.script_name.clone(),
            mode: mode.to_string(),
        });
    }
    Ok(mode)
}

pub fn validate_battle_stat_drop_guard_item(item: &Item) -> Result<u8, BattleItemError> {
    match item.battle_stat_drop_guard {
        Some(true) => {}
        Some(false) => {
            return Err(BattleItemError::InvalidBattleStatDropGuard {
                item_id: item.script_name.clone(),
            });
        }
        None => {
            return Err(BattleItemError::MissingBattleStatDropGuard {
                item_id: item.script_name.clone(),
            });
        }
    }
    let turns = item.battle_stat_drop_guard_turns.ok_or_else(|| {
        BattleItemError::MissingBattleStatDropGuardTurns {
            item_id: item.script_name.clone(),
        }
    })?;
    if turns == 0 {
        return Err(BattleItemError::InvalidBattleStatDropGuardTurns {
            item_id: item.script_name.clone(),
            turns,
        });
    }
    Ok(turns)
}

pub fn apply_active_battle_item_effect(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    if item.revive_hp_percent.is_some() {
        return apply_revive(pokemon, item, consumed);
    }
    if item.vitamin_stat.is_some()
        || item.vitamin_stat_exp.is_some()
        || item.vitamin_max_stat_exp.is_some()
    {
        return apply_vitamin(pokemon, item, consumed);
    }
    if item.battle_stat_boost_stat.is_some() || item.battle_stat_boost_stages.is_some() {
        return apply_battle_stat_boost(pokemon, item, consumed);
    }
    if item.battle_focus_energy.is_some() {
        return apply_battle_focus_energy(pokemon, item, consumed);
    }
    if item.confusion_heal.is_some() {
        return apply_confusion_heal(pokemon, item, consumed);
    }
    if !item.status_heals.is_empty() && item.parameter != 0 {
        return apply_full_restore(pokemon, item, consumed);
    }
    if !item.status_heals.is_empty() {
        return apply_status_heal(pokemon, item, consumed);
    }
    if item.parameter != 0 {
        return apply_restore_hp(pokemon, item, consumed);
    }

    Err(BattleItemError::MissingBattleItemPayload {
        item_id: item.script_name.clone(),
    })
}

pub fn apply_battle_pp_item_effect(
    pokemon: &mut Pokemon,
    item: &Item,
    moves: &BTreeMap<String, Move>,
    move_slot: Option<usize>,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    if item.pp_up_stages.is_some() {
        return apply_pp_up(pokemon, item, moves, move_slot, consumed);
    }
    if item.pp_restore_scope.is_some() || item.pp_restore_points.is_some() {
        return apply_restore_pp(pokemon, item, moves, move_slot, consumed);
    }

    Err(BattleItemError::MissingBattleItemPayload {
        item_id: item.script_name.clone(),
    })
}

pub fn apply_party_wide_item_effect(
    party: &mut Party,
    item: &Item,
    consumed: bool,
) -> Result<PartyItemOutcome, BattleItemError> {
    if item.party_revive_hp_percent.is_some() {
        return apply_sacred_ash(party, item, consumed);
    }

    Err(BattleItemError::MissingBattleItemPayload {
        item_id: item.script_name.clone(),
    })
}

fn apply_sacred_ash(
    party: &mut Party,
    item: &Item,
    consumed: bool,
) -> Result<PartyItemOutcome, BattleItemError> {
    let percent = item.party_revive_hp_percent.ok_or_else(|| {
        BattleItemError::MissingPartyReviveHpPercent {
            item_id: item.script_name.clone(),
        }
    })?;
    if percent == 0 || percent > 100 {
        return Err(BattleItemError::InvalidPartyReviveHpPercent {
            item_id: item.script_name.clone(),
            percent,
        });
    }

    let mut changes = Vec::new();
    for (party_index, pokemon) in party.pokemon.iter().enumerate() {
        let Some(pokemon) = pokemon.as_ref() else {
            continue;
        };
        if pokemon.hp != 0 {
            continue;
        }
        let hp_after = ((u32::from(pokemon.max_hp) * u32::from(percent)) / 100)
            .clamp(1, u32::from(pokemon.max_hp)) as u16;
        changes.push(PartyItemReviveChange {
            party_index,
            pokemon_id: pokemon.species.id.clone(),
            hp_before: pokemon.hp,
            hp_after,
        });
    }
    if changes.is_empty() {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }
    for change in &changes {
        if let Some(pokemon) = party.pokemon[change.party_index].as_mut() {
            pokemon.hp = change.hp_after;
        }
    }

    Ok(PartyItemOutcome {
        item_id: item.script_name.clone(),
        revive_changes: changes,
        consumed,
    })
}

pub fn apply_rare_candy_item_effect(
    pokemon: &mut Pokemon,
    item: &Item,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    reward_rules: &BattleRewardRules,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let level_gain =
        item.rare_candy_level_gain
            .ok_or_else(|| BattleItemError::MissingRareCandyLevelGain {
                item_id: item.script_name.clone(),
            })?;
    if level_gain == 0 {
        return Err(BattleItemError::InvalidRareCandyLevelGain {
            item_id: item.script_name.clone(),
            level_gain,
        });
    }
    if reward_rules.max_level > 0 && pokemon.level >= reward_rules.max_level {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    let hp_before = pokemon.hp;
    let status_before = pokemon.status.clone();
    let level_up = apply_direct_level_gain(
        pokemon,
        moves,
        learnsets,
        growth_rates,
        reward_rules,
        level_gain,
    )
    .map_err(|error| rare_candy_level_up_error(&item.script_name, error))?;
    let evolution_context = EvolutionContext {
        species,
        moves,
        learnsets,
        time_of_day,
        current_item: None,
        force_evolution: false,
        link_mode: LinkMode::None,
    };
    let evolution = check_and_evolve(pokemon, evolutions, &evolution_context, true)
        .map_err(|error| rare_candy_evolution_error(&item.script_name, error))?;

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: level_up.level_before,
        level_after: pokemon.level,
        experience_before: level_up.experience_before,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: level_up.learned_moves,
        evolution_target: evolution.target_species,
        consumed,
    })
}

pub fn apply_evolution_stone_item_effect(
    pokemon: &mut Pokemon,
    item: &Item,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let hp_before = pokemon.hp;
    let level_before = pokemon.level;
    let experience_before = pokemon.experience;
    let status_before = pokemon.status.clone();
    let evolution_context = EvolutionContext {
        species,
        moves,
        learnsets,
        time_of_day,
        current_item: Some(item.script_name.as_str()),
        force_evolution: true,
        link_mode: LinkMode::None,
    };
    let evolution = check_and_evolve(pokemon, evolutions, &evolution_context, true)
        .map_err(|error| evolution_stone_error(&item.script_name, error))?;
    let Some(evolution_target) = evolution.target_species else {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    };
    let learned_moves = evolution
        .events
        .iter()
        .filter_map(|event| match event {
            EvolutionEvent::MoveLearned(move_id) => Some(move_id.clone()),
            _ => None,
        })
        .collect();

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before,
        level_after: pokemon.level,
        experience_before,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves,
        evolution_target: Some(evolution_target),
        consumed,
    })
}

fn apply_restore_hp(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let status_before = pokemon.status.clone();
    let hp_before = pokemon.hp;
    restore_hp(pokemon, item)?;
    if pokemon.hp == hp_before {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_full_restore(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let status_before = pokemon.status.clone();
    let hp_before = pokemon.hp;
    restore_hp(pokemon, item)?;
    clear_status(pokemon);
    if pokemon.hp == hp_before && pokemon.status == status_before {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_status_heal(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    if item.status_heals.is_empty() {
        return Err(BattleItemError::MissingStatusHeals {
            item_id: item.script_name.clone(),
        });
    }
    let status_before = pokemon.status.clone();
    let hp_before = pokemon.hp;
    let Some(status) = pokemon.status.as_deref() else {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    };
    if !item
        .status_heals
        .iter()
        .any(|healed_status| healed_status == status)
    {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }
    clear_status(pokemon);

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_revive(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let Some(percent) = item.revive_hp_percent else {
        return Err(BattleItemError::MissingReviveHpPercent {
            item_id: item.script_name.clone(),
        });
    };
    if percent == 0 || percent > 100 {
        return Err(BattleItemError::InvalidReviveHpPercent {
            item_id: item.script_name.clone(),
            percent,
        });
    }
    if pokemon.hp > 0 {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    let status_before = pokemon.status.clone();
    let hp_before = pokemon.hp;
    let revived_hp = ((u32::from(pokemon.max_hp) * u32::from(percent)) / 100)
        .clamp(1, u32::from(pokemon.max_hp)) as u16;
    pokemon.hp = revived_hp;

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_restore_pp(
    pokemon: &mut Pokemon,
    item: &Item,
    moves: &BTreeMap<String, Move>,
    move_slot: Option<usize>,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let Some(scope) = item.pp_restore_scope.as_deref() else {
        return Err(BattleItemError::MissingPpRestoreScope {
            item_id: item.script_name.clone(),
        });
    };
    if matches!(item.pp_restore_points, Some(0)) {
        return Err(BattleItemError::InvalidPpRestorePoints {
            item_id: item.script_name.clone(),
            points: 0,
        });
    }

    let target_slots = match scope {
        "MOVE" => {
            let slot = move_slot.ok_or_else(|| BattleItemError::MissingMoveSlot {
                item_id: item.script_name.clone(),
            })?;
            if slot >= pokemon.moves.len() {
                return Err(BattleItemError::MoveSlotOutOfRange {
                    item_id: item.script_name.clone(),
                    slot,
                });
            }
            vec![slot]
        }
        "POKEMON" => (0..pokemon.moves.len()).collect(),
        other => {
            return Err(BattleItemError::InvalidPpRestoreScope {
                item_id: item.script_name.clone(),
                scope: other.to_string(),
            });
        }
    };

    let mut changes = Vec::new();
    for slot in target_slots {
        let learned = &pokemon.moves[slot];
        let move_data = moves
            .get(&learned.name)
            .ok_or_else(|| BattleItemError::UnknownMove {
                item_id: item.script_name.clone(),
                move_id: learned.name.clone(),
            })?;
        let max_pp = max_move_pp(move_data.pp, learned.pp_ups);
        let pp_after = match item.pp_restore_points {
            Some(points) => max_pp.min(learned.current_pp.saturating_add(points)),
            None => max_pp,
        };
        if pp_after > learned.current_pp {
            changes.push(BattleItemPpChange {
                move_slot: slot,
                move_id: learned.name.clone(),
                pp_before: learned.current_pp,
                pp_after,
            });
        }
    }

    if changes.is_empty() {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    let status_before = pokemon.status.clone();
    let hp_before = pokemon.hp;
    for change in &changes {
        pokemon.moves[change.move_slot].current_pp = change.pp_after;
    }

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: changes,
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_pp_up(
    pokemon: &mut Pokemon,
    item: &Item,
    moves: &BTreeMap<String, Move>,
    move_slot: Option<usize>,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let stages = item
        .pp_up_stages
        .ok_or_else(|| BattleItemError::MissingPpUpStages {
            item_id: item.script_name.clone(),
        })?;
    if stages == 0 || stages > 3 {
        return Err(BattleItemError::InvalidPpUpStages {
            item_id: item.script_name.clone(),
            stages,
        });
    }
    let slot = move_slot.ok_or_else(|| BattleItemError::MissingMoveSlot {
        item_id: item.script_name.clone(),
    })?;
    if slot >= pokemon.moves.len() {
        return Err(BattleItemError::MoveSlotOutOfRange {
            item_id: item.script_name.clone(),
            slot,
        });
    }
    let learned = &pokemon.moves[slot];
    let move_data = moves
        .get(&learned.name)
        .ok_or_else(|| BattleItemError::UnknownMove {
            item_id: item.script_name.clone(),
            move_id: learned.name.clone(),
        })?;
    if learned.pp_ups >= 3 {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    let pp_ups_after = learned.pp_ups.saturating_add(stages).min(3);
    let pp_before = learned.current_pp;
    let max_before = max_move_pp(move_data.pp, learned.pp_ups);
    let max_after = max_move_pp(move_data.pp, pp_ups_after);
    let pp_gain = max_after.saturating_sub(max_before);
    let pp_after = max_after.min(pp_before.saturating_add(pp_gain));
    let status_before = pokemon.status.clone();
    let hp_before = pokemon.hp;

    pokemon.moves[slot].pp_ups = pp_ups_after;
    pokemon.moves[slot].current_pp = pp_after;

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: vec![BattleItemPpChange {
            move_slot: slot,
            move_id: pokemon.moves[slot].name.clone(),
            pp_before,
            pp_after,
        }],
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_vitamin(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let stat_id =
        item.vitamin_stat
            .as_deref()
            .ok_or_else(|| BattleItemError::MissingVitaminStat {
                item_id: item.script_name.clone(),
            })?;
    let stat = vitamin_stat(stat_id).ok_or_else(|| BattleItemError::InvalidVitaminStat {
        item_id: item.script_name.clone(),
        stat: stat_id.to_string(),
    })?;
    let amount = item
        .vitamin_stat_exp
        .ok_or_else(|| BattleItemError::MissingVitaminStatExp {
            item_id: item.script_name.clone(),
        })?;
    if amount == 0 {
        return Err(BattleItemError::InvalidVitaminStatExp {
            item_id: item.script_name.clone(),
            amount,
        });
    }
    let max =
        item.vitamin_max_stat_exp
            .ok_or_else(|| BattleItemError::MissingVitaminMaxStatExp {
                item_id: item.script_name.clone(),
            })?;
    if max == 0 || max < amount {
        return Err(BattleItemError::InvalidVitaminMaxStatExp {
            item_id: item.script_name.clone(),
            max,
        });
    }

    let stat_exp_before = pokemon_stat_exp(pokemon, stat);
    if stat_exp_before >= max {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }
    let stat_exp_after = max.min(stat_exp_before.saturating_add(amount));
    let stat_before = pokemon_stat_value(pokemon, stat);
    let hp_before = pokemon.hp;
    let max_hp_before = pokemon.max_hp;
    let status_before = pokemon.status.clone();

    set_pokemon_stat_exp(pokemon, stat, stat_exp_after);
    recalculate_pokemon_stats(pokemon, max_hp_before);
    let stat_after = pokemon_stat_value(pokemon, stat);

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: vec![BattleItemStatChange {
            stat: stat_id.to_string(),
            stat_exp_before,
            stat_exp_after,
            stat_before,
            stat_after,
        }],
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_battle_stat_boost(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let stat_id = item.battle_stat_boost_stat.as_deref().ok_or_else(|| {
        BattleItemError::MissingBattleStatBoostStat {
            item_id: item.script_name.clone(),
        }
    })?;
    let stat = battle_stat_boost_stat(stat_id).ok_or_else(|| {
        BattleItemError::InvalidBattleStatBoostStat {
            item_id: item.script_name.clone(),
            stat: stat_id.to_string(),
        }
    })?;
    let stages = item.battle_stat_boost_stages.ok_or_else(|| {
        BattleItemError::MissingBattleStatBoostStages {
            item_id: item.script_name.clone(),
        }
    })?;
    if stages == 0 || stages > 6 {
        return Err(BattleItemError::InvalidBattleStatBoostStages {
            item_id: item.script_name.clone(),
            stages,
        });
    }
    let stage_before = pokemon.stat_boosts.get(&stat).copied().ok_or_else(|| {
        BattleItemError::InvalidBattleStatBoostStat {
            item_id: item.script_name.clone(),
            stat: stat_id.to_string(),
        }
    })?;
    if stage_before >= 6 {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }
    let stage_after = (stage_before + stages as i8).min(6);
    let hp_before = pokemon.hp;
    let status_before = pokemon.status.clone();
    pokemon.stat_boosts.insert(stat, stage_after);

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before: pokemon.confusion_turns,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before: pokemon.focus_energy,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: vec![BattleItemStageChange {
            stat: stat_id.to_string(),
            stage_before,
            stage_after,
        }],
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_battle_focus_energy(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let focus_energy =
        item.battle_focus_energy
            .ok_or_else(|| BattleItemError::MissingBattleFocusEnergy {
                item_id: item.script_name.clone(),
            })?;
    if !focus_energy {
        return Err(BattleItemError::InvalidBattleFocusEnergy {
            item_id: item.script_name.clone(),
        });
    }
    if pokemon.focus_energy {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    let hp_before = pokemon.hp;
    let status_before = pokemon.status.clone();
    let confusion_turns_before = pokemon.confusion_turns;
    let focus_energy_before = pokemon.focus_energy;
    pokemon.focus_energy = true;

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn apply_confusion_heal(
    pokemon: &mut Pokemon,
    item: &Item,
    consumed: bool,
) -> Result<BattleItemOutcome, BattleItemError> {
    let confusion_heal =
        item.confusion_heal
            .ok_or_else(|| BattleItemError::MissingConfusionHeal {
                item_id: item.script_name.clone(),
            })?;
    if !confusion_heal {
        return Err(BattleItemError::InvalidConfusionHeal {
            item_id: item.script_name.clone(),
        });
    }
    if pokemon.confusion_turns == 0 {
        return Err(BattleItemError::NoTargetChange {
            item_id: item.script_name.clone(),
        });
    }

    let hp_before = pokemon.hp;
    let status_before = pokemon.status.clone();
    let confusion_turns_before = pokemon.confusion_turns;
    let focus_energy_before = pokemon.focus_energy;
    pokemon.confusion_turns = 0;

    Ok(BattleItemOutcome {
        item_id: item.script_name.clone(),
        hp_before,
        hp_after: pokemon.hp,
        level_before: pokemon.level,
        level_after: pokemon.level,
        experience_before: pokemon.experience,
        experience_after: pokemon.experience,
        status_before,
        status_after: pokemon.status.clone(),
        confusion_turns_before,
        confusion_turns_after: pokemon.confusion_turns,
        focus_energy_before,
        focus_energy_after: pokemon.focus_energy,
        pp_changes: Vec::new(),
        stat_changes: Vec::new(),
        battle_stat_stage_changes: Vec::new(),
        learned_moves: Vec::new(),
        evolution_target: None,
        consumed,
    })
}

fn battle_stat_boost_stat(stat_id: &str) -> Option<Stat> {
    match stat_id {
        "ATTACK" => Some(Stat::Attack),
        "DEFENSE" => Some(Stat::Defense),
        "SPEED" => Some(Stat::Speed),
        "SPECIAL_ATTACK" => Some(Stat::SpecialAttack),
        "ACCURACY" => Some(Stat::Accuracy),
        _ => None,
    }
}

fn rare_candy_level_up_error(item_id: &str, error: BattleRewardError) -> BattleItemError {
    match error {
        BattleRewardError::MissingLearnset { species_id } => {
            BattleItemError::RareCandyMissingLearnset {
                item_id: item_id.to_string(),
                species_id,
            }
        }
        BattleRewardError::MissingMoveData { move_id } => {
            BattleItemError::RareCandyMissingMoveData {
                item_id: item_id.to_string(),
                move_id,
            }
        }
        other => BattleItemError::RareCandyEvolution {
            item_id: item_id.to_string(),
            error: other.to_string(),
        },
    }
}

fn rare_candy_evolution_error(item_id: &str, error: EvolutionError) -> BattleItemError {
    BattleItemError::RareCandyEvolution {
        item_id: item_id.to_string(),
        error: error.to_string(),
    }
}

fn evolution_stone_error(item_id: &str, error: EvolutionError) -> BattleItemError {
    BattleItemError::EvolutionStone {
        item_id: item_id.to_string(),
        error: error.to_string(),
    }
}

fn vitamin_stat(stat_id: &str) -> Option<Stat> {
    match stat_id {
        "HP" => Some(Stat::Hp),
        "ATTACK" => Some(Stat::Attack),
        "DEFENSE" => Some(Stat::Defense),
        "SPEED" => Some(Stat::Speed),
        "SPECIAL" => Some(Stat::SpecialAttack),
        _ => None,
    }
}

fn pokemon_stat_exp(pokemon: &Pokemon, stat: Stat) -> u16 {
    match stat {
        Stat::Hp => pokemon.hp_exp,
        Stat::Attack => pokemon.attack_exp,
        Stat::Defense => pokemon.defense_exp,
        Stat::Speed => pokemon.speed_exp,
        Stat::SpecialAttack | Stat::SpecialDefense => pokemon.special_exp,
        Stat::Accuracy | Stat::Evasion => 0,
    }
}

fn set_pokemon_stat_exp(pokemon: &mut Pokemon, stat: Stat, value: u16) {
    match stat {
        Stat::Hp => pokemon.hp_exp = value,
        Stat::Attack => pokemon.attack_exp = value,
        Stat::Defense => pokemon.defense_exp = value,
        Stat::Speed => pokemon.speed_exp = value,
        Stat::SpecialAttack | Stat::SpecialDefense => pokemon.special_exp = value,
        Stat::Accuracy | Stat::Evasion => {}
    }
}

fn pokemon_stat_value(pokemon: &Pokemon, stat: Stat) -> u16 {
    match stat {
        Stat::Hp => pokemon.max_hp,
        Stat::Attack => pokemon.attack,
        Stat::Defense => pokemon.defense,
        Stat::Speed => pokemon.speed,
        Stat::SpecialAttack | Stat::SpecialDefense => pokemon.special_attack,
        Stat::Accuracy | Stat::Evasion => 0,
    }
}

fn recalculate_pokemon_stats(pokemon: &mut Pokemon, max_hp_before: u16) {
    let stats = calculate_stats(
        &pokemon.species,
        pokemon.level,
        pokemon.dvs,
        StatExperience {
            hp: pokemon.hp_exp,
            attack: pokemon.attack_exp,
            defense: pokemon.defense_exp,
            speed: pokemon.speed_exp,
            special: pokemon.special_exp,
        },
    );
    pokemon.max_hp = stats.max_hp;
    pokemon.attack = stats.attack;
    pokemon.defense = stats.defense;
    pokemon.speed = stats.speed;
    pokemon.special_attack = stats.special_attack;
    pokemon.special_defense = stats.special_defense;
    if stats.max_hp > max_hp_before {
        pokemon.hp = pokemon
            .hp
            .saturating_add(stats.max_hp - max_hp_before)
            .min(stats.max_hp);
    } else {
        pokemon.hp = pokemon.hp.min(stats.max_hp);
    }
}

fn max_move_pp(base_pp: u8, pp_ups: u8) -> u8 {
    base_pp.saturating_add((base_pp / 5).saturating_mul(pp_ups.min(3)))
}

fn restore_hp(pokemon: &mut Pokemon, item: &Item) -> Result<(), BattleItemError> {
    if item.parameter == 0 || item.parameter < -1 {
        return Err(BattleItemError::InvalidHealAmount {
            item_id: item.script_name.clone(),
            amount: item.parameter,
        });
    }
    if pokemon.hp == 0 {
        return Err(BattleItemError::TargetFainted {
            item_id: item.script_name.clone(),
        });
    }

    if item.parameter == -1 {
        pokemon.hp = pokemon.max_hp;
    } else {
        let amount =
            u16::try_from(item.parameter).map_err(|_| BattleItemError::InvalidHealAmount {
                item_id: item.script_name.clone(),
                amount: item.parameter,
            })?;
        pokemon.hp = pokemon.max_hp.min(pokemon.hp.saturating_add(amount));
    }
    Ok(())
}

fn clear_status(pokemon: &mut Pokemon) {
    pokemon.status = None;
    pokemon.sleep_turns = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BaseStats, Dv, LearnedMove, PokemonSpecies, growth_rate, item_pocket, pokemon_type,
    };
    use crate::systems::evolution::EvolutionEntry;
    use crate::systems::experience::crystal_growth_rate_catalog_for_tests;
    use crate::systems::learnsets::LearnsetEntry;

    fn test_pokemon(hp: u16, max_hp: u16) -> Pokemon {
        let species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(max_hp, 49, 65, 45, 49, 65));
        let mut pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
        pokemon.hp = hp;
        pokemon.max_hp = max_hp;
        pokemon
    }

    fn test_item(effect: &str, parameter: i16) -> Item {
        let mut item = Item {
            name: "POTION".to_string(),
            description: String::new(),
            effect: effect.to_string(),
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
            parameter,
            property: String::new(),
            pocket: item_pocket("ITEM"),
            field_menu: "ITEMMENU_PARTY".to_string(),
            field_usable: true,
            battle_menu: "ITEMMENU_PARTY".to_string(),
            battle_usable: true,
            script_name: "POTION".to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        };
        if effect == "FULL_RESTORE" {
            item.status_heals = vec![
                "POISON".to_string(),
                "BURN".to_string(),
                "FREEZE".to_string(),
                "SLEEP".to_string(),
                "PARALYSIS".to_string(),
            ];
        }
        item
    }

    fn status_item(status_heals: Vec<&str>) -> Item {
        let mut item = test_item("STATUS_HEAL", 0);
        item.status_heals = status_heals
            .into_iter()
            .map(|status| status.to_string())
            .collect();
        item
    }

    fn revive_item(percent: Option<u8>) -> Item {
        let mut item = test_item("REVIVE", 0);
        item.revive_hp_percent = percent;
        item
    }

    fn pp_item(scope: Option<&str>, points: Option<u8>) -> Item {
        let mut item = test_item("MOD_RESTORE_PP", 0);
        item.script_name = "ETHER".to_string();
        item.name = "ETHER".to_string();
        item.pp_restore_scope = scope.map(str::to_string);
        item.pp_restore_points = points;
        item
    }

    #[test]
    fn battle_escape_item_uses_payload_without_hardcoded_effect_name() {
        let mut item = test_item("MOD_DOLL", 0);
        item.script_name = "MOD_DOLL".to_string();
        item.battle_escape_mode = Some("WILD_BATTLE".to_string());

        assert_eq!(
            validate_battle_escape_item(&item).expect("payload escape item accepted"),
            "WILD_BATTLE"
        );

        item.battle_escape_mode = Some("TRAINER_BATTLE".to_string());
        assert_eq!(
            validate_battle_escape_item(&item).expect_err("invalid mode rejected"),
            BattleItemError::InvalidBattleEscapeMode {
                item_id: "MOD_DOLL".to_string(),
                mode: "TRAINER_BATTLE".to_string(),
            }
        );
    }

    #[test]
    fn battle_stat_drop_guard_item_uses_payload_without_hardcoded_effect_name() {
        let mut item = test_item("MOD_GUARD", 0);
        item.script_name = "MOD_GUARD".to_string();
        item.battle_stat_drop_guard = Some(true);
        item.battle_stat_drop_guard_turns = Some(5);

        assert_eq!(
            validate_battle_stat_drop_guard_item(&item)
                .expect("payload stat drop guard item accepted"),
            5
        );

        item.battle_stat_drop_guard_turns = Some(0);
        assert_eq!(
            validate_battle_stat_drop_guard_item(&item).expect_err("zero turns rejected"),
            BattleItemError::InvalidBattleStatDropGuardTurns {
                item_id: "MOD_GUARD".to_string(),
                turns: 0,
            }
        );
    }

    fn pp_up_item(stages: Option<u8>) -> Item {
        let mut item = test_item("MOD_PP_UP", 0);
        item.script_name = "PP_UP".to_string();
        item.name = "PP_UP".to_string();
        item.pp_up_stages = stages;
        item
    }

    fn vitamin_item(stat: Option<&str>, amount: Option<u16>, max: Option<u16>) -> Item {
        let mut item = test_item("VITAMIN", 0);
        item.script_name = "HP_UP".to_string();
        item.name = "HP UP".to_string();
        item.vitamin_stat = stat.map(str::to_string);
        item.vitamin_stat_exp = amount;
        item.vitamin_max_stat_exp = max;
        item
    }

    fn rare_candy_item(level_gain: Option<u8>) -> Item {
        let mut item = test_item("RARE_CANDY", 0);
        item.script_name = "RARE_CANDY".to_string();
        item.name = "RARE CANDY".to_string();
        item.rare_candy_level_gain = level_gain;
        item
    }

    fn species_catalog() -> BTreeMap<String, PokemonSpecies> {
        let mut chikorita =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        chikorita.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        let mut bayleef =
            PokemonSpecies::new_for_tests("BAYLEEF", BaseStats::new(60, 62, 80, 60, 63, 80));
        bayleef.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        let mut pikachu =
            PokemonSpecies::new_for_tests("PIKACHU", BaseStats::new(35, 55, 30, 90, 50, 50));
        pikachu.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        let mut raichu =
            PokemonSpecies::new_for_tests("RAICHU", BaseStats::new(60, 90, 55, 100, 90, 80));
        raichu.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        [
            ("CHIKORITA".to_string(), chikorita),
            ("BAYLEEF".to_string(), bayleef),
            ("PIKACHU".to_string(), pikachu),
            ("RAICHU".to_string(), raichu),
        ]
        .into_iter()
        .collect()
    }

    fn rare_candy_moves() -> BTreeMap<String, Move> {
        let mut moves = move_catalog();
        moves.insert(
            "RAZOR_LEAF".to_string(),
            Move {
                name: "RAZOR_LEAF".to_string(),
                move_type: pokemon_type("GRASS"),
                power: 55,
                accuracy: 95,
                pp: 25,
                effect: "NORMAL_HIT".to_string(),
                effect_chance: 0,
                stat: None,
                amount: None,
            },
        );
        moves.insert(
            "THUNDERBOLT".to_string(),
            Move {
                name: "THUNDERBOLT".to_string(),
                move_type: pokemon_type("ELECTRIC"),
                power: 95,
                accuracy: 100,
                pp: 15,
                effect: "NORMAL_HIT".to_string(),
                effect_chance: 0,
                stat: None,
                amount: None,
            },
        );
        moves
    }

    fn reward_rules() -> BattleRewardRules {
        BattleRewardRules {
            max_level: 100,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
        }
    }

    fn evolution_stone_item(id: &str) -> Item {
        let mut item = test_item("MOD_STONE", 0);
        item.script_name = id.to_string();
        item.name = id.replace('_', " ");
        item
    }

    fn battle_boost_item(effect: &str, stat: Option<&str>, stages: Option<u8>) -> Item {
        let mut item = test_item(effect, 0);
        item.script_name = if effect == "X_ACCURACY" {
            "X_ACCURACY".to_string()
        } else {
            "X_ATTACK".to_string()
        };
        item.name = item.script_name.replace('_', " ");
        item.battle_stat_boost_stat = stat.map(str::to_string);
        item.battle_stat_boost_stages = stages;
        item
    }

    fn dire_hit_item(focus_energy: Option<bool>) -> Item {
        let mut item = test_item("DIRE_HIT", 0);
        item.script_name = "DIRE_HIT".to_string();
        item.name = "DIRE HIT".to_string();
        item.battle_focus_energy = focus_energy;
        item
    }

    fn bitter_berry_item(confusion_heal: Option<bool>) -> Item {
        let mut item = test_item("BITTER_BERRY", 0);
        item.script_name = "BITTER_BERRY".to_string();
        item.name = "BITTER BERRY".to_string();
        item.confusion_heal = confusion_heal;
        item
    }

    fn sacred_ash_item(percent: Option<u8>) -> Item {
        let mut item = test_item("MOD_ASH", 0);
        item.script_name = "MOD_ASH".to_string();
        item.name = "SACRED ASH".to_string();
        item.party_revive_hp_percent = percent;
        item
    }

    fn move_catalog() -> BTreeMap<String, Move> {
        [
            (
                "TACKLE".to_string(),
                Move {
                    name: "TACKLE".to_string(),
                    move_type: pokemon_type("NORMAL"),
                    power: 40,
                    accuracy: 100,
                    pp: 35,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                },
            ),
            (
                "GROWL".to_string(),
                Move {
                    name: "GROWL".to_string(),
                    move_type: pokemon_type("NORMAL"),
                    power: 0,
                    accuracy: 100,
                    pp: 40,
                    effect: "ATTACK_DOWN".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                },
            ),
        ]
        .into_iter()
        .collect()
    }

    fn pokemon_with_pp(tackle_pp: u8, growl_pp: u8) -> Pokemon {
        let mut pokemon = test_pokemon(35, 35);
        pokemon.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: tackle_pp,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: growl_pp,
                pp_ups: 0,
            },
        ];
        pokemon
    }

    #[test]
    fn battle_items_restore_hp_by_exact_modpack_effect() {
        let item = test_item("RESTORE_HP", 20);
        let mut pokemon = test_pokemon(17, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("heals Pokemon");

        assert_eq!(outcome.item_id, "POTION");
        assert_eq!(outcome.hp_before, 17);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(outcome.status_before, None);
        assert_eq!(outcome.status_after, None);
        assert!(outcome.consumed);
        assert_eq!(pokemon.hp, 35);
    }

    #[test]
    fn battle_items_max_restore_hp_uses_exact_negative_one_parameter() {
        let item = test_item("RESTORE_HP", -1);
        let mut pokemon = test_pokemon(1, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("max potion heals");

        assert_eq!(outcome.hp_before, 1);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(pokemon.hp, 35);
    }

    #[test]
    fn battle_items_full_restore_heals_hp_and_clears_status() {
        let item = test_item("FULL_RESTORE", -1);
        let mut pokemon = test_pokemon(17, 35);
        pokemon.status = Some("POISON".to_string());

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("full restore works");

        assert_eq!(outcome.item_id, "POTION");
        assert_eq!(outcome.hp_before, 17);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(outcome.status_before, Some("POISON".to_string()));
        assert_eq!(outcome.status_after, None);
        assert_eq!(pokemon.hp, 35);
        assert_eq!(pokemon.status, None);
    }

    #[test]
    fn battle_items_full_restore_can_clear_status_without_hp_change() {
        let item = test_item("FULL_RESTORE", -1);
        let mut pokemon = test_pokemon(35, 35);
        pokemon.status = Some("PARALYSIS".to_string());

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("status cleared");

        assert_eq!(outcome.hp_before, 35);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(outcome.status_before, Some("PARALYSIS".to_string()));
        assert_eq!(outcome.status_after, None);
        assert_eq!(pokemon.status, None);
    }

    #[test]
    fn battle_items_status_heal_uses_exact_modpack_status_list() {
        let item = status_item(vec!["POISON"]);
        let mut pokemon = test_pokemon(35, 35);
        pokemon.status = Some("POISON".to_string());

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("status healed");

        assert_eq!(outcome.item_id, "POTION");
        assert_eq!(outcome.hp_before, 35);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(outcome.status_before, Some("POISON".to_string()));
        assert_eq!(outcome.status_after, None);
        assert_eq!(pokemon.status, None);
    }

    #[test]
    fn battle_items_status_heal_clears_sleep_turns_with_sleep_status() {
        let item = status_item(vec!["SLEEP"]);
        let mut pokemon = test_pokemon(35, 35);
        pokemon.status = Some("SLEEP".to_string());
        pokemon.sleep_turns = 3;

        apply_active_battle_item_effect(&mut pokemon, &item, true).expect("sleep healed");

        assert_eq!(pokemon.status, None);
        assert_eq!(pokemon.sleep_turns, 0);
    }

    #[test]
    fn battle_items_status_heal_rejects_missing_or_mismatched_pack_status_data() {
        let mut poisoned = test_pokemon(35, 35);
        poisoned.status = Some("POISON".to_string());
        let poisoned_before = poisoned.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut poisoned, &status_item(Vec::new()), true)
                .expect_err("payload-less status item is missing battle item payload"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(poisoned, poisoned_before);

        let mut burned = test_pokemon(35, 35);
        burned.status = Some("BURN".to_string());
        let burned_before = burned.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut burned, &status_item(vec!["POISON"]), true)
                .expect_err("mismatched status must not heal"),
            BattleItemError::NoTargetChange {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(burned, burned_before);
    }

    #[test]
    fn battle_items_revive_uses_exact_modpack_hp_percent() {
        let item = revive_item(Some(50));
        let mut pokemon = test_pokemon(0, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("revives Pokemon");

        assert_eq!(outcome.item_id, "POTION");
        assert_eq!(outcome.hp_before, 0);
        assert_eq!(outcome.hp_after, 17);
        assert_eq!(pokemon.hp, 17);
    }

    #[test]
    fn battle_items_max_revive_uses_exact_full_hp_percent() {
        let item = revive_item(Some(100));
        let mut pokemon = test_pokemon(0, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("max revive works");

        assert_eq!(outcome.hp_before, 0);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(pokemon.hp, 35);
    }

    #[test]
    fn battle_items_revive_rejects_missing_invalid_or_unfainted_targets_without_mutation() {
        let mut fainted = test_pokemon(0, 35);
        let fainted_before = fainted.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut fainted, &revive_item(None), true)
                .expect_err("payload-less revive item is missing battle item payload"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(fainted, fainted_before);

        let mut invalid = test_pokemon(0, 35);
        let invalid_before = invalid.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut invalid, &revive_item(Some(0)), true)
                .expect_err("zero percent revive is invalid"),
            BattleItemError::InvalidReviveHpPercent {
                item_id: "POTION".to_string(),
                percent: 0,
            }
        );
        assert_eq!(invalid, invalid_before);

        let mut healthy = test_pokemon(35, 35);
        let healthy_before = healthy.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut healthy, &revive_item(Some(50)), true)
                .expect_err("revive cannot target non-fainted Pokemon"),
            BattleItemError::NoTargetChange {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(healthy, healthy_before);
    }

    #[test]
    fn battle_items_restore_selected_move_pp_by_exact_modpack_points() {
        let item = pp_item(Some("MOVE"), Some(10));
        let moves = move_catalog();
        let mut pokemon = pokemon_with_pp(20, 1);

        let outcome = apply_battle_pp_item_effect(&mut pokemon, &item, &moves, Some(0), true)
            .expect("ether restores selected move");

        assert_eq!(outcome.item_id, "ETHER");
        assert_eq!(outcome.hp_before, 35);
        assert_eq!(outcome.hp_after, 35);
        assert_eq!(
            outcome.pp_changes,
            vec![BattleItemPpChange {
                move_slot: 0,
                move_id: "TACKLE".to_string(),
                pp_before: 20,
                pp_after: 30,
            }]
        );
        assert_eq!(pokemon.moves[0].current_pp, 30);
        assert_eq!(pokemon.moves[1].current_pp, 1);
        assert!(outcome.consumed);
    }

    #[test]
    fn battle_items_max_ether_restores_selected_move_pp_to_full() {
        let item = pp_item(Some("MOVE"), None);
        let moves = move_catalog();
        let mut pokemon = pokemon_with_pp(2, 1);

        let outcome = apply_battle_pp_item_effect(&mut pokemon, &item, &moves, Some(0), true)
            .expect("max ether restores selected move");

        assert_eq!(outcome.pp_changes[0].pp_after, 35);
        assert_eq!(pokemon.moves[0].current_pp, 35);
        assert_eq!(pokemon.moves[1].current_pp, 1);
    }

    #[test]
    fn battle_items_pp_restore_uses_stored_pp_up_stages_for_max_pp() {
        let item = pp_item(Some("MOVE"), None);
        let moves = move_catalog();
        let mut pokemon = pokemon_with_pp(2, 1);
        pokemon.moves[0].pp_ups = 1;

        let outcome = apply_battle_pp_item_effect(&mut pokemon, &item, &moves, Some(0), true)
            .expect("max ether respects PP Up stages");

        assert_eq!(outcome.pp_changes[0].pp_after, 42);
        assert_eq!(pokemon.moves[0].current_pp, 42);
    }

    #[test]
    fn battle_items_elixir_restores_all_moves_by_exact_modpack_points() {
        let item = pp_item(Some("POKEMON"), Some(10));
        let moves = move_catalog();
        let mut pokemon = pokemon_with_pp(28, 1);

        let outcome = apply_battle_pp_item_effect(&mut pokemon, &item, &moves, None, true)
            .expect("elixir restores all moves");

        assert_eq!(
            outcome.pp_changes,
            vec![
                BattleItemPpChange {
                    move_slot: 0,
                    move_id: "TACKLE".to_string(),
                    pp_before: 28,
                    pp_after: 35,
                },
                BattleItemPpChange {
                    move_slot: 1,
                    move_id: "GROWL".to_string(),
                    pp_before: 1,
                    pp_after: 11,
                },
            ]
        );
        assert_eq!(pokemon.moves[0].current_pp, 35);
        assert_eq!(pokemon.moves[1].current_pp, 11);
    }

    #[test]
    fn battle_items_max_elixir_restores_all_moves_to_full() {
        let item = pp_item(Some("POKEMON"), None);
        let moves = move_catalog();
        let mut pokemon = pokemon_with_pp(28, 1);

        let outcome = apply_battle_pp_item_effect(&mut pokemon, &item, &moves, None, true)
            .expect("max elixir restores all moves");

        assert_eq!(outcome.pp_changes[0].pp_after, 35);
        assert_eq!(outcome.pp_changes[1].pp_after, 40);
        assert_eq!(pokemon.moves[0].current_pp, 35);
        assert_eq!(pokemon.moves[1].current_pp, 40);
    }

    #[test]
    fn battle_items_pp_restore_rejects_missing_bad_scope_and_slot_without_mutation() {
        let moves = move_catalog();
        let mut missing_scope = pokemon_with_pp(20, 1);
        let missing_scope_before = missing_scope.clone();
        assert_eq!(
            apply_battle_pp_item_effect(
                &mut missing_scope,
                &pp_item(None, Some(10)),
                &moves,
                Some(0),
                true
            )
            .expect_err("scope is required"),
            BattleItemError::MissingPpRestoreScope {
                item_id: "ETHER".to_string(),
            }
        );
        assert_eq!(missing_scope, missing_scope_before);

        let mut bad_scope = pokemon_with_pp(20, 1);
        let bad_scope_before = bad_scope.clone();
        assert_eq!(
            apply_battle_pp_item_effect(
                &mut bad_scope,
                &pp_item(Some("move"), Some(10)),
                &moves,
                Some(0),
                true
            )
            .expect_err("case changed scope is invalid"),
            BattleItemError::InvalidPpRestoreScope {
                item_id: "ETHER".to_string(),
                scope: "move".to_string(),
            }
        );
        assert_eq!(bad_scope, bad_scope_before);

        let mut missing_slot = pokemon_with_pp(20, 1);
        let missing_slot_before = missing_slot.clone();
        assert_eq!(
            apply_battle_pp_item_effect(
                &mut missing_slot,
                &pp_item(Some("MOVE"), Some(10)),
                &moves,
                None,
                true
            )
            .expect_err("move scope requires selected slot"),
            BattleItemError::MissingMoveSlot {
                item_id: "ETHER".to_string(),
            }
        );
        assert_eq!(missing_slot, missing_slot_before);
    }

    #[test]
    fn battle_items_pp_restore_rejects_unknown_moves_and_no_effect_without_mutation() {
        let moves = move_catalog();
        let mut unknown = pokemon_with_pp(20, 1);
        unknown.moves[1].name = "growl".to_string();
        let unknown_before = unknown.clone();
        assert_eq!(
            apply_battle_pp_item_effect(
                &mut unknown,
                &pp_item(Some("POKEMON"), Some(10)),
                &moves,
                None,
                true
            )
            .expect_err("move ids are exact modpack ids"),
            BattleItemError::UnknownMove {
                item_id: "ETHER".to_string(),
                move_id: "growl".to_string(),
            }
        );
        assert_eq!(unknown, unknown_before);

        let mut full = pokemon_with_pp(35, 40);
        let full_before = full.clone();
        assert_eq!(
            apply_battle_pp_item_effect(
                &mut full,
                &pp_item(Some("POKEMON"), Some(10)),
                &moves,
                None,
                true
            )
            .expect_err("full PP has no item effect"),
            BattleItemError::NoTargetChange {
                item_id: "ETHER".to_string(),
            }
        );
        assert_eq!(full, full_before);
    }

    #[test]
    fn battle_items_pp_up_raises_move_pp_stage_and_current_pp() {
        let item = pp_up_item(Some(1));
        let moves = move_catalog();
        let mut pokemon = pokemon_with_pp(20, 1);

        let outcome = apply_battle_pp_item_effect(&mut pokemon, &item, &moves, Some(0), true)
            .expect("PP Up applies");

        assert_eq!(outcome.item_id, "PP_UP");
        assert_eq!(
            outcome.pp_changes,
            vec![BattleItemPpChange {
                move_slot: 0,
                move_id: "TACKLE".to_string(),
                pp_before: 20,
                pp_after: 27,
            }]
        );
        assert_eq!(pokemon.moves[0].pp_ups, 1);
        assert_eq!(pokemon.moves[0].current_pp, 27);
        assert_eq!(pokemon.moves[1].pp_ups, 0);
        assert_eq!(pokemon.moves[1].current_pp, 1);
    }

    #[test]
    fn battle_items_pp_up_rejects_missing_invalid_and_maxed_targets_without_mutation() {
        let moves = move_catalog();
        let mut missing = pokemon_with_pp(20, 1);
        let missing_before = missing.clone();
        assert_eq!(
            apply_battle_pp_item_effect(&mut missing, &pp_up_item(None), &moves, Some(0), true)
                .expect_err("payload-less PP Up is missing battle item payload"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "PP_UP".to_string(),
            }
        );
        assert_eq!(missing, missing_before);

        let mut invalid = pokemon_with_pp(20, 1);
        let invalid_before = invalid.clone();
        assert_eq!(
            apply_battle_pp_item_effect(&mut invalid, &pp_up_item(Some(0)), &moves, Some(0), true)
                .expect_err("zero PP Up stages are invalid"),
            BattleItemError::InvalidPpUpStages {
                item_id: "PP_UP".to_string(),
                stages: 0,
            }
        );
        assert_eq!(invalid, invalid_before);

        let mut maxed = pokemon_with_pp(56, 1);
        maxed.moves[0].pp_ups = 3;
        let maxed_before = maxed.clone();
        assert_eq!(
            apply_battle_pp_item_effect(&mut maxed, &pp_up_item(Some(1)), &moves, Some(0), true)
                .expect_err("maxed PP Up stages cannot increase"),
            BattleItemError::NoTargetChange {
                item_id: "PP_UP".to_string(),
            }
        );
        assert_eq!(maxed, maxed_before);
    }

    #[test]
    fn battle_items_vitamin_raises_stat_exp_from_exact_pack_fields() {
        let item = vitamin_item(Some("HP"), Some(2560), Some(25600));
        let species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        let mut pokemon = Pokemon::new_for_tests(species, 50, Dv::default());
        pokemon.hp = pokemon.max_hp - 5;
        pokemon.hp_exp = 0;
        let hp_before = pokemon.hp;
        let max_hp_before = pokemon.max_hp;

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("vitamin applies");

        assert_eq!(outcome.item_id, "HP_UP");
        assert_eq!(outcome.hp_before, hp_before);
        assert_eq!(outcome.stat_changes.len(), 1);
        assert_eq!(outcome.stat_changes[0].stat, "HP");
        assert_eq!(outcome.stat_changes[0].stat_exp_before, 0);
        assert_eq!(outcome.stat_changes[0].stat_exp_after, 2560);
        assert_eq!(outcome.stat_changes[0].stat_before, max_hp_before);
        assert_eq!(outcome.stat_changes[0].stat_after, pokemon.max_hp);
        assert_eq!(pokemon.hp_exp, 2560);
        assert!(pokemon.max_hp >= max_hp_before);
        assert_eq!(pokemon.hp, outcome.hp_after);
        assert!(outcome.consumed);
    }

    #[test]
    fn battle_items_vitamin_special_recalculates_both_special_stats() {
        let item = vitamin_item(Some("SPECIAL"), Some(2560), Some(25600));
        let mut pokemon = test_pokemon(35, 35);
        let special_attack_before = pokemon.special_attack;
        let special_defense_before = pokemon.special_defense;

        let outcome = apply_active_battle_item_effect(&mut pokemon, &item, true)
            .expect("special vitamin applies");

        assert_eq!(pokemon.special_exp, 2560);
        assert_eq!(outcome.stat_changes[0].stat, "SPECIAL");
        assert_eq!(outcome.stat_changes[0].stat_before, special_attack_before);
        assert_eq!(outcome.stat_changes[0].stat_after, pokemon.special_attack);
        assert!(pokemon.special_attack >= special_attack_before);
        assert!(pokemon.special_defense >= special_defense_before);
    }

    #[test]
    fn battle_items_vitamin_rejects_missing_bad_and_capped_pack_data_without_mutation() {
        let mut missing_stat = test_pokemon(35, 35);
        let missing_stat_before = missing_stat.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut missing_stat,
                &vitamin_item(None, Some(2560), Some(25600)),
                true
            )
            .expect_err("vitamin stat is required"),
            BattleItemError::MissingVitaminStat {
                item_id: "HP_UP".to_string(),
            }
        );
        assert_eq!(missing_stat, missing_stat_before);

        let mut bad_stat = test_pokemon(35, 35);
        let bad_stat_before = bad_stat.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut bad_stat,
                &vitamin_item(Some("hp"), Some(2560), Some(25600)),
                true
            )
            .expect_err("case changed vitamin stat is invalid"),
            BattleItemError::InvalidVitaminStat {
                item_id: "HP_UP".to_string(),
                stat: "hp".to_string(),
            }
        );
        assert_eq!(bad_stat, bad_stat_before);

        let mut missing_amount = test_pokemon(35, 35);
        let missing_amount_before = missing_amount.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut missing_amount,
                &vitamin_item(Some("HP"), None, Some(25600)),
                true
            )
            .expect_err("vitamin amount is required"),
            BattleItemError::MissingVitaminStatExp {
                item_id: "HP_UP".to_string(),
            }
        );
        assert_eq!(missing_amount, missing_amount_before);

        let mut zero_amount = test_pokemon(35, 35);
        let zero_amount_before = zero_amount.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut zero_amount,
                &vitamin_item(Some("HP"), Some(0), Some(25600)),
                true
            )
            .expect_err("zero vitamin amount is invalid"),
            BattleItemError::InvalidVitaminStatExp {
                item_id: "HP_UP".to_string(),
                amount: 0,
            }
        );
        assert_eq!(zero_amount, zero_amount_before);

        let mut missing_max = test_pokemon(35, 35);
        let missing_max_before = missing_max.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut missing_max,
                &vitamin_item(Some("HP"), Some(2560), None),
                true
            )
            .expect_err("vitamin max is required"),
            BattleItemError::MissingVitaminMaxStatExp {
                item_id: "HP_UP".to_string(),
            }
        );
        assert_eq!(missing_max, missing_max_before);

        let mut bad_max = test_pokemon(35, 35);
        let bad_max_before = bad_max.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut bad_max,
                &vitamin_item(Some("HP"), Some(2560), Some(1000)),
                true
            )
            .expect_err("vitamin max must cover amount"),
            BattleItemError::InvalidVitaminMaxStatExp {
                item_id: "HP_UP".to_string(),
                max: 1000,
            }
        );
        assert_eq!(bad_max, bad_max_before);

        let mut capped = test_pokemon(35, 35);
        capped.hp_exp = 25600;
        let capped_before = capped.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut capped,
                &vitamin_item(Some("HP"), Some(2560), Some(25600)),
                true
            )
            .expect_err("vitamin cap has no target change"),
            BattleItemError::NoTargetChange {
                item_id: "HP_UP".to_string(),
            }
        );
        assert_eq!(capped, capped_before);
    }

    #[test]
    fn rare_candy_raises_one_level_sets_exp_and_learns_exact_level_move() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut item = rare_candy_item(Some(1));
        item.effect = "MOD_CANDY".to_string();
        let species = species_catalog();
        let moves = rare_candy_moves();
        let learnsets = [(
            "CHIKORITA".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(10, "RAZOR_LEAF".to_string()),
            ],
        )]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );
        let mut pokemon = Pokemon::new_for_tests(species["CHIKORITA"].clone(), 9, Dv::default());
        pokemon.experience = 9_i32.pow(3);
        pokemon.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        let hp_before = pokemon.hp;

        let outcome = apply_rare_candy_item_effect(
            &mut pokemon,
            &item,
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &reward_rules(),
            &evolutions,
            TimeOfDay::Day,
            true,
        )
        .expect("rare candy applies");

        assert_eq!(outcome.item_id, "RARE_CANDY");
        assert_eq!(outcome.level_before, 9);
        assert_eq!(outcome.level_after, 10);
        assert_eq!(outcome.experience_after, 10_i32.pow(3));
        assert_eq!(outcome.learned_moves, vec!["RAZOR_LEAF".to_string()]);
        assert_eq!(outcome.evolution_target, None);
        assert_eq!(pokemon.level, 10);
        assert_eq!(pokemon.experience, 10_i32.pow(3));
        assert!(pokemon.hp > hp_before);
        assert_eq!(pokemon.moves[1].name, "RAZOR_LEAF");
        assert_eq!(pokemon.moves[1].current_pp, 25);
        assert!(outcome.consumed);
    }

    #[test]
    fn rare_candy_can_trigger_level_evolution_from_modpack_tables() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let item = rare_candy_item(Some(1));
        let species = species_catalog();
        let moves = rare_candy_moves();
        let learnsets = [
            (
                "CHIKORITA".to_string(),
                vec![LearnsetEntry(1, "TACKLE".to_string())],
            ),
            (
                "BAYLEEF".to_string(),
                vec![LearnsetEntry(1, "TACKLE".to_string())],
            ),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [(
                "CHIKORITA".to_string(),
                vec![EvolutionEntry::level("BAYLEEF", 16)],
            )]
            .into_iter()
            .collect(),
        );
        let mut pokemon = Pokemon::new_for_tests(species["CHIKORITA"].clone(), 15, Dv::default());
        pokemon.experience = 15_i32.pow(3);

        let outcome = apply_rare_candy_item_effect(
            &mut pokemon,
            &item,
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &reward_rules(),
            &evolutions,
            TimeOfDay::Day,
            true,
        )
        .expect("rare candy evolves");

        assert_eq!(outcome.level_before, 15);
        assert_eq!(outcome.level_after, 16);
        assert_eq!(outcome.evolution_target, Some("BAYLEEF".to_string()));
        assert_eq!(pokemon.species.id, "BAYLEEF");
        assert_eq!(pokemon.level, 16);
    }

    #[test]
    fn rare_candy_rejects_missing_invalid_or_maxed_targets_without_mutation() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let species = species_catalog();
        let moves = rare_candy_moves();
        let learnsets = [("CHIKORITA".to_string(), Vec::new())]
            .into_iter()
            .collect();
        let evolutions = EvolutionTable(
            [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
        );

        let mut missing = Pokemon::new_for_tests(species["CHIKORITA"].clone(), 9, Dv::default());
        let missing_before = missing.clone();
        assert_eq!(
            apply_rare_candy_item_effect(
                &mut missing,
                &rare_candy_item(None),
                &species,
                &moves,
                &learnsets,
                &growth_rates,
                &reward_rules(),
                &evolutions,
                TimeOfDay::Day,
                true,
            )
            .expect_err("level gain is required"),
            BattleItemError::MissingRareCandyLevelGain {
                item_id: "RARE_CANDY".to_string(),
            }
        );
        assert_eq!(missing, missing_before);

        let mut invalid = Pokemon::new_for_tests(species["CHIKORITA"].clone(), 9, Dv::default());
        let invalid_before = invalid.clone();
        assert_eq!(
            apply_rare_candy_item_effect(
                &mut invalid,
                &rare_candy_item(Some(0)),
                &species,
                &moves,
                &learnsets,
                &growth_rates,
                &reward_rules(),
                &evolutions,
                TimeOfDay::Day,
                true,
            )
            .expect_err("zero level gain is invalid"),
            BattleItemError::InvalidRareCandyLevelGain {
                item_id: "RARE_CANDY".to_string(),
                level_gain: 0,
            }
        );
        assert_eq!(invalid, invalid_before);

        let mut maxed = Pokemon::new_for_tests(species["CHIKORITA"].clone(), 100, Dv::default());
        let maxed_before = maxed.clone();
        assert_eq!(
            apply_rare_candy_item_effect(
                &mut maxed,
                &rare_candy_item(Some(1)),
                &species,
                &moves,
                &learnsets,
                &growth_rates,
                &reward_rules(),
                &evolutions,
                TimeOfDay::Day,
                true,
            )
            .expect_err("max level has no item effect"),
            BattleItemError::NoTargetChange {
                item_id: "RARE_CANDY".to_string(),
            }
        );
        assert_eq!(maxed, maxed_before);
    }

    #[test]
    fn evolution_stone_uses_exact_item_evolution_table_and_learns_target_moves() {
        let item = evolution_stone_item("THUNDERSTONE");
        let species = species_catalog();
        let moves = rare_candy_moves();
        let learnsets = [(
            "RAICHU".to_string(),
            vec![LearnsetEntry(20, "THUNDERBOLT".to_string())],
        )]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [(
                "PIKACHU".to_string(),
                vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
            )]
            .into_iter()
            .collect(),
        );
        let mut pokemon = Pokemon::new_for_tests(species["PIKACHU"].clone(), 20, Dv::default());
        pokemon.hp = pokemon.max_hp - 4;

        let outcome = apply_evolution_stone_item_effect(
            &mut pokemon,
            &item,
            &species,
            &moves,
            &learnsets,
            &evolutions,
            TimeOfDay::Day,
            true,
        )
        .expect("stone evolves");

        assert_eq!(outcome.item_id, "THUNDERSTONE");
        assert_eq!(outcome.evolution_target, Some("RAICHU".to_string()));
        assert_eq!(outcome.learned_moves, vec!["THUNDERBOLT".to_string()]);
        assert_eq!(pokemon.species.id, "RAICHU");
        assert_eq!(pokemon.moves[0].name, "THUNDERBOLT");
        assert_eq!(pokemon.moves[0].current_pp, 15);
        assert!(outcome.consumed);
    }

    #[test]
    fn evolution_stone_rejects_nonmatching_item_without_mutation() {
        let item = evolution_stone_item("FIRE_STONE");
        let species = species_catalog();
        let moves = rare_candy_moves();
        let learnsets = [("RAICHU".to_string(), Vec::new())].into_iter().collect();
        let evolutions = EvolutionTable(
            [(
                "PIKACHU".to_string(),
                vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
            )]
            .into_iter()
            .collect(),
        );
        let mut pokemon = Pokemon::new_for_tests(species["PIKACHU"].clone(), 20, Dv::default());
        let before = pokemon.clone();

        assert_eq!(
            apply_evolution_stone_item_effect(
                &mut pokemon,
                &item,
                &species,
                &moves,
                &learnsets,
                &evolutions,
                TimeOfDay::Day,
                true,
            )
            .expect_err("wrong stone has no effect"),
            BattleItemError::NoTargetChange {
                item_id: "FIRE_STONE".to_string(),
            }
        );
        assert_eq!(pokemon, before);
    }

    #[test]
    fn battle_stat_boost_items_raise_exact_stat_stage_from_pack_data() {
        let item = battle_boost_item("X_ITEM", Some("ATTACK"), Some(1));
        let mut pokemon = test_pokemon(35, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("X Attack applies");

        assert_eq!(outcome.item_id, "X_ATTACK");
        assert_eq!(
            outcome.battle_stat_stage_changes,
            vec![BattleItemStageChange {
                stat: "ATTACK".to_string(),
                stage_before: 0,
                stage_after: 1,
            }]
        );
        assert_eq!(pokemon.stat_boosts[&Stat::Attack], 1);
        assert!(outcome.consumed);
    }

    #[test]
    fn battle_stat_boost_items_support_exact_accuracy_effect() {
        let item = battle_boost_item("X_ACCURACY", Some("ACCURACY"), Some(1));
        let mut pokemon = test_pokemon(35, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("X Accuracy applies");

        assert_eq!(outcome.item_id, "X_ACCURACY");
        assert_eq!(outcome.battle_stat_stage_changes[0].stat, "ACCURACY");
        assert_eq!(pokemon.stat_boosts[&Stat::Accuracy], 1);
    }

    #[test]
    fn battle_stat_boost_items_reject_bad_pack_data_and_capped_stats_without_mutation() {
        let mut missing_stat = test_pokemon(35, 35);
        let missing_stat_before = missing_stat.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut missing_stat,
                &battle_boost_item("X_ITEM", None, Some(1)),
                true,
            )
            .expect_err("stat is required"),
            BattleItemError::MissingBattleStatBoostStat {
                item_id: "X_ATTACK".to_string(),
            }
        );
        assert_eq!(missing_stat, missing_stat_before);

        let mut bad_stat = test_pokemon(35, 35);
        let bad_stat_before = bad_stat.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut bad_stat,
                &battle_boost_item("X_ITEM", Some("attack"), Some(1)),
                true,
            )
            .expect_err("case changed stat is invalid"),
            BattleItemError::InvalidBattleStatBoostStat {
                item_id: "X_ATTACK".to_string(),
                stat: "attack".to_string(),
            }
        );
        assert_eq!(bad_stat, bad_stat_before);

        let mut missing_stages = test_pokemon(35, 35);
        let missing_stages_before = missing_stages.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut missing_stages,
                &battle_boost_item("X_ITEM", Some("ATTACK"), None),
                true,
            )
            .expect_err("stages are required"),
            BattleItemError::MissingBattleStatBoostStages {
                item_id: "X_ATTACK".to_string(),
            }
        );
        assert_eq!(missing_stages, missing_stages_before);

        let mut invalid_stages = test_pokemon(35, 35);
        let invalid_stages_before = invalid_stages.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut invalid_stages,
                &battle_boost_item("X_ITEM", Some("ATTACK"), Some(0)),
                true,
            )
            .expect_err("zero stages are invalid"),
            BattleItemError::InvalidBattleStatBoostStages {
                item_id: "X_ATTACK".to_string(),
                stages: 0,
            }
        );
        assert_eq!(invalid_stages, invalid_stages_before);

        let mut capped = test_pokemon(35, 35);
        capped.stat_boosts.insert(Stat::Attack, 6);
        let capped_before = capped.clone();
        assert_eq!(
            apply_active_battle_item_effect(
                &mut capped,
                &battle_boost_item("X_ITEM", Some("ATTACK"), Some(1)),
                true,
            )
            .expect_err("max stage has no effect"),
            BattleItemError::NoTargetChange {
                item_id: "X_ATTACK".to_string(),
            }
        );
        assert_eq!(capped, capped_before);
    }

    #[test]
    fn dire_hit_sets_focus_energy_from_exact_pack_data() {
        let item = dire_hit_item(Some(true));
        let mut pokemon = test_pokemon(35, 35);

        let outcome =
            apply_active_battle_item_effect(&mut pokemon, &item, true).expect("Dire Hit applies");

        assert_eq!(outcome.item_id, "DIRE_HIT");
        assert!(!outcome.focus_energy_before);
        assert!(outcome.focus_energy_after);
        assert!(pokemon.focus_energy);
        assert!(outcome.consumed);
    }

    #[test]
    fn dire_hit_rejects_missing_false_or_repeated_focus_without_mutation() {
        let mut missing = test_pokemon(35, 35);
        let missing_before = missing.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut missing, &dire_hit_item(None), true)
                .expect_err("payload-less focus item is missing battle item payload"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "DIRE_HIT".to_string(),
            }
        );
        assert_eq!(missing, missing_before);

        let mut invalid = test_pokemon(35, 35);
        let invalid_before = invalid.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut invalid, &dire_hit_item(Some(false)), true)
                .expect_err("false focus-energy metadata is invalid"),
            BattleItemError::InvalidBattleFocusEnergy {
                item_id: "DIRE_HIT".to_string(),
            }
        );
        assert_eq!(invalid, invalid_before);

        let mut already_focused = test_pokemon(35, 35);
        already_focused.focus_energy = true;
        let already_focused_before = already_focused.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut already_focused, &dire_hit_item(Some(true)), true)
                .expect_err("already-focused target is unchanged"),
            BattleItemError::NoTargetChange {
                item_id: "DIRE_HIT".to_string(),
            }
        );
        assert_eq!(already_focused, already_focused_before);
    }

    #[test]
    fn bitter_berry_clears_confusion_from_exact_pack_data() {
        let item = bitter_berry_item(Some(true));
        let mut pokemon = test_pokemon(35, 35);
        pokemon.confusion_turns = 3;

        let outcome = apply_active_battle_item_effect(&mut pokemon, &item, true)
            .expect("Bitter Berry applies");

        assert_eq!(outcome.item_id, "BITTER_BERRY");
        assert_eq!(outcome.confusion_turns_before, 3);
        assert_eq!(outcome.confusion_turns_after, 0);
        assert_eq!(pokemon.confusion_turns, 0);
        assert!(outcome.consumed);
    }

    #[test]
    fn bitter_berry_rejects_missing_false_or_unconfused_target_without_mutation() {
        let mut missing = test_pokemon(35, 35);
        missing.confusion_turns = 2;
        let missing_before = missing.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut missing, &bitter_berry_item(None), true)
                .expect_err("payload-less confusion item is missing battle item payload"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "BITTER_BERRY".to_string(),
            }
        );
        assert_eq!(missing, missing_before);

        let mut invalid = test_pokemon(35, 35);
        invalid.confusion_turns = 2;
        let invalid_before = invalid.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut invalid, &bitter_berry_item(Some(false)), true)
                .expect_err("false confusion metadata is invalid"),
            BattleItemError::InvalidConfusionHeal {
                item_id: "BITTER_BERRY".to_string(),
            }
        );
        assert_eq!(invalid, invalid_before);

        let mut unconfused = test_pokemon(35, 35);
        let unconfused_before = unconfused.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut unconfused, &bitter_berry_item(Some(true)), true)
                .expect_err("unconfused target is unchanged"),
            BattleItemError::NoTargetChange {
                item_id: "BITTER_BERRY".to_string(),
            }
        );
        assert_eq!(unconfused, unconfused_before);
    }

    #[test]
    fn sacred_ash_revives_all_fainted_party_members_from_pack_percent() {
        let item = sacred_ash_item(Some(100));
        let mut party = Party::default();
        let mut fainted_a = test_pokemon(0, 35);
        fainted_a.species.id = "CHIKORITA".to_string();
        let mut healthy = test_pokemon(12, 40);
        healthy.species.id = "BAYLEEF".to_string();
        let mut fainted_b = test_pokemon(0, 50);
        fainted_b.species.id = "MEGANIUM".to_string();
        party.pokemon[0] = Some(fainted_a);
        party.pokemon[1] = Some(healthy);
        party.pokemon[2] = Some(fainted_b);

        let outcome =
            apply_party_wide_item_effect(&mut party, &item, true).expect("Sacred Ash applies");

        assert_eq!(outcome.item_id, "MOD_ASH");
        assert_eq!(
            outcome.revive_changes,
            vec![
                PartyItemReviveChange {
                    party_index: 0,
                    pokemon_id: "CHIKORITA".to_string(),
                    hp_before: 0,
                    hp_after: 35,
                },
                PartyItemReviveChange {
                    party_index: 2,
                    pokemon_id: "MEGANIUM".to_string(),
                    hp_before: 0,
                    hp_after: 50,
                },
            ]
        );
        assert_eq!(party.pokemon[0].as_ref().expect("slot 0").hp, 35);
        assert_eq!(party.pokemon[1].as_ref().expect("slot 1").hp, 12);
        assert_eq!(party.pokemon[2].as_ref().expect("slot 2").hp, 50);
        assert!(outcome.consumed);
    }

    #[test]
    fn sacred_ash_rejects_missing_invalid_or_no_targets_without_mutation() {
        let mut missing = Party::default();
        missing.pokemon[0] = Some(test_pokemon(0, 35));
        let missing_before = missing.clone();
        assert_eq!(
            apply_party_wide_item_effect(&mut missing, &sacred_ash_item(None), true)
                .expect_err("payload-less item is not a party revive item"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "MOD_ASH".to_string(),
            }
        );
        assert_eq!(missing, missing_before);

        let mut invalid = Party::default();
        invalid.pokemon[0] = Some(test_pokemon(0, 35));
        let invalid_before = invalid.clone();
        assert_eq!(
            apply_party_wide_item_effect(&mut invalid, &sacred_ash_item(Some(0)), true)
                .expect_err("zero percent is invalid"),
            BattleItemError::InvalidPartyReviveHpPercent {
                item_id: "MOD_ASH".to_string(),
                percent: 0,
            }
        );
        assert_eq!(invalid, invalid_before);

        let mut healthy = Party::default();
        healthy.pokemon[0] = Some(test_pokemon(35, 35));
        let healthy_before = healthy.clone();
        assert_eq!(
            apply_party_wide_item_effect(&mut healthy, &sacred_ash_item(Some(100)), true)
                .expect_err("no fainted target has no effect"),
            BattleItemError::NoTargetChange {
                item_id: "MOD_ASH".to_string(),
            }
        );
        assert_eq!(healthy, healthy_before);
    }

    #[test]
    fn battle_items_reject_unsupported_payload_less_effect_without_mutation() {
        let item = test_item("MOD_UNDECLARED", 0);
        let mut pokemon = test_pokemon(17, 35);
        let before = pokemon.clone();

        let error = apply_active_battle_item_effect(&mut pokemon, &item, true)
            .expect_err("payload-less effect is not accepted");

        assert_eq!(
            error,
            BattleItemError::MissingBattleItemPayload {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(pokemon, before);
    }

    #[test]
    fn battle_items_reject_invalid_or_no_effect_healing_without_mutation() {
        let mut fainted = test_pokemon(0, 35);
        let fainted_before = fainted.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut fainted, &test_item("RESTORE_HP", 20), true)
                .expect_err("fainted Pokemon cannot be healed by HP items"),
            BattleItemError::TargetFainted {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(fainted, fainted_before);

        let mut full = test_pokemon(35, 35);
        let full_before = full.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut full, &test_item("RESTORE_HP", 20), true)
                .expect_err("full HP has no item effect"),
            BattleItemError::NoTargetChange {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(full, full_before);

        let mut damaged = test_pokemon(17, 35);
        let damaged_before = damaged.clone();
        assert_eq!(
            apply_active_battle_item_effect(&mut damaged, &test_item("RESTORE_HP", 0), true)
                .expect_err("zero heal item is missing HP payload"),
            BattleItemError::MissingBattleItemPayload {
                item_id: "POTION".to_string(),
            }
        );
        assert_eq!(damaged, damaged_before);
    }
}
