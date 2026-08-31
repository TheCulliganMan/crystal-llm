use super::*;
use crate::battle::stats::BattleStatMultiplier;
use crate::models::{
    BaseStats, Dv, Item, LearnedMove, PokemonSpecies, PokemonType, item_pocket, pokemon_type,
};
use crate::random::{CrystalRandomState, ExactBattleRandom, Random, ReplayDivider};

struct ScriptedBattleRandom {
    bytes: std::vec::IntoIter<u8>,
    calls: usize,
}

impl ScriptedBattleRandom {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes.into_iter(),
            calls: 0,
        }
    }
}

impl BattleRandomSource for ScriptedBattleRandom {
    fn battle_random_byte(&mut self) -> u8 {
        self.calls += 1;
        self.bytes.next().expect("scripted battle RNG exhausted")
    }
}

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
        serde_json::from_value::<BattleTurnError>(switch_json).expect("deserialize switch error"),
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
                    (
                        "GRASS".to_string(),
                        crate::battle::damage::TypeMultiplier::one(),
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
    let types = [
        "NORMAL",
        "ELECTRIC",
        "FIGHTING",
        "FLYING",
        "GHOST",
        "GROUND",
        "DARK",
        "ROCK",
        "FIRE",
        "PSYCHIC_TYPE",
        "PSYCHIC",
        "DRAGON",
        "WATER",
        "GRASS",
        "BUG",
        "POISON",
        "STEEL",
        "ICE",
    ];
    let mut matchups: BTreeMap<_, _> = types
        .iter()
        .map(|attacker| {
            (
                pokemon_type(attacker),
                types
                    .iter()
                    .map(|defender| {
                        (
                            pokemon_type(defender),
                            crate::battle::damage::TypeMultiplier::one(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    matchups.entry(pokemon_type("NORMAL")).or_default().insert(
        pokemon_type("GHOST"),
        crate::battle::damage::TypeMultiplier::zero(),
    );
    matchups
        .entry(pokemon_type("ELECTRIC"))
        .or_default()
        .insert(
            pokemon_type("GROUND"),
            crate::battle::damage::TypeMultiplier::zero(),
        );
    matchups
        .entry(pokemon_type("FIGHTING"))
        .or_default()
        .insert(
            pokemon_type("NORMAL"),
            crate::battle::damage::TypeMultiplier {
                numerator: 2,
                denominator: 1,
            },
        );
    matchups
        .entry(pokemon_type("FIGHTING"))
        .or_default()
        .insert(
            pokemon_type("GHOST"),
            crate::battle::damage::TypeMultiplier::zero(),
        );
    matchups.entry(pokemon_type("POISON")).or_default().insert(
        pokemon_type("STEEL"),
        crate::battle::damage::TypeMultiplier::zero(),
    );
    TypeEffectivenessTable {
        matchups,
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
            "PSYCHIC".to_string(),
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
            ("BURN".to_string(), 1),
            ("FLAME_WHEEL".to_string(), 1),
            ("SACRED_FIRE".to_string(), 1),
            ("FREEZE_HIT".to_string(), 1),
            ("PARALYZE_HIT".to_string(), 1),
            ("PARALYZE".to_string(), 1),
            ("SLEEP".to_string(), 1),
            ("POISON_HIT".to_string(), 1),
            ("TOXIC".to_string(), 1),
            ("POISON".to_string(), 1),
            ("FLINCH_HIT".to_string(), 1),
            ("CONFUSE".to_string(), 1),
            ("CONFUSE_HIT".to_string(), 1),
            ("SWAGGER".to_string(), 1),
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
            ("TRI_ATTACK".to_string(), 1),
            ("BATON_PASS".to_string(), 1),
            ("RAPID_SPIN".to_string(), 1),
            ("THIEF".to_string(), 1),
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
            ("TRIPLE_KICK".to_string(), 1),
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
    let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, speed, 65, 65));
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

fn battle_state(player: Pokemon, enemy: Pokemon, _fixture_seed: u32) -> BattleCombatState {
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
    BattleCombatState::new(player.clone(), enemy.clone())
        .with_parties(vec![player, player_bench], vec![enemy, enemy_bench])
}

#[test]
fn trace_is_battle_local_and_restores_the_party_ability_on_switch() {
    let mut player = pokemon("RALTS", 30, pokemon_type("PSYCHIC"), "SPLASH");
    player.species.ability = "TRACE".to_string();
    let mut enemy = pokemon("DUSKULL", 20, pokemon_type("GHOST"), "SPLASH");
    enemy.species.ability = "LEVITATE".to_string();
    let mut state = battle_state(player, enemy, 0);

    apply_switch_in_ability(&mut state, BattleSide::Player).expect("Trace switch-in");
    assert_eq!(state.player.species.ability, "LEVITATE");
    assert_eq!(
        state.player_trace_original_ability.as_deref(),
        Some("TRACE")
    );

    switch_battle_combat_pokemon(&mut state, BattleSide::Player, 1).expect("switch traced user");
    assert_eq!(state.player_party[0].species.ability, "TRACE");
    assert_eq!(state.player_trace_original_ability, None);
}

#[test]
fn soundproof_members_are_not_cured_by_heal_bell() {
    let mut player = pokemon("WHISMUR", 30, pokemon_type("NORMAL"), "HEAL_BELL");
    player.species.ability = "SOUNDPROOF".to_string();
    player.status = Some("POISON".to_string());
    let enemy = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    state.player_party[0] = state.player.clone();
    state.player_party[1].status = Some("BURN".to_string());
    state.player_party[1].species.ability = "NATURAL_CURE".to_string();
    let mut events = Vec::new();

    apply_heal_bell_effect(&mut state, BattleSide::Player, "HEAL_BELL", &mut events);

    assert_eq!(state.player.status.as_deref(), Some("POISON"));
    assert_eq!(state.player_party[0].status.as_deref(), Some("POISON"));
    assert_eq!(state.player_party[1].status, None);
}

#[test]
fn pickup_uses_the_emerald_level_table_after_a_victory() {
    let mut player = pokemon("ZIGZAGOON", 30, pokemon_type("NORMAL"), "TACKLE");
    player.species.ability = "PICKUP".to_string();
    player.level = 1;
    player.item = None;
    let enemy = pokemon("POOCHYENA", 20, pokemon_type("DARK"), "SPLASH");
    let mut state =
        BattleCombatState::new(player.clone(), enemy).with_parties(vec![player], Vec::new());
    let mut rng = ScriptedBattleRandom::new(vec![0, 0, 0, 0]);
    let mut events = Vec::new();

    apply_pickup_after_victory(&mut state, &mut rng, &mut events);

    assert_eq!(state.player.item.as_deref(), Some("POTION"));
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::PickupFound { party_index: 0, item_id }] if item_id == "POTION"
    ));
}

#[test]
fn weather_intimidate_and_forecast_activate_on_switch_in() {
    let mut player = pokemon("CASTFORM", 30, pokemon_type("NORMAL"), "SPLASH");
    player.species.ability = "FORECAST".to_string();
    let mut enemy = pokemon("GROUDON", 20, pokemon_type("GROUND"), "SPLASH");
    enemy.species.ability = "DROUGHT".to_string();
    let mut state = battle_state(player, enemy, 0);

    apply_switch_in_ability(&mut state, BattleSide::Enemy).expect("Drought switch-in");
    apply_switch_in_ability(&mut state, BattleSide::Player).expect("Forecast switch-in");
    assert_eq!(state.weather, Weather::Sun);
    assert_eq!(state.weather_turns, 0);
    assert_eq!(state.player_type_override.as_ref().unwrap().type1, "FIRE");

    state.enemy.species.ability = "INTIMIDATE".to_string();
    apply_switch_in_ability(&mut state, BattleSide::Enemy).expect("Intimidate switch-in");
    assert_eq!(state.player.stat_boosts[&Stat::Attack], -1);
}

#[test]
fn speed_boost_skips_the_switch_in_turn_and_truant_toggles_after_move_attempts() {
    let mut player = pokemon("NINCADA", 30, pokemon_type("BUG"), "SPLASH");
    player.species.ability = "SPEED_BOOST".to_string();
    let mut enemy = pokemon("SLAKOTH", 20, pokemon_type("NORMAL"), "SPLASH");
    enemy.species.ability = "TRUANT".to_string();
    let mut state = battle_state(player, enemy, 0);
    let mut rng = ScriptedBattleRandom::new(Vec::new());
    let mut events = Vec::new();

    apply_end_turn_abilities(&mut state, &mut rng, &mut events).expect("first turn abilities");
    assert_eq!(state.player.stat_boosts[&Stat::Speed], 0);
    advance_truant_after_move_attempt(&mut state, BattleSide::Enemy);
    assert!(state.enemy_truant_loafing);

    apply_end_turn_abilities(&mut state, &mut rng, &mut events).expect("second turn abilities");
    assert_eq!(state.player.stat_boosts[&Stat::Speed], 1);
    advance_truant_after_move_attempt(&mut state, BattleSide::Enemy);
    assert!(!state.enemy_truant_loafing);
}

#[test]
fn white_herb_from_the_pickup_table_restores_all_lowered_stages_once() {
    let mut player = pokemon("ZIGZAGOON", 30, pokemon_type("NORMAL"), "SPLASH");
    player.item = Some("WHITE_HERB".to_string());
    player.stat_boosts.insert(Stat::Attack, -2);
    player.stat_boosts.insert(Stat::Speed, -1);
    player.stat_boosts.insert(Stat::Defense, 2);
    let enemy = pokemon("POOCHYENA", 20, pokemon_type("DARK"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    let mut white_herb = held_boost_item("WHITE_HERB", "HELD_WHITE_HERB");
    white_herb.consumable = true;
    let items = BTreeMap::from([("WHITE_HERB".to_string(), white_herb)]);
    let mut events = Vec::new();

    apply_held_white_herb(&mut state, BattleSide::Player, &items, &mut events)
        .expect("White Herb activation");

    assert_eq!(state.player.stat_boosts[&Stat::Attack], 0);
    assert_eq!(state.player.stat_boosts[&Stat::Speed], 0);
    assert_eq!(state.player.stat_boosts[&Stat::Defense], 2);
    assert_eq!(state.player.item, None);
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::HeldItemStatsRestored { side: BattleSide::Player, item_id, stats }]
            if item_id == "WHITE_HERB" && stats.len() == 2
    ));
}

#[test]
fn hive_badge_adds_minimum_one_bug_damage_only_outside_excluded_battles() {
    let player = pokemon("PARAS", 25, pokemon_type("GRASS"), "FURY_CUTTER");
    let enemy = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "SPLASH");
    let bug_move = move_data("FURY_CUTTER", pokemon_type("BUG"), 1, 100);
    let mut state = battle_state(player, enemy, 0);
    state.badge_boosts_enabled = true;
    state.obedience_badges[1] = true;

    assert!(badge_type_boost_active(
        &state,
        BattleSide::Player,
        &bug_move.move_type,
    ));
    let plain = calculate_damage(
        &state.player,
        &state.enemy,
        &bug_move,
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        DamageContext::default(),
    )
    .expect("plain minimum damage");
    let boosted = calculate_damage(
        &state.player,
        &state.enemy,
        &bug_move,
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        DamageContext {
            attacker_type_badge_boost: badge_type_boost_active(
                &state,
                BattleSide::Player,
                &bug_move.move_type,
            ),
            ..DamageContext::default()
        },
    )
    .expect("Hive Badge minimum damage");

    assert_eq!(plain.damage, 2);
    assert_eq!(boosted.damage, 3);

    let moves = BTreeMap::from([
        (bug_move.name.clone(), bug_move.clone()),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let resolved_damage = |state: BattleCombatState| {
        let mut rng = Random::new(7);
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
        .expect("badge damage turn resolves");
        player_damage_amount(&outcome.events)
    };
    let mut without_badge = state.clone();
    without_badge.obedience_badges[1] = false;
    let regular_damage = resolved_damage(without_badge);
    assert_eq!(resolved_damage(state.clone()), regular_damage + 1);

    state.link_battle = true;
    assert!(!badge_type_boost_active(
        &state,
        BattleSide::Player,
        &bug_move.move_type,
    ));
    assert_eq!(resolved_damage(state.clone()), regular_damage);
    state.link_battle = false;
    state.badge_boosts_enabled = false; // wInBattleTowerBattle gate.
    assert!(!badge_type_boost_active(
        &state,
        BattleSide::Player,
        &bug_move.move_type,
    ));
    assert_eq!(resolved_damage(state), regular_damage);
}

#[test]
fn mineral_badge_alone_boosts_defense_and_storm_badge_does_not() {
    let player = pokemon("STEELIX", 30, pokemon_type("STEEL"), "TACKLE");
    let enemy = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    state.badge_boosts_enabled = true;
    state.obedience_badges[4] = true;

    assert!(badge_boost_active(
        &state,
        BattleSide::Player,
        Stat::Defense,
    ));

    state.obedience_badges[4] = false;
    state.obedience_badges[5] = true;
    assert!(!badge_boost_active(
        &state,
        BattleSide::Player,
        Stat::Defense,
    ));
}

#[test]
fn glacier_badge_special_defense_boost_preserves_the_asm_accumulator_bug() {
    let player = pokemon("DELIBIRD", 30, pokemon_type("ICE"), "TACKLE");
    let enemy = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    state.badge_boosts_enabled = true;
    state.obedience_badges[6] = true;

    for (special_attack, expected) in [
        (205, false),
        (206, true),
        (432, true),
        (433, false),
        (660, false),
        (661, true),
    ] {
        state.player.special_attack = special_attack;
        assert!(badge_boost_active(
            &state,
            BattleSide::Player,
            Stat::SpecialAttack,
        ));
        assert_eq!(
            badge_boost_active(&state, BattleSide::Player, Stat::SpecialDefense),
            expected,
            "unexpected Glacier Badge Special Defense result at Special Attack {special_attack}"
        );
    }
}

#[test]
fn cascade_badge_applies_the_kanto_water_type_boost() {
    let player = pokemon("TOTODILE", 30, pokemon_type("WATER"), "WATER_GUN");
    let enemy = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    state.badge_boosts_enabled = true;
    state.kanto_badges[1] = true;

    assert!(badge_type_boost_active(&state, BattleSide::Player, "WATER",));
    assert!(!badge_type_boost_active(&state, BattleSide::Player, "FIRE",));
}

#[test]
fn active_battle_combat_carries_both_badge_regions_and_link_gate() {
    let mut player = pokemon("TOTODILE", 30, pokemon_type("WATER"), "WATER_GUN");
    player.species.int_id = 158;
    let mut enemy = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "SPLASH");
    enemy.species.int_id = 19;
    let mut state = GameState::default();
    state.storage.party.pokemon[0] = Some(player);
    state.sync_party_from_storage();
    state.battle_active_party_index = Some(0);
    state.battle_active_enemy_party_index = Some(0);
    state.battle = BattleMemory::Wild {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "ROUTE_29".to_string(),
        roaming_slot: None,
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy],
    };
    state.badges.johto[1] = true;
    state.badges.kanto[1] = true;

    let combat = active_battle_combat_state(&state).expect("ordinary combat state");
    assert!(combat.obedience_badges[1]);
    assert!(combat.kanto_badges[1]);
    assert!(combat.badge_boosts_enabled);

    state.link_session.link_mode = 1;
    state.link_session.battle_random = Some(crate::random::LinkBattleRandomState {
        seeds: [0; 10],
        count: 0,
    });
    let missing_owner_error = serde_json::from_value::<GameState>(
        serde_json::to_value(&state).expect("serialize unowned active link battle"),
    )
    .expect_err("active link battle requires a serial clock owner")
    .to_string();
    assert!(
        missing_owner_error.contains("requires an established serial clock owner"),
        "{missing_owner_error}"
    );
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    let link_combat = active_battle_combat_state(&state).expect("link combat state");
    assert!(link_combat.link_battle);
    assert_eq!(
        link_combat.serial_connection_status,
        LinkSerialConnectionStatus::UsingInternalClock
    );
    assert!(!link_combat.badge_boosts_enabled);
}

#[test]
fn commit_battle_turn_outcome_updates_party_and_enemy_together() {
    let mut state = GameState::default();
    let mut player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
    let mut enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
    state.storage.party.pokemon[0] = Some(player.clone());
    state.battle_active_enemy_party_index = Some(0);
    state.battle = crate::state::BattleMemory::Wild {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "ROUTE_29".to_string(),
        roaming_slot: None,
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy.clone()],
    };

    player.hp = 31;
    enemy.hp = 4;
    let mut combat_state = BattleCombatState::new(player.clone(), enemy.clone())
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

    assert_eq!(state.battle_pay_day_money, 35);
    assert_eq!(state.storage.party.pokemon[0].as_ref().unwrap().hp, 31);
    assert_eq!(
        state.script_runtime.active_battle_combat,
        Some(outcome.state.clone())
    );
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
fn commit_battle_turn_outcome_accumulates_enemy_pay_day_with_24_bit_wrap() {
    let mut state = GameState::default();
    let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
    let enemy = pokemon("MEOWTH", 72, pokemon_type("NORMAL"), "PAY_DAY");
    state.storage.party.pokemon[0] = Some(player.clone());
    state.battle_active_enemy_party_index = Some(0);
    state.battle_pay_day_money = 0x00ff_fff8;
    state.battle = crate::state::BattleMemory::Wild {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "ROUTE_29".to_string(),
        roaming_slot: None,
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy.clone()],
    };
    let combat_state = BattleCombatState::new(player.clone(), enemy.clone())
        .with_parties(vec![player], vec![enemy]);
    let outcome = BattleTurnOutcome {
        state: combat_state,
        order: Vec::new(),
        events: vec![BattleEvent::PayDayMoney {
            side: BattleSide::Enemy,
            move_name: "PAY_DAY".to_string(),
            amount: 10,
        }],
    };

    commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit enemy Pay Day");

    assert_eq!(state.battle_pay_day_money, 2);
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
        roaming_slot: None,
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy.clone()],
    };
    let mut resolved_player = player.clone();
    resolved_player.status = None;

    let outcome = BattleTurnOutcome {
        state: BattleCombatState::new(resolved_player, enemy),
        order: Vec::new(),
        events: vec![BattleEvent::HealBellChimed {
            side: BattleSide::Player,
            active_status_before: Some("BAD_POISON".to_string()),
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
fn commit_player_faint_applies_exact_happiness_loss_to_saved_and_combat_state() {
    for (enemy_level, happiness_before, happiness_after) in
        [(20, 220, 219), (50, 150, 145), (50, 220, 210)]
    {
        let mut state = GameState::default();
        let mut player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        player.level = 20;
        player.hp = 0;
        player.happiness = happiness_before;
        let mut enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        enemy.level = enemy_level;
        state.storage.party.pokemon[0] = Some(player.clone());
        state.battle_active_enemy_party_index = Some(0);
        state.battle = crate::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            roaming_slot: None,
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };
        let outcome = BattleTurnOutcome {
            state: BattleCombatState::new(player, enemy),
            order: Vec::new(),
            events: vec![BattleEvent::Fainted {
                side: BattleSide::Player,
            }],
        };

        commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit fainted player");

        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().happiness,
            happiness_after
        );
        assert_eq!(
            state
                .script_runtime
                .active_battle_combat
                .as_ref()
                .unwrap()
                .player
                .happiness,
            happiness_after
        );
    }
}

#[test]
fn committing_player_faint_clears_persistent_poison_burn_and_sleep_without_healing_hp() {
    for (status, sleep_turns) in [("POISON", 0), ("BURN", 0), ("SLEEP", 3)] {
        let mut state = GameState::default();
        let mut player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
        player.hp = 0;
        player.status = Some(status.to_string());
        player.sleep_turns = sleep_turns;
        let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
        state.storage.party.pokemon[0] = Some(player.clone());
        state.battle_active_enemy_party_index = Some(0);
        state.battle = crate::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            roaming_slot: None,
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };
        let outcome = BattleTurnOutcome {
            state: BattleCombatState::new(player, enemy),
            order: Vec::new(),
            events: vec![BattleEvent::Fainted {
                side: BattleSide::Player,
            }],
        };

        commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit fainted player status");

        let saved = state.storage.party.pokemon[0]
            .as_ref()
            .expect("fainted party slot");
        assert_eq!(saved.hp, 0, "{status}");
        assert_eq!(saved.status, None, "{status}");
        assert_eq!(saved.sleep_turns, 0, "{status}");
        let combat = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .expect("committed combat state");
        assert_eq!(combat.player.hp, 0, "{status}");
        assert_eq!(combat.player.status, None, "{status}");
        assert_eq!(combat.player.sleep_turns, 0, "{status}");
    }
}

#[test]
fn pursuit_switch_faint_preserves_the_outgoing_players_old_status_and_happiness() {
    let mut state = GameState::default();
    let mut player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "TACKLE");
    player.hp = 0;
    player.status = Some("POISON".to_string());
    player.happiness = 200;
    let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "PURSUIT");
    state.storage.party.pokemon[0] = Some(player.clone());
    state.battle_active_enemy_party_index = Some(0);
    state.battle = crate::state::BattleMemory::Wild {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "ROUTE_29".to_string(),
        roaming_slot: None,
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy.clone()],
    };
    let outcome = BattleTurnOutcome {
        state: BattleCombatState::new(player, enemy),
        order: vec![BattleSide::Enemy, BattleSide::Player],
        events: vec![
            BattleEvent::PursuitPower {
                side: BattleSide::Enemy,
                move_name: "PURSUIT".to_string(),
                target: BattleSide::Player,
                power: 80,
            },
            BattleEvent::Fainted {
                side: BattleSide::Player,
            },
        ],
    };

    commit_battle_turn_outcome(&mut state, 0, &outcome).expect("commit PursuitSwitch player faint");

    let saved = state.storage.party.pokemon[0]
        .as_ref()
        .expect("Pursuit-fainted party slot");
    assert_eq!(saved.hp, 0);
    assert_eq!(saved.status.as_deref(), Some("POISON"));
    assert_eq!(saved.happiness, 200);
    let combat = state
        .script_runtime
        .active_battle_combat
        .as_ref()
        .expect("committed combat state");
    assert_eq!(combat.player.status.as_deref(), Some("POISON"));
    assert_eq!(combat.player.happiness, 200);
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
            roaming_slot: None,
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };

        let mut combat_state = BattleCombatState::new(player.clone(), enemy.clone())
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
        assert_eq!(state.battle_result, 2, "{side:?} flee is DRAW");
    }
}

#[test]
fn commit_wild_battle_escape_attempt_updates_attempts_and_deactivates_on_success() {
    let mut state = GameState::default();
    let enemy = pokemon("RATTATA", 72, pokemon_type("NORMAL"), "TACKLE");
    state.battle = crate::state::BattleMemory::Wild {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "ROUTE_29".to_string(),
        roaming_slot: None,
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
        },
    );
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
        },
    );
    assert_eq!(state.battle_escape_attempts, 0);
    assert_eq!(state.battle, crate::state::BattleMemory::Inactive);
    assert_eq!(state.battle_active_party_index, None);
    assert_eq!(state.battle_result, 2);
    assert_eq!(state.battle_active_enemy_party_index, None);
}

fn move_data(name: &str, move_type: PokemonType, power: u16, accuracy: u8) -> Move {
    move_data_with_effect(name, move_type, power, accuracy, "NORMAL_HIT")
}

fn battle_item(id: &str, heal_amount: i16, battle_usable: bool) -> Item {
    Item {
        name: id.replace('_', " "),
        description: String::new(),
        effect: if heal_amount != 0 {
            "RESTORE_HP".to_string()
        } else {
            "NONE".to_string()
        },
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

fn full_status_item(id: &str, full_restore: bool) -> Item {
    let mut item = battle_item(id, if full_restore { -1 } else { 0 }, true);
    item.status_heals = vec![
        "SLEEP".to_string(),
        "POISON".to_string(),
        "BURN".to_string(),
        "FREEZE".to_string(),
        "PARALYSIS".to_string(),
    ];
    item.confusion_heal = Some(true);
    item
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
        source_index: 1,
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

fn apply_test_damage_hit(
    state: &mut BattleCombatState,
    move_data: &Move,
    items: &BTreeMap<String, Item>,
    rng: &mut Random,
    events: &mut Vec<BattleEvent>,
) -> DamageHitResult {
    apply_damage_hit(
        state,
        BattleSide::Player,
        &move_data.name,
        move_data,
        1,
        false,
        false,
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        items,
        rng,
        events,
    )
    .expect("test damage hit resolves")
}

fn consume_standard_damage_calculation_rng(rng: &mut Random) {
    let _critical_roll = rng.battle_random_byte();
    let _damage_roll = crystal_damage_variation_roll(rng);
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
        source_index: 1,
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
    let thunder = move_data_with_effect("THUNDER", pokemon_type("ELECTRIC"), 120, 70, "THUNDER");

    let rain_accuracy = accuracy_byte_with_weather(
        &thunder,
        BattleSide::Player,
        &attacker,
        &defender,
        &stat_multipliers(),
        Weather::Rain,
        false,
    )
    .expect("rain thunder accuracy resolves");
    let sun_accuracy = accuracy_byte_with_weather(
        &thunder,
        BattleSide::Player,
        &attacker,
        &defender,
        &stat_multipliers(),
        Weather::Sun,
        false,
    )
    .expect("sun thunder accuracy resolves");

    assert_eq!(rain_accuracy, u8::MAX);
    assert_eq!(sun_accuracy, 128);
}

#[test]
fn foresight_ignores_non_advantageous_accuracy_evasion_levels_only() {
    let mut attacker = pokemon("PIDGEY", 30, pokemon_type("FLYING"), "TACKLE");
    let mut defender = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    attacker.stat_boosts.insert(Stat::Accuracy, -6);
    defender.stat_boosts.insert(Stat::Evasion, 6);
    let tackle = move_data("TACKLE", pokemon_type("NORMAL"), 35, 75);

    let unidentified = accuracy_byte_with_weather(
        &tackle,
        BattleSide::Player,
        &attacker,
        &defender,
        &stat_multipliers(),
        Weather::None,
        false,
    )
    .expect("ordinary accuracy/evasion resolves");
    let identified = accuracy_byte_with_weather(
        &tackle,
        BattleSide::Player,
        &attacker,
        &defender,
        &stat_multipliers(),
        Weather::None,
        true,
    )
    .expect("Foresight accuracy/evasion resolves");

    assert!(unidentified < identified);
    assert_eq!(identified, (75 * 255 / 100) as u8);

    attacker.stat_boosts.insert(Stat::Accuracy, 6);
    defender.stat_boosts.insert(Stat::Evasion, -6);
    let identified_advantage = accuracy_byte_with_weather(
        &tackle,
        BattleSide::Player,
        &attacker,
        &defender,
        &stat_multipliers(),
        Weather::None,
        true,
    )
    .expect("advantageous identified accuracy resolves");
    assert!(identified_advantage > identified);
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
    assert_ne!(rng.seed(), 7);
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
fn zero_pp_party_automatically_uses_struggle() {
    let mut player = pokemon("PIKACHU", 90, pokemon_type("ELECTRIC"), "TACKLE");
    let mut enemy = pokemon("GEODUDE", 20, pokemon_type("ROCK"), "TACKLE");
    player.moves[0].current_pp = 0;
    enemy.moves[0].current_pp = 0;
    let mut moves = BTreeMap::new();
    moves.insert(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    );
    moves.insert(
        "STRUGGLE".to_string(),
        move_data("STRUGGLE", pokemon_type("NORMAL"), 50, 100),
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
    .expect("struggle turn resolves");

    assert!(outcome.events.iter().any(|event| {
        matches!(event, BattleEvent::MoveUsed { move_name, .. } if move_name == "STRUGGLE")
    }));
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::NoPp { .. }))
    );
    assert_eq!(outcome.state.player.moves[0].current_pp, 0);
    assert_eq!(outcome.state.enemy.moves[0].current_pp, 0);
}

#[test]
fn only_disabled_move_automatically_uses_struggle() {
    let player = pokemon("PIKACHU", 90, pokemon_type("ELECTRIC"), "TACKLE");
    let enemy = pokemon("GEODUDE", 20, pokemon_type("ROCK"), "TACKLE");
    let mut state = battle_state(player, enemy, 7);
    state.player_disable = Some(BattleDisableState {
        move_name: "TACKLE".to_string(),
        turns_remaining: 3,
    });
    state.player_substitute_hp = 20;
    state.player_nightmare_source = Some(BattleSide::Enemy);
    state.player_trap = Some(BattleTrapState {
        source: BattleSide::Enemy,
        move_name: "WRAP".to_string(),
        turns_remaining: 3,
    });
    state.enemy_trap = Some(BattleTrapState {
        source: BattleSide::Player,
        move_name: "BIND".to_string(),
        turns_remaining: 2,
    });
    state.enemy_disable = Some(BattleDisableState {
        move_name: "TACKLE".to_string(),
        turns_remaining: 3,
    });
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "STRUGGLE".to_string(),
            move_data("STRUGGLE", pokemon_type("NORMAL"), 50, 100),
        ),
    ]);
    let mut rng = Random::new(7);

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
    .expect("disabled-only battlers use Struggle");

    assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(event, BattleEvent::MoveUsed { move_name, .. } if move_name == "STRUGGLE"))
                .count(),
            2
        );
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::DisabledMove { .. } | BattleEvent::NoPp { .. }
    )));
}

#[test]
fn pp_up_bits_on_a_zero_pp_move_preserve_the_player_disable_struggle_bug() {
    let mut player = pokemon("PIKACHU", 90, pokemon_type("ELECTRIC"), "TACKLE");
    player.moves.push(LearnedMove {
        name: "GROWL".to_string(),
        current_pp: 0,
        pp_ups: 1,
    });
    let enemy = pokemon("GEODUDE", 20, pokemon_type("ROCK"), "SPLASH");
    let mut state = battle_state(player, enemy, 7);
    state.player_disable = Some(BattleDisableState {
        move_name: "TACKLE".to_string(),
        turns_remaining: 3,
    });
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "GROWL".to_string(),
            move_data_with_effect("GROWL", pokemon_type("NORMAL"), 0, 100, "ATTACK_DOWN"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
        (
            "STRUGGLE".to_string(),
            move_data("STRUGGLE", pokemon_type("NORMAL"), 50, 100),
        ),
    ]);
    let mut rng = Random::new(7);

    let outcome = resolve_battle_turn(
        state,
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
    .expect("PP Up Disable bug turn resolves");

    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::NoPp {
            side: BattleSide::Player,
            move_name,
        } if move_name == "GROWL"
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed { move_name, .. } if move_name == "STRUGGLE"
    )));
}

