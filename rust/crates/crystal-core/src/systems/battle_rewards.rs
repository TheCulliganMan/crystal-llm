use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use thiserror::Error;

use crate::battle::start::{
    ActiveBattleEnemyError, ActiveBattlePartyError, claim_active_trainer_battle_reward_index,
    deactivate_battle_after_win, require_active_battle_party_index, update_active_battle_enemy,
};
use crate::battle::stats::BattleStatMultiplierTables;
use crate::battle::turn::{BattleTurnError, recalculate_loaded_stats};
use crate::models::pokemon::StatExperience;
use crate::models::{LearnedMove, Move, Pokemon, PokemonSpecies, calculate_stats};
use crate::random::{CrystalRandom, DividerSource};
use crate::state::{BattleMemory, GameState, PendingMomPurchase, PendingMoveLearn};
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
    pub mom_money_increment: u32,
    pub mom_random_items: Vec<MomPurchaseRule>,
    pub mom_progression_items: Vec<MomPurchaseRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleLevelUpHappinessContext {
    pub current_landmark: u8,
    pub gain_level: [i16; 3],
    pub gain_level_at_home: [i16; 3],
}

impl BattleLevelUpHappinessContext {
    fn changes(self, pokemon: &Pokemon) -> [i16; 3] {
        let caught_location = pokemon
            .caught_data
            .as_ref()
            .map_or(0, |caught| caught.location);
        let at_home = caught_location == self.current_landmark;
        if at_home {
            self.gain_level_at_home
        } else {
            self.gain_level
        }
    }
}

pub fn happiness_delta(happiness: u8, changes: [i16; 3]) -> i16 {
    changes[usize::from(happiness >= 100) + usize::from(happiness >= 200)]
}

