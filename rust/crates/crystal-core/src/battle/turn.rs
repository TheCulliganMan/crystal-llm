use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::battle::damage::{
    DamageCalculationError, DamageContext, DamageResult, Weather, calculate_damage,
};
use crate::battle::stats::apply_stage;
use crate::models::{Move, Pokemon, Stat};
use crate::random::Random;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleTurnError {
    MissingMoveSlot { side: BattleSide, slot: usize },
    MissingMoveData { side: BattleSide, move_name: String },
    MissingStat { side: BattleSide, stat: Stat },
    MissingStatStage { side: BattleSide, stat: Stat },
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
}

pub fn resolve_battle_turn(
    mut state: BattleCombatState,
    input: BattleTurnInput,
    moves: &BTreeMap<String, Move>,
    rng: &mut Random,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    let mut events = Vec::new();
    let order = determine_turn_order(&state, &input, moves, rng)?;

    for side in order.iter().copied() {
        if state.pokemon(side).hp == 0 || state.pokemon(side.other()).hp == 0 {
            continue;
        }
        let action = match side {
            BattleSide::Player => &input.player,
            BattleSide::Enemy => &input.enemy,
        };
        execute_action(&mut state, side, action, moves, rng, &mut events)?;
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
    rng: &mut Random,
) -> Result<Vec<BattleSide>, BattleTurnError> {
    let player_priority = action_priority(BattleSide::Player, &state.player, &input.player, moves)?;
    let enemy_priority = action_priority(BattleSide::Enemy, &state.enemy, &input.enemy, moves)?;
    if player_priority != enemy_priority {
        return Ok(if player_priority > enemy_priority {
            vec![BattleSide::Player, BattleSide::Enemy]
        } else {
            vec![BattleSide::Enemy, BattleSide::Player]
        });
    }

    let player_speed = battle_speed(BattleSide::Player, &state.player)?;
    let enemy_speed = battle_speed(BattleSide::Enemy, &state.enemy)?;
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
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    match action {
        BattleAction::Move { slot } => execute_move_slot(state, side, *slot, moves, rng, events),
        BattleAction::Switch { .. } | BattleAction::Item { .. } | BattleAction::Run => Ok(()),
    }
}

fn execute_move_slot(
    state: &mut BattleCombatState,
    side: BattleSide,
    slot: usize,
    moves: &BTreeMap<String, Move>,
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

    let accuracy = accuracy_byte(move_data);
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
    let (attacker, defender) = match side {
        BattleSide::Player => (&state.player, &state.enemy),
        BattleSide::Enemy => (&state.enemy, &state.player),
    };
    let result = calculate_damage(
        attacker,
        defender,
        move_data,
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
            Ok(move_priority(move_data))
        }
        BattleAction::Switch { .. } | BattleAction::Item { .. } | BattleAction::Run => Ok(0),
    }
}

fn move_priority(move_data: &Move) -> i8 {
    match move_data.name.as_str() {
        "PROTECT" | "DETECT" | "ENDURE" | "MACH_PUNCH" | "QUICK_ATTACK" | "EXTREMESPEED" => 1,
        "VITAL_THROW" | "COUNTER" | "MIRROR_COAT" | "ROAR" | "WHIRLWIND" => -1,
        _ => 0,
    }
}

fn battle_speed(side: BattleSide, pokemon: &Pokemon) -> Result<u16, BattleTurnError> {
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
    Ok(apply_stage(base, stage))
}

fn accuracy_byte(move_data: &Move) -> u8 {
    if move_data.accuracy == 0 || move_data.accuracy >= 100 {
        return u8::MAX;
    }
    ((move_data.accuracy as u16 * 255) / 100).clamp(1, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, LearnedMove, PokemonSpecies, PokemonType};

    fn species(id: &str, speed: u16, pokemon_type: PokemonType) -> PokemonSpecies {
        let mut species =
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, speed, 65, 65));
        species.type1 = pokemon_type;
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
        }];
        pokemon
    }

    fn move_data(name: &str, move_type: PokemonType, power: u16, accuracy: u8) -> Move {
        Move {
            name: name.to_string(),
            move_type,
            power,
            accuracy,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    #[test]
    fn faster_move_user_attacks_first_and_damage_is_deterministic() {
        let player = pokemon("PIKACHU", 90, PokemonType::Electric, "THUNDERSHOCK");
        let enemy = pokemon("GEODUDE", 20, PokemonType::Rock, "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "THUNDERSHOCK".to_string(),
            move_data("THUNDERSHOCK", PokemonType::Electric, 40, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", PokemonType::Normal, 35, 100),
        );
        let mut rng = Random::new(7);
        let outcome = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
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
        let player = pokemon("RATTATA", 30, PokemonType::Normal, "QUICK_ATTACK");
        let enemy = pokemon("PIDGEY", 90, PokemonType::Normal, "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "QUICK_ATTACK".to_string(),
            move_data("QUICK_ATTACK", PokemonType::Normal, 40, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", PokemonType::Normal, 35, 100),
        );
        let mut rng = Random::new(1);
        let order = determine_turn_order(
            &BattleCombatState::new(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &mut rng,
        )
        .expect("turn order resolves");

        assert_eq!(order, vec![BattleSide::Player, BattleSide::Enemy]);
    }

    #[test]
    fn fainted_defender_does_not_take_second_action() {
        let player = pokemon("MACHOP", 80, PokemonType::Fighting, "KARATE_CHOP");
        let mut enemy = pokemon("RATTATA", 10, PokemonType::Normal, "TACKLE");
        enemy.hp = 1;
        let mut moves = BTreeMap::new();
        moves.insert(
            "KARATE_CHOP".to_string(),
            move_data("KARATE_CHOP", PokemonType::Fighting, 50, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", PokemonType::Normal, 35, 100),
        );
        let mut rng = Random::new(3);
        let outcome = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
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
    fn turn_order_rejects_missing_selected_move_without_priority_fallback() {
        let player = pokemon("RATTATA", 30, PokemonType::Normal, "QUICK_ATTACK");
        let enemy = pokemon("PIDGEY", 90, PokemonType::Normal, "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", PokemonType::Normal, 35, 100),
        );
        let mut rng = Random::new(1);

        let error = determine_turn_order(
            &BattleCombatState::new(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
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
        let player = pokemon("RATTATA", 30, PokemonType::Normal, "QUICK_ATTACK");
        let enemy = pokemon("PIDGEY", 90, PokemonType::Normal, "TACKLE");
        let mut moves = BTreeMap::new();
        moves.insert(
            "QUICK_ATTACK".to_string(),
            move_data("QUICK_ATTACK", PokemonType::Normal, 40, 100),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", PokemonType::Normal, 35, 100),
        );
        let mut rng = Random::new(1);

        let error = resolve_battle_turn(
            BattleCombatState::new(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 9 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
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
        let mut player = pokemon("RATTATA", 30, PokemonType::Normal, "TACKLE");
        let enemy = pokemon("PIDGEY", 90, PokemonType::Normal, "TACKLE");
        player.stat_boosts.remove(&Stat::Speed);
        let mut moves = BTreeMap::new();
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", PokemonType::Normal, 35, 100),
        );
        let mut rng = Random::new(1);

        let error = determine_turn_order(
            &BattleCombatState::new(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
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
