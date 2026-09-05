use super::turn::{BattleCombatState, battle_pokemon_gender};
use super::{
    damage::{
        DamageCalculationError, DamageContext, TypeCategories, TypeEffectivenessTable,
        WeatherModifiers, calculate_damage, is_physical_type,
    },
    stats::BattleStatMultiplierTables,
};
use crate::models::{Move, Pokemon, Stat};
use crate::random::BattleRandomSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainerAiTypeMatchup {
    Immune,
    NotVeryEffective,
    Neutral,
    SuperEffective,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainerAiDamageEvaluation<'a> {
    pub effect: &'a str,
    pub power: u16,
    pub damage: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainerSmartMove<'a> {
    pub move_id: &'a str,
    pub effect: &'a str,
    pub power: u16,
    pub accuracy: u8,
    pub matchup: TrainerAiTypeMatchup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainerSmartPlayerMove<'a> {
    pub move_id: &'a str,
    pub effect: &'a str,
    pub power: u16,
    pub physical: bool,
    pub matchup_against_enemy: TrainerAiTypeMatchup,
    pub matchup_against_player: TrainerAiTypeMatchup,
    pub current_pp: Option<u8>,
}

/// Source-ordered `AI_Smart` scoring pass. Handlers are intentionally kept
/// effect-local while the outer pass owns slot order and Random consumption.
pub fn apply_trainer_smart_scores(
    state: &BattleCombatState,
    scores: &mut [i16],
    moves: &[Option<TrainerSmartMove<'_>>],
    player_moves: &[TrainerSmartPlayerMove<'_>],
    player_type_matchups_against_enemy: &[TrainerAiTypeMatchup],
    enemy_move_matchups_against_enemy: &[TrainerAiTypeMatchup],
    priority_damage_by_slot: &[Option<u16>],
    rng: &mut dyn BattleRandomSource,
) {
    // HandleBerserkGene leaves hBattleTurn on the enemy before AIChooseMove.
    // A few Smart handlers mutate it without restoring it, which is observable
    // through CheckTypeMatchup's documented current-move-type bug.
    let mut battle_turn_enemy = true;
    for slot in 0..scores.len().min(moves.len()) {
        let Some(selected) = moves[slot] else {
            break;
        };
        if selected.effect == "LOCK_ON" {
            if smart_lock_on_apply(state, scores, moves, slot, rng) {
                battle_turn_enemy = true;
            }
            continue;
        }
        let current_matchup_against_enemy = if battle_turn_enemy {
            enemy_move_matchups_against_enemy
                .get(slot)
                .copied()
                .unwrap_or(TrainerAiTypeMatchup::Neutral)
        } else {
            // First-turn Conversion2 reads the byte before Moves as a type and
            // leaves hBattleTurn on the player. In the original ROM that value
            // has no matchup entries, so subsequent checks are neutral.
            TrainerAiTypeMatchup::Neutral
        };
        let current_matchup_against_player = if battle_turn_enemy {
            selected.matchup
        } else {
            TrainerAiTypeMatchup::Neutral
        };
        let switch_score = trainer_smart_switch_score(
            moves,
            player_moves,
            player_type_matchups_against_enemy.len(),
            current_matchup_against_enemy,
            current_matchup_against_player,
        );
        let delta = match selected.effect {
            "SLEEP" => smart_sleep_delta(moves, rng),
            "LEECH_HIT" => smart_leech_hit_delta(state, selected.matchup, rng),
            "SELFDESTRUCT" => smart_selfdestruct_delta(state, rng),
            "DREAM_EATER" => i16::from(rng.battle_random_byte() >= 25) * -3,
            "EVASION_UP" => smart_evasion_up_delta(state, rng),
            "ALWAYS_HIT" => smart_always_hit_delta(state, rng),
            "ACCURACY_DOWN" => smart_accuracy_down_delta(state, rng),
            "RESET_STATS" => smart_reset_stats_delta(state, rng),
            "BIDE" => {
                if state.enemy.hp == state.enemy.max_hp || rng.battle_random_byte() < 25 {
                    0
                } else {
                    1
                }
            }
            "HEAL" | "MORNING_SUN" | "SYNTHESIS" | "MOONLIGHT" => smart_heal_delta(state, rng),
            "TOXIC" | "LEECH_SEED" => {
                i16::from(!hp_above_half(state.player.hp, state.player.max_hp))
            }
            "LIGHT_SCREEN" | "REFLECT" => {
                if state.enemy.hp == state.enemy.max_hp || rng.battle_random_byte() < 20 {
                    0
                } else {
                    1
                }
            }
            "OHKO" => {
                if state.enemy.level < state.player.level {
                    10
                } else {
                    i16::from(!hp_above_half(state.player.hp, state.player.max_hp))
                }
            }
            "TRAP_TARGET" => smart_trap_target_delta(state, rng),
            "CONFUSE" => smart_confuse_delta(state, rng),
            "SP_DEF_UP_2" => smart_sp_def_up_2_delta(state, rng),
            "FLY" => {
                if state.player_airborne_move.is_some() && smart_enemy_faster(state) {
                    -3
                } else {
                    0
                }
            }
            "SUPER_FANG" => i16::from(!hp_above_quarter(state.player.hp, state.player.max_hp)),
            "PARALYZE" => smart_paralyze_delta(state, rng),
            "SPEED_DOWN_HIT" if selected.move_id == "ICY_WIND" => {
                if hp_above_quarter(state.enemy.hp, state.enemy.max_hp)
                    && state.player_turns_taken == 0
                    && !smart_enemy_faster(state)
                    && rng.battle_random_byte() >= 30
                {
                    -2
                } else {
                    0
                }
            }
            "SUBSTITUTE" => i16::from(!hp_above_half(state.enemy.hp, state.enemy.max_hp)) * 10,
            "HYPER_BEAM" => smart_hyper_beam_delta(state, rng),
            "RAGE" => smart_rage_delta(state, rng),
            "PAIN_SPLIT" => i16::from(u32::from(state.enemy.hp) * 2 > u32::from(state.player.hp)),
            "SNORE" | "SLEEP_TALK" => {
                if state.enemy.sleep_turns == 1 {
                    3
                } else {
                    -3
                }
            }
            "DEFROST_OPPONENT" => {
                if state.enemy.status.as_deref() == Some("FREEZE") {
                    -3
                } else {
                    0
                }
            }
            "DESTINY_BOND" | "REVERSAL" | "SKULL_BASH" => {
                i16::from(hp_above_quarter(state.enemy.hp, state.enemy.max_hp))
            }
            "HEAL_BELL" => smart_heal_bell_delta(state, rng),
            "THIEF" => 30,
            "NIGHTMARE" => {
                if rng.battle_random_byte() >= 128 {
                    -1
                } else {
                    0
                }
            }
            "FLAME_WHEEL" => {
                if state.enemy.status.as_deref() == Some("FREEZE") {
                    -5
                } else {
                    0
                }
            }
            "PROTECT" => smart_protect_delta(state, rng),
            "FORESIGHT" => smart_foresight_delta(state, rng),
            "SANDSTORM" => smart_sandstorm_delta(state, rng),
            "ENDURE" => smart_endure_delta(state, moves, rng),
            "FURY_CUTTER" => smart_fury_rollout_delta(state, true, rng),
            "ROLLOUT" => smart_fury_rollout_delta(state, false, rng),
            "SWAGGER" | "ATTRACT" => {
                if state.player_turns_taken == 0 {
                    if rng.battle_random_byte() < 200 {
                        -1
                    } else {
                        0
                    }
                } else if rng.battle_random_byte() >= 50 {
                    1
                } else {
                    0
                }
            }
            "SAFEGUARD" => {
                if !hp_above_half(state.player.hp, state.player.max_hp)
                    && rng.battle_random_byte() >= 50
                {
                    1
                } else {
                    0
                }
            }
            "PURSUIT" => {
                if !hp_above_quarter(state.player.hp, state.player.max_hp) {
                    if rng.battle_random_byte() >= 128 {
                        -2
                    } else {
                        0
                    }
                } else if rng.battle_random_byte() >= 50 {
                    1
                } else {
                    0
                }
            }
            "RAPID_SPIN" => smart_rapid_spin_delta(state, rng),
            "BELLY_DRUM" => smart_belly_drum_delta(state),
            "PSYCH_UP" => smart_psych_up_delta(state, rng),
            "TWISTER" | "GUST" => smart_flying_target_delta(state, rng),
            "FUTURE_SIGHT" => {
                if smart_enemy_faster(state) && state.player_airborne_move.is_some() {
                    -2
                } else {
                    0
                }
            }
            "STOMP" => {
                if state.player_minimized && rng.battle_random_byte() >= 50 {
                    -1
                } else {
                    0
                }
            }
            "SOLARBEAM" => match state.weather {
                super::damage::Weather::Sun => {
                    if rng.battle_random_byte() >= 50 {
                        -2
                    } else {
                        0
                    }
                }
                super::damage::Weather::Rain => {
                    if rng.battle_random_byte() >= 25 {
                        2
                    } else {
                        0
                    }
                }
                super::damage::Weather::None | super::damage::Weather::Sandstorm => 0,
            },
            "THUNDER" => {
                if matches!(state.weather, super::damage::Weather::Sun)
                    && rng.battle_random_byte() >= 25
                {
                    1
                } else {
                    0
                }
            }
            "CURSE" => smart_curse_delta(state, rng),
            "MAGNITUDE" | "EARTHQUAKE" => smart_underground_target_delta(state, rng),
            "RAIN_DANCE" => smart_weather_delta(state, moves, true, rng),
            "SUNNY_DAY" => smart_weather_delta(state, moves, false, rng),
            "MIRROR_MOVE" => smart_mirror_move_delta(state, player_moves, rng),
            "MIMIC" => smart_mimic_delta(state, player_moves, rng),
            "COUNTER" => smart_counter_coat_delta(state, player_moves, true, rng),
            "MIRROR_COAT" => smart_counter_coat_delta(state, player_moves, false, rng),
            "ENCORE" => smart_encore_delta(state, player_moves, rng),
            "SPITE" => smart_spite_delta(state, player_moves, rng),
            "DISABLE" => smart_disable_delta(state, player_moves, rng),
            "RAZOR_WIND" | "UNUSED_2B" => smart_razor_wind_delta(state, player_moves, rng),
            "HIDDEN_POWER" => {
                if matches!(
                    selected.matchup,
                    TrainerAiTypeMatchup::Immune | TrainerAiTypeMatchup::NotVeryEffective
                ) || selected.power < 50
                {
                    1
                } else if selected.matchup == TrainerAiTypeMatchup::SuperEffective
                    || selected.power == 70
                {
                    -1
                } else {
                    0
                }
            }
            "FORCE_SWITCH" | "BATON_PASS" => i16::from(switch_score >= 10),
            "MEAN_LOOK" => smart_mean_look_delta(state, switch_score, rng),
            "PERISH_SONG" => smart_perish_song_delta(state, switch_score, rng),
            "PRIORITY_HIT" => {
                if smart_enemy_faster(state) {
                    0
                } else if state.player_airborne_move.is_some() {
                    10
                } else if priority_damage_by_slot
                    .get(slot)
                    .copied()
                    .flatten()
                    .is_some_and(|damage| damage > state.player.hp)
                {
                    -3
                } else {
                    0
                }
            }
            "CONVERSION2" => {
                if state.player_last_move.is_some() && rng.battle_random_byte() >= 25 {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        };
        scores[slot] += delta;
        match selected.effect {
            "LEECH_HIT" | "HIDDEN_POWER" => battle_turn_enemy = true,
            "MIMIC"
                if state.player_last_counter_move.is_some()
                    && hp_above_half(state.enemy.hp, state.enemy.max_hp) =>
            {
                battle_turn_enemy = true;
            }
            "PRIORITY_HIT"
                if !smart_enemy_faster(state) && state.player_airborne_move.is_none() =>
            {
                battle_turn_enemy = true;
            }
            "CONVERSION2" if state.player_last_move.is_none() => battle_turn_enemy = false,
            _ => {}
        }
    }
}

fn smart_sleep_delta(
    moves: &[Option<TrainerSmartMove<'_>>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if moves
        .iter()
        .flatten()
        .any(|move_data| matches!(move_data.effect, "DREAM_EATER" | "NIGHTMARE"))
        && rng.battle_random_byte() >= 128
    {
        -2
    } else {
        0
    }
}

fn smart_leech_hit_delta(
    state: &BattleCombatState,
    matchup: TrainerAiTypeMatchup,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    match matchup {
        TrainerAiTypeMatchup::Immune | TrainerAiTypeMatchup::NotVeryEffective => {
            i16::from(rng.battle_random_byte() >= 100)
        }
        TrainerAiTypeMatchup::Neutral => 0,
        TrainerAiTypeMatchup::SuperEffective => {
            if state.enemy.hp == state.enemy.max_hp || rng.battle_random_byte() < 50 {
                0
            } else {
                -1
            }
        }
    }
}

fn smart_selfdestruct_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let enemy_has_reserve = state
        .enemy_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != state.enemy_party_index && pokemon.hp != 0);
    let player_has_reserve = state
        .player_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != state.player_party_index && pokemon.hp != 0);
    if (!enemy_has_reserve && player_has_reserve)
        || hp_above_half(state.enemy.hp, state.enemy.max_hp)
    {
        return 3;
    }
    if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
        return 0;
    }
    i16::from(rng.battle_random_byte() >= 20) * 3
}

fn smart_evasion_up_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if stat_stage(&state.enemy, Stat::Evasion) >= 6 {
        return 10;
    }
    let toxic = state.player_toxic_turns != 0;
    let mut delta = 0;
    if state.enemy.hp == state.enemy.max_hp {
        if toxic || rng.battle_random_byte() < 178 {
            return -2;
        }
    } else if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
        delta += 2;
    } else {
        if rng.battle_random_byte() < 10 {
            return -2;
        }
        if !hp_above_half(state.enemy.hp, state.enemy.max_hp) {
            if rng.battle_random_byte() >= 128 {
                delta += 2;
            }
        } else if rng.battle_random_byte() < 50 {
            return -2;
        }
    }
    delta + smart_evasion_accuracy_tail(state, toxic, rng)
}

