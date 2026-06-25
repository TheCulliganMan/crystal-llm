use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::battle::damage::{
    DamageCalculationError, DamageContext, DamageResult, TypeCategories, TypeEffectivenessTable,
    Weather, WeatherModifiers, calculate_damage,
};
use crate::battle::stats::{BattleStatMultiplierTables, accuracy_stage_multiplier, apply_stage};
use crate::models::{Item, Move, Pokemon, Stat};
use crate::random::Random;
use crate::systems::battle_items::{
    BattleItemError, BattleItemOutcome, apply_active_battle_item_effect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
    pub effect_priorities: Vec<MoveEffectPriority>,
    pub move_priorities: Vec<MovePriorityOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveEffectPriority {
    pub move_effect: String,
    pub priority: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovePriorityOverride {
    pub r#move: String,
    pub priority: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleTurnError {
    MissingMoveSlot {
        side: BattleSide,
        slot: usize,
    },
    MissingMoveData {
        side: BattleSide,
        move_name: String,
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
    DamageCalculation(DamageCalculationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        .iter()
        .find(|entry| entry.move_effect == move_data.effect)
        .map(|entry| entry.priority)
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
            type_modifiers: vec![
                crate::battle::damage::WeatherTypeModifier {
                    weather: "WEATHER_RAIN".to_string(),
                    move_type: pokemon_type("WATER"),
                    multiplier: crate::battle::damage::TypeMultiplier {
                        numerator: 3,
                        denominator: 2,
                    },
                },
                crate::battle::damage::WeatherTypeModifier {
                    weather: "WEATHER_RAIN".to_string(),
                    move_type: pokemon_type("FIRE"),
                    multiplier: crate::battle::damage::TypeMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                },
                crate::battle::damage::WeatherTypeModifier {
                    weather: "WEATHER_SUN".to_string(),
                    move_type: pokemon_type("FIRE"),
                    multiplier: crate::battle::damage::TypeMultiplier {
                        numerator: 3,
                        denominator: 2,
                    },
                },
                crate::battle::damage::WeatherTypeModifier {
                    weather: "WEATHER_SUN".to_string(),
                    move_type: pokemon_type("WATER"),
                    multiplier: crate::battle::damage::TypeMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                },
            ],
            move_effect_modifiers: vec![],
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
            effect_priorities: vec![
                MoveEffectPriority {
                    move_effect: "PROTECT".to_string(),
                    priority: 3,
                },
                MoveEffectPriority {
                    move_effect: "ENDURE".to_string(),
                    priority: 3,
                },
                MoveEffectPriority {
                    move_effect: "PRIORITY_HIT".to_string(),
                    priority: 2,
                },
                MoveEffectPriority {
                    move_effect: "FORCE_SWITCH".to_string(),
                    priority: 0,
                },
                MoveEffectPriority {
                    move_effect: "COUNTER".to_string(),
                    priority: 0,
                },
                MoveEffectPriority {
                    move_effect: "MIRROR_COAT".to_string(),
                    priority: 0,
                },
                MoveEffectPriority {
                    move_effect: "NORMAL_HIT".to_string(),
                    priority: 1,
                },
            ],
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
            effect_priorities: vec![MoveEffectPriority {
                move_effect: "PRIORITY_HIT".to_string(),
                priority: 2,
            }],
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
        .expect_err("missing move data must not fall back to normal priority");

        assert_eq!(
            error,
            BattleTurnError::MissingMoveData {
                side: BattleSide::Player,
                move_name: "QUICK_ATTACK".to_string()
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