#[test]
fn zero_pp_party_rejects_a_catalog_without_struggle() {
    let mut player = pokemon("PIKACHU", 90, pokemon_type("ELECTRIC"), "TACKLE");
    let enemy = pokemon("GEODUDE", 20, pokemon_type("ROCK"), "TACKLE");
    player.moves[0].current_pp = 0;
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut rng = Random::new(7);
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
    .expect_err("the canonical STRUGGLE definition is required");

    assert_eq!(
        error,
        BattleTurnError::MissingMoveData {
            side: BattleSide::Player,
            move_name: "STRUGGLE".to_string(),
        }
    );
}

#[test]
fn held_type_boost_item_increases_matching_move_damage_from_item_catalog() {
    let mut boosted_player = pokemon("DRATINI", 90, pokemon_type("DRAGON"), "DRAGONBREATH");
    boosted_player.item = Some("DRAGON_FANG".to_string());
    let plain_player = pokemon("DRATINI", 90, pokemon_type("DRAGON"), "DRAGONBREATH");
    let enemy = pokemon("WOOPER", 20, pokemon_type("WATER"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "DRAGONBREATH".to_string(),
            move_data("DRAGONBREATH", pokemon_type("DRAGON"), 60, 100),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut dragon_fang = held_boost_item("DRAGON_FANG", "HELD_DRAGON_BOOST");
    dragon_fang.parameter = 10;
    let items = BTreeMap::from([("DRAGON_FANG".to_string(), dragon_fang)]);

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
    assert!(boosted_damage > plain_damage);
    assert!(boosted.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::HeldItemDamageBoost {
                side: BattleSide::Player,
                item_id,
                held_effect,
                move_type,
                parameter,
            } if item_id == "DRAGON_FANG"
                && held_effect == "HELD_DRAGON_BOOST"
                && move_type == "DRAGON"
                && *parameter == 10
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
fn fury_cutter_doubles_post_formula_damage_and_wraps_its_source_counter() {
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
    let mut second_hit_state = battle_state(player.clone(), enemy.clone(), 0);
    second_hit_state.player_fury_cutter_chain = 1;
    let mut second_hit_rng =
        ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX, 0, u8::MAX, u8::MAX]);
    let second_hit = resolve_battle_turn(
        second_hit_state,
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
        &mut second_hit_rng,
    )
    .expect("second Fury Cutter hit resolves");
    let second_hit_raw_damage = second_hit
        .events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                result,
                ..
            } if move_name == "FURY_CUTTER" => Some(result.damage),
            _ => None,
        })
        .expect("second Fury Cutter hit deals damage");
    assert_eq!(second_hit_raw_damage, 18);
    assert_eq!(second_hit.state.player_fury_cutter_chain, 2);

    let mut substitute_state = battle_state(player.clone(), enemy.clone(), 0);
    substitute_state.player_fury_cutter_chain = 1;
    set_substitute_hp(&mut substitute_state, BattleSide::Enemy, 20);
    let mut substitute_rng =
        ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX, 0, u8::MAX, u8::MAX]);
    let substitute_hit = resolve_battle_turn(
        substitute_state,
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
        &mut substitute_rng,
    )
    .expect("Fury Cutter hit into Substitute resolves");
    assert!(substitute_hit.events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteDamaged {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "FURY_CUTTER"
    )));
    assert_eq!(substitute_hit.state.player_fury_cutter_chain, 2);

    let mut wrapped_state = battle_state(player, enemy, 0);
    wrapped_state.player_fury_cutter_chain = u8::MAX;
    let mut wrapped_rng = ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX, 0, u8::MAX, u8::MAX]);
    let wrapped = resolve_battle_turn(
        wrapped_state,
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
        &mut wrapped_rng,
    )
    .expect("wrapped Fury Cutter hit resolves");
    let wrapped_raw_damage = wrapped
        .events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                result,
                ..
            } if move_name == "FURY_CUTTER" => Some(result.damage),
            _ => None,
        })
        .expect("wrapped Fury Cutter hit deals damage");
    assert_eq!(wrapped_raw_damage, u16::MAX);
    assert_eq!(wrapped.state.player_fury_cutter_chain, 0);
}

#[test]
fn fury_cutter_miss_resets_chain() {
    let player = pokemon("SCYTHER", 90, pokemon_type("BUG"), "FURY_CUTTER");
    let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "FURY_CUTTER".to_string(),
            move_data_with_effect("FURY_CUTTER", pokemon_type("BUG"), 10, 1, "FURY_CUTTER"),
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
        current_pp: 0,
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
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
            move_data_with_effect("ROLLOUT", pokemon_type("ROCK"), 30, 1, "ROLLOUT"),
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
    assert_eq!(outcome.state.player_rollout_chain, 2);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Missed {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "ROLLOUT"
    )));

    let mut fresh_rng = ScriptedBattleRandom::new(vec![u8::MAX; 6]);
    let fresh_attempt = resolve_battle_turn(
        outcome.state,
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
        &mut fresh_rng,
    )
    .expect("fresh Rollout attempt resolves after the miss");
    assert_eq!(fresh_attempt.state.player_rollout_turns, 0);
    assert_eq!(fresh_attempt.state.player_rollout_chain, 0);
}

#[test]
fn rollout_retains_its_raw_count_after_substitute_and_terminal_hits() {
    let player = pokemon("GEODUDE", 90, pokemon_type("ROCK"), "ROLLOUT");
    let enemy = pokemon("SNORLAX", 10, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "ROLLOUT".to_string(),
            move_data_with_effect("ROLLOUT", pokemon_type("ROCK"), 30, 100, "ROLLOUT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
        ),
    ]);

    let mut substitute_state = battle_state(player.clone(), enemy.clone(), 0);
    substitute_state.player_rollout_chain = 1;
    substitute_state.player_rollout_turns = 4;
    set_substitute_hp(&mut substitute_state, BattleSide::Enemy, u16::MAX);
    let mut substitute_rng =
        ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX, 0, u8::MAX, u8::MAX]);
    let substitute_hit = resolve_battle_turn(
        substitute_state,
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
        &mut substitute_rng,
    )
    .expect("Rollout hit into Substitute resolves");
    assert!(substitute_hit.events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteDamaged {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "ROLLOUT"
    )));
    assert_eq!(substitute_hit.state.player_rollout_chain, 2);
    assert_eq!(substitute_hit.state.player_rollout_turns, 3);

    let mut terminal_state = battle_state(player, enemy, 0);
    terminal_state.player_rollout_chain = 4;
    terminal_state.player_rollout_turns = 1;
    let mut terminal_rng =
        ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX, 0, u8::MAX, u8::MAX]);
    let terminal_hit = resolve_battle_turn(
        terminal_state,
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
        &mut terminal_rng,
    )
    .expect("terminal Rollout hit resolves");
    assert_eq!(terminal_hit.state.player_rollout_chain, 5);
    assert_eq!(terminal_hit.state.player_rollout_turns, 0);
}

#[test]
fn rage_builds_a_separate_counter_when_active_user_is_damaged() {
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
    assert_eq!(outcome.state.player_rage_counter, 1);
    assert_eq!(
        outcome.state.player.stat_boosts.get(&Stat::Attack),
        Some(&0)
    );
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::RageBuilding {
            side: BattleSide::Player,
            counter: 1,
        }
    )));
}

#[test]
fn rage_counter_at_255_suppresses_the_building_event() {
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
    let mut state = battle_state(player, enemy, rng.seed());
    state.player.speed = 1;
    state.enemy.speed = 1_000;
    state.player_rage_active = true;
    state.player_rage_counter = u8::MAX;

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
    .expect("terminal Rage counter turn resolves");

    assert_eq!(outcome.state.player_rage_counter, u8::MAX);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::RageBuilding {
            side: BattleSide::Player,
            ..
        }
    )));
}

#[test]
fn successful_move_execution_wraps_the_source_turn_counters() {
    let player = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut rng = Random::new(28);
    let mut state = battle_state(player, enemy, rng.seed());
    state.player_turns_taken = u8::MAX;
    state.enemy_turns_taken = u8::MAX;

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
    .expect("turn-counter wrap turn resolves");

    assert_eq!(outcome.state.player_turns_taken, 0);
    assert_eq!(outcome.state.enemy_turns_taken, 0);
}

#[test]
fn switching_clears_rage_active_state() {
    let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "RAGE");
    let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    state.player_rage_active = true;
    state.player_rage_counter = 3;

    clear_side_volatile_conditions(&mut state, BattleSide::Player);

    assert!(!state.player_rage_active);
    assert_eq!(state.player_rage_counter, 0);
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
            move_data("TACKLE", pokemon_type("NORMAL"), 60, 100),
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
    let (bide_turns, bide_roll) = outcome
        .events
        .iter()
        .find_map(|event| match event {
            BattleEvent::BideStarted {
                side: BattleSide::Player,
                move_name,
                turns,
                roll,
            } if move_name == "BIDE" => Some((*turns, *roll)),
            _ => None,
        })
        .expect("Bide start event");
    assert_eq!(bide_turns, (bide_roll & 1) + 2);
    assert_eq!(outcome.state.player_bide_turns, bide_turns);
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
fn active_bide_storeenergy_precedes_and_can_skip_doturn() {
    let mut player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
    player.status = Some("SLEEP".to_string());
    player.sleep_turns = 2;
    let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "SPLASH");
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
    let mut state = battle_state(player, enemy, 0);
    state.player_last_move = Some("BIDE".to_string());
    state.player_bide_turns = 2;
    state.player_bide_damage = 12;
    let mut rng = ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX]);

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
    .expect("active Bide storage turn resolves");

    assert_eq!(outcome.state.player_bide_turns, 1);
    assert_eq!(outcome.state.player_bide_damage, 12);
    assert_eq!(outcome.state.player.sleep_turns, 2);
    assert!(outcome.events.contains(&BattleEvent::BideStoring {
        side: BattleSide::Player,
        move_name: "BIDE".to_string(),
        turns_remaining: 1,
        stored_damage: 12,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SleepTurn {
            side: BattleSide::Player,
            ..
        }
    )));
}

#[test]
fn bide_prepares_release_before_doturn_can_replace_it_with_confusion_damage() {
    let mut player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
    player.confusion_turns = 2;
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
    let mut state = battle_state(player, enemy, 0);
    state.player_last_move = Some("BIDE".to_string());
    state.player_bide_turns = 1;
    state.player_bide_damage = 12;
    let mut rng = ScriptedBattleRandom::new(vec![0, u8::MAX]);

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
    .expect("Bide release interrupted by confusion resolves");

    assert_eq!(outcome.state.player_bide_turns, 0);
    assert_eq!(outcome.state.player_bide_damage, 0);
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(outcome.events.contains(&BattleEvent::BideUnleashed {
        side: BattleSide::Player,
        move_name: "BIDE".to_string(),
        stored_damage: 12,
    }));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ConfusionSelfDamage {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "BIDE"
    )));
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::BideReleased { .. }))
    );
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
    let mut rng = Random::new(31);
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
fn bide_reaches_checkhit_only_on_release_and_protect_preserves_lock_on() {
    let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
    let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "PROTECT");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "BIDE".to_string(),
            move_data_with_effect("BIDE", pokemon_type("NORMAL"), 0, 100, "BIDE"),
        ),
        (
            "PROTECT".to_string(),
            move_data_with_effect("PROTECT", pokemon_type("NORMAL"), 0, 100, "PROTECT"),
        ),
    ]);
    let mut start_rng = Random::new(37);
    let start = resolve_battle_turn(
        battle_state(player.clone(), enemy.clone(), start_rng.seed()),
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
        &mut start_rng,
    )
    .expect("Bide start before checkhit resolves");
    assert!(start.events.iter().any(|event| matches!(
        event,
        BattleEvent::BideStarted {
            side: BattleSide::Player,
            ..
        }
    )));
    assert!(!start.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveProtected {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "BIDE"
    )));

    let mut rng = Random::new(37);
    let mut state = battle_state(player, enemy, rng.seed());
    state.player_last_move = Some("BIDE".to_string());
    state.player_bide_turns = 1;
    state.player_bide_damage = 12;
    state.player_lock_on_target = true;

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
    .expect("protected Bide release resolves");

    assert_eq!(outcome.state.player_bide_turns, 0);
    assert_eq!(outcome.state.player_bide_damage, 0);
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(outcome.state.player_lock_on_target);
    assert!(outcome.events.contains(&BattleEvent::MoveProtected {
        side: BattleSide::Player,
        move_name: "BIDE".to_string(),
        target: BattleSide::Enemy,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::BideReleased { .. } | BattleEvent::LockOnConsumed { .. }
    )));
}

#[test]
fn bide_release_uses_substitute_rage_and_kings_rock_tail() {
    let mut player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
    player.item = Some("KINGS_ROCK".to_string());
    let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "RAGE");
    let enemy_hp = enemy.hp;
    let mut state = battle_state(player, enemy, 0);
    state.enemy_substitute_hp = 10;
    state.enemy_rage_active = true;
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 255;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let move_data = move_data_with_effect("BIDE", pokemon_type("NORMAL"), 0, 100, "BIDE");
    let mut rng = Random::new(20);
    let mut events = Vec::new();

    apply_bide_release_effect(
        &mut state,
        BattleSide::Player,
        "BIDE",
        &move_data,
        12,
        &type_categories(),
        &items,
        &mut rng,
        &mut events,
    )
    .expect("Bide Substitute tail resolves");

    assert_eq!(state.enemy.hp, enemy_hp);
    assert_eq!(state.enemy_substitute_hp, 0);
    assert_eq!(state.enemy_rage_counter, 1);
    assert!(state.enemy.flinching);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteDamaged {
            side: BattleSide::Player,
            damage: 10,
            ..
        }
    )));
    assert!(events.contains(&BattleEvent::BideReleased {
        side: BattleSide::Player,
        move_name: "BIDE".to_string(),
        target: BattleSide::Enemy,
        stored_damage: 12,
        damage: 0,
        target_hp_before: enemy_hp,
        target_hp_after: enemy_hp,
    }));
    assert!(events.contains(&BattleEvent::RageBuilding {
        side: BattleSide::Enemy,
        counter: 1,
    }));
    assert!(events.contains(&BattleEvent::FlinchApplied {
        side: BattleSide::Player,
        move_name: "BIDE".to_string(),
        target: BattleSide::Enemy,
    }));
}

#[test]
fn bide_release_uses_survival_retained_damage_and_destiny_bond_tail() {
    let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "BIDE");
    let mut enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "BIDE");
    enemy.hp = 20;
    let mut endured = battle_state(player.clone(), enemy.clone(), 0);
    endured.enemy_endure_active = true;
    endured.enemy_bide_turns = 2;
    let move_data = move_data_with_effect("BIDE", pokemon_type("NORMAL"), 0, 100, "BIDE");
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_bide_release_effect(
        &mut endured,
        BattleSide::Player,
        "BIDE",
        &move_data,
        12,
        &type_categories(),
        &BTreeMap::new(),
        &mut rng,
        &mut events,
    )
    .expect("Bide Endure tail resolves");

    assert_eq!(endured.enemy.hp, 1);
    assert_eq!(endured.enemy_bide_damage, 19);
    assert_eq!(
        endured.enemy_last_damage,
        Some(BattleLastDamageState {
            source: BattleSide::Player,
            move_name: "BIDE".to_string(),
            category: BattleDamageCategory::Physical,
            damage: 19,
        }),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::EnduredHit {
            target: BattleSide::Enemy,
            held_item: None,
            ..
        }
    )));
    assert!(events.contains(&BattleEvent::BideStoredDamage {
        side: BattleSide::Enemy,
        source: BattleSide::Player,
        damage: 19,
        stored_damage: 19,
    }));

    let mut focus_enemy = enemy.clone();
    focus_enemy.item = Some("FOCUS_BAND".to_string());
    let mut focused = battle_state(player.clone(), focus_enemy, 0);
    let mut focus_band = held_boost_item("FOCUS_BAND", "HELD_FOCUS_BAND");
    focus_band.parameter = 255;
    let focus_items = BTreeMap::from([("FOCUS_BAND".to_string(), focus_band)]);
    let mut focused_events = Vec::new();
    apply_bide_release_effect(
        &mut focused,
        BattleSide::Player,
        "BIDE",
        &move_data,
        12,
        &type_categories(),
        &focus_items,
        &mut Random::new(1),
        &mut focused_events,
    )
    .expect("Bide Focus Band tail resolves");
    assert_eq!(focused.enemy.hp, 1);
    assert!(focused_events.iter().any(|event| matches!(
        event,
        BattleEvent::EnduredHit {
            target: BattleSide::Enemy,
            held_item: Some(item),
            ..
        } if item == "FOCUS_BAND"
    )));

    let mut bonded = battle_state(player, enemy, 0);
    bonded.enemy_destiny_bond_active = true;
    let mut bonded_events = Vec::new();
    apply_bide_release_effect(
        &mut bonded,
        BattleSide::Player,
        "BIDE",
        &move_data,
        12,
        &type_categories(),
        &BTreeMap::new(),
        &mut Random::new(1),
        &mut bonded_events,
    )
    .expect("Bide Destiny Bond tail resolves");
    assert_eq!(bonded.player.hp, 0);
    assert_eq!(bonded.enemy.hp, 0);
    assert!(bonded_events.iter().any(|event| matches!(
        event,
        BattleEvent::DestinyBondActivated {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            move_name,
            ..
        } if move_name == "BIDE"
    )));
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
            enemy: BattleAction::Move { slot: 0 },
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
            enemy: BattleAction::Move { slot: 0 },
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
fn high_critical_moves_use_the_asm_two_stage_bonus() {
    let attacker = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SLASH");
    for move_name in [
        "KARATE_CHOP",
        "RAZOR_WIND",
        "RAZOR_LEAF",
        "CRABHAMMER",
        "SLASH",
        "AEROBLAST",
        "CROSS_CHOP",
    ] {
        let mut rng = Random::new(7);
        let (_, _, threshold) = roll_critical_hit(
            BattleSide::Player,
            move_name,
            &attacker,
            &BTreeMap::new(),
            &mut rng,
        )
        .expect("critical roll resolves");
        assert_eq!(threshold, 64, "{move_name}");
    }
}

#[test]
fn traded_overlevel_pokemon_can_disobey_until_badges_raise_the_cap() {
    let mut player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "TACKLE");
    player.original_trainer_id = 2;
    let mut state = battle_state(
        player.clone(),
        pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "TACKLE"),
        0,
    )
    .with_obedience(1, [false; 8]);
    let mut disobeyed = false;
    for seed in 0..256 {
        let mut rng = Random::new(seed);
        let mut events = Vec::new();
        if player_disobeys(&state, &mut rng, &mut events) {
            disobeyed = events.contains(&BattleEvent::Disobeyed {
                side: BattleSide::Player,
            });
            if disobeyed {
                break;
            }
        }
    }
    assert!(
        disobeyed,
        "an overlevel traded Pokemon should sometimes disobey"
    );
    state.obedience_badges[7] = true;
    let mut rng = Random::new(7);
    let mut events = Vec::new();
    assert!(!player_disobeys(&state, &mut rng, &mut events));
}

#[test]
fn disobedience_can_nap_or_make_the_player_pokemon_hit_itself() {
    let mut player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "TACKLE");
    player.original_trainer_id = 2;
    let base = battle_state(
        player,
        pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "TACKLE"),
        0,
    )
    .with_obedience(1, [false; 8]);
    let mut saw_nap = false;
    let mut saw_confusion = false;
    let mut saw_idle = [false; 4];
    for seed in 0..4096 {
        let mut state = base.clone();
        let mut rng = Random::new(seed);
        let mut events = Vec::new();
        apply_player_obedience(
            &mut state,
            0,
            &BTreeMap::from([(
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            )]),
            &BTreeMap::new(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
            &mut events,
        )
        .expect("obedience resolves");
        saw_nap |= state.player.status.as_deref() == Some("SLEEP");
        saw_confusion |= events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::ConfusionSelfDamage { move_name, .. }
                    if move_name == "DISOBEDIENCE"
            )
        });
        for event in &events {
            if let BattleEvent::DisobedienceIdle { roll, .. } = event {
                saw_idle[usize::from(*roll)] = true;
            }
        }
        if saw_nap && saw_confusion && saw_idle.into_iter().all(|seen| seen) {
            break;
        }
    }
    assert!(saw_nap, "overlevel traded Pokemon should sometimes nap");
    assert!(
        saw_confusion,
        "overlevel traded Pokemon should sometimes hit itself"
    );
    assert_eq!(saw_idle, [true; 4]);
}

#[test]
fn obedience_swap_uses_the_complete_random_byte() {
    assert_eq!(swap_nibbles(0x12), 0x21);
    assert_eq!(swap_nibbles(0xa5), 0x5a);
    assert_eq!(swap_nibbles(0xf0), 0x0f);
    assert!(swap_nibbles(0x02) < 50);
    assert!(swap_nibbles(0x05) >= 50);
}

#[test]
fn disobedience_can_select_a_different_move_with_pp() {
    let mut player = pokemon("RATTATA", 50, pokemon_type("NORMAL"), "TACKLE");
    player.original_trainer_id = 2;
    let mut alternate = player.moves[0].clone();
    alternate.name = "GROWL".to_string();
    alternate.current_pp = 40;
    player.moves.push(alternate);
    let state = battle_state(
        player,
        pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "TACKLE"),
        0,
    )
    .with_obedience(1, [false; 8]);

    let mut saw_alternate = false;
    for seed in 0..4096 {
        let mut rng = Random::new(seed);
        if obedience_result(&state, 0, &mut rng) == ObedienceResult::UseMove(1) {
            saw_alternate = true;
            break;
        }
    }
    assert!(
        saw_alternate,
        "overlevel traded Pokemon should sometimes use another move"
    );
}

#[test]
fn committed_charge_turn_skips_the_obedience_command_like_the_asm() {
    let mut player = pokemon("MEGANIUM", 90, pokemon_type("GRASS"), "SOLARBEAM");
    player.original_trainer_id = 2;
    let enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0).with_obedience(1, [false; 8]);
    state.player_charging_move = Some("SOLARBEAM".to_string());
    let seed = (0..4096)
        .find(|seed| {
            let mut rng = Random::new(*seed);
            obedience_result(&state, 0, &mut rng) != ObedienceResult::Obey
        })
        .expect("an overlevel traded Pokemon has a disobedience seed");
    let enemy_hp = state.enemy.hp;
    let moves = BTreeMap::from([
        (
            "SOLARBEAM".to_string(),
            move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 40, 100, "SOLARBEAM"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(seed);

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
    .expect("committed SolarBeam release resolves");

    assert_eq!(outcome.state.player_charging_move, None);
    assert!(outcome.state.enemy.hp < enemy_hp);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Disobeyed { .. }
            | BattleEvent::DisobedienceIdle { .. }
            | BattleEvent::DisobedienceIgnoredSleeping { .. }
    )));
}

#[test]
fn return_and_frustration_power_use_exact_happiness_formula() {
    assert_eq!(return_power(255), 102);
    assert_eq!(return_power(70), 28);
    assert_eq!(return_power(0), 0);
    assert_eq!(frustration_power(0), 102);
    assert_eq!(frustration_power(185), 28);
    assert_eq!(frustration_power(255), 0);

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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "REVERSAL".to_string(),
            move_data_with_effect("REVERSAL", pokemon_type("FIGHTING"), 1, 100, "REVERSAL"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);

    let mut desperate_rng = Random::new(22);
    let desperate_outcome = resolve_battle_turn(
        battle_state(desperate, enemy.clone(), desperate_rng.seed()),
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
        &mut desperate_rng,
    )
    .expect("desperate reversal resolves");

    let mut healthy_rng = Random::new(22);
    let healthy_outcome = resolve_battle_turn(
        battle_state(healthy, enemy, healthy_rng.seed()),
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
fn pursuit_does_not_intercept_a_switch_request_replaced_by_a_move_lock() {
    let player = pokemon("UMBREON", 40, pokemon_type("DARK"), "PURSUIT");
    let enemy = pokemon("MEGANIUM", 120, pokemon_type("GRASS"), "SOLARBEAM");
    let mut state = battle_state(player, enemy, 0);
    state.enemy_charging_move = Some("SOLARBEAM".to_string());
    let moves = BTreeMap::from([
        (
            "PURSUIT".to_string(),
            move_data_with_effect("PURSUIT", pokemon_type("DARK"), 40, 100, "PURSUIT"),
        ),
        (
            "SOLARBEAM".to_string(),
            move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 0, 100, "SOLARBEAM"),
        ),
    ]);
    let mut rng = Random::new(27);

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
    .expect("locked target's move replaces its switch request");

    assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
    assert_eq!(outcome.state.enemy_party_index, 0);
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::PursuitPower { .. }))
    );
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Switched {
            side: BattleSide::Enemy,
            ..
        }
    )));
}

#[test]
fn pursuit_damages_the_departing_target_before_a_surviving_switch_on_both_sides() {
    let moves = BTreeMap::from([(
        "PURSUIT".to_string(),
        move_data_with_effect("PURSUIT", pokemon_type("DARK"), 40, 100, "PURSUIT"),
    )]);
    for (seed, pursuit_side) in [(0x51, BattleSide::Player), (0x52, BattleSide::Enemy)] {
        let pursuit_user = pokemon("UMBREON", 120, pokemon_type("DARK"), "PURSUIT");
        let switching_target = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let (player, enemy, input) = if pursuit_side == BattleSide::Player {
            (
                pursuit_user,
                switching_target,
                BattleTurnInput {
                    player: BattleAction::Move { slot: 0 },
                    enemy: BattleAction::Switch { party_index: 1 },
                },
            )
        } else {
            (
                switching_target,
                pursuit_user,
                BattleTurnInput {
                    player: BattleAction::Switch { party_index: 1 },
                    enemy: BattleAction::Move { slot: 0 },
                },
            )
        };
        let mut rng = Random::new(seed);
        let outcome = resolve_battle_turn(
            battle_state(player, enemy, rng.seed()),
            input,
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("surviving Pursuit switch resolves");
        let target = pursuit_side.other();
        assert_eq!(outcome.order, vec![pursuit_side, target]);
        let pursuit_event = outcome
            .events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    BattleEvent::PursuitPower { side, target: event_target, power: 80, .. }
                        if *side == pursuit_side && *event_target == target
                )
            })
            .expect("doubled Pursuit event");
        let switch_event = outcome
            .events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    BattleEvent::Switched { side, party_index: 1 } if *side == target
                )
            })
            .expect("surviving target switches");
        assert!(pursuit_event < switch_event);
        assert_eq!(
            outcome.state.pokemon(target).species.id,
            if target == BattleSide::Player {
                "BAYLEEF"
            } else {
                "PIDGEOTTO"
            }
        );
    }
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
fn one_member_party_beat_up_skips_the_kings_rock_tail() {
    let mut player = pokemon("UMBREON", 120, pokemon_type("DARK"), "BEAT_UP");
    player.item = Some("KINGS_ROCK".to_string());
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
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 255;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let mut rng = Random::new(28);

    let outcome = resolve_battle_turn_with_items(
        BattleCombatState::new(player.clone(), enemy.clone())
            .with_parties(vec![player], vec![enemy]),
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
    .expect("one-member Beat Up resolves");

    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::BeatUpParticipant {
            side: BattleSide::Player,
            party_index: 0,
            ..
        }
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "BEAT_UP"
    )));
}

#[test]
fn player_beat_up_active_status_selection_compares_party_index_to_hp_low_byte() {
    let mut player = pokemon("UMBREON", 120, pokemon_type("DARK"), "BEAT_UP");
    player.hp = 256;
    player.max_hp = 300;
    player.status = Some("POISON".to_string());
    let mut stale_party_player = player.clone();
    stale_party_player.status = None;
    let enemy = pokemon("SENTRET", 40, pokemon_type("NORMAL"), "SPLASH");
    let mut state =
        BattleCombatState::new(player, enemy).with_parties(vec![stale_party_player], vec![]);

    // wCurBattleMon is 0 and the participant's HP low byte is also 0, so
    // the buggy comparison selects the poisoned live battle status.
    assert!(beat_up_participants(&state, BattleSide::Player).is_empty());

    state.player.hp = 255;
    state.player_party[0].hp = 255;
    // Now the comparison fails and Beat Up reads the stale, healthy party
    // status even though this is still the active battler.
    let participants = beat_up_participants(&state, BattleSide::Player);
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].0, 0);
}