fn smart_accuracy_down_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let toxic = state.player_toxic_turns != 0;
    let mut delta = 0;
    if state.player.hp == state.player.max_hp && hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        if toxic || rng.battle_random_byte() < 178 {
            return -2;
        }
    } else if !hp_above_quarter(state.player.hp, state.player.max_hp) {
        delta += 2;
    } else {
        if rng.battle_random_byte() < 10 {
            return -2;
        }
        if !hp_above_half(state.player.hp, state.player.max_hp) {
            if rng.battle_random_byte() >= 128 {
                delta += 2;
            }
        } else if rng.battle_random_byte() < 50 {
            return -2;
        }
    }
    delta + smart_evasion_accuracy_tail(state, toxic, rng)
}

fn smart_evasion_accuracy_tail(
    state: &BattleCombatState,
    toxic: bool,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if toxic {
        return if rng.battle_random_byte() >= 80 {
            -2
        } else {
            0
        };
    }
    if state.player_leech_seed_source.is_some() {
        return if rng.battle_random_byte() >= 128 {
            -1
        } else {
            0
        };
    }
    if stat_stage(&state.enemy, Stat::Evasion) > stat_stage(&state.player, Stat::Accuracy) {
        return 1;
    }
    if state.player_fury_cutter_chain != 0 || state.player_rollout_turns != 0 {
        -2
    } else {
        1
    }
}

fn smart_always_hit_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if (stat_stage(&state.enemy, Stat::Accuracy) <= -3
        || stat_stage(&state.player, Stat::Evasion) >= 3)
        && rng.battle_random_byte() >= 50
    {
        -2
    } else {
        0
    }
}

fn smart_reset_stats_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    const LEVEL_STATS: [Stat; 7] = [
        Stat::Attack,
        Stat::Defense,
        Stat::Speed,
        Stat::SpecialAttack,
        Stat::SpecialDefense,
        Stat::Accuracy,
        Stat::Evasion,
    ];
    let useful = LEVEL_STATS
        .iter()
        .any(|stat| stat_stage(&state.enemy, *stat) <= -3)
        || LEVEL_STATS
            .iter()
            .any(|stat| stat_stage(&state.player, *stat) >= 3);
    if useful {
        if rng.battle_random_byte() >= 40 {
            -1
        } else {
            0
        }
    } else {
        1
    }
}

fn smart_heal_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
        if rng.battle_random_byte() >= 25 {
            -2
        } else {
            0
        }
    } else if hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        1
    } else {
        0
    }
}

fn smart_trap_target_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let encourage = state.player_trap.is_none()
        && (state.player_toxic_turns != 0
            || state.player_attracted_by.is_some()
            || state.player_identified
            || state.player_rollout_turns != 0
            || state.player_nightmare_source.is_some()
            || state.player_turns_taken == 0);
    if encourage {
        if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
            0
        } else if rng.battle_random_byte() >= 128 {
            -2
        } else {
            0
        }
    } else if rng.battle_random_byte() >= 128 {
        1
    } else {
        0
    }
}

fn smart_confuse_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if hp_above_half(state.player.hp, state.player.max_hp) {
        return 0;
    }
    let mut delta = i16::from(rng.battle_random_byte() >= 25);
    if !hp_above_quarter(state.player.hp, state.player.max_hp) {
        delta += 1;
    }
    delta
}

fn smart_sp_def_up_2_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if !hp_above_half(state.enemy.hp, state.enemy.max_hp)
        || stat_stage(&state.enemy, Stat::SpecialDefense) >= 4
    {
        return 1;
    }
    if stat_stage(&state.enemy, Stat::SpecialDefense) >= 2 {
        return 0;
    }
    let player = trainer_ai_effective_pokemon(state, false);
    let special_type = |type_id: &str| {
        matches!(
            type_id,
            "FIRE" | "WATER" | "GRASS" | "ELECTRIC" | "PSYCHIC_TYPE" | "ICE" | "DRAGON" | "DARK"
        )
    };
    if (special_type(&player.species.type1) || special_type(&player.species.type2))
        && rng.battle_random_byte() >= 50
    {
        -2
    } else {
        0
    }
}

fn smart_paralyze_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if !hp_above_quarter(state.player.hp, state.player.max_hp) {
        return i16::from(rng.battle_random_byte() >= 128);
    }
    if smart_enemy_faster(state) || !hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
        return 0;
    }
    if rng.battle_random_byte() >= 50 {
        -2
    } else {
        0
    }
}

fn smart_hyper_beam_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        if rng.battle_random_byte() < 40 {
            return 0;
        }
        1 + i16::from(rng.battle_random_byte() >= 128)
    } else if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp)
        && rng.battle_random_byte() >= 128
    {
        -1
    } else {
        0
    }
}

fn smart_rage_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if state.enemy_rage_active {
        let mut delta = -i16::from(rng.battle_random_byte() >= 128);
        if state.enemy_rage_counter >= 2 {
            delta -= 1;
        }
        if state.enemy_rage_counter >= 3 {
            delta -= 1;
        }
        delta
    } else if !hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        1
    } else if rng.battle_random_byte() < 50 {
        -1
    } else {
        0
    }
}

