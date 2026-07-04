use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use thiserror::Error;

use crate::battle::start::deactivate_battle;
use crate::models::{Bag, CaptureStorageLocation, Item, PokedexState, Pokemon, PokemonStorage};
use crate::random::Random;
use crate::state::{BattleMemory, GameState};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRules {
    pub fast_ball_species: BTreeSet<String>,
    pub heavy_ball_modifiers: BTreeMap<String, i16>,
    pub ball_rules: BTreeMap<String, CaptureBallRule>,
    pub guaranteed_capture_balls: BTreeSet<String>,
    pub status_bonus: BTreeMap<String, u8>,
}

const BATTLE_ONLY_CAPTURE_BALLS: &[&str] = &["SAFARI_BALL"];

impl<'de> Deserialize<'de> for CaptureRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCaptureRules {
            fast_ball_species: BTreeSet<String>,
            heavy_ball_modifiers: BTreeMap<String, i16>,
            ball_rules: BTreeMap<String, CaptureBallRule>,
            guaranteed_capture_balls: BTreeSet<String>,
            status_bonus: BTreeMap<String, u8>,
        }

        let raw = RawCaptureRules::deserialize(deserializer)?;
        let rules = Self {
            fast_ball_species: raw.fast_ball_species,
            heavy_ball_modifiers: raw.heavy_ball_modifiers,
            ball_rules: raw.ball_rules,
            guaranteed_capture_balls: raw.guaranteed_capture_balls,
            status_bonus: raw.status_bonus,
        };
        rules.validate_shape().map_err(D::Error::custom)?;
        Ok(rules)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureBallRule {
    pub multiplier_numerator: u16,
    pub multiplier_denominator: u16,
    pub battle_type: String,
    pub skip_hp_calc: bool,
    pub use_heavy_ball_weight_modifier: bool,
    pub use_level_ball_multiplier: bool,
    pub require_same_species: bool,
    pub require_same_gender: bool,
    pub require_fast_species: bool,
}