#[test]
fn beat_up_applies_the_users_matching_held_type_parameter_to_every_hit() {
    let mut boosted = pokemon("UMBREON", 120, pokemon_type("DARK"), "BEAT_UP");
    boosted.item = Some("BLACKGLASSES".to_string());
    let mut plain = boosted.clone();
    plain.item = None;
    let ally = pokemon("HOUNDOUR", 90, pokemon_type("DARK"), "TACKLE");
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
    let mut glasses = held_boost_item("BLACKGLASSES", "HELD_DARK_BOOST");
    glasses.parameter = 50;
    let items = BTreeMap::from([("BLACKGLASSES".to_string(), glasses)]);
    let resolve = |player: Pokemon| {
        let mut rng = Random::new(28);
        resolve_battle_turn_with_items(
            battle_state(player.clone(), enemy.clone(), rng.seed())
                .with_parties(vec![player, ally.clone()], vec![enemy.clone()]),
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
        .expect("Beat Up held-type comparison resolves")
    };
    let plain = resolve(plain);
    let boosted = resolve(boosted);
    assert!(boosted.state.enemy.hp < plain.state.enemy.hp);
    assert_eq!(
        boosted
            .events
            .iter()
            .filter(|event| matches!(
                event,
                BattleEvent::HeldItemDamageBoost {
                    side: BattleSide::Player,
                    parameter: 50,
                    ..
                }
            ))
            .count(),
        2
    );
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
    let mut rng = Random::new(31);

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
    player.focus_energy = true;
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
    let mut rng = Random::new(31);
    let mut state = battle_state(player.clone(), enemy.clone(), rng.seed())
        .with_parties(vec![player, ally], vec![enemy]);
    state.player_substitute_hp = 20;
    state.player_disable = Some(BattleDisableState {
        move_name: "TACKLE".to_string(),
        turns_remaining: 3,
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
    assert_eq!(outcome.state.player.confusion_turns, 0);
    assert!(outcome.state.player.focus_energy);
    assert_eq!(outcome.state.player_substitute_hp, 20);
    assert_eq!(outcome.state.player_nightmare_source, None);
    assert_eq!(outcome.state.player_attracted_by, None);
    assert_eq!(outcome.state.enemy_attracted_by, None);
    assert_eq!(outcome.state.player_trap, None);
    assert_eq!(outcome.state.enemy_trap, None);
    assert!(!outcome.state.player.flinching);
    assert_eq!(outcome.state.player_party[1], outcome.state.player);
    assert_eq!(outcome.state.player_disable, None);
    assert!(outcome.events.contains(&BattleEvent::BatonPassed {
        side: BattleSide::Player,
        move_name: "BATON_PASS".to_string(),
        party_index: 1,
        stat_boosts: outcome.state.player.stat_boosts.clone(),
        confusion_turns: 0,
        focus_energy: true,
    }));
    assert!(outcome.events.contains(&BattleEvent::Switched {
        side: BattleSide::Player,
        party_index: 1,
    }));
}

#[test]
fn baton_pass_switch_in_triggers_berserk_gene_from_item_catalog() {
    let mut player = pokemon("EEVEE", 120, pokemon_type("NORMAL"), "BATON_PASS");
    player.confusion_turns = 3;
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
    let mut rng = Random::new(6);
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

    assert_eq!(outcome.state.player.item.as_deref(), Some("BERSERK_GENE"));
    assert_eq!(
        outcome.state.player.stat_boosts.get(&Stat::Attack),
        Some(&0)
    );
    assert_eq!(outcome.state.player.confusion_turns, 2);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemActivated { item_id, .. } if item_id == "BERSERK_GENE"
    )));

    let mut next_turn_state = outcome.state;
    let mut start_events = Vec::new();
    apply_berserk_gene_start_of_turn(
        &mut next_turn_state,
        BattleSide::Player,
        &items,
        &mut start_events,
    )
    .expect("Berserk Gene next-turn boundary resolves");
    assert_eq!(next_turn_state.player.item, None);
    assert_eq!(
        next_turn_state
            .player
            .stat_boosts
            .get(&Stat::Attack)
            .copied(),
        Some(2)
    );
    assert_eq!(next_turn_state.player.confusion_turns, 2);
    assert!(start_events.contains(&BattleEvent::HeldItemActivated {
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "MAGNITUDE".to_string(),
            move_data_with_effect("MAGNITUDE", pokemon_type("GROUND"), 1, 100, "MAGNITUDE"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "HIDDEN_POWER".to_string(),
            move_data_with_effect(
                "HIDDEN_POWER",
                pokemon_type("ELECTRIC"),
                1,
                100,
                "HIDDEN_POWER",
            ),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    assert!(matches!(present_roll(179), PresentRoll::Damage(80)));
    assert!(matches!(present_roll(180), PresentRoll::Damage(120)));
    assert!(matches!(present_roll(204), PresentRoll::Damage(120)));
    assert!(matches!(present_roll(205), PresentRoll::Heal));
    assert!(matches!(present_roll(255), PresentRoll::Heal));
}

#[test]
fn present_damage_branch_selects_power_before_damage() {
    let player = pokemon("DELIBIRD", 90, pokemon_type("ICE"), "PRESENT");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "PRESENT".to_string(),
            move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn link_colosseum_present_uses_the_stab_clobbered_damage_registers() {
    let player = pokemon("DELIBIRD", 90, pokemon_type("NORMAL"), "PRESENT");
    let mut enemy = pokemon("CUBONE", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 200;
    enemy.max_hp = 200;
    let mut state = BattleCombatState::new(player, enemy);
    state.link_battle = true;
    state.link_colosseum = true;
    let present = move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT");
    // Power 40, no critical hit, maximum damage variation.
    let mut rng = ScriptedBattleRandom::new(vec![0, 255, 255]);
    let mut events = Vec::new();

    apply_damage_hit(
        &mut state,
        BattleSide::Player,
        "PRESENT",
        &present,
        1,
        false,
        false,
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &BTreeMap::new(),
        &mut rng,
        &mut events,
    )
    .expect("link Present damage resolves");

    // The first STAB command leaves b=10 (neutral matchup), c=0 (user's
    // second NORMAL type), and e=0 (target's second NORMAL type). DamageCalc
    // clamps c to 1, yielding 18 base damage and 27 after the later STAB.
    assert!(
        events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage: 27,
                ..
            } if move_name == "PRESENT"
        )),
        "events: {events:?}"
    );
}

#[test]
fn present_heal_branch_restores_target_hp_without_damage() {
    let player = pokemon("DELIBIRD", 90, pokemon_type("ICE"), "PRESENT");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 10;
    enemy.max_hp = 44;
    let moves = BTreeMap::from([
        (
            "PRESENT".to_string(),
            move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(15);

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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "PRESENT".to_string(),
            move_data_with_effect("PRESENT", pokemon_type("NORMAL"), 1, 100, "PRESENT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(15);

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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "GUST".to_string(),
            move_data_with_effect("GUST", pokemon_type("FLYING"), 40, 100, "GUST"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
            move_data_with_effect_chance("STOMP", pokemon_type("NORMAL"), 65, 100, "STOMP", 100),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "STRUGGLE".to_string(),
            move_data_with_effect("STRUGGLE", pokemon_type("NORMAL"), 50, 100, "RECOIL_HIT"),
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
fn stomp_doubles_already_varied_damage_against_a_minimized_target() {
    let moves = BTreeMap::from([
        (
            "STOMP".to_string(),
            move_data_with_effect_chance("STOMP", pokemon_type("NORMAL"), 65, 100, "STOMP", 0),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let input = BattleTurnInput {
        player: BattleAction::Move { slot: 0 },
        enemy: BattleAction::Move { slot: 0 },
    };
    let player = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "STOMP");
    let enemy = pokemon("SNORLAX", 90, pokemon_type("NORMAL"), "SPLASH");

    let damage_for = |minimized: bool| {
        let mut rng = Random::new(22);
        let mut state = battle_state(player.clone(), enemy.clone(), rng.seed());
        state.enemy_minimized = minimized;
        resolve_battle_turn(
            state,
            input.clone(),
            &moves,
            &move_priorities(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
        )
        .expect("stomp turn resolves")
        .events
        .into_iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                ..
            } if move_name == "STOMP" => Some(damage),
            _ => None,
        })
        .expect("stomp damage event")
    };

    let ordinary_damage = damage_for(false);
    assert_eq!(damage_for(true), ordinary_damage.saturating_mul(2));
}

#[test]
fn razor_wind_effect_charges_before_damage() {
    let player = pokemon("PIDGEOT", 90, pokemon_type("FLYING"), "RAZOR_WIND");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "RAZOR_WIND".to_string(),
            move_data_with_effect("RAZOR_WIND", pokemon_type("NORMAL"), 80, 100, "RAZOR_WIND"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "SKULL_BASH".to_string(),
            move_data_with_effect("SKULL_BASH", pokemon_type("NORMAL"), 100, 100, "SKULL_BASH"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn sky_attack_release_consumes_its_zero_chance_byte_and_never_flinches() {
    let player = pokemon("PIDGEOT", 90, pokemon_type("FLYING"), "SKY_ATTACK");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let sky_attack = move_data_with_effect_chance(
        "SKY_ATTACK",
        pokemon_type("FLYING"),
        140,
        90,
        "SKY_ATTACK",
        0,
    );
    let moves = BTreeMap::from([
        ("SKY_ATTACK".to_string(), sky_attack.clone()),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(22);
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
    .expect("sky attack charge resolves");
    assert_eq!(
        first.state.player_charging_move,
        Some("SKY_ATTACK".to_string())
    );

    let release_state = first.state;
    let release_accuracy = accuracy_byte_with_weather(
        &sky_attack,
        BattleSide::Player,
        &release_state.player,
        &release_state.enemy,
        &stat_multipliers(),
        release_state.weather,
        false,
    )
    .expect("Sky Attack accuracy");
    let mut expected_rng = rng;
    let accuracy_roll = expected_rng.battle_random_byte();
    assert!(accuracy_roll < release_accuracy, "fixture must hit");
    consume_standard_damage_calculation_rng(&mut expected_rng);
    let effect_chance_roll = expected_rng.battle_random_byte();

    let released = resolve_battle_turn(
        release_state,
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
    assert_eq!(rng.seed(), expected_rng.seed());
    assert!(!released.state.enemy.flinching);
    assert!(!released.events.iter().any(|event| matches!(
        event,
        BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
        } if move_name == "SKY_ATTACK"
    )));
    assert!(
        released
            .events
            .contains(&BattleEvent::SecondaryFlinchMissed {
                side: BattleSide::Player,
                move_name: "SKY_ATTACK".to_string(),
                target: BattleSide::Enemy,
                chance_percent: 0,
                roll: effect_chance_roll,
            })
    );
}

#[test]
fn solarbeam_effect_charges_before_damage_outside_sun() {
    let player = pokemon("BELLSPROUT", 90, pokemon_type("GRASS"), "SOLARBEAM");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "SOLARBEAM".to_string(),
            move_data_with_effect("SOLARBEAM", pokemon_type("GRASS"), 120, 100, "SOLARBEAM"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "RETURN".to_string(),
            move_data_with_effect("RETURN", pokemon_type("NORMAL"), 1, 100, "RETURN"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);

    let mut friendly_rng = Random::new(22);
    let friendly_outcome = resolve_battle_turn(
        battle_state(friendly, enemy.clone(), friendly_rng.seed()),
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
        &mut friendly_rng,
    )
    .expect("friendly return resolves");

    let mut indifferent_rng = Random::new(22);
    let indifferent_outcome = resolve_battle_turn(
        battle_state(indifferent, enemy, indifferent_rng.seed()),
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
        &mut indifferent_rng,
    )
    .expect("indifferent return resolves");

    assert!(friendly_outcome.state.enemy.hp < indifferent_outcome.state.enemy.hp);
    assert_eq!(indifferent_outcome.state.enemy.hp, enemy_hp);
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
    let mut player = pokemon("VOLTORB", 90, pokemon_type("ELECTRIC"), "SONICBOOM");
    player.stat_boosts.remove(&Stat::Attack);
    let mut enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.stat_boosts.remove(&Stat::Defense);
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
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    .expect("static damage turn resolves");

    assert_eq!(outcome.state.enemy.hp, enemy_hp - 20);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Player,
            move_name,
            damage: 20,
            critical: false,
            critical_roll: 0,
            critical_threshold: 0,
            roll: 255,
            ..
        } if move_name == "SONICBOOM"
    )));
}

#[test]
fn level_damage_move_uses_attacker_level_as_damage_amount() {
    let mut player = pokemon("MACHOP", 45, pokemon_type("FIGHTING"), "SEISMIC_TOSS");
    player.level = 37;
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
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
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    .expect("level damage turn resolves");

    assert_eq!(outcome.state.enemy.hp, enemy_hp - 37);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Player,
            move_name,
            damage: 37,
            result,
            ..
        } if move_name == "SEISMIC_TOSS"
            && result.type_multiplier == crate::battle::damage::TypeMultiplier::one()
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
            move_data_with_effect_chance("THIEF", pokemon_type("DARK"), 40, 100, "THIEF", 100),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 60, 100),
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
fn psywave_rejection_samples_nonzero_damage_below_one_and_a_half_levels() {
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
    .expect("psywave turn resolves");

    let damage = enemy_hp - outcome.state.enemy.hp;
    assert!((1..60).contains(&damage));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Player,
            move_name,
            damage: event_damage,
            result,
            ..
        } if move_name == "PSYWAVE"
            && *event_damage == damage
            && result.damage == damage
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
    let count_index = outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::MultiHitCount { .. }))
        .expect("multi-hit total is emitted");
    let last_damage_index = outcome
        .events
        .iter()
        .rposition(|event| matches!(event, BattleEvent::Damage { .. }))
        .expect("multi-hit move dealt damage");
    assert!(count_index > last_damage_index);
    assert!(outcome.state.enemy.hp < enemy_hp);
}

#[test]
fn multi_hit_checks_accuracy_only_before_the_first_strike() {
    let player = pokemon("BEEDRILL", 90, pokemon_type("BUG"), "FURY_ATTACK");
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "FURY_ATTACK".to_string(),
            move_data_with_effect("FURY_ATTACK", pokemon_type("NORMAL"), 15, 50, "MULTI_HIT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    // accuracy, hit-one critical/variation, two-hit count, hit-two
    // critical/variation. The fifth byte would miss if incorrectly sampled as
    // hit-two accuracy.
    let mut rng = ScriptedBattleRandom::new(vec![0, 255, 255, 0, 255, 255]);

    let outcome = resolve_battle_turn(
        battle_state(player, enemy, 0),
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
    .expect("Fury Attack resolves from one accuracy check");

    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, BattleEvent::Damage { move_name, .. } if move_name == "FURY_ATTACK"))
            .count(),
        2
    );
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::Missed { .. }))
    );
}

#[test]
fn twineedle_samples_poison_chance_once_before_its_first_strike() {
    let player = pokemon("BEEDRILL", 90, pokemon_type("BUG"), "TWINEEDLE");
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "TWINEEDLE".to_string(),
            move_data_with_effect_chance(
                "TWINEEDLE",
                pokemon_type("BUG"),
                25,
                100,
                "POISON_MULTI_HIT",
                20,
            ),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut state = battle_state(player, enemy, 0);
    state.enemy_substitute_hp = 1;
    // First-hit critical/variation, then a byte that would make an incorrect
    // post-Substitute second effect-chance roll succeed, then the second-hit
    // critical/variation.
    let mut rng = ScriptedBattleRandom::new(vec![255, 255, 0, 255, 255, 255, 255, 255]);

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
    .expect("Twineedle resolves after breaking Substitute");

    assert_eq!(outcome.state.enemy_substitute_hp, 0);
    assert_eq!(outcome.state.enemy.status, None);
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::StatusApplied { .. }))
    );
}

#[test]
fn hp_berry_waits_until_after_every_multihit_strike() {
    let player = pokemon("DODUO", 90, pokemon_type("NORMAL"), "DOUBLE_KICK");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.max_hp = 100;
    enemy.hp = 49;
    enemy.item = Some("BERRY".to_string());
    let moves = BTreeMap::from([
        (
            "DOUBLE_KICK".to_string(),
            move_data_with_effect(
                "DOUBLE_KICK",
                pokemon_type("FIGHTING"),
                1,
                100,
                "DOUBLE_HIT",
            ),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut berry = held_boost_item("BERRY", "HELD_BERRY");
    berry.parameter = 10;
    let items = BTreeMap::from([("BERRY".to_string(), berry)]);
    let mut rng = Random::new(2);

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
    .expect("multihit berry turn resolves");

    let damage_indices = outcome
        .events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            matches!(
                event,
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    ..
                } if move_name == "DOUBLE_KICK"
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(damage_indices.len(), 2);
    let count_index = outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::MultiHitCount { .. }))
        .expect("multihit count event");
    let heal_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::HeldItemHpHealed {
                    side: BattleSide::Enemy,
                    ..
                }
            )
        })
        .expect("between-turn berry event");
    assert!(damage_indices[1] < count_index);
    assert!(count_index < heal_index);
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
    assert!((2..=5).contains(&damage_events));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MultiHitCount {
            side: BattleSide::Player,
            move_name,
            hits,
            roll: Some(0 | 1 | 4..=7),
        } if move_name == "FURY_ATTACK" && usize::from(*hits) == damage_events
    )));
    assert!(outcome.state.enemy.hp < enemy_hp);
}

#[test]
fn triple_kick_samples_one_to_three_hits_after_the_first_kick() {
    let player = pokemon("HITMONTOP", 90, pokemon_type("FIGHTING"), "TRIPLE_KICK");
    let mut enemy = pokemon("SNORLAX", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 500;
    enemy.max_hp = 500;
    let moves = BTreeMap::from([
        (
            "TRIPLE_KICK".to_string(),
            move_data_with_effect(
                "TRIPLE_KICK",
                pokemon_type("FIGHTING"),
                10,
                100,
                "TRIPLE_KICK",
            ),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    .expect("Triple Kick resolves");

    let kick_damage = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::Damage {
                side: BattleSide::Player,
                move_name,
                damage,
                ..
            } if move_name == "TRIPLE_KICK" => Some(*damage),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!((1..=3).contains(&kick_damage.len()));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MultiHitCount {
            side: BattleSide::Player,
            move_name,
            hits,
            roll: Some(1..=3),
        } if move_name == "TRIPLE_KICK" && usize::from(*hits) == kick_damage.len()
    )));
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
            move_data("TACKLE", pokemon_type("NORMAL"), 60, 100),
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
fn creating_a_substitute_ends_the_users_partial_trap() {
    let player = pokemon("MR_MIME", 90, pokemon_type("PSYCHIC_TYPE"), "SUBSTITUTE");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "SUBSTITUTE".to_string(),
            move_data_with_effect("SUBSTITUTE", pokemon_type("NORMAL"), 0, 100, "SUBSTITUTE"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut state = battle_state(player, enemy, 3);
    state.player_trap = Some(BattleTrapState {
        source: BattleSide::Enemy,
        move_name: "WRAP".to_string(),
        turns_remaining: 3,
    });
    let mut rng = Random::new(3);

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
    .expect("Substitute while partially trapped resolves");

    assert!(outcome.state.player_substitute_hp > 0);
    assert_eq!(outcome.state.player_trap, None);
}

#[test]
fn substitute_blocks_direct_status_without_mutating_the_target() {
    let player = pokemon("PIKACHU", 90, pokemon_type("ELECTRIC"), "THUNDER_WAVE");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "THUNDER_WAVE".to_string(),
            move_data_with_effect("THUNDER_WAVE", pokemon_type("ELECTRIC"), 0, 100, "PARALYZE"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut state = battle_state(player, enemy, 3);
    state.enemy_substitute_hp = 20;
    let mut rng = Random::new(3);

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
    .expect("status move against Substitute resolves");

    assert_eq!(outcome.state.enemy.status, None);
    assert_eq!(outcome.state.enemy_substitute_hp, 20);
    assert!(outcome.events.contains(&BattleEvent::SubstituteBlocked {
        side: BattleSide::Player,
        move_name: "THUNDER_WAVE".to_string(),
        target: BattleSide::Enemy,
    }));
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
        amount: 24,
    }));
}

#[test]
fn pay_day_scatters_money_when_damage_hits_a_substitute() {
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
    let mut state = battle_state(player, enemy, 1);
    state.enemy_substitute_hp = 30;
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
    .expect("Pay Day against Substitute resolves");

    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteDamaged {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "PAY_DAY"
    )));
    assert!(outcome.events.contains(&BattleEvent::PayDayMoney {
        side: BattleSide::Player,
        move_name: "PAY_DAY".to_string(),
        amount: 24,
    }));
}

#[test]
fn ohko_move_faints_target_on_successful_level_checked_hit() {
    let mut player = pokemon("NIDOKING", 90, pokemon_type("NORMAL"), "HORN_DRILL");
    player.level = 40;
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
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
            damage: u16::MAX,
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
fn ohko_x_accuracy_bypasses_accuracy_but_not_fly_dig_gate() {
    let mut player = pokemon("NIDOKING", 90, pokemon_type("NORMAL"), "FISSURE");
    player.level = 40;
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.level = 40;
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "FISSURE".to_string(),
            move_data_with_effect("FISSURE", pokemon_type("GROUND"), 1, 1, "OHKO"),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "FLY".to_string(),
            move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 95, "FLY"),
        ),
    ]);
    let resolve = |airborne: bool| {
        let mut state = battle_state(player.clone(), enemy.clone(), 0);
        state.player_x_accuracy = true;
        if airborne {
            state.enemy_airborne_move = Some("FLY".to_string());
        }
        let mut rng = Random::new(1);
        resolve_battle_turn(
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
        .expect("X Accuracy OHKO resolves")
    };

    let grounded = resolve(false);
    assert_eq!(grounded.state.enemy.hp, 0);
    assert!(
        !grounded
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::OhkoFailed { .. }))
    );

    let flying = resolve(true);
    assert_eq!(flying.state.enemy.hp, enemy_hp);
    assert!(flying.events.iter().any(|event| matches!(
        event,
        BattleEvent::AirborneAvoided {
            side: BattleSide::Player,
            move_name,
            airborne_move,
            ..
        } if move_name == "FISSURE" && airborne_move == "FLY"
    )));
}

#[test]
fn ohko_damage_uses_substitute_endure_and_destiny_bond_tail() {
    let mut player = pokemon("NIDOKING", 90, pokemon_type("NORMAL"), "HORN_DRILL");
    player.level = 40;
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.level = 40;
    let move_data = move_data_with_effect("HORN_DRILL", pokemon_type("NORMAL"), 1, 30, "OHKO");

    let mut substitute = battle_state(player.clone(), enemy.clone(), 0);
    substitute.player_x_accuracy = true;
    substitute.enemy_substitute_hp = 20;
    let mut substitute_events = Vec::new();
    apply_ohko_effect(
        &mut substitute,
        BattleSide::Player,
        "HORN_DRILL",
        &move_data,
        &BTreeMap::new(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &mut Random::new(1),
        &mut substitute_events,
    )
    .expect("OHKO against Substitute resolves");
    assert_eq!(substitute.enemy.hp, enemy.hp);
    assert_eq!(substitute.enemy_substitute_hp, 0);
    assert!(substitute_events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteDamaged {
            side: BattleSide::Player,
            substitute_hp_after: 0,
            ..
        }
    )));

    let mut endured = battle_state(player.clone(), enemy.clone(), 0);
    endured.player_x_accuracy = true;
    endured.enemy_endure_active = true;
    let mut endured_events = Vec::new();
    apply_ohko_effect(
        &mut endured,
        BattleSide::Player,
        "HORN_DRILL",
        &move_data,
        &BTreeMap::new(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &mut Random::new(1),
        &mut endured_events,
    )
    .expect("OHKO against Endure resolves");
    assert_eq!(endured.enemy.hp, 1);
    assert!(endured_events.iter().any(|event| matches!(
        event,
        BattleEvent::EnduredHit {
            target: BattleSide::Enemy,
            held_item: None,
            ..
        }
    )));

    let mut bonded = battle_state(player, enemy, 0);
    bonded.player_x_accuracy = true;
    bonded.enemy_destiny_bond_active = true;
    let mut bonded_events = Vec::new();
    apply_ohko_effect(
        &mut bonded,
        BattleSide::Player,
        "HORN_DRILL",
        &move_data,
        &BTreeMap::new(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &mut Random::new(1),
        &mut bonded_events,
    )
    .expect("OHKO against Destiny Bond resolves");
    assert_eq!(bonded.enemy.hp, 0);
    assert_eq!(bonded.player.hp, 0);
    assert!(bonded_events.iter().any(|event| matches!(
        event,
        BattleEvent::DestinyBondActivated {
            side: BattleSide::Enemy,
            source: BattleSide::Player,
            ..
        }
    )));
}

#[test]
fn direct_status_move_applies_status_after_accuracy_without_damage() {
    let player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "HYPNOSIS");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let mut moves = BTreeMap::new();
    moves.insert(
        "HYPNOSIS".to_string(),
        move_data_with_effect("HYPNOSIS", pokemon_type("PSYCHIC_TYPE"), 0, 0, "SLEEP"),
    );
    moves.insert(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    );
    let mut priorities = move_priorities();
    priorities.effect_priorities.insert("SLEEP".to_string(), 1);
    let mut rng = Random::new(11);
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_recharge_move = Some("HYPER_BEAM".to_string());

    let outcome = resolve_battle_turn(
        state,
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
    assert!((2..=7).contains(&outcome.state.enemy.sleep_turns));
    assert_eq!(
        outcome.state.enemy_recharge_move,
        Some("HYPER_BEAM".to_string())
    );
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
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::RechargeTurn {
            side: BattleSide::Enemy,
            ..
        }
    )));
}

#[test]
fn splash_is_explicit_noop_effect_without_damage() {
    let player = pokemon("MAGIKARP", 50, pokemon_type("WATER"), "SPLASH");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.status = Some("POISON".to_string());
    let mut moves = BTreeMap::new();
    moves.insert(
        "HYPNOSIS".to_string(),
        move_data_with_effect("HYPNOSIS", pokemon_type("PSYCHIC_TYPE"), 0, 0, "SLEEP"),
    );
    moves.insert(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    // Toxic initializes the target's counter to zero. It is incremented only
    // when that poisoned battler later reaches its own residual phase.
    assert_eq!(outcome.state.enemy_toxic_turns, 0);
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
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "EMBER".to_string(),
            move_data_with_effect_chance("EMBER", pokemon_type("FIRE"), 40, 100, "BURN_HIT", 100),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn lethal_poison_sting_and_fire_punch_consume_effect_chance_without_status() {
    for (move_name, move_type, power, effect, effect_chance, status) in [
        ("POISON_STING", "POISON", 15, "POISON_HIT", 30, "POISON"),
        ("FIRE_PUNCH", "FIRE", 75, "BURN_HIT", 10, "BURN"),
    ] {
        let player = pokemon("ATTACKER", 50, pokemon_type(move_type), move_name);
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        enemy.hp = 1;
        let mut state = battle_state(player, enemy, 0);
        let move_data = move_data_with_effect_chance(
            move_name,
            pokemon_type(move_type),
            power,
            100,
            effect,
            effect_chance,
        );
        let mut rng = Random::new(37);
        let mut expected_rng = rng;
        consume_standard_damage_calculation_rng(&mut expected_rng);
        let _effect_chance_roll = expected_rng.battle_random_byte();
        let mut events = Vec::new();

        let result = apply_test_damage_hit(
            &mut state,
            &move_data,
            &BTreeMap::new(),
            &mut rng,
            &mut events,
        );

        assert_eq!(result, DamageHitResult::Stop, "{move_name}");
        assert_eq!(state.enemy.hp, 0, "{move_name}");
        assert_eq!(state.enemy.status, None, "{move_name}");
        assert_eq!(rng.seed(), expected_rng.seed(), "{move_name}");
        assert!(!events.iter().any(|event| matches!(
            event,
            BattleEvent::StatusApplied {
                move_name: applied_move,
                status: applied_status,
                ..
            } if applied_move == move_name && applied_status == status
        )));
    }
}

#[test]
fn damaging_effect_chance_does_not_sample_behind_a_substitute() {
    let player = pokemon("BEEDRILL", 50, pokemon_type("POISON"), "POISON_STING");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    set_substitute_hp(&mut state, BattleSide::Enemy, 1);
    let move_data = move_data_with_effect_chance(
        "POISON_STING",
        pokemon_type("POISON"),
        15,
        100,
        "POISON_HIT",
        30,
    );
    let mut rng = Random::new(37);
    let mut expected_rng = rng;
    consume_standard_damage_calculation_rng(&mut expected_rng);
    let mut events = Vec::new();

    let result = apply_test_damage_hit(
        &mut state,
        &move_data,
        &BTreeMap::new(),
        &mut rng,
        &mut events,
    );

    assert_eq!(result, DamageHitResult::Continue);
    assert_eq!(rng.seed(), expected_rng.seed());
    assert_eq!(substitute_hp(&state, BattleSide::Enemy), 0);
    assert_eq!(state.enemy.status, None);
}

#[test]
fn damaging_effect_chance_precedes_focus_band_rng() {
    const POISON_THRESHOLD: u8 = (30 * 255 / 100) as u8;
    let (seed, expected_effect_roll, expected_focus_roll, expected_seed) = (1..10_000)
        .find_map(|seed| {
            let mut probe = Random::new(seed);
            consume_standard_damage_calculation_rng(&mut probe);
            let effect_roll = probe.battle_random_byte();
            let focus_roll = probe.battle_random_byte();
            (effect_roll < POISON_THRESHOLD
                && focus_roll >= POISON_THRESHOLD
                && focus_roll < u8::MAX)
                .then_some((seed, effect_roll, focus_roll, probe.seed()))
        })
        .expect("fixture with distinguishable effect and Focus Band rolls");
    let player = pokemon("BEEDRILL", 50, pokemon_type("POISON"), "POISON_STING");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 2;
    enemy.item = Some("FOCUS_BAND".to_string());
    let mut state = battle_state(player, enemy, seed);
    let move_data = move_data_with_effect_chance(
        "POISON_STING",
        pokemon_type("POISON"),
        15,
        100,
        "POISON_HIT",
        30,
    );
    let mut focus_band = held_boost_item("FOCUS_BAND", "HELD_FOCUS_BAND");
    focus_band.parameter = 255;
    let items = BTreeMap::from([("FOCUS_BAND".to_string(), focus_band)]);
    let mut rng = Random::new(seed);
    let mut events = Vec::new();

    let result = apply_test_damage_hit(&mut state, &move_data, &items, &mut rng, &mut events);

    assert_eq!(result, DamageHitResult::Continue);
    assert_eq!(rng.seed(), expected_seed);
    assert_eq!(state.enemy.hp, 1);
    assert_eq!(state.enemy.status.as_deref(), Some("POISON"));
    assert!(events.contains(&BattleEvent::StatusApplied {
        side: BattleSide::Player,
        move_name: "POISON_STING".to_string(),
        target: BattleSide::Enemy,
        status: "POISON".to_string(),
    }));
    assert!(expected_effect_roll < POISON_THRESHOLD);
    assert!(expected_focus_roll >= POISON_THRESHOLD);
}

#[test]
fn successful_burn_effect_defrosts_an_already_frozen_target() {
    let player = pokemon("CYNDAQUIL", 50, pokemon_type("FIRE"), "EMBER");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.status = Some("FREEZE".to_string());
    let mut state = battle_state(player, enemy, 1);
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_secondary_status_after_success(
        &mut state,
        BattleSide::Player,
        "EMBER",
        BattleSide::Enemy,
        "BURN",
        &mut rng,
        &mut events,
    );

    assert_eq!(state.enemy.status, None);
    assert!(events.contains(&BattleEvent::StatusHealed {
        side: BattleSide::Player,
        move_name: "EMBER".to_string(),
        target: BattleSide::Enemy,
        status_before: "FREEZE".to_string(),
    }));
}

#[test]
fn successful_freeze_effect_is_suppressed_by_sunlight() {
    let player = pokemon("JYNX", 50, pokemon_type("ICE"), "ICE_PUNCH");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 1);
    state.weather = Weather::Sun;
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_secondary_status_after_success(
        &mut state,
        BattleSide::Player,
        "ICE_PUNCH",
        BattleSide::Enemy,
        "FREEZE",
        &mut rng,
        &mut events,
    );

    assert_eq!(state.enemy.status, None);
    assert!(events.is_empty());
}

#[test]
fn damaging_secondary_status_is_silent_for_an_already_statused_target() {
    let player = pokemon("MAGMAR", 50, pokemon_type("FIRE"), "FIRE_PUNCH");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.status = Some("PARALYSIS".to_string());
    let mut state = battle_state(player, enemy, 1);
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_secondary_status_after_success(
        &mut state,
        BattleSide::Player,
        "FIRE_PUNCH",
        BattleSide::Enemy,
        "BURN",
        &mut rng,
        &mut events,
    );

    assert_eq!(state.enemy.status.as_deref(), Some("PARALYSIS"));
    assert!(events.is_empty());
}

#[test]
fn damaging_secondary_status_is_silent_for_an_immune_target_before_safeguard() {
    let player = pokemon("MAGMAR", 50, pokemon_type("FIRE"), "FIRE_PUNCH");
    let enemy = pokemon("VULPIX", 40, pokemon_type("FIRE"), "TACKLE");
    let mut state = battle_state(player, enemy, 1);
    state.enemy_safeguard_turns = 3;
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_secondary_status_after_success(
        &mut state,
        BattleSide::Player,
        "FIRE_PUNCH",
        BattleSide::Enemy,
        "BURN",
        &mut rng,
        &mut events,
    );

    assert_eq!(state.enemy.status, None);
    assert!(events.is_empty());
}

#[test]
fn twineedle_can_poison_a_steel_type_in_generation_two() {
    let player = pokemon("BEEDRILL", 50, pokemon_type("BUG"), "TWINEEDLE");
    let enemy = pokemon("MAGNEMITE", 40, pokemon_type("STEEL"), "TACKLE");
    let mut state = battle_state(player, enemy, 1);
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_secondary_status_after_success(
        &mut state,
        BattleSide::Player,
        "TWINEEDLE",
        BattleSide::Enemy,
        "POISON",
        &mut rng,
        &mut events,
    );

    assert_eq!(state.enemy.status.as_deref(), Some("POISON"));
    assert!(events.contains(&BattleEvent::StatusApplied {
        side: BattleSide::Player,
        move_name: "TWINEEDLE".to_string(),
        target: BattleSide::Enemy,
        status: "POISON".to_string(),
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
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SecondaryStatusMissed {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
            status,
            chance_percent: 1,
            roll,
        } if move_name == "POISON_STING" && status == "POISON" && *roll >= 2
    )));
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
    // Confuse Ray installs 2-5 turns, then the slower target immediately
    // consumes one counter on its action in this same resolved turn.
    assert!((1..=4).contains(&outcome.state.enemy.confusion_turns));
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
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.confusion_turns = 3;
    let moves = BTreeMap::from([
        (
            "CONFUSE_RAY".to_string(),
            move_data_with_effect("CONFUSE_RAY", pokemon_type("GHOST"), 0, 100, "CONFUSE"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn damaging_secondary_confusion_is_silent_when_confusion_already_exists() {
    let player = pokemon("PSYDUCK", 50, pokemon_type("WATER"), "CONFUSION");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.confusion_turns = 3;
    let mut state = battle_state(player, enemy, 1);
    let mut rng = Random::new(1);
    let mut events = Vec::new();

    apply_secondary_confusion_effect(
        &mut state,
        BattleSide::Player,
        "CONFUSION",
        EffectChanceResult {
            chance_percent: 100,
            succeeds: true,
            roll: Some(0),
        },
        &mut rng,
        &mut events,
    );

    assert_eq!(state.enemy.confusion_turns, 3);
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::ConfusionFailed { .. } | BattleEvent::ConfusionApplied { .. }
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
fn confusion_self_damage_uses_the_selected_moves_type_boost_item_parameter() {
    let mut boosted = pokemon("PSYDUCK", 50, pokemon_type("WATER"), "WATER_GUN");
    boosted.confusion_turns = 3;
    boosted.item = Some("MYSTIC_WATER".to_string());
    let mut plain = boosted.clone();
    plain.item = None;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
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
    let mut mystic_water = held_boost_item("MYSTIC_WATER", "HELD_WATER_BOOST");
    mystic_water.parameter = 50;
    let items = BTreeMap::from([("MYSTIC_WATER".to_string(), mystic_water)]);
    let resolve = |player: Pokemon| {
        let mut rng = Random::new(1);
        resolve_battle_turn_with_items(
            battle_state(player, enemy.clone(), rng.seed()),
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
        .expect("confusion held-type comparison resolves")
    };
    let plain = resolve(plain);
    let boosted = resolve(boosted);
    assert!(boosted.state.player.hp < plain.state.player.hp);
    assert!(boosted.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemDamageBoost {
            side: BattleSide::Player,
            item_id,
            move_type,
            parameter: 50,
            ..
        } if item_id == "MYSTIC_WATER" && move_type == "WATER"
    )));
}

#[test]
fn confusion_self_damage_does_not_apply_thick_club_stat_boost() {
    let mut club = pokemon("CUBONE", 50, pokemon_type("GROUND"), "TACKLE");
    club.confusion_turns = 3;
    club.item = Some("THICK_CLUB".to_string());
    club.attack = 300;
    club.hp = 500;
    club.max_hp = 500;
    let mut plain = club.clone();
    plain.item = None;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
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
    let items = BTreeMap::from([(
        "THICK_CLUB".to_string(),
        held_boost_item("THICK_CLUB", "HELD_NONE"),
    )]);
    let resolve = |player: Pokemon| {
        let mut rng = Random::new(1);
        resolve_battle_turn_with_items(
            battle_state(player, enemy.clone(), rng.seed()),
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
        .expect("confusion Thick Club comparison resolves")
    };
    let plain = resolve(plain);
    let club = resolve(club);
    let self_damage = |outcome: &BattleTurnOutcome| {
        outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::ConfusionSelfDamage { damage, .. } => Some(*damage),
                _ => None,
            })
            .expect("confusion self damage")
    };
    assert_eq!(
        self_damage(&club),
        self_damage(&plain),
        "club={} plain={}",
        self_damage(&club),
        self_damage(&plain)
    );
}

#[test]
fn confusion_retains_selfdestructs_defense_halving_bug() {
    let mut normal_player = pokemon("GEODUDE", 50, pokemon_type("ROCK"), "TACKLE");
    normal_player.confusion_turns = 3;
    let mut exploding_player = normal_player.clone();
    exploding_player.moves[0].name = "EXPLOSION".to_string();
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "EXPLOSION".to_string(),
            move_data_with_effect(
                "EXPLOSION",
                pokemon_type("NORMAL"),
                250,
                100,
                "SELFDESTRUCT",
            ),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let resolve = |player: Pokemon| {
        let mut rng = Random::new(1);
        resolve_battle_turn(
            battle_state(player, enemy.clone(), rng.seed()),
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
        .expect("confusion self-damage comparison resolves")
    };
    let normal = resolve(normal_player);
    let explosion = resolve(exploding_player);
    let self_damage = |outcome: &BattleTurnOutcome| {
        outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::ConfusionSelfDamage { damage, .. } => Some(*damage),
                _ => None,
            })
            .expect("confusion self-damage event")
    };

    assert!(self_damage(&explosion) > self_damage(&normal));
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "SOFTBOILED".to_string(),
            move_data_with_effect("SOFTBOILED", pokemon_type("NORMAL"), 0, 100, "HEAL"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
        animation_param: 2,
    }));
}

#[test]
fn direct_heal_move_caps_at_max_hp_and_reports_exact_amount() {
    let mut player = pokemon("MILTANK", 50, pokemon_type("NORMAL"), "MILK_DRINK");
    player.hp = player.max_hp - 3;
    let hp_before = player.hp;
    let max_hp = player.max_hp;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "MILK_DRINK".to_string(),
            move_data_with_effect("MILK_DRINK", pokemon_type("NORMAL"), 0, 100, "HEAL"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
        animation_param: 2,
    }));
}