fn smart_enemy_faster(state: &BattleCombatState) -> bool {
    let mut enemy_speed = state.enemy.speed;
    if state.enemy_paralysis_speed_penalty_active {
        enemy_speed = (enemy_speed / 4).max(1);
    }
    let mut player_speed = state.player.speed;
    if state.player.status.as_deref() == Some("PARALYSIS") {
        player_speed = (player_speed / 4).max(1);
    }
    if state.badge_boosts_enabled && !state.link_battle && state.obedience_badges[2] {
        player_speed = player_speed.saturating_add(player_speed / 8).min(999);
    }
    enemy_speed > player_speed
}

fn smart_heal_bell_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let any_party_status = state
        .enemy_party
        .iter()
        .any(|pokemon| pokemon.hp != 0 && pokemon.status.is_some());
    if !any_party_status {
        return if state.enemy.status.is_some() { 0 } else { 10 };
    }
    let mut delta = -i16::from(state.enemy.status.is_some());
    if matches!(state.enemy.status.as_deref(), Some("SLEEP" | "FREEZE"))
        && rng.battle_random_byte() >= 128
    {
        delta -= 2;
    }
    delta
}

fn smart_protect_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let greatly_discourage = state.enemy_protect_counter != 0;
    let encourage = !greatly_discourage
        && !state.player_lock_on_target
        && (state.player_fury_cutter_chain >= 3
            || state.player_charging_move.is_some()
            || state.player_toxic_turns != 0
            || state.player_leech_seed_source.is_some()
            || state.player_curse_source.is_some()
            || (state.player_rollout_turns != 0 && state.player_rollout_chain >= 3));
    if encourage {
        return if rng.battle_random_byte() >= 50 {
            -1
        } else {
            0
        };
    }
    let base = i16::from(greatly_discourage);
    if rng.battle_random_byte() < 20 {
        base
    } else {
        base + 2
    }
}

fn smart_foresight_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let player = trainer_ai_effective_pokemon(state, false);
    let useful = stat_stage(&state.enemy, Stat::Accuracy) <= -3
        || stat_stage(&state.player, Stat::Evasion) >= 3
        || matches!(player.species.type1.as_str(), "GHOST")
        || matches!(player.species.type2.as_str(), "GHOST");
    if useful {
        if rng.battle_random_byte() >= 100 {
            -2
        } else {
            0
        }
    } else if rng.battle_random_byte() >= 20 {
        1
    } else {
        0
    }
}

fn smart_sandstorm_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let player = trainer_ai_effective_pokemon(state, false);
    let immune = |type_id: &str| matches!(type_id, "ROCK" | "GROUND" | "STEEL");
    if immune(&player.species.type1) || immune(&player.species.type2) {
        2
    } else if !hp_above_half(state.player.hp, state.player.max_hp) {
        1
    } else if rng.battle_random_byte() >= 128 {
        -1
    } else {
        0
    }
}

fn smart_endure_delta(
    state: &BattleCombatState,
    moves: &[Option<TrainerSmartMove<'_>>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if state.enemy_protect_counter != 0 || state.enemy.hp == state.enemy.max_hp {
        return 2;
    }
    if hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
        return 1;
    }
    if moves
        .iter()
        .flatten()
        .any(|move_data| move_data.effect == "REVERSAL")
    {
        return if rng.battle_random_byte() >= 50 {
            -3
        } else {
            0
        };
    }
    if state.enemy_lock_on_target && rng.battle_random_byte() >= 128 {
        -2
    } else {
        0
    }
}

fn smart_fury_rollout_delta(
    state: &BattleCombatState,
    fury_cutter: bool,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let mut delta = 0;
    if fury_cutter {
        let count = state.enemy_fury_cutter_chain;
        if count >= 1 {
            delta -= 1;
        }
        if count >= 2 {
            delta -= 2;
        }
        if count >= 3 {
            delta -= 3;
        }
    }
    let unstable = state.enemy_attracted_by.is_some()
        || state.enemy.confusion_turns != 0
        || state.enemy.status.as_deref() == Some("PARALYSIS")
        || !hp_above_quarter(state.enemy.hp, state.enemy.max_hp)
        || stat_stage(&state.enemy, Stat::Accuracy) < 0
        || stat_stage(&state.player, Stat::Evasion) >= 1;
    if unstable {
        if rng.battle_random_byte() >= 50 {
            delta += 1;
        }
    } else if rng.battle_random_byte() < 200 {
        delta -= 2;
    }
    delta
}

fn smart_rapid_spin_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if (state.enemy_trap.is_some() || state.enemy_leech_seed_source.is_some() || state.enemy_spikes)
        && rng.battle_random_byte() >= 50
    {
        -2
    } else {
        0
    }
}

fn smart_belly_drum_delta(state: &BattleCombatState) -> i16 {
    if stat_stage(&state.enemy, Stat::Attack) >= 3 {
        5
    } else if state.enemy.hp == state.enemy.max_hp {
        0
    } else if hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        1
    } else {
        6
    }
}

fn smart_psych_up_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    const LEVEL_STATS: [Stat; 7] = [
        Stat::Attack,
        Stat::Defense,
        Stat::Speed,
        Stat::SpecialAttack,
        Stat::SpecialDefense,
        Stat::Accuracy,
        Stat::Evasion,
    ];
    let enemy_sum = LEVEL_STATS
        .iter()
        .map(|stat| i16::from(stat_stage(&state.enemy, *stat)))
        .sum::<i16>();
    let player_sum = LEVEL_STATS
        .iter()
        .map(|stat| i16::from(stat_stage(&state.player, *stat)))
        .sum::<i16>();
    if enemy_sum >= player_sum {
        2
    } else if stat_stage(&state.player, Stat::Accuracy) < -1
        || stat_stage(&state.enemy, Stat::Evasion) >= 1
    {
        0
    } else if rng.battle_random_byte() >= 50 {
        -1
    } else {
        0
    }
}

fn smart_flying_target_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    if state.player_last_counter_move.as_deref() != Some("FLY") {
        return 0;
    }
    if state.player_airborne_move.as_deref() == Some("FLY") {
        if smart_enemy_faster(state) { -2 } else { 0 }
    } else if !smart_enemy_faster(state) && rng.battle_random_byte() >= 128 {
        -1
    } else {
        0
    }
}

fn smart_lock_on_apply(
    state: &BattleCombatState,
    scores: &mut [i16],
    moves: &[Option<TrainerSmartMove<'_>>],
    selected_slot: usize,
    rng: &mut dyn BattleRandomSource,
) -> bool {
    if state.player_lock_on_target {
        for (score, move_data) in scores.iter_mut().zip(moves) {
            let Some(move_data) = move_data else {
                break;
            };
            if move_data.accuracy < 71 {
                *score -= 2;
            }
        }
        scores[selected_slot] += 10;
        return false;
    }
    if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp)
        || (!hp_above_half(state.enemy.hp, state.enemy.max_hp) && !smart_enemy_faster(state))
    {
        scores[selected_slot] += 1;
        return false;
    }
    let player_evasion = stat_stage(&state.player, Stat::Evasion);
    let enemy_accuracy = stat_stage(&state.enemy, Stat::Accuracy);
    let maybe_encourage = player_evasion >= 3 || enemy_accuracy <= -3;
    if player_evasion >= 1 || enemy_accuracy < 0 {
        if maybe_encourage && rng.battle_random_byte() >= 128 {
            scores[selected_slot] -= 2;
        }
        return false;
    }
    let checked_type_matchup = moves
        .iter()
        .flatten()
        .any(|move_data| move_data.accuracy < 71);
    if moves.iter().flatten().any(|move_data| {
        move_data.accuracy < 71
            && matches!(
                move_data.matchup,
                TrainerAiTypeMatchup::Neutral | TrainerAiTypeMatchup::SuperEffective
            )
    }) {
        return checked_type_matchup;
    }
    if maybe_encourage {
        if rng.battle_random_byte() >= 128 {
            scores[selected_slot] -= 2;
        }
    } else {
        scores[selected_slot] += 1;
    }
    checked_type_matchup
}

fn smart_curse_delta(state: &BattleCombatState, rng: &mut dyn BattleRandomSource) -> i16 {
    let enemy = trainer_ai_effective_pokemon(state, true);
    let player = trainer_ai_effective_pokemon(state, false);
    let enemy_is_ghost = enemy.species.type1 == "GHOST" || enemy.species.type2 == "GHOST";
    if !enemy_is_ghost {
        if !hp_above_half(state.enemy.hp, state.enemy.max_hp)
            || stat_stage(&state.enemy, Stat::Attack) >= 4
        {
            return 1;
        }
        if stat_stage(&state.enemy, Stat::Attack) >= 2 {
            return 0;
        }
        if player.species.type1 == "GHOST" {
            return 2;
        }
        if smart_special_type(&player.species.type1) || smart_special_type(&player.species.type2) {
            return 0;
        }
        return if rng.battle_random_byte() >= 50 {
            -2
        } else {
            0
        };
    }
    if state.player_curse_source.is_some() {
        return 10;
    }
    let enemy_has_reserve = state
        .enemy_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != state.enemy_party_index && pokemon.hp != 0);
    let player_has_reserve = state
        .player_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != state.player_party_index && pokemon.hp != 0);
    if !enemy_has_reserve && player_has_reserve {
        return 4;
    }
    if enemy_has_reserve && !player_has_reserve {
        return if rng.battle_random_byte() >= 128 {
            -2
        } else {
            0
        };
    }
    if !hp_above_quarter(state.enemy.hp, state.enemy.max_hp) {
        4
    } else if !hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        2
    } else if state.enemy.hp != state.enemy.max_hp || state.player_turns_taken != 0 {
        0
    } else if rng.battle_random_byte() >= 128 {
        -2
    } else {
        0
    }
}

fn smart_underground_target_delta(
    state: &BattleCombatState,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if state.player_last_counter_move.as_deref() != Some("DIG") {
        return 0;
    }
    if state.player_airborne_move.as_deref() == Some("DIG") {
        if smart_enemy_faster(state) { -2 } else { 0 }
    } else if !smart_enemy_faster(state) && rng.battle_random_byte() >= 128 {
        -1
    } else {
        0
    }
}

