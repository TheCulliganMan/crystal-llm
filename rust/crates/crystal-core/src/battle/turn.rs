use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::battle::damage::{
    DamageCalculationError, DamageContext, DamageResult, TypeCategories, TypeEffectivenessTable,
    TypeMultiplier, Weather, WeatherModifiers, apply_metal_powder_damage_stats, calculate_damage,
    calculate_type_effectiveness_multiplier_with_foresight, is_physical_type,
    truncate_damage_stats,
};
use crate::battle::start::{
    ActiveBattleEnemyError, ActiveBattlePartyError, deactivate_battle_after_draw,
    require_active_battle_enemy_party_index, require_active_battle_party_index,
    update_active_battle_enemy,
};
use crate::battle::stats::{BattleStatMultiplierTables, accuracy_stage_multiplier, apply_stage};
use crate::models::pokemon::default_stat_boosts;
use crate::models::{Dv, Item, LearnedMove, Move, Pokemon, PokemonSpecies, PokemonType, Stat};
use crate::random::Random;
use crate::state::{BattleMemory, GameState, LINK_MODE_COLOSSEUM, LinkSerialConnectionStatus};
use crate::systems::battle_escape::{
    BattleEscapeAttempt, BattleEscapeError, BattleEscapeRules, attempt_wild_battle_escape,
};
use crate::systems::battle_items::{BattleItemOutcome, apply_active_battle_item_effect};
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Crystal processes paired between-turn effects enemy-first only on the
/// Game Boy using the external serial clock. Internal-clock and non-link
/// execution both retain the ordinary player-first order.
pub const fn between_turn_side_order(
    serial_connection_status: LinkSerialConnectionStatus,
) -> [BattleSide; 2] {
    match serial_connection_status {
        LinkSerialConnectionStatus::UsingExternalClock => [BattleSide::Enemy, BattleSide::Player],
        LinkSerialConnectionStatus::NotEstablished
        | LinkSerialConnectionStatus::UsingInternalClock => [BattleSide::Player, BattleSide::Enemy],
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
    #[serde(default = "default_battle_time_of_day")]
    pub time_of_day: TimeOfDay,
    #[serde(default)]
    pub link_battle: bool,
    #[serde(default)]
    pub link_colosseum: bool,
    #[serde(default)]
    pub serial_connection_status: LinkSerialConnectionStatus,
    pub player_mist_active: bool,
    pub enemy_mist_active: bool,
    pub player_safeguard_turns: u8,
    pub enemy_safeguard_turns: u8,
    pub player_reflect_turns: u8,
    pub enemy_reflect_turns: u8,
    pub player_light_screen_turns: u8,
    pub enemy_light_screen_turns: u8,
    pub player_minimized: bool,
    pub enemy_minimized: bool,
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
    #[serde(default)]
    pub player_x_accuracy: bool,
    #[serde(default)]
    pub enemy_x_accuracy: bool,
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
    /// Crystal's four-entry wPlayerUsedMoves history for the currently active
    /// player Pokemon. Trainer switch AI scores every known move, not merely
    /// the most recent one.
    #[serde(default)]
    pub player_used_moves: Vec<String>,
    #[serde(default)]
    pub player_turns_taken: u8,
    #[serde(default)]
    pub enemy_turns_taken: u8,
    #[serde(default)]
    pub force_switch_blocked: bool,
    /// SleepTarget uses a narrower rejection mask in Battle Tower battles.
    #[serde(default = "default_sleep_turn_mask")]
    pub sleep_turn_mask: u8,
    #[serde(default)]
    pub enemy_effect_ai_random_fail: bool,
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
    #[serde(default)]
    pub player_rage_counter: u8,
    #[serde(default)]
    pub enemy_rage_counter: u8,
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
    /// One-shot trainer consumables already used in this battle. This belongs
    /// to the active combat record rather than script variables: it is battle
    /// state, must survive a native save/reload, and must disappear exactly
    /// when the battle ends.
    #[serde(default)]
    pub trainer_items_used: BTreeSet<String>,
    #[serde(default)]
    pub obedience_trainer_id: Option<u16>,
    #[serde(default)]
    pub obedience_badges: [bool; 8],
    #[serde(default)]
    pub kanto_badges: [bool; 8],
    #[serde(default)]
    pub badge_boosts_enabled: bool,
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
            time_of_day: default_battle_time_of_day(),
            link_battle: false,
            link_colosseum: false,
            serial_connection_status: LinkSerialConnectionStatus::NotEstablished,
            player_mist_active: false,
            enemy_mist_active: false,
            player_safeguard_turns: 0,
            enemy_safeguard_turns: 0,
            player_reflect_turns: 0,
            enemy_reflect_turns: 0,
            player_light_screen_turns: 0,
            enemy_light_screen_turns: 0,
            player_minimized: false,
            enemy_minimized: false,
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
            player_x_accuracy: false,
            enemy_x_accuracy: false,
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
            player_used_moves: Vec::new(),
            player_turns_taken: 0,
            enemy_turns_taken: 0,
            force_switch_blocked: false,
            sleep_turn_mask: default_sleep_turn_mask(),
            enemy_effect_ai_random_fail: false,
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
            player_rage_counter: 0,
            enemy_rage_counter: 0,
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
            trainer_items_used: BTreeSet::new(),
            obedience_trainer_id: None,
            obedience_badges: [false; 8],
            kanto_badges: [false; 8],
            badge_boosts_enabled: false,
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

    pub fn with_obedience(mut self, trainer_id: u16, johto_badges: [bool; 8]) -> Self {
        self.obedience_trainer_id = Some(trainer_id);
        self.obedience_badges = johto_badges;
        self
    }

    pub fn with_kanto_badges(mut self, kanto_badges: [bool; 8]) -> Self {
        self.kanto_badges = kanto_badges;
        self
    }

    pub fn with_badge_boosts_enabled(mut self, enabled: bool) -> Self {
        self.badge_boosts_enabled = enabled;
        self
    }

    pub fn with_link_context(
        mut self,
        time_of_day: TimeOfDay,
        link_mode: u8,
        serial_connection_status: LinkSerialConnectionStatus,
    ) -> Self {
        self.time_of_day = time_of_day;
        self.link_battle = link_mode != 0;
        self.link_colosseum = link_mode == LINK_MODE_COLOSSEUM;
        self.serial_connection_status = serial_connection_status;
        self
    }
}

const fn default_sleep_turn_mask() -> u8 {
    0x07
}

const fn default_battle_time_of_day() -> TimeOfDay {
    TimeOfDay::Day
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
    AutomaticStruggle {
        side: BattleSide,
    },
    MoveSelected {
        side: BattleSide,
        slot: usize,
        move_name: String,
    },
    Disobeyed {
        side: BattleSide,
    },
    DisobedienceIdle {
        side: BattleSide,
        roll: u8,
    },
    DisobedienceIgnoredSleeping {
        side: BattleSide,
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
        parameter: u8,
    },
    BeatUpParticipant {
        side: BattleSide,
        move_name: String,
        party_index: usize,
        species: String,
        nickname: String,
        shiny: bool,
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
    HealBellChimed {
        side: BattleSide,
        active_status_before: Option<String>,
    },
    HeldItemStatusHealed {
        side: BattleSide,
        item_id: String,
        held_effect: String,
        status_before: Option<String>,
        confusion_turns_before: u16,
    },
    HeldItemHpHealed {
        side: BattleSide,
        item_id: String,
        hp_before: u16,
        hp_after: u16,
        amount: u16,
    },
    HeldItemPpRestored {
        side: BattleSide,
        item_id: String,
        move_name: String,
        slot: usize,
        pp_before: u8,
        pp_after: u8,
        amount: u8,
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
    BideUnleashed {
        side: BattleSide,
        move_name: String,
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
    RageBuilding {
        side: BattleSide,
        counter: u8,
    },
    StatStageUnchanged {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
        amount: i8,
        stage: i8,
    },
    StatStageFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
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
    },
    MistFailed {
        side: BattleSide,
        move_name: String,
    },
    MistProtected {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        stat: Stat,
        amount: i8,
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
        trap_move: Option<String>,
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
    FutureSightLanded {
        side: BattleSide,
        source: BattleSide,
        move_name: String,
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
    SubstituteBlocked {
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
        held_item: Option<String>,
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
        turns: u16,
    },
    ConfusionFailed {
        side: BattleSide,
        move_name: String,
        target: BattleSide,
        turns_remaining: u16,
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
        turns_remaining: u16,
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
        animation_param: u8,
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
        confusion_turns: u16,
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
    RunPrevented {
        side: BattleSide,
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
    #[error("active battle combat state requires an active battle")]
    InactiveBattle,
    #[error("active battle combat state party error: {0:?}")]
    ActiveParty(#[from] ActiveBattlePartyError),
    #[error("active battle combat state enemy error: {0:?}")]
    ActiveEnemyIndex(ActiveBattleEnemyError),
    #[error("battle turn active party index {index} is outside the party")]
    PartyIndexOutOfRange { index: usize },
    #[error("battle turn active party index {index} has no Pokemon")]
    EmptyPartySlot { index: usize },
    #[error("battle turn active enemy update failed: {0:?}")]
    ActiveEnemy(#[from] ActiveBattleEnemyError),
}

pub fn active_battle_combat_state(
    state: &GameState,
) -> Result<BattleCombatState, BattleTurnCommitError> {
    let link_battle = state.link_session.link_mode != 0;
    let badge_boosts_enabled = !link_battle
        && !matches!(
            &state.battle,
            BattleMemory::Trainer { battle_type, .. }
                if battle_type == "BATTLETYPE_BATTLE_TOWER"
        );
    if let Some(mut combat) = state.script_runtime.active_battle_combat.clone() {
        combat.link_battle = link_battle;
        combat.link_colosseum = state.link_session.link_mode == LINK_MODE_COLOSSEUM;
        combat.serial_connection_status = state.link_session.serial_connection_status;
        combat.obedience_trainer_id = Some(state.player_id);
        combat.obedience_badges = state.badges.johto;
        combat.kanto_badges = state.badges.kanto;
        combat.badge_boosts_enabled = badge_boosts_enabled;
        return Ok(combat);
    }
    let active_party_index = require_active_battle_party_index(state)?;
    let active_enemy_index = require_active_battle_enemy_party_index(state)
        .map_err(BattleTurnCommitError::ActiveEnemyIndex)?;
    let player = state.storage.party.pokemon[active_party_index]
        .clone()
        .ok_or(BattleTurnCommitError::EmptyPartySlot {
            index: active_party_index,
        })?;
    let player_party = state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let (enemy, enemy_party) = match &state.battle {
        BattleMemory::Wild {
            enemy_pokemon,
            enemy_party,
            ..
        }
        | BattleMemory::StaticWild {
            enemy_pokemon,
            enemy_party,
            ..
        }
        | BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } => (enemy_pokemon.clone(), enemy_party.clone()),
        BattleMemory::Inactive => return Err(BattleTurnCommitError::InactiveBattle),
    };
    validate_enemy_party_snapshot(state, &enemy_party, active_enemy_index)?;
    let (force_switch_blocked, sleep_turn_mask) = match &state.battle {
        BattleMemory::Wild { battle_type, .. } | BattleMemory::StaticWild { battle_type, .. } => (
            matches!(
                battle_type.as_str(),
                "BATTLETYPE_FORCESHINY"
                    | "BATTLETYPE_TRAP"
                    | "BATTLETYPE_CELEBI"
                    | "BATTLETYPE_SUICUNE"
            ),
            0x07,
        ),
        BattleMemory::Trainer { battle_type, .. } => (
            false,
            if battle_type == "BATTLETYPE_BATTLE_TOWER" {
                0x03
            } else {
                0x07
            },
        ),
        BattleMemory::Inactive => return Err(BattleTurnCommitError::InactiveBattle),
    };
    let mut combat = BattleCombatState::new(player, enemy, state.rng_seed)
        .with_parties(player_party, enemy_party)
        .with_party_indices(active_party_index, active_enemy_index)
        .with_obedience(state.player_id, state.badges.johto)
        .with_kanto_badges(state.badges.kanto)
        .with_link_context(
            state.time.time_of_day,
            state.link_session.link_mode,
            state.link_session.serial_connection_status,
        )
        .with_badge_boosts_enabled(badge_boosts_enabled);
    combat.force_switch_blocked = force_switch_blocked;
    combat.sleep_turn_mask = sleep_turn_mask;
    combat.enemy_effect_ai_random_fail =
        state.link_session.link_mode == 0 && sleep_turn_mask != 0x03;
    Ok(combat)
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
    if outcome.state.player.hp == 0
        && outcome.events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Fainted {
                    side: BattleSide::Player
                }
            )
        })
    {
        let fainted = slot
            .as_mut()
            .expect("validated active battle party slot became empty");
        // PlayerFaint explicitly clears wBattleMonStatus before
        // UpdateBattleMonInParty copies HP/status back to the persistent slot.
        fainted.status = None;
        fainted.sleep_turns = 0;
        apply_battle_faint_happiness(fainted, outcome.state.enemy.level);
    }
    if commit_party_index != active_party_index {
        state.battle_active_party_index = Some(commit_party_index);
    }
    if battle_outcome_used_player_heal_bell(outcome) {
        apply_party_heal_bell_commit(state, commit_party_index);
    }
    state.rng_seed = outcome.state.rng_seed_after;
    state.sync_party_from_storage();
    crate::battle::start::activate_amulet_coin_for_active_party(state);
    commit_enemy_party_snapshot(state, &outcome.state.enemy_party)?;
    state.battle_active_enemy_party_index = Some(outcome.state.enemy_party_index);
    update_active_battle_enemy(state, outcome.state.enemy.clone())?;
    commit_pay_day_money(state, outcome);
    let mut committed_combat = outcome.state.clone();
    let committed_player = state.storage.party.pokemon[commit_party_index]
        .as_ref()
        .expect("validated active battle party slot became empty")
        .clone();
    committed_combat.player = committed_player.clone();
    if let Some(party_slot) = committed_combat.player_party.get_mut(commit_party_index) {
        *party_slot = committed_player;
    }
    state.script_runtime.active_battle_combat = Some(committed_combat);
    if outcome
        .events
        .iter()
        .any(|event| matches!(event, BattleEvent::Fled { .. }))
    {
        deactivate_battle_after_draw(state);
    }
    Ok(())
}

fn apply_battle_faint_happiness(pokemon: &mut Pokemon, enemy_level: u8) {
    let much_stronger = u16::from(enemy_level) >= u16::from(pokemon.level) + 30;
    let reduction = if much_stronger {
        if pokemon.happiness < 200 { 5 } else { 10 }
    } else {
        1
    };
    pokemon.happiness = pokemon.happiness.saturating_sub(reduction);
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
    pokemon.stat_boosts = default_stat_boosts();
}

fn battle_outcome_used_player_heal_bell(outcome: &BattleTurnOutcome) -> bool {
    outcome.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::HealBellChimed {
                side: BattleSide::Player,
                ..
            }
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
        deactivate_battle_after_draw(state);
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
    state: BattleCombatState,
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
    for side in between_turn_side_order(state.serial_connection_status) {
        apply_berserk_gene_start_of_turn(&mut state, side, items, &mut events)?;
    }
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
        if !battle_continues_after_actions(&state, &events)
            || state.pokemon(side).hp == 0
            || state.pokemon(side.other()).hp == 0
        {
            continue;
        }
        let newly_immobilized = events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::StatusApplied {
                    side: source,
                    target,
                    status,
                    ..
                } if *source == side.other()
                    && *target == side
                    && matches!(status.as_str(), "SLEEP" | "FREEZE")
            )
        }) && matches!(
            state.pokemon(side).status.as_deref(),
            Some("SLEEP" | "FREEZE")
        );
        if newly_immobilized {
            set_destiny_bond_active(&mut state, side, false);
            end_opponent_action_volatiles(&mut state, side);
            acted_before.push(side);
            if battle_continues_after_actions(&state, &events)
                && state.pokemon(side).hp > 0
                && state.pokemon(side.other()).hp > 0
            {
                apply_post_action_residual_damage(&mut state, side, &mut events);
            }
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
        end_opponent_action_volatiles(&mut state, side);
        acted_before.push(side);
        if battle_continues_after_actions(&state, &events)
            && state.pokemon(side).hp > 0
            && state.pokemon(side.other()).hp > 0
        {
            apply_post_action_residual_damage(&mut state, side, &mut events);
        }
    }

    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_effects(&mut state, moves, items, rng, &mut events)?;
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
    set_destiny_bond_active(&mut state, BattleSide::Player, false);
    // A wild-battle run command passes through ParsePlayerAction's non-move
    // path before TryToRun, so even a blocked or failed attempt clears these
    // committed/chained move states.
    reset_bide_state(&mut state, BattleSide::Player);
    reset_fury_cutter_chain(&mut state, BattleSide::Player);
    reset_protect_counter(&mut state, BattleSide::Player);
    set_rage_active(&mut state, BattleSide::Player, false);
    let escape = if state.force_switch_blocked {
        events.push(BattleEvent::RunPrevented {
            side: BattleSide::Player,
        });
        None
    } else if let Some(trap) = escape_trap_state(&state, BattleSide::Player).cloned() {
        events.push(BattleEvent::RunBlocked {
            side: BattleSide::Player,
            source: trap.source,
            move_name: trap.move_name,
        });
        None
    } else if let Some(trap) = trap_state(&state, BattleSide::Player).cloned() {
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
    end_opponent_action_volatiles(&mut state, BattleSide::Player);
    let order = if escape.as_ref().is_some_and(|escape| escape.escaped) {
        vec![BattleSide::Player]
    } else {
        if state.player.hp > 0 && state.enemy.hp > 0 {
            apply_post_action_residual_damage(&mut state, BattleSide::Player, &mut events);
        }
        if battle_continues_after_actions(&state, &events)
            && state.player.hp > 0
            && state.enemy.hp > 0
        {
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
            end_opponent_action_volatiles(&mut state, BattleSide::Enemy);
            if battle_continues_after_actions(&state, &events)
                && state.player.hp > 0
                && state.enemy.hp > 0
            {
                apply_post_action_residual_damage(&mut state, BattleSide::Enemy, &mut events);
            }
        }
        vec![BattleSide::Player, BattleSide::Enemy]
    };

    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_effects(&mut state, moves, items, rng, &mut events)?;
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
    force_switch_ends_battle: bool,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
) -> Result<BattleTurnOutcome, BattleTurnError> {
    let mut events = Vec::new();
    let acted_before = [BattleSide::Player];
    validate_active_battle_side_is_not_fainted(&state, BattleSide::Player)?;
    validate_active_battle_side_can_act(&state, BattleSide::Enemy, &enemy_action, false)?;
    clear_turn_last_damage(&mut state);
    set_destiny_bond_active(&mut state, BattleSide::Player, false);
    end_opponent_action_volatiles(&mut state, BattleSide::Player);
    if state.player.hp > 0 && state.enemy.hp > 0 {
        apply_post_action_residual_damage(&mut state, BattleSide::Player, &mut events);
    }
    if battle_continues_after_actions(&state, &events) && state.player.hp > 0 && state.enemy.hp > 0
    {
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
            &acted_before,
            force_switch_ends_battle,
            &mut events,
        )?;
        end_opponent_action_volatiles(&mut state, BattleSide::Enemy);
        if battle_continues_after_actions(&state, &events)
            && state.player.hp > 0
            && state.enemy.hp > 0
        {
            apply_post_action_residual_damage(&mut state, BattleSide::Enemy, &mut events);
        }
    }
    if battle_continues_after_actions(&state, &events) {
        apply_end_turn_effects(&mut state, moves, items, rng, &mut events)?;
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
    if state.link_battle && matches!(input.enemy, BattleAction::Switch { .. }) {
        if matches!(input.player, BattleAction::Switch { .. }) {
            let first_roll_wins = rng.battle_random_byte() < 128;
            let internal_clock = matches!(
                state.serial_connection_status,
                LinkSerialConnectionStatus::UsingInternalClock
            );
            let player_first = if internal_clock {
                !first_roll_wins
            } else {
                first_roll_wins
            };
            return Ok(if player_first {
                vec![BattleSide::Player, BattleSide::Enemy]
            } else {
                vec![BattleSide::Enemy, BattleSide::Player]
            });
        }
        return Ok(vec![BattleSide::Enemy, BattleSide::Player]);
    }
    if !matches!(
        input.player,
        BattleAction::Move { .. } | BattleAction::MoveSwitch { .. }
    ) {
        // Player menu actions normally execute before the enemy move, but
        // PursuitSwitch explicitly runs the enemy's selected Pursuit against
        // the departing player battler before RecallPlayerMon.
        return Ok(if enemy_priority == 11 {
            vec![BattleSide::Enemy, BattleSide::Player]
        } else {
            vec![BattleSide::Player, BattleSide::Enemy]
        });
    }
    if player_priority != enemy_priority {
        return Ok(if player_priority > enemy_priority {
            vec![BattleSide::Player, BattleSide::Enemy]
        } else {
            vec![BattleSide::Enemy, BattleSide::Player]
        });
    }

    let player_quick_claw = quick_claw_parameter(state, BattleSide::Player, items)?;
    let enemy_quick_claw = quick_claw_parameter(state, BattleSide::Enemy, items)?;
    match (player_quick_claw, enemy_quick_claw) {
        (Some(player_parameter), None) => {
            if rng.battle_random_byte() < player_parameter {
                return Ok(vec![BattleSide::Player, BattleSide::Enemy]);
            }
        }
        (None, Some(enemy_parameter)) => {
            if rng.battle_random_byte() < enemy_parameter {
                return Ok(vec![BattleSide::Enemy, BattleSide::Player]);
            }
        }
        (Some(player_parameter), Some(enemy_parameter)) => {
            // Crystal samples the external-clock side first. Ordinary local
            // battles share the external-clock branch, while an internal-
            // clock link player samples itself first.
            let player_first_sample = matches!(
                state.serial_connection_status,
                LinkSerialConnectionStatus::UsingInternalClock
            );
            if player_first_sample {
                if rng.battle_random_byte() < player_parameter {
                    return Ok(vec![BattleSide::Player, BattleSide::Enemy]);
                }
                if rng.battle_random_byte() < enemy_parameter {
                    return Ok(vec![BattleSide::Enemy, BattleSide::Player]);
                }
            } else {
                if rng.battle_random_byte() < enemy_parameter {
                    return Ok(vec![BattleSide::Enemy, BattleSide::Player]);
                }
                if rng.battle_random_byte() < player_parameter {
                    return Ok(vec![BattleSide::Player, BattleSide::Enemy]);
                }
            }
        }
        (None, None) => {}
    }

    let player_speed = battle_speed(state, BattleSide::Player, stat_multipliers)?;
    let enemy_speed = battle_speed(state, BattleSide::Enemy, stat_multipliers)?;
    if player_speed != enemy_speed {
        return Ok(if player_speed > enemy_speed {
            vec![BattleSide::Player, BattleSide::Enemy]
        } else {
            vec![BattleSide::Enemy, BattleSide::Player]
        });
    }

    Ok(if rng.battle_random_byte() < 128 {
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
    set_destiny_bond_active(state, side, false);
    if side == BattleSide::Player {
        let selected_slot = match action {
            BattleAction::Move { slot } | BattleAction::MoveSwitch { slot, .. } => Some(*slot),
            _ => None,
        };
        if let Some(selected_slot) = selected_slot {
            match apply_player_obedience(
                state,
                selected_slot,
                moves,
                items,
                stat_multipliers,
                type_categories,
                type_effectiveness,
                weather_modifiers,
                rng,
                events,
            )? {
                ObedienceAction::Obey => {}
                ObedienceAction::Stop => return Ok(()),
                ObedienceAction::UseMove(slot) => {
                    let result = execute_move_slot(
                        state,
                        side,
                        slot,
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
                    );
                    set_last_move(state, side, None);
                    clear_encore_state(state, side);
                    return result;
                }
            }
        }
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
            if let Some(airborne_move) = airborne_move_state(state, side).map(ToOwned::to_owned) {
                if let Some(slot) = state
                    .pokemon(side)
                    .moves
                    .iter()
                    .position(|learned| learned.name == airborne_move)
                {
                    return execute_move_slot(
                        state,
                        side,
                        slot,
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
                    );
                }
                set_airborne_move_state(state, side, None);
                events.push(BattleEvent::AirborneEnded {
                    side,
                    move_name: airborne_move,
                });
                return Ok(());
            }
            if let Some(trap) = trap_state(state, side) {
                events.push(BattleEvent::SwitchBlocked {
                    side,
                    party_index: *party_index,
                    source: trap.source,
                    move_name: trap.move_name.clone(),
                });
                return Ok(());
            }
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
            Ok(())
        }
        BattleAction::Item { item_id } => execute_item(state, side, item_id, items, events),
        BattleAction::Run => Err(BattleTurnError::RunNotAllowed { side }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObedienceResult {
    Obey,
    UseMove(usize),
    DoNothing(u8),
    IgnoredSleeping,
    Nap,
    Confusion,
}

enum ObedienceAction {
    Obey,
    Stop,
    UseMove(usize),
}

fn swap_nibbles(value: u8) -> u8 {
    value.rotate_left(4)
}

fn apply_player_obedience(
    state: &mut BattleCombatState,
    selected_slot: usize,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<ObedienceAction, BattleTurnError> {
    let result = obedience_result(state, selected_slot, rng);
    if result == ObedienceResult::Obey {
        return Ok(ObedienceAction::Obey);
    }
    match result {
        ObedienceResult::Nap => {
            let sleep_roll = loop {
                let roll = swap_nibbles(rng.battle_random_byte().wrapping_mul(2));
                let turns = roll & 0x07;
                if turns != 0 {
                    break turns;
                }
            };
            state.player.status = Some("SLEEP".to_string());
            state.player.sleep_turns = sleep_roll;
            events.push(BattleEvent::StatusApplied {
                side: BattleSide::Player,
                move_name: "DISOBEDIENCE_NAP".to_string(),
                target: BattleSide::Player,
                status: "SLEEP".to_string(),
            });
        }
        ObedienceResult::Confusion => {
            let selected_move_name = battle_moves(state, BattleSide::Player)
                .get(selected_slot)
                .ok_or(BattleTurnError::MissingMoveSlot {
                    side: BattleSide::Player,
                    slot: selected_slot,
                })?
                .name
                .clone();
            let selected_move =
                moves
                    .get(&selected_move_name)
                    .ok_or_else(|| BattleTurnError::MissingMoveData {
                        side: BattleSide::Player,
                        move_name: selected_move_name.clone(),
                    })?;
            events.push(BattleEvent::Disobeyed {
                side: BattleSide::Player,
            });
            apply_confusion_self_damage(
                state,
                BattleSide::Player,
                "DISOBEDIENCE",
                selected_move,
                items,
                stat_multipliers,
                type_categories,
                type_effectiveness,
                weather_modifiers,
                rng,
                events,
            )?;
            set_last_move(state, BattleSide::Player, None);
            clear_encore_state(state, BattleSide::Player);
            return Ok(ObedienceAction::Stop);
        }
        ObedienceResult::UseMove(slot) => return Ok(ObedienceAction::UseMove(slot)),
        ObedienceResult::DoNothing(roll) => {
            events.push(BattleEvent::DisobedienceIdle {
                side: BattleSide::Player,
                roll,
            });
            set_last_move(state, BattleSide::Player, None);
            clear_encore_state(state, BattleSide::Player);
            return Ok(ObedienceAction::Stop);
        }
        ObedienceResult::IgnoredSleeping => {
            events.push(BattleEvent::DisobedienceIgnoredSleeping {
                side: BattleSide::Player,
            });
            set_last_move(state, BattleSide::Player, None);
            clear_encore_state(state, BattleSide::Player);
            return Ok(ObedienceAction::Stop);
        }
        ObedienceResult::Obey => {}
    }
    events.push(BattleEvent::Disobeyed {
        side: BattleSide::Player,
    });
    set_last_move(state, BattleSide::Player, None);
    clear_encore_state(state, BattleSide::Player);
    Ok(ObedienceAction::Stop)
}

#[cfg(test)]
fn player_disobeys(
    state: &BattleCombatState,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> bool {
    let disobeyed = obedience_result(state, 0, rng) != ObedienceResult::Obey;
    if disobeyed {
        events.push(BattleEvent::Disobeyed {
            side: BattleSide::Player,
        });
    }
    disobeyed
}

fn obedience_result(
    state: &BattleCombatState,
    selected_slot: usize,
    rng: &mut Random,
) -> ObedienceResult {
    let Some(player_id) = state.obedience_trainer_id else {
        return ObedienceResult::Obey;
    };
    let pokemon = &state.player;
    if pokemon.original_trainer_id == player_id {
        return ObedienceResult::Obey;
    }
    let obedience_level: u16 = if state.obedience_badges[7] {
        101
    } else if state.obedience_badges[4] {
        70
    } else if state.obedience_badges[3] {
        50
    } else if state.obedience_badges[2] {
        30
    } else {
        10
    };
    if u16::from(pokemon.level) <= obedience_level {
        return ObedienceResult::Obey;
    }
    let total = obedience_level.saturating_add(u16::from(pokemon.level));
    // `swap a` exchanges the nibbles of the complete random byte; it does not
    // truncate the roll to its high nibble.
    let first = loop {
        let roll = u16::from(swap_nibbles(rng.battle_random_byte()));
        if roll < total {
            break roll;
        }
    };
    if first < obedience_level {
        return ObedienceResult::Obey;
    }
    if battle_moves(state, BattleSide::Player)
        .get(selected_slot)
        .is_some_and(|learned| matches!(learned.name.as_str(), "SNORE" | "SLEEP_TALK"))
        && pokemon.status.as_deref() == Some("SLEEP")
    {
        return ObedienceResult::IgnoredSleeping;
    }
    let second = loop {
        let roll = u16::from(rng.battle_random_byte());
        if roll < total {
            break roll;
        }
    };
    if second < obedience_level {
        let moves = battle_moves(state, BattleSide::Player);
        if moves.len() <= 1 || disable_state(state, BattleSide::Player).is_some() {
            return ObedienceResult::DoNothing(rng.battle_random_byte() & 0x03);
        }
        let candidates = moves
            .iter()
            .enumerate()
            .filter_map(|(slot, learned)| {
                (slot != selected_slot && learned.current_pp > 0).then_some(slot)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return ObedienceResult::DoNothing(rng.battle_random_byte() & 0x03);
        }
        loop {
            let slot = usize::from(rng.battle_random_byte() & 0x03);
            if candidates.contains(&slot) {
                return ObedienceResult::UseMove(slot);
            }
        }
    }
    let levels_over = i16::from(pokemon.level) - obedience_level as i16;
    let nap_roll = i16::from(swap_nibbles(rng.battle_random_byte())) - levels_over;
    if nap_roll < 0 {
        return ObedienceResult::Nap;
    }
    if nap_roll < levels_over {
        ObedienceResult::Confusion
    } else {
        ObedienceResult::DoNothing(rng.battle_random_byte() & 0x03)
    }
}

fn quick_claw_parameter(
    state: &BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
) -> Result<Option<u8>, BattleTurnError> {
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(None);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if item.held_effect != "HELD_QUICK_CLAW" {
        return Ok(None);
    }
    if !(0..=255).contains(&item.parameter) {
        return Err(BattleTurnError::InvalidHeldItemParameter {
            side,
            item_id: item_id.to_string(),
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
        });
    }
    Ok(Some(item.parameter as u8))
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
    let outcome = if item.script_name == "X_ACCURACY" {
        if x_accuracy_active(state, side) {
            return Err(BattleTurnError::BattleItem {
                side,
                item_id: item_id.to_string(),
                error: "X Accuracy is already active".to_string(),
            });
        }
        let pokemon = state.pokemon(side);
        let outcome = BattleItemOutcome {
            item_id: item.script_name.clone(),
            hp_before: pokemon.hp,
            hp_after: pokemon.hp,
            level_before: pokemon.level,
            level_after: pokemon.level,
            experience_before: pokemon.experience,
            experience_after: pokemon.experience,
            status_before: pokemon.status.clone(),
            status_after: pokemon.status.clone(),
            confusion_turns_before: pokemon.confusion_turns,
            confusion_turns_after: pokemon.confusion_turns,
            focus_energy_before: pokemon.focus_energy,
            focus_energy_after: pokemon.focus_energy,
            pp_changes: Vec::new(),
            stat_changes: Vec::new(),
            battle_stat_stage_changes: Vec::new(),
            learned_moves: Vec::new(),
            pending_move_learns: Vec::new(),
            deferred_level_evolution: false,
            evolution_target: None,
            evolution_cancel_snapshot: None,
            consumed: false,
        };
        set_x_accuracy_active(state, side, true);
        outcome
    } else if item.battle_stat_drop_guard == Some(true) {
        let pokemon = state.pokemon(side);
        let outcome = BattleItemOutcome {
            item_id: item.script_name.clone(),
            hp_before: pokemon.hp,
            hp_after: pokemon.hp,
            level_before: pokemon.level,
            level_after: pokemon.level,
            experience_before: pokemon.experience,
            experience_after: pokemon.experience,
            status_before: pokemon.status.clone(),
            status_after: pokemon.status.clone(),
            confusion_turns_before: pokemon.confusion_turns,
            confusion_turns_after: pokemon.confusion_turns,
            focus_energy_before: pokemon.focus_energy,
            focus_energy_after: pokemon.focus_energy,
            pp_changes: Vec::new(),
            stat_changes: Vec::new(),
            battle_stat_stage_changes: Vec::new(),
            learned_moves: Vec::new(),
            pending_move_learns: Vec::new(),
            deferred_level_evolution: false,
            evolution_target: None,
            evolution_cancel_snapshot: None,
            consumed: false,
        };
        match side {
            BattleSide::Player => state.player_mist_active = true,
            BattleSide::Enemy => state.enemy_mist_active = true,
        }
        outcome
    } else {
        apply_active_battle_item_effect(state.pokemon_mut(side), item, false).map_err(|error| {
            BattleTurnError::BattleItem {
                side,
                item_id: item_id.to_string(),
                error: error.to_string(),
            }
        })?
    };
    // ParsePlayerAction clears the player's committed/chained move state
    // before an accepted non-move command. AI_TryItem owns the same reset for
    // an enemy trainer item.
    reset_bide_state(state, side);
    reset_fury_cutter_chain(state, side);
    reset_protect_counter(state, side);
    set_rage_active(state, side, false);
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
    let Some(encored_slot) = battle_moves(state, side)
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
    events.push(BattleEvent::EncoreForcedMove {
        side,
        requested_slot,
        requested_move,
        encored_slot,
        encored_move: encore.move_name.clone(),
        turns_remaining: encore.turns_remaining,
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
    let disabled_move = disable_state(state, side)
        .filter(|disable| disable.turns_remaining > 0)
        .map(|disable| disable.move_name.as_str());
    let no_legal_moves = battle_moves(state, side)
        .iter()
        .all(|learned| learned.current_pp == 0 || disabled_move == Some(learned.name.as_str()));
    let automatic_struggle = no_legal_moves;
    let automatic_turn_state = recharge_move_state(state, side).is_some()
        || airborne_move_state(state, side).is_some()
        || charging_move_state(state, side).is_some()
        || state.pokemon(side).rampage_turns > 0
        || bide_turns(state, side) > 0
        || rollout_turns(state, side) > 0;
    let requested_move_name = if automatic_struggle {
        "STRUGGLE".to_string()
    } else {
        battle_moves(state, side)
            .get(slot)
            .map(|learned| learned.name.clone())
            .ok_or(BattleTurnError::MissingMoveSlot { side, slot })?
    };
    validate_battle_turn_move_name(side, &requested_move_name)?;
    let Some(requested_move_data) = moves.get(&requested_move_name) else {
        return Err(BattleTurnError::MissingMoveData {
            side,
            move_name: requested_move_name,
        });
    };
    if automatic_struggle && side == BattleSide::Player && !automatic_turn_state {
        events.push(BattleEvent::AutomaticStruggle { side });
    }
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

    let forced_move =
        bide_forced || rollout_forced || rampage_forced || airborne_forced || charge_forced;
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

    if !automatic_struggle {
        let learned_move = battle_moves_mut(state, side)
            .get_mut(slot)
            .ok_or(BattleTurnError::MissingMoveSlot { side, slot })?;
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
    }
    events.push(BattleEvent::MoveUsed {
        side,
        move_name: move_name.clone(),
    });
    if side == BattleSide::Player && !state.player_used_moves.contains(&move_name) {
        if state.player_used_moves.len() == 4 {
            state.player_used_moves.remove(0);
        }
        state.player_used_moves.push(move_name.clone());
    }
    set_last_move(state, side, Some(move_name.clone()));

    if move_blocked_by_status_or_confusion(
        state,
        side,
        &move_name,
        move_data,
        items,
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

    // BattleCommand_DoTurn increments this only after sleep, freeze,
    // paralysis, flinch, attraction, and confusion have allowed the move to
    // proceed into its effect command stream.
    match side {
        BattleSide::Player => state.player_turns_taken = state.player_turns_taken.saturating_add(1),
        BattleSide::Enemy => state.enemy_turns_taken = state.enemy_turns_taken.saturating_add(1),
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
    if move_data.effect == "SELFDESTRUCT"
        && state.pokemon(side).hp == 0
        && !events.iter().any(
            |event| matches!(event, BattleEvent::Fainted { side: fainted } if *fainted == side),
        )
    {
        events.push(BattleEvent::Fainted { side });
    }
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
            items,
            stat_multipliers,
            type_categories,
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
    let bide_release_damage = if direct_bide_effect(move_data) {
        match advance_bide_effect(state, side, move_name, rng, events) {
            BideAdvance::Handled => return Ok(()),
            BideAdvance::Release { stored_damage } => Some(stored_damage),
        }
    } else {
        None
    };
    // `BattleCommand_CheckHit` performs these gates before Lock-On is
    // consumed, accuracy is calculated, or BattleRandom is sampled.
    if bide_release_damage.is_some() || move_effect_uses_check_hit(&move_data.effect) {
        if dream_eater_fails(state, side, move_data) {
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
            if move_data.effect == "SELFDESTRUCT" {
                // Selfdestruct follows `checkhit` in its effect stream and
                // therefore still executes after the protected miss.
                apply_selfdestruct_effect(state, side, move_name, events);
            }
            apply_jump_kick_crash_effect(state, side, move_name, move_data, events);
            return Ok(());
        }
        if matches!(move_data.effect.as_str(), "LEECH_HIT" | "DREAM_EATER")
            && move_blocked_by_substitute(state, side, move_name, side.other(), events)
        {
            return Ok(());
        }
    }
    let lock_on_active = lock_on_target_state(state, side) && !direct_lock_on_effect(move_data);
    let x_accuracy_active = x_accuracy_active(state, side);
    let mut ordinary_accuracy = u8::MAX;
    if !x_accuracy_active && (!lock_on_active || move_checks_accuracy_per_hit(move_data)) {
        let (attacker, defender) = match side {
            BattleSide::Player => (&state.player, &state.enemy),
            BattleSide::Enemy => (&state.enemy, &state.player),
        };
        ordinary_accuracy = accuracy_byte_with_weather(
            move_data,
            side,
            attacker,
            defender,
            stat_multipliers,
            state.weather,
            identified_state(state, side.other()),
        )?;
        if ordinary_accuracy < u8::MAX {
            ordinary_accuracy =
                apply_brightpowder_accuracy(state, side.other(), items, ordinary_accuracy)?;
        }
    }
    let accuracy = if lock_on_active || x_accuracy_active {
        u8::MAX
    } else {
        ordinary_accuracy
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
        let lock_on_hits = lock_on_active
            && (airborne_move == "DIG"
                || !matches!(move_name, "EARTHQUAKE" | "FISSURE" | "MAGNITUDE"));
        if !lock_on_hits && !move_hits_airborne_target(move_data, &airborne_move) {
            events.push(BattleEvent::AirborneAvoided {
                side,
                move_name: move_name.to_string(),
                target: side.other(),
                airborne_move,
            });
            if move_data.effect == "SELFDESTRUCT" {
                apply_selfdestruct_effect(state, side, move_name, events);
            }
            apply_jump_kick_crash_effect(state, side, &move_name, move_data, events);
            return Ok(());
        }
    }
    if accuracy < u8::MAX && !move_checks_accuracy_per_hit(move_data) {
        let roll = rng.battle_random_byte();
        if roll >= accuracy {
            events.push(BattleEvent::Missed {
                side,
                move_name: move_name.to_string(),
                accuracy,
                roll,
            });
            if move_data.effect == "SELFDESTRUCT" {
                apply_selfdestruct_effect(state, side, move_name, events);
            }
            apply_jump_kick_crash_effect(state, side, &move_name, move_data, events);
            reset_fury_cutter_chain(state, side);
            reset_rollout_state(state, side);
            return Ok(());
        }
    }
    // Dream Eater's `checkhit` gate was handled before Lock-On/accuracy above.
    if snore_fails(state, side, move_data) {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    // The source effect stream executes `selfdestruct` immediately after
    // `checkhit`. A miss marks the attack failed but does not skip this
    // command; successful checks reach it here before Protect/type/Substitute
    // damage application settles.
    if move_data.effect == "SELFDESTRUCT" {
        apply_selfdestruct_effect(state, side, move_name, events);
    }
    if counter_effect(move_data).is_some() {
        apply_counter_effect(
            state,
            side,
            &move_name,
            move_data,
            moves,
            type_effectiveness,
            items,
            rng,
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
        apply_teleport_effect(
            state,
            side,
            &move_name,
            force_switch_ends_battle,
            rng,
            events,
        );
        return Ok(());
    }

    if let Some(status) = direct_status_effect(move_data) {
        let target = side.other();
        if move_blocked_by_safeguard(state, side, &move_name, target, status, events) {
            return Ok(());
        }
        if status == "SLEEP"
            && side == BattleSide::Enemy
            && state.enemy_effect_ai_random_fail
            && !state.player_lock_on_target
            && state.pokemon(target).status.as_deref() != Some("SLEEP")
        {
            let roll = rng.battle_random_byte();
            if roll < 64 {
                events.push(BattleEvent::StatusFailed {
                    side,
                    move_name: move_name.to_string(),
                    target,
                    existing_status: None,
                });
                return Ok(());
            }
        }
        if move_blocked_by_substitute(state, side, &move_name, target, events) {
            return Ok(());
        }
        let target_types = effective_pokemon_types(state, target);
        let sleep_turn_mask = state.sleep_turn_mask;
        let defender = state.pokemon_mut(target);
        if defender.status.is_none() {
            let applied = apply_status_to_target(
                defender,
                &target_types,
                side,
                &move_name,
                target,
                status,
                sleep_turn_mask,
                rng,
                events,
            );
            if applied && status == "BAD_POISON" {
                set_toxic_turns(state, target, 1);
            }
            if applied {
                // Sleep/Poison/Burn/Freeze/Paralyze target commands call
                // UseHeldStatusHealingItem immediately after installing the
                // status.  This is distinct from the blanket between-turn
                // HandleHealingItems pass for a status that was already set.
                apply_held_status_healing(state, target, items, events)?;
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
        let event_start = events.len();
        apply_confusion_to_target(state, side, &move_name, rng, events);
        if confusion_applied_since(events, event_start, side.other()) {
            apply_held_status_healing(state, side.other(), items, events)?;
        }
        return Ok(());
    }
    if direct_swagger_effect(move_data) {
        let event_start = events.len();
        apply_swagger_effect(state, side, &move_name, rng, events)?;
        if confusion_applied_since(events, event_start, side.other()) {
            apply_held_status_healing(state, side.other(), items, events)?;
        }
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
            type_effectiveness,
            rng,
            events,
        )?;
        return Ok(());
    }
    if let Some(stored_damage) = bide_release_damage {
        apply_bide_release_effect(
            state,
            side,
            move_name,
            move_data,
            stored_damage,
            type_categories,
            items,
            rng,
            events,
        )?;
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
        apply_force_switch_effect(
            state,
            side,
            &move_name,
            force_switch_ends_battle,
            rng,
            acted_before,
            events,
        )?;
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
            items,
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
            accuracy,
            ordinary_accuracy,
            force_switch_ends_battle,
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
    if move_data.power == 0
        && apply_stat_stage_effect(state, side, &move_name, move_data, rng, events)?
    {
        return Ok(());
    }

    let mut hit_count = match move_data.effect.as_str() {
        "DOUBLE_HIT" | "POISON_MULTI_HIT" => 2,
        _ => 1,
    };
    let mut hit_count_roll = None;
    let mut completed_hits = 0u8;
    let mut completed_loop = true;
    let mut poison_multi_effect_succeeds = false;
    let mut hit_index = 0u8;
    while hit_index < hit_count {
        if move_checks_accuracy_per_hit(move_data) {
            let hit_accuracy = if hit_index == 0 {
                accuracy
            } else {
                ordinary_accuracy
            };
            if hit_accuracy < u8::MAX {
                let roll = rng.battle_random_byte();
                if roll >= hit_accuracy {
                    events.push(BattleEvent::Missed {
                        side,
                        move_name: move_name.to_string(),
                        accuracy: hit_accuracy,
                        roll,
                    });
                    completed_loop = false;
                    break;
                }
            }
        }
        if move_data.effect == "POISON_MULTI_HIT" {
            // Twineedle's effectchance command is inside its hit loop and
            // precedes critical/damage calculation, unlike the other
            // damaging effect scripts sampled by apply_damage_hit.
            let effect_chance = sample_effect_chance_against_target(state, side, move_data, rng);
            poison_multi_effect_succeeds = effect_chance.succeeds;
            if !effect_chance.succeeds {
                if let Some(roll) = effect_chance.roll {
                    events.push(BattleEvent::SecondaryStatusMissed {
                        side,
                        move_name: move_name.to_string(),
                        target: side.other(),
                        status: "POISON".to_string(),
                        chance_percent: effect_chance.chance_percent,
                        roll,
                    });
                }
            }
        }
        let event_start = events.len();
        let damage_result = apply_damage_hit(
            state,
            side,
            &move_name,
            move_data,
            hit_index.saturating_add(1),
            target_switching,
            acted_before.contains(&side.other()),
            stat_multipliers,
            type_categories,
            type_effectiveness,
            weather_modifiers,
            items,
            rng,
            events,
        )?;
        let hit_applied = events[event_start..].iter().any(|event| {
            matches!(
                event,
                BattleEvent::Damage { .. } | BattleEvent::SubstituteDamaged { .. }
            )
        });
        match damage_result {
            DamageHitResult::NoEffect => return Ok(()),
            DamageHitResult::Continue => completed_hits += u8::from(hit_applied),
            DamageHitResult::Stop => {
                completed_hits += u8::from(hit_applied);
                completed_loop = false;
                break;
            }
        }
        if hit_index == 0 {
            if move_data.effect == "MULTI_HIT" {
                let (sampled_hits, sampled_roll) = sample_multi_hit_count(rng);
                hit_count = sampled_hits;
                hit_count_roll = Some(sampled_roll);
            } else if move_data.effect == "TRIPLE_KICK" {
                let sampled = loop {
                    let sampled = rng.battle_random_byte() & 3;
                    if sampled != 0 {
                        break sampled;
                    }
                };
                hit_count = sampled;
                hit_count_roll = Some(sampled);
            }
        }
        hit_index = hit_index.saturating_add(1);
    }
    if completed_loop && completed_hits > 0 && (hit_count > 1 || move_data.effect == "TRIPLE_KICK")
    {
        events.push(BattleEvent::MultiHitCount {
            side,
            move_name: move_name.to_string(),
            hits: completed_hits,
            roll: hit_count_roll,
        });
    }
    if completed_loop && completed_hits > 0 {
        apply_kings_rock_flinch(state, side, &move_name, move_data, 1, items, rng, events)?;
        if move_data.effect == "POISON_MULTI_HIT" && poison_multi_effect_succeeds {
            let event_start = events.len();
            apply_secondary_status_after_success(
                state,
                side,
                &move_name,
                side.other(),
                "POISON",
                rng,
                events,
            );
            if status_applied_since(events, event_start, side.other()) {
                apply_held_status_healing(state, side.other(), items, events)?;
            }
        }
    }
    Ok(())
}

fn held_item_type_boost_percent(
    state: &BattleCombatState,
    side: BattleSide,
    move_type: &PokemonType,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<u8, BattleTurnError> {
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(0);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if held_item_boosted_move_type(&item.held_effect) != Some(move_type.as_str()) {
        return Ok(0);
    }
    if !(1..=255).contains(&item.parameter) {
        return Err(BattleTurnError::InvalidHeldItemParameter {
            side,
            item_id: item_id.to_string(),
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
        });
    }
    let parameter = item.parameter as u8;
    events.push(BattleEvent::HeldItemDamageBoost {
        side,
        item_id: item_id.to_string(),
        held_effect: item.held_effect.clone(),
        move_type: move_type.clone(),
        parameter,
    });
    Ok(parameter)
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
        "HELD_PSYCHIC_BOOST" => Some("PSYCHIC_TYPE"),
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
    hit_number: u8,
    target_switching: bool,
    target_already_acted: bool,
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
    if protect_active(state, side.other()) {
        events.push(BattleEvent::MoveProtected {
            side,
            move_name: move_name.to_string(),
            target: side.other(),
        });
        apply_jump_kick_crash_effect(state, side, move_name, move_data, events);
        return Ok(DamageHitResult::Stop);
    }
    let (critical, critical_roll, critical_threshold, damage_roll, result) =
        if is_fixed_damage_effect(move_data) {
            let type_multiplier = calculate_type_effectiveness_multiplier_with_foresight(
                type_effectiveness,
                &move_data.move_type,
                &effective_pokemon_types(state, side.other()),
                identified_state(state, side.other()),
            )
            .map_err(BattleTurnError::DamageCalculation)?;
            let (damage, type_multiplier) = if type_multiplier.numerator == 0 {
                (0, type_multiplier)
            } else {
                (
                    fixed_damage_amount(&attacker, &defender, move_data, rng)
                        .expect("fixed-damage effect lost its exact amount"),
                    TypeMultiplier::one(),
                )
            };
            (
                false,
                0,
                0,
                255,
                DamageResult {
                    damage,
                    type_multiplier,
                },
            )
        } else {
            let held_type_boost_percent = held_item_type_boost_percent(
                state,
                side,
                &prepared_move_data.move_type,
                items,
                events,
            )?;
            let (critical, critical_roll, critical_threshold) =
                roll_critical_hit(side, move_name, &attacker, items, rng)?;
            let damage_roll = crystal_damage_variation_roll(rng);
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
                    attacker_badge_boost: badge_boost_active(
                        state,
                        side,
                        if is_physical_type(type_categories, &prepared_move_data.move_type)
                            .map_err(BattleTurnError::DamageCalculation)?
                        {
                            Stat::Attack
                        } else {
                            Stat::SpecialAttack
                        },
                    ),
                    defender_badge_boost: badge_boost_active(
                        state,
                        side.other(),
                        if is_physical_type(type_categories, &prepared_move_data.move_type)
                            .map_err(BattleTurnError::DamageCalculation)?
                        {
                            Stat::Defense
                        } else {
                            Stat::SpecialDefense
                        },
                    ),
                    attacker_type_badge_boost: prepared_move_data.name != "STRUGGLE"
                        && badge_type_boost_active(state, side, &prepared_move_data.move_type),
                    defender_metal_powder: ditto_holds_metal_powder(state, side.other()),
                    defender_screen: active_damage_screen(
                        state,
                        side.other(),
                        type_categories,
                        &prepared_move_data,
                    )?
                    .is_some(),
                    link_colosseum: state.link_colosseum,
                    held_type_boost_percent,
                    pre_stab_multiplier: if prepared_move_data.effect == "TRIPLE_KICK" {
                        hit_number.max(1)
                    } else {
                        1
                    },
                    rage_counter: if prepared_move_data.effect == "RAGE" {
                        rage_counter(state, side)
                    } else {
                        0
                    },
                },
            )
            .map_err(BattleTurnError::DamageCalculation)?;
            (
                critical,
                critical_roll,
                critical_threshold,
                damage_roll,
                result,
            )
        };
    if result.type_multiplier.numerator == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        apply_jump_kick_crash_effect(state, side, move_name, move_data, events);
        return Ok(DamageHitResult::NoEffect);
    }

    // In the compiled effect streams this command is after checkhit but
    // before move animation/applydamage. It therefore samples even when the
    // impending damage will faint the target, and it must precede the Focus
    // Band roll inside applydamage. A pre-existing Substitute makes the
    // command fail without calling BattleRandom.
    let pre_damage_effect_chance = sample_pre_damage_effect_chance(state, side, move_data, rng);

    let defender_hp_before = state.pokemon(side.other()).hp;
    let raw_damage = if move_data.effect == "STOMP" && minimized_state(state, side.other()) {
        result.damage.saturating_mul(2)
    } else {
        result.damage
    };
    if let Some(hit_result) = apply_substitute_damage(state, side, move_name, raw_damage, events) {
        apply_rage_counter_increment(state, side.other(), events)?;
        if move_data.effect == "DEFENSE_DOWN_HIT" {
            // This script deliberately has another effectchance after
            // applydamage. If the hit just broke the Substitute, that second
            // command now samples and can lower the real target's Defense.
            let effect_chance = sample_effect_chance_against_target(state, side, move_data, rng);
            apply_secondary_stat_stage_effect(
                state,
                side,
                move_name,
                move_data,
                effect_chance,
                rng,
                events,
            )?;
        }
        if move_data.effect == "HYPER_BEAM" {
            set_recharge_move_state(state, side, Some(move_name.to_string()));
            events.push(BattleEvent::RechargeStarted {
                side,
                move_name: move_name.to_string(),
            });
        }
        return Ok(hit_result);
    }
    let mut damage = raw_damage.min(defender_hp_before);
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
                held_item: None,
            });
        }
    }
    if damage >= defender_hp_before
        && defender_hp_before > 1
        && focus_band_survives(state, side.other(), items, rng)?
    {
        let lethal_damage = damage;
        damage = defender_hp_before - 1;
        events.push(BattleEvent::EnduredHit {
            side,
            move_name: move_name.to_string(),
            target: side.other(),
            raw_damage: lethal_damage,
            held_item: state.pokemon(side.other()).item.clone(),
        });
    }
    let applied_result = DamageResult {
        damage: raw_damage,
        type_multiplier: result.type_multiplier,
    };
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
        apply_rage_counter_increment(state, side.other(), events)?;
        apply_bide_damage_storage(state, side, side.other(), damage, events);
    }
    let target_fainted_from_hit = state.pokemon(side.other()).hp == 0;
    let secondary_stat_handled = if move_data.effect == "RAPID_SPIN" {
        apply_post_damage_stat_effect(
            state,
            side,
            move_name,
            move_data,
            damage,
            pre_damage_effect_chance,
            rng,
            events,
        )?
    } else if target_fainted_from_hit {
        move_data.effect == "ALL_UP_HIT" || secondary_stat_hit_effect(move_data)
    } else {
        apply_post_damage_stat_effect(
            state,
            side,
            move_name,
            move_data,
            damage,
            pre_damage_effect_chance,
            rng,
            events,
        )?
    };
    apply_post_damage_hp_effect(
        state,
        side,
        move_name,
        move_data,
        damage,
        pre_damage_effect_chance,
        events,
    );
    apply_direct_damage_faint_events(state, side, side.other(), move_name, events);
    if move_data.effect == "HYPER_BEAM" && damage > 0 {
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
    }
    if move_defers_post_hit_effects(move_data) {
        return Ok(DamageHitResult::Continue);
    } else if move_data.effect == "TRAP_TARGET" {
        apply_trap_target_effect(state, side, move_name, rng, events);
    } else if move_data.effect == "TRI_ATTACK" {
        let event_start = events.len();
        apply_tri_attack_effect(state, side, move_name, move_data, rng, events);
        if status_applied_since(events, event_start, side.other()) {
            apply_held_status_healing(state, side.other(), items, events)?;
        }
    } else if let Some((status, _chance_percent)) = secondary_status_effect(move_data) {
        let event_start = events.len();
        apply_secondary_status_effect(
            state,
            side,
            move_name,
            status,
            pre_damage_effect_chance
                .expect("damaging status effect script must sample before damage"),
            rng,
            events,
        );
        if status_applied_since(events, event_start, side.other()) {
            apply_held_status_healing(state, side.other(), items, events)?;
        }
    } else if secondary_confusion_effect(move_data).is_some() {
        let event_start = events.len();
        apply_secondary_confusion_effect(
            state,
            side,
            move_name,
            pre_damage_effect_chance
                .expect("damaging confusion effect script must sample before damage"),
            rng,
            events,
        );
        if confusion_applied_since(events, event_start, side.other()) {
            apply_held_status_healing(state, side.other(), items, events)?;
        }
    } else if secondary_flinch_effect(move_data).is_some() {
        apply_secondary_flinch_effect(
            state,
            side,
            move_name,
            pre_damage_effect_chance
                .expect("damaging flinch effect script must sample before damage"),
            target_already_acted,
            events,
        );
    } else if !secondary_stat_handled {
        if let Some(effect_chance) = pre_damage_effect_chance {
            apply_secondary_stat_stage_effect(
                state,
                side,
                move_name,
                move_data,
                effect_chance,
                rng,
                events,
            )?;
        }
    }
    apply_kings_rock_flinch(
        state, side, move_name, move_data, damage, items, rng, events,
    )?;
    Ok(DamageHitResult::Continue)
}

fn move_defers_post_hit_effects(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "MULTI_HIT" | "DOUBLE_HIT" | "POISON_MULTI_HIT" | "TRIPLE_KICK"
    )
}

fn apply_kings_rock_flinch(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    damage: u16,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    // Crystal uses a separate 8-bit BattleRandom roll against the held
    // parameter whenever the effect script contains `kingsrock`. Sky Attack
    // and Snore intentionally execute this even after their built-in flinch
    // command.
    if damage == 0 || !move_has_kings_rock_command(move_data) {
        return Ok(());
    }
    let target = side.other();
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(());
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if item.held_effect != "HELD_FLINCH" || state.pokemon(target).hp == 0 {
        return Ok(());
    }
    // HeldFlinch checks the target's live Substitute after damage and before
    // consuming its independent King's Rock random byte.
    if substitute_hp(state, target) != 0 {
        return Ok(());
    }
    if !(1..=255).contains(&item.parameter) {
        return Err(BattleTurnError::InvalidHeldItemParameter {
            side,
            item_id: item_id.to_string(),
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
        });
    }
    if (rng.battle_random_byte() as i16) < item.parameter {
        set_recharge_move_state(state, target, None);
        state.pokemon_mut(target).flinching = true;
        events.push(BattleEvent::FlinchApplied {
            side,
            move_name: move_name.to_string(),
            target,
        });
    }
    Ok(())
}

fn move_has_kings_rock_command(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "NORMAL_HIT"
            | "LEECH_HIT"
            | "SELFDESTRUCT"
            | "ALWAYS_HIT"
            | "BIDE"
            | "RAMPAGE"
            | "MULTI_HIT"
            | "PAY_DAY"
            | "UNUSED_25"
            | "RAZOR_WIND"
            | "SUPER_FANG"
            | "STATIC_DAMAGE"
            | "UNUSED_2B"
            | "DOUBLE_HIT"
            | "JUMP_KICK"
            | "RECOIL_HIT"
            | "SKY_ATTACK"
            | "POISON_MULTI_HIT"
            | "UNUSED_4E"
            | "RAGE"
            | "LEVEL_DAMAGE"
            | "PSYWAVE"
            | "COUNTER"
            | "SNORE"
            | "REVERSAL"
            | "FALSE_SWIPE"
            | "PRIORITY_HIT"
            | "TRIPLE_KICK"
            | "THIEF"
            | "UNUSED_6E"
            | "ROLLOUT"
            | "FURY_CUTTER"
            | "RETURN"
            | "PRESENT"
            | "FRUSTRATION"
            | "MAGNITUDE"
            | "PURSUIT"
            | "RAPID_SPIN"
            | "UNUSED_82"
            | "UNUSED_83"
            | "HIDDEN_POWER"
            | "MIRROR_COAT"
            | "SKULL_BASH"
            | "SOLARBEAM"
            | "BEAT_UP"
            | "FLY"
    )
}

fn apply_beat_up_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    accuracy: u8,
    ordinary_accuracy: u8,
    wild_battle: bool,
    _target_switching: bool,
    stat_multipliers: &BattleStatMultiplierTables,
    _type_categories: &TypeCategories,
    _type_effectiveness: &TypeEffectivenessTable,
    _weather_modifiers: &WeatherModifiers,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let participants = beat_up_participants(state, side);
    if participants.is_empty() {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        // BeatUpFailText returns to the shared tail, so Crystal still executes
        // King's Rock when every party participant is ineligible.
        apply_kings_rock_flinch(state, side, move_name, move_data, 1, items, rng, events)?;
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

    let mut completed_loop = true;
    let mut hit_any = false;
    for (hit_index, (party_index, participant)) in participants.into_iter().enumerate() {
        let hit_accuracy = if hit_index == 0 {
            accuracy
        } else {
            ordinary_accuracy
        };
        if hit_accuracy < u8::MAX {
            let roll = rng.battle_random_byte();
            if roll >= hit_accuracy {
                events.push(BattleEvent::Missed {
                    side,
                    move_name: move_name.to_string(),
                    accuracy: hit_accuracy,
                    roll,
                });
                completed_loop = false;
                break;
            }
        }
        let held_type_boost_percent =
            held_item_type_boost_percent(state, side, &move_data.move_type, items, events)?;
        let active_attacker = effective_battle_pokemon(state, side);
        let (critical, critical_roll, critical_threshold) =
            roll_critical_hit(side, move_name, &active_attacker, items, rng)?;
        let damage_roll = crystal_damage_variation_roll(rng);
        let result = if side == BattleSide::Enemy && wild_battle {
            calculate_wild_enemy_beat_up_damage(
                state,
                side,
                move_data.power,
                stat_multipliers,
                held_type_boost_percent,
                critical,
                damage_roll,
            )?
        } else {
            calculate_beat_up_damage(
                &participant,
                state.pokemon(side.other()),
                move_data.power,
                held_type_boost_percent,
                critical,
                damage_roll,
            )
        };
        events.push(BattleEvent::BeatUpParticipant {
            side,
            move_name: move_name.to_string(),
            party_index,
            species: participant.species.id.clone(),
            nickname: participant.nickname.clone(),
            shiny: participant.dvs.defense == 10
                && participant.dvs.speed == 10
                && participant.dvs.special == 10
                && matches!(participant.dvs.attack, 2 | 3 | 6 | 7 | 10 | 11 | 14 | 15),
        });
        let defender_hp_before = state.pokemon(side.other()).hp;
        let raw_damage = result.damage;
        if apply_substitute_damage(state, side, move_name, raw_damage, events).is_some() {
            hit_any = true;
            apply_rage_counter_increment(state, side.other(), events)?;
            continue;
        }
        let mut damage = raw_damage.min(defender_hp_before);
        if endure_active(state, side.other()) && defender_hp_before > 1 {
            let endured_damage = damage;
            damage = damage.min(defender_hp_before - 1);
            if endured_damage != damage {
                events.push(BattleEvent::EnduredHit {
                    side,
                    move_name: move_name.to_string(),
                    target: side.other(),
                    raw_damage: endured_damage,
                    held_item: None,
                });
            }
        }
        if damage >= defender_hp_before
            && defender_hp_before > 1
            && focus_band_survives(state, side.other(), items, rng)?
        {
            let lethal_damage = damage;
            damage = defender_hp_before - 1;
            events.push(BattleEvent::EnduredHit {
                side,
                move_name: move_name.to_string(),
                target: side.other(),
                raw_damage: lethal_damage,
                held_item: state.pokemon(side.other()).item.clone(),
            });
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
            critical,
            critical_roll,
            critical_threshold,
            roll: damage_roll,
            result: DamageResult {
                damage: raw_damage,
                type_multiplier: result.type_multiplier,
            },
        });
        hit_any = true;
        if damage > 0 {
            record_last_damage(
                state,
                side.other(),
                BattleLastDamageState {
                    source: side,
                    move_name: move_name.to_string(),
                    category: BattleDamageCategory::Physical,
                    damage,
                },
            );
            apply_rage_counter_increment(state, side.other(), events)?;
            apply_bide_damage_storage(state, side, side.other(), damage, events);
        }
        apply_direct_damage_faint_events(state, side, side.other(), move_name, events);
        if state.pokemon(side.other()).hp == 0 || state.pokemon(side).hp == 0 {
            return Ok(());
        }
    }

    if completed_loop && hit_any {
        apply_kings_rock_flinch(state, side, move_name, move_data, 1, items, rng, events)?;
    }

    Ok(())
}

fn calculate_beat_up_damage(
    participant: &Pokemon,
    defender: &Pokemon,
    power: u16,
    held_type_boost_percent: u8,
    critical: bool,
    damage_roll: u8,
) -> DamageResult {
    let level_factor = (u32::from(participant.level) * 2) / 5 + 2;
    let attack = u32::from(participant.species.base_stats.attack.max(1));
    let defense = u32::from(defender.species.base_stats.defense.max(1));
    let mut damage = level_factor
        .saturating_mul(u32::from(power))
        .saturating_mul(attack)
        / defense
        / 50;
    if held_type_boost_percent != 0 {
        damage = damage.saturating_mul(100 + u32::from(held_type_boost_percent)) / 100;
    }
    if critical {
        damage = damage.saturating_mul(2);
    }
    damage = damage.min(997) + 2;
    damage = damage.saturating_mul(u32::from(damage_roll.max(1))) / 255;
    DamageResult {
        damage: damage.max(1).min(u32::from(u16::MAX)) as u16,
        type_multiplier: TypeMultiplier::one(),
    }
}

fn calculate_wild_enemy_beat_up_damage(
    state: &BattleCombatState,
    side: BattleSide,
    power: u16,
    stat_multipliers: &BattleStatMultiplierTables,
    held_type_boost_percent: u8,
    critical: bool,
    damage_roll: u8,
) -> Result<DamageResult, BattleTurnError> {
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
    let ignore_stages = critical && defense_stage > attack_stage;
    let mut attack = if ignore_stages {
        attacker.special_attack
    } else {
        apply_stage(stat_multipliers, attacker.special_attack, attack_stage).ok_or(
            BattleTurnError::MissingStatMultiplier {
                side,
                stage: attack_stage,
            },
        )?
    };
    let mut defense = if ignore_stages {
        defender.special_defense
    } else {
        apply_stage(stat_multipliers, defender.special_defense, defense_stage).ok_or(
            BattleTurnError::MissingStatMultiplier {
                side: defender_side,
                stage: defense_stage,
            },
        )?
    };
    if attacker.species.id == "PIKACHU" && attacker.item.as_deref() == Some("LIGHT_BALL") {
        attack = attack.wrapping_mul(2);
    }
    if !ignore_stages && screen_turns(state, defender_side, BattleScreen::LightScreen) != 0 {
        defense = defense.wrapping_mul(2);
    }
    let (mut attack, mut defense) = truncate_damage_stats(attack, defense, state.link_colosseum);
    if defender.species.id == "DITTO" && defender.item.as_deref() == Some("METAL_POWDER") {
        (attack, defense) = apply_metal_powder_damage_stats(attack, defense);
    }
    let level_factor = (u32::from(attacker.level) * 2) / 5 + 2;
    let mut damage = level_factor
        .saturating_mul(u32::from(power))
        .saturating_mul(u32::from(attack))
        / u32::from(defense)
        / 50;
    if held_type_boost_percent != 0 {
        damage = damage.saturating_mul(100 + u32::from(held_type_boost_percent)) / 100;
    }
    if critical {
        damage = damage.saturating_mul(2);
    }
    damage = damage.min(997) + 2;
    damage = damage.saturating_mul(u32::from(damage_roll.max(1))) / 255;
    Ok(DamageResult {
        damage: damage.max(1).min(u32::from(u16::MAX)) as u16,
        type_multiplier: TypeMultiplier::one(),
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EffectChanceResult {
    chance_percent: u8,
    succeeds: bool,
    roll: Option<u8>,
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

    let roll = rng.battle_random_byte();
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
        103..=179 => PresentRoll::Damage(80),
        180..=204 => PresentRoll::Damage(120),
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
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let attacker = state.pokemon(side).clone();
    let defender = state.pokemon(side.other()).clone();
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
    let target = side.other();
    if protect_active(state, target) {
        events.push(BattleEvent::MoveProtected {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    }
    let level_delta = attacker.level.saturating_sub(defender.level);
    let base_accuracy = (u16::from(move_data.accuracy.min(100)) * 255 / 100) as u8;
    let mut accuracy = base_accuracy.saturating_add(level_delta.saturating_mul(2));
    let lock_on_active = lock_on_target_state(state, side);
    if lock_on_active {
        set_lock_on_target_state(state, side, false);
        events.push(BattleEvent::LockOnConsumed {
            side,
            move_name: move_name.to_string(),
            target,
        });
    }
    let airborne_move = airborne_move_state(state, target).map(ToOwned::to_owned);
    let lock_on_guarantees_hit = lock_on_active
        && !airborne_move
            .as_deref()
            .is_some_and(|airborne| airborne == "FLY" && move_name == "FISSURE");
    let x_accuracy_guarantees_hit = x_accuracy_active(state, side);
    let roll = if lock_on_guarantees_hit {
        0
    } else {
        if let Some(airborne_move) = airborne_move {
            if !move_hits_airborne_target(move_data, &airborne_move) {
                events.push(BattleEvent::AirborneAvoided {
                    side,
                    move_name: move_name.to_string(),
                    target,
                    airborne_move,
                });
                return Ok(());
            }
        }
        if x_accuracy_guarantees_hit {
            0
        } else {
            let attacker_accuracy = *attacker.stat_boosts.get(&Stat::Accuracy).ok_or(
                BattleTurnError::MissingStatStage {
                    side,
                    stat: Stat::Accuracy,
                },
            )?;
            let defender_evasion = *defender.stat_boosts.get(&Stat::Evasion).ok_or(
                BattleTurnError::MissingStatStage {
                    side: target,
                    stat: Stat::Evasion,
                },
            )?;
            if !(identified_state(state, target) && defender_evasion >= attacker_accuracy) {
                let stage = (attacker_accuracy - defender_evasion).clamp(-6, 6);
                let multiplier = accuracy_stage_multiplier(stat_multipliers, stage)
                    .ok_or(BattleTurnError::MissingAccuracyMultiplier { stage })?;
                accuracy = multiplier.multiply_floor(i32::from(accuracy)).clamp(1, 255) as u8;
            }
            accuracy = apply_brightpowder_accuracy(state, target, items, accuracy)?;
            let roll = rng.battle_random_byte();
            if roll >= accuracy {
                events.push(BattleEvent::OhkoFailed {
                    side,
                    move_name: move_name.to_string(),
                    reason: OhkoFailureReason::Missed { accuracy, roll },
                });
                return Ok(());
            }
            roll
        }
    };

    let defender_hp_before = state.pokemon(target).hp;
    if apply_substitute_damage(state, side, move_name, u16::MAX, events).is_some() {
        apply_rage_counter_increment(state, target, events)?;
        return Ok(());
    }
    let mut damage = defender_hp_before;
    if endure_active(state, target) && defender_hp_before > 1 {
        damage = defender_hp_before - 1;
        events.push(BattleEvent::EnduredHit {
            side,
            move_name: move_name.to_string(),
            target,
            raw_damage: defender_hp_before,
            held_item: None,
        });
    } else if defender_hp_before > 1 && focus_band_survives(state, target, items, rng)? {
        damage = defender_hp_before - 1;
        events.push(BattleEvent::EnduredHit {
            side,
            move_name: move_name.to_string(),
            target,
            raw_damage: defender_hp_before,
            held_item: state.pokemon(target).item.clone(),
        });
    }
    state.pokemon_mut(target).hp = defender_hp_before.saturating_sub(damage);
    let defender_hp_after = state.pokemon(target).hp;
    events.push(BattleEvent::Damage {
        side,
        move_name: move_name.to_string(),
        damage,
        defender_hp_before,
        defender_hp_after,
        critical: false,
        critical_roll: 0,
        critical_threshold: 0,
        roll,
        result: DamageResult {
            damage: u16::MAX,
            type_multiplier,
        },
    });
    record_last_damage(
        state,
        target,
        BattleLastDamageState {
            source: side,
            move_name: move_name.to_string(),
            category: damage_category(type_categories, move_data)?,
            damage,
        },
    );
    apply_rage_counter_increment(state, target, events)?;
    apply_bide_damage_storage(state, side, target, damage, events);
    apply_direct_damage_faint_events(state, side, target, move_name, events);
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

fn move_blocked_by_status_or_confusion(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let pokemon = state.pokemon_mut(side);
    let mut woke_up = false;
    let blocked_by_status = match pokemon.status.as_deref() {
        Some("SLEEP") => {
            pokemon.sleep_turns = pokemon.sleep_turns.saturating_sub(1);
            if pokemon.sleep_turns == 0 {
                pokemon.status = None;
                woke_up = true;
                events.push(BattleEvent::WokeUp {
                    side,
                    move_name: move_name.to_string(),
                });
                false
            } else {
                events.push(BattleEvent::SleepTurn {
                    side,
                    move_name: move_name.to_string(),
                    turns_remaining: pokemon.sleep_turns,
                });
                !move_usable_while_asleep(move_data)
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
    if woke_up {
        set_nightmare_source(state, side, None);
    }
    if blocked_by_status {
        return Ok(true);
    }
    if move_blocked_by_flinch(state, side, move_name, events) {
        return Ok(true);
    }
    tick_disable_before_action(state, side, events);
    if move_blocked_by_paralysis(state, side, move_name, rng, events) {
        return Ok(true);
    }
    if move_blocked_by_attract(state, side, move_name, rng, events) {
        return Ok(true);
    }
    move_blocked_by_confusion(
        state,
        side,
        move_name,
        move_data,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        events,
    )
}

fn tick_disable_before_action(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    let Some(disable) = disable_state(state, side).cloned() else {
        return;
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
                ..disable
            }),
        );
    }
}

fn move_blocked_by_paralysis(
    state: &BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> bool {
    if state.pokemon(side).status.as_deref() != Some("PARALYSIS") {
        return false;
    }
    let roll = rng.battle_random_byte();
    if roll >= 64 {
        return false;
    }
    events.push(BattleEvent::FullyParalyzed {
        side,
        move_name: move_name.to_string(),
        roll,
    });
    true
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
    let roll = rng.battle_random_byte() & 1;
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
    selected_move: &Move,
    items: &BTreeMap<String, Item>,
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

    let confusion_roll = rng.battle_random_byte() & 1;
    events.push(BattleEvent::ConfusedTurn {
        side,
        move_name: move_name.to_string(),
        turns_remaining,
        roll: confusion_roll,
    });
    if confusion_roll != 0 {
        return Ok(false);
    }

    apply_confusion_self_damage(
        state,
        side,
        move_name,
        selected_move,
        items,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        rng,
        events,
    )?;
    Ok(true)
}

fn apply_confusion_self_damage(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    selected_move: &Move,
    items: &BTreeMap<String, Item>,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let damage_roll = crystal_damage_variation_roll(rng);
    let mut damage_move = confusion_damage_move();
    // DamageCalc reads the already-loaded selected move effect even though
    // confusion replaces its power with 40. Selecting Selfdestruct or
    // Explosion therefore retains the ROM's defense-halving confusion bug.
    if selected_move.effect == "SELFDESTRUCT" {
        damage_move.effect = selected_move.effect.clone();
    }
    let held_type_boost_percent =
        held_item_type_boost_percent(state, side, &selected_move.move_type, items, events)?;
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
            attacker_badge_boost: badge_boost_active(state, side, Stat::Attack),
            defender_badge_boost: badge_boost_active(state, side, Stat::Defense),
            attacker_type_badge_boost: false,
            defender_metal_powder: ditto_holds_metal_powder(state, side),
            defender_screen: screen_turns(state, side, BattleScreen::Reflect) != 0,
            link_colosseum: state.link_colosseum,
            held_type_boost_percent,
            pre_stab_multiplier: 1,
            rage_counter: 0,
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
    Ok(())
}

fn ditto_holds_metal_powder(state: &BattleCombatState, side: BattleSide) -> bool {
    let pokemon = state.pokemon(side);
    pokemon.species.id == "DITTO" && pokemon.item.as_deref() == Some("METAL_POWDER")
}

fn apply_held_status_healing(
    state: &mut BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
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
    let held_effect = item.held_effect.clone();
    let status_before = state.pokemon(side).status.clone();
    let confusion_turns_before = state.pokemon(side).confusion_turns;
    let heals_status = held_item_heals_status(&held_effect, status_before.as_deref());
    let heals_confusion = matches!(
        held_effect.as_str(),
        "HELD_HEAL_CONFUSION" | "HELD_HEAL_STATUS"
    ) && confusion_turns_before != 0;
    if !heals_status && !heals_confusion {
        return Ok(());
    }
    if heals_status {
        {
            let pokemon = state.pokemon_mut(side);
            pokemon.status = None;
            pokemon.sleep_turns = 0;
        }
        set_toxic_turns(state, side, 0);
        if status_before.as_deref() == Some("SLEEP") {
            set_nightmare_source(state, side, None);
        }
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
    Ok(())
}

fn apply_held_hp_healing(
    state: &mut BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let pokemon = state.pokemon(side);
    if pokemon.hp == 0 || pokemon.hp.saturating_mul(2) >= pokemon.max_hp {
        return Ok(());
    }
    let Some(item_id) = pokemon.item.clone() else {
        return Ok(());
    };
    let item = items
        .get(&item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.clone(),
        })?;
    if item.held_effect != "HELD_BERRY" {
        return Ok(());
    }
    if item.parameter <= 0 {
        return Err(BattleTurnError::InvalidHeldItemParameter {
            side,
            item_id,
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
        });
    }
    let hp_before = pokemon.hp;
    let amount = item.parameter as u16;
    let hp_after = hp_before.saturating_add(amount).min(pokemon.max_hp);
    let pokemon = state.pokemon_mut(side);
    pokemon.hp = hp_after;
    pokemon.item = None;
    events.push(BattleEvent::HeldItemHpHealed {
        side,
        item_id,
        hp_before,
        hp_after,
        amount: hp_after - hp_before,
    });
    Ok(())
}

fn apply_end_turn_leftovers(
    state: &mut BattleCombatState,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    for side in between_turn_side_order(state.serial_connection_status) {
        let pokemon = state.pokemon(side);
        if pokemon.hp == 0 || pokemon.hp >= pokemon.max_hp {
            continue;
        }
        let Some(item_id) = pokemon.item.clone() else {
            continue;
        };
        let item = items
            .get(&item_id)
            .ok_or_else(|| BattleTurnError::UnknownHeldItem {
                side,
                item_id: item_id.clone(),
            })?;
        if item.held_effect != "HELD_LEFTOVERS" {
            continue;
        }
        let hp_before = pokemon.hp;
        let amount = (pokemon.max_hp / 16).max(1);
        let hp_after = hp_before.saturating_add(amount).min(pokemon.max_hp);
        state.pokemon_mut(side).hp = hp_after;
        events.push(BattleEvent::HealApplied {
            side,
            move_name: item_id,
            hp_before,
            hp_after,
            amount: hp_after - hp_before,
            animation_param: 0,
        });
    }
    Ok(())
}

fn apply_end_turn_mystery_berry(
    state: &mut BattleCombatState,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    for side in between_turn_side_order(state.serial_connection_status) {
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
        if item.held_effect != "HELD_RESTORE_PP" {
            continue;
        }
        let Some(slot) = battle_moves(state, side)
            .iter()
            .position(|learned| learned.current_pp == 0)
        else {
            continue;
        };
        let learned = &battle_moves(state, side)[slot];
        let move_name = learned.name.clone();
        let move_data = moves
            .get(&move_name)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: move_name.clone(),
            })?;
        let pp_before = learned.current_pp;
        let restore = if move_name == "SKETCH" { 1 } else { 5 };
        let pp_after = pp_before
            .saturating_add(restore)
            .min(crate::models::max_move_pp(move_data.pp, learned.pp_ups));
        battle_moves_mut(state, side)[slot].current_pp = pp_after;
        state.pokemon_mut(side).item = None;
        events.push(BattleEvent::HeldItemPpRestored {
            side,
            item_id,
            move_name,
            slot,
            pp_before,
            pp_after,
            amount: pp_after - pp_before,
        });
    }
    Ok(())
}

fn apply_end_turn_defrost(
    state: &mut BattleCombatState,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    for side in between_turn_side_order(state.serial_connection_status) {
        if state.pokemon(side).status.as_deref() != Some("FREEZE") {
            continue;
        }
        let just_frozen = events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::StatusApplied {
                    target,
                    status,
                    ..
                } if *target == side && status == "FREEZE"
            )
        });
        if just_frozen || rng.battle_random_byte() >= 25 {
            continue;
        }
        state.pokemon_mut(side).status = None;
        events.push(BattleEvent::StatusHealed {
            side: side.other(),
            move_name: "DEFROST".to_string(),
            target: side,
            status_before: "FREEZE".to_string(),
        });
    }
}

fn apply_end_turn_encore(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in between_turn_side_order(state.serial_connection_status) {
        let Some(encore) = encore_state(state, side).cloned() else {
            continue;
        };
        let turns_remaining = encore.turns_remaining.saturating_sub(1);
        let forced_move_has_pp = battle_moves(state, side)
            .iter()
            .find(|learned| learned.name == encore.move_name)
            .is_some_and(|learned| learned.current_pp != 0);
        if turns_remaining == 0 || !forced_move_has_pp {
            clear_encore_state(state, side);
            events.push(BattleEvent::EncoreEnded {
                side,
                move_name: encore.move_name,
            });
        } else {
            set_encore_state(
                state,
                side,
                Some(BattleEncoreState {
                    move_name: encore.move_name,
                    turns_remaining,
                }),
            );
        }
    }
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

fn focus_band_survives(
    state: &BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
) -> Result<bool, BattleTurnError> {
    let Some(item_id) = state.pokemon(side).item.as_deref() else {
        return Ok(false);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side,
            item_id: item_id.to_string(),
        })?;
    if item.held_effect != "HELD_FOCUS_BAND" {
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
    Ok((rng.battle_random_byte() as i16) < item.parameter)
}

fn apply_brightpowder_accuracy(
    state: &BattleCombatState,
    target: BattleSide,
    items: &BTreeMap<String, Item>,
    accuracy: u8,
) -> Result<u8, BattleTurnError> {
    let Some(item_id) = state.pokemon(target).item.as_deref() else {
        return Ok(accuracy);
    };
    let item = items
        .get(item_id)
        .ok_or_else(|| BattleTurnError::UnknownHeldItem {
            side: target,
            item_id: item_id.to_string(),
        })?;
    if item.held_effect != "HELD_BRIGHTPOWDER" {
        return Ok(accuracy);
    }
    if !(1..=255).contains(&item.parameter) {
        return Err(BattleTurnError::InvalidHeldItemParameter {
            side: target,
            item_id: item_id.to_string(),
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
        });
    }
    Ok(accuracy.saturating_sub(item.parameter as u8))
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

fn apply_end_turn_residual_status(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon(side);
    if pokemon.hp == 0 {
        return;
    }
    let Some(status) = pokemon.status.as_deref() else {
        return;
    };
    if !matches!(status, "POISON" | "BAD_POISON" | "BURN") {
        return;
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

fn apply_post_action_residual_damage(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    apply_end_turn_residual_status(state, side, events);
    if state.pokemon(side).hp == 0 {
        return;
    }
    apply_end_turn_leech_seed(state, side, events);
    if state.pokemon(side).hp == 0 {
        return;
    }
    apply_end_turn_nightmare(state, side, events);
    if state.pokemon(side).hp == 0 {
        return;
    }
    apply_end_turn_curse(state, side, events);
}

fn apply_end_turn_effects(
    state: &mut BattleCombatState,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    apply_end_turn_future_sight(state, items, rng, events)?;
    if either_active_pokemon_fainted(state) {
        return Ok(());
    }
    apply_end_turn_weather(state, events);
    if either_active_pokemon_fainted(state) {
        return Ok(());
    }
    apply_end_turn_trap(state, events);
    if either_active_pokemon_fainted(state) {
        return Ok(());
    }
    apply_end_turn_perish_song(state, events);
    if either_active_pokemon_fainted(state) {
        return Ok(());
    }
    clear_inactive_escape_traps(state, events);
    apply_end_turn_leftovers(state, items, events)?;
    apply_end_turn_mystery_berry(state, moves, items, events)?;
    apply_end_turn_defrost(state, rng, events);
    apply_end_turn_safeguard(state, events);
    apply_end_turn_screens(state, events);
    for side in between_turn_side_order(state.serial_connection_status) {
        apply_held_hp_healing(state, side, items, events)?;
        apply_held_status_healing(state, side, items, events)?;
    }
    apply_end_turn_encore(state, events);
    Ok(())
}

fn either_active_pokemon_fainted(state: &BattleCombatState) -> bool {
    state.player.hp == 0 || state.enemy.hp == 0
}

fn apply_end_turn_leech_seed(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    let Some(source) = leech_seed_source(state, side) else {
        return;
    };
    if state.pokemon(side).hp == 0 {
        return;
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

fn apply_end_turn_nightmare(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    let Some(source) = nightmare_source(state, side) else {
        return;
    };
    if state.pokemon(side).hp == 0 {
        return;
    }
    if state.pokemon(side).status.as_deref() != Some("SLEEP") {
        set_nightmare_source(state, side, None);
        events.push(BattleEvent::NightmareEnded { side, source });
        return;
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

fn apply_end_turn_curse(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) {
    let Some(source) = curse_source(state, side) else {
        return;
    };
    if state.pokemon(side).hp == 0 {
        return;
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

fn apply_end_turn_trap(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in between_turn_side_order(state.serial_connection_status) {
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
                move_name: trap.move_name.clone(),
            });
            continue;
        }
        if substitute_hp(state, side) != 0 {
            continue;
        }
        let turns_remaining = trap.turns_remaining.saturating_sub(1);
        if turns_remaining == 0 {
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
        {
            let trapped = state.pokemon_mut(side);
            trapped.hp = trapped.hp.saturating_sub(damage);
        }
        set_trap_state(
            state,
            side,
            Some(BattleTrapState {
                turns_remaining,
                ..trap.clone()
            }),
        );
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
        }
    }
}

fn clear_inactive_escape_traps(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in between_turn_side_order(state.serial_connection_status) {
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
    for side in between_turn_side_order(state.serial_connection_status) {
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

fn apply_end_turn_future_sight(
    state: &mut BattleCombatState,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    for side in between_turn_side_order(state.serial_connection_status) {
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
        events.push(BattleEvent::FutureSightLanded {
            side,
            source: queued.source,
            move_name: queued.move_name.clone(),
        });
        let damage_roll = crystal_damage_variation_roll(rng);
        let varied_damage = ((u32::from(queued.damage) * u32::from(damage_roll)) / 255)
            .max(1)
            .min(u32::from(u16::MAX)) as u16;
        if apply_substitute_damage(
            state,
            queued.source,
            &queued.move_name,
            varied_damage,
            events,
        )
        .is_some()
        {
            apply_rage_counter_increment(state, side, events)?;
            continue;
        }
        let hp_before = state.pokemon(side).hp;
        let mut damage = varied_damage.min(hp_before);
        if damage >= hp_before && hp_before > 1 && focus_band_survives(state, side, items, rng)? {
            let lethal_damage = damage;
            damage = hp_before - 1;
            events.push(BattleEvent::EnduredHit {
                side: queued.source,
                move_name: queued.move_name.clone(),
                target: side,
                raw_damage: lethal_damage,
                held_item: state.pokemon(side).item.clone(),
            });
        }
        state.pokemon_mut(side).hp = hp_before.saturating_sub(damage);
        let hp_after_damage = state.pokemon(side).hp;
        events.push(BattleEvent::FutureSightDamage {
            side,
            source: queued.source,
            move_name: queued.move_name.clone(),
            damage,
            hp_before,
            hp_after: hp_after_damage,
        });
        if damage > 0 {
            record_last_damage(
                state,
                side,
                BattleLastDamageState {
                    source: queued.source,
                    move_name: queued.move_name.clone(),
                    category: BattleDamageCategory::Special,
                    damage,
                },
            );
            apply_bide_damage_storage(state, queued.source, side, damage, events);
        }
        apply_rage_counter_increment(state, side, events)?;
        if state.pokemon(side).hp == 0 {
            events.push(BattleEvent::Fainted { side });
        }
    }
    Ok(())
}

fn apply_end_turn_safeguard(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in between_turn_side_order(state.serial_connection_status) {
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
    for side in between_turn_side_order(state.serial_connection_status) {
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

fn apply_end_turn_weather(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    if state.weather == Weather::None || state.weather_turns == 0 {
        return;
    }
    let weather = state.weather;
    let turns_remaining = state.weather_turns.saturating_sub(1);
    state.weather_turns = turns_remaining;
    if turns_remaining == 0 {
        state.weather = Weather::None;
        events.push(BattleEvent::WeatherEnded { weather });
        return;
    }
    events.push(BattleEvent::WeatherContinues {
        weather,
        turns_remaining,
    });
    if weather == Weather::Sandstorm {
        apply_end_turn_sandstorm_damage(state, events);
    }
}

fn apply_end_turn_sandstorm_damage(state: &mut BattleCombatState, events: &mut Vec<BattleEvent>) {
    for side in between_turn_side_order(state.serial_connection_status) {
        if state.pokemon(side).hp == 0 || pokemon_is_sandstorm_immune(state, side) {
            continue;
        }
        let pokemon = state.pokemon_mut(side);
        let hp_before = pokemon.hp;
        let damage = (pokemon.max_hp / 8).max(1);
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
        "BURN" => Some("BURN"),
        "FREEZE" => Some("FREEZE"),
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

/// Exact `MoveEffects` entries whose source command stream contains
/// `checkhit`. This boundary owns Dream Eater, Protect, draining-Substitute,
/// Lock-On, airborne, weather/X Accuracy, and ordinary accuracy ordering.
fn move_effect_uses_check_hit(effect: &str) -> bool {
    matches!(
        effect,
        "NORMAL_HIT"
            | "SLEEP"
            | "POISON_HIT"
            | "LEECH_HIT"
            | "BURN_HIT"
            | "FREEZE_HIT"
            | "PARALYZE_HIT"
            | "SELFDESTRUCT"
            | "DREAM_EATER"
            | "ALWAYS_HIT"
            | "ATTACK_DOWN"
            | "DEFENSE_DOWN"
            | "SPEED_DOWN"
            | "SPECIAL_ATTACK_DOWN"
            | "SPECIAL_DEFENSE_DOWN"
            | "ACCURACY_DOWN"
            | "EVASION_DOWN"
            | "RAMPAGE"
            | "FORCE_SWITCH"
            | "MULTI_HIT"
            | "FLINCH_HIT"
            | "TOXIC"
            | "PAY_DAY"
            | "TRI_ATTACK"
            | "RAZOR_WIND"
            | "SUPER_FANG"
            | "STATIC_DAMAGE"
            | "TRAP_TARGET"
            | "DOUBLE_HIT"
            | "JUMP_KICK"
            | "RECOIL_HIT"
            | "CONFUSE"
            | "ATTACK_DOWN_2"
            | "DEFENSE_DOWN_2"
            | "SPEED_DOWN_2"
            | "SPECIAL_ATTACK_DOWN_2"
            | "SPECIAL_DEFENSE_DOWN_2"
            | "ACCURACY_DOWN_2"
            | "EVASION_DOWN_2"
            | "POISON"
            | "PARALYZE"
            | "ATTACK_DOWN_HIT"
            | "DEFENSE_DOWN_HIT"
            | "SPEED_DOWN_HIT"
            | "SPECIAL_ATTACK_DOWN_HIT"
            | "SPECIAL_DEFENSE_DOWN_HIT"
            | "ACCURACY_DOWN_HIT"
            | "EVASION_DOWN_HIT"
            | "SKY_ATTACK"
            | "CONFUSE_HIT"
            | "POISON_MULTI_HIT"
            | "HYPER_BEAM"
            | "RAGE"
            | "MIMIC"
            | "LEECH_SEED"
            | "DISABLE"
            | "LEVEL_DAMAGE"
            | "PSYWAVE"
            | "ENCORE"
            | "PAIN_SPLIT"
            | "SNORE"
            | "CONVERSION2"
            | "LOCK_ON"
            | "REVERSAL"
            | "SPITE"
            | "FALSE_SWIPE"
            | "PRIORITY_HIT"
            | "TRIPLE_KICK"
            | "THIEF"
            | "FLAME_WHEEL"
            | "FORESIGHT"
            | "ROLLOUT"
            | "SWAGGER"
            | "FURY_CUTTER"
            | "ATTRACT"
            | "RETURN"
            | "PRESENT"
            | "FRUSTRATION"
            | "SACRED_FIRE"
            | "MAGNITUDE"
            | "PURSUIT"
            | "RAPID_SPIN"
            | "HIDDEN_POWER"
            | "DEFENSE_UP_HIT"
            | "ATTACK_UP_HIT"
            | "ALL_UP_HIT"
            | "SKULL_BASH"
            | "TWISTER"
            | "EARTHQUAKE"
            | "FUTURE_SIGHT"
            | "GUST"
            | "STOMP"
            | "SOLARBEAM"
            | "THUNDER"
            | "BEAT_UP"
            | "FLY"
    )
}

fn move_checks_accuracy_per_hit(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "MULTI_HIT" | "POISON_MULTI_HIT" | "TRIPLE_KICK" | "BEAT_UP"
    )
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
        return move_data.effect == "EARTHQUAKE"
            || matches!(move_data.name.as_str(), "FISSURE" | "MAGNITUDE");
    }
    matches!(
        move_data.name.as_str(),
        "GUST" | "WHIRLWIND" | "THUNDER" | "TWISTER"
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
    if move_data.power == 0 {
        return None;
    }
    let status = match move_data.effect.as_str() {
        "BURN_HIT" | "FLAME_WHEEL" | "SACRED_FIRE" => "BURN",
        "FREEZE_HIT" => "FREEZE",
        "POISON_HIT" | "POISON_MULTI_HIT" => "POISON",
        "PARALYZE_HIT" | "THUNDER" => "PARALYSIS",
        _ => return None,
    };
    Some((status, move_data.effect_chance.min(100)))
}

fn damaging_effect_script_has_pre_damage_effect_chance(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "POISON_HIT"
            | "BURN_HIT"
            | "FREEZE_HIT"
            | "PARALYZE_HIT"
            | "ATTACK_DOWN_HIT"
            | "DEFENSE_DOWN_HIT"
            | "SPEED_DOWN_HIT"
            | "SPECIAL_ATTACK_DOWN_HIT"
            | "SPECIAL_DEFENSE_DOWN_HIT"
            | "ACCURACY_DOWN_HIT"
            | "EVASION_DOWN_HIT"
            | "DEFENSE_UP_HIT"
            | "ATTACK_UP_HIT"
            | "ALL_UP_HIT"
            | "FLINCH_HIT"
            | "CONFUSE_HIT"
            | "SKY_ATTACK"
            | "SNORE"
            | "THIEF"
            | "FLAME_WHEEL"
            | "SACRED_FIRE"
            | "TWISTER"
            | "EARTHQUAKE"
            | "STOMP"
            | "THUNDER"
    )
}

fn secondary_confusion_effect(move_data: &Move) -> Option<u8> {
    if move_data.power == 0 {
        return None;
    }
    (move_data.effect == "CONFUSE_HIT").then_some(move_data.effect_chance.min(100))
}

fn secondary_flinch_effect(move_data: &Move) -> Option<u8> {
    if move_data.power == 0 {
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
            | "SPECIAL_ATTACK_DOWN_HIT"
            | "SPECIAL_DEFENSE_DOWN_HIT"
            | "ACCURACY_DOWN_HIT"
            | "EVASION_DOWN_HIT"
    )
}

fn roll_critical_hit(
    side: BattleSide,
    move_name: &str,
    attacker: &Pokemon,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
) -> Result<(bool, u8, u8), BattleTurnError> {
    // Crystal tallies critical-hit stages from the move table, Focus Energy,
    // species-specific items, and Scope Lens, then indexes the ROM's chance
    // table: 1/15, 1/8, 1/4, 1/3, and 1/2.
    let mut stage = 0usize;
    if attacker.focus_energy {
        stage += 1;
    }
    if matches!(
        move_name,
        "KARATE_CHOP"
            | "RAZOR_WIND"
            | "RAZOR_LEAF"
            | "CRABHAMMER"
            | "SLASH"
            | "AEROBLAST"
            | "CROSS_CHOP"
    ) {
        stage += 2;
    }
    if matches!(attacker.species.id.as_str(), "CHANSEY" | "FARFETCH_D") {
        if let Some(item_id) = attacker.item.as_deref() {
            let _item = items
                .get(item_id)
                .ok_or_else(|| BattleTurnError::UnknownHeldItem {
                    side,
                    item_id: item_id.to_string(),
                })?;
            let species_item = (attacker.species.id == "CHANSEY" && item_id == "LUCKY_PUNCH")
                || (attacker.species.id == "FARFETCH_D" && item_id == "STICK");
            if species_item {
                stage += 2;
            }
        }
    }
    if let Some(item_id) = attacker.item.as_deref() {
        let item = items
            .get(item_id)
            .ok_or_else(|| BattleTurnError::UnknownHeldItem {
                side,
                item_id: item_id.to_string(),
            })?;
        if item.held_effect == "HELD_CRITICAL_UP" {
            stage += 1;
        }
    }
    let threshold = [17u8, 32, 64, 86, 128][stage.min(4)];
    let roll = rng.battle_random_byte();
    Ok((roll < threshold, roll, threshold))
}

fn sample_multi_hit_count(rng: &mut Random) -> (u8, u8) {
    let first = rng.battle_random_byte() & 0x03;
    if first < 2 {
        return (first + 2, first);
    }
    let second = rng.battle_random_byte() & 0x03;
    (second + 2, 4 + second)
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
    if matches!(move_data.effect.as_str(), "EARTHQUAKE" | "MAGNITUDE")
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
            let roll = rng.battle_random_byte();
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
            let level = attacker.level.max(1);
            let upper = level.saturating_add(level / 2);
            let damage = loop {
                let roll = rng.battle_random_byte();
                if roll != 0 && roll < upper {
                    break roll;
                }
            };
            Some(u16::from(damage))
        }
        _ => None,
    }
}

fn is_fixed_damage_effect(move_data: &Move) -> bool {
    matches!(
        move_data.effect.as_str(),
        "STATIC_DAMAGE" | "LEVEL_DAMAGE" | "SUPER_FANG" | "PSYWAVE"
    )
}

fn apply_counter_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    moves: &BTreeMap<String, Move>,
    type_effectiveness: &TypeEffectivenessTable,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
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
    let countered_move =
        moves
            .get(&last_damage.move_name)
            .ok_or_else(|| BattleTurnError::MissingMoveData {
                side,
                move_name: last_damage.move_name.clone(),
            })?;
    if counter_effect(countered_move).is_some() {
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
    let raw_damage = last_damage.damage.saturating_mul(2);
    if raw_damage == 0 {
        events.push(BattleEvent::NoEffect {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    if apply_substitute_damage(state, side, move_name, raw_damage, events).is_some() {
        apply_rage_counter_increment(state, target, events)?;
        return Ok(());
    }
    let mut damage = raw_damage.min(defender_hp_before);
    if endure_active(state, target) && defender_hp_before > 1 {
        let endured_damage = damage;
        damage = damage.min(defender_hp_before - 1);
        if endured_damage != damage {
            events.push(BattleEvent::EnduredHit {
                side,
                move_name: move_name.to_string(),
                target,
                raw_damage: endured_damage,
                held_item: None,
            });
        }
    }
    if damage >= defender_hp_before
        && defender_hp_before > 1
        && focus_band_survives(state, target, items, rng)?
    {
        let lethal_damage = damage;
        damage = defender_hp_before - 1;
        events.push(BattleEvent::EnduredHit {
            side,
            move_name: move_name.to_string(),
            target,
            raw_damage: lethal_damage,
            held_item: state.pokemon(target).item.clone(),
        });
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
    if damage != 0 {
        record_last_damage(
            state,
            target,
            BattleLastDamageState {
                source: side,
                move_name: move_name.to_string(),
                category: required_category,
                damage,
            },
        );
        apply_rage_counter_increment(state, target, events)?;
        apply_bide_damage_storage(state, side, target, damage, events);
    }
    apply_direct_damage_faint_events(state, side, target, move_name, events);
    if state.pokemon(target).hp != 0 && state.pokemon(side).hp != 0 {
        apply_kings_rock_flinch(
            state, side, move_name, move_data, damage, items, rng, events,
        )?;
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

fn move_blocked_by_substitute(
    state: &BattleCombatState,
    side: BattleSide,
    move_name: &str,
    target: BattleSide,
    events: &mut Vec<BattleEvent>,
) -> bool {
    if substitute_hp(state, target) == 0 {
        return false;
    }
    events.push(BattleEvent::SubstituteBlocked {
        side,
        move_name: move_name.to_string(),
        target,
    });
    true
}

fn apply_post_damage_hp_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    damage: u16,
    pre_damage_effect_chance: Option<EffectChanceResult>,
    events: &mut Vec<BattleEvent>,
) {
    if damage == 0 {
        return;
    }
    match move_data.effect.as_str() {
        "DREAM_EATER" | "LEECH_HIT" => apply_drain_effect(state, side, move_name, damage, events),
        "RECOIL_HIT" => apply_recoil_effect(state, side, move_name, damage, events),
        "PAY_DAY" => apply_pay_day_effect(state, side, move_name, events),
        "THIEF"
            if pre_damage_effect_chance
                .expect("Thief effect script must sample before damage")
                .succeeds =>
        {
            apply_thief_effect(state, side, move_name, events)
        }
        _ => {}
    }
}

fn apply_post_damage_stat_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    damage: u16,
    pre_damage_effect_chance: Option<EffectChanceResult>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    if damage == 0 {
        return Ok(false);
    }
    match move_data.effect.as_str() {
        "ALL_UP_HIT" => {
            let effect_chance =
                pre_damage_effect_chance.expect("AllUpHit effect script must sample before damage");
            if !effect_chance.succeeds {
                return Ok(true);
            }
            for stat in [
                Stat::Attack,
                Stat::Defense,
                Stat::Speed,
                Stat::SpecialAttack,
                Stat::SpecialDefense,
            ] {
                let stage = *state
                    .pokemon(side)
                    .stat_boosts
                    .get(&stat)
                    .ok_or(BattleTurnError::MissingStatStage { side, stat })?;
                if stage >= 6 {
                    continue;
                }
                apply_stat_stage_delta(state, side, move_name, stat, 1, events)?;
            }
            Ok(true)
        }
        _ if secondary_stat_hit_effect(move_data) => {
            // DefenseDownHit uniquely executes a second effectchance after
            // applydamage; the latter result overwrites the first one.
            let effect_chance = if move_data.effect == "DEFENSE_DOWN_HIT" {
                sample_effect_chance_against_target(state, side, move_data, rng)
            } else {
                pre_damage_effect_chance
                    .expect("damaging stat effect script must sample before damage")
            };
            apply_secondary_stat_stage_effect(
                state,
                side,
                move_name,
                move_data,
                effect_chance,
                rng,
                events,
            )?;
            Ok(true)
        }
        "RAPID_SPIN" => {
            apply_rapid_spin_effect(state, side, move_name, events);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn secondary_effect_roll(chance_percent: u8, rng: &mut Random) -> (bool, Option<u8>) {
    let threshold = (u16::from(chance_percent.min(100)) * 255 / 100) as u8;
    let roll = rng.battle_random_byte();
    (roll < threshold, Some(roll))
}

fn sample_effect_chance_against_target(
    state: &BattleCombatState,
    side: BattleSide,
    move_data: &Move,
    rng: &mut Random,
) -> EffectChanceResult {
    let chance_percent = move_data.effect_chance.min(100);
    if substitute_hp(state, side.other()) != 0 {
        return EffectChanceResult {
            chance_percent,
            succeeds: false,
            roll: None,
        };
    }
    let (succeeds, roll) = secondary_effect_roll(chance_percent, rng);
    EffectChanceResult {
        chance_percent,
        succeeds,
        roll,
    }
}

fn sample_pre_damage_effect_chance(
    state: &BattleCombatState,
    side: BattleSide,
    move_data: &Move,
    rng: &mut Random,
) -> Option<EffectChanceResult> {
    damaging_effect_script_has_pre_damage_effect_chance(move_data)
        .then(|| sample_effect_chance_against_target(state, side, move_data, rng))
}

fn crystal_damage_variation_roll(rng: &mut Random) -> u8 {
    loop {
        let roll = rng.battle_random_byte().rotate_right(1);
        if roll >= 217 {
            return roll;
        }
    }
}

fn apply_rapid_spin_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let trap_move = trap_state(state, side).map(|trap| trap.move_name.clone());
    let cleared_trap = trap_move.is_some();
    let cleared_leech_seed = leech_seed_source(state, side).is_some();
    let cleared_spikes = spikes_state(state, side);
    clear_trap_state(state, side);
    set_leech_seed_source(state, side, None);
    set_spikes_state(state, side, false);
    events.push(BattleEvent::RapidSpinCleared {
        side,
        move_name: move_name.to_string(),
        cleared_trap,
        trap_move,
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
    let roll = rng.battle_random_byte() & 1;
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

fn apply_rage_counter_increment(
    state: &mut BattleCombatState,
    side: BattleSide,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if rage_active(state, side) && state.pokemon(side).hp != 0 {
        let counter = rage_counter(state, side).saturating_add(1);
        set_rage_counter(state, side, counter);
        events.push(BattleEvent::RageBuilding { side, counter });
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
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let (Some(stat), Some(amount)) = (move_data.stat, move_data.amount) else {
        return Ok(false);
    };
    if amount == 0 {
        return Ok(false);
    }
    let target = stat_effect_target(side, amount);
    if target != side && move_blocked_by_substitute(state, side, move_name, target, events) {
        return Ok(true);
    }
    let target_stage = *state
        .pokemon(target)
        .stat_boosts
        .get(&stat)
        .ok_or(BattleTurnError::MissingStatStage { side: target, stat })?;
    if amount < 0
        && side == BattleSide::Enemy
        && state.enemy_effect_ai_random_fail
        && !state.player_lock_on_target
        && !(move_data.power > 0 && stat == Stat::Accuracy)
        && !mist_active(state, target)
        && target_stage > -6
        && rng.battle_random_byte() < 64
    {
        events.push(BattleEvent::StatStageFailed {
            side,
            move_name: move_name.to_string(),
            target,
            stat,
        });
        return Ok(true);
    }
    apply_stat_stage_delta(state, side, move_name, stat, amount, events)?;
    if move_name == "MINIMIZE"
        && target == side
        && *state.pokemon(side).stat_boosts.get(&Stat::Evasion).ok_or(
            BattleTurnError::MissingStatStage {
                side,
                stat: Stat::Evasion,
            },
        )? > target_stage
    {
        set_minimized_state(state, side, true);
    }
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
    if amount < 0 && target != side && mist_active(state, target) {
        events.push(BattleEvent::MistProtected {
            side,
            move_name: move_name.to_string(),
            target,
            stat,
            amount,
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

fn mist_active(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_mist_active,
        BattleSide::Enemy => state.enemy_mist_active,
    }
}

fn set_mist_active(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_mist_active = active,
        BattleSide::Enemy => state.enemy_mist_active = active,
    }
}

fn minimized_state(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_minimized,
        BattleSide::Enemy => state.enemy_minimized,
    }
}

fn set_minimized_state(state: &mut BattleCombatState, side: BattleSide, minimized: bool) {
    match side {
        BattleSide::Player => state.player_minimized = minimized,
        BattleSide::Enemy => state.enemy_minimized = minimized,
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
    if !active {
        set_rage_counter(state, side, 0);
    }
}

fn rage_counter(state: &BattleCombatState, side: BattleSide) -> u8 {
    match side {
        BattleSide::Player => state.player_rage_counter,
        BattleSide::Enemy => state.enemy_rage_counter,
    }
}

fn set_rage_counter(state: &mut BattleCombatState, side: BattleSide, counter: u8) {
    match side {
        BattleSide::Player => state.player_rage_counter = counter,
        BattleSide::Enemy => state.enemy_rage_counter = counter,
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

fn end_opponent_action_volatiles(state: &mut BattleCombatState, acting_side: BattleSide) {
    let opponent = acting_side.other();
    set_protect_active(state, opponent, false);
    set_endure_active(state, opponent, false);
    set_destiny_bond_active(state, opponent, false);
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
    clear_trap_state(state, side);
    clear_traps_sourced_by(state, side);
    clear_escape_trap_state(state, side);
    clear_escape_traps_sourced_by(state, side);
    set_lock_on_target_state(state, side, false);
    set_lock_on_target_state(state, side.other(), false);
    set_x_accuracy_active(state, side, false);
    set_attracted_by_state(state, side, None);
    clear_attracted_by_source(state, side);
    set_recharge_move_state(state, side, None);
    set_airborne_move_state(state, side, None);
    set_charging_move_state(state, side, None);
    set_destiny_bond_active(state, side, false);
    set_mist_active(state, side, false);
    set_safeguard_turns(state, side, 0);
    set_minimized_state(state, side, false);
    set_toxic_turns(state, side, 0);
    set_last_move(state, side, None);
    if side == BattleSide::Player {
        state.player_used_moves.clear();
        state.player_turns_taken = 0;
    } else {
        state.enemy_turns_taken = 0;
    }
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
    if pokemon.status.as_deref() == Some("BAD_POISON") {
        pokemon.status = Some("POISON".to_string());
    }
    pokemon.flinching = false;
    pokemon.confusion_turns = 0;
    pokemon.perish_song_turns = 0;
    pokemon.focus_energy = false;
    pokemon.rampage_turns = 0;
}

fn clear_baton_pass_non_passable_conditions(state: &mut BattleCombatState, side: BattleSide) {
    clear_trap_state(state, BattleSide::Player);
    clear_trap_state(state, BattleSide::Enemy);
    clear_escape_traps_sourced_by(state, side);
    set_attracted_by_state(state, BattleSide::Player, None);
    set_attracted_by_state(state, BattleSide::Enemy, None);
    set_recharge_move_state(state, side, None);
    set_airborne_move_state(state, side, None);
    set_charging_move_state(state, side, None);
    set_destiny_bond_active(state, side, false);
    set_last_move(state, side, None);
    clear_encore_state(state, side);
    clear_disable_state(state, side);
    set_protect_active(state, side, false);
    set_endure_active(state, side, false);
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
    effect_chance: EffectChanceResult,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let (Some(stat), Some(amount)) = (move_data.stat, move_data.amount) else {
        return Ok(false);
    };
    if move_data.power == 0 || amount == 0 {
        return Ok(false);
    }
    if !roll_secondary_stat_stage_effect(side, move_name, move_data, effect_chance, events)? {
        return Ok(true);
    }
    let target = stat_effect_target(side, amount);
    if target != side && mist_active(state, target) {
        return Ok(true);
    }
    let target_stage = *state
        .pokemon(target)
        .stat_boosts
        .get(&stat)
        .ok_or(BattleTurnError::MissingStatStage { side: target, stat })?;
    if (amount > 0 && target_stage >= 6) || (amount < 0 && target_stage <= -6) {
        return Ok(true);
    }
    if amount < 0
        && side == BattleSide::Enemy
        && state.enemy_effect_ai_random_fail
        && !state.player_lock_on_target
        && !(stat == Stat::Accuracy)
        && target_stage > -6
        && rng.battle_random_byte() < 64
    {
        return Ok(true);
    }
    if target != side && substitute_hp(state, target) != 0 {
        return Ok(true);
    }
    apply_stat_stage_delta_to_target(state, side, move_name, target, stat, amount, events)?;
    Ok(true)
}

fn roll_secondary_stat_stage_effect(
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    effect_chance: EffectChanceResult,
    events: &mut Vec<BattleEvent>,
) -> Result<bool, BattleTurnError> {
    let (Some(stat), Some(amount)) = (move_data.stat, move_data.amount) else {
        return Ok(false);
    };
    if amount == 0 {
        return Ok(false);
    }
    if !effect_chance.succeeds {
        if let Some(roll) = effect_chance.roll {
            events.push(BattleEvent::SecondaryStatStageMissed {
                side,
                move_name: move_name.to_string(),
                target: stat_effect_target(side, amount),
                stat,
                amount,
                chance_percent: effect_chance.chance_percent,
                roll,
            });
        }
        return Ok(false);
    }
    Ok(true)
}

fn apply_secondary_status_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    status: &str,
    effect_chance: EffectChanceResult,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if !effect_chance.succeeds {
        if let Some(roll) = effect_chance.roll {
            events.push(BattleEvent::SecondaryStatusMissed {
                side,
                move_name: move_name.to_string(),
                target,
                status: status.to_string(),
                chance_percent: effect_chance.chance_percent,
                roll,
            });
        }
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
    let target = side.other();
    let (succeeds, chance_roll) = secondary_effect_roll(chance_percent, rng);
    if !succeeds {
        events.push(BattleEvent::SecondaryStatusMissed {
            side,
            move_name: move_name.to_string(),
            target,
            status: "TRI_ATTACK".to_string(),
            chance_percent,
            roll: chance_roll.expect("effectchance always consumes a battle byte"),
        });
        return;
    }

    let status_roll = loop {
        let status_roll = (rng.battle_random_byte() >> 4) & 0x03;
        if status_roll != 0 {
            break status_roll;
        }
    };
    let status = match status_roll {
        1 => "PARALYSIS",
        2 => "FREEZE",
        3 => "BURN",
        _ => unreachable!("Tri Attack rejects the zero masked status roll"),
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
    if status == "BURN" && state.pokemon(target).status.as_deref() == Some("FREEZE") {
        state.pokemon_mut(target).status = None;
        events.push(BattleEvent::StatusHealed {
            side,
            move_name: move_name.to_string(),
            target,
            status_before: "FREEZE".to_string(),
        });
        return;
    }
    // The damaging target commands return silently for an existing status or
    // an immune type. These gates precede Safeguard in the source stream.
    if state.pokemon(target).status.is_some() {
        return;
    }
    let target_types = effective_pokemon_types(state, target);
    if pokemon_is_status_immune(&target_types, status) {
        return;
    }
    // FreezeTarget returns silently in sunlight after its chance byte has
    // already been sampled by the effect script.
    if status == "FREEZE" && state.weather == Weather::Sun {
        return;
    }
    if move_blocked_by_safeguard(state, side, move_name, target, status, events) {
        return;
    }
    let sleep_turn_mask = state.sleep_turn_mask;
    let applied = apply_status_to_target(
        state.pokemon_mut(target),
        &target_types,
        side,
        move_name,
        target,
        status,
        sleep_turn_mask,
        rng,
        events,
    );
    if applied && status == "FREEZE" {
        set_recharge_move_state(state, target, None);
    }
}

fn status_applied_since(events: &[BattleEvent], start: usize, target: BattleSide) -> bool {
    events[start..].iter().any(|event| {
        matches!(
            event,
            BattleEvent::StatusApplied {
                target: applied_target,
                ..
            } if *applied_target == target
        )
    })
}

fn confusion_applied_since(events: &[BattleEvent], start: usize, target: BattleSide) -> bool {
    events[start..].iter().any(|event| {
        matches!(
            event,
            BattleEvent::ConfusionApplied {
                target: applied_target,
                ..
            } if *applied_target == target
        )
    })
}

fn apply_secondary_flinch_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    effect_chance: EffectChanceResult,
    target_already_acted: bool,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if !effect_chance.succeeds {
        if let Some(roll) = effect_chance.roll {
            events.push(BattleEvent::SecondaryFlinchMissed {
                side,
                move_name: move_name.to_string(),
                target,
                chance_percent: effect_chance.chance_percent,
                roll,
            });
        }
        return;
    }
    if target_already_acted
        || substitute_hp(state, target) != 0
        || matches!(
            state.pokemon(target).status.as_deref(),
            Some("SLEEP" | "FREEZE")
        )
    {
        return;
    }

    set_recharge_move_state(state, target, None);
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
    if move_blocked_by_substitute(state, side, move_name, target, events) {
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
    pokemon.confusion_turns = 2 + u16::from(rng.battle_random_byte() & 3);
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
    pokemon.confusion_turns = 2 + u16::from(rng.battle_random_byte() & 3);
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
    effect_chance: EffectChanceResult,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if !effect_chance.succeeds {
        if let Some(roll) = effect_chance.roll {
            events.push(BattleEvent::SecondaryConfusionMissed {
                side,
                move_name: move_name.to_string(),
                target,
                chance_percent: effect_chance.chance_percent,
                roll,
            });
        }
        return;
    }
    if move_blocked_by_safeguard(state, side, move_name, target, "CONFUSION", events) {
        return;
    }
    if substitute_hp(state, target) != 0 || state.pokemon(target).confusion_turns != 0 {
        return;
    }
    let turns = 2 + u16::from(rng.battle_random_byte() & 3);
    state.pokemon_mut(target).confusion_turns = turns;
    events.push(BattleEvent::ConfusionApplied {
        side,
        move_name: move_name.to_string(),
        target,
        turns,
    });
}

fn apply_direct_heal_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    events: &mut Vec<BattleEvent>,
) {
    let weather = state.weather;
    let time_of_day = state.time_of_day;
    let link_battle = state.link_battle;
    if move_data.name == "REST" {
        apply_rest_heal_effect(state, side, move_name, events);
        return;
    }
    let pokemon = state.pokemon_mut(side);
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
    let animation_param = time_based_heal_param(move_data, time_of_day, weather, link_battle);
    let amount = direct_heal_amount(pokemon.max_hp, move_data, animation_param)
        .min(pokemon.max_hp - pokemon.hp);
    pokemon.hp += amount;
    events.push(BattleEvent::HealApplied {
        side,
        move_name: move_name.to_string(),
        hp_before,
        hp_after: pokemon.hp,
        amount,
        animation_param,
    });
}

fn apply_heal_bell_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    _move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let status_before = {
        let pokemon = state.pokemon_mut(side);
        let status_before = pokemon.status.take();
        pokemon.sleep_turns = 0;
        status_before
    };
    let party = match side {
        BattleSide::Player => &mut state.player_party,
        BattleSide::Enemy => &mut state.enemy_party,
    };
    for pokemon in party {
        pokemon.status = None;
        pokemon.sleep_turns = 0;
    }
    set_nightmare_source(state, side, None);
    events.push(BattleEvent::HealBellChimed {
        side,
        active_status_before: status_before,
    });
}

fn apply_pain_split_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
    if move_blocked_by_substitute(state, side, move_name, target, events) {
        return;
    }
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
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let pokemon = state.pokemon_mut(side);
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
    pokemon.hp = pokemon.max_hp;
    pokemon.status = Some("SLEEP".to_string());
    pokemon.sleep_turns = 2;
    events.push(BattleEvent::HealApplied {
        side,
        move_name: move_name.to_string(),
        hp_before,
        hp_after: pokemon.hp,
        amount: pokemon.max_hp - hp_before,
        animation_param: 0,
    });
    events.push(BattleEvent::StatusApplied {
        side,
        move_name: move_name.to_string(),
        target: side,
        status: "SLEEP".to_string(),
    });
    set_nightmare_source(state, side, None);
}

fn time_based_heal_param(
    move_data: &Move,
    time_of_day: TimeOfDay,
    weather: Weather,
    link_battle: bool,
) -> u8 {
    if !matches!(
        move_data.effect.as_str(),
        "MOONLIGHT" | "MORNING_SUN" | "SYNTHESIS"
    ) {
        return 2;
    }
    let matching_time = match move_data.effect.as_str() {
        "MORNING_SUN" => time_of_day == TimeOfDay::Morning,
        "SYNTHESIS" => time_of_day == TimeOfDay::Day,
        "MOONLIGHT" => time_of_day == TimeOfDay::Night,
        _ => unreachable!(),
    };
    let mut multiplier_index = if link_battle || matching_time {
        2_u8
    } else {
        1_u8
    };
    match weather {
        Weather::Sun => multiplier_index += 1,
        Weather::Rain | Weather::Sandstorm => multiplier_index -= 1,
        Weather::None => {}
    }
    multiplier_index
}

fn direct_heal_amount(max_hp: u16, move_data: &Move, animation_param: u8) -> u16 {
    match move_data.effect.as_str() {
        "MOONLIGHT" | "MORNING_SUN" | "SYNTHESIS" => match animation_param {
            0 => (max_hp / 8).max(1),
            1 => (max_hp / 4).max(1),
            2 => (max_hp / 2).max(1),
            3 => max_hp,
            _ => unreachable!("time-based heal parameter is a two-bit multiplier index"),
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
    let player_count = state.player.perish_song_turns;
    let enemy_count = state.enemy.perish_song_turns;
    if player_count != 0 && enemy_count != 0 {
        for (target, turns_remaining) in [
            (BattleSide::Player, player_count),
            (BattleSide::Enemy, enemy_count),
        ] {
            events.push(BattleEvent::PerishSongFailed {
                side,
                move_name: move_name.to_string(),
                target,
                turns_remaining,
            });
        }
        return;
    }

    for target in [BattleSide::Player, BattleSide::Enemy] {
        let pokemon = state.pokemon_mut(target);
        if pokemon.perish_song_turns != 0 {
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
        if substitute_hp(state, target) > 0 || curse_source(state, target).is_some() {
            events.push(BattleEvent::CurseFailed {
                side,
                move_name: move_name.to_string(),
                target,
            });
            return Ok(());
        }
        let hp_before = state.pokemon(side).hp;
        let hp_cost = (state.pokemon(side).max_hp / 2).min(hp_before);
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
    if mist_active(state, side) {
        events.push(BattleEvent::MistFailed {
            side,
            move_name: move_name.to_string(),
        });
        return;
    }
    set_mist_active(state, side, true);
    events.push(BattleEvent::MistApplied {
        side,
        move_name: move_name.to_string(),
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
    if state.pokemon(side).status.as_deref() != Some("SLEEP") {
        events.push(BattleEvent::SleepTalkFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let disabled_move = disable_state(state, side).map(|disabled| disabled.move_name.as_str());
    let candidates: Vec<(usize, String)> = battle_moves(state, side)
        .iter()
        .enumerate()
        .filter(|(_, learned)| {
            learned.name != move_name
                && disabled_move != Some(learned.name.as_str())
                && moves.get(&learned.name).is_some_and(|move_data| {
                    !matches!(
                        move_data.effect.as_str(),
                        "SKULL_BASH" | "RAZOR_WIND" | "SKY_ATTACK" | "SOLARBEAM" | "FLY" | "BIDE"
                    )
                })
        })
        .map(|(slot, learned)| (slot, learned.name.clone()))
        .collect();
    if candidates.is_empty() {
        events.push(BattleEvent::SleepTalkFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let (roll, selected_slot, selected_move) = loop {
        let roll = rng.battle_random_byte() & 0x03;
        let Some((selected_slot, selected_move)) = battle_moves(state, side)
            .get(usize::from(roll))
            .map(|learned| (usize::from(roll), learned.name.clone()))
        else {
            continue;
        };
        if candidates
            .iter()
            .any(|(slot, candidate)| *slot == selected_slot && candidate == &selected_move)
        {
            break (roll, selected_slot, selected_move);
        }
    };
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
    const METRONOME_EXCEPTIONS: [&str; 12] = [
        "METRONOME",
        "STRUGGLE",
        "SKETCH",
        "MIMIC",
        "COUNTER",
        "MIRROR_COAT",
        "PROTECT",
        "DETECT",
        "ENDURE",
        "DESTINY_BOND",
        "SLEEP_TALK",
        "THIEF",
    ];
    let user_moves: std::collections::BTreeSet<&str> = battle_moves(state, side)
        .iter()
        .map(|learned| learned.name.as_str())
        .collect();
    let candidates: Vec<&str> = moves
        .keys()
        .map(String::as_str)
        .filter(|candidate| {
            !METRONOME_EXCEPTIONS.contains(candidate) && !user_moves.contains(candidate)
        })
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
    if move_blocked_by_substitute(state, side, move_name, target, events) {
        return Ok(());
    }
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
    for (slot, learned_move) in battle_moves(state, side).iter().enumerate() {
        let move_data =
            moves
                .get(&learned_move.name)
                .ok_or_else(|| BattleTurnError::MissingMoveData {
                    side,
                    move_name: learned_move.name.clone(),
                })?;
        if move_data.move_type == "CURSE_TYPE" || current_types.contains(&move_data.move_type) {
            continue;
        }
        candidates.push((slot, learned_move.name.clone(), move_data.move_type.clone()));
    }
    if candidates.is_empty() {
        events.push(BattleEvent::ConversionFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let (roll, selected_move, new_type) = loop {
        let roll = rng.battle_random_byte() & 0x03;
        if let Some((_, selected_move, new_type)) = candidates
            .iter()
            .find(|(slot, _, _)| *slot == usize::from(roll))
        {
            break (roll, selected_move.clone(), new_type.clone());
        }
    };
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
    if source_move.move_type == "CURSE_TYPE" {
        events.push(BattleEvent::Conversion2Failed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let mut candidates = Vec::new();
    for slot in 0..32u8 {
        let Some(candidate_type) = conversion2_type_slot(slot) else {
            continue;
        };
        // BIRD occupies a sampled numeric slot but has no matchup-table
        // entries, so BattleCheckTypeMatchup leaves it neutral and rejects it.
        let multiplier = if candidate_type == "BIRD" {
            TypeMultiplier::one()
        } else {
            // The ROM matchup table records only non-neutral pairs. Conversion2
            // probes that sparse table directly; absence is neutral rather than
            // malformed battle data.
            type_effectiveness
                .matchups
                .get(&source_move.move_type)
                .and_then(|defenders| defenders.get(&candidate_type))
                .copied()
                .unwrap_or_else(TypeMultiplier::one)
        };
        if multiplier.numerator == 0 || multiplier.numerator < multiplier.denominator {
            candidates.push((slot, candidate_type));
        }
    }
    if candidates.is_empty() {
        events.push(BattleEvent::Conversion2Failed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let (roll, new_type) = loop {
        let roll = rng.battle_random_byte() & 0x1f;
        if let Some((_, candidate_type)) = candidates.iter().find(|(slot, _)| *slot == roll) {
            break (roll, candidate_type.clone());
        }
    };
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

fn conversion2_type_slot(slot: u8) -> Option<PokemonType> {
    Some(
        match slot {
            0 => "NORMAL",
            1 => "FIGHTING",
            2 => "FLYING",
            3 => "POISON",
            4 => "GROUND",
            5 => "ROCK",
            6 => "BIRD",
            7 => "BUG",
            8 => "GHOST",
            9 => "STEEL",
            20 => "FIRE",
            21 => "WATER",
            22 => "GRASS",
            23 => "ELECTRIC",
            24 => "PSYCHIC_TYPE",
            25 => "ICE",
            26 => "DRAGON",
            27 => "DARK",
            _ => return None,
        }
        .to_string(),
    )
}

enum BideAdvance {
    Handled,
    Release { stored_damage: u16 },
}

fn advance_bide_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> BideAdvance {
    let turns_remaining = bide_turns(state, side);
    if turns_remaining == 0 {
        let roll = rng.battle_random_byte();
        let turns = (roll & 1) + 2;
        set_bide_turns(state, side, turns);
        set_bide_damage(state, side, 0);
        events.push(BattleEvent::BideStarted {
            side,
            move_name: move_name.to_string(),
            turns,
            roll,
        });
        return BideAdvance::Handled;
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
        return BideAdvance::Handled;
    }

    let stored_damage = bide_damage(state, side);
    reset_bide_state(state, side);
    events.push(BattleEvent::BideUnleashed {
        side,
        move_name: move_name.to_string(),
        stored_damage,
    });
    BideAdvance::Release { stored_damage }
}

fn apply_bide_release_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    move_data: &Move,
    stored_damage: u16,
    type_categories: &TypeCategories,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    if stored_damage == 0 {
        events.push(BattleEvent::BideFailed {
            side,
            move_name: move_name.to_string(),
        });
        return Ok(());
    }
    let target = side.other();
    let target_hp_before = state.pokemon(target).hp;
    let raw_damage = stored_damage.saturating_mul(2);
    if apply_substitute_damage(state, side, move_name, raw_damage, events).is_some() {
        events.push(BattleEvent::BideReleased {
            side,
            move_name: move_name.to_string(),
            target,
            stored_damage,
            damage: 0,
            target_hp_before,
            target_hp_after: target_hp_before,
        });
        apply_rage_counter_increment(state, target, events)?;
        if state.pokemon(target).hp != 0 && state.pokemon(side).hp != 0 {
            apply_kings_rock_flinch(
                state, side, move_name, move_data, raw_damage, items, rng, events,
            )?;
        }
        return Ok(());
    }

    let mut damage = raw_damage.min(target_hp_before);
    if endure_active(state, target) && target_hp_before > 1 {
        let lethal_damage = damage;
        damage = damage.min(target_hp_before - 1);
        if damage != lethal_damage {
            events.push(BattleEvent::EnduredHit {
                side,
                move_name: move_name.to_string(),
                target,
                raw_damage: lethal_damage,
                held_item: None,
            });
        }
    }
    if damage >= target_hp_before
        && target_hp_before > 1
        && focus_band_survives(state, target, items, rng)?
    {
        let lethal_damage = damage;
        damage = target_hp_before - 1;
        events.push(BattleEvent::EnduredHit {
            side,
            move_name: move_name.to_string(),
            target,
            raw_damage: lethal_damage,
            held_item: state.pokemon(target).item.clone(),
        });
    }
    state.pokemon_mut(target).hp = target_hp_before.saturating_sub(damage);
    let target_hp_after = state.pokemon(target).hp;
    events.push(BattleEvent::BideReleased {
        side,
        move_name: move_name.to_string(),
        target,
        stored_damage,
        damage,
        target_hp_before,
        target_hp_after,
    });
    if damage != 0 {
        record_last_damage(
            state,
            target,
            BattleLastDamageState {
                source: side,
                move_name: move_name.to_string(),
                category: damage_category(type_categories, move_data)?,
                damage,
            },
        );
        apply_rage_counter_increment(state, target, events)?;
        apply_bide_damage_storage(state, side, target, damage, events);
    }
    apply_direct_damage_faint_events(state, side, target, move_name, events);
    if state.pokemon(target).hp != 0 && state.pokemon(side).hp != 0 {
        apply_kings_rock_flinch(
            state, side, move_name, move_data, damage, items, rng, events,
        )?;
    }
    Ok(())
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
    let encored_move_is_eligible =
        !matches!(encored_move.as_str(), "STRUGGLE" | "ENCORE" | "MIRROR_MOVE")
            && battle_moves(state, target)
                .iter()
                .find(|learned| learned.name == encored_move)
                .is_some_and(|learned| learned.current_pp != 0);
    if !encored_move_is_eligible || encore_state(state, target).is_some() {
        events.push(BattleEvent::EncoreFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return;
    }
    let roll = rng.battle_random_byte() & 3;
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
    if move_blocked_by_substitute(state, side, move_name, target, events) {
        return;
    }
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
    if move_blocked_by_substitute(state, side, move_name, target, events) {
        return;
    }
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
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    force_switch_ends_battle: bool,
    rng: &mut Random,
    acted_before: &[BattleSide],
    events: &mut Vec<BattleEvent>,
) -> Result<(), BattleTurnError> {
    let target = side.other();
    if state.force_switch_blocked {
        events.push(BattleEvent::ForceSwitchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    }
    if force_switch_ends_battle {
        let attacker_level = state.pokemon(side).level;
        let target_level = state.pokemon(target).level;
        let succeeds = if attacker_level >= target_level {
            true
        } else {
            let upper = u16::from(attacker_level) + u16::from(target_level) + 1;
            let roll = loop {
                let roll = u16::from(rng.battle_random_byte());
                if roll < upper {
                    break roll;
                }
            };
            // BattleCommand_ForceSwitch keeps the user's level in b and
            // shifts that register twice after rejection sampling.
            roll >= u16::from(attacker_level / 4)
        };
        if !succeeds {
            events.push(BattleEvent::ForceSwitchFailed {
                side,
                move_name: move_name.to_string(),
                target,
            });
            return Ok(());
        }
        events.push(BattleEvent::ForceSwitchApplied {
            side,
            move_name: move_name.to_string(),
            target,
        });
        events.push(BattleEvent::Fled { side: target });
        return Ok(());
    }

    let party = match target {
        BattleSide::Player => &state.player_party,
        BattleSide::Enemy => &state.enemy_party,
    };
    let active_index = match target {
        BattleSide::Player => state.player_party_index,
        BattleSide::Enemy => state.enemy_party_index,
    };
    let alive_reserves = party
        .iter()
        .enumerate()
        .filter_map(|(index, pokemon)| (index != active_index && pokemon.hp > 0).then_some(index))
        .collect::<Vec<_>>();
    if alive_reserves.is_empty() || !acted_before.contains(&target) {
        events.push(BattleEvent::ForceSwitchFailed {
            side,
            move_name: move_name.to_string(),
            target,
        });
        return Ok(());
    }
    let party_count = party.len() as u8;
    let party_index = loop {
        let candidate = usize::from(rng.battle_random_byte() & 0x07);
        if candidate < usize::from(party_count)
            && candidate != active_index
            && party[candidate].hp > 0
        {
            break candidate;
        }
    };
    events.push(BattleEvent::ForceSwitchApplied {
        side,
        move_name: move_name.to_string(),
        target,
    });
    clear_side_volatile_conditions(state, target);
    switch_battle_combat_pokemon(state, target, party_index)?;
    events.push(BattleEvent::Switched {
        side: target,
        party_index,
    });
    apply_switch_in_spikes(state, target, events);
    Ok(())
}

fn apply_teleport_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    force_switch_ends_battle: bool,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) {
    // BattleCommand_Teleport checks the opponent's CANT_RUN substatus, not
    // the user's. This odd source behavior means a user that trapped its foe
    // cannot Teleport away, while being trapped itself is not this gate.
    if state.force_switch_blocked
        || escape_trap_state(state, side.other()).is_some()
        || !force_switch_ends_battle
    {
        events.push(BattleEvent::TeleportFailed {
            side,
            move_name: move_name.to_string(),
        });
        return;
    }
    let attacker_level = state.pokemon(side).level;
    let target_level = state.pokemon(side.other()).level;
    let succeeds = if attacker_level >= target_level {
        true
    } else {
        let upper = u16::from(attacker_level) + u16::from(target_level) + 1;
        let roll = loop {
            let roll = u16::from(rng.battle_random_byte());
            if roll < upper {
                break roll;
            }
        };
        // BattleCommand_Teleport likewise divides the user's level. The ASM
        // comments call this the opposing level on the enemy path, but the
        // register still contains the Teleport user's level.
        roll >= u16::from(attacker_level / 4)
    };
    if succeeds {
        events.push(BattleEvent::Fled { side });
    } else {
        events.push(BattleEvent::TeleportFailed {
            side,
            move_name: move_name.to_string(),
        });
    }
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

fn apply_berserk_gene_start_of_turn(
    state: &mut BattleCombatState,
    side: BattleSide,
    items: &BTreeMap<String, Item>,
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
    // HandleBerserkGene sets the confused substatus without initializing its
    // byte counter. Zero therefore wraps through 256 decrements; an existing
    // confusion count is preserved. No battle RNG is consumed here.
    if state.pokemon(side).confusion_turns == 0 {
        state.pokemon_mut(side).confusion_turns = 256;
        events.push(BattleEvent::ConfusionApplied {
            side,
            move_name: "HELD_ATTACK_UP".to_string(),
            target: side,
            turns: 256,
        });
    }
    Ok(())
}

fn apply_escape_trap_effect(
    state: &mut BattleCombatState,
    side: BattleSide,
    move_name: &str,
    events: &mut Vec<BattleEvent>,
) {
    let target = side.other();
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
    let roll = rng.battle_random_byte() & 3;
    // The source stores 3..6 before the same-turn residual decrement,
    // producing the documented 2..5 trapped turns.
    let turns = roll + 3;
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
    if move_blocked_by_substitute(state, side, move_name, target, events) {
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

    let roll = loop {
        let roll = rng.battle_random_byte() & 7;
        if roll != 0 {
            break roll;
        }
    };
    let turns = roll + 1;
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
    if acted_before.contains(&side.other()) || substitute_hp(state, side) > 0 {
        reset_protect_counter(state, side);
        return (false, counter_before, None);
    }
    if counter_before == 0 {
        set_protect_counter(state, side, 1);
        return (true, counter_before, None);
    }

    let divisor_shift = counter_before.min(8);
    let threshold = 255u16 / (1u16 << divisor_shift);
    let roll = rng.battle_random_byte();
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

    let roll = rng.battle_random_byte() & 3;
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
    items: &BTreeMap<String, Item>,
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
    let damage =
        calculate_future_sight_damage(state, side, move_data, stat_multipliers, items, events)?;
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
    items: &BTreeMap<String, Item>,
    events: &mut Vec<BattleEvent>,
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
    let mut attack = apply_stage(stat_multipliers, attacker.special_attack, attack_stage).ok_or(
        BattleTurnError::MissingStatMultiplier {
            side,
            stage: attack_stage,
        },
    )?;
    if attacker.species.id == "PIKACHU" && attacker.item.as_deref() == Some("LIGHT_BALL") {
        attack = attack.wrapping_mul(2);
    }
    let mut defense = apply_stage(stat_multipliers, defender.special_defense, defense_stage)
        .ok_or(BattleTurnError::MissingStatMultiplier {
            side: defender_side,
            stage: defense_stage,
        })?;
    if screen_turns(state, defender_side, BattleScreen::LightScreen) != 0 {
        defense = defense.wrapping_mul(2);
    }
    let (mut attack, mut defense) = truncate_damage_stats(attack, defense, state.link_colosseum);
    if defender.species.id == "DITTO" && defender.item.as_deref() == Some("METAL_POWDER") {
        (attack, defense) = apply_metal_powder_damage_stats(attack, defense);
    }
    let held_type_boost_percent =
        held_item_type_boost_percent(state, side, &move_data.move_type, items, events)?;
    let level_factor = (u32::from(attacker.level) * 2) / 5 + 2;
    let mut damage = level_factor
        .saturating_mul(u32::from(move_data.power))
        .saturating_mul(u32::from(attack));
    damage /= u32::from(defense);
    damage /= 50;
    if held_type_boost_percent != 0 {
        damage = damage.saturating_mul(100 + u32::from(held_type_boost_percent)) / 100;
    }
    damage = damage.min(997) + 2;
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
    _items: &BTreeMap<String, Item>,
    _rng: &mut Random,
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
    if state.pokemon(side).status.as_deref() != Some("SLEEP") {
        set_nightmare_source(state, side, None);
    }
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
    Ok(())
}

pub(crate) fn switch_battle_combat_pokemon(
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
    switched.turns_in_battle = switched.turns_in_battle.saturating_add(1).max(1);
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
    sleep_turn_mask: u8,
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
        let sampled = loop {
            let sampled = rng.battle_random_byte() & sleep_turn_mask;
            if sampled != 0 && sampled != sleep_turn_mask {
                break sampled;
            }
        };
        target_pokemon.sleep_turns = sampled + 1;
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
        "POISON" | "BAD_POISON" => types
            .iter()
            .any(|pokemon_type| matches!(pokemon_type.as_str(), "POISON" | "STEEL")),
        "BURN" => types.iter().any(|pokemon_type| pokemon_type == "FIRE"),
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

fn pokemon_is_sandstorm_immune(state: &BattleCombatState, side: BattleSide) -> bool {
    airborne_move_state(state, side).is_some_and(|move_name| move_name == "DIG")
        || effective_pokemon_types(state, side)
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

fn x_accuracy_active(state: &BattleCombatState, side: BattleSide) -> bool {
    match side {
        BattleSide::Player => state.player_x_accuracy,
        BattleSide::Enemy => state.enemy_x_accuracy,
    }
}

fn set_x_accuracy_active(state: &mut BattleCombatState, side: BattleSide, active: bool) {
    match side {
        BattleSide::Player => state.player_x_accuracy = active,
        BattleSide::Enemy => state.enemy_x_accuracy = active,
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

pub fn battle_pokemon_gender(pokemon: &Pokemon) -> Option<BattlePokemonGender> {
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

fn action_priority(
    state: &BattleCombatState,
    side: BattleSide,
    action: &BattleAction,
    target_action: &BattleAction,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    move_priorities: &MovePriorityTable,
) -> Result<i8, BattleTurnError> {
    if recharge_move_state(state, side).is_some() {
        return Ok(move_priorities.base_priority);
    }
    if let Some(committed_move) = forced_move_name_for_priority(state, side) {
        validate_battle_turn_move_name(side, &committed_move)?;
        let move_data =
            moves
                .get(&committed_move)
                .ok_or_else(|| BattleTurnError::MissingMoveData {
                    side,
                    move_name: committed_move.clone(),
                })?;
        let priority = move_priority(move_data, move_priorities)?;
        return Ok(
            if move_data.effect == "PURSUIT" && matches!(target_action, BattleAction::Switch { .. })
            {
                11
            } else {
                priority
            },
        );
    }
    match action {
        BattleAction::Move { slot } | BattleAction::MoveSwitch { slot, .. } => {
            let disabled_move = disable_state(state, side)
                .filter(|disable| disable.turns_remaining > 0)
                .map(|disable| disable.move_name.as_str());
            let move_name = if battle_moves(state, side).iter().all(|learned| {
                learned.current_pp == 0 || disabled_move == Some(learned.name.as_str())
            }) {
                "STRUGGLE".to_string()
            } else {
                battle_moves(state, side)
                    .get(*slot)
                    .map(|selected| selected.name.clone())
                    .ok_or(BattleTurnError::MissingMoveSlot { side, slot: *slot })?
            };
            validate_battle_turn_move_name(side, &move_name)?;
            let move_data =
                moves
                    .get(&move_name)
                    .ok_or_else(|| BattleTurnError::MissingMoveData {
                        side,
                        move_name: move_name.clone(),
                    })?;
            let priority = move_priority(move_data, move_priorities)?;
            if move_data.effect == "PURSUIT" && matches!(target_action, BattleAction::Switch { .. })
            {
                Ok(11)
            } else {
                Ok(priority)
            }
        }
        BattleAction::Switch { .. } => Ok(10),
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
            Ok(10)
        }
        BattleAction::Run => Ok(10),
    }
}

fn forced_move_name_for_priority(state: &BattleCombatState, side: BattleSide) -> Option<String> {
    let mut forced = encore_state(state, side)
        .filter(|encore| encore.turns_remaining > 0)
        .and_then(|encore| {
            battle_moves(state, side)
                .iter()
                .any(|learned| learned.name == encore.move_name)
                .then(|| encore.move_name.clone())
        });
    if bide_turns(state, side) > 0 {
        forced = last_move(state, side).map(ToOwned::to_owned);
    }
    if rollout_turns(state, side) > 0 {
        forced = last_move(state, side).map(ToOwned::to_owned);
    }
    if state.pokemon(side).rampage_turns > 0 {
        forced = last_move(state, side).map(ToOwned::to_owned);
    }
    if let Some(move_name) = airborne_move_state(state, side) {
        forced = Some(move_name.to_string());
    }
    if let Some(move_name) = charging_move_state(state, side) {
        forced = Some(move_name.to_string());
    }
    forced
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
    state: &BattleCombatState,
    side: BattleSide,
    stat_multipliers: &BattleStatMultiplierTables,
) -> Result<u16, BattleTurnError> {
    let pokemon = effective_battle_pokemon(state, side);
    let mut base = pokemon.speed;
    if badge_boost_active(state, side, Stat::Speed) {
        base = base.saturating_add(base / 8).min(999);
    }
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
    Ok(apply_paralysis_speed_penalty(&pokemon, speed))
}

fn badge_boost_active(state: &BattleCombatState, side: BattleSide, stat: Stat) -> bool {
    if side != BattleSide::Player || state.link_battle || !state.badge_boosts_enabled {
        return false;
    }
    let badge_index = match stat {
        Stat::Attack => 0,
        Stat::Defense => 4,
        Stat::Speed => 2,
        Stat::SpecialAttack | Stat::SpecialDefense => 6,
        _ => return false,
    };
    state.obedience_badges[badge_index]
}

fn badge_type_boost_active(state: &BattleCombatState, side: BattleSide, move_type: &str) -> bool {
    const JOHTO_BADGE_TYPES: [&str; 8] = [
        "FLYING", "BUG", "NORMAL", "GHOST", "STEEL", "FIGHTING", "ICE", "DRAGON",
    ];
    const KANTO_BADGE_TYPES: [&str; 8] = [
        "ROCK",
        "WATER",
        "ELECTRIC",
        "GRASS",
        "POISON",
        "PSYCHIC_TYPE",
        "FIRE",
        "GROUND",
    ];

    if side != BattleSide::Player || state.link_battle || !state.badge_boosts_enabled {
        return false;
    }
    state
        .obedience_badges
        .iter()
        .zip(JOHTO_BADGE_TYPES)
        .any(|(owned, badge_type)| *owned && badge_type == move_type)
        || state
            .kanto_badges
            .iter()
            .zip(KANTO_BADGE_TYPES)
            .any(|(owned, badge_type)| *owned && badge_type == move_type)
}

fn apply_paralysis_speed_penalty(pokemon: &Pokemon, speed: u16) -> u16 {
    if pokemon.status.as_deref() == Some("PARALYSIS") {
        (speed / 4).max(1)
    } else {
        speed
    }
}

#[cfg(test)]
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
        false,
    )
}

fn accuracy_byte_with_weather(
    move_data: &Move,
    attacker_side: BattleSide,
    attacker: &Pokemon,
    defender: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
    weather: Weather,
    defender_identified: bool,
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
    let base = ((move_data.accuracy as i32 * 255) / 100).clamp(1, 255);
    // `CheckHit.StatModifiers` returns before both multiplier lookups when
    // Foresight identifies a target whose Evasion level is at least the
    // user's Accuracy level. The move therefore retains its base byte.
    if defender_identified && defender_evasion >= attacker_accuracy {
        return Ok(base as u8);
    }
    let stage = (attacker_accuracy - defender_evasion).clamp(-6, 6);
    let multiplier = accuracy_stage_multiplier(stat_multipliers, stage)
        .ok_or(BattleTurnError::MissingAccuracyMultiplier { stage })?;
    Ok(multiplier.multiply_floor(base).clamp(1, 255) as u8)
}

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;