#[test]
fn rest_fully_heals_and_sets_exact_sleep_turns() {
    let mut player = pokemon("SNORLAX", 30, pokemon_type("NORMAL"), "REST");
    player.hp = player.max_hp / 4;
    player.status = Some("BAD_POISON".to_string());
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

    let mut state = battle_state(player, enemy, rng.seed());
    state.player_toxic_turns = 4;
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
    .expect("rest turn resolves");

    assert_eq!(outcome.state.player.hp, max_hp);
    assert_eq!(outcome.state.player.status.as_deref(), Some("SLEEP"));
    // BattleCommand_Heal writes REST_SLEEP_TURNS + 1 because the sleep
    // gate decrements before deciding whether the user can act.
    assert_eq!(outcome.state.player.sleep_turns, 3);
    assert_eq!(outcome.state.player_toxic_turns, 0);
    assert!(outcome.events.contains(&BattleEvent::HealApplied {
        side: BattleSide::Player,
        move_name: "REST".to_string(),
        hp_before,
        hp_after: max_hp,
        amount: max_hp - hp_before,
        animation_param: 0,
    }));
    assert!(outcome.events.contains(&BattleEvent::StatusApplied {
        side: BattleSide::Player,
        move_name: "REST".to_string(),
        target: BattleSide::Player,
        status: "SLEEP".to_string(),
    }));
}

#[test]
fn rest_does_not_clear_an_existing_nightmare() {
    let mut player = pokemon("SNORLAX", 30, pokemon_type("NORMAL"), "REST");
    player.hp = player.max_hp / 2;
    let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "REST".to_string(),
            move_data_with_effect("REST", pokemon_type("PSYCHIC_TYPE"), 0, 100, "HEAL"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(1);
    let mut state = battle_state(player, enemy, rng.seed());
    state.player_nightmare_source = Some(BattleSide::Enemy);

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
    .expect("Rest under Nightmare resolves");

    assert_eq!(
        outcome.state.player_nightmare_source,
        Some(BattleSide::Enemy)
    );
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::NightmareDamage {
            side: BattleSide::Player,
            source: BattleSide::Enemy,
            ..
        }
    )));
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
fn heal_bell_clears_active_status_and_chimes_without_curing_confusion() {
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
    let mut state = battle_state(player, enemy, 6);
    state.player_toxic_turns = 3;
    state.player.confusion_turns = 2;
    let mut rng = Random::new(6);

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
    assert_eq!(outcome.state.player_toxic_turns, 3);
    assert_eq!(outcome.state.player.confusion_turns, 1);
    assert!(outcome.events.contains(&BattleEvent::HealBellChimed {
        side: BattleSide::Player,
        active_status_before: Some("BAD_POISON".to_string()),
    }));
}