fn smart_weather_delta(
    state: &BattleCombatState,
    moves: &[Option<TrainerSmartMove<'_>>],
    rain: bool,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let player = trainer_ai_effective_pokemon(state, false);
    let (bad_type, good_type) = if rain {
        ("WATER", "FIRE")
    } else {
        ("FIRE", "WATER")
    };
    for type_id in [&player.species.type1, &player.species.type2] {
        if type_id == bad_type {
            return 3;
        }
        if type_id == good_type {
            return if hp_above_half(state.player.hp, state.player.max_hp)
                && (state.player_turns_taken == 0 || state.enemy_turns_taken == 0)
            {
                -2
            } else {
                0
            };
        }
    }
    let useful = moves.iter().flatten().any(|move_data| {
        if rain {
            matches!(
                move_data.move_id,
                "WATER_GUN"
                    | "HYDRO_PUMP"
                    | "SURF"
                    | "BUBBLEBEAM"
                    | "THUNDER"
                    | "WATERFALL"
                    | "CLAMP"
                    | "BUBBLE"
                    | "CRABHAMMER"
                    | "OCTAZOOKA"
                    | "WHIRLPOOL"
            )
        } else {
            matches!(
                move_data.move_id,
                "FIRE_PUNCH"
                    | "EMBER"
                    | "FLAMETHROWER"
                    | "FIRE_SPIN"
                    | "FIRE_BLAST"
                    | "SACRED_FIRE"
                    | "MORNING_SUN"
                    | "SYNTHESIS"
            )
        }
    });
    if !useful || !hp_above_half(state.player.hp, state.player.max_hp) {
        3
    } else if rng.battle_random_byte() >= 128 {
        -1
    } else {
        0
    }
}

fn smart_special_type(type_id: &str) -> bool {
    matches!(
        type_id,
        "FIRE" | "WATER" | "GRASS" | "ELECTRIC" | "PSYCHIC_TYPE" | "ICE" | "DRAGON" | "DARK"
    )
}

fn smart_last_player_move<'a>(
    state: &BattleCombatState,
    player_moves: &'a [TrainerSmartPlayerMove<'a>],
) -> Option<&'a TrainerSmartPlayerMove<'a>> {
    let last = state.player_last_move.as_deref()?;
    player_moves
        .iter()
        .find(|move_data| move_data.move_id == last)
}

fn smart_last_player_counter_move<'a>(
    state: &BattleCombatState,
    player_moves: &'a [TrainerSmartPlayerMove<'a>],
) -> Option<&'a TrainerSmartPlayerMove<'a>> {
    let last = state.player_last_counter_move.as_deref()?;
    player_moves
        .iter()
        .find(|move_data| move_data.move_id == last)
}

fn smart_mirror_move_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let Some(last) = smart_last_player_counter_move(state, player_moves) else {
        return if smart_enemy_faster(state) { 10 } else { 0 };
    };
    if !smart_useful_move(last.move_id) {
        return 0;
    }
    let mut delta = -i16::from(rng.battle_random_byte() >= 128);
    if smart_enemy_faster(state) && rng.battle_random_byte() >= 25 {
        delta -= 1;
    }
    delta
}

fn smart_mimic_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let Some(last) = smart_last_player_counter_move(state, player_moves) else {
        return if smart_enemy_faster(state) { 10 } else { 1 };
    };
    if !hp_above_half(state.enemy.hp, state.enemy.max_hp)
        || matches!(
            last.matchup_against_player,
            TrainerAiTypeMatchup::Immune | TrainerAiTypeMatchup::NotVeryEffective
        )
    {
        return 1;
    }
    let mut delta = 0;
    if last.matchup_against_player == TrainerAiTypeMatchup::SuperEffective
        && rng.battle_random_byte() >= 128
    {
        delta -= 1;
    }
    if smart_useful_move(last.move_id) && rng.battle_random_byte() >= 128 {
        delta -= 1;
    }
    delta
}

fn smart_counter_coat_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    physical: bool,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let matching = player_moves
        .iter()
        .filter(|move_data| move_data.power != 0 && move_data.physical == physical)
        .count();
    if matching == 0 {
        return 1;
    }
    let last_matches = smart_last_player_counter_move(state, player_moves)
        .is_some_and(|move_data| move_data.power != 0 && move_data.physical == physical);
    if (matching >= 3 || last_matches) && rng.battle_random_byte() >= 100 {
        -1
    } else {
        0
    }
}

fn smart_encore_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if !smart_enemy_faster(state) {
        return 3;
    }
    let Some(last) = smart_last_player_move(state, player_moves) else {
        return 10;
    };
    if last.power != 0 {
        match last.matchup_against_enemy {
            TrainerAiTypeMatchup::Immune => {}
            TrainerAiTypeMatchup::NotVeryEffective => return 0,
            TrainerAiTypeMatchup::Neutral | TrainerAiTypeMatchup::SuperEffective => {
                if !state
                    .player_last_counter_move
                    .as_deref()
                    .is_some_and(smart_encore_move)
                {
                    return 3;
                }
            }
        }
    } else if !state
        .player_last_counter_move
        .as_deref()
        .is_some_and(smart_encore_move)
    {
        return 3;
    }
    if rng.battle_random_byte() >= 70 {
        -2
    } else {
        0
    }
}

fn smart_spite_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let Some(last) = smart_last_player_counter_move(state, player_moves) else {
        if smart_enemy_faster(state) {
            return 10;
        }
        return i16::from(rng.battle_random_byte() >= 128);
    };
    let Some(current_pp) = last.current_pp else {
        return 0;
    };
    if current_pp < 6 {
        if rng.battle_random_byte() >= 100 {
            -2
        } else {
            0
        }
    } else if current_pp >= 15 {
        1
    } else if rng.battle_random_byte() < 100 {
        1
    } else {
        0
    }
}

fn smart_disable_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if smart_enemy_faster(state)
        && smart_last_player_counter_move(state, player_moves)
            .is_some_and(|move_data| smart_useful_move(move_data.move_id))
    {
        if rng.battle_random_byte() >= 100 {
            -1
        } else {
            0
        }
    } else if rng.battle_random_byte() >= 20 {
        1
    } else {
        0
    }
}

fn smart_razor_wind_delta(
    state: &BattleCombatState,
    player_moves: &[TrainerSmartPlayerMove<'_>],
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if state.enemy.perish_song_turns != 0 && state.enemy.perish_song_turns < 3 {
        return 1;
    }
    if player_moves
        .iter()
        .any(|move_data| move_data.effect == "PROTECT")
    {
        return 6;
    }
    if state.enemy.confusion_turns != 0 || !hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        return i16::from(rng.battle_random_byte() >= 200);
    }
    0
}

fn smart_useful_move(move_id: &str) -> bool {
    matches!(
        move_id,
        "DOUBLE_EDGE"
            | "SING"
            | "FLAMETHROWER"
            | "HYDRO_PUMP"
            | "SURF"
            | "ICE_BEAM"
            | "BLIZZARD"
            | "HYPER_BEAM"
            | "SLEEP_POWDER"
            | "THUNDERBOLT"
            | "THUNDER"
            | "EARTHQUAKE"
            | "TOXIC"
            | "PSYCHIC_M"
            | "HYPNOSIS"
            | "RECOVER"
            | "FIRE_BLAST"
            | "SOFTBOILED"
            | "SUPER_FANG"
    )
}

fn smart_encore_move(move_id: &str) -> bool {
    matches!(
        move_id,
        "SWORDS_DANCE"
            | "WHIRLWIND"
            | "LEER"
            | "ROAR"
            | "DISABLE"
            | "MIST"
            | "LEECH_SEED"
            | "GROWTH"
            | "POISONPOWDER"
            | "STRING_SHOT"
            | "MEDITATE"
            | "AGILITY"
            | "TELEPORT"
            | "SCREECH"
            | "HAZE"
            | "FOCUS_ENERGY"
            | "DREAM_EATER"
            | "POISON_GAS"
            | "SPLASH"
            | "SHARPEN"
            | "CONVERSION"
            | "SUPER_FANG"
            | "SUBSTITUTE"
            | "TRIPLE_KICK"
            | "SPIDER_WEB"
            | "MIND_READER"
            | "FLAME_WHEEL"
            | "AEROBLAST"
            | "COTTON_SPORE"
            | "POWDER_SNOW"
    )
}

pub fn trainer_smart_hidden_power(state: &BattleCombatState) -> (String, u16) {
    super::turn::hidden_power_type_power(&trainer_ai_effective_pokemon(state, true))
}

