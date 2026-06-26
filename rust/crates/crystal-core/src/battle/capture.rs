use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::{Bag, CaptureStorageLocation, Item, PokedexState, Pokemon, PokemonStorage};
use crate::random::Random;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRules {
    pub fast_ball_species: BTreeSet<String>,
    pub heavy_ball_modifiers: BTreeMap<String, i16>,
    pub ball_rules: BTreeMap<String, CaptureBallRule>,
    pub guaranteed_capture_balls: BTreeSet<String>,
    pub status_bonus: BTreeMap<String, u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureBallRuleIssue {
    InvalidBallId,
    InvalidBattleType,
    InvalidMultiplierDenominator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureRulesIssue {
    MissingBallRules,
    UnknownFastBallSpecies {
        species: String,
    },
    UnknownHeavyBallSpecies {
        species: String,
    },
    UnknownBallRuleItem {
        ball_id: String,
    },
    InvalidGuaranteedCaptureBall {
        ball_id: String,
    },
    UnknownGuaranteedCaptureBall {
        ball_id: String,
    },
    InvalidBallRule {
        ball_id: String,
        issue: CaptureBallRuleIssue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureWobbleProbabilityIssue {
    MissingTable,
    InvalidCatchRate,
    UnorderedCatchRate { catch_rate: u8, previous: u8 },
    IncompleteTable,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureWobbleProbability {
    pub catch_rate: u8,
    pub chance: u8,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    #[error("missing Heavy Ball modifier for species '{0}'")]
    MissingHeavyBallModifier(String),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

fn required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
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
    if !bag.consume_ball(ball).map_err(CaptureUseError::Bag)? {
        return Ok(None);
    }
    context.ball_id = ball.script_name.clone();
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
    if rules.ball_rules.contains_key(&ball.script_name)
        || rules.guaranteed_capture_balls.contains(&ball.script_name)
    {
        return Ok(());
    }
    Err(CaptureError::UnknownBall(ball.script_name.clone()))
}

pub fn capture_ball_rule_issues(
    ball_id: &str,
    rule: &CaptureBallRule,
) -> Vec<CaptureBallRuleIssue> {
    let mut issues = Vec::new();
    if ball_id.trim().is_empty() || ball_id.trim() != ball_id {
        issues.push(CaptureBallRuleIssue::InvalidBallId);
    }
    if rule.battle_type.trim() != rule.battle_type {
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
    ball_item_ids: &BTreeSet<String>,
    has_ball_pocket_items: bool,
) -> Vec<CaptureRulesIssue> {
    let mut issues = Vec::new();
    if has_ball_pocket_items && rules.ball_rules.is_empty() {
        issues.push(CaptureRulesIssue::MissingBallRules);
    }
    for species in &rules.fast_ball_species {
        if !species_ids.contains(species) {
            issues.push(CaptureRulesIssue::UnknownFastBallSpecies {
                species: species.clone(),
            });
        }
    }
    for species in rules.heavy_ball_modifiers.keys() {
        if !species_ids.contains(species) {
            issues.push(CaptureRulesIssue::UnknownHeavyBallSpecies {
                species: species.clone(),
            });
        }
    }
    for (ball_id, rule) in &rules.ball_rules {
        if !ball_item_ids.is_empty() && !ball_item_ids.contains(ball_id) {
            issues.push(CaptureRulesIssue::UnknownBallRuleItem {
                ball_id: ball_id.clone(),
            });
        }
        for issue in capture_ball_rule_issues(ball_id, rule) {
            issues.push(CaptureRulesIssue::InvalidBallRule {
                ball_id: ball_id.clone(),
                issue,
            });
        }
    }
    for ball_id in &rules.guaranteed_capture_balls {
        if ball_id.trim().is_empty() || ball_id.trim() != ball_id {
            issues.push(CaptureRulesIssue::InvalidGuaranteedCaptureBall {
                ball_id: ball_id.clone(),
            });
        } else if !ball_item_ids.is_empty() && !ball_item_ids.contains(ball_id) {
            issues.push(CaptureRulesIssue::UnknownGuaranteedCaptureBall {
                ball_id: ball_id.clone(),
            });
        }
    }
    issues
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

pub fn compute_final_catch_rate(
    player: &Pokemon,
    enemy: &Pokemon,
    context: &CaptureAttemptContext,
    rules: &CaptureRules,
) -> Result<u8, CaptureError> {
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
    for entry in probabilities {
        if final_catch_rate <= entry.catch_rate {
            return Ok(entry.chance);
        }
    }
    Err(CaptureError::MissingWobbleProbability(final_catch_rate))
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
            validate_capture_ball_rule_shape(" POKE_BALL", &rule),
            Err(CaptureError::InvalidBallRule {
                ball_id: " POKE_BALL".to_string(),
                message: "ball id must be an exact nonempty id".to_string(),
            })
        );

        rule.battle_type = " BATTLETYPE_FISH".to_string();
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
    fn capture_rules_issues_validate_definitive_pack_references() {
        let mut rules = CaptureRules {
            fast_ball_species: BTreeSet::from(["MAGNEMITE".to_string(), "magnemite".to_string()]),
            heavy_ball_modifiers: BTreeMap::from([
                ("SNORLAX".to_string(), 40),
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
        let ball_item_ids = BTreeSet::from(["POKE_BALL".to_string(), "MASTER_BALL".to_string()]);

        assert_eq!(
            capture_rules_issues(&rules, &species, &ball_item_ids, true),
            vec![
                CaptureRulesIssue::UnknownFastBallSpecies {
                    species: "magnemite".to_string()
                },
                CaptureRulesIssue::UnknownHeavyBallSpecies {
                    species: "snorlax".to_string()
                },
                CaptureRulesIssue::UnknownBallRuleItem {
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
            capture_rules_issues(&rules, &species, &ball_item_ids, true)
                .into_iter()
                .next(),
            Some(CaptureRulesIssue::MissingBallRules)
        );
        assert!(
            !capture_rules_issues(&rules, &species, &ball_item_ids, false)
                .contains(&CaptureRulesIssue::MissingBallRules)
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
}