#[test]
fn heal_bell_chimes_when_the_active_pokemon_is_already_healthy() {
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
    assert!(outcome.events.contains(&BattleEvent::HealBellChimed {
        side: BattleSide::Player,
        active_status_before: None,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusHealFailed { move_name, .. } if move_name == "HEAL_BELL"
    )));
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
fn time_based_heal_moves_use_crystal_time_weather_and_link_multipliers() {
    let synthesis = move_data_with_effect("SYNTHESIS", pokemon_type("GRASS"), 0, 100, "SYNTHESIS");
    let morning_sun =
        move_data_with_effect("MORNING_SUN", pokemon_type("NORMAL"), 0, 100, "MORNING_SUN");
    let moonlight = move_data_with_effect("MOONLIGHT", pokemon_type("NORMAL"), 0, 100, "MOONLIGHT");
    let recover = move_data_with_effect("RECOVER", pokemon_type("NORMAL"), 0, 100, "HEAL");

    let synthesis_day = time_based_heal_param(&synthesis, TimeOfDay::Day, Weather::None, false);
    let synthesis_morning =
        time_based_heal_param(&synthesis, TimeOfDay::Morning, Weather::None, false);
    let synthesis_sun = time_based_heal_param(&synthesis, TimeOfDay::Day, Weather::Sun, false);
    let morning_rain =
        time_based_heal_param(&morning_sun, TimeOfDay::Morning, Weather::Rain, false);
    let moonlight_mismatched_sand =
        time_based_heal_param(&moonlight, TimeOfDay::Day, Weather::Sandstorm, false);
    let linked_moonlight = time_based_heal_param(&moonlight, TimeOfDay::Day, Weather::None, true);

    assert_eq!(synthesis_day, 2);
    assert_eq!(direct_heal_amount(96, &synthesis, synthesis_day), 48);
    assert_eq!(synthesis_morning, 1);
    assert_eq!(direct_heal_amount(96, &synthesis, synthesis_morning), 24);
    assert_eq!(synthesis_sun, 3);
    assert_eq!(direct_heal_amount(96, &synthesis, synthesis_sun), 96);
    assert_eq!(morning_rain, 1);
    assert_eq!(direct_heal_amount(96, &morning_sun, morning_rain), 24);
    assert_eq!(moonlight_mismatched_sand, 0);
    assert_eq!(
        direct_heal_amount(96, &moonlight, moonlight_mismatched_sand),
        12
    );
    assert_eq!(linked_moonlight, 2);
    assert_eq!(direct_heal_amount(96, &moonlight, linked_moonlight), 48);
    assert_eq!(direct_heal_amount(96, &recover, 3), 48);
}

#[test]
fn direct_heal_move_fails_without_inferred_overheal() {
    let player = pokemon("STARYU", 50, pokemon_type("WATER"), "RECOVER");
    let hp = player.hp;
    let max_hp = player.max_hp;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "RECOVER".to_string(),
            move_data_with_effect("RECOVER", pokemon_type("NORMAL"), 0, 100, "HEAL"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    player.max_hp = 100;
    let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 80;
    enemy.max_hp = 100;
    let moves = BTreeMap::from([
        (
            "PAIN_SPLIT".to_string(),
            move_data_with_effect("PAIN_SPLIT", pokemon_type("NORMAL"), 0, 100, "PAIN_SPLIT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 90;
    enemy.max_hp = 60;
    let moves = BTreeMap::from([
        (
            "PAIN_SPLIT".to_string(),
            move_data_with_effect("PAIN_SPLIT", pokemon_type("NORMAL"), 0, 100, "PAIN_SPLIT"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.perish_song_turns = 1;
    let moves = BTreeMap::from([
        (
            "PERISH_SONG".to_string(),
            move_data_with_effect("PERISH_SONG", pokemon_type("NORMAL"), 0, 100, "PERISH_SONG"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn perish_song_applies_to_the_unaffected_battler_without_reporting_failure() {
    let mut player = pokemon("MISDREAVUS", 50, pokemon_type("GHOST"), "PERISH_SONG");
    player.perish_song_turns = 2;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "PERISH_SONG".to_string(),
            move_data_with_effect("PERISH_SONG", pokemon_type("NORMAL"), 0, 100, "PERISH_SONG"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    .expect("partially active perish song turn resolves");

    assert_eq!(outcome.state.player.perish_song_turns, 1);
    assert_eq!(outcome.state.enemy.perish_song_turns, 3);
    assert!(outcome.events.contains(&BattleEvent::PerishSongApplied {
        side: BattleSide::Player,
        move_name: "PERISH_SONG".to_string(),
        target: BattleSide::Enemy,
        turns: 4,
    }));
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::PerishSongFailed { .. }))
    );
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
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
fn belly_drum_at_half_hp_sharply_boosts_then_fails() {
    let mut player = pokemon("POLIWRATH", 50, pokemon_type("WATER"), "BELLY_DRUM");
    player.hp = player.max_hp / 2;
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
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 64;
    enemy.max_hp = 64;
    let moves = BTreeMap::from([
        (
            "CURSE".to_string(),
            move_data_with_effect("CURSE", pokemon_type("UNKNOWN_T"), 0, 100, "CURSE"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
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
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn future_sight_queues_stored_damage_on_attacking_side() {
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
        .player_future_sight
        .as_ref()
        .expect("future sight queued on player side");
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
fn future_sight_stores_damage_with_the_matching_held_type_parameter() {
    let mut boosted = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "FUTURE_SIGHT");
    boosted.item = Some("TWISTEDSPOON".to_string());
    let mut plain = boosted.clone();
    plain.item = None;
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
    let mut spoon = held_boost_item("TWISTEDSPOON", "HELD_PSYCHIC_BOOST");
    spoon.parameter = 50;
    let items = BTreeMap::from([("TWISTEDSPOON".to_string(), spoon)]);
    let resolve = |player: Pokemon| {
        let mut rng = Random::new(128);
        resolve_battle_turn_with_items(
            battle_state(player, enemy.clone(), rng.seed()),
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
        .expect("Future Sight held-type comparison resolves")
    };
    let plain = resolve(plain);
    let boosted = resolve(boosted);
    assert!(
        boosted.state.player_future_sight.as_ref().unwrap().damage
            > plain.state.player_future_sight.as_ref().unwrap().damage
    );
    assert!(boosted.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemDamageBoost {
            side: BattleSide::Player,
            parameter: 50,
            ..
        }
    )));
}

#[test]
fn future_sight_fails_when_attacking_side_already_has_queued_attack() {
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
    state.player_future_sight = Some(BattleFutureSightState {
        source: BattleSide::Player,
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
        outcome.state.player_future_sight,
        Some(BattleFutureSightState {
            source: BattleSide::Player,
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
    state.player_future_sight = Some(BattleFutureSightState {
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

    assert_eq!(outcome.state.player_future_sight, None);
    assert!(outcome.events.contains(&BattleEvent::FutureSightLanded {
        side: BattleSide::Enemy,
        source: BattleSide::Player,
        move_name: "FUTURE_SIGHT".to_string(),
    }));
    let (damage, hp_after) = outcome
        .events
        .iter()
        .find_map(|event| match event {
            BattleEvent::FutureSightDamage {
                side: BattleSide::Enemy,
                source: BattleSide::Player,
                move_name,
                damage,
                hp_before: 20,
                hp_after,
            } if move_name == "FUTURE_SIGHT" => Some((*damage, *hp_after)),
            _ => None,
        })
        .expect("Future Sight landing damage event");
    assert!((14..=17).contains(&damage));
    assert_eq!(hp_after, 20 - damage);
    assert_eq!(outcome.state.enemy.hp, hp_after);
}

#[test]
fn future_sight_countdowns_are_owned_and_resolved_by_the_attacking_side() {
    let player = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "SPLASH");
    let enemy = pokemon("NATU", 60, pokemon_type("PSYCHIC_TYPE"), "SPLASH");
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut rng = Random::new(130);
    let mut state = battle_state(player, enemy, rng.seed());
    state.player_future_sight = Some(BattleFutureSightState {
        source: BattleSide::Player,
        move_name: "FUTURE_SIGHT".to_string(),
        turns_remaining: 1,
        damage: 1,
    });
    state.enemy_future_sight = Some(BattleFutureSightState {
        source: BattleSide::Enemy,
        move_name: "FUTURE_SIGHT".to_string(),
        turns_remaining: 1,
        damage: 1,
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
    .expect("both future sight attacks resolve");

    let landed: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::FutureSightLanded { side, source, .. } => Some((*side, *source)),
            _ => None,
        })
        .collect();
    assert_eq!(
        landed,
        vec![
            (BattleSide::Enemy, BattleSide::Player),
            (BattleSide::Player, BattleSide::Enemy),
        ]
    );
}

#[test]
fn future_sight_does_not_trigger_hp_berry_before_later_sandstorm_ko() {
    let player = pokemon("GEODUDE", 70, pokemon_type("ROCK"), "SPLASH");
    let mut enemy = pokemon("RATTATA", 60, pokemon_type("NORMAL"), "SPLASH");
    enemy.max_hp = 100;
    enemy.hp = 20;
    enemy.item = Some("BERRY".to_string());
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut berry = held_boost_item("BERRY", "HELD_BERRY");
    berry.parameter = 10;
    let items = BTreeMap::from([("BERRY".to_string(), berry)]);
    let mut rng = Random::new(133);
    let mut state = battle_state(player, enemy, rng.seed());
    state.player_future_sight = Some(BattleFutureSightState {
        source: BattleSide::Player,
        move_name: "FUTURE_SIGHT".to_string(),
        turns_remaining: 1,
        damage: 10,
    });
    state.weather = Weather::Sandstorm;
    state.weather_turns = 2;

    let outcome = resolve_battle_turn_with_items(
        state,
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
    .expect("Future Sight then sandstorm turn resolves");

    assert_eq!(outcome.state.enemy.hp, 0);
    assert_eq!(outcome.state.enemy.item.as_deref(), Some("BERRY"));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::FutureSightDamage {
            side: BattleSide::Enemy,
            hp_after: 10..=12,
            ..
        }
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SandstormDamage {
            side: BattleSide::Enemy,
            hp_after: 0,
            ..
        }
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemHpHealed {
            side: BattleSide::Enemy,
            ..
        }
    )));
}

#[test]
fn future_sight_landing_damages_substitute_before_real_hp() {
    let player = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "SPLASH");
    let mut enemy = pokemon("RATTATA", 60, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 20;
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut rng = Random::new(131);
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_substitute_hp = 20;
    state.player_future_sight = Some(BattleFutureSightState {
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
    .expect("future sight Substitute turn resolves");

    assert_eq!(outcome.state.player_future_sight, None);
    assert_eq!(outcome.state.enemy.hp, 20);
    assert!(outcome.events.contains(&BattleEvent::FutureSightLanded {
        side: BattleSide::Enemy,
        source: BattleSide::Player,
        move_name: "FUTURE_SIGHT".to_string(),
    }));
    let (damage, substitute_hp_after) = outcome
        .events
        .iter()
        .find_map(|event| match event {
            BattleEvent::SubstituteDamaged {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                damage,
                substitute_hp_before: 20,
                substitute_hp_after,
            } if move_name == "FUTURE_SIGHT" => Some((*damage, *substitute_hp_after)),
            _ => None,
        })
        .expect("Future Sight Substitute damage event");
    assert!((14..=17).contains(&damage));
    assert_eq!(substitute_hp_after, 20 - damage);
    assert_eq!(outcome.state.enemy_substitute_hp, substitute_hp_after);
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::FutureSightDamage { .. }))
    );
}

#[test]
fn future_sight_lands_after_the_turns_endure_state_has_cleared() {
    let player = pokemon("XATU", 70, pokemon_type("PSYCHIC_TYPE"), "SPLASH");
    let mut enemy = pokemon("RATTATA", 60, pokemon_type("NORMAL"), "SPLASH");
    enemy.hp = 10;
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut rng = Random::new(132);
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_endure_active = true;
    state.player_future_sight = Some(BattleFutureSightState {
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
    .expect("Future Sight after Endure turn resolves");

    assert!(!outcome.state.enemy_endure_active);
    assert_eq!(outcome.state.enemy.hp, 0);
    assert!(outcome.events.contains(&BattleEvent::Fainted {
        side: BattleSide::Enemy,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::EnduredHit {
            target: BattleSide::Enemy,
            ..
        }
    )));
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
    let escape_trap = BattleEscapeTrapState {
        source: BattleSide::Enemy,
        move_name: "MEAN_LOOK".to_string(),
    };
    state.player_escape_trap = Some(escape_trap.clone());
    state.player_trap = Some(BattleTrapState {
        source: BattleSide::Enemy,
        move_name: "BIND".to_string(),
        turns_remaining: 3,
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
    .expect("rapid spin turn resolves");

    assert_eq!(outcome.state.player_trap, None);
    assert_eq!(outcome.state.player_leech_seed_source, None);
    assert!(!outcome.state.player_spikes);
    assert_eq!(outcome.state.player_escape_trap, Some(escape_trap));
    assert!(outcome.events.contains(&BattleEvent::RapidSpinCleared {
        side: BattleSide::Player,
        move_name: "RAPID_SPIN".to_string(),
        cleared_trap: true,
        trap_move: Some("BIND".to_string()),
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
    .expect("no-effect rapid spin turn resolves");

    assert_eq!(
        outcome.state.player_trap,
        Some(BattleTrapState {
            turns_remaining: 2,
            ..trap
        })
    );
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
fn rapid_spin_clears_hazards_before_a_target_knockout_ends_the_script() {
    let player = pokemon("STARYU", 50, pokemon_type("WATER"), "RAPID_SPIN");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.hp = 1;
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
    .expect("Rapid Spin knockout resolves");

    assert_eq!(outcome.state.enemy.hp, 0);
    assert_eq!(outcome.state.player_trap, None);
    assert_eq!(outcome.state.player_leech_seed_source, None);
    assert!(!outcome.state.player_spikes);
    let clear_index = outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::RapidSpinCleared { .. }))
        .expect("Rapid Spin clear event");
    let faint_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Fainted {
                    side: BattleSide::Enemy
                }
            )
        })
        .expect("target faint event");
    assert!(clear_index < faint_index);
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
fn counter_uses_hidden_powers_base_normal_type_for_its_category() {
    let player = pokemon("MACHOP", 50, pokemon_type("FIGHTING"), "COUNTER");
    let mut enemy = pokemon("UNOWN", 80, pokemon_type("PSYCHIC_TYPE"), "HIDDEN_POWER");
    // Attack low bits 2 and Defense low bits 0 resolve Hidden Power to Fire,
    // a special type. Counter nevertheless reloads HIDDEN_POWER's base move
    // record (NORMAL) when it decides which reflection command may succeed.
    enemy.dvs = Dv::from_non_hp(10, 8, 10, 10);
    let moves = BTreeMap::from([
        (
            "COUNTER".to_string(),
            move_data_with_effect("COUNTER", pokemon_type("FIGHTING"), 1, 100, "COUNTER"),
        ),
        (
            "HIDDEN_POWER".to_string(),
            move_data_with_effect(
                "HIDDEN_POWER",
                pokemon_type("NORMAL"),
                1,
                100,
                "HIDDEN_POWER",
            ),
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
    .expect("Counter against special-type Hidden Power resolves");

    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HiddenPowerResolved {
            side: BattleSide::Enemy,
            move_name,
            move_type,
            ..
        } if move_name == "HIDDEN_POWER" && move_type == "FIRE"
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::CounterDamage {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
            countered_move,
            category: BattleDamageCategory::Physical,
            ..
        } if move_name == "COUNTER" && countered_move == "HIDDEN_POWER"
    )));
}

#[test]
fn counter_kings_rock_observes_the_substitute_after_reflected_damage_breaks_it() {
    let mut player = pokemon("MACHOP", 50, pokemon_type("FIGHTING"), "COUNTER");
    player.item = Some("KINGS_ROCK".to_string());
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
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 255;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let mut state = battle_state(player, enemy, 20);
    state.enemy_substitute_hp = 1;
    let mut rng = Random::new(20);

    let outcome = resolve_battle_turn_with_items(
        state,
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
    .expect("Counter through a broken Substitute resolves");

    assert!(outcome.events.contains(&BattleEvent::SubstituteBroken {
        side: BattleSide::Player,
        move_name: "COUNTER".to_string(),
        target: BattleSide::Enemy,
    }));
    assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
        side: BattleSide::Player,
        move_name: "COUNTER".to_string(),
        target: BattleSide::Enemy,
    }));
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
            move_data_with_effect_chance(
                "ANCIENTPOWER",
                pokemon_type("NORMAL"),
                60,
                100,
                "ALL_UP_HIT",
                u8::MAX,
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
fn all_up_hit_does_not_raise_stats_after_knocking_out_target() {
    let player = pokemon("DUNSPARCE", 50, pokemon_type("NORMAL"), "ANCIENTPOWER");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.hp = 1;
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
    .expect("knockout all-up hit resolves");

    assert_eq!(outcome.state.enemy.hp, 0);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "ANCIENTPOWER"
    )));
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
fn selfdestruct_effect_faints_user_even_when_target_is_immune() {
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
fn damaging_stat_stage_move_is_silent_at_the_stage_cap() {
    let player = pokemon("DIGLETT", 50, pokemon_type("NORMAL"), "MUD_SLAP");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.stat_boosts.insert(Stat::Accuracy, -6);
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
    .expect("capped damaging stat move resolves");

    assert!(outcome.state.enemy.hp < enemy_hp);
    assert_eq!(
        outcome.state.enemy.stat_boosts.get(&Stat::Accuracy),
        Some(&-6)
    );
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageFailed { move_name, .. } if move_name == "MUD_SLAP"
    )));
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

    assert!(outcome.state.player_mist_active);
    assert!(outcome.events.contains(&BattleEvent::MistApplied {
        side: BattleSide::Player,
        move_name: "MIST".to_string(),
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
    state.player_mist_active = true;

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
    }));
    assert!(outcome.state.player_mist_active);
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
    state.player_mist_active = true;

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

    assert!(outcome.state.player_mist_active);
    assert!(outcome.events.contains(&BattleEvent::MistFailed {
        side: BattleSide::Player,
        move_name: "MIST".to_string(),
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
fn safeguard_blocks_direct_sleep_and_burn() {
    for (move_name, effect) in [("SLEEP_POWDER", "SLEEP"), ("WILL_O_WISP", "BURN")] {
        let player = pokemon("MEGANIUM", 90, pokemon_type("GRASS"), "SAFEGUARD");
        let enemy = pokemon("GASTLY", 40, pokemon_type("GHOST"), move_name);
        let moves = BTreeMap::from([
            (
                "SAFEGUARD".to_string(),
                move_data_with_effect("SAFEGUARD", pokemon_type("NORMAL"), 0, 100, "SAFEGUARD"),
            ),
            (
                move_name.to_string(),
                move_data_with_effect(move_name, pokemon_type("NORMAL"), 0, 100, effect),
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
        .expect("Safeguard-protected status turn resolves");

        assert_eq!(outcome.state.player.status, None, "{effect}");
        assert!(outcome.events.contains(&BattleEvent::SafeguardProtected {
            side: BattleSide::Enemy,
            move_name: move_name.to_string(),
            target: BattleSide::Player,
            effect: effect.to_string(),
            turns_remaining: 5,
        }));
    }
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
fn reflect_reduces_physical_damage_inside_the_damage_formula() {
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
    let state_without_screen = battle_state(player, enemy, 22);
    let mut state = state_without_screen.clone();
    state.enemy_reflect_turns = 3;
    let mut plain_rng = Random::new(22);
    let plain = resolve_battle_turn(
        state_without_screen,
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
        &mut plain_rng,
    )
    .expect("plain physical damage resolves");
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
    .expect("reflect damage reduction resolves");

    let damage = |outcome: &BattleTurnOutcome| {
        outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "TACKLE" => Some(*damage),
                _ => None,
            })
            .expect("player damage event")
    };
    assert!(damage(&outcome) < damage(&plain));
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
fn light_screen_reduces_special_damage_inside_the_damage_formula() {
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
    let state_without_screen = battle_state(player, enemy, 22);
    let mut state = state_without_screen.clone();
    state.enemy_light_screen_turns = 3;
    let mut plain_rng = Random::new(22);
    let plain = resolve_battle_turn(
        state_without_screen,
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
        &mut plain_rng,
    )
    .expect("plain special damage resolves");
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
    .expect("light screen damage reduction resolves");

    let damage = |outcome: &BattleTurnOutcome| {
        outcome
            .events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    move_name,
                    damage,
                    ..
                } if move_name == "WATER_GUN" => Some(*damage),
                _ => None,
            })
            .expect("player damage event")
    };
    assert!(damage(&outcome) < damage(&plain));
}

#[test]
fn destiny_bond_lasts_through_the_opposing_action_then_clears() {
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

    assert!(!outcome.state.player_destiny_bond_active);
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
fn destiny_bond_clears_before_an_early_recharge_exit() {
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
    state.player_recharge_move = Some("HYPER_BEAM".to_string());

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
    assert!(outcome.events.contains(&BattleEvent::RechargeTurn {
        side: BattleSide::Player,
        move_name: "HYPER_BEAM".to_string(),
    }));
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
    assert_eq!(outcome.state.enemy_escape_trap, None);
    assert!((2..=5).contains(&trap.turns_remaining));
    assert!(
        outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::TrapApplied {
                side: BattleSide::Player,
                move_name,
                target: BattleSide::Enemy,
                turns: 3..=6,
                roll: 0..=3,
            } if move_name == "BIND"
        )),
        "trap application event missing"
    );
}

#[test]
fn trapped_side_can_act_before_residual_damage_ticks() {
    let player = pokemon("ONIX", 50, pokemon_type("ROCK"), "SPLASH");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([
        (
            "EMBER".to_string(),
            move_data_with_effect("EMBER", pokemon_type("FIRE"), 40, 100, "BURN_HIT"),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Enemy,
            move_name,
            ..
        } if move_name == "TACKLE"
    )));
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

    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert_eq!(outcome.state.enemy_trap, None);
    assert!(outcome.events.contains(&BattleEvent::TrapEnded {
        side: BattleSide::Enemy,
        source: BattleSide::Player,
        move_name: "WRAP".to_string(),
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::TrapDamage {
            side: BattleSide::Enemy,
            ..
        }
    )));
}

#[test]
fn substitute_pauses_partial_trap_counter_and_residual_damage() {
    let player = pokemon("ONIX", 50, pokemon_type("ROCK"), "SPLASH");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let enemy_hp = enemy.hp;
    let mut state = battle_state(player, enemy, 1);
    state.enemy_trap = Some(BattleTrapState {
        source: BattleSide::Player,
        move_name: "WRAP".to_string(),
        turns_remaining: 2,
    });
    state.enemy_substitute_hp = 10;
    let mut events = Vec::new();

    apply_end_turn_trap(&mut state, &mut events);

    assert_eq!(state.enemy.hp, enemy_hp);
    assert_eq!(
        state.enemy_trap.as_ref().map(|trap| trap.turns_remaining),
        Some(2)
    );
    assert!(events.is_empty());
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
fn encore_refuses_forbidden_or_exhausted_last_moves_before_rng() {
    for last_move in ["STRUGGLE", "ENCORE", "MIRROR_MOVE", "TACKLE"] {
        let player = pokemon("CLEFAIRY", 50, pokemon_type("NORMAL"), "ENCORE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), last_move);
        enemy.moves[0].current_pp = 0;
        let mut state = battle_state(player, enemy, 1);
        state.enemy_last_move = Some(last_move.to_string());
        let mut events = Vec::new();
        let mut rng = Random::new(1);
        let seed_before = rng.seed();

        apply_encore_effect(
            &mut state,
            BattleSide::Player,
            "ENCORE",
            &mut rng,
            &mut events,
        );

        assert_eq!(state.enemy_encore, None, "{last_move}");
        assert_eq!(rng.seed(), seed_before, "{last_move}");
        assert!(events.contains(&BattleEvent::EncoreFailed {
            side: BattleSide::Player,
            move_name: "ENCORE".to_string(),
            target: BattleSide::Enemy,
        }));
    }
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
            turns_remaining: 1,
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
        (
            "STRUGGLE".to_string(),
            move_data_with_effect("STRUGGLE", pokemon_type("NORMAL"), 50, 100, "RECOIL_HIT"),
        ),
    ]);
    let mut rng = Random::new(1);
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_last_move = Some("TACKLE".to_string());
    state.enemy_last_counter_move = Some("TACKLE".to_string());

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
    assert!((2..=8).contains(&disable.turns_remaining));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::DisableApplied {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
            disabled_move,
            turns: 2..=8,
            roll: 1..=7,
        } if move_name == "DISABLE" && disabled_move == "TACKLE"
    )));
}

#[test]
fn disable_rejects_struggle_and_zero_packed_pp_but_preserves_pp_up_bits() {
    fn resolve_disable(last_move: &str, current_pp: u8, pp_ups: u8) -> BattleTurnOutcome {
        let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "DISABLE");
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
        enemy.moves[0].current_pp = current_pp;
        enemy.moves[0].pp_ups = pp_ups;
        let moves = BTreeMap::from([
            (
                "DISABLE".to_string(),
                move_data_with_effect("DISABLE", pokemon_type("NORMAL"), 0, 100, "DISABLE"),
            ),
            (
                "TACKLE".to_string(),
                move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
            ),
            (
                "STRUGGLE".to_string(),
                move_data_with_effect("STRUGGLE", pokemon_type("NORMAL"), 50, 100, "RECOIL_HIT"),
            ),
        ]);
        let mut rng = Random::new(1);
        let mut state = battle_state(player, enemy, rng.seed());
        state.enemy_last_move = Some(last_move.to_string());
        state.enemy_last_counter_move = Some(last_move.to_string());

        resolve_battle_turn(
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
        .expect("Disable boundary resolves")
    }

    let struggle = resolve_disable("STRUGGLE", 35, 0);
    assert_eq!(struggle.state.enemy_disable, None);
    assert!(struggle.events.contains(&BattleEvent::DisableFailed {
        side: BattleSide::Player,
        move_name: "DISABLE".to_string(),
        target: BattleSide::Enemy,
    }));

    let empty_pp = resolve_disable("TACKLE", 0, 0);
    assert_eq!(empty_pp.state.enemy_disable, None);
    assert!(empty_pp.events.contains(&BattleEvent::DisableFailed {
        side: BattleSide::Player,
        move_name: "DISABLE".to_string(),
        target: BattleSide::Enemy,
    }));

    let pp_up_only = resolve_disable("TACKLE", 0, 1);
    assert!(pp_up_only.events.iter().any(|event| matches!(
        event,
        BattleEvent::DisableApplied {
            side: BattleSide::Player,
            target: BattleSide::Enemy,
            disabled_move,
            ..
        } if disabled_move == "TACKLE"
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
        (
            "STRUGGLE".to_string(),
            move_data_with_effect("STRUGGLE", pokemon_type("NORMAL"), 50, 100, "RECOIL_HIT"),
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

    assert!(outcome.state.player.hp < player_hp);
    assert_eq!(outcome.state.enemy.moves[0].current_pp, 5);
    assert!(outcome.events.contains(&BattleEvent::MoveUsed {
        side: BattleSide::Enemy,
        move_name: "STRUGGLE".to_string(),
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Enemy,
            move_name,
            ..
        } if move_name == "TACKLE"
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Enemy,
            move_name,
            ..
        } if move_name == "STRUGGLE"
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
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ProtectApplied {
            side: BattleSide::Player,
            move_name,
            counter: 1,
            roll: Some(_),
        } if move_name == "PROTECT"
    )));
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
fn protect_first_use_rejects_zero_and_shifted_zero_fails_without_rng() {
    let seed = (0_u32..)
        .find(|seed| {
            let mut probe = Random::new(*seed);
            probe.battle_random_byte() == 0 && probe.battle_random_byte() != 0
        })
        .expect("a zero-then-nonzero Protect stream exists");
    let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "PROTECT");
    let enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, seed);
    let mut rng = Random::new(seed);
    let mut expected_rng = rng;
    assert_eq!(expected_rng.battle_random_byte(), 0);
    let accepted = expected_rng.battle_random_byte();
    assert_ne!(accepted, 0);

    assert_eq!(
        roll_protect_success(&mut state, BattleSide::Player, &[], &mut rng),
        (true, 0, Some(accepted))
    );
    assert_eq!(rng.seed(), expected_rng.seed());
    assert_eq!(state.player_protect_counter, 1);

    state.player_protect_counter = 8;
    let seed_before = rng.seed();
    assert_eq!(
        roll_protect_success(&mut state, BattleSide::Player, &[], &mut rng),
        (false, 8, None)
    );
    assert_eq!(rng.seed(), seed_before);
    assert_eq!(state.player_protect_counter, 0);
}

#[test]
fn protect_preempts_accuracy_rng_and_blocks_status_like_asm_checkhit() {
    for (enemy_move, enemy_effect) in [("TACKLE", "NORMAL_HIT"), ("SLEEP_POWDER", "SLEEP")] {
        let player = pokemon("CHIKORITA", 45, pokemon_type("GRASS"), "PROTECT");
        let enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), enemy_move);
        let moves = BTreeMap::from([
            (
                "PROTECT".to_string(),
                move_data_with_effect("PROTECT", pokemon_type("NORMAL"), 0, 100, "PROTECT"),
            ),
            (
                enemy_move.to_string(),
                move_data_with_effect(
                    enemy_move,
                    pokemon_type("NORMAL"),
                    u16::from(enemy_effect == "NORMAL_HIT") * 35,
                    50,
                    enemy_effect,
                ),
            ),
        ]);
        let mut rng = Random::new(0x1234_5678);
        let mut baseline_rng = rng;
        let mut baseline_moves = moves.clone();
        baseline_moves.get_mut(enemy_move).unwrap().accuracy = 100;
        let mut priorities = move_priorities();
        priorities
            .effect_priorities
            .insert(enemy_effect.to_string(), 1);
        resolve_battle_turn(
            battle_state(player.clone(), enemy.clone(), baseline_rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: BattleAction::Move { slot: 0 },
            },
            &baseline_moves,
            &priorities,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut baseline_rng,
        )
        .expect("perfect-accuracy Protect baseline resolves");

        let mut protected_state = battle_state(player, enemy, rng.seed());
        protected_state.enemy_lock_on_target = true;
        let outcome = resolve_battle_turn(
            protected_state,
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
        .expect("Protect checkhit ordering resolves");

        assert_eq!(
            rng.seed(),
            baseline_rng.seed(),
            "Protect must preempt accuracy RNG",
        );
        assert_eq!(outcome.state.player.status, None);
        assert!(
            outcome.state.enemy_lock_on_target,
            "Protect must preempt Lock-On consumption",
        );
        assert!(outcome.events.contains(&BattleEvent::MoveProtected {
            side: BattleSide::Enemy,
            move_name: enemy_move.to_string(),
            target: BattleSide::Player,
        }));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Missed {
                side: BattleSide::Enemy,
                move_name,
                ..
            } if move_name == enemy_move
        )));
        assert!(!outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::LockOnConsumed {
                side: BattleSide::Enemy,
                ..
            }
        )));
    }
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
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::EndureApplied {
            side: BattleSide::Enemy,
            move_name,
            counter: 1,
            roll: Some(_),
        } if move_name == "ENDURE"
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::EnduredHit {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
            raw_damage,
            held_item: None,
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
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ProtectApplied {
            side: BattleSide::Player,
            move_name,
            counter: 1,
            roll: Some(_),
        } if move_name == "PROTECT"
    )));
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
    state.enemy_last_counter_move = Some("TACKLE".to_string());

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
fn spite_reduces_a_transformed_targets_effective_move_pp_only() {
    let player = pokemon("GASTLY", 90, pokemon_type("GHOST"), "SPITE");
    let enemy = pokemon("DITTO", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "SPITE".to_string(),
            move_data_with_effect("SPITE", pokemon_type("GHOST"), 0, 100, "SPITE"),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(1);
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_last_move = Some("TACKLE".to_string());
    state.enemy_last_counter_move = Some("TACKLE".to_string());
    state.enemy_transform = Some(BattleTransformState {
        species: species("RATTATA", 40, pokemon_type("NORMAL")),
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
    .expect("Spite against Transform resolves");

    assert_eq!(outcome.state.enemy.moves[0].name, "SPLASH");
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SpiteApplied {
            side: BattleSide::Player,
            target: BattleSide::Enemy,
            target_move,
            pp_before: 5,
            pp_after: 0..=3,
            reduction: 2..=5,
            ..
        } if target_move == "TACKLE"
    )));
    assert!(
        outcome
            .state
            .enemy_transform
            .as_ref()
            .expect("Transform remains active")
            .moves[0]
            .current_pp
            <= 2
    );
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
        (
            "STRUGGLE".to_string(),
            move_data("STRUGGLE", pokemon_type("NORMAL"), 50, 100),
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
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::WeatherContinues { .. }))
    );
}

#[test]
fn expiring_sandstorm_does_not_deal_a_terminal_damage_tick() {
    let player = pokemon("POLIWAG", 90, pokemon_type("WATER"), "NO_DAMAGE");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "NO_DAMAGE");
    let player_hp = player.hp;
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([(
        "NO_DAMAGE".to_string(),
        move_data_with_effect("NO_DAMAGE", pokemon_type("NORMAL"), 0, 100, "NORMAL_HIT"),
    )]);
    let mut rng = Random::new(1);
    let mut state = battle_state(player, enemy, rng.seed());
    state.weather = Weather::Sandstorm;
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
    .expect("terminal sandstorm turn resolves");

    assert_eq!(outcome.state.player.hp, player_hp);
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(outcome.events.contains(&BattleEvent::WeatherEnded {
        weather: Weather::Sandstorm,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::WeatherContinues { .. } | BattleEvent::SandstormDamage { .. }
    )));
}

#[test]
fn sandstorm_sets_weather_and_damages_non_immune_pokemon() {
    let player = pokemon("GEODUDE", 40, pokemon_type("ROCK"), "SANDSTORM");
    let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "NO_DAMAGE");
    let enemy_hp = enemy.hp;
    let enemy_max_hp = enemy.max_hp;
    let sandstorm_damage = (enemy_max_hp / 8).max(1);
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
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_recharge_move = Some("HYPER_BEAM".to_string());

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
    .expect("flinch turn resolves");

    assert!(outcome.state.enemy.hp < enemy_hp);
    assert_eq!(outcome.state.enemy_recharge_move, None);
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
fn slower_secondary_flinch_effect_does_not_apply_after_target_acted() {
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
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::FlinchApplied {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
        } if move_name == "HEADBUTT"
    )));
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::Flinched { .. }))
    );
}

#[test]
fn damaging_secondary_burn_is_silent_for_fire_type_immunity() {
    let player = pokemon("CYNDAQUIL", 50, pokemon_type("FIRE"), "EMBER");
    let enemy = pokemon("MAGMAR", 40, pokemon_type("FIRE"), "TACKLE");
    let moves = BTreeMap::from([
        (
            "EMBER".to_string(),
            move_data_with_effect_chance("EMBER", pokemon_type("FIRE"), 40, 100, "BURN_HIT", 100),
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
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusImmune {
            move_name,
            status,
            ..
        } if move_name == "EMBER" && status == "BURN"
    )));
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
fn enemy_direct_poison_and_paralysis_share_the_asm_quarter_failure_roll() {
    for (move_name, move_type, effect, status) in [
        ("POISONPOWDER", pokemon_type("POISON"), "POISON", "POISON"),
        (
            "THUNDER_WAVE",
            pokemon_type("ELECTRIC"),
            "PARALYZE",
            "PARALYSIS",
        ),
    ] {
        let player = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        let enemy = pokemon("GASTLY", 90, pokemon_type("GHOST"), move_name);
        let moves = BTreeMap::from([
            (
                move_name.to_string(),
                move_data_with_effect(move_name, move_type, 0, 100, effect),
            ),
            (
                "SPLASH".to_string(),
                move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
            ),
        ]);
        let seed = (0_u32..)
            .find(|seed| {
                let mut probe = Random::new(*seed);
                probe.battle_random_byte() < 64
            })
            .expect("a failing AI sample exists");
        let mut state = battle_state(player, enemy, seed);
        state.enemy_effect_ai_random_fail = true;
        let mut rng = Random::new(seed);

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
        .expect("enemy direct status turn resolves");

        assert_eq!(outcome.state.player.status, None, "{move_name} {status}");
        assert!(outcome.events.contains(&BattleEvent::StatusFailed {
            side: BattleSide::Enemy,
            move_name: move_name.to_string(),
            target: BattleSide::Player,
            existing_status: None,
        }));
    }
}

#[test]
fn direct_poison_type_immunity_preempts_the_enemy_ai_failure_roll() {
    let player = pokemon("MAGNEMITE", 40, pokemon_type("STEEL"), "SPLASH");
    let enemy = pokemon("GASTLY", 90, pokemon_type("GHOST"), "POISONPOWDER");
    let moves = BTreeMap::from([
        (
            "POISONPOWDER".to_string(),
            move_data_with_effect("POISONPOWDER", pokemon_type("POISON"), 0, 100, "POISON"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let seed = (0_u32..)
        .find(|seed| {
            let mut probe = Random::new(*seed);
            probe.battle_random_byte() < 64
        })
        .expect("a failing AI sample exists");
    let mut state = battle_state(player, enemy, seed);
    state.enemy_effect_ai_random_fail = true;
    let mut rng = Random::new(seed);

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
    .expect("immune direct poison turn resolves");

    assert_eq!(outcome.state.player.status, None);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusImmune {
            move_name,
            status,
            ..
        } if move_name == "POISONPOWDER" && status == "POISON"
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusFailed { move_name, .. } if move_name == "POISONPOWDER"
    )));
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
fn damaging_secondary_freeze_is_silent_for_ice_type_immunity() {
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
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusImmune {
            move_name,
            status,
            ..
        } if move_name == "ICE_BEAM" && status == "FREEZE"
    )));
}

#[test]
fn frozen_pokemon_does_not_reach_the_asm_pp_command() {
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

    assert_eq!(outcome.state.player.moves[0].current_pp, 5);
    assert_eq!(outcome.state.player.status.as_deref(), Some("FREEZE"));
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(outcome.events.contains(&BattleEvent::FrozenTurn {
        side: BattleSide::Player,
        move_name: "TACKLE".to_string(),
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Player,
            ..
        }
    )));
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
fn sleeping_pokemon_ticks_sleep_without_reaching_the_asm_pp_command() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.status = Some("SLEEP".to_string());
    player.sleep_turns = 2;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
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
    .expect("sleep turn resolves");

    assert_eq!(outcome.state.player.moves[0].current_pp, 5);
    assert_eq!(outcome.state.player.status.as_deref(), Some("SLEEP"));
    assert_eq!(outcome.state.player.sleep_turns, 1);
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(outcome.events.contains(&BattleEvent::SleepTurn {
        side: BattleSide::Player,
        move_name: "TACKLE".to_string(),
        turns_remaining: 1,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Player,
            ..
        }
    )));
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
            move_data_with_effect_chance("SNORE", pokemon_type("NORMAL"), 40, 100, "SNORE", 100),
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
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SleepTalkSelected {
            side: BattleSide::Player,
            move_name,
            selected_slot: 1,
            selected_move,
            roll: 1,
        } if move_name == "SLEEP_TALK" && selected_move == "TACKLE"
    )));
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
    assert!(outcome.events.contains(&BattleEvent::SleepTalkFailed {
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
    state.enemy_last_counter_move = Some("TACKLE".to_string());

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
fn transformed_mimic_replaces_only_the_effective_move_slot() {
    let player = pokemon("DITTO", 90, pokemon_type("NORMAL"), "TRANSFORM");
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
    state.enemy_last_counter_move = Some("TACKLE".to_string());
    state.player_transform = Some(BattleTransformState {
        species: species("MR_MIME", 90, pokemon_type("PSYCHIC_TYPE")),
        dvs: Dv::from_non_hp(10, 10, 10, 10),
        moves: vec![LearnedMove {
            name: "MIMIC".to_string(),
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
    .expect("transformed Mimic turn resolves");

    assert_eq!(outcome.state.player.moves[0].name, "TRANSFORM");
    let transform = outcome
        .state
        .player_transform
        .as_ref()
        .expect("Transform remains active");
    assert_eq!(transform.moves[0].name, "TACKLE");
    assert_eq!(transform.moves[0].current_pp, 5);
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
    state.enemy_last_counter_move = Some("TACKLE".to_string());

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
    let moves = BTreeMap::from([
        (
            "TRANSFORM".to_string(),
            move_data_with_effect("TRANSFORM", pokemon_type("NORMAL"), 0, 100, "TRANSFORM"),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
    ]);
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
    .expect("transform failure resolves");

    assert_eq!(outcome.state.player_transform, None);
    assert!(outcome.events.contains(&BattleEvent::TransformFailed {
        side: BattleSide::Player,
        move_name: "TRANSFORM".to_string(),
        target: BattleSide::Enemy,
    }));
}

#[test]
fn sketch_checks_the_opponents_transform_state_instead_of_the_users() {
    let player = pokemon("SMEARGLE", 50, pokemon_type("NORMAL"), "SKETCH");
    let enemy = pokemon("DITTO", 40, pokemon_type("NORMAL"), "TRANSFORM");
    let mut state = battle_state(player, enemy, 0);
    apply_transform_effect(&mut state, BattleSide::Enemy, "TRANSFORM", &mut Vec::new());
    set_last_move(&mut state, BattleSide::Enemy, Some("TACKLE".to_string()));
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut events = Vec::new();

    apply_sketch_effect(
        &mut state,
        BattleSide::Player,
        Some(0),
        "SKETCH",
        &moves,
        &mut events,
    )
    .expect("Sketch resolves against transformed target");

    assert_eq!(state.player.moves[0].name, "SKETCH");
    assert!(events.contains(&BattleEvent::SketchFailed {
        side: BattleSide::Player,
        move_name: "SKETCH".to_string(),
        target: BattleSide::Enemy,
    }));
}

#[test]
fn sketch_fails_in_link_battles_and_when_the_user_already_knows_the_move() {
    let mut player = pokemon("SMEARGLE", 50, pokemon_type("NORMAL"), "SKETCH");
    player.moves.push(LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 35,
        pp_ups: 0,
    });
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);

    let mut known = battle_state(player.clone(), enemy.clone(), 0);
    set_last_move(&mut known, BattleSide::Enemy, Some("TACKLE".to_string()));
    apply_sketch_effect(
        &mut known,
        BattleSide::Player,
        Some(0),
        "SKETCH",
        &moves,
        &mut Vec::new(),
    )
    .expect("already-known Sketch resolves");
    assert_eq!(known.player.moves[0].name, "SKETCH");

    player.moves.truncate(1);
    let mut link = battle_state(player, enemy, 0);
    link.link_battle = true;
    set_last_move(&mut link, BattleSide::Enemy, Some("TACKLE".to_string()));
    apply_sketch_effect(
        &mut link,
        BattleSide::Player,
        Some(0),
        "SKETCH",
        &moves,
        &mut Vec::new(),
    )
    .expect("link Sketch resolves");
    assert_eq!(link.player.moves[0].name, "SKETCH");
    assert_eq!(last_move(&link, BattleSide::Player), None);
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
        roll: 1,
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
    player.moves.push(LearnedMove {
        name: "SPLASH".to_string(),
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
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
            player: BattleAction::Move { slot: 2 },
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
fn conversion2_numeric_slots_match_crystal_type_constants() {
    assert_eq!(conversion2_type_slot(0).as_deref(), Some("NORMAL"));
    assert_eq!(conversion2_type_slot(6).as_deref(), Some("BIRD"));
    assert_eq!(conversion2_type_slot(9).as_deref(), Some("STEEL"));
    assert_eq!(conversion2_type_slot(10), None);
    assert_eq!(conversion2_type_slot(19), None);
    assert_eq!(conversion2_type_slot(20).as_deref(), Some("FIRE"));
    assert_eq!(conversion2_type_slot(27).as_deref(), Some("DARK"));
    assert_eq!(conversion2_type_slot(28), None);
    assert_eq!(conversion2_type_slot(31), None);
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
    state.enemy_last_counter_move = Some("TACKLE".to_string());

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
    let mut moves = BTreeMap::from([
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
        (
            "EMBER".to_string(),
            move_data("EMBER", pokemon_type("FIRE"), 40, 100),
        ),
    ]);
    for (source_index, move_data) in moves.values_mut().enumerate() {
        move_data.source_index = (source_index + 1) as u8;
    }
    moves.get_mut("EMBER").unwrap().source_index = 64;
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
    .expect("metronome selected move resolves");

    assert_eq!(outcome.state.player.moves[0].current_pp, 4);
    assert_eq!(outcome.state.player.moves[1].current_pp, 5);
    assert!(outcome.state.enemy.hp < enemy_hp);
    assert!(outcome.events.contains(&BattleEvent::MetronomeSelected {
        side: BattleSide::Player,
        move_name: "METRONOME".to_string(),
        selected_move: "EMBER".to_string(),
        roll: 64,
    }));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "EMBER"
    )));
}

#[test]
fn metronome_rejection_loop_consumes_exact_raw_battle_random_bytes() {
    let mut player = pokemon("CLEFAIRY", 90, pokemon_type("NORMAL"), "METRONOME");
    player.moves.push(LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 5,
        pp_ups: 0,
    });
    let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "SPLASH");
    let mut moves = BTreeMap::from([
        (
            "METRONOME".to_string(),
            move_data_with_effect("METRONOME", pokemon_type("NORMAL"), 0, 100, "METRONOME"),
        ),
        (
            "STRUGGLE".to_string(),
            move_data("STRUGGLE", pokemon_type("NORMAL"), 50, 100),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "EMBER".to_string(),
            move_data("EMBER", pokemon_type("FIRE"), 40, 100),
        ),
    ]);
    moves.get_mut("METRONOME").unwrap().source_index = 118;
    moves.get_mut("STRUGGLE").unwrap().source_index = 165;
    moves.get_mut("TACKLE").unwrap().source_index = 33;
    moves.get_mut("EMBER").unwrap().source_index = 64;
    let state = battle_state(player, enemy, 0);
    // Starting from hRandomSub = 0, these pairs yield raw bytes
    // 0, 252, METRONOME, STRUGGLE, TACKLE, then EMBER.
    let mut divider = ReplayDivider::new([0, 0, 0, 4, 0, 134, 0, 209, 0, 132, 0, 225]);
    let mut rng = ExactBattleRandom::new(CrystalRandomState::default(), &mut divider);

    assert_eq!(
        select_metronome_move(&state, BattleSide::Player, &moves, &mut rng),
        Ok((64, "EMBER"))
    );
    assert_eq!(rng.divider_error(), None);
    drop(rng);
    assert_eq!(divider.consumed(), 12);
    assert_eq!(divider.remaining(), 0);
}

#[test]
fn metronome_rejects_an_incomplete_indexed_move_catalog() {
    let player = pokemon("CLEFAIRY", 90, pokemon_type("NORMAL"), "METRONOME");
    let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "METRONOME");
    let moves = BTreeMap::from([(
        "METRONOME".to_string(),
        move_data_with_effect("METRONOME", pokemon_type("NORMAL"), 0, 100, "METRONOME"),
    )]);
    let mut rng = Random::new(21);

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
    .expect_err("Metronome requires the indexed source move catalog");

    assert!(matches!(
        error,
        BattleTurnError::MissingMoveSourceIndex { .. }
    ));
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy_hp = enemy.hp;
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

    assert_eq!(outcome.state.player.moves[0].current_pp, 5);
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Player,
            ..
        }
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::FullyParalyzed {
            side: BattleSide::Player,
            move_name,
            roll: 0..=63,
        } if move_name == "TACKLE"
    )));
}

#[test]
fn confusion_precedes_attract_at_the_asm_action_boundary() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.confusion_turns = 2;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    state.player_attracted_by = Some(BattleSide::Enemy);
    let selected_move = move_data("TACKLE", pokemon_type("NORMAL"), 35, 100);
    let mut rng = ScriptedBattleRandom::new(vec![1, 0]);
    let mut events = Vec::new();

    assert!(
        move_blocked_before_disabled_move(
            &mut state,
            BattleSide::Player,
            "TACKLE",
            &selected_move,
            &BTreeMap::new(),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            &mut rng,
            &mut events,
        )
        .expect("pre-action boundary resolves")
    );

    let confusion_index = events
        .iter()
        .position(|event| matches!(event, BattleEvent::ConfusedTurn { .. }))
        .expect("confusion must be checked");
    let attract_index = events
        .iter()
        .position(|event| matches!(event, BattleEvent::InfatuatedTurn { .. }))
        .expect("Attract must be checked after passing confusion");
    assert!(confusion_index < attract_index);
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
fn held_poison_cure_berry_consumes_after_residual_damage() {
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
    .expect("held poison cure turn resolves");

    let residual = (player_hp / 8).max(1);
    assert_eq!(outcome.state.player.hp, player_hp - residual);
    assert_eq!(outcome.state.player.status, None);
    assert_eq!(outcome.state.player.item, None);
    let residual_index = outcome
        .events
        .iter()
        .position(|event| {
            event
                == &BattleEvent::ResidualStatusDamage {
                    side: BattleSide::Player,
                    status: "POISON".to_string(),
                    damage: residual,
                    hp_before: player_hp,
                    hp_after: player_hp - residual,
                }
        })
        .expect("poison damage resolves before the held-item phase");
    let healing_index = outcome
        .events
        .iter()
        .position(|event| {
            event
                == &BattleEvent::HeldItemStatusHealed {
                    side: BattleSide::Player,
                    item_id: "PSNCURE_BERRY".to_string(),
                    held_effect: "HELD_HEAL_POISON".to_string(),
                    status_before: Some("POISON".to_string()),
                    confusion_turns_before: 0,
                }
        })
        .expect("held poison cure berry activates between turns");
    assert!(residual_index < healing_index);
}

#[test]
fn direct_poison_command_triggers_held_cure_before_residual_damage() {
    let player = pokemon("GASTLY", 50, pokemon_type("GHOST"), "POISONPOWDER");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.item = Some("PSNCURE_BERRY".to_string());
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
    let items = BTreeMap::from([(
        "PSNCURE_BERRY".to_string(),
        held_status_item("PSNCURE_BERRY", "HELD_HEAL_POISON"),
    )]);
    let mut rng = Random::new(1);

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
    .expect("direct poison and held cure resolve");

    assert_eq!(outcome.state.enemy.status, None);
    assert_eq!(outcome.state.enemy.item, None);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemStatusHealed {
            side: BattleSide::Enemy,
            item_id,
            status_before: Some(status),
            ..
        } if item_id == "PSNCURE_BERRY" && status == "POISON"
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ResidualStatusDamage {
            side: BattleSide::Enemy,
            status,
            ..
        } if status == "POISON"
    )));
}

#[test]
fn damaging_poison_command_triggers_held_cure_immediately() {
    let player = pokemon("BEEDRILL", 50, pokemon_type("POISON"), "POISON_STING");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.item = Some("PSNCURE_BERRY".to_string());
    let moves = BTreeMap::from([
        (
            "POISON_STING".to_string(),
            move_data_with_effect_chance(
                "POISON_STING",
                pokemon_type("POISON"),
                15,
                100,
                "POISON_HIT",
                100,
            ),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
    ]);
    let items = BTreeMap::from([(
        "PSNCURE_BERRY".to_string(),
        held_status_item("PSNCURE_BERRY", "HELD_HEAL_POISON"),
    )]);
    let mut rng = Random::new(1);

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
    .expect("damaging poison and held cure resolve");

    let status_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::StatusApplied {
                    target: BattleSide::Enemy,
                    status,
                    ..
                } if status == "POISON"
            )
        })
        .expect("poison applies");
    let cure_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::HeldItemStatusHealed {
                    side: BattleSide::Enemy,
                    item_id,
                    ..
                } if item_id == "PSNCURE_BERRY"
            )
        })
        .expect("held poison cure activates");
    assert!(status_index < cure_index);
    assert_eq!(outcome.state.enemy.status, None);
    assert_eq!(outcome.state.enemy.item, None);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ResidualStatusDamage {
            side: BattleSide::Enemy,
            status,
            ..
        } if status == "POISON"
    )));
}