fn trainer_smart_switch_score(
    enemy_moves: &[Option<TrainerSmartMove<'_>>],
    player_moves: &[TrainerSmartPlayerMove<'_>],
    player_type_count: usize,
    current_matchup_against_enemy: TrainerAiTypeMatchup,
    current_matchup_against_player: TrainerAiTypeMatchup,
) -> i16 {
    let mut score = 10i16;
    if player_moves.is_empty() {
        if current_matchup_against_enemy == TrainerAiTypeMatchup::SuperEffective {
            score -= player_type_count as i16;
        }
    } else {
        let damaging_count = player_moves
            .iter()
            .filter(|move_data| move_data.power != 0)
            .count();
        if damaging_count != 0 {
            score += match current_matchup_against_enemy {
                TrainerAiTypeMatchup::SuperEffective => -1,
                TrainerAiTypeMatchup::Neutral => 0,
                TrainerAiTypeMatchup::NotVeryEffective => 1,
                TrainerAiTypeMatchup::Immune => 2,
            };
        } else {
            score += 2;
        }
    }
    let mut enemy_matchup_score = 0u16;
    for move_data in enemy_moves.iter().flatten() {
        if move_data.power == 0 {
            continue;
        }
        match current_matchup_against_player {
            TrainerAiTypeMatchup::Immune => {}
            TrainerAiTypeMatchup::NotVeryEffective => enemy_matchup_score += 1,
            TrainerAiTypeMatchup::Neutral => enemy_matchup_score += 5,
            TrainerAiTypeMatchup::SuperEffective => enemy_matchup_score = 100,
        }
    }
    if enemy_matchup_score == 0 {
        score -= 2;
    } else if enemy_matchup_score < 5 {
        score -= 1;
    } else if enemy_matchup_score >= 100 {
        score += 1;
    }
    score
}

fn smart_mean_look_delta(
    state: &BattleCombatState,
    switch_score: i16,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if !hp_above_half(state.enemy.hp, state.enemy.max_hp) {
        return 1;
    }
    let player_has_reserve = state
        .player_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != state.player_party_index && pokemon.hp != 0);
    if !player_has_reserve {
        return 10;
    }
    let encourage = state.enemy_toxic_turns != 0
        || state.player_attracted_by.is_some()
        || state.player_identified
        || state.player_rollout_turns != 0
        || state.player_nightmare_source.is_some();
    if encourage {
        if rng.battle_random_byte() >= 50 {
            -3
        } else {
            0
        }
    } else if switch_score >= 11 {
        0
    } else {
        1
    }
}

fn smart_perish_song_delta(
    state: &BattleCombatState,
    switch_score: i16,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let enemy_has_reserve = state
        .enemy_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != state.enemy_party_index && pokemon.hp != 0);
    if !enemy_has_reserve {
        return 5;
    }
    if state.player_escape_trap.is_some() {
        return if rng.battle_random_byte() >= 128 {
            -1
        } else {
            0
        };
    }
    if switch_score < 10 {
        0
    } else if rng.battle_random_byte() >= 128 {
        1
    } else {
        0
    }
}

fn stat_stage(pokemon: &Pokemon, stat: Stat) -> i8 {
    pokemon.stat_boosts.get(&stat).copied().unwrap_or(0)
}

fn hp_above_half(hp: u16, max_hp: u16) -> bool {
    u32::from(hp) * 2 > u32::from(max_hp)
}

fn hp_above_quarter(hp: u16, max_hp: u16) -> bool {
    u32::from(hp) * 4 > u32::from(max_hp)
}

/// Exact `AIDamageCalc` result for an enemy move. The routine intentionally
/// uses the prior live critical-hit register, maximum damage (no variation),
/// and the four constant-damage effects' separate command path.
pub fn trainer_ai_damage(
    state: &BattleCombatState,
    move_data: &Move,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    held_type_boost_percent: u8,
    rng: &mut dyn BattleRandomSource,
) -> Result<u16, DamageCalculationError> {
    match move_data.effect.as_str() {
        "SUPER_FANG" => return Ok((state.player.hp / 2).max(1)),
        "STATIC_DAMAGE" => return Ok(move_data.power),
        "LEVEL_DAMAGE" => return Ok(u16::from(state.enemy.level)),
        "PSYWAVE" => {
            let limit = state.enemy.level.saturating_add(state.enemy.level / 2);
            loop {
                let roll = rng.battle_random_byte();
                if roll != 0 && roll < limit {
                    return Ok(u16::from(roll));
                }
            }
        }
        _ => {}
    }

    let mut attacker = trainer_ai_effective_pokemon(state, true);
    let defender = trainer_ai_effective_pokemon(state, false);
    if attacker.status.as_deref() == Some("BURN") {
        attacker.status = None;
    }
    let physical = is_physical_type(type_categories, &move_data.move_type)?;
    calculate_damage(
        &attacker,
        &defender,
        move_data,
        stat_multipliers,
        type_categories,
        type_effectiveness,
        weather_modifiers,
        DamageContext {
            is_critical: state.critical_hit_register != 0,
            defender_identified: state.player_identified,
            weather: state.weather,
            random_roll: 255,
            defender_badge_boost: trainer_ai_player_badge_stat_boost(
                state,
                if physical {
                    Stat::Defense
                } else {
                    Stat::SpecialDefense
                },
            ),
            defender_metal_powder: state.player.species.id == "DITTO"
                && state.player.item.as_deref() == Some("METAL_POWDER"),
            attacker_species_item_boost: if physical {
                matches!(state.enemy.species.id.as_str(), "CUBONE" | "MAROWAK")
                    && state.enemy.item.as_deref() == Some("THICK_CLUB")
            } else {
                state.enemy.species.id == "PIKACHU"
                    && state.enemy.item.as_deref() == Some("LIGHT_BALL")
            },
            defender_screen: if physical {
                state.player_reflect_turns != 0
            } else {
                state.player_light_screen_turns != 0
            },
            link_colosseum: state.link_colosseum,
            held_type_boost_percent,
            attacker_burn_penalty: state.enemy_burn_attack_penalty_active,
            attacker_loaded_stat: Some(if physical {
                state.enemy_loaded_stats.attack
            } else {
                state.enemy_loaded_stats.special_attack
            }),
            defender_loaded_stat: Some(if physical {
                state.player_loaded_stats.defense
            } else {
                state.player_loaded_stats.special_defense
            }),
            ..DamageContext::default()
        },
    )
    .map(|result| result.damage)
}

