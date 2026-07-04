use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::battle::capture::{
    CaptureAttemptContext, CaptureOutcome, CaptureRules, CaptureUseError,
    CaptureWobbleProbability, StoredCapture, complete_active_wild_capture, throw_ball_from_bag,
};
use crate::battle::damage::{TypeCategories, TypeEffectivenessTable, WeatherModifiers};
use crate::battle::start::deactivate_battle;
use crate::battle::stats::BattleStatMultiplierTables;
use crate::battle::turn::{
    BattleAction, BattleEvent, BattleSide, BattleTurnCommitError, BattleTurnError,
    BattleTurnInput, BattleTurnOutcome, MovePriorityTable, active_battle_combat_state,
    commit_battle_turn_outcome, resolve_battle_turn_with_items, resolve_wild_battle_turn_with_items,
};
use crate::models::{Item, Move};
use crate::random::Random;
use crate::state::{BattleMemory, GameState};
use crate::systems::battle_escape::BattleEscapeRules;
use crate::systems::battle_items::{
    BattleItemError, apply_battle_escape_item_use, validate_battle_escape_item,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ActiveBattleFlowEnd {
    Ongoing,
    PlayerFled,
    EnemyFled,
    Caught,
    EnemyFainted,
    PlayerFainted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveBattleChoiceOutcome {
    pub end: ActiveBattleFlowEnd,
    pub turn: Option<BattleTurnOutcome>,
    pub capture: Option<CaptureOutcome>,
    pub stored_capture: Option<StoredCapture>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActiveBattleFlowError {
    #[error("cannot resolve battle choice without an active battle")]
    InactiveBattle,
    #[error("unknown battle item {item_id}")]
    UnknownItem { item_id: String },
    #[error("battle item {item_id} is not in the bag")]
    MissingBagItem { item_id: String },
    #[error("battle turn error: {0:?}")]
    Turn(BattleTurnError),
    #[error("battle turn commit error: {0:?}")]
    Commit(#[from] BattleTurnCommitError),
    #[error("capture use error: {0:?}")]
    CaptureUse(#[from] CaptureUseError),
    #[error("capture completion error: {0}")]
    CaptureComplete(String),
    #[error("battle item error: {0:?}")]
    BattleItem(#[from] BattleItemError),
    #[error("bag update error: {0}")]
    Bag(String),
}

impl From<BattleTurnError> for ActiveBattleFlowError {
    fn from(error: BattleTurnError) -> Self {
        Self::Turn(error)
    }
}

pub fn resolve_active_battle_choice(
    state: &mut GameState,
    player_action: BattleAction,
    enemy_action: BattleAction,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    escape_rules: &BattleEscapeRules,
    capture_rules: &CaptureRules,
    wobble_probabilities: &[CaptureWobbleProbability],
    rng: &mut Random,
) -> Result<ActiveBattleChoiceOutcome, ActiveBattleFlowError> {
    if let BattleAction::Item { item_id } = &player_action {
        if let Some(outcome) = resolve_active_battle_capture_or_escape_item(
            state,
            item_id,
            items,
            capture_rules,
            wobble_probabilities,
            rng,
        )? {
            return Ok(outcome);
        }
    }

    let combat = active_battle_combat_state(state)?;
    let active_party_index = combat.player_party_index;
    let input = BattleTurnInput {
        player: player_action,
        enemy: enemy_action,
    };
    let turn = if matches!(state.battle, BattleMemory::Wild { .. } | BattleMemory::StaticWild { .. })
    {
        resolve_wild_battle_turn_with_items(
            combat,
            input,
            moves,
            items,
            move_priorities,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            escape_rules,
            state.battle_escape_attempts,
            rng,
        )?
    } else {
        resolve_battle_turn_with_items(
            combat,
            input,
            moves,
            items,
            move_priorities,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
        )?
    };
    state.battle_escape_attempts = turn
        .events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::RunAttempt {
                side: BattleSide::Player,
                outcome,
            } => Some(outcome.attempts_after),
            _ => None,
        })
        .last()
        .unwrap_or(state.battle_escape_attempts);
    let end = battle_flow_end_from_turn(&turn);
    commit_battle_turn_outcome(state, active_party_index, &turn)?;
    Ok(ActiveBattleChoiceOutcome {
        end,
        turn: Some(turn),
        capture: None,
        stored_capture: None,
    })
}

fn resolve_active_battle_capture_or_escape_item(
    state: &mut GameState,
    item_id: &str,
    items: &BTreeMap<String, Item>,
    capture_rules: &CaptureRules,
    wobble_probabilities: &[CaptureWobbleProbability],
    rng: &mut Random,
) -> Result<Option<ActiveBattleChoiceOutcome>, ActiveBattleFlowError> {
    let item = items
        .get(item_id)
        .ok_or_else(|| ActiveBattleFlowError::UnknownItem {
            item_id: item_id.to_string(),
        })?;
    if item.battle_capture_ball == Some(true) {
        let (player, enemy, context) = active_capture_context(state, item_id)?;
        let capture = throw_ball_from_bag(
            &mut state.bag,
            item,
            &player,
            &enemy,
            context,
            capture_rules,
            wobble_probabilities,
            rng,
        )?
        .ok_or_else(|| ActiveBattleFlowError::MissingBagItem {
            item_id: item_id.to_string(),
        })?;
        state.rng_seed = capture.rng_seed_after;
        let stored_capture = if capture.caught && !capture.blocked {
            complete_active_wild_capture(state, &capture)
                .map_err(ActiveBattleFlowError::CaptureComplete)?
        } else {
            None
        };
        let end = if stored_capture.is_some() {
            ActiveBattleFlowEnd::Caught
        } else {
            ActiveBattleFlowEnd::Ongoing
        };
        return Ok(Some(ActiveBattleChoiceOutcome {
            end,
            turn: None,
            capture: Some(capture),
            stored_capture,
        }));
    }
    if item.battle_escape_mode.is_some() {
        validate_battle_escape_item(item)?;
        if !state
            .bag
            .remove_item(item, 1)
            .map_err(ActiveBattleFlowError::Bag)?
        {
            return Err(ActiveBattleFlowError::MissingBagItem {
                item_id: item_id.to_string(),
            });
        }
        apply_battle_escape_item_use(state)?;
        return Ok(Some(ActiveBattleChoiceOutcome {
            end: ActiveBattleFlowEnd::PlayerFled,
            turn: None,
            capture: None,
            stored_capture: None,
        }));
    }
    Ok(None)
}

fn active_capture_context(
    state: &GameState,
    ball_id: &str,
) -> Result<(crate::models::Pokemon, crate::models::Pokemon, CaptureAttemptContext), ActiveBattleFlowError>
{
    let active_party_index = state
        .battle_active_party_index
        .ok_or(ActiveBattleFlowError::InactiveBattle)?;
    let player = state.storage.party.pokemon[active_party_index]
        .clone()
        .ok_or(ActiveBattleFlowError::InactiveBattle)?;
    match &state.battle {
        BattleMemory::Wild {
            battle_type,
            enemy_pokemon,
            ..
        }
        | BattleMemory::StaticWild {
            battle_type,
            enemy_pokemon,
            ..
        } => Ok((
            player,
            enemy_pokemon.clone(),
            CaptureAttemptContext {
                ball_id: ball_id.to_string(),
                battle_type: battle_type.clone(),
                trainer_battle: false,
                player_gender: None,
                enemy_gender: None,
            },
        )),
        BattleMemory::Trainer {
            battle_type,
            enemy_pokemon,
            ..
        } => Ok((
            player,
            enemy_pokemon.clone(),
            CaptureAttemptContext {
                ball_id: ball_id.to_string(),
                battle_type: battle_type.clone(),
                trainer_battle: true,
                player_gender: None,
                enemy_gender: None,
            },
        )),
        BattleMemory::Inactive => Err(ActiveBattleFlowError::InactiveBattle),
    }
}

fn battle_flow_end_from_turn(turn: &BattleTurnOutcome) -> ActiveBattleFlowEnd {
    if turn.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::Fled {
                side: BattleSide::Player
            }
        )
    }) {
        return ActiveBattleFlowEnd::PlayerFled;
    }
    if turn.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::Fled {
                side: BattleSide::Enemy
            }
        )
    }) {
        return ActiveBattleFlowEnd::EnemyFled;
    }
    if turn.state.enemy.hp == 0 {
        return ActiveBattleFlowEnd::EnemyFainted;
    }
    if turn.state.player.hp == 0 {
        return ActiveBattleFlowEnd::PlayerFainted;
    }
    ActiveBattleFlowEnd::Ongoing
}

pub fn force_end_active_battle_to_overworld(state: &mut GameState) -> ActiveBattleFlowEnd {
    let end = match &state.battle {
        BattleMemory::Inactive => ActiveBattleFlowEnd::Ongoing,
        BattleMemory::Wild { enemy_pokemon, .. } | BattleMemory::StaticWild { enemy_pokemon, .. } => {
            if enemy_pokemon.hp == 0 {
                ActiveBattleFlowEnd::EnemyFainted
            } else {
                ActiveBattleFlowEnd::PlayerFled
            }
        }
        BattleMemory::Trainer { enemy_party, .. } => {
            if enemy_party.iter().all(|pokemon| pokemon.hp == 0) {
                ActiveBattleFlowEnd::EnemyFainted
            } else {
                ActiveBattleFlowEnd::PlayerFainted
            }
        }
    };
    deactivate_battle(state);
    end
}
