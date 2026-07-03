use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::battle::damage::{
    DamageCalculationError, DamageContext, DamageResult, TypeCategories, TypeEffectivenessTable,
    Weather, WeatherModifiers, calculate_damage,
    calculate_type_effectiveness_multiplier_with_foresight, is_physical_type,
};
use crate::battle::start::{ActiveBattleEnemyError, deactivate_battle, update_active_battle_enemy};
use crate::battle::stats::{BattleStatMultiplierTables, accuracy_stage_multiplier, apply_stage};
use crate::models::{Dv, Item, LearnedMove, Move, Pokemon, PokemonSpecies, PokemonType, Stat};
use crate::random::Random;
use crate::state::GameState;
use crate::systems::battle_escape::{
    BattleEscapeAttempt, BattleEscapeError, BattleEscapeRules, attempt_wild_battle_escape,
};
use crate::systems::battle_items::{BattleItemOutcome, apply_active_battle_item_effect};

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
pub struct BattleTrapState {
    pub source: BattleSide,
    pub move_name: String,
    pub turns_remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleDisableState {
    pub move_name: String,
    pub turns_remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleEncoreState {
    pub move_name: String,
    pub turns_remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleEscapeTrapState {
    pub source: BattleSide,
    pub move_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleDamageCategory {
    Physical,
    Special,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleLastDamageState {
    pub source: BattleSide,
    pub move_name: String,
    pub category: BattleDamageCategory,
    pub damage: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleTypeOverride {
    pub type1: PokemonType,
    pub type2: PokemonType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleFutureSightState {
    pub source: BattleSide,
    pub move_name: String,
    pub turns_remaining: u8,
    pub damage: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleTransformState {
    pub species: PokemonSpecies,
    pub dvs: Dv,
    pub moves: Vec<LearnedMove>,
    pub stat_boosts: BTreeMap<Stat, i8>,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub special_attack: u16,
    pub special_defense: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum BattlePokemonGender {
    Male,
    Female,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum BattleScreen {
    Reflect,
    LightScreen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleCombatState {
    pub player: Pokemon,
    pub enemy: Pokemon,
    pub player_party: Vec<Pokemon>,
    pub enemy_party: Vec<Pokemon>,
    pub player_party_index: usize,
    pub enemy_party_index: usize,
    pub weather: Weather,
    pub player_mist_turns: u8,
    pub enemy_mist_turns: u8,
    pub player_safeguard_turns: u8,
    pub enemy_safeguard_turns: u8,
    pub player_reflect_turns: u8,
    pub enemy_reflect_turns: u8,
    pub player_light_screen_turns: u8,
    pub enemy_light_screen_turns: u8,
    pub player_leech_seed_source: Option<BattleSide>,
    pub enemy_leech_seed_source: Option<BattleSide>,
    pub player_curse_source: Option<BattleSide>,
    pub enemy_curse_source: Option<BattleSide>,
    pub player_spikes: bool,
    pub enemy_spikes: bool,
    pub player_nightmare_source: Option<BattleSide>,
    pub enemy_nightmare_source: Option<BattleSide>,
    pub player_toxic_turns: u8,
    pub enemy_toxic_turns: u8,
    pub player_trap: Option<BattleTrapState>,
    pub enemy_trap: Option<BattleTrapState>,
    pub player_escape_trap: Option<BattleEscapeTrapState>,
    pub enemy_escape_trap: Option<BattleEscapeTrapState>,
    pub player_lock_on_target: bool,
    pub enemy_lock_on_target: bool,
    pub player_attracted_by: Option<BattleSide>,
    pub enemy_attracted_by: Option<BattleSide>,
    pub player_recharge_move: Option<String>,
    pub enemy_recharge_move: Option<String>,
    pub player_airborne_move: Option<String>,
    pub enemy_airborne_move: Option<String>,
    pub player_charging_move: Option<String>,
    pub enemy_charging_move: Option<String>,
    pub player_last_move: Option<String>,
    pub enemy_last_move: Option<String>,
    pub player_destiny_bond_active: bool,
    pub enemy_destiny_bond_active: bool,
    pub player_encore: Option<BattleEncoreState>,
    pub enemy_encore: Option<BattleEncoreState>,
    pub player_disable: Option<BattleDisableState>,
    pub enemy_disable: Option<BattleDisableState>,
    pub player_protect_active: bool,
    pub enemy_protect_active: bool,
    pub player_endure_active: bool,
    pub enemy_endure_active: bool,
    pub player_substitute_hp: u16,
    pub enemy_substitute_hp: u16,
    pub player_protect_counter: u8,
    pub enemy_protect_counter: u8,
    pub player_identified: bool,
    pub enemy_identified: bool,
    pub player_last_damage: Option<BattleLastDamageState>,
    pub enemy_last_damage: Option<BattleLastDamageState>,
    pub player_fury_cutter_chain: u8,
    pub enemy_fury_cutter_chain: u8,
    pub player_rollout_chain: u8,
    pub enemy_rollout_chain: u8,
    pub player_rollout_turns: u8,
    pub enemy_rollout_turns: u8,
    pub player_defense_curled: bool,
    pub enemy_defense_curled: bool,
    pub player_rage_active: bool,
    pub enemy_rage_active: bool,
    pub player_bide_turns: u8,
    pub enemy_bide_turns: u8,
    pub player_bide_damage: u16,
    pub enemy_bide_damage: u16,
    pub player_future_sight: Option<BattleFutureSightState>,
    pub enemy_future_sight: Option<BattleFutureSightState>,
    pub player_transform: Option<BattleTransformState>,
    pub enemy_transform: Option<BattleTransformState>,
    pub player_type_override: Option<BattleTypeOverride>,
    pub enemy_type_override: Option<BattleTypeOverride>,
    pub weather_turns: u8,
    pub turn: u32,
    pub rng_seed_after: u32,
}

impl BattleCombatState {
    pub fn new(player: Pokemon, enemy: Pokemon, rng_seed: u32) -> Self {
        let player_party = vec![player.clone()];
        let enemy_party = vec![enemy.clone()];
        Self {
            player,
            enemy,
            player_party,
            enemy_party,
            player_party_index: 0,
            enemy_party_index: 0,
            weather: Weather::None,
            player_mist_turns: 0,
            enemy_mist_turns: 0,
            player_safeguard_turns: 0,
            enemy_safeguard_turns: 0,
            player_reflect_turns: 0,
            enemy_reflect_turns: 0,
            player_light_screen_turns: 0,
            enemy_light_screen_turns: 0,
            player_leech_seed_source: None,
            enemy_leech_seed_source: None,
            player_curse_source: None,
            enemy_curse_source: None,
            player_spikes: false,
            enemy_spikes: false,
            player_nightmare_source: None,
            enemy_nightmare_source: None,
            player_toxic_turns: 0,
            enemy_toxic_turns: 0,
            player_trap: None,
            enemy_trap: None,
            player_escape_trap: None,
            enemy_escape_trap: None,
            player_lock_on_target: false,
            enemy_lock_on_target: false,
            player_attracted_by: None,
            enemy_attracted_by: None,
            player_recharge_move: None,
            enemy_recharge_move: None,
            player_airborne_move: None,
            enemy_airborne_move: None,
            player_charging_move: None,
            enemy_charging_move: None,
            player_last_move: None,
            enemy_last_move: None,
            player_destiny_bond_active: false,
            enemy_destiny_bond_active: false,
            player_encore: None,
            enemy_encore: None,
            player_disable: None,
            enemy_disable: None,
            player_protect_active: false,
            enemy_protect_active: false,
            player_endure_active: false,
            enemy_endure_active: false,
            player_substitute_hp: 0,
            enemy_substitute_hp: 0,
            player_protect_counter: 0,
            enemy_protect_counter: 0,
            player_identified: false,
            enemy_identified: false,
            player_last_damage: None,
            enemy_last_damage: None,
            player_fury_cutter_chain: 0,
            enemy_fury_cutter_chain: 0,
            player_rollout_chain: 0,
            enemy_rollout_chain: 0,
            player_rollout_turns: 0,
            enemy_rollout_turns: 0,
            player_defense_curled: false,
            enemy_defense_curled: false,
            player_rage_active: false,
            enemy_rage_active: false,
            player_bide_turns: 0,
            enemy_bide_turns: 0,
            player_bide_damage: 0,
            enemy_bide_damage: 0,
            player_future_sight: None,
            enemy_future_sight: None,
            player_transform: None,
            enemy_transform: None,
            player_type_override: None,
            enemy_type_override: None,
            weather_turns: 0,
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

    pub fn with_parties(mut self, player_party: Vec<Pokemon>, enemy_party: Vec<Pokemon>) -> Self {
        self.player_party = player_party;
        self.enemy_party = enemy_party;
        self
    }

    pub fn with_party_indices(
        mut self,
        player_party_index: usize,
        enemy_party_index: usize,
    ) -> Self {
        self.player_party_index = player_party_index;
        self.enemy_party_index = enemy_party_index;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleAction {
    Move { slot: usize },
    MoveSwitch { slot: usize, party_index: usize },
    Switch { party_index: usize },
    Item { item_id: String },
    Run,
}

impl<'de> Deserialize<'de> for BattleAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        enum RawBattleAction {
            Move { slot: usize },
            MoveSwitch { slot: usize, party_index: usize },
            Switch { party_index: usize },
            Item { item_id: String },
            Run,
        }

        match RawBattleAction::deserialize(deserializer)? {
            RawBattleAction::Move { slot } => {
                if slot >= 4 {
                    return Err(D::Error::custom(format!(
                        "battle move slot {slot} is outside Crystal move range 0..3"
                    )));
                }
                Ok(Self::Move { slot })
            }
            RawBattleAction::MoveSwitch { slot, party_index } => {
                if slot >= 4 {
                    return Err(D::Error::custom(format!(
                        "battle move slot {slot} is outside Crystal move range 0..3"
                    )));
                }
                if party_index >= crate::models::PARTY_SIZE {
                    return Err(D::Error::custom(format!(
                        "battle move switch party index {party_index} is outside party range"
                    )));
                }
                Ok(Self::MoveSwitch { slot, party_index })
            }
            RawBattleAction::Switch { party_index } => {
                if party_index >= crate::models::PARTY_SIZE {
                    return Err(D::Error::custom(format!(
                        "battle switch party index {party_index} is outside party range"
                    )));
                }
                Ok(Self::Switch { party_index })
            }
            RawBattleAction::Item { item_id } => {
                validate_battle_turn_item_id(BattleSide::Player, &item_id)
                    .map_err(|error| D::Error::custom(format!("{error:?}")))?;
                Ok(Self::Item { item_id })
            }
            RawBattleAction::Run => Ok(Self::Run),
        }
    }
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MovePriorityTable {
    pub base_priority: i8,
    pub effect_priorities: BTreeMap<String, i8>,
    pub move_priorities: Vec<MovePriorityOverride>,
}

impl<'de> Deserialize<'de> for MovePriorityTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMovePriorityTable {
            base_priority: i8,
            effect_priorities: BTreeMap<String, i8>,
            move_priorities: Vec<MovePriorityOverride>,
        }

        let raw = RawMovePriorityTable::deserialize(deserializer)?;
        let table = Self {
            base_priority: raw.base_priority,
            effect_priorities: raw.effect_priorities,
            move_priorities: raw.move_priorities,
        };
        table.validate_shape().map_err(D::Error::custom)?;
        Ok(table)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MovePriorityOverride {
    pub r#move: String,
    pub priority: i8,
}

impl<'de> Deserialize<'de> for MovePriorityOverride {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMovePriorityOverride {
            r#move: String,
            priority: i8,
        }

        let raw = RawMovePriorityOverride::deserialize(deserializer)?;
        if !is_exact_battle_turn_token(&raw.r#move) {
            return Err(D::Error::custom(format!(
                "move priority id {:?} is not exact",
                raw.r#move
            )));
        }
        if raw.priority < 0 {
            return Err(D::Error::custom(format!(
                "move priority {} for {} must be nonnegative",
                raw.priority, raw.r#move
            )));
        }
        Ok(Self {
            r#move: raw.r#move,
            priority: raw.priority,
        })
    }
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
    DuplicateMovePriority {
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

    let mut seen_move_priorities = std::collections::BTreeSet::new();
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
        if !seen_move_priorities.insert(entry.r#move.as_str()) {
            issues.push(MovePriorityTableIssue::DuplicateMovePriority {
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

impl MovePriorityTable {
    fn validate_shape(&self) -> Result<(), String> {
        if self.base_priority < 0 {
            return Err(format!(
                "move priority base_priority {} must be nonnegative",
                self.base_priority
            ));
        }
        if self.effect_priorities.is_empty() {
            return Err("move priority effect_priorities must be explicit".to_string());
        }
        for (move_effect, priority) in &self.effect_priorities {
            if !is_exact_battle_turn_token(move_effect) {
                return Err(format!(
                    "move priority effect id {move_effect:?} is not exact"
                ));
            }
            if *priority < 0 {
                return Err(format!(
                    "move priority effect {move_effect} has negative priority {priority}"
                ));
            }
        }
        let mut seen_moves = std::collections::BTreeSet::new();
        for entry in &self.move_priorities {
            if !seen_moves.insert(entry.r#move.as_str()) {
                return Err(format!(
                    "move priority override {} is duplicated",
                    entry.r#move
                ));
            }
        }
        Ok(())
    }
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
    UnknownHeldItem {
        side: BattleSide,
        item_id: String,
    },
    InvalidHeldItemParameter {
        side: BattleSide,
        item_id: String,
        held_effect: String,
        parameter: i16,
    },
    UnusableItem {
        side: BattleSide,
        item_id: String,
    },
    BattleItem {
        side: BattleSide,
        item_id: String,
        error: String,
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
    UnsupportedMoveEffect {
        side: BattleSide,
        move_name: String,
        effect: String,
        reason: String,
    },
    MissingMoveSwitchTarget {
        side: BattleSide,
        move_name: String,
        effect: String,
    },
    SwitchTargetAlreadyActive {
        side: BattleSide,
        party_index: usize,
    },
    SwitchTargetOutOfRange {
        side: BattleSide,
        party_index: usize,
    },
    SwitchTargetFainted {
        side: BattleSide,
        party_index: usize,
    },
    ActivePartyIndexOutOfRange {
        side: BattleSide,
        party_index: usize,
    },
    RunNotAllowed {
        side: BattleSide,
    },
    ActivePokemonFainted {
        side: BattleSide,
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
        critical: bool,
        critical_roll: u8,
        critical_threshold: u8,
        roll: u8,
        result: DamageResult,
    },
    HeldItemDamageBoost {
        side: BattleSide,
        item_id: String,
        held_effect: String,
        move_type: PokemonType,
        damage_before: u16,
        damage_after: u16,
    },
    BeatUpParticipant {
        side: BattleSide,
        move_name: String,
        party_index: usize,
        species: String,
    },
    FuryCutterPower {
        side: BattleSide,
        move_name: String,
        chain: u8,
        power: u16,
    },
    RolloutPower {
        side: BattleSide,
        move_name: String,
        chain: u8,
        defense_curled: bool,
        power: u16,
    },
    PursuitPower {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        power: u16,
    },
    EarthquakePower {
        side: BattleSide,
        move_name: String,
        target_move: String,
        power: u16,
    },
    MagnitudePower {
        side: BattleSide,
        move_name: String,
        roll: u8,
        power: u16,
    },
    HiddenPowerResolved {
        side: BattleSide,
        move_name: String,
        move_type: PokemonType,
        power: u16,
    },
    PresentPower {
        side: BattleSide,
        move_name: String,
        roll: u8,
        power: u16,
    },
    PresentHeal {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        roll: u8,
        hp_before: u16,
        hp_after: u16,
        amount: u16,
    },
    PresentFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        roll: u8,
    },
    MultiHitCount {
        side: BattleSide,
        move_name: String,
        hits: u8,
        roll: Option<u8>,
    },
    PayDayMoney {
        side: BattleSide,
        move_name: String,
        amount: u32,
    },
    OhkoFailed {
        side: BattleSide,
        move_name: String,
        reason: OhkoFailureReason,
    },
    Splash {
        side: BattleSide,
        move_name: String,
    },
    TeleportFailed {
        side: BattleSide,
        move_name: String,
    },
    ResidualStatusDamage {
        side: BattleSide,
        status: String,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    StatusApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        status: String,
    },
    StatusFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        existing_status: Option<String>,
    },
    StatusHealed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        status_before: String,
    },
    HeldItemStatusHealed {
        side: BattleSide,
        item_id: String,
        held_effect: String,
        status_before: Option<String>,
        confusion_turns_before: u8,
    },
    HeldItemActivated {
        side: BattleSide,
        item_id: String,
        held_effect: String,
    },
    StatusHealFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    StatusImmune {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        status: String,
        target_type1: PokemonType,
        target_type2: PokemonType,
    },
    SafeguardApplied {
        side: BattleSide,
        move_name: String,
        turns: u8,
    },
    SafeguardFailed {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    SafeguardProtected {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        effect: String,
        turns_remaining: u8,
    },
    SafeguardCount {
        side: BattleSide,
        turns_remaining: u8,
    },
    ReflectApplied {
        side: BattleSide,
        move_name: String,
        turns: u8,
    },
    ReflectFailed {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    ReflectCount {
        side: BattleSide,
        turns_remaining: u8,
    },
    LightScreenApplied {
        side: BattleSide,
        move_name: String,
        turns: u8,
    },
    LightScreenFailed {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    LightScreenCount {
        side: BattleSide,
        turns_remaining: u8,
    },
    ScreenDamageReduced {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        screen: BattleScreen,
        damage_before: u16,
        damage_after: u16,
    },
    DestinyBondApplied {
        side: BattleSide,
        move_name: String,
    },
    DestinyBondActivated {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
        source_hp_before: u16,
    },
    SleepTalkSelected {
        side: BattleSide,
        move_name: String,
        selected_slot: usize,
        selected_move: String,
        roll: u8,
    },
    SleepTalkFailed {
        side: BattleSide,
        move_name: String,
    },
    MirrorMoveSelected {
        side: BattleSide,
        move_name: String,
        copied_move: String,
    },
    MirrorMoveFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    MetronomeSelected {
        side: BattleSide,
        move_name: String,
        selected_move: String,
        roll: u8,
    },
    MetronomeFailed {
        side: BattleSide,
        move_name: String,
    },
    MimicApplied {
        side: BattleSide,
        move_name: String,
        slot: usize,
        replaced_move: String,
        copied_move: String,
    },
    MimicFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    SketchApplied {
        side: BattleSide,
        move_name: String,
        slot: usize,
        replaced_move: String,
        copied_move: String,
        copied_pp: u8,
    },
    SketchFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    ConversionApplied {
        side: BattleSide,
        move_name: String,
        selected_move: String,
        new_type: PokemonType,
        roll: u8,
    },
    ConversionFailed {
        side: BattleSide,
        move_name: String,
    },
    Conversion2Applied {
        side: BattleSide,
        move_name: String,
        source_move: String,
        source_type: PokemonType,
        new_type: PokemonType,
        roll: u8,
    },
    Conversion2Failed {
        side: BattleSide,
        move_name: String,
    },
    BideStarted {
        side: BattleSide,
        move_name: String,
        turns: u8,
        roll: u8,
    },
    BideForcedMove {
        side: BattleSide,
        requested_slot: usize,
        requested_move: String,
        bide_slot: usize,
        bide_move: String,
        turns_remaining: u8,
    },
    BideStoring {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
        stored_damage: u16,
    },
    BideStoredDamage {
        side: BattleSide,
        source: BattleSide,
        damage: u16,
        stored_damage: u16,
    },
    BideReleased {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stored_damage: u16,
        damage: u16,
        target_hp_before: u16,
        target_hp_after: u16,
    },
    BideFailed {
        side: BattleSide,
        move_name: String,
    },
    EncoreApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        encored_move: String,
        turns: u8,
        roll: u8,
    },
    EncoreFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    EncoreForcedMove {
        side: BattleSide,
        requested_slot: usize,
        requested_move: String,
        encored_slot: usize,
        encored_move: String,
        turns_remaining: u8,
    },
    EncoreEnded {
        side: BattleSide,
        move_name: String,
    },
    SecondaryStatusMissed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        status: String,
        chance_percent: u8,
        roll: u8,
    },
    FlinchApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    SecondaryFlinchMissed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        chance_percent: u8,
        roll: u8,
    },
    StatStageChanged {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
        amount: i8,
        stage_before: i8,
        stage_after: i8,
    },
    StatStageUnchanged {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
        amount: i8,
        stage: i8,
    },
    SecondaryStatStageMissed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
        amount: i8,
        chance_percent: u8,
        roll: u8,
    },
    MistApplied {
        side: BattleSide,
        move_name: String,
        turns: u8,
    },
    MistFailed {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    MistProtected {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
        amount: i8,
        turns_remaining: u8,
    },
    MistCount {
        side: BattleSide,
        turns_remaining: u8,
    },
    LeechSeedApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    LeechSeedFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    LeechSeedImmune {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        target_type1: PokemonType,
        target_type2: PokemonType,
    },
    LeechSeedDamage {
        side: BattleSide,
        source: BattleSide,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    LeechSeedDrain {
        side: BattleSide,
        target: BattleSide,
        amount: u16,
        hp_before: u16,
        hp_after: u16,
    },
    CurseApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        hp_cost: u16,
        hp_before: u16,
        hp_after: u16,
    },
    CurseFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    CurseDamage {
        side: BattleSide,
        source: BattleSide,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    CurseEnded {
        side: BattleSide,
        source: BattleSide,
    },
    NightmareApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    NightmareFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    NightmareDamage {
        side: BattleSide,
        source: BattleSide,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    NightmareEnded {
        side: BattleSide,
        source: BattleSide,
    },
    TrapApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns: u8,
        roll: u8,
    },
    TrapFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns_remaining: u8,
    },
    TrappedTurn {
        side: BattleSide,
        move_name: String,
        source: BattleSide,
        trap_move_name: String,
        turns_remaining: u8,
    },
    TrapDamage {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
        turns_remaining: u8,
    },
    TrapEnded {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
    },
    EscapeTrapApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    EscapeTrapFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    EscapeTrapEnded {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
    },
    LockOnApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    LockOnFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    LockOnConsumed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    AttractApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        user_gender: BattlePokemonGender,
        target_gender: BattlePokemonGender,
    },
    AttractFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        user_gender: Option<BattlePokemonGender>,
        target_gender: Option<BattlePokemonGender>,
    },
    InfatuatedTurn {
        side: BattleSide,
        move_name: String,
        source: BattleSide,
        roll: u8,
    },
    InfatuatedImmobilized {
        side: BattleSide,
        move_name: String,
        source: BattleSide,
        roll: u8,
    },
    RechargeTurn {
        side: BattleSide,
        move_name: String,
    },
    RechargeStarted {
        side: BattleSide,
        move_name: String,
    },
    AirborneStarted {
        side: BattleSide,
        move_name: String,
    },
    AirborneForcedMove {
        side: BattleSide,
        requested_slot: usize,
        requested_move: String,
        airborne_slot: usize,
        airborne_move: String,
    },
    AirborneAvoided {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        airborne_move: String,
    },
    AirborneEnded {
        side: BattleSide,
        move_name: String,
    },
    ChargeStarted {
        side: BattleSide,
        move_name: String,
    },
    ChargeForcedMove {
        side: BattleSide,
        requested_slot: usize,
        requested_move: String,
        charged_slot: usize,
        charged_move: String,
    },
    ChargeEnded {
        side: BattleSide,
        move_name: String,
    },
    ForceSwitchApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    ForceSwitchFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    SpikesApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    SpikesFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    SpikesDamage {
        side: BattleSide,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    SpikesImmune {
        side: BattleSide,
    },
    SwitchBlocked {
        side: BattleSide,
        party_index: usize,
        source: BattleSide,
        move_name: String,
    },
    RapidSpinCleared {
        side: BattleSide,
        move_name: String,
        cleared_trap: bool,
        cleared_leech_seed: bool,
        cleared_spikes: bool,
    },
    FutureSightQueued {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        damage: u16,
        turns: u8,
    },
    FutureSightFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    FutureSightCount {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    FutureSightDamage {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    TransformApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        species: String,
    },
    TransformFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    CounterDamage {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        countered_move: String,
        category: BattleDamageCategory,
        source_damage: u16,
        damage: u16,
        defender_hp_before: u16,
        defender_hp_after: u16,
    },
    ForesightApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    ForesightFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    DisableApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        disabled_move: String,
        turns: u8,
        roll: u8,
    },
    DisableFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    DisabledMove {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    DisableCount {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    DisableEnded {
        side: BattleSide,
        move_name: String,
    },
    ProtectApplied {
        side: BattleSide,
        move_name: String,
        counter: u8,
        roll: Option<u8>,
    },
    ProtectFailed {
        side: BattleSide,
        move_name: String,
        counter_before: u8,
        roll: Option<u8>,
    },
    MoveProtected {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    SubstituteCreated {
        side: BattleSide,
        move_name: String,
        hp_cost: u16,
        substitute_hp: u16,
        hp_before: u16,
        hp_after: u16,
    },
    SubstituteFailed {
        side: BattleSide,
        move_name: String,
        reason: String,
    },
    SubstituteDamaged {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        damage: u16,
        substitute_hp_before: u16,
        substitute_hp_after: u16,
    },
    SubstituteBroken {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    EndureApplied {
        side: BattleSide,
        move_name: String,
        counter: u8,
        roll: Option<u8>,
    },
    EndureFailed {
        side: BattleSide,
        move_name: String,
        counter_before: u8,
        roll: Option<u8>,
    },
    EnduredHit {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        raw_damage: u16,
    },
    SpiteApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        target_move: String,
        pp_before: u8,
        pp_after: u8,
        reduction: u8,
        roll: u8,
    },
    SpiteFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    StatsReset {
        side: BattleSide,
        move_name: String,
    },
    PsychUpApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
    },
    WeatherApplied {
        side: BattleSide,
        move_name: String,
        weather: Weather,
        turns: u8,
    },
    WeatherContinues {
        weather: Weather,
        turns_remaining: u8,
    },
    SandstormDamage {
        side: BattleSide,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    WeatherEnded {
        weather: Weather,
    },
    ConfusionApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns: u8,
    },
    ConfusionFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns_remaining: u8,
    },
    RampageStarted {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
        roll: u8,
    },
    RampageForcedMove {
        side: BattleSide,
        requested_slot: usize,
        requested_move: String,
        rampage_slot: usize,
        rampage_move: String,
        turns_remaining: u8,
    },
    RolloutForcedMove {
        side: BattleSide,
        requested_slot: usize,
        requested_move: String,
        rollout_slot: usize,
        rollout_move: String,
        turns_remaining: u8,
    },
    RampageEnded {
        side: BattleSide,
        move_name: String,
    },
    SecondaryConfusionMissed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        chance_percent: u8,
        roll: u8,
    },
    ConfusedTurn {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
        roll: u8,
    },
    ConfusionEnded {
        side: BattleSide,
        move_name: String,
    },
    ConfusionSelfDamage {
        side: BattleSide,
        move_name: String,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
        roll: u8,
        result: DamageResult,
    },
    HealApplied {
        side: BattleSide,
        move_name: String,
        hp_before: u16,
        hp_after: u16,
        amount: u16,
    },
    HealFailed {
        side: BattleSide,
        move_name: String,
        hp: u16,
        max_hp: u16,
    },
    HpDrained {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        damage: u16,
        hp_before: u16,
        hp_after: u16,
        amount: u16,
    },
    PainSplitApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        user_hp_before: u16,
        user_hp_after: u16,
        target_hp_before: u16,
        target_hp_after: u16,
    },
    RecoilDamage {
        side: BattleSide,
        move_name: String,
        damage_dealt: u16,
        recoil_damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    JumpKickCrash {
        side: BattleSide,
        move_name: String,
        crash_damage: u16,
        hp_before: u16,
        hp_after: u16,
    },
    SelfdestructDamage {
        side: BattleSide,
        move_name: String,
        hp_before: u16,
    },
    PerishSongApplied {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns: u8,
    },
    PerishSongFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns_remaining: u8,
    },
    PerishSongCount {
        side: BattleSide,
        turns_remaining: u8,
    },
    FocusEnergyApplied {
        side: BattleSide,
        move_name: String,
    },
    FocusEnergyFailed {
        side: BattleSide,
        move_name: String,
    },
    SleepTurn {
        side: BattleSide,
        move_name: String,
        turns_remaining: u8,
    },
    WokeUp {
        side: BattleSide,
        move_name: String,
    },
    FullyParalyzed {
        side: BattleSide,
        move_name: String,
        roll: u8,
    },
    Flinched {
        side: BattleSide,
        move_name: String,
    },
    FrozenTurn {
        side: BattleSide,
        move_name: String,
    },
    Fainted {
        side: BattleSide,
    },
    Switched {
        side: BattleSide,
        party_index: usize,
    },
    BatonPassed {
        side: BattleSide,
        move_name: String,
        party_index: usize,
        stat_boosts: BTreeMap<Stat, i8>,
        confusion_turns: u8,
        focus_energy: bool,
    },
    ItemUsed {
        side: BattleSide,
        item_id: String,
    },
    BattleItemEffect {
        side: BattleSide,
        outcome: BattleItemOutcome,
    },
    HeldItemStolen {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        item_id: String,
    },
    HeldItemStealFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        reason: String,
    },
    RunAttempt {
        side: BattleSide,
        outcome: BattleEscapeAttempt,
    },
    RunBlocked {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
    },
    HeldItemEscape {
        side: BattleSide,
        item_id: String,
        held_effect: String,
    },
    Fled {
        side: BattleSide,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OhkoFailureReason {
    TargetLevelTooHigh {
        attacker_level: u8,
        defender_level: u8,
    },
    Missed {
        accuracy: u8,
        roll: u8,
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
    let commit_party_index = outcome.state.player_party_index;
    validate_player_party_snapshot(state, &outcome.state.player_party, commit_party_index)?;
    validate_enemy_party_snapshot(
        state,
        &outcome.state.enemy_party,
        outcome.state.enemy_party_index,
    )?;
    commit_player_party_snapshot(state, &outcome.state.player_party)?;
    let slot = state
        .storage
        .party
        .pokemon
        .get_mut(commit_party_index)
        .ok_or(BattleTurnCommitError::PartyIndexOutOfRange {
            index: commit_party_index,
        })?;
    if slot.is_none() {
        return Err(BattleTurnCommitError::EmptyPartySlot {
            index: commit_party_index,
        });
    }
    *slot = Some(outcome.state.player.clone());
    if commit_party_index != active_party_index {
        state.battle_active_party_index = Some(commit_party_index);
    }
    if battle_outcome_used_player_heal_bell(outcome) {
        apply_party_heal_bell_commit(state, commit_party_index);
    }
    state.rng_seed = outcome.state.rng_seed_after;
    state.sync_party_from_storage();
    commit_enemy_party_snapshot(state, &outcome.state.enemy_party)?;
    state.battle_active_enemy_party_index = Some(outcome.state.enemy_party_index);
    update_active_battle_enemy(state, outcome.state.enemy.clone())?;
    commit_pay_day_money(state, outcome);
    if outcome
        .events
        .iter()
        .any(|event| matches!(event, BattleEvent::Fled { .. }))
    {
        deactivate_battle(state);
    }
    Ok(())
}

fn commit_pay_day_money(state: &mut GameState, outcome: &BattleTurnOutcome) {
    for amount in outcome.events.iter().filter_map(|event| match event {
        BattleEvent::PayDayMoney {
            side: BattleSide::Player,
            amount,
            ..
        } => Some(*amount),
        BattleEvent::PayDayMoney { .. } => None,
        _ => None,
    }) {
        state.battle_pay_day_money = state.battle_pay_day_money.saturating_add(amount);
    }
}

fn validate_player_party_snapshot(
    state: &GameState,
    party: &[Pokemon],
    active_index: usize,
) -> Result<(), BattleTurnCommitError> {
    if active_index >= party.len() || active_index >= state.storage.party.pokemon.len() {
        return Err(BattleTurnCommitError::PartyIndexOutOfRange {
            index: active_index,
        });
    }
    if party.len() > state.storage.party.pokemon.len() {
        return Err(BattleTurnCommitError::PartyIndexOutOfRange { index: party.len() });
    }
    Ok(())
}

fn validate_enemy_party_snapshot(
    state: &GameState,
    party: &[Pokemon],
    active_index: usize,
) -> Result<(), BattleTurnCommitError> {
    match &state.battle {
        crate::state::BattleMemory::Wild { .. }
        | crate::state::BattleMemory::StaticWild { .. }
        | crate::state::BattleMemory::Trainer { .. } => {
            if active_index >= party.len() {
                return Err(BattleTurnCommitError::ActiveEnemy(
                    ActiveBattleEnemyError::EnemyPartyIndexOutOfRange {
                        index: active_index,
                    },
                ));
            }
            Ok(())
        }
        crate::state::BattleMemory::Inactive => Ok(()),
    }
}

fn commit_player_party_snapshot(
    state: &mut GameState,
    party: &[Pokemon],
) -> Result<(), BattleTurnCommitError> {
    for (index, pokemon) in party.iter().enumerate() {
        let slot = state
            .storage
            .party
            .pokemon
            .get_mut(index)
            .ok_or(BattleTurnCommitError::PartyIndexOutOfRange { index })?;
        *slot = Some(pokemon.clone());
    }
    Ok(())
}

fn commit_enemy_party_snapshot(
    state: &mut GameState,
    party: &[Pokemon],
) -> Result<(), BattleTurnCommitError> {
    match &mut state.battle {
        crate::state::BattleMemory::Wild { enemy_party, .. }
        | crate::state::BattleMemory::StaticWild { enemy_party, .. }
        | crate::state::BattleMemory::Trainer { enemy_party, .. } => {
            *enemy_party = party.to_vec();
            Ok(())
        }
        crate::state::BattleMemory::Inactive => Ok(()),
    }
}

fn clear_switch_in_pokemon_battle_state(pokemon: &mut Pokemon) {
    pokemon.flinching = false;
    pokemon.rampage_turns = 0;
    pokemon.confusion_turns = 0;
    pokemon.perish_song_turns = 0;
    pokemon.focus_energy = false;
    pokemon.turns_in_battle = 0;
    pokemon.stat_boosts.clear();
}

fn battle_outcome_used_player_heal_bell(outcome: &BattleTurnOutcome) -> bool {
    outcome.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::StatusHealed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Player,
                ..
            } if move_name == "HEAL_BELL"
        )
    })
}

fn apply_party_heal_bell_commit(state: &mut GameState, active_party_index: usize) {
    for (party_index, slot) in state.storage.party.pokemon.iter_mut().enumerate() {
        if party_index == active_party_index {
            continue;
        }
        let Some(pokemon) = slot.as_mut() else {
            continue;
        };
        pokemon.status = None;
        pokemon.sleep_turns = 0;
    }
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
    resolve_battle_turn_with_items_for_context(
        state,
        input,
        moves,
        items,
        move_priorities,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        false,
    )
}

fn resolve_battle_turn_with_items_for_context(
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
    force_switch_ends_battle: bool,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    let mut events = Vec::new();
    let mut acted_before = Vec::new();
    validate_active_battle_turn_input(&state, &input, false, false)?;
    clear_turn_last_damage(&mut state);
    apply_held_status_healing(&mut state, items, &mut events)?;
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
            match side {
                BattleSide::Player => matches!(input.enemy, BattleAction::Switch { .. }),
                BattleSide::Enemy => matches!(input.player, BattleAction::Switch { .. }),
            },
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            &acted_before,
            force_switch_ends_battle,
            &mut events,
        )?;
        acted_before.push(side);
    }

    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_leech_seed(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_nightmare(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_curse(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_held_status_healing(&mut state, items, &mut events)?;
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_residual_status(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_trap(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        clear_inactive_escape_traps(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_perish_song(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_future_sight(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_mist(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_safeguard(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_screens(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_disable(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_weather(&mut state, &mut events);
    }
    clear_end_turn_flinching(&mut state);
    clear_turn_last_damage(&mut state);
    state.turn = state.turn.saturating_add(1);
    state.rng_seed_after = rng.seed();
    sync_active_combat_pokemon_into_parties(&mut state)?;
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

pub fn resolve_wild_battle_turn_with_items(
    mut state: BattleCombatState,
    input: BattleTurnInput,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    escape_rules: &BattleEscapeRules,
    attempts_before: u8,
    rng: &mut Random,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    validate_active_battle_turn_input(&state, &input, true, true)?;
    clear_turn_last_damage(&mut state);
    if matches!(input.enemy, BattleAction::Run) {
        state.turn = state.turn.saturating_add(1);
        state.rng_seed_after = rng.seed();
        sync_active_combat_pokemon_into_parties(&mut state)?;
        return Ok(BattleTurnOutcome {
            state,
            order: vec![BattleSide::Enemy],
            events: vec![BattleEvent::Fled {
                side: BattleSide::Enemy,
            }],
        });
    }
    if !matches!(input.player, BattleAction::Run) {
        return resolve_battle_turn_with_items_for_context(
            state,
            input,
            moves,
            items,
            move_priorities,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            true,
        );
    }

    let mut events = Vec::new();
    let acted_before = vec![BattleSide::Player];
    apply_held_status_healing(&mut state, items, &mut events)?;
    let escape = if let Some(trap) = escape_trap_state(&state, BattleSide::Player).cloned() {
        events.push(BattleEvent::RunBlocked {
            side: BattleSide::Player,
            source: trap.source,
            move_name: trap.move_name,
        });
        None
    } else if let Some((item_id, held_effect)) =
        held_escape_item(&state, BattleSide::Player, items)?
    {
        let escape = BattleEscapeAttempt {
            escaped: true,
            chance: escape_rules.rng_roll_values,
            roll: None,
            attempts_before,
            attempts_after: attempts_before,
            rng_seed_after: rng.seed(),
        };
        events.push(BattleEvent::HeldItemEscape {
            side: BattleSide::Player,
            item_id,
            held_effect,
        });
        events.push(BattleEvent::RunAttempt {
            side: BattleSide::Player,
            outcome: escape.clone(),
        });
        Some(escape)
    } else {
        let escape =
            resolve_wild_battle_run(&state, escape_rules, attempts_before, stat_multipliers, rng)?;
        events.push(BattleEvent::RunAttempt {
            side: BattleSide::Player,
            outcome: escape.clone(),
        });
        Some(escape)
    };
    let order = if escape.as_ref().is_some_and(|escape| escape.escaped) {
        vec![BattleSide::Player]
    } else {
        execute_action(
            &mut state,
            BattleSide::Enemy,
            &input.enemy,
            false,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            &acted_before,
            true,
            &mut events,
        )?;
        vec![BattleSide::Player, BattleSide::Enemy]
    };

    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_leech_seed(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_nightmare(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_held_status_healing(&mut state, items, &mut events)?;
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_residual_status(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_trap(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        clear_inactive_escape_traps(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_perish_song(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_future_sight(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_mist(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_safeguard(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_screens(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_disable(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_weather(&mut state, &mut events);
    }
    clear_end_turn_flinching(&mut state);
    clear_turn_last_damage(&mut state);
    state.turn = state.turn.saturating_add(1);
    state.rng_seed_after = rng.seed();
    sync_active_combat_pokemon_into_parties(&mut state)?;
    Ok(BattleTurnOutcome {
        state,
        order,
        events,
    })
}

fn validate_active_battle_turn_input(
    state: &BattleCombatState,
    input: &BattleTurnInput,
    allow_player_run: bool,
    allow_enemy_run: bool,
) -> Result<(), BattleTurnError> {
    validate_active_battle_side_can_act(
        state,
        BattleSide::Player,
        &input.player,
        allow_player_run,
    )?;
    validate_active_battle_side_can_act(state, BattleSide::Enemy, &input.enemy, allow_enemy_run)?;
    Ok(())
}

fn validate_active_battle_side_can_act(
    state: &BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
    allow_run: bool,
) -> Result<(), BattleTurnError> {
    validate_active_battle_side_is_not_fainted(state, side)?;
    if matches!(action, BattleAction::Run) && !allow_run {
        return Err(BattleTurnError::RunNotAllowed { side });
    }
    validate_battle_action_switch_target(state, side, action)?;
    Ok(())
}

fn validate_battle_action_switch_target(
    state: &BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
) -> Result<(), BattleTurnError> {
    let party_index = match action {
        BattleAction::Switch { party_index } | BattleAction::MoveSwitch { party_index, .. } => {
            *party_index
        }
        BattleAction::Move { .. } | BattleAction::Item { .. } | BattleAction::Run => {
            return Ok(());
        }
    };
    let party = match side {
        BattleSide::Player => &state.player_party,
        BattleSide::Enemy => &state.enemy_party,
    };
    let active_index = match side {
        BattleSide::Player => state.player_party_index,
        BattleSide::Enemy => state.enemy_party_index,
    };
    if party_index == active_index {
        return Err(BattleTurnError::SwitchTargetAlreadyActive { side, party_index });
    }
    let target = party
        .get(party_index)
        .ok_or(BattleTurnError::SwitchTargetOutOfRange { side, party_index })?;
    if target.hp == 0 {
        return Err(BattleTurnError::SwitchTargetFainted { side, party_index });
    }
    Ok(())
}

fn validate_active_battle_side_is_not_fainted(
    state: &BattleCombatState,
    side: BattleSide,
) -> Result<(), BattleTurnError> {
    if state.pokemon(side).hp == 0 {
        return Err(BattleTurnError::ActivePokemonFainted { side });
    }
    Ok(())
}

pub fn resolve_battle_enemy_action_with_items(
    mut state: BattleCombatState,
    enemy_action: BattleAction,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    let mut events = Vec::new();
    validate_active_battle_side_is_not_fainted(&state, BattleSide::Player)?;
    validate_active_battle_side_can_act(&state, BattleSide::Enemy, &enemy_action, false)?;
    clear_turn_last_damage(&mut state);
    apply_held_status_healing(&mut state, items, &mut events)?;
    execute_action(
        &mut state,
        BattleSide::Enemy,
        &enemy_action,
        false,
        moves,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        &[],
        false,
        &mut events,
    )?;
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_nightmare(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_held_status_healing(&mut state, items, &mut events)?;
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_residual_status(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_perish_song(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_safeguard(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_screens(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_disable(&mut state, &mut events);
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_weather(&mut state, &mut events);
    }
    clear_end_turn_flinching(&mut state);
    clear_turn_last_damage(&mut state);
    state.turn = state.turn.saturating_add(1);
    state.rng_seed_after = rng.seed();
    sync_active_combat_pokemon_into_parties(&mut state)?;
    Ok(BattleTurnOutcome {
        state,
        order: vec![BattleSide::Enemy],
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
        state,
        BattleSide::Player,
        &input.player,
        &input.enemy,
        moves,
        items,
        move_priorities,
    )?;
    let enemy_priority = action_priority(
        state,
        BattleSide::Enemy,
        &input.enemy,
        &input.player,
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

    let player_quick_claw =
        quick_claw_activates(state, BattleSide::Player, &input.player, items, rng)?;
    let enemy_quick_claw =
        quick_claw_activates(state, BattleSide::Enemy, &input.enemy, items, rng)?;
    if player_quick_claw != enemy_quick_claw {
        return Ok(if player_quick_claw {
            vec![BattleSide::Player, BattleSide::Enemy]
        } else {
            vec![BattleSide::Enemy, BattleSide::Player]
        });
    }

    let player_speed = battle_speed(
        BattleSide::Player,
        &effective_battle_pokemon(state, BattleSide::Player),
        stat_multipliers,
    )?;
    let enemy_speed = battle_speed(
        BattleSide::Enemy,
        &effective_battle_pokemon(state, BattleSide::Enemy),
        stat_multipliers,
    )?;
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
    target_switching: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    acted_before: &[BattleSide],
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if committed_charge_move_state(state, side).is_some() {
        let slot = selected_or_committed_move_slot(state, side, action)?;
        return execute_move_slot(
            state,
            side,
            slot,
            action_move_switch_party_index(action),
            target_switching,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            acted_before,
            force_switch_ends_battle,
            events,
        );
    }
    match action {
        BattleAction::Move { slot } => execute_move_slot(
            state,
            side,
            *slot,
            None,
            target_switching,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            acted_before,
            force_switch_ends_battle,
            events,
        ),
        BattleAction::MoveSwitch { slot, party_index } => execute_move_slot(
            state,
            side,
            *slot,
            Some(*party_index),
            target_switching,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            acted_before,
            force_switch_ends_battle,
            events,
        ),
        BattleAction::Switch { party_index } => {
            if let Some(trap) = escape_trap_state(state, side) {
                events.push(BattleEvent::SwitchBlocked {
                    side,
                    party_index: *party_index,
                    source: trap.source,
                    move_name: trap.move_name.clone(),
                });
                return Ok(());
            }
            clear_side_volatile_conditions(state, side);
            switch_battle_combat_pokemon(state, side, *party_index)?;
            events.push(BattleEvent::Switched {
                side,
                party_index: *party_index,
            });
            apply_switch_in_spikes(state, side, events);
            apply_switch_in_held_effects(state, side, items, rng, events)?;
            Ok(())
        }
        BattleAction::Item { item_id } => execute_item(state, side, item_id, items, events),
        BattleAction::Run => Err(BattleTurnError::RunNotAllowed { side }),
    }
}

fn selected_or_committed_move_slot(
    state: &BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
) -> Result<usize, BattleTurnError> {
    if let Some(committed_move) = committed_charge_move_state(state, side) {
        return state
            .pokemon(side)
            .moves
            .iter()
            .position(|learned| learned.name == committed_move)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: committed_move.to_string(),
            });
    }
    match action {
        BattleAction::Move { slot } | BattleAction::MoveSwitch { slot, .. } => Ok(*slot),
        BattleAction::Switch { .. } | BattleAction::Item { .. } | BattleAction::Run => {
            Err(BattleTurnError::MissingMoveSlot { side, slot: 0 })
        }
    }
}

fn action_move_switch_party_index(action: &BattleAction) -> Option<usize> {
    match action {
        BattleAction::MoveSwitch { party_index, .. } => Some(*party_index),
        _ => None,
    }
}

fn quick_claw_activates(
    state: &BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
) -> Result<bool, BattleTurnError> {
    if !matches!(
        action,
        BattleAction::Move { .. } | BattleAction::MoveSwitch { .. }
    ) {
        return Ok(false);
    }
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(false);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if item.held_effect != "HELD_QUICK_CLAW" {
        return Ok(false);
    }
    if !(1..=255).contains(&item.parameter) {
        return Err(BattleTurnError::InvalidHeldItemParameter {
            side,
            item_id: item_id.to_string(),
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
        });
    }
    Ok((rng.randrange(256) as i16) < item.parameter)
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
                error: error.to_string(),
            }
        })?;
    events.push(BattleEvent::BattleItemEffect { side, outcome });
    Ok(())
}

fn resolve_encored_move<'a>(
    state: &mut BattleCombatState,
    side: BattleSide,
    requested_slot: usize,
    requested_move: String,
    requested_move_data: &'a Move,
    moves: &'a BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(usize, String, &'a Move), BattleTurnError> {
    let Some(encore) = encore_state(state, side).cloned() else {
        return Ok((requested_slot, requested_move, requested_move_data));
    };
    let Some(encored_slot) = state
        .pokemon(side)
        .moves
        .iter()
        .position(|learned| learned.name == encore.move_name)
    else {
        clear_encore_state(state, side);
        events.push(BattleEvent::EncoreEnded {
            side,
            move_name: encore.move_name,
        });
        return Ok((requested_slot, requested_move, requested_move_data));
    };
    let turns_remaining = encore.turns_remaining.saturating_sub(1);
    if turns_remaining == 0 {
        clear_encore_state(state, side);
        events.push(BattleEvent::EncoreEnded {
            side,
            move_name: encore.move_name.clone(),
        });
    } else {
        set_encore_state(
            state,
            side,
            Some(BattleEncoreState {
                move_name: encore.move_name.clone(),
                turns_remaining,
            }),
        );
    }
    events.push(BattleEvent::EncoreForcedMove {
        side,
        requested_slot,
        requested_move,
        encored_slot,
        encored_move: encore.move_name.clone(),
        turns_remaining,
    });
    let encored_data =
        moves
            .get(&encore.move_name)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: encore.move_name.clone(),
            })?;
    Ok((encored_slot, encore.move_name, encored_data))
}

fn resolve_rampage_move<'a>(
    state: &mut BattleCombatState,
    side: BattleSide,
    requested_slot: usize,
    requested_move_name: String,
    requested_move_data: &'a Move,
    moves: &'a BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(usize, String, &'a Move, bool), BattleTurnError> {
    if state.pokemon(side).rampage_turns == 0 {
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    }
    let Some(rampage_move) = last_move(state, side).map(ToOwned::to_owned) else {
        state.pokemon_mut(side).rampage_turns = 0;
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(rampage_slot) = state
        .pokemon(side)
        .moves
        .iter()
        .position(|learned| learned.name == rampage_move)
    else {
        state.pokemon_mut(side).rampage_turns = 0;
        events.push(BattleEvent::RampageEnded {
            side,
            move_name: rampage_move,
        });
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(rampage_data) = moves.get(&rampage_move) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: rampage_move,
        });
    };
    let turns_remaining = state.pokemon(side).rampage_turns;
    events.push(BattleEvent::RampageForcedMove {
        side,
        requested_slot,
        requested_move: requested_move_name,
        rampage_slot,
        rampage_move: rampage_move.clone(),
        turns_remaining,
    });
    Ok((rampage_slot, rampage_move, rampage_data, true))
}

fn resolve_bide_move<'a>(
    state: &mut BattleCombatState,
    side: BattleSide,
    requested_slot: usize,
    requested_move_name: String,
    requested_move_data: &'a Move,
    moves: &'a BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(usize, String, &'a Move, bool), BattleTurnError> {
    if bide_turns(state, side) == 0 {
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    }
    let Some(bide_move) = last_move(state, side).map(ToOwned::to_owned) else {
        reset_bide_state(state, side);
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(bide_slot) = state
        .pokemon(side)
        .moves
        .iter()
        .position(|learned| learned.name == bide_move)
    else {
        reset_bide_state(state, side);
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(bide_data) = moves.get(&bide_move) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: bide_move,
        });
    };
    events.push(BattleEvent::BideForcedMove {
        side,
        requested_slot,
        requested_move: requested_move_name,
        bide_slot,
        bide_move: bide_move.clone(),
        turns_remaining: bide_turns(state, side),
    });
    Ok((bide_slot, bide_move, bide_data, true))
}

fn resolve_rollout_move<'a>(
    state: &mut BattleCombatState,
    side: BattleSide,
    requested_slot: usize,
    requested_move_name: String,
    requested_move_data: &'a Move,
    moves: &'a BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(usize, String, &'a Move, bool), BattleTurnError> {
    if rollout_turns(state, side) == 0 {
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    }
    let Some(rollout_move) = last_move(state, side).map(ToOwned::to_owned) else {
        reset_rollout_state(state, side);
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(rollout_slot) = state
        .pokemon(side)
        .moves
        .iter()
        .position(|learned| learned.name == rollout_move)
    else {
        reset_rollout_state(state, side);
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(rollout_data) = moves.get(&rollout_move) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: rollout_move,
        });
    };
    let turns_remaining = rollout_turns(state, side);
    events.push(BattleEvent::RolloutForcedMove {
        side,
        requested_slot,
        requested_move: requested_move_name,
        rollout_slot,
        rollout_move: rollout_move.clone(),
        turns_remaining,
    });
    Ok((rollout_slot, rollout_move, rollout_data, true))
}

fn resolve_airborne_move<'a>(
    state: &mut BattleCombatState,
    side: BattleSide,
    requested_slot: usize,
    requested_move_name: String,
    requested_move_data: &'a Move,
    moves: &'a BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(usize, String, &'a Move, bool), BattleTurnError> {
    let Some(airborne_move) = airborne_move_state(state, side).map(ToOwned::to_owned) else {
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(airborne_slot) = state
        .pokemon(side)
        .moves
        .iter()
        .position(|learned| learned.name == airborne_move)
    else {
        set_airborne_move_state(state, side, None);
        events.push(BattleEvent::AirborneEnded {
            side,
            move_name: airborne_move,
        });
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(airborne_data) = moves.get(&airborne_move) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: airborne_move,
        });
    };
    events.push(BattleEvent::AirborneForcedMove {
        side,
        requested_slot,
        requested_move: requested_move_name,
        airborne_slot,
        airborne_move: airborne_move.clone(),
    });
    Ok((airborne_slot, airborne_move, airborne_data, true))
}

fn resolve_charging_move<'a>(
    state: &mut BattleCombatState,
    side: BattleSide,
    requested_slot: usize,
    requested_move_name: String,
    requested_move_data: &'a Move,
    moves: &'a BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(usize, String, &'a Move, bool), BattleTurnError> {
    let Some(charged_move) = charging_move_state(state, side).map(ToOwned::to_owned) else {
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(charged_slot) = battle_moves(state, side)
        .iter()
        .position(|learned| learned.name == charged_move)
    else {
        set_charging_move_state(state, side, None);
        events.push(BattleEvent::ChargeEnded {
            side,
            move_name: charged_move,
        });
        return Ok((
            requested_slot,
            requested_move_name,
            requested_move_data,
            false,
        ));
    };
    let Some(charged_data) = moves.get(&charged_move) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: charged_move,
        });
    };
    events.push(BattleEvent::ChargeForcedMove {
        side,
        requested_slot,
        requested_move: requested_move_name,
        charged_slot,
        charged_move: charged_move.clone(),
    });
    Ok((charged_slot, charged_move, charged_data, true))
}

fn execute_move_slot(
    state: &mut BattleCombatState,
    side: BattleSide,
    slot: usize,
    move_switch_party_index: Option<usize>,
    target_switching: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    acted_before: &[BattleSide],
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let Some(requested_move_name) = battle_moves(state, side)
        .get(slot)
        .map(|learned| learned.name.clone())
    else {
        return Err(BattleTurnError::MissingMoveSlot { side, slot });
    };
    validate_battle_turn_move_name(side, &requested_move_name)?;
    let Some(requested_move_data) = moves.get(&requested_move_name) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: requested_move_name,
        });
    };
    events.push(BattleEvent::MoveSelected {
        side,
        slot,
        move_name: requested_move_name.clone(),
    });

    if let Some(recharge_move) = recharge_move_state(state, side).map(ToOwned::to_owned) {
        set_recharge_move_state(state, side, None);
        events.push(BattleEvent::RechargeTurn {
            side,
            move_name: recharge_move,
        });
        return Ok(());
    }

    let (slot, move_name, move_data) = resolve_encored_move(
        state,
        side,
        slot,
        requested_move_name,
        requested_move_data,
        moves,
        events,
    )?;
    let (slot, move_name, move_data, bide_forced) =
        resolve_bide_move(state, side, slot, move_name, move_data, moves, events)?;
    let (slot, move_name, move_data, rollout_forced) =
        resolve_rollout_move(state, side, slot, move_name, move_data, moves, events)?;
    let (slot, move_name, move_data, rampage_forced) =
        resolve_rampage_move(state, side, slot, move_name, move_data, moves, events)?;
    let (slot, move_name, move_data, airborne_forced) =
        resolve_airborne_move(state, side, slot, move_name, move_data, moves, events)?;
    let (slot, move_name, move_data, charge_forced) =
        resolve_charging_move(state, side, slot, move_name, move_data, moves, events)?;

    if let Some(disable) = disable_state(state, side) {
        if disable.move_name == move_name && disable.turns_remaining > 0 {
            events.push(BattleEvent::DisabledMove {
                side,
                move_name,
                turns_remaining: disable.turns_remaining,
            });
            return Ok(());
        }
    }

    let learned_move = battle_moves_mut(state, side).get_mut(slot);
    let Some(learned_move) = learned_move else {
        return Err(BattleTurnError::MissingMoveSlot { side, slot });
    };
    let forced_move =
        bide_forced || rollout_forced || rampage_forced || airborne_forced || charge_forced;
    if !forced_move && learned_move.current_pp == 0 {
        events.push(BattleEvent::NoPp {
            side,
            move_name: move_name.clone(),
        });
        return Ok(());
    }
    if !forced_move {
        learned_move.current_pp = learned_move.current_pp.saturating_sub(1);
    }
    events.push(BattleEvent::MoveUsed {
        side,
        move_name: move_name.clone(),
    });
    if !direct_destiny_bond_effect(move_data) {
        set_destiny_bond_active(state, side, false);
    }
    set_last_move(state, side, Some(move_name.clone()));

    if move_blocked_by_status_or_confusion(
        state,
        side,
        &move_name,
        move_data,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        events,
    )? {
        reset_fury_cutter_chain(state, side);
        reset_rollout_state(state, side);
        reset_bide_state(state, side);
        return Ok(());
    }

    execute_move_effect(
        state,
        side,
        Some(slot),
        &move_name,
        move_data,
        move_switch_party_index,
        target_switching,
        moves,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        acted_before,
        force_switch_ends_battle,
        events,
    )?;
    apply_rampage_progress(
        state,
        side,
        &move_name,
        move_data,
        rampage_forced,
        rng,
        events,
    );
    apply_rollout_progress(state, side, &move_name, move_data, rollout_forced, events);
    apply_fury_cutter_progress(state, side, &move_name, move_data, events);
    Ok(())
}

fn execute_move_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    executing_slot: Option<usize>,
    move_name: &str,
    move_data: &Move,
    move_switch_party_index: Option<usize>,
    target_switching: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    acted_before: &[BattleSide],
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if move_data.effect == "RAGE" {
        set_rage_active(state, side, true);
    }
    if move_data.effect == "OHKO" {
        apply_ohko_effect(
            state,
            side,
            &move_name,
            move_data,
            type_effectiveness,
            rng,
            events,
        )?;
        return Ok(());
    }
    if direct_airborne_effect(move_data) {
        if airborne_move_state(state, side).is_some() {
            set_airborne_move_state(state, side, None);
            events.push(BattleEvent::AirborneEnded {
                side,
                move_name: move_name.to_string(),
            });
        } else {
            set_airborne_move_state(state, side, Some(move_name.to_string()));
            events.push(BattleEvent::AirborneStarted {
                side,
                move_name: move_name.to_string(),
            });
            return Ok(());
        }
    }
    if direct_charge_effect(move_data) {
        if charging_move_state(state, side).is_some() {
            set_charging_move_state(state, side, None);
            events.push(BattleEvent::ChargeEnded {
                side,
                move_name: move_name.to_string(),
            });
        } else if !charge_move_skips_charge(state, move_data) {
            set_charging_move_state(state, side, Some(move_name.to_string()));
            events.push(BattleEvent::ChargeStarted {
                side,
                move_name: move_name.to_string(),
            });
            if move_data.effect == "SKULL_BASH" {
                apply_stat_stage_delta(state, side, &move_name, Stat::Defense, 1, events)?;
            }
            return Ok(());
        }
    }
    let lock_on_active = lock_on_target_state(state, side) && !direct_lock_on_effect(move_data);
    let accuracy = if lock_on_active {
        u8::MAX
    } else {
        let (attacker, defender) = match side {
            BattleSide::Player => (&state.player, &state.enemy),
            BattleSide::Enemy => (&state.enemy, &state.player),
        };
        accuracy_byte_with_weather(
            move_data,
            side,
            attacker,
            defender,
            stat_multipliers,
            state.weather,
        )?
    };
    if lock_on_active {
        set_lock_on_target_state(state, side, false);
        events.push(BattleEvent::LockOnConsumed {
            side,
            move_name: move_name.to_string(),
            target: side.other(),
        });
    }
    if let Some(airborne_move) = airborne_move_state(state, side.other()).map(ToOwned::to_owned) {
        if !lock_on_active && !move_hits_airborne_target(move_data, &airborne_move) {
            events.push(BattleEvent::AirborneAvoided {
                side,
                move_name: move_name.to_string(),
                target: side.other(),
                airborne_move,
            });
            apply_jump_kick_crash_effect(state, side, &move_name, move_data, events);
            return Ok(());
        }
    }
    if accuracy < u8::MAX {
        let roll = rng.randrange(256) as u8;
        if roll >= accuracy {
            events.push(BattleEvent::Missed {
                side,
                move_name: move_name.to_string(),
                accuracy,
                roll,
            });
            apply_jump_kick_crash_effect(state, side, &move_name, move_data, events);
            return Ok(());
        }
    }
    if dream_eater_fails(state, side, move_data) {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    if snore_fails(state, side, move_data) {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    if sleep_talk_fails(state, side, move_data) {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    if counter_effect(move_data).is_some() {
        apply_counter_effect(
            state,
            side,
            &move_name,
            move_data,
            type_effectiveness,
            events,
        )?;
        return Ok(());
    }

    if direct_splash_effect(move_data) {
        events.push(BattleEvent::Splash {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }

    if direct_teleport_effect(move_data) {
        apply_teleport_effect(state, side, &move_name, force_switch_ends_battle, events);
        return Ok(());
    }

    if let Some(status) = direct_status_effect(move_data) {
        let target = side.other();
        if move_blocked_by_safeguard(state, side, &move_name, target, status, events) {
            return Ok(());
        }
        let target_types = effective_pokemon_types(state, target);
        let defender = state.pokemon_mut(target);
        if defender.status.is_none() {
            let applied = apply_status_to_target(
                defender,
                &target_types,
                side,
                &move_name,
                target,
                status,
                rng,
                events,
            );
            if applied && status == "BAD_POISON" {
                set_toxic_turns(state, target, 1);
            }
        } else {
            events.push(BattleEvent::StatusFailed {
                side,
                move_name: move_name.to_string(),
                target,
                existing_status: defender.status.clone(),
            });
        }
        return Ok(());
    }
    if direct_confusion_effect(move_data) {
        apply_confusion_to_target(state, side, &move_name, rng, events);
        return Ok(());
    }
    if direct_swagger_effect(move_data) {
        apply_swagger_effect(state, side, &move_name, rng, events)?;
        return Ok(());
    }
    if direct_heal_effect(move_data) {
        apply_direct_heal_effect(state, side, &move_name, move_data, events);
        return Ok(());
    }
    if direct_heal_bell_effect(move_data) {
        apply_heal_bell_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_pain_split_effect(move_data) {
        apply_pain_split_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_perish_song_effect(move_data) {
        apply_perish_song_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_focus_energy_effect(move_data) {
        apply_focus_energy_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_belly_drum_effect(move_data) {
        apply_belly_drum_effect(state, side, &move_name, events)?;
        return Ok(());
    }
    if direct_defense_curl_effect(move_data) {
        apply_defense_curl_effect(state, side, &move_name, events)?;
        return Ok(());
    }
    if direct_curse_effect(move_data) {
        apply_curse_effect(state, side, &move_name, events)?;
        return Ok(());
    }
    if direct_mist_effect(move_data) {
        apply_mist_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_safeguard_effect(move_data) {
        apply_safeguard_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_substitute_effect(move_data) {
        apply_substitute_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_reflect_effect(move_data) {
        apply_screen_effect(state, side, &move_name, BattleScreen::Reflect, events);
        return Ok(());
    }
    if direct_light_screen_effect(move_data) {
        apply_screen_effect(state, side, &move_name, BattleScreen::LightScreen, events);
        return Ok(());
    }
    if direct_destiny_bond_effect(move_data) {
        apply_destiny_bond_effect(state, side, move_name, events);
        return Ok(());
    }
    if direct_sleep_talk_effect(move_data) {
        apply_sleep_talk_effect(
            state,
            side,
            move_name,
            target_switching,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            acted_before,
            force_switch_ends_battle,
            events,
        )?;
        return Ok(());
    }
    if direct_mirror_move_effect(move_data) {
        apply_mirror_move_effect(
            state,
            side,
            move_name,
            target_switching,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            acted_before,
            force_switch_ends_battle,
            events,
        )?;
        return Ok(());
    }
    if direct_metronome_effect(move_data) {
        apply_metronome_effect(
            state,
            side,
            move_name,
            target_switching,
            moves,
            items,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            rng,
            acted_before,
            force_switch_ends_battle,
            events,
        )?;
        return Ok(());
    }
    if direct_mimic_effect(move_data) {
        apply_mimic_effect(state, side, executing_slot, move_name, moves, events);
        return Ok(());
    }
    if direct_sketch_effect(move_data) {
        apply_sketch_effect(state, side, executing_slot, move_name, moves, events)?;
        return Ok(());
    }
    if direct_conversion_effect(move_data) {
        apply_conversion_effect(state, side, move_name, moves, rng, events)?;
        return Ok(());
    }
    if direct_conversion2_effect(move_data) {
        apply_conversion2_effect(
            state,
            side,
            move_name,
            moves,
            type_categories,
            type_effectiveness,
            rng,
            events,
        )?;
        return Ok(());
    }
    if direct_bide_effect(move_data) {
        apply_bide_effect(state, side, move_name, rng, events);
        return Ok(());
    }
    if direct_encore_effect(move_data) {
        apply_encore_effect(state, side, move_name, rng, events);
        return Ok(());
    }
    if direct_leech_seed_effect(move_data) {
        apply_leech_seed_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_nightmare_effect(move_data) {
        apply_nightmare_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_force_switch_effect(move_data) {
        apply_force_switch_effect(state, side, &move_name, force_switch_ends_battle, events);
        return Ok(());
    }
    if direct_spikes_effect(move_data) {
        apply_spikes_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_escape_trap_effect(move_data) {
        apply_escape_trap_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_lock_on_effect(move_data) {
        apply_lock_on_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_attract_effect(move_data) {
        apply_attract_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_disable_effect(move_data) {
        apply_disable_effect(state, side, &move_name, rng, events);
        return Ok(());
    }
    if direct_protect_effect(move_data) {
        apply_protect_effect(state, side, &move_name, acted_before, rng, events);
        return Ok(());
    }
    if direct_endure_effect(move_data) {
        apply_endure_effect(state, side, &move_name, acted_before, rng, events);
        return Ok(());
    }
    if direct_spite_effect(move_data) {
        apply_spite_effect(state, side, &move_name, rng, events);
        return Ok(());
    }
    if direct_future_sight_effect(move_data) {
        apply_future_sight_effect(
            state,
            side,
            &move_name,
            move_data,
            stat_multipliers,
            rng,
            events,
        )?;
        return Ok(());
    }
    if direct_transform_effect(move_data) {
        apply_transform_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_baton_pass_effect(move_data) {
        apply_baton_pass_effect(
            state,
            side,
            &move_name,
            move_switch_party_index,
            items,
            rng,
            events,
        )?;
        return Ok(());
    }
    if direct_reset_stats_effect(move_data) {
        apply_reset_stats_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_psych_up_effect(move_data) {
        apply_psych_up_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_foresight_effect(move_data) {
        apply_foresight_effect(state, side, &move_name, events);
        return Ok(());
    }
    if direct_beat_up_effect(move_data) {
        apply_beat_up_effect(
            state,
            side,
            &move_name,
            move_data,
            target_switching,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            items,
            rng,
            events,
        )?;
        return Ok(());
    }
    if let Some(weather) = direct_weather_effect(move_data) {
        apply_weather_effect(state, side, &move_name, weather, events);
        return Ok(());
    }
    if move_data.power == 0 && apply_stat_stage_effect(state, side, &move_name, move_data, events)?
    {
        return Ok(());
    }

    let (hit_count, hit_count_roll) = move_hit_count(move_data, rng);
    if hit_count > 1 {
        events.push(BattleEvent::MultiHitCount {
            side,
            move_name: move_name.to_string(),
            hits: hit_count,
            roll: hit_count_roll,
        });
    }
    for _ in 0..hit_count {
        let damage_result = apply_damage_hit(
            state,
            side,
            &move_name,
            move_data,
            target_switching,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            items,
            rng,
            events,
        )?;
        match damage_result {
            DamageHitResult::NoEffect => return Ok(()),
            DamageHitResult::Continue => {}
            DamageHitResult::Stop => return Ok(()),
        }
    }
    Ok(())
}

fn apply_held_item_damage_boost(
    state: &BattleCombatState,
    side: BattleSide,
    move_type: &PokemonType,
    damage: u16,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<u16, BattleTurnError> {
    if damage == 0 {
        return Ok(damage);
    }
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(damage);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if held_item_boosted_move_type(&item.held_effect) != Some(move_type.as_str()) {
        return Ok(damage);
    }
    let boosted = ((u32::from(damage) * 11) / 10).min(u32::from(u16::MAX)) as u16;
    if boosted != damage {
        events.push(BattleEvent::HeldItemDamageBoost {
            side,
            item_id: item_id.to_string(),
            held_effect: item.held_effect.clone(),
            move_type: move_type.clone(),
            damage_before: damage,
            damage_after: boosted,
        });
    }
    Ok(boosted)
}

fn held_item_boosted_move_type(held_effect: &str) -> Option<&'static str> {
    match held_effect {
        "HELD_BUG_BOOST" => Some("BUG"),
        "HELD_DARK_BOOST" => Some("DARK"),
        "HELD_DRAGON_BOOST" => Some("DRAGON"),
        "HELD_ELECTRIC_BOOST" => Some("ELECTRIC"),
        "HELD_FIGHTING_BOOST" => Some("FIGHTING"),
        "HELD_FIRE_BOOST" => Some("FIRE"),
        "HELD_FLYING_BOOST" => Some("FLYING"),
        "HELD_GHOST_BOOST" => Some("GHOST"),
        "HELD_GRASS_BOOST" => Some("GRASS"),
        "HELD_GROUND_BOOST" => Some("GROUND"),
        "HELD_ICE_BOOST" => Some("ICE"),
        "HELD_NORMAL_BOOST" => Some("NORMAL"),
        "HELD_POISON_BOOST" => Some("POISON"),
        "HELD_PSYCHIC_BOOST" => Some("PSYCHIC"),
        "HELD_ROCK_BOOST" => Some("ROCK"),
        "HELD_STEEL_BOOST" => Some("STEEL"),
        "HELD_WATER_BOOST" => Some("WATER"),
        _ => None,
    }
}

fn apply_damage_hit(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    target_switching: bool,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<DamageHitResult, BattleTurnError> {
    let mut prepared_move_data = if move_data.effect == "PRESENT" {
        match apply_present_effect(
            state,
            side,
            move_name,
            move_data,
            type_effectiveness,
            rng,
            events,
        )? {
            PresentEffectOutcome::Damage(power) => {
                let mut prepared = move_data.clone();
                prepared.power = power;
                prepared
            }
            PresentEffectOutcome::Handled(result) => return Ok(result),
        }
    } else {
        let attacker = match side {
            BattleSide::Player => &state.player,
            BattleSide::Enemy => &state.enemy,
        };
        damage_move_data(side, move_name, attacker, move_data, rng, events)
    };
    apply_state_damage_power_modifiers(
        state,
        side,
        target_switching,
        &mut prepared_move_data,
        events,
    );
    let attacker = effective_battle_pokemon(state, side);
    let defender = effective_battle_pokemon(state, side.other());
    let (critical, critical_roll, critical_threshold) = roll_critical_hit(&attacker, rng);
    let damage_roll = 217 + rng.randrange(39) as u8;
    let result = calculate_damage(
        &attacker,
        &defender,
        &prepared_move_data,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        DamageContext {
            is_critical: critical,
            is_confusion_damage: false,
            defender_identified: identified_state(state, side.other()),
            weather: state.weather,
            random_roll: damage_roll,
        },
    )
    .map_err(BattleTurnError::DamageCalculation)?;
    if result.type_multiplier.numerator == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        apply_jump_kick_crash_effect(state, side, move_name, move_data, events);
        return Ok(DamageHitResult::NoEffect);
    }

    let defender_hp_before = state.pokemon(side.other()).hp;
    let mut raw_damage = fixed_damage_amount(&attacker, &defender, &prepared_move_data, rng)
        .unwrap_or(result.damage);
    raw_damage = apply_held_item_damage_boost(
        state,
        side,
        &prepared_move_data.move_type,
        raw_damage,
        items,
        events,
    )?;
    if protect_active(state, side.other()) {
        events.push(BattleEvent::MoveProtected {
            side,
            move_name: move_name.to_string(),
            target: side.other(),
        });
        apply_jump_kick_crash_effect(state, side, move_name, move_data, events);
        return Ok(DamageHitResult::Stop);
    }
    let mut damage = raw_damage.min(defender_hp_before);
    if !critical {
        if let Some(screen) =
            active_damage_screen(state, side.other(), type_categories, &prepared_move_data)?
        {
            let damage_before = damage;
            damage = (damage / 2).max(1).min(defender_hp_before);
            if damage != damage_before {
                events.push(BattleEvent::ScreenDamageReduced {
                    side,
                    move_name: move_name.to_string(),
                    target: side.other(),
                    screen,
                    damage_before,
                    damage_after: damage,
                });
            }
        }
    }
    if move_data.effect == "FALSE_SWIPE" && defender_hp_before > 1 {
        damage = damage.min(defender_hp_before - 1);
    }
    if endure_active(state, side.other()) && defender_hp_before > 1 {
        let endured_damage = damage;
        damage = damage.min(defender_hp_before - 1);
        if endured_damage != damage {
            events.push(BattleEvent::EnduredHit {
                side,
                move_name: move_name.to_string(),
                target: side.other(),
                raw_damage: endured_damage,
            });
        }
    }
    let applied_result = DamageResult {
        damage: raw_damage,
        type_multiplier: result.type_multiplier,
    };
    if let Some(result) = apply_substitute_damage(state, side, move_name, damage, events) {
        return Ok(result);
    }
    let defender = match side {
        BattleSide::Player => &mut state.enemy,
        BattleSide::Enemy => &mut state.player,
    };
    defender.hp = defender.hp.saturating_sub(damage);
    let defender_hp_after = defender.hp;
    events.push(BattleEvent::Damage {
        side,
        move_name: move_name.to_string(),
        damage,
        defender_hp_before,
        defender_hp_after,
        critical,
        critical_roll,
        critical_threshold,
        roll: damage_roll,
        result: applied_result,
    });
    if damage > 0 {
        record_last_damage(
            state,
            side.other(),
            BattleLastDamageState {
                source: side,
                move_name: move_name.to_string(),
                category: damage_category(type_categories, &prepared_move_data)?,
                damage,
            },
        );
        apply_rage_damage_boost(state, side.other(), events)?;
        apply_bide_damage_storage(state, side, side.other(), damage, events);
    }
    let secondary_stat_handled =
        apply_post_damage_stat_effect(state, side, move_name, move_data, damage, rng, events)?;
    apply_post_damage_hp_effect(state, side, move_name, move_data, damage, events);
    apply_direct_damage_faint_events(state, side, side.other(), move_name, events);
    if move_data.effect == "HYPER_BEAM" && damage > 0 && state.pokemon(side.other()).hp != 0 {
        set_recharge_move_state(state, side, Some(move_name.to_string()));
        events.push(BattleEvent::RechargeStarted {
            side,
            move_name: move_name.to_string(),
        });
    }
    if state.pokemon(side).hp == 0
        && !events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Fainted {
                    side: fainted_side
                } if *fainted_side == side
            )
        })
    {
        events.push(BattleEvent::Fainted { side });
        return Ok(DamageHitResult::Stop);
    }
    if state.pokemon(side.other()).hp == 0 {
        return Ok(DamageHitResult::Stop);
    } else if move_data.effect == "TRAP_TARGET" {
        apply_trap_target_effect(state, side, move_name, rng, events);
    } else if move_data.effect == "TRI_ATTACK" {
        apply_tri_attack_effect(state, side, move_name, move_data, rng, events);
    } else if let Some((status, chance_percent)) = secondary_status_effect(move_data) {
        apply_secondary_status_effect(state, side, move_name, status, chance_percent, rng, events);
    } else if let Some(chance_percent) = secondary_confusion_effect(move_data) {
        apply_secondary_confusion_effect(state, side, move_name, chance_percent, rng, events);
    } else if let Some(chance_percent) = secondary_flinch_effect(move_data) {
        apply_secondary_flinch_effect(state, side, move_name, chance_percent, rng, events);
    } else if !secondary_stat_handled {
        apply_secondary_stat_stage_effect(state, side, move_name, move_data, rng, events)?;
    }
    Ok(DamageHitResult::Continue)
}

fn apply_beat_up_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    target_switching: bool,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    _items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let participants = beat_up_participants(state, side);
    if participants.is_empty() {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    if protect_active(state, side.other()) {
        events.push(BattleEvent::MoveProtected {
            side,
            move_name: move_name.to_string(),
            target: side.other(),
        });
        return Ok(());
    }

    for (party_index, participant) in participants {
        let mut prepared_move_data =
            damage_move_data(side, move_name, &participant, move_data, rng, events);
        apply_state_damage_power_modifiers(
            state,
            side,
            target_switching,
            &mut prepared_move_data,
            events,
        );
        let defender = effective_battle_pokemon(state, side.other());
        let damage_roll = 217 + rng.randrange(39) as u8;
        let result = calculate_damage(
            &participant,
            &defender,
            &prepared_move_data,
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            DamageContext {
                is_critical: false,
                is_confusion_damage: false,
                defender_identified: identified_state(state, side.other()),
                weather: state.weather,
                random_roll: damage_roll,
            },
        )
        .map_err(BattleTurnError::DamageCalculation)?;
        events.push(BattleEvent::BeatUpParticipant {
            side,
            move_name: move_name.to_string(),
            party_index,
            species: participant.species.id.clone(),
        });
        if result.type_multiplier.numerator == 0 {
            events.push(BattleEvent::NoEffect {
                side,
                move_name: move_name.to_string(),
            });
            return Ok(());
        }

        let defender_hp_before = state.pokemon(side.other()).hp;
        let raw_damage = result.damage;
        let mut damage = raw_damage.min(defender_hp_before);
        if let Some(screen) =
            active_damage_screen(state, side.other(), type_categories, &prepared_move_data)?
        {
            let damage_before = damage;
            damage = (damage / 2).max(1).min(defender_hp_before);
            if damage != damage_before {
                events.push(BattleEvent::ScreenDamageReduced {
                    side,
                    move_name: move_name.to_string(),
                    target: side.other(),
                    screen,
                    damage_before,
                    damage_after: damage,
                });
            }
        }
        if endure_active(state, side.other()) && defender_hp_before > 1 {
            let endured_damage = damage;
            damage = damage.min(defender_hp_before - 1);
            if endured_damage != damage {
                events.push(BattleEvent::EnduredHit {
                    side,
                    move_name: move_name.to_string(),
                    target: side.other(),
                    raw_damage: endured_damage,
                });
            }
        }
        let defender_mut = state.pokemon_mut(side.other());
        defender_mut.hp = defender_mut.hp.saturating_sub(damage);
        let defender_hp_after = defender_mut.hp;
        events.push(BattleEvent::Damage {
            side,
            move_name: move_name.to_string(),
            damage,
            defender_hp_before,
            defender_hp_after,
            critical: false,
            critical_roll: 0,
            critical_threshold: 0,
            roll: damage_roll,
            result: DamageResult {
                damage: raw_damage,
                type_multiplier: result.type_multiplier,
            },
        });
        if damage > 0 {
            record_last_damage(
                state,
                side.other(),
                BattleLastDamageState {
                    source: side,
                    move_name: move_name.to_string(),
                    category: damage_category(type_categories, &prepared_move_data)?,
                    damage,
                },
            );
            apply_rage_damage_boost(state, side.other(), events)?;
            apply_bide_damage_storage(state, side, side.other(), damage, events);
        }
        apply_direct_damage_faint_events(state, side, side.other(), move_name, events);
        if state.pokemon(side.other()).hp == 0 || state.pokemon(side).hp == 0 {
            return Ok(());
        }
    }

    Ok(())
}

fn beat_up_participants(state: &BattleCombatState, side: BattleSide) -> Vec<(usize, Pokemon)> {
    let party = match side {
        BattleSide::Player => &state.player_party,
        BattleSide::Enemy => &state.enemy_party,
    };
    party
        .iter()
        .enumerate()
        .filter(|(_, pokemon)| pokemon.hp > 0 && pokemon.status.is_none())
        .map(|(index, pokemon)| (index, pokemon.clone()))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DamageHitResult {
    NoEffect,
    Continue,
    Stop,
}

enum PresentEffectOutcome {
    Damage(u16),
    Handled(DamageHitResult),
}

fn apply_present_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    type_effectiveness: &TypeEffectivenessTable,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<PresentEffectOutcome, BattleTurnError> {
    let multiplier = calculate_type_effectiveness_multiplier_with_foresight(
        type_effectiveness,
        &move_data.move_type,
        &effective_pokemon_types(state, side.other()),
        identified_state(state, side.other()),
    )
    .map_err(BattleTurnError::DamageCalculation)?;
    if multiplier.numerator == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(PresentEffectOutcome::Handled(DamageHitResult::NoEffect));
    }

    let roll = rng.randrange(256) as u8;
    match present_roll(roll) {
        PresentRoll::Damage(power) => {
            events.push(BattleEvent::PresentPower {
                side,
                move_name: move_name.to_string(),
                roll,
                power,
            });
            Ok(PresentEffectOutcome::Damage(power))
        }
        PresentRoll::Heal => {
            let target = side.other();
            let defender = state.pokemon_mut(target);
            if defender.hp >= defender.max_hp {
                events.push(BattleEvent::PresentFailed {
                    side,
                    move_name: move_name.to_string(),
                    target,
                    roll,
                });
                return Ok(PresentEffectOutcome::Handled(DamageHitResult::Stop));
            }
            let hp_before = defender.hp;
            let amount = (defender.max_hp / 4)
                .max(1)
                .min(defender.max_hp - defender.hp);
            defender.hp += amount;
            events.push(BattleEvent::PresentHeal {
                side,
                move_name: move_name.to_string(),
                target,
                roll,
                hp_before,
                hp_after: defender.hp,
                amount,
            });
            Ok(PresentEffectOutcome::Handled(DamageHitResult::Stop))
        }
    }
}

enum PresentRoll {
    Damage(u16),
    Heal,
}

fn present_roll(roll: u8) -> PresentRoll {
    match roll {
        0..=102 => PresentRoll::Damage(40),
        103..=178 => PresentRoll::Damage(80),
        179..=204 => PresentRoll::Damage(120),
        _ => PresentRoll::Heal,
    }
}

fn apply_direct_damage_faint_events(
    state: &mut BattleCombatState,
    attacker: BattleSide,
    defender: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    if state.pokemon(defender).hp != 0 {
        return;
    }
    events.push(BattleEvent::Fainted { side: defender });
    if !destiny_bond_active(state, defender) || state.pokemon(attacker).hp == 0 {
        return;
    }
    let source_hp_before = state.pokemon(attacker).hp;
    state.pokemon_mut(attacker).hp = 0;
    set_destiny_bond_active(state, defender, false);
    events.push(BattleEvent::DestinyBondActivated {
        side: defender,
        source: attacker,
        move_name: move_name.to_string(),
        source_hp_before,
    });
    events.push(BattleEvent::Fainted { side: attacker });
}

fn apply_ohko_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    type_effectiveness: &TypeEffectivenessTable,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let attacker = state.pokemon(side);
    let defender = state.pokemon(side.other());
    let type_multiplier = calculate_type_effectiveness_multiplier_with_foresight(
        type_effectiveness,
        &move_data.move_type,
        &effective_pokemon_types(state, side.other()),
        identified_state(state, side.other()),
    )
    .map_err(BattleTurnError::DamageCalculation)?;
    if type_multiplier.numerator == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    if attacker.level < defender.level {
        events.push(BattleEvent::OhkoFailed {
            side,
            move_name: move_name.to_string(),
            reason: OhkoFailureReason::TargetLevelTooHigh {
                attacker_level: attacker.level,
                defender_level: defender.level,
            },
        });
        return Ok(());
    }
    let level_delta = attacker.level.saturating_sub(defender.level);
    let accuracy_percent = move_data
        .accuracy
        .saturating_add(level_delta.saturating_mul(2))
        .min(100);
    let accuracy = ((u16::from(accuracy_percent) * 255) / 100).clamp(1, 255) as u8;
    let roll = rng.randrange(256) as u8;
    if roll >= accuracy {
        events.push(BattleEvent::OhkoFailed {
            side,
            move_name: move_name.to_string(),
            reason: OhkoFailureReason::Missed { accuracy, roll },
        });
        return Ok(());
    }

    let defender_hp_before = state.pokemon(side.other()).hp;
    let defender = state.pokemon_mut(side.other());
    defender.hp = 0;
    events.push(BattleEvent::Damage {
        side,
        move_name: move_name.to_string(),
        damage: defender_hp_before,
        defender_hp_before,
        defender_hp_after: 0,
        critical: false,
        critical_roll: 0,
        critical_threshold: 0,
        roll,
        result: DamageResult {
            damage: defender_hp_before,
            type_multiplier,
        },
    });
    apply_direct_damage_faint_events(state, side, side.other(), move_name, events);
    Ok(())
}

fn type_override(state: &BattleCombatState, side: BattleSide) -> Option<&BattleTypeOverride> {
    match side {
        BattleSide::Player => state.player_type_override.as_ref(),
        BattleSide::Enemy => state.enemy_type_override.as_ref(),
    }
}

fn set_type_override(
    state: &mut BattleCombatState,
    side: BattleSide,
    value: Option<BattleTypeOverride>,
) {
    match side {
        BattleSide::Player => state.player_type_override = value,
        BattleSide::Enemy => state.enemy_type_override = value,
    }
}

fn transform_state(state: &BattleCombatState, side: BattleSide) -> Option<&BattleTransformState> {
    match side {
        BattleSide::Player => state.player_transform.as_ref(),
        BattleSide::Enemy => state.enemy_transform.as_ref(),
    }
}

fn set_transform_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    value: Option<BattleTransformState>,
) {
    match side {
        BattleSide::Player => state.player_transform = value,
        BattleSide::Enemy => state.enemy_transform = value,
    }
}

fn battle_moves(state: &BattleCombatState, side: BattleSide) -> &[LearnedMove] {
    transform_state(state, side)
        .map(|transform| transform.moves.as_slice())
        .unwrap_or_else(|| state.pokemon(side).moves.as_slice())
}

fn battle_moves_mut(state: &mut BattleCombatState, side: BattleSide) -> &mut Vec<LearnedMove> {
    if transform_state(state, side).is_some() {
        match side {
            BattleSide::Player => &mut state.player_transform.as_mut().expect("transform").moves,
            BattleSide::Enemy => &mut state.enemy_transform.as_mut().expect("transform").moves,
        }
    } else {
        match side {
            BattleSide::Player => &mut state.player.moves,
            BattleSide::Enemy => &mut state.enemy.moves,
        }
    }
}

fn effective_battle_pokemon(state: &BattleCombatState, side: BattleSide) -> Pokemon {
    let mut pokemon = state.pokemon(side).clone();
    if let Some(transform) = transform_state(state, side) {
        pokemon.species = transform.species.clone();
        pokemon.dvs = transform.dvs;
        pokemon.moves = transform.moves.clone();
        pokemon.stat_boosts = transform.stat_boosts.clone();
        pokemon.attack = transform.attack;
        pokemon.defense = transform.defense;
        pokemon.speed = transform.speed;
        pokemon.special_attack = transform.special_attack;
        pokemon.special_defense = transform.special_defense;
    }
    if let Some(types) = type_override(state, side) {
        pokemon.species.type1 = types.type1.clone();
        pokemon.species.type2 = types.type2.clone();
    }
    pokemon
}

fn effective_pokemon_types(state: &BattleCombatState, side: BattleSide) -> Vec<PokemonType> {
    let transformed = transform_state(state, side);
    let pokemon = state.pokemon(side);
    let (type1, type2) = type_override(state, side)
        .map(|types| (&types.type1, &types.type2))
        .unwrap_or_else(|| {
            transformed
                .map(|transform| (&transform.species.type1, &transform.species.type2))
                .unwrap_or((&pokemon.species.type1, &pokemon.species.type2))
        });
    let mut types = vec![type1.clone()];
    if type2 != type1 {
        types.push(type2.clone());
    }
    types
}

fn pokemon_types_include(state: &BattleCombatState, side: BattleSide, type_id: &str) -> bool {
    effective_pokemon_types(state, side)
        .iter()
        .any(|pokemon_type| pokemon_type == type_id)
}

fn declared_battle_types(type_categories: &TypeCategories) -> Vec<PokemonType> {
    let mut types = Vec::new();
    for type_id in type_categories
        .physical
        .iter()
        .chain(type_categories.special.iter())
    {
        if !types.contains(type_id) {
            types.push(type_id.clone());
        }
    }
    types
}

fn move_blocked_by_status_or_confusion(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let pokemon = state.pokemon_mut(side);
    let blocked_by_status = match pokemon.status.as_deref() {
        Some("SLEEP") => {
            if pokemon.sleep_turns == 0 {
                pokemon.status = None;
                events.push(BattleEvent::WokeUp {
                    side,
                    move_name: move_name.to_string(),
                });
                false
            } else {
                pokemon.sleep_turns = pokemon.sleep_turns.saturating_sub(1);
                events.push(BattleEvent::SleepTurn {
                    side,
                    move_name: move_name.to_string(),
                    turns_remaining: pokemon.sleep_turns,
                });
                !move_usable_while_asleep(move_data)
            }
        }
        Some("PARALYSIS") => {
            let roll = rng.randrange(4) as u8;
            if roll == 0 {
                events.push(BattleEvent::FullyParalyzed {
                    side,
                    move_name: move_name.to_string(),
                    roll,
                });
                true
            } else {
                false
            }
        }
        Some("FREEZE") => {
            if move_thaws_user(move_data) {
                pokemon.status = None;
                events.push(BattleEvent::StatusHealed {
                    side,
                    move_name: move_name.to_string(),
                    target: side,
                    status_before: "FREEZE".to_string(),
                });
                false
            } else {
                events.push(BattleEvent::FrozenTurn {
                    side,
                    move_name: move_name.to_string(),
                });
                true
            }
        }
        _ => false,
    };
    if blocked_by_status {
        return Ok(true);
    }
    if move_blocked_by_flinch(state, side, move_name, events) {
        return Ok(true);
    }
    if move_blocked_by_trap(state, side, move_name, events) {
        return Ok(true);
    }
    if move_blocked_by_attract(state, side, move_name, rng, events) {
        return Ok(true);
    }
    move_blocked_by_confusion(
        state,
        side,
        move_name,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        events,
    )
}

fn move_blocked_by_flinch(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) -> bool {
    let pokemon = state.pokemon_mut(side);
    if !pokemon.flinching {
        return false;
    }
    pokemon.flinching = false;
    events.push(BattleEvent::Flinched {
        side,
        move_name: move_name.to_string(),
    });
    true
}

fn move_blocked_by_trap(
    state: &BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) -> bool {
    let Some(trap) = trap_state(state, side) else {
        return false;
    };
    events.push(BattleEvent::TrappedTurn {
        side,
        move_name: move_name.to_string(),
        source: trap.source,
        trap_move_name: trap.move_name.clone(),
        turns_remaining: trap.turns_remaining,
    });
    true
}

fn move_blocked_by_attract(
    state: &BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> bool {
    let Some(source) = attracted_by_state(state, side) else {
        return false;
    };
    let roll = rng.randrange(2) as u8;
    events.push(BattleEvent::InfatuatedTurn {
        side,
        move_name: move_name.to_string(),
        source,
        roll,
    });
    if roll == 0 {
        events.push(BattleEvent::InfatuatedImmobilized {
            side,
            move_name: move_name.to_string(),
            source,
            roll,
        });
        return true;
    }
    false
}

fn move_blocked_by_confusion(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let confusion_turns = state.pokemon(side).confusion_turns;
    if confusion_turns == 0 {
        return Ok(false);
    }
    let turns_remaining = confusion_turns.saturating_sub(1);
    state.pokemon_mut(side).confusion_turns = turns_remaining;
    if turns_remaining == 0 {
        events.push(BattleEvent::ConfusionEnded {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(false);
    }

    let confusion_roll = rng.randrange(2) as u8;
    events.push(BattleEvent::ConfusedTurn {
        side,
        move_name: move_name.to_string(),
        turns_remaining,
        roll: confusion_roll,
    });
    if confusion_roll != 0 {
        return Ok(false);
    }

    let damage_roll = 217 + rng.randrange(39) as u8;
    let damage_move = confusion_damage_move();
    let attacker = state.pokemon(side).clone();
    let result = calculate_damage(
        &attacker,
        &attacker,
        &damage_move,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        DamageContext {
            is_critical: false,
            is_confusion_damage: true,
            defender_identified: false,
            weather: state.weather,
            random_roll: damage_roll,
        },
    )
    .map_err(BattleTurnError::DamageCalculation)?;
    let pokemon = state.pokemon_mut(side);
    let hp_before = pokemon.hp;
    let damage = result.damage.min(hp_before);
    pokemon.hp = pokemon.hp.saturating_sub(damage);
    events.push(BattleEvent::ConfusionSelfDamage {
        side,
        move_name: move_name.to_string(),
        damage,
        hp_before,
        hp_after: pokemon.hp,
        roll: damage_roll,
        result,
    });
    if pokemon.hp == 0 {
        events.push(BattleEvent::Fainted { side });
    }
    Ok(true)
}

fn apply_held_status_healing(
    state: &mut BattleCombatState,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if state.pokemon(side).hp == 0 {
            continue;
        }
        let Some(item_id) = state.pokemon(side).item.clone() else {
            continue;
        };
        let item = items
            .get(&item_id)
            .ok_or_else(|| BattleTurnError::UnknownHeldItem {
                side,
                item_id: item_id.clone(),
            })?;
        let held_effect = item.held_effect.clone();
        let status_before = state.pokemon(side).status.clone();
        let confusion_turns_before = state.pokemon(side).confusion_turns;
        let heals_status = held_item_heals_status(&held_effect, status_before.as_deref());
        let heals_confusion = held_effect == "HELD_HEAL_CONFUSION" && confusion_turns_before != 0;
        if !heals_status && !heals_confusion {
            continue;
        }
        if heals_status {
            {
                let pokemon = state.pokemon_mut(side);
                pokemon.status = None;
                pokemon.sleep_turns = 0;
            }
            set_toxic_turns(state, side, 0);
        }
        if heals_confusion {
            state.pokemon_mut(side).confusion_turns = 0;
        }
        state.pokemon_mut(side).item = None;
        events.push(BattleEvent::HeldItemStatusHealed {
            side,
            item_id,
            held_effect,
            status_before,
            confusion_turns_before,
        });
    }
    Ok(())
}

fn held_item_heals_status(held_effect: &str, status: Option<&str>) -> bool {
    match (held_effect, status) {
        ("HELD_HEAL_PARALYZE", Some("PARALYSIS")) => true,
        ("HELD_HEAL_POISON", Some("POISON" | "BAD_POISON")) => true,
        ("HELD_HEAL_FREEZE", Some("FREEZE")) => true,
        ("HELD_HEAL_SLEEP", Some("SLEEP")) => true,
        ("HELD_HEAL_BURN", Some("BURN")) => true,
        (
            "HELD_HEAL_STATUS",
            Some("POISON" | "BAD_POISON" | "BURN" | "FREEZE" | "SLEEP" | "PARALYSIS"),
        ) => true,
        _ => false,
    }
}

fn held_escape_item(
    state: &BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
) -> Result<Option<(String, String)>, BattleTurnError> {
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(None);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if item.held_effect != "HELD_ESCAPE" {
        return Ok(None);
    }
    Ok(Some((item_id.to_string(), item.held_effect.clone())))
}

fn apply_end_turn_residual_status(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let pokemon = state.pokemon(side);
        if pokemon.hp == 0 {
            continue;
        }
        let Some(status) = pokemon.status.as_deref() else {
            continue;
        };
        if !matches!(status, "POISON" | "BAD_POISON" | "BURN") {
            continue;
        }

        let status = status.to_string();
        let hp_before = pokemon.hp;
        let damage = if status == "BAD_POISON" {
            let toxic_turns = toxic_turns(state, side).max(1);
            let damage = (pokemon.max_hp / 16)
                .max(1)
                .saturating_mul(u16::from(toxic_turns))
                .min(hp_before);
            set_toxic_turns(state, side, toxic_turns.saturating_add(1));
            damage
        } else {
            (pokemon.max_hp / 8).max(1).min(hp_before)
        };
        let pokemon = state.pokemon_mut(side);
        pokemon.hp = pokemon.hp.saturating_sub(damage);
        events.push(BattleEvent::ResidualStatusDamage {
            side,
            status,
            damage,
            hp_before,
            hp_after: pokemon.hp,
        });
        if pokemon.hp == 0 {
            events.push(BattleEvent::Fainted { side });
        }
    }
}

fn apply_end_turn_leech_seed(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(source) = leech_seed_source(state, side) else {
            continue;
        };
        if state.pokemon(side).hp == 0 {
            continue;
        }

        let hp_before = state.pokemon(side).hp;
        let damage = (state.pokemon(side).max_hp / 8).max(1).min(hp_before);
        {
            let seeded = state.pokemon_mut(side);
            seeded.hp = seeded.hp.saturating_sub(damage);
            events.push(BattleEvent::LeechSeedDamage {
                side,
                source,
                damage,
                hp_before,
                hp_after: seeded.hp,
            });
            if seeded.hp == 0 {
                events.push(BattleEvent::Fainted { side });
            }
        }

        let source_hp = state.pokemon(source).hp;
        let source_max_hp = state.pokemon(source).max_hp;
        if source_hp != 0 && source_hp < source_max_hp {
            let heal_amount = damage.min(source_max_hp - source_hp);
            let recipient = state.pokemon_mut(source);
            recipient.hp += heal_amount;
            events.push(BattleEvent::LeechSeedDrain {
                side: source,
                target: side,
                amount: heal_amount,
                hp_before: source_hp,
                hp_after: recipient.hp,
            });
        }
    }
}

fn apply_end_turn_nightmare(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(source) = nightmare_source(state, side) else {
            continue;
        };
        if state.pokemon(side).hp == 0 {
            continue;
        }
        if state.pokemon(side).status.as_deref() != Some("SLEEP") || state.pokemon(source).hp == 0 {
            set_nightmare_source(state, side, None);
            events.push(BattleEvent::NightmareEnded { side, source });
            continue;
        }

        let hp_before = state.pokemon(side).hp;
        let damage = (state.pokemon(side).max_hp / 4).max(1).min(hp_before);
        let target = state.pokemon_mut(side);
        target.hp = target.hp.saturating_sub(damage);
        events.push(BattleEvent::NightmareDamage {
            side,
            source,
            damage,
            hp_before,
            hp_after: target.hp,
        });
        if target.hp == 0 {
            events.push(BattleEvent::Fainted { side });
        }
    }
}

fn apply_end_turn_curse(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(source) = curse_source(state, side) else {
            continue;
        };
        if state.pokemon(side).hp == 0 {
            continue;
        }
        let hp_before = state.pokemon(side).hp;
        let damage = (state.pokemon(side).max_hp / 4).max(1).min(hp_before);
        let target = state.pokemon_mut(side);
        target.hp = target.hp.saturating_sub(damage);
        events.push(BattleEvent::CurseDamage {
            side,
            source,
            damage,
            hp_before,
            hp_after: target.hp,
        });
        if target.hp == 0 {
            events.push(BattleEvent::Fainted { side });
        }
    }
}

fn apply_end_turn_trap(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(trap) = trap_state(state, side).cloned() else {
            continue;
        };
        if state.pokemon(side).hp == 0 {
            continue;
        }
        if state.pokemon(trap.source).hp == 0 {
            clear_trap_state(state, side);
            events.push(BattleEvent::TrapEnded {
                side,
                source: trap.source,
                move_name: trap.move_name,
            });
            continue;
        }

        let hp_before = state.pokemon(side).hp;
        let damage = (state.pokemon(side).max_hp / 16).max(1).min(hp_before);
        let turns_remaining = trap.turns_remaining.saturating_sub(1);
        {
            let trapped = state.pokemon_mut(side);
            trapped.hp = trapped.hp.saturating_sub(damage);
        }
        if turns_remaining == 0 {
            clear_trap_state(state, side);
        } else {
            set_trap_state(
                state,
                side,
                Some(BattleTrapState {
                    turns_remaining,
                    ..trap.clone()
                }),
            );
        }
        events.push(BattleEvent::TrapDamage {
            side,
            source: trap.source,
            move_name: trap.move_name.clone(),
            damage,
            hp_before,
            hp_after: state.pokemon(side).hp,
            turns_remaining,
        });
        if state.pokemon(side).hp == 0 {
            events.push(BattleEvent::Fainted { side });
        } else if turns_remaining == 0 {
            events.push(BattleEvent::TrapEnded {
                side,
                source: trap.source,
                move_name: trap.move_name,
            });
        }
    }
}

fn clear_inactive_escape_traps(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(trap) = escape_trap_state(state, side).cloned() else {
            continue;
        };
        if state.pokemon(trap.source).hp != 0 {
            continue;
        }
        clear_escape_trap_state(state, side);
        events.push(BattleEvent::EscapeTrapEnded {
            side,
            source: trap.source,
            move_name: trap.move_name,
        });
    }
}

fn apply_end_turn_perish_song(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let pokemon = state.pokemon_mut(side);
        if pokemon.hp == 0 || pokemon.perish_song_turns == 0 {
            continue;
        }
        pokemon.perish_song_turns = pokemon.perish_song_turns.saturating_sub(1);
        events.push(BattleEvent::PerishSongCount {
            side,
            turns_remaining: pokemon.perish_song_turns,
        });
        if pokemon.perish_song_turns == 0 {
            pokemon.hp = 0;
            events.push(BattleEvent::Fainted { side });
        }
    }
}

fn apply_end_turn_future_sight(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(queued) = future_sight_state(state, side).cloned() else {
            continue;
        };
        let turns_remaining = queued.turns_remaining.saturating_sub(1);
        if turns_remaining > 0 {
            set_future_sight_state(
                state,
                side,
                Some(BattleFutureSightState {
                    turns_remaining,
                    ..queued.clone()
                }),
            );
            events.push(BattleEvent::FutureSightCount {
                side,
                source: queued.source,
                move_name: queued.move_name,
                turns_remaining,
            });
            continue;
        }

        set_future_sight_state(state, side, None);
        if state.pokemon(side).hp == 0 {
            continue;
        }
        let hp_before = state.pokemon(side).hp;
        let damage = queued.damage.min(hp_before);
        state.pokemon_mut(side).hp = hp_before.saturating_sub(damage);
        events.push(BattleEvent::FutureSightDamage {
            side,
            source: queued.source,
            move_name: queued.move_name,
            damage,
            hp_before,
            hp_after: state.pokemon(side).hp,
        });
        if state.pokemon(side).hp == 0 {
            events.push(BattleEvent::Fainted { side });
        }
    }
}

fn apply_end_turn_mist(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let turns = mist_turns(state, side);
        if turns == 0 {
            continue;
        }
        let turns_remaining = turns.saturating_sub(1);
        set_mist_turns(state, side, turns_remaining);
        events.push(BattleEvent::MistCount {
            side,
            turns_remaining,
        });
    }
}

fn apply_end_turn_safeguard(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let turns = safeguard_turns(state, side);
        if turns == 0 {
            continue;
        }
        let turns_remaining = turns.saturating_sub(1);
        set_safeguard_turns(state, side, turns_remaining);
        events.push(BattleEvent::SafeguardCount {
            side,
            turns_remaining,
        });
    }
}

fn apply_end_turn_screens(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        for screen in [BattleScreen::Reflect, BattleScreen::LightScreen] {
            let turns = screen_turns(state, side, screen);
            if turns == 0 {
                continue;
            }
            let turns_remaining = turns.saturating_sub(1);
            set_screen_turns(state, side, screen, turns_remaining);
            match screen {
                BattleScreen::Reflect => events.push(BattleEvent::ReflectCount {
                    side,
                    turns_remaining,
                }),
                BattleScreen::LightScreen => events.push(BattleEvent::LightScreenCount {
                    side,
                    turns_remaining,
                }),
            }
        }
    }
}

fn apply_end_turn_disable(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        let Some(disable) = disable_state(state, side).cloned() else {
            continue;
        };
        let turns_remaining = disable.turns_remaining.saturating_sub(1);
        if turns_remaining == 0 {
            clear_disable_state(state, side);
            events.push(BattleEvent::DisableEnded {
                side,
                move_name: disable.move_name,
            });
        } else {
            set_disable_state(
                state,
                side,
                Some(BattleDisableState {
                    turns_remaining,
                    ..disable.clone()
                }),
            );
            events.push(BattleEvent::DisableCount {
                side,
                move_name: disable.move_name,
                turns_remaining,
            });
        }
    }
}

fn apply_end_turn_weather(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    if state.weather == Weather::None || state.weather_turns == 0 {
        return;
    }
    let weather = state.weather;
    if weather == Weather::Sandstorm {
        apply_end_turn_sandstorm_damage(state, events);
    }
    state.weather_turns = state.weather_turns.saturating_sub(1);
    if state.weather_turns == 0 {
        state.weather = Weather::None;
        events.push(BattleEvent::WeatherEnded { weather });
    } else {
        events.push(BattleEvent::WeatherContinues {
            weather,
            turns_remaining: state.weather_turns,
        });
    }
}

fn apply_end_turn_sandstorm_damage(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if state.pokemon(side).hp == 0 || pokemon_is_sandstorm_immune(state, side) {
            continue;
        }
        let pokemon = state.pokemon_mut(side);
        let hp_before = pokemon.hp;
        let damage = (pokemon.max_hp / 16).max(1);
        pokemon.hp = pokemon.hp.saturating_sub(damage);
        events.push(BattleEvent::SandstormDamage {
            side,
            damage,
            hp_before,
            hp_after: pokemon.hp,
        });
        if pokemon.hp == 0 {
            events.push(BattleEvent::Fainted { side });
        }
    }
}

fn clear_end_turn_flinching(state: &mut BattleCombatState) {
    state.player.flinching = false;
    state.enemy.flinching = false;
    clear_end_turn_protect_endure(state);
}

fn battle_continues_after_actions(state: &BattleCombatState, events: &[BattleEvent]) -> bool {
    state.player.hp != 0
        && state.enemy.hp != 0
        && !events
            .iter()
            .any(|event| matches!(event, BattleEvent::Fled { .. }))
}

pub fn battle_move_effect_is_supported(effect: &str) -> bool {
    SUPPORTED_BATTLE_MOVE_EFFECTS.binary_search(&effect).is_ok()
}

pub fn supported_battle_move_effects() -> &'static [&'static str] {
    SUPPORTED_BATTLE_MOVE_EFFECTS
}

const SUPPORTED_BATTLE_MOVE_EFFECTS: &[&str] = &[
    "ACCURACY_DOWN",
    "ACCURACY_DOWN_HIT",
    "ALL_UP_HIT",
    "ALWAYS_HIT",
    "ATTACK_DOWN",
    "ATTACK_DOWN_2",
    "ATTACK_DOWN_HIT",
    "ATTACK_UP",
    "ATTACK_UP_2",
    "ATTACK_UP_HIT",
    "ATTRACT",
    "BATON_PASS",
    "BEAT_UP",
    "BELLY_DRUM",
    "BIDE",
    "BURN_HIT",
    "CONFUSE",
    "CONFUSE_HIT",
    "CONVERSION",
    "CONVERSION2",
    "COUNTER",
    "CURSE",
    "DEFENSE_CURL",
    "DEFENSE_DOWN",
    "DEFENSE_DOWN_2",
    "DEFENSE_DOWN_HIT",
    "DEFENSE_UP",
    "DEFENSE_UP_2",
    "DEFENSE_UP_HIT",
    "DESTINY_BOND",
    "DISABLE",
    "DOUBLE_HIT",
    "DREAM_EATER",
    "EARTHQUAKE",
    "ENCORE",
    "ENDURE",
    "EVASION_DOWN",
    "EVASION_DOWN_HIT",
    "EVASION_UP",
    "FALSE_SWIPE",
    "FLAME_WHEEL",
    "FLINCH_HIT",
    "FLY",
    "FOCUS_ENERGY",
    "FORCE_SWITCH",
    "FORESIGHT",
    "FREEZE_HIT",
    "FRUSTRATION",
    "FURY_CUTTER",
    "FUTURE_SIGHT",
    "GUST",
    "HEAL",
    "HEAL_BELL",
    "HIDDEN_POWER",
    "HYPER_BEAM",
    "JUMP_KICK",
    "LEECH_HIT",
    "LEECH_SEED",
    "LEVEL_DAMAGE",
    "LIGHT_SCREEN",
    "LOCK_ON",
    "MAGNITUDE",
    "MEAN_LOOK",
    "METRONOME",
    "MIMIC",
    "MIRROR_COAT",
    "MIRROR_MOVE",
    "MIST",
    "MOONLIGHT",
    "MORNING_SUN",
    "MULTI_HIT",
    "NIGHTMARE",
    "NORMAL_HIT",
    "OHKO",
    "PAIN_SPLIT",
    "PARALYZE",
    "PARALYZE_HIT",
    "PAY_DAY",
    "PERISH_SONG",
    "POISON",
    "POISON_HIT",
    "POISON_MULTI_HIT",
    "PRESENT",
    "PRIORITY_HIT",
    "PROTECT",
    "PSYCH_UP",
    "PSYWAVE",
    "PURSUIT",
    "RAGE",
    "RAIN_DANCE",
    "RAMPAGE",
    "RAPID_SPIN",
    "RAZOR_WIND",
    "RECOIL_HIT",
    "REFLECT",
    "RESET_STATS",
    "RETURN",
    "REVERSAL",
    "ROLLOUT",
    "SACRED_FIRE",
    "SAFEGUARD",
    "SANDSTORM",
    "SELFDESTRUCT",
    "SKETCH",
    "SKULL_BASH",
    "SKY_ATTACK",
    "SLEEP",
    "SLEEP_TALK",
    "SNORE",
    "SOLARBEAM",
    "SPECIAL_ATTACK_UP",
    "SPECIAL_DEFENSE_DOWN_HIT",
    "SPECIAL_DEFENSE_UP_2",
    "SPEED_DOWN",
    "SPEED_DOWN_2",
    "SPEED_DOWN_HIT",
    "SPEED_UP_2",
    "SPIKES",
    "SPITE",
    "SPLASH",
    "STATIC_DAMAGE",
    "STOMP",
    "SUBSTITUTE",
    "SUNNY_DAY",
    "SUPER_FANG",
    "SWAGGER",
    "SYNTHESIS",
    "TELEPORT",
    "THIEF",
    "THUNDER",
    "TOXIC",
    "TRANSFORM",
    "TRAP_TARGET",
    "TRIPLE_KICK",
    "TRI_ATTACK",
    "TWISTER",
];

fn direct_status_effect(move_data: &Move) -> Option<&'static str> {
    if move_data.power != 0 {
        return None;
    }
    match move_data.effect.as_str() {
        "SLEEP" => Some("SLEEP"),
        "POISON" => Some("POISON"),
        "TOXIC" => Some("BAD_POISON"),
        "PARALYZE" => Some("PARALYSIS"),
        _ => None,
    }
}

fn direct_splash_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SPLASH"
}

fn direct_teleport_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "TELEPORT"
}

fn direct_confusion_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "CONFUSE"
}

fn direct_swagger_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SWAGGER"
}

fn direct_heal_effect(move_data: &Move) -> bool {
    move_data.power == 0
        && matches!(
            move_data.effect.as_str(),
            "HEAL" | "MOONLIGHT" | "MORNING_SUN" | "SYNTHESIS"
        )
}

fn direct_heal_bell_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "HEAL_BELL"
}

fn direct_pain_split_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "PAIN_SPLIT"
}

fn direct_perish_song_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "PERISH_SONG"
}

fn direct_focus_energy_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "FOCUS_ENERGY"
}

fn direct_belly_drum_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "BELLY_DRUM"
}

fn direct_defense_curl_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "DEFENSE_CURL"
}

fn direct_curse_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "CURSE"
}

fn direct_mist_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "MIST"
}

fn direct_safeguard_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SAFEGUARD"
}

fn direct_substitute_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SUBSTITUTE"
}

fn direct_reflect_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "REFLECT"
}

fn direct_light_screen_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "LIGHT_SCREEN"
}

fn direct_destiny_bond_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "DESTINY_BOND"
}

fn direct_sleep_talk_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SLEEP_TALK"
}

fn direct_mirror_move_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "MIRROR_MOVE"
}

fn direct_metronome_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "METRONOME"
}

fn direct_mimic_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "MIMIC"
}

fn direct_sketch_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SKETCH"
}

fn direct_conversion_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "CONVERSION"
}

fn direct_conversion2_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "CONVERSION2"
}

fn direct_bide_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "BIDE"
}

fn direct_encore_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "ENCORE"
}

fn direct_leech_seed_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "LEECH_SEED"
}

fn direct_nightmare_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "NIGHTMARE"
}

fn direct_force_switch_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "FORCE_SWITCH"
}

fn direct_spikes_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SPIKES"
}

fn direct_escape_trap_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "MEAN_LOOK"
}

fn direct_lock_on_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "LOCK_ON"
}

fn direct_attract_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "ATTRACT"
}

fn direct_disable_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "DISABLE"
}

fn direct_protect_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "PROTECT"
}

fn direct_endure_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "ENDURE"
}

fn direct_spite_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "SPITE"
}

fn direct_future_sight_effect(move_data: &Move) -> bool {
    move_data.effect == "FUTURE_SIGHT"
}

fn direct_transform_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "TRANSFORM"
}

fn direct_baton_pass_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "BATON_PASS"
}

fn direct_reset_stats_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "RESET_STATS"
}

fn direct_psych_up_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "PSYCH_UP"
}

fn direct_foresight_effect(move_data: &Move) -> bool {
    move_data.power == 0 && move_data.effect == "FORESIGHT"
}

fn direct_beat_up_effect(move_data: &Move) -> bool {
    move_data.effect == "BEAT_UP"
}

fn direct_weather_effect(move_data: &Move) -> Option<Weather> {
    if move_data.power != 0 {
        return None;
    }
    match move_data.effect.as_str() {
        "RAIN_DANCE" => Some(Weather::Rain),
        "SANDSTORM" => Some(Weather::Sandstorm),
        "SUNNY_DAY" => Some(Weather::Sun),
        _ => None,
    }
}

fn direct_airborne_effect(move_data: &Move) -> bool {
    move_data.effect == "FLY"
}

fn direct_charge_effect(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "RAZOR_WIND" | "SOLARBEAM" | "SKULL_BASH" | "SKY_ATTACK"
    )
}

fn charge_move_skips_charge(state: &BattleCombatState, move_data: &Move) -> bool {
    move_data.effect == "SOLARBEAM" && state.weather == Weather::Sun
}

fn move_hits_airborne_target(move_data: &Move, airborne_move: &str) -> bool {
    if airborne_move == "DIG" {
        return matches!(
            move_data.effect.as_str(),
            "EARTHQUAKE" | "GUST" | "THUNDER" | "TWISTER"
        );
    }
    matches!(
        move_data.effect.as_str(),
        "FLY" | "GUST" | "THUNDER" | "TWISTER"
    )
}

fn move_usable_while_asleep(move_data: &Move) -> bool {
    matches!(move_data.effect.as_str(), "SNORE" | "SLEEP_TALK")
}

fn move_thaws_user(move_data: &Move) -> bool {
    matches!(move_data.effect.as_str(), "FLAME_WHEEL" | "SACRED_FIRE")
}

fn counter_effect(move_data: &Move) -> Option<BattleDamageCategory> {
    match move_data.effect.as_str() {
        "COUNTER" => Some(BattleDamageCategory::Physical),
        "MIRROR_COAT" => Some(BattleDamageCategory::Special),
        _ => None,
    }
}

fn secondary_status_effect(move_data: &Move) -> Option<(&'static str, u8)> {
    if move_data.power == 0 || move_data.effect_chance == 0 {
        return None;
    }
    let status = match move_data.effect.as_str() {
        "BURN_HIT" | "FLAME_WHEEL" | "SACRED_FIRE" => "BURN",
        "FREEZE_HIT" => "FREEZE",
        "POISON_HIT" | "POISON_MULTI_HIT" => "POISON",
        "PARALYZE_HIT" => "PARALYSIS",
        _ => return None,
    };
    Some((status, move_data.effect_chance.min(100)))
}

fn secondary_confusion_effect(move_data: &Move) -> Option<u8> {
    if move_data.power == 0 || move_data.effect_chance == 0 {
        return None;
    }
    (move_data.effect == "CONFUSE_HIT").then_some(move_data.effect_chance.min(100))
}

fn secondary_flinch_effect(move_data: &Move) -> Option<u8> {
    if move_data.power == 0 || move_data.effect_chance == 0 {
        return None;
    }
    matches!(
        move_data.effect.as_str(),
        "FLINCH_HIT" | "SNORE" | "STOMP" | "TWISTER" | "SKY_ATTACK"
    )
    .then_some(move_data.effect_chance.min(100))
}

fn secondary_stat_hit_effect(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "ALL_UP_HIT"
            | "ATTACK_UP_HIT"
            | "ATTACK_DOWN_HIT"
            | "DEFENSE_UP_HIT"
            | "DEFENSE_DOWN_HIT"
            | "SPEED_DOWN_HIT"
            | "ACCURACY_DOWN_HIT"
            | "EVASION_DOWN_HIT"
    )
}

fn roll_critical_hit(attacker: &Pokemon, rng: &mut Random) -> (bool, u8, u8) {
    let threshold = if attacker.focus_energy { 32 } else { 17 };
    let roll = rng.randrange(256) as u8;
    (roll < threshold, roll, threshold)
}

fn move_hit_count(move_data: &Move, rng: &mut Random) -> (u8, Option<u8>) {
    match move_data.effect.as_str() {
        "DOUBLE_HIT" => (2, None),
        "MULTI_HIT" | "POISON_MULTI_HIT" => {
            let roll = rng.randrange(8) as u8;
            let hits = match roll {
                0..=2 => 2,
                3..=5 => 3,
                6 => 4,
                _ => 5,
            };
            (hits, Some(roll))
        }
        _ => (1, None),
    }
}

fn damage_move_data(
    side: BattleSide,
    move_name: &str,
    attacker: &Pokemon,
    move_data: &Move,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Move {
    let mut damage_move_data = move_data.clone();
    if move_data.effect == "HIDDEN_POWER" {
        let (move_type, power) = hidden_power_type_power(attacker);
        damage_move_data.move_type = move_type.clone();
        damage_move_data.power = power;
        events.push(BattleEvent::HiddenPowerResolved {
            side,
            move_name: move_name.to_string(),
            move_type,
            power,
        });
    } else {
        damage_move_data.power =
            dynamic_move_power(side, move_name, attacker, move_data, rng, events);
    }
    damage_move_data
}

fn apply_state_damage_power_modifiers(
    state: &BattleCombatState,
    side: BattleSide,
    target_switching: bool,
    move_data: &mut Move,
    events: &mut Vec<BattleEvent>,
) {
    if move_data.effect == "PURSUIT" && target_switching {
        move_data.power = move_data.power.saturating_mul(2);
        events.push(BattleEvent::PursuitPower {
            side,
            move_name: move_data.name.clone(),
            target: side.other(),
            power: move_data.power,
        });
    }
    if matches!(move_data.effect.as_str(), "GUST" | "TWISTER")
        && airborne_move_state(state, side.other()).is_some()
    {
        move_data.power = move_data.power.saturating_mul(2);
    }
    if move_data.effect == "EARTHQUAKE"
        && airborne_move_state(state, side.other())
            .is_some_and(|airborne_move| airborne_move == "DIG")
    {
        move_data.power = move_data.power.saturating_mul(2);
        events.push(BattleEvent::EarthquakePower {
            side,
            move_name: move_data.name.clone(),
            target_move: "DIG".to_string(),
            power: move_data.power,
        });
    }
    if move_data.effect == "FURY_CUTTER" {
        let chain = fury_cutter_chain(state, side).min(4);
        move_data.power = move_data.power.saturating_mul(1u16 << chain);
        events.push(BattleEvent::FuryCutterPower {
            side,
            move_name: move_data.name.clone(),
            chain,
            power: move_data.power,
        });
    }
    if move_data.effect == "ROLLOUT" {
        let chain = rollout_chain(state, side).min(4);
        let defense_curled = defense_curled(state, side);
        move_data.power = move_data.power.saturating_mul(1u16 << chain);
        if defense_curled {
            move_data.power = move_data.power.saturating_mul(2);
        }
        events.push(BattleEvent::RolloutPower {
            side,
            move_name: move_data.name.clone(),
            chain,
            defense_curled,
            power: move_data.power,
        });
    }
}

fn dynamic_move_power(
    side: BattleSide,
    move_name: &str,
    attacker: &Pokemon,
    move_data: &Move,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> u16 {
    match move_data.effect.as_str() {
        "RETURN" => return_power(attacker.happiness),
        "FRUSTRATION" => frustration_power(attacker.happiness),
        "REVERSAL" => reversal_power(attacker.hp, attacker.max_hp),
        "MAGNITUDE" => {
            let roll = rng.randrange(256) as u8;
            let power = magnitude_power(roll);
            events.push(BattleEvent::MagnitudePower {
                side,
                move_name: move_name.to_string(),
                roll,
                power,
            });
            power
        }
        _ => move_data.power,
    }
}

fn return_power(happiness: u8) -> u16 {
    ((happiness as u16 * 10) / 25).max(1)
}

fn frustration_power(happiness: u8) -> u16 {
    (((255 - happiness as u16) * 10) / 25).max(1)
}

fn reversal_power(hp: u16, max_hp: u16) -> u16 {
    let ratio = u32::from(hp) * 48 / u32::from(max_hp.max(1));
    match ratio {
        0..=1 => 200,
        2..=4 => 150,
        5..=9 => 100,
        10..=16 => 80,
        17..=32 => 40,
        _ => 20,
    }
}

fn magnitude_power(roll: u8) -> u16 {
    match roll {
        0..=12 => 10,
        13..=38 => 30,
        39..=89 => 50,
        90..=166 => 70,
        167..=217 => 90,
        218..=242 => 110,
        _ => 150,
    }
}

fn hidden_power_type_power(attacker: &Pokemon) -> (PokemonType, u16) {
    let attack = attacker.dvs.attack & 0x0f;
    let defense = attacker.dvs.defense & 0x0f;
    let speed = attacker.dvs.speed & 0x0f;
    let special = attacker.dvs.special & 0x0f;
    let type_index = (((attack & 0x03) << 2) | (defense & 0x03)) as usize;
    let power =
        ((((attack >> 3) + ((defense >> 3) << 1) + ((speed >> 3) << 2) + ((special >> 3) << 3))
            as u16
            * 5
            + u16::from(special & 0x03))
            / 2)
            + 31;
    (hidden_power_type(type_index), power.clamp(31, 70))
}

fn hidden_power_type(type_index: usize) -> PokemonType {
    const HIDDEN_POWER_TYPES: [&str; 16] = [
        "FIGHTING",
        "FLYING",
        "POISON",
        "GROUND",
        "ROCK",
        "BUG",
        "GHOST",
        "STEEL",
        "FIRE",
        "WATER",
        "GRASS",
        "ELECTRIC",
        "PSYCHIC_TYPE",
        "ICE",
        "DRAGON",
        "DARK",
    ];
    HIDDEN_POWER_TYPES[type_index].to_string()
}

fn fixed_damage_amount(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_data: &Move,
    rng: &mut Random,
) -> Option<u16> {
    match move_data.effect.as_str() {
        "STATIC_DAMAGE" => Some(move_data.power.max(1)),
        "LEVEL_DAMAGE" => Some(attacker.level.max(1) as u16),
        "SUPER_FANG" => Some((defender.hp / 2).max(1)),
        "PSYWAVE" => {
            let level = attacker.level.max(1) as u32;
            let roll = rng.randrange(256);
            Some(((level * 255 + level * roll * 2) / 510).max(1) as u16)
        }
        _ => None,
    }
}

fn apply_counter_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    type_effectiveness: &TypeEffectivenessTable,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let Some(required_category) = counter_effect(move_data) else {
        return Ok(());
    };
    let Some(last_damage) = last_damage_state(state, side).cloned() else {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    };
    if last_damage.source != side.other() || last_damage.category != required_category {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }

    let multiplier = calculate_type_effectiveness_multiplier_with_foresight(
        type_effectiveness,
        &move_data.move_type,
        &effective_pokemon_types(state, side.other()),
        identified_state(state, side.other()),
    )
    .map_err(BattleTurnError::DamageCalculation)?;
    if multiplier.numerator == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }

    let target = side.other();
    let defender_hp_before = state.pokemon(target).hp;
    let damage = last_damage.damage.saturating_mul(2).min(defender_hp_before);
    if damage == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let defender = state.pokemon_mut(target);
    defender.hp = defender.hp.saturating_sub(damage);
    let defender_hp_after = defender.hp;
    events.push(BattleEvent::CounterDamage {
        side,
        move_name: move_name.to_string(),
        target,
        countered_move: last_damage.move_name,
        category: required_category,
        source_damage: last_damage.damage,
        damage,
        defender_hp_before,
        defender_hp_after,
    });
    if defender_hp_after == 0 {
        events.push(BattleEvent::Fainted { side: target });
    }
    Ok(())
}

fn apply_substitute_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    if substitute_hp(state, side) > 0 {
        events.push(BattleEvent::SubstituteFailed {
            side,
            move_name: move_name.to_string(),
            reason: "already_active".to_string(),
        });
        return;
    }
    let hp_before = state.pokemon(side).hp;
    let hp_cost = (state.pokemon(side).max_hp / 4).max(1);
    if hp_before <= hp_cost {
        events.push(BattleEvent::SubstituteFailed {
            side,
            move_name: move_name.to_string(),
            reason: "insufficient_hp".to_string(),
        });
        return;
    }
    let pokemon = state.pokemon_mut(side);
    pokemon.hp -= hp_cost;
    let hp_after = pokemon.hp;
    set_substitute_hp(state, side, hp_cost);
    events.push(BattleEvent::SubstituteCreated {
        side,
        move_name: move_name.to_string(),
        hp_cost,
        substitute_hp: hp_cost,
        hp_before,
        hp_after,
    });
}

fn apply_substitute_damage(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    damage: u16,
    events: &mut Vec<BattleEvent>,
) -> Option<DamageHitResult> {
    let target = side.other();
    let substitute_hp_before = substitute_hp(state, target);
    if damage == 0 || substitute_hp_before == 0 {
        return None;
    }
    let substitute_damage = damage.min(substitute_hp_before);
    let substitute_hp_after = substitute_hp_before - substitute_damage;
    set_substitute_hp(state, target, substitute_hp_after);
    events.push(BattleEvent::SubstituteDamaged {
        side,
        move_name: move_name.to_string(),
        target,
        damage: substitute_damage,
        substitute_hp_before,
        substitute_hp_after,
    });
    if substitute_hp_after == 0 {
        events.push(BattleEvent::SubstituteBroken {
            side,
            move_name: move_name.to_string(),
            target,
        });
    }
    Some(DamageHitResult::Continue)
}

fn apply_post_damage_hp_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    damage: u16,
    events: &mut Vec<BattleEvent>,
) {
    if damage == 0 {
        return;
    }
    match move_data.effect.as_str() {
        "DREAM_EATER" | "LEECH_HIT" => apply_drain_effect(state, side, move_name, damage, events),
        "RECOIL_HIT" => apply_recoil_effect(state, side, move_name, damage, events),
        "SELFDESTRUCT" => apply_selfdestruct_effect(state, side, move_name, events),
        "PAY_DAY" => apply_pay_day_effect(state, side, move_name, events),
        "THIEF" => apply_thief_effect(state, side, move_name, events),
        _ => {}
    }
}

fn apply_post_damage_stat_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    damage: u16,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    if damage == 0 {
        return Ok(false);
    }
    match move_data.effect.as_str() {
        "ALL_UP_HIT" => {
            if !roll_secondary_effect_chance(move_data, rng) {
                return Ok(true);
            }
            for stat in [
                Stat::Attack,
                Stat::Defense,
                Stat::Speed,
                Stat::SpecialAttack,
                Stat::SpecialDefense,
            ] {
                apply_stat_stage_delta(state, side, move_name, stat, 1, events)?;
            }
            Ok(true)
        }
        _ if secondary_stat_hit_effect(move_data) => {
            apply_secondary_stat_stage_effect(state, side, move_name, move_data, rng, events)?;
            Ok(true)
        }
        "RAPID_SPIN" => {
            apply_rapid_spin_effect(state, side, move_name, events);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn roll_secondary_effect_chance(move_data: &Move, rng: &mut Random) -> bool {
    let chance_percent = move_data.effect_chance.min(100);
    if chance_percent == 0 || chance_percent == 100 {
        return true;
    }
    (rng.randrange(100) as u8) < chance_percent
}

fn apply_rapid_spin_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let cleared_trap = trap_state(state, side).is_some();
    let cleared_leech_seed = leech_seed_source(state, side).is_some();
    let cleared_spikes = spikes_state(state, side);
    clear_trap_state(state, side);
    set_leech_seed_source(state, side, None);
    set_spikes_state(state, side, false);
    events.push(BattleEvent::RapidSpinCleared {
        side,
        move_name: move_name.to_string(),
        cleared_trap,
        cleared_leech_seed,
        cleared_spikes,
    });
}

fn apply_pay_day_effect(
    state: &BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let amount = u32::from(state.pokemon(side).level) * 5;
    events.push(BattleEvent::PayDayMoney {
        side,
        move_name: move_name.to_string(),
        amount,
    });
}

fn apply_thief_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if state.pokemon(side).item.is_some() {
        events.push(BattleEvent::HeldItemStealFailed {
            side,
            move_name: move_name.to_string(),
            target,
            reason: "attacker_already_holds_item".to_string(),
        });
        return;
    }
    let Some(item_id) = state.pokemon_mut(target).item.take() else {
        events.push(BattleEvent::HeldItemStealFailed {
            side,
            move_name: move_name.to_string(),
            target,
            reason: "target_has_no_item".to_string(),
        });
        return;
    };
    state.pokemon_mut(side).item = Some(item_id.clone());
    events.push(BattleEvent::HeldItemStolen {
        side,
        move_name: move_name.to_string(),
        target,
        item_id,
    });
}

fn apply_drain_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    damage: u16,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon_mut(side);
    if pokemon.hp >= pokemon.max_hp {
        return;
    }
    let hp_before = pokemon.hp;
    let amount = damage.div_ceil(2).min(pokemon.max_hp - pokemon.hp);
    pokemon.hp += amount;
    events.push(BattleEvent::HpDrained {
        side,
        move_name: move_name.to_string(),
        target: side.other(),
        damage,
        hp_before,
        hp_after: pokemon.hp,
        amount,
    });
}

fn apply_recoil_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    damage: u16,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon_mut(side);
    if pokemon.hp == 0 {
        return;
    }
    let hp_before = pokemon.hp;
    let recoil_damage = (damage / 4).max(1).min(pokemon.hp);
    pokemon.hp = pokemon.hp.saturating_sub(recoil_damage);
    events.push(BattleEvent::RecoilDamage {
        side,
        move_name: move_name.to_string(),
        damage_dealt: damage,
        recoil_damage,
        hp_before,
        hp_after: pokemon.hp,
    });
}

fn apply_jump_kick_crash_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    events: &mut Vec<BattleEvent>,
) {
    if move_data.effect != "JUMP_KICK" {
        return;
    }
    let pokemon = state.pokemon_mut(side);
    if pokemon.hp == 0 {
        return;
    }
    let hp_before = pokemon.hp;
    let crash_damage = (pokemon.max_hp / 2).max(1).min(pokemon.hp);
    pokemon.hp = pokemon.hp.saturating_sub(crash_damage);
    events.push(BattleEvent::JumpKickCrash {
        side,
        move_name: move_name.to_string(),
        crash_damage,
        hp_before,
        hp_after: pokemon.hp,
    });
    if pokemon.hp == 0 {
        events.push(BattleEvent::Fainted { side });
    }
}

fn apply_rampage_progress(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    rampage_forced: bool,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    if move_data.effect != "RAMPAGE" {
        state.pokemon_mut(side).rampage_turns = 0;
        return;
    }
    if state.pokemon(side).hp == 0 {
        state.pokemon_mut(side).rampage_turns = 0;
        return;
    }
    if rampage_forced {
        let turns_remaining = state.pokemon(side).rampage_turns.saturating_sub(1);
        state.pokemon_mut(side).rampage_turns = turns_remaining;
        if turns_remaining == 0 {
            events.push(BattleEvent::RampageEnded {
                side,
                move_name: move_name.to_string(),
            });
            apply_confusion_to_side(state, side, move_name, rng, events);
        }
        return;
    }
    let roll = rng.randrange(2) as u8;
    let turns_remaining = 1 + roll;
    state.pokemon_mut(side).rampage_turns = turns_remaining;
    events.push(BattleEvent::RampageStarted {
        side,
        move_name: move_name.to_string(),
        turns_remaining,
        roll,
    });
}

fn apply_fury_cutter_progress(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    events: &[BattleEvent],
) {
    if move_data.effect != "FURY_CUTTER" {
        reset_fury_cutter_chain(state, side);
        return;
    }
    let dealt_damage = events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::Damage {
                side: damage_side,
                move_name: damage_move,
                damage,
                ..
            } if *damage_side == side && damage_move == move_name && *damage != 0
        )
    });
    if dealt_damage {
        let next_chain = fury_cutter_chain(state, side).saturating_add(1).min(4);
        set_fury_cutter_chain(state, side, next_chain);
    } else {
        reset_fury_cutter_chain(state, side);
    }
}

fn apply_rollout_progress(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    rollout_forced: bool,
    events: &[BattleEvent],
) {
    if move_data.effect != "ROLLOUT" {
        reset_rollout_state(state, side);
        return;
    }
    let dealt_damage = events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::Damage {
                side: damage_side,
                move_name: damage_move,
                damage,
                ..
            } if *damage_side == side && damage_move == move_name && *damage != 0
        )
    });
    if !dealt_damage {
        reset_rollout_state(state, side);
        return;
    }
    if rollout_forced {
        let turns_remaining = rollout_turns(state, side).saturating_sub(1);
        set_rollout_turns(state, side, turns_remaining);
        if turns_remaining == 0 {
            set_rollout_chain(state, side, 0);
        } else {
            let next_chain = rollout_chain(state, side).saturating_add(1).min(4);
            set_rollout_chain(state, side, next_chain);
        }
    } else {
        set_rollout_turns(state, side, 4);
        set_rollout_chain(state, side, 1);
    }
}

fn apply_rage_damage_boost(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if rage_active(state, side) && state.pokemon(side).hp != 0 {
        apply_stat_stage_delta_to_target(state, side, "RAGE", side, Stat::Attack, 1, events)?;
    }
    Ok(())
}

fn apply_bide_damage_storage(
    state: &mut BattleCombatState,
    source: BattleSide,
    target: BattleSide,
    damage: u16,
    events: &mut Vec<BattleEvent>,
) {
    if bide_turns(state, target) == 0 || damage == 0 || state.pokemon(target).hp == 0 {
        return;
    }
    let stored_damage = bide_damage(state, target).saturating_add(damage);
    set_bide_damage(state, target, stored_damage);
    events.push(BattleEvent::BideStoredDamage {
        side: target,
        source,
        damage,
        stored_damage,
    });
}

fn apply_selfdestruct_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon_mut(side);
    if pokemon.hp == 0 {
        return;
    }
    let hp_before = pokemon.hp;
    pokemon.hp = 0;
    events.push(BattleEvent::SelfdestructDamage {
        side,
        move_name: move_name.to_string(),
        hp_before,
    });
}

fn confusion_damage_move() -> Move {
    Move {
        name: "CONFUSION_DAMAGE".to_string(),
        move_type: "NORMAL".to_string(),
        power: 40,
        accuracy: 100,
        pp: 1,
        effect: "NORMAL_HIT".to_string(),
        effect_chance: 0,
        stat: None,
        amount: None,
    }
}

fn apply_stat_stage_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let (Some(stat), Some(amount)) = (move_data.stat, move_data.amount) else {
        return Ok(false);
    };
    if amount == 0 {
        return Ok(false);
    }
    apply_stat_stage_delta(state, side, move_name, stat, amount, events)?;
    Ok(true)
}

fn apply_stat_stage_delta(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    stat: Stat,
    amount: i8,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let target = stat_effect_target(side, amount);
    apply_stat_stage_delta_to_target(state, side, move_name, target, stat, amount, events)
}

fn apply_stat_stage_delta_to_target(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target: BattleSide,
    stat: Stat,
    amount: i8,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let mist_turns = mist_turns(state, target);
    if amount < 0 && target != side && mist_turns > 0 {
        events.push(BattleEvent::MistProtected {
            side,
            move_name: move_name.to_string(),
            target,
            stat,
            amount,
            turns_remaining: mist_turns,
        });
        return Ok(());
    }
    let pokemon = state.pokemon_mut(target);
    let stage_before = *pokemon
        .stat_boosts
        .get(&stat)
        .ok_or(BattleTurnError::MissingStatStage { side: target, stat })?;
    let stage_after = (stage_before + amount).clamp(-6, 6);
    if stage_after == stage_before {
        events.push(BattleEvent::StatStageUnchanged {
            side,
            move_name: move_name.to_string(),
            target,
            stat,
            amount,
            stage: stage_before,
        });
    } else {
        pokemon.stat_boosts.insert(stat, stage_after);
        events.push(BattleEvent::StatStageChanged {
            side,
            move_name: move_name.to_string(),
            target,
            stat,
            amount,
            stage_before,
            stage_after,
        });
    }
    Ok(())
}

fn stat_effect_target(side: BattleSide, amount: i8) -> BattleSide {
    if amount > 0 { side } else { side.other() }
}

fn mist_turns(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_mist_turns,
        BattleSide::Enemy => state.enemy_mist_turns,
    }
}

fn set_mist_turns(state: &mut BattleCombatState, side: BattleSide, turns: u8) {
    match side {
        BattleSide::Player => state.player_mist_turns = turns,
        BattleSide::Enemy => state.enemy_mist_turns = turns,
    }
}

fn safeguard_turns(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_safeguard_turns,
        BattleSide::Enemy => state.enemy_safeguard_turns,
    }
}

fn set_safeguard_turns(state: &mut BattleCombatState, side: BattleSide, turns: u8) {
    match side {
        BattleSide::Player => state.player_safeguard_turns = turns,
        BattleSide::Enemy => state.enemy_safeguard_turns = turns,
    }
}

fn screen_turns(state: &BattleCombatState, side: BattleSide, screen: BattleScreen) -> u8 {
    match (side, screen) {
        (BattleSide::Player, BattleScreen::Reflect) => state.player_reflect_turns,
        (BattleSide::Enemy, BattleScreen::Reflect) => state.enemy_reflect_turns,
        (BattleSide::Player, BattleScreen::LightScreen) => state.player_light_screen_turns,
        (BattleSide::Enemy, BattleScreen::LightScreen) => state.enemy_light_screen_turns,
    }
}

fn set_screen_turns(
    state: &mut BattleCombatState,
    side: BattleSide,
    screen: BattleScreen,
    turns: u8,
) {
    match (side, screen) {
        (BattleSide::Player, BattleScreen::Reflect) => state.player_reflect_turns = turns,
        (BattleSide::Enemy, BattleScreen::Reflect) => state.enemy_reflect_turns = turns,
        (BattleSide::Player, BattleScreen::LightScreen) => state.player_light_screen_turns = turns,
        (BattleSide::Enemy, BattleScreen::LightScreen) => state.enemy_light_screen_turns = turns,
    }
}

fn destiny_bond_active(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_destiny_bond_active,
        BattleSide::Enemy => state.enemy_destiny_bond_active,
    }
}

fn set_destiny_bond_active(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_destiny_bond_active = active,
        BattleSide::Enemy => state.enemy_destiny_bond_active = active,
    }
}

fn leech_seed_source(state: &BattleCombatState, side: BattleSide) -> Option<BattleSide> {
    match side {
        BattleSide::Player => state.player_leech_seed_source,
        BattleSide::Enemy => state.enemy_leech_seed_source,
    }
}

fn set_leech_seed_source(
    state: &mut BattleCombatState,
    side: BattleSide,
    source: Option<BattleSide>,
) {
    match side {
        BattleSide::Player => state.player_leech_seed_source = source,
        BattleSide::Enemy => state.enemy_leech_seed_source = source,
    }
}

fn curse_source(state: &BattleCombatState, side: BattleSide) -> Option<BattleSide> {
    match side {
        BattleSide::Player => state.player_curse_source,
        BattleSide::Enemy => state.enemy_curse_source,
    }
}

fn set_curse_source(state: &mut BattleCombatState, side: BattleSide, source: Option<BattleSide>) {
    match side {
        BattleSide::Player => state.player_curse_source = source,
        BattleSide::Enemy => state.enemy_curse_source = source,
    }
}

fn spikes_state(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_spikes,
        BattleSide::Enemy => state.enemy_spikes,
    }
}

fn set_spikes_state(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_spikes = active,
        BattleSide::Enemy => state.enemy_spikes = active,
    }
}

fn toxic_turns(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_toxic_turns,
        BattleSide::Enemy => state.enemy_toxic_turns,
    }
}

fn set_toxic_turns(state: &mut BattleCombatState, side: BattleSide, turns: u8) {
    match side {
        BattleSide::Player => state.player_toxic_turns = turns,
        BattleSide::Enemy => state.enemy_toxic_turns = turns,
    }
}

fn nightmare_source(state: &BattleCombatState, side: BattleSide) -> Option<BattleSide> {
    match side {
        BattleSide::Player => state.player_nightmare_source,
        BattleSide::Enemy => state.enemy_nightmare_source,
    }
}

fn set_nightmare_source(
    state: &mut BattleCombatState,
    side: BattleSide,
    source: Option<BattleSide>,
) {
    match side {
        BattleSide::Player => state.player_nightmare_source = source,
        BattleSide::Enemy => state.enemy_nightmare_source = source,
    }
}

fn clear_nightmares_sourced_by(state: &mut BattleCombatState, source: BattleSide) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if nightmare_source(state, side) == Some(source) {
            set_nightmare_source(state, side, None);
        }
    }
}

fn trap_state(state: &BattleCombatState, side: BattleSide) -> Option<&BattleTrapState> {
    match side {
        BattleSide::Player => state.player_trap.as_ref(),
        BattleSide::Enemy => state.enemy_trap.as_ref(),
    }
}

fn set_trap_state(state: &mut BattleCombatState, side: BattleSide, trap: Option<BattleTrapState>) {
    match side {
        BattleSide::Player => state.player_trap = trap,
        BattleSide::Enemy => state.enemy_trap = trap,
    }
}

fn clear_trap_state(state: &mut BattleCombatState, side: BattleSide) {
    set_trap_state(state, side, None);
}

fn clear_traps_sourced_by(state: &mut BattleCombatState, source: BattleSide) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if trap_state(state, side).is_some_and(|trap| trap.source == source) {
            clear_trap_state(state, side);
        }
    }
}

fn escape_trap_state(
    state: &BattleCombatState,
    side: BattleSide,
) -> Option<&BattleEscapeTrapState> {
    match side {
        BattleSide::Player => state.player_escape_trap.as_ref(),
        BattleSide::Enemy => state.enemy_escape_trap.as_ref(),
    }
}

fn set_escape_trap_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    trap: Option<BattleEscapeTrapState>,
) {
    match side {
        BattleSide::Player => state.player_escape_trap = trap,
        BattleSide::Enemy => state.enemy_escape_trap = trap,
    }
}

fn clear_escape_trap_state(state: &mut BattleCombatState, side: BattleSide) {
    set_escape_trap_state(state, side, None);
}

fn clear_escape_traps_sourced_by(state: &mut BattleCombatState, source: BattleSide) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if escape_trap_state(state, side).is_some_and(|trap| trap.source == source) {
            clear_escape_trap_state(state, side);
        }
    }
}

fn last_move(state: &BattleCombatState, side: BattleSide) -> Option<&str> {
    match side {
        BattleSide::Player => state.player_last_move.as_deref(),
        BattleSide::Enemy => state.enemy_last_move.as_deref(),
    }
}

fn set_last_move(state: &mut BattleCombatState, side: BattleSide, move_name: Option<String>) {
    match side {
        BattleSide::Player => state.player_last_move = move_name,
        BattleSide::Enemy => state.enemy_last_move = move_name,
    }
}

fn encore_state(state: &BattleCombatState, side: BattleSide) -> Option<&BattleEncoreState> {
    match side {
        BattleSide::Player => state.player_encore.as_ref(),
        BattleSide::Enemy => state.enemy_encore.as_ref(),
    }
}

fn set_encore_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    encore: Option<BattleEncoreState>,
) {
    match side {
        BattleSide::Player => state.player_encore = encore,
        BattleSide::Enemy => state.enemy_encore = encore,
    }
}

fn clear_encore_state(state: &mut BattleCombatState, side: BattleSide) {
    set_encore_state(state, side, None);
}

fn disable_state(state: &BattleCombatState, side: BattleSide) -> Option<&BattleDisableState> {
    match side {
        BattleSide::Player => state.player_disable.as_ref(),
        BattleSide::Enemy => state.enemy_disable.as_ref(),
    }
}

fn set_disable_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    disable: Option<BattleDisableState>,
) {
    match side {
        BattleSide::Player => state.player_disable = disable,
        BattleSide::Enemy => state.enemy_disable = disable,
    }
}

fn clear_disable_state(state: &mut BattleCombatState, side: BattleSide) {
    set_disable_state(state, side, None);
}

fn fury_cutter_chain(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_fury_cutter_chain,
        BattleSide::Enemy => state.enemy_fury_cutter_chain,
    }
}

fn set_fury_cutter_chain(state: &mut BattleCombatState, side: BattleSide, chain: u8) {
    match side {
        BattleSide::Player => state.player_fury_cutter_chain = chain,
        BattleSide::Enemy => state.enemy_fury_cutter_chain = chain,
    }
}

fn reset_fury_cutter_chain(state: &mut BattleCombatState, side: BattleSide) {
    set_fury_cutter_chain(state, side, 0);
}

fn rollout_chain(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_rollout_chain,
        BattleSide::Enemy => state.enemy_rollout_chain,
    }
}

fn set_rollout_chain(state: &mut BattleCombatState, side: BattleSide, chain: u8) {
    match side {
        BattleSide::Player => state.player_rollout_chain = chain,
        BattleSide::Enemy => state.enemy_rollout_chain = chain,
    }
}

fn rollout_turns(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_rollout_turns,
        BattleSide::Enemy => state.enemy_rollout_turns,
    }
}

fn set_rollout_turns(state: &mut BattleCombatState, side: BattleSide, turns: u8) {
    match side {
        BattleSide::Player => state.player_rollout_turns = turns,
        BattleSide::Enemy => state.enemy_rollout_turns = turns,
    }
}

fn reset_rollout_state(state: &mut BattleCombatState, side: BattleSide) {
    set_rollout_chain(state, side, 0);
    set_rollout_turns(state, side, 0);
}

fn defense_curled(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_defense_curled,
        BattleSide::Enemy => state.enemy_defense_curled,
    }
}

fn set_defense_curled(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_defense_curled = active,
        BattleSide::Enemy => state.enemy_defense_curled = active,
    }
}

fn rage_active(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_rage_active,
        BattleSide::Enemy => state.enemy_rage_active,
    }
}

fn set_rage_active(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_rage_active = active,
        BattleSide::Enemy => state.enemy_rage_active = active,
    }
}

fn bide_turns(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_bide_turns,
        BattleSide::Enemy => state.enemy_bide_turns,
    }
}

fn set_bide_turns(state: &mut BattleCombatState, side: BattleSide, turns: u8) {
    match side {
        BattleSide::Player => state.player_bide_turns = turns,
        BattleSide::Enemy => state.enemy_bide_turns = turns,
    }
}

fn bide_damage(state: &BattleCombatState, side: BattleSide) -> u16 {
    match side {
        BattleSide::Player => state.player_bide_damage,
        BattleSide::Enemy => state.enemy_bide_damage,
    }
}

fn set_bide_damage(state: &mut BattleCombatState, side: BattleSide, damage: u16) {
    match side {
        BattleSide::Player => state.player_bide_damage = damage,
        BattleSide::Enemy => state.enemy_bide_damage = damage,
    }
}

fn reset_bide_state(state: &mut BattleCombatState, side: BattleSide) {
    set_bide_turns(state, side, 0);
    set_bide_damage(state, side, 0);
}

fn future_sight_state(
    state: &BattleCombatState,
    side: BattleSide,
) -> Option<&BattleFutureSightState> {
    match side {
        BattleSide::Player => state.player_future_sight.as_ref(),
        BattleSide::Enemy => state.enemy_future_sight.as_ref(),
    }
}

fn set_future_sight_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    future_sight: Option<BattleFutureSightState>,
) {
    match side {
        BattleSide::Player => state.player_future_sight = future_sight,
        BattleSide::Enemy => state.enemy_future_sight = future_sight,
    }
}

fn protect_active(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_protect_active,
        BattleSide::Enemy => state.enemy_protect_active,
    }
}

fn set_protect_active(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_protect_active = active,
        BattleSide::Enemy => state.enemy_protect_active = active,
    }
}

fn endure_active(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_endure_active,
        BattleSide::Enemy => state.enemy_endure_active,
    }
}

fn set_endure_active(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_endure_active = active,
        BattleSide::Enemy => state.enemy_endure_active = active,
    }
}

fn substitute_hp(state: &BattleCombatState, side: BattleSide) -> u16 {
    match side {
        BattleSide::Player => state.player_substitute_hp,
        BattleSide::Enemy => state.enemy_substitute_hp,
    }
}

fn set_substitute_hp(state: &mut BattleCombatState, side: BattleSide, hp: u16) {
    match side {
        BattleSide::Player => state.player_substitute_hp = hp,
        BattleSide::Enemy => state.enemy_substitute_hp = hp,
    }
}

fn protect_counter(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_protect_counter,
        BattleSide::Enemy => state.enemy_protect_counter,
    }
}

fn set_protect_counter(state: &mut BattleCombatState, side: BattleSide, counter: u8) {
    match side {
        BattleSide::Player => state.player_protect_counter = counter,
        BattleSide::Enemy => state.enemy_protect_counter = counter,
    }
}

fn reset_protect_counter(state: &mut BattleCombatState, side: BattleSide) {
    set_protect_counter(state, side, 0);
}

fn is_protect_counter_move(move_name: &str) -> bool {
    matches!(move_name, "PROTECT" | "DETECT" | "ENDURE")
}

fn clear_end_turn_protect_endure(state: &mut BattleCombatState) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if !last_move(state, side).is_some_and(is_protect_counter_move) {
            reset_protect_counter(state, side);
        }
        set_protect_active(state, side, false);
        set_endure_active(state, side, false);
    }
}

fn clear_side_volatile_conditions(state: &mut BattleCombatState, side: BattleSide) {
    set_leech_seed_source(state, side, None);
    set_curse_source(state, side, None);
    set_nightmare_source(state, side, None);
    clear_nightmares_sourced_by(state, side);
    clear_trap_state(state, side);
    clear_traps_sourced_by(state, side);
    clear_escape_trap_state(state, side);
    clear_escape_traps_sourced_by(state, side);
    set_lock_on_target_state(state, side, false);
    set_lock_on_target_state(state, side.other(), false);
    set_attracted_by_state(state, side, None);
    clear_attracted_by_source(state, side);
    set_recharge_move_state(state, side, None);
    set_airborne_move_state(state, side, None);
    set_charging_move_state(state, side, None);
    set_destiny_bond_active(state, side, false);
    set_mist_turns(state, side, 0);
    set_safeguard_turns(state, side, 0);
    set_toxic_turns(state, side, 0);
    set_last_move(state, side, None);
    clear_encore_state(state, side);
    clear_disable_state(state, side);
    set_protect_active(state, side, false);
    set_endure_active(state, side, false);
    set_substitute_hp(state, side, 0);
    reset_protect_counter(state, side);
    reset_fury_cutter_chain(state, side);
    reset_rollout_state(state, side);
    set_defense_curled(state, side, false);
    set_rage_active(state, side, false);
    reset_bide_state(state, side);
    set_identified(state, side, false);
    set_type_override(state, side, None);
    set_transform_state(state, side, None);
    let pokemon = state.pokemon_mut(side);
    pokemon.flinching = false;
    pokemon.confusion_turns = 0;
    pokemon.perish_song_turns = 0;
    pokemon.focus_energy = false;
    pokemon.rampage_turns = 0;
}

fn clear_baton_pass_non_passable_conditions(state: &mut BattleCombatState, side: BattleSide) {
    clear_nightmares_sourced_by(state, side);
    clear_traps_sourced_by(state, side);
    clear_escape_traps_sourced_by(state, side);
    set_recharge_move_state(state, side, None);
    set_airborne_move_state(state, side, None);
    set_charging_move_state(state, side, None);
    set_destiny_bond_active(state, side, false);
    set_last_move(state, side, None);
    clear_encore_state(state, side);
    clear_disable_state(state, side);
    set_protect_active(state, side, false);
    set_endure_active(state, side, false);
    set_substitute_hp(state, side, 0);
    reset_protect_counter(state, side);
    reset_fury_cutter_chain(state, side);
    reset_rollout_state(state, side);
    set_rage_active(state, side, false);
    reset_bide_state(state, side);
    set_type_override(state, side, None);
    set_transform_state(state, side, None);
    let pokemon = state.pokemon_mut(side);
    pokemon.flinching = false;
    pokemon.rampage_turns = 0;
}

fn apply_secondary_stat_stage_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let (Some(stat), Some(amount)) = (move_data.stat, move_data.amount) else {
        return Ok(false);
    };
    if move_data.power == 0 || amount == 0 {
        return Ok(false);
    }
    if !roll_secondary_stat_stage_effect(side, move_name, move_data, rng, events)? {
        return Ok(true);
    }
    apply_stat_stage_effect(state, side, move_name, move_data, events)
}

fn roll_secondary_stat_stage_effect(
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let (Some(stat), Some(amount)) = (move_data.stat, move_data.amount) else {
        return Ok(false);
    };
    if amount == 0 {
        return Ok(false);
    }
    let chance_percent = move_data.effect_chance.min(100);
    if chance_percent != 0 && chance_percent != 100 {
        let roll = rng.randrange(100) as u8;
        if roll >= chance_percent {
            events.push(BattleEvent::SecondaryStatStageMissed {
                side,
                move_name: move_name.to_string(),
                target: stat_effect_target(side, amount),
                stat,
                amount,
                chance_percent,
                roll,
            });
            return Ok(false);
        }
    }
    Ok(true)
}

fn apply_secondary_status_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    status: &str,
    chance_percent: u8,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let roll = rng.randrange(100) as u8;
    let target = side.other();
    if roll >= chance_percent {
        events.push(BattleEvent::SecondaryStatusMissed {
            side,
            move_name: move_name.to_string(),
            target,
            status: status.to_string(),
            chance_percent,
            roll,
        });
        return;
    }

    apply_secondary_status_after_success(state, side, move_name, target, status, rng, events);
}

fn apply_tri_attack_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let chance_percent = move_data.effect_chance.min(100);
    let chance_roll = rng.randrange(100) as u8;
    let target = side.other();
    if chance_roll >= chance_percent {
        events.push(BattleEvent::SecondaryStatusMissed {
            side,
            move_name: move_name.to_string(),
            target,
            status: "TRI_ATTACK".to_string(),
            chance_percent,
            roll: chance_roll,
        });
        return;
    }

    let status = match rng.randrange(3) {
        0 => "BURN",
        1 => "FREEZE",
        _ => "PARALYSIS",
    };
    apply_secondary_status_after_success(state, side, move_name, target, status, rng, events);
}

fn apply_secondary_status_after_success(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target: BattleSide,
    status: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    if move_blocked_by_safeguard(state, side, move_name, target, status, events) {
        return;
    }
    let target_types = effective_pokemon_types(state, target);
    let defender = state.pokemon_mut(target);
    if defender.status.is_none() {
        apply_status_to_target(
            defender,
            &target_types,
            side,
            move_name,
            target,
            status,
            rng,
            events,
        );
    } else {
        events.push(BattleEvent::StatusFailed {
            side,
            move_name: move_name.to_string(),
            target,
            existing_status: defender.status.clone(),
        });
    }
}

fn apply_secondary_flinch_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    chance_percent: u8,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let roll = rng.randrange(100) as u8;
    let target = side.other();
    if roll >= chance_percent {
        events.push(BattleEvent::SecondaryFlinchMissed {
            side,
            move_name: move_name.to_string(),
            target,
            chance_percent,
            roll,
        });
        return;
    }

    state.pokemon_mut(target).flinching = true;
    events.push(BattleEvent::FlinchApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_confusion_to_target(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if move_blocked_by_safeguard(state, side, move_name, target, "CONFUSION", events) {
        return;
    }
    let pokemon = state.pokemon_mut(target);
    if pokemon.confusion_turns != 0 {
        events.push(BattleEvent::ConfusionFailed {
            side,
            move_name: move_name.to_string(),
            target,
            turns_remaining: pokemon.confusion_turns,
        });
        return;
    }
    pokemon.confusion_turns = 2 + rng.randrange(4) as u8;
    events.push(BattleEvent::ConfusionApplied {
        side,
        move_name: move_name.to_string(),
        target,
        turns: pokemon.confusion_turns,
    });
}

fn apply_confusion_to_side(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon_mut(side);
    if pokemon.confusion_turns != 0 {
        events.push(BattleEvent::ConfusionFailed {
            side,
            move_name: move_name.to_string(),
            target: side,
            turns_remaining: pokemon.confusion_turns,
        });
        return;
    }
    pokemon.confusion_turns = 2 + rng.randrange(4) as u8;
    events.push(BattleEvent::ConfusionApplied {
        side,
        move_name: move_name.to_string(),
        target: side,
        turns: pokemon.confusion_turns,
    });
}

fn apply_swagger_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let target = side.other();
    apply_stat_stage_delta_to_target(state, side, move_name, target, Stat::Attack, 2, events)?;
    apply_confusion_to_target(state, side, move_name, rng, events);
    Ok(())
}

fn move_blocked_by_safeguard(
    state: &BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target: BattleSide,
    effect: &str,
    events: &mut Vec<BattleEvent>,
) -> bool {
    let turns_remaining = safeguard_turns(state, target);
    if turns_remaining == 0 {
        return false;
    }
    events.push(BattleEvent::SafeguardProtected {
        side,
        move_name: move_name.to_string(),
        target,
        effect: effect.to_string(),
        turns_remaining,
    });
    true
}

fn apply_secondary_confusion_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    chance_percent: u8,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let roll = rng.randrange(100) as u8;
    let target = side.other();
    if roll >= chance_percent {
        events.push(BattleEvent::SecondaryConfusionMissed {
            side,
            move_name: move_name.to_string(),
            target,
            chance_percent,
            roll,
        });
        return;
    }
    apply_confusion_to_target(state, side, move_name, rng, events);
}

fn apply_direct_heal_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    events: &mut Vec<BattleEvent>,
) {
    let weather = state.weather;
    let pokemon = state.pokemon_mut(side);
    if move_data.name == "REST" {
        apply_rest_heal_effect(pokemon, side, move_name, events);
        return;
    }
    if pokemon.hp >= pokemon.max_hp {
        events.push(BattleEvent::HealFailed {
            side,
            move_name: move_name.to_string(),
            hp: pokemon.hp,
            max_hp: pokemon.max_hp,
        });
        return;
    }
    let hp_before = pokemon.hp;
    let amount =
        direct_heal_amount(pokemon.max_hp, move_data, weather).min(pokemon.max_hp - pokemon.hp);
    pokemon.hp += amount;
    events.push(BattleEvent::HealApplied {
        side,
        move_name: move_name.to_string(),
        hp_before,
        hp_after: pokemon.hp,
        amount,
    });
}

fn apply_heal_bell_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let status_before = {
        let pokemon = state.pokemon_mut(side);
        let Some(status_before) = pokemon.status.take() else {
            events.push(BattleEvent::StatusHealFailed {
                side,
                move_name: move_name.to_string(),
                target: side,
            });
            return;
        };
        pokemon.sleep_turns = 0;
        status_before
    };
    set_toxic_turns(state, side, 0);
    events.push(BattleEvent::StatusHealed {
        side,
        move_name: move_name.to_string(),
        target: side,
        status_before,
    });
}

fn apply_pain_split_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let user_hp_before = state.pokemon(side).hp;
    let target_hp_before = state.pokemon(target).hp;
    let split_hp = ((u32::from(user_hp_before) + u32::from(target_hp_before)) / 2) as u16;
    let user_hp_after = split_hp.min(state.pokemon(side).max_hp);
    let target_hp_after = split_hp.min(state.pokemon(target).max_hp);
    state.pokemon_mut(side).hp = user_hp_after;
    state.pokemon_mut(target).hp = target_hp_after;
    events.push(BattleEvent::PainSplitApplied {
        side,
        move_name: move_name.to_string(),
        target,
        user_hp_before,
        user_hp_after,
        target_hp_before,
        target_hp_after,
    });
}

fn apply_rest_heal_effect(
    pokemon: &mut Pokemon,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    if pokemon.status.as_deref() == Some("SLEEP")
        || (pokemon.hp >= pokemon.max_hp && pokemon.status.is_none())
    {
        events.push(BattleEvent::HealFailed {
            side,
            move_name: move_name.to_string(),
            hp: pokemon.hp,
            max_hp: pokemon.max_hp,
        });
        return;
    }
    let hp_before = pokemon.hp;
    pokemon.hp = pokemon.max_hp;
    pokemon.status = Some("SLEEP".to_string());
    pokemon.sleep_turns = 2;
    events.push(BattleEvent::HealApplied {
        side,
        move_name: move_name.to_string(),
        hp_before,
        hp_after: pokemon.hp,
        amount: pokemon.max_hp - hp_before,
    });
    events.push(BattleEvent::StatusApplied {
        side,
        move_name: move_name.to_string(),
        target: side,
        status: "SLEEP".to_string(),
    });
}

fn direct_heal_amount(max_hp: u16, move_data: &Move, weather: Weather) -> u16 {
    match move_data.effect.as_str() {
        "MOONLIGHT" | "MORNING_SUN" | "SYNTHESIS" => match weather {
            Weather::Sun => ((u32::from(max_hp) * 2) / 3).max(1) as u16,
            Weather::Rain | Weather::Sandstorm => (max_hp / 4).max(1),
            Weather::None => (max_hp / 2).max(1),
        },
        _ => (max_hp / 2).max(1),
    }
}

fn apply_perish_song_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    for target in [BattleSide::Player, BattleSide::Enemy] {
        let pokemon = state.pokemon_mut(target);
        if pokemon.hp == 0 {
            continue;
        }
        if pokemon.perish_song_turns != 0 {
            events.push(BattleEvent::PerishSongFailed {
                side,
                move_name: move_name.to_string(),
                target,
                turns_remaining: pokemon.perish_song_turns,
            });
            continue;
        }
        pokemon.perish_song_turns = 4;
        events.push(BattleEvent::PerishSongApplied {
            side,
            move_name: move_name.to_string(),
            target,
            turns: pokemon.perish_song_turns,
        });
    }
}

fn apply_focus_energy_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon_mut(side);
    if pokemon.focus_energy {
        events.push(BattleEvent::FocusEnergyFailed {
            side,
            move_name: move_name.to_string(),
        });
        return;
    }
    pokemon.focus_energy = true;
    events.push(BattleEvent::FocusEnergyApplied {
        side,
        move_name: move_name.to_string(),
    });
}

fn apply_belly_drum_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let attack_stage = *state.pokemon(side).stat_boosts.get(&Stat::Attack).ok_or(
        BattleTurnError::MissingStatStage {
            side,
            stat: Stat::Attack,
        },
    )?;
    if attack_stage >= 6 {
        let pokemon = state.pokemon(side);
        events.push(BattleEvent::HealFailed {
            side,
            move_name: move_name.to_string(),
            hp: pokemon.hp,
            max_hp: pokemon.max_hp,
        });
        return Ok(());
    }

    if state.pokemon(side).hp < state.pokemon(side).max_hp / 2 {
        apply_stat_stage_delta(state, side, move_name, Stat::Attack, 2, events)?;
        let pokemon = state.pokemon(side);
        events.push(BattleEvent::HealFailed {
            side,
            move_name: move_name.to_string(),
            hp: pokemon.hp,
            max_hp: pokemon.max_hp,
        });
        return Ok(());
    }

    let hp_cost = state.pokemon(side).max_hp / 2;
    let pokemon = state.pokemon_mut(side);
    pokemon.hp = pokemon.hp.saturating_sub(hp_cost);
    pokemon.stat_boosts.insert(Stat::Attack, 6);
    events.push(BattleEvent::StatStageChanged {
        side,
        move_name: move_name.to_string(),
        target: side,
        stat: Stat::Attack,
        amount: 6 - attack_stage,
        stage_before: attack_stage,
        stage_after: 6,
    });
    Ok(())
}

fn apply_defense_curl_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    set_defense_curled(state, side, true);
    apply_stat_stage_delta(state, side, move_name, Stat::Defense, 1, events)
}

fn apply_curse_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if pokemon_types_include(state, side, "GHOST") {
        let target = side.other();
        if curse_source(state, target).is_some() || state.pokemon(side).hp <= 1 {
            events.push(BattleEvent::CurseFailed {
                side,
                move_name: move_name.to_string(),
                target,
            });
            return Ok(());
        }
        let hp_before = state.pokemon(side).hp;
        let hp_cost = (state.pokemon(side).max_hp / 2).max(1).min(hp_before);
        let user = state.pokemon_mut(side);
        user.hp = user.hp.saturating_sub(hp_cost);
        let hp_after = user.hp;
        set_curse_source(state, target, Some(side));
        events.push(BattleEvent::CurseApplied {
            side,
            move_name: move_name.to_string(),
            target,
            hp_cost,
            hp_before,
            hp_after,
        });
        if hp_after == 0 {
            events.push(BattleEvent::Fainted { side });
        }
        return Ok(());
    }

    let pokemon = state.pokemon(side);
    let attack_stage =
        *pokemon
            .stat_boosts
            .get(&Stat::Attack)
            .ok_or(BattleTurnError::MissingStatStage {
                side,
                stat: Stat::Attack,
            })?;
    let defense_stage =
        *pokemon
            .stat_boosts
            .get(&Stat::Defense)
            .ok_or(BattleTurnError::MissingStatStage {
                side,
                stat: Stat::Defense,
            })?;
    if attack_stage >= 6 && defense_stage >= 6 {
        events.push(BattleEvent::StatStageUnchanged {
            side,
            move_name: move_name.to_string(),
            target: side,
            stat: Stat::Attack,
            amount: 1,
            stage: attack_stage,
        });
        return Ok(());
    }

    apply_stat_stage_delta_to_target(state, side, move_name, side, Stat::Attack, 1, events)?;
    apply_stat_stage_delta_to_target(state, side, move_name, side, Stat::Defense, 1, events)?;
    apply_stat_stage_delta_to_target(state, side, move_name, side, Stat::Speed, -1, events)?;
    Ok(())
}

fn apply_mist_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let turns_remaining = mist_turns(state, side);
    if turns_remaining != 0 {
        events.push(BattleEvent::MistFailed {
            side,
            move_name: move_name.to_string(),
            turns_remaining,
        });
        return;
    }
    set_mist_turns(state, side, 5);
    events.push(BattleEvent::MistApplied {
        side,
        move_name: move_name.to_string(),
        turns: 5,
    });
}

fn apply_safeguard_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let turns_remaining = safeguard_turns(state, side);
    if turns_remaining != 0 {
        events.push(BattleEvent::SafeguardFailed {
            side,
            move_name: move_name.to_string(),
            turns_remaining,
        });
        return;
    }
    set_safeguard_turns(state, side, 5);
    events.push(BattleEvent::SafeguardApplied {
        side,
        move_name: move_name.to_string(),
        turns: 5,
    });
}

fn apply_screen_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    screen: BattleScreen,
    events: &mut Vec<BattleEvent>,
) {
    let turns_remaining = screen_turns(state, side, screen);
    if turns_remaining != 0 {
        match screen {
            BattleScreen::Reflect => events.push(BattleEvent::ReflectFailed {
                side,
                move_name: move_name.to_string(),
                turns_remaining,
            }),
            BattleScreen::LightScreen => events.push(BattleEvent::LightScreenFailed {
                side,
                move_name: move_name.to_string(),
                turns_remaining,
            }),
        }
        return;
    }
    set_screen_turns(state, side, screen, 5);
    match screen {
        BattleScreen::Reflect => events.push(BattleEvent::ReflectApplied {
            side,
            move_name: move_name.to_string(),
            turns: 5,
        }),
        BattleScreen::LightScreen => events.push(BattleEvent::LightScreenApplied {
            side,
            move_name: move_name.to_string(),
            turns: 5,
        }),
    }
}

fn apply_destiny_bond_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    set_destiny_bond_active(state, side, true);
    events.push(BattleEvent::DestinyBondApplied {
        side,
        move_name: move_name.to_string(),
    });
}

fn apply_sleep_talk_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target_switching: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    acted_before: &[BattleSide],
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let candidates: Vec<(usize, String)> = battle_moves(state, side)
        .iter()
        .enumerate()
        .filter(|(_, learned)| learned.current_pp != 0 && learned.name != move_name)
        .map(|(slot, learned)| (slot, learned.name.clone()))
        .collect();
    if candidates.is_empty() {
        events.push(BattleEvent::SleepTalkFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let roll = rng.randrange(candidates.len() as u32) as u8;
    let (selected_slot, selected_move) = candidates[usize::from(roll)].clone();
    let selected_data =
        moves
            .get(&selected_move)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: selected_move.clone(),
            })?;
    events.push(BattleEvent::SleepTalkSelected {
        side,
        move_name: move_name.to_string(),
        selected_slot,
        selected_move: selected_move.clone(),
        roll,
    });
    execute_move_effect(
        state,
        side,
        Some(selected_slot),
        &selected_move,
        selected_data,
        None,
        target_switching,
        moves,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        acted_before,
        force_switch_ends_battle,
        events,
    )
}

fn apply_mirror_move_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target_switching: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    acted_before: &[BattleSide],
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let target = side.other();
    let Some(copied_move) = last_move(state, target).map(ToOwned::to_owned) else {
        events.push(BattleEvent::MirrorMoveFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    };
    if copied_move == move_name {
        events.push(BattleEvent::MirrorMoveFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    }
    let copied_data = moves
        .get(&copied_move)
        .ok_or_else(|| BattleTurnError::MissingMoveData {
            side,
            move_name: copied_move.clone(),
        })?;
    events.push(BattleEvent::MirrorMoveSelected {
        side,
        move_name: move_name.to_string(),
        copied_move: copied_move.clone(),
    });
    execute_move_effect(
        state,
        side,
        None,
        &copied_move,
        copied_data,
        None,
        target_switching,
        moves,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        acted_before,
        force_switch_ends_battle,
        events,
    )
}

fn apply_metronome_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target_switching: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    acted_before: &[BattleSide],
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let candidates: Vec<&str> = moves
        .keys()
        .map(String::as_str)
        .filter(|candidate| *candidate != move_name)
        .collect();
    if candidates.is_empty() {
        events.push(BattleEvent::MetronomeFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let roll = rng.randrange(candidates.len() as u32) as u8;
    let selected_move = candidates[usize::from(roll)];
    let selected_data =
        moves
            .get(selected_move)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: selected_move.to_string(),
            })?;
    events.push(BattleEvent::MetronomeSelected {
        side,
        move_name: move_name.to_string(),
        selected_move: selected_move.to_string(),
        roll,
    });
    execute_move_effect(
        state,
        side,
        None,
        selected_move,
        selected_data,
        None,
        target_switching,
        moves,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        acted_before,
        force_switch_ends_battle,
        events,
    )
}

fn apply_mimic_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    executing_slot: Option<usize>,
    move_name: &str,
    moves: &BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let Some(slot) = executing_slot else {
        events.push(BattleEvent::MimicFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    let Some(copied_move) = last_move(state, target).map(ToOwned::to_owned) else {
        events.push(BattleEvent::MimicFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    if copied_move == move_name || !moves.contains_key(&copied_move) {
        events.push(BattleEvent::MimicFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    let Some(learned_move) = state.pokemon_mut(side).moves.get_mut(slot) else {
        events.push(BattleEvent::MimicFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    let replaced_move = learned_move.name.clone();
    learned_move.name = copied_move.clone();
    learned_move.current_pp = 5;
    learned_move.pp_ups = 0;
    events.push(BattleEvent::MimicApplied {
        side,
        move_name: move_name.to_string(),
        slot,
        replaced_move,
        copied_move,
    });
}

fn apply_sketch_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    executing_slot: Option<usize>,
    move_name: &str,
    moves: &BTreeMap<String, Move>,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let target = side.other();
    let Some(slot) = executing_slot else {
        events.push(BattleEvent::SketchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    };
    let Some(copied_move) = last_move(state, target).map(ToOwned::to_owned) else {
        events.push(BattleEvent::SketchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    };
    if copied_move == move_name {
        events.push(BattleEvent::SketchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    }
    let copied_data = moves
        .get(&copied_move)
        .ok_or_else(|| BattleTurnError::MissingMoveData {
            side,
            move_name: copied_move.clone(),
        })?;
    let Some(learned_move) = state.pokemon_mut(side).moves.get_mut(slot) else {
        events.push(BattleEvent::SketchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    };
    let replaced_move = learned_move.name.clone();
    learned_move.name = copied_move.clone();
    learned_move.current_pp = copied_data.pp;
    learned_move.pp_ups = 0;
    events.push(BattleEvent::SketchApplied {
        side,
        move_name: move_name.to_string(),
        slot,
        replaced_move,
        copied_move,
        copied_pp: copied_data.pp,
    });
    Ok(())
}

fn apply_conversion_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    moves: &BTreeMap<String, Move>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let current_types = effective_pokemon_types(state, side);
    let mut candidates = Vec::new();
    for learned_move in battle_moves(state, side) {
        let move_data =
            moves
                .get(&learned_move.name)
                .ok_or_else(|| BattleTurnError::MissingMoveData {
                    side,
                    move_name: learned_move.name.clone(),
                })?;
        if current_types.len() == 1 && current_types[0] == move_data.move_type {
            continue;
        }
        candidates.push((learned_move.name.clone(), move_data.move_type.clone()));
    }
    if candidates.is_empty() {
        events.push(BattleEvent::ConversionFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let roll = rng.randrange(candidates.len() as u32) as u8;
    let (selected_move, new_type) = candidates[usize::from(roll)].clone();
    set_type_override(
        state,
        side,
        Some(BattleTypeOverride {
            type1: new_type.clone(),
            type2: new_type.clone(),
        }),
    );
    events.push(BattleEvent::ConversionApplied {
        side,
        move_name: move_name.to_string(),
        selected_move,
        new_type,
        roll,
    });
    Ok(())
}

fn apply_conversion2_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    moves: &BTreeMap<String, Move>,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let Some(last_damage) = last_damage_state(state, side).cloned() else {
        events.push(BattleEvent::Conversion2Failed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    };
    if last_damage.source != side.other() {
        events.push(BattleEvent::Conversion2Failed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let source_move =
        moves
            .get(&last_damage.move_name)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: last_damage.move_name.clone(),
            })?;
    let current_types = effective_pokemon_types(state, side);
    let mut candidates = Vec::new();
    for candidate_type in declared_battle_types(type_categories) {
        if current_types.len() == 1 && current_types[0] == candidate_type {
            continue;
        }
        let multiplier = calculate_type_effectiveness_multiplier_with_foresight(
            type_effectiveness,
            &source_move.move_type,
            std::slice::from_ref(&candidate_type),
            false,
        )
        .map_err(BattleTurnError::DamageCalculation)?;
        if multiplier.numerator == 0 || multiplier.numerator < multiplier.denominator {
            candidates.push(candidate_type);
        }
    }
    if candidates.is_empty() {
        events.push(BattleEvent::Conversion2Failed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let roll = rng.randrange(candidates.len() as u32) as u8;
    let new_type = candidates[usize::from(roll)].clone();
    set_type_override(
        state,
        side,
        Some(BattleTypeOverride {
            type1: new_type.clone(),
            type2: new_type.clone(),
        }),
    );
    events.push(BattleEvent::Conversion2Applied {
        side,
        move_name: move_name.to_string(),
        source_move: last_damage.move_name,
        source_type: source_move.move_type.clone(),
        new_type,
        roll,
    });
    Ok(())
}

fn apply_bide_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let turns_remaining = bide_turns(state, side);
    if turns_remaining == 0 {
        let roll = rng.randrange(2) as u8;
        let turns = 2 + roll;
        set_bide_turns(state, side, turns);
        set_bide_damage(state, side, 0);
        events.push(BattleEvent::BideStarted {
            side,
            move_name: move_name.to_string(),
            turns,
            roll,
        });
        return;
    }

    if turns_remaining > 1 {
        let next_turns = turns_remaining - 1;
        set_bide_turns(state, side, next_turns);
        events.push(BattleEvent::BideStoring {
            side,
            move_name: move_name.to_string(),
            turns_remaining: next_turns,
            stored_damage: bide_damage(state, side),
        });
        return;
    }

    let stored_damage = bide_damage(state, side);
    reset_bide_state(state, side);
    if stored_damage == 0 {
        events.push(BattleEvent::BideFailed {
            side,
            move_name: move_name.to_string(),
        });
        return;
    }
    let target = side.other();
    let target_hp_before = state.pokemon(target).hp;
    let damage = stored_damage.saturating_mul(2).min(target_hp_before);
    state.pokemon_mut(target).hp = target_hp_before.saturating_sub(damage);
    events.push(BattleEvent::BideReleased {
        side,
        move_name: move_name.to_string(),
        target,
        stored_damage,
        damage,
        target_hp_before,
        target_hp_after: state.pokemon(target).hp,
    });
    if state.pokemon(target).hp == 0 {
        events.push(BattleEvent::Fainted { side: target });
    }
}

fn apply_encore_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let Some(encored_move) = last_move(state, target).map(ToOwned::to_owned) else {
        events.push(BattleEvent::EncoreFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    if encore_state(state, target).is_some() {
        events.push(BattleEvent::EncoreFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    let roll = rng.randrange(4) as u8;
    let turns = roll + 3;
    set_encore_state(
        state,
        target,
        Some(BattleEncoreState {
            move_name: encored_move.clone(),
            turns_remaining: turns,
        }),
    );
    events.push(BattleEvent::EncoreApplied {
        side,
        move_name: move_name.to_string(),
        target,
        encored_move,
        turns,
        roll,
    });
}

fn apply_leech_seed_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let target_types = effective_pokemon_types(state, target);
    if target_types
        .iter()
        .any(|pokemon_type| pokemon_type == "GRASS")
    {
        events.push(BattleEvent::LeechSeedImmune {
            side,
            move_name: move_name.to_string(),
            target,
            target_type1: target_types[0].clone(),
            target_type2: target_types
                .get(1)
                .cloned()
                .unwrap_or_else(|| target_types[0].clone()),
        });
        return;
    }
    if leech_seed_source(state, target).is_some() {
        events.push(BattleEvent::LeechSeedFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    set_leech_seed_source(state, target, Some(side));
    events.push(BattleEvent::LeechSeedApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_nightmare_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if state.pokemon(target).status.as_deref() != Some("SLEEP")
        || nightmare_source(state, target).is_some()
    {
        events.push(BattleEvent::NightmareFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    set_nightmare_source(state, target, Some(side));
    events.push(BattleEvent::NightmareApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn active_damage_screen(
    state: &BattleCombatState,
    defender: BattleSide,
    type_categories: &TypeCategories,
    move_data: &Move,
) -> Result<Option<BattleScreen>, BattleTurnError> {
    let screen = match damage_category(type_categories, move_data)? {
        BattleDamageCategory::Physical => BattleScreen::Reflect,
        BattleDamageCategory::Special => BattleScreen::LightScreen,
    };
    Ok((screen_turns(state, defender, screen) != 0).then_some(screen))
}

fn apply_force_switch_effect(
    _state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if !force_switch_ends_battle {
        events.push(BattleEvent::ForceSwitchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    events.push(BattleEvent::ForceSwitchApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
    events.push(BattleEvent::Fled { side: target });
}

fn apply_teleport_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    force_switch_ends_battle: bool,
    events: &mut Vec<BattleEvent>,
) {
    if let Some(trap) = escape_trap_state(state, side).cloned() {
        events.push(BattleEvent::RunBlocked {
            side,
            source: trap.source,
            move_name: trap.move_name,
        });
        return;
    }
    if !force_switch_ends_battle {
        events.push(BattleEvent::TeleportFailed {
            side,
            move_name: move_name.to_string(),
        });
        return;
    }
    events.push(BattleEvent::Fled { side });
}

fn apply_spikes_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if spikes_state(state, target) {
        events.push(BattleEvent::SpikesFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    set_spikes_state(state, target, true);
    events.push(BattleEvent::SpikesApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_switch_in_spikes(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    if !spikes_state(state, side) || state.pokemon(side).hp == 0 {
        return;
    }
    if pokemon_types_include(state, side, "FLYING") {
        events.push(BattleEvent::SpikesImmune { side });
        return;
    }
    let hp_before = state.pokemon(side).hp;
    let damage = (state.pokemon(side).max_hp / 8).max(1).min(hp_before);
    let pokemon = state.pokemon_mut(side);
    pokemon.hp = pokemon.hp.saturating_sub(damage);
    let hp_after = pokemon.hp;
    events.push(BattleEvent::SpikesDamage {
        side,
        damage,
        hp_before,
        hp_after,
    });
    if hp_after == 0 {
        events.push(BattleEvent::Fainted { side });
    }
}

fn apply_switch_in_held_effects(
    state: &mut BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if state.pokemon(side).hp == 0 {
        return Ok(());
    }
    let Some(item_id) = state.pokemon(side).item.clone() else {
        return Ok(());
    };
    let item = items
        .get(&item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.clone(),
        })?;
    if item.held_effect != "HELD_ATTACK_UP" {
        return Ok(());
    }
    let held_effect = item.held_effect.clone();
    state.pokemon_mut(side).item = None;
    events.push(BattleEvent::HeldItemActivated {
        side,
        item_id,
        held_effect,
    });
    apply_stat_stage_delta_to_target(state, side, "HELD_ATTACK_UP", side, Stat::Attack, 2, events)?;
    apply_confusion_to_side(state, side, "HELD_ATTACK_UP", rng, events);
    Ok(())
}

fn apply_escape_trap_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if escape_trap_state(state, target).is_some() {
        events.push(BattleEvent::EscapeTrapFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    set_escape_trap_state(
        state,
        target,
        Some(BattleEscapeTrapState {
            source: side,
            move_name: move_name.to_string(),
        }),
    );
    events.push(BattleEvent::EscapeTrapApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_trap_target_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if let Some(trap) = trap_state(state, target) {
        events.push(BattleEvent::TrapFailed {
            side,
            move_name: move_name.to_string(),
            target,
            turns_remaining: trap.turns_remaining,
        });
        return;
    }
    let roll = rng.randrange(4) as u8;
    let turns = roll + 2;
    set_trap_state(
        state,
        target,
        Some(BattleTrapState {
            source: side,
            move_name: move_name.to_string(),
            turns_remaining: turns,
        }),
    );
    events.push(BattleEvent::TrapApplied {
        side,
        move_name: move_name.to_string(),
        target,
        turns,
        roll,
    });
}

fn apply_lock_on_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if lock_on_target_state(state, side) {
        events.push(BattleEvent::LockOnFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    set_lock_on_target_state(state, side, true);
    events.push(BattleEvent::LockOnApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_attract_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let user_gender = battle_pokemon_gender(state.pokemon(side));
    let target_gender = battle_pokemon_gender(state.pokemon(target));
    if user_gender.is_none()
        || target_gender.is_none()
        || user_gender == target_gender
        || attracted_by_state(state, target).is_some()
    {
        events.push(BattleEvent::AttractFailed {
            side,
            move_name: move_name.to_string(),
            target,
            user_gender,
            target_gender,
        });
        return;
    }

    set_attracted_by_state(state, target, Some(side));
    events.push(BattleEvent::AttractApplied {
        side,
        move_name: move_name.to_string(),
        target,
        user_gender: user_gender.expect("checked user gender"),
        target_gender: target_gender.expect("checked target gender"),
    });
}

fn apply_disable_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let Some(disabled_move) = last_move(state, target).map(ToOwned::to_owned) else {
        events.push(BattleEvent::DisableFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    if disable_state(state, target).is_some() {
        events.push(BattleEvent::DisableFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }

    let roll = rng.randrange(4) as u8;
    let turns = roll + 4;
    set_disable_state(
        state,
        target,
        Some(BattleDisableState {
            move_name: disabled_move.clone(),
            turns_remaining: turns,
        }),
    );
    events.push(BattleEvent::DisableApplied {
        side,
        move_name: move_name.to_string(),
        target,
        disabled_move,
        turns,
        roll,
    });
}

fn roll_protect_success(
    state: &mut BattleCombatState,
    side: BattleSide,
    acted_before: &[BattleSide],
    rng: &mut Random,
) -> (bool, u8, Option<u8>) {
    let counter_before = protect_counter(state, side);
    if acted_before.contains(&side.other()) {
        reset_protect_counter(state, side);
        return (false, counter_before, None);
    }
    if counter_before == 0 {
        set_protect_counter(state, side, 1);
        return (true, counter_before, None);
    }

    let divisor_shift = counter_before.min(8);
    let threshold = 255u16 / (1u16 << divisor_shift);
    let roll = rng.randrange(256) as u8;
    if u16::from(roll) < threshold {
        set_protect_counter(state, side, counter_before.saturating_add(1));
        (true, counter_before, Some(roll))
    } else {
        reset_protect_counter(state, side);
        (false, counter_before, Some(roll))
    }
}

fn apply_protect_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    acted_before: &[BattleSide],
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let (success, counter_before, roll) = roll_protect_success(state, side, acted_before, rng);
    if !success {
        events.push(BattleEvent::ProtectFailed {
            side,
            move_name: move_name.to_string(),
            counter_before,
            roll,
        });
        return;
    }
    set_protect_active(state, side, true);
    events.push(BattleEvent::ProtectApplied {
        side,
        move_name: move_name.to_string(),
        counter: protect_counter(state, side),
        roll,
    });
}

fn apply_endure_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    acted_before: &[BattleSide],
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let (success, counter_before, roll) = roll_protect_success(state, side, acted_before, rng);
    if !success {
        events.push(BattleEvent::EndureFailed {
            side,
            move_name: move_name.to_string(),
            counter_before,
            roll,
        });
        return;
    }
    set_endure_active(state, side, true);
    events.push(BattleEvent::EndureApplied {
        side,
        move_name: move_name.to_string(),
        counter: protect_counter(state, side),
        roll,
    });
}

fn apply_spite_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let Some(target_move) = last_move(state, target).map(ToOwned::to_owned) else {
        events.push(BattleEvent::SpiteFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    let Some(learned_move) = state
        .pokemon_mut(target)
        .moves
        .iter_mut()
        .find(|learned| learned.name == target_move)
    else {
        events.push(BattleEvent::SpiteFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    };
    if learned_move.current_pp == 0 {
        events.push(BattleEvent::SpiteFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }

    let roll = rng.randrange(4) as u8;
    let reduction = roll + 2;
    let pp_before = learned_move.current_pp;
    learned_move.current_pp = learned_move.current_pp.saturating_sub(reduction);
    events.push(BattleEvent::SpiteApplied {
        side,
        move_name: move_name.to_string(),
        target,
        target_move,
        pp_before,
        pp_after: learned_move.current_pp,
        reduction,
        roll,
    });
}

fn apply_future_sight_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    stat_multipliers: &BattleStatMultiplierTables,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let target = side.other();
    if future_sight_state(state, target).is_some() {
        events.push(BattleEvent::FutureSightFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    }
    let damage = calculate_future_sight_damage(state, side, move_data, stat_multipliers, rng)?;
    set_future_sight_state(
        state,
        target,
        Some(BattleFutureSightState {
            source: side,
            move_name: move_name.to_string(),
            turns_remaining: 3,
            damage,
        }),
    );
    events.push(BattleEvent::FutureSightQueued {
        side,
        move_name: move_name.to_string(),
        target,
        damage,
        turns: 3,
    });
    Ok(())
}

fn calculate_future_sight_damage(
    state: &BattleCombatState,
    side: BattleSide,
    move_data: &Move,
    stat_multipliers: &BattleStatMultiplierTables,
    rng: &mut Random,
) -> Result<u16, BattleTurnError> {
    let attacker = state.pokemon(side);
    let defender_side = side.other();
    let defender = state.pokemon(defender_side);
    let attack_stage = *attacker.stat_boosts.get(&Stat::SpecialAttack).ok_or(
        BattleTurnError::MissingStatStage {
            side,
            stat: Stat::SpecialAttack,
        },
    )?;
    let defense_stage = *defender.stat_boosts.get(&Stat::SpecialDefense).ok_or(
        BattleTurnError::MissingStatStage {
            side: defender_side,
            stat: Stat::SpecialDefense,
        },
    )?;
    let attack = apply_stage(
        stat_multipliers,
        attacker.species.base_stats.special_attack,
        attack_stage,
    )
    .ok_or(BattleTurnError::MissingStatMultiplier {
        side,
        stage: attack_stage,
    })?;
    let defense = apply_stage(
        stat_multipliers,
        defender.species.base_stats.special_defense,
        defense_stage,
    )
    .ok_or(BattleTurnError::MissingStatMultiplier {
        side: defender_side,
        stage: defense_stage,
    })?;
    let level_factor = (u32::from(attacker.level) * 2) / 5 + 2;
    let mut damage = level_factor
        .saturating_mul(u32::from(move_data.power))
        .saturating_mul(u32::from(attack));
    damage /= u32::from(defense).max(1);
    damage = damage / 50 + 2;
    damage = damage.max(1);
    let damage_roll = 217 + rng.randrange(39);
    damage = (damage * damage_roll) / 255;
    Ok(damage.max(1).min(u32::from(u16::MAX)) as u16)
}

fn apply_transform_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if transform_state(state, target).is_some() {
        events.push(BattleEvent::TransformFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    let target_pokemon = state.pokemon(target);
    let target_species_id = target_pokemon.species.id.clone();
    let target_species = target_pokemon.species.clone();
    let target_dvs = target_pokemon.dvs;
    let target_stat_boosts = target_pokemon.stat_boosts.clone();
    let target_attack = target_pokemon.attack;
    let target_defense = target_pokemon.defense;
    let target_speed = target_pokemon.speed;
    let target_special_attack = target_pokemon.special_attack;
    let target_special_defense = target_pokemon.special_defense;
    let transformed_moves: Vec<LearnedMove> = battle_moves(state, target)
        .iter()
        .map(|learned| LearnedMove {
            name: learned.name.clone(),
            current_pp: if learned.name == "SKETCH" { 1 } else { 5 },
            pp_ups: 0,
        })
        .collect();
    set_transform_state(
        state,
        side,
        Some(BattleTransformState {
            species: target_species,
            dvs: target_dvs,
            moves: transformed_moves,
            stat_boosts: target_stat_boosts,
            attack: target_attack,
            defense: target_defense,
            speed: target_speed,
            special_attack: target_special_attack,
            special_defense: target_special_defense,
        }),
    );
    events.push(BattleEvent::TransformApplied {
        side,
        move_name: move_name.to_string(),
        target,
        species: target_species_id,
    });
}

fn apply_baton_pass_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    party_index: Option<usize>,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let party_index = party_index.ok_or_else(|| BattleTurnError::MissingMoveSwitchTarget {
        side,
        move_name: move_name.to_string(),
        effect: "BATON_PASS".to_string(),
    })?;
    let party = match side {
        BattleSide::Player => &state.player_party,
        BattleSide::Enemy => &state.enemy_party,
    };
    let Some(target) = party.get(party_index) else {
        return Err(BattleTurnError::SwitchTargetOutOfRange { side, party_index });
    };
    if target.hp == 0 {
        return Err(BattleTurnError::SwitchTargetFainted { side, party_index });
    }

    let stat_boosts = state.pokemon(side).stat_boosts.clone();
    let confusion_turns = state.pokemon(side).confusion_turns;
    let focus_energy = state.pokemon(side).focus_energy;
    clear_baton_pass_non_passable_conditions(state, side);
    switch_battle_combat_pokemon(state, side, party_index)?;
    state.pokemon_mut(side).stat_boosts = stat_boosts.clone();
    state.pokemon_mut(side).confusion_turns = confusion_turns;
    state.pokemon_mut(side).focus_energy = focus_energy;
    events.push(BattleEvent::BatonPassed {
        side,
        move_name: move_name.to_string(),
        party_index,
        stat_boosts,
        confusion_turns,
        focus_energy,
    });
    events.push(BattleEvent::Switched { side, party_index });
    apply_switch_in_spikes(state, side, events);
    apply_switch_in_held_effects(state, side, items, rng, events)?;
    Ok(())
}

fn switch_battle_combat_pokemon(
    state: &mut BattleCombatState,
    side: BattleSide,
    party_index: usize,
) -> Result<(), BattleTurnError> {
    let active_index = match side {
        BattleSide::Player => state.player_party_index,
        BattleSide::Enemy => state.enemy_party_index,
    };
    let outgoing = state.pokemon(side).clone();
    let party = match side {
        BattleSide::Player => &mut state.player_party,
        BattleSide::Enemy => &mut state.enemy_party,
    };
    let active_slot =
        party
            .get_mut(active_index)
            .ok_or(BattleTurnError::ActivePartyIndexOutOfRange {
                side,
                party_index: active_index,
            })?;
    *active_slot = outgoing;
    let mut switched = party
        .get(party_index)
        .cloned()
        .ok_or(BattleTurnError::SwitchTargetOutOfRange { side, party_index })?;
    if switched.hp == 0 {
        return Err(BattleTurnError::SwitchTargetFainted { side, party_index });
    }
    clear_switch_in_pokemon_battle_state(&mut switched);
    *state.pokemon_mut(side) = switched;
    match side {
        BattleSide::Player => state.player_party_index = party_index,
        BattleSide::Enemy => state.enemy_party_index = party_index,
    }
    Ok(())
}

fn sync_active_combat_pokemon_into_parties(
    state: &mut BattleCombatState,
) -> Result<(), BattleTurnError> {
    let player_index = state.player_party_index;
    let player = state.player.clone();
    let player_slot = state.player_party.get_mut(player_index).ok_or_else(|| {
        BattleTurnError::ActivePartyIndexOutOfRange {
            side: BattleSide::Player,
            party_index: player_index,
        }
    })?;
    *player_slot = player;

    let enemy_index = state.enemy_party_index;
    let enemy = state.enemy.clone();
    let enemy_slot = state.enemy_party.get_mut(enemy_index).ok_or_else(|| {
        BattleTurnError::ActivePartyIndexOutOfRange {
            side: BattleSide::Enemy,
            party_index: enemy_index,
        }
    })?;
    *enemy_slot = enemy;
    Ok(())
}

fn apply_foresight_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if identified_state(state, target) {
        events.push(BattleEvent::ForesightFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    set_identified(state, target, true);
    events.push(BattleEvent::ForesightApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_reset_stats_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    for target in [BattleSide::Player, BattleSide::Enemy] {
        for stage in state.pokemon_mut(target).stat_boosts.values_mut() {
            *stage = 0;
        }
    }
    events.push(BattleEvent::StatsReset {
        side,
        move_name: move_name.to_string(),
    });
}

fn apply_psych_up_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    let copied = state.pokemon(target).stat_boosts.clone();
    state.pokemon_mut(side).stat_boosts = copied;
    events.push(BattleEvent::PsychUpApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
}

fn apply_weather_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    weather: Weather,
    events: &mut Vec<BattleEvent>,
) {
    state.weather = weather;
    state.weather_turns = 5;
    events.push(BattleEvent::WeatherApplied {
        side,
        move_name: move_name.to_string(),
        weather,
        turns: state.weather_turns,
    });
}

fn apply_status_to_target(
    target_pokemon: &mut Pokemon,
    target_types: &[PokemonType],
    side: BattleSide,
    move_name: &str,
    target: BattleSide,
    status: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> bool {
    if pokemon_is_status_immune(target_types, status) {
        events.push(BattleEvent::StatusImmune {
            side,
            move_name: move_name.to_string(),
            target,
            status: status.to_string(),
            target_type1: target_types[0].clone(),
            target_type2: target_types
                .get(1)
                .cloned()
                .unwrap_or_else(|| target_types[0].clone()),
        });
        return false;
    }
    target_pokemon.status = Some(status.to_string());
    if status == "SLEEP" {
        target_pokemon.sleep_turns = 1 + rng.randrange(7) as u8;
    }
    events.push(BattleEvent::StatusApplied {
        side,
        move_name: move_name.to_string(),
        target,
        status: status.to_string(),
    });
    true
}

fn pokemon_is_status_immune(types: &[PokemonType], status: &str) -> bool {
    match status {
        "BURN" => types.iter().any(|pokemon_type| pokemon_type == "FIRE"),
        "POISON" | "BAD_POISON" => types
            .iter()
            .any(|pokemon_type| pokemon_type == "POISON" || pokemon_type == "STEEL"),
        "FREEZE" => types.iter().any(|pokemon_type| pokemon_type == "ICE"),
        _ => false,
    }
}

fn dream_eater_fails(state: &BattleCombatState, side: BattleSide, move_data: &Move) -> bool {
    move_data.effect == "DREAM_EATER"
        && state.pokemon(side.other()).status.as_deref() != Some("SLEEP")
}

fn snore_fails(state: &BattleCombatState, side: BattleSide, move_data: &Move) -> bool {
    move_data.effect == "SNORE" && state.pokemon(side).status.as_deref() != Some("SLEEP")
}

fn sleep_talk_fails(state: &BattleCombatState, side: BattleSide, move_data: &Move) -> bool {
    move_data.effect == "SLEEP_TALK" && state.pokemon(side).status.as_deref() != Some("SLEEP")
}

fn pokemon_is_sandstorm_immune(state: &BattleCombatState, side: BattleSide) -> bool {
    effective_pokemon_types(state, side)
        .iter()
        .any(|pokemon_type| {
            pokemon_type == "ROCK" || pokemon_type == "GROUND" || pokemon_type == "STEEL"
        })
}

fn damage_category(
    type_categories: &TypeCategories,
    move_data: &Move,
) -> Result<BattleDamageCategory, BattleTurnError> {
    if is_physical_type(type_categories, &move_data.move_type)
        .map_err(BattleTurnError::DamageCalculation)?
    {
        Ok(BattleDamageCategory::Physical)
    } else {
        Ok(BattleDamageCategory::Special)
    }
}

fn last_damage_state(
    state: &BattleCombatState,
    side: BattleSide,
) -> Option<&BattleLastDamageState> {
    match side {
        BattleSide::Player => state.player_last_damage.as_ref(),
        BattleSide::Enemy => state.enemy_last_damage.as_ref(),
    }
}

fn record_last_damage(
    state: &mut BattleCombatState,
    side: BattleSide,
    damage: BattleLastDamageState,
) {
    match side {
        BattleSide::Player => state.player_last_damage = Some(damage),
        BattleSide::Enemy => state.enemy_last_damage = Some(damage),
    }
}

fn clear_turn_last_damage(state: &mut BattleCombatState) {
    state.player_last_damage = None;
    state.enemy_last_damage = None;
}

fn identified_state(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_identified,
        BattleSide::Enemy => state.enemy_identified,
    }
}

fn set_identified(state: &mut BattleCombatState, side: BattleSide, identified: bool) {
    match side {
        BattleSide::Player => state.player_identified = identified,
        BattleSide::Enemy => state.enemy_identified = identified,
    }
}

fn lock_on_target_state(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_lock_on_target,
        BattleSide::Enemy => state.enemy_lock_on_target,
    }
}

fn set_lock_on_target_state(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_lock_on_target = active,
        BattleSide::Enemy => state.enemy_lock_on_target = active,
    }
}

fn attracted_by_state(state: &BattleCombatState, side: BattleSide) -> Option<BattleSide> {
    match side {
        BattleSide::Player => state.player_attracted_by,
        BattleSide::Enemy => state.enemy_attracted_by,
    }
}

fn set_attracted_by_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    source: Option<BattleSide>,
) {
    match side {
        BattleSide::Player => state.player_attracted_by = source,
        BattleSide::Enemy => state.enemy_attracted_by = source,
    }
}

fn clear_attracted_by_source(state: &mut BattleCombatState, source: BattleSide) {
    for side in [BattleSide::Player, BattleSide::Enemy] {
        if attracted_by_state(state, side) == Some(source) {
            set_attracted_by_state(state, side, None);
        }
    }
}

fn battle_pokemon_gender(pokemon: &Pokemon) -> Option<BattlePokemonGender> {
    match pokemon.species.gender_ratio {
        255 => None,
        254 => Some(BattlePokemonGender::Female),
        0 => Some(BattlePokemonGender::Male),
        ratio if pokemon.dvs.attack.saturating_mul(17) < ratio => Some(BattlePokemonGender::Female),
        _ => Some(BattlePokemonGender::Male),
    }
}

fn recharge_move_state(state: &BattleCombatState, side: BattleSide) -> Option<&str> {
    match side {
        BattleSide::Player => state.player_recharge_move.as_deref(),
        BattleSide::Enemy => state.enemy_recharge_move.as_deref(),
    }
}

fn set_recharge_move_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: Option<String>,
) {
    match side {
        BattleSide::Player => state.player_recharge_move = move_name,
        BattleSide::Enemy => state.enemy_recharge_move = move_name,
    }
}

fn airborne_move_state(state: &BattleCombatState, side: BattleSide) -> Option<&str> {
    match side {
        BattleSide::Player => state.player_airborne_move.as_deref(),
        BattleSide::Enemy => state.enemy_airborne_move.as_deref(),
    }
}

fn set_airborne_move_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: Option<String>,
) {
    match side {
        BattleSide::Player => state.player_airborne_move = move_name,
        BattleSide::Enemy => state.enemy_airborne_move = move_name,
    }
}

fn charging_move_state(state: &BattleCombatState, side: BattleSide) -> Option<&str> {
    match side {
        BattleSide::Player => state.player_charging_move.as_deref(),
        BattleSide::Enemy => state.enemy_charging_move.as_deref(),
    }
}

fn set_charging_move_state(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: Option<String>,
) {
    match side {
        BattleSide::Player => state.player_charging_move = move_name,
        BattleSide::Enemy => state.enemy_charging_move = move_name,
    }
}

fn committed_charge_move_state(state: &BattleCombatState, side: BattleSide) -> Option<&str> {
    airborne_move_state(state, side).or_else(|| charging_move_state(state, side))
}

fn action_priority(
    state: &BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
    target_action: &BattleAction,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
) -> Result<i8, BattleTurnError> {
    if let Some(committed_move) = committed_charge_move_state(state, side) {
        validate_battle_turn_move_name(side, committed_move)?;
        let move_data =
            moves
                .get(committed_move)
                .ok_or_else(|| BattleTurnError::MissingMoveData {
                    side,
                    move_name: committed_move.to_string(),
                })?;
        return Ok(move_priority(move_data, move_priorities)?);
    }
    match action {
        BattleAction::Move { slot } | BattleAction::MoveSwitch { slot, .. } => {
            let move_name = battle_moves(state, side)
                .get(*slot)
                .ok_or(BattleTurnError::MissingMoveSlot { side, slot: *slot })?
                .name
                .clone();
            validate_battle_turn_move_name(side, &move_name)?;
            let move_data =
                moves
                    .get(&move_name)
                    .ok_or_else(|| BattleTurnError::MissingMoveData {
                        side,
                        move_name: move_name.clone(),
                    })?;
            if move_data.effect == "PURSUIT" && matches!(target_action, BattleAction::Switch { .. })
            {
                return Ok(7);
            }
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
        BattleAction::Run => Ok(6),
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
    let speed = apply_stage(stat_multipliers, base, stage)
        .ok_or(BattleTurnError::MissingStatMultiplier { side, stage })?;
    Ok(apply_paralysis_speed_penalty(pokemon, speed))
}

fn apply_paralysis_speed_penalty(pokemon: &Pokemon, speed: u16) -> u16 {
    if pokemon.status.as_deref() == Some("PARALYSIS") {
        (speed / 4).max(1)
    } else {
        speed
    }
}

fn accuracy_byte(
    move_data: &Move,
    attacker_side: BattleSide,
    attacker: &Pokemon,
    defender: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
) -> Result<u8, BattleTurnError> {
    accuracy_byte_with_weather(
        move_data,
        attacker_side,
        attacker,
        defender,
        stat_multipliers,
        Weather::None,
    )
}

fn accuracy_byte_with_weather(
    move_data: &Move,
    attacker_side: BattleSide,
    attacker: &Pokemon,
    defender: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
    weather: Weather,
) -> Result<u8, BattleTurnError> {
    if move_data.effect == "ALWAYS_HIT"
        || (move_data.effect == "THUNDER" && weather == Weather::Rain)
    {
        return Ok(u8::MAX);
    }
    if move_data.effect == "THUNDER" && weather == Weather::Sun {
        return Ok(((50 * 255) / 100 + 1).clamp(1, 255) as u8);
    }
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

        let switch_error = BattleTurnError::SwitchTargetFainted {
            side: BattleSide::Player,
            party_index: 2,
        };
        let switch_json = serde_json::to_value(&switch_error).expect("serialize switch error");
        assert_eq!(
            serde_json::from_value::<BattleTurnError>(switch_json)
                .expect("deserialize switch error"),
            switch_error
        );
        let active_index_error = BattleTurnError::ActivePartyIndexOutOfRange {
            side: BattleSide::Enemy,
            party_index: 6,
        };
        let active_index_json =
            serde_json::to_value(&active_index_error).expect("serialize active index error");
        assert_eq!(
            serde_json::from_value::<BattleTurnError>(active_index_json)
                .expect("deserialize active index error"),
            active_index_error
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
            matchups: BTreeMap::from([
                (
                    pokemon_type("NORMAL"),
                    BTreeMap::from([
                        (
                            pokemon_type("NORMAL"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("ELECTRIC"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("FIGHTING"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("FLYING"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("GHOST"),
                            crate::battle::damage::TypeMultiplier::zero(),
                        ),
                    ]),
                ),
                (
                    pokemon_type("GROUND"),
                    BTreeMap::from([(
                        pokemon_type("NORMAL"),
                        crate::battle::damage::TypeMultiplier::one(),
                    )]),
                ),
                (
                    pokemon_type("DARK"),
                    BTreeMap::from([(
                        pokemon_type("NORMAL"),
                        crate::battle::damage::TypeMultiplier::one(),
                    )]),
                ),
                (
                    pokemon_type("ELECTRIC"),
                    BTreeMap::from([
                        (
                            pokemon_type("ROCK"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("GROUND"),
                            crate::battle::damage::TypeMultiplier::zero(),
                        ),
                    ]),
                ),
                (
                    pokemon_type("FIGHTING"),
                    BTreeMap::from([
                        (
                            pokemon_type("NORMAL"),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 2,
                                denominator: 1,
                            },
                        ),
                        (
                            pokemon_type("GHOST"),
                            crate::battle::damage::TypeMultiplier::zero(),
                        ),
                    ]),
                ),
                (
                    pokemon_type("FIRE"),
                    BTreeMap::from([
                        (
                            pokemon_type("NORMAL"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("FIGHTING"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("PSYCHIC_TYPE"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                    ]),
                ),
                (
                    pokemon_type("PSYCHIC_TYPE"),
                    BTreeMap::from([
                        (
                            pokemon_type("NORMAL"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            pokemon_type("FIRE"),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                    ]),
                ),
            ]),
            foresight_matchups: BTreeMap::from([
                (
                    pokemon_type("NORMAL"),
                    BTreeMap::from([(
                        pokemon_type("GHOST"),
                        crate::battle::damage::TypeMultiplier::one(),
                    )]),
                ),
                (
                    pokemon_type("FIGHTING"),
                    BTreeMap::from([(
                        pokemon_type("GHOST"),
                        crate::battle::damage::TypeMultiplier::one(),
                    )]),
                ),
            ]),
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

    #[test]
    fn supported_battle_move_effect_inventory_is_exact_and_sorted() {
        let effects = supported_battle_move_effects();
        assert!(
            effects.windows(2).all(|pair| pair[0] < pair[1]),
            "battle move effect inventory must stay sorted for binary search"
        );
        for effect in [
            "NORMAL_HIT",
            "MULTI_HIT",
            "POISON_MULTI_HIT",
            "FLINCH_HIT",
            "STATIC_DAMAGE",
            "SUBSTITUTE",
            "THIEF",
            "TRANSFORM",
            "TRI_ATTACK",
            "SWAGGER",
            "CONVERSION2",
        ] {
            assert!(
                battle_move_effect_is_supported(effect),
                "missing supported effect {effect}"
            );
        }
        assert!(!battle_move_effect_is_supported("MODDED_EFFECT"));
        assert!(!battle_move_effect_is_supported("normal_hit"));
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
                ("ALL_UP_HIT".to_string(), 1),
                ("ATTACK_UP".to_string(), 1),
                ("ATTACK_UP_HIT".to_string(), 1),
                ("ATTACK_UP_2".to_string(), 1),
                ("ATTACK_DOWN".to_string(), 1),
                ("ATTACK_DOWN_HIT".to_string(), 1),
                ("ATTACK_DOWN_2".to_string(), 1),
                ("ACCURACY_DOWN".to_string(), 1),
                ("ACCURACY_DOWN_HIT".to_string(), 1),
                ("DEFENSE_DOWN".to_string(), 1),
                ("DEFENSE_DOWN_HIT".to_string(), 1),
                ("DEFENSE_DOWN_2".to_string(), 1),
                ("DEFENSE_UP".to_string(), 1),
                ("DEFENSE_UP_HIT".to_string(), 1),
                ("DEFENSE_UP_2".to_string(), 1),
                ("SPEED_UP".to_string(), 1),
                ("SPEED_UP_2".to_string(), 1),
                ("SPEED_DOWN".to_string(), 1),
                ("SPEED_DOWN_2".to_string(), 1),
                ("SPECIAL_ATTACK_UP".to_string(), 1),
                ("SPEED_DOWN_HIT".to_string(), 1),
                ("EVASION_UP".to_string(), 1),
                ("EVASION_DOWN".to_string(), 1),
                ("EVASION_DOWN_HIT".to_string(), 1),
                ("BURN_HIT".to_string(), 1),
                ("FLAME_WHEEL".to_string(), 1),
                ("SACRED_FIRE".to_string(), 1),
                ("FREEZE_HIT".to_string(), 1),
                ("PARALYZE_HIT".to_string(), 1),
                ("POISON_HIT".to_string(), 1),
                ("TOXIC".to_string(), 1),
                ("FLINCH_HIT".to_string(), 1),
                ("CONFUSE".to_string(), 1),
                ("CONFUSE_HIT".to_string(), 1),
                ("BELLY_DRUM".to_string(), 1),
                ("CURSE".to_string(), 1),
                ("DEFENSE_CURL".to_string(), 1),
                ("HEAL".to_string(), 1),
                ("HEAL_BELL".to_string(), 1),
                ("PAIN_SPLIT".to_string(), 1),
                ("TELEPORT".to_string(), 1),
                ("ALWAYS_HIT".to_string(), 1),
                ("DREAM_EATER".to_string(), 1),
                ("SNORE".to_string(), 1),
                ("SLEEP_TALK".to_string(), 1),
                ("MIRROR_MOVE".to_string(), 1),
                ("METRONOME".to_string(), 1),
                ("MIMIC".to_string(), 1),
                ("SKETCH".to_string(), 1),
                ("CONVERSION".to_string(), 1),
                ("CONVERSION2".to_string(), 1),
                ("BIDE".to_string(), 1),
                ("ENCORE".to_string(), 1),
                ("FALSE_SWIPE".to_string(), 1),
                ("LEECH_HIT".to_string(), 1),
                ("MOONLIGHT".to_string(), 1),
                ("MORNING_SUN".to_string(), 1),
                ("RECOIL_HIT".to_string(), 1),
                ("PERISH_SONG".to_string(), 1),
                ("FOCUS_ENERGY".to_string(), 1),
                ("FUTURE_SIGHT".to_string(), 1),
                ("TRANSFORM".to_string(), 1),
                ("BATON_PASS".to_string(), 1),
                ("RAPID_SPIN".to_string(), 1),
                ("PURSUIT".to_string(), 1),
                ("BEAT_UP".to_string(), 1),
                ("MIST".to_string(), 1),
                ("SAFEGUARD".to_string(), 1),
                ("SUBSTITUTE".to_string(), 1),
                ("REFLECT".to_string(), 1),
                ("LIGHT_SCREEN".to_string(), 1),
                ("DESTINY_BOND".to_string(), 1),
                ("LEECH_SEED".to_string(), 1),
                ("NIGHTMARE".to_string(), 1),
                ("SPIKES".to_string(), 1),
                ("MEAN_LOOK".to_string(), 1),
                ("LOCK_ON".to_string(), 1),
                ("ATTRACT".to_string(), 1),
                ("TRAP_TARGET".to_string(), 1),
                ("DISABLE".to_string(), 1),
                ("SELFDESTRUCT".to_string(), 1),
                ("SPITE".to_string(), 1),
                ("RESET_STATS".to_string(), 1),
                ("PSYCH_UP".to_string(), 1),
                ("FORESIGHT".to_string(), 1),
                ("RAIN_DANCE".to_string(), 1),
                ("SANDSTORM".to_string(), 1),
                ("SUNNY_DAY".to_string(), 1),
                ("SYNTHESIS".to_string(), 1),
                ("THUNDER".to_string(), 1),
                ("HYPER_BEAM".to_string(), 1),
                ("RETURN".to_string(), 1),
                ("FRUSTRATION".to_string(), 1),
                ("REVERSAL".to_string(), 1),
                ("RAGE".to_string(), 1),
                ("FURY_CUTTER".to_string(), 1),
                ("ROLLOUT".to_string(), 1),
                ("MAGNITUDE".to_string(), 1),
                ("HIDDEN_POWER".to_string(), 1),
                ("PRESENT".to_string(), 1),
                ("SPLASH".to_string(), 1),
                ("GUST".to_string(), 1),
                ("TWISTER".to_string(), 1),
                ("STOMP".to_string(), 1),
                ("SKULL_BASH".to_string(), 1),
                ("SKY_ATTACK".to_string(), 1),
                ("RAZOR_WIND".to_string(), 1),
                ("SOLARBEAM".to_string(), 1),
                ("FLY".to_string(), 1),
                ("EARTHQUAKE".to_string(), 1),
                ("JUMP_KICK".to_string(), 1),
                ("RAMPAGE".to_string(), 1),
                ("STATIC_DAMAGE".to_string(), 1),
                ("LEVEL_DAMAGE".to_string(), 1),
                ("SUPER_FANG".to_string(), 1),
                ("PSYWAVE".to_string(), 1),
                ("DOUBLE_HIT".to_string(), 1),
                ("MULTI_HIT".to_string(), 1),
                ("POISON_MULTI_HIT".to_string(), 1),
                ("PAY_DAY".to_string(), 1),
                ("OHKO".to_string(), 1),
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

    fn battle_state(player: Pokemon, enemy: Pokemon, rng_seed: u32) -> BattleCombatState {
        let player_bench = pokemon(
            "BAYLEEF",
            player.speed.saturating_sub(1),
            player.species.type1.clone(),
            player.moves[0].name.as_str(),
        );
        let enemy_bench = pokemon(
            "PIDGEOTTO",
            enemy.speed.saturating_sub(1),
            enemy.species.type1.clone(),
            enemy.moves[0].name.as_str(),
        );
        BattleCombatState::new(player.clone(), enemy.clone(), rng_seed)
            .with_parties(vec![player, player_bench], vec![enemy, enemy_bench])
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
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };

        player.hp = 31;
        enemy.hp = 4;
        let mut combat_state = BattleCombatState::new(player.clone(), enemy.clone(), 0x1234_5678)
            .with_parties(vec![player.clone()], vec![enemy.clone()]);
        combat_state.turn = 1;
        let outcome = BattleTurnOutcome {
            state: combat_state,
            order: Vec::new(),
            events: vec![BattleEvent::PayDayMoney {
                side: BattleSide::Player,
                move_name: "PAY_DAY".to_string(),
                amount: 35,
            }],
        };

        commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit battle turn");

        assert_eq!(state.rng_seed, 0x1234_5678);
        assert_eq!(state.battle_pay_day_money, 35);
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
    fn commit_battle_turn_outcome_applies_heal_bell_to_player_party() {
        let mut state = GameState::default();
        let mut player = pokemon("MILTANK", 45, pokemon_type("NORMAL"), "HEAL_BELL");
        player.status = Some("BAD_POISON".to_string());
        let mut benched = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        benched.status = Some("PARALYSIS".to_string());
        benched.sleep_turns = 3;
        let mut empty_status = pokemon("TOTODILE", 45, pokemon_type("WATER"), "TACKLE");
        empty_status.status = None;
        empty_status.sleep_turns = 0;
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        state.storage.party.pokemon[0] = Some(player.clone());
        state.storage.party.pokemon[1] = Some(benched);
        state.storage.party.pokemon[2] = Some(empty_status);
        state.battle_active_enemy_party_index = Some(0);
        state.battle = crate::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };
        let mut resolved_player = player.clone();
        resolved_player.status = None;

        let outcome = BattleTurnOutcome {
            state: BattleCombatState::new(resolved_player, enemy, 0x1234_5678),
            order: Vec::new(),
            events: vec![BattleEvent::StatusHealed {
                side: BattleSide::Player,
                move_name: "HEAL_BELL".to_string(),
                target: BattleSide::Player,
                status_before: "BAD_POISON".to_string(),
            }],
        };

        commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit heal bell battle turn");

        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().status,
            None
        );
        assert_eq!(
            state.storage.party.pokemon[1].as_ref().unwrap().status,
            None
        );
        assert_eq!(
            state.storage.party.pokemon[1].as_ref().unwrap().sleep_turns,
            0
        );
        assert_eq!(
            state.storage.party.pokemon[2].as_ref().unwrap().status,
            None
        );
        assert_eq!(
            state.storage.party.pokemon[1].as_ref().unwrap().status,
            None
        );
    }

    #[test]
    fn commit_battle_turn_outcome_deactivates_when_either_side_fled() {
        for side in [BattleSide::Player, BattleSide::Enemy] {
            let mut state = GameState::default();
            let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
            let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
            state.storage.party.pokemon[0] = Some(player.clone());
            state.battle_active_enemy_party_index = Some(0);
            state.battle = crate::state::BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "ROUTE_29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy.clone()],
            };

            let mut combat_state =
                BattleCombatState::new(player.clone(), enemy.clone(), 0x2222_0000 + side as u32)
                    .with_parties(vec![player], vec![enemy]);
            combat_state.turn = 1;
            let outcome = BattleTurnOutcome {
                state: combat_state,
                order: vec![side],
                events: vec![BattleEvent::Fled { side }],
            };

            commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit fled battle turn");

            assert_eq!(state.battle, crate::state::BattleMemory::Inactive);
            assert_eq!(state.battle_active_party_index, None);
            assert_eq!(state.battle_active_enemy_party_index, None);
            assert_eq!(state.rng_seed, outcome.state.rng_seed_after);
        }
    }

    #[test]
    fn commit_wild_battle_escape_attempt_updates_attempts_rng_and_deactivates_on_success() {
        let mut state = GameState::default();
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        state.battle = crate::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
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
            battle_capture_ball: None,
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

    fn held_boost_item(id: &str, held_effect: &str) -> Item {
        let mut item = battle_item(id, 0, false);
        item.effect = "NONE".to_string();
        item.held_effect = held_effect.to_string();
        item.field_usable = false;
        item.consumable = false;
        item
    }

    fn quick_claw_item(parameter: i16) -> Item {
        let mut item = held_boost_item("QUICK_CLAW", "HELD_QUICK_CLAW");
        item.parameter = parameter;
        item
    }

    fn held_status_item(id: &str, held_effect: &str) -> Item {
        held_boost_item(id, held_effect)
    }

    fn player_damage_amount(events: &[BattleEvent]) -> u16 {
        events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("player damage event")
    }

    fn move_data_with_effect(
        name: &str,
        move_type: PokemonType,
        power: u16,
        accuracy: u8,
        effect: &str,
    ) -> Move {
        move_data_with_effect_chance(name, move_type, power, accuracy, effect, 0)
    }

    fn move_data_with_effect_chance(
        name: &str,
        move_type: PokemonType,
        power: u16,
        accuracy: u8,
        effect: &str,
        effect_chance: u8,
    ) -> Move {
        Move {
            name: name.to_string(),
            move_type,
            power,
            accuracy,
            pp: 35,
            effect: effect.to_string(),
            effect_chance,
            stat: None,
            amount: None,
        }
    }

    fn move_data_with_stat(
        name: &str,
        move_type: PokemonType,
        power: u16,
        accuracy: u8,
        effect: &str,
        effect_chance: u8,
        stat: Stat,
        amount: i8,
    ) -> Move {
        Move {
            name: name.to_string(),
            move_type,
            power,
            accuracy,
            pp: 35,
            effect: effect.to_string(),
            effect_chance,
            stat: Some(stat),
            amount: Some(amount),
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
    fn always_hit_effect_bypasses_accuracy_stage_math() {
        let attacker = pokemon("PIDGEY", 30, pokemon_type("FLYING"), "SWIFT");
        let defender = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let mut tables = stat_multipliers();
        tables.accuracy.clear();

        let accuracy = accuracy_byte(
            &move_data_with_effect("SWIFT", pokemon_type("NORMAL"), 60, 1, "ALWAYS_HIT"),
            BattleSide::Player,
            &attacker,
            &defender,
            &tables,
        )
        .expect("always-hit accuracy resolves without stage table");

        assert_eq!(accuracy, u8::MAX);
    }

    #[test]
    fn thunder_weather_accuracy_uses_exact_weather_rules() {
        let attacker = pokemon("PIKACHU", 30, pokemon_type("ELECTRIC"), "THUNDER");
        let defender = pokemon("PIDGEY", 40, pokemon_type("FLYING"), "TACKLE");
        let thunder =
            move_data_with_effect("THUNDER", pokemon_type("ELECTRIC"), 120, 70, "THUNDER");

        let rain_accuracy = accuracy_byte_with_weather(
            &thunder,
            BattleSide::Player,
            &attacker,
            &defender,
            &stat_multipliers(),
            Weather::Rain,
        )
        .expect("rain thunder accuracy resolves");
        let sun_accuracy = accuracy_byte_with_weather(
            &thunder,
            BattleSide::Player,
            &attacker,
            &defender,
            &stat_multipliers(),
            Weather::Sun,
        )
        .expect("sun thunder accuracy resolves");

        assert_eq!(rain_accuracy, u8::MAX);
        assert_eq!(sun_accuracy, 128);
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
            battle_state(player, enemy, rng.seed()),
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
    fn held_type_boost_item_increases_matching_move_damage_from_item_catalog() {
        let mut boosted_player = pokemon("DRATINI", 90, pokemon_type("DRAGON"), "DRAGONBREATH");
        boosted_player.item = Some("DRAGON_FANG".to_string());
        let plain_player = pokemon("DRATINI", 90, pokemon_type("DRAGON"), "DRAGONBREATH");
        let enemy = pokemon("WOOPER", 20, pokemon_type("WATER"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "DRAGONBREATH".to_string(),
                move_data("DRAGONBREATH", pokemon_type("DRAGON"), 60, 100),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let items = BTreeMap::from([(
            "DRAGON_FANG".to_string(),
            held_boost_item("DRAGON_FANG", "HELD_DRAGON_BOOST"),
        )]);

        let input = BattleTurnInput {
            player: BattleAction::Move { slot: 0 },
            enemy: BattleAction::Move { slot: 0 },
        };
        let mut plain_rng = Random::new(7);
        let plain = resolve_battle_turn_with_items(
            battle_state(plain_player, enemy.clone(), plain_rng.seed()),
            input.clone(),
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut plain_rng,
        )
        .expect("plain battle turn resolves");
        let mut boosted_rng = Random::new(7);
        let boosted = resolve_battle_turn_with_items(
            battle_state(boosted_player, enemy, boosted_rng.seed()),
            input,
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut boosted_rng,
        )
        .expect("boosted battle turn resolves");

        let plain_damage = player_damage_amount(&plain.events);
        let boosted_damage = player_damage_amount(&boosted.events);
        assert_eq!(boosted_damage, ((u32::from(plain_damage) * 11) / 10) as u16);
        assert!(boosted.events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::HeldItemDamageBoost {
                    side: BattleSide::Player,
                    item_id,
                    held_effect,
                    move_type,
                    damage_before,
                    damage_after,
                } if item_id == "DRAGON_FANG"
                    && held_effect == "HELD_DRAGON_BOOST"
                    && move_type == "DRAGON"
                    && *damage_before == plain_damage
                    && *damage_after == boosted_damage
            )
        }));
    }

    #[test]
    fn switch_to_active_party_index_is_rejected() {
        let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(8);

        let error = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 0 },
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
        .expect_err("switching to active party index rejects");

        assert_eq!(
            error,
            BattleTurnError::SwitchTargetAlreadyActive {
                side: BattleSide::Player,
                party_index: 0,
            }
        );
    }

    #[test]
    fn switch_to_out_of_range_party_index_is_rejected() {
        let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(8);

        let error = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 2 },
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
        .expect_err("switching to missing party index rejects");

        assert_eq!(
            error,
            BattleTurnError::SwitchTargetOutOfRange {
                side: BattleSide::Player,
                party_index: 2,
            }
        );
    }

    #[test]
    fn switch_to_fainted_party_index_is_rejected() {
        let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(8);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_party[1].hp = 0;

        let error = resolve_battle_turn(
            state,
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
        .expect_err("switching to fainted party index rejects");

        assert_eq!(
            error,
            BattleTurnError::SwitchTargetFainted {
                side: BattleSide::Player,
                party_index: 1,
            }
        );
    }

    #[test]
    fn unfocused_attack_records_noncritical_roll_from_deterministic_rng() {
        let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("noncritical turn resolves");

        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                critical: false,
                critical_roll: 22,
                critical_threshold: 17,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn fury_cutter_power_doubles_on_consecutive_hits() {
        let player = pokemon("SCYTHER", 90, pokemon_type("BUG"), "FURY_CUTTER");
        let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "FURY_CUTTER".to_string(),
                move_data_with_effect("FURY_CUTTER", pokemon_type("BUG"), 10, 100, "FURY_CUTTER"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(23);

        let first = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("first fury cutter turn resolves");

        assert_eq!(first.state.player_fury_cutter_chain, 1);
        assert!(first.events.contains(&BattleEvent::FuryCutterPower {
            side: BattleSide::Player,
            move_name: "FURY_CUTTER".to_string(),
            chain: 0,
            power: 10,
        }));

        let second = resolve_battle_turn(
            first.state,
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
        .expect("second fury cutter turn resolves");

        assert_eq!(second.state.player_fury_cutter_chain, 2);
        assert!(second.events.contains(&BattleEvent::FuryCutterPower {
            side: BattleSide::Player,
            move_name: "FURY_CUTTER".to_string(),
            chain: 1,
            power: 20,
        }));
    }

    #[test]
    fn fury_cutter_miss_resets_chain() {
        let player = pokemon("SCYTHER", 90, pokemon_type("BUG"), "FURY_CUTTER");
        let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "FURY_CUTTER".to_string(),
                move_data_with_effect("FURY_CUTTER", pokemon_type("BUG"), 10, 0, "FURY_CUTTER"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(24);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_fury_cutter_chain = 2;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("missed fury cutter turn resolves");

        assert_eq!(outcome.state.player_fury_cutter_chain, 0);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::FuryCutterPower { .. }))
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Missed {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "FURY_CUTTER"
        )));
    }

    #[test]
    fn rollout_forces_consecutive_turn_and_doubles_power() {
        let mut player = pokemon("GEODUDE", 90, pokemon_type("ROCK"), "ROLLOUT");
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "ROLLOUT".to_string(),
                move_data_with_effect("ROLLOUT", pokemon_type("ROCK"), 30, 100, "ROLLOUT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(25);

        let first = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("first rollout turn resolves");

        assert_eq!(first.state.player_rollout_turns, 4);
        assert_eq!(first.state.player_rollout_chain, 1);
        assert!(first.events.contains(&BattleEvent::RolloutPower {
            side: BattleSide::Player,
            move_name: "ROLLOUT".to_string(),
            chain: 0,
            defense_curled: false,
            power: 30,
        }));

        let second = resolve_battle_turn(
            first.state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 1 },
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
        .expect("forced rollout turn resolves");

        assert_eq!(second.state.player_rollout_turns, 3);
        assert_eq!(second.state.player_rollout_chain, 2);
        assert!(second.events.contains(&BattleEvent::RolloutForcedMove {
            side: BattleSide::Player,
            requested_slot: 1,
            requested_move: "TACKLE".to_string(),
            rollout_slot: 0,
            rollout_move: "ROLLOUT".to_string(),
            turns_remaining: 4,
        }));
        assert!(second.events.contains(&BattleEvent::RolloutPower {
            side: BattleSide::Player,
            move_name: "ROLLOUT".to_string(),
            chain: 1,
            defense_curled: false,
            power: 60,
        }));
    }

    #[test]
    fn defense_curl_doubles_rollout_power() {
        let mut player = pokemon("GEODUDE", 90, pokemon_type("ROCK"), "DEFENSE_CURL");
        player.moves.push(LearnedMove {
            name: "ROLLOUT".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "DEFENSE_CURL".to_string(),
                move_data_with_effect(
                    "DEFENSE_CURL",
                    pokemon_type("NORMAL"),
                    0,
                    100,
                    "DEFENSE_CURL",
                ),
            ),
            (
                "ROLLOUT".to_string(),
                move_data_with_effect("ROLLOUT", pokemon_type("ROCK"), 30, 100, "ROLLOUT"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(26);

        let curled = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("defense curl turn resolves");
        assert!(curled.state.player_defense_curled);

        let rollout = resolve_battle_turn(
            curled.state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 1 },
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
        .expect("curled rollout turn resolves");

        assert!(rollout.events.contains(&BattleEvent::RolloutPower {
            side: BattleSide::Player,
            move_name: "ROLLOUT".to_string(),
            chain: 0,
            defense_curled: true,
            power: 60,
        }));
    }

    #[test]
    fn rollout_miss_resets_sequence() {
        let player = pokemon("GEODUDE", 90, pokemon_type("ROCK"), "ROLLOUT");
        let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "ROLLOUT".to_string(),
                move_data_with_effect("ROLLOUT", pokemon_type("ROCK"), 30, 0, "ROLLOUT"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(27);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_rollout_turns = 3;
        state.player_rollout_chain = 2;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("missed rollout resolves");

        assert_eq!(outcome.state.player_rollout_turns, 0);
        assert_eq!(outcome.state.player_rollout_chain, 0);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Missed {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "ROLLOUT"
        )));
    }

    #[test]
    fn rage_raises_attack_when_active_user_is_damaged() {
        let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "RAGE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "RAGE".to_string(),
                move_data_with_effect("RAGE", pokemon_type("NORMAL"), 20, 100, "RAGE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 120, 100),
            ),
        ]);
        let mut rng = Random::new(28);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("rage turn resolves");

        assert!(outcome.state.player_rage_active);
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack),
            Some(&1)
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::StatStageChanged {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Player,
                stat: Stat::Attack,
                amount: 1,
                stage_before: 0,
                stage_after: 1,
            } if move_name == "RAGE"
        )));
    }

    #[test]
    fn switching_clears_rage_active_state() {
        let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "RAGE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_rage_active = true;

        clear_side_volatile_conditions(&mut state, BattleSide::Player);

        assert!(!state.player_rage_active);
    }

    #[test]
    fn bide_starts_and_stores_incoming_damage() {
        let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "BIDE".to_string(),
                move_data_with_effect("BIDE", pokemon_type("NORMAL"), 0, 100, "BIDE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(29);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("bide start turn resolves");

        assert!(outcome.state.player_bide_turns != 0);
        assert!(outcome.state.player_bide_damage != 0);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::BideStarted {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "BIDE"
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::BideStoredDamage {
                side: BattleSide::Player,
                source: BattleSide::Enemy,
                damage,
                stored_damage,
            } if *damage != 0 && damage == stored_damage
        )));
    }

    #[test]
    fn bide_forced_release_deals_double_stored_damage() {
        let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "BIDE".to_string(),
                move_data_with_effect("BIDE", pokemon_type("NORMAL"), 0, 100, "BIDE"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(30);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_last_move = Some("BIDE".to_string());
        state.player_bide_turns = 1;
        state.player_bide_damage = 12;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("bide release turn resolves");

        assert_eq!(outcome.state.player_bide_turns, 0);
        assert_eq!(outcome.state.player_bide_damage, 0);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - 24);
        assert!(outcome.events.contains(&BattleEvent::BideForcedMove {
            side: BattleSide::Player,
            requested_slot: 0,
            requested_move: "BIDE".to_string(),
            bide_slot: 0,
            bide_move: "BIDE".to_string(),
            turns_remaining: 1,
        }));
        assert!(outcome.events.contains(&BattleEvent::BideReleased {
            side: BattleSide::Player,
            move_name: "BIDE".to_string(),
            target: BattleSide::Enemy,
            stored_damage: 12,
            damage: 24,
            target_hp_before: enemy_hp,
            target_hp_after: enemy_hp - 24,
        }));
    }

    #[test]
    fn focus_energy_attack_uses_focused_critical_threshold() {
        let mut focused_player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        focused_player.focus_energy = true;
        let unfocused_player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        let focused_enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let unfocused_enemy = focused_enemy.clone();
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);

        let mut focused_rng = Random::new(22);
        let focused = resolve_battle_turn(
            battle_state(focused_player, focused_enemy, focused_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut focused_rng,
        )
        .expect("focused critical turn resolves");

        let mut unfocused_rng = Random::new(22);
        let unfocused = resolve_battle_turn(
            battle_state(unfocused_player, unfocused_enemy, unfocused_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut unfocused_rng,
        )
        .expect("unfocused comparison turn resolves");

        assert!(focused.state.enemy.hp < unfocused.state.enemy.hp);
        assert!(focused.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                critical: true,
                critical_roll: 22,
                critical_threshold: 32,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn return_and_frustration_power_use_exact_happiness_formula() {
        assert_eq!(return_power(255), 102);
        assert_eq!(return_power(70), 28);
        assert_eq!(return_power(0), 1);
        assert_eq!(frustration_power(0), 102);
        assert_eq!(frustration_power(185), 28);
        assert_eq!(frustration_power(255), 1);

        let base_return = move_data_with_effect("RETURN", pokemon_type("NORMAL"), 1, 100, "RETURN");
        let base_frustration =
            move_data_with_effect("FRUSTRATION", pokemon_type("NORMAL"), 1, 100, "FRUSTRATION");
        let mut friendly = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "RETURN");
        friendly.happiness = 255;
        let mut unhappy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "FRUSTRATION");
        unhappy.happiness = 0;
        let mut rng = Random::new(1);
        let mut events = Vec::new();

        assert_eq!(
            dynamic_move_power(
                BattleSide::Player,
                "RETURN",
                &friendly,
                &base_return,
                &mut rng,
                &mut events
            ),
            102
        );
        assert_eq!(
            dynamic_move_power(
                BattleSide::Player,
                "FRUSTRATION",
                &unhappy,
                &base_frustration,
                &mut rng,
                &mut events
            ),
            102
        );
    }

    #[test]
    fn reversal_power_uses_exact_hp_ratio_breakpoints() {
        assert_eq!(reversal_power(1, 48), 200);
        assert_eq!(reversal_power(4, 48), 150);
        assert_eq!(reversal_power(9, 48), 100);
        assert_eq!(reversal_power(16, 48), 80);
        assert_eq!(reversal_power(32, 48), 40);
        assert_eq!(reversal_power(33, 48), 20);
        assert_eq!(reversal_power(10, 0), 20);

        let base_reversal =
            move_data_with_effect("REVERSAL", pokemon_type("FIGHTING"), 1, 100, "REVERSAL");
        let mut attacker = pokemon("HERACROSS", 90, pokemon_type("BUG"), "REVERSAL");
        attacker.max_hp = 48;
        attacker.hp = 1;
        let mut rng = Random::new(1);
        let mut events = Vec::new();

        assert_eq!(
            dynamic_move_power(
                BattleSide::Player,
                "REVERSAL",
                &attacker,
                &base_reversal,
                &mut rng,
                &mut events
            ),
            200
        );
    }

    #[test]
    fn reversal_damage_scales_from_attacker_hp_ratio_not_pack_power() {
        let mut desperate = pokemon("HERACROSS", 90, pokemon_type("BUG"), "REVERSAL");
        desperate.max_hp = 48;
        desperate.hp = 1;
        let mut healthy = desperate.clone();
        healthy.hp = 48;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "REVERSAL".to_string(),
            move_data_with_effect("REVERSAL", pokemon_type("FIGHTING"), 1, 100, "REVERSAL"),
        )]);

        let mut desperate_rng = Random::new(22);
        let desperate_outcome = resolve_battle_turn(
            battle_state(desperate, enemy.clone(), desperate_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut desperate_rng,
        )
        .expect("desperate reversal resolves");

        let mut healthy_rng = Random::new(22);
        let healthy_outcome = resolve_battle_turn(
            battle_state(healthy, enemy, healthy_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut healthy_rng,
        )
        .expect("healthy reversal resolves");

        assert!(desperate_outcome.state.enemy.hp < healthy_outcome.state.enemy.hp);
        assert!(desperate_outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "REVERSAL"
        )));
    }

    #[test]
    fn pursuit_intercepts_switching_target_with_double_power() {
        let player = pokemon("UMBREON", 40, pokemon_type("DARK"), "PURSUIT");
        let mut enemy = pokemon("RATTATA", 120, pokemon_type("NORMAL"), "SPLASH");
        enemy.hp = 12;
        let moves = BTreeMap::from([(
            "PURSUIT".to_string(),
            move_data_with_effect("PURSUIT", pokemon_type("DARK"), 40, 100, "PURSUIT"),
        )]);
        let mut rng = Random::new(26);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("pursuit switch intercept resolves");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert_eq!(outcome.state.enemy.hp, 0);
        assert!(outcome.events.contains(&BattleEvent::PursuitPower {
            side: BattleSide::Player,
            move_name: "PURSUIT".to_string(),
            target: BattleSide::Enemy,
            power: 80,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Switched {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }

    #[test]
    fn pursuit_keeps_normal_power_when_target_is_not_switching() {
        let player = pokemon("UMBREON", 120, pokemon_type("DARK"), "PURSUIT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "PURSUIT".to_string(),
                move_data_with_effect("PURSUIT", pokemon_type("DARK"), 40, 100, "PURSUIT"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(27);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("normal pursuit resolves");

        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::PursuitPower { .. }))
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "PURSUIT"
        )));
    }

    #[test]
    fn beat_up_hits_once_for_each_eligible_party_member() {
        let player = pokemon("UMBREON", 120, pokemon_type("DARK"), "BEAT_UP");
        let ally = pokemon("HOUNDOUR", 90, pokemon_type("DARK"), "TACKLE");
        let mut poisoned = pokemon("ZUBAT", 80, pokemon_type("POISON"), "TACKLE");
        poisoned.status = Some("POISON".to_string());
        let mut fainted = pokemon("RATTATA", 70, pokemon_type("NORMAL"), "TACKLE");
        fainted.hp = 0;
        let mut enemy = pokemon("SENTRET", 40, pokemon_type("NORMAL"), "SPLASH");
        enemy.hp = 200;
        let moves = BTreeMap::from([
            (
                "BEAT_UP".to_string(),
                move_data_with_effect("BEAT_UP", pokemon_type("DARK"), 10, 100, "BEAT_UP"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(28);

        let outcome = resolve_battle_turn(
            battle_state(player.clone(), enemy.clone(), rng.seed())
                .with_parties(vec![player, ally, poisoned, fainted], vec![enemy]),
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
        .expect("beat up resolves");

        let participants = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                BattleEvent::BeatUpParticipant {
                    side,
                    party_index,
                    species,
                    ..
                } if *side == BattleSide::Player => Some((*party_index, species.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(participants, vec![(0, "UMBREON"), (1, "HOUNDOUR")]);
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(
                    event,
                    BattleEvent::Damage {
                        side: BattleSide::Player,
                        move_name,
                        ..
                    } if move_name == "BEAT_UP"
                ))
                .count(),
            2
        );
        assert!(outcome.state.enemy.hp < 200);
    }

    #[test]
    fn baton_pass_requires_explicit_move_switch_destination() {
        let player = pokemon("EEVEE", 120, pokemon_type("NORMAL"), "BATON_PASS");
        let enemy = pokemon("SENTRET", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "BATON_PASS".to_string(),
                move_data_with_effect("BATON_PASS", pokemon_type("NORMAL"), 0, 100, "BATON_PASS"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(29);

        let error = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect_err("baton pass without destination rejects");

        assert_eq!(
            error,
            BattleTurnError::MissingMoveSwitchTarget {
                side: BattleSide::Player,
                move_name: "BATON_PASS".to_string(),
                effect: "BATON_PASS".to_string(),
            }
        );
    }

    #[test]
    fn baton_pass_to_active_party_index_is_rejected() {
        let player = pokemon("EEVEE", 120, pokemon_type("NORMAL"), "BATON_PASS");
        let enemy = pokemon("SENTRET", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "BATON_PASS".to_string(),
                move_data_with_effect("BATON_PASS", pokemon_type("NORMAL"), 0, 100, "BATON_PASS"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(30);

        let error = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::MoveSwitch {
                    slot: 0,
                    party_index: 0,
                },
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
        .expect_err("baton pass to active party index rejects");

        assert_eq!(
            error,
            BattleTurnError::SwitchTargetAlreadyActive {
                side: BattleSide::Player,
                party_index: 0,
            }
        );
    }

    #[test]
    fn baton_pass_preserves_modeled_passable_state_while_switching() {
        let mut player = pokemon("EEVEE", 120, pokemon_type("NORMAL"), "BATON_PASS");
        player.stat_boosts.insert(Stat::Attack, 3);
        player.stat_boosts.insert(Stat::Speed, -2);
        player.confusion_turns = 4;
        player.focus_energy = true;
        player.flinching = true;
        let ally = pokemon("UMBREON", 90, pokemon_type("DARK"), "TACKLE");
        let enemy = pokemon("SENTRET", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "BATON_PASS".to_string(),
                move_data_with_effect("BATON_PASS", pokemon_type("NORMAL"), 0, 100, "BATON_PASS"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(30);
        let mut state = battle_state(player.clone(), enemy.clone(), rng.seed())
            .with_parties(vec![player, ally], vec![enemy]);
        state.player_disable = Some(BattleDisableState {
            move_name: "TACKLE".to_string(),
            turns_remaining: 3,
        });
        state.player_transform = Some(BattleTransformState {
            species: state.enemy.species.clone(),
            dvs: state.enemy.dvs,
            moves: state.enemy.moves.clone(),
            stat_boosts: state.enemy.stat_boosts.clone(),
            attack: state.enemy.attack,
            defense: state.enemy.defense,
            speed: state.enemy.speed,
            special_attack: state.enemy.special_attack,
            special_defense: state.enemy.special_defense,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::MoveSwitch {
                    slot: 0,
                    party_index: 1,
                },
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
        .expect("baton pass resolves");

        assert_eq!(outcome.state.player.species.id, "UMBREON");
        assert_eq!(outcome.state.player_party_index, 1);
        assert_eq!(outcome.state.player_party[0].species.id, "EEVEE");
        assert_eq!(outcome.state.player_party[0].moves[0].current_pp, 4);
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack),
            Some(&3)
        );
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Speed),
            Some(&-2)
        );
        assert_eq!(outcome.state.player.confusion_turns, 4);
        assert!(outcome.state.player.focus_energy);
        assert!(!outcome.state.player.flinching);
        assert_eq!(outcome.state.player_party[1], outcome.state.player);
        assert_eq!(outcome.state.player_disable, None);
        assert_eq!(outcome.state.player_transform, None);
        assert!(outcome.events.contains(&BattleEvent::BatonPassed {
            side: BattleSide::Player,
            move_name: "BATON_PASS".to_string(),
            party_index: 1,
            stat_boosts: outcome.state.player.stat_boosts.clone(),
            confusion_turns: 4,
            focus_energy: true,
        }));
        assert!(outcome.events.contains(&BattleEvent::Switched {
            side: BattleSide::Player,
            party_index: 1,
        }));
    }

    #[test]
    fn baton_pass_switch_in_triggers_berserk_gene_from_item_catalog() {
        let player = pokemon("EEVEE", 120, pokemon_type("NORMAL"), "BATON_PASS");
        let mut ally = pokemon("UMBREON", 90, pokemon_type("DARK"), "TACKLE");
        ally.item = Some("BERSERK_GENE".to_string());
        let enemy = pokemon("SENTRET", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "BATON_PASS".to_string(),
                move_data_with_effect("BATON_PASS", pokemon_type("NORMAL"), 0, 100, "BATON_PASS"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let items = BTreeMap::from([(
            "BERSERK_GENE".to_string(),
            held_status_item("BERSERK_GENE", "HELD_ATTACK_UP"),
        )]);
        let mut rng = Random::new(30);
        let state = battle_state(player.clone(), enemy.clone(), rng.seed())
            .with_parties(vec![player, ally], vec![enemy]);

        let outcome = resolve_battle_turn_with_items(
            state,
            BattleTurnInput {
                player: BattleAction::MoveSwitch {
                    slot: 0,
                    party_index: 1,
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
        .expect("baton pass Berserk Gene resolves");

        assert_eq!(outcome.state.player.item, None);
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack).copied(),
            Some(2)
        );
        assert!((2..=5).contains(&outcome.state.player.confusion_turns));
        assert!(outcome.events.contains(&BattleEvent::HeldItemActivated {
            side: BattleSide::Player,
            item_id: "BERSERK_GENE".to_string(),
            held_effect: "HELD_ATTACK_UP".to_string(),
        }));
    }

    #[test]
    fn magnitude_power_uses_exact_random_roll_breakpoints() {
        assert_eq!(magnitude_power(0), 10);
        assert_eq!(magnitude_power(12), 10);
        assert_eq!(magnitude_power(13), 30);
        assert_eq!(magnitude_power(38), 30);
        assert_eq!(magnitude_power(39), 50);
        assert_eq!(magnitude_power(89), 50);
        assert_eq!(magnitude_power(90), 70);
        assert_eq!(magnitude_power(166), 70);
        assert_eq!(magnitude_power(167), 90);
        assert_eq!(magnitude_power(217), 90);
        assert_eq!(magnitude_power(218), 110);
        assert_eq!(magnitude_power(242), 110);
        assert_eq!(magnitude_power(243), 150);
        assert_eq!(magnitude_power(255), 150);
    }

    #[test]
    fn magnitude_roll_selects_power_before_damage() {
        let player = pokemon("GEODUDE", 90, pokemon_type("ROCK"), "MAGNITUDE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "MAGNITUDE".to_string(),
            move_data_with_effect("MAGNITUDE", pokemon_type("GROUND"), 1, 100, "MAGNITUDE"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("magnitude resolves");

        let (roll, power) = outcome
            .events
            .iter()
            .find_map(|event| {
                if let BattleEvent::MagnitudePower {
                    side: BattleSide::Player,
                    move_name,
                    roll,
                    power,
                } = event
                {
                    (move_name == "MAGNITUDE").then_some((*roll, *power))
                } else {
                    None
                }
            })
            .expect("magnitude power event");
        assert_eq!(power, magnitude_power(roll));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                ..
            } if move_name == "MAGNITUDE" && *damage > 0
        )));
    }

    #[test]
    fn hidden_power_type_and_power_use_exact_dv_formula() {
        let mut fighting = pokemon("UNOWN", 90, pokemon_type("PSYCHIC_TYPE"), "HIDDEN_POWER");
        fighting.dvs = Dv::from_non_hp(0, 0, 0, 0);
        assert_eq!(
            hidden_power_type_power(&fighting),
            (pokemon_type("FIGHTING"), 31)
        );

        let mut dark = pokemon("UNOWN", 90, pokemon_type("PSYCHIC_TYPE"), "HIDDEN_POWER");
        dark.dvs = Dv::from_non_hp(15, 15, 15, 15);
        assert_eq!(hidden_power_type_power(&dark), (pokemon_type("DARK"), 70));

        let mut electric = pokemon("UNOWN", 90, pokemon_type("PSYCHIC_TYPE"), "HIDDEN_POWER");
        electric.dvs = Dv::from_non_hp(2, 3, 8, 1);
        assert_eq!(
            hidden_power_type_power(&electric),
            (pokemon_type("ELECTRIC"), 41)
        );
    }

    #[test]
    fn hidden_power_resolved_type_drives_damage_matchup() {
        let mut player = pokemon("UNOWN", 90, pokemon_type("PSYCHIC_TYPE"), "HIDDEN_POWER");
        player.dvs = Dv::from_non_hp(0, 0, 0, 0);
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "HIDDEN_POWER".to_string(),
            move_data_with_effect(
                "HIDDEN_POWER",
                pokemon_type("ELECTRIC"),
                1,
                100,
                "HIDDEN_POWER",
            ),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("hidden power resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::HiddenPowerResolved {
            side: BattleSide::Player,
            move_name: "HIDDEN_POWER".to_string(),
            move_type: pokemon_type("FIGHTING"),
            power: 31,
        }));
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "HIDDEN_POWER".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Damage { .. }))
        );
    }

    #[test]
    fn present_roll_uses_exact_random_roll_breakpoints() {
        assert!(matches!(present_roll(0), PresentRoll::Damage(40)));
        assert!(matches!(present_roll(102), PresentRoll::Damage(40)));
        assert!(matches!(present_roll(103), PresentRoll::Damage(80)));
        assert!(matches!(present_roll(178), PresentRoll::Damage(80)));
        assert!(matches!(present_roll(179), PresentRoll::Damage(120)));
        assert!(matches!(present_roll(204), PresentRoll::Damage(120)));
        assert!(matches!(present_roll(205), PresentRoll::Heal));
        assert!(matches!(present_roll(255), PresentRoll::Heal));
    }

    #[test]
    fn present_damage_branch_selects_power_before_damage() {
        let player = pokemon("DELIBIRD", 90, pokemon_type("ICE"), "PRESENT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "PRESENT".to_string(),
            move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT"),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("present damage resolves");

        assert!(outcome.events.contains(&BattleEvent::PresentPower {
            side: BattleSide::Player,
            move_name: "PRESENT".to_string(),
            roll: 64,
            power: 40,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                ..
            } if move_name == "PRESENT" && *damage > 0
        )));
    }

    #[test]
    fn present_heal_branch_restores_target_hp_without_damage() {
        let player = pokemon("DELIBIRD", 90, pokemon_type("ICE"), "PRESENT");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 10;
        enemy.max_hp = 44;
        let moves = BTreeMap::from([(
            "PRESENT".to_string(),
            move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT"),
        )]);
        let mut rng = Random::new(15);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("present heal resolves");

        assert_eq!(outcome.state.enemy.hp, 21);
        assert!(outcome.events.contains(&BattleEvent::PresentHeal {
            side: BattleSide::Player,
            move_name: "PRESENT".to_string(),
            target: BattleSide::Enemy,
            roll: 207,
            hp_before: 10,
            hp_after: 21,
            amount: 11,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Damage { .. }))
        );
    }

    #[test]
    fn present_heal_branch_fails_when_target_hp_is_full() {
        let player = pokemon("DELIBIRD", 90, pokemon_type("ICE"), "PRESENT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "PRESENT".to_string(),
            move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT"),
        )]);
        let mut rng = Random::new(15);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("present full-hp heal resolves");

        assert!(outcome.events.contains(&BattleEvent::PresentFailed {
            side: BattleSide::Player,
            move_name: "PRESENT".to_string(),
            target: BattleSide::Enemy,
            roll: 207,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Damage { .. }))
        );
    }

    #[test]
    fn gust_effect_resolves_as_exported_damage_move() {
        let player = pokemon("PIDGEY", 90, pokemon_type("FLYING"), "GUST");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "GUST".to_string(),
            move_data_with_effect("GUST", pokemon_type("FLYING"), 40, 100, "GUST"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("gust resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "GUST"
        )));
    }

    #[test]
    fn gust_deals_double_damage_to_airborne_fly_target() {
        let player = pokemon("PIDGEY", 90, pokemon_type("FLYING"), "GUST");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("FLYING"), "FLY");
        let moves = BTreeMap::from([
            (
                "GUST".to_string(),
                move_data_with_effect("GUST", pokemon_type("FLYING"), 40, 100, "GUST"),
            ),
            (
                "FLY".to_string(),
                move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 100, "FLY"),
            ),
        ]);
        let mut normal_rng = Random::new(22);
        let normal = resolve_battle_turn(
            battle_state(player.clone(), enemy.clone(), normal_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut normal_rng,
        )
        .expect("normal gust turn resolves");

        let mut airborne_state = battle_state(player, enemy, 22);
        airborne_state.enemy_airborne_move = Some("FLY".to_string());
        let mut airborne_rng = Random::new(22);
        let airborne = resolve_battle_turn(
            airborne_state,
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
            &mut airborne_rng,
        )
        .expect("airborne gust turn resolves");

        let normal_damage = normal
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "GUST" => Some(*damage),
                _ => None,
            })
            .expect("normal gust damage");
        let airborne_damage = airborne
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "GUST" => Some(*damage),
                _ => None,
            })
            .expect("airborne gust damage");
        assert!(airborne_damage > normal_damage);
    }

    #[test]
    fn twister_hits_airborne_target_with_double_power_and_flinch_chance() {
        let player = pokemon("DRATINI", 90, pokemon_type("DRAGON"), "TWISTER");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("FLYING"), "FLY");
        let moves = BTreeMap::from([
            (
                "TWISTER".to_string(),
                move_data_with_effect_chance(
                    "TWISTER",
                    pokemon_type("DRAGON"),
                    40,
                    100,
                    "TWISTER",
                    100,
                ),
            ),
            (
                "FLY".to_string(),
                move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 100, "FLY"),
            ),
        ]);
        let mut normal_rng = Random::new(22);
        let normal = resolve_battle_turn(
            battle_state(player.clone(), enemy.clone(), normal_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut normal_rng,
        )
        .expect("normal twister resolves");

        let mut state = battle_state(player, enemy, 22);
        state.enemy_airborne_move = Some("FLY".to_string());
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("airborne twister resolves");

        let normal_damage = normal
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "TWISTER" => Some(*damage),
                _ => None,
            })
            .expect("normal twister damage");
        let airborne_damage = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "TWISTER" => Some(*damage),
                _ => None,
            })
            .expect("airborne twister damage");
        assert!(airborne_damage > normal_damage);
        assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name: "TWISTER".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::AirborneAvoided { .. }))
        );
    }

    #[test]
    fn stomp_effect_resolves_as_exported_damage_and_flinch_move() {
        let player = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "STOMP");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "STOMP".to_string(),
                move_data_with_effect_chance(
                    "STOMP",
                    pokemon_type("NORMAL"),
                    65,
                    100,
                    "STOMP",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("stomp resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "STOMP"
        )));
        assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name: "STOMP".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(outcome.events.contains(&BattleEvent::Flinched {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn razor_wind_effect_charges_before_damage() {
        let player = pokemon("PIDGEOT", 90, pokemon_type("FLYING"), "RAZOR_WIND");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "RAZOR_WIND".to_string(),
            move_data_with_effect("RAZOR_WIND", pokemon_type("NORMAL"), 80, 100, "RAZOR_WIND"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("razor wind resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert_eq!(
            outcome.state.player_charging_move,
            Some("RAZOR_WIND".to_string())
        );
        assert!(outcome.events.contains(&BattleEvent::ChargeStarted {
            side: BattleSide::Player,
            move_name: "RAZOR_WIND".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "RAZOR_WIND"
        )));
    }

    #[test]
    fn skull_bash_charges_and_raises_defense_before_damage() {
        let player = pokemon("SQUIRTLE", 90, pokemon_type("WATER"), "SKULL_BASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SKULL_BASH".to_string(),
                move_data_with_effect("SKULL_BASH", pokemon_type("NORMAL"), 100, 100, "SKULL_BASH"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("skull bash charge resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert_eq!(
            outcome.state.player_charging_move,
            Some("SKULL_BASH".to_string())
        );
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Defense),
            Some(&1)
        );
        assert!(outcome.events.contains(&BattleEvent::ChargeStarted {
            side: BattleSide::Player,
            move_name: "SKULL_BASH".to_string(),
        }));
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "SKULL_BASH".to_string(),
            target: BattleSide::Player,
            stat: Stat::Defense,
            amount: 1,
            stage_before: 0,
            stage_after: 1,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SKULL_BASH"
        )));
    }

    #[test]
    fn sky_attack_charges_then_forced_release_can_flinch() {
        let player = pokemon("PIDGEOT", 90, pokemon_type("FLYING"), "SKY_ATTACK");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SKY_ATTACK".to_string(),
                move_data_with_effect_chance(
                    "SKY_ATTACK",
                    pokemon_type("FLYING"),
                    140,
                    100,
                    "SKY_ATTACK",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(22);
        let first = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("sky attack charge resolves");
        assert_eq!(
            first.state.player_charging_move,
            Some("SKY_ATTACK".to_string())
        );

        let released = resolve_battle_turn(
            first.state,
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
        .expect("sky attack release resolves");

        assert!(released.events.contains(&BattleEvent::ChargeForcedMove {
            side: BattleSide::Player,
            requested_slot: 0,
            requested_move: "SKY_ATTACK".to_string(),
            charged_slot: 0,
            charged_move: "SKY_ATTACK".to_string(),
        }));
        assert!(released.events.contains(&BattleEvent::ChargeEnded {
            side: BattleSide::Player,
            move_name: "SKY_ATTACK".to_string(),
        }));
        assert!(released.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SKY_ATTACK"
        )));
        assert!(released.events.contains(&BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name: "SKY_ATTACK".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn solarbeam_effect_charges_before_damage_outside_sun() {
        let player = pokemon("BELLSPROUT", 90, pokemon_type("GRASS"), "SOLARBEAM");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "SOLARBEAM".to_string(),
            move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 120, 100, "SOLARBEAM"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("solarbeam resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert_eq!(
            outcome.state.player_charging_move,
            Some("SOLARBEAM".to_string())
        );
        assert!(outcome.events.contains(&BattleEvent::ChargeStarted {
            side: BattleSide::Player,
            move_name: "SOLARBEAM".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SOLARBEAM"
        )));
    }

    #[test]
    fn return_damage_scales_from_attacker_happiness_not_pack_power() {
        let mut friendly = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "RETURN");
        friendly.happiness = 255;
        let mut indifferent = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "RETURN");
        indifferent.happiness = 0;
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "RETURN".to_string(),
            move_data_with_effect("RETURN", pokemon_type("NORMAL"), 1, 100, "RETURN"),
        )]);

        let mut friendly_rng = Random::new(22);
        let friendly_outcome = resolve_battle_turn(
            battle_state(friendly, enemy.clone(), friendly_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut friendly_rng,
        )
        .expect("friendly return resolves");

        let mut indifferent_rng = Random::new(22);
        let indifferent_outcome = resolve_battle_turn(
            battle_state(indifferent, enemy, indifferent_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut indifferent_rng,
        )
        .expect("indifferent return resolves");

        assert!(friendly_outcome.state.enemy.hp < indifferent_outcome.state.enemy.hp);
        assert!(friendly_outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "RETURN"
        )));
    }

    #[test]
    fn static_damage_move_uses_pack_power_as_damage_amount() {
        let player = pokemon("VOLTORB", 90, pokemon_type("ELECTRIC"), "SONICBOOM");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SONICBOOM".to_string(),
                move_data_with_effect(
                    "SONICBOOM",
                    pokemon_type("NORMAL"),
                    20,
                    100,
                    "STATIC_DAMAGE",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("static damage turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp - 20);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage: 20,
                ..
            } if move_name == "SONICBOOM"
        )));
    }

    #[test]
    fn level_damage_move_uses_attacker_level_as_damage_amount() {
        let mut player = pokemon("MACHOP", 45, pokemon_type("FIGHTING"), "SEISMIC_TOSS");
        player.level = 37;
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SEISMIC_TOSS".to_string(),
                move_data_with_effect(
                    "SEISMIC_TOSS",
                    pokemon_type("FIGHTING"),
                    1,
                    100,
                    "LEVEL_DAMAGE",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("level damage turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp - 37);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage: 37,
                ..
            } if move_name == "SEISMIC_TOSS"
        )));
    }

    #[test]
    fn thief_transfers_target_held_item_after_successful_damage() {
        let player = pokemon("SNEASEL", 60, pokemon_type("DARK"), "THIEF");
        let mut enemy = pokemon("ABRA", 45, pokemon_type("PSYCHIC_TYPE"), "TACKLE");
        enemy.item = Some("TWISTEDSPOON".to_string());
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "THIEF".to_string(),
                move_data_with_effect("THIEF", pokemon_type("DARK"), 40, 100, "THIEF"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let items = BTreeMap::from([(
            "TWISTEDSPOON".to_string(),
            held_boost_item("TWISTEDSPOON", "HELD_PSYCHIC_BOOST"),
        )]);
        let mut rng = Random::new(13);

        let outcome = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
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
        .expect("thief resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert_eq!(outcome.state.player.item.as_deref(), Some("TWISTEDSPOON"));
        assert_eq!(outcome.state.enemy.item, None);
        assert_eq!(
            outcome.state.player_party[0].item.as_deref(),
            Some("TWISTEDSPOON")
        );
        assert_eq!(outcome.state.enemy_party[0].item, None);
        assert!(outcome.events.contains(&BattleEvent::HeldItemStolen {
            side: BattleSide::Player,
            move_name: "THIEF".to_string(),
            target: BattleSide::Enemy,
            item_id: "TWISTEDSPOON".to_string(),
        }));
    }

    #[test]
    fn fixed_damage_move_still_respects_type_no_effect() {
        let mut player = pokemon("MACHOP", 45, pokemon_type("FIGHTING"), "SEISMIC_TOSS");
        player.level = 37;
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SEISMIC_TOSS".to_string(),
                move_data_with_effect(
                    "SEISMIC_TOSS",
                    pokemon_type("FIGHTING"),
                    1,
                    100,
                    "LEVEL_DAMAGE",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("fixed damage no-effect turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "SEISMIC_TOSS".to_string(),
        }));
    }

    #[test]
    fn super_fang_deals_half_of_current_defender_hp() {
        let player = pokemon("RATICATE", 90, pokemon_type("NORMAL"), "SUPER_FANG");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let expected_damage = (enemy_hp / 2).max(1);
        let moves = BTreeMap::from([
            (
                "SUPER_FANG".to_string(),
                move_data_with_effect("SUPER_FANG", pokemon_type("NORMAL"), 1, 100, "SUPER_FANG"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("super fang turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp - expected_damage);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                result,
                ..
            } if move_name == "SUPER_FANG"
                && *damage == expected_damage
                && result.damage == expected_damage
        )));
    }

    #[test]
    fn psywave_uses_deterministic_scaled_level_damage() {
        let mut player = pokemon("MISDREAVUS", 90, pokemon_type("GHOST"), "PSYWAVE");
        player.level = 40;
        let enemy = pokemon("PIDGEY", 80, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "PSYWAVE".to_string(),
                move_data_with_effect("PSYWAVE", pokemon_type("PSYCHIC"), 1, 100, "PSYWAVE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("psywave turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp - 33);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage: 33,
                result,
                ..
            } if move_name == "PSYWAVE" && result.damage == 33
        )));
    }

    #[test]
    fn double_hit_move_applies_two_damage_hits() {
        let player = pokemon("DODUO", 90, pokemon_type("NORMAL"), "DOUBLE_KICK");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "DOUBLE_KICK".to_string(),
                move_data_with_effect(
                    "DOUBLE_KICK",
                    pokemon_type("FIGHTING"),
                    30,
                    100,
                    "DOUBLE_HIT",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("double hit turn resolves");

        let damage_events = outcome
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::Damage {
                        side: BattleSide::Player,
                        move_name,
                        ..
                    } if move_name == "DOUBLE_KICK"
                )
            })
            .count();
        assert_eq!(damage_events, 2);
        assert!(outcome.events.contains(&BattleEvent::MultiHitCount {
            side: BattleSide::Player,
            move_name: "DOUBLE_KICK".to_string(),
            hits: 2,
            roll: None,
        }));
        assert!(outcome.state.enemy.hp < enemy_hp);
    }

    #[test]
    fn multi_hit_move_uses_deterministic_two_to_five_hit_count() {
        let player = pokemon("BEEDRILL", 90, pokemon_type("BUG"), "FURY_ATTACK");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "FURY_ATTACK".to_string(),
                move_data_with_effect("FURY_ATTACK", pokemon_type("NORMAL"), 15, 100, "MULTI_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("multi hit turn resolves");

        let damage_events = outcome
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::Damage {
                        side: BattleSide::Player,
                        move_name,
                        ..
                    } if move_name == "FURY_ATTACK"
                )
            })
            .count();
        assert_eq!(damage_events, 5);
        assert!(outcome.events.contains(&BattleEvent::MultiHitCount {
            side: BattleSide::Player,
            move_name: "FURY_ATTACK".to_string(),
            hits: 5,
            roll: Some(7),
        }));
        assert!(outcome.state.enemy.hp < enemy_hp);
    }

    #[test]
    fn substitute_costs_hp_and_absorbs_damage_before_hp() {
        let player = pokemon("MR_MIME", 90, pokemon_type("PSYCHIC_TYPE"), "SUBSTITUTE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "SUBSTITUTE".to_string(),
                move_data_with_effect("SUBSTITUTE", pokemon_type("NORMAL"), 0, 100, "SUBSTITUTE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(2);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("substitute turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp - player_hp / 4);
        assert_eq!(outcome.state.player_substitute_hp, 0);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SubstituteCreated {
                side: BattleSide::Player,
                move_name,
                hp_cost,
                substitute_hp,
                ..
            } if move_name == "SUBSTITUTE" && *hp_cost == player_hp / 4 && *substitute_hp == player_hp / 4
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SubstituteDamaged {
                side: BattleSide::Enemy,
                move_name,
                target: BattleSide::Player,
                substitute_hp_before,
                substitute_hp_after: 0,
                ..
            } if move_name == "TACKLE" && *substitute_hp_before == player_hp / 4
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SubstituteBroken {
                side: BattleSide::Enemy,
                move_name,
                target: BattleSide::Player
            } if move_name == "TACKLE"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn pay_day_records_exact_level_based_money_event_after_damage() {
        let mut player = pokemon("MEOWTH", 90, pokemon_type("NORMAL"), "PAY_DAY");
        player.level = 12;
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "PAY_DAY".to_string(),
                move_data_with_effect("PAY_DAY", pokemon_type("NORMAL"), 40, 100, "PAY_DAY"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("pay day turn resolves");

        assert!(outcome.events.contains(&BattleEvent::PayDayMoney {
            side: BattleSide::Player,
            move_name: "PAY_DAY".to_string(),
            amount: 60,
        }));
    }

    #[test]
    fn ohko_move_faints_target_on_successful_level_checked_hit() {
        let mut player = pokemon("NIDOKING", 90, pokemon_type("NORMAL"), "HORN_DRILL");
        player.level = 40;
        let mut enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.level = 40;
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "HORN_DRILL".to_string(),
                move_data_with_effect("HORN_DRILL", pokemon_type("NORMAL"), 1, 30, "OHKO"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("ohko turn resolves");

        assert_eq!(outcome.state.enemy.hp, 0);
        assert!(outcome.events.contains(&BattleEvent::Damage {
            side: BattleSide::Player,
            move_name: "HORN_DRILL".to_string(),
            damage: enemy_hp,
            defender_hp_before: enemy_hp,
            defender_hp_after: 0,
            critical: false,
            critical_roll: 0,
            critical_threshold: 0,
            roll: 64,
            result: DamageResult {
                damage: enemy_hp,
                type_multiplier: crate::battle::damage::TypeMultiplier::one(),
            },
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
    }

    #[test]
    fn ohko_move_fails_when_attacker_level_is_lower() {
        let mut player = pokemon("NIDOKING", 90, pokemon_type("NORMAL"), "HORN_DRILL");
        player.level = 39;
        let mut enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.level = 40;
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "HORN_DRILL".to_string(),
                move_data_with_effect("HORN_DRILL", pokemon_type("NORMAL"), 1, 30, "OHKO"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("ohko level failure resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::OhkoFailed {
            side: BattleSide::Player,
            move_name: "HORN_DRILL".to_string(),
            reason: OhkoFailureReason::TargetLevelTooHigh {
                attacker_level: 39,
                defender_level: 40,
            },
        }));
    }

    #[test]
    fn ohko_move_respects_type_immunity() {
        let mut player = pokemon("NIDOKING", 90, pokemon_type("NORMAL"), "HORN_DRILL");
        player.level = 40;
        let mut enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        enemy.level = 40;
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "HORN_DRILL".to_string(),
                move_data_with_effect("HORN_DRILL", pokemon_type("NORMAL"), 1, 30, "OHKO"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("ohko type immunity resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "HORN_DRILL".to_string(),
        }));
    }

    #[test]
    fn direct_status_move_applies_status_after_accuracy_without_damage() {
        let player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "HYPNOSIS");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let mut moves = BTreeMap::new();
        moves.insert(
            "HYPNOSIS".to_string(),
            move_data_with_effect("HYPNOSIS", pokemon_type("PSYCHIC_TYPE"), 0, 0, "SLEEP"),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut priorities = move_priorities();
        priorities.effect_priorities.insert("SLEEP".to_string(), 1);
        let mut rng = Random::new(11);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &priorities,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("status turn resolves");

        assert_eq!(outcome.state.enemy.status.as_deref(), Some("SLEEP"));
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "HYPNOSIS".to_string(),
            target: BattleSide::Enemy,
            status: "SLEEP".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage { move_name, .. } if move_name == "HYPNOSIS"
        )));
    }

    #[test]
    fn splash_is_explicit_noop_effect_without_damage() {
        let player = pokemon("MAGIKARP", 50, pokemon_type("WATER"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("splash turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::Splash {
            side: BattleSide::Player,
            move_name: "SPLASH".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SPLASH"
        )));
    }

    #[test]
    fn direct_status_move_does_not_overwrite_existing_status() {
        let player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "HYPNOSIS");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.status = Some("POISON".to_string());
        let mut moves = BTreeMap::new();
        moves.insert(
            "HYPNOSIS".to_string(),
            move_data_with_effect("HYPNOSIS", pokemon_type("PSYCHIC_TYPE"), 0, 0, "SLEEP"),
        );
        moves.insert(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        );
        let mut priorities = move_priorities();
        priorities.effect_priorities.insert("SLEEP".to_string(), 1);
        let mut rng = Random::new(13);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &priorities,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("status turn resolves");

        assert_eq!(outcome.state.enemy.status.as_deref(), Some("POISON"));
        assert!(outcome.events.contains(&BattleEvent::StatusFailed {
            side: BattleSide::Player,
            move_name: "HYPNOSIS".to_string(),
            target: BattleSide::Enemy,
            existing_status: Some("POISON".to_string()),
        }));
    }

    #[test]
    fn toxic_applies_bad_poison_and_initializes_toxic_counter() {
        let player = pokemon("NIDORAN_F", 50, pokemon_type("POISON"), "TOXIC");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "TOXIC".to_string(),
                move_data_with_effect("TOXIC", pokemon_type("POISON"), 0, 100, "TOXIC"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(41);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("toxic resolves");

        assert_eq!(outcome.state.enemy.status.as_deref(), Some("BAD_POISON"));
        assert_eq!(outcome.state.enemy_toxic_turns, 2);
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "TOXIC".to_string(),
            target: BattleSide::Enemy,
            status: "BAD_POISON".to_string(),
        }));
    }

    #[test]
    fn toxic_respects_poison_and_steel_immunity() {
        let player = pokemon("NIDORAN_F", 50, pokemon_type("POISON"), "TOXIC");
        let enemy = pokemon("MAGNEMITE", 40, pokemon_type("STEEL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "TOXIC".to_string(),
                move_data_with_effect("TOXIC", pokemon_type("POISON"), 0, 100, "TOXIC"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(42);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("toxic immunity resolves");

        assert_eq!(outcome.state.enemy.status, None);
        assert_eq!(outcome.state.enemy_toxic_turns, 0);
        assert!(outcome.events.contains(&BattleEvent::StatusImmune {
            side: BattleSide::Player,
            move_name: "TOXIC".to_string(),
            target: BattleSide::Enemy,
            status: "BAD_POISON".to_string(),
            target_type1: pokemon_type("STEEL"),
            target_type2: pokemon_type("STEEL"),
        }));
    }

    #[test]
    fn damaging_secondary_status_effect_applies_after_damage_with_pack_chance() {
        let player = pokemon("CYNDAQUIL", 50, pokemon_type("FIRE"), "EMBER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "EMBER".to_string(),
                move_data_with_effect_chance(
                    "EMBER",
                    pokemon_type("FIRE"),
                    40,
                    100,
                    "BURN_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("secondary status turn resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert_eq!(outcome.state.enemy.status.as_deref(), Some("BURN"));
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "EMBER".to_string(),
            target: BattleSide::Enemy,
            status: "BURN".to_string(),
        }));
    }

    #[test]
    fn damaging_secondary_status_effect_records_missed_pack_chance_roll() {
        let player = pokemon("BEEDRILL", 50, pokemon_type("POISON"), "POISON_STING");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "POISON_STING".to_string(),
                move_data_with_effect_chance(
                    "POISON_STING",
                    pokemon_type("POISON"),
                    15,
                    100,
                    "POISON_HIT",
                    1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("secondary status miss turn resolves");

        assert_eq!(outcome.state.enemy.status, None);
        assert!(
            outcome
                .events
                .contains(&BattleEvent::SecondaryStatusMissed {
                    side: BattleSide::Player,
                    move_name: "POISON_STING".to_string(),
                    target: BattleSide::Enemy,
                    status: "POISON".to_string(),
                    chance_percent: 1,
                    roll: 34,
                })
        );
    }

    #[test]
    fn sacred_fire_uses_exported_burn_chance_after_damage() {
        let player = pokemon("HO_OH", 50, pokemon_type("FIRE"), "SACRED_FIRE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SACRED_FIRE".to_string(),
                move_data_with_effect_chance(
                    "SACRED_FIRE",
                    pokemon_type("FIRE"),
                    100,
                    100,
                    "SACRED_FIRE",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(2);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("sacred fire secondary burn resolves");

        assert_eq!(outcome.state.enemy.status.as_deref(), Some("BURN"));
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "SACRED_FIRE".to_string(),
            target: BattleSide::Enemy,
            status: "BURN".to_string(),
        }));
    }

    #[test]
    fn direct_confusion_move_sets_pack_backed_confusion_turns_without_status() {
        let player = pokemon("ZUBAT", 50, pokemon_type("POISON"), "CONFUSE_RAY");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "CONFUSE_RAY".to_string(),
                move_data_with_effect("CONFUSE_RAY", pokemon_type("GHOST"), 0, 100, "CONFUSE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("direct confusion turn resolves");

        assert_eq!(outcome.state.enemy.status, None);
        assert!((1..=5).contains(&outcome.state.enemy.confusion_turns));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::ConfusionApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                turns
            } if move_name == "CONFUSE_RAY" && (2..=5).contains(turns)
        )));
    }

    #[test]
    fn direct_confusion_move_does_not_overwrite_existing_confusion() {
        let player = pokemon("ZUBAT", 50, pokemon_type("POISON"), "CONFUSE_RAY");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.confusion_turns = 3;
        let moves = BTreeMap::from([
            (
                "CONFUSE_RAY".to_string(),
                move_data_with_effect("CONFUSE_RAY", pokemon_type("GHOST"), 0, 100, "CONFUSE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("confusion failure turn resolves");

        assert!(outcome.events.contains(&BattleEvent::ConfusionFailed {
            side: BattleSide::Player,
            move_name: "CONFUSE_RAY".to_string(),
            target: BattleSide::Enemy,
            turns_remaining: 3,
        }));
    }

    #[test]
    fn swagger_raises_target_attack_and_confuses_target_from_exact_effect() {
        let player = pokemon("QUAGSIRE", 50, pokemon_type("WATER"), "SWAGGER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SWAGGER".to_string(),
                move_data_with_effect("SWAGGER", pokemon_type("NORMAL"), 0, 100, "SWAGGER"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(11);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("swagger resolves");

        assert_eq!(outcome.state.enemy.stat_boosts.get(&Stat::Attack), Some(&2));
        assert!((2..=5).contains(&outcome.state.enemy.confusion_turns));
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "SWAGGER".to_string(),
            target: BattleSide::Enemy,
            stat: Stat::Attack,
            amount: 2,
            stage_before: 0,
            stage_after: 2,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::ConfusionApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                turns
            } if move_name == "SWAGGER" && (2..=5).contains(turns)
        )));
    }

    #[test]
    fn damaging_secondary_confusion_uses_pack_chance_after_damage() {
        let player = pokemon("PSYDUCK", 50, pokemon_type("WATER"), "CONFUSION");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "CONFUSION".to_string(),
                move_data_with_effect_chance(
                    "CONFUSION",
                    pokemon_type("PSYCHIC_TYPE"),
                    50,
                    100,
                    "CONFUSE_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("secondary confusion turn resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!((1..=5).contains(&outcome.state.enemy.confusion_turns));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::ConfusionApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                ..
            } if move_name == "CONFUSION"
        )));
    }

    #[test]
    fn damaging_secondary_confusion_records_missed_pack_chance_roll() {
        let player = pokemon("PSYDUCK", 50, pokemon_type("WATER"), "CONFUSION");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "CONFUSION".to_string(),
                move_data_with_effect_chance(
                    "CONFUSION",
                    pokemon_type("PSYCHIC_TYPE"),
                    50,
                    100,
                    "CONFUSE_HIT",
                    1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("secondary confusion miss resolves");

        assert_eq!(outcome.state.enemy.confusion_turns, 0);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SecondaryConfusionMissed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                chance_percent: 1,
                ..
            } if move_name == "CONFUSION"
        )));
    }

    #[test]
    fn confusion_turn_can_block_move_with_self_damage() {
        let mut player = pokemon("PSYDUCK", 50, pokemon_type("WATER"), "WATER_GUN");
        player.confusion_turns = 3;
        let player_hp = player.hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "WATER_GUN".to_string(),
                move_data("WATER_GUN", pokemon_type("WATER"), 40, 100),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("confusion self damage turn resolves");

        assert_eq!(outcome.state.player.confusion_turns, 2);
        assert!(outcome.state.player.hp < player_hp);
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::ConfusedTurn {
            side: BattleSide::Player,
            move_name: "WATER_GUN".to_string(),
            turns_remaining: 2,
            roll: 0,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::ConfusionSelfDamage {
                side: BattleSide::Player,
                move_name,
                hp_before,
                hp_after,
                ..
            } if move_name == "WATER_GUN" && *hp_before == player_hp && *hp_after < *hp_before
        )));
    }

    #[test]
    fn confusion_expiring_turn_continues_into_selected_move() {
        let mut player = pokemon("PSYDUCK", 90, pokemon_type("WATER"), "WATER_GUN");
        player.confusion_turns = 1;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "WATER_GUN".to_string(),
                move_data("WATER_GUN", pokemon_type("WATER"), 40, 100),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("confusion expiry turn resolves");

        assert_eq!(outcome.state.player.confusion_turns, 0);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::ConfusionEnded {
            side: BattleSide::Player,
            move_name: "WATER_GUN".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "WATER_GUN"
        )));
    }

    #[test]
    fn direct_heal_move_restores_half_max_hp_from_exact_pack_effect() {
        let mut player = pokemon("CHANSEY", 50, pokemon_type("NORMAL"), "SOFTBOILED");
        player.hp = player.max_hp / 4;
        let hp_before = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SOFTBOILED".to_string(),
                move_data_with_effect("SOFTBOILED", pokemon_type("NORMAL"), 0, 100, "HEAL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("direct heal turn resolves");

        let expected_amount = max_hp / 2;
        assert_eq!(outcome.state.player.hp, hp_before + expected_amount);
        assert!(outcome.events.contains(&BattleEvent::HealApplied {
            side: BattleSide::Player,
            move_name: "SOFTBOILED".to_string(),
            hp_before,
            hp_after: hp_before + expected_amount,
            amount: expected_amount,
        }));
    }

    #[test]
    fn direct_heal_move_caps_at_max_hp_and_reports_exact_amount() {
        let mut player = pokemon("MILTANK", 50, pokemon_type("NORMAL"), "MILK_DRINK");
        player.hp = player.max_hp - 3;
        let hp_before = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "MILK_DRINK".to_string(),
                move_data_with_effect("MILK_DRINK", pokemon_type("NORMAL"), 0, 100, "HEAL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("capped heal turn resolves");

        assert_eq!(outcome.state.player.hp, max_hp);
        assert!(outcome.events.contains(&BattleEvent::HealApplied {
            side: BattleSide::Player,
            move_name: "MILK_DRINK".to_string(),
            hp_before,
            hp_after: max_hp,
            amount: 3,
        }));
    }

    #[test]
    fn rest_fully_heals_and_sets_exact_sleep_turns() {
        let mut player = pokemon("SNORLAX", 30, pokemon_type("NORMAL"), "REST");
        player.hp = player.max_hp / 4;
        let hp_before = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "REST".to_string(),
                move_data_with_effect("REST", pokemon_type("PSYCHIC_TYPE"), 0, 100, "HEAL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("rest turn resolves");

        assert_eq!(outcome.state.player.hp, max_hp);
        assert_eq!(outcome.state.player.status.as_deref(), Some("SLEEP"));
        assert_eq!(outcome.state.player.sleep_turns, 2);
        assert!(outcome.events.contains(&BattleEvent::HealApplied {
            side: BattleSide::Player,
            move_name: "REST".to_string(),
            hp_before,
            hp_after: max_hp,
            amount: max_hp - hp_before,
        }));
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "REST".to_string(),
            target: BattleSide::Player,
            status: "SLEEP".to_string(),
        }));
    }

    #[test]
    fn rest_fails_at_full_hp_without_existing_status() {
        let player = pokemon("SNORLAX", 30, pokemon_type("NORMAL"), "REST");
        let hp = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "REST".to_string(),
                move_data_with_effect("REST", pokemon_type("PSYCHIC_TYPE"), 0, 100, "HEAL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("failed rest turn resolves");

        assert_eq!(outcome.state.player.hp, hp);
        assert_eq!(outcome.state.player.status, None);
        assert_eq!(outcome.state.player.sleep_turns, 0);
        assert!(outcome.events.contains(&BattleEvent::HealFailed {
            side: BattleSide::Player,
            move_name: "REST".to_string(),
            hp,
            max_hp,
        }));
    }

    #[test]
    fn heal_bell_clears_active_status_and_toxic_counter() {
        let mut player = pokemon("MILTANK", 50, pokemon_type("NORMAL"), "HEAL_BELL");
        player.status = Some("BAD_POISON".to_string());
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "HEAL_BELL".to_string(),
                move_data_with_effect("HEAL_BELL", pokemon_type("NORMAL"), 0, 100, "HEAL_BELL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut state = battle_state(player, enemy, 1);
        state.player_toxic_turns = 3;
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("heal bell turn resolves");

        assert_eq!(outcome.state.player.status, None);
        assert_eq!(outcome.state.player.sleep_turns, 0);
        assert_eq!(outcome.state.player_toxic_turns, 0);
        assert!(outcome.events.contains(&BattleEvent::StatusHealed {
            side: BattleSide::Player,
            move_name: "HEAL_BELL".to_string(),
            target: BattleSide::Player,
            status_before: "BAD_POISON".to_string(),
        }));
    }

    #[test]
    fn heal_bell_reports_failed_active_status_heal_without_damage() {
        let player = pokemon("MILTANK", 50, pokemon_type("NORMAL"), "HEAL_BELL");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "HEAL_BELL".to_string(),
                move_data_with_effect("HEAL_BELL", pokemon_type("NORMAL"), 0, 100, "HEAL_BELL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("failed heal bell turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::StatusHealFailed {
            side: BattleSide::Player,
            move_name: "HEAL_BELL".to_string(),
            target: BattleSide::Player,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "HEAL_BELL"
        )));
    }

    #[test]
    fn weather_heal_moves_use_crystal_weather_amounts() {
        let synthesis =
            move_data_with_effect("SYNTHESIS", pokemon_type("GRASS"), 0, 100, "SYNTHESIS");
        let morning_sun =
            move_data_with_effect("MORNING_SUN", pokemon_type("NORMAL"), 0, 100, "MORNING_SUN");
        let moonlight =
            move_data_with_effect("MOONLIGHT", pokemon_type("NORMAL"), 0, 100, "MOONLIGHT");
        let recover = move_data_with_effect("RECOVER", pokemon_type("NORMAL"), 0, 100, "HEAL");

        assert_eq!(direct_heal_amount(96, &synthesis, Weather::None), 48);
        assert_eq!(direct_heal_amount(96, &synthesis, Weather::Sun), 64);
        assert_eq!(direct_heal_amount(96, &morning_sun, Weather::Rain), 24);
        assert_eq!(direct_heal_amount(96, &moonlight, Weather::Sandstorm), 24);
        assert_eq!(direct_heal_amount(96, &recover, Weather::Sun), 48);
    }

    #[test]
    fn direct_heal_move_fails_without_inferred_overheal() {
        let player = pokemon("STARYU", 50, pokemon_type("WATER"), "RECOVER");
        let hp = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "RECOVER".to_string(),
                move_data_with_effect("RECOVER", pokemon_type("NORMAL"), 0, 100, "HEAL"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("failed heal turn resolves");

        assert_eq!(outcome.state.player.hp, hp);
        assert!(outcome.events.contains(&BattleEvent::HealFailed {
            side: BattleSide::Player,
            move_name: "RECOVER".to_string(),
            hp,
            max_hp,
        }));
    }

    #[test]
    fn pain_split_averages_current_hp_between_user_and_target() {
        let mut player = pokemon("MISDREAVUS", 90, pokemon_type("GHOST"), "PAIN_SPLIT");
        player.hp = 20;
        let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 80;
        let moves = BTreeMap::from([
            (
                "PAIN_SPLIT".to_string(),
                move_data_with_effect("PAIN_SPLIT", pokemon_type("NORMAL"), 0, 100, "PAIN_SPLIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(32);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("pain split turn resolves");

        assert_eq!(outcome.state.player.hp, 50);
        assert_eq!(outcome.state.enemy.hp, 50);
        assert!(outcome.events.contains(&BattleEvent::PainSplitApplied {
            side: BattleSide::Player,
            move_name: "PAIN_SPLIT".to_string(),
            target: BattleSide::Enemy,
            user_hp_before: 20,
            user_hp_after: 50,
            target_hp_before: 80,
            target_hp_after: 50,
        }));
    }

    #[test]
    fn pain_split_caps_each_side_at_its_own_max_hp() {
        let mut player = pokemon("MISDREAVUS", 90, pokemon_type("GHOST"), "PAIN_SPLIT");
        player.hp = 100;
        player.max_hp = 100;
        let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 90;
        enemy.max_hp = 60;
        let moves = BTreeMap::from([
            (
                "PAIN_SPLIT".to_string(),
                move_data_with_effect("PAIN_SPLIT", pokemon_type("NORMAL"), 0, 100, "PAIN_SPLIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(33);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("capped pain split turn resolves");

        assert_eq!(outcome.state.player.hp, 95);
        assert_eq!(outcome.state.enemy.hp, 60);
        assert!(outcome.events.contains(&BattleEvent::PainSplitApplied {
            side: BattleSide::Player,
            move_name: "PAIN_SPLIT".to_string(),
            target: BattleSide::Enemy,
            user_hp_before: 100,
            user_hp_after: 95,
            target_hp_before: 90,
            target_hp_after: 60,
        }));
    }

    #[test]
    fn perish_song_applies_exact_count_to_both_active_pokemon() {
        let player = pokemon("MISDREAVUS", 50, pokemon_type("GHOST"), "PERISH_SONG");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "PERISH_SONG".to_string(),
                move_data_with_effect("PERISH_SONG", pokemon_type("NORMAL"), 0, 100, "PERISH_SONG"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("perish song turn resolves");

        assert_eq!(outcome.state.player.perish_song_turns, 3);
        assert_eq!(outcome.state.enemy.perish_song_turns, 3);
        assert!(outcome.events.contains(&BattleEvent::PerishSongApplied {
            side: BattleSide::Player,
            move_name: "PERISH_SONG".to_string(),
            target: BattleSide::Player,
            turns: 4,
        }));
        assert!(outcome.events.contains(&BattleEvent::PerishSongApplied {
            side: BattleSide::Player,
            move_name: "PERISH_SONG".to_string(),
            target: BattleSide::Enemy,
            turns: 4,
        }));
        assert!(outcome.events.contains(&BattleEvent::PerishSongCount {
            side: BattleSide::Player,
            turns_remaining: 3,
        }));
        assert!(outcome.events.contains(&BattleEvent::PerishSongCount {
            side: BattleSide::Enemy,
            turns_remaining: 3,
        }));
    }

    #[test]
    fn perish_song_does_not_overwrite_existing_count() {
        let mut player = pokemon("MISDREAVUS", 50, pokemon_type("GHOST"), "PERISH_SONG");
        player.perish_song_turns = 2;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.perish_song_turns = 1;
        let moves = BTreeMap::from([
            (
                "PERISH_SONG".to_string(),
                move_data_with_effect("PERISH_SONG", pokemon_type("NORMAL"), 0, 100, "PERISH_SONG"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("active perish song turn resolves");

        assert_eq!(outcome.state.player.perish_song_turns, 1);
        assert_eq!(outcome.state.enemy.perish_song_turns, 0);
        assert!(outcome.events.contains(&BattleEvent::PerishSongFailed {
            side: BattleSide::Player,
            move_name: "PERISH_SONG".to_string(),
            target: BattleSide::Player,
            turns_remaining: 2,
        }));
        assert!(outcome.events.contains(&BattleEvent::PerishSongFailed {
            side: BattleSide::Player,
            move_name: "PERISH_SONG".to_string(),
            target: BattleSide::Enemy,
            turns_remaining: 1,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
    }

    #[test]
    fn perish_song_countdown_faints_when_counter_reaches_zero() {
        let mut player = pokemon("MISDREAVUS", 50, pokemon_type("GHOST"), "TACKLE");
        player.perish_song_turns = 1;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.perish_song_turns = 1;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("perish countdown resolves");

        assert_eq!(outcome.state.player.hp, 0);
        assert_eq!(outcome.state.enemy.hp, 0);
        assert!(outcome.events.contains(&BattleEvent::PerishSongCount {
            side: BattleSide::Player,
            turns_remaining: 0,
        }));
        assert!(outcome.events.contains(&BattleEvent::PerishSongCount {
            side: BattleSide::Enemy,
            turns_remaining: 0,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Player,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
    }

    #[test]
    fn focus_energy_move_sets_existing_focus_energy_state() {
        let player = pokemon("PIDGEY", 50, pokemon_type("FLYING"), "FOCUS_ENERGY");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "FOCUS_ENERGY".to_string(),
                move_data_with_effect(
                    "FOCUS_ENERGY",
                    pokemon_type("NORMAL"),
                    0,
                    100,
                    "FOCUS_ENERGY",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("focus energy turn resolves");

        assert!(outcome.state.player.focus_energy);
        assert!(outcome.events.contains(&BattleEvent::FocusEnergyApplied {
            side: BattleSide::Player,
            move_name: "FOCUS_ENERGY".to_string(),
        }));
    }

    #[test]
    fn focus_energy_move_reports_failure_when_already_focused() {
        let mut player = pokemon("PIDGEY", 50, pokemon_type("FLYING"), "FOCUS_ENERGY");
        player.focus_energy = true;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "FOCUS_ENERGY".to_string(),
                move_data_with_effect(
                    "FOCUS_ENERGY",
                    pokemon_type("NORMAL"),
                    0,
                    100,
                    "FOCUS_ENERGY",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("repeated focus energy turn resolves");

        assert!(outcome.state.player.focus_energy);
        assert!(outcome.events.contains(&BattleEvent::FocusEnergyFailed {
            side: BattleSide::Player,
            move_name: "FOCUS_ENERGY".to_string(),
        }));
    }

    #[test]
    fn belly_drum_costs_half_hp_and_maximizes_attack() {
        let player = pokemon("POLIWRATH", 50, pokemon_type("WATER"), "BELLY_DRUM");
        let player_hp = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "BELLY_DRUM".to_string(),
                move_data_with_effect("BELLY_DRUM", pokemon_type("NORMAL"), 0, 100, "BELLY_DRUM"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("belly drum turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp - max_hp / 2);
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack),
            Some(&6)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "BELLY_DRUM".to_string(),
            target: BattleSide::Player,
            stat: Stat::Attack,
            amount: 6,
            stage_before: 0,
            stage_after: 6,
        }));
    }

    #[test]
    fn belly_drum_below_half_hp_sharply_boosts_then_fails() {
        let mut player = pokemon("POLIWRATH", 50, pokemon_type("WATER"), "BELLY_DRUM");
        player.hp = player.max_hp / 2 - 1;
        let player_hp = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "BELLY_DRUM".to_string(),
                move_data_with_effect("BELLY_DRUM", pokemon_type("NORMAL"), 0, 100, "BELLY_DRUM"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("low-hp belly drum turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack),
            Some(&2)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "BELLY_DRUM".to_string(),
            target: BattleSide::Player,
            stat: Stat::Attack,
            amount: 2,
            stage_before: 0,
            stage_after: 2,
        }));
        assert!(outcome.events.contains(&BattleEvent::HealFailed {
            side: BattleSide::Player,
            move_name: "BELLY_DRUM".to_string(),
            hp: player_hp,
            max_hp,
        }));
    }

    #[test]
    fn defense_curl_raises_defense_stage() {
        let player = pokemon("JIGGLYPUFF", 50, pokemon_type("NORMAL"), "DEFENSE_CURL");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "DEFENSE_CURL".to_string(),
                move_data_with_effect(
                    "DEFENSE_CURL",
                    pokemon_type("NORMAL"),
                    0,
                    100,
                    "DEFENSE_CURL",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("defense curl turn resolves");

        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Defense),
            Some(&1)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "DEFENSE_CURL".to_string(),
            target: BattleSide::Player,
            stat: Stat::Defense,
            amount: 1,
            stage_before: 0,
            stage_after: 1,
        }));
    }

    #[test]
    fn non_ghost_curse_raises_attack_defense_and_lowers_speed() {
        let player = pokemon("SLOWPOKE", 30, pokemon_type("WATER"), "CURSE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "CURSE".to_string(),
                move_data_with_effect("CURSE", pokemon_type("UNKNOWN_T"), 0, 100, "CURSE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("non-ghost curse turn resolves");

        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack),
            Some(&1)
        );
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Defense),
            Some(&1)
        );
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Speed),
            Some(&-1)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "CURSE".to_string(),
            target: BattleSide::Player,
            stat: Stat::Speed,
            amount: -1,
            stage_before: 0,
            stage_after: -1,
        }));
    }

    #[test]
    fn ghost_curse_sacrifices_hp_marks_target_and_deals_residual() {
        let mut player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "CURSE");
        player.hp = 80;
        player.max_hp = 80;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 64;
        enemy.max_hp = 64;
        let moves = BTreeMap::from([
            (
                "CURSE".to_string(),
                move_data_with_effect("CURSE", pokemon_type("UNKNOWN_T"), 0, 100, "CURSE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("ghost curse turn resolves");

        assert_eq!(outcome.state.player.hp, 40);
        assert_eq!(outcome.state.enemy.hp, 48);
        assert_eq!(outcome.state.enemy_curse_source, Some(BattleSide::Player));
        assert!(outcome.events.contains(&BattleEvent::CurseApplied {
            side: BattleSide::Player,
            move_name: "CURSE".to_string(),
            target: BattleSide::Enemy,
            hp_cost: 40,
            hp_before: 80,
            hp_after: 40,
        }));
        assert!(outcome.events.contains(&BattleEvent::CurseDamage {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            damage: 16,
            hp_before: 64,
            hp_after: 48,
        }));
    }

    #[test]
    fn ghost_curse_fails_when_target_is_already_cursed() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "CURSE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_curse_source = Some(BattleSide::Player);
        let moves = BTreeMap::from([
            (
                "CURSE".to_string(),
                move_data_with_effect("CURSE", pokemon_type("UNKNOWN_T"), 0, 100, "CURSE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("duplicate ghost curse turn resolves");

        assert!(outcome.events.contains(&BattleEvent::CurseFailed {
            side: BattleSide::Player,
            move_name: "CURSE".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn switching_clears_curse_from_cursed_target() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "TACKLE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_curse_source = Some(BattleSide::Player);
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("curse target switch turn resolves");

        assert_eq!(outcome.state.player_curse_source, None);
        assert_eq!(outcome.state.enemy_curse_source, None);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::CurseDamage { .. }))
        );
    }

    #[test]
    fn leech_hit_heals_half_damage_dealt_after_damage() {
        let mut player = pokemon("ODDISH", 50, pokemon_type("GRASS"), "ABSORB");
        player.hp = player.max_hp / 2;
        let hp_before = player.hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "ABSORB".to_string(),
                move_data_with_effect("ABSORB", pokemon_type("GRASS"), 20, 100, "LEECH_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("drain turn resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.state.player.hp > hp_before);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::HpDrained {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                damage,
                hp_before: event_hp_before,
                hp_after,
                amount,
            } if move_name == "ABSORB"
                && *damage > 0
                && *event_hp_before == hp_before
                && *hp_after == hp_before + *amount
                && *amount == (*damage).div_ceil(2)
        )));
    }

    #[test]
    fn leech_hit_caps_heal_at_missing_hp() {
        let mut player = pokemon("ODDISH", 50, pokemon_type("GRASS"), "MEGA_DRAIN");
        player.hp = player.max_hp - 1;
        let hp_before = player.hp;
        let max_hp = player.max_hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "MEGA_DRAIN".to_string(),
                move_data_with_effect("MEGA_DRAIN", pokemon_type("GRASS"), 40, 100, "LEECH_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("capped drain turn resolves");

        assert_eq!(outcome.state.player.hp, max_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::HpDrained {
                side: BattleSide::Player,
                move_name,
                hp_before: event_hp_before,
                hp_after,
                amount: 1,
                ..
            } if move_name == "MEGA_DRAIN" && *event_hp_before == hp_before && *hp_after == max_hp
        )));
    }

    #[test]
    fn dream_eater_drains_hp_only_against_sleeping_target() {
        let mut player = pokemon("DROWZEE", 50, pokemon_type("PSYCHIC_TYPE"), "DREAM_EATER");
        player.hp = player.max_hp / 2;
        let player_hp = player.hp;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.status = Some("SLEEP".to_string());
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "DREAM_EATER".to_string(),
                move_data_with_effect(
                    "DREAM_EATER",
                    pokemon_type("NORMAL"),
                    100,
                    100,
                    "DREAM_EATER",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("dream eater turn resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.state.player.hp > player_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::HpDrained { move_name, .. } if move_name == "DREAM_EATER"
        )));
    }

    #[test]
    fn dream_eater_fails_without_sleeping_target() {
        let player = pokemon("DROWZEE", 50, pokemon_type("PSYCHIC_TYPE"), "DREAM_EATER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "DREAM_EATER".to_string(),
                move_data_with_effect(
                    "DREAM_EATER",
                    pokemon_type("NORMAL"),
                    100,
                    100,
                    "DREAM_EATER",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("failed dream eater turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "DREAM_EATER".to_string(),
        }));
    }

    #[test]
    fn false_swipe_damage_cannot_faint_target_above_one_hp() {
        let player = pokemon("SCYTHER", 50, pokemon_type("BUG"), "FALSE_SWIPE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 2;
        let moves = BTreeMap::from([
            (
                "FALSE_SWIPE".to_string(),
                move_data_with_effect(
                    "FALSE_SWIPE",
                    pokemon_type("NORMAL"),
                    200,
                    100,
                    "FALSE_SWIPE",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("false swipe turn resolves");

        assert_eq!(outcome.state.enemy.hp, 1);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage: 1,
                defender_hp_before: 2,
                defender_hp_after: 1,
                ..
            } if move_name == "FALSE_SWIPE"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Fainted {
                side: BattleSide::Enemy
            }
        )));
    }

    #[test]
    fn future_sight_queues_stored_damage_on_target_side() {
        let player = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "FUTURE_SIGHT");
        let enemy = pokemon("RATTATA", 60, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "FUTURE_SIGHT".to_string(),
                move_data_with_effect(
                    "FUTURE_SIGHT",
                    pokemon_type("PSYCHIC_TYPE"),
                    80,
                    100,
                    "FUTURE_SIGHT",
                ),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(128);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("future sight queue turn resolves");

        let queued = outcome
            .state
            .enemy_future_sight
            .as_ref()
            .expect("future sight queued on enemy side");
        assert_eq!(queued.source, BattleSide::Player);
        assert_eq!(queued.move_name, "FUTURE_SIGHT");
        assert_eq!(queued.turns_remaining, 2);
        assert!(queued.damage > 0);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::FutureSightQueued {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                damage,
                turns: 3,
            } if move_name == "FUTURE_SIGHT" && *damage > 0
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::FutureSightCount {
                side: BattleSide::Enemy,
                source: BattleSide::Player,
                move_name,
                turns_remaining: 2,
            } if move_name == "FUTURE_SIGHT"
        )));
    }

    #[test]
    fn future_sight_fails_when_target_side_already_has_queued_attack() {
        let player = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "FUTURE_SIGHT");
        let enemy = pokemon("RATTATA", 60, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "FUTURE_SIGHT".to_string(),
                move_data_with_effect(
                    "FUTURE_SIGHT",
                    pokemon_type("PSYCHIC_TYPE"),
                    80,
                    100,
                    "FUTURE_SIGHT",
                ),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(129);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_future_sight = Some(BattleFutureSightState {
            source: BattleSide::Enemy,
            move_name: "FUTURE_SIGHT".to_string(),
            turns_remaining: 3,
            damage: 11,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("duplicate future sight turn resolves");

        assert_eq!(
            outcome.state.enemy_future_sight,
            Some(BattleFutureSightState {
                source: BattleSide::Enemy,
                move_name: "FUTURE_SIGHT".to_string(),
                turns_remaining: 2,
                damage: 11,
            })
        );
        assert!(outcome.events.contains(&BattleEvent::FutureSightFailed {
            side: BattleSide::Player,
            move_name: "FUTURE_SIGHT".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn future_sight_hits_when_countdown_reaches_zero() {
        let player = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "SPLASH");
        let mut enemy = pokemon("RATTATA", 60, pokemon_type("NORMAL"), "SPLASH");
        enemy.hp = 20;
        let moves = BTreeMap::from([(
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        )]);
        let mut rng = Random::new(130);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_future_sight = Some(BattleFutureSightState {
            source: BattleSide::Player,
            move_name: "FUTURE_SIGHT".to_string(),
            turns_remaining: 1,
            damage: 17,
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("future sight damage turn resolves");

        assert_eq!(outcome.state.enemy_future_sight, None);
        assert_eq!(outcome.state.enemy.hp, 3);
        assert!(outcome.events.contains(&BattleEvent::FutureSightDamage {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            move_name: "FUTURE_SIGHT".to_string(),
            damage: 17,
            hp_before: 20,
            hp_after: 3,
        }));
    }

    #[test]
    fn rapid_spin_clears_user_trap_leech_seed_and_spikes_after_damage() {
        let player = pokemon("STARYU", 50, pokemon_type("WATER"), "RAPID_SPIN");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "RAPID_SPIN".to_string(),
                move_data_with_effect("RAPID_SPIN", pokemon_type("NORMAL"), 20, 100, "RAPID_SPIN"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_leech_seed_source = Some(BattleSide::Enemy);
        state.player_spikes = true;
        state.player_trap = Some(BattleTrapState {
            source: BattleSide::Enemy,
            move_name: "BIND".to_string(),
            turns_remaining: 3,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("rapid spin turn resolves");

        assert_eq!(outcome.state.player_trap, None);
        assert_eq!(outcome.state.player_leech_seed_source, None);
        assert!(!outcome.state.player_spikes);
        assert!(outcome.events.contains(&BattleEvent::RapidSpinCleared {
            side: BattleSide::Player,
            move_name: "RAPID_SPIN".to_string(),
            cleared_trap: true,
            cleared_leech_seed: true,
            cleared_spikes: true,
        }));
    }

    #[test]
    fn rapid_spin_does_not_clear_conditions_without_damage() {
        let player = pokemon("STARYU", 50, pokemon_type("WATER"), "RAPID_SPIN");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "RAPID_SPIN".to_string(),
                move_data_with_effect("RAPID_SPIN", pokemon_type("NORMAL"), 20, 100, "RAPID_SPIN"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        let trap = BattleTrapState {
            source: BattleSide::Enemy,
            move_name: "BIND".to_string(),
            turns_remaining: 3,
        };
        state.player_leech_seed_source = Some(BattleSide::Enemy);
        state.player_spikes = true;
        state.player_trap = Some(trap.clone());

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("no-effect rapid spin turn resolves");

        assert_eq!(outcome.state.player_trap, Some(trap));
        assert_eq!(
            outcome.state.player_leech_seed_source,
            Some(BattleSide::Enemy)
        );
        assert!(outcome.state.player_spikes);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::RapidSpinCleared { .. }))
        );
    }

    #[test]
    fn counter_reflects_physical_damage_after_opponent_moves() {
        let player = pokemon("MACHOP", 50, pokemon_type("FIGHTING"), "COUNTER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "COUNTER".to_string(),
                move_data_with_effect("COUNTER", pokemon_type("FIGHTING"), 1, 100, "COUNTER"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("counter turn resolves");

        assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
        let source_damage = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Enemy,
                    move_name,
                    damage,
                    ..
                } if move_name == "TACKLE" => Some(*damage),
                _ => None,
            })
            .expect("enemy tackle damages before counter");
        let reflected = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::CounterDamage {
                    side: BattleSide::Player,
                    move_name,
                    target: BattleSide::Enemy,
                    countered_move,
                    category: BattleDamageCategory::Physical,
                    source_damage: reflected_source_damage,
                    damage,
                    defender_hp_before,
                    defender_hp_after,
                } if move_name == "COUNTER" && countered_move == "TACKLE" => Some((
                    *reflected_source_damage,
                    *damage,
                    *defender_hp_before,
                    *defender_hp_after,
                )),
                _ => None,
            })
            .expect("counter reflects tackle damage");
        assert_eq!(reflected.0, source_damage);
        assert_eq!(reflected.1, source_damage * 2);
        assert_eq!(reflected.2 - reflected.3, reflected.1);
        assert_eq!(outcome.state.player_last_damage, None);
        assert_eq!(outcome.state.enemy_last_damage, None);
    }

    #[test]
    fn counter_fails_after_special_damage() {
        let player = pokemon("MACHOP", 50, pokemon_type("FIGHTING"), "COUNTER");
        let enemy = pokemon("CYNDAQUIL", 40, pokemon_type("FIRE"), "EMBER");
        let moves = BTreeMap::from([
            (
                "COUNTER".to_string(),
                move_data_with_effect("COUNTER", pokemon_type("FIGHTING"), 1, 100, "COUNTER"),
            ),
            (
                "EMBER".to_string(),
                move_data("EMBER", pokemon_type("FIRE"), 40, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("counter failure turn resolves");

        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::NoEffect {
                side: BattleSide::Player,
                move_name
            } if move_name == "COUNTER"
        )));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::CounterDamage { .. }))
        );
    }

    #[test]
    fn mirror_coat_reflects_special_damage_after_opponent_moves() {
        let player = pokemon("ESPEON", 50, pokemon_type("PSYCHIC_TYPE"), "MIRROR_COAT");
        let enemy = pokemon("CYNDAQUIL", 40, pokemon_type("FIRE"), "EMBER");
        let moves = BTreeMap::from([
            (
                "MIRROR_COAT".to_string(),
                move_data_with_effect(
                    "MIRROR_COAT",
                    pokemon_type("PSYCHIC_TYPE"),
                    1,
                    100,
                    "MIRROR_COAT",
                ),
            ),
            (
                "EMBER".to_string(),
                move_data("EMBER", pokemon_type("FIRE"), 40, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("mirror coat turn resolves");

        let source_damage = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Enemy,
                    move_name,
                    damage,
                    ..
                } if move_name == "EMBER" => Some(*damage),
                _ => None,
            })
            .expect("enemy ember damages before mirror coat");
        let reflected = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::CounterDamage {
                    side: BattleSide::Player,
                    move_name,
                    target: BattleSide::Enemy,
                    countered_move,
                    category: BattleDamageCategory::Special,
                    source_damage: reflected_source_damage,
                    damage,
                    defender_hp_before,
                    defender_hp_after,
                } if move_name == "MIRROR_COAT" && countered_move == "EMBER" => Some((
                    *reflected_source_damage,
                    *damage,
                    *defender_hp_before,
                    *defender_hp_after,
                )),
                _ => None,
            })
            .expect("mirror coat reflects ember damage");
        assert_eq!(reflected.0, source_damage);
        assert_eq!(reflected.1, source_damage * 2);
        assert_eq!(reflected.2 - reflected.3, reflected.1);
    }

    #[test]
    fn foresight_identifies_target_and_allows_normal_hit_on_ghost() {
        let player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "FORESIGHT");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "FORESIGHT".to_string(),
                move_data_with_effect("FORESIGHT", pokemon_type("NORMAL"), 0, 100, "FORESIGHT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let identified = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("foresight turn resolves");

        assert!(identified.state.enemy_identified);
        assert!(identified.events.contains(&BattleEvent::ForesightApplied {
            side: BattleSide::Player,
            move_name: "FORESIGHT".to_string(),
            target: BattleSide::Enemy,
        }));

        let mut next_state = identified.state;
        next_state.player.moves[0].name = "TACKLE".to_string();
        let hit = resolve_battle_turn(
            next_state,
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
        .expect("identified ghost can be hit");

        assert!(hit.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                ..
            } if move_name == "TACKLE" && *damage > 0
        )));
    }

    #[test]
    fn normal_hit_still_fails_against_unidentified_ghost() {
        let player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("unidentified ghost immunity turn resolves");

        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::NoEffect {
                side: BattleSide::Player,
                move_name
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn repeated_foresight_reports_failure_without_changing_state() {
        let player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "FORESIGHT");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "FORESIGHT".to_string(),
                move_data_with_effect("FORESIGHT", pokemon_type("NORMAL"), 0, 100, "FORESIGHT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_identified = true;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("repeated foresight turn resolves");

        assert!(outcome.state.enemy_identified);
        assert!(outcome.events.contains(&BattleEvent::ForesightFailed {
            side: BattleSide::Player,
            move_name: "FORESIGHT".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn all_up_hit_raises_user_battle_stats_after_damage() {
        let player = pokemon("DUNSPARCE", 50, pokemon_type("NORMAL"), "ANCIENTPOWER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "ANCIENTPOWER".to_string(),
                move_data_with_effect(
                    "ANCIENTPOWER",
                    pokemon_type("NORMAL"),
                    60,
                    100,
                    "ALL_UP_HIT",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("all-up hit turn resolves");

        for stat in [
            Stat::Attack,
            Stat::Defense,
            Stat::Speed,
            Stat::SpecialAttack,
            Stat::SpecialDefense,
        ] {
            assert_eq!(outcome.state.player.stat_boosts.get(&stat), Some(&1));
            assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
                side: BattleSide::Player,
                move_name: "ANCIENTPOWER".to_string(),
                target: BattleSide::Player,
                stat,
                amount: 1,
                stage_before: 0,
                stage_after: 1,
            }));
        }
    }

    #[test]
    fn post_damage_stat_effect_does_not_apply_without_damage() {
        let player = pokemon("DUNSPARCE", 50, pokemon_type("NORMAL"), "ANCIENTPOWER");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "ANCIENTPOWER".to_string(),
                move_data_with_effect(
                    "ANCIENTPOWER",
                    pokemon_type("NORMAL"),
                    60,
                    100,
                    "ALL_UP_HIT",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("no-effect all-up hit turn resolves");

        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "ANCIENTPOWER".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::StatStageChanged { .. }))
        );
    }

    #[test]
    fn recoil_hit_damages_user_after_damage() {
        let player = pokemon("RHYHORN", 50, pokemon_type("ROCK"), "TAKE_DOWN");
        let player_hp = player.hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "TAKE_DOWN".to_string(),
                move_data_with_effect("TAKE_DOWN", pokemon_type("NORMAL"), 90, 100, "RECOIL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("recoil turn resolves");

        assert!(outcome.state.player.hp < player_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RecoilDamage {
                side: BattleSide::Player,
                move_name,
                damage_dealt,
                recoil_damage,
                hp_before,
                hp_after,
            } if move_name == "TAKE_DOWN"
                && *damage_dealt > 0
                && *recoil_damage == (*damage_dealt / 4).max(1)
                && *hp_before == player_hp
                && *hp_after == player_hp - *recoil_damage
        )));
    }

    #[test]
    fn recoil_hit_can_faint_user_after_damage() {
        let mut player = pokemon("RHYHORN", 50, pokemon_type("ROCK"), "DOUBLE_EDGE");
        player.hp = 1;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "DOUBLE_EDGE".to_string(),
                move_data_with_effect(
                    "DOUBLE_EDGE",
                    pokemon_type("NORMAL"),
                    120,
                    100,
                    "RECOIL_HIT",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("fainting recoil turn resolves");

        assert_eq!(outcome.state.player.hp, 0);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RecoilDamage {
                side: BattleSide::Player,
                move_name,
                recoil_damage: 1,
                hp_before: 1,
                hp_after: 0,
                ..
            } if move_name == "DOUBLE_EDGE"
        )));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Player,
        }));
    }

    #[test]
    fn selfdestruct_effect_faints_user_after_successful_damage() {
        let player = pokemon("VOLTORB", 90, pokemon_type("ELECTRIC"), "SELFDESTRUCT");
        let player_hp = player.hp;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SELFDESTRUCT".to_string(),
                move_data_with_effect(
                    "SELFDESTRUCT",
                    pokemon_type("NORMAL"),
                    200,
                    100,
                    "SELFDESTRUCT",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("selfdestruct turn resolves");

        assert_eq!(outcome.state.player.hp, 0);
        assert!(outcome.events.contains(&BattleEvent::SelfdestructDamage {
            side: BattleSide::Player,
            move_name: "SELFDESTRUCT".to_string(),
            hp_before: player_hp,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Player,
        }));
    }

    #[test]
    fn selfdestruct_effect_does_not_fire_without_effective_damage() {
        let player = pokemon("VOLTORB", 90, pokemon_type("ELECTRIC"), "SELFDESTRUCT");
        let player_hp = player.hp;
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SELFDESTRUCT".to_string(),
                move_data_with_effect(
                    "SELFDESTRUCT",
                    pokemon_type("NORMAL"),
                    200,
                    100,
                    "SELFDESTRUCT",
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("ineffective selfdestruct turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::SelfdestructDamage { .. }))
        );
    }

    #[test]
    fn direct_stat_stage_move_uses_pack_stat_and_amount_fields() {
        let player = pokemon("SANDSHREW", 50, pokemon_type("GROUND"), "SAND_ATTACK");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SAND_ATTACK".to_string(),
                move_data_with_stat(
                    "SAND_ATTACK",
                    pokemon_type("GROUND"),
                    0,
                    0,
                    "ACCURACY_DOWN",
                    0,
                    Stat::Accuracy,
                    -1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("direct stat move resolves");

        assert_eq!(
            outcome.state.enemy.stat_boosts.get(&Stat::Accuracy),
            Some(&-1)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "SAND_ATTACK".to_string(),
            target: BattleSide::Enemy,
            stat: Stat::Accuracy,
            amount: -1,
            stage_before: 0,
            stage_after: -1,
        }));
    }

    #[test]
    fn exported_stat_stage_effects_have_explicit_priority_entries() {
        let priorities = move_priorities();
        for effect in [
            "ATTACK_UP",
            "ATTACK_UP_HIT",
            "ATTACK_UP_2",
            "ATTACK_DOWN",
            "ATTACK_DOWN_HIT",
            "ATTACK_DOWN_2",
            "ACCURACY_DOWN",
            "ACCURACY_DOWN_HIT",
            "DEFENSE_DOWN",
            "DEFENSE_DOWN_HIT",
            "DEFENSE_DOWN_2",
            "DEFENSE_UP",
            "DEFENSE_UP_HIT",
            "DEFENSE_UP_2",
            "SPEED_UP",
            "SPEED_UP_2",
            "SPEED_DOWN",
            "SPEED_DOWN_2",
            "SPECIAL_ATTACK_UP",
            "SPEED_DOWN_HIT",
            "EVASION_UP",
            "EVASION_DOWN",
            "EVASION_DOWN_HIT",
        ] {
            let move_data = move_data_with_stat(
                effect,
                pokemon_type("NORMAL"),
                if effect.ends_with("_HIT") { 40 } else { 0 },
                100,
                effect,
                100,
                Stat::Attack,
                if effect.contains("_UP") { 1 } else { -1 },
            );
            assert_eq!(
                move_priority(&move_data, &priorities),
                Ok(1),
                "missing explicit priority for {effect}"
            );
        }
    }

    #[test]
    fn exported_secondary_status_and_flinch_effects_have_explicit_priority_entries() {
        let priorities = move_priorities();
        for effect in [
            "BURN_HIT",
            "FREEZE_HIT",
            "PARALYZE_HIT",
            "POISON_HIT",
            "POISON_MULTI_HIT",
            "FLINCH_HIT",
        ] {
            let move_data = move_data_with_effect(effect, pokemon_type("NORMAL"), 40, 100, effect);
            assert_eq!(
                move_priority(&move_data, &priorities),
                Ok(1),
                "missing explicit priority for {effect}"
            );
        }
    }

    #[test]
    fn direct_positive_stat_stage_move_targets_user_from_pack_amount() {
        let player = pokemon("SCYTHER", 50, pokemon_type("BUG"), "SWORDS_DANCE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SWORDS_DANCE".to_string(),
                move_data_with_stat(
                    "SWORDS_DANCE",
                    pokemon_type("NORMAL"),
                    0,
                    100,
                    "ATTACK_UP_2",
                    0,
                    Stat::Attack,
                    2,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("direct positive stat move resolves");

        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack),
            Some(&2)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "SWORDS_DANCE".to_string(),
            target: BattleSide::Player,
            stat: Stat::Attack,
            amount: 2,
            stage_before: 0,
            stage_after: 2,
        }));
    }

    #[test]
    fn damaging_stat_stage_move_applies_after_damage_with_pack_chance() {
        let player = pokemon("DIGLETT", 50, pokemon_type("NORMAL"), "MUD_SLAP");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "MUD_SLAP".to_string(),
                move_data_with_stat(
                    "MUD_SLAP",
                    pokemon_type("NORMAL"),
                    20,
                    100,
                    "ACCURACY_DOWN_HIT",
                    100,
                    Stat::Accuracy,
                    -1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("damaging stat move resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert_eq!(
            outcome.state.enemy.stat_boosts.get(&Stat::Accuracy),
            Some(&-1)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "MUD_SLAP".to_string(),
            target: BattleSide::Enemy,
            stat: Stat::Accuracy,
            amount: -1,
            stage_before: 0,
            stage_after: -1,
        }));
    }

    #[test]
    fn damaging_stat_stage_move_records_missed_pack_chance_roll() {
        let player = pokemon("DIGLETT", 50, pokemon_type("NORMAL"), "MUD_SLAP");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "MUD_SLAP".to_string(),
                move_data_with_stat(
                    "MUD_SLAP",
                    pokemon_type("NORMAL"),
                    20,
                    100,
                    "ACCURACY_DOWN_HIT",
                    1,
                    Stat::Accuracy,
                    -1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("damaging stat miss resolves");

        assert_eq!(
            outcome.state.enemy.stat_boosts.get(&Stat::Accuracy),
            Some(&0)
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SecondaryStatStageMissed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                stat: Stat::Accuracy,
                amount: -1,
                chance_percent: 1,
                ..
            } if move_name == "MUD_SLAP"
        )));
    }

    #[test]
    fn defense_up_hit_respects_secondary_pack_chance() {
        let player = pokemon("STEELIX", 30, pokemon_type("STEEL"), "STEEL_WING");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "STEEL_WING".to_string(),
                move_data_with_stat(
                    "STEEL_WING",
                    pokemon_type("STEEL"),
                    70,
                    100,
                    "DEFENSE_UP_HIT",
                    1,
                    Stat::Defense,
                    1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("defense up hit resolves");

        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Defense),
            Some(&0)
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SecondaryStatStageMissed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Player,
                stat: Stat::Defense,
                amount: 1,
                chance_percent: 1,
                ..
            } if move_name == "STEEL_WING"
        )));
    }

    #[test]
    fn all_up_hit_respects_secondary_pack_chance() {
        let player = pokemon("KABUTO", 30, pokemon_type("ROCK"), "ANCIENTPOWER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "ANCIENTPOWER".to_string(),
                move_data_with_effect_chance(
                    "ANCIENTPOWER",
                    pokemon_type("ROCK"),
                    60,
                    100,
                    "ALL_UP_HIT",
                    1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("all up hit resolves");

        for stat in [
            Stat::Attack,
            Stat::Defense,
            Stat::Speed,
            Stat::SpecialAttack,
            Stat::SpecialDefense,
        ] {
            assert_eq!(outcome.state.player.stat_boosts.get(&stat), Some(&0));
        }
    }

    #[test]
    fn stat_stage_move_reports_unchanged_at_stage_boundary() {
        let player = pokemon("SANDSHREW", 50, pokemon_type("GROUND"), "SAND_ATTACK");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.stat_boosts.insert(Stat::Accuracy, -6);
        let moves = BTreeMap::from([
            (
                "SAND_ATTACK".to_string(),
                move_data_with_stat(
                    "SAND_ATTACK",
                    pokemon_type("GROUND"),
                    0,
                    0,
                    "ACCURACY_DOWN",
                    0,
                    Stat::Accuracy,
                    -1,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("boundary stat move resolves");

        assert_eq!(
            outcome.state.enemy.stat_boosts.get(&Stat::Accuracy),
            Some(&-6)
        );
        assert!(outcome.events.contains(&BattleEvent::StatStageUnchanged {
            side: BattleSide::Player,
            move_name: "SAND_ATTACK".to_string(),
            target: BattleSide::Enemy,
            stat: Stat::Accuracy,
            amount: -1,
            stage: -6,
        }));
    }

    #[test]
    fn mist_move_sets_side_stat_drop_guard_from_exact_pack_effect() {
        let player = pokemon("LAPRAS", 50, pokemon_type("WATER"), "MIST");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "MIST_NOOP");
        let moves = BTreeMap::from([
            (
                "MIST".to_string(),
                move_data_with_effect("MIST", pokemon_type("ICE"), 0, 100, "MIST"),
            ),
            (
                "MIST_NOOP".to_string(),
                move_data_with_effect("MIST_NOOP", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("mist turn resolves");

        assert_eq!(outcome.state.player_mist_turns, 4);
        assert!(outcome.events.contains(&BattleEvent::MistApplied {
            side: BattleSide::Player,
            move_name: "MIST".to_string(),
            turns: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::MistCount {
            side: BattleSide::Player,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn mist_blocks_opponent_stat_drop_without_mutating_stage() {
        let player = pokemon("LAPRAS", 50, pokemon_type("WATER"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TAIL_WHIP");
        let moves = BTreeMap::from([
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TAIL_WHIP".to_string(),
                move_data_with_stat(
                    "TAIL_WHIP",
                    pokemon_type("NORMAL"),
                    0,
                    100,
                    "DEFENSE_DOWN",
                    0,
                    Stat::Defense,
                    -1,
                ),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_mist_turns = 2;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("mist-protected stat drop resolves");

        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Defense),
            Some(&0)
        );
        assert!(outcome.events.contains(&BattleEvent::MistProtected {
            side: BattleSide::Enemy,
            move_name: "TAIL_WHIP".to_string(),
            target: BattleSide::Player,
            stat: Stat::Defense,
            amount: -1,
            turns_remaining: 2,
        }));
        assert!(outcome.events.contains(&BattleEvent::MistCount {
            side: BattleSide::Player,
            turns_remaining: 1,
        }));
    }

    #[test]
    fn mist_reports_failure_when_already_active() {
        let player = pokemon("LAPRAS", 50, pokemon_type("WATER"), "MIST");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "MIST".to_string(),
                move_data_with_effect("MIST", pokemon_type("ICE"), 0, 100, "MIST"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_mist_turns = 3;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("active mist failure resolves");

        assert_eq!(outcome.state.player_mist_turns, 2);
        assert!(outcome.events.contains(&BattleEvent::MistFailed {
            side: BattleSide::Player,
            move_name: "MIST".to_string(),
            turns_remaining: 3,
        }));
    }

    #[test]
    fn safeguard_move_sets_side_status_guard_from_exact_pack_effect() {
        let player = pokemon("MEGANIUM", 50, pokemon_type("GRASS"), "SAFEGUARD");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "SAFEGUARD".to_string(),
                move_data_with_effect("SAFEGUARD", pokemon_type("NORMAL"), 0, 100, "SAFEGUARD"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("safeguard turn resolves");

        assert_eq!(outcome.state.player_safeguard_turns, 4);
        assert!(outcome.events.contains(&BattleEvent::SafeguardApplied {
            side: BattleSide::Player,
            move_name: "SAFEGUARD".to_string(),
            turns: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::SafeguardCount {
            side: BattleSide::Player,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn safeguard_blocks_opponent_direct_status_without_mutating_status() {
        let player = pokemon("MEGANIUM", 90, pokemon_type("GRASS"), "SAFEGUARD");
        let enemy = pokemon("EKANS", 40, pokemon_type("POISON"), "POISON_POWDER");
        let moves = BTreeMap::from([
            (
                "SAFEGUARD".to_string(),
                move_data_with_effect("SAFEGUARD", pokemon_type("NORMAL"), 0, 100, "SAFEGUARD"),
            ),
            (
                "POISON_POWDER".to_string(),
                move_data_with_effect("POISON_POWDER", pokemon_type("POISON"), 0, 100, "POISON"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("safeguard protected status turn resolves");

        assert_eq!(outcome.state.player.status, None);
        assert!(outcome.events.contains(&BattleEvent::SafeguardProtected {
            side: BattleSide::Enemy,
            move_name: "POISON_POWDER".to_string(),
            target: BattleSide::Player,
            effect: "POISON".to_string(),
            turns_remaining: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::SafeguardCount {
            side: BattleSide::Player,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn safeguard_blocks_opponent_confusion_without_mutating_turns() {
        let player = pokemon("MEGANIUM", 50, pokemon_type("GRASS"), "SPLASH");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "CONFUSE_RAY");
        let moves = BTreeMap::from([
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "CONFUSE_RAY".to_string(),
                move_data_with_effect("CONFUSE_RAY", pokemon_type("GHOST"), 0, 100, "CONFUSE"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_safeguard_turns = 2;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("safeguard protected confusion turn resolves");

        assert_eq!(outcome.state.player.confusion_turns, 0);
        assert!(outcome.events.contains(&BattleEvent::SafeguardProtected {
            side: BattleSide::Enemy,
            move_name: "CONFUSE_RAY".to_string(),
            target: BattleSide::Player,
            effect: "CONFUSION".to_string(),
            turns_remaining: 2,
        }));
        assert!(outcome.events.contains(&BattleEvent::SafeguardCount {
            side: BattleSide::Player,
            turns_remaining: 1,
        }));
    }

    #[test]
    fn safeguard_reports_failure_when_already_active() {
        let player = pokemon("MEGANIUM", 50, pokemon_type("GRASS"), "SAFEGUARD");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "SAFEGUARD".to_string(),
                move_data_with_effect("SAFEGUARD", pokemon_type("NORMAL"), 0, 100, "SAFEGUARD"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_safeguard_turns = 3;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("active safeguard failure resolves");

        assert_eq!(outcome.state.player_safeguard_turns, 2);
        assert!(outcome.events.contains(&BattleEvent::SafeguardFailed {
            side: BattleSide::Player,
            move_name: "SAFEGUARD".to_string(),
            turns_remaining: 3,
        }));
    }

    #[test]
    fn reflect_move_sets_side_physical_screen_from_exact_pack_effect() {
        let player = pokemon("MEGANIUM", 50, pokemon_type("GRASS"), "REFLECT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "REFLECT".to_string(),
                move_data_with_effect("REFLECT", pokemon_type("PSYCHIC_TYPE"), 0, 100, "REFLECT"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("reflect turn resolves");

        assert_eq!(outcome.state.player_reflect_turns, 4);
        assert!(outcome.events.contains(&BattleEvent::ReflectApplied {
            side: BattleSide::Player,
            move_name: "REFLECT".to_string(),
            turns: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::ReflectCount {
            side: BattleSide::Player,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn reflect_reports_failure_when_already_active() {
        let player = pokemon("MEGANIUM", 50, pokemon_type("GRASS"), "REFLECT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "REFLECT".to_string(),
                move_data_with_effect("REFLECT", pokemon_type("PSYCHIC_TYPE"), 0, 100, "REFLECT"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_reflect_turns = 3;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("active reflect failure resolves");

        assert_eq!(outcome.state.player_reflect_turns, 2);
        assert!(outcome.events.contains(&BattleEvent::ReflectFailed {
            side: BattleSide::Player,
            move_name: "REFLECT".to_string(),
            turns_remaining: 3,
        }));
        assert!(outcome.events.contains(&BattleEvent::ReflectCount {
            side: BattleSide::Player,
            turns_remaining: 2,
        }));
    }

    #[test]
    fn reflect_halves_physical_damage_against_protected_side() {
        let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(22);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_reflect_turns = 3;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("reflect damage reduction resolves");

        let (damage_before, damage_after) = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::ScreenDamageReduced {
                    side: BattleSide::Player,
                    move_name,
                    target: BattleSide::Enemy,
                    screen: BattleScreen::Reflect,
                    damage_before,
                    damage_after,
                } if move_name == "TACKLE" => Some((*damage_before, *damage_after)),
                _ => None,
            })
            .expect("reflect reduced physical damage");
        assert_eq!(damage_after, (damage_before / 2).max(1));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                critical: false,
                ..
            } if move_name == "TACKLE" && *damage == damage_after
        )));
    }

    #[test]
    fn light_screen_move_sets_side_special_screen_from_exact_pack_effect() {
        let player = pokemon("MEGANIUM", 50, pokemon_type("GRASS"), "LIGHT_SCREEN");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "LIGHT_SCREEN".to_string(),
                move_data_with_effect(
                    "LIGHT_SCREEN",
                    pokemon_type("PSYCHIC_TYPE"),
                    0,
                    100,
                    "LIGHT_SCREEN",
                ),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("light screen turn resolves");

        assert_eq!(outcome.state.player_light_screen_turns, 4);
        assert!(outcome.events.contains(&BattleEvent::LightScreenApplied {
            side: BattleSide::Player,
            move_name: "LIGHT_SCREEN".to_string(),
            turns: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::LightScreenCount {
            side: BattleSide::Player,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn light_screen_halves_special_damage_against_protected_side() {
        let player = pokemon("PSYDUCK", 90, pokemon_type("WATER"), "WATER_GUN");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "WATER_GUN".to_string(),
                move_data("WATER_GUN", pokemon_type("WATER"), 40, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(22);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_light_screen_turns = 3;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("light screen damage reduction resolves");

        let (damage_before, damage_after) = outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::ScreenDamageReduced {
                    side: BattleSide::Player,
                    move_name,
                    target: BattleSide::Enemy,
                    screen: BattleScreen::LightScreen,
                    damage_before,
                    damage_after,
                } if move_name == "WATER_GUN" => Some((*damage_before, *damage_after)),
                _ => None,
            })
            .expect("light screen reduced special damage");
        assert_eq!(damage_after, (damage_before / 2).max(1));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                critical: false,
                ..
            } if move_name == "WATER_GUN" && *damage == damage_after
        )));
    }

    #[test]
    fn destiny_bond_sets_side_state_from_exact_pack_effect() {
        let player = pokemon("GASTLY", 120, pokemon_type("GHOST"), "DESTINY_BOND");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "DESTINY_BOND".to_string(),
                move_data_with_effect(
                    "DESTINY_BOND",
                    pokemon_type("GHOST"),
                    0,
                    100,
                    "DESTINY_BOND",
                ),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("destiny bond turn resolves");

        assert!(outcome.state.player_destiny_bond_active);
        assert!(outcome.events.contains(&BattleEvent::DestinyBondApplied {
            side: BattleSide::Player,
            move_name: "DESTINY_BOND".to_string(),
        }));
    }

    #[test]
    fn destiny_bond_faints_direct_damage_attacker_when_bonded_target_faints() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let mut enemy = pokemon("WOBBUFFET", 120, pokemon_type("NORMAL"), "DESTINY_BOND");
        enemy.hp = 1;
        let moves = BTreeMap::from([
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "DESTINY_BOND".to_string(),
                move_data_with_effect(
                    "DESTINY_BOND",
                    pokemon_type("GHOST"),
                    0,
                    100,
                    "DESTINY_BOND",
                ),
            ),
        ]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("destiny bond activation resolves");

        assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
        assert_eq!(outcome.state.enemy.hp, 0);
        assert_eq!(outcome.state.player.hp, 0);
        assert!(!outcome.state.enemy_destiny_bond_active);
        assert!(outcome.events.contains(&BattleEvent::DestinyBondApplied {
            side: BattleSide::Enemy,
            move_name: "DESTINY_BOND".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::DestinyBondActivated {
                side: BattleSide::Enemy,
                source: BattleSide::Player,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(
                    event,
                    BattleEvent::Fainted {
                        side: BattleSide::Player
                    }
                ))
                .count(),
            1
        );
    }

    #[test]
    fn destiny_bond_clears_when_side_uses_another_move() {
        let player = pokemon("GASTLY", 120, pokemon_type("GHOST"), "TACKLE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(22);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_destiny_bond_active = true;

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("destiny bond clear resolves");

        assert!(!outcome.state.player_destiny_bond_active);
    }

    #[test]
    fn leech_seed_sets_target_side_source_from_exact_pack_effect() {
        let player = pokemon("BULBASAUR", 50, pokemon_type("GRASS"), "LEECH_SEED");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "LEECH_SEED".to_string(),
                move_data_with_effect("LEECH_SEED", pokemon_type("GRASS"), 0, 100, "LEECH_SEED"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("leech seed turn resolves");

        assert_eq!(
            outcome.state.enemy_leech_seed_source,
            Some(BattleSide::Player)
        );
        assert!(outcome.events.contains(&BattleEvent::LeechSeedApplied {
            side: BattleSide::Player,
            move_name: "LEECH_SEED".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn leech_seed_fails_against_grass_type_without_source_state() {
        let player = pokemon("BULBASAUR", 50, pokemon_type("GRASS"), "LEECH_SEED");
        let enemy = pokemon("ODDISH", 40, pokemon_type("GRASS"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "LEECH_SEED".to_string(),
                move_data_with_effect("LEECH_SEED", pokemon_type("GRASS"), 0, 100, "LEECH_SEED"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("leech seed grass immunity resolves");

        assert_eq!(outcome.state.enemy_leech_seed_source, None);
        assert!(outcome.events.contains(&BattleEvent::LeechSeedImmune {
            side: BattleSide::Player,
            move_name: "LEECH_SEED".to_string(),
            target: BattleSide::Enemy,
            target_type1: pokemon_type("GRASS"),
            target_type2: pokemon_type("GRASS"),
        }));
    }

    #[test]
    fn leech_seed_fails_when_target_already_seeded() {
        let player = pokemon("BULBASAUR", 50, pokemon_type("GRASS"), "LEECH_SEED");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "LEECH_SEED".to_string(),
                move_data_with_effect("LEECH_SEED", pokemon_type("GRASS"), 0, 100, "LEECH_SEED"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_leech_seed_source = Some(BattleSide::Player);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("already seeded leech seed resolves");

        assert_eq!(
            outcome.state.enemy_leech_seed_source,
            Some(BattleSide::Player)
        );
        assert!(outcome.events.contains(&BattleEvent::LeechSeedFailed {
            side: BattleSide::Player,
            move_name: "LEECH_SEED".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn leech_seed_drains_seeded_side_at_end_of_turn() {
        let mut player = pokemon("BULBASAUR", 50, pokemon_type("GRASS"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        player.hp = player.max_hp - 5;
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_leech_seed_source = Some(BattleSide::Player);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("leech seed residual resolves");

        let damage = (enemy_hp / 8).max(1);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - damage);
        assert_eq!(outcome.state.player.hp, outcome.state.player.max_hp);
        assert!(outcome.events.contains(&BattleEvent::LeechSeedDamage {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            damage,
            hp_before: enemy_hp,
            hp_after: enemy_hp - damage,
        }));
        assert!(outcome.events.contains(&BattleEvent::LeechSeedDrain {
            side: BattleSide::Player,
            target: BattleSide::Enemy,
            amount: 5,
            hp_before: outcome.state.player.max_hp - 5,
            hp_after: outcome.state.player.max_hp,
        }));
    }

    #[test]
    fn nightmare_applies_to_sleeping_target() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "NIGHTMARE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        enemy.status = Some("SLEEP".to_string());
        enemy.sleep_turns = 2;
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "NIGHTMARE".to_string(),
                move_data_with_effect("NIGHTMARE", pokemon_type("GHOST"), 0, 100, "NIGHTMARE"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("nightmare resolves");

        let damage = (enemy_hp / 4).max(1);
        assert_eq!(
            outcome.state.enemy_nightmare_source,
            Some(BattleSide::Player)
        );
        assert_eq!(outcome.state.enemy.hp, enemy_hp - damage);
        assert!(outcome.events.contains(&BattleEvent::NightmareApplied {
            side: BattleSide::Player,
            move_name: "NIGHTMARE".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(outcome.events.contains(&BattleEvent::NightmareDamage {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            damage,
            hp_before: enemy_hp,
            hp_after: enemy_hp - damage,
        }));
    }

    #[test]
    fn nightmare_fails_against_awake_target() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "NIGHTMARE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "NIGHTMARE".to_string(),
                move_data_with_effect("NIGHTMARE", pokemon_type("GHOST"), 0, 100, "NIGHTMARE"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("failed nightmare resolves");

        assert_eq!(outcome.state.enemy_nightmare_source, None);
        assert!(outcome.events.contains(&BattleEvent::NightmareFailed {
            side: BattleSide::Player,
            move_name: "NIGHTMARE".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn nightmare_damages_sleeping_target_at_end_of_turn() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPLASH");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        enemy.status = Some("SLEEP".to_string());
        enemy.sleep_turns = 2;
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_nightmare_source = Some(BattleSide::Player);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("nightmare residual resolves");

        let damage = (enemy_hp / 4).max(1);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - damage);
        assert_eq!(
            outcome.state.enemy_nightmare_source,
            Some(BattleSide::Player)
        );
        assert!(outcome.events.contains(&BattleEvent::NightmareDamage {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            damage,
            hp_before: enemy_hp,
            hp_after: enemy_hp - damage,
        }));
    }

    #[test]
    fn nightmare_ends_when_target_is_awake() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([(
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_nightmare_source = Some(BattleSide::Player);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("nightmare ending resolves");

        assert_eq!(outcome.state.enemy_nightmare_source, None);
        assert!(outcome.events.contains(&BattleEvent::NightmareEnded {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
        }));
    }

    #[test]
    fn switching_clears_nightmare_source_and_target_state() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([(
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_nightmare_source = Some(BattleSide::Player);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("switch clears nightmare");

        assert_eq!(outcome.state.player_nightmare_source, None);
        assert_eq!(outcome.state.enemy_nightmare_source, None);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::NightmareDamage { .. }))
        );
    }

    #[test]
    fn switching_clears_leech_seed_side_state() {
        let player = pokemon("BULBASAUR", 50, pokemon_type("GRASS"), "TACKLE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_leech_seed_source = Some(BattleSide::Enemy);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("switch clears leech seed");

        assert_eq!(outcome.state.player_leech_seed_source, None);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::LeechSeedDamage { .. }))
        );
    }

    #[test]
    fn trap_target_applies_after_successful_damage() {
        let player = pokemon("ONIX", 50, pokemon_type("ROCK"), "BIND");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "BIND".to_string(),
                move_data_with_effect("BIND", pokemon_type("NORMAL"), 15, 100, "TRAP_TARGET"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("trap target turn resolves");

        let trap = outcome.state.enemy_trap.as_ref().expect("enemy trapped");
        assert_eq!(trap.source, BattleSide::Player);
        assert_eq!(trap.move_name, "BIND");
        assert!((1..=4).contains(&trap.turns_remaining));
        assert!(
            outcome.events.iter().any(|event| matches!(
                event,
                BattleEvent::TrapApplied {
                    side: BattleSide::Player,
                    move_name,
                    target: BattleSide::Enemy,
                    turns: 2..=5,
                    roll: 0..=3,
                } if move_name == "BIND"
            )),
            "trap application event missing"
        );
    }

    #[test]
    fn trapped_side_cannot_act_before_residual_damage_ticks() {
        let player = pokemon("ONIX", 50, pokemon_type("ROCK"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_trap = Some(BattleTrapState {
            source: BattleSide::Player,
            move_name: "BIND".to_string(),
            turns_remaining: 2,
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("trapped target turn resolves");

        let damage = (enemy_hp / 16).max(1);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - damage);
        assert_eq!(
            outcome
                .state
                .enemy_trap
                .as_ref()
                .map(|trap| trap.turns_remaining),
            Some(1)
        );
        assert!(outcome.events.contains(&BattleEvent::TrappedTurn {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
            source: BattleSide::Player,
            trap_move_name: "BIND".to_string(),
            turns_remaining: 2,
        }));
        assert!(outcome.events.contains(&BattleEvent::TrapDamage {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            move_name: "BIND".to_string(),
            damage,
            hp_before: enemy_hp,
            hp_after: enemy_hp - damage,
            turns_remaining: 1,
        }));
    }

    #[test]
    fn trap_ends_when_residual_turns_expire() {
        let player = pokemon("ONIX", 50, pokemon_type("ROCK"), "SPLASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_trap = Some(BattleTrapState {
            source: BattleSide::Player,
            move_name: "WRAP".to_string(),
            turns_remaining: 1,
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("trap ending turn resolves");

        let damage = (enemy_hp / 16).max(1);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - damage);
        assert_eq!(outcome.state.enemy_trap, None);
        assert!(outcome.events.contains(&BattleEvent::TrapEnded {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            move_name: "WRAP".to_string(),
        }));
    }

    #[test]
    fn switching_clears_traps_on_and_from_switching_side() {
        let player = pokemon("ONIX", 50, pokemon_type("ROCK"), "TACKLE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_trap = Some(BattleTrapState {
            source: BattleSide::Player,
            move_name: "BIND".to_string(),
            turns_remaining: 2,
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("switch clears sourced trap");

        assert_eq!(outcome.state.enemy_trap, None);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::TrapDamage { .. }))
        );
    }

    #[test]
    fn encore_applies_to_targets_last_move_and_forces_that_move() {
        let player = pokemon("CLEFAIRY", 90, pokemon_type("NORMAL"), "ENCORE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves.push(LearnedMove {
            name: "SPLASH".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "ENCORE".to_string(),
                move_data_with_effect("ENCORE", pokemon_type("NORMAL"), 0, 100, "ENCORE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("encore turn resolves");

        let encore = outcome
            .state
            .enemy_encore
            .as_ref()
            .expect("enemy remains encored after forced move");
        assert_eq!(encore.move_name, "TACKLE");
        assert!((2..=5).contains(&encore.turns_remaining));
        assert_eq!(outcome.state.enemy.moves[0].current_pp, 4);
        assert_eq!(outcome.state.enemy.moves[1].current_pp, 5);
        assert!(outcome.state.player.hp < player_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::EncoreApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                encored_move,
                turns: 3..=6,
                roll: 0..=3,
            } if move_name == "ENCORE" && encored_move == "TACKLE"
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::EncoreForcedMove {
                side: BattleSide::Enemy,
                requested_slot: 1,
                requested_move,
                encored_slot: 0,
                encored_move,
                ..
            } if requested_move == "SPLASH" && encored_move == "TACKLE"
        )));
    }

    #[test]
    fn encore_fails_without_target_last_move() {
        let player = pokemon("CLEFAIRY", 90, pokemon_type("NORMAL"), "ENCORE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "ENCORE".to_string(),
                move_data_with_effect("ENCORE", pokemon_type("NORMAL"), 0, 100, "ENCORE"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("encore failure turn resolves");

        assert_eq!(outcome.state.enemy_encore, None);
        assert!(outcome.events.contains(&BattleEvent::EncoreFailed {
            side: BattleSide::Player,
            move_name: "ENCORE".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn encore_final_forced_turn_executes_and_then_clears() {
        let player = pokemon("CLEFAIRY", 20, pokemon_type("NORMAL"), "SPLASH");
        let mut enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves.push(LearnedMove {
            name: "SPLASH".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_encore = Some(BattleEncoreState {
            move_name: "TACKLE".to_string(),
            turns_remaining: 1,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("final encore turn resolves");

        assert_eq!(outcome.state.enemy_encore, None);
        assert_eq!(outcome.state.enemy.moves[0].current_pp, 4);
        assert_eq!(outcome.state.enemy.moves[1].current_pp, 5);
        assert!(outcome.state.player.hp < player_hp);
        assert!(outcome.events.contains(&BattleEvent::EncoreEnded {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::EncoreForcedMove {
                side: BattleSide::Enemy,
                requested_slot: 1,
                requested_move,
                encored_slot: 0,
                encored_move,
                turns_remaining: 0,
            } if requested_move == "SPLASH" && encored_move == "TACKLE"
        )));
    }

    #[test]
    fn disable_applies_to_targets_last_move() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "DISABLE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "DISABLE".to_string(),
                move_data_with_effect("DISABLE", pokemon_type("NORMAL"), 0, 100, "DISABLE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
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
        .expect("disable turn resolves");

        let disable = outcome
            .state
            .enemy_disable
            .as_ref()
            .expect("enemy move disabled");
        assert_eq!(disable.move_name, "TACKLE");
        assert!((3..=6).contains(&disable.turns_remaining));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::DisableApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                disabled_move,
                turns: 4..=7,
                roll: 0..=3,
            } if move_name == "DISABLE" && disabled_move == "TACKLE"
        )));
    }

    #[test]
    fn disabled_move_cannot_execute_or_spend_pp() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "NO_DAMAGE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 5;
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_disable = Some(BattleDisableState {
            move_name: "TACKLE".to_string(),
            turns_remaining: 2,
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("disabled move turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert_eq!(outcome.state.enemy.moves[0].current_pp, 5);
        assert!(outcome.events.contains(&BattleEvent::DisabledMove {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
            turns_remaining: 2,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn disable_fails_without_target_last_move() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "DISABLE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        let moves = BTreeMap::from([
            (
                "DISABLE".to_string(),
                move_data_with_effect("DISABLE", pokemon_type("NORMAL"), 0, 100, "DISABLE"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("disable fail turn resolves");

        assert_eq!(outcome.state.enemy_disable, None);
        assert!(outcome.events.contains(&BattleEvent::DisableFailed {
            side: BattleSide::Player,
            move_name: "DISABLE".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn disable_counts_down_and_ends() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "NO_DAMAGE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        let moves = BTreeMap::from([(
            "NO_DAMAGE".to_string(),
            move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_disable = Some(BattleDisableState {
            move_name: "TACKLE".to_string(),
            turns_remaining: 1,
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("disable expiry turn resolves");

        assert_eq!(outcome.state.enemy_disable, None);
        assert!(outcome.events.contains(&BattleEvent::DisableEnded {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn switching_clears_disable_on_switching_side() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "NO_DAMAGE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());
        state.enemy_disable = Some(BattleDisableState {
            move_name: "TACKLE".to_string(),
            turns_remaining: 2,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("switch clears disable");

        assert_eq!(outcome.state.enemy_last_move, None);
        assert_eq!(outcome.state.enemy_disable, None);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::DisableCount { .. }))
        );
    }

    #[test]
    fn protect_blocks_incoming_damage_and_effects() {
        let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "PROTECT");
        let enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "PROTECT".to_string(),
                move_data_with_effect("PROTECT", pokemon_type("NORMAL"), 0, 100, "PROTECT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("protect turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert_eq!(outcome.state.player_protect_counter, 1);
        assert!(!outcome.state.player_protect_active);
        assert!(outcome.events.contains(&BattleEvent::ProtectApplied {
            side: BattleSide::Player,
            move_name: "PROTECT".to_string(),
            counter: 1,
            roll: None,
        }));
        assert!(outcome.events.contains(&BattleEvent::MoveProtected {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
            target: BattleSide::Player,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn endure_leaves_target_at_one_hp_against_lethal_damage() {
        let player = pokemon("MACHOP", 45, pokemon_type("FIGHTING"), "MEGA_PUNCH");
        let mut enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "ENDURE");
        enemy.hp = 10;
        let moves = BTreeMap::from([
            (
                "MEGA_PUNCH".to_string(),
                move_data("MEGA_PUNCH", pokemon_type("NORMAL"), 250, 100),
            ),
            (
                "ENDURE".to_string(),
                move_data_with_effect("ENDURE", pokemon_type("NORMAL"), 0, 100, "ENDURE"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("endure turn resolves");

        assert_eq!(outcome.state.enemy.hp, 1);
        assert_eq!(outcome.state.enemy_protect_counter, 1);
        assert!(!outcome.state.enemy_endure_active);
        assert!(outcome.events.contains(&BattleEvent::EndureApplied {
            side: BattleSide::Enemy,
            move_name: "ENDURE".to_string(),
            counter: 1,
            roll: None,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::EnduredHit {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                raw_damage,
            } if move_name == "MEGA_PUNCH" && *raw_damage >= 10
        )));
        assert!(!outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
    }

    #[test]
    fn slower_endure_fails_after_opponent_protects_first() {
        let player = pokemon("CHIKORITA", 90, pokemon_type("GRASS"), "PROTECT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "ENDURE");
        let moves = BTreeMap::from([
            (
                "PROTECT".to_string(),
                move_data_with_effect("PROTECT", pokemon_type("NORMAL"), 0, 100, "PROTECT"),
            ),
            (
                "ENDURE".to_string(),
                move_data_with_effect("ENDURE", pokemon_type("NORMAL"), 0, 100, "ENDURE"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("protect/endure priority tie resolves");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert!(outcome.events.contains(&BattleEvent::ProtectApplied {
            side: BattleSide::Player,
            move_name: "PROTECT".to_string(),
            counter: 1,
            roll: None,
        }));
        assert!(outcome.events.contains(&BattleEvent::EndureFailed {
            side: BattleSide::Enemy,
            move_name: "ENDURE".to_string(),
            counter_before: 0,
            roll: None,
        }));
        assert_eq!(outcome.state.enemy_protect_counter, 0);
    }

    #[test]
    fn spite_reduces_targets_last_move_pp() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPITE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 5;
        let moves = BTreeMap::from([
            (
                "SPITE".to_string(),
                move_data_with_effect("SPITE", pokemon_type("GHOST"), 0, 100, "SPITE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
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
        .expect("spite turn resolves");

        assert!(outcome.state.enemy.moves[0].current_pp <= 3);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SpiteApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                target_move,
                pp_before: 5,
                pp_after,
                reduction: 2..=5,
                roll: 0..=3,
            } if move_name == "SPITE" && target_move == "TACKLE" && *pp_after <= 3
        )));
    }

    #[test]
    fn spite_fails_without_target_last_move() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPITE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        let moves = BTreeMap::from([
            (
                "SPITE".to_string(),
                move_data_with_effect("SPITE", pokemon_type("GHOST"), 0, 100, "SPITE"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("spite fail turn resolves");

        assert!(outcome.events.contains(&BattleEvent::SpiteFailed {
            side: BattleSide::Player,
            move_name: "SPITE".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn spite_fails_when_targets_last_move_has_no_pp() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPITE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 0;
        let moves = BTreeMap::from([
            (
                "SPITE".to_string(),
                move_data_with_effect("SPITE", pokemon_type("GHOST"), 0, 100, "SPITE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
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
        .expect("spite no-pp turn resolves");

        assert_eq!(outcome.state.enemy.moves[0].current_pp, 0);
        assert!(outcome.events.contains(&BattleEvent::SpiteFailed {
            side: BattleSide::Player,
            move_name: "SPITE".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn reset_stats_clears_both_sides_stat_stages() {
        let mut player = pokemon("MURKROW", 90, pokemon_type("DARK"), "HAZE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        player.stat_boosts.insert(Stat::Attack, 3);
        player.stat_boosts.insert(Stat::Accuracy, -2);
        enemy.stat_boosts.insert(Stat::Defense, -4);
        enemy.stat_boosts.insert(Stat::Evasion, 5);
        let moves = BTreeMap::from([
            (
                "HAZE".to_string(),
                move_data_with_effect("HAZE", pokemon_type("ICE"), 0, 100, "RESET_STATS"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("reset stats turn resolves");

        assert!(
            outcome
                .state
                .player
                .stat_boosts
                .values()
                .all(|stage| *stage == 0)
        );
        assert!(
            outcome
                .state
                .enemy
                .stat_boosts
                .values()
                .all(|stage| *stage == 0)
        );
        assert!(outcome.events.contains(&BattleEvent::StatsReset {
            side: BattleSide::Player,
            move_name: "HAZE".to_string(),
        }));
    }

    #[test]
    fn psych_up_copies_targets_stat_stages() {
        let mut player = pokemon("ESPEON", 90, pokemon_type("PSYCHIC_TYPE"), "PSYCH_UP");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        player.stat_boosts.insert(Stat::Attack, -3);
        player.stat_boosts.insert(Stat::Defense, 2);
        enemy.stat_boosts.insert(Stat::Attack, 4);
        enemy.stat_boosts.insert(Stat::Defense, -1);
        enemy.stat_boosts.insert(Stat::Speed, 3);
        let expected = enemy.stat_boosts.clone();
        let moves = BTreeMap::from([
            (
                "PSYCH_UP".to_string(),
                move_data_with_effect("PSYCH_UP", pokemon_type("NORMAL"), 0, 100, "PSYCH_UP"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("psych up turn resolves");

        assert_eq!(outcome.state.player.stat_boosts, expected);
        assert!(outcome.events.contains(&BattleEvent::PsychUpApplied {
            side: BattleSide::Player,
            move_name: "PSYCH_UP".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn rain_dance_sets_weather_and_counts_down() {
        let player = pokemon("POLIWAG", 90, pokemon_type("WATER"), "RAIN_DANCE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        let moves = BTreeMap::from([
            (
                "RAIN_DANCE".to_string(),
                move_data_with_effect("RAIN_DANCE", pokemon_type("WATER"), 0, 100, "RAIN_DANCE"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("rain dance turn resolves");

        assert_eq!(outcome.state.weather, Weather::Rain);
        assert_eq!(outcome.state.weather_turns, 4);
        assert!(outcome.events.contains(&BattleEvent::WeatherApplied {
            side: BattleSide::Player,
            move_name: "RAIN_DANCE".to_string(),
            weather: Weather::Rain,
            turns: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::WeatherContinues {
            weather: Weather::Rain,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn weather_expires_at_end_turn() {
        let player = pokemon("POLIWAG", 90, pokemon_type("WATER"), "NO_DAMAGE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        let moves = BTreeMap::from([(
            "NO_DAMAGE".to_string(),
            move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        )]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.weather = Weather::Sun;
        state.weather_turns = 1;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("weather expiry turn resolves");

        assert_eq!(outcome.state.weather, Weather::None);
        assert_eq!(outcome.state.weather_turns, 0);
        assert!(outcome.events.contains(&BattleEvent::WeatherEnded {
            weather: Weather::Sun,
        }));
    }

    #[test]
    fn sandstorm_sets_weather_and_damages_non_immune_pokemon() {
        let player = pokemon("GEODUDE", 40, pokemon_type("ROCK"), "SANDSTORM");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "NO_DAMAGE");
        let enemy_hp = enemy.hp;
        let enemy_max_hp = enemy.max_hp;
        let sandstorm_damage = (enemy_max_hp / 16).max(1);
        let moves = BTreeMap::from([
            (
                "SANDSTORM".to_string(),
                move_data_with_effect("SANDSTORM", pokemon_type("ROCK"), 0, 100, "SANDSTORM"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player.clone(), enemy, rng.seed()),
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
        .expect("sandstorm turn resolves");

        assert_eq!(outcome.state.weather, Weather::Sandstorm);
        assert_eq!(outcome.state.weather_turns, 4);
        assert_eq!(outcome.state.player.hp, player.hp);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - sandstorm_damage);
        assert!(outcome.events.contains(&BattleEvent::WeatherApplied {
            side: BattleSide::Player,
            move_name: "SANDSTORM".to_string(),
            weather: Weather::Sandstorm,
            turns: 5,
        }));
        assert!(outcome.events.contains(&BattleEvent::SandstormDamage {
            side: BattleSide::Enemy,
            damage: sandstorm_damage,
            hp_before: enemy_hp,
            hp_after: enemy_hp - sandstorm_damage,
        }));
        assert!(outcome.events.contains(&BattleEvent::WeatherContinues {
            weather: Weather::Sandstorm,
            turns_remaining: 4,
        }));
    }

    #[test]
    fn sandstorm_does_not_damage_rock_ground_or_steel_pokemon() {
        let player = pokemon("ONIX", 40, pokemon_type("GROUND"), "SANDSTORM");
        let enemy = pokemon("MAGNEMITE", 30, pokemon_type("STEEL"), "NO_DAMAGE");
        let moves = BTreeMap::from([
            (
                "SANDSTORM".to_string(),
                move_data_with_effect("SANDSTORM", pokemon_type("ROCK"), 0, 100, "SANDSTORM"),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player.clone(), enemy.clone(), rng.seed()),
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
        .expect("sandstorm immunity turn resolves");

        assert_eq!(outcome.state.weather, Weather::Sandstorm);
        assert_eq!(outcome.state.weather_turns, 4);
        assert_eq!(outcome.state.player.hp, player.hp);
        assert_eq!(outcome.state.enemy.hp, enemy.hp);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::SandstormDamage { .. }))
        );
    }

    #[test]
    fn sunny_day_weather_boosts_fire_damage() {
        let player = pokemon("CYNDAQUIL", 90, pokemon_type("FIRE"), "EMBER");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
        let moves = BTreeMap::from([
            (
                "EMBER".to_string(),
                move_data("EMBER", pokemon_type("FIRE"), 40, 100),
            ),
            (
                "NO_DAMAGE".to_string(),
                move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut normal_rng = Random::new(1);
        let normal = resolve_battle_turn(
            battle_state(player.clone(), enemy.clone(), normal_rng.seed()),
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
            &mut normal_rng,
        )
        .expect("normal fire turn resolves");

        let mut sun_rng = Random::new(1);
        let mut sunny_state = battle_state(player, enemy, sun_rng.seed());
        sunny_state.weather = Weather::Sun;
        sunny_state.weather_turns = 2;
        let sunny = resolve_battle_turn(
            sunny_state,
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
            &mut sun_rng,
        )
        .expect("sunny fire turn resolves");

        let normal_damage = normal
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "EMBER" => Some(*damage),
                _ => None,
            })
            .expect("normal fire damage");
        let sunny_damage = sunny
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "EMBER" => Some(*damage),
                _ => None,
            })
            .expect("sunny fire damage");
        assert!(sunny_damage > normal_damage);
    }

    #[test]
    fn faster_secondary_flinch_effect_blocks_target_once() {
        let player = pokemon("DUNSPARCE", 90, pokemon_type("NORMAL"), "HEADBUTT");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "HEADBUTT".to_string(),
                move_data_with_effect_chance(
                    "HEADBUTT",
                    pokemon_type("NORMAL"),
                    70,
                    100,
                    "FLINCH_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("flinch turn resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(!outcome.state.enemy.flinching);
        assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name: "HEADBUTT".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(outcome.events.contains(&BattleEvent::Flinched {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn slower_secondary_flinch_effect_is_cleared_at_end_of_turn() {
        let player = pokemon("SLOWPOKE", 10, pokemon_type("NORMAL"), "HEADBUTT");
        let enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "HEADBUTT".to_string(),
                move_data_with_effect_chance(
                    "HEADBUTT",
                    pokemon_type("NORMAL"),
                    70,
                    100,
                    "FLINCH_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("slow flinch turn resolves");

        assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
        assert!(!outcome.state.enemy.flinching);
        assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name: "HEADBUTT".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Flinched { .. }))
        );
    }

    #[test]
    fn damaging_secondary_burn_respects_fire_type_immunity() {
        let player = pokemon("CYNDAQUIL", 50, pokemon_type("FIRE"), "EMBER");
        let enemy = pokemon("MAGMAR", 40, pokemon_type("FIRE"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "EMBER".to_string(),
                move_data_with_effect_chance(
                    "EMBER",
                    pokemon_type("FIRE"),
                    40,
                    100,
                    "BURN_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("burn immunity turn resolves");

        assert_eq!(outcome.state.enemy.status, None);
        assert!(outcome.events.contains(&BattleEvent::StatusImmune {
            side: BattleSide::Player,
            move_name: "EMBER".to_string(),
            target: BattleSide::Enemy,
            status: "BURN".to_string(),
            target_type1: "FIRE".to_string(),
            target_type2: "FIRE".to_string(),
        }));
    }

    #[test]
    fn direct_poison_respects_poison_and_steel_type_immunity() {
        let player = pokemon("GASTLY", 50, pokemon_type("GHOST"), "POISONPOWDER");
        let mut enemy = pokemon("MAGNEMITE", 40, pokemon_type("ELECTRIC"), "TACKLE");
        enemy.species.type2 = pokemon_type("STEEL");
        let moves = BTreeMap::from([
            (
                "POISONPOWDER".to_string(),
                move_data_with_effect("POISONPOWDER", pokemon_type("POISON"), 0, 100, "POISON"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("poison immunity turn resolves");

        assert_eq!(outcome.state.enemy.status, None);
        assert!(outcome.events.contains(&BattleEvent::StatusImmune {
            side: BattleSide::Player,
            move_name: "POISONPOWDER".to_string(),
            target: BattleSide::Enemy,
            status: "POISON".to_string(),
            target_type1: "ELECTRIC".to_string(),
            target_type2: "STEEL".to_string(),
        }));
    }

    #[test]
    fn damaging_secondary_freeze_effect_applies_after_damage() {
        let player = pokemon("JYNX", 50, pokemon_type("ICE"), "ICE_BEAM");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "ICE_BEAM".to_string(),
                move_data_with_effect_chance(
                    "ICE_BEAM",
                    pokemon_type("ICE"),
                    95,
                    100,
                    "FREEZE_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("freeze secondary turn resolves");

        assert_eq!(outcome.state.enemy.status.as_deref(), Some("FREEZE"));
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "ICE_BEAM".to_string(),
            target: BattleSide::Enemy,
            status: "FREEZE".to_string(),
        }));
    }

    #[test]
    fn tri_attack_randomly_applies_one_of_its_three_secondary_statuses() {
        let player = pokemon("DODRIO", 50, pokemon_type("NORMAL"), "TRI_ATTACK");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "TRI_ATTACK".to_string(),
                move_data_with_effect_chance(
                    "TRI_ATTACK",
                    pokemon_type("NORMAL"),
                    80,
                    100,
                    "TRI_ATTACK",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(7);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("tri attack secondary status resolves");

        let status = outcome
            .state
            .enemy
            .status
            .as_deref()
            .expect("Tri Attack applies one status at 100 percent chance");
        assert!(matches!(status, "BURN" | "FREEZE" | "PARALYSIS"));
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::StatusApplied {
            side: BattleSide::Player,
            move_name: "TRI_ATTACK".to_string(),
            target: BattleSide::Enemy,
            status: status.to_string(),
        }));
    }

    #[test]
    fn damaging_secondary_freeze_respects_ice_type_immunity() {
        let player = pokemon("JYNX", 50, pokemon_type("ICE"), "ICE_BEAM");
        let enemy = pokemon("DEWGONG", 40, pokemon_type("ICE"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "ICE_BEAM".to_string(),
                move_data_with_effect_chance(
                    "ICE_BEAM",
                    pokemon_type("ICE"),
                    95,
                    100,
                    "FREEZE_HIT",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("freeze immunity turn resolves");

        assert_eq!(outcome.state.enemy.status, None);
        assert!(outcome.events.contains(&BattleEvent::StatusImmune {
            side: BattleSide::Player,
            move_name: "ICE_BEAM".to_string(),
            target: BattleSide::Enemy,
            status: "FREEZE".to_string(),
            target_type1: "ICE".to_string(),
            target_type2: "ICE".to_string(),
        }));
    }

    #[test]
    fn frozen_pokemon_spends_pp_without_moving() {
        let mut player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("FREEZE".to_string());
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("frozen turn resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.status.as_deref(), Some("FREEZE"));
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::FrozenTurn {
            side: BattleSide::Player,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn flame_wheel_thaws_frozen_user_and_attacks() {
        let mut player = pokemon("CYNDAQUIL", 50, pokemon_type("FIRE"), "FLAME_WHEEL");
        player.status = Some("FREEZE".to_string());
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "FLAME_WHEEL".to_string(),
                move_data_with_effect_chance(
                    "FLAME_WHEEL",
                    pokemon_type("FIRE"),
                    60,
                    100,
                    "FLAME_WHEEL",
                    0,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(3);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("flame wheel thaw turn resolves");

        assert_eq!(outcome.state.player.status, None);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::StatusHealed {
            side: BattleSide::Player,
            move_name: "FLAME_WHEEL".to_string(),
            target: BattleSide::Player,
            status_before: "FREEZE".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::FrozenTurn { .. }))
        );
    }

    #[test]
    fn sleeping_pokemon_spends_pp_and_loses_sleep_turn_without_moving() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("SLEEP".to_string());
        player.sleep_turns = 2;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("sleep turn resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.status.as_deref(), Some("SLEEP"));
        assert_eq!(outcome.state.player.sleep_turns, 1);
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::SleepTurn {
            side: BattleSide::Player,
            move_name: "TACKLE".to_string(),
            turns_remaining: 1,
        }));
    }

    #[test]
    fn snore_can_attack_while_sleeping_and_ticks_sleep_counter() {
        let mut player = pokemon("SNORLAX", 50, pokemon_type("NORMAL"), "SNORE");
        player.status = Some("SLEEP".to_string());
        player.sleep_turns = 2;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SNORE".to_string(),
                move_data_with_effect_chance("SNORE", pokemon_type("NORMAL"), 40, 100, "SNORE", 0),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("snore sleeping turn resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.status.as_deref(), Some("SLEEP"));
        assert_eq!(outcome.state.player.sleep_turns, 1);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::SleepTurn {
            side: BattleSide::Player,
            move_name: "SNORE".to_string(),
            turns_remaining: 1,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SNORE"
        )));
    }

    #[test]
    fn snore_secondary_flinch_uses_exported_effect_chance() {
        let mut player = pokemon("SNORLAX", 90, pokemon_type("NORMAL"), "SNORE");
        player.status = Some("SLEEP".to_string());
        player.sleep_turns = 2;
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SNORE".to_string(),
                move_data_with_effect_chance(
                    "SNORE",
                    pokemon_type("NORMAL"),
                    40,
                    100,
                    "SNORE",
                    100,
                ),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("snore flinch turn resolves");

        assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name: "SNORE".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(outcome.events.contains(&BattleEvent::Flinched {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn snore_fails_without_sleep_status() {
        let player = pokemon("SNORLAX", 50, pokemon_type("NORMAL"), "SNORE");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SNORE".to_string(),
                move_data_with_effect_chance("SNORE", pokemon_type("NORMAL"), 40, 100, "SNORE", 30),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("awake snore turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "SNORE".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SNORE"
        )));
    }

    #[test]
    fn sleep_talk_selects_existing_move_without_extra_pp_spend() {
        let mut player = pokemon("SNORLAX", 90, pokemon_type("NORMAL"), "SLEEP_TALK");
        player.status = Some("SLEEP".to_string());
        player.sleep_turns = 2;
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SLEEP_TALK".to_string(),
                move_data_with_effect("SLEEP_TALK", pokemon_type("NORMAL"), 0, 100, "SLEEP_TALK"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("sleep talk selected move resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.moves[1].current_pp, 5);
        assert_eq!(outcome.state.player.status.as_deref(), Some("SLEEP"));
        assert_eq!(outcome.state.player.sleep_turns, 1);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::SleepTalkSelected {
            side: BattleSide::Player,
            move_name: "SLEEP_TALK".to_string(),
            selected_slot: 1,
            selected_move: "TACKLE".to_string(),
            roll: 0,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn sleep_talk_fails_without_sleep_status() {
        let mut player = pokemon("SNORLAX", 90, pokemon_type("NORMAL"), "SLEEP_TALK");
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SLEEP_TALK".to_string(),
                move_data_with_effect("SLEEP_TALK", pokemon_type("NORMAL"), 0, 100, "SLEEP_TALK"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("awake sleep talk turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "SLEEP_TALK".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::SleepTalkSelected { .. }))
        );
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                ..
            }
        )));
    }

    #[test]
    fn sleep_talk_fails_without_callable_move_candidates() {
        let mut player = pokemon("SNORLAX", 90, pokemon_type("NORMAL"), "SLEEP_TALK");
        player.status = Some("SLEEP".to_string());
        player.sleep_turns = 2;
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "SLEEP_TALK".to_string(),
                move_data_with_effect("SLEEP_TALK", pokemon_type("NORMAL"), 0, 100, "SLEEP_TALK"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("empty sleep talk candidate turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::SleepTurn {
            side: BattleSide::Player,
            move_name: "SLEEP_TALK".to_string(),
            turns_remaining: 1,
        }));
        assert!(outcome.events.contains(&BattleEvent::SleepTalkFailed {
            side: BattleSide::Player,
            move_name: "SLEEP_TALK".to_string(),
        }));
    }

    #[test]
    fn mimic_replaces_user_slot_with_targets_last_move() {
        let player = pokemon("MR_MIME", 90, pokemon_type("PSYCHIC_TYPE"), "MIMIC");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "MIMIC".to_string(),
                move_data_with_effect("MIMIC", pokemon_type("NORMAL"), 0, 100, "MIMIC"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(18);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("mimic turn resolves");

        assert_eq!(outcome.state.player.moves[0].name, "TACKLE");
        assert_eq!(outcome.state.player.moves[0].current_pp, 5);
        assert_eq!(outcome.state.player.moves[0].pp_ups, 0);
        assert!(outcome.events.contains(&BattleEvent::MimicApplied {
            side: BattleSide::Player,
            move_name: "MIMIC".to_string(),
            slot: 0,
            replaced_move: "MIMIC".to_string(),
            copied_move: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn mimic_fails_without_targets_last_move() {
        let player = pokemon("MR_MIME", 90, pokemon_type("PSYCHIC_TYPE"), "MIMIC");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "MIMIC".to_string(),
                move_data_with_effect("MIMIC", pokemon_type("NORMAL"), 0, 100, "MIMIC"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(19);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("mimic failure turn resolves");

        assert_eq!(outcome.state.player.moves[0].name, "MIMIC");
        assert!(outcome.events.contains(&BattleEvent::MimicFailed {
            side: BattleSide::Player,
            move_name: "MIMIC".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn sketch_permanently_replaces_user_slot_with_targets_last_move() {
        let player = pokemon("SMEARGLE", 90, pokemon_type("NORMAL"), "SKETCH");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SKETCH".to_string(),
                move_data_with_effect("SKETCH", pokemon_type("NORMAL"), 0, 100, "SKETCH"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(20);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("sketch turn resolves");

        assert_eq!(outcome.state.player.moves[0].name, "TACKLE");
        assert_eq!(outcome.state.player.moves[0].current_pp, 35);
        assert_eq!(outcome.state.player.moves[0].pp_ups, 0);
        assert!(outcome.events.contains(&BattleEvent::SketchApplied {
            side: BattleSide::Player,
            move_name: "SKETCH".to_string(),
            slot: 0,
            replaced_move: "SKETCH".to_string(),
            copied_move: "TACKLE".to_string(),
            copied_pp: 35,
        }));
    }

    #[test]
    fn sketch_fails_without_targets_last_move() {
        let player = pokemon("SMEARGLE", 90, pokemon_type("NORMAL"), "SKETCH");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "SKETCH".to_string(),
                move_data_with_effect("SKETCH", pokemon_type("NORMAL"), 0, 100, "SKETCH"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(21);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("sketch failure turn resolves");

        assert_eq!(outcome.state.player.moves[0].name, "SKETCH");
        assert!(outcome.events.contains(&BattleEvent::SketchFailed {
            side: BattleSide::Player,
            move_name: "SKETCH".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn transform_copies_target_battle_state_without_mutating_base_pokemon() {
        let player = pokemon("DITTO", 50, pokemon_type("NORMAL"), "TRANSFORM");
        let mut enemy = pokemon("MEW", 90, pokemon_type("PSYCHIC_TYPE"), "PSYCHIC_M");
        enemy.moves.push(LearnedMove {
            name: "SKETCH".to_string(),
            current_pp: 1,
            pp_ups: 0,
        });
        enemy.dvs = Dv::from_non_hp(15, 14, 13, 12);
        enemy.stat_boosts.insert(Stat::Attack, 2);
        enemy.stat_boosts.insert(Stat::Speed, -1);
        let moves = BTreeMap::from([
            (
                "TRANSFORM".to_string(),
                move_data_with_effect("TRANSFORM", pokemon_type("NORMAL"), 0, 100, "TRANSFORM"),
            ),
            (
                "PSYCHIC_M".to_string(),
                move_data("PSYCHIC_M", pokemon_type("PSYCHIC_TYPE"), 90, 100),
            ),
            (
                "SKETCH".to_string(),
                move_data_with_effect("SKETCH", pokemon_type("NORMAL"), 0, 100, "SKETCH"),
            ),
        ]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("transform turn resolves");

        assert_eq!(outcome.state.player.species.id, "DITTO");
        assert_eq!(outcome.state.player.moves[0].name, "TRANSFORM");
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        let transform = outcome
            .state
            .player_transform
            .as_ref()
            .expect("player transform state");
        assert_eq!(transform.species.id, "MEW");
        assert_eq!(transform.moves[0].name, "PSYCHIC_M");
        assert_eq!(transform.moves[0].current_pp, 5);
        assert_eq!(transform.moves[1].name, "SKETCH");
        assert_eq!(transform.moves[1].current_pp, 1);
        assert_eq!(transform.dvs, Dv::from_non_hp(15, 14, 13, 12));
        assert_eq!(transform.stat_boosts.get(&Stat::Attack), Some(&2));
        assert_eq!(transform.stat_boosts.get(&Stat::Speed), Some(&-1));
        let effective = effective_battle_pokemon(&outcome.state, BattleSide::Player);
        assert_eq!(effective.species.id, "MEW");
        assert_eq!(effective.moves[0].name, "PSYCHIC_M");
        assert!(outcome.events.contains(&BattleEvent::TransformApplied {
            side: BattleSide::Player,
            move_name: "TRANSFORM".to_string(),
            target: BattleSide::Enemy,
            species: "MEW".to_string(),
        }));
    }

    #[test]
    fn transformed_pokemon_uses_copied_move_slots_without_mutating_base_moves() {
        let player = pokemon("DITTO", 50, pokemon_type("NORMAL"), "TRANSFORM");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let mut rng = Random::new(23);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_transform = Some(BattleTransformState {
            species: species("RATTATA", 30, pokemon_type("NORMAL")),
            dvs: Dv::from_non_hp(10, 10, 10, 10),
            moves: vec![LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 5,
                pp_ups: 0,
            }],
            stat_boosts: state.player.stat_boosts.clone(),
            attack: state.player.attack,
            defense: state.player.defense,
            speed: state.player.speed,
            special_attack: state.player.special_attack,
            special_defense: state.player.special_defense,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("transformed copied move resolves");

        assert_eq!(outcome.state.player.moves[0].name, "TRANSFORM");
        assert_eq!(outcome.state.player.moves[0].current_pp, 5);
        let transform = outcome.state.player_transform.as_ref().unwrap();
        assert_eq!(transform.moves[0].name, "TACKLE");
        assert_eq!(transform.moves[0].current_pp, 4);
        assert!(outcome.events.contains(&BattleEvent::MoveUsed {
            side: BattleSide::Player,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn transform_fails_against_already_transformed_target() {
        let player = pokemon("DITTO", 50, pokemon_type("NORMAL"), "TRANSFORM");
        let enemy = pokemon("DITTO", 30, pokemon_type("NORMAL"), "TRANSFORM");
        let moves = BTreeMap::from([(
            "TRANSFORM".to_string(),
            move_data_with_effect("TRANSFORM", pokemon_type("NORMAL"), 0, 100, "TRANSFORM"),
        )]);
        let mut rng = Random::new(24);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_transform = Some(BattleTransformState {
            species: species("RATTATA", 30, pokemon_type("NORMAL")),
            dvs: Dv::from_non_hp(10, 10, 10, 10),
            moves: vec![LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 5,
                pp_ups: 0,
            }],
            stat_boosts: state.enemy.stat_boosts.clone(),
            attack: state.enemy.attack,
            defense: state.enemy.defense,
            speed: state.enemy.speed,
            special_attack: state.enemy.special_attack,
            special_defense: state.enemy.special_defense,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("transform failure resolves");

        assert_eq!(outcome.state.player_transform, None);
        assert!(outcome.events.contains(&BattleEvent::TransformFailed {
            side: BattleSide::Player,
            move_name: "TRANSFORM".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn switching_clears_transform_state() {
        let player = pokemon("DITTO", 50, pokemon_type("NORMAL"), "TRANSFORM");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([(
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        )]);
        let mut rng = Random::new(25);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_transform = Some(BattleTransformState {
            species: species("RATTATA", 30, pokemon_type("NORMAL")),
            dvs: Dv::from_non_hp(10, 10, 10, 10),
            moves: vec![LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 5,
                pp_ups: 0,
            }],
            stat_boosts: state.player.stat_boosts.clone(),
            attack: state.player.attack,
            defense: state.player.defense,
            speed: state.player.speed,
            special_attack: state.player.special_attack,
            special_defense: state.player.special_defense,
        });

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("transform switch resolves");

        assert_eq!(outcome.state.player_transform, None);
        assert!(outcome.events.contains(&BattleEvent::Switched {
            side: BattleSide::Player,
            party_index: 1,
        }));
    }

    #[test]
    fn conversion_changes_user_to_known_move_type_without_mutating_species() {
        let mut player = pokemon("PORYGON", 90, pokemon_type("NORMAL"), "CONVERSION");
        player.moves.push(LearnedMove {
            name: "EMBER".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "CONVERSION".to_string(),
                move_data_with_effect("CONVERSION", pokemon_type("NORMAL"), 0, 100, "CONVERSION"),
            ),
            (
                "EMBER".to_string(),
                move_data_with_effect("EMBER", pokemon_type("FIRE"), 40, 100, "BURN_HIT"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(20);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("conversion turn resolves");

        assert_eq!(
            outcome.state.player_type_override,
            Some(BattleTypeOverride {
                type1: pokemon_type("FIRE"),
                type2: pokemon_type("FIRE"),
            })
        );
        assert_eq!(outcome.state.player.species.type1, pokemon_type("NORMAL"));
        assert!(outcome.events.contains(&BattleEvent::ConversionApplied {
            side: BattleSide::Player,
            move_name: "CONVERSION".to_string(),
            selected_move: "EMBER".to_string(),
            new_type: pokemon_type("FIRE"),
            roll: 0,
        }));
    }

    #[test]
    fn conversion_type_override_controls_status_immunity() {
        let mut player = pokemon("PORYGON", 90, pokemon_type("NORMAL"), "CONVERSION");
        player.moves.push(LearnedMove {
            name: "EMBER".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("MAGMAR", 10, pokemon_type("FIRE"), "WILL_O_WISP");
        let moves = BTreeMap::from([
            (
                "CONVERSION".to_string(),
                move_data_with_effect("CONVERSION", pokemon_type("NORMAL"), 0, 100, "CONVERSION"),
            ),
            (
                "EMBER".to_string(),
                move_data_with_effect("EMBER", pokemon_type("FIRE"), 40, 100, "BURN_HIT"),
            ),
            (
                "WILL_O_WISP".to_string(),
                move_data_with_effect("WILL_O_WISP", pokemon_type("FIRE"), 0, 100, "BURN"),
            ),
        ]);
        let mut rng = Random::new(21);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_type_override = Some(BattleTypeOverride {
            type1: pokemon_type("FIRE"),
            type2: pokemon_type("FIRE"),
        });

        let outcome = resolve_battle_turn(
            state,
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
        .expect("conversion immunity turn resolves");

        assert_eq!(outcome.state.player.status, None);
        assert!(outcome.events.contains(&BattleEvent::StatusImmune {
            side: BattleSide::Enemy,
            move_name: "WILL_O_WISP".to_string(),
            target: BattleSide::Player,
            status: "BURN".to_string(),
            target_type1: pokemon_type("FIRE"),
            target_type2: pokemon_type("FIRE"),
        }));
    }

    #[test]
    fn conversion2_changes_user_to_type_that_resists_last_damaging_move() {
        let player = pokemon("PORYGON", 5, pokemon_type("NORMAL"), "CONVERSION2");
        let enemy = pokemon("MAGMAR", 90, pokemon_type("FIRE"), "EMBER");
        let moves = BTreeMap::from([
            (
                "CONVERSION2".to_string(),
                move_data_with_effect("CONVERSION2", pokemon_type("NORMAL"), 0, 100, "CONVERSION2"),
            ),
            (
                "EMBER".to_string(),
                move_data_with_effect("EMBER", pokemon_type("FIRE"), 40, 100, "BURN_HIT"),
            ),
        ]);
        let conversion2_type_categories = TypeCategories {
            physical: vec!["NORMAL".to_string(), "ROCK".to_string()],
            special: vec!["FIRE".to_string(), "WATER".to_string()],
        };
        let conversion2_type_effectiveness = TypeEffectivenessTable {
            matchups: BTreeMap::from([
                (
                    "FIRE".to_string(),
                    BTreeMap::from([
                        (
                            "NORMAL".to_string(),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            "FIRE".to_string(),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 1,
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
                        (
                            "ROCK".to_string(),
                            crate::battle::damage::TypeMultiplier {
                                numerator: 1,
                                denominator: 2,
                            },
                        ),
                    ]),
                ),
                (
                    "NORMAL".to_string(),
                    BTreeMap::from([
                        (
                            "NORMAL".to_string(),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            "FIRE".to_string(),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            "WATER".to_string(),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                        (
                            "ROCK".to_string(),
                            crate::battle::damage::TypeMultiplier::one(),
                        ),
                    ]),
                ),
            ]),
            foresight_matchups: BTreeMap::new(),
        };
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &conversion2_type_categories,
            &conversion2_type_effectiveness,
            &weather_modifiers(),
            &mut rng,
        )
        .expect("conversion2 turn resolves");

        let override_type = outcome
            .state
            .player_type_override
            .as_ref()
            .expect("conversion2 type override")
            .type1
            .clone();
        let multiplier = calculate_type_effectiveness_multiplier_with_foresight(
            &conversion2_type_effectiveness,
            pokemon_type("FIRE"),
            std::slice::from_ref(&override_type),
            false,
        )
        .expect("conversion2 selected declared type effectiveness");
        assert!(multiplier.numerator == 0 || multiplier.numerator < multiplier.denominator);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Conversion2Applied {
                side: BattleSide::Player,
                move_name,
                source_move,
                source_type,
                new_type,
                ..
            } if move_name == "CONVERSION2"
                && source_move == "EMBER"
                && source_type == &pokemon_type("FIRE")
                && new_type == &override_type
        )));
    }

    #[test]
    fn conversion2_fails_without_prior_damage() {
        let player = pokemon("PORYGON", 90, pokemon_type("NORMAL"), "CONVERSION2");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let moves = BTreeMap::from([
            (
                "CONVERSION2".to_string(),
                move_data_with_effect("CONVERSION2", pokemon_type("NORMAL"), 0, 100, "CONVERSION2"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(23);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("conversion2 failure turn resolves");

        assert_eq!(outcome.state.player_type_override, None);
        assert!(outcome.events.contains(&BattleEvent::Conversion2Failed {
            side: BattleSide::Player,
            move_name: "CONVERSION2".to_string(),
        }));
    }

    #[test]
    fn mirror_move_copies_targets_last_move_without_extra_pp_spend() {
        let mut player = pokemon("PIDGEY", 90, pokemon_type("FLYING"), "MIRROR_MOVE");
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "MIRROR_MOVE".to_string(),
                move_data_with_effect("MIRROR_MOVE", pokemon_type("FLYING"), 0, 100, "MIRROR_MOVE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(18);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some("TACKLE".to_string());

        let outcome = resolve_battle_turn(
            state,
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
        .expect("mirror move copied move resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.moves[1].current_pp, 5);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::MirrorMoveSelected {
            side: BattleSide::Player,
            move_name: "MIRROR_MOVE".to_string(),
            copied_move: "TACKLE".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn mirror_move_fails_without_targets_last_move() {
        let player = pokemon("PIDGEY", 90, pokemon_type("FLYING"), "MIRROR_MOVE");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "MIRROR_MOVE".to_string(),
                move_data_with_effect("MIRROR_MOVE", pokemon_type("FLYING"), 0, 100, "MIRROR_MOVE"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
        ]);
        let mut rng = Random::new(19);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("mirror move failure resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::MirrorMoveFailed {
            side: BattleSide::Player,
            move_name: "MIRROR_MOVE".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                ..
            }
        )));
    }

    #[test]
    fn metronome_selects_pack_move_deterministically_without_extra_pp_spend() {
        let mut player = pokemon("CLEFAIRY", 90, pokemon_type("NORMAL"), "METRONOME");
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "METRONOME".to_string(),
                move_data_with_effect("METRONOME", pokemon_type("NORMAL"), 0, 100, "METRONOME"),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("metronome selected move resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.moves[1].current_pp, 5);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::MetronomeSelected {
            side: BattleSide::Player,
            move_name: "METRONOME".to_string(),
            selected_move: "TACKLE".to_string(),
            roll: 1,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn metronome_fails_when_pack_has_no_other_move_candidate() {
        let player = pokemon("CLEFAIRY", 90, pokemon_type("NORMAL"), "METRONOME");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "METRONOME");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "METRONOME".to_string(),
            move_data_with_effect("METRONOME", pokemon_type("NORMAL"), 0, 100, "METRONOME"),
        )]);
        let mut rng = Random::new(21);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("metronome failure resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::MetronomeFailed {
            side: BattleSide::Player,
            move_name: "METRONOME".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                ..
            }
        )));
    }

    #[test]
    fn zero_sleep_turns_wakes_before_move_resolution() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("SLEEP".to_string());
        player.sleep_turns = 0;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(19);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("wake turn resolves");

        assert_eq!(outcome.state.player.status, None);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::WokeUp {
            side: BattleSide::Player,
            move_name: "TACKLE".to_string(),
        }));
    }

    #[test]
    fn paralysis_can_prevent_attempted_move_deterministically() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("PARALYSIS".to_string());
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(0);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("paralysis turn resolves");

        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::FullyParalyzed {
            side: BattleSide::Player,
            move_name: "TACKLE".to_string(),
            roll: 0,
        }));
    }

    #[test]
    fn poison_deals_end_turn_residual_damage_after_actions() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("POISON".to_string());
        let player_hp = player.hp;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 0;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(23);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("poison residual turn resolves");

        let residual = (player_hp / 8).max(1);
        assert_eq!(outcome.state.player.hp, player_hp - residual);
        assert!(outcome.events.contains(&BattleEvent::ResidualStatusDamage {
            side: BattleSide::Player,
            status: "POISON".to_string(),
            damage: residual,
            hp_before: player_hp,
            hp_after: player_hp - residual,
        }));
    }

    #[test]
    fn held_poison_cure_berry_consumes_before_residual_damage() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("POISON".to_string());
        player.item = Some("PSNCURE_BERRY".to_string());
        let player_hp = player.hp;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 0;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let items = BTreeMap::from([(
            "PSNCURE_BERRY".to_string(),
            held_status_item("PSNCURE_BERRY", "HELD_HEAL_POISON"),
        )]);
        let mut rng = Random::new(23);

        let outcome = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
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
        .expect("held poison cure turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert_eq!(outcome.state.player.status, None);
        assert_eq!(outcome.state.player.item, None);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::ResidualStatusDamage { .. }))
        );
        assert!(outcome.events.contains(&BattleEvent::HeldItemStatusHealed {
            side: BattleSide::Player,
            item_id: "PSNCURE_BERRY".to_string(),
            held_effect: "HELD_HEAL_POISON".to_string(),
            status_before: Some("POISON".to_string()),
            confusion_turns_before: 0,
        }));
    }

    #[test]
    fn held_confusion_cure_berry_consumes_without_status_aliases() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.confusion_turns = 3;
        player.item = Some("BITTER_BERRY".to_string());
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 0;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let items = BTreeMap::from([(
            "BITTER_BERRY".to_string(),
            held_status_item("BITTER_BERRY", "HELD_HEAL_CONFUSION"),
        )]);
        let mut rng = Random::new(23);

        let outcome = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
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
        .expect("held confusion cure turn resolves");

        assert_eq!(outcome.state.player.confusion_turns, 0);
        assert_eq!(outcome.state.player.item, None);
        assert!(outcome.events.contains(&BattleEvent::HeldItemStatusHealed {
            side: BattleSide::Player,
            item_id: "BITTER_BERRY".to_string(),
            held_effect: "HELD_HEAL_CONFUSION".to_string(),
            status_before: None,
            confusion_turns_before: 3,
        }));
    }

    #[test]
    fn bad_poison_residual_damage_scales_with_toxic_counter() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("BAD_POISON".to_string());
        let player_hp = player.hp;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = 0;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(43);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_toxic_turns = 3;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("bad poison residual turn resolves");

        let residual = (player_hp / 16).max(1) * 3;
        assert_eq!(outcome.state.player.hp, player_hp - residual);
        assert_eq!(outcome.state.player_toxic_turns, 4);
        assert!(outcome.events.contains(&BattleEvent::ResidualStatusDamage {
            side: BattleSide::Player,
            status: "BAD_POISON".to_string(),
            damage: residual,
            hp_before: player_hp,
            hp_after: player_hp - residual,
        }));
    }

    #[test]
    fn switching_resets_bad_poison_counter_without_removing_status() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("BAD_POISON".to_string());
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 44);
        state.player_toxic_turns = 4;

        clear_side_volatile_conditions(&mut state, BattleSide::Player);

        assert_eq!(state.player.status.as_deref(), Some("BAD_POISON"));
        assert_eq!(state.player_toxic_turns, 0);
    }

    #[test]
    fn burn_residual_damage_can_faint_at_end_of_turn() {
        let player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.status = Some("BURN".to_string());
        enemy.hp = 1;
        enemy.moves[0].current_pp = 0;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(29);

        let outcome = resolve_battle_enemy_action_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleAction::Move { slot: 0 },
            &moves,
            &BTreeMap::new(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("burn residual turn resolves");

        assert_eq!(outcome.state.enemy.hp, 0);
        assert!(outcome.events.contains(&BattleEvent::ResidualStatusDamage {
            side: BattleSide::Enemy,
            status: "BURN".to_string(),
            damage: 1,
            hp_before: 1,
            hp_after: 0,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
    }

    #[test]
    fn residual_status_damage_does_not_apply_after_action_faint() {
        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "QUICK_ATTACK");
        player.status = Some("POISON".to_string());
        let player_hp = player.hp;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 1;
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
        let mut rng = Random::new(31);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("fainting turn resolves");

        assert_eq!(outcome.state.enemy.hp, 0);
        assert_eq!(outcome.state.player.hp, player_hp);
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Enemy,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::ResidualStatusDamage { .. }))
        );
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
            &battle_state(player, enemy, rng.seed()),
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
    fn paralysis_speed_penalty_affects_turn_order_for_exact_status_token() {
        let mut player = pokemon("RATTATA", 100, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("PARALYSIS".to_string());
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let order = determine_turn_order(
            &battle_state(player, enemy, rng.seed()),
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

        assert_eq!(order, vec![BattleSide::Enemy, BattleSide::Player]);
    }

    #[test]
    fn quick_claw_can_override_speed_with_exact_held_effect_and_parameter() {
        let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
        player.item = Some("QUICK_CLAW".to_string());
        let enemy = pokemon("PIDGEY", 100, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let items = BTreeMap::from([("QUICK_CLAW".to_string(), quick_claw_item(60))]);
        let mut rng = Random::new(20);

        let order = determine_turn_order(
            &battle_state(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &mut rng,
        )
        .expect("turn order resolves");

        assert_eq!(order, vec![BattleSide::Player, BattleSide::Enemy]);
    }

    #[test]
    fn quick_claw_rejects_invalid_pack_parameter_without_probability_fallback() {
        let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
        player.item = Some("QUICK_CLAW".to_string());
        let enemy = pokemon("PIDGEY", 100, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let items = BTreeMap::from([("QUICK_CLAW".to_string(), quick_claw_item(0))]);
        let mut rng = Random::new(20);

        let error = determine_turn_order(
            &battle_state(player, enemy, rng.seed()),
            &BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &mut rng,
        )
        .expect_err("invalid Quick Claw parameter must fail");

        assert_eq!(
            error,
            BattleTurnError::InvalidHeldItemParameter {
                side: BattleSide::Player,
                item_id: "QUICK_CLAW".to_string(),
                held_effect: "HELD_QUICK_CLAW".to_string(),
                parameter: 0,
            }
        );
    }

    #[test]
    fn paralysis_speed_penalty_does_not_coerce_malformed_status_token() {
        let mut player = pokemon("RATTATA", 100, pokemon_type("NORMAL"), "TACKLE");
        player.status = Some("paralysis".to_string());
        let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let order = determine_turn_order(
            &battle_state(player, enemy, rng.seed()),
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
            battle_state(player, enemy, rng.seed()),
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
            battle_state(player, enemy, rng.seed()),
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
    fn berserk_gene_activates_on_switch_in_with_exact_held_effect() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let items = [(
            "BERSERK_GENE".to_string(),
            held_status_item("BERSERK_GENE", "HELD_ATTACK_UP"),
        )]
        .into_iter()
        .collect();
        let mut state = battle_state(player, enemy, 1);
        state.player_party[1].item = Some("BERSERK_GENE".to_string());
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn_with_items(
            state,
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
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
        .expect("Berserk Gene switch turn resolves");

        assert_eq!(outcome.state.player.item, None);
        assert_eq!(
            outcome.state.player.stat_boosts.get(&Stat::Attack).copied(),
            Some(2)
        );
        assert!((2..=5).contains(&outcome.state.player.confusion_turns));
        assert!(outcome.events.contains(&BattleEvent::HeldItemActivated {
            side: BattleSide::Player,
            item_id: "BERSERK_GENE".to_string(),
            held_effect: "HELD_ATTACK_UP".to_string(),
        }));
        assert!(outcome.events.contains(&BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name: "HELD_ATTACK_UP".to_string(),
            target: BattleSide::Player,
            stat: Stat::Attack,
            amount: 2,
            stage_before: 0,
            stage_after: 2,
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
            battle_state(player, enemy, rng.seed()),
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
            battle_state(player.clone(), enemy.clone(), rng.seed()),
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
            battle_state(player.clone(), enemy.clone(), rng.seed()),
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
            battle_state(player.clone(), enemy.clone(), rng.seed()),
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
    fn generic_battle_turn_rejects_run_without_escape_rules() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let error = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("generic/trainer battle turns cannot flee without escape rules");

        assert_eq!(
            error,
            BattleTurnError::RunNotAllowed {
                side: BattleSide::Player
            }
        );

        let enemy_error = resolve_battle_turn_with_items(
            battle_state(
                pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE"),
                pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE"),
                rng.seed(),
            ),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Run,
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("generic/trainer enemy turns cannot flee without escape rules");

        assert_eq!(
            enemy_error,
            BattleTurnError::RunNotAllowed {
                side: BattleSide::Enemy
            }
        );
    }

    #[test]
    fn battle_turn_rejects_fainted_active_pokemon_before_advancing_turn() {
        let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        player.hp = 0;
        let enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let error = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("fainted active player must switch through the battle party path");

        assert_eq!(
            error,
            BattleTurnError::ActivePokemonFainted {
                side: BattleSide::Player
            }
        );

        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let mut enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 0;
        let error = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("fainted active enemy must resolve rewards or trainer advance first");

        assert_eq!(
            error,
            BattleTurnError::ActivePokemonFainted {
                side: BattleSide::Enemy
            }
        );
    }

    #[test]
    fn enemy_battle_response_rejects_fainted_active_player() {
        let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        player.hp = 0;
        let enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let error = resolve_battle_enemy_action_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleAction::Move { slot: 0 },
            &moves,
            &BTreeMap::new(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect_err("enemy response cannot advance while player replacement is pending");

        assert_eq!(
            error,
            BattleTurnError::ActivePokemonFainted {
                side: BattleSide::Player
            }
        );
    }

    #[test]
    fn wild_battle_turn_run_can_escape_before_enemy_action() {
        let player = pokemon("RATTATA", 999, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let outcome = resolve_wild_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            2,
            &mut rng,
        )
        .expect("run resolves through wild escape rules");

        assert_eq!(outcome.order, vec![BattleSide::Player]);
        assert_eq!(outcome.state.turn, 1);
        assert!(matches!(
            &outcome.events[..],
            [BattleEvent::RunAttempt {
                side: BattleSide::Player,
                outcome
            }] if outcome.escaped && outcome.attempts_before == 2
        ));
    }

    #[test]
    fn failed_wild_battle_turn_run_allows_enemy_action() {
        let player = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 999, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let outcome = resolve_wild_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 1,
                enemy_speed_divisor: 1,
                failed_attempt_bonus: 0,
                rng_roll_values: 256,
            },
            0,
            &mut rng,
        )
        .expect("failed run still resolves the enemy action");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert!(matches!(
            outcome.events.first(),
            Some(BattleEvent::RunAttempt {
                side: BattleSide::Player,
                outcome
            }) if !outcome.escaped && outcome.attempts_after == 1
        ));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::MoveSelected {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }

    #[test]
    fn held_escape_item_forces_wild_run_without_consuming_item() {
        let mut player = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        player.item = Some("SMOKE_BALL".to_string());
        let enemy = pokemon("PIDGEY", 999, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let items = BTreeMap::from([(
            "SMOKE_BALL".to_string(),
            held_status_item("SMOKE_BALL", "HELD_ESCAPE"),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_wild_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 1,
                enemy_speed_divisor: 1,
                failed_attempt_bonus: 0,
                rng_roll_values: 256,
            },
            3,
            &mut rng,
        )
        .expect("held escape item forces wild battle escape");

        assert_eq!(outcome.order, vec![BattleSide::Player]);
        assert_eq!(outcome.state.player.item, Some("SMOKE_BALL".to_string()));
        assert_eq!(outcome.state.turn, 1);
        assert!(outcome.events.contains(&BattleEvent::HeldItemEscape {
            side: BattleSide::Player,
            item_id: "SMOKE_BALL".to_string(),
            held_effect: "HELD_ESCAPE".to_string(),
        }));
        assert!(matches!(
            outcome.events.last(),
            Some(BattleEvent::RunAttempt {
                side: BattleSide::Player,
                outcome
            }) if outcome.escaped
                && outcome.roll == None
                && outcome.chance == 256
                && outcome.attempts_before == 3
                && outcome.attempts_after == 3
        ));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::MoveSelected { .. }))
        );
    }

    #[test]
    fn escape_trap_blocks_held_escape_item() {
        let mut player = pokemon("RATTATA", 999, pokemon_type("NORMAL"), "TACKLE");
        player.item = Some("SMOKE_BALL".to_string());
        let enemy = pokemon("GASTLY", 10, pokemon_type("GHOST"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_escape_trap = Some(BattleEscapeTrapState {
            source: BattleSide::Enemy,
            move_name: "MEAN_LOOK".to_string(),
        });
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let items = BTreeMap::from([(
            "SMOKE_BALL".to_string(),
            held_status_item("SMOKE_BALL", "HELD_ESCAPE"),
        )]);
        let mut rng = Random::new(21);

        let outcome = resolve_wild_battle_turn_with_items(
            state,
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &items,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            2,
            &mut rng,
        )
        .expect("escape trap still blocks held escape item");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert!(outcome.events.contains(&BattleEvent::RunBlocked {
            side: BattleSide::Player,
            source: BattleSide::Enemy,
            move_name: "MEAN_LOOK".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::HeldItemEscape { .. }))
        );
    }

    #[test]
    fn mean_look_applies_escape_trap_without_damage() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "MEAN_LOOK");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([
            (
                "MEAN_LOOK".to_string(),
                move_data_with_effect("MEAN_LOOK", pokemon_type("NORMAL"), 0, 100, "MEAN_LOOK"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(17);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("mean look turn resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert_eq!(
            outcome.state.enemy_escape_trap,
            Some(BattleEscapeTrapState {
                source: BattleSide::Player,
                move_name: "MEAN_LOOK".to_string(),
            })
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::EscapeTrapApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
            } if move_name == "MEAN_LOOK"
        )));
    }

    #[test]
    fn jump_kick_hit_resolves_as_exported_damage_move() {
        let player = pokemon("HITMONLEE", 90, pokemon_type("FIGHTING"), "JUMP_KICK");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "JUMP_KICK".to_string(),
            move_data_with_effect("JUMP_KICK", pokemon_type("FIGHTING"), 70, 100, "JUMP_KICK"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("jump kick hit resolves");

        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "JUMP_KICK"
        )));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::JumpKickCrash { .. }))
        );
    }

    #[test]
    fn jump_kick_miss_crashes_for_half_max_hp() {
        let player = pokemon("HITMONLEE", 90, pokemon_type("FIGHTING"), "JUMP_KICK");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let player_hp = player.hp;
        let crash_damage = (player.max_hp / 2).max(1);
        let moves = BTreeMap::from([(
            "JUMP_KICK".to_string(),
            move_data_with_effect("JUMP_KICK", pokemon_type("FIGHTING"), 70, 1, "JUMP_KICK"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("jump kick miss resolves");

        assert_eq!(outcome.state.player.hp, player_hp - crash_damage);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Missed {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "JUMP_KICK"
        )));
        assert!(outcome.events.contains(&BattleEvent::JumpKickCrash {
            side: BattleSide::Player,
            move_name: "JUMP_KICK".to_string(),
            crash_damage,
            hp_before: player_hp,
            hp_after: player_hp - crash_damage,
        }));
    }

    #[test]
    fn jump_kick_no_effect_crashes_and_can_faint_user() {
        let mut player = pokemon("HITMONLEE", 90, pokemon_type("FIGHTING"), "JUMP_KICK");
        player.hp = 10;
        player.max_hp = 44;
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([(
            "JUMP_KICK".to_string(),
            move_data_with_effect("JUMP_KICK", pokemon_type("FIGHTING"), 70, 100, "JUMP_KICK"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("jump kick no-effect resolves");

        assert_eq!(outcome.state.player.hp, 0);
        assert!(outcome.events.contains(&BattleEvent::NoEffect {
            side: BattleSide::Player,
            move_name: "JUMP_KICK".to_string(),
        }));
        assert!(outcome.events.contains(&BattleEvent::JumpKickCrash {
            side: BattleSide::Player,
            move_name: "JUMP_KICK".to_string(),
            crash_damage: 10,
            hp_before: 10,
            hp_after: 0,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fainted {
            side: BattleSide::Player,
        }));
    }

    #[test]
    fn rampage_first_use_starts_forced_turns_after_damage() {
        let player = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "THRASH");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([(
            "THRASH".to_string(),
            move_data_with_effect("THRASH", pokemon_type("NORMAL"), 90, 100, "RAMPAGE"),
        )]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("rampage first use resolves");

        assert!((1..=2).contains(&outcome.state.player.rampage_turns));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RampageStarted {
                side: BattleSide::Player,
                move_name,
                turns_remaining: 1..=2,
                ..
            } if move_name == "THRASH"
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "THRASH"
        )));
    }

    #[test]
    fn rampage_forced_turn_uses_locked_move_without_extra_pp_and_then_confuses_user() {
        let mut player = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "THRASH");
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        player.rampage_turns = 1;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_last_move = Some("THRASH".to_string());
        let moves = BTreeMap::from([
            (
                "THRASH".to_string(),
                move_data_with_effect("THRASH", pokemon_type("NORMAL"), 90, 100, "RAMPAGE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("forced rampage turn resolves");

        assert_eq!(outcome.state.player.rampage_turns, 0);
        assert!((2..=5).contains(&outcome.state.player.confusion_turns));
        assert_eq!(outcome.state.player.moves[0].current_pp, 5);
        assert_eq!(outcome.state.player.moves[1].current_pp, 5);
        assert!(outcome.events.contains(&BattleEvent::RampageForcedMove {
            side: BattleSide::Player,
            requested_slot: 1,
            requested_move: "TACKLE".to_string(),
            rampage_slot: 0,
            rampage_move: "THRASH".to_string(),
            turns_remaining: 1,
        }));
        assert!(outcome.events.contains(&BattleEvent::RampageEnded {
            side: BattleSide::Player,
            move_name: "THRASH".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::ConfusionApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Player,
                ..
            } if move_name == "THRASH"
        )));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "THRASH"
        )));
    }

    #[test]
    fn lock_on_marks_next_player_move_as_sure_hit() {
        let player = pokemon("MAGNEMITE", 90, pokemon_type("ELECTRIC"), "LOCK_ON");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "LOCK_ON".to_string(),
                move_data_with_effect("LOCK_ON", pokemon_type("NORMAL"), 0, 100, "LOCK_ON"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(22);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("lock-on turn resolves");

        assert!(outcome.state.player_lock_on_target);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::LockOnApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
            } if move_name == "LOCK_ON"
        )));
    }

    #[test]
    fn lock_on_fails_when_user_already_has_target_locked() {
        let player = pokemon("MAGNEMITE", 90, pokemon_type("ELECTRIC"), "LOCK_ON");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_lock_on_target = true;
        let moves = BTreeMap::from([
            (
                "LOCK_ON".to_string(),
                move_data_with_effect("LOCK_ON", pokemon_type("NORMAL"), 0, 100, "LOCK_ON"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(23);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("duplicate lock-on turn resolves");

        assert!(outcome.state.player_lock_on_target);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::LockOnFailed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
            } if move_name == "LOCK_ON"
        )));
    }

    #[test]
    fn lock_on_consumes_sure_hit_without_accuracy_stage_table() {
        let player = pokemon("MAGNEMITE", 90, pokemon_type("ELECTRIC"), "TACKLE");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let mut state = battle_state(player, enemy, 0);
        state.player_lock_on_target = true;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 1),
        )]);
        let mut stat_multipliers = stat_multipliers();
        stat_multipliers.accuracy.clear();
        let mut rng = Random::new(24);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers,
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("lock-on bypasses accuracy table for next move");

        assert!(!outcome.state.player_lock_on_target);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::LockOnConsumed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
            } if move_name == "TACKLE"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Missed {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn switching_clears_lock_on_target_state() {
        let player = pokemon("MAGNEMITE", 90, pokemon_type("ELECTRIC"), "TACKLE");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_lock_on_target = true;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(25);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
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

        assert!(!outcome.state.player_lock_on_target);
    }

    #[test]
    fn attract_applies_to_opposite_gender_target() {
        let mut player = pokemon("JIGGLYPUFF", 90, pokemon_type("NORMAL"), "ATTRACT");
        player.species.gender_ratio = 127;
        player.dvs = Dv::from_non_hp(10, 10, 10, 10);
        let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        enemy.species.gender_ratio = 127;
        enemy.dvs = Dv::from_non_hp(2, 10, 10, 10);
        let moves = BTreeMap::from([
            (
                "ATTRACT".to_string(),
                move_data_with_effect("ATTRACT", pokemon_type("NORMAL"), 0, 100, "ATTRACT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(26);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("attract turn resolves");

        assert_eq!(outcome.state.enemy_attracted_by, Some(BattleSide::Player));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::AttractApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                user_gender: BattlePokemonGender::Male,
                target_gender: BattlePokemonGender::Female,
            } if move_name == "ATTRACT"
        )));
    }

    #[test]
    fn attract_fails_against_same_gender_target() {
        let mut player = pokemon("JIGGLYPUFF", 90, pokemon_type("NORMAL"), "ATTRACT");
        player.species.gender_ratio = 127;
        player.dvs = Dv::from_non_hp(10, 10, 10, 10);
        let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        enemy.species.gender_ratio = 127;
        enemy.dvs = Dv::from_non_hp(10, 10, 10, 10);
        let moves = BTreeMap::from([
            (
                "ATTRACT".to_string(),
                move_data_with_effect("ATTRACT", pokemon_type("NORMAL"), 0, 100, "ATTRACT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(27);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("same-gender attract turn resolves");

        assert_eq!(outcome.state.enemy_attracted_by, None);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::AttractFailed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                user_gender: Some(BattlePokemonGender::Male),
                target_gender: Some(BattlePokemonGender::Male),
            } if move_name == "ATTRACT"
        )));
    }

    #[test]
    fn infatuation_can_immobilize_before_accuracy_or_damage() {
        let player = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("JIGGLYPUFF", 90, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_attracted_by = Some(BattleSide::Player);
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(1);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("infatuated turn resolves");

        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::InfatuatedImmobilized {
                side: BattleSide::Enemy,
                move_name,
                source: BattleSide::Player,
                roll: 0,
            } if move_name == "TACKLE"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn switching_clears_attract_for_source_and_target() {
        let player = pokemon("JIGGLYPUFF", 90, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_attracted_by = Some(BattleSide::Player);
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(28);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("attract source switch turn resolves");

        assert_eq!(outcome.state.enemy_attracted_by, None);
    }

    #[test]
    fn fly_first_turn_enters_airborne_state_without_damage() {
        let player = pokemon("PIDGEOT", 100, pokemon_type("FLYING"), "FLY");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "FLY".to_string(),
            move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 95, "FLY"),
        )]);
        let mut rng = Random::new(31);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("fly first turn resolves");

        assert_eq!(outcome.state.player_airborne_move, Some("FLY".to_string()));
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert!(outcome.events.contains(&BattleEvent::AirborneStarted {
            side: BattleSide::Player,
            move_name: "FLY".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "FLY"
        )));
    }

    #[test]
    fn fly_second_turn_forces_stored_move_without_extra_pp_and_lands_damage() {
        let mut player = pokemon("PIDGEOT", 100, pokemon_type("FLYING"), "FLY");
        player.moves[0].current_pp = 4;
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let mut state = battle_state(player, enemy, 0);
        state.player_airborne_move = Some("FLY".to_string());
        let moves = BTreeMap::from([
            (
                "FLY".to_string(),
                move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 95, "FLY"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(32);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("fly landing turn resolves");

        assert_eq!(outcome.state.player_airborne_move, None);
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.moves[1].current_pp, 5);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::AirborneForcedMove {
            side: BattleSide::Player,
            requested_slot: 1,
            requested_move: "TACKLE".to_string(),
            airborne_slot: 0,
            airborne_move: "FLY".to_string(),
        }));
        assert!(outcome.events.contains(&BattleEvent::AirborneEnded {
            side: BattleSide::Player,
            move_name: "FLY".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "FLY"
        )));
    }

    #[test]
    fn fly_second_turn_overrides_non_move_action() {
        let mut player = pokemon("PIDGEOT", 100, pokemon_type("FLYING"), "FLY");
        player.moves[0].current_pp = 4;
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let mut state = battle_state(player, enemy, 0);
        state.player_airborne_move = Some("FLY".to_string());
        let moves = BTreeMap::from([(
            "FLY".to_string(),
            move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 100, "FLY"),
        )]);
        let mut rng = Random::new(34);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Switch { party_index: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("airborne forced switch override resolves");

        assert_eq!(outcome.state.player_airborne_move, None);
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::AirborneForcedMove {
            side: BattleSide::Player,
            requested_slot: 0,
            requested_move: "FLY".to_string(),
            airborne_slot: 0,
            airborne_move: "FLY".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Switched {
                side: BattleSide::Player,
                ..
            }
        )));
    }

    #[test]
    fn ordinary_move_misses_target_during_fly_airborne_turn() {
        let player = pokemon("PIDGEOT", 100, pokemon_type("FLYING"), "FLY");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "FLY".to_string(),
                move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 100, "FLY"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(33);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("fly avoidance turn resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert!(outcome.events.contains(&BattleEvent::AirborneAvoided {
            side: BattleSide::Enemy,
            move_name: "TACKLE".to_string(),
            target: BattleSide::Player,
            airborne_move: "FLY".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }

    #[test]
    fn earthquake_hits_and_doubles_power_against_dig_target() {
        let player = pokemon("DIGLETT", 40, pokemon_type("NORMAL"), "DIG");
        let enemy = pokemon("DONPHAN", 100, pokemon_type("GROUND"), "EARTHQUAKE");
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "DIG".to_string(),
                move_data_with_effect("DIG", pokemon_type("GROUND"), 60, 100, "FLY"),
            ),
            (
                "EARTHQUAKE".to_string(),
                move_data_with_effect("EARTHQUAKE", pokemon_type("GROUND"), 100, 100, "EARTHQUAKE"),
            ),
        ]);
        let mut rng = Random::new(34);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_airborne_move = Some("DIG".to_string());

        let outcome = resolve_battle_turn(
            state,
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
        .expect("earthquake against dig resolves");

        assert!(outcome.state.player.hp < player_hp);
        assert!(outcome.events.contains(&BattleEvent::EarthquakePower {
            side: BattleSide::Enemy,
            move_name: "EARTHQUAKE".to_string(),
            target_move: "DIG".to_string(),
            power: 200,
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == "EARTHQUAKE"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::AirborneAvoided {
                side: BattleSide::Enemy,
                move_name,
                airborne_move,
                ..
            } if move_name == "EARTHQUAKE" && airborne_move == "DIG"
        )));
    }

    #[test]
    fn earthquake_still_misses_actual_fly_target() {
        let player = pokemon("PIDGEOT", 40, pokemon_type("NORMAL"), "FLY");
        let enemy = pokemon("DONPHAN", 100, pokemon_type("GROUND"), "EARTHQUAKE");
        let player_hp = player.hp;
        let moves = BTreeMap::from([
            (
                "FLY".to_string(),
                move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 100, "FLY"),
            ),
            (
                "EARTHQUAKE".to_string(),
                move_data_with_effect("EARTHQUAKE", pokemon_type("GROUND"), 100, 100, "EARTHQUAKE"),
            ),
        ]);
        let mut rng = Random::new(35);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_airborne_move = Some("FLY".to_string());

        let outcome = resolve_battle_turn(
            state,
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
        .expect("earthquake against fly resolves");

        assert_eq!(outcome.state.player.hp, player_hp);
        assert!(outcome.events.contains(&BattleEvent::AirborneAvoided {
            side: BattleSide::Enemy,
            move_name: "EARTHQUAKE".to_string(),
            target: BattleSide::Player,
            airborne_move: "FLY".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::EarthquakePower { .. }))
        );
    }

    #[test]
    fn solarbeam_first_turn_charges_without_damage_outside_sun() {
        let player = pokemon("MEGANIUM", 80, pokemon_type("GRASS"), "SOLARBEAM");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "SOLARBEAM".to_string(),
            move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 120, 100, "SOLARBEAM"),
        )]);
        let mut rng = Random::new(35);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("solarbeam charge turn resolves");

        assert_eq!(
            outcome.state.player_charging_move,
            Some("SOLARBEAM".to_string())
        );
        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert!(outcome.events.contains(&BattleEvent::ChargeStarted {
            side: BattleSide::Player,
            move_name: "SOLARBEAM".to_string(),
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SOLARBEAM"
        )));
    }

    #[test]
    fn charged_move_second_turn_forces_stored_move_without_extra_pp() {
        let mut player = pokemon("MEGANIUM", 80, pokemon_type("GRASS"), "SOLARBEAM");
        player.moves[0].current_pp = 4;
        player.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 5,
            pp_ups: 0,
        });
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let mut state = battle_state(player, enemy, 0);
        state.player_charging_move = Some("SOLARBEAM".to_string());
        let moves = BTreeMap::from([
            (
                "SOLARBEAM".to_string(),
                move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 120, 100, "SOLARBEAM"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(36);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 1 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("solarbeam release turn resolves");

        assert_eq!(outcome.state.player_charging_move, None);
        assert_eq!(outcome.state.player.moves[0].current_pp, 4);
        assert_eq!(outcome.state.player.moves[1].current_pp, 5);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::ChargeForcedMove {
            side: BattleSide::Player,
            requested_slot: 1,
            requested_move: "TACKLE".to_string(),
            charged_slot: 0,
            charged_move: "SOLARBEAM".to_string(),
        }));
        assert!(outcome.events.contains(&BattleEvent::ChargeEnded {
            side: BattleSide::Player,
            move_name: "SOLARBEAM".to_string(),
        }));
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                ..
            } if move_name == "SOLARBEAM"
        )));
    }

    #[test]
    fn solarbeam_attacks_immediately_in_sun() {
        let player = pokemon("MEGANIUM", 80, pokemon_type("GRASS"), "SOLARBEAM");
        let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let mut state = battle_state(player, enemy, 0);
        state.weather = Weather::Sun;
        let moves = BTreeMap::from([(
            "SOLARBEAM".to_string(),
            move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 120, 100, "SOLARBEAM"),
        )]);
        let mut rng = Random::new(37);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("sunny solarbeam turn resolves");

        assert_eq!(outcome.state.player_charging_move, None);
        assert!(outcome.state.enemy.hp < enemy_hp);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::ChargeStarted { .. }))
        );
    }

    #[test]
    fn hyper_beam_starts_recharge_after_surviving_damage() {
        let player = pokemon("DRAGONITE", 90, pokemon_type("NORMAL"), "HYPER_BEAM");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "HYPER_BEAM".to_string(),
                move_data_with_effect("HYPER_BEAM", pokemon_type("NORMAL"), 40, 100, "HYPER_BEAM"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(29);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("hyper beam turn resolves");

        assert_eq!(
            outcome.state.player_recharge_move,
            Some("HYPER_BEAM".to_string())
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RechargeStarted {
                side: BattleSide::Player,
                move_name,
            } if move_name == "HYPER_BEAM"
        )));
    }

    #[test]
    fn hyper_beam_recharge_turn_clears_state_without_spending_pp() {
        let mut player = pokemon("DRAGONITE", 90, pokemon_type("NORMAL"), "TACKLE");
        player.moves[0].current_pp = 3;
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_recharge_move = Some("HYPER_BEAM".to_string());
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(30);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("recharge turn resolves");

        assert_eq!(outcome.state.player_recharge_move, None);
        assert_eq!(outcome.state.player.moves[0].current_pp, 3);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RechargeTurn {
                side: BattleSide::Player,
                move_name,
            } if move_name == "HYPER_BEAM"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::MoveUsed {
                side: BattleSide::Player,
                move_name,
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn hyper_beam_does_not_start_recharge_after_ko() {
        let player = pokemon("DRAGONITE", 90, pokemon_type("NORMAL"), "HYPER_BEAM");
        let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        enemy.hp = 1;
        let moves = BTreeMap::from([
            (
                "HYPER_BEAM".to_string(),
                move_data_with_effect("HYPER_BEAM", pokemon_type("NORMAL"), 150, 100, "HYPER_BEAM"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(31);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("hyper beam ko turn resolves");

        assert_eq!(outcome.state.player_recharge_move, None);
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RechargeStarted {
                side: BattleSide::Player,
                ..
            }
        )));
    }

    #[test]
    fn mean_look_fails_when_target_is_already_escape_trapped() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "MEAN_LOOK");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_escape_trap = Some(BattleEscapeTrapState {
            source: BattleSide::Player,
            move_name: "SPIDER_WEB".to_string(),
        });
        let moves = BTreeMap::from([
            (
                "MEAN_LOOK".to_string(),
                move_data_with_effect("MEAN_LOOK", pokemon_type("NORMAL"), 0, 100, "MEAN_LOOK"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(18);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("duplicate mean look turn resolves");

        assert_eq!(
            outcome.state.enemy_escape_trap,
            Some(BattleEscapeTrapState {
                source: BattleSide::Player,
                move_name: "SPIDER_WEB".to_string(),
            })
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::EscapeTrapFailed {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
            } if move_name == "MEAN_LOOK"
        )));
    }

    #[test]
    fn mean_look_blocks_switching_for_trapped_side() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "TACKLE");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_escape_trap = Some(BattleEscapeTrapState {
            source: BattleSide::Player,
            move_name: "MEAN_LOOK".to_string(),
        });
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(19);

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("trapped switch turn resolves");

        assert_eq!(
            outcome.state.enemy_escape_trap,
            Some(BattleEscapeTrapState {
                source: BattleSide::Player,
                move_name: "MEAN_LOOK".to_string(),
            })
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::SwitchBlocked {
                side: BattleSide::Enemy,
                party_index: 1,
                source: BattleSide::Player,
                move_name,
            } if move_name == "MEAN_LOOK"
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Switched {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }

    #[test]
    fn source_switch_clears_escape_trap_it_created() {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "TACKLE");
        let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.enemy_escape_trap = Some(BattleEscapeTrapState {
            source: BattleSide::Player,
            move_name: "MEAN_LOOK".to_string(),
        });
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(20);

        let outcome = resolve_battle_turn(
            state,
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
        .expect("source switch turn resolves");

        assert_eq!(outcome.state.enemy_escape_trap, None);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Switched {
                side: BattleSide::Player,
                party_index: 1,
            }
        )));
    }

    #[test]
    fn mean_look_blocks_wild_run_and_enemy_still_acts() {
        let player = pokemon("RATTATA", 999, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("GASTLY", 10, pokemon_type("GHOST"), "TACKLE");
        let mut state = battle_state(player, enemy, 0);
        state.player_escape_trap = Some(BattleEscapeTrapState {
            source: BattleSide::Enemy,
            move_name: "MEAN_LOOK".to_string(),
        });
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(21);

        let outcome = resolve_wild_battle_turn_with_items(
            state,
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            2,
            &mut rng,
        )
        .expect("blocked wild run still resolves enemy action");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::RunBlocked {
                side: BattleSide::Player,
                source: BattleSide::Enemy,
                move_name,
            } if move_name == "MEAN_LOOK"
        )));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::RunAttempt { .. }))
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::MoveSelected {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == "TACKLE"
        )));
    }

    #[test]
    fn force_switch_move_ends_wild_battle() {
        let player = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "WHIRLWIND");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "WHIRLWIND".to_string(),
                move_data_with_effect("WHIRLWIND", pokemon_type("NORMAL"), 0, 100, "FORCE_SWITCH"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(31);

        let outcome = resolve_wild_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            0,
            &mut rng,
        )
        .expect("wild force switch resolves");

        assert!(outcome.events.contains(&BattleEvent::ForceSwitchApplied {
            side: BattleSide::Player,
            move_name: "WHIRLWIND".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(outcome.events.contains(&BattleEvent::Fled {
            side: BattleSide::Enemy,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Damage { .. }))
        );
    }

    #[test]
    fn force_switch_move_fails_without_trainer_replacement_context() {
        let player = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "WHIRLWIND");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "WHIRLWIND".to_string(),
                move_data_with_effect("WHIRLWIND", pokemon_type("NORMAL"), 0, 100, "FORCE_SWITCH"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(32);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("generic force switch resolves as failed");

        assert!(outcome.events.contains(&BattleEvent::ForceSwitchFailed {
            side: BattleSide::Player,
            move_name: "WHIRLWIND".to_string(),
            target: BattleSide::Enemy,
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Fled { .. }))
        );
    }

    #[test]
    fn teleport_ends_wild_battle_for_user() {
        let player = pokemon("ABRA", 90, pokemon_type("PSYCHIC_TYPE"), "TELEPORT");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "TELEPORT".to_string(),
                move_data_with_effect("TELEPORT", pokemon_type("PSYCHIC_TYPE"), 0, 100, "TELEPORT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(33);

        let outcome = resolve_wild_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            0,
            &mut rng,
        )
        .expect("wild teleport resolves");

        assert!(outcome.events.contains(&BattleEvent::Fled {
            side: BattleSide::Player,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::MoveSelected {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }

    #[test]
    fn teleport_fails_outside_wild_escape_context() {
        let player = pokemon("ABRA", 90, pokemon_type("PSYCHIC_TYPE"), "TELEPORT");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "TELEPORT".to_string(),
                move_data_with_effect("TELEPORT", pokemon_type("PSYCHIC_TYPE"), 0, 100, "TELEPORT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(34);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("generic teleport resolves as failed");

        assert!(outcome.events.contains(&BattleEvent::TeleportFailed {
            side: BattleSide::Player,
            move_name: "TELEPORT".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Fled { .. }))
        );
    }

    #[test]
    fn teleport_is_blocked_by_escape_trap() {
        let player = pokemon("ABRA", 90, pokemon_type("PSYCHIC_TYPE"), "TELEPORT");
        let enemy = pokemon("GASTLY", 30, pokemon_type("GHOST"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "TELEPORT".to_string(),
                move_data_with_effect("TELEPORT", pokemon_type("PSYCHIC_TYPE"), 0, 100, "TELEPORT"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(35);
        let mut state = battle_state(player, enemy, rng.seed());
        state.player_escape_trap = Some(BattleEscapeTrapState {
            source: BattleSide::Enemy,
            move_name: "MEAN_LOOK".to_string(),
        });

        let outcome = resolve_wild_battle_turn_with_items(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            0,
            &mut rng,
        )
        .expect("trapped teleport resolves as blocked");

        assert!(outcome.events.contains(&BattleEvent::RunBlocked {
            side: BattleSide::Player,
            source: BattleSide::Enemy,
            move_name: "MEAN_LOOK".to_string(),
        }));
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| matches!(event, BattleEvent::Fled { .. }))
        );
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::MoveSelected {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }

    #[test]
    fn spikes_applies_to_target_side_once() {
        let player = pokemon("PINECO", 90, pokemon_type("BUG"), "SPIKES");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SPIKES".to_string(),
                move_data_with_effect("SPIKES", pokemon_type("GROUND"), 0, 100, "SPIKES"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(33);

        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
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
        .expect("spikes resolves");

        assert!(outcome.state.enemy_spikes);
        assert!(outcome.events.contains(&BattleEvent::SpikesApplied {
            side: BattleSide::Player,
            move_name: "SPIKES".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn spikes_fails_when_target_side_already_has_spikes() {
        let player = pokemon("PINECO", 90, pokemon_type("BUG"), "SPIKES");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let moves = BTreeMap::from([
            (
                "SPIKES".to_string(),
                move_data_with_effect("SPIKES", pokemon_type("GROUND"), 0, 100, "SPIKES"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
        ]);
        let mut rng = Random::new(34);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_spikes = true;

        let outcome = resolve_battle_turn(
            state,
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
        .expect("duplicate spikes resolves");

        assert!(outcome.state.enemy_spikes);
        assert!(outcome.events.contains(&BattleEvent::SpikesFailed {
            side: BattleSide::Player,
            move_name: "SPIKES".to_string(),
            target: BattleSide::Enemy,
        }));
    }

    #[test]
    fn switching_into_spikes_takes_one_eighth_max_hp_damage() {
        let player = pokemon("PINECO", 90, pokemon_type("BUG"), "TACKLE");
        let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(35);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_spikes = true;

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("spikes switch resolves");

        let damage = (enemy_hp / 8).max(1);
        assert_eq!(outcome.state.enemy.hp, enemy_hp - damage);
        assert!(outcome.events.contains(&BattleEvent::SpikesDamage {
            side: BattleSide::Enemy,
            damage,
            hp_before: enemy_hp,
            hp_after: enemy_hp - damage,
        }));
    }

    #[test]
    fn flying_pokemon_switches_into_spikes_without_damage() {
        let player = pokemon("PINECO", 90, pokemon_type("BUG"), "TACKLE");
        let enemy = pokemon("PIDGEY", 30, pokemon_type("FLYING"), "TACKLE");
        let enemy_hp = enemy.hp;
        let moves = BTreeMap::from([(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]);
        let mut rng = Random::new(36);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_spikes = true;

        let outcome = resolve_battle_turn(
            state,
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Switch { party_index: 1 },
            },
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("flying spikes switch resolves");

        assert_eq!(outcome.state.enemy.hp, enemy_hp);
        assert!(outcome.events.contains(&BattleEvent::SpikesImmune {
            side: BattleSide::Enemy,
        }));
    }

    #[test]
    fn wild_enemy_run_action_ends_battle_as_fled_event() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(1);

        let outcome = resolve_wild_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Run,
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            0,
            &mut rng,
        )
        .expect("wild enemy run resolves as a flee event");
        assert_eq!(outcome.order, vec![BattleSide::Enemy]);
        assert_eq!(outcome.state.turn, 1);
        assert_eq!(
            outcome.events,
            vec![BattleEvent::Fled {
                side: BattleSide::Enemy
            }]
        );
    }

    #[test]
    fn generic_battle_turn_player_run_records_fled_event() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(7);

        let outcome = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Run,
                enemy: BattleAction::Move { slot: 0 },
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("generic battle run is a deterministic flee action");

        assert_eq!(outcome.order.first(), Some(&BattleSide::Player));
        assert!(outcome.events.contains(&BattleEvent::Fled {
            side: BattleSide::Player
        }));
    }

    #[test]
    fn generic_battle_turn_enemy_run_records_fled_event() {
        let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
        let moves = [(
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        )]
        .into_iter()
        .collect();
        let mut rng = Random::new(7);

        let outcome = resolve_battle_turn_with_items(
            battle_state(player, enemy, rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Run,
            },
            &moves,
            &BTreeMap::new(),
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("generic enemy battle run is a deterministic flee action");

        assert_eq!(outcome.order.first(), Some(&BattleSide::Enemy));
        assert!(outcome.events.contains(&BattleEvent::Fled {
            side: BattleSide::Enemy
        }));
    }

    #[test]
    fn core_wild_battle_run_uses_exported_escape_rules() {
        let player = pokemon("RATTATA", 999, pokemon_type("NORMAL"), "TACKLE");
        let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
        let state = battle_state(player, enemy, 99);
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
        let state = battle_state(player, enemy, 1);
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
            &battle_state(player.clone(), enemy.clone(), rng.seed()),
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
            &battle_state(malformed_player, enemy, rng.seed()),
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
            battle_state(player, enemy, rng.seed()),
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
            &battle_state(player, enemy, rng.seed()),
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