fn trainer_ai_effective_pokemon(state: &BattleCombatState, enemy: bool) -> Pokemon {
    let (pokemon, transformed, type_override) = if enemy {
        (
            &state.enemy,
            state.enemy_transform.as_ref(),
            state.enemy_type_override.as_ref(),
        )
    } else {
        (
            &state.player,
            state.player_transform.as_ref(),
            state.player_type_override.as_ref(),
        )
    };
    let mut pokemon = pokemon.clone();
    if let Some(transform) = transformed {
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
    if let Some(types) = type_override {
        pokemon.species.type1 = types.type1.clone();
        pokemon.species.type2 = types.type2.clone();
    }
    pokemon
}

fn trainer_ai_player_badge_stat_boost(state: &BattleCombatState, stat: Stat) -> bool {
    if state.link_battle || !state.badge_boosts_enabled {
        return false;
    }
    match stat {
        Stat::Defense => state.obedience_badges[4],
        Stat::SpecialDefense => {
            state.obedience_badges[6]
                && ((206..=432)
                    .contains(&trainer_ai_effective_pokemon(state, false).special_attack)
                    || trainer_ai_effective_pokemon(state, false).special_attack >= 661)
        }
        _ => false,
    }
}

pub fn trainer_ai_held_type_boost_percent(
    held_effect: &str,
    parameter: i16,
    move_type: &str,
) -> Option<Result<u8, i16>> {
    let boosted_type = match held_effect {
        "HELD_BUG_BOOST" => "BUG",
        "HELD_DARK_BOOST" => "DARK",
        "HELD_DRAGON_BOOST" => "DRAGON",
        "HELD_ELECTRIC_BOOST" => "ELECTRIC",
        "HELD_FIGHTING_BOOST" => "FIGHTING",
        "HELD_FIRE_BOOST" => "FIRE",
        "HELD_FLYING_BOOST" => "FLYING",
        "HELD_GHOST_BOOST" => "GHOST",
        "HELD_GRASS_BOOST" => "GRASS",
        "HELD_GROUND_BOOST" => "GROUND",
        "HELD_ICE_BOOST" => "ICE",
        "HELD_NORMAL_BOOST" => "NORMAL",
        "HELD_POISON_BOOST" => "POISON",
        "HELD_PSYCHIC_BOOST" => "PSYCHIC_TYPE",
        "HELD_ROCK_BOOST" => "ROCK",
        "HELD_STEEL_BOOST" => "STEEL",
        "HELD_WATER_BOOST" => "WATER",
        _ => return None,
    };
    if boosted_type != move_type {
        return None;
    }
    Some(
        u8::try_from(parameter)
            .ok()
            .filter(|value| *value != 0)
            .ok_or(parameter),
    )
}

/// Exact `AI_Basic` score delta, including `AI_Redundant`'s side-selection
/// quirks and documented Nightmare/Future Sight bugs.
pub fn trainer_basic_score_delta(state: &BattleCombatState, effect: &str) -> i16 {
    if trainer_ai_effect_is_redundant(state, effect)
        || (trainer_ai_status_only_effect(effect)
            && (state.player.status.is_some() || state.player_safeguard_turns != 0))
    {
        10
    } else {
        0
    }
}

fn trainer_ai_effect_is_redundant(state: &BattleCombatState, effect: &str) -> bool {
    match effect {
        "DREAM_EATER" => state.player.status.as_deref() != Some("SLEEP"),
        "HEAL" | "MORNING_SUN" | "SYNTHESIS" | "MOONLIGHT" => state.enemy.hp == state.enemy.max_hp,
        "LIGHT_SCREEN" => state.enemy_light_screen_turns != 0,
        "MIST" => state.enemy_mist_active,
        "FOCUS_ENERGY" => state.enemy.focus_energy,
        "CONFUSE" => state.player.confusion_turns != 0 || state.player_safeguard_turns != 0,
        "TRANSFORM" => state.enemy_transform.is_some(),
        "REFLECT" => state.enemy_reflect_turns != 0,
        "SUBSTITUTE" => state.enemy_substitute_hp != 0,
        "LEECH_SEED" => state.player_leech_seed_source.is_some(),
        "DISABLE" => state
            .player_disable
            .as_ref()
            .is_some_and(|disable| disable.turns_remaining != 0),
        "ENCORE" => state
            .player_encore
            .as_ref()
            .is_some_and(|encore| encore.turns_remaining != 0),
        "SNORE" | "SLEEP_TALK" => state.enemy.status.as_deref() != Some("SLEEP"),
        // AI_Redundant checks the enemy's own CANT_RUN substatus here.
        "MEAN_LOOK" => state.enemy_escape_trap.is_some(),
        // Source bug: any major status passes the sleep prerequisite.
        "NIGHTMARE" => state.player.status.is_none() || state.player_nightmare_source.is_some(),
        "SPIKES" => state.player_spikes,
        "FORESIGHT" => state.player_identified,
        "PERISH_SONG" => state.player.perish_song_turns != 0,
        "SANDSTORM" => matches!(state.weather, super::damage::Weather::Sandstorm),
        "ATTRACT" => {
            let player_gender = battle_pokemon_gender(&state.player);
            let enemy_gender = battle_pokemon_gender(&state.enemy);
            player_gender.is_none()
                || enemy_gender.is_none()
                || player_gender == enemy_gender
                || state.player_attracted_by.is_some()
        }
        "SAFEGUARD" => state.enemy_safeguard_turns != 0,
        "RAIN_DANCE" => matches!(state.weather, super::damage::Weather::Rain),
        "SUNNY_DAY" => matches!(state.weather, super::damage::Weather::Sun),
        "TELEPORT" => true,
        "SWAGGER" => state.player.confusion_turns != 0,
        // Source bug: AI_Redundant tests an unused screen bit, so this never
        // recognizes an already queued Future Sight.
        "FUTURE_SIGHT" => false,
        _ => false,
    }
}

fn trainer_ai_status_only_effect(effect: &str) -> bool {
    matches!(effect, "SLEEP" | "TOXIC" | "POISON" | "PARALYZE")
}

/// Exact `AI_Setup` score delta for one move.
///
/// The source calls `Random` once for every stat-changing move examined. A
/// first-turn move is encouraged by two points on rolls 128..=255; otherwise
/// it is discouraged by two points on rolls 30..=255.
pub fn trainer_setup_score_delta(
    effect: &str,
    enemy_turns_taken: u8,
    player_turns_taken: u8,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    let first_turn = if trainer_ai_stat_up_effect(effect) {
        enemy_turns_taken == 0
    } else if trainer_ai_stat_down_effect(effect) {
        player_turns_taken == 0
    } else {
        return 0;
    };
    let roll = rng.battle_random_byte();
    if first_turn {
        if roll >= 128 { -2 } else { 0 }
    } else if roll >= 30 {
        2
    } else {
        0
    }
}

/// Exact `AI_Types` score delta for one move.
pub fn trainer_types_score_delta(
    selected_type: &str,
    selected_power: u16,
    known_moves: &[(&str, u16)],
    matchup: TrainerAiTypeMatchup,
) -> i16 {
    match matchup {
        TrainerAiTypeMatchup::Immune => 10,
        TrainerAiTypeMatchup::Neutral => 0,
        TrainerAiTypeMatchup::SuperEffective => i16::from(selected_power != 0) * -1,
        TrainerAiTypeMatchup::NotVeryEffective => i16::from(
            known_moves
                .iter()
                .any(|candidate| candidate.0 != selected_type && candidate.1 != 0),
        ),
    }
}

/// Exact `AI_Offensive` score delta for one move.
pub fn trainer_offensive_score_delta(power: u16) -> i16 {
    if power == 0 { 2 } else { 0 }
}

/// Exact `AI_Opportunist` layer gate. The source samples once at 25%..=50%
/// HP, discourages unconditionally at or below 25%, and does nothing above
/// 50%.
pub fn trainer_opportunist_discourages(
    hp: u16,
    max_hp: u16,
    rng: &mut dyn BattleRandomSource,
) -> bool {
    if u32::from(hp) * 2 > u32::from(max_hp) {
        false
    } else if u32::from(hp) * 4 <= u32::from(max_hp) {
        true
    } else {
        rng.battle_random_byte() >= 128
    }
}

pub fn trainer_ai_stall_move(name: &str) -> bool {
    matches!(
        name,
        "SWORDS_DANCE"
            | "TAIL_WHIP"
            | "LEER"
            | "GROWL"
            | "DISABLE"
            | "MIST"
            | "COUNTER"
            | "LEECH_SEED"
            | "GROWTH"
            | "STRING_SHOT"
            | "MEDITATE"
            | "AGILITY"
            | "RAGE"
            | "MIMIC"
            | "SCREECH"
            | "HARDEN"
            | "WITHDRAW"
            | "DEFENSE_CURL"
            | "BARRIER"
            | "LIGHT_SCREEN"
            | "HAZE"
            | "REFLECT"
            | "FOCUS_ENERGY"
            | "BIDE"
            | "AMNESIA"
            | "TRANSFORM"
            | "SPLASH"
            | "ACID_ARMOR"
            | "SHARPEN"
            | "CONVERSION"
            | "SUBSTITUTE"
            | "FLAME_WHEEL"
    )
}

/// Exact `AI_Cautious` pass. A roll of 230 or greater returns from the entire
/// layer, preserving the original early-return bug.
pub fn apply_trainer_cautious_scores(
    scores: &mut [i16],
    move_names: &[Option<&str>],
    enemy_turns_taken: u8,
    rng: &mut dyn BattleRandomSource,
) {
    if enemy_turns_taken == 0 {
        return;
    }
    for (score, name) in scores.iter_mut().zip(move_names) {
        let Some(name) = name else {
            return;
        };
        if !trainer_ai_residual_move(name) {
            continue;
        }
        if rng.battle_random_byte() >= 230 {
            return;
        }
        *score += 1;
    }
}

fn trainer_ai_residual_move(name: &str) -> bool {
    matches!(
        name,
        "MIST"
            | "LEECH_SEED"
            | "POISONPOWDER"
            | "STUN_SPORE"
            | "THUNDER_WAVE"
            | "FOCUS_ENERGY"
            | "BIDE"
            | "POISON_GAS"
            | "TRANSFORM"
            | "CONVERSION"
            | "SUBSTITUTE"
            | "SPIKES"
    )
}

/// Exact `AI_Status` score delta. Toxic and regular poison have the source's
/// explicit Poison-type immunity before the ordinary type matchup check.
pub fn trainer_status_score_delta(
    effect: &str,
    power: u16,
    player_types: &[&str],
    matchup: TrainerAiTypeMatchup,
) -> i16 {
    let checks_type_immunity = match effect {
        "TOXIC" | "POISON" => {
            if player_types.contains(&"POISON") {
                return 10;
            }
            true
        }
        "SLEEP" | "PARALYZE" => true,
        _ => power != 0,
    };
    if checks_type_immunity && matchup == TrainerAiTypeMatchup::Immune {
        10
    } else {
        0
    }
}

/// Exact score pass for `AI_Aggressive` once each move's `AIDamageCalc`
/// result has been produced. Equal damage replaces the prior candidate, so
/// the last tied move is the source-selected strongest move.
pub fn apply_trainer_aggressive_scores(
    scores: &mut [i16],
    moves: &[Option<TrainerAiDamageEvaluation<'_>>],
) {
    let mut strongest_slot = None;
    let mut strongest_damage = 0;
    for (slot, move_data) in moves.iter().enumerate() {
        let Some(move_data) = move_data else {
            break;
        };
        if move_data.power != 0 && move_data.damage >= strongest_damage {
            strongest_damage = move_data.damage;
            strongest_slot = Some(slot);
        }
    }
    let Some(strongest_slot) = strongest_slot else {
        return;
    };
    for (slot, (score, move_data)) in scores.iter_mut().zip(moves).enumerate() {
        let Some(move_data) = move_data else {
            break;
        };
        if slot != strongest_slot
            && move_data.power >= 2
            && !trainer_ai_reckless_effect(move_data.effect)
        {
            *score += 1;
        }
    }
}

fn trainer_ai_reckless_effect(effect: &str) -> bool {
    matches!(
        effect,
        "SELFDESTRUCT" | "RAMPAGE" | "MULTI_HIT" | "DOUBLE_HIT"
    )
}

/// Exact `AI_Risky` delta once `AIDamageCalc` has produced the candidate
/// damage. Risky effects are excluded at full HP; below full HP they reach
/// the KO check only on rolls 200..=255. The source requires damage to be
/// strictly greater than current HP rather than equal to it.
pub fn trainer_risky_score_delta(
    effect: &str,
    power: u16,
    damage: u16,
    player_hp: u16,
    enemy_hp: u16,
    enemy_max_hp: u16,
    rng: &mut dyn BattleRandomSource,
) -> i16 {
    if !trainer_risky_should_check_ko(effect, power, enemy_hp, enemy_max_hp, rng) {
        return 0;
    }
    trainer_risky_ko_score_delta(damage, player_hp)
}

pub fn trainer_risky_should_check_ko(
    effect: &str,
    power: u16,
    enemy_hp: u16,
    enemy_max_hp: u16,
    rng: &mut dyn BattleRandomSource,
) -> bool {
    power != 0
        && (!matches!(effect, "SELFDESTRUCT" | "OHKO")
            || (enemy_hp != enemy_max_hp && rng.battle_random_byte() >= 200))
}

pub fn trainer_risky_ko_score_delta(damage: u16, player_hp: u16) -> i16 {
    if damage > player_hp { -5 } else { 0 }
}

fn trainer_ai_stat_up_effect(effect: &str) -> bool {
    matches!(
        effect,
        "ATTACK_UP"
            | "DEFENSE_UP"
            | "SPEED_UP"
            | "SPECIAL_ATTACK_UP"
            | "SPECIAL_DEFENSE_UP"
            | "ACCURACY_UP"
            | "EVASION_UP"
            | "ATTACK_UP_2"
            | "DEFENSE_UP_2"
            | "SPEED_UP_2"
            | "SPECIAL_ATTACK_UP_2"
            | "SPECIAL_DEFENSE_UP_2"
            | "ACCURACY_UP_2"
            | "EVASION_UP_2"
    )
}

fn trainer_ai_stat_down_effect(effect: &str) -> bool {
    matches!(
        effect,
        "ATTACK_DOWN"
            | "DEFENSE_DOWN"
            | "SPEED_DOWN"
            | "SPECIAL_ATTACK_DOWN"
            | "SPECIAL_DEFENSE_DOWN"
            | "ACCURACY_DOWN"
            | "EVASION_DOWN"
            | "ATTACK_DOWN_2"
            | "DEFENSE_DOWN_2"
            | "SPEED_DOWN_2"
            | "SPECIAL_ATTACK_DOWN_2"
            | "SPECIAL_DEFENSE_DOWN_2"
            | "ACCURACY_DOWN_2"
            | "EVASION_DOWN_2"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies, pokemon_type};
    use std::collections::VecDeque;

    struct ScriptedRandom(VecDeque<u8>);

    impl BattleRandomSource for ScriptedRandom {
        fn battle_random_byte(&mut self) -> u8 {
            self.0.pop_front().expect("scripted AI random byte")
        }
    }

    fn ai_test_state() -> BattleCombatState {
        let species = |id: &str| {
            let mut species =
                PokemonSpecies::new_for_tests(id, BaseStats::new(40, 40, 40, 40, 40, 40));
            species.type1 = pokemon_type("NORMAL");
            species.type2 = pokemon_type("NORMAL");
            species
        };
        let player = Pokemon::new_for_tests(species("PLAYER"), 20, Dv::from_non_hp(10, 10, 10, 10));
        let enemy = Pokemon::new_for_tests(species("ENEMY"), 20, Dv::from_non_hp(10, 10, 10, 10));
        BattleCombatState::new(player, enemy)
    }

    #[test]
    fn setup_layer_uses_source_thresholds_and_turn_owner() {
        let mut rng = ScriptedRandom(VecDeque::from([127, 128, 29, 30]));
        assert_eq!(trainer_setup_score_delta("ATTACK_UP", 0, 9, &mut rng), 0);
        assert_eq!(trainer_setup_score_delta("ATTACK_UP", 0, 9, &mut rng), -2);
        assert_eq!(trainer_setup_score_delta("ATTACK_UP", 1, 0, &mut rng), 0);
        assert_eq!(trainer_setup_score_delta("ATTACK_UP", 1, 0, &mut rng), 2);
    }

    #[test]
    fn types_and_offensive_layers_use_source_score_deltas() {
        let mixed_damaging = [("FIRE", 40), ("NORMAL", 35)];
        assert_eq!(
            trainer_types_score_delta("FIRE", 40, &mixed_damaging, TrainerAiTypeMatchup::Immune,),
            10,
        );
        assert_eq!(
            trainer_types_score_delta(
                "FIRE",
                40,
                &mixed_damaging,
                TrainerAiTypeMatchup::SuperEffective,
            ),
            -1,
        );
        assert_eq!(
            trainer_types_score_delta(
                "FIRE",
                40,
                &mixed_damaging,
                TrainerAiTypeMatchup::NotVeryEffective,
            ),
            1,
        );
        assert_eq!(
            trainer_types_score_delta(
                "FIRE",
                40,
                &[("FIRE", 40), ("NORMAL", 0)],
                TrainerAiTypeMatchup::NotVeryEffective,
            ),
            0,
        );
        assert_eq!(trainer_offensive_score_delta(0), 2);
        assert_eq!(trainer_offensive_score_delta(1), 0);
    }

    #[test]
    fn opportunist_uses_exact_hp_boundaries_and_single_mid_hp_roll() {
        let mut rng = ScriptedRandom(VecDeque::from([127, 128]));
        assert!(!trainer_opportunist_discourages(51, 100, &mut rng));
        assert!(!trainer_opportunist_discourages(50, 100, &mut rng));
        assert!(trainer_opportunist_discourages(50, 100, &mut rng));
        assert!(trainer_opportunist_discourages(25, 100, &mut rng));
        assert!(rng.0.is_empty());
    }

    #[test]
    fn cautious_preserves_threshold_and_early_return_bug() {
        let names = [Some("MIST"), Some("TACKLE"), Some("SPIKES")];
        let mut scores = [20, 20, 20];
        let mut rng = ScriptedRandom(VecDeque::from([229, 230]));
        apply_trainer_cautious_scores(&mut scores, &names, 1, &mut rng);
        assert_eq!(scores, [21, 20, 20]);
        assert!(rng.0.is_empty());

        let mut first_turn_scores = [20];
        let mut untouched_rng = ScriptedRandom(VecDeque::from([7]));
        apply_trainer_cautious_scores(
            &mut first_turn_scores,
            &[Some("MIST")],
            0,
            &mut untouched_rng,
        );
        assert_eq!(untouched_rng.0.len(), 1);
    }

    #[test]
    fn status_checks_only_source_selected_immunities() {
        assert_eq!(
            trainer_status_score_delta(
                "POISON",
                0,
                &["POISON", "GRASS"],
                TrainerAiTypeMatchup::Neutral,
            ),
            10,
        );
        assert_eq!(
            trainer_status_score_delta("PARALYZE", 0, &["GROUND"], TrainerAiTypeMatchup::Immune,),
            10,
        );
        assert_eq!(
            trainer_status_score_delta("CONFUSE", 0, &["GHOST"], TrainerAiTypeMatchup::Immune,),
            0,
        );
        assert_eq!(
            trainer_status_score_delta("NONE", 40, &["GHOST"], TrainerAiTypeMatchup::Immune,),
            10,
        );
    }

    #[test]
    fn aggressive_uses_last_damage_tie_and_reckless_exceptions() {
        let moves = [
            Some(TrainerAiDamageEvaluation {
                effect: "NONE",
                power: 40,
                damage: 20,
            }),
            Some(TrainerAiDamageEvaluation {
                effect: "NONE",
                power: 50,
                damage: 30,
            }),
            Some(TrainerAiDamageEvaluation {
                effect: "NONE",
                power: 60,
                damage: 30,
            }),
            Some(TrainerAiDamageEvaluation {
                effect: "MULTI_HIT",
                power: 15,
                damage: 10,
            }),
        ];
        let mut scores = [20; 4];
        apply_trainer_aggressive_scores(&mut scores, &moves);
        assert_eq!(scores, [21, 21, 20, 20]);
    }

    #[test]
    fn risky_uses_source_hp_rng_and_strict_ko_thresholds() {
        let mut rng = ScriptedRandom(VecDeque::from([199, 200]));
        assert_eq!(
            trainer_risky_score_delta("SELFDESTRUCT", 200, 999, 10, 20, 20, &mut rng),
            0,
        );
        assert_eq!(rng.0.len(), 2);
        assert_eq!(
            trainer_risky_score_delta("SELFDESTRUCT", 200, 999, 10, 19, 20, &mut rng),
            0,
        );
        assert_eq!(
            trainer_risky_score_delta("SELFDESTRUCT", 200, 11, 10, 19, 20, &mut rng),
            -5,
        );
        assert_eq!(
            trainer_risky_score_delta("NONE", 40, 10, 10, 19, 20, &mut rng),
            0,
        );
        assert!(rng.0.is_empty());
    }

    #[test]
    fn ai_damage_uses_psywave_rejection_sampling_and_live_enemy_level() {
        let species = |id: &str| {
            let mut species =
                PokemonSpecies::new_for_tests(id, BaseStats::new(40, 40, 40, 40, 40, 40));
            species.type1 = pokemon_type("NORMAL");
            species.type2 = pokemon_type("NORMAL");
            species
        };
        let player = Pokemon::new_for_tests(species("PLAYER"), 20, Dv::from_non_hp(10, 10, 10, 10));
        let enemy = Pokemon::new_for_tests(species("ENEMY"), 20, Dv::from_non_hp(10, 10, 10, 10));
        let state = BattleCombatState::new(player, enemy);
        let move_data = Move {
            source_index: 149,
            name: "PSYWAVE".to_string(),
            move_type: pokemon_type("PSYCHIC_TYPE"),
            power: 1,
            accuracy: 80,
            pp: 15,
            effect: "PSYWAVE".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        };
        let mut rng = ScriptedRandom(VecDeque::from([0, 30, 29]));
        assert_eq!(
            trainer_ai_damage(
                &state,
                &move_data,
                &BattleStatMultiplierTables::default(),
                &TypeCategories::default(),
                &TypeEffectivenessTable::default(),
                &WeatherModifiers::default(),
                0,
                &mut rng,
            )
            .expect("constant AI damage"),
            29,
        );
        assert!(rng.0.is_empty());
    }

    #[test]
    fn ai_held_type_boost_requires_matching_type_and_valid_parameter() {
        assert_eq!(
            trainer_ai_held_type_boost_percent("HELD_FIRE_BOOST", 10, "FIRE"),
            Some(Ok(10)),
        );
        assert_eq!(
            trainer_ai_held_type_boost_percent("HELD_FIRE_BOOST", 10, "WATER"),
            None,
        );
        assert_eq!(
            trainer_ai_held_type_boost_percent("HELD_FIRE_BOOST", 0, "FIRE"),
            Some(Err(0)),
        );
    }

    #[test]
    fn smart_sleep_and_leech_hit_use_source_slot_rng_order() {
        let state = ai_test_state();
        let moves = [
            Some(TrainerSmartMove {
                move_id: "HYPNOSIS",
                effect: "SLEEP",
                power: 0,
                accuracy: 60,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "SING",
                effect: "SLEEP",
                power: 0,
                accuracy: 55,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "DREAM_EATER",
                effect: "DREAM_EATER",
                power: 100,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "MEGA_DRAIN",
                effect: "LEECH_HIT",
                power: 40,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::NotVeryEffective,
            }),
        ];
        let mut scores = [20; 4];
        let mut rng = ScriptedRandom(VecDeque::from([127, 128, 24, 99]));
        apply_trainer_smart_scores(&state, &mut scores, &moves, &[], &[], &[], &[], &mut rng);
        assert_eq!(scores, [20, 18, 20, 20]);
        assert!(rng.0.is_empty());
    }

    #[test]
    fn smart_hp_handlers_preserve_exact_boundaries() {
        let mut state = ai_test_state();
        state.enemy.hp = state.enemy.max_hp / 4;
        state.player.hp = state.player.max_hp / 2;
        let moves = [
            Some(TrainerSmartMove {
                move_id: "RECOVER",
                effect: "HEAL",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "TOXIC",
                effect: "TOXIC",
                power: 0,
                accuracy: 85,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "FISSURE",
                effect: "OHKO",
                power: 1,
                accuracy: 30,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
        ];
        let mut scores = [20; 3];
        let mut rng = ScriptedRandom(VecDeque::from([25]));
        apply_trainer_smart_scores(&state, &mut scores, &moves, &[], &[], &[], &[], &mut rng);
        assert_eq!(scores, [18, 21, 21]);
        assert!(rng.0.is_empty());
    }

    #[test]
    fn smart_selfdestruct_checks_both_reserve_parties_before_hp() {
        let mut state = ai_test_state();
        let mut player_reserve = state.player.clone();
        player_reserve.hp = 1;
        state.player_party.push(player_reserve);
        state.enemy.hp = state.enemy.max_hp / 4;
        let moves = [Some(TrainerSmartMove {
            move_id: "EXPLOSION",
            effect: "SELFDESTRUCT",
            power: 250,
            accuracy: 100,
            matchup: TrainerAiTypeMatchup::Neutral,
        })];
        let mut scores = [20];
        let mut rng = ScriptedRandom(VecDeque::new());
        apply_trainer_smart_scores(&state, &mut scores, &moves, &[], &[], &[], &[], &mut rng);
        assert_eq!(scores, [23]);
    }

    #[test]
    fn smart_lock_on_reweights_low_accuracy_moves_before_dismissing_itself() {
        let mut state = ai_test_state();
        state.player_lock_on_target = true;
        let moves = [
            Some(TrainerSmartMove {
                move_id: "THUNDER",
                effect: "THUNDER",
                power: 120,
                accuracy: 70,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "LOCK_ON",
                effect: "LOCK_ON",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
        ];
        let mut scores = [20; 2];
        let mut rng = ScriptedRandom(VecDeque::new());
        apply_trainer_smart_scores(&state, &mut scores, &moves, &[], &[], &[], &[], &mut rng);
        assert_eq!(scores, [18, 30]);
    }

    #[test]
    fn smart_sunny_day_preserves_omitted_solarbeam_source_bug() {
        let state = ai_test_state();
        let moves = [
            Some(TrainerSmartMove {
                move_id: "SUNNY_DAY",
                effect: "SUNNY_DAY",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "SOLARBEAM",
                effect: "SOLARBEAM",
                power: 120,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
        ];
        let mut scores = [20; 2];
        let mut rng = ScriptedRandom(VecDeque::new());
        apply_trainer_smart_scores(&state, &mut scores, &moves, &[], &[], &[], &[], &mut rng);
        assert_eq!(scores, [23, 20]);
    }

    #[test]
    fn smart_move_history_handlers_share_source_revealed_move_order() {
        let mut state = ai_test_state();
        state.enemy.speed = 100;
        state.player.speed = 50;
        state.player_last_move = Some("SUPER_FANG".to_string());
        state.player_last_counter_move = Some("SUPER_FANG".to_string());
        let moves = [
            Some(TrainerSmartMove {
                move_id: "MIRROR_MOVE",
                effect: "MIRROR_MOVE",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "COUNTER",
                effect: "COUNTER",
                power: 1,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "ENCORE",
                effect: "ENCORE",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "SPITE",
                effect: "SPITE",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
        ];
        let player_moves = [TrainerSmartPlayerMove {
            move_id: "SUPER_FANG",
            effect: "SUPER_FANG",
            power: 1,
            physical: true,
            matchup_against_enemy: TrainerAiTypeMatchup::Neutral,
            matchup_against_player: TrainerAiTypeMatchup::Neutral,
            current_pp: Some(5),
        }];
        let mut scores = [20; 4];
        let mut rng = ScriptedRandom(VecDeque::from([128, 25, 100, 70, 100]));
        apply_trainer_smart_scores(
            &state,
            &mut scores,
            &moves,
            &player_moves,
            &[],
            &[],
            &[],
            &mut rng,
        );
        assert_eq!(scores, [18, 19, 18, 18]);
        assert!(rng.0.is_empty());
    }

    #[test]
    fn smart_switch_score_preserves_check_type_matchup_current_move_bug() {
        let enemy_moves = [Some(TrainerSmartMove {
            move_id: "ROAR",
            effect: "FORCE_SWITCH",
            power: 0,
            accuracy: 100,
            matchup: TrainerAiTypeMatchup::Neutral,
        })];
        let player_moves = [TrainerSmartPlayerMove {
            move_id: "KARATE_CHOP",
            effect: "NORMAL_HIT",
            power: 50,
            physical: true,
            matchup_against_enemy: TrainerAiTypeMatchup::SuperEffective,
            matchup_against_player: TrainerAiTypeMatchup::Neutral,
            current_pp: Some(25),
        }];

        // CheckPlayerMoveTypeMatchups appears to iterate Karate Chop, but the
        // ASM's CheckTypeMatchup reloads Roar's current battle move type. Here
        // Normal is immune against the candidate Ghost, so the two halves are
        // +2 (player attacks) and -2 (no damaging enemy attacks): score 10.
        assert_eq!(
            trainer_smart_switch_score(
                &enemy_moves,
                &player_moves,
                1,
                TrainerAiTypeMatchup::Immune,
                TrainerAiTypeMatchup::Neutral,
            ),
            10,
        );
    }

    #[test]
    fn smart_encore_uses_last_move_then_last_counter_move() {
        let mut state = ai_test_state();
        state.enemy.speed = 100;
        state.player.speed = 50;
        state.player_last_move = Some("TACKLE".to_string());
        state.player_last_counter_move = Some("SWORDS_DANCE".to_string());
        let moves = [Some(TrainerSmartMove {
            move_id: "ENCORE",
            effect: "ENCORE",
            power: 0,
            accuracy: 100,
            matchup: TrainerAiTypeMatchup::Neutral,
        })];
        let player_moves = [
            TrainerSmartPlayerMove {
                move_id: "TACKLE",
                effect: "NORMAL_HIT",
                power: 35,
                physical: true,
                matchup_against_enemy: TrainerAiTypeMatchup::Neutral,
                matchup_against_player: TrainerAiTypeMatchup::Neutral,
                current_pp: Some(30),
            },
            TrainerSmartPlayerMove {
                move_id: "SWORDS_DANCE",
                effect: "ATTACK_UP_2",
                power: 0,
                physical: true,
                matchup_against_enemy: TrainerAiTypeMatchup::Neutral,
                matchup_against_player: TrainerAiTypeMatchup::Neutral,
                current_pp: Some(30),
            },
        ];
        let mut scores = [20];
        let mut rng = ScriptedRandom(VecDeque::from([70]));
        apply_trainer_smart_scores(
            &state,
            &mut scores,
            &moves,
            &player_moves,
            &[],
            &[],
            &[],
            &mut rng,
        );
        assert_eq!(scores, [18]);
        assert!(rng.0.is_empty());
    }

    #[test]
    fn smart_conversion2_leaves_player_turn_type_register_for_later_slots() {
        let mut state = ai_test_state();
        let reserve = state.player.clone();
        state.player_party = vec![state.player.clone(), reserve];
        state.player_party_index = 0;
        let moves = [
            Some(TrainerSmartMove {
                move_id: "CONVERSION2",
                effect: "CONVERSION2",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
            Some(TrainerSmartMove {
                move_id: "MEAN_LOOK",
                effect: "MEAN_LOOK",
                power: 0,
                accuracy: 100,
                matchup: TrainerAiTypeMatchup::SuperEffective,
            }),
            Some(TrainerSmartMove {
                move_id: "TACKLE",
                effect: "NORMAL_HIT",
                power: 35,
                accuracy: 95,
                matchup: TrainerAiTypeMatchup::Neutral,
            }),
        ];
        let player_moves = [TrainerSmartPlayerMove {
            move_id: "SCRATCH",
            effect: "NORMAL_HIT",
            power: 40,
            physical: true,
            matchup_against_enemy: TrainerAiTypeMatchup::Neutral,
            matchup_against_player: TrainerAiTypeMatchup::Neutral,
            current_pp: Some(35),
        }];
        let mut scores = [20; 3];
        let mut rng = ScriptedRandom(VecDeque::new());
        apply_trainer_smart_scores(
            &state,
            &mut scores,
            &moves,
            &player_moves,
            &[],
            &[
                TrainerAiTypeMatchup::Neutral,
                TrainerAiTypeMatchup::NotVeryEffective,
                TrainerAiTypeMatchup::Neutral,
            ],
            &[],
            &mut rng,
        );
        assert_eq!(scores, [20, 21, 20]);
    }

    #[test]
    fn smart_priority_requires_damage_strictly_greater_than_hp() {
        let mut state = ai_test_state();
        state.enemy.speed = 40;
        state.player.speed = 80;
        let moves = [Some(TrainerSmartMove {
            move_id: "QUICK_ATTACK",
            effect: "PRIORITY_HIT",
            power: 40,
            accuracy: 100,
            matchup: TrainerAiTypeMatchup::Neutral,
        })];
        let mut rng = ScriptedRandom(VecDeque::new());
        let mut scores = [20];
        apply_trainer_smart_scores(
            &state,
            &mut scores,
            &moves,
            &[],
            &[],
            &[],
            &[Some(state.player.hp)],
            &mut rng,
        );
        assert_eq!(scores, [20]);

        let mut scores = [20];
        apply_trainer_smart_scores(
            &state,
            &mut scores,
            &moves,
            &[],
            &[],
            &[],
            &[Some(state.player.hp + 1)],
            &mut rng,
        );
        assert_eq!(scores, [17]);
    }
}