pub fn apply_happiness_change(pokemon: &mut Pokemon, changes: [i16; 3]) {
    if pokemon.is_egg {
        return;
    }
    pokemon.happiness = (i16::from(pokemon.happiness) + happiness_delta(pokemon.happiness, changes))
        .clamp(0, 255) as u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MomPurchaseKind {
    Item,
    Doll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MomPurchaseRule {
    pub trigger: u32,
    pub cost: u32,
    pub kind: MomPurchaseKind,
    pub target: String,
    pub decoration_flag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MomPurchaseSelection {
    pub progression: bool,
    pub selected_index: u8,
    pub rule: MomPurchaseRule,
}

pub fn select_mom_purchase<S>(
    state: &mut GameState,
    rules: &BattleRewardRules,
    divider: &mut S,
) -> Result<Option<MomPurchaseSelection>, String>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    if state.pending_mom_purchase.is_some() {
        return Err("cannot select a Mom purchase while one is pending".to_string());
    }
    rules.validate_shape()?;
    if let Some(rule) = rules
        .mom_progression_items
        .get(usize::from(state.mom_item_index))
        .filter(|rule| state.moms_money >= rule.trigger)
    {
        return Ok(Some(MomPurchaseSelection {
            progression: true,
            selected_index: state.mom_item_index,
            rule: rule.clone(),
        }));
    }

    loop {
        if state.mom_item_trigger_balance > state.moms_money {
            return Ok(None);
        }
        if state.mom_item_trigger_balance == state.moms_money {
            state.mom_item_trigger_balance = state
                .mom_item_trigger_balance
                .checked_add(rules.mom_money_increment)
                .ok_or_else(|| "Mom item trigger balance overflow".to_string())?;
            let count = u8::try_from(rules.mom_random_items.len())
                .map_err(|_| "Mom random item table exceeds one-byte range".to_string())?;
            let mut rng = CrystalRandom::new(state.random_state, divider);
            let selected_index = rng
                .random_range(count)
                .map_err(|error| format!("Mom RandomRange divider source: {error}"))?;
            state.random_state = rng.state();
            return Ok(Some(MomPurchaseSelection {
                progression: false,
                selected_index,
                rule: rules.mom_random_items[usize::from(selected_index)].clone(),
            }));
        }
        state.mom_item_trigger_balance = state
            .mom_item_trigger_balance
            .checked_add(rules.mom_money_increment)
            .ok_or_else(|| "Mom item trigger balance overflow".to_string())?;
    }
}

pub fn settle_pending_mom_purchase(state: &mut GameState) -> Result<PendingMomPurchase, String> {
    let purchase = state
        .pending_mom_purchase
        .take()
        .ok_or_else(|| "cannot settle Mom purchase because none is pending".to_string())?;
    state.moms_money = state
        .moms_money
        .checked_sub(purchase.cost)
        .ok_or_else(|| "Mom purchase cost exceeds saved money".to_string())?;
    if purchase.progression {
        state.mom_item_index = state
            .mom_item_index
            .checked_add(1)
            .ok_or_else(|| "Mom progression index overflow".to_string())?;
    }
    Ok(purchase)
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
            mom_money_increment: u32,
            mom_random_items: Vec<MomPurchaseRule>,
            mom_progression_items: Vec<MomPurchaseRule>,
        }

        let raw = RawBattleRewardRules::deserialize(deserializer)?;
        let rules = Self {
            max_level: raw.max_level,
            wild_exp_divisor: raw.wild_exp_divisor,
            trainer_exp_numerator: raw.trainer_exp_numerator,
            trainer_exp_denominator: raw.trainer_exp_denominator,
            mom_money_increment: raw.mom_money_increment,
            mom_random_items: raw.mom_random_items,
            mom_progression_items: raw.mom_progression_items,
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
            mom_money_increment: 0,
            mom_random_items: Vec::new(),
            mom_progression_items: Vec::new(),
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
    MomPurchaseRules,
}

impl BattleRewardRulesField {
    pub const fn subject(self) -> &'static str {
        match self {
            Self::MaxLevel => "battle_reward_rules:max_level",
            Self::WildExpDivisor => "battle_reward_rules:wild_exp_divisor",
            Self::TrainerExpNumerator => "battle_reward_rules:trainer_exp_numerator",
            Self::TrainerExpDenominator => "battle_reward_rules:trainer_exp_denominator",
            Self::MomPurchaseRules => "battle_reward_rules:mom_purchase_rules",
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
    InvalidMomPurchaseRules { reason: String },
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
            Self::InvalidMomPurchaseRules { .. } => BattleRewardRulesField::MomPurchaseRules,
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
    if let Err(reason) = validate_mom_purchase_rules(rules) {
        issues.push(BattleRewardRulesIssue::InvalidMomPurchaseRules { reason });
    }
    issues
}

fn validate_mom_purchase_rules(rules: &BattleRewardRules) -> Result<(), String> {
    if rules.mom_money_increment == 0 {
        return Err("mom_money_increment must be positive".to_string());
    }
    if rules.mom_random_items.is_empty() || rules.mom_progression_items.is_empty() {
        return Err("both Mom item sets must be nonempty".to_string());
    }
    for (set_name, entries) in [
        ("mom_random_items", &rules.mom_random_items),
        ("mom_progression_items", &rules.mom_progression_items),
    ] {
        let mut previous_trigger = None;
        for (index, entry) in entries.iter().enumerate() {
            if entry.cost == 0 {
                return Err(format!("{set_name}[{index}] cost must be positive"));
            }
            if entry.target.is_empty()
                || !entry
                    .target
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
            {
                return Err(format!(
                    "{set_name}[{index}] target is not an exact ASM token"
                ));
            }
            match entry.kind {
                MomPurchaseKind::Item if entry.decoration_flag.is_some() => {
                    return Err(format!("{set_name}[{index}] item has a decoration flag"));
                }
                MomPurchaseKind::Doll
                    if entry.decoration_flag.as_deref().is_none_or(|flag| {
                        flag.is_empty()
                            || !flag.bytes().all(|byte| {
                                byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
                            })
                    }) =>
                {
                    return Err(format!(
                        "{set_name}[{index}] doll lacks an exact decoration flag"
                    ));
                }
                MomPurchaseKind::Doll => {
                    let expected = format!("EVENT_{}", entry.target);
                    if entry.decoration_flag.as_deref() != Some(expected.as_str()) {
                        return Err(format!(
                            "{set_name}[{index}] doll decoration flag must be {expected}"
                        ));
                    }
                }
                _ => {}
            }
            if set_name == "mom_random_items" && entry.trigger != 0 {
                return Err(format!("{set_name}[{index}] trigger must be zero"));
            }
            if let Some(previous) = previous_trigger
                && entry.trigger <= previous
                && set_name == "mom_progression_items"
            {
                return Err(format!("{set_name} triggers must be strictly increasing"));
            }
            previous_trigger = Some(entry.trigger);
        }
    }
    Ok(())
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
    pub recipient_outcomes: Vec<BattleRewardRecipientOutcome>,
    pub post_battle_evolutions: Vec<BattlePartyEvolutionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleRewardRecipientOutcome {
    pub party_index: usize,
    pub nickname: String,
    pub experience_awarded: i32,
    pub level_before: u8,
    pub level_after: u8,
    pub learned_moves: Vec<String>,
    pub pending_move_learns: Vec<LearnedMove>,
    pub evolution: EvolutionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePartyEvolutionOutcome {
    pub party_index: usize,
    pub nickname: String,
    pub evolution: EvolutionReport,
}

fn recipient_reward_outcome(
    party_index: usize,
    nickname: String,
    outcome: &BattleRewardOutcome,
) -> BattleRewardRecipientOutcome {
    BattleRewardRecipientOutcome {
        party_index,
        nickname,
        experience_awarded: outcome.experience_awarded,
        level_before: outcome.level_before,
        level_after: outcome.level_after,
        learned_moves: outcome.learned_moves.clone(),
        pending_move_learns: outcome.pending_move_learns.clone(),
        evolution: outcome.evolution.clone(),
    }
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
    #[error("battle reward recipient count {count} cannot be represented")]
    InvalidRecipientCount { count: usize },
    #[error("pending move learn is missing")]
    MissingPendingMoveLearn,
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
    #[error("HM move {move_id} cannot be forgotten while learning a move")]
    CannotForgetHmMove { move_id: String },
    #[error("pending move learn species {species_id} does not match party index {party_index}")]
    PendingMoveLearnSpeciesMismatch {
        party_index: usize,
        species_id: String,
    },
    #[error("pending move learn level {level} does not match party index {party_index}")]
    PendingMoveLearnLevelMismatch { party_index: usize, level: u8 },
    #[error("post-battle Pokerus divider failed: {error}")]
    PokerusDivider { error: String },
    #[error("active level-up battle-stat refresh failed: {0:?}")]
    BattleStats(BattleTurnError),
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

fn reward_recipient_count(count: usize) -> Result<i32, BattleRewardError> {
    if count == 0 {
        return Err(BattleRewardError::InvalidRecipientCount { count });
    }
    i32::try_from(count).map_err(|_| BattleRewardError::InvalidRecipientCount { count })
}

fn split_experience_award(
    rules: &BattleRewardRules,
    defeated: &Pokemon,
    divisor: i32,
    trainer_battle: bool,
) -> Result<i32, BattleRewardError> {
    require_battle_reward_rules(rules)?;
    require_positive_i32(divisor, "experience_recipient_divisor")?;
    let adjusted_base_exp = i32::from(defeated.species.base_exp) / divisor;
    let wild_award = (adjusted_base_exp * i32::from(defeated.level)) / rules.wild_exp_divisor;
    if trainer_battle {
        Ok((wild_award * rules.trainer_exp_numerator) / rules.trainer_exp_denominator)
    } else {
        Ok(wild_award)
    }
}

pub fn sync_active_combat_player_party_from_storage(state: &mut GameState) {
    let Some(combat) = state.script_runtime.active_battle_combat.as_mut() else {
        return;
    };
    for (index, combat_pokemon) in combat.player_party.iter_mut().enumerate() {
        let Some(Some(stored_pokemon)) = state.storage.party.pokemon.get(index) else {
            continue;
        };
        *combat_pokemon = stored_pokemon.clone();
    }
    if let Some(Some(active_pokemon)) = state.storage.party.pokemon.get(combat.player_party_index) {
        let stat_boosts = combat.player.stat_boosts.clone();
        let flinching = combat.player.flinching;
        let rampage_turns = combat.player.rampage_turns;
        let confusion_turns = combat.player.confusion_turns;
        let perish_song_turns = combat.player.perish_song_turns;
        let focus_energy = combat.player.focus_energy;
        let turns_in_battle = combat.player.turns_in_battle;
        combat.player = active_pokemon.clone();
        combat.player.stat_boosts = stat_boosts;
        combat.player.flinching = flinching;
        combat.player.rampage_turns = rampage_turns;
        combat.player.confusion_turns = confusion_turns;
        combat.player.perish_song_turns = perish_song_turns;
        combat.player.focus_energy = focus_energy;
        combat.player.turns_in_battle = turns_in_battle;
    }
}

fn sync_active_combat_player_reward_from_storage(
    state: &mut GameState,
    stat_multipliers: &BattleStatMultiplierTables,
    leveled_up: bool,
) -> Result<(), BattleRewardError> {
    let Some(combat) = state.script_runtime.active_battle_combat.as_mut() else {
        return Ok(());
    };
    for (index, combat_pokemon) in combat.player_party.iter_mut().enumerate() {
        let Some(Some(stored_pokemon)) = state.storage.party.pokemon.get(index) else {
            continue;
        };
        *combat_pokemon = stored_pokemon.clone();
    }
    let Some(Some(stored)) = state.storage.party.pokemon.get(combat.player_party_index) else {
        return Ok(());
    };

    // The level-up path updates persistent party data while the active BattleMon
    // retains its volatile battle bytes. It then rebuilds live stats in the
    // source order: stages, status, badges. Transform keeps supplying its copied
    // raw words, but uses the newly loaded level for subsequent damage.
    let stat_boosts = combat.player.stat_boosts.clone();
    let flinching = combat.player.flinching;
    let rampage_turns = combat.player.rampage_turns;
    let confusion_turns = combat.player.confusion_turns;
    let perish_song_turns = combat.player.perish_song_turns;
    let focus_energy = combat.player.focus_energy;
    let turns_in_battle = combat.player.turns_in_battle;
    let level = combat.player.level;
    let hp = combat.player.hp;
    let max_hp = combat.player.max_hp;
    let raw_stats = (
        combat.player.attack,
        combat.player.defense,
        combat.player.speed,
        combat.player.special_attack,
        combat.player.special_defense,
    );
    combat.player = stored.clone();
    combat.player.stat_boosts = stat_boosts;
    combat.player.flinching = flinching;
    combat.player.rampage_turns = rampage_turns;
    combat.player.confusion_turns = confusion_turns;
    combat.player.perish_song_turns = perish_song_turns;
    combat.player.focus_energy = focus_energy;
    combat.player.turns_in_battle = turns_in_battle;
    if leveled_up {
        combat.player_badge_before_status = false;
        recalculate_loaded_stats(
            combat,
            crate::battle::turn::BattleSide::Player,
            stat_multipliers,
        )
        .map_err(BattleRewardError::BattleStats)?;
    } else {
        combat.player.level = level;
        combat.player.hp = hp;
        combat.player.max_hp = max_hp;
        combat.player.attack = raw_stats.0;
        combat.player.defense = raw_stats.1;
        combat.player.speed = raw_stats.2;
        combat.player.special_attack = raw_stats.3;
        combat.player.special_defense = raw_stats.4;
    }
    Ok(())
}

fn reset_trainer_reward_participants(state: &mut GameState, active_index: usize) {
    for pokemon in state.storage.party.pokemon.iter_mut().flatten() {
        pokemon.turns_in_battle = 0;
    }
    if let Some(Some(active)) = state.storage.party.pokemon.get_mut(active_index) {
        active.turns_in_battle = 1;
    }
}

pub fn claim_active_trainer_battle_rewards(
    state: &mut GameState,
    rules: &BattleRewardRules,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    stat_multipliers: &BattleStatMultiplierTables,
    level_up_happiness: BattleLevelUpHappinessContext,
    time_of_day: TimeOfDay,
) -> Result<BattleRewardOutcome, ActiveTrainerBattleRewardError> {
    let rewards_disabled = state.link_session.link_mode != 0
        || matches!(
            &state.battle,
            BattleMemory::Trainer { battle_type, .. }
                if battle_type == "BATTLETYPE_BATTLE_TOWER"
        );
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
    if rewards_disabled {
        let active = state.storage.party.pokemon[active_index].as_ref().ok_or(
            ActiveBattlePartyError::EmptyPartySlot {
                index: active_index,
            },
        )?;
        let outcome = BattleRewardOutcome {
            defeated_species: enemy.species.id.clone(),
            experience_awarded: 0,
            level_before: active.level,
            level_after: active.level,
            learned_moves: Vec::new(),
            pending_move_learns: Vec::new(),
            deferred_level_evolution: false,
            evolution: EvolutionReport::default(),
            recipient_outcomes: Vec::new(),
            post_battle_evolutions: Vec::new(),
        };
        update_active_battle_enemy(state, enemy)?;
        claim_active_trainer_battle_reward_index(state)?;
        reset_trainer_reward_participants(state, active_index);
        state.sync_party_from_storage();
        sync_active_combat_player_party_from_storage(state);
        return Ok(outcome);
    }
    let active_level_before = state.storage.party.pokemon[active_index]
        .as_ref()
        .ok_or(ActiveBattlePartyError::EmptyPartySlot {
            index: active_index,
        })?
        .level;
    let mut participant_indices = state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .filter_map(|(index, pokemon)| {
            pokemon
                .as_ref()
                .filter(|pokemon| pokemon.turns_in_battle > 0 && pokemon.hp > 0)
                .map(|_| index)
        })
        .collect::<Vec<_>>();
    if !participant_indices.contains(&active_index) {
        participant_indices.push(active_index);
    }
    participant_indices.sort_unstable();
    let exp_share_indices = state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .filter_map(|(index, pokemon)| {
            pokemon
                .as_ref()
                .filter(|pokemon| pokemon.hp > 0 && pokemon.item.as_deref() == Some("EXP_SHARE"))
                .map(|_| index)
        })
        .collect::<Vec<_>>();
    let participant_count = reward_recipient_count(participant_indices.len())?;
    let exp_share_count = if exp_share_indices.is_empty() {
        None
    } else {
        Some(reward_recipient_count(exp_share_indices.len())?)
    };
    let participant_divisor = if exp_share_indices.is_empty() {
        participant_count
    } else {
        participant_count.saturating_mul(2)
    };
    let participant_experience = split_experience_award(rules, &enemy, participant_divisor, true)?;
    let stat_experience_divisor = if exp_share_indices.is_empty() {
        participant_count
    } else {
        participant_count.saturating_mul(2)
    };
    let mut active_outcome = None;
    let mut recipient_outcomes = Vec::new();
    // GiveExperiencePoints advances wCurPartyMon from slot 0 through the
    // party, irrespective of which participant is currently active.
    for participant_index in participant_indices {
        let participant_traded = state.storage.party.pokemon[participant_index]
            .as_ref()
            .is_some_and(|pokemon| pokemon.original_trainer_id != state.player_id);
        let participant_outcome = {
            let participant = state.storage.party.pokemon[participant_index]
                .as_mut()
                .ok_or(ActiveBattlePartyError::EmptyPartySlot {
                    index: participant_index,
                })?;
            apply_battle_rewards_with_experience(
                rules,
                participant,
                &enemy,
                species,
                moves,
                learnsets,
                growth_rates,
                evolutions,
                level_up_happiness,
                time_of_day,
                false,
                participant_traded,
                participant_experience,
                stat_experience_divisor,
            )?
        };
        if participant_outcome.level_after > participant_outcome.level_before {
            state
                .battle_evolvable_party_indices
                .insert(participant_index);
        }
        queue_pending_move_learn(state, participant_index, &participant_outcome)?;
        recipient_outcomes.push(recipient_reward_outcome(
            participant_index,
            state.storage.party.pokemon[participant_index]
                .as_ref()
                .unwrap()
                .nickname
                .clone(),
            &participant_outcome,
        ));
        if participant_index == active_index {
            active_outcome = Some(participant_outcome);
        }
    }
    let mut outcome = active_outcome.ok_or(ActiveBattlePartyError::EmptyPartySlot {
        index: active_index,
    })?;
    state.sync_party_from_storage();
    update_active_battle_enemy(state, enemy.clone())?;
    claim_active_trainer_battle_reward_index(state)?;
    for share_index in exp_share_indices {
        let exp_share_count =
            exp_share_count.ok_or(BattleRewardError::InvalidRecipientCount { count: 0 })?;
        // GiveExperiencePoints restores wBackupEnemyMonBaseStats here, but
        // that backup was taken after the Exp. Share halving pass.
        let exp_share_divisor = exp_share_count.saturating_mul(2);
        let holder_traded = state.storage.party.pokemon[share_index]
            .as_ref()
            .is_some_and(|pokemon| pokemon.original_trainer_id != state.player_id);
        let Some(holder) = state.storage.party.pokemon[share_index].as_mut() else {
            continue;
        };
        let share_outcome = apply_battle_rewards_with_experience(
            rules,
            holder,
            &enemy,
            species,
            moves,
            learnsets,
            growth_rates,
            evolutions,
            level_up_happiness,
            time_of_day,
            false,
            holder_traded,
            split_experience_award(rules, &enemy, exp_share_divisor, true)?,
            exp_share_divisor,
        )?;
        if share_outcome.level_after > share_outcome.level_before {
            state.battle_evolvable_party_indices.insert(share_index);
        }
        queue_pending_move_learn(state, share_index, &share_outcome)?;
        recipient_outcomes.push(recipient_reward_outcome(
            share_index,
            state.storage.party.pokemon[share_index]
                .as_ref()
                .unwrap()
                .nickname
                .clone(),
            &share_outcome,
        ));
    }
    outcome.recipient_outcomes = recipient_outcomes;
    let trainer_defeated = match (&state.battle, state.battle_active_enemy_party_index) {
        (BattleMemory::Trainer { enemy_party, .. }, Some(active_enemy_index)) => enemy_party
            .iter()
            .enumerate()
            .all(|(index, pokemon)| index == active_enemy_index || pokemon.hp == 0),
        _ => false,
    };
    if trainer_defeated
        && state.pending_move_learn.is_none()
        && state.pending_move_learn_queue.is_empty()
    {
        let evolutions = evolve_flagged_party_after_battle(
            state,
            species,
            moves,
            learnsets,
            evolutions,
            time_of_day,
        )?;
        for evolution in &evolutions {
            if evolution.party_index == active_index {
                outcome.evolution = evolution.evolution.clone();
            }
        }
        outcome.post_battle_evolutions = evolutions;
    }
    let active_leveled_up = state.storage.party.pokemon[active_index]
        .as_ref()
        .is_some_and(|pokemon| pokemon.level > active_level_before);
    reset_trainer_reward_participants(state, active_index);
    state.sync_party_from_storage();
    sync_active_combat_player_reward_from_storage(state, stat_multipliers, active_leveled_up)?;
    Ok(outcome)
}

fn evolve_flagged_party_after_battle(
    state: &mut GameState,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
) -> Result<Vec<BattlePartyEvolutionOutcome>, BattleRewardError> {
    let flagged = state
        .battle_evolvable_party_indices
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let context = crate::systems::evolution::EvolutionContext {
        species,
        moves,
        learnsets,
        time_of_day,
        current_item: None,
        force_evolution: false,
        link_mode: crate::systems::evolution::LinkMode::None,
    };
    let mut outcomes = Vec::new();
    for party_index in flagged {
        let nickname = state.storage.party.pokemon[party_index]
            .as_ref()
            .ok_or(BattleRewardError::PendingMoveLearnEmptyPartySlot { party_index })?
            .nickname
            .clone();
        let report = {
            let pokemon = state.storage.party.pokemon[party_index]
                .as_mut()
                .ok_or(BattleRewardError::PendingMoveLearnEmptyPartySlot { party_index })?;
            check_and_evolve(pokemon, evolutions, &context, true)?
        };
        state.battle_evolvable_party_indices.remove(&party_index);
        if report.target_species.is_none() {
            continue;
        }
        if report.cancel_snapshot.is_none() {
            let evolved_pokemon = state.storage.party.pokemon[party_index]
                .as_ref()
                .unwrap()
                .clone();
            state.pokedex.record_caught_pokemon(&evolved_pokemon);
        }
        rebase_pending_move_learns_for_party(state, party_index, true);
        if !report.pending_move_learns.is_empty() {
            let queue_outcome = BattleRewardOutcome {
                defeated_species: String::new(),
                experience_awarded: 0,
                level_before: state.storage.party.pokemon[party_index]
                    .as_ref()
                    .unwrap()
                    .level,
                level_after: state.storage.party.pokemon[party_index]
                    .as_ref()
                    .unwrap()
                    .level,
                learned_moves: Vec::new(),
                pending_move_learns: report.pending_move_learns.clone(),
                deferred_level_evolution: false,
                evolution: report.clone(),
                recipient_outcomes: Vec::new(),
                post_battle_evolutions: Vec::new(),
            };
            queue_pending_move_learn(state, party_index, &queue_outcome)?;
        }
        outcomes.push(BattlePartyEvolutionOutcome {
            party_index,
            nickname,
            evolution: report,
        });
    }
    state.sync_party_from_storage();
    sync_active_combat_player_party_from_storage(state);
    Ok(outcomes)
}

pub fn claim_active_wild_battle_rewards<S>(
    state: &mut GameState,
    rules: &BattleRewardRules,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    evolutions: &EvolutionTable,
    level_up_happiness: BattleLevelUpHappinessContext,
    time_of_day: TimeOfDay,
    divider: &mut S,
) -> Result<BattleRewardOutcome, ActiveWildBattleRewardError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
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
    let mut participant_indices = state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .filter_map(|(index, pokemon)| {
            pokemon
                .as_ref()
                .filter(|pokemon| pokemon.turns_in_battle > 0 && pokemon.hp > 0)
                .map(|_| index)
        })
        .collect::<Vec<_>>();
    if !participant_indices.contains(&active_index) {
        participant_indices.push(active_index);
    }
    participant_indices.sort_unstable();
    let exp_share_indices = state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .filter_map(|(index, pokemon)| {
            pokemon
                .as_ref()
                .filter(|pokemon| pokemon.hp > 0 && pokemon.item.as_deref() == Some("EXP_SHARE"))
                .map(|_| index)
        })
        .collect::<Vec<_>>();
    let participant_count = reward_recipient_count(participant_indices.len())?;
    let exp_share_count = if exp_share_indices.is_empty() {
        None
    } else {
        Some(reward_recipient_count(exp_share_indices.len())?)
    };
    let participant_divisor = if exp_share_indices.is_empty() {
        participant_count
    } else {
        participant_count.saturating_mul(2)
    };
    let participant_experience = split_experience_award(rules, &enemy, participant_divisor, false)?;
    let stat_experience_divisor = if exp_share_indices.is_empty() {
        participant_count
    } else {
        participant_count.saturating_mul(2)
    };
    let mut active_outcome = None;
    let mut recipient_outcomes = Vec::new();
    for participant_index in participant_indices {
        let participant_traded = state.storage.party.pokemon[participant_index]
            .as_ref()
            .is_some_and(|pokemon| pokemon.original_trainer_id != state.player_id);
        let participant_outcome = {
            let participant = state.storage.party.pokemon[participant_index]
                .as_mut()
                .ok_or(ActiveBattlePartyError::EmptyPartySlot {
                    index: participant_index,
                })?;
            apply_battle_rewards_with_experience(
                rules,
                participant,
                &enemy,
                species,
                moves,
                learnsets,
                growth_rates,
                evolutions,
                level_up_happiness,
                time_of_day,
                false,
                participant_traded,
                participant_experience,
                stat_experience_divisor,
            )?
        };
        if participant_outcome.level_after > participant_outcome.level_before {
            state
                .battle_evolvable_party_indices
                .insert(participant_index);
        }
        queue_pending_move_learn(state, participant_index, &participant_outcome)?;
        recipient_outcomes.push(recipient_reward_outcome(
            participant_index,
            state.storage.party.pokemon[participant_index]
                .as_ref()
                .unwrap()
                .nickname
                .clone(),
            &participant_outcome,
        ));
        if participant_index == active_index {
            active_outcome = Some(participant_outcome);
        }
    }
    let mut outcome = active_outcome.ok_or(ActiveBattlePartyError::EmptyPartySlot {
        index: active_index,
    })?;
    for share_index in exp_share_indices {
        let exp_share_count =
            exp_share_count.ok_or(BattleRewardError::InvalidRecipientCount { count: 0 })?;
        // GiveExperiencePoints restores wBackupEnemyMonBaseStats here, but
        // that backup was taken after the Exp. Share halving pass.
        let exp_share_divisor = exp_share_count.saturating_mul(2);
        let holder_traded = state.storage.party.pokemon[share_index]
            .as_ref()
            .is_some_and(|pokemon| pokemon.original_trainer_id != state.player_id);
        let Some(holder) = state.storage.party.pokemon[share_index].as_mut() else {
            continue;
        };
        let share_outcome = apply_battle_rewards_with_experience(
            rules,
            holder,
            &enemy,
            species,
            moves,
            learnsets,
            growth_rates,
            evolutions,
            level_up_happiness,
            time_of_day,
            false,
            holder_traded,
            split_experience_award(rules, &enemy, exp_share_divisor, false)?,
            exp_share_divisor,
        )?;
        if share_outcome.level_after > share_outcome.level_before {
            state.battle_evolvable_party_indices.insert(share_index);
        }
        queue_pending_move_learn(state, share_index, &share_outcome)?;
        recipient_outcomes.push(recipient_reward_outcome(
            share_index,
            state.storage.party.pokemon[share_index]
                .as_ref()
                .unwrap()
                .nickname
                .clone(),
            &share_outcome,
        ));
    }
    outcome.recipient_outcomes = recipient_outcomes;
    if state.pending_move_learn.is_none() && state.pending_move_learn_queue.is_empty() {
        let evolution_outcomes = evolve_flagged_party_after_battle(
            state,
            species,
            moves,
            learnsets,
            evolutions,
            time_of_day,
        )?;
        for evolution in &evolution_outcomes {
            if evolution.party_index == active_index {
                outcome.evolution = evolution.evolution.clone();
            }
        }
        outcome.post_battle_evolutions = evolution_outcomes;
    }
    deactivate_battle_after_win(state);
    state
        .spread_pokerus_after_battle(divider)
        .map_err(|error| BattleRewardError::PokerusDivider {
            error: error.to_string(),
        })?;
    state.sync_party_from_storage();
    Ok(outcome)
}

fn queue_pending_move_learn(
    state: &mut GameState,
    party_index: usize,
    outcome: &BattleRewardOutcome,
) -> Result<(), BattleRewardError> {
    if outcome.evolution.target_species.is_some() {
        rebase_pending_move_learns_for_party(state, party_index, true);
    }
    if outcome.pending_move_learns.is_empty() {
        return Ok(());
    }
    if outcome.deferred_level_evolution && !outcome.pending_move_learns.is_empty() {
        if let Some(pending) = state
            .pending_move_learn
            .as_mut()
            .filter(|pending| pending.party_index == party_index)
        {
            pending.defer_level_evolution = false;
        }
        for pending in state
            .pending_move_learn_queue
            .iter_mut()
            .filter(|pending| pending.party_index == party_index)
        {
            pending.defer_level_evolution = false;
        }
    }
    let pokemon = state.storage.party.pokemon[party_index]
        .as_ref()
        .ok_or(BattleRewardError::PendingMoveLearnEmptyPartySlot { party_index })?;
    if pokemon.moves.len() < 4 {
        return Err(BattleRewardError::PendingMoveLearnRequiresFullMoveList { party_index });
    }
    let species_id = pokemon.species.id.clone();
    let level = pokemon.level;
    let pending_count = outcome.pending_move_learns.len();
    for (index, learned_move) in outcome.pending_move_learns.iter().enumerate() {
        if pokemon
            .moves
            .iter()
            .any(|known| known.name == learned_move.name)
            || state.pending_move_learn.iter().any(|pending| {
                pending.party_index == party_index && pending.learned_move.name == learned_move.name
            })
            || state.pending_move_learn_queue.iter().any(|pending| {
                pending.party_index == party_index && pending.learned_move.name == learned_move.name
            })
        {
            continue;
        }
        let pending = PendingMoveLearn {
            party_index,
            species_id: species_id.clone(),
            level,
            learned_move: learned_move.clone(),
            defer_level_evolution: outcome.deferred_level_evolution && index + 1 == pending_count,
        };
        if state.pending_move_learn.is_none() {
            state.pending_move_learn = Some(pending);
        } else {
            state.pending_move_learn_queue.push(pending);
        }
    }
    Ok(())
}

pub fn promote_next_pending_move_learn(state: &mut GameState) {
    if state.pending_move_learn.is_none() && !state.pending_move_learn_queue.is_empty() {
        state.pending_move_learn = Some(state.pending_move_learn_queue.remove(0));
    }
}

pub fn rebase_pending_move_learns_for_party(
    state: &mut GameState,
    party_index: usize,
    evolution_resolved: bool,
) {
    let Some(Some(pokemon)) = state.storage.party.pokemon.get(party_index) else {
        return;
    };
    let species_id = pokemon.species.id.clone();
    let level = pokemon.level;
    if let Some(pending) = state
        .pending_move_learn
        .as_mut()
        .filter(|pending| pending.party_index == party_index)
    {
        pending.species_id = species_id.clone();
        pending.level = level;
        if evolution_resolved {
            pending.defer_level_evolution = false;
        }
    }
    for pending in state
        .pending_move_learn_queue
        .iter_mut()
        .filter(|pending| pending.party_index == party_index)
    {
        pending.species_id = species_id.clone();
        pending.level = level;
        if evolution_resolved {
            pending.defer_level_evolution = false;
        }
    }
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
    level_up_happiness: BattleLevelUpHappinessContext,
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
        level_up_happiness,
        time_of_day,
        true,
        false,
        wild_experience_award(rules, defeated)?,
        1,
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
    level_up_happiness: BattleLevelUpHappinessContext,
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
        level_up_happiness,
        time_of_day,
        true,
        false,
        trainer_experience_award(rules, defeated)?,
        1,
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
    sync_active_combat_player_party_from_storage(state);
    if let Some(combat) = state.script_runtime.active_battle_combat.as_mut()
        && combat.player_party_index == pending.party_index
        && combat
            .player_disable
            .as_ref()
            .is_some_and(|disable| disable.move_name == replaced_move)
    {
        combat.player_disable = None;
    }
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
    level_up_happiness: BattleLevelUpHappinessContext,
    time_of_day: TimeOfDay,
    evolve_now: bool,
    traded: bool,
    base_experience_awarded: i32,
    stat_experience_divisor: i32,
) -> Result<BattleRewardOutcome, BattleRewardError> {
    if defeated.hp != 0 {
        return Err(BattleRewardError::DefeatedPokemonNotFainted);
    }
    // Crystal applies traded and Lucky Egg boosts after the participant split
    // and trainer multiplier, using the same sequential 1.5x integer
    // calculation as BoostExp.
    let traded_experience = if traded {
        base_experience_awarded.saturating_mul(3) / 2
    } else {
        base_experience_awarded
    };
    let experience_awarded = if player.item.as_deref() == Some("LUCKY_EGG") {
        traded_experience.saturating_mul(3) / 2
    } else {
        traded_experience
    };
    let mut rewarded = player.clone();
    let level_before = rewarded.level;
    let maximum_experience =
        calculate_experience(growth_rates, &rewarded.species.growth_rate, rules.max_level)?;
    rewarded.experience = rewarded
        .experience
        .saturating_add(experience_awarded)
        .min(maximum_experience);
    add_stat_experience(
        &mut rewarded,
        defeated.species.base_stats,
        stat_experience_divisor,
    );
    let level_up =
        apply_experience_level_ups(&mut rewarded, moves, learnsets, growth_rates, rules)?;
    if level_up.level_after > level_up.level_before {
        let level_up_change = level_up_happiness.changes(&rewarded);
        apply_happiness_change(&mut rewarded, level_up_change);
    }
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
    let evolution = if !evolve_now || deferred_level_evolution {
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
        recipient_outcomes: Vec::new(),
        post_battle_evolutions: Vec::new(),
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

fn add_stat_experience(player: &mut Pokemon, base_stats: crate::models::BaseStats, divisor: i32) {
    let divisor = divisor.max(1) as u16;
    let multiplier = if player.pokerus != 0 { 2 } else { 1 };
    let adjusted = |value: u16| (value / divisor).saturating_mul(multiplier);
    player.hp_exp = player.hp_exp.saturating_add(adjusted(base_stats.hp));
    player.attack_exp = player
        .attack_exp
        .saturating_add(adjusted(base_stats.attack));
    player.defense_exp = player
        .defense_exp
        .saturating_add(adjusted(base_stats.defense));
    player.speed_exp = player.speed_exp.saturating_add(adjusted(base_stats.speed));
    player.special_exp = player
        .special_exp
        .saturating_add(adjusted(base_stats.special_attack));
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
    level_up_happiness: BattleLevelUpHappinessContext,
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
        leveled.experience =
            calculate_experience(growth_rates, &leveled.species.growth_rate, leveled.level)?;
        refresh_level_stats(&mut leveled);
        let level_up_change = level_up_happiness.changes(&leveled);
        apply_happiness_change(&mut leveled, level_up_change);
        let level_moves = learn_moves_for_current_level(&mut leveled, moves, learnsets)?;
        for learned in level_moves.learned {
            learned_moves.push(learned.name);
        }
        pending_move_learns.extend(level_moves.pending);
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
    use crate::battle::stats::{BattleStatMultiplier, apply_stage};
    use crate::models::pokemon::CaughtData;
    use crate::models::{BaseStats, Dv, GrowthRate, growth_rate, pokemon_type};
    use crate::random::ReplayDivider;
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
            source_index: 1,
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
            mom_money_increment: 2_300,
            mom_random_items: vec![MomPurchaseRule {
                trigger: 0,
                cost: 600,
                kind: MomPurchaseKind::Item,
                target: "SUPER_POTION".to_string(),
                decoration_flag: None,
            }],
            mom_progression_items: vec![MomPurchaseRule {
                trigger: 900,
                cost: 600,
                kind: MomPurchaseKind::Item,
                target: "SUPER_POTION".to_string(),
                decoration_flag: None,
            }],
        }
    }

    fn battle_stat_multipliers() -> BattleStatMultiplierTables {
        BattleStatMultiplierTables {
            stat: [
                (25, 100),
                (28, 100),
                (33, 100),
                (40, 100),
                (50, 100),
                (66, 100),
                (1, 1),
                (15, 10),
                (2, 1),
                (25, 10),
                (3, 1),
                (35, 10),
                (4, 1),
            ]
            .into_iter()
            .map(|(numerator, denominator)| BattleStatMultiplier {
                numerator,
                denominator,
            })
            .collect(),
            accuracy: Vec::new(),
        }
    }

    fn level_up_happiness() -> BattleLevelUpHappinessContext {
        BattleLevelUpHappinessContext {
            current_landmark: 1,
            gain_level: [5, 3, 2],
            gain_level_at_home: [10, 6, 4],
        }
    }

    #[test]
    fn happiness_change_uses_source_thresholds_clamps_and_skips_eggs() {
        let changes = [5, 3, 2];
        let make_pokemon = || {
            Pokemon::new_for_tests(
                species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
                10,
                Dv::default(),
            )
        };
        let mut low = make_pokemon();
        low.happiness = 99;
        apply_happiness_change(&mut low, changes);
        assert_eq!(low.happiness, 104);

        let mut middle = make_pokemon();
        middle.happiness = 199;
        apply_happiness_change(&mut middle, changes);
        assert_eq!(middle.happiness, 202);

        let mut high = make_pokemon();
        high.happiness = 254;
        apply_happiness_change(&mut high, changes);
        assert_eq!(high.happiness, 255);

        let mut bitter = make_pokemon();
        bitter.happiness = 4;
        apply_happiness_change(&mut bitter, [-5, -5, -10]);
        assert_eq!(bitter.happiness, 0);

        let mut egg = make_pokemon();
        egg.happiness = 70;
        egg.is_egg = true;
        apply_happiness_change(&mut egg, changes);
        assert_eq!(egg.happiness, 70);
    }

    #[test]
    fn active_level_up_rebuilds_loaded_stats_after_stages_status_and_badges() {
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.status = Some("BURN".to_string());
        let enemy = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let mut badges = [false; 8];
        badges[0] = true;
        let mut combat = crate::battle::turn::BattleCombatState::new(player.clone(), enemy)
            .with_obedience(1, badges)
            .with_badge_boosts_enabled(true);
        combat
            .player
            .stat_boosts
            .insert(crate::models::Stat::Attack, 1);
        combat.player.confusion_turns = 4;

        let mut rewarded = player;
        rewarded.level += 1;
        rewarded.attack = 101;
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(rewarded.clone());
        state.script_runtime.active_battle_combat = Some(combat);

        let tables = battle_stat_multipliers();
        sync_active_combat_player_reward_from_storage(&mut state, &tables, true)
            .expect("active level-up refreshes loaded stats");

        let combat = state.script_runtime.active_battle_combat.as_ref().unwrap();
        let staged = apply_stage(&tables, rewarded.attack, 1).unwrap();
        let burned = (staged / 2).max(1);
        assert_eq!(combat.player_loaded_stats.attack, burned + burned / 8);
        assert_eq!(combat.player.level, rewarded.level);
        assert_eq!(combat.player.confusion_turns, 4);
        assert_eq!(combat.player.stat_boosts[&crate::models::Stat::Attack], 1);
        assert!(!combat.player_badge_before_status);
    }

    #[test]
    fn mom_progression_purchase_precedes_random_trigger_schedule() {
        let mut state = GameState::default();
        state.moms_money = 900;
        let mut divider = ReplayDivider::new([]);

        let selection = select_mom_purchase(&mut state, &reward_rules(), &mut divider)
            .expect("select Mom purchase")
            .expect("progression purchase");

        assert!(selection.progression);
        assert_eq!(selection.selected_index, 0);
        assert_eq!(selection.rule.target, "SUPER_POTION");
        assert_eq!(state.mom_item_trigger_balance, 0);
        assert_eq!(divider.consumed(), 0);
    }

    #[test]
    fn mom_random_purchase_advances_exact_trigger_and_uses_random_range() {
        let mut rules = reward_rules();
        rules.mom_random_items.push(MomPurchaseRule {
            trigger: 0,
            cost: 90,
            kind: MomPurchaseKind::Item,
            target: "ANTIDOTE".to_string(),
            decoration_flag: None,
        });
        let mut state = GameState::default();
        state.mom_item_index = 1;
        state.moms_money = 0;
        let mut divider = ReplayDivider::new([0, 0]);

        let selection = select_mom_purchase(&mut state, &rules, &mut divider)
            .expect("select random Mom purchase")
            .expect("random purchase");

        assert!(!selection.progression);
        assert_eq!(selection.selected_index, 1);
        assert_eq!(selection.rule.target, "ANTIDOTE");
        assert_eq!(state.mom_item_trigger_balance, 2_300);
        assert_eq!(divider.consumed(), 2);
    }

    #[test]
    fn deferred_mom_settlement_deducts_and_only_advances_progression_set() {
        let mut state = GameState::default();
        state.moms_money = 900;
        state.pending_mom_purchase = Some(PendingMomPurchase {
            progression: true,
            selected_index: 0,
            cost: 600,
            target: "SUPER_POTION".to_string(),
            decoration_flag: None,
        });

        let settled = settle_pending_mom_purchase(&mut state).expect("settle Mom purchase");

        assert_eq!(settled.cost, 600);
        assert_eq!(state.moms_money, 300);
        assert_eq!(state.mom_item_index, 1);
        assert_eq!(state.pending_mom_purchase, None);
    }

    #[test]
    fn reward_recipient_counts_reject_zero_and_unrepresentable_values() {
        assert_eq!(reward_recipient_count(1), Ok(1));
        assert_eq!(
            reward_recipient_count(0),
            Err(BattleRewardError::InvalidRecipientCount { count: 0 })
        );
        assert_eq!(
            reward_recipient_count(usize::MAX),
            Err(BattleRewardError::InvalidRecipientCount { count: usize::MAX })
        );
    }

    #[test]
    fn recipient_split_precedes_level_and_trainer_multipliers() {
        let defeated = Pokemon::new_for_tests(
            species("PIDGEY", 5, growth_rate("GROWTH_MEDIUM_FAST")),
            6,
            Dv::default(),
        );

        assert_eq!(
            split_experience_award(&reward_rules(), &defeated, 2, false),
            Ok(1)
        );
        assert_eq!(
            split_experience_award(&reward_rules(), &defeated, 2, true),
            Ok(1)
        );
        assert_eq!(
            wild_experience_award(&reward_rules(), &defeated).unwrap() / 2,
            2
        );
        assert_eq!(
            trainer_experience_award(&reward_rules(), &defeated).unwrap() / 2,
            3
        );
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
                BattleRewardRulesIssue::InvalidMomPurchaseRules {
                    reason: "mom_money_increment must be positive".to_string(),
                },
            ]
        );

        let rules = BattleRewardRules {
            max_level: 0,
            wild_exp_divisor: 0,
            trainer_exp_numerator: -1,
            trainer_exp_denominator: 0,
            ..reward_rules()
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
                level_up_happiness(),
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
            ..reward_rules()
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
                level_up_happiness(),
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
            level_up_happiness(),
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
    fn active_wild_battle_rewards_follow_party_slot_order_and_deactivate() {
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
        player.turns_in_battle = 1;
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
        state.storage.party.pokemon[0] = Some(player.clone());
        let mut active_player = player.clone();
        active_player.nickname = "ACTIVE".to_string();
        state.storage.party.pokemon[1] = Some(active_player);
        state.battle_active_party_index = Some(1);
        state.battle = BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            roaming_slot: None,
            enemy_pokemon: defeated.clone(),
            enemy_party: vec![defeated.clone()],
        };

        let mut divider = ReplayDivider::new([]);
        let outcome = claim_active_wild_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            level_up_happiness(),
            TimeOfDay::Day,
            &mut divider,
        )
        .expect("claim wild rewards");

        assert_eq!(outcome.defeated_species, "PIDGEY");
        assert_eq!(outcome.recipient_outcomes[0].party_index, 0);
        assert_eq!(outcome.recipient_outcomes[1].party_index, 1);
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
    fn wild_exp_share_finishes_both_reward_passes_before_evolution() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience =
            calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        player.item = Some("EXP_SHARE".to_string());
        player.turns_in_battle = 1;
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 255, growth_rate("GROWTH_MEDIUM_FAST")),
            100,
            Dv::default(),
        );
        defeated.hp = 0;
        let bayleef = species("BAYLEEF", 141, growth_rate("GROWTH_SLOW"));
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
            (bayleef.id.clone(), bayleef),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
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
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(player);
        state.battle_active_party_index = Some(0);
        state.battle = BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            roaming_slot: None,
            enemy_pokemon: defeated.clone(),
            enemy_party: vec![defeated],
        };

        let mut divider = ReplayDivider::new([]);
        let outcome = claim_active_wild_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            level_up_happiness(),
            TimeOfDay::Day,
            &mut divider,
        )
        .expect("claim both wild reward passes");

        let rewarded = state.storage.party.pokemon[0].as_ref().unwrap();
        assert_eq!(
            outcome
                .recipient_outcomes
                .iter()
                .map(|recipient| recipient.experience_awarded)
                .collect::<Vec<_>>(),
            vec![1_814, 1_814]
        );
        assert_eq!(rewarded.level, 19);
        assert_eq!(rewarded.species.id, "BAYLEEF");
        assert_eq!(outcome.evolution.target_species.as_deref(), Some("BAYLEEF"));
    }

    #[test]
    fn exp_share_pass_reuses_the_halved_enemy_record() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut participant = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            50,
            Dv::default(),
        );
        participant.turns_in_battle = 1;
        let mut second_participant = participant.clone();
        second_participant.nickname = "SECOND".to_string();
        let mut holder = participant.clone();
        holder.nickname = "HOLDER".to_string();
        holder.turns_in_battle = 0;
        holder.item = Some("EXP_SHARE".to_string());

        let mut defeated_species = species("PIDGEY", 65, growth_rate("GROWTH_MEDIUM_FAST"));
        defeated_species.base_stats = BaseStats::new(65, 65, 65, 65, 65, 65);
        let mut defeated = Pokemon::new_for_tests(defeated_species, 7, Dv::default());
        defeated.hp = 0;
        let species = [
            (participant.species.id.clone(), participant.species.clone()),
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
            [
                ("CHIKORITA".to_string(), Vec::new()),
                ("PIDGEY".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
        );
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(participant);
        state.storage.party.pokemon[1] = Some(second_participant);
        state.storage.party.pokemon[2] = Some(holder);
        state.battle_active_party_index = Some(1);
        state.battle = BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            roaming_slot: None,
            enemy_pokemon: defeated.clone(),
            enemy_party: vec![defeated],
        };

        let experience_before = state
            .storage
            .party
            .pokemon
            .iter()
            .take(3)
            .map(|pokemon| pokemon.as_ref().unwrap().experience)
            .collect::<Vec<_>>();
        let mut divider = ReplayDivider::new([]);
        let outcome = claim_active_wild_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            level_up_happiness(),
            TimeOfDay::Day,
            &mut divider,
        )
        .expect("claim split participant and Exp. Share rewards");

        assert_eq!(
            outcome
                .recipient_outcomes
                .iter()
                .map(|recipient| (recipient.party_index, recipient.experience_awarded))
                .collect::<Vec<_>>(),
            vec![(0, 16), (1, 16), (2, 32)]
        );
        for (index, expected_gain) in [(0, 16), (1, 16), (2, 32)] {
            let rewarded = state.storage.party.pokemon[index].as_ref().unwrap();
            assert_eq!(
                rewarded.experience - experience_before[index],
                expected_gain
            );
            assert_eq!(rewarded.hp_exp, expected_gain as u16);
            assert_eq!(rewarded.attack_exp, expected_gain as u16);
            assert_eq!(rewarded.defense_exp, expected_gain as u16);
            assert_eq!(rewarded.speed_exp, expected_gain as u16);
            assert_eq!(rewarded.special_exp, expected_gain as u16);
        }
    }

    #[test]
    fn active_trainer_battle_rewards_follow_party_slot_order_and_commit_enemy() {
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
        player.turns_in_battle = 1;
        let mut previous_participant = player.clone();
        previous_participant.nickname = "BENCH".to_string();
        state.storage.party.pokemon[0] = Some(player.clone());
        state.storage.party.pokemon[1] = Some(previous_participant.clone());
        state.battle_active_party_index = Some(1);
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
            enemy_party: vec![defeated.clone()],
            reward: 64,
            encounter_music: "MUSIC_YOUNGSTER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };
        state.script_runtime.active_battle_combat = Some(
            crate::battle::turn::BattleCombatState::new(
                previous_participant.clone(),
                defeated.clone(),
            )
            .with_parties(vec![player, previous_participant], vec![defeated])
            .with_party_indices(1, 0),
        );

        let mut tower_state = state.clone();
        let BattleMemory::Trainer { battle_type, .. } = &mut tower_state.battle else {
            unreachable!();
        };
        *battle_type = "BATTLETYPE_BATTLE_TOWER".to_string();
        let tower_experience_before = tower_state.storage.party.pokemon[1]
            .as_ref()
            .unwrap()
            .experience;
        let tower_outcome = claim_active_trainer_battle_rewards(
            &mut tower_state,
            &reward_rules(),
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            &battle_stat_multipliers(),
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("settle Battle Tower opponent");
        assert_eq!(tower_outcome.experience_awarded, 0);
        assert_eq!(
            tower_state.storage.party.pokemon[1]
                .as_ref()
                .unwrap()
                .experience,
            tower_experience_before
        );
        assert!(tower_state.battle_rewarded_enemy_party_indices.contains(&0));

        let outcome = claim_active_trainer_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &moves,
            &learnsets,
            &growth_rates,
            &evolutions,
            &battle_stat_multipliers(),
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("claim trainer rewards");

        assert_eq!(outcome.defeated_species, "PIDGEY");
        assert_eq!(outcome.recipient_outcomes.len(), 2);
        assert_eq!(outcome.recipient_outcomes[0].party_index, 0);
        assert_eq!(outcome.recipient_outcomes[1].party_index, 1);
        assert_eq!(outcome.recipient_outcomes[1].nickname, "BENCH");
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
        let rewarded = state.storage.party.pokemon[1].as_ref().unwrap();
        let combat = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .expect("active trainer combat remains between opponents");
        assert_eq!(combat.player.experience, rewarded.experience);
        assert_eq!(combat.player.level, rewarded.level);
        assert_eq!(combat.player_party[1], *rewarded);
        assert_eq!(rewarded.turns_in_battle, 1);
        assert_eq!(
            state.storage.party.pokemon[0]
                .as_ref()
                .unwrap()
                .turns_in_battle,
            0
        );
        assert_eq!(combat.player_party[0].turns_in_battle, 0);
    }

    #[test]
    fn trainer_level_evolution_waits_until_every_enemy_is_defeated() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience =
            calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        player.turns_in_battle = 1;
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let reserve_enemy = Pokemon::new_for_tests(
            species("RATTATA", 57, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let bayleef = species("BAYLEEF", 141, growth_rate("GROWTH_MEDIUM_FAST"));
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
            (
                reserve_enemy.species.id.clone(),
                reserve_enemy.species.clone(),
            ),
            (bayleef.id.clone(), bayleef),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
            ("BAYLEEF".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
            ("RATTATA".to_string(), Vec::new()),
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
                ("RATTATA".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
        );
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(player.clone());
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
            enemy_party: vec![defeated.clone(), reserve_enemy.clone()],
            reward: 64,
            encounter_music: "MUSIC_YOUNGSTER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };
        state.script_runtime.active_battle_combat = Some(
            crate::battle::turn::BattleCombatState::new(player.clone(), defeated.clone())
                .with_parties(vec![player], vec![defeated, reserve_enemy])
                .with_party_indices(0, 0),
        );

        let outcome = claim_active_trainer_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            &battle_stat_multipliers(),
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("claim first trainer opponent rewards");

        assert_eq!(outcome.level_after, 16);
        assert_eq!(outcome.evolution, EvolutionReport::default());
        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().species.id,
            "CHIKORITA"
        );
        crate::battle::start::advance_active_trainer_battle(&mut state)
            .expect("advance to second trainer opponent");
        let BattleMemory::Trainer { enemy_pokemon, .. } = &mut state.battle else {
            unreachable!();
        };
        enemy_pokemon.hp = 0;
        state
            .script_runtime
            .active_battle_combat
            .as_mut()
            .unwrap()
            .enemy
            .hp = 0;

        let final_outcome = claim_active_trainer_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            &battle_stat_multipliers(),
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("claim final trainer opponent rewards");

        assert_eq!(
            final_outcome.evolution.target_species.as_deref(),
            Some("BAYLEEF")
        );
        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().species.id,
            "BAYLEEF"
        );
        assert!(!state.pokedex.seen_species.contains("BAYLEEF"));
        assert!(!state.pokedex.caught_species.contains("BAYLEEF"));
        assert!(state.battle_evolvable_party_indices.is_empty());
    }

    #[test]
    fn final_trainer_reward_reports_an_earlier_participants_post_battle_evolution() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let chikorita = species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST"));
        let bayleef = species("BAYLEEF", 141, growth_rate("GROWTH_MEDIUM_FAST"));
        let pidgey = species("PIDGEY", 50, growth_rate("GROWTH_MEDIUM_FAST"));
        let earlier_participant = Pokemon::new_for_tests(chikorita.clone(), 16, Dv::default());
        let mut active = Pokemon::new_for_tests(pidgey.clone(), 10, Dv::default());
        active.turns_in_battle = 1;
        let mut defeated = Pokemon::new_for_tests(pidgey.clone(), 5, Dv::default());
        defeated.hp = 0;
        let species = [
            (chikorita.id.clone(), chikorita),
            (bayleef.id.clone(), bayleef),
            (pidgey.id.clone(), pidgey),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("CHIKORITA".to_string(), Vec::new()),
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
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(earlier_participant.clone());
        state.storage.party.pokemon[1] = Some(active.clone());
        state.battle_active_party_index = Some(1);
        state.battle_active_enemy_party_index = Some(0);
        state.battle_evolvable_party_indices.insert(0);
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
            enemy_party: vec![defeated.clone()],
            reward: 64,
            encounter_music: "MUSIC_YOUNGSTER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };
        state.script_runtime.active_battle_combat = Some(
            crate::battle::turn::BattleCombatState::new(active.clone(), defeated.clone())
                .with_parties(vec![earlier_participant, active], vec![defeated])
                .with_party_indices(1, 0),
        );

        let outcome = claim_active_trainer_battle_rewards(
            &mut state,
            &reward_rules(),
            &species,
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            &battle_stat_multipliers(),
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("claim final trainer rewards");

        assert_eq!(outcome.recipient_outcomes.len(), 1);
        assert_eq!(outcome.recipient_outcomes[0].party_index, 1);
        assert_eq!(outcome.post_battle_evolutions.len(), 1);
        assert_eq!(outcome.post_battle_evolutions[0].party_index, 0);
        assert_eq!(outcome.post_battle_evolutions[0].nickname, "CHIKORITA");
        assert_eq!(
            outcome.post_battle_evolutions[0]
                .evolution
                .target_species
                .as_deref(),
            Some("BAYLEEF")
        );
        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().species.id,
            "BAYLEEF"
        );
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
            level_up_happiness(),
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
    fn battle_level_up_happiness_applies_once_after_all_gained_levels() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        player.happiness = 99;
        player.caught_data = Some(CaughtData {
            level: 5,
            time_of_day: Some(TimeOfDay::Day),
            original_trainer_gender: 0,
            location: 1,
        });
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 1_000, growth_rate("GROWTH_MEDIUM_FAST")),
            100,
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
            [
                ("CHIKORITA".to_string(), Vec::new()),
                ("PIDGEY".to_string(), Vec::new()),
            ]
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
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("trainer rewards");

        assert!(outcome.level_after > outcome.level_before + 1);
        assert_eq!(player.happiness, 109);
    }

    #[test]
    fn absent_caught_data_compares_as_crystals_zero_location_byte() {
        let player = Pokemon::new_for_tests(
            species("CHIKORITA", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let context = BattleLevelUpHappinessContext {
            current_landmark: 0,
            ..level_up_happiness()
        };

        assert_eq!(
            happiness_delta(player.happiness, context.changes(&player)),
            10
        );
    }

    #[test]
    fn level_up_happiness_is_applied_before_happiness_evolution() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("EEVEE", 92, growth_rate("GROWTH_MEDIUM_FAST")),
            15,
            Dv::default(),
        );
        player.experience =
            calculate_experience(&growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        player.happiness = 219;
        let mut defeated = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        defeated.hp = 0;
        let espeon = species("ESPEON", 197, growth_rate("GROWTH_MEDIUM_FAST"));
        let species = [
            (player.species.id.clone(), player.species.clone()),
            (defeated.species.id.clone(), defeated.species.clone()),
            (espeon.id.clone(), espeon),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            ("EEVEE".to_string(), Vec::new()),
            ("ESPEON".to_string(), Vec::new()),
            ("PIDGEY".to_string(), Vec::new()),
        ]
        .into_iter()
        .collect();
        let evolutions = EvolutionTable(
            [
                (
                    "EEVEE".to_string(),
                    vec![EvolutionEntry::happiness("ESPEON", "TR_MORNDAY")],
                ),
                ("ESPEON".to_string(), Vec::new()),
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
            &BTreeMap::new(),
            &learnsets,
            &growth_rates,
            &evolutions,
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("wild rewards");

        assert_eq!(player.happiness, 221);
        assert_eq!(outcome.evolution.target_species.as_deref(), Some("ESPEON"));
        assert_eq!(player.species.id, "ESPEON");
    }

    #[test]
    fn rewards_leave_calculated_stats_stale_after_stat_exp_without_level_up() {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let mut player = Pokemon::new_for_tests(
            species("TYPHLOSION", 64, growth_rate("GROWTH_MEDIUM_FAST")),
            60,
            Dv::from_non_hp(10, 10, 10, 10),
        );
        let level_before = player.level;
        let stats_before = (
            player.max_hp,
            player.attack,
            player.defense,
            player.speed,
            player.special_attack,
            player.special_defense,
        );
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
            level_up_happiness(),
            TimeOfDay::Day,
        )
        .expect("trainer rewards");

        assert_eq!(outcome.level_before, level_before);
        assert_eq!(outcome.level_after, level_before);
        assert_eq!(player.happiness, 70);
        assert!(player.hp_exp > 0);
        player
            .validate_saved_state()
            .expect("valid rewarded Pokemon");
        assert_eq!(
            (
                player.max_hp,
                player.attack,
                player.defense,
                player.speed,
                player.special_attack,
                player.special_defense,
            ),
            stats_before,
            "Crystal stores new stat experience immediately but does not call CalcMonStats until a level is gained"
        );
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
                level_up_happiness(),
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
                level_up_happiness(),
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
                level_up_happiness(),
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
    fn pending_move_replacement_refreshes_retained_trainer_combat_moves() {
        let mut state = pending_move_learn_state();
        let player = state.storage.party.pokemon[0].as_ref().unwrap().clone();
        let enemy = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let mut combat = crate::battle::turn::BattleCombatState::new(player.clone(), enemy.clone())
            .with_parties(vec![player], vec![enemy])
            .with_party_indices(0, 0);
        combat
            .player
            .stat_boosts
            .insert(crate::models::Stat::Attack, 2);
        combat.player.confusion_turns = 3;
        combat.player_loaded_stats.attack = 777;
        combat.player_disable = Some(crate::battle::turn::BattleDisableState {
            move_name: "GROWL".to_string(),
            turns_remaining: 4,
        });
        state.script_runtime.active_battle_combat = Some(combat);

        replace_pending_move_learn(&mut state, 1).expect("replace pending move");

        let stored = state.storage.party.pokemon[0].as_ref().unwrap();
        assert_eq!(stored.moves[1].name, "RAZOR_LEAF");
        let combat = state.script_runtime.active_battle_combat.as_ref().unwrap();
        assert_eq!(combat.player.moves, stored.moves);
        assert_eq!(combat.player_party[0].moves, stored.moves);
        assert_eq!(combat.player.stat_boosts[&crate::models::Stat::Attack], 2);
        assert_eq!(combat.player.confusion_turns, 3);
        assert_eq!(combat.player_loaded_stats.attack, 777);
        assert_eq!(combat.player_disable, None);
    }

    #[test]
    fn pending_move_replacement_keeps_disable_for_a_different_move() {
        let mut state = pending_move_learn_state();
        let player = state.storage.party.pokemon[0].as_ref().unwrap().clone();
        let enemy = Pokemon::new_for_tests(
            species("PIDGEY", 91, growth_rate("GROWTH_MEDIUM_FAST")),
            5,
            Dv::default(),
        );
        let mut combat = crate::battle::turn::BattleCombatState::new(player.clone(), enemy.clone())
            .with_parties(vec![player], vec![enemy])
            .with_party_indices(0, 0);
        combat.player_disable = Some(crate::battle::turn::BattleDisableState {
            move_name: "TACKLE".to_string(),
            turns_remaining: 4,
        });
        state.script_runtime.active_battle_combat = Some(combat);

        replace_pending_move_learn(&mut state, 1).expect("replace non-disabled move");

        assert_eq!(
            state
                .script_runtime
                .active_battle_combat
                .as_ref()
                .unwrap()
                .player_disable,
            Some(crate::battle::turn::BattleDisableState {
                move_name: "TACKLE".to_string(),
                turns_remaining: 4,
            })
        );
    }

    #[test]
    fn pending_move_learns_queue_and_promote_in_reward_order() {
        let mut state = pending_move_learn_state();
        let queued_move = LearnedMove {
            name: "SYNTHESIS".to_string(),
            current_pp: 5,
            pp_ups: 0,
        };
        let outcome = BattleRewardOutcome {
            defeated_species: "PIDGEY".to_string(),
            experience_awarded: 1,
            level_before: 16,
            level_after: 17,
            learned_moves: Vec::new(),
            pending_move_learns: vec![queued_move.clone()],
            deferred_level_evolution: true,
            evolution: EvolutionReport::default(),
            recipient_outcomes: Vec::new(),
            post_battle_evolutions: Vec::new(),
        };

        queue_pending_move_learn(&mut state, 0, &outcome).expect("queue second move learn");

        assert_eq!(
            state.pending_move_learn.as_ref().unwrap().learned_move.name,
            "RAZOR_LEAF"
        );
        assert_eq!(state.pending_move_learn_queue.len(), 1);
        assert_eq!(state.pending_move_learn_queue[0].learned_move, queued_move);
        assert!(state.pending_move_learn_queue[0].defer_level_evolution);

        decline_pending_move_learn(&mut state).expect("resolve first move learn");
        promote_next_pending_move_learn(&mut state);
        assert_eq!(
            state.pending_move_learn.as_ref().unwrap().learned_move.name,
            "SYNTHESIS"
        );
        assert!(state.pending_move_learn_queue.is_empty());

        let evolved = state.storage.party.pokemon[0].as_mut().unwrap();
        evolved.species = species("BAYLEEF", 141, growth_rate("GROWTH_MEDIUM_FAST"));
        evolved.level = 17;
        rebase_pending_move_learns_for_party(&mut state, 0, true);
        let pending = state.pending_move_learn.as_ref().unwrap();
        assert_eq!(pending.species_id, "BAYLEEF");
        assert_eq!(pending.level, 17);
        assert!(!pending.defer_level_evolution);
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