#[test]
fn leftovers_recovers_one_sixteenth_at_end_of_turn() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.item = Some("LEFTOVERS".to_string());
    player.hp = player.max_hp.saturating_sub(20);
    let player_hp = player.hp;
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.moves[0].current_pp = 0;
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([(
        "LEFTOVERS".to_string(),
        held_boost_item("LEFTOVERS", "HELD_LEFTOVERS"),
    )]);
    let mut rng = Random::new(23);
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
    .expect("leftovers turn resolves");
    assert_eq!(
        outcome.state.player.hp,
        player_hp.saturating_add((outcome.state.player.max_hp / 16).max(1))
    );
    assert!(outcome.events.contains(&BattleEvent::HealApplied {
        side: BattleSide::Player,
        move_name: "LEFTOVERS".to_string(),
        hp_before: player_hp,
        hp_after: outcome.state.player.hp,
        amount: outcome.state.player.hp - player_hp,
        animation_param: 0,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemStatusHealed {
            held_effect,
            ..
        } if held_effect == "HELD_LEFTOVERS"
    )));
}

#[test]
fn serial_clock_owner_controls_dual_side_between_turn_event_order() {
    let cases = [
        (
            LinkSerialConnectionStatus::NotEstablished,
            [BattleSide::Player, BattleSide::Enemy],
        ),
        (
            LinkSerialConnectionStatus::UsingInternalClock,
            [BattleSide::Player, BattleSide::Enemy],
        ),
        (
            LinkSerialConnectionStatus::UsingExternalClock,
            [BattleSide::Enemy, BattleSide::Player],
        ),
    ];
    let items = BTreeMap::from([(
        "LEFTOVERS".to_string(),
        held_boost_item("LEFTOVERS", "HELD_LEFTOVERS"),
    )]);

    for (serial_connection_status, expected_order) in cases {
        assert_eq!(
            between_turn_side_order(serial_connection_status),
            expected_order
        );

        let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "SPLASH");
        player.item = Some("LEFTOVERS".to_string());
        player.hp = player.max_hp.saturating_sub(10);
        player.perish_song_turns = 2;
        let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
        enemy.item = Some("LEFTOVERS".to_string());
        enemy.hp = enemy.max_hp.saturating_sub(10);
        enemy.perish_song_turns = 2;
        let mut state = battle_state(player, enemy, 1);
        state.serial_connection_status = serial_connection_status;

        let mut perish_events = Vec::new();
        apply_end_turn_perish_song(&mut state, &mut perish_events);
        let perish_order = perish_events
            .iter()
            .filter_map(|event| match event {
                BattleEvent::PerishSongCount { side, .. } => Some(*side),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(perish_order, expected_order);

        let mut healing_events = Vec::new();
        apply_end_turn_leftovers(&mut state, &items, &mut healing_events)
            .expect("both Leftovers effects resolve");
        let healing_order = healing_events
            .iter()
            .filter_map(|event| match event {
                BattleEvent::HealApplied { side, .. } => Some(*side),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(healing_order, expected_order);
    }
}

#[test]
fn mystery_berry_restores_first_empty_battle_move_and_is_consumed() {
    let mut player = pokemon("SMEARGLE", 50, pokemon_type("NORMAL"), "TACKLE");
    player.moves.push(LearnedMove {
        name: "SKETCH".to_string(),
        current_pp: 0,
        pp_ups: 0,
    });
    player.moves[0].current_pp = 3;
    player.item = Some("MYSTERY_BERRY".to_string());
    let mut state = battle_state(
        player,
        pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE"),
        1,
    );
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "SKETCH".to_string(),
            move_data("SKETCH", pokemon_type("NORMAL"), 0, 100),
        ),
    ]);
    let items = BTreeMap::from([(
        "MYSTERY_BERRY".to_string(),
        held_boost_item("MYSTERY_BERRY", "HELD_RESTORE_PP"),
    )]);
    let mut events = Vec::new();

    apply_end_turn_mystery_berry(&mut state, &moves, &items, &mut events)
        .expect("Mystery Berry resolves");

    assert_eq!(state.player.moves[0].current_pp, 3);
    assert_eq!(state.player.moves[1].current_pp, 1);
    assert_eq!(state.player.item, None);
    assert!(events.contains(&BattleEvent::HeldItemPpRestored {
        side: BattleSide::Player,
        item_id: "MYSTERY_BERRY".to_string(),
        move_name: "SKETCH".to_string(),
        slot: 1,
        pp_before: 0,
        pp_after: 1,
        amount: 1,
    }));
}

#[test]
fn newly_frozen_battler_cannot_naturally_defrost_on_the_same_turn() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.status = Some("FREEZE".to_string());
    let mut state = battle_state(
        player,
        pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE"),
        1,
    );
    let mut events = vec![BattleEvent::StatusApplied {
        side: BattleSide::Enemy,
        move_name: "ICE_PUNCH".to_string(),
        target: BattleSide::Player,
        status: "FREEZE".to_string(),
    }];
    let mut rng = Random::new(1);
    let seed_before = rng.seed();

    apply_end_turn_defrost(&mut state, &mut rng, &mut events);

    assert_eq!(state.player.status.as_deref(), Some("FREEZE"));
    assert_eq!(rng.seed(), seed_before);
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusHealed {
            target: BattleSide::Player,
            ..
        }
    )));
}

#[test]
fn held_hp_berry_requires_strictly_less_than_half_hp() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.hp = player.max_hp / 2;
    player.item = Some("BERRY".to_string());
    let hp_before = player.hp;
    let mut berry = held_boost_item("BERRY", "HELD_BERRY");
    berry.parameter = 10;
    let items = BTreeMap::from([("BERRY".to_string(), berry)]);
    let mut state = battle_state(
        player,
        pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE"),
        1,
    );
    let mut events = Vec::new();

    apply_held_hp_healing(&mut state, BattleSide::Player, &items, &mut events)
        .expect("held HP berry resolves");

    assert_eq!(state.player.hp, hp_before);
    assert_eq!(state.player.item.as_deref(), Some("BERRY"));
    assert!(events.is_empty());

    state.player.hp = hp_before - 1;
    let below_half = state.player.hp;
    apply_held_hp_healing(&mut state, BattleSide::Player, &items, &mut events)
        .expect("held HP berry resolves below half");

    assert_eq!(state.player.hp, (below_half + 10).min(state.player.max_hp));
    assert_eq!(state.player.item, None);
    assert_eq!(
        events,
        vec![BattleEvent::HeldItemHpHealed {
            side: BattleSide::Player,
            item_id: "BERRY".to_string(),
            hp_before: below_half,
            hp_after: state.player.hp,
            amount: state.player.hp - below_half,
        }]
    );
}

#[test]
fn focus_band_uses_its_exact_asm_held_parameter() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.item = Some("FOCUS_BAND".to_string());
    let state = battle_state(
        player,
        pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE"),
        0,
    );
    let mut focus_band = held_boost_item("FOCUS_BAND", "HELD_FOCUS_BAND");
    focus_band.parameter = 30;
    let items = BTreeMap::from([("FOCUS_BAND".to_string(), focus_band)]);
    let mut success_rng = Random::new(20);
    let mut failure_rng = Random::new(1);
    assert!(
        focus_band_survives(&state, BattleSide::Player, &items, &mut success_rng)
            .expect("focus band lookup")
    );
    assert!(
        !focus_band_survives(&state, BattleSide::Player, &items, &mut failure_rng)
            .expect("focus band lookup")
    );
}

#[test]
fn kings_rock_is_gated_by_the_move_effect_script() {
    assert!(move_has_kings_rock_command(&move_data(
        "TACKLE",
        pokemon_type("NORMAL"),
        35,
        100,
    )));
    for effect in [
        "BURN_HIT",
        "DEFENSE_DOWN_HIT",
        "DREAM_EATER",
        "FLAME_WHEEL",
        "FLINCH_HIT",
        "SACRED_FIRE",
        "TRAP_TARGET",
        "TRI_ATTACK",
    ] {
        assert!(!move_has_kings_rock_command(&move_data_with_effect(
            "SCRIPTED_MOVE",
            pokemon_type("NORMAL"),
            40,
            100,
            effect,
        )));
    }
}

#[test]
fn kings_rock_does_not_add_a_second_roll_to_builtin_flinch_scripts() {
    let mut player = pokemon("DUNSPARCE", 50, pokemon_type("NORMAL"), "HEADBUTT");
    player.item = Some("KINGS_ROCK".to_string());
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 255;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let move_data = move_data_with_effect_chance(
        "HEADBUTT",
        pokemon_type("NORMAL"),
        70,
        100,
        "FLINCH_HIT",
        30,
    );
    let mut events = Vec::new();
    let mut rng = Random::new(20);
    let seed_before = rng.seed();

    apply_kings_rock_flinch(
        &mut state,
        BattleSide::Player,
        "HEADBUTT",
        &move_data,
        1,
        &items,
        &mut rng,
        &mut events,
    )
    .expect("built-in flinch King's Rock gate");

    assert_eq!(rng.seed(), seed_before);
    assert!(!state.enemy.flinching);
    assert!(events.is_empty());
}

#[test]
fn kings_rock_observes_a_substitute_after_the_hit_breaks_it() {
    let mut player = pokemon("HOOTHOOT", 90, pokemon_type("NORMAL"), "TACKLE");
    player.item = Some("KINGS_ROCK".to_string());
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 255;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let mut state = battle_state(player, enemy, 20);
    state.enemy_substitute_hp = 1;
    let mut rng = Random::new(20);

    let outcome = resolve_battle_turn_with_items(
        state,
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
    .expect("King's Rock hit through a broken Substitute resolves");

    assert!(outcome.events.contains(&BattleEvent::SubstituteBroken {
        side: BattleSide::Player,
        move_name: "TACKLE".to_string(),
        target: BattleSide::Enemy,
    }));
    assert!(outcome.events.contains(&BattleEvent::FlinchApplied {
        side: BattleSide::Player,
        move_name: "TACKLE".to_string(),
        target: BattleSide::Enemy,
    }));
}

#[test]
fn ordinary_hit_executes_the_kings_rock_command_once() {
    let mut player = pokemon("HOOTHOOT", 90, pokemon_type("NORMAL"), "TACKLE");
    player.item = Some("KINGS_ROCK".to_string());
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 255;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let mut rng = Random::new(20);

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
    .expect("ordinary King's Rock hit resolves");

    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(
                event,
                BattleEvent::FlinchApplied {
                    side: BattleSide::Player,
                    move_name,
                    target: BattleSide::Enemy,
                } if move_name == "TACKLE"
            ))
            .count(),
        1,
    );
}

fn source_effect_command_inventory(command: &str) -> Vec<(String, String, bool)> {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let constants = std::fs::read_to_string(
        repo_root.join("vendor/pokecrystal/constants/move_effect_constants.asm"),
    )
    .expect("move effect constants");
    let pointers = std::fs::read_to_string(
        repo_root.join("vendor/pokecrystal/data/moves/effects_pointers.asm"),
    )
    .expect("move effect pointers");
    let scripts =
        std::fs::read_to_string(repo_root.join("vendor/pokecrystal/data/moves/effects.asm"))
            .expect("move effect scripts");
    let moves = std::fs::read_to_string(repo_root.join("vendor/pokecrystal/data/moves/moves.asm"))
        .expect("source moves");
    let used_effects = moves
        .lines()
        .filter_map(|line| {
            let (_, tail) = line.trim().strip_prefix("move ")?.split_once(',')?;
            tail.trim()
                .strip_prefix("EFFECT_")
                .and_then(|effect| effect.split(',').next())
                .map(str::trim)
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    let effect_ids = constants
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("const EFFECT_")
                .and_then(|tail| tail.split_whitespace().next())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    let pointer_labels = pointers
        .lines()
        .filter_map(|line| line.trim().strip_prefix("dw ").map(str::trim))
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(effect_ids.len(), pointer_labels.len());

    let mut label_has_command = BTreeMap::<String, bool>::new();
    let mut pending_labels = Vec::<String>::new();
    let mut active_labels = Vec::<String>::new();
    let mut active_has_command = false;
    let flush = |labels: &mut Vec<String>, has_command: bool, out: &mut BTreeMap<String, bool>| {
        for label in labels.drain(..) {
            out.insert(label, has_command);
        }
    };
    for line in scripts.lines() {
        let trimmed = line.trim();
        if !line.chars().next().is_some_and(char::is_whitespace)
            && let Some(label) = trimmed.strip_suffix(':')
            && label
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            if !active_labels.is_empty() {
                flush(
                    &mut active_labels,
                    active_has_command,
                    &mut label_has_command,
                );
                active_has_command = false;
            }
            pending_labels.push(label.to_string());
            continue;
        }
        if !trimmed.is_empty() && !trimmed.starts_with(';') && !pending_labels.is_empty() {
            active_labels.append(&mut pending_labels);
        }
        if !active_labels.is_empty() && trimmed == command {
            active_has_command = true;
        }
    }
    flush(
        &mut active_labels,
        active_has_command,
        &mut label_has_command,
    );

    effect_ids
        .into_iter()
        .zip(pointer_labels)
        .filter(|(effect_id, _)| used_effects.contains(effect_id))
        .map(|(effect_id, pointer_label)| {
            let has_command = *label_has_command
                .get(&pointer_label)
                .unwrap_or_else(|| panic!("missing source effect script {pointer_label}"));
            let canonical_effect_id = effect_id
                .replace("SP_ATK_", "SPECIAL_ATTACK_")
                .replace("SP_DEF_", "SPECIAL_DEFENSE_");
            (canonical_effect_id, pointer_label, has_command)
        })
        .collect()
}

#[test]
fn kings_rock_eligibility_matches_every_used_source_effect_script() {
    for (effect_id, pointer_label, expected) in source_effect_command_inventory("kingsrock") {
        let move_data =
            move_data_with_effect("SOURCE_EFFECT", pokemon_type("NORMAL"), 40, 100, &effect_id);
        assert_eq!(
            move_has_kings_rock_command(&move_data),
            expected,
            "EFFECT_{effect_id} -> {pointer_label}"
        );
    }
}

#[test]
fn shared_check_hit_eligibility_matches_every_used_source_effect_script() {
    for (effect_id, pointer_label, expected) in source_effect_command_inventory("checkhit") {
        // Bide reaches the shared check-hit implementation only on its release
        // turn, after its direct setup/storage state machine has completed.
        let reaches_check_hit = move_effect_uses_check_hit(&effect_id) || effect_id == "BIDE";
        assert_eq!(
            reaches_check_hit, expected,
            "EFFECT_{effect_id} -> {pointer_label}"
        );
    }
}

#[test]
fn pre_damage_effect_chance_eligibility_matches_every_used_source_effect_script() {
    for (effect_id, pointer_label, expected) in source_effect_command_inventory("effectchance") {
        let move_data = move_data_with_effect_chance(
            "SOURCE_EFFECT",
            pokemon_type("NORMAL"),
            40,
            100,
            &effect_id,
            30,
        );
        // PoisonMultiHit owns its effectchance immediately before the first
        // critical command; EndLoop returns to critical for later strikes.
        // Every other damaging script samples through apply_damage_hit.
        let samples_before_damage = damaging_effect_script_has_pre_damage_effect_chance(&move_data)
            || effect_id == "POISON_MULTI_HIT";
        assert_eq!(
            samples_before_damage, expected,
            "EFFECT_{effect_id} -> {pointer_label}"
        );
    }
}

#[test]
fn brightpowder_subtracts_its_asm_accuracy_penalty() {
    let mut target = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    target.item = Some("BRIGHTPOWDER".to_string());
    let state = battle_state(
        pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE"),
        target,
        0,
    );
    let mut item = held_boost_item("BRIGHTPOWDER", "HELD_BRIGHTPOWDER");
    item.parameter = 20;
    let items = BTreeMap::from([("BRIGHTPOWDER".to_string(), item)]);
    assert_eq!(
        apply_brightpowder_accuracy(&state, BattleSide::Enemy, &items, 100)
            .expect("brightpowder lookup"),
        80
    );
}

#[test]
fn x_accuracy_sets_persistent_sure_hit_substatus_instead_of_accuracy_stage() {
    let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
    let enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    let accuracy_stage_before = state.player.stat_boosts[&Stat::Accuracy];
    let mut item = battle_item("X_ACCURACY", 0, true);
    item.battle_stat_boost_stat = Some("ACCURACY".to_string());
    item.battle_stat_boost_stages = Some(1);
    let items = BTreeMap::from([("X_ACCURACY".to_string(), item)]);
    let mut events = Vec::new();

    execute_item(
        &mut state,
        BattleSide::Player,
        "X_ACCURACY",
        &items,
        &mut events,
    )
    .expect("X Accuracy applies");

    assert!(state.player_x_accuracy);
    assert_eq!(
        state.player.stat_boosts[&Stat::Accuracy],
        accuracy_stage_before
    );
    let effect = events
        .iter()
        .find_map(|event| match event {
            BattleEvent::BattleItemEffect { outcome, .. } => Some(outcome),
            _ => None,
        })
        .expect("X Accuracy item outcome");
    assert!(effect.battle_stat_stage_changes.is_empty());
    assert!(matches!(
        execute_item(
            &mut state,
            BattleSide::Player,
            "X_ACCURACY",
            &items,
            &mut Vec::new(),
        ),
        Err(BattleTurnError::BattleItem { .. })
    ));
    clear_side_volatile_conditions(&mut state, BattleSide::Player);
    assert!(!state.player_x_accuracy);
}

#[test]
fn x_accuracy_bypasses_brightpowder_and_accuracy_rng() {
    let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("PIDGEY", 10, pokemon_type("NORMAL"), "SPLASH");
    enemy.item = Some("BRIGHTPOWDER".to_string());
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 1),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut brightpowder = held_boost_item("BRIGHTPOWDER", "HELD_BRIGHTPOWDER");
    brightpowder.parameter = 20;
    let items = BTreeMap::from([("BRIGHTPOWDER".to_string(), brightpowder)]);
    let mut state = battle_state(player, enemy, 0);
    state.player_x_accuracy = true;
    let mut rng = Random::new(1);

    let outcome = resolve_battle_turn_with_items(
        state,
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
    .expect("X Accuracy sure-hit turn resolves");

    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "TACKLE"
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Missed {
            side: BattleSide::Player,
            ..
        }
    )));
}

#[test]
fn metal_powder_boosts_only_ditto_inside_shared_damage_calculation() {
    let attacker = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
    let ditto = pokemon("DITTO", 40, pokemon_type("NORMAL"), "SPLASH");
    let mut held_ditto = ditto.clone();
    held_ditto.item = Some("METAL_POWDER".to_string());
    let mut held_non_ditto = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    held_non_ditto.defense = ditto.defense;
    held_non_ditto.max_hp = ditto.max_hp;
    held_non_ditto.hp = ditto.hp;
    held_non_ditto.item = Some("METAL_POWDER".to_string());
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
    let items = BTreeMap::from([(
        "METAL_POWDER".to_string(),
        held_boost_item("METAL_POWDER", "HELD_METAL_POWDER"),
    )]);
    let resolve = |defender: Pokemon| {
        let mut rng = Random::new(13);
        resolve_battle_turn_with_items(
            battle_state(attacker.clone(), defender, rng.seed()),
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
        .expect("Metal Powder damage comparison resolves")
    };
    let plain = resolve(ditto);
    let boosted = resolve(held_ditto);
    let non_ditto = resolve(held_non_ditto);

    assert!(player_damage_amount(&boosted.events) < player_damage_amount(&plain.events));
    assert_eq!(
        player_damage_amount(&non_ditto.events),
        player_damage_amount(&plain.events)
    );
}

#[test]
fn kings_rock_uses_its_exact_asm_held_parameter() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.item = Some("KINGS_ROCK".to_string());
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    let mut kings_rock = held_boost_item("KINGS_ROCK", "HELD_FLINCH");
    kings_rock.parameter = 30;
    let items = BTreeMap::from([("KINGS_ROCK".to_string(), kings_rock)]);
    let move_data = move_data("TACKLE", pokemon_type("NORMAL"), 35, 100);
    let mut events = Vec::new();
    let mut rng = Random::new(20);
    apply_kings_rock_flinch(
        &mut state,
        BattleSide::Player,
        "TACKLE",
        &move_data,
        1,
        &items,
        &mut rng,
        &mut events,
    )
    .expect("kings rock lookup");
    assert!(state.enemy.flinching);
    assert!(events.contains(&BattleEvent::FlinchApplied {
        side: BattleSide::Player,
        move_name: "TACKLE".to_string(),
        target: BattleSide::Enemy,
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
    .expect("held confusion cure turn resolves");

    assert_eq!(outcome.state.player.confusion_turns, 0);
    assert_eq!(outcome.state.player.item, None);
    assert!(outcome.events.contains(&BattleEvent::HeldItemStatusHealed {
        side: BattleSide::Player,
        item_id: "BITTER_BERRY".to_string(),
        held_effect: "HELD_HEAL_CONFUSION".to_string(),
        status_before: None,
        confusion_turns_before: 2,
    }));
}

#[test]
fn confuse_target_command_triggers_held_cure_before_the_target_acts() {
    let mut player = pokemon("GASTLY", 50, pokemon_type("GHOST"), "CONFUSE_RAY");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    player.speed = 1_000;
    enemy.speed = 1;
    enemy.item = Some("BITTER_BERRY".to_string());
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
    let items = BTreeMap::from([(
        "BITTER_BERRY".to_string(),
        held_status_item("BITTER_BERRY", "HELD_HEAL_CONFUSION"),
    )]);
    let mut rng = Random::new(1);

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
    .expect("confusion and immediate held cure resolve");

    let cure_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::HeldItemStatusHealed {
                    side: BattleSide::Enemy,
                    item_id,
                    confusion_turns_before: 2..=5,
                    ..
                } if item_id == "BITTER_BERRY"
            )
        })
        .expect("held confusion cure activates");
    let target_action_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    side: BattleSide::Enemy,
                    move_name,
                    ..
                } if move_name == "TACKLE"
            )
        })
        .expect("cured target still takes its selected action");
    assert!(cure_index < target_action_index);
    assert_eq!(outcome.state.enemy.confusion_turns, 0);
    assert_eq!(outcome.state.enemy.item, None);
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
    .expect("bad poison residual turn resolves");

    // ResidualDamage increments wPlayerToxicCount before multiplying, so a
    // stored count of 3 produces the fourth Toxic damage step.
    let residual = (player_hp / 16).max(1) * 4;
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
fn bad_poison_counter_wraps_and_executes_256_damage_additions() {
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
    state.player_toxic_turns = u8::MAX;

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
    .expect("wrapped bad poison residual turn resolves");

    assert_eq!(outcome.state.player_toxic_turns, 0);
    assert_eq!(outcome.state.player.hp, 0);
    assert!(outcome.events.contains(&BattleEvent::ResidualStatusDamage {
        side: BattleSide::Player,
        status: "BAD_POISON".to_string(),
        damage: player_hp,
        hp_before: player_hp,
        hp_after: 0,
    }));
}

#[test]
fn switching_resets_bad_poison_counter_and_normalizes_persistent_status() {
    let mut player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    player.status = Some("BAD_POISON".to_string());
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 44);
    state.player_toxic_turns = 4;

    clear_side_volatile_conditions(&mut state, BattleSide::Player);

    assert_eq!(state.player.status.as_deref(), Some("POISON"));
    assert_eq!(state.player_toxic_turns, 0);
}

#[test]
fn burn_residual_damage_can_faint_at_end_of_turn() {
    let player = pokemon("HOOTHOOT", 50, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    enemy.status = Some("BURN".to_string());
    enemy.hp = 1;
    enemy.moves[0].current_pp = 0;
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "STRUGGLE".to_string(),
            move_data("STRUGGLE", pokemon_type("NORMAL"), 50, 100),
        ),
    ]);
    let mut rng = Random::new(29);

    let outcome = resolve_battle_enemy_action_with_items(
        battle_state(player, enemy, rng.seed()),
        BattleAction::Move { slot: 0 },
        false,
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
fn forced_encore_move_priority_overrides_requested_slot_priority() {
    let player = pokemon("RATTATA", 100, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "QUICK_ATTACK");
    enemy.moves.push(LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 5,
        pp_ups: 0,
    });
    let moves = BTreeMap::from([
        (
            "QUICK_ATTACK".to_string(),
            move_data_with_effect(
                "QUICK_ATTACK",
                pokemon_type("NORMAL"),
                40,
                100,
                "PRIORITY_HIT",
            ),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
    ]);
    let mut state = battle_state(player, enemy, 1);
    state.enemy_encore = Some(BattleEncoreState {
        move_name: "QUICK_ATTACK".to_string(),
        turns_remaining: 2,
    });
    let mut rng = Random::new(1);

    let order = determine_turn_order(
        &state,
        &BattleTurnInput {
            player: BattleAction::Move { slot: 0 },
            enemy: BattleAction::Move { slot: 1 },
        },
        &moves,
        &BTreeMap::new(),
        &move_priorities(),
        &stat_multipliers(),
        &mut rng,
    )
    .expect("forced Encore priority resolves");

    assert_eq!(order, vec![BattleSide::Enemy, BattleSide::Player]);
}

#[test]
fn every_retained_move_state_uses_its_committed_base_priority_not_the_requested_slot() {
    let player = pokemon("RATTATA", 100, pokemon_type("NORMAL"), "TACKLE");
    let enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "QUICK_ATTACK");
    let mut moves = BTreeMap::from([
        (
            "QUICK_ATTACK".to_string(),
            move_data_with_effect(
                "QUICK_ATTACK",
                pokemon_type("NORMAL"),
                40,
                100,
                "PRIORITY_HIT",
            ),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
    ]);
    for move_name in ["BIDE", "ROLLOUT", "THRASH", "FLY", "SOLAR_BEAM"] {
        moves.insert(
            move_name.to_string(),
            move_data(move_name, pokemon_type("NORMAL"), 35, 100),
        );
    }
    let base = battle_state(player, enemy, 1);
    let mut states = Vec::new();
    let mut bide = base.clone();
    bide.enemy_bide_turns = 1;
    bide.enemy_last_move = Some("BIDE".to_string());
    states.push(("Bide", bide));
    let mut rollout = base.clone();
    rollout.enemy_rollout_turns = 1;
    rollout.enemy_last_move = Some("ROLLOUT".to_string());
    states.push(("Rollout", rollout));
    let mut rampage = base.clone();
    rampage.enemy.rampage_turns = 1;
    rampage.enemy_last_move = Some("THRASH".to_string());
    states.push(("rampage", rampage));
    let mut airborne = base.clone();
    airborne.enemy_airborne_move = Some("FLY".to_string());
    states.push(("Fly", airborne));
    let mut charging = base.clone();
    charging.enemy_charging_move = Some("SOLAR_BEAM".to_string());
    states.push(("charge", charging));
    let mut recharge = base;
    recharge.enemy_recharge_move = Some("HYPER_BEAM".to_string());
    states.push(("recharge", recharge));

    for (label, state) in states {
        let mut rng = Random::new(1);
        let order = determine_turn_order(
            &state,
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
        .unwrap_or_else(|error| panic!("{label} retained priority failed: {error:?}"));
        assert_eq!(
            order,
            vec![BattleSide::Player, BattleSide::Enemy],
            "{label} must not inherit Quick Attack priority from the requested slot"
        );
    }
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
fn both_quick_claws_sample_enemy_first_outside_internal_clock_link_play() {
    let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
    player.item = Some("QUICK_CLAW".to_string());
    let mut enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "TACKLE");
    enemy.item = Some("QUICK_CLAW".to_string());
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("QUICK_CLAW".to_string(), quick_claw_item(80))]);
    let mut rng = Random::new(1);
    let mut expected_rng = rng;
    assert_eq!(expected_rng.battle_random_byte(), 64);

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
    .expect("dual Quick Claw turn order resolves");

    assert_eq!(order, vec![BattleSide::Enemy, BattleSide::Player]);
    assert_eq!(
        rng.seed(),
        expected_rng.seed(),
        "first activation returns immediately"
    );
}

#[test]
fn both_quick_claws_sample_player_first_for_internal_clock_link_player() {
    let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
    player.item = Some("QUICK_CLAW".to_string());
    let mut enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "TACKLE");
    enemy.item = Some("QUICK_CLAW".to_string());
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("QUICK_CLAW".to_string(), quick_claw_item(80))]);
    let mut state = battle_state(player, enemy, 1);
    state.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    let mut rng = Random::new(1);
    let mut expected_rng = rng;
    assert_eq!(expected_rng.battle_random_byte(), 64);

    let order = determine_turn_order(
        &state,
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
    .expect("internal-clock dual Quick Claw turn order resolves");

    assert_eq!(order, vec![BattleSide::Player, BattleSide::Enemy]);
    assert_eq!(
        rng.seed(),
        expected_rng.seed(),
        "first activation returns immediately"
    );
}

#[test]
fn zero_parameter_quick_claw_never_activates_but_consumes_its_asm_roll() {
    let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
    player.item = Some("QUICK_CLAW".to_string());
    let enemy = pokemon("PIDGEY", 100, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("QUICK_CLAW".to_string(), quick_claw_item(0))]);
    let mut rng = Random::new(20);

    let mut expected_rng = rng;
    expected_rng.battle_random_byte();
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
    .expect("zero-parameter Quick Claw resolves as a failed activation");

    assert_eq!(order, vec![BattleSide::Enemy, BattleSide::Player]);
    assert_eq!(rng.seed(), expected_rng.seed());
}

#[test]
fn quick_claw_rejects_parameter_outside_the_asm_byte_range() {
    let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
    player.item = Some("QUICK_CLAW".to_string());
    let enemy = pokemon("PIDGEY", 100, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("QUICK_CLAW".to_string(), quick_claw_item(-1))]);
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
    .expect_err("out-of-byte Quick Claw parameter must fail");

    assert_eq!(
        error,
        BattleTurnError::InvalidHeldItemParameter {
            side: BattleSide::Player,
            item_id: "QUICK_CLAW".to_string(),
            held_effect: "HELD_QUICK_CLAW".to_string(),
            parameter: -1,
        }
    );
}