impl<'de> Deserialize<'de> for CaptureBallRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCaptureBallRule {
            multiplier_numerator: u16,
            multiplier_denominator: u16,
            battle_type: String,
            skip_hp_calc: bool,
            use_heavy_ball_weight_modifier: bool,
            use_level_ball_multiplier: bool,
            require_same_species: bool,
            require_same_gender: bool,
            require_fast_species: bool,
        }

        let raw = RawCaptureBallRule::deserialize(deserializer)?;
        let rule = Self {
            multiplier_numerator: raw.multiplier_numerator,
            multiplier_denominator: raw.multiplier_denominator,
            battle_type: raw.battle_type,
            skip_hp_calc: raw.skip_hp_calc,
            use_heavy_ball_weight_modifier: raw.use_heavy_ball_weight_modifier,
            use_level_ball_multiplier: raw.use_level_ball_multiplier,
            require_same_species: raw.require_same_species,
            require_same_gender: raw.require_same_gender,
            require_fast_species: raw.require_fast_species,
        };
        rule.validate_shape().map_err(D::Error::custom)?;
        Ok(rule)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CaptureBallRuleIssue {
    InvalidBallId,
    InvalidBattleType,
    InvalidMultiplierDenominator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CaptureRulesIssue {
    MissingBallRules,
    InvalidFastBallSpecies {
        species: String,
    },
    UnknownFastBallSpecies {
        species: String,
    },
    InvalidHeavyBallSpecies {
        species: String,
    },
    UnknownHeavyBallSpecies {
        species: String,
    },
    InvalidBallRuleItem {
        ball_id: String,
    },
    UnknownBallRuleItem {
        ball_id: String,
    },
    UnusableBallRuleItem {
        ball_id: String,
    },
    InvalidGuaranteedCaptureBall {
        ball_id: String,
    },
    UnknownGuaranteedCaptureBall {
        ball_id: String,
    },
    UnusableGuaranteedCaptureBall {
        ball_id: String,
    },
    InvalidBallRule {
        ball_id: String,
        issue: CaptureBallRuleIssue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CaptureWobbleProbabilityIssue {
    MissingTable,
    InvalidCatchRate,
    UnorderedCatchRate { catch_rate: u8, previous: u8 },
    IncompleteTable,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureWobbleProbability {
    pub catch_rate: u8,
    pub chance: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct CaptureWobbleProbabilityTable(pub Vec<CaptureWobbleProbability>);

impl<'de> Deserialize<'de> for CaptureWobbleProbabilityTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let probabilities = Vec::<CaptureWobbleProbability>::deserialize(deserializer)?;
        if let Some(issue) = capture_wobble_probability_issues(&probabilities, true)
            .into_iter()
            .next()
        {
            return Err(D::Error::custom(format!(
                "invalid capture wobble probability table: {issue:?}"
            )));
        }
        Ok(Self(probabilities))
    }
}

impl<'de> Deserialize<'de> for CaptureWobbleProbability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCaptureWobbleProbability {
            catch_rate: u8,
            chance: u8,
        }

        let raw = RawCaptureWobbleProbability::deserialize(deserializer)?;
        let probability = Self {
            catch_rate: raw.catch_rate,
            chance: raw.chance,
        };
        probability.validate_shape().map_err(D::Error::custom)?;
        Ok(probability)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    #[error("missing capture rules")]
    MissingRules,
    #[error("missing capture wobble probability table")]
    MissingWobbleTable,
    #[error("missing Heavy Ball modifier for species '{0}'")]
    MissingHeavyBallModifier(String),
    #[error("invalid capture ball '{0}'")]
    InvalidBall(String),
    #[error("invalid capture context {field} '{value}'")]
    InvalidCaptureContext { field: String, value: String },
    #[error("unknown capture ball '{0}'")]
    UnknownBall(String),
    #[error("invalid capture ball rule for '{ball_id}': {message}")]
    InvalidBallRule { ball_id: String, message: String },
    #[error("missing capture wobble probability for catch rate {0}")]
    MissingWobbleProbability(u8),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureUseError {
    #[error("{0}")]
    Bag(String),
    #[error(transparent)]
    Capture(#[from] CaptureError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureAttemptContext {
    pub ball_id: String,
    pub battle_type: String,
    pub trainer_battle: bool,
    #[serde(deserialize_with = "required_nullable_string")]
    pub player_gender: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub enemy_gender: Option<String>,
}

impl<'de> Deserialize<'de> for CaptureAttemptContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCaptureAttemptContext {
            ball_id: String,
            battle_type: String,
            trainer_battle: bool,
            #[serde(deserialize_with = "required_nullable_string")]
            player_gender: Option<String>,
            #[serde(deserialize_with = "required_nullable_string")]
            enemy_gender: Option<String>,
        }

        let raw = RawCaptureAttemptContext::deserialize(deserializer)?;
        let context = Self {
            ball_id: raw.ball_id,
            battle_type: raw.battle_type,
            trainer_battle: raw.trainer_battle,
            player_gender: raw.player_gender,
            enemy_gender: raw.enemy_gender,
        };
        validate_capture_attempt_context(&context).map_err(serde::de::Error::custom)?;
        Ok(context)
    }
}

fn required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

pub fn validate_capture_attempt_context(
    context: &CaptureAttemptContext,
) -> Result<(), CaptureError> {
    validate_capture_ball_id(&context.ball_id)?;
    validate_capture_context_token("battle type", &context.battle_type)?;
    if let Some(gender) = context.player_gender.as_deref() {
        validate_capture_context_token("player gender", gender)?;
    }
    if let Some(gender) = context.enemy_gender.as_deref() {
        validate_capture_context_token("enemy gender", gender)?;
    }
    Ok(())
}

fn validate_capture_context_token(field: &str, value: &str) -> Result<(), CaptureError> {
    if is_exact_capture_token(value) {
        Ok(())
    } else {
        Err(CaptureError::InvalidCaptureContext {
            field: field.to_string(),
            value: value.to_string(),
        })
    }
}

impl CaptureAttemptContext {
    pub fn wild(ball_id: impl Into<String>) -> Self {
        Self {
            ball_id: ball_id.into(),
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            trainer_battle: false,
            player_gender: None,
            enemy_gender: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallCatchRateResult {
    pub rate: u8,
    pub skip_hp_calc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureOutcome {
    pub caught: bool,
    pub blocked: bool,
    pub wobble_count: u8,
    pub animation_shakes: u8,
    pub final_catch_rate: u8,
    pub rng_seed_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredCapture {
    pub pokemon: Pokemon,
    pub location: CaptureStorageLocation,
}

pub fn resolve_capture_attempt(
    player: &Pokemon,
    enemy: &Pokemon,
    context: &CaptureAttemptContext,
    rules: &CaptureRules,
    wobble_probabilities: &[CaptureWobbleProbability],
    rng: &mut Random,
) -> Result<CaptureOutcome, CaptureError> {
    validate_capture_attempt_context(context)?;
    require_capture_runtime_rules(rules, wobble_probabilities)?;
    if context.trainer_battle {
        return Ok(CaptureOutcome {
            caught: false,
            blocked: true,
            wobble_count: 0,
            animation_shakes: 0,
            final_catch_rate: 0,
            rng_seed_after: rng.seed(),
        });
    }

    if rules.guaranteed_capture_balls.contains(&context.ball_id) {
        return Ok(CaptureOutcome {
            caught: true,
            blocked: false,
            wobble_count: 3,
            animation_shakes: 4,
            final_catch_rate: 255,
            rng_seed_after: rng.seed(),
        });
    }

    let final_catch_rate = compute_final_catch_rate(player, enemy, context, rules)?;
    let roll = rng.randrange(256) as u8;
    if roll <= final_catch_rate {
        return Ok(CaptureOutcome {
            caught: true,
            blocked: false,
            wobble_count: 3,
            animation_shakes: 4,
            final_catch_rate,
            rng_seed_after: rng.seed(),
        });
    }

    let wobble_chance = wobble_chance_for_rate(final_catch_rate, wobble_probabilities)?;
    let mut wobble_count = 0;
    for _ in 0..3 {
        if (rng.randrange(256) as u8) < wobble_chance {
            wobble_count += 1;
        } else {
            break;
        }
    }

    Ok(CaptureOutcome {
        caught: false,
        blocked: false,
        wobble_count,
        animation_shakes: wobble_count,
        final_catch_rate,
        rng_seed_after: rng.seed(),
    })
}

pub fn throw_ball_from_bag(
    bag: &mut Bag,
    ball: &Item,
    player: &Pokemon,
    enemy: &Pokemon,
    mut context: CaptureAttemptContext,
    rules: &CaptureRules,
    wobble_probabilities: &[CaptureWobbleProbability],
    rng: &mut Random,
) -> Result<Option<CaptureOutcome>, CaptureUseError> {
    validate_capture_ball_item(rules, ball)?;
    context.ball_id = ball.script_name.clone();
    if context.trainer_battle {
        if bag.quantity(ball) == 0 {
            return Ok(None);
        }
        return Ok(Some(resolve_capture_attempt(
            player,
            enemy,
            &context,
            rules,
            wobble_probabilities,
            rng,
        )?));
    }
    if !bag.consume_ball(ball).map_err(CaptureUseError::Bag)? {
        return Ok(None);
    }
    Ok(Some(resolve_capture_attempt(
        player,
        enemy,
        &context,
        rules,
        wobble_probabilities,
        rng,
    )?))
}

pub fn validate_capture_ball_item(rules: &CaptureRules, ball: &Item) -> Result<(), CaptureError> {
    require_capture_rules(rules)?;
    validate_capture_ball_id(&ball.script_name)?;
    if rules.ball_rules.contains_key(&ball.script_name)
        || rules.guaranteed_capture_balls.contains(&ball.script_name)
    {
        return Ok(());
    }
    Err(CaptureError::UnknownBall(ball.script_name.clone()))
}

fn validate_capture_ball_id(ball_id: &str) -> Result<(), CaptureError> {
    if !is_exact_capture_token(ball_id) {
        return Err(CaptureError::InvalidBall(ball_id.to_string()));
    }
    Ok(())
}

pub fn capture_ball_rule_issues(
    ball_id: &str,
    rule: &CaptureBallRule,
) -> Vec<CaptureBallRuleIssue> {
    let mut issues = Vec::new();
    if !is_exact_capture_token(ball_id) {
        issues.push(CaptureBallRuleIssue::InvalidBallId);
    }
    if !rule.battle_type.is_empty() && !is_exact_capture_token(&rule.battle_type) {
        issues.push(CaptureBallRuleIssue::InvalidBattleType);
    }
    if rule.multiplier_denominator == 0 {
        issues.push(CaptureBallRuleIssue::InvalidMultiplierDenominator);
    }
    issues
}

pub fn capture_rules_issues(
    rules: &CaptureRules,
    species_ids: &BTreeSet<String>,
    ball_items: &BTreeMap<String, Item>,
    has_ball_pocket_items: bool,
) -> Vec<CaptureRulesIssue> {
    let mut issues = Vec::new();
    if has_ball_pocket_items && rules.ball_rules.is_empty() {
        issues.push(CaptureRulesIssue::MissingBallRules);
    }
    for species in &rules.fast_ball_species {
        if !is_exact_capture_token(species) {
            issues.push(CaptureRulesIssue::InvalidFastBallSpecies {
                species: species.clone(),
            });
        } else if !species_ids.contains(species) {
            issues.push(CaptureRulesIssue::UnknownFastBallSpecies {
                species: species.clone(),
            });
        }
    }
    for species in rules.heavy_ball_modifiers.keys() {
        if !is_exact_capture_token(species) {
            issues.push(CaptureRulesIssue::InvalidHeavyBallSpecies {
                species: species.clone(),
            });
        } else if !species_ids.contains(species) {
            issues.push(CaptureRulesIssue::UnknownHeavyBallSpecies {
                species: species.clone(),
            });
        }
    }
    for (ball_id, rule) in &rules.ball_rules {
        if !is_exact_capture_token(ball_id) {
            issues.push(CaptureRulesIssue::InvalidBallRuleItem {
                ball_id: ball_id.clone(),
            });
        } else if !ball_items.is_empty() && !BATTLE_ONLY_CAPTURE_BALLS.contains(&ball_id.as_str()) {
            match ball_items.get(ball_id) {
                Some(item) if !item.battle_usable => {
                    issues.push(CaptureRulesIssue::UnusableBallRuleItem {
                        ball_id: ball_id.clone(),
                    });
                }
                Some(_) => {}
                None => issues.push(CaptureRulesIssue::UnknownBallRuleItem {
                    ball_id: ball_id.clone(),
                }),
            }
        }
        for issue in capture_ball_rule_issues(ball_id, rule) {
            issues.push(CaptureRulesIssue::InvalidBallRule {
                ball_id: ball_id.clone(),
                issue,
            });
        }
    }
    for ball_id in &rules.guaranteed_capture_balls {
        if !is_exact_capture_token(ball_id) {
            issues.push(CaptureRulesIssue::InvalidGuaranteedCaptureBall {
                ball_id: ball_id.clone(),
            });
        } else if !ball_items.is_empty() {
            match ball_items.get(ball_id) {
                Some(item) if !item.battle_usable => {
                    issues.push(CaptureRulesIssue::UnusableGuaranteedCaptureBall {
                        ball_id: ball_id.clone(),
                    });
                }
                Some(_) => {}
                None => issues.push(CaptureRulesIssue::UnknownGuaranteedCaptureBall {
                    ball_id: ball_id.clone(),
                }),
            }
        }
    }
    issues
}

fn is_exact_capture_token(value: &str) -> bool {
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

impl CaptureRules {
    fn validate_shape(&self) -> Result<(), String> {
        for species in &self.fast_ball_species {
            validate_exact_capture_shape_token("fast ball species", species)?;
        }
        for species in self.heavy_ball_modifiers.keys() {
            validate_exact_capture_shape_token("heavy ball species", species)?;
        }
        for (ball_id, rule) in &self.ball_rules {
            validate_capture_ball_rule_shape(ball_id, rule).map_err(|error| error.to_string())?;
        }
        for ball_id in &self.guaranteed_capture_balls {
            validate_exact_capture_shape_token("guaranteed capture ball", ball_id)?;
        }
        for (status, bonus) in &self.status_bonus {
            validate_exact_capture_shape_token("capture status bonus", status)?;
            if *bonus == 0 {
                return Err(format!(
                    "capture status bonus for {status} must be positive"
                ));
            }
        }
        Ok(())
    }
}

impl CaptureBallRule {
    fn validate_shape(&self) -> Result<(), String> {
        if self.multiplier_denominator == 0 {
            return Err("capture ball rule multiplier denominator must be nonzero".to_string());
        }
        if self.multiplier_numerator == 0 {
            return Err("capture ball rule multiplier numerator must be nonzero".to_string());
        }
        if !self.battle_type.is_empty() {
            validate_exact_capture_shape_token("capture battle type", &self.battle_type)?;
        }
        Ok(())
    }
}

impl CaptureWobbleProbability {
    fn validate_shape(&self) -> Result<(), String> {
        if self.catch_rate == 0 {
            return Err("capture wobble catch_rate must be positive".to_string());
        }
        Ok(())
    }
}

fn validate_exact_capture_shape_token(subject: &str, value: &str) -> Result<(), String> {
    if is_exact_capture_token(value) {
        Ok(())
    } else {
        Err(format!("{subject} {value:?} is not exact"))
    }
}

pub fn validate_capture_ball_rule_shape(
    ball_id: &str,
    rule: &CaptureBallRule,
) -> Result<(), CaptureError> {
    let Some(issue) = capture_ball_rule_issues(ball_id, rule).into_iter().next() else {
        return Ok(());
    };
    let message = match issue {
        CaptureBallRuleIssue::InvalidBallId => "ball id must be an exact nonempty id",
        CaptureBallRuleIssue::InvalidBattleType => "battle type must be exact when present",
        CaptureBallRuleIssue::InvalidMultiplierDenominator => {
            "multiplier denominator must be nonzero"
        }
    };
    Err(CaptureError::InvalidBallRule {
        ball_id: ball_id.to_string(),
        message: message.to_string(),
    })
}

pub fn capture_wobble_probability_issues(
    probabilities: &[CaptureWobbleProbability],
    has_ball_pocket_items: bool,
) -> Vec<CaptureWobbleProbabilityIssue> {
    if !has_ball_pocket_items {
        return Vec::new();
    }
    if probabilities.is_empty() {
        return vec![CaptureWobbleProbabilityIssue::MissingTable];
    }
    let mut issues = Vec::new();
    let mut previous = 0;
    for entry in probabilities {
        if entry.catch_rate == 0 {
            issues.push(CaptureWobbleProbabilityIssue::InvalidCatchRate);
        }
        if entry.catch_rate < previous {
            issues.push(CaptureWobbleProbabilityIssue::UnorderedCatchRate {
                catch_rate: entry.catch_rate,
                previous,
            });
        }
        previous = entry.catch_rate;
    }
    if previous != u8::MAX {
        issues.push(CaptureWobbleProbabilityIssue::IncompleteTable);
    }
    issues
}

pub fn store_captured_pokemon(
    outcome: &CaptureOutcome,
    storage: &mut PokemonStorage,
    pokemon: Pokemon,
) -> Result<Option<StoredCapture>, String> {
    if !outcome.caught {
        return Ok(None);
    }
    let location = storage.register_capture(pokemon.clone())?;
    Ok(Some(StoredCapture { pokemon, location }))
}

pub fn complete_captured_pokemon(
    outcome: &CaptureOutcome,
    storage: &mut PokemonStorage,
    pokedex: &mut PokedexState,
    pokemon: Pokemon,
) -> Result<Option<StoredCapture>, String> {
    let stored = store_captured_pokemon(outcome, storage, pokemon)?;
    if let Some(stored) = &stored {
        pokedex.record_caught_pokemon(&stored.pokemon);
    }
    Ok(stored)
}

pub fn complete_active_wild_capture(
    state: &mut GameState,
    outcome: &CaptureOutcome,
) -> Result<Option<StoredCapture>, String> {
    if !outcome.caught {
        return Err("cannot complete capture from an uncaught capture outcome".to_string());
    }
    if outcome.blocked {
        return Err("cannot complete capture from a blocked capture outcome".to_string());
    }
    if outcome.rng_seed_after != state.rng_seed {
        return Err(format!(
            "capture outcome rng seed {} does not match saved rng seed {}",
            outcome.rng_seed_after, state.rng_seed
        ));
    }
    let enemy_pokemon = match &state.battle {
        BattleMemory::Wild { enemy_pokemon, .. }
        | BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
        BattleMemory::Trainer { trainer_id, .. } => {
            return Err(format!("cannot capture during trainer battle {trainer_id}"));
        }
        BattleMemory::Inactive => {
            return Err("cannot complete capture without an active wild battle".to_string());
        }
    };
    let stored = complete_captured_pokemon(
        outcome,
        &mut state.storage,
        &mut state.pokedex,
        enemy_pokemon,
    )?;
    if stored.is_some() {
        state.battle_result |= 1 << 6;
        deactivate_battle(state);
    }
    state.sync_party_from_storage();
    Ok(stored)
}

pub fn compute_final_catch_rate(
    player: &Pokemon,
    enemy: &Pokemon,
    context: &CaptureAttemptContext,
    rules: &CaptureRules,
) -> Result<u8, CaptureError> {
    require_capture_rules(rules)?;
    let rate_result = apply_ball_multiplier(&context.ball_id, player, enemy, context, rules)?;
    if rate_result.skip_hp_calc {
        return Ok(rate_result.rate);
    }
    let mut final_rate = compute_hp_adjusted_catch_rate(rate_result.rate, enemy.hp, enemy.max_hp);
    if let Some(status) = enemy.status.as_deref() {
        if let Some(bonus) = rules.status_bonus.get(status) {
            final_rate = clamp_catch_rate(final_rate as i32 + i32::from(*bonus), 1);
        }
    }
    Ok(clamp_catch_rate(final_rate as i32, 1))
}

pub fn apply_ball_multiplier(
    ball_id: &str,
    player: &Pokemon,
    enemy: &Pokemon,
    context: &CaptureAttemptContext,
    rules: &CaptureRules,
) -> Result<BallCatchRateResult, CaptureError> {
    require_capture_rules(rules)?;
    validate_capture_ball_id(ball_id)?;
    let mut rate = clamp_catch_rate(enemy.species.catch_rate as i32, 0);
    let mut skip_hp_calc = false;

    let rule = rules
        .ball_rules
        .get(ball_id)
        .ok_or_else(|| CaptureError::UnknownBall(ball_id.to_string()))?;

    if rule.use_heavy_ball_weight_modifier {
        let modifier = rules
            .heavy_ball_modifiers
            .get(&enemy.species.id)
            .copied()
            .ok_or_else(|| CaptureError::MissingHeavyBallModifier(enemy.species.id.clone()))?;
        rate = clamp_catch_rate(rate as i32 + modifier as i32, 1);
    }

    if rule.use_level_ball_multiplier {
        let player_level = player.level;
        let enemy_level = enemy.level.max(1);
        if player_level > enemy_level {
            rate = clamp_catch_rate((rate as i32) << 1, 0);
            if (player_level >> 1) > enemy_level {
                rate = clamp_catch_rate((rate as i32) << 1, 0);
                if (player_level >> 2) > enemy_level {
                    rate = clamp_catch_rate((rate as i32) << 1, 0);
                }
            }
        }
    }

    let battle_type_matches =
        rule.battle_type.is_empty() || context.battle_type == rule.battle_type;
    let species_matches = !rule.require_same_species || player.species.id == enemy.species.id;
    let gender_matches = !rule.require_same_gender
        || (context.player_gender.is_some() && context.player_gender == context.enemy_gender);
    let fast_species_matches =
        !rule.require_fast_species || rules.fast_ball_species.contains(&enemy.species.id);
    if battle_type_matches
        && species_matches
        && gender_matches
        && fast_species_matches
        && (rule.multiplier_numerator != rule.multiplier_denominator
            || rule.multiplier_denominator == 0)
    {
        rate = apply_rule_multiplier(ball_id, rate, rule)?;
    }

    if rule.skip_hp_calc {
        skip_hp_calc = true;
    }
    Ok(BallCatchRateResult { rate, skip_hp_calc })
}

fn apply_rule_multiplier(
    ball_id: &str,
    rate: u8,
    rule: &CaptureBallRule,
) -> Result<u8, CaptureError> {
    if rule.multiplier_denominator == 0 {
        return Err(CaptureError::InvalidBallRule {
            ball_id: ball_id.to_string(),
            message: "multiplier denominator must be nonzero".to_string(),
        });
    }
    let multiplied = u32::from(rate) * u32::from(rule.multiplier_numerator);
    Ok(clamp_catch_rate(
        (multiplied / u32::from(rule.multiplier_denominator)) as i32,
        0,
    ))
}

pub fn compute_hp_adjusted_catch_rate(catch_rate: u8, hp: u16, max_hp: u16) -> u8 {
    let hp_value = hp;
    let max_value = max_hp.max(1);
    let mut hp2 = hp_value.wrapping_mul(2);
    let mut max3 = max_value.wrapping_mul(3);
    if (max3 & 0xff00) != 0 {
        hp2 >>= 2;
        max3 >>= 2;
    }
    let mut hp_low = (hp2 & 0x00ff) as u8;
    if hp_low == 0 {
        hp_low = 1;
    }
    let max_low = (max3 & 0x00ff) as u8;
    assert!(max_low != 0, "catch divisor is zero for max HP {max_value}");
    let diff = max_low.wrapping_sub(hp_low);
    let product = catch_rate as u16 * diff as u16;
    let mut result = (product / max_low as u16) as u8;
    if result == 0 {
        result = 1;
    }
    result
}

pub fn wobble_chance_for_rate(
    final_catch_rate: u8,
    probabilities: &[CaptureWobbleProbability],
) -> Result<u8, CaptureError> {
    if probabilities.is_empty() {
        return Err(CaptureError::MissingWobbleTable);
    }
    for entry in probabilities {
        if final_catch_rate <= entry.catch_rate {
            return Ok(entry.chance);
        }
    }
    Err(CaptureError::MissingWobbleProbability(final_catch_rate))
}

fn require_capture_runtime_rules(
    rules: &CaptureRules,
    probabilities: &[CaptureWobbleProbability],
) -> Result<(), CaptureError> {
    require_capture_rules(rules)?;
    if probabilities.is_empty() {
        return Err(CaptureError::MissingWobbleTable);
    }
    Ok(())
}

fn require_capture_rules(rules: &CaptureRules) -> Result<(), CaptureError> {
    if rules.ball_rules.is_empty() && rules.guaranteed_capture_balls.is_empty() {
        return Err(CaptureError::MissingRules);
    }
    Ok(())
}

fn clamp_catch_rate(value: i32, min: u8) -> u8 {
    value.clamp(min as i32, 0xff) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies, pokemon_type};

    fn wobble_probabilities() -> Vec<CaptureWobbleProbability> {
        vec![
            CaptureWobbleProbability {
                catch_rate: 1,
                chance: 63,
            },
            CaptureWobbleProbability {
                catch_rate: 2,
                chance: 75,
            },
            CaptureWobbleProbability {
                catch_rate: 3,
                chance: 84,
            },
            CaptureWobbleProbability {
                catch_rate: 4,
                chance: 90,
            },
            CaptureWobbleProbability {
                catch_rate: 5,
                chance: 95,
            },
            CaptureWobbleProbability {
                catch_rate: 7,
                chance: 103,
            },
            CaptureWobbleProbability {
                catch_rate: 10,
                chance: 113,
            },
            CaptureWobbleProbability {
                catch_rate: 15,
                chance: 126,
            },
            CaptureWobbleProbability {
                catch_rate: 20,
                chance: 134,
            },
            CaptureWobbleProbability {
                catch_rate: 30,
                chance: 149,
            },
            CaptureWobbleProbability {
                catch_rate: 40,
                chance: 160,
            },
            CaptureWobbleProbability {
                catch_rate: 50,
                chance: 169,
            },
            CaptureWobbleProbability {
                catch_rate: 60,
                chance: 177,
            },
            CaptureWobbleProbability {
                catch_rate: 80,
                chance: 191,
            },
            CaptureWobbleProbability {
                catch_rate: 100,
                chance: 201,
            },
            CaptureWobbleProbability {
                catch_rate: 120,
                chance: 211,
            },
            CaptureWobbleProbability {
                catch_rate: 140,
                chance: 220,
            },
            CaptureWobbleProbability {
                catch_rate: 160,
                chance: 227,
            },
            CaptureWobbleProbability {
                catch_rate: 180,
                chance: 234,
            },
            CaptureWobbleProbability {
                catch_rate: 200,
                chance: 240,
            },
            CaptureWobbleProbability {
                catch_rate: 220,
                chance: 246,
            },
            CaptureWobbleProbability {
                catch_rate: 240,
                chance: 251,
            },
            CaptureWobbleProbability {
                catch_rate: 254,
                chance: 253,
            },
            CaptureWobbleProbability {
                catch_rate: 255,
                chance: 255,
            },
        ]
    }

    fn pokemon(id: &str, catch_rate: u8, level: u8, hp: u16, max_hp: u16) -> Pokemon {
        let mut species =
            PokemonSpecies::new_for_tests(id, BaseStats::new(max_hp, 49, 49, 45, 65, 65));
        species.catch_rate = catch_rate;
        species.type1 = pokemon_type("NORMAL");
        species.type2 = pokemon_type("NORMAL");
        let mut pokemon = Pokemon::new_for_tests(species, level, Dv::from_non_hp(10, 10, 10, 10));
        pokemon.hp = hp;
        pokemon.max_hp = max_hp;
        pokemon
    }

    fn ball_rule(numerator: u16, denominator: u16) -> CaptureBallRule {
        CaptureBallRule {
            multiplier_numerator: numerator,
            multiplier_denominator: denominator,
            battle_type: String::new(),
            skip_hp_calc: false,
            use_heavy_ball_weight_modifier: false,
            use_level_ball_multiplier: false,
            require_same_species: false,
            require_same_gender: false,
            require_fast_species: false,
        }
    }

    fn capture_rules() -> CaptureRules {
        let mut rules = CaptureRules {
            fast_ball_species: BTreeSet::new(),
            heavy_ball_modifiers: BTreeMap::new(),
            ball_rules: [
                ("POKE_BALL".to_string(), ball_rule(1, 1)),
                ("FRIEND_BALL".to_string(), ball_rule(1, 1)),
                ("ULTRA_BALL".to_string(), ball_rule(2, 1)),
                ("GREAT_BALL".to_string(), ball_rule(3, 2)),
                ("SAFARI_BALL".to_string(), ball_rule(3, 2)),
                ("PARK_BALL".to_string(), ball_rule(3, 2)),
                ("HEAVY_BALL".to_string(), ball_rule(1, 1)),
                ("LEVEL_BALL".to_string(), ball_rule(1, 1)),
                ("LURE_BALL".to_string(), ball_rule(3, 1)),
                ("MOON_BALL".to_string(), ball_rule(1, 1)),
                ("LOVE_BALL".to_string(), ball_rule(8, 1)),
                ("FAST_BALL".to_string(), ball_rule(4, 1)),
            ]
            .into_iter()
            .collect(),
            guaranteed_capture_balls: ["MASTER_BALL".to_string()].into_iter().collect(),
            status_bonus: [("SLEEP".to_string(), 10), ("FREEZE".to_string(), 10)]
                .into_iter()
                .collect(),
        };
        rules
            .ball_rules
            .insert("MASTER_BALL".to_string(), ball_rule(1, 1));
        let heavy = rules
            .ball_rules
            .get_mut("HEAVY_BALL")
            .expect("test heavy ball rule exists");
        heavy.use_heavy_ball_weight_modifier = true;
        let lure = rules
            .ball_rules
            .get_mut("LURE_BALL")
            .expect("test lure ball rule exists");
        lure.battle_type = "BATTLETYPE_FISH".to_string();
        let level = rules
            .ball_rules
            .get_mut("LEVEL_BALL")
            .expect("test level ball rule exists");
        level.use_level_ball_multiplier = true;
        level.skip_hp_calc = true;
        let love = rules
            .ball_rules
            .get_mut("LOVE_BALL")
            .expect("test love ball rule exists");
        love.require_same_species = true;
        love.require_same_gender = true;
        let fast = rules
            .ball_rules
            .get_mut("FAST_BALL")
            .expect("test fast ball rule exists");
        fast.require_fast_species = true;
        rules
    }

    fn test_ball(script_name: &str) -> Item {
        use crate::models::item_pocket;

        Item {
            name: script_name.to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
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
            price: 200,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("BALL"),
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: script_name.to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    #[test]
    fn hp_and_sleep_bonus_match_gen_two_catch_rate() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let mut enemy = pokemon("PIDGEY", 100, 5, 10, 20);
        enemy.status = Some("SLEEP".to_string());
        let rate = compute_final_catch_rate(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("POKE_BALL"),
            &capture_rules(),
        )
        .expect("poke ball capture rate should resolve");
        assert_eq!(rate, 76);
    }

    #[test]
    fn level_ball_ignores_hp_and_status_after_level_multiplier() {
        let player = pokemon("CHIKORITA", 45, 21, 20, 20);
        let mut enemy = pokemon("PIDGEY", 100, 10, 1, 20);
        enemy.status = Some("SLEEP".to_string());
        let context = CaptureAttemptContext::wild("LEVEL_BALL");
        let low_hp = compute_final_catch_rate(&player, &enemy, &context, &capture_rules())
            .expect("level ball capture rate should resolve");
        enemy.hp = 20;
        let high_hp = compute_final_catch_rate(&player, &enemy, &context, &capture_rules())
            .expect("level ball capture rate should resolve");
        assert_eq!(low_hp, 200);
        assert_eq!(high_hp, 200);
    }

    #[test]
    fn fast_ball_species_are_explicit_capture_rule_data() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let magnemite = pokemon("MAGNEMITE", 45, 10, 1, 20);
        let mut rules = capture_rules();
        rules.fast_ball_species.insert("MAGNEMITE".to_string());
        let boosted = compute_final_catch_rate(
            &player,
            &magnemite,
            &CaptureAttemptContext::wild("FAST_BALL"),
            &rules,
        )
        .expect("fast ball capture rate should resolve");
        let unboosted = compute_final_catch_rate(
            &player,
            &magnemite,
            &CaptureAttemptContext::wild("FAST_BALL"),
            &capture_rules(),
        )
        .expect("fast ball capture rate should resolve");
        assert_eq!(boosted, 174);
        assert_eq!(unboosted, 43);
    }

    #[test]
    fn heavy_ball_species_modifiers_are_explicit_capture_rule_data() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let mut kadabra = pokemon("KADABRA", 100, 10, 20, 20);
        kadabra.species.weight = 1250;
        let mut rules = capture_rules();
        rules.heavy_ball_modifiers.insert("KADABRA".to_string(), 40);
        let rate = compute_final_catch_rate(
            &player,
            &kadabra,
            &CaptureAttemptContext::wild("HEAVY_BALL"),
            &rules,
        )
        .expect("heavy ball modifier should come from capture rules");
        assert_eq!(rate, 46);
    }

    #[test]
    fn heavy_ball_requires_explicit_species_modifier_data() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let kadabra = pokemon("KADABRA", 100, 10, 20, 20);

        let error = compute_final_catch_rate(
            &player,
            &kadabra,
            &CaptureAttemptContext::wild("HEAVY_BALL"),
            &capture_rules(),
        )
        .expect_err("missing Heavy Ball modifier must not be computed as a fallback");

        assert_eq!(
            error,
            CaptureError::MissingHeavyBallModifier("KADABRA".to_string())
        );
    }

    #[test]
    fn unknown_capture_ball_does_not_fallback_to_poke_ball_behavior() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 100, 5, 10, 20);

        let error = compute_final_catch_rate(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("MOD_BALL"),
            &capture_rules(),
        )
        .expect_err("unknown ball ids must be implemented explicitly");

        assert_eq!(error, CaptureError::UnknownBall("MOD_BALL".to_string()));
    }

    #[test]
    fn malformed_capture_ball_id_rejects_before_guaranteed_or_rule_lookup() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 100, 5, 10, 20);
        let mut rules = capture_rules();
        rules
            .guaranteed_capture_balls
            .insert("MASTER BALL".to_string());

        let error = resolve_capture_attempt(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("MASTER BALL"),
            &rules,
            &wobble_probabilities(),
            &mut Random::new(1),
        )
        .expect_err("malformed guaranteed ball ids are invalid content");

        assert_eq!(error, CaptureError::InvalidBall("MASTER BALL".to_string()));
    }

    #[test]
    fn plain_capture_balls_are_explicit_known_ids() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 100, 5, 10, 20);
        let rules = capture_rules();

        let poke_ball = compute_final_catch_rate(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("POKE_BALL"),
            &rules,
        )
        .expect("poke ball is explicit");
        let friend_ball = compute_final_catch_rate(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("FRIEND_BALL"),
            &rules,
        )
        .expect("friend ball is explicit");

        assert_eq!(friend_ball, poke_ball);
    }

    #[test]
    fn capture_ball_rule_shape_is_exact_without_coercion() {
        let mut rule = ball_rule(1, 1);
        assert_eq!(capture_ball_rule_issues("POKE_BALL", &rule), Vec::new());
        validate_capture_ball_rule_shape("POKE_BALL", &rule).expect("valid shape");

        assert_eq!(
            capture_ball_rule_issues(" POKE_BALL", &rule),
            vec![CaptureBallRuleIssue::InvalidBallId]
        );
        assert_eq!(
            capture_ball_rule_issues("POKE BALL", &rule),
            vec![CaptureBallRuleIssue::InvalidBallId]
        );
        assert_eq!(
            validate_capture_ball_rule_shape(" POKE_BALL", &rule),
            Err(CaptureError::InvalidBallRule {
                ball_id: " POKE_BALL".to_string(),
                message: "ball id must be an exact nonempty id".to_string(),
            })
        );

        rule.battle_type = "BATTLETYPE FISH".to_string();
        assert_eq!(
            capture_ball_rule_issues("LURE_BALL", &rule),
            vec![CaptureBallRuleIssue::InvalidBattleType]
        );

        rule.battle_type.clear();
        rule.multiplier_denominator = 0;
        assert_eq!(
            capture_ball_rule_issues("LEVEL_BALL", &rule),
            vec![CaptureBallRuleIssue::InvalidMultiplierDenominator]
        );
    }

    #[test]
    fn capture_rule_tokens_reject_reserved_pack_prefixes() {
        let mut rule = ball_rule(1, 1);
        rule.battle_type = "legacy_battletype_fish".to_string();
        assert_eq!(
            capture_ball_rule_issues("fallback_lure_ball", &rule),
            vec![
                CaptureBallRuleIssue::InvalidBallId,
                CaptureBallRuleIssue::InvalidBattleType,
            ]
        );

        let rules = CaptureRules {
            fast_ball_species: BTreeSet::from(["fallback_magnemite".to_string()]),
            heavy_ball_modifiers: BTreeMap::from([("legacy_snorlax".to_string(), 40)]),
            ball_rules: BTreeMap::from([("fallback_poke_ball".to_string(), ball_rule(1, 1))]),
            guaranteed_capture_balls: BTreeSet::from(["legacy_master_ball".to_string()]),
            status_bonus: BTreeMap::new(),
        };

        assert_eq!(
            capture_rules_issues(&rules, &BTreeSet::new(), &BTreeMap::new(), true),
            vec![
                CaptureRulesIssue::InvalidFastBallSpecies {
                    species: "fallback_magnemite".to_string(),
                },
                CaptureRulesIssue::InvalidHeavyBallSpecies {
                    species: "legacy_snorlax".to_string(),
                },
                CaptureRulesIssue::InvalidBallRuleItem {
                    ball_id: "fallback_poke_ball".to_string(),
                },
                CaptureRulesIssue::InvalidBallRule {
                    ball_id: "fallback_poke_ball".to_string(),
                    issue: CaptureBallRuleIssue::InvalidBallId,
                },
                CaptureRulesIssue::InvalidGuaranteedCaptureBall {
                    ball_id: "legacy_master_ball".to_string(),
                },
            ]
        );
    }

    #[test]
    fn capture_rules_issues_validate_definitive_pack_references() {
        let mut rules = CaptureRules {
            fast_ball_species: BTreeSet::from([
                "MAGNEMITE".to_string(),
                "MAGNE MITE".to_string(),
                "magnemite".to_string(),
            ]),
            heavy_ball_modifiers: BTreeMap::from([
                ("SNORLAX".to_string(), 40),
                ("SNOR LAX".to_string(), 40),
                ("snorlax".to_string(), 40),
            ]),
            ball_rules: BTreeMap::from([(" POKE_BALL".to_string(), ball_rule(1, 0))]),
            guaranteed_capture_balls: BTreeSet::from([
                " MASTER_BALL".to_string(),
                "MOD_BALL".to_string(),
            ]),
            status_bonus: BTreeMap::new(),
        };
        let species = BTreeSet::from(["MAGNEMITE".to_string(), "SNORLAX".to_string()]);
        let ball_items = BTreeMap::from([
            ("POKE_BALL".to_string(), test_ball("POKE_BALL")),
            ("MASTER_BALL".to_string(), test_ball("MASTER_BALL")),
        ]);

        assert_eq!(
            capture_rules_issues(&rules, &species, &ball_items, true),
            vec![
                CaptureRulesIssue::InvalidFastBallSpecies {
                    species: "MAGNE MITE".to_string()
                },
                CaptureRulesIssue::UnknownFastBallSpecies {
                    species: "magnemite".to_string()
                },
                CaptureRulesIssue::InvalidHeavyBallSpecies {
                    species: "SNOR LAX".to_string()
                },
                CaptureRulesIssue::UnknownHeavyBallSpecies {
                    species: "snorlax".to_string()
                },
                CaptureRulesIssue::InvalidBallRuleItem {
                    ball_id: " POKE_BALL".to_string(),
                },
                CaptureRulesIssue::InvalidBallRule {
                    ball_id: " POKE_BALL".to_string(),
                    issue: CaptureBallRuleIssue::InvalidBallId,
                },
                CaptureRulesIssue::InvalidBallRule {
                    ball_id: " POKE_BALL".to_string(),
                    issue: CaptureBallRuleIssue::InvalidMultiplierDenominator,
                },
                CaptureRulesIssue::InvalidGuaranteedCaptureBall {
                    ball_id: " MASTER_BALL".to_string(),
                },
                CaptureRulesIssue::UnknownGuaranteedCaptureBall {
                    ball_id: "MOD_BALL".to_string(),
                },
            ]
        );

        rules.ball_rules.clear();
        assert_eq!(
            capture_rules_issues(&rules, &species, &ball_items, true)
                .into_iter()
                .next(),
            Some(CaptureRulesIssue::MissingBallRules)
        );
        assert!(
            !capture_rules_issues(&rules, &species, &ball_items, false)
                .contains(&CaptureRulesIssue::MissingBallRules)
        );
    }

    #[test]
    fn capture_rules_issues_reject_capture_balls_that_are_not_battle_usable() {
        let rules = CaptureRules {
            fast_ball_species: BTreeSet::new(),
            heavy_ball_modifiers: BTreeMap::new(),
            ball_rules: BTreeMap::from([("POKE_BALL".to_string(), ball_rule(1, 1))]),
            guaranteed_capture_balls: BTreeSet::from(["MASTER_BALL".to_string()]),
            status_bonus: BTreeMap::new(),
        };
        let mut poke_ball = test_ball("POKE_BALL");
        poke_ball.battle_usable = false;
        let mut master_ball = test_ball("MASTER_BALL");
        master_ball.battle_usable = false;
        let ball_items = BTreeMap::from([
            ("POKE_BALL".to_string(), poke_ball),
            ("MASTER_BALL".to_string(), master_ball),
        ]);

        assert_eq!(
            capture_rules_issues(&rules, &BTreeSet::new(), &ball_items, true),
            vec![
                CaptureRulesIssue::UnusableBallRuleItem {
                    ball_id: "POKE_BALL".to_string(),
                },
                CaptureRulesIssue::UnusableGuaranteedCaptureBall {
                    ball_id: "MASTER_BALL".to_string(),
                },
            ]
        );
    }

    #[test]
    fn capture_wobble_probability_issues_validate_complete_exact_table() {
        assert_eq!(
            capture_wobble_probability_issues(&[], true),
            vec![CaptureWobbleProbabilityIssue::MissingTable]
        );
        assert_eq!(capture_wobble_probability_issues(&[], false), []);

        let probabilities = vec![
            CaptureWobbleProbability {
                catch_rate: 0,
                chance: 0,
            },
            CaptureWobbleProbability {
                catch_rate: 10,
                chance: 20,
            },
            CaptureWobbleProbability {
                catch_rate: 9,
                chance: 30,
            },
        ];
        assert_eq!(
            capture_wobble_probability_issues(&probabilities, true),
            vec![
                CaptureWobbleProbabilityIssue::InvalidCatchRate,
                CaptureWobbleProbabilityIssue::UnorderedCatchRate {
                    catch_rate: 9,
                    previous: 10,
                },
                CaptureWobbleProbabilityIssue::IncompleteTable,
            ]
        );

        assert_eq!(
            capture_wobble_probability_issues(
                &[
                    CaptureWobbleProbability {
                        catch_rate: 1,
                        chance: 63,
                    },
                    CaptureWobbleProbability {
                        catch_rate: 255,
                        chance: 255,
                    },
                ],
                true,
            ),
            []
        );
    }

    #[test]
    fn capture_rules_json_requires_explicit_pack_fields() {
        let missing_heavy_modifiers =
            serde_json::from_str::<CaptureRules>(
                r#"{"fast_ball_species":["MAGNEMITE"],"ball_rules":{},"guaranteed_capture_balls":[],"status_bonus":{}}"#,
            )
                .expect_err("heavy ball modifiers must be explicit, even when empty")
                .to_string();
        assert!(
            missing_heavy_modifiers.contains("missing field `heavy_ball_modifiers`"),
            "{missing_heavy_modifiers}"
        );

        let missing_fast_species =
            serde_json::from_str::<CaptureRules>(
                r#"{"heavy_ball_modifiers":{"KADABRA":40},"ball_rules":{},"guaranteed_capture_balls":[],"status_bonus":{}}"#,
            )
                .expect_err("fast ball species must be explicit, even when empty")
                .to_string();
        assert!(
            missing_fast_species.contains("missing field `fast_ball_species`"),
            "{missing_fast_species}"
        );

        let missing_ball_rules = serde_json::from_str::<CaptureRules>(
            r#"{"fast_ball_species":[],"heavy_ball_modifiers":{},"guaranteed_capture_balls":[],"status_bonus":{}}"#,
        )
        .expect_err("ball rules must be explicit")
        .to_string();
        assert!(
            missing_ball_rules.contains("missing field `ball_rules`"),
            "{missing_ball_rules}"
        );

        let explicit_empty = serde_json::from_str::<CaptureRules>(
            r#"{"fast_ball_species":[],"heavy_ball_modifiers":{},"ball_rules":{},"guaranteed_capture_balls":[],"status_bonus":{}}"#,
        )
        .expect("empty capture rule sets are valid when explicitly declared");
        assert!(explicit_empty.fast_ball_species.is_empty());
        assert!(explicit_empty.heavy_ball_modifiers.is_empty());
        assert!(explicit_empty.ball_rules.is_empty());
    }

    #[test]
    fn capture_attempt_is_deterministic_and_blocks_trainer_battles() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut rng = Random::new(1);
        let outcome = resolve_capture_attempt(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("POKE_BALL"),
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect("capture attempt should resolve");
        assert!(outcome.caught);
        assert_eq!(outcome.animation_shakes, 4);
        assert_eq!(outcome.rng_seed_after, rng.seed());

        let mut trainer_context = CaptureAttemptContext::wild("MASTER_BALL");
        trainer_context.trainer_battle = true;
        let blocked = resolve_capture_attempt(
            &player,
            &enemy,
            &trainer_context,
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect("trainer battle block should resolve");
        assert!(blocked.blocked);
        assert!(!blocked.caught);
    }

    #[test]
    fn capture_attempt_requires_definitive_runtime_rules_before_any_outcome() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut rng = Random::new(1);

        let missing_rules = resolve_capture_attempt(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("MASTER_BALL"),
            &CaptureRules::default(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect_err("missing capture rules must not become guaranteed capture behavior");
        assert_eq!(missing_rules, CaptureError::MissingRules);

        let mut trainer_context = CaptureAttemptContext::wild("POKE_BALL");
        trainer_context.trainer_battle = true;
        let missing_wobble = resolve_capture_attempt(
            &player,
            &enemy,
            &trainer_context,
            &capture_rules(),
            &[],
            &mut rng,
        )
        .expect_err("missing wobble table must not become trainer block behavior");
        assert_eq!(missing_wobble, CaptureError::MissingWobbleTable);
    }

    #[test]
    fn capture_helpers_reject_missing_rules_without_unknown_ball_fallback() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 100, 5, 10, 20);
        let ball = test_ball("POKE_BALL");

        assert_eq!(
            validate_capture_ball_item(&CaptureRules::default(), &ball),
            Err(CaptureError::MissingRules)
        );
        assert_eq!(
            compute_final_catch_rate(
                &player,
                &enemy,
                &CaptureAttemptContext::wild("POKE_BALL"),
                &CaptureRules::default(),
            ),
            Err(CaptureError::MissingRules)
        );
        assert_eq!(
            apply_ball_multiplier(
                "POKE_BALL",
                &player,
                &enemy,
                &CaptureAttemptContext::wild("POKE_BALL"),
                &CaptureRules::default(),
            ),
            Err(CaptureError::MissingRules)
        );
        assert_eq!(
            wobble_chance_for_rate(200, &[]),
            Err(CaptureError::MissingWobbleTable)
        );
    }

    #[test]
    fn successful_capture_registers_pokemon_in_storage() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut rng = Random::new(1);
        let outcome = resolve_capture_attempt(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("MASTER_BALL"),
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect("master ball capture should resolve");
        let mut storage = PokemonStorage::default();
        let stored = store_captured_pokemon(&outcome, &mut storage, enemy.clone())
            .expect("store captured pokemon")
            .expect("captured");

        assert_eq!(stored.location, CaptureStorageLocation::Party { slot: 0 });
        assert_eq!(
            storage.party.pokemon[0]
                .as_ref()
                .map(|pokemon| &pokemon.species.id),
            Some(&enemy.species.id)
        );
    }

    #[test]
    fn completing_successful_capture_registers_exact_species_in_pokedex() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("modpack-PIDGEY", 255, 2, 1, 20);
        let mut rng = Random::new(1);
        let outcome = resolve_capture_attempt(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("MASTER_BALL"),
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect("master ball capture should resolve");
        let mut storage = PokemonStorage::default();
        let mut pokedex = PokedexState::default();

        let stored = complete_captured_pokemon(&outcome, &mut storage, &mut pokedex, enemy.clone())
            .expect("complete captured pokemon")
            .expect("captured");

        assert_eq!(stored.location, CaptureStorageLocation::Party { slot: 0 });
        assert!(pokedex.has_seen("modpack-PIDGEY"));
        assert!(pokedex.has_caught("modpack-PIDGEY"));
        assert!(!pokedex.has_seen("PIDGEY"));
        assert!(!pokedex.has_caught("MODPACK-PIDGEY"));
    }

    #[test]
    fn complete_active_wild_capture_commits_storage_pokedex_battle_result_and_sync() {
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut state = GameState::default();
        state.battle = BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy],
        };
        state.battle_active_party_index = Some(0);
        state.battle_active_enemy_party_index = Some(0);
        let outcome = CaptureOutcome {
            caught: true,
            blocked: false,
            wobble_count: 4,
            animation_shakes: 4,
            final_catch_rate: u8::MAX,
            rng_seed_after: 1,
        };

        let stored = complete_active_wild_capture(&mut state, &outcome)
            .expect("complete active capture")
            .expect("stored capture");

        assert_eq!(stored.location, CaptureStorageLocation::Party { slot: 0 });
        assert!(state.pokedex.has_seen("PIDGEY"));
        assert!(state.pokedex.has_caught("PIDGEY"));
        assert_eq!(state.battle_result & (1 << 6), 1 << 6);
        assert_eq!(state.battle, BattleMemory::Inactive);
        assert_eq!(state.battle_active_party_index, None);
        assert_eq!(
            state.party.pokemon[0]
                .as_ref()
                .map(|pokemon| pokemon.species.as_str()),
            Some("PIDGEY")
        );
    }

    #[test]
    fn failed_capture_does_not_register_pokedex_caught() {
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let outcome = CaptureOutcome {
            caught: false,
            blocked: false,
            wobble_count: 0,
            animation_shakes: 0,
            final_catch_rate: 1,
            rng_seed_after: 1,
        };
        let mut storage = PokemonStorage::default();
        let mut pokedex = PokedexState::default();

        let stored = complete_captured_pokemon(&outcome, &mut storage, &mut pokedex, enemy)
            .expect("failed capture should be a no-op");

        assert_eq!(stored, None);
        assert_eq!(pokedex.seen_count(), 0);
        assert_eq!(pokedex.caught_count(), 0);
    }

    #[test]
    fn throwing_ball_consumes_exact_bag_item_before_capture_roll() {
        use crate::models::item_pocket;

        let ball = Item {
            name: "POKE BALL".to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
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
            price: 200,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("BALL"),
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: "POKE_BALL".to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        };
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut bag = Bag::default();
        bag.add_item(&ball, 1).expect("add ball");
        let mut rng = Random::new(1);

        let outcome = throw_ball_from_bag(
            &mut bag,
            &ball,
            &player,
            &enemy,
            CaptureAttemptContext::wild("IGNORED"),
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect("throw ball")
        .expect("ball was available");

        assert_eq!(bag.quantity(&ball), 0);
        assert!(outcome.caught);
        assert_eq!(
            throw_ball_from_bag(
                &mut bag,
                &ball,
                &player,
                &enemy,
                CaptureAttemptContext::wild("IGNORED"),
                &capture_rules(),
                &wobble_probabilities(),
                &mut rng,
            )
            .expect("empty bag"),
            None
        );
    }

    #[test]
    fn throwing_ball_in_trainer_battle_is_blocked_without_consuming_bag_item() {
        let ball = test_ball("POKE_BALL");
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut bag = Bag::default();
        bag.add_item(&ball, 1).expect("add ball");
        let mut rng = Random::new(1);
        let mut context = CaptureAttemptContext::wild("IGNORED");
        context.trainer_battle = true;

        let outcome = throw_ball_from_bag(
            &mut bag,
            &ball,
            &player,
            &enemy,
            context,
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect("trainer ball throw resolves")
        .expect("ball was available");

        assert!(outcome.blocked);
        assert!(!outcome.caught);
        assert_eq!(outcome.rng_seed_after, rng.seed());
        assert_eq!(bag.quantity(&ball), 1);

        let mut empty_bag = Bag::default();
        let mut context = CaptureAttemptContext::wild("IGNORED");
        context.trainer_battle = true;
        assert_eq!(
            throw_ball_from_bag(
                &mut empty_bag,
                &ball,
                &player,
                &enemy,
                context,
                &capture_rules(),
                &wobble_probabilities(),
                &mut rng,
            )
            .expect("empty trainer ball throw resolves"),
            None
        );
    }

    #[test]
    fn undeclared_capture_ball_rule_rejects_before_consumption() {
        use crate::models::item_pocket;

        let ball = Item {
            name: "MOD BALL".to_string(),
            description: String::new(),
            effect: "MOD_CAPTURE".to_string(),
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
            price: 200,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("BALL"),
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: "MOD_BALL".to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        };
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut bag = Bag::default();
        bag.add_item(&ball, 1).expect("add ball");
        let mut rng = Random::new(1);

        let error = throw_ball_from_bag(
            &mut bag,
            &ball,
            &player,
            &enemy,
            CaptureAttemptContext::wild("IGNORED"),
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect_err("unknown ball rejects before consumption");

        assert_eq!(
            error,
            CaptureUseError::Capture(CaptureError::UnknownBall("MOD_BALL".to_string()))
        );
        assert_eq!(bag.quantity(&ball), 1);
    }

    #[test]
    fn malformed_capture_ball_rejects_before_consumption() {
        use crate::models::item_pocket;

        let ball = Item {
            name: "MOD BALL".to_string(),
            description: String::new(),
            effect: "MOD_CAPTURE".to_string(),
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
            price: 200,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("BALL"),
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: "MOD BALL".to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        };
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut bag = Bag::default();
        bag.balls.insert("MOD BALL".to_string(), 1);
        let mut rng = Random::new(1);

        let error = throw_ball_from_bag(
            &mut bag,
            &ball,
            &player,
            &enemy,
            CaptureAttemptContext::wild("IGNORED"),
            &capture_rules(),
            &wobble_probabilities(),
            &mut rng,
        )
        .expect_err("invalid ball rejects before consumption");

        assert_eq!(
            error,
            CaptureUseError::Capture(CaptureError::InvalidBall("MOD BALL".to_string()))
        );
        assert_eq!(bag.balls["MOD BALL"], 1);
    }

    #[test]
    fn reserved_capture_ball_id_rejects_before_rule_lookup() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 255, 2, 1, 20);
        let mut rules = capture_rules();
        rules
            .guaranteed_capture_balls
            .insert("fallback_master_ball".to_string());
        let mut rng = Random::new(1);

        let error = resolve_capture_attempt(
            &player,
            &enemy,
            &CaptureAttemptContext::wild("fallback_master_ball"),
            &rules,
            &wobble_probabilities(),
            &mut rng,
        )
        .expect_err("reserved capture ball ids are invalid content");

        assert_eq!(
            error,
            CaptureError::InvalidBall("fallback_master_ball".to_string())
        );
    }

    #[test]
    fn capture_wobble_requires_explicit_probability_row() {
        assert_eq!(
            wobble_chance_for_rate(
                200,
                &[CaptureWobbleProbability {
                    catch_rate: 100,
                    chance: 201
                }]
            ),
            Err(CaptureError::MissingWobbleProbability(200))
        );
    }

    #[test]
    fn capture_context_json_rejects_unknown_ball_fallback_fields() {
        let error = serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
            "ball_id": "POKE_BALL",
            "battle_type": "BATTLETYPE_NORMAL",
            "trainer_battle": false,
            "player_gender": null,
            "enemy_gender": null,
            "fallback_ball_id": "POKE BALL"
        }))
        .expect_err("capture context must not accept fallback ball ids")
        .to_string();

        assert!(
            error.contains("unknown field `fallback_ball_id`"),
            "{error}"
        );
    }

    #[test]
    fn capture_issue_json_rejects_unknown_fallback_fields() {
        let rules_error = serde_json::from_value::<CaptureRulesIssue>(serde_json::json!({
            "unknown_ball_rule_item": {
                "ball_id": "MOD_BALL",
                "fallback_ball_id": "POKE_BALL"
            }
        }))
        .expect_err("capture rule issues must not accept fallback ball ids")
        .to_string();
        assert!(
            rules_error.contains("unknown field `fallback_ball_id`"),
            "{rules_error}"
        );

        let ball_rule_error = serde_json::from_value::<CaptureRulesIssue>(serde_json::json!({
            "invalid_ball_rule": {
                "ball_id": "MOD_BALL",
                "issue": {
                    "invalid_battle_type": {
                        "default_battle_type": "BATTLETYPE_NORMAL"
                    }
                }
            }
        }))
        .expect_err("capture ball rule issues must not accept default battle types")
        .to_string();
        assert!(
            ball_rule_error.contains("unknown field `default_battle_type`"),
            "{ball_rule_error}"
        );

        let wobble_error =
            serde_json::from_value::<CaptureWobbleProbabilityIssue>(serde_json::json!({
                "unordered_catch_rate": {
                    "catch_rate": 100,
                    "previous": 200,
                    "fallback_chance": 0
                }
            }))
            .expect_err("wobble issues must not accept fallback chances")
            .to_string();
        assert!(
            wobble_error.contains("unknown field `fallback_chance`"),
            "{wobble_error}"
        );
    }

    #[test]
    fn capture_context_json_requires_explicit_nullable_gender_inputs() {
        let missing_player_gender =
            serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
                "ball_id": "LOVE_BALL",
                "battle_type": "BATTLETYPE_NORMAL",
                "trainer_battle": false,
                "enemy_gender": "female"
            }))
            .expect_err("player gender must be explicit, even when null")
            .to_string();
        assert!(
            missing_player_gender.contains("missing field `player_gender`"),
            "{missing_player_gender}"
        );

        let missing_enemy_gender =
            serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
                "ball_id": "LOVE_BALL",
                "battle_type": "BATTLETYPE_NORMAL",
                "trainer_battle": false,
                "player_gender": "male"
            }))
            .expect_err("enemy gender must be explicit, even when null")
            .to_string();
        assert!(
            missing_enemy_gender.contains("missing field `enemy_gender`"),
            "{missing_enemy_gender}"
        );

        let explicit_nulls = serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
            "ball_id": "POKE_BALL",
            "battle_type": "BATTLETYPE_NORMAL",
            "trainer_battle": false,
            "player_gender": null,
            "enemy_gender": null
        }))
        .expect("nullable gender inputs are valid when explicitly declared");

        assert_eq!(explicit_nulls.player_gender, None);
        assert_eq!(explicit_nulls.enemy_gender, None);
    }

    #[test]
    fn capture_context_json_validates_exact_runtime_tokens() {
        let invalid_ball = serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
            "ball_id": "POKE BALL",
            "battle_type": "BATTLETYPE_NORMAL",
            "trainer_battle": false,
            "player_gender": null,
            "enemy_gender": null
        }))
        .expect_err("capture context ball id must be exact during decode")
        .to_string();
        assert!(
            invalid_ball.contains("invalid capture ball 'POKE BALL'"),
            "{invalid_ball}"
        );

        let invalid_battle_type =
            serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
                "ball_id": "POKE_BALL",
                "battle_type": "BATTLETYPE NORMAL",
                "trainer_battle": false,
                "player_gender": null,
                "enemy_gender": null
            }))
            .expect_err("capture context battle type must be exact during decode")
            .to_string();
        assert!(
            invalid_battle_type.contains("invalid capture context battle type"),
            "{invalid_battle_type}"
        );

        let invalid_gender = serde_json::from_value::<CaptureAttemptContext>(serde_json::json!({
            "ball_id": "LOVE_BALL",
            "battle_type": "BATTLETYPE_NORMAL",
            "trainer_battle": false,
            "player_gender": "male trainer",
            "enemy_gender": "female"
        }))
        .expect_err("capture context gender values must be exact during decode")
        .to_string();
        assert!(
            invalid_gender.contains("invalid capture context player gender"),
            "{invalid_gender}"
        );
    }

    #[test]
    fn capture_resolution_validates_context_before_rules_or_rng() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let enemy = pokemon("PIDGEY", 100, 5, 10, 20);
        let context = CaptureAttemptContext {
            ball_id: "POKE_BALL".to_string(),
            battle_type: "BATTLETYPE NORMAL".to_string(),
            trainer_battle: false,
            player_gender: None,
            enemy_gender: None,
        };
        let mut rng = Random::new(1);
        let seed_before = rng.seed();

        let error = resolve_capture_attempt(
            &player,
            &enemy,
            &context,
            &CaptureRules::default(),
            &[],
            &mut rng,
        )
        .expect_err("malformed context must fail before missing rules or rng mutation");

        assert_eq!(
            error,
            CaptureError::InvalidCaptureContext {
                field: "battle type".to_string(),
                value: "BATTLETYPE NORMAL".to_string(),
            }
        );
        assert_eq!(rng.seed(), seed_before);
    }
}
