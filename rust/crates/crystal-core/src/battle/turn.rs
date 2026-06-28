use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::battle::damage::{
    DamageCalculationError, DamageContext, DamageResult, TypeCategories, TypeEffectivenessTable,
    Weather, WeatherModifiers, calculate_damage,
};
use crate::battle::start::{ActiveBattleEnemyError, deactivate_battle, update_active_battle_enemy};
use crate::battle::stats::{BattleStatMultiplierTables, accuracy_stage_multiplier, apply_stage};
use crate::models::{Item, Move, Pokemon, Stat};
use crate::random::Random;
use crate::state::GameState;
use crate::systems::battle_escape::{
    BattleEscapeAttempt, BattleEscapeError, BattleEscapeRules, attempt_wild_battle_escape,
};
use crate::systems::battle_items::{
    BattleItemError, BattleItemOutcome, apply_active_battle_item_effect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleSide {
    Player,
    Enemy,
}

impl BattleSide {
    pub const fn other(self) -> Self {
        match self {
            Self::Player => Self::Enemy,
            Self::Enemy => Self::Player,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleCombatState {
    pub player: Pokemon,
    pub enemy: Pokemon,
    pub weather: Weather,
    pub turn: u32,
    pub rng_seed_after: u32,
}

impl BattleCombatState {
    pub fn new(player: Pokemon, enemy: Pokemon, rng_seed: u32) -> Self {
        Self {
            player,
            enemy,
            weather: Weather::None,
            turn: 0,
            rng_seed_after: rng_seed,
        }
    }

    pub fn pokemon(&self, side: BattleSide) -> &Pokemon {
        match side {
            BattleSide::Player => &self.player,
            BattleSide::Enemy => &self.enemy,
        }
    }

    pub fn pokemon_mut(&mut self, side: BattleSide) -> &mut Pokemon {
        match side {
            BattleSide::Player => &mut self.player,
            BattleSide::Enemy => &mut self.enemy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleAction {
    Move { slot: usize },
    Switch { party_index: usize },
    Item { item_id: String },
    Run,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleTurnInput {
    pub player: BattleAction,
    pub enemy: BattleAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleTurnOutcome {
    pub state: BattleCombatState,
    pub order: Vec<BattleSide>,
    pub events: Vec<BattleEvent>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovePriorityTable {
    pub base_priority: i8,
    pub effect_priorities: BTreeMap<String, i8>,
    pub move_priorities: Vec<MovePriorityOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovePriorityOverride {
    pub r#move: String,
    pub priority: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MovePriorityTableIssue {
    InvalidBasePriority {
        priority: i8,
    },
    MissingEffectPriorities,
    InvalidMoveEffectPriorityId {
        move_effect: String,
    },
    InvalidMoveEffectPriority {
        move_effect: String,
        priority: i8,
    },
    MissingMoveEffectPriority {
        move_name: String,
        move_effect: String,
    },
    InvalidMovePriorityId {
        move_name: String,
    },
    UnknownMovePriority {
        move_name: String,
    },
    InvalidMovePriority {
        move_name: String,
        priority: i8,
    },
}

pub fn move_priority_table_issues(
    priorities: &MovePriorityTable,
    moves: &BTreeMap<String, Move>,
    required: bool,
) -> Vec<MovePriorityTableIssue> {
    let mut issues = Vec::new();
    if !required {
        return issues;
    }

    if priorities.base_priority < 0 {
        issues.push(MovePriorityTableIssue::InvalidBasePriority {
            priority: priorities.base_priority,
        });
    }
    if priorities.effect_priorities.is_empty() {
        issues.push(MovePriorityTableIssue::MissingEffectPriorities);
    }

    for (move_effect, priority) in &priorities.effect_priorities {
        if !is_exact_battle_turn_token(move_effect) {
            issues.push(MovePriorityTableIssue::InvalidMoveEffectPriorityId {
                move_effect: move_effect.clone(),
            });
        }
        if *priority < 0 {
            issues.push(MovePriorityTableIssue::InvalidMoveEffectPriority {
                move_effect: move_effect.clone(),
                priority: *priority,
            });
        }
    }

    for move_data in moves.values() {
        if !priorities.effect_priorities.contains_key(&move_data.effect) {
            issues.push(MovePriorityTableIssue::MissingMoveEffectPriority {
                move_name: move_data.name.clone(),
                move_effect: move_data.effect.clone(),
            });
        }
    }

    for entry in &priorities.move_priorities {
        if !is_exact_battle_turn_token(&entry.r#move) {
            issues.push(MovePriorityTableIssue::InvalidMovePriorityId {
                move_name: entry.r#move.clone(),
            });
        } else if !moves.contains_key(&entry.r#move) {
            issues.push(MovePriorityTableIssue::UnknownMovePriority {
                move_name: entry.r#move.clone(),
            });
        }
        if entry.priority < 0 {
            issues.push(MovePriorityTableIssue::InvalidMovePriority {
                move_name: entry.r#move.clone(),
                priority: entry.priority,
            });
        }
    }

    issues
}

fn is_exact_battle_turn_token(value: &str) -> bool {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleTurnError {
    MissingMoveSlot {
        side: BattleSide,
        slot: usize,
    },
    MissingMoveData {
        side: BattleSide,
        move_name: String,
    },
    InvalidMoveName {
        side: BattleSide,
        move_name: String,
    },
    InvalidItem {
        side: BattleSide,
        item_id: String,
    },
    UnknownItem {
        side: BattleSide,
        item_id: String,
    },
    UnusableItem {
        side: BattleSide,
        item_id: String,
    },
    BattleItem {
        side: BattleSide,
        item_id: String,
        error: BattleItemError,
    },
    UnsupportedRunAction {
        side: BattleSide,
    },
    MissingStat {
        side: BattleSide,
        stat: Stat,
    },
    MissingStatStage {
        side: BattleSide,
        stat: Stat,
    },
    MissingStatMultiplier {
        side: BattleSide,
        stage: i8,
    },
    MissingAccuracyMultiplier {
        stage: i8,
    },
    MissingMovePriorityTable,
    MissingMoveEffectPriority {
        move_effect: String,
    },
    BattleEscape(BattleEscapeError),
    DamageCalculation(DamageCalculationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleEvent {
    MoveSelected {
        side: BattleSide,
        slot: usize,
        move_name: String,
    },
    NoPp {
        side: BattleSide,
        move_name: String,
    },
    MoveUsed {
        side: BattleSide,
        move_name: String,
    },
    Missed {
        side: BattleSide,
        move_name: String,
        accuracy: u8,
        roll: u8,
    },
    NoEffect {
        side: BattleSide,
        move_name: String,
    },
    Damage {
        side: BattleSide,
        move_name: String,
        damage: u16,
        defender_hp_before: u16,
        defender_hp_after: u16,
        roll: u8,
        result: DamageResult,
    },
    Fainted {
        side: BattleSide,
    },
    Switched {
        side: BattleSide,
        party_index: usize,
    },
    ItemUsed {
        side: BattleSide,
        item_id: String,
    },
    BattleItemEffect {
        side: BattleSide,
        outcome: BattleItemOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum BattleTurnCommitError {
    #[error("battle turn active party index {index} is outside the party")]
    PartyIndexOutOfRange { index: usize },
    #[error("battle turn active party index {index} has no Pokemon")]
    EmptyPartySlot { index: usize },
    #[error("battle turn active enemy update failed: {0:?}")]
    ActiveEnemy(#[from] ActiveBattleEnemyError),
}

pub fn commit_battle_turn_outcome(
    state: &mut GameState,
    active_party_index: usize,
    outcome: &BattleTurnOutcome,
) -> Result<(), BattleTurnCommitError> {
    let slot = state
        .storage
        .party
        .pokemon
        .get_mut(active_party_index)
        .ok_or(BattleTurnCommitError::PartyIndexOutOfRange {
            index: active_party_index,
        })?;
    if slot.is_none() {
        return Err(BattleTurnCommitError::EmptyPartySlot {
            index: active_party_index,
        });
    }
    *slot = Some(outcome.state.player.clone());
    state.rng_seed = outcome.state.rng_seed_after;
    state.sync_party_from_storage();
    update_active_battle_enemy(state, outcome.state.enemy.clone())?;
    Ok(())
}

pub fn commit_wild_battle_escape_attempt(state: &mut GameState, outcome: &BattleEscapeAttempt) {
    state.rng_seed = outcome.rng_seed_after;
    state.battle_escape_attempts = outcome.attempts_after;
    if outcome.escaped {
        deactivate_battle(state);
    }
}

pub fn resolve_battle_turn(
    state: BattleCombatState,
    input: BattleTurnInput,
    moves: &BTreeMap<String, Move>,
    move_priorities: &MovePriorityTable,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    resolve_battle_turn_with_items(
        state,
        input,
        moves,
        &BTreeMap::new(),
        move_priorities,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
    )
}

pub fn resolve_battle_turn_with_items(
    mut state: BattleCombatState,
    input: BattleTurnInput,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    let mut events = Vec::new();
    let order = determine_turn_order(
        &state,
        &input,
        moves,
        items,
        move_priorities,
        stat_multipliers,
        rng,
    )?;

    for side in order.iter().copied() {
        if state.pokemon(side).hp == 0 || state.pokemon(side.other()).hp == 0 {
            continue;
        }
        let action = match side {
            BattleSide::Player => &input.player,
            BattleSide::Enemy => &input.enemy,
        };
        execute_action(
            &mut state,
            side,
            action,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            &mut events,
        )?;
    }

    state.turn = state.turn.saturating_add(1);
    state.rng_seed_after = rng.seed();
    Ok(BattleTurnOutcome {
        state,
        order,
        events,
    })
}

pub fn resolve_wild_battle_run(
    state: &BattleCombatState,
    rules: &BattleEscapeRules,
    attempts_before: u8,
    stat_multipliers: &BattleStatMultiplierTables,
    rng: &mut Random,
) -> Result<BattleEscapeAttempt, BattleTurnError> {
    attempt_wild_battle_escape(
        &state.player,
        &state.enemy,
        stat_multipliers,
        rules,
        attempts_before,
        rng,
    )
    .map_err(BattleTurnError::BattleEscape)
}

pub fn determine_turn_order(
    state: &BattleCombatState,
    input: &BattleTurnInput,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
    stat_multipliers: &BattleStatMultiplierTables,
    rng: &mut Random,
) -> Result<Vec<BattleSide>, BattleTurnError> {
    let player_priority = action_priority(
        BattleSide::Player,
        &state.player,
        &input.player,
        moves,
        items,
        move_priorities,
    )?;
    let enemy_priority = action_priority(
        BattleSide::Enemy,
        &state.enemy,
        &input.enemy,
        moves,
        items,
        move_priorities,
    )?;
    if player_priority != enemy_priority {
        return Ok(if player_priority > enemy_priority {
            vec![BattleSide::Player, BattleSide::Enemy]
        } else {
            vec![BattleSide::Enemy, BattleSide::Player]
        });
    }

    let player_speed = battle_speed(BattleSide::Player, &state.player, stat_multipliers)?;
    let enemy_speed = battle_speed(BattleSide::Enemy, &state.enemy, stat_multipliers)?;
    if player_speed != enemy_speed {
        return Ok(if player_speed > enemy_speed {
            vec![BattleSide::Player, BattleSide::Enemy]
        } else {
            vec![BattleSide::Enemy, BattleSide::Player]
        });
    }

    Ok(if rng.randrange(2) == 0 {
        vec![BattleSide::Player, BattleSide::Enemy]
    } else {
        vec![BattleSide::Enemy, BattleSide::Player]
    })
}

fn execute_action(
    state: &mut BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    match action {
        BattleAction::Move { slot } => execute_move_slot(
            state,
            side,
            *slot,
            moves,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            events,
        ),
        BattleAction::Switch { party_index } => {
            events.push(BattleEvent::Switched {
                side,
                party_index: *party_index,
            });
            Ok(())
        }
        BattleAction::Item { item_id } => execute_item(state, side, item_id, items, events),
        BattleAction::Run => Err(BattleTurnError::UnsupportedRunAction { side }),
    }
}

fn execute_item(
    state: &mut BattleCombatState,
    side: BattleSide,
    item_id: &str,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    validate_battle_turn_item_id(side, item_id)?;
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if !item.battle_usable {
        return Err(BattleTurnError::UnusableItem {
            side,
            item_id: item_id.to_string(),
        });
    }
    events.push(BattleEvent::ItemUsed {
        side,
        item_id: item_id.to_string(),
    });
    let outcome =
        apply_active_battle_item_effect(state.pokemon_mut(side), item, false).map_err(|error| {
            BattleTurnError::BattleItem {
                side,
                item_id: item_id.to_string(),
                error,
            }
        })?;
    events.push(BattleEvent::BattleItemEffect { side, outcome });
    Ok(())
}

fn execute_move_slot(
    state: &mut BattleCombatState,
    side: BattleSide,
    slot: usize,
    moves: &BTreeMap<String, Move>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let Some(move_name) = state
        .pokemon(side)
        .moves
        .get(slot)
        .map(|learned| learned.name.clone())
    else {
        return Err(BattleTurnError::MissingMoveSlot { side, slot });
    };
    validate_battle_turn_move_name(side, &move_name)?;
    let Some(move_data) = moves.get(&move_name) else {
        return Err(BattleTurnError::MissingMoveData { side, move_name });
    };
    events.push(BattleEvent::MoveSelected {
        side,
        slot,
        move_name: move_name.clone(),
    });

    let learned_move = match side {
        BattleSide::Player => state.player.moves.get_mut(slot),
        BattleSide::Enemy => state.enemy.moves.get_mut(slot),
    };
    let Some(learned_move) = learned_move else {
        return Err(BattleTurnError::MissingMoveSlot { side, slot });
    };
    if learned_move.current_pp == 0 {
        events.push(BattleEvent::NoPp {
            side,
            move_name: move_name.clone(),
        });
        return Ok(());
    }
    learned_move.current_pp = learned_move.current_pp.saturating_sub(1);
    events.push(BattleEvent::MoveUsed {
        side,
        move_name: move_name.clone(),
    });

    let (attacker, defender) = match side {
        BattleSide::Player => (&state.player, &state.enemy),
        BattleSide::Enemy => (&state.enemy, &state.player),
    };
    let accuracy = accuracy_byte(move_data, side, attacker, defender, stat_multipliers)?;
    if accuracy < u8::MAX {
        let roll = rng.randrange(256) as u8;
        if roll >= accuracy {
            events.push(BattleEvent::Missed {
                side,
                move_name,
                accuracy,
                roll,
            });
            return Ok(());
        }
    }

    let damage_roll = 217 + rng.randrange(39) as u8;
    let result = calculate_damage(
        attacker,
        defender,
        move_data,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        DamageContext {
            is_critical: false,
            is_confusion_damage: false,
            weather: state.weather,
            random_roll: damage_roll,
        },
    )
    .map_err(BattleTurnError::DamageCalculation)?;
    if result.type_multiplier.numerator == 0 {
        events.push(BattleEvent::NoEffect { side, move_name });
        return Ok(());
    }

    let defender_hp_before = state.pokemon(side.other()).hp;
    let damage = result.damage.min(defender_hp_before);
    let defender = match side {
        BattleSide::Player => &mut state.enemy,
        BattleSide::Enemy => &mut state.player,
    };
    defender.hp = defender.hp.saturating_sub(damage);
    events.push(BattleEvent::Damage {
        side,
        move_name,
        damage,
        defender_hp_before,
        defender_hp_after: defender.hp,
        roll: damage_roll,
        result,
    });
    if defender.hp == 0 {
        events.push(BattleEvent::Fainted { side: side.other() });
    }
    Ok(())
}

fn action_priority(
    side: BattleSide,
    pokemon: &Pokemon,
    action: &BattleAction,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
) -> Result<i8, BattleTurnError> {
    match action {
        BattleAction::Move { slot } => {
            let learned = pokemon
                .moves
                .get(*slot)
                .ok_or(BattleTurnError::MissingMoveSlot { side, slot: *slot })?;
            validate_battle_turn_move_name(side, &learned.name)?;
            let move_data =
                moves
                    .get(&learned.name)
                    .ok_or_else(|| BattleTurnError::MissingMoveData {
                        side,
                        move_name: learned.name.clone(),
                    })?;
            Ok(move_priority(move_data, move_priorities)?)
        }
        BattleAction::Switch { .. } => Ok(6),
        BattleAction::Item { item_id } => {
            validate_battle_turn_item_id(side, item_id)?;
            let item = items
                .get(item_id)
                .ok_or_else(|| BattleTurnError::UnknownItem {
                    side,
                    item_id: item_id.clone(),
                })?;
            if !item.battle_usable {
                return Err(BattleTurnError::UnusableItem {
                    side,
                    item_id: item_id.clone(),
                });
            }
            Ok(6)
        }
        BattleAction::Run => Err(BattleTurnError::UnsupportedRunAction { side }),
    }
}

fn validate_battle_turn_move_name(
    side: BattleSide,
    move_name: &str,
) -> Result<(), BattleTurnError> {
    if !is_exact_battle_turn_token(move_name) {
        return Err(BattleTurnError::InvalidMoveName {
            side,
            move_name: move_name.to_string(),
        });
    }
    Ok(())
}

fn validate_battle_turn_item_id(side: BattleSide, item_id: &str) -> Result<(), BattleTurnError> {
    if !is_exact_battle_turn_token(item_id) {
        return Err(BattleTurnError::InvalidItem {
            side,
            item_id: item_id.to_string(),
        });
    }
    Ok(())
}

pub fn move_priority(
    move_data: &Move,
    priorities: &MovePriorityTable,
) -> Result<i8, BattleTurnError> {
    if priorities.effect_priorities.is_empty() {
        return Err(BattleTurnError::MissingMovePriorityTable);
    }
    if let Some(override_priority) = priorities
        .move_priorities
        .iter()
        .find(|entry| entry.r#move == move_data.name)
    {
        return Ok(override_priority.priority);
    }
    Ok(priorities
        .effect_priorities
        .get(&move_data.effect)
        .copied()
        .ok_or_else(|| BattleTurnError::MissingMoveEffectPriority {
            move_effect: move_data.effect.clone(),
        })?)
}

fn battle_speed(
    side: BattleSide,
    pokemon: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
) -> Result<u16, BattleTurnError> {
    let base = pokemon
        .calculate_stat(Stat::Speed)
        .ok_or(BattleTurnError::MissingStat {
            side,
            stat: Stat::Speed,
        })?;
    let stage =
        *pokemon
            .stat_boosts
            .get(&Stat::Speed)
            .ok_or(BattleTurnError::MissingStatStage {
                side,
                stat: Stat::Speed,
            })?;
    apply_stage(stat_multipliers, base, stage)
        .ok_or(BattleTurnError::MissingStatMultiplier { side, stage })
}

fn accuracy_byte(
    move_data: &Move,
    attacker_side: BattleSide,
    attacker: &Pokemon,
    defender: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
) -> Result<u8, BattleTurnError> {
    if move_data.accuracy == 0 {
        return Ok(u8::MAX);
    }
    let attacker_accuracy =
        *attacker
            .stat_boosts
            .get(&Stat::Accuracy)
            .ok_or(BattleTurnError::MissingStatStage {
                side: attacker_side,
                stat: Stat::Accuracy,
            })?;
    let defender_side = attacker_side.other();
    let defender_evasion =
        *defender
            .stat_boosts
            .get(&Stat::Evasion)
            .ok_or(BattleTurnError::MissingStatStage {
                side: defender_side,
                stat: Stat::Evasion,
            })?;
    let stage = (attacker_accuracy - defender_evasion).clamp(-6, 6);
    let multiplier = accuracy_stage_multiplier(stat_multipliers, stage)
        .ok_or(BattleTurnError::MissingAccuracyMultiplier { stage })?;
    let base = ((move_data.accuracy as i32 * 255) / 100).clamp(1, 255);
    Ok(multiplier.multiply_floor(base).clamp(1, 255) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::stats::BattleStatMultiplier;
    use crate::models::{
        BaseStats, Dv, Item, LearnedMove, PokemonSpecies, PokemonType, item_pocket, pokemon_type,
    };

    #[test]
    fn battle_turn_serialized_variants_reject_unknown_fallback_fields() {
        let side_error = serde_json::from_value::<BattleSide>(serde_json::json!({
            "player": {
                "legacy_side": "PLAYER"
            }
        }))
        .expect_err("battle sides must not accept legacy object payloads");
        assert!(
            side_error.to_string().contains("invalid type")
                || side_error.to_string().contains("unknown variant"),
            "{side_error}"
        );

        let action_error = serde_json::from_value::<BattleAction>(serde_json::json!({
            "move": {
                "slot": 0,
                "fallback_slot": 1
            }
        }))
        .expect_err("battle actions must not accept fallback move slots");
        assert!(
            action_error
                .to_string()
                .contains("unknown field `fallback_slot`"),
            "{action_error}"
        );

        let turn_error = serde_json::from_value::<BattleTurnError>(serde_json::json!({
            "UnknownItem": {
                "side": "player",
                "item_id": "POTION",
                "fallback_item_id": "BERRY"
            }
        }))
        .expect_err("battle turn errors must not accept fallback item ids");
        assert!(
            turn_error
                .to_string()
                .contains("unknown field `fallback_item_id`"),
            "{turn_error}"
        );

        let event_error = serde_json::from_value::<BattleEvent>(serde_json::json!({
            "MoveUsed": {
                "side": "enemy",
                "move_name": "TACKLE",
                "legacy_move_name": "Tackle"
            }
        }))
        .expect_err("battle events must not accept legacy move names");
        assert!(
            event_error
                .to_string()
                .contains("unknown field `legacy_move_name`"),
            "{event_error}"
        );

        let priority_error = serde_json::from_value::<MovePriorityTableIssue>(serde_json::json!({
            "UnknownMovePriority": {
                "move_name": "EXTREME_SPEED",
                "default_priority": 0
            }
        }))
        .expect_err("move priority issues must not accept default priorities");
        assert!(
            priority_error
                .to_string()
                .contains("unknown field `default_priority`"),
            "{priority_error}"
        );
    }

    fn stat_multipliers() -> BattleStatMultiplierTables {
        BattleStatMultiplierTables {
            stat: vec![
                BattleStatMultiplier {
                    numerator: 25,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 28,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 33,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 40,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 50,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 66,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 15,
                    denominator: 10,
                },
                BattleStatMultiplier {
                    numerator: 2,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 25,
                    denominator: 10,
                },
                BattleStatMultiplier {
                    numerator: 3,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 35,
                    denominator: 10,
                },
                BattleStatMultiplier {
                    numerator: 4,
                    denominator: 1,
                },
            ],
            accuracy: vec![
                BattleStatMultiplier {
                    numerator: 33,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 36,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 43,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 50,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 60,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 75,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 133,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 166,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 2,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 233,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 133,
                    denominator: 50,
                },
                BattleStatMultiplier {
                    numerator: 3,
                    denominator: 1,
                },
            ],
        }
    }

    fn weather_modifiers() -> WeatherModifiers {
        WeatherModifiers {
            type_modifiers: [
                (
                    "WEATHER_RAIN".to_string(),
                    [
                        (
                            "WATER".to_string(),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 3,
                                denominator: 2,
                            },
                        ),
                        (
                            "FIRE".to_string(),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 1,
                                denominator: 2,
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
                (
                    "WEATHER_SUN".to_string(),
                    [
                        (
                            "FIRE".to_string(),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 3,
                                denominator: 2,
                            },
                        ),
                        (
                            "WATER".to_string(),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 1,
                                denominator: 2,
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            ]
            .into_iter()
            .collect(),
            move_effect_modifiers: BTreeMap::new(),
        }
    }

    fn type_effectiveness_table() -> TypeEffectivenessTable {
        TypeEffectivenessTable {
            matchups: vec![
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("NORMAL"),
                    multiplier: crate::battle::damage::TypeMultiplier::one(),
                },
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("ELECTRIC"),
                    multiplier: crate::battle::damage::TypeMultiplier::one(),
                },
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("FIGHTING"),
                    multiplier: crate::battle::damage::TypeMultiplier::one(),
                },
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("FLYING"),
                    multiplier: crate::battle::damage::TypeMultiplier::one(),
                },
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("ELECTRIC"),
                    defender: pokemon_type("ROCK"),
                    multiplier: crate::battle::damage::TypeMultiplier::one(),
                },
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("FIGHTING"),
                    defender: pokemon_type("NORMAL"),
                    multiplier: crate::battle::damage::TypeMultiplier {
                        numerator: 2,
                        denominator: 1,
                    },
                },
                crate::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("ELECTRIC"),
                    defender: pokemon_type("GROUND"),
                    multiplier: crate::battle::damage::TypeMultiplier::zero(),
                },
            ],
            foresight_matchups: vec![crate::battle::damage::TypeEffectivenessEntry {
                attacker: pokemon_type("NORMAL"),
                defender: pokemon_type("GHOST"),
                multiplier: crate::battle::damage::TypeMultiplier::zero(),
            }],
        }
    }

    fn type_categories() -> TypeCategories {
        TypeCategories {
            physical: vec![
                "NORMAL".to_string(),
                "FIGHTING".to_string(),
                "FLYING".to_string(),
                "POISON".to_string(),
                "GROUND".to_string(),
                "ROCK".to_string(),
                "BUG".to_string(),
                "GHOST".to_string(),
                "STEEL".to_string(),
            ],
            special: vec![
                "FIRE".to_string(),
                "WATER".to_string(),
                "GRASS".to_string(),
                "ELECTRIC".to_string(),
                "PSYCHIC_TYPE".to_string(),
                "ICE".to_string(),
                "DRAGON".to_string(),
                "DARK".to_string(),
            ],
        }
    }

    fn move_priorities() -> MovePriorityTable {
        MovePriorityTable {
            base_priority: 1,
            effect_priorities: [
                ("PROTECT".to_string(), 3),
                ("ENDURE".to_string(), 3),
                ("PRIORITY_HIT".to_string(), 2),
                ("FORCE_SWITCH".to_string(), 0),
                ("COUNTER".to_string(), 0),
                ("MIRROR_COAT".to_string(), 0),
                ("NORMAL_HIT".to_string(), 1),
            ]
            .into_iter()
            .collect(),
            move_priorities: vec![MovePriorityOverride {
                r#move: "VITAL_THROW".to_string(),
                priority: 0,
            }],
        }
    }

    fn species(id: &str, speed: u16, pokemon_type: PokemonType) -> PokemonSpecies {
        let mut species =
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, speed, 65, 65));
        species.type1 = pokemon_type.clone();
        species.type2 = pokemon_type;
        species
    }

    fn pokemon(id: &str, speed: u16, pokemon_type: PokemonType, move_name: &str) -> Pokemon {
        let mut pokemon = Pokemon::new_for_tests(
            species(id, speed, pokemon_type),
            20,
            Dv::from_non_hp(10, 10, 10, 10),
        );
        pokemon.moves = vec![LearnedMove {
            name: move_name.to_string(),
            current_pp: 5,
            pp_ups: 0,
        }];
        pokemon
    }

    #[test]
    fn commit_battle_turn_outcome_updates_party_enemy_and_rng_together() {
        let mut state = GameState::default();
        let mut player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        let mut enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        state.storage.party.pokemon[0] = Some(player.clone());
        state.battle_active_enemy_party_index = Some(0);
        state.battle = crate::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };

        player.hp = 31;
        enemy.hp = 4;
        let outcome = BattleTurnOutcome {
            state: BattleCombatState {
                player: player.clone(),
                enemy: enemy.clone(),
                weather: Weather::None,
                turn: 1,
                rng_seed_after: 0x1234_5678,
            },
            order: Vec::new(),
            events: Vec::new(),
        };

        commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit battle turn");

        assert_eq!(state.rng_seed, 0x1234_5678);
        assert_eq!(state.storage.party.pokemon[0].as_ref().unwrap().hp, 31);
        assert_eq!(
            state.party.pokemon[0].as_ref().unwrap().species,
            "CHIKORITA"
        );
        let crate::state::BattleMemory::Wild {
            enemy_pokemon,
            enemy_party,
            ..
        } = &state.battle
        else {
            panic!("expected wild battle");
        };
        assert_eq!(enemy_pokemon.hp, 4);
        assert_eq!(enemy_party[0].hp, 4);
    }

    #[test]
    fn commit_wild_battle_escape_attempt_updates_attempts_rng_and_deactivates_on_success() {
        let mut state = GameState::default();
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        state.battle = crate::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy],
        };
        state.battle_active_party_index = Some(0);
        state.battle_active_enemy_party_index = Some(0);
        state.battle_escape_attempts = 2;

        commit_wild_battle_escape_attempt(
            &mut state,
            &BattleEscapeAttempt {
                escaped: false,
                chance: 64,
                roll: Some(90),
                attempts_before: 2,
                attempts_after: 3,
                rng_seed_after: 0x1111_2222,
            },
        );
        assert_eq!(state.rng_seed, 0x1111_2222);
        assert_eq!(state.battle_escape_attempts, 3);
        assert!(matches!(
            state.battle,
            crate::state::BattleMemory::Wild { .. }
        ));

        commit_wild_battle_escape_attempt(
            &mut state,
            &BattleEscapeAttempt {
                escaped: true,
                chance: 64,
                roll: Some(12),
                attempts_before: 3,
                attempts_after: 3,
                rng_seed_after: 0x3333_4444,
            },
        );
        assert_eq!(state.rng_seed, 0x3333_4444);
        assert_eq!(state.battle_escape_attempts, 0);
        assert_eq!(state.battle, crate::state::BattleMemory::Inactive);
        assert_eq!(state.battle_active_party_index, None);
        assert_eq!(state.battle_active_enemy_party_index, None);
    }

    fn move_data(name: &str, move_type: PokemonType, power: u16, accuracy: u8) -> Move {
        move_data_with_effect(name, move_type, power, accuracy, "NORMAL_HIT")
    }

    fn battle_item(id: &str, heal_amount: i16, battle_usable: bool) -> Item {
        Item {
            name: id.replace('_', " "),
            description: String::new(),
            effect: String::new(),
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
            parameter: heal_amount,
            property: String::new(),
            pocket: item_pocket("ITEM"),
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable,
            script_name: id.to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn move_data_with_effect(
        name: &str,
        move_type: PokemonType,
        power: u16,
        accuracy: u8,
        effect: &str,
    ) -> Move {
        Move {
            name: name.to_string(),
            move_type,
            power,
            accuracy,
            pp: 35,
            effect: effect.to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    #[test]
    fn accuracy_byte_uses_asm_accuracy_and_evasion_stages_for_hundred_accuracy_moves() {
        let mut attacker = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let mut defender = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        attacker.stat_boosts.insert(Stat::Accuracy, -1);
        defender.stat_boosts.insert(Stat::Evasion, 1);

        let accuracy = accuracy_byte(
            &move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            BattleSide::Player,
            &attacker,
            &defender,
            &stat_multipliers(),
        )
        .expect("accuracy calculates");

        assert_eq!(accuracy, 153);
    }

    #[test]
    fn accuracy_byte_rejects_missing_asm_multiplier_without_identity_fallback() {
        let attacker = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let defender = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let mut tables = stat_multipliers();
        tables.accuracy.clear();

        let error = accuracy_byte(
            &move_data("TACKLE", pokemon_type("NORMAL"), 35, 95),
            BattleSide::Player,
            &attacker,
            &defender,
            &tables,
        )
        .expect_err("missing accuracy multiplier must not use identity fallback");

        assert_eq!(
            error,
            BattleTurnError::MissingAccuracyMultiplier { stage: 0 }
        );
    }

    #[test]
    fn faster_move_user_attacks_first_and_damage_is_deterministic() {
        let player = pokemon("PIKACHU", 90, pokemon_type("ELECTRIC"), "THUNDERSHOCK");
        let enemy = pokemon("GEODUDE", 20, pokemon_type("ROCK"), "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "THUNDERSHOCK".to_string(),
            move_data("THUNDERSHOCK", pokemon_type("ELECTRIC"), 40, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut rng = Random::new(7);
        let outcome = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("battle turn resolves");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert_eq!(outcome.state.turn, 1);
        assert_eq!(outcome.state.rng_seed_after, rng.seed());
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.enemy.moves[0].current_pp, 4);
        assert!(outcome.state.enemy.hp < outcome.state.enemy.max_hp);
        assert!(outcome.state.player.hp < outcome.state.player.max_hp);
        assert!(matches!(
            outcome.events[0],
            BattleEvent::MoveSelected {
                side: BattleSide::Player,
                ..
            }
        ));
    }

    #[test]
    fn move_priority_overrides_speed() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "QUICK_ATTACK");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "QUICK_ATTACK".to_string(),
            move_data_with_effect(
                "QUICK_ATTACK",
                pokemon_type("NORMAL"),
                40,
                100,
                "PRIORITY_HIT",
            ),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut rng = Random::new(1);
        let order = determine_turn_order(
            &BattleCombatState::new(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &mut rng,
        )
        .expect("turn order resolves");

        assert_eq!(order, vec![BattleSide::Player, BattleSide::Enemy]);
    }

    #[test]
    fn move_priority_uses_effect_table_and_move_override_without_name_lists() {
        let quick_attack = move_data_with_effect(
            "CUSTOM_FAST_MOVE",
            pokemon_type("NORMAL"),
            40,
            100,
            "PRIORITY_HIT",
        );
        let vital_throw = move_data("VITAL_THROW", pokemon_type("FIGHTING"), 70, 100);
        let tackle = move_data("TACKLE", pokemon_type("NORMAL"), 35, 100);

        assert_eq!(
            move_priority(&quick_attack, &move_priorities()).expect("priority"),
            2
        );
        assert_eq!(
            move_priority(&vital_throw, &move_priorities()).expect("priority"),
            0
        );
        assert_eq!(
            move_priority(&tackle, &move_priorities()).expect("priority"),
            1
        );
    }

    #[test]
    fn move_priority_table_issues_validate_exact_pack_tokens() {
        let mut moves = BTreeMap::new();
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        moves.insert(
            "QUICK_ATTACK".to_string(),
            move_data_with_effect(
                "QUICK_ATTACK",
                pokemon_type("NORMAL"),
                40,
                100,
                "PRIORITY_HIT",
            ),
        );
        let priorities = MovePriorityTable {
            base_priority: -1,
            effect_priorities: [
                ("NORMAL_HIT".to_string(), -1),
                (" NORMAL_HIT".to_string(), 0),
                ("NORMAL HIT".to_string(), 0),
            ]
            .into_iter()
            .collect(),
            move_priorities: vec![
                MovePriorityOverride {
                    r#move: " QUICK_ATTACK".to_string(),
                    priority: -1,
                },
                MovePriorityOverride {
                    r#move: "QUICK ATTACK".to_string(),
                    priority: 1,
                },
                MovePriorityOverride {
                    r#move: "EXTREME_SPEED".to_string(),
                    priority: 1,
                },
            ],
        };

        assert_eq!(
            move_priority_table_issues(&priorities, &moves, true),
            vec![
                MovePriorityTableIssue::InvalidBasePriority { priority: -1 },
                MovePriorityTableIssue::InvalidMoveEffectPriorityId {
                    move_effect: " NORMAL_HIT".to_string(),
                },
                MovePriorityTableIssue::InvalidMoveEffectPriorityId {
                    move_effect: "NORMAL HIT".to_string(),
                },
                MovePriorityTableIssue::InvalidMoveEffectPriority {
                    move_effect: "NORMAL_HIT".to_string(),
                    priority: -1,
                },
                MovePriorityTableIssue::MissingMoveEffectPriority {
                    move_name: "QUICK_ATTACK".to_string(),
                    move_effect: "PRIORITY_HIT".to_string(),
                },
                MovePriorityTableIssue::InvalidMovePriorityId {
                    move_name: " QUICK_ATTACK".to_string(),
                },
                MovePriorityTableIssue::InvalidMovePriority {
                    move_name: " QUICK_ATTACK".to_string(),
                    priority: -1,
                },
                MovePriorityTableIssue::InvalidMovePriorityId {
                    move_name: "QUICK ATTACK".to_string(),
                },
                MovePriorityTableIssue::UnknownMovePriority {
                    move_name: "EXTREME_SPEED".to_string(),
                },
            ],
        );
        assert_eq!(
            move_priority_table_issues(&MovePriorityTable::default(), &moves, true),
            vec![
                MovePriorityTableIssue::MissingEffectPriorities,
                MovePriorityTableIssue::MissingMoveEffectPriority {
                    move_name: "QUICK_ATTACK".to_string(),
                    move_effect: "PRIORITY_HIT".to_string(),
                },
                MovePriorityTableIssue::MissingMoveEffectPriority {
                    move_name: "TACKLE".to_string(),
                    move_effect: "NORMAL_HIT".to_string(),
                },
            ],
        );
        assert_eq!(
            move_priority_table_issues(&MovePriorityTable::default(), &moves, false),
            []
        );
    }

    #[test]
    fn move_priority_table_issues_reject_reserved_pack_prefix_tokens() {
        let priorities = MovePriorityTable {
            base_priority: 0,
            effect_priorities: [("fallback_priority_hit".to_string(), 1)]
                .into_iter()
                .collect(),
            move_priorities: vec![MovePriorityOverride {
                r#move: "legacy_quick_attack".to_string(),
                priority: 1,
            }],
        };

        assert_eq!(
            move_priority_table_issues(&priorities, &BTreeMap::new(), true),
            vec![
                MovePriorityTableIssue::InvalidMoveEffectPriorityId {
                    move_effect: "fallback_priority_hit".to_string(),
                },
                MovePriorityTableIssue::InvalidMovePriorityId {
                    move_name: "legacy_quick_attack".to_string(),
                },
            ]
        );
    }

    #[test]
    fn move_priority_rejects_missing_table_without_base_priority_fallback() {
        let tackle = move_data("TACKLE", pokemon_type("NORMAL"), 35, 100);
        let mut priorities = move_priorities();
        priorities.effect_priorities.clear();

        let error = move_priority(&tackle, &priorities)
            .expect_err("missing priority table must not fall back to base priority");

        assert_eq!(error, BattleTurnError::MissingMovePriorityTable);
    }

    #[test]
    fn move_priority_rejects_missing_effect_without_base_priority_fallback() {
        let tackle = move_data("TACKLE", pokemon_type("NORMAL"), 35, 100);
        let priorities = MovePriorityTable {
            base_priority: 1,
            effect_priorities: [("PRIORITY_HIT".to_string(), 2)].into_iter().collect(),
            move_priorities: vec![],
        };

        let error = move_priority(&tackle, &priorities)
            .expect_err("missing move effect priority must not use base priority");

        assert_eq!(
            error,
            BattleTurnError::MissingMoveEffectPriority {
                move_effect: "NORMAL_HIT".to_string()
            }
        );
    }

    #[test]
    fn fainted_defender_does_not_take_second_action() {
        let player = pokemon("MACHOP", 80, pokemon_type("FIGHTING"), "KARATE_CHOP");
        let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 1;
        let mut moves = BTreeMap::new();
        moves.insert(
            "KARATE_CHOP".to_string(),
            move_data("KARATE_CHOP", pokemon_type("FIGHTING"), 50, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut rng = Random::new(3);
        let outcome = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("battle turn resolves");

        assert_eq!(outcome.state.enemy.hp, 0);
        assert_eq!(outcome.state.enemy.moves[0].current_pp, 5);
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy
        }));
    }

    #[test]
    fn battle_actions_serialize_exact_modpack_item_ids_without_enum_mapping() {
        let action = BattleAction::Item {
            item_id: "johto_plus:EMBER_ORB".to_string(),
        };
        let json = serde_json::to_string(&action).expect("serialize action");

        assert_eq!(json, r#"{"item":{"item_id":"johto_plus:EMBER_ORB"}}"#);
        assert_eq!(
            serde_json::from_str::<BattleAction>(&json).expect("deserialize action"),
            action
        );
    }

    #[test]
    fn switch_action_records_explicit_event_without_silent_noop() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("switch turn resolves");

        assert_eq!(outcome.order[0], BattleSide::Player);
        assert!(outcome.events.contains(&BattleEvent::Switched {
            side: BattleSide::Player,
            party_index: 1
        }));
    }

    #[test]
    fn battle_item_action_uses_exact_item_payload_before_moves() {
        let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        player.hp = 10;
        player.max_hp = 40;
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let items = [("POTION".to_string(), battle_item("POTION", 20, true))]
            .into_iter()
            .collect();
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn_with_items(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Item {
                    item_id: "POTION".to_string(),
                },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("battle item resolves");

        assert_eq!(outcome.order[0], BattleSide::Player);
        assert!(outcome.events.contains(&BattleEvent::ItemUsed {
            side: BattleSide::Player,
            item_id: "POTION".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::BattleItemEffect {
                side: BattleSide::Player,
                outcome,
            } if outcome.item_id == "POTION"
                && outcome.hp_before == 10
                && outcome.hp_after == 30
                && !outcome.consumed
        )));
        assert!(outcome.state.player.hp < 30);
        assert!(outcome.state.player.hp > 10);
    }

    #[test]
    fn unknown_and_unusable_item_actions_are_explicit_errors_not_noops() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let items = [("MAIL".to_string(), battle_item("MAIL", 20, false))]
            .into_iter()
            .collect();
        let mut rng = Random::new(1);

        let unknown = resolve_battle_turn_with_items(
            BattleCombatState::new(player.clone(), enemy.clone(), rng.seed()),
            BattleTurnInput {
                player: BattleAction::Item {
                    item_id: "POTION".to_string(),
                },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("unknown item rejects");
        assert_eq!(
            unknown,
            BattleTurnError::UnknownItem {
                side: BattleSide::Player,
                item_id: "POTION".to_string()
            }
        );

        let invalid = resolve_battle_turn_with_items(
            BattleCombatState::new(player.clone(), enemy.clone(), rng.seed()),
            BattleTurnInput {
                player: BattleAction::Item {
                    item_id: "POT ION".to_string(),
                },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("malformed item id rejects before unknown lookup");
        assert_eq!(
            invalid,
            BattleTurnError::InvalidItem {
                side: BattleSide::Player,
                item_id: "POT ION".to_string()
            }
        );

        let unusable = resolve_battle_turn_with_items(
            BattleCombatState::new(player.clone(), enemy.clone(), rng.seed()),
            BattleTurnInput {
                player: BattleAction::Item {
                    item_id: "MAIL".to_string(),
                },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("unusable battle item rejects");
        assert_eq!(
            unusable,
            BattleTurnError::UnusableItem {
                side: BattleSide::Player,
                item_id: "MAIL".to_string()
            }
        );
    }

    #[test]
    fn run_action_is_an_explicit_error_not_noop() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let run_error = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("run must be handled by explicit escape runtime");
        assert_eq!(
            run_error,
            BattleTurnError::UnsupportedRunAction {
                side: BattleSide::Player
            }
        );
    }

    #[test]
    fn core_wild_battle_run_uses_exported_escape_rules() {
        let player = pokemon("RATTATA", 999, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
        let state = BattleCombatState::new(player, enemy, 99);
        let rules = BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        };
        let mut rng = Random::new(state.rng_seed_after);

        let escape = resolve_wild_battle_run(&state, &rules, 3, &stat_multipliers(), &mut rng)
            .expect("wild battle run resolves in core battle code");

        assert!(escape.escaped);
        assert_eq!(escape.roll, None);
        assert_eq!(escape.attempts_before, 3);
        assert_eq!(escape.attempts_after, 3);
        assert_eq!(escape.rng_seed_after, 99);
    }

    #[test]
    fn core_wild_battle_run_surfaces_missing_escape_data_without_default_rules() {
        let mut player = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "TACKLE");
        player.stat_boosts.remove(&Stat::Speed);
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let state = BattleCombatState::new(player, enemy, 1);
        let rules = BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        };
        let mut rng = Random::new(state.rng_seed_after);

        let error = resolve_wild_battle_run(&state, &rules, 0, &stat_multipliers(), &mut rng)
            .expect_err("missing speed stage must reject");

        assert_eq!(
            error,
            BattleTurnError::BattleEscape(BattleEscapeError::MissingStatStage {
                side: crate::systems::battle_escape::EscapeSide::Player,
                stat: Stat::Speed,
            })
        );
    }

    #[test]
    fn turn_order_rejects_missing_selected_move_without_priority_fallback() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "QUICK_ATTACK");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut rng = Random::new(1);

        let error = determine_turn_order(
            &BattleCombatState::new(player.clone(), enemy.clone(), rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &mut rng,
        )
        .expect_err("missing move data must not fall back to normal priority");

        assert_eq!(
            error,
            BattleTurnError::MissingMoveData {
                side: BattleSide::Player,
                move_name: "QUICK_ATTACK".to_string()
            }
        );

        let malformed_player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "QUICK ATTACK");
        let malformed_error = determine_turn_order(
            &BattleCombatState::new(malformed_player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &mut rng,
        )
        .expect_err("malformed move data must not fall back to normal priority");

        assert_eq!(
            malformed_error,
            BattleTurnError::InvalidMoveName {
                side: BattleSide::Player,
                move_name: "QUICK ATTACK".to_string()
            }
        );
    }

    #[test]
    fn turn_resolution_rejects_missing_move_slot_without_noop_fallback() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "QUICK_ATTACK");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "QUICK_ATTACK".to_string(),
            move_data("QUICK_ATTACK", pokemon_type("NORMAL"), 40, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut rng = Random::new(1);

        let error = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 9 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("missing move slot must not become a no-op");

        assert_eq!(
            error,
            BattleTurnError::MissingMoveSlot {
                side: BattleSide::Player,
                slot: 9
            }
        );
    }

    #[test]
    fn turn_order_requires_explicit_speed_stage_without_zero_fallback() {
        let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        player.stat_boosts.remove(&Stat::Speed);
        let mut moves = BTreeMap::new();
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut rng = Random::new(1);

        let error = determine_turn_order(
            &BattleCombatState::new(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &mut rng,
        )
        .expect_err("missing speed stage must not default to zero");

        assert_eq!(
            error,
            BattleTurnError::MissingStatStage {
                side: BattleSide::Player,
                stat: Stat::Speed,
            }
        );
    }
}