#[test]
fn equal_priority_item_actions_return_player_first_without_quick_claw_rng() {
    let mut player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
    player.item = Some("QUICK_CLAW".to_string());
    let mut enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "TACKLE");
    enemy.item = Some("QUICK_CLAW".to_string());
    let items = BTreeMap::from([
        ("QUICK_CLAW".to_string(), quick_claw_item(255)),
        ("POTION".to_string(), battle_item("POTION", 20, true)),
    ]);
    let mut rng = Random::new(1);
    let seed_before = rng.seed();

    let order = determine_turn_order(
        &battle_state(player, enemy, seed_before),
        &BattleTurnInput {
            player: BattleAction::Item {
                item_id: "POTION".to_string(),
            },
            enemy: BattleAction::Item {
                item_id: "POTION".to_string(),
            },
        },
        &BTreeMap::new(),
        &items,
        &move_priorities(),
        &stat_multipliers(),
        &mut rng,
    )
    .expect("equal-priority item actions resolve");

    assert_eq!(order, vec![BattleSide::Player, BattleSide::Enemy]);
    assert_eq!(rng.seed(), seed_before);
}

#[test]
fn trainer_switch_and_item_use_the_preselected_move_for_order_then_execute_before_moves() {
    let player = pokemon("RATTATA", 20, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("MACHOP", 120, pokemon_type("FIGHTING"), "VITAL_THROW");
    enemy.hp = enemy.max_hp.saturating_sub(10);
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 1, 100),
        ),
        (
            "VITAL_THROW".to_string(),
            move_data("VITAL_THROW", pokemon_type("FIGHTING"), 70, 100),
        ),
    ]);
    let items = BTreeMap::from([("POTION".to_string(), battle_item("POTION", 20, true))]);

    for enemy_action in [
        BattleAction::TrainerSwitch {
            selected_move_slot: 0,
            party_index: 1,
        },
        BattleAction::TrainerItem {
            selected_move_slot: 0,
            item_id: "POTION".to_string(),
        },
    ] {
        let mut rng = Random::new(1);
        let outcome = resolve_battle_turn_with_items(
            battle_state(player.clone(), enemy.clone(), rng.seed()),
            BattleTurnInput {
                player: BattleAction::Move { slot: 0 },
                enemy: enemy_action.clone(),
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
        .expect("trainer post-order action resolves");

        assert_eq!(
            outcome.order,
            vec![BattleSide::Player, BattleSide::Enemy],
            "{enemy_action:?}"
        );
        let player_damage = outcome
            .events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    BattleEvent::Damage {
                        side: BattleSide::Player,
                        ..
                    }
                )
            })
            .expect("player damage occurs");
        let trainer_action = outcome
            .events
            .iter()
            .position(|event| match (&enemy_action, event) {
                (
                    BattleAction::TrainerSwitch { .. },
                    BattleEvent::Switched {
                        side: BattleSide::Enemy,
                        ..
                    },
                ) => true,
                (
                    BattleAction::TrainerItem { .. },
                    BattleEvent::ItemUsed {
                        side: BattleSide::Enemy,
                        ..
                    },
                ) => true,
                _ => false,
            })
            .expect("trainer action occurs");
        assert!(trainer_action < player_damage, "{enemy_action:?}");
    }
}

#[test]
fn trainer_post_order_selector_runs_after_equal_speed_order_rng() {
    let player = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut rng = ScriptedBattleRandom::new(vec![0x11, 0, 0xc8]);
    let mut observed_move_roll = None;
    let mut observed_decision_roll = None;
    let mut select_enemy_move = |_: &BattleCombatState, rng: &mut dyn BattleRandomSource| {
        observed_move_roll = Some(rng.battle_random_byte());
        Ok(0usize)
    };
    let mut select_enemy_action = |_: &BattleCombatState,
                                   rng: &mut dyn BattleRandomSource|
     -> Result<BattleAction, BattleTurnError> {
        observed_decision_roll = Some(rng.battle_random_byte());
        Ok(BattleAction::TrainerSwitch {
            selected_move_slot: 0,
            party_index: 1,
        })
    };

    let outcome = resolve_battle_turn_with_enemy_ai_actions(
        battle_state(player, enemy, 0),
        BattleAction::Move { slot: 0 },
        &moves,
        &BTreeMap::new(),
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        &mut select_enemy_move,
        &mut select_enemy_action,
    )
    .expect("post-order trainer switch resolves");

    assert_eq!(observed_move_roll, Some(0x11));
    assert_eq!(observed_decision_roll, Some(0xc8));
    assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
    assert_eq!(outcome.state.enemy_party_index, 1);
}

#[test]
fn enemy_move_selector_observes_berserk_gene_start_of_turn_state() {
    let player = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let mut enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.item = Some("BERSERK_GENE".to_string());
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let items = BTreeMap::from([(
        "BERSERK_GENE".to_string(),
        held_status_item("BERSERK_GENE", "HELD_ATTACK_UP"),
    )]);
    let mut observed = None;
    let mut select_enemy_move = |state: &BattleCombatState, _: &mut dyn BattleRandomSource| {
        observed = Some((
            state.enemy.item.clone(),
            state.enemy.stat_boosts.get(&Stat::Attack).copied(),
            state.enemy.confusion_turns,
        ));
        Ok(0usize)
    };
    let mut select_enemy_action =
        |_: &BattleCombatState, _: &mut dyn BattleRandomSource| Ok(BattleAction::Move { slot: 0 });
    let mut rng = Random::new(1);

    resolve_battle_turn_with_enemy_ai_actions(
        battle_state(player, enemy, rng.seed()),
        BattleAction::Move { slot: 0 },
        &moves,
        &items,
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        &mut select_enemy_move,
        &mut select_enemy_action,
    )
    .expect("turn with core-owned enemy move selection resolves");

    assert_eq!(observed, Some((None, Some(2), 256)));
}

#[test]
fn failed_wild_run_keeps_berserk_ai_escape_and_enemy_flee_rng_in_source_order() {
    let mut player = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    player.speed = 1;
    let mut enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    enemy.speed = 255;
    enemy.item = Some("BERSERK_GENE".to_string());
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let items = BTreeMap::from([(
        "BERSERK_GENE".to_string(),
        held_status_item("BERSERK_GENE", "HELD_ATTACK_UP"),
    )]);
    let mut observed = None;
    let mut select_enemy_move = |state: &BattleCombatState, rng: &mut dyn BattleRandomSource| {
        observed = Some((
            state.enemy.item.clone(),
            state.enemy.stat_boosts.get(&Stat::Attack).copied(),
            state.enemy.confusion_turns,
            rng.battle_random_byte(),
        ));
        Ok(0usize)
    };
    let mut select_enemy_action =
        |_: &BattleCombatState, _: &mut dyn BattleRandomSource| Ok(BattleAction::Move { slot: 0 });
    let mut rng = ScriptedBattleRandom::new(vec![0x11, 0xfe, 128, 255]);

    let outcome = resolve_wild_battle_turn_with_enemy_ai_actions(
        battle_state(player, enemy, 1),
        BattleAction::Run,
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
        0,
        &mut rng,
        &mut select_enemy_move,
        &mut select_enemy_action,
    )
    .expect("failed run remains one core-owned wild turn");

    assert_eq!(observed, Some((None, Some(2), 256, 0x11)));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::RunAttempt {
            outcome: BattleEscapeAttempt {
                escaped: false,
                roll: Some(0xfe),
                ..
            },
            ..
        }
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Fled {
            side: BattleSide::Enemy
        }
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Enemy,
            move_name,
            ..
        } if move_name == "SPLASH"
    )));
    assert_eq!(rng.calls, 4);
}

#[test]
fn trainer_basic_redundancy_preserves_source_side_checks_and_bugs() {
    let player = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);

    state.player.status = Some("POISON".to_string());
    assert_eq!(
        crate::battle::ai::trainer_basic_score_delta(&state, "NIGHTMARE"),
        0,
        "Nightmare's source bug accepts any major status",
    );
    state.enemy_future_sight = Some(BattleFutureSightState {
        source: BattleSide::Enemy,
        move_name: "FUTURE_SIGHT".to_string(),
        turns_remaining: 2,
        damage: 10,
    });
    assert_eq!(
        crate::battle::ai::trainer_basic_score_delta(&state, "FUTURE_SIGHT"),
        0,
        "the unused-screen check never recognizes queued Future Sight",
    );
    state.player_escape_trap = Some(BattleEscapeTrapState {
        source: BattleSide::Enemy,
        move_name: "MEAN_LOOK".to_string(),
    });
    assert_eq!(
        crate::battle::ai::trainer_basic_score_delta(&state, "MEAN_LOOK"),
        0,
        "Mean Look checks the enemy's own CANT_RUN state",
    );
    state.enemy_escape_trap = Some(BattleEscapeTrapState {
        source: BattleSide::Player,
        move_name: "MEAN_LOOK".to_string(),
    });
    assert_eq!(
        crate::battle::ai::trainer_basic_score_delta(&state, "MEAN_LOOK"),
        10,
    );
    state.player_safeguard_turns = 3;
    assert_eq!(
        crate::battle::ai::trainer_basic_score_delta(&state, "PARALYZE"),
        10,
    );
}

#[test]
fn trainer_post_order_action_executes_before_the_nominally_first_move() {
    let player = pokemon("RATTATA", 100, pokemon_type("NORMAL"), "TACKLE");
    let enemy = pokemon("MACHOP", 40, pokemon_type("FIGHTING"), "VITAL_THROW");
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 40, 100),
        ),
        (
            "VITAL_THROW".to_string(),
            move_data_with_effect(
                "VITAL_THROW",
                pokemon_type("FIGHTING"),
                70,
                100,
                "ALWAYS_HIT",
            ),
        ),
    ]);
    let mut rng = Random::new(1);
    let mut select_enemy_move = |_: &BattleCombatState, _: &mut dyn BattleRandomSource| Ok(0usize);
    let mut select_enemy_action = |_: &BattleCombatState,
                                   _: &mut dyn BattleRandomSource|
     -> Result<BattleAction, BattleTurnError> {
        Ok(BattleAction::TrainerSwitch {
            selected_move_slot: 0,
            party_index: 1,
        })
    };

    let outcome = resolve_battle_turn_with_enemy_ai_actions(
        battle_state(player, enemy, rng.seed()),
        BattleAction::Move { slot: 0 },
        &moves,
        &BTreeMap::new(),
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        &mut select_enemy_move,
        &mut select_enemy_action,
    )
    .expect("post-order trainer switch resolves");

    assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
    let switch_event = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Switched {
                    side: BattleSide::Enemy,
                    party_index: 1,
                }
            )
        })
        .expect("trainer switch occurs");
    let damage_event = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Damage {
                    side: BattleSide::Player,
                    ..
                }
            )
        })
        .expect("player move damages the replacement");
    assert!(switch_event < damage_event);
}

#[test]
fn pursuit_intercepts_a_post_order_trainer_switch_without_changing_move_order() {
    let player = pokemon("UMBREON", 40, pokemon_type("DARK"), "PURSUIT");
    let enemy = pokemon("RATTATA", 100, pokemon_type("NORMAL"), "SPLASH");
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
    let mut rng = Random::new(1);
    let mut select_enemy_move = |_: &BattleCombatState, _: &mut dyn BattleRandomSource| Ok(0usize);
    let mut select_enemy_action = |_: &BattleCombatState,
                                   _: &mut dyn BattleRandomSource|
     -> Result<BattleAction, BattleTurnError> {
        Ok(BattleAction::TrainerSwitch {
            selected_move_slot: 0,
            party_index: 1,
        })
    };

    let outcome = resolve_battle_turn_with_enemy_ai_actions(
        battle_state(player, enemy, rng.seed()),
        BattleAction::Move { slot: 0 },
        &moves,
        &BTreeMap::new(),
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        &mut select_enemy_move,
        &mut select_enemy_action,
    )
    .expect("post-order trainer switch with Pursuit resolves");

    assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
    let pursuit_event = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::PursuitPower {
                    side: BattleSide::Player,
                    target: BattleSide::Enemy,
                    power: 80,
                    ..
                }
            )
        })
        .expect("Pursuit doubles against the outgoing enemy");
    let switch_event = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Switched {
                    side: BattleSide::Enemy,
                    party_index: 1,
                }
            )
        })
        .expect("trainer switch occurs");
    assert!(pursuit_event < switch_event);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, BattleEvent::PursuitPower { .. }))
            .count(),
        1,
    );
}

#[test]
fn link_double_switch_order_uses_clock_owned_coin_flip_without_speed_or_claw_checks() {
    let player = pokemon("SLOWPOKE", 20, pokemon_type("WATER"), "TACKLE");
    let enemy = pokemon("PIDGEY", 100, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 1);
    state.link_battle = true;
    state.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    let mut rng = Random::new(1);
    let mut expected_rng = rng;
    assert_eq!(expected_rng.battle_random_byte(), 64);

    let order = determine_turn_order(
        &state,
        &BattleTurnInput {
            player: BattleAction::Switch { party_index: 1 },
            enemy: BattleAction::Switch { party_index: 1 },
        },
        &BTreeMap::new(),
        &BTreeMap::new(),
        &move_priorities(),
        &stat_multipliers(),
        &mut rng,
    )
    .expect("link double-switch order resolves");

    assert_eq!(order, vec![BattleSide::Enemy, BattleSide::Player]);
    assert_eq!(rng.seed(), expected_rng.seed());
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
        item_id: "EMBER_ORB".to_string(),
    };
    let json = serde_json::to_string(&action).expect("serialize action");

    assert_eq!(json, r#"{"item":{"item_id":"EMBER_ORB"}}"#);
    assert_eq!(
        serde_json::from_str::<BattleAction>(&json).expect("deserialize action"),
        action
    );

    let party_action = BattleAction::PartyItem {
        item_id: "MAX_ETHER".to_string(),
        party_index: 5,
        move_slot: Some(3),
    };
    let json = serde_json::to_string(&party_action).expect("serialize targeted item action");
    assert_eq!(
        json,
        r#"{"party_item":{"item_id":"MAX_ETHER","party_index":5,"move_slot":3}}"#
    );
    assert_eq!(
        serde_json::from_str::<BattleAction>(&json).expect("deserialize targeted item action"),
        party_action
    );
    let ball_action = BattleAction::Ball {
        item_id: "HEAVY_BALL".to_string(),
    };
    let json = serde_json::to_string(&ball_action).expect("serialize Ball action");
    assert_eq!(json, r#"{"ball":{"item_id":"HEAVY_BALL"}}"#);
    assert_eq!(
        serde_json::from_str::<BattleAction>(&json).expect("deserialize Ball action"),
        ball_action
    );
    assert!(
        serde_json::from_str::<BattleAction>(
            r#"{"party_item":{"item_id":"MAX_ETHER","party_index":6,"move_slot":0}}"#
        )
        .expect_err("party index six rejects")
        .to_string()
        .contains("outside party range")
    );
    assert!(
        serde_json::from_str::<BattleAction>(
            r#"{"party_item":{"item_id":"MAX_ETHER","party_index":0,"move_slot":4}}"#
        )
        .expect_err("move slot four rejects")
        .to_string()
        .contains("outside Crystal move range")
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
fn berserk_gene_activates_at_the_next_turn_start_with_exact_held_effect() {
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

    assert_eq!(outcome.state.player.item.as_deref(), Some("BERSERK_GENE"));
    assert_eq!(outcome.state.player.confusion_turns, 0);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemActivated { item_id, .. } if item_id == "BERSERK_GENE"
    )));

    let mut next_turn_state = outcome.state;
    let mut start_events = Vec::new();
    apply_berserk_gene_start_of_turn(
        &mut next_turn_state,
        BattleSide::Player,
        &items,
        &mut start_events,
    )
    .expect("Berserk Gene next-turn boundary resolves");
    assert_eq!(next_turn_state.player.item, None);
    assert_eq!(
        next_turn_state
            .player
            .stat_boosts
            .get(&Stat::Attack)
            .copied(),
        Some(2)
    );
    assert_eq!(next_turn_state.player.confusion_turns, 256);
    assert!(start_events.contains(&BattleEvent::HeldItemActivated {
        side: BattleSide::Player,
        item_id: "BERSERK_GENE".to_string(),
        held_effect: "HELD_ATTACK_UP".to_string(),
    }));
    assert!(start_events.contains(&BattleEvent::StatStageChanged {
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
fn party_revive_item_is_one_turn_action_and_player_residual_precedes_enemy_response() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.hp = 160;
    player.max_hp = 160;
    player.status = Some("POISON".to_string());
    let mut fainted = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE");
    fainted.hp = 0;
    fainted.max_hp = 80;
    let enemy = pokemon("SENTRET", 1, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player.clone(), enemy, 1);
    state.player_party = vec![player, fainted];
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut revive = battle_item("REVIVE", 0, true);
    revive.effect = "REVIVE".to_string();
    revive.revive_hp_percent = Some(50);
    let items = BTreeMap::from([("REVIVE".to_string(), revive)]);
    let mut rng = Random::new(1);

    let outcome = resolve_battle_turn_with_items(
        state,
        BattleTurnInput {
            player: BattleAction::PartyItem {
                item_id: "REVIVE".to_string(),
                party_index: 1,
                move_slot: None,
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
    .expect("party Revive resolves as one turn");

    assert_eq!(outcome.state.player_party[1].hp, 40);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::BattlePartyItemEffect {
            side: BattleSide::Player,
            party_index: 1,
            move_slot: None,
            outcome,
        } if outcome.item_id == "REVIVE" && outcome.hp_before == 0 && outcome.hp_after == 40
    )));
    let item_index = outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::BattlePartyItemEffect { .. }))
        .expect("party item event");
    let residual_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::ResidualStatusDamage {
                    side: BattleSide::Player,
                    status,
                    ..
                } if status == "POISON"
            )
        })
        .expect("player poison residual");
    let enemy_move_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    side: BattleSide::Enemy,
                    ..
                }
            )
        })
        .expect("enemy response");
    assert!(item_index < residual_index && residual_index < enemy_move_index);
}

#[test]
fn active_party_pp_item_updates_loaded_pp_but_pp_up_remains_party_only() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.moves[0].current_pp = 1;
    let enemy = pokemon("SENTRET", 1, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut ether = battle_item("ETHER", 0, true);
    ether.effect = "RESTORE_PP".to_string();
    ether.pp_restore_scope = Some("MOVE".to_string());
    ether.pp_restore_points = Some(10);
    let mut pp_up = battle_item("PP_UP", 0, true);
    pp_up.effect = "PP_UP".to_string();
    pp_up.pp_up_stages = Some(1);

    let mut ether_rng = Random::new(1);
    let ether_outcome = resolve_battle_turn_with_items(
        battle_state(player.clone(), enemy.clone(), 1),
        BattleTurnInput {
            player: BattleAction::PartyItem {
                item_id: "ETHER".to_string(),
                party_index: 0,
                move_slot: Some(0),
            },
            enemy: BattleAction::Move { slot: 0 },
        },
        &moves,
        &BTreeMap::from([("ETHER".to_string(), ether)]),
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut ether_rng,
    )
    .expect("active Ether resolves");
    assert_eq!(ether_outcome.state.player.moves[0].current_pp, 11);
    assert_eq!(ether_outcome.state.player_party[0].moves[0].current_pp, 11);

    let mut pp_up_rng = Random::new(1);
    let pp_up_outcome = resolve_battle_turn_with_items(
        battle_state(player, enemy, 1),
        BattleTurnInput {
            player: BattleAction::PartyItem {
                item_id: "PP_UP".to_string(),
                party_index: 0,
                move_slot: Some(0),
            },
            enemy: BattleAction::Move { slot: 0 },
        },
        &moves,
        &BTreeMap::from([("PP_UP".to_string(), pp_up)]),
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut pp_up_rng,
    )
    .expect("active PP Up resolves");
    assert_eq!(pp_up_outcome.state.player.moves[0].pp_ups, 0);
    assert_eq!(pp_up_outcome.state.player_party[0].moves[0].pp_ups, 1);
}

#[test]
fn failed_ball_is_one_player_action_with_residual_before_enemy_response() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.hp = 160;
    player.max_hp = 160;
    player.status = Some("POISON".to_string());
    let enemy = pokemon("SENTRET", 1, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 1);
    state.player_bide_turns = 2;
    state.player_bide_damage = 40;
    state.player_fury_cutter_chain = 3;
    state.player_protect_counter = 2;
    state.player_rage_active = true;
    state.player_rage_counter = 2;
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("POKE_BALL".to_string(), battle_item("POKE_BALL", 0, true))]);
    let mut rng = Random::new(1);
    let mut execute_ball = |_: &mut BattleCombatState,
                            item_id: &str,
                            _: &mut dyn BattleRandomSource,
                            events: &mut Vec<BattleEvent>| {
        events.push(BattleEvent::BallThrown {
            side: BattleSide::Player,
            outcome: CaptureOutcome {
                caught: false,
                blocked: false,
                storage_full: false,
                wobble_count: 1,
                animation_shakes: 1,
                final_catch_rate: 80,
                ball_id: Some(item_id.to_string()),
            },
        });
        Ok(())
    };

    let outcome = resolve_battle_turn_with_ball_action_and_enemy_ai_actions(
        state,
        BattleAction::Ball {
            item_id: "POKE_BALL".to_string(),
        },
        BattleAction::Move { slot: 0 },
        &moves,
        &items,
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        true,
        None,
        &mut execute_ball,
    )
    .expect("failed Ball resolves as one turn");

    let ball_index = outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::BallThrown { .. }))
        .expect("Ball event");
    let residual_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::ResidualStatusDamage {
                    side: BattleSide::Player,
                    ..
                }
            )
        })
        .expect("player residual");
    let enemy_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    side: BattleSide::Enemy,
                    ..
                }
            )
        })
        .expect("enemy response");
    assert!(ball_index < residual_index && residual_index < enemy_index);
    assert_eq!(outcome.state.player_bide_turns, 0);
    assert_eq!(outcome.state.player_bide_damage, 0);
    assert_eq!(outcome.state.player_fury_cutter_chain, 0);
    assert_eq!(outcome.state.player_protect_counter, 0);
    assert!(!outcome.state.player_rage_active);
    assert_eq!(outcome.state.player_rage_counter, 0);
}

#[test]
fn caught_ball_ends_action_phase_before_residual_and_enemy_response() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.status = Some("POISON".to_string());
    let enemy = pokemon("SENTRET", 1, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("POKE_BALL".to_string(), battle_item("POKE_BALL", 0, true))]);
    let mut rng = Random::new(1);
    let mut execute_ball = |_: &mut BattleCombatState,
                            item_id: &str,
                            _: &mut dyn BattleRandomSource,
                            events: &mut Vec<BattleEvent>| {
        events.push(BattleEvent::BallThrown {
            side: BattleSide::Player,
            outcome: CaptureOutcome {
                caught: true,
                blocked: false,
                storage_full: false,
                wobble_count: 3,
                animation_shakes: 4,
                final_catch_rate: 255,
                ball_id: Some(item_id.to_string()),
            },
        });
        Ok(())
    };

    let outcome = resolve_battle_turn_with_ball_action_and_enemy_ai_actions(
        battle_state(player, enemy, 1),
        BattleAction::Ball {
            item_id: "POKE_BALL".to_string(),
        },
        BattleAction::Move { slot: 0 },
        &moves,
        &items,
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        true,
        None,
        &mut execute_ball,
    )
    .expect("caught Ball resolves");

    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::BallThrown { outcome, .. } if outcome.caught
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ResidualStatusDamage { .. }
            | BattleEvent::MoveUsed {
                side: BattleSide::Enemy,
                ..
            }
    )));
}

#[test]
fn wild_enemy_flee_tables_gates_and_rng_reads_match_try_enemy_flee() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    let state_for = |species: &str| {
        battle_state(
            player.clone(),
            pokemon(species, 30, pokemon_type("NORMAL"), "TACKLE"),
            1,
        )
    };

    let mut always_rng = ScriptedBattleRandom::new(vec![]);
    assert!(wild_enemy_flees(&state_for("RAIKOU"), &mut always_rng));
    assert_eq!(always_rng.calls, 0);

    for (species, roll, expected) in [
        ("CUBONE", 127, true),
        ("CUBONE", 128, false),
        ("MAGNEMITE", 25, true),
        ("MAGNEMITE", 26, false),
        ("RATTATA", 0, false),
    ] {
        let mut rng = ScriptedBattleRandom::new(vec![roll]);
        assert_eq!(wild_enemy_flees(&state_for(species), &mut rng), expected);
        assert_eq!(rng.calls, 1, "{species} roll {roll}");
    }

    let mut trapped_player = state_for("RAIKOU");
    trapped_player.player_escape_trap = Some(BattleEscapeTrapState {
        source: BattleSide::Enemy,
        move_name: "MEAN_LOOK".to_string(),
    });
    let mut trapped_enemy = state_for("RAIKOU");
    trapped_enemy.enemy_trap = Some(BattleTrapState {
        source: BattleSide::Player,
        move_name: "WRAP".to_string(),
        turns_remaining: 2,
    });
    let mut sleeping_enemy = state_for("RAIKOU");
    sleeping_enemy.enemy.status = Some("SLEEP".to_string());
    for state in [trapped_player, trapped_enemy, sleeping_enemy] {
        let mut rng = ScriptedBattleRandom::new(vec![]);
        assert!(!wild_enemy_flees(&state, &mut rng));
        assert_eq!(rng.calls, 0);
    }

    let mut protected_battle = state_for("RAIKOU");
    protected_battle.force_switch_blocked = true;
    let mut protected_rng = ScriptedBattleRandom::new(vec![]);
    assert!(wild_enemy_flees(&protected_battle, &mut protected_rng));
    assert_eq!(protected_rng.calls, 0);
}

#[test]
fn wild_enemy_move_selection_rejection_samples_original_slots_and_zero_read_shortcuts() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "MOVE_0");
    enemy.moves = [("MOVE_0", 0), ("MOVE_1", 5), ("MOVE_2", 0), ("MOVE_3", 5)]
        .into_iter()
        .map(|(name, current_pp)| LearnedMove {
            name: name.to_string(),
            current_pp,
            pp_ups: 0,
        })
        .collect();
    let mut state = battle_state(player, enemy, 1);

    let mut rejection_rng = ScriptedBattleRandom::new(vec![0, 2, 3]);
    assert_eq!(select_wild_enemy_move_slot(&state, &mut rejection_rng), 3);
    assert_eq!(rejection_rng.calls, 3);

    state.enemy_disable = Some(BattleDisableState {
        move_name: "MOVE_3".to_string(),
        turns_remaining: 2,
    });
    let mut disabled_rng = ScriptedBattleRandom::new(vec![3, 1]);
    assert_eq!(select_wild_enemy_move_slot(&state, &mut disabled_rng), 1);
    assert_eq!(disabled_rng.calls, 2);

    for learned in &mut state.enemy.moves {
        learned.current_pp = 0;
    }
    state.enemy_disable = None;
    let mut struggle_rng = ScriptedBattleRandom::new(vec![]);
    assert_eq!(select_wild_enemy_move_slot(&state, &mut struggle_rng), 0);
    assert_eq!(struggle_rng.calls, 0);

    state.enemy.moves[2].current_pp = 5;
    state.enemy_charging_move = Some("MOVE_2".to_string());
    let mut committed_rng = ScriptedBattleRandom::new(vec![]);
    assert_eq!(select_wild_enemy_move_slot(&state, &mut committed_rng), 2);
    assert_eq!(committed_rng.calls, 0);
}

#[test]
fn failed_ball_player_residual_precedes_wild_enemy_flee() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.hp = 160;
    player.max_hp = 160;
    player.status = Some("POISON".to_string());
    let enemy = pokemon("CUBONE", 30, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([("POKE_BALL".to_string(), battle_item("POKE_BALL", 0, true))]);
    let mut rng = ScriptedBattleRandom::new(vec![0]);
    let mut execute_ball = |_: &mut BattleCombatState,
                            item_id: &str,
                            _: &mut dyn BattleRandomSource,
                            events: &mut Vec<BattleEvent>| {
        events.push(BattleEvent::BallThrown {
            side: BattleSide::Player,
            outcome: CaptureOutcome {
                caught: false,
                blocked: false,
                storage_full: false,
                wobble_count: 0,
                animation_shakes: 0,
                final_catch_rate: 1,
                ball_id: Some(item_id.to_string()),
            },
        });
        Ok(())
    };

    let outcome = resolve_battle_turn_with_ball_action_and_enemy_ai_actions(
        battle_state(player, enemy, 1),
        BattleAction::Ball {
            item_id: "POKE_BALL".to_string(),
        },
        BattleAction::Move { slot: 0 },
        &moves,
        &items,
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
        true,
        None,
        &mut execute_ball,
    )
    .expect("failed Ball reaches TryEnemyFlee");

    let residual_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::ResidualStatusDamage {
                    side: BattleSide::Player,
                    ..
                }
            )
        })
        .expect("player residual");
    let fled_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Fled {
                    side: BattleSide::Enemy
                }
            )
        })
        .expect("wild enemy fled");
    assert!(residual_index < fled_index);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Enemy,
            ..
        }
    )));
    assert_eq!(rng.calls, 1);
}

#[test]
fn faster_always_flee_wild_enemy_leaves_before_either_move_without_rng() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.speed = 1;
    let mut enemy = pokemon("RAIKOU", 30, pokemon_type("ELECTRIC"), "TACKLE");
    enemy.speed = 255;
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut rng = ScriptedBattleRandom::new(vec![]);

    let outcome = resolve_wild_battle_turn_with_items(
        battle_state(player, enemy, 1),
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
    .expect("faster roaming enemy reaches TryEnemyFlee first");

    assert_eq!(rng.calls, 0);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Fled {
            side: BattleSide::Enemy
        }
    )));
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::MoveUsed { .. }))
    );
}

#[test]
fn faster_often_flee_enemy_attacks_when_flee_roll_fails() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.speed = 1;
    let mut enemy = pokemon("CUBONE", 30, pokemon_type("GROUND"), "TACKLE");
    enemy.speed = 255;
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let mut rng = ScriptedBattleRandom::new(vec![128, 0, 0, 255, 0, 0, 255]);

    let outcome = resolve_wild_battle_turn_with_items(
        battle_state(player, enemy, 1),
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
    .expect("failed TryEnemyFlee proceeds to the enemy action");

    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Fled {
            side: BattleSide::Enemy
        }
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Enemy,
            ..
        }
    )));
    assert!(rng.calls > 1, "the failed flee byte must precede move RNG");
}

