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
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    #[error("missing Heavy Ball modifier for species '{0}'")]
    MissingHeavyBallModifier(String),
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
    #[serde(default)]
    pub player_gender: Option<String>,
    #[serde(default)]
    pub enemy_gender: Option<String>,
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

    let final_catch_rate = compute_final_catch_rate(player, enemy, context, rules)?;
    if context.ball_id == "MASTER_BALL" {
        return Ok(CaptureOutcome {
            caught: true,
            blocked: false,
            wobble_count: 3,
            animation_shakes: 4,
            final_catch_rate,
            rng_seed_after: rng.seed(),
        });
    }

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

    let wobble_chance = wobble_chance_for_rate(final_catch_rate);
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
    rng: &mut Random,
) -> Result<Option<CaptureOutcome>, CaptureUseError> {
    if !bag.consume_ball(ball).map_err(CaptureUseError::Bag)? {
        return Ok(None);
    }
    context.ball_id = ball.script_name.clone();
    Ok(Some(resolve_capture_attempt(
        player, enemy, &context, rules, rng,
    )?))
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
    if matches!(enemy.status.as_deref(), Some("SLEEP" | "FREEZE")) {
        final_rate = clamp_catch_rate(final_rate as i32 + 10, 1);
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

    match ball_id {
        "ULTRA_BALL" => rate = clamp_catch_rate((rate as i32) << 1, 0),
        "GREAT_BALL" | "SAFARI_BALL" | "PARK_BALL" => {
            rate = clamp_catch_rate(rate as i32 + ((rate as i32) >> 1), 0);
        }
        "HEAVY_BALL" => {
            let modifier = rules
                .heavy_ball_modifiers
                .get(&enemy.species.id)
                .copied()
                .ok_or_else(|| CaptureError::MissingHeavyBallModifier(enemy.species.id.clone()))?;
            rate = clamp_catch_rate(rate as i32 + modifier as i32, 1);
        }
        "LEVEL_BALL" => {
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
            skip_hp_calc = true;
        }
        "LURE_BALL" => {
            if context.battle_type == "BATTLETYPE_FISH" {
                rate = clamp_catch_rate(rate as i32 * 3, 0);
            }
        }
        "MOON_BALL" => {}
        "LOVE_BALL" => {
            if player.species.id == enemy.species.id
                && context.player_gender.is_some()
                && context.player_gender == context.enemy_gender
            {
                rate = clamp_catch_rate(rate as i32 * 8, 0);
            }
        }
        "FAST_BALL" => {
            if rules.fast_ball_species.contains(&enemy.species.id) {
                rate = clamp_catch_rate(rate as i32 * 4, 0);
            }
        }
        _ => {}
    }

    Ok(BallCatchRateResult { rate, skip_hp_calc })
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

pub fn wobble_chance_for_rate(final_catch_rate: u8) -> u8 {
    for (threshold, chance) in WOBBLE_PROBABILITIES.iter().copied() {
        if final_catch_rate <= threshold {
            return chance;
        }
    }
    255
}

fn clamp_catch_rate(value: i32, min: u8) -> u8 {
    value.clamp(min as i32, 0xff) as u8
}

const WOBBLE_PROBABILITIES: &[(u8, u8)] = &[
    (1, 63),
    (2, 75),
    (3, 84),
    (4, 90),
    (5, 95),
    (7, 103),
    (10, 113),
    (15, 126),
    (20, 134),
    (30, 149),
    (40, 160),
    (50, 169),
    (60, 177),
    (80, 191),
    (100, 201),
    (120, 211),
    (140, 220),
    (160, 227),
    (180, 234),
    (200, 240),
    (220, 246),
    (240, 251),
    (254, 253),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies, PokemonType};

    fn pokemon(id: &str, catch_rate: u8, level: u8, hp: u16, max_hp: u16) -> Pokemon {
        let mut species =
            PokemonSpecies::new_for_tests(id, BaseStats::new(max_hp, 49, 49, 45, 65, 65));
        species.catch_rate = catch_rate;
        species.type1 = PokemonType::Normal;
        species.type2 = PokemonType::Normal;
        let mut pokemon = Pokemon::new_for_tests(species, level, Dv::from_non_hp(10, 10, 10, 10));
        pokemon.hp = hp;
        pokemon.max_hp = max_hp;
        pokemon
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
            &CaptureRules::default(),
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
        let low_hp = compute_final_catch_rate(&player, &enemy, &context, &CaptureRules::default())
            .expect("level ball capture rate should resolve");
        enemy.hp = 20;
        let high_hp = compute_final_catch_rate(&player, &enemy, &context, &CaptureRules::default())
            .expect("level ball capture rate should resolve");
        assert_eq!(low_hp, 200);
        assert_eq!(high_hp, 200);
    }

    #[test]
    fn fast_ball_species_are_explicit_capture_rule_data() {
        let player = pokemon("CHIKORITA", 45, 5, 20, 20);
        let magnemite = pokemon("MAGNEMITE", 45, 10, 1, 20);
        let mut rules = CaptureRules::default();
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
            &CaptureRules::default(),
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
        let mut rules = CaptureRules::default();
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
            &CaptureRules::default(),
        )
        .expect_err("missing Heavy Ball modifier must not be computed as a fallback");

        assert_eq!(
            error,
            CaptureError::MissingHeavyBallModifier("KADABRA".to_string())
        );
    }

    #[test]
    fn capture_rules_json_requires_explicit_pack_fields() {
        let missing_heavy_modifiers =
            serde_json::from_str::<CaptureRules>(r#"{"fast_ball_species":["MAGNEMITE"]}"#)
                .expect_err("heavy ball modifiers must be explicit, even when empty")
                .to_string();
        assert!(
            missing_heavy_modifiers.contains("missing field `heavy_ball_modifiers`"),
            "{missing_heavy_modifiers}"
        );

        let missing_fast_species =
            serde_json::from_str::<CaptureRules>(r#"{"heavy_ball_modifiers":{"KADABRA":40}}"#)
                .expect_err("fast ball species must be explicit, even when empty")
                .to_string();
        assert!(
            missing_fast_species.contains("missing field `fast_ball_species`"),
            "{missing_fast_species}"
        );

        let explicit_empty = serde_json::from_str::<CaptureRules>(
            r#"{"fast_ball_species":[],"heavy_ball_modifiers":{}}"#,
        )
        .expect("empty capture rule sets are valid when explicitly declared");
        assert!(explicit_empty.fast_ball_species.is_empty());
        assert!(explicit_empty.heavy_ball_modifiers.is_empty());
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
            &CaptureRules::default(),
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
            &CaptureRules::default(),
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
            &CaptureRules::default(),
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
            &CaptureRules::default(),
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
        use crate::models::ItemPocket;

        let ball = Item {
            name: "POKE BALL".to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
            price: 200,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: ItemPocket::Ball,
            field_menu: String::new(),
            battle_menu: String::new(),
            script_name: "POKE_BALL".to_string(),
            consumable: true,
            tmhm_index: None,
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
            &CaptureRules::default(),
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
                &CaptureRules::default(),
                &mut rng,
            )
            .expect("empty bag"),
            None
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
}