#[test]
fn enemy_ai_full_heal_preserves_confusion_and_nightmare_like_ai_heal_status() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
    enemy.status = Some("BURN".to_string());
    enemy.confusion_turns = 3;
    let mut state = battle_state(player, enemy, 1);
    state.enemy_nightmare_source = Some(BattleSide::Player);
    let items = BTreeMap::from([(
        "FULL_HEAL".to_string(),
        full_status_item("FULL_HEAL", false),
    )]);
    let mut events = Vec::new();

    execute_item(
        &mut state,
        BattleSide::Enemy,
        "FULL_HEAL",
        &items,
        &mut events,
    )
    .expect("enemy AI Full Heal resolves");

    assert_eq!(state.enemy.status, None);
    assert_eq!(state.enemy.confusion_turns, 3);
    assert_eq!(state.enemy_nightmare_source, Some(BattleSide::Player));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::BattleItemEffect {
            side: BattleSide::Enemy,
            outcome,
        } if outcome.confusion_turns_before == 3 && outcome.confusion_turns_after == 3
    )));
}

#[test]
fn enemy_ai_full_restore_explicitly_clears_confusion_but_not_nightmare() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
    enemy.hp = 10;
    enemy.status = Some("BURN".to_string());
    enemy.confusion_turns = 3;
    let mut state = battle_state(player, enemy, 1);
    state.enemy_nightmare_source = Some(BattleSide::Player);
    let items = BTreeMap::from([(
        "FULL_RESTORE".to_string(),
        full_status_item("FULL_RESTORE", true),
    )]);
    let mut events = Vec::new();

    execute_item(
        &mut state,
        BattleSide::Enemy,
        "FULL_RESTORE",
        &items,
        &mut events,
    )
    .expect("enemy AI Full Restore resolves");

    assert_eq!(state.enemy.hp, state.enemy.max_hp);
    assert_eq!(state.enemy.status, None);
    assert_eq!(state.enemy.confusion_turns, 0);
    assert_eq!(state.enemy_nightmare_source, Some(BattleSide::Player));
}

#[test]
fn enemy_ai_full_heal_retains_the_cached_burn_attack_penalty() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
    let mut enemy = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "TACKLE");
    enemy.status = Some("BURN".to_string());
    let mut healed_state = battle_state(player, enemy, 1);
    let mut burned_state = healed_state.clone();
    let items = BTreeMap::from([(
        "FULL_HEAL".to_string(),
        full_status_item("FULL_HEAL", false),
    )]);

    execute_item(
        &mut healed_state,
        BattleSide::Enemy,
        "FULL_HEAL",
        &items,
        &mut Vec::new(),
    )
    .expect("enemy AI Full Heal resolves");
    assert_eq!(healed_state.enemy.status, None);

    let tackle = move_data("TACKLE", pokemon_type("NORMAL"), 35, 100);
    let mut healed_events = Vec::new();
    let mut healed_rng = Random::new(81);
    apply_damage_hit(
        &mut healed_state,
        BattleSide::Enemy,
        "TACKLE",
        &tackle,
        1,
        false,
        false,
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &BTreeMap::new(),
        &mut healed_rng,
        &mut healed_events,
    )
    .expect("healed enemy damage resolves");
    let mut burned_events = Vec::new();
    let mut burned_rng = Random::new(81);
    apply_damage_hit(
        &mut burned_state,
        BattleSide::Enemy,
        "TACKLE",
        &tackle,
        1,
        false,
        false,
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &BTreeMap::new(),
        &mut burned_rng,
        &mut burned_events,
    )
    .expect("burned enemy damage resolves");

    let damage = |events: &[BattleEvent]| {
        events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage {
                    side: BattleSide::Enemy,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("enemy damage event")
    };
    assert_eq!(damage(&healed_events), damage(&burned_events));
}

#[test]
fn enemy_ai_full_restore_retains_the_cached_paralysis_speed_penalty() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
    let mut enemy = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "TACKLE");
    enemy.hp = enemy.max_hp.saturating_sub(1);
    enemy.status = Some("PARALYSIS".to_string());
    let mut healed_state = battle_state(player, enemy, 1);
    let paralyzed_state = healed_state.clone();
    let items = BTreeMap::from([(
        "FULL_RESTORE".to_string(),
        full_status_item("FULL_RESTORE", true),
    )]);

    execute_item(
        &mut healed_state,
        BattleSide::Enemy,
        "FULL_RESTORE",
        &items,
        &mut Vec::new(),
    )
    .expect("enemy AI Full Restore resolves");

    assert_eq!(healed_state.enemy.status, None);
    let cached_speed = battle_speed(&healed_state, BattleSide::Enemy, &stat_multipliers())
        .expect("healed enemy speed");
    assert_eq!(
        cached_speed,
        battle_speed(&paralyzed_state, BattleSide::Enemy, &stat_multipliers())
            .expect("paralyzed enemy speed")
    );

    apply_stat_stage_delta(
        &mut healed_state,
        BattleSide::Enemy,
        "AGILITY",
        Stat::Speed,
        1,
        &mut Vec::new(),
    )
    .expect("enemy Speed recalculation resolves");
    assert!(
        battle_speed(&healed_state, BattleSide::Enemy, &stat_multipliers())
            .expect("recalculated enemy speed")
            > cached_speed
    );
}

#[test]
fn switched_enemy_burn_and_paralysis_penalties_are_skipped_only_in_ordinary_trainer_battles() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
    let active_enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut switched_enemy = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "TACKLE");
    switched_enemy.status = Some("PARALYSIS".to_string());
    let base_state = BattleCombatState::new(player, active_enemy.clone())
        .with_parties(vec![], vec![active_enemy, switched_enemy]);

    let mut ordinary = base_state.clone();
    switch_battle_combat_pokemon(&mut ordinary, BattleSide::Enemy, 1)
        .expect("ordinary enemy switch resolves");
    assert_eq!(ordinary.enemy.status.as_deref(), Some("PARALYSIS"));
    assert!(!ordinary.enemy_paralysis_speed_penalty_active);
    assert_eq!(
        battle_speed(&ordinary, BattleSide::Enemy, &stat_multipliers())
            .expect("ordinary switched enemy speed"),
        ordinary.enemy.speed
    );

    let mut link = base_state.clone();
    link.link_battle = true;
    switch_battle_combat_pokemon(&mut link, BattleSide::Enemy, 1)
        .expect("link enemy switch resolves");
    assert!(link.enemy_paralysis_speed_penalty_active);
    assert_eq!(
        battle_speed(&link, BattleSide::Enemy, &stat_multipliers())
            .expect("link switched enemy speed"),
        (link.enemy.speed / 4).max(1)
    );

    let mut tower = base_state;
    tower.sleep_turn_mask = 0x03;
    switch_battle_combat_pokemon(&mut tower, BattleSide::Enemy, 1)
        .expect("Battle Tower enemy switch resolves");
    assert!(tower.enemy_paralysis_speed_penalty_active);
    assert_eq!(
        battle_speed(&tower, BattleSide::Enemy, &stat_multipliers())
            .expect("Battle Tower switched enemy speed"),
        (tower.enemy.speed / 4).max(1)
    );

    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
    let active_enemy = pokemon("PIDGEY", 40, pokemon_type("NORMAL"), "TACKLE");
    let mut burned_enemy = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "TACKLE");
    burned_enemy.status = Some("BURN".to_string());
    let burn_base = BattleCombatState::new(player, active_enemy.clone())
        .with_parties(vec![], vec![active_enemy, burned_enemy]);
    let mut ordinary_burn = burn_base.clone();
    switch_battle_combat_pokemon(&mut ordinary_burn, BattleSide::Enemy, 1)
        .expect("ordinary burned enemy switch resolves");
    assert!(!ordinary_burn.enemy_burn_attack_penalty_active);
    let mut link_burn = burn_base;
    link_burn.link_battle = true;
    switch_battle_combat_pokemon(&mut link_burn, BattleSide::Enemy, 1)
        .expect("link burned enemy switch resolves");
    assert!(link_burn.enemy_burn_attack_penalty_active);
}

#[test]
fn player_full_heal_clears_nightmare_through_heal_status() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    player.status = Some("SLEEP".to_string());
    player.sleep_turns = 3;
    let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 1);
    state.player_nightmare_source = Some(BattleSide::Enemy);
    let items = BTreeMap::from([(
        "FULL_HEAL".to_string(),
        full_status_item("FULL_HEAL", false),
    )]);
    let mut events = Vec::new();

    execute_item(
        &mut state,
        BattleSide::Player,
        "FULL_HEAL",
        &items,
        &mut events,
    )
    .expect("player Full Heal resolves");

    assert_eq!(state.player.status, None);
    assert_eq!(state.player.sleep_turns, 0);
    assert_eq!(state.player_nightmare_source, None);
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
fn unchecked_spikes_zero_hp_battler_can_act_until_the_next_faint_check() {
    let mut player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "QUICK_ATTACK");
    player.hp = 0;
    player.speed = 200;
    let mut enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "TACKLE");
    enemy.speed = 1;
    let moves = [
        (
            "QUICK_ATTACK".to_string(),
            move_data_with_effect(
                "QUICK_ATTACK",
                pokemon_type("NORMAL"),
                40,
                100,
                "PRIORITY_HIT",
            ),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
    ]
    .into_iter()
    .collect();
    let mut state = battle_state(player, enemy, 1);
    state.player_spikes_zero_hp_unchecked = true;
    let mut rng = Random::new(1);

    let outcome = resolve_battle_turn_with_items(
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
        &mut rng,
    )
    .expect("unchecked Spikes KO can enter one command boundary");

    assert!(
        outcome.events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    side: BattleSide::Player,
                    move_name,
                } if move_name == "QUICK_ATTACK"
            )
        }),
        "events={:?} order={:?}",
        outcome.events,
        outcome.order
    );
    assert!(outcome.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::Fainted {
                side: BattleSide::Player
            }
        )
    }));
    assert!(!outcome.events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::MoveUsed {
                side: BattleSide::Enemy,
                ..
            }
        )
    }));
    assert!(!outcome.state.player_spikes_zero_hp_unchecked);
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
        false,
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
fn rampage_duration_is_sampled_before_the_first_accuracy_check() {
    let player = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "THRASH");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let moves = BTreeMap::from([
        (
            "THRASH".to_string(),
            move_data_with_effect("THRASH", pokemon_type("NORMAL"), 90, 1, "RAMPAGE"),
        ),
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
    ]);
    let mut rng = ScriptedBattleRandom::new(vec![1, u8::MAX]);

    let outcome = resolve_battle_turn(
        battle_state(player, enemy, 0),
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
    .expect("initial missed rampage resolves");

    assert_eq!(outcome.state.player.rampage_turns, 2);
    assert!(outcome.events.contains(&BattleEvent::RampageStarted {
        side: BattleSide::Player,
        move_name: "THRASH".to_string(),
        turns_remaining: 2,
        roll: 1,
    }));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Missed {
            side: BattleSide::Player,
            move_name,
            roll: u8::MAX,
            ..
        } if move_name == "THRASH"
    )));
}

#[test]
fn rampage_forced_turn_uses_locked_move_without_extra_pp_and_processes_end_confusion() {
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
    assert!((1..=2).contains(&outcome.state.player.confusion_turns));
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
fn final_rampage_turn_applies_confusion_before_doturn_and_can_self_hit() {
    let mut player = pokemon("TAUROS", 90, pokemon_type("NORMAL"), "THRASH");
    player.rampage_turns = 1;
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let enemy_hp = enemy.hp;
    let mut state = battle_state(player, enemy, 0);
    state.player_last_move = Some("THRASH".to_string());
    let mut safeguarded_state = state.clone();
    safeguarded_state.player_safeguard_turns = 5;
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
    let mut rng = ScriptedBattleRandom::new(vec![0, 0, u8::MAX, 0, 0, u8::MAX, 0, 0]);

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
    .expect("final rampage turn resolves");

    assert_eq!(outcome.state.player.rampage_turns, 0);
    assert_eq!(outcome.state.player.confusion_turns, 1);
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ConfusionApplied {
            side: BattleSide::Player,
            turns: 2,
            ..
        }
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::ConfusionSelfDamage {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "THRASH"
    )));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            side: BattleSide::Player,
            move_name,
            ..
        } if move_name == "THRASH"
    )));

    let mut safeguarded_rng = ScriptedBattleRandom::new(vec![0, u8::MAX, u8::MAX]);
    let safeguarded = resolve_battle_turn(
        safeguarded_state,
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
        &mut safeguarded_rng,
    )
    .expect("Safeguard-protected final rampage turn resolves");
    assert_eq!(safeguarded.state.player.rampage_turns, 0);
    assert_eq!(safeguarded.state.player.confusion_turns, 0);
    assert!(
        !safeguarded
            .events
            .iter()
            .any(|event| matches!(event, BattleEvent::ConfusionApplied { .. }))
    );
    assert!(safeguarded.events.iter().any(|event| matches!(
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
        BattleEvent::LockOnApplied {
            side: BattleSide::Player,
            move_name,
            target: BattleSide::Enemy,
        } if move_name == "LOCK_ON"
    )));
}

#[test]
fn lock_on_consumes_sure_hit_without_accuracy_stage_table() {
    let player = pokemon("MAGNEMITE", 90, pokemon_type("ELECTRIC"), "TACKLE");
    let mut enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
    enemy.hp = 1;
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
            enemy: BattleAction::Move { slot: 0 },
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
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    state.player_airborne_move = Some("FLY".to_string());
    let moves = BTreeMap::from([
        (
            "FLY".to_string(),
            move_data_with_effect("FLY", pokemon_type("FLYING"), 70, 100, "FLY"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
    ]);
    let mut rng = Random::new(34);

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
    .expect("airborne forced switch override resolves");

    assert_eq!(outcome.state.player_airborne_move, None);
    assert_eq!(outcome.state.player.moves[0].current_pp, 4);
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
fn asm_fly_and_dig_hit_exceptions_are_exact() {
    let move_with = |name: &str, effect: &str| {
        move_data_with_effect(name, pokemon_type("NORMAL"), 40, 100, effect)
    };

    for name in ["GUST", "WHIRLWIND", "THUNDER", "TWISTER"] {
        assert!(move_hits_airborne_target(&move_with(name, name), "FLY"));
    }
    assert!(!move_hits_airborne_target(&move_with("FLY", "FLY"), "FLY"));

    assert!(move_hits_airborne_target(
        &move_with("EARTHQUAKE", "EARTHQUAKE"),
        "DIG"
    ));
    assert!(move_hits_airborne_target(
        &move_with("FISSURE", "OHKO"),
        "DIG"
    ));
    assert!(move_hits_airborne_target(
        &move_with("MAGNITUDE", "MAGNITUDE"),
        "DIG"
    ));
    assert!(!move_hits_airborne_target(
        &move_with("GUST", "GUST"),
        "DIG"
    ));
}

#[test]
fn earthquake_hits_and_doubles_power_against_dig_target() {
    let mut player = pokemon("DIGLETT", 40, pokemon_type("NORMAL"), "DIG");
    player.speed = 10;
    let mut enemy = pokemon("DONPHAN", 100, pokemon_type("GROUND"), "EARTHQUAKE");
    enemy.speed = 100;
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
    let mut player = pokemon("PIDGEOT", 40, pokemon_type("NORMAL"), "FLY");
    player.speed = 10;
    let mut enemy = pokemon("DONPHAN", 100, pokemon_type("GROUND"), "EARTHQUAKE");
    enemy.speed = 100;
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

    let mut state = battle_state(player, enemy, rng.seed());
    state.player_last_move = Some("TACKLE".to_string());
    state.player_last_counter_move = Some("TACKLE".to_string());
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
    .expect("solarbeam charge turn resolves");

    assert_eq!(
        outcome.state.player_charging_move,
        Some("SOLARBEAM".to_string())
    );
    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert_eq!(outcome.state.player.moves[0].current_pp, 4);
    assert_eq!(outcome.state.player_last_move.as_deref(), Some("TACKLE"));
    assert_eq!(
        outcome.state.player_last_counter_move.as_deref(),
        Some("TACKLE")
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
    state.player_last_move = Some("TACKLE".to_string());
    state.player_last_counter_move = Some("TACKLE".to_string());
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
    assert_eq!(outcome.state.player_last_move.as_deref(), Some("SOLARBEAM"));
    assert_eq!(
        outcome.state.player_last_counter_move.as_deref(),
        Some("SOLARBEAM")
    );
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
fn charged_move_lock_replaces_a_switch_request_before_turn_order() {
    let mut player = pokemon("MEGANIUM", 20, pokemon_type("GRASS"), "SOLARBEAM");
    player.hp = 500;
    player.max_hp = 500;
    let enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
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
            move_data("TACKLE", pokemon_type("NORMAL"), 1, 100),
        ),
    ]);
    let mut rng = Random::new(36);

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
    .expect("charged move replaces switch request");

    assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
    assert_eq!(outcome.state.player_party_index, 0);
    assert_eq!(outcome.state.player_charging_move, None);
    assert!(outcome.state.enemy.hp < enemy_hp);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::Switched {
            side: BattleSide::Player,
            ..
        }
    )));
}

#[test]
fn recharge_lock_ignores_a_switch_request_and_its_target() {
    let player = pokemon("DRAGONITE", 20, pokemon_type("NORMAL"), "TACKLE");
    let enemy = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "SPLASH");
    let mut state = battle_state(player, enemy, 0);
    state.player_recharge_move = Some("HYPER_BEAM".to_string());
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    let mut rng = Random::new(36);

    let outcome = resolve_battle_turn(
        state,
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
    .expect("recharge replaces switch request before target validation");

    assert_eq!(outcome.order, vec![BattleSide::Enemy, BattleSide::Player]);
    assert_eq!(outcome.state.player_party_index, 0);
    assert_eq!(outcome.state.player_recharge_move, None);
    assert!(outcome.events.contains(&BattleEvent::RechargeTurn {
        side: BattleSide::Player,
        move_name: "HYPER_BEAM".to_string(),
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
fn bide_and_encore_do_not_replace_a_player_switch_request() {
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
    )]);
    for locked_move in ["BIDE", "TACKLE"] {
        let player = pokemon("RATTATA", 20, pokemon_type("NORMAL"), locked_move);
        let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "SPLASH");
        let mut state = battle_state(player, enemy, 0);
        if locked_move == "BIDE" {
            state.player_last_move = Some("BIDE".to_string());
            state.player_bide_turns = 2;
            state.player_bide_damage = 12;
        } else {
            state.player_encore = Some(BattleEncoreState {
                move_name: "TACKLE".to_string(),
                turns_remaining: 2,
            });
        }
        let mut rng = Random::new(37);

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
        .expect("Bide and Encore permit player switching");

        assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
        assert_eq!(outcome.state.player_party_index, 1, "{locked_move}");
        assert_eq!(outcome.state.player_bide_turns, 0, "{locked_move}");
        assert_eq!(outcome.state.player_encore, None, "{locked_move}");
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            BattleEvent::Switched {
                side: BattleSide::Player,
                party_index: 1,
                ..
            }
        )));
    }
}

#[test]
fn enemy_bide_replaces_a_trainer_switch_request_before_turn_order() {
    let player = pokemon("RATTATA", 90, pokemon_type("NORMAL"), "TACKLE");
    let enemy = pokemon("PIDGEY", 20, pokemon_type("NORMAL"), "BIDE");
    let mut state = battle_state(player, enemy, 0);
    state.enemy_last_move = Some("BIDE".to_string());
    state.enemy_bide_turns = 2;
    state.enemy_bide_damage = 0;
    let moves = BTreeMap::from([
        (
            "TACKLE".to_string(),
            move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
        ),
        (
            "BIDE".to_string(),
            move_data_with_effect("BIDE", pokemon_type("NORMAL"), 0, 100, "BIDE"),
        ),
    ]);
    let mut rng = Random::new(38);

    let outcome = resolve_battle_turn(
        state,
        BattleTurnInput {
            player: BattleAction::Move { slot: 0 },
            enemy: BattleAction::TrainerSwitch {
                selected_move_slot: 0,
                party_index: 1,
            },
        },
        &moves,
        &move_priorities(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
    )
    .expect("enemy Bide replaces trainer switch request");

    assert_eq!(outcome.order, vec![BattleSide::Player, BattleSide::Enemy]);
    assert_eq!(outcome.state.enemy_party_index, 0);
    assert_eq!(outcome.state.enemy_bide_turns, 1);
    assert!(outcome.state.enemy_bide_damage > 0);
    assert!(outcome.events.contains(&BattleEvent::BideStoring {
        side: BattleSide::Enemy,
        move_name: "BIDE".to_string(),
        turns_remaining: 1,
        stored_damage: outcome.state.enemy_bide_damage,
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
fn hyper_beam_starts_recharge_before_target_ko() {
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
    .expect("hyper beam ko turn resolves");

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
fn hyper_beam_starts_recharge_after_hitting_a_substitute() {
    let player = pokemon("DRAGONITE", 90, pokemon_type("NORMAL"), "HYPER_BEAM");
    let enemy = pokemon("RATTATA", 40, pokemon_type("NORMAL"), "TACKLE");
    let enemy_hp = enemy.hp;
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
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_substitute_hp = 20;

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
    .expect("Hyper Beam Substitute hit resolves");

    assert_eq!(outcome.state.enemy.hp, enemy_hp);
    assert_eq!(outcome.state.enemy_substitute_hp, 0);
    assert_eq!(
        outcome.state.player_recharge_move.as_deref(),
        Some("HYPER_BEAM")
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
fn partial_trap_blocks_switching_without_a_mean_look_record() {
    let player = pokemon("ONIX", 90, pokemon_type("ROCK"), "TACKLE");
    let enemy = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
    let mut state = battle_state(player, enemy, 0);
    state.enemy_trap = Some(BattleTrapState {
        source: BattleSide::Player,
        move_name: "BIND".to_string(),
        turns_remaining: 2,
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
    .expect("partial-trap switch turn resolves");

    assert_eq!(outcome.state.enemy_escape_trap, None);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::SwitchBlocked {
            side: BattleSide::Enemy,
            party_index: 1,
            source: BattleSide::Player,
            move_name,
        } if move_name == "BIND"
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
fn partial_trap_and_protected_battle_type_block_wild_run_before_smoke_ball() {
    let mut player = pokemon("RATTATA", 999, pokemon_type("NORMAL"), "TACKLE");
    player.item = Some("SMOKE_BALL".to_string());
    let enemy = pokemon("GASTLY", 10, pokemon_type("GHOST"), "TACKLE");
    let moves = BTreeMap::from([(
        "TACKLE".to_string(),
        move_data("TACKLE", pokemon_type("NORMAL"), 35, 100),
    )]);
    let items = BTreeMap::from([(
        "SMOKE_BALL".to_string(),
        held_status_item("SMOKE_BALL", "HELD_ESCAPE"),
    )]);
    let rules = BattleEscapeRules {
        player_speed_multiplier: 32,
        enemy_speed_divisor: 4,
        failed_attempt_bonus: 30,
        rng_roll_values: 256,
    };

    let mut trapped = battle_state(player.clone(), enemy.clone(), 0);
    trapped.player_trap = Some(BattleTrapState {
        source: BattleSide::Enemy,
        move_name: "WRAP".to_string(),
        turns_remaining: 2,
    });
    let mut trapped_rng = Random::new(21);
    let trapped_outcome = resolve_wild_battle_turn_with_items(
        trapped,
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
        &rules,
        0,
        &mut trapped_rng,
    )
    .expect("partial trap blocks run");
    assert!(trapped_outcome.events.contains(&BattleEvent::RunBlocked {
        side: BattleSide::Player,
        source: BattleSide::Enemy,
        move_name: "WRAP".to_string(),
    }));
    assert!(!trapped_outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemEscape { .. } | BattleEvent::RunAttempt { .. }
    )));

    let mut protected = battle_state(player, enemy, 0);
    protected.force_switch_blocked = true;
    let mut protected_rng = Random::new(21);
    let protected_outcome = resolve_wild_battle_turn_with_items(
        protected,
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
        &rules,
        0,
        &mut protected_rng,
    )
    .expect("protected battle type blocks run");
    assert!(
        protected_outcome
            .events
            .contains(&BattleEvent::RunPrevented {
                side: BattleSide::Player,
            })
    );
    assert!(!protected_outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::HeldItemEscape { .. } | BattleEvent::RunAttempt { .. }
    )));
}

#[test]
fn force_switch_move_ends_wild_battle() {
    let player = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "WHIRLWIND");
    let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
    let moves = BTreeMap::from([
        (
            "WHIRLWIND".to_string(),
            move_data_with_effect("WHIRLWIND", pokemon_type("NORMAL"), 0, 100, "FORCE_SWITCH"),
        ),
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn wild_enemy_force_switch_response_after_item_ends_battle() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "SPLASH");
    let enemy = pokemon("PIDGEY", 90, pokemon_type("NORMAL"), "WHIRLWIND");
    let moves = BTreeMap::from([
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
        (
            "WHIRLWIND".to_string(),
            move_data_with_effect("WHIRLWIND", pokemon_type("NORMAL"), 0, 100, "FORCE_SWITCH"),
        ),
    ]);
    let mut rng = Random::new(31);

    let outcome = resolve_battle_enemy_action_with_items(
        battle_state(player, enemy, rng.seed()),
        BattleAction::Move { slot: 0 },
        true,
        &moves,
        &BTreeMap::new(),
        &stat_multipliers(),
        &type_categories(),
        &type_effectiveness_table(),
        &weather_modifiers(),
        &mut rng,
    )
    .expect("wild enemy item response resolves");

    assert!(outcome.events.contains(&BattleEvent::ForceSwitchApplied {
        side: BattleSide::Enemy,
        move_name: "WHIRLWIND".to_string(),
        target: BattleSide::Player,
    }));
    assert!(outcome.events.contains(&BattleEvent::Fled {
        side: BattleSide::Player,
    }));
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

    let mut state = battle_state(player.clone(), enemy.clone(), rng.seed())
        .with_parties(vec![player], vec![enemy]);
    state.player_x_accuracy = true;
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
fn wild_enemy_teleport_always_succeeds_regardless_of_level_difference() {
    let mut player = pokemon("SNORLAX", 30, pokemon_type("NORMAL"), "SPLASH");
    player.level = 100;
    let enemy = pokemon("ABRA", 90, pokemon_type("PSYCHIC_TYPE"), "TELEPORT");
    let moves = BTreeMap::from([
        (
            "SPLASH".to_string(),
            move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
        ),
        (
            "TELEPORT".to_string(),
            move_data_with_effect("TELEPORT", pokemon_type("PSYCHIC_TYPE"), 0, 100, "TELEPORT"),
        ),
    ]);
    // If the player's Teleport odds were (incorrectly) reused for the wild
    // enemy, this accepted roll is below level 20 / 4 and would fail.
    let mut rng = ScriptedBattleRandom::new(vec![0]);

    let outcome = resolve_wild_battle_turn_with_items(
        battle_state(player, enemy, 0),
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
    .expect("wild enemy Teleport resolves");

    assert!(outcome.events.contains(&BattleEvent::Fled {
        side: BattleSide::Enemy,
    }));
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::TeleportFailed {
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
fn teleport_is_blocked_when_opponent_cannot_escape() {
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
    state.enemy_escape_trap = Some(BattleEscapeTrapState {
        source: BattleSide::Player,
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
    let player = pokemon("PINECO", 90, pokemon_type("BUG"), "SPLASH");
    let enemy = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
fn enemy_ai_spikes_ko_is_checked_after_the_players_selected_move() {
    let player = pokemon("RATTATA", 30, pokemon_type("NORMAL"), "TACKLE");
    let enemy = pokemon("PIDGEY", 30, pokemon_type("NORMAL"), "SPLASH");
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
    let mut rng = Random::new(35);
    let mut state = battle_state(player, enemy, rng.seed());
    state.enemy_spikes = true;
    state.enemy_party[1].max_hp = 8;
    state.enemy_party[1].hp = 1;

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
    .expect("enemy AI Spikes KO resolves through the post-player faint check");

    let spikes_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::SpikesDamage {
                    side: BattleSide::Enemy,
                    hp_after: 0,
                    ..
                }
            )
        })
        .expect("lethal enemy Spikes damage");
    let player_move_index = outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::MoveUsed { side: BattleSide::Player, move_name } if move_name == "TACKLE"))
        .expect("player selected move still executes");
    let faint_index = outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Fainted {
                    side: BattleSide::Enemy
                }
            )
        })
        .expect("enemy faint check after player action");
    assert!(spikes_index < player_move_index);
    assert!(player_move_index < faint_index);
}

#[test]
fn flying_pokemon_switches_into_spikes_without_damage() {
    let player = pokemon("PINECO", 90, pokemon_type("BUG"), "SPLASH");
    let enemy = pokemon("PIDGEY", 30, pokemon_type("FLYING"), "TACKLE");
    let enemy_hp = enemy.hp;
    let moves = BTreeMap::from([(
        "SPLASH".to_string(),
        move_data_with_effect("SPLASH", pokemon_type("NORMAL"), 0, 100, "SPLASH"),
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
    .expect_err("generic battle run needs explicit wild escape rules");

    assert_eq!(
        error,
        BattleTurnError::RunNotAllowed {
            side: BattleSide::Player
        }
    );
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

    let error = resolve_battle_turn_with_items(
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
    .expect_err("generic enemy battle run needs explicit wild escape rules");

    assert_eq!(
        error,
        BattleTurnError::RunNotAllowed {
            side: BattleSide::Enemy
        }
    );
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
    let mut rng = Random::new(99);

    let escape = resolve_wild_battle_run(&state, &rules, 3, &stat_multipliers(), &mut rng)
        .expect("wild battle run resolves in core battle code");

    assert!(escape.escaped);
    assert_eq!(escape.roll, None);
    assert_eq!(escape.attempts_before, 3);
    assert_eq!(escape.attempts_after, 4);
}

#[test]
fn core_wild_battle_run_compares_loaded_battle_speeds() {
    let mut player = pokemon("RATTATA", 10, pokemon_type("NORMAL"), "TACKLE");
    let mut enemy = pokemon("PIDGEY", 120, pokemon_type("NORMAL"), "TACKLE");
    // The cartridge receives wBattleMonSpeed/wEnemyMonSpeed here. Keep the
    // species-derived ordering opposite so a recalculation is observable.
    player.speed = 100;
    enemy.speed = 20;
    let state = battle_state(player, enemy, 99);
    let rules = BattleEscapeRules {
        player_speed_multiplier: 32,
        enemy_speed_divisor: 4,
        failed_attempt_bonus: 30,
        rng_roll_values: 256,
    };
    let mut rng = Random::new(99);

    let escape = resolve_wild_battle_run(&state, &rules, 0, &stat_multipliers(), &mut rng)
        .expect("loaded battle speeds resolve");

    assert!(escape.escaped);
    assert_eq!(escape.roll, None);
    assert_eq!(escape.attempts_after, 1);
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
    let mut rng = Random::new(1);

    let error = resolve_wild_battle_run(&state, &rules, 0, &stat_multipliers(), &mut rng)
        .expect_err("missing speed stage must reject");

    assert_eq!(
        error,
        BattleTurnError::MissingStatStage {
            side: BattleSide::Player,
            stat: Stat::Speed,
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
