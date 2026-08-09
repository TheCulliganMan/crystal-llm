fn visible_battle_player_name(snapshot: &RuntimeShellSnapshot) -> &str {
    if snapshot
        .battle
        .as_ref()
        .is_some_and(|battle| battle.battle_type == "BATTLETYPE_TUTORIAL")
    {
        "DUDE"
    } else {
        &snapshot.trainer.player_name
    }
}

fn visible_player_withdraw_message(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    nickname: &str,
) -> String {
    let battle = snapshot
        .battle
        .as_ref()
        .expect("player withdrawal requires an active battle");
    let hp_at_send_out = runtime_shell
        .battle_enemy_hp_at_player_send_out
        .expect("player withdrawal requires the enemy HP captured at send-out");
    let damage = hp_at_send_out.saturating_sub(battle.enemy_pokemon.hp);
    let percent = if battle.enemy_pokemon.max_hp == 0 {
        0
    } else {
        u32::from(damage).saturating_mul(100) / u32::from(battle.enemy_pokemon.max_hp)
    };
    match percent {
        0 => format!("{nickname}, that's enough! Come back!"),
        1..=29 => format!("{nickname}, come back!"),
        30..=69 => format!("{nickname}, OK! Come back!"),
        _ => format!("{nickname}, good! Come back!"),
    }
}

fn queue_visible_player_recall_animation(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    trigger_message: &str,
) {
    let (animation_label, animation_frames, sound_events, cry_events, object_events, mut bg_events) =
        visible_battle_animation_definition(snapshot, "BattleAnim_ReturnMon".to_string(), 0)
            .expect("exported battle animations must contain BattleAnim_ReturnMon");
    const WITHDRAW_DELAY_FRAMES: u16 = 50;
    for event in &mut bg_events {
        event.frame = event.frame.saturating_add(WITHDRAW_DELAY_FRAMES);
    }
    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
        trigger_message: trigger_message.to_string(),
        move_id: "RETURN_MON".to_string(),
        animation_label,
        player_move: true,
        started: false,
        waiting_for_hp: false,
        frame: 0,
        total_frames: WITHDRAW_DELAY_FRAMES.saturating_add(animation_frames),
        sound_events: sound_events
            .into_iter()
            .map(|(frame, sound)| (frame.saturating_add(WITHDRAW_DELAY_FRAMES), sound))
            .collect(),
        next_sound_event: 0,
        cry_events: cry_events
            .into_iter()
            .map(|(frame, cry)| (frame.saturating_add(WITHDRAW_DELAY_FRAMES), cry))
            .collect(),
        next_cry_event: 0,
        object_events: object_events
            .into_iter()
            .map(|mut event| {
                event.frame = event.frame.saturating_add(WITHDRAW_DELAY_FRAMES);
                event
            })
            .collect(),
        bg_events,
        actor_species_override: None,
        actor_shiny_override: None,
    });
}

fn stage_visible_battle_messages(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    events: &[crate::core::battle::turn::BattleEvent],
) {
    use crate::core::battle::damage::Weather;
    use crate::core::battle::turn::{BattleEvent, BattleSide};
    use crate::core::models::Stat;

    let battle = snapshot
        .battle
        .as_ref()
        .expect("battle events require an active battle snapshot");
    let active_player_party_index = battle
        .active_player_party_index
        .expect("battle events require an active player party slot");
    let player_name = std::cell::RefCell::new(snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_player_party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .expect("active battle party slot must exist in the runtime snapshot")
        .to_string());
    let enemy_name = std::cell::RefCell::new(
        battle
            .enemy_pokemon
            .nickname
            .as_str()
            .to_string(),
    );
    let name = |side: BattleSide| match side {
        BattleSide::Player => player_name.borrow().clone(),
        BattleSide::Enemy => enemy_name.borrow().clone(),
    };
    let stat_name = |stat: &Stat| match stat {
        Stat::Hp => "HP",
        Stat::Attack => "ATTACK",
        Stat::Defense => "DEFENSE",
        Stat::Speed => "SPEED",
        Stat::SpecialAttack => "SPCL.ATK",
        Stat::SpecialDefense => "SPCL.DEF",
        Stat::Accuracy => "ACCURACY",
        Stat::Evasion => "EVASION",
    };
    let message_count_before = runtime_shell.battle_messages.len();
    let stage_message_scenes = message_count_before == 0;
    if stage_message_scenes {
        runtime_shell.battle_message_scenes.clear();
        runtime_shell.pending_battle_scenes_after_message.clear();
    }
    let mut event_scene = snapshot.clone();
    if let Some(battle) = event_scene.battle.as_mut() {
        for event in events {
            match event {
                BattleEvent::AirborneStarted { side, .. } => match side {
                    BattleSide::Player => battle.player_semi_invulnerable = false,
                    BattleSide::Enemy => battle.enemy_semi_invulnerable = false,
                },
                BattleEvent::AirborneEnded { side, .. } => match side {
                    BattleSide::Player => battle.player_semi_invulnerable = true,
                    BattleSide::Enemy => battle.enemy_semi_invulnerable = true,
                },
                _ => {}
            }
        }
    }
    let mut event_scene_baton_pass_sides = BTreeSet::new();
    let mut leech_seed_scene_starts = BTreeMap::new();
    let mut leech_seed_animation_triggers = BTreeMap::new();
    let mut residual_animation_triggers = BTreeMap::new();
    let mut last_sandstorm_boundary_message = None;
    let mut perish_song_result_staged = false;
    let mut held_escape_item = None;
    let mut effectiveness_text_shown = BTreeSet::new();
    let mut last_move_message_by_side = BTreeMap::new();
    let disobedience_nap = events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::StatusApplied { move_name, .. } if move_name == "DISOBEDIENCE_NAP"
        )
    });
    let disobedience_self_hit = events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::ConfusionSelfDamage { move_name, .. } if move_name == "DISOBEDIENCE"
        )
    });
    let forced_switches = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::ForceSwitchApplied {
                target, move_name, ..
            } => Some((*target, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let baton_passed_sides = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::BatonPassed { side, .. } => Some(*side),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let successful_force_switch_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::ForceSwitchApplied {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let failed_force_switch_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::ForceSwitchFailed {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let teleported_sides = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::MoveUsed { side, move_name } if move_name == "TELEPORT" => Some(*side),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let missed_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::Missed { side, move_name, .. } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let semi_invulnerable_misses = missed_moves
        .iter()
        .copied()
        .filter(|(_, move_name)| matches!(*move_name, "FLY" | "DIG"))
        .collect::<BTreeSet<_>>();
    let preparation_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::ChargeStarted { side, move_name }
            | BattleEvent::AirborneStarted { side, move_name } => {
                Some((*side, move_name.as_str()))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let charge_release_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::ChargeEnded { side, move_name }
            | BattleEvent::AirborneEnded { side, move_name } => {
                Some((*side, move_name.as_str()))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let present_heals = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::PresentHeal {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let time_based_heal_params = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::HealApplied {
                side,
                move_name,
                animation_param,
                ..
            } if matches!(move_name.as_str(), "MORNING_SUN" | "SYNTHESIS" | "MOONLIGHT") => {
                Some(((*side, move_name.as_str()), *animation_param))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let present_failures = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::PresentFailed {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let non_ghost_curse_successes = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::StatStageChanged {
                side, move_name, ..
            } if move_name == "CURSE" => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let curse_failures = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::MoveUsed { side, move_name } if move_name == "CURSE" => {
                let succeeded = events.iter().any(|candidate| match candidate {
                    BattleEvent::CurseApplied {
                        side: candidate_side,
                        move_name: candidate_move,
                        ..
                    }
                    | BattleEvent::StatStageChanged {
                        side: candidate_side,
                        move_name: candidate_move,
                        ..
                    } => candidate_side == side && candidate_move == move_name,
                    _ => false,
                });
                (!succeeded).then_some((*side, move_name.as_str()))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let selfdestruct_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::SelfdestructDamage {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let completed_multi_hits = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::MultiHitCount {
                side,
                move_name,
                hits,
                ..
            } => Some(((*side, move_name.as_str()), *hits)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let bide_starts = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::BideStarted {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let bide_storing_turns = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::BideStoring {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let command_owned_animation_failures = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::SubstituteFailed {
                side, move_name, ..
            }
            | BattleEvent::TransformFailed {
                side, move_name, ..
            }
            | BattleEvent::MimicFailed {
                side, move_name, ..
            }
            | BattleEvent::SketchFailed {
                side, move_name, ..
            }
            | BattleEvent::ConversionFailed { side, move_name }
            | BattleEvent::Conversion2Failed { side, move_name }
            | BattleEvent::MetronomeFailed { side, move_name }
            | BattleEvent::SleepTalkFailed { side, move_name }
            | BattleEvent::MirrorMoveFailed {
                side, move_name, ..
            }
            | BattleEvent::TeleportFailed { side, move_name }
            | BattleEvent::BideFailed { side, move_name }
            | BattleEvent::HealFailed {
                side, move_name, ..
            }
            | BattleEvent::FutureSightFailed {
                side, move_name, ..
            }
            | BattleEvent::MistFailed { side, move_name }
            | BattleEvent::FocusEnergyFailed { side, move_name }
            | BattleEvent::SpikesFailed {
                side, move_name, ..
            }
            | BattleEvent::ForesightFailed {
                side, move_name, ..
            }
            | BattleEvent::PerishSongFailed {
                side, move_name, ..
            }
            | BattleEvent::ProtectFailed {
                side, move_name, ..
            }
            | BattleEvent::EndureFailed {
                side, move_name, ..
            }
            | BattleEvent::SafeguardFailed {
                side, move_name, ..
            }
            | BattleEvent::NightmareFailed {
                side, move_name, ..
            }
            | BattleEvent::SpiteFailed {
                side, move_name, ..
            }
            | BattleEvent::DisableFailed {
                side, move_name, ..
            }
            | BattleEvent::EncoreFailed {
                side, move_name, ..
            }
            | BattleEvent::ReflectFailed {
                side, move_name, ..
            }
            | BattleEvent::LightScreenFailed {
                side, move_name, ..
            }
            | BattleEvent::AttractFailed {
                side, move_name, ..
            }
            | BattleEvent::OhkoFailed {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let unsuccessful_pure_stat_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::StatStageFailed {
                side, move_name, ..
            }
            | BattleEvent::StatStageUnchanged {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .filter(|key| {
            let has_success = events.iter().any(|event| {
                matches!(
                    event,
                    BattleEvent::StatStageChanged { side, move_name, .. }
                        if (*side, move_name.as_str()) == *key
                )
            });
            let is_pure_status = snapshot.moves.iter().any(|move_data| {
                (move_data.move_id == key.1 || move_data.name == key.1)
                    && move_data.power == 0
            });
            !has_success && is_pure_status
        })
        .collect::<BTreeSet<_>>();
    let called_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::MetronomeSelected {
                side,
                move_name,
                selected_move,
                ..
            }
            | BattleEvent::SleepTalkSelected {
                side,
                move_name,
                selected_move,
                ..
            } => Some(((*side, move_name.as_str()), selected_move.as_str())),
            BattleEvent::MirrorMoveSelected {
                side,
                move_name,
                copied_move,
            } => Some(((*side, move_name.as_str()), copied_move.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let queued_future_sight_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::FutureSightQueued {
                side, move_name, ..
            } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut baton_pass_sides = BTreeSet::new();
    let mut deferred_airborne_end_scenes = BTreeMap::new();
    for event in events {
        let event_scene_before = event_scene.clone();
        if let BattleEvent::AirborneEnded { side, move_name } = event
            && semi_invulnerable_misses.contains(&(*side, move_name.as_str()))
            && event_scene.battle.as_ref().is_some_and(|battle| match side {
                BattleSide::Player => battle.player_substitute_hp == 0,
                BattleSide::Enemy => battle.enemy_substitute_hp == 0,
            })
        {
            // FailureText prints the miss first, then AppearUserRaiseSub
            // redraws Fly/Dig's user. Keep the hidden scene under that page
            // and retain the revealed scene for its acknowledgement.
            let mut revealed_scene = event_scene.clone();
            apply_visible_battle_event_to_scene(
                &mut revealed_scene,
                event,
                &mut event_scene_baton_pass_sides,
            );
            deferred_airborne_end_scenes
                .insert((*side, move_name.as_str()), revealed_scene);
        } else {
            apply_visible_battle_event_to_scene(
                &mut event_scene,
                event,
                &mut event_scene_baton_pass_sides,
            );
        }
        let event_message_count_before = runtime_shell.battle_messages.len();
        if let BattleEvent::HeldItemEscape { side, item_id, .. } = event {
            held_escape_item = Some((*side, item_id.clone()));
            continue;
        }
        if matches!(
            event,
            BattleEvent::PerishSongApplied { .. } | BattleEvent::PerishSongFailed { .. }
        ) {
            if perish_song_result_staged {
                continue;
            }
            perish_song_result_staged = true;
        }
        if let BattleEvent::MoveUsed { side, move_name } = event
            && bide_storing_turns.contains(&(*side, move_name.as_str()))
        {
            // StoreEnergy returns after StoringEnergyText on intermediate
            // turns, before Crystal reaches usedmovetext or moveanim.
            continue;
        }
        if let BattleEvent::RapidSpinCleared {
            side,
            trap_move,
            cleared_leech_seed,
            cleared_spikes,
            ..
        } = event
        {
            if *cleared_leech_seed {
                runtime_shell
                    .battle_messages
                    .push_back(format!("{} shed LEECH SEED!", name(*side)));
            }
            if *cleared_spikes {
                runtime_shell
                    .battle_messages
                    .push_back(format!("{} blew away SPIKES!", name(*side)));
            }
            if let Some(trap_move) = trap_move {
                runtime_shell.battle_messages.push_back(format!(
                    "{} was released by {}!",
                    name(*side),
                    battle_move_display_name(snapshot, trap_move)
                ));
            }
            if stage_message_scenes {
                for _ in event_message_count_before..runtime_shell.battle_messages.len() {
                    runtime_shell
                        .battle_message_scenes
                        .push_back(Box::new(event_scene.clone()));
                }
            }
            continue;
        }
        if let BattleEvent::Damage {
            side,
            move_name,
            critical,
            result,
            ..
        } = event
        {
            if snapshot
                .moves
                .iter()
                .any(|move_data| {
                    (move_data.move_id == *move_name || move_data.name == *move_name)
                        && move_data.effect == "OHKO"
                })
            {
                runtime_shell
                    .battle_messages
                    .push_back("It's a one-hit KO!".to_string());
            }
            if *critical {
                runtime_shell
                    .battle_messages
                    .push_back("A critical hit!".to_string());
            }
            let side_key = match side {
                BattleSide::Player => 0u8,
                BattleSide::Enemy => 1u8,
            };
            if effectiveness_text_shown.insert((side_key, move_name.as_str())) {
                if result.type_multiplier.numerator > result.type_multiplier.denominator {
                    runtime_shell
                        .battle_messages
                        .push_back("It's super effective!".to_string());
                } else if result.type_multiplier.numerator < result.type_multiplier.denominator {
                    runtime_shell
                        .battle_messages
                        .push_back("It's not very effective...".to_string());
                }
            }
            if stage_message_scenes {
                for _ in event_message_count_before..runtime_shell.battle_messages.len() {
                    runtime_shell
                        .battle_message_scenes
                        .push_back(Box::new(event_scene.clone()));
                }
                if event_message_count_before == runtime_shell.battle_messages.len() {
                    // Neutral, non-critical damage has no follow-up textbox in
                    // Crystal. Retain the pre-hit frame under the "used"
                    // line and apply damage only when that exact page closes.
                    if let Some(trigger_message) = runtime_shell.battle_messages.back().cloned() {
                        runtime_shell
                            .pending_battle_scenes_after_message
                            .push_back((trigger_message, Box::new(event_scene.clone())));
                    }
                }
            }
            continue;
        }
        let message = match event {
            BattleEvent::AutomaticStruggle { side } => {
                let message = format!("{} has no moves left!", name(*side));
                runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                    trigger_message: message.clone(),
                    move_id: "STRUGGLE".to_string(),
                    animation_label: "BattleCommand_NoMovesDelay".to_string(),
                    player_move: *side == BattleSide::Player,
                    started: false,
                    waiting_for_hp: false,
                    frame: 0,
                    total_frames: 60,
                    sound_events: Vec::new(),
                    next_sound_event: 0,
                    cry_events: Vec::new(),
                    next_cry_event: 0,
                    object_events: Vec::new(),
                    bg_events: Vec::new(),
                    actor_species_override: None,
                    actor_shiny_override: None,
                });
                Some(message)
            }
            BattleEvent::MoveUsed { side, move_name } => {
                let message = format!(
                    "{}\nused {}!",
                    name(*side),
                    battle_move_display_name(snapshot, move_name)
                );
                last_move_message_by_side.insert(*side, message.clone());
                let user_has_substitute = event_scene.battle.as_ref().is_some_and(|battle| {
                    match side {
                        BattleSide::Player => battle.player_substitute_hp > 0,
                        BattleSide::Enemy => battle.enemy_substitute_hp > 0,
                    }
                });
                if move_name != "BEAT_UP"
                    && (!missed_moves.contains(&(*side, move_name.as_str()))
                        || selfdestruct_moves.contains(&(*side, move_name.as_str())))
                    && !present_failures.contains(&(*side, move_name.as_str()))
                    && !curse_failures.contains(&(*side, move_name.as_str()))
                    && !failed_force_switch_moves.contains(&(*side, move_name.as_str()))
                    && !bide_storing_turns.contains(&(*side, move_name.as_str()))
                    && !command_owned_animation_failures
                        .contains(&(*side, move_name.as_str()))
                    && !queued_future_sight_moves.contains(&(*side, move_name.as_str()))
                    && !unsuccessful_pure_stat_moves
                        .contains(&(*side, move_name.as_str()))
                {
                    // Crystal passes parameter 1 on setup turns, initial Bide,
                    // non-Ghost Curse, successful forced switches, and
                    // Selfdestruct/Explosion, and parameter 3 through
                    // Present's healing branch. Their failed branches use
                    // AnimateFailedMove and never run the ordinary move
                    // animation.
                    let animation_param = if let Some(animation_param) = time_based_heal_params
                        .get(&(*side, move_name.as_str()))
                    {
                        *animation_param
                    } else if present_heals
                        .contains(&(*side, move_name.as_str()))
                    {
                        3
                    } else if non_ghost_curse_successes
                        .contains(&(*side, move_name.as_str()))
                    {
                        1
                    } else if successful_force_switch_moves
                        .contains(&(*side, move_name.as_str()))
                    {
                        1
                    } else if selfdestruct_moves.contains(&(*side, move_name.as_str())) {
                        1
                    } else if bide_starts.contains(&(*side, move_name.as_str())) {
                        1
                    } else {
                        u8::from(
                            preparation_moves.contains(&(*side, move_name.as_str())),
                        )
                    };
                    let animation_params = completed_multi_hits
                        .get(&(*side, move_name.as_str()))
                        .map(|hits| {
                            (0..*hits)
                                .map(|hit| {
                                    if move_name == "TRIPLE_KICK" {
                                        hit
                                    } else {
                                        (hit + 1) & 1
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_else(|| vec![animation_param]);
                    let animation_count = animation_params.len();
                    for (animation_index, animation_param) in
                        animation_params.into_iter().enumerate()
                    {
                        let move_key = (*side, move_name.as_str());
                        let lower_substitute = user_has_substitute
                            && animation_index == 0
                            && !charge_release_moves.contains(&move_key);
                        let raise_substitute = user_has_substitute
                            && animation_index + 1 == animation_count
                            && !(preparation_moves.contains(&move_key)
                                && matches!(move_name.as_str(), "FLY" | "DIG"));
                        if let Some((animation_label, total_frames, sound_events, cry_events, object_events, mut bg_events)) =
                            visible_move_animation_definition_with_substitute(
                                snapshot,
                                move_name,
                                i32::from(animation_param),
                                lower_substitute,
                                raise_substitute,
                            )
                        {
                            if user_has_substitute
                                && animation_count > 1
                                && animation_index > 0
                            {
                                bg_events.insert(0, VisibleMoveBgEvent {
                                    frame: 0,
                                    effect_id: "BATTLE_ACTOR_DROPSUB".to_string(),
                                    duration: 0,
                                    target: "BG_EFFECT_USER".to_string(),
                                    param: 0,
                                    incremented: false,
                                });
                            }
                            runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                                trigger_message: message.clone(),
                                move_id: move_name.clone(),
                                animation_label,
                                player_move: *side == BattleSide::Player,
                                started: false,
                                waiting_for_hp: false,
                                frame: 0,
                                total_frames,
                                sound_events,
                                next_sound_event: 0,
                                cry_events,
                                next_cry_event: 0,
                                object_events,
                                bg_events,
                                actor_species_override: None,
                                actor_shiny_override: None,
                            });
                        }
                    }
                    let mut next_called_move =
                        called_moves.get(&(*side, move_name.as_str())).copied();
                    let mut seen_called_moves = BTreeSet::new();
                    while let Some(called_move) = next_called_move {
                        if !seen_called_moves.insert(called_move) {
                            break;
                        }
                        let called_key = (*side, called_move);
                        let called_failed = (missed_moves.contains(&called_key)
                            && !selfdestruct_moves.contains(&called_key))
                            || present_failures.contains(&called_key)
                            || curse_failures.contains(&called_key)
                            || failed_force_switch_moves.contains(&called_key)
                            || command_owned_animation_failures.contains(&called_key);
                        if !called_failed {
                            let called_param = if let Some(animation_param) =
                                time_based_heal_params.get(&called_key)
                            {
                                *animation_param
                            } else if present_heals.contains(&called_key) {
                                3
                            } else if non_ghost_curse_successes.contains(&called_key)
                                || successful_force_switch_moves.contains(&called_key)
                                || selfdestruct_moves.contains(&called_key)
                                || bide_starts.contains(&called_key)
                                || preparation_moves.contains(&called_key)
                            {
                                1
                            } else {
                                0
                            };
                            let called_params = completed_multi_hits
                                .get(&called_key)
                                .map(|hits| {
                                    (0..*hits)
                                        .map(|hit| {
                                            if called_move == "TRIPLE_KICK" {
                                                hit
                                            } else {
                                                (hit + 1) & 1
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_else(|| vec![called_param]);
                            let called_animation_count = called_params.len();
                            for (called_animation_index, called_param) in
                                called_params.into_iter().enumerate()
                            {
                                let lower_substitute = user_has_substitute
                                    && called_animation_index == 0
                                    && !charge_release_moves.contains(&called_key);
                                let raise_substitute = user_has_substitute
                                    && called_animation_index + 1 == called_animation_count
                                    && !(preparation_moves.contains(&called_key)
                                        && matches!(called_move, "FLY" | "DIG"));
                                if let Some((animation_label, total_frames, sound_events, cry_events, object_events, mut bg_events)) =
                                    visible_move_animation_definition_with_substitute(
                                        snapshot,
                                        called_move,
                                        i32::from(called_param),
                                        lower_substitute,
                                        raise_substitute,
                                    )
                                {
                                    if user_has_substitute
                                        && called_animation_count > 1
                                        && called_animation_index > 0
                                    {
                                        bg_events.insert(0, VisibleMoveBgEvent {
                                            frame: 0,
                                            effect_id: "BATTLE_ACTOR_DROPSUB".to_string(),
                                            duration: 0,
                                            target: "BG_EFFECT_USER".to_string(),
                                            param: 0,
                                            incremented: false,
                                        });
                                    }
                                    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                                        trigger_message: message.clone(),
                                        move_id: called_move.to_string(),
                                        animation_label,
                                        player_move: *side == BattleSide::Player,
                                        started: false,
                                        waiting_for_hp: false,
                                        frame: 0,
                                        total_frames,
                                        sound_events,
                                        next_sound_event: 0,
                                        cry_events,
                                        next_cry_event: 0,
                                        object_events,
                                        bg_events,
                                        actor_species_override: None,
                                        actor_shiny_override: None,
                                    });
                                }
                            }
                        } else {
                            let delayed_animation = if user_has_substitute {
                                if semi_invulnerable_misses.contains(&called_key) {
                                    visible_substitute_raise_after_delay_definition(snapshot)
                                } else {
                                    visible_substitute_move_delay_definition(snapshot)
                                }
                            } else {
                                Some((
                                    "BattleCommand_MoveDelay".to_string(),
                                    40,
                                    Vec::new(),
                                    Vec::new(),
                                    Vec::new(),
                                    Vec::new(),
                                ))
                            };
                            if let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                                delayed_animation
                            {
                                runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                                    trigger_message: message.clone(),
                                    move_id: called_move.to_string(),
                                    animation_label,
                                    player_move: *side == BattleSide::Player,
                                    started: false,
                                    waiting_for_hp: false,
                                    frame: 0,
                                    total_frames,
                                    sound_events,
                                    next_sound_event: 0,
                                    cry_events,
                                    next_cry_event: 0,
                                    object_events,
                                    bg_events,
                                    actor_species_override: None,
                                    actor_shiny_override: None,
                                });
                            }
                        }
                        next_called_move = called_moves.get(&called_key).copied();
                    }
                } else if (missed_moves.contains(&(*side, move_name.as_str()))
                    && !selfdestruct_moves.contains(&(*side, move_name.as_str())))
                    || present_failures.contains(&(*side, move_name.as_str()))
                    || curse_failures.contains(&(*side, move_name.as_str()))
                    || failed_force_switch_moves.contains(&(*side, move_name.as_str()))
                    || command_owned_animation_failures
                        .contains(&(*side, move_name.as_str()))
                    || unsuccessful_pure_stat_moves
                        .contains(&(*side, move_name.as_str()))
                {
                    // MoveAnimNoSub turns a miss into MoveDelay, while these
                    // command-owned failures call AnimateFailedMove or its
                    // explicit LowerSub/MoveDelay/RaiseSub equivalent. Both
                    // retain the used page for exactly 40 frames before the
                    // failure result is exposed when no Substitute is active.
                    let delayed_animation = if user_has_substitute {
                        if semi_invulnerable_misses
                            .contains(&(*side, move_name.as_str()))
                        {
                            visible_substitute_raise_after_delay_definition(snapshot)
                        } else {
                            visible_substitute_move_delay_definition(snapshot)
                        }
                    } else {
                        Some((
                            "BattleCommand_MoveDelay".to_string(),
                            40,
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                        ))
                    };
                    if let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                        delayed_animation
                    {
                        runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                            trigger_message: message.clone(),
                            move_id: move_name.clone(),
                            animation_label,
                            player_move: *side == BattleSide::Player,
                            started: false,
                            waiting_for_hp: false,
                            frame: 0,
                            total_frames,
                            sound_events,
                            next_sound_event: 0,
                            cry_events,
                            next_cry_event: 0,
                            object_events,
                            bg_events,
                            actor_species_override: None,
                            actor_shiny_override: None,
                        });
                    }
                }
                Some(message)
            }
            BattleEvent::NoPp {
                side, move_name, ..
            } => Some(format!(
                "{}\nhas no PP left for\n{}!",
                name(*side),
                battle_move_display_name(snapshot, move_name)
            )),
            BattleEvent::Missed { side, .. } => {
                Some(format!("{}'s\nattack missed!", name(*side)))
            }
            BattleEvent::AirborneAvoided { target, .. } => {
                Some(format!("{}\nevaded the attack!", name(*target)))
            }
            BattleEvent::NoEffect { side, .. } => {
                Some(format!("It doesn't affect\n{}!", name(side.other())))
            }
            BattleEvent::MagnitudePower { power, .. } => {
                let magnitude = match power {
                    0..=10 => 4,
                    11..=30 => 5,
                    31..=50 => 6,
                    51..=70 => 7,
                    71..=90 => 8,
                    91..=110 => 9,
                    _ => 10,
                };
                Some(format!("Magnitude {magnitude}!"))
            }
            BattleEvent::BeatUpParticipant {
                side,
                move_name,
                party_index,
                species,
                nickname,
                shiny,
            } => {
                let message = format!("{nickname}'s\nattack!");
                let active_index = snapshot.battle.as_ref().and_then(|battle| match side {
                    BattleSide::Player => battle.active_player_party_index,
                    BattleSide::Enemy => battle.active_enemy_party_index,
                });
                let animation_param = if active_index == Some(*party_index) { 0 } else { 1 };
                if let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                    visible_move_animation_definition(snapshot, move_name, animation_param)
                {
                    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                        trigger_message: message.clone(),
                        move_id: move_name.clone(),
                        animation_label,
                        player_move: *side == BattleSide::Player,
                        started: false,
                        waiting_for_hp: false,
                        frame: 0,
                        total_frames,
                        sound_events,
                        next_sound_event: 0,
                        cry_events,
                        next_cry_event: 0,
                        object_events,
                        bg_events,
                        actor_species_override: (animation_param != 0).then(|| species.clone()),
                        actor_shiny_override: (animation_param != 0).then_some(*shiny),
                    });
                }
                Some(message)
            }
            BattleEvent::MultiHitCount { hits, .. } => {
                Some(format!("Hit {hits} times!"))
            }
            BattleEvent::PayDayMoney { .. } => {
                Some("Coins scattered\neverywhere!".to_string())
            }
            BattleEvent::PresentFailed { target, .. } => {
                Some(format!("{} refused\nthe gift!", name(*target)))
            }
            BattleEvent::TeleportFailed { .. }
            | BattleEvent::StatusHealFailed { .. }
            | BattleEvent::ConfusionFailed { .. }
            | BattleEvent::LeechSeedFailed { .. }
            | BattleEvent::CurseFailed { .. }
            | BattleEvent::ConversionFailed { .. }
            | BattleEvent::Conversion2Failed { .. }
            | BattleEvent::MetronomeFailed { .. }
            | BattleEvent::MimicFailed { .. }
            | BattleEvent::SketchFailed { .. }
            | BattleEvent::SleepTalkFailed { .. }
            | BattleEvent::MirrorMoveFailed { .. }
            | BattleEvent::ForceSwitchFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::TrapFailed { .. } => None,
            BattleEvent::LeechSeedImmune { target, .. } => {
                Some(format!("It doesn't affect\n{}!", name(*target)))
            }
            BattleEvent::OhkoFailed { side, reason, .. } => Some(match reason {
                crate::core::battle::turn::OhkoFailureReason::TargetLevelTooHigh { .. } => {
                    format!("{}'s unaffected!", name(side.other()))
                }
                crate::core::battle::turn::OhkoFailureReason::Missed { .. } => {
                    "The attack missed!".to_string()
                }
            }),
            BattleEvent::Disobeyed { .. } if disobedience_nap => None,
            BattleEvent::Disobeyed { side } if disobedience_self_hit => {
                Some(format!("{} won't\nobey!", name(*side)))
            }
            BattleEvent::Disobeyed { side } => {
                Some(format!("{} ignored\norders!", name(*side)))
            }
            BattleEvent::DisobedienceIdle { side, roll } => Some(format!(
                "{} {}",
                name(*side),
                match roll {
                    0 => "is\nloafing around.",
                    1 => "won't\nobey!",
                    2 => "turned\naway!",
                    _ => "ignored\norders!",
                }
            )),
            BattleEvent::DisobedienceIgnoredSleeping { side } => {
                Some(format!("{} ignored\norders…sleeping!", name(*side)))
            }
            BattleEvent::StatusApplied {
                target,
                status,
                move_name,
                side,
            } => {
                if move_name == "REST" {
                    None
                } else {
                if let Some(label) = match status.as_str() {
                    "POISON" | "BAD_POISON" => Some("BattleAnim_Psn"),
                    "BURN" => Some("BattleAnim_Brn"),
                    "PARALYSIS" => Some("BattleAnim_Par"),
                    "FREEZE" => Some("BattleAnim_Frz"),
                    _ => None,
                } && let Some(trigger_message) = last_move_message_by_side.get(side)
                {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        trigger_message,
                        *target,
                        label,
                        false,
                    );
                }
                let message = match status.as_str() {
                    "SLEEP" if move_name == "DISOBEDIENCE_NAP" => {
                        format!("{} began\nto nap!", name(*target))
                    }
                    "SLEEP" => format!("{}\nfell asleep!", name(*target)),
                    "POISON" => format!("{}\nwas poisoned!", name(*target)),
                    "BAD_POISON" => format!("{}'s\nbadly poisoned!", name(*target)),
                    "BURN" => format!("{}\nwas burned!", name(*target)),
                    "PARALYSIS" => format!(
                        "{}'s\nparalyzed! Maybe\nit can't attack!",
                        name(*target)
                    ),
                    "FREEZE" => format!("{}\nwas frozen solid!", name(*target)),
                    _ => unreachable!("core emitted unsupported battle status {status}"),
                };
                Some(message)
                }
            }
            BattleEvent::StatusFailed {
                target,
                existing_status: None,
                ..
            } => Some(format!("It didn't affect\n{}!", name(*target))),
            BattleEvent::StatusFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::StatusHealed {
                target,
                move_name,
                status_before,
                ..
            } => {
                if move_name == "DEFROST" || status_before == "FREEZE" {
                    Some(format!("{}\nwas defrosted!", name(*target)))
                } else {
                    Some(format!("{}'s\n{} was cured!", name(*target), status_before))
                }
            }
            BattleEvent::HealBellChimed { .. } => Some("A bell chimed!\n".to_string()),
            BattleEvent::StatusImmune { target, .. } => {
                Some(format!("It doesn't affect\n{}!", name(*target)))
            }
            BattleEvent::ResidualStatusDamage { side, status, .. } => {
                let message = if status == "BURN" {
                    format!("{}'s\nhurt by its burn!", name(*side))
                } else {
                    format!("{}\nis hurt by poison!", name(*side))
                };
                queue_visible_status_animation(
                    runtime_shell,
                    snapshot,
                    &message,
                    *side,
                    if status == "BURN" {
                        "BattleAnim_Brn"
                    } else {
                        "BattleAnim_Psn"
                    },
                    false,
                );
                Some(message)
            }
            BattleEvent::StatStageChanged {
                target,
                stat,
                amount,
                ..
            } => {
                let change = if *amount > 1 {
                    "went way up"
                } else if *amount > 0 {
                    "went up"
                } else if *amount < -1 {
                    "sharply fell"
                } else {
                    "fell"
                };
                Some(format!("{}'s\n{} {}!", name(*target), stat_name(stat), change))
            }
            BattleEvent::RageBuilding { side, .. } => {
                Some(format!("{}'s\nRAGE is building!", name(*side)))
            }
            BattleEvent::StatStageUnchanged {
                target,
                stat,
                amount,
                ..
            } => Some(format!(
                "{}'s\n{} won't\n{} anymore!",
                name(*target),
                stat_name(stat),
                if *amount >= 0 { "rise" } else { "drop" }
            )),
            BattleEvent::StatStageFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::ReflectApplied { side, .. } => {
                Some(format!("{}'s\nDEFENSE rose!", name(*side)))
            }
            BattleEvent::LightScreenApplied { side, .. } => {
                Some(format!("{}'s\nSPCL.DEF rose!", name(*side)))
            }
            BattleEvent::SafeguardApplied { side, .. } => {
                Some(format!("{}'s\ncovered by a veil!", name(*side)))
            }
            BattleEvent::SafeguardProtected { target, .. } => {
                Some(format!("{}\nis protected by\nSAFEGUARD!", name(*target)))
            }
            BattleEvent::SafeguardFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::SafeguardCount {
                side,
                turns_remaining: 0,
            } => Some(format!(
                "{} <PKMN>'s\nSAFEGUARD faded!",
                if *side == BattleSide::Player {
                    "Your"
                } else {
                    "Enemy"
                }
            )),
            BattleEvent::ReflectCount {
                side,
                turns_remaining: 0,
            } => Some(format!(
                "{} <PKMN>'s\nREFLECT faded!",
                if *side == BattleSide::Player {
                    "Your"
                } else {
                    "Enemy"
                }
            )),
            BattleEvent::LightScreenCount {
                side,
                turns_remaining: 0,
            } => Some(format!(
                "{} <PKMN>'s\nLIGHT SCREEN fell!",
                if *side == BattleSide::Player {
                    "Your"
                } else {
                    "Enemy"
                }
            )),
            BattleEvent::ReflectFailed { .. } | BattleEvent::LightScreenFailed { .. } => {
                Some("But it failed!".to_string())
            }
            BattleEvent::MistApplied { side, .. } => {
                Some(format!("{}'s\nshrouded in MIST!", name(*side)))
            }
            BattleEvent::MistProtected { target, .. } => {
                Some(format!("{}'s\nprotected by MIST.", name(*target)))
            }
            BattleEvent::MistFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::LeechSeedApplied { target, .. } => {
                Some(format!("{}\nwas seeded!", name(*target)))
            }
            BattleEvent::LeechSeedDamage { side, source, .. } => {
                let message = format!("LEECH SEED saps\n{}!", name(*side));
                let prior_message = runtime_shell.battle_messages.back().cloned();
                let animation_trigger = prior_message
                    .clone()
                    .unwrap_or_else(|| message.clone());
                leech_seed_animation_triggers
                    .insert((*source, *side), animation_trigger.clone());
                queue_visible_status_animation(
                    runtime_shell,
                    snapshot,
                    &animation_trigger,
                    *source,
                    "BattleAnim_Sap",
                    prior_message.is_none(),
                );
                if events.iter().any(|event| {
                    matches!(
                        event,
                        BattleEvent::LeechSeedDrain { side: drain_side, target, .. }
                            if drain_side == source && target == side
                    )
                }) {
                    for boundary in ["RESTORE", "COMPLETE"] {
                        runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                            trigger_message: animation_trigger.clone(),
                            move_id: format!("LEECH_SEED_{boundary}"),
                            animation_label: format!("BattleCommand_LeechSeed{boundary}"),
                            player_move: *source == BattleSide::Player,
                            started: false,
                            waiting_for_hp: false,
                            frame: 0,
                            total_frames: 1,
                            sound_events: Vec::new(),
                            next_sound_event: 0,
                            cry_events: Vec::new(),
                            next_cry_event: 0,
                            object_events: Vec::new(),
                            bg_events: Vec::new(),
                            actor_species_override: None,
                            actor_shiny_override: None,
                        });
                    }
                }
                Some(message)
            }
            BattleEvent::CurseDamage { side, .. } => {
                let message = format!("{}'s\nhurt by the CURSE!", name(*side));
                let prior_message = runtime_shell.battle_messages.back().cloned();
                let trigger = prior_message.clone().unwrap_or_else(|| message.clone());
                residual_animation_triggers.insert(("CURSE", *side), trigger.clone());
                queue_visible_status_animation(
                    runtime_shell,
                    snapshot,
                    &trigger,
                    *side,
                    "BattleAnim_InNightmare",
                    prior_message.is_none(),
                );
                queue_visible_terminal_animation_boundary(
                    runtime_shell,
                    &trigger,
                    *side,
                    "CURSE_DAMAGE",
                );
                Some(message)
            }
            BattleEvent::CurseApplied { side, target, .. } => {
                runtime_shell.battle_messages.push_back(format!(
                    "{}\ncut its own HP and",
                    name(*side)
                ));
                Some(format!("put a CURSE on\n{}!", name(*target)))
            }
            BattleEvent::NightmareDamage { side, .. } => {
                let message = format!("{}\nhas a NIGHTMARE!", name(*side));
                let prior_message = runtime_shell.battle_messages.back().cloned();
                let trigger = prior_message.clone().unwrap_or_else(|| message.clone());
                residual_animation_triggers.insert(("NIGHTMARE", *side), trigger.clone());
                queue_visible_status_animation(
                    runtime_shell,
                    snapshot,
                    &trigger,
                    *side,
                    "BattleAnim_InNightmare",
                    prior_message.is_none(),
                );
                queue_visible_terminal_animation_boundary(
                    runtime_shell,
                    &trigger,
                    *side,
                    "NIGHTMARE_DAMAGE",
                );
                Some(message)
            }
            BattleEvent::SpikesDamage { side, .. } => {
                Some(format!("{}'s\nhurt by SPIKES!", name(*side)))
            }
            BattleEvent::FutureSightLanded {
                side,
                source,
                move_name,
            } => {
                let message = format!("{}\nwas hit by FUTURE\nSIGHT!", name(*side));
                if let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                    visible_move_animation_definition(snapshot, move_name, 0)
                {
                    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                        trigger_message: message.clone(),
                        move_id: move_name.clone(),
                        animation_label,
                        player_move: *source == BattleSide::Player,
                        started: false,
                        waiting_for_hp: false,
                        frame: 0,
                        total_frames,
                        sound_events,
                        next_sound_event: 0,
                        cry_events,
                        next_cry_event: 0,
                        object_events,
                        bg_events,
                        actor_species_override: None,
                        actor_shiny_override: None,
                    });
                }
                Some(message)
            }
            BattleEvent::SandstormDamage { side, .. } => {
                let message = format!("The SANDSTORM hits\n{}!", name(*side));
                if let Some(trigger_message) = last_sandstorm_boundary_message.as_deref() {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        trigger_message,
                        side.other(),
                        "BattleAnim_InSandstorm",
                        false,
                    );
                }
                last_sandstorm_boundary_message = Some(message.clone());
                Some(message)
            }
            BattleEvent::SubstituteCreated { side, .. } => {
                Some(format!("{}\nmade a SUBSTITUTE!", name(*side)))
            }
            BattleEvent::SubstituteDamaged { target, .. } => {
                Some(format!("The SUBSTITUTE\ntook damage for\n{}!", name(*target)))
            }
            BattleEvent::SubstituteBroken { target, .. } => {
                Some(format!("{}'s\nSUBSTITUTE faded!", name(*target)))
            }
            BattleEvent::SubstituteBlocked { target, .. } => {
                Some(format!("It didn't affect\n{}!", name(*target)))
            }
            BattleEvent::SubstituteFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::TrapApplied {
                side,
                target,
                move_name,
                ..
            } => Some(match move_name.as_str() {
                "BIND" => format!("{}\nused BIND on\n{}!", name(*side), name(*target)),
                "WRAP" => format!("{}\nwas WRAPPED by\n{}!", name(*target), name(*side)),
                "CLAMP" => format!("{}\nwas CLAMPED by\n{}!", name(*target), name(*side)),
                "FIRE_SPIN" | "WHIRLPOOL" => format!("{}\nwas trapped!", name(*target)),
                _ => format!(
                    "{} was trapped by {}!",
                    name(*target),
                    battle_move_display_name(snapshot, move_name)
                ),
            }),
            BattleEvent::TrapDamage {
                side,
                source,
                move_name,
                ..
            } => {
                let message = format!(
                    "{}'s\nhurt by\n{}!",
                    name(*side),
                    battle_move_display_name(snapshot, move_name)
                );
                let affected_is_hidden = event_scene.battle.as_ref().is_some_and(|battle| {
                    match side {
                        BattleSide::Player => battle.player_semi_invulnerable,
                        BattleSide::Enemy => battle.enemy_semi_invulnerable,
                    }
                });
                if !affected_is_hidden
                    && let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                        visible_move_animation_definition(snapshot, move_name, 0)
                {
                    let prior_message = runtime_shell.battle_messages.back().cloned();
                    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                        trigger_message: prior_message.clone().unwrap_or_else(|| message.clone()),
                        move_id: move_name.clone(),
                        animation_label,
                        player_move: *source == BattleSide::Player,
                        started: prior_message.is_none(),
                        waiting_for_hp: false,
                        frame: 0,
                        total_frames,
                        sound_events,
                        next_sound_event: 0,
                        cry_events,
                        next_cry_event: 0,
                        object_events,
                        bg_events,
                        actor_species_override: None,
                        actor_shiny_override: None,
                    });
                }
                Some(message)
            }
            BattleEvent::TrapEnded { side, move_name, .. } => {
                Some(format!(
                    "{}\nwas released from\n{}!",
                    name(*side),
                    battle_move_display_name(snapshot, move_name)
                ))
            }
            BattleEvent::EscapeTrapApplied { target, .. } => {
                Some(format!("{}\ncan't escape now!", name(*target)))
            }
            BattleEvent::EscapeTrapEnded { .. } => None,
            BattleEvent::ConfusionApplied { side, target, .. } => {
                if let Some(trigger_message) = last_move_message_by_side.get(side) {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        trigger_message,
                        *target,
                        "BattleAnim_Confused",
                        false,
                    );
                } else if matches!(
                    event,
                    BattleEvent::ConfusionApplied { move_name, .. }
                        if move_name == "HELD_ATTACK_UP"
                ) && let Some(trigger_message) = runtime_shell.battle_messages.back().cloned()
                {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        &trigger_message,
                        *target,
                        "BattleAnim_Confused",
                        false,
                    );
                }
                Some(format!("{}\nbecame confused!", name(*target)))
            }
            BattleEvent::ConfusedTurn { side, .. } => {
                let message = format!("{}\nis confused!", name(*side));
                queue_visible_status_animation(
                    runtime_shell,
                    snapshot,
                    &message,
                    *side,
                    "BattleAnim_Confused",
                    false,
                );
                Some(message)
            }
            BattleEvent::ConfusionEnded { side, .. } => {
                Some(format!("{}'s\nconfused no more!", name(*side)))
            }
            BattleEvent::ConfusionSelfDamage { side, .. } => {
                let message = "It hurt itself in\nits confusion!".to_string();
                let prior_message = runtime_shell.battle_messages.back().cloned();
                if let Some(trigger_message) = prior_message.as_deref() {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        trigger_message,
                        *side,
                        "BattleAnim_HitConfusion",
                        false,
                    );
                } else {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        &message,
                        *side,
                        "BattleAnim_HitConfusion",
                        true,
                    );
                }
                Some(message)
            }
            BattleEvent::AttractApplied { target, .. } => {
                Some(format!("{}\nfell in love!", name(*target)))
            }
            BattleEvent::AttractFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::InfatuatedTurn { side, source, .. } => {
                let message = format!(
                    "{}\nis in love with\n{}!",
                    name(*side),
                    name(*source)
                );
                queue_visible_status_animation(
                    runtime_shell,
                    snapshot,
                    &message,
                    *side,
                    "BattleAnim_InLove",
                    false,
                );
                Some(message)
            }
            BattleEvent::InfatuatedImmobilized { side, .. } => {
                Some(format!(
                    "{}'s\ninfatuation kept\nit from attacking!",
                    name(*side)
                ))
            }
            BattleEvent::DisableApplied {
                target,
                disabled_move,
                ..
            } => Some(format!(
                "{}'s\n{} was\nDISABLED!",
                name(*target),
                battle_move_display_name(snapshot, disabled_move)
            )),
            BattleEvent::DisabledMove { .. } => {
                Some("The move is\nDISABLED!".to_string())
            }
            BattleEvent::DisableEnded { side, .. } => {
                Some(format!("{}'s\ndisabled no more!", name(*side)))
            }
            BattleEvent::DisableFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::EncoreApplied { target, .. } => {
                Some(format!("{}\ngot an ENCORE!", name(*target)))
            }
            BattleEvent::EncoreEnded { side, .. } => {
                Some(format!("{}'s\nENCORE ended!", name(*side)))
            }
            BattleEvent::EncoreFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::ProtectApplied { side, .. } => {
                Some(format!("{}\nPROTECTED itself!", name(*side)))
            }
            BattleEvent::MoveProtected { target, .. } => {
                Some(format!("{}'s\nPROTECTING itself!", name(*target)))
            }
            BattleEvent::ProtectFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::SpikesApplied { target, .. } => Some(format!(
                "SPIKES scattered\nall around\n{}!",
                name(*target)
            )),
            BattleEvent::SpikesFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::FutureSightQueued { side, .. } => {
                Some(format!("{}\nforesaw an attack!", name(*side)))
            }
            BattleEvent::FutureSightFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::PerishSongApplied { .. } => {
                Some("Both <PKMN> will\nfaint in 3 turns!".to_string())
            }
            BattleEvent::PerishSongCount {
                side,
                turns_remaining,
            } => Some(format!(
                "{}'s\nPERISH count is {}!",
                name(*side),
                turns_remaining
            )),
            BattleEvent::PerishSongFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::FocusEnergyApplied { side, .. } => {
                Some(format!("{}'s\ngetting pumped!", name(*side)))
            }
            BattleEvent::FocusEnergyFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::ForesightApplied { side, target, .. } => {
                Some(format!("{}\nidentified\n{}!", name(*side), name(*target)))
            }
            BattleEvent::ForesightFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::NightmareApplied { target, .. } => {
                Some(format!("{}\nstarted to have a\nNIGHTMARE!", name(*target)))
            }
            BattleEvent::NightmareFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::PsychUpApplied { side, target, .. } => {
                runtime_shell
                    .battle_messages
                    .push_back(format!("{}\ncopied the stat", name(*side)));
                Some(format!("changes of\n{}!", name(*target)))
            }
            BattleEvent::TransformApplied {
                side, species, ..
            } => Some(format!(
                "{}\nTRANSFORMED into\n{}!",
                name(*side),
                crate::core::models::pokemon_species_display_name(species)
            )),
            BattleEvent::TransformFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::MimicApplied {
                side, copied_move, ..
            } => Some(format!(
                "{}\nlearned\n{}!",
                name(*side),
                battle_move_display_name(snapshot, copied_move)
            )),
            BattleEvent::SketchApplied {
                side, copied_move, ..
            } => Some(format!(
                "{}\nSKETCHED\n{}!",
                name(*side),
                battle_move_display_name(snapshot, copied_move)
            )),
            BattleEvent::ConversionApplied { side, new_type, .. }
            | BattleEvent::Conversion2Applied { side, new_type, .. } => Some(format!(
                "{}\ntransformed into\nthe {}-type!",
                name(*side),
                new_type
            )),
            BattleEvent::StatsReset { .. } => {
                Some("All stat changes\nwere eliminated!".to_string())
            }
            BattleEvent::LockOnApplied { side, .. } => {
                Some(format!("{}\ntook aim!", name(*side)))
            }
            BattleEvent::DestinyBondApplied { side, .. } => {
                Some(format!(
                    "{}'s\ntrying to take its\nopponent with it!",
                    name(*side)
                ))
            }
            BattleEvent::DestinyBondActivated { side, source, .. } => {
                if let Some(trigger_message) = runtime_shell.battle_messages.back().cloned()
                    && let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                        visible_move_animation_definition(snapshot, "DESTINY_BOND", 1)
                {
                    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                        trigger_message,
                        move_id: "DESTINY_BOND".to_string(),
                        animation_label,
                        player_move: *side == BattleSide::Player,
                        started: false,
                        waiting_for_hp: false,
                        frame: 0,
                        total_frames,
                        sound_events,
                        next_sound_event: 0,
                        cry_events,
                        next_cry_event: 0,
                        object_events,
                        bg_events,
                        actor_species_override: None,
                        actor_shiny_override: None,
                    });
                }
                Some(format!(
                    "{}\ntook down with it,\n{}!",
                    name(*side),
                    name(*source)
                ))
            }
            BattleEvent::EndureApplied { side, .. } => {
                Some(format!("{}\nbraced itself!", name(*side)))
            }
            BattleEvent::EndureFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::EnduredHit {
                target,
                held_item: Some(item_id),
                ..
            } => Some(format!(
                "{} hung on with\n{}!",
                name(*target),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::EnduredHit { target, .. } => {
                Some(format!("{}\nENDURED the hit!", name(*target)))
            }
            BattleEvent::BideStarted { side, .. } | BattleEvent::BideStoring { side, .. } => {
                Some(format!("{}\nis storing energy!", name(*side)))
            }
            BattleEvent::BideReleased { side, .. } => {
                Some(format!("{}\nunleashed energy!", name(*side)))
            }
            BattleEvent::BideFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::SpiteApplied {
                target,
                target_move,
                reduction,
                ..
            } => Some(format!(
                "{}'s\n{} was\nreduced by {}!",
                name(*target),
                battle_move_display_name(snapshot, target_move),
                reduction
            )),
            BattleEvent::SpiteFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::Splash { .. } => Some("But nothing happened!".to_string()),
            BattleEvent::HealApplied {
                side, move_name, ..
            } => {
                if move_name == "LEFTOVERS" {
                    Some(format!(
                        "{}\nrecovered with\n{}.",
                        name(*side),
                        item_display_name(snapshot, move_name)
                    ))
                } else if move_name == "REST" {
                    let had_status = match side {
                        BattleSide::Player => snapshot
                            .battle
                            .as_ref()
                            .and_then(|battle| battle.active_player_party_index)
                            .and_then(|active| {
                                snapshot.party.slots.iter().find(|slot| slot.index == active)
                            })
                            .is_some_and(|slot| slot.pokemon.status.is_some()),
                        BattleSide::Enemy => snapshot
                            .battle
                            .as_ref()
                            .is_some_and(|battle| battle.enemy_pokemon.status.is_some()),
                    };
                    Some(if had_status {
                        format!("{}\nfell asleep and\nbecame healthy!", name(*side))
                    } else {
                        format!("{}\nwent to sleep!", name(*side))
                    })
                } else {
                    Some(format!("{}\nregained health!", name(*side)))
                }
            }
            BattleEvent::PresentHeal { target, .. } => {
                Some(format!("{}\nregained health!", name(*target)))
            }
            BattleEvent::HpDrained { target, .. } => {
                Some(format!("Sucked health from\n{}!", name(*target)))
            }
            BattleEvent::PainSplitApplied { .. } => {
                Some("The battlers\nshared pain!".to_string())
            }
            BattleEvent::HeldItemHpHealed {
                side, item_id, ..
            } => {
                let message = format!(
                    "{}\nrecovered with\n{}.",
                    name(*side),
                    item_display_name(snapshot, item_id)
                );
                let uses_recovery_animation = snapshot
                    .items
                    .iter()
                    .find(|item| item.item_id == *item_id)
                    .is_some_and(|item| item.held_effect != "HELD_LEFTOVERS");
                if uses_recovery_animation {
                    queue_visible_item_recovery_animation(
                        runtime_shell,
                        snapshot,
                        &message,
                        *side,
                    );
                }
                Some(message)
            }
            BattleEvent::HeldItemPpRestored {
                side,
                item_id,
                ..
            } => {
                let message = format!(
                    "{}\nrecovered PP using\n{}.",
                    name(*side),
                    item_display_name(snapshot, item_id)
                );
                queue_visible_item_recovery_animation(
                    runtime_shell,
                    snapshot,
                    &message,
                    *side,
                );
                Some(message)
            }
            BattleEvent::HeldItemStatusHealed {
                side,
                item_id,
                status_before,
                confusion_turns_before,
                ..
            } => {
                let display_name = item_display_name(snapshot, item_id);
                let message = if status_before.is_none() && *confusion_turns_before > 0 {
                    format!(
                        "A {} rid\n{}\nof its confusion.",
                        display_name,
                        name(*side)
                    )
                } else {
                    format!(
                        "{}\nrecovered using a\n{}!",
                        name(*side),
                        display_name
                    )
                };
                queue_visible_item_recovery_animation(
                    runtime_shell,
                    snapshot,
                    &message,
                    *side,
                );
                Some(message)
            }
            BattleEvent::HealFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::RecoilDamage { side, .. } => {
                Some(format!("{}'s\nhit with recoil!", name(*side)))
            }
            BattleEvent::JumpKickCrash {
                side, move_name, ..
            } => {
                let message = format!("{}\nkept going and\ncrashed!", name(*side));
                if let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                    visible_move_animation_definition(snapshot, move_name, 1)
                {
                    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                        trigger_message: message.clone(),
                        move_id: move_name.clone(),
                        animation_label,
                        player_move: *side == BattleSide::Player,
                        started: false,
                        waiting_for_hp: false,
                        frame: 0,
                        total_frames,
                        sound_events,
                        next_sound_event: 0,
                        cry_events,
                        next_cry_event: 0,
                        object_events,
                        bg_events,
                        actor_species_override: None,
                        actor_shiny_override: None,
                    });
                }
                Some(message)
            }
            BattleEvent::SleepTurn { side, .. } => {
                let message = format!("{}\nis fast asleep!", name(*side));
                if *side == BattleSide::Player {
                    queue_visible_pre_message_animation(
                        runtime_shell,
                        snapshot,
                        &message,
                        *side,
                        "BattleAnim_Slp",
                    );
                } else {
                    queue_visible_status_animation(
                        runtime_shell,
                        snapshot,
                        &message,
                        *side,
                        "BattleAnim_Slp",
                        false,
                    );
                }
                Some(message)
            }
            BattleEvent::RechargeTurn { side, .. } => {
                Some(format!("{}\nmust recharge!", name(*side)))
            }
            BattleEvent::ChargeStarted { side, move_name } => {
                let action = match move_name.as_str() {
                    "SOLARBEAM" => "took in sunlight!",
                    "SKULL_BASH" => "lowered its head!",
                    "SKY_ATTACK" => "is glowing!",
                    "RAZOR_WIND" => "made a whirlwind!",
                    _ => "began charging power!",
                };
                Some(format!("{}\n{action}", name(*side)))
            }
            BattleEvent::AirborneStarted { side, move_name } => {
                let action = if move_name == "DIG" {
                    "dug a hole!"
                } else {
                    "flew up high!"
                };
                Some(format!("{}\n{action}", name(*side)))
            }
            BattleEvent::WeatherApplied { weather, .. } => {
                Some(match weather {
                    Weather::Rain => "A downpour\nstarted!",
                    Weather::Sun => "The sunlight got\nbright!",
                    Weather::Sandstorm => "A SANDSTORM\nbrewed!",
                    Weather::None => "The weather returned to normal.",
                }
                .to_string())
            }
            BattleEvent::WeatherContinues { weather, .. } => {
                let message = match weather {
                    Weather::Rain => "Rain continues to\nfall.",
                    Weather::Sun => "The sunlight is\nstrong.",
                    Weather::Sandstorm => "The SANDSTORM\nrages.",
                    Weather::None => "The weather returned to normal.",
                }
                .to_string();
                if *weather == Weather::Sandstorm {
                    last_sandstorm_boundary_message = Some(message.clone());
                }
                Some(message)
            }
            BattleEvent::WeatherEnded { weather } => Some(match weather {
                Weather::Rain => "The rain stopped.",
                Weather::Sun => "The sunlight\nfaded.",
                Weather::Sandstorm => "The SANDSTORM\nsubsided.",
                Weather::None => unreachable!("core ended WEATHER_NONE"),
            }
            .to_string()),
            BattleEvent::ItemUsed { side, item_id } => {
                let item_name = item_display_name(snapshot, item_id);
                if *side == BattleSide::Enemy {
                    let trainer_name = snapshot
                        .battle
                        .as_ref()
                        .and_then(|battle| match &battle.kind {
                            RuntimeBattleKind::Trainer { trainer_name, .. } => {
                                Some(trainer_name.as_str())
                            }
                            _ => None,
                        })
                        .expect("enemy item use requires an active trainer battle identity");
                    Some(format!(
                        "{trainer_name}\nused {item_name}\non {}!",
                        name(*side)
                    ))
                } else {
                    Some(format!(
                        "{} used the {item_name}.",
                        visible_battle_player_name(snapshot)
                    ))
                }
            }
            BattleEvent::BattleItemEffect { side, outcome } => {
                if outcome.item_id == "GUARD_SPEC"
                    || (!outcome.focus_energy_before && outcome.focus_energy_after)
                {
                    None
                } else if let Some(change) = outcome.battle_stat_stage_changes.first() {
                    let stat = battle_stat_display_name(&change.stat);
                    Some(format!(
                        "{}'s\n{stat} {}!",
                        name(*side),
                        if change.stage_after > change.stage_before {
                            "rose"
                        } else {
                            "fell"
                        }
                    ))
                } else {
                    None
                }
            }
            BattleEvent::HeldItemStolen { side, item_id, .. } => Some(format!(
                "{}\nstole {}\nfrom its foe!",
                name(*side),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::HeldItemStealFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::HeldItemActivated { side, item_id, .. } => Some(format!(
                "{}'s\n{}\nactivated!",
                name(*side),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::SwitchBlocked { side, .. } => {
                Some(format!("{}\ncan't be recalled!", name(*side)))
            }
            BattleEvent::RunBlocked { .. } | BattleEvent::RunPrevented { .. } => {
                Some("Can't escape!".to_string())
            }
            BattleEvent::RunAttempt { outcome, .. } if !outcome.escaped => {
                Some("Can't escape!".to_string())
            }
            BattleEvent::RunAttempt { side, outcome } if outcome.escaped => {
                if let Some((item_side, item_id)) = held_escape_item.take() {
                    debug_assert_eq!(item_side, *side);
                    Some(format!(
                        "{}\nfled using a\n{}!",
                        name(*side),
                        item_display_name(snapshot, &item_id)
                    ))
                } else {
                    Some("Got away safely!".to_string())
                }
            }
            BattleEvent::Fled { side } => {
                if teleported_sides.contains(side) {
                    Some(format!("{}\nfled from battle!", name(*side)))
                } else if let Some(move_name) = forced_switches.get(side) {
                    Some(if *move_name == "ROAR" {
                        format!("{}\nfled in fear!", name(*side))
                    } else {
                        format!("{}\nwas blown away!\n{move_name}!", name(*side))
                    })
                } else {
                    Some(match side {
                        BattleSide::Enemy => format!("Enemy {}\nfled!", name(*side)),
                        BattleSide::Player => "Got away safely!".to_string(),
                    })
                }
            }
            BattleEvent::Fainted { side } => {
                let message = match side {
                    BattleSide::Player => format!("{}\nfainted!", name(*side)),
                    BattleSide::Enemy => format!("Enemy {}\nfainted!", name(*side)),
                };
                runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                    trigger_message: message.clone(),
                    move_id: "FAINT_MON".to_string(),
                    animation_label: "BattleAnim_FaintMon".to_string(),
                    player_move: *side == BattleSide::Player,
                    started: false,
                    waiting_for_hp: false,
                    frame: 0,
                    total_frames: 24,
                    sound_events: vec![(0, "SFX_FAINT".to_string())],
                    next_sound_event: 0,
                    cry_events: Vec::new(),
                    next_cry_event: 0,
                    object_events: Vec::new(),
                    bg_events: vec![VisibleMoveBgEvent {
                        frame: 0,
                        effect_id: "BATTLE_BG_EFFECT_FAINT_MON".to_string(),
                        duration: 20,
                        target: "BG_EFFECT_USER".to_string(),
                        param: 4,
                        incremented: false,
                    }],
                    actor_species_override: None,
                    actor_shiny_override: None,
                });
                Some(message)
            }
            BattleEvent::Switched { side, party_index } if forced_switches.contains_key(side) => {
                match side {
                    BattleSide::Player => snapshot
                        .party
                        .slots
                        .iter()
                        .find(|slot| slot.index == *party_index)
                        .map(|slot| format!("{}\nwas dragged out!", slot.pokemon.nickname)),
                    BattleSide::Enemy => snapshot
                        .battle
                        .as_ref()
                        .and_then(|battle| battle.enemy_party.get(*party_index))
                        .map(|pokemon| format!("{}\nwas dragged out!", pokemon.nickname)),
                }
            }
            BattleEvent::Switched { side, party_index } => match side {
                BattleSide::Player => {
                    if !forced_switches.contains_key(side) && !baton_passed_sides.contains(side) {
                        let withdraw = visible_player_withdraw_message(
                            runtime_shell,
                            snapshot,
                            &name(BattleSide::Player),
                        );
                        runtime_shell.battle_messages.push_back(withdraw.clone());
                        queue_visible_player_recall_animation(runtime_shell, snapshot, &withdraw);
                    }
                    Some(
                        visible_player_send_out_message(snapshot, *party_index)
                            .expect("player switch event must resolve source send-out text"),
                    )
                }
                BattleSide::Enemy => snapshot
                    .battle
                    .as_ref()
                    .and_then(|battle| {
                        battle
                            .enemy_party
                            .get(*party_index)
                            .map(|pokemon| (battle, pokemon))
                    })
                    .map(|(battle, pokemon)| match &battle.kind {
                        RuntimeBattleKind::Trainer { trainer_name, .. } => {
                            format!("{trainer_name}\nsent out\n{}!", pokemon.nickname)
                        }
                        _ => format!("{}\nwas dragged out!", pokemon.nickname),
                    }),
            },
            BattleEvent::FullyParalyzed { side, .. } => {
                Some(format!("{}'s\nfully paralyzed!", name(*side)))
            }
            BattleEvent::Flinched { side, .. } => {
                Some(format!("{}\nflinched!", name(*side)))
            }
            BattleEvent::FrozenTurn { side, .. } => {
                Some(format!("{}\nis frozen solid!", name(*side)))
            }
            BattleEvent::WokeUp { side, .. } => Some(format!("{}\nwoke up!", name(*side))),
            _ => None,
        };
        if let Some(message) = message {
            runtime_shell.battle_messages.push_back(message);
        }
        if stage_message_scenes {
            for _ in event_message_count_before..runtime_shell.battle_messages.len() {
                runtime_shell
                    .battle_message_scenes
                    .push_back(Box::new(if matches!(
                        event,
                        BattleEvent::Fainted { .. }
                            | BattleEvent::ResidualStatusDamage { .. }
                            | BattleEvent::LeechSeedDamage { .. }
                            | BattleEvent::NightmareDamage { .. }
                            | BattleEvent::CurseDamage { .. }
                            | BattleEvent::SpikesDamage { .. }
                    ) {
                        event_scene_before.clone()
                    } else {
                        event_scene.clone()
                    }));
            }
            if matches!(event, BattleEvent::Switched { side: BattleSide::Player, .. })
                && runtime_shell.battle_messages.len().saturating_sub(event_message_count_before) == 2
                && let Some(scene) = runtime_shell
                    .battle_message_scenes
                    .get_mut(event_message_count_before)
            {
                *scene = Box::new(event_scene_before.clone());
            }
            if let BattleEvent::Fainted { side } = event {
                let trigger = match side {
                    BattleSide::Player => format!("{}\nfainted!", name(*side)),
                    BattleSide::Enemy => format!("Enemy {}\nfainted!", name(*side)),
                };
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
            }
            if let BattleEvent::ResidualStatusDamage { side, status, .. } = event {
                let trigger = if status == "BURN" {
                    format!("{}'s\nhurt by its burn!", name(*side))
                } else {
                    format!("{}\nis hurt by poison!", name(*side))
                };
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
            }
            if let BattleEvent::NightmareDamage { side, .. } | BattleEvent::CurseDamage { side, .. } = event {
                let (kind, fallback_trigger) = if matches!(event, BattleEvent::NightmareDamage { .. }) {
                    ("NIGHTMARE", format!("{} has a NIGHTMARE!", name(*side)))
                } else {
                    ("CURSE", format!("{} is hurt by the CURSE!", name(*side)))
                };
                let trigger = residual_animation_triggers
                    .get(&(kind, *side))
                    .cloned()
                    .unwrap_or(fallback_trigger);
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
            }
            if let BattleEvent::LeechSeedDamage { side, source, .. } = event {
                let trigger = leech_seed_animation_triggers
                    .get(&(*source, *side))
                    .cloned()
                    .unwrap_or_else(|| format!("LEECH SEED saps\n{}!", name(*side)));
                leech_seed_scene_starts.insert((*source, *side), event_message_count_before);
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
            }
            if let BattleEvent::SpikesDamage { side, .. } = event {
                let trigger = format!("{}'s hurt by SPIKES!", name(*side));
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
            }
            if let BattleEvent::Missed { side, move_name, .. } = event
                && let Some(revealed_scene) = deferred_airborne_end_scenes
                    .remove(&(*side, move_name.as_str()))
            {
                let trigger = format!("{}'s attack missed!", name(*side));
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(revealed_scene.clone())));
                event_scene = revealed_scene;
            }
            if let BattleEvent::LeechSeedDrain { side, target, .. } = event
                && let Some(scene_start) = leech_seed_scene_starts.get(&(*side, *target)).copied()
            {
                let trigger = leech_seed_animation_triggers
                    .get(&(*side, *target))
                    .cloned()
                    .unwrap_or_else(|| format!("LEECH SEED saps\n{}!", name(*target)));
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
                for scene in runtime_shell
                    .battle_message_scenes
                    .iter_mut()
                    .skip(scene_start.saturating_add(1))
                {
                    apply_visible_battle_event_to_scene(
                        scene,
                        event,
                        &mut event_scene_baton_pass_sides,
                    );
                }
            }
            if event_message_count_before == runtime_shell.battle_messages.len()
                && !matches!(event, BattleEvent::LeechSeedDrain { .. })
                && battle_event_changes_visible_scene(event)
            {
                // Several Crystal effects deliberately have no additional
                // textbox after their initiating/result line. Keep their HP
                // or battler mutation on that preceding visible boundary so
                // the HUD animates before control returns instead of jumping
                // only after the message queue vanishes.
                if let Some(scene) = runtime_shell.battle_message_scenes.back_mut() {
                    *scene = Box::new(event_scene.clone());
                }
            }
        } else if let BattleEvent::Missed { side, move_name, .. } = event
            && let Some(revealed_scene) = deferred_airborne_end_scenes
                .remove(&(*side, move_name.as_str()))
        {
            event_scene = revealed_scene;
        }
        if let BattleEvent::Switched { side, party_index } = event {
            let switched_name = match side {
                BattleSide::Player => snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == *party_index)
                    .map(|slot| slot.pokemon.nickname.clone()),
                BattleSide::Enemy => snapshot
                    .battle
                    .as_ref()
                    .and_then(|battle| battle.enemy_party.get(*party_index))
                    .map(|pokemon| pokemon.nickname.clone()),
            };
            if let Some(switched_name) = switched_name {
                match side {
                    BattleSide::Player => *player_name.borrow_mut() = switched_name,
                    BattleSide::Enemy => *enemy_name.borrow_mut() = switched_name,
                }
            }
            runtime_shell.battle_enemy_hp_at_player_send_out = match side {
                BattleSide::Player => snapshot
                    .battle
                    .as_ref()
                    .map(|battle| battle.enemy_pokemon.hp),
                BattleSide::Enemy => snapshot
                    .battle
                    .as_ref()
                    .and_then(|battle| battle.enemy_party.get(*party_index))
                    .map(|pokemon| pokemon.hp),
            };
        }
    }
    let has_visible_scene_change = events.iter().any(battle_event_changes_visible_scene);
    if runtime_shell.battle_messages.len() > message_count_before || has_visible_scene_change {
        let mut scene = snapshot.clone();
        for event in events {
            apply_visible_battle_event_to_scene(&mut scene, event, &mut baton_pass_sides);
        }
        let old_player_pixels = snapshot
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
            .map(|slot| battle_hud_hp_pixels(slot.pokemon.hp, slot.pokemon.max_hp))
            .unwrap_or(0);
        let (old_player_hp, old_player_max_hp) = snapshot
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
            .map(|slot| (slot.pokemon.hp, slot.pokemon.max_hp))
            .unwrap_or((0, 0));
        let old_enemy_pixels = snapshot
            .battle
            .as_ref()
            .map(|battle| battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp))
            .unwrap_or(0);
        let displayed_scene = if stage_message_scenes {
            runtime_shell
                .battle_message_scenes
                .front()
                .map(|scene| scene.as_ref())
                .unwrap_or(&scene)
        } else {
            &scene
        };
        let new_player_pixels = displayed_scene
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|index| {
                displayed_scene
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == index)
            })
            .map(|slot| battle_hud_hp_pixels(slot.pokemon.hp, slot.pokemon.max_hp))
            .unwrap_or(old_player_pixels);
        let (new_player_hp, new_player_max_hp) = displayed_scene
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|index| {
                displayed_scene
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == index)
            })
            .map(|slot| (slot.pokemon.hp, slot.pokemon.max_hp))
            .unwrap_or((old_player_hp, old_player_max_hp));
        let new_enemy_pixels = displayed_scene
            .battle
            .as_ref()
            .map(|battle| battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp))
            .unwrap_or(old_enemy_pixels);
        let tween = runtime_shell
            .battle_hp_tween
            .get_or_insert(VisibleBattleHpTween {
                player_hp: old_player_hp,
                player_target_hp: old_player_hp,
                player_max_hp: old_player_max_hp,
                player_pixels: old_player_pixels,
                player_target_pixels: old_player_pixels,
                player_frames_until_step: 0,
                enemy_pixels: old_enemy_pixels,
                enemy_target_pixels: old_enemy_pixels,
                enemy_frames_until_step: 0,
            });
        tween.player_target_hp = new_player_hp;
        tween.player_max_hp = new_player_max_hp;
        tween.player_target_pixels = new_player_pixels;
        tween.enemy_target_pixels = new_enemy_pixels;
        tween.player_frames_until_step = 0;
        tween.enemy_frames_until_step = 0;
        for event in events {
            if let BattleEvent::Switched { side, .. } = event {
                match side {
                    BattleSide::Player => {
                        tween.player_hp = new_player_hp;
                        tween.player_target_hp = new_player_hp;
                        tween.player_max_hp = new_player_max_hp;
                        tween.player_pixels = new_player_pixels;
                    }
                    BattleSide::Enemy => tween.enemy_pixels = new_enemy_pixels,
                }
            }
        }
        runtime_shell.battle_message_scene = if stage_message_scenes {
            runtime_shell
                .battle_message_scenes
                .front()
                .cloned()
                .or_else(|| Some(Box::new(scene)))
        } else {
            Some(Box::new(scene))
        };
    }
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn advance_visible_battle_text_reveal(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    acceleration_requested: bool,
) -> bool {
    let Some(text) = runtime_shell.battle_messages.front().cloned() else {
        return runtime_shell.battle_text_reveal.take().is_some();
    };
    let reveal = runtime_shell
        .battle_text_reveal
        .get_or_insert_with(|| VisibleBattleTextReveal {
            text: text.clone(),
            page_index: 0,
            visible_chars: 0,
            frames_until_next_char: 0,
        });
    if reveal.text != text {
        *reveal = VisibleBattleTextReveal {
            text,
            page_index: 0,
            visible_chars: 0,
            frames_until_next_char: 0,
        };
    }
    let text_len = battle_message_page(&reveal.text, reveal.page_index)
        .chars()
        .count();
    if reveal.visible_chars >= text_len {
        return false;
    }
    if snapshot.trainer.options.no_text_scroll {
        reveal.visible_chars = text_len;
        reveal.frames_until_next_char = 0;
        return true;
    }
    if acceleration_requested {
        reveal.frames_until_next_char = 0;
    }
    if reveal.frames_until_next_char > 0 {
        reveal.frames_until_next_char -= 1;
        return false;
    }
    reveal.visible_chars = reveal.visible_chars.saturating_add(1).min(text_len);
    let frames_per_char = if acceleration_requested {
        1
    } else {
        visible_text_frames_per_char(snapshot.trainer.options.text_speed)
    };
    reveal.frames_until_next_char = frames_per_char.saturating_sub(1);
    true
}

fn visible_battle_message_text<'a>(
    runtime_shell: &'a BevyRuntimeShell,
    message: &'a str,
) -> String {
    runtime_shell
        .battle_text_reveal
        .as_ref()
        .filter(|reveal| reveal.text == message)
        .map(|reveal| {
            battle_message_page(message, reveal.page_index)
                .chars()
                .take(reveal.visible_chars)
                .collect()
        })
        .unwrap_or_default()
}

fn laid_out_battle_message(message: &str) -> String {
    // Keep the complete layout here. Crystal's standard textbox exposes two
    // text baselines at once and `_ContText` advances through later lines;
    // limiting layout to the four interior tile rows silently discarded the
    // tail before the paging layer had any chance to present it.
    wrap_boot_text_for_box(message, 18, usize::MAX).join("\n")
}

fn battle_message_pages(message: &str) -> Vec<String> {
    let lines = laid_out_battle_message(message)
        .split('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines
        .chunks(2)
        .map(|page| page.join("\n"))
        .collect()
}

fn battle_message_page(message: &str, page_index: usize) -> String {
    battle_message_pages(message)
        .get(page_index)
        .cloned()
        .unwrap_or_default()
}

fn visible_battle_message_has_more_pages(
    runtime_shell: &BevyRuntimeShell,
    message: &str,
) -> bool {
    runtime_shell
        .battle_text_reveal
        .as_ref()
        .filter(|reveal| reveal.text == message)
        .is_some_and(|reveal| reveal.page_index + 1 < battle_message_pages(message).len())
}

fn advance_visible_battle_message_page(
    runtime_shell: &mut BevyRuntimeShell,
    message: &str,
) -> bool {
    let page_count = battle_message_pages(message).len();
    let Some(reveal) = runtime_shell
        .battle_text_reveal
        .as_mut()
        .filter(|reveal| reveal.text == message)
    else {
        return false;
    };
    if reveal.page_index + 1 >= page_count {
        return false;
    }
    reveal.page_index += 1;
    reveal.visible_chars = 0;
    reveal.frames_until_next_char = 0;
    true
}

fn visible_battle_message_lines(
    runtime_shell: &BevyRuntimeShell,
    message: &str,
) -> Vec<String> {
    visible_battle_message_text(runtime_shell, message)
        .split('\n')
        .map(str::to_string)
        .collect()
}

fn visible_battle_message_is_complete(runtime_shell: &BevyRuntimeShell, message: &str) -> bool {
    runtime_shell
        .battle_text_reveal
        .as_ref()
        .is_some_and(|reveal| {
            reveal.text == message
                && reveal.visible_chars
                    >= battle_message_page(message, reveal.page_index).chars().count()
        })
}

fn queue_visible_status_animation(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    trigger_message: &str,
    side: crate::core::battle::turn::BattleSide,
    label: &str,
    started: bool,
) {
    let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
        visible_battle_animation_definition(snapshot, label.to_string(), 0)
    else {
        return;
    };
    runtime_shell
        .visible_move_animations
        .push_back(VisibleMoveAnimation {
            trigger_message: trigger_message.to_string(),
            move_id: label.to_string(),
            animation_label,
            player_move: side == crate::core::battle::turn::BattleSide::Player,
            started,
            waiting_for_hp: false,
            frame: 0,
            total_frames,
            sound_events,
            next_sound_event: 0,
            cry_events,
            next_cry_event: 0,
            object_events,
            bg_events,
            actor_species_override: None,
            actor_shiny_override: None,
        });
}

fn queue_visible_item_recovery_animation(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    result_message: &str,
    side: crate::core::battle::turn::BattleSide,
) {
    let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
        visible_move_animation_definition(snapshot, "RECOVER", 0)
    else {
        return;
    };
    let prior_message = runtime_shell.battle_messages.back().cloned();
    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
        trigger_message: prior_message
            .clone()
            .unwrap_or_else(|| result_message.to_string()),
        move_id: "ITEM_RECOVERY".to_string(),
        animation_label,
        player_move: side == crate::core::battle::turn::BattleSide::Player,
        started: prior_message.is_none(),
        waiting_for_hp: false,
        frame: 0,
        total_frames,
        sound_events,
        next_sound_event: 0,
        cry_events,
        next_cry_event: 0,
        object_events,
        bg_events,
        actor_species_override: None,
        actor_shiny_override: None,
    });
}

fn queue_visible_pre_message_animation(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    result_message: &str,
    side: crate::core::battle::turn::BattleSide,
    label: &str,
) {
    let prior_message = runtime_shell.battle_messages.back().cloned();
    queue_visible_status_animation(
        runtime_shell,
        snapshot,
        prior_message.as_deref().unwrap_or(result_message),
        side,
        label,
        prior_message.is_none(),
    );
}

fn queue_visible_terminal_animation_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    trigger_message: &str,
    side: crate::core::battle::turn::BattleSide,
    boundary_id: &str,
) {
    runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
        trigger_message: trigger_message.to_string(),
        move_id: boundary_id.to_string(),
        animation_label: format!("BattleCommand_{boundary_id}"),
        player_move: side == crate::core::battle::turn::BattleSide::Player,
        started: false,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 1,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: Vec::new(),
        actor_species_override: None,
        actor_shiny_override: None,
    });
}

fn visible_move_animation_definition(
    snapshot: &RuntimeShellSnapshot,
    move_id: &str,
    animation_param: i32,
) -> Option<(
    String,
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let normalized = move_id.replace(' ', "_").to_ascii_uppercase();
    let move_index = snapshot
        .presentation
        .move_names
        .iter()
        .position(|name| name.replace(' ', "_").to_ascii_uppercase() == normalized)?;
    // BattleAnimations begins with the status-animation entry; move one is
    // therefore table entry one, matching TypeScript's table.slice(1).
    let label = snapshot
        .presentation
        .battle_animation_table
        .get(move_index.saturating_add(1))?
        .clone();
    visible_battle_animation_definition(snapshot, label, animation_param)
}

fn visible_battle_animation_definition(
    snapshot: &RuntimeShellSnapshot,
    label: String,
    animation_param: i32,
) -> Option<(
    String,
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let (timeline_frame, sound_events, cry_events, object_events, bg_events) =
        compile_visible_battle_animation_timeline(snapshot, &label, animation_param)?;
    // TypeScript/ASM execute consecutive commands in one update and yield only
    // at an explicit wait (or while a live object/background effect continues).
    let final_sound_frame = sound_events.last().map_or(0, |(frame, _)| *frame);
    let final_cry_frame = cry_events.last().map_or(0, |(frame, _)| *frame);
    let bundle = serde_json::from_str::<serde_json::Value>(
        &snapshot.presentation.battle_anim_bundle,
    )
    .ok()?;
    let final_object_frame = object_events
        .iter()
        .filter_map(|event| {
            let VisibleMoveObjectCommand::Spawn { object_id, .. } = &event.command else {
                return None;
            };
            let object = bundle.get("objects")?.get(object_id)?;
            if object
                .get("function")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|function| function != "BATTLE_ANIM_FUNC_NULL")
            {
                return None;
            }
            let frameset = object.get("frameset")?.as_str()?;
            let lifetime = visible_null_battle_animation_object_lifetime(&bundle, frameset)?;
            let natural_end = event.frame.saturating_add(lifetime.saturating_sub(1));
            let cleared_at = object_events
                .iter()
                .find(|candidate| {
                    candidate.frame >= event.frame
                        && matches!(&candidate.command, VisibleMoveObjectCommand::Clear)
                })
                .map(|candidate| candidate.frame);
            Some(cleared_at.map_or(natural_end, |clear| natural_end.min(clear)))
        })
        .max()
        .unwrap_or(0);
    let final_bg_frame = bg_events
        .iter()
        .filter(|effect| !effect.incremented)
        .filter_map(|effect| {
            let terminating_increment = bg_events
                .iter()
                .find(|candidate| {
                    candidate.incremented
                        && candidate.effect_id == effect.effect_id
                        && candidate.frame >= effect.frame
                })
                .map(|candidate| candidate.frame);
            let lifetime = match effect.effect_id.as_str() {
                "BATTLE_BG_EFFECT_TACKLE"
                | "BATTLE_BG_EFFECT_BODY_SLAM"
                | "BATTLE_BG_EFFECT_BETA_PURSUIT"
                | "BATTLE_BG_EFFECT_ROLLOUT"
                | "BATTLE_BG_EFFECT_VITAL_THROW" => 8,
                "BATTLE_BG_EFFECT_SHAKE_SCREEN_X" | "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y" => {
                    effect.duration.max(1)
                }
                "BATTLE_BG_EFFECT_FLASH_INVERTED" | "BATTLE_BG_EFFECT_FLASH_WHITE" => {
                    if effect.duration == 0 { 4 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_WHITE_HUES" | "BATTLE_BG_EFFECT_BLACK_HUES" => {
                    effect.duration.max(1)
                }
                "BATTLE_BG_EFFECT_ALTERNATE_HUES"
                | "BATTLE_BG_EFFECT_CYCLE_BGPALS_INVERTED"
                | "BATTLE_BG_EFFECT_ACID_ARMOR" => {
                    if effect.duration == 0 { 4 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_CYCLE_OBPALS_GRAY_AND_YELLOW"
                | "BATTLE_BG_EFFECT_CYCLE_MID_OBPALS_GRAY_AND_YELLOW"
                | "BATTLE_BG_EFFECT_CYCLE_MON_LIGHT_DARK_REPEATING" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_START_WATER"
                | "BATTLE_BG_EFFECT_WATER"
                | "BATTLE_BG_EFFECT_END_WATER"
                | "BATTLE_BG_EFFECT_WHIRLPOOL" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_NIGHT_SHADE" | "BATTLE_BG_EFFECT_TELEPORT" => {
                    effect.duration.max(1)
                }
                "BATTLE_BG_EFFECT_PSYCHIC" => {
                    if effect.duration == 0 { 4 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_WOBBLE_MON" | "BATTLE_BG_EFFECT_WAVE_DEFORM_MON" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_WOBBLE_PLAYER" | "BATTLE_BG_EFFECT_WOBBLE_SCREEN" => {
                    if effect.duration == 0 { 8 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_VIBRATE_MON" => effect.duration.max(2),
                "BATTLE_BG_EFFECT_DIG" | "BATTLE_BG_EFFECT_FLAIL" | "BATTLE_BG_EFFECT_DOUBLE_TEAM" => {
                    if effect.duration == 0 {
                        u16::from(if effect.param == 0 { 3 } else { effect.param })
                            .saturating_mul(2)
                    } else {
                        effect.duration
                    }
                }
                "BATTLE_BG_EFFECT_BOUNCE_DOWN" => 4,
                "BATTLE_BG_EFFECT_REMOVE_MON" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_FAINT_MON" => {
                    if effect.duration == 0 { 14 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1" | "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON2" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_ENTER_MON" => effect.duration.max(1).saturating_mul(3),
                "BATTLE_BG_EFFECT_RETURN_MON" => effect.duration.max(1).saturating_mul(4),
                "BATTLE_BG_EFFECT_BATTLEROBJ_1ROW" | "BATTLE_BG_EFFECT_BATTLEROBJ_2ROW" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT" | "BATTLE_BG_EFFECT_FADE_MON_TO_BLACK" => {
                    u16::from((effect.param >> 4).max(1)).saturating_mul(3)
                }
                "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT_REPEATING"
                | "BATTLE_BG_EFFECT_FADE_MON_TO_BLACK_REPEATING"
                | "BATTLE_BG_EFFECT_FADE_MONS_TO_BLACK_REPEATING" => {
                    return terminating_increment.or(Some(timeline_frame));
                }
                "BATTLE_BG_EFFECT_FADE_MON_TO_WHITE_WAIT_FADE_BACK" => {
                    let step_delay = u16::from((effect.param >> 4).max(1));
                    let initial_delay = u16::from((effect.param & 0x0f).max(effect.param >> 4).max(1));
                    let natural_end = effect.frame
                        .saturating_add(initial_delay)
                        .saturating_add(step_delay.saturating_mul(16));
                    return Some(terminating_increment.map_or(natural_end, |frame| frame.min(natural_end)));
                }
                "BATTLE_BG_EFFECT_RAPID_FLASH"
                | "BATTLE_BG_EFFECT_FLASH_MON_REPEATING" => {
                    return terminating_increment.or(Some(timeline_frame));
                }
                "BATTLE_BG_EFFECT_FADE_MON_FROM_WHITE" => {
                    if effect.duration == 0 { 8 } else { effect.duration }
                }
                _ => return None,
            };
            let last_reset = bg_events
                .iter()
                .filter(|candidate| {
                    candidate.incremented
                        && candidate.effect_id == effect.effect_id
                        && candidate.frame >= effect.frame
                })
                .map(|candidate| candidate.frame)
                .max()
                .unwrap_or(effect.frame);
            Some(last_reset.saturating_add(lifetime.saturating_sub(1)))
        })
        .max()
        .unwrap_or(0);
    Some((
        label,
        timeline_frame
            .max(final_sound_frame)
            .max(final_cry_frame)
            .max(final_object_frame)
            .max(final_bg_frame)
            .max(1),
        sound_events,
        cry_events,
        object_events,
        bg_events,
    ))
}

fn visible_move_animation_definition_with_substitute(
    snapshot: &RuntimeShellSnapshot,
    move_id: &str,
    animation_param: i32,
    lower_substitute: bool,
    raise_substitute: bool,
) -> Option<(
    String,
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    if !lower_substitute && !raise_substitute {
        return visible_move_animation_definition(snapshot, move_id, animation_param);
    }
    let mut parts = Vec::new();
    if lower_substitute {
        parts.push(visible_move_animation_definition(snapshot, "SUBSTITUTE", 1)?);
    }
    parts.push(visible_move_animation_definition(snapshot, move_id, animation_param)?);
    if raise_substitute {
        parts.push(visible_move_animation_definition(snapshot, "SUBSTITUTE", 2)?);
    }

    let mut labels = Vec::new();
    let mut total_frames = 0_u16;
    let mut sounds = Vec::new();
    let mut cries = Vec::new();
    let mut objects = Vec::new();
    let mut bg_effects = Vec::new();
    for (label, frames, part_sounds, part_cries, part_objects, part_bg_effects) in parts {
        labels.push(label);
        sounds.extend(
            part_sounds
                .into_iter()
                .map(|(frame, sound)| (frame.saturating_add(total_frames), sound)),
        );
        cries.extend(
            part_cries
                .into_iter()
                .map(|(frame, selector)| (frame.saturating_add(total_frames), selector)),
        );
        objects.extend(part_objects.into_iter().map(|mut event| {
            event.frame = event.frame.saturating_add(total_frames);
            event
        }));
        bg_effects.extend(part_bg_effects.into_iter().map(|mut event| {
            event.frame = event.frame.saturating_add(total_frames);
            event
        }));
        total_frames = total_frames.saturating_add(frames);
    }
    Some((
        labels.join(" → "),
        total_frames,
        sounds,
        cries,
        objects,
        bg_effects,
    ))
}

fn visible_substitute_move_delay_definition(
    snapshot: &RuntimeShellSnapshot,
) -> Option<(
    String,
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let (_, lower_frames, mut sounds, mut cries, mut objects, mut bg_effects) =
        visible_move_animation_definition(snapshot, "SUBSTITUTE", 1)?;
    let (_, raise_frames, raise_sounds, raise_cries, raise_objects, raise_bg_effects) =
        visible_move_animation_definition(snapshot, "SUBSTITUTE", 2)?;
    let raise_offset = lower_frames.saturating_add(40);
    sounds.extend(
        raise_sounds
            .into_iter()
            .map(|(frame, sound)| (frame.saturating_add(raise_offset), sound)),
    );
    cries.extend(
        raise_cries
            .into_iter()
            .map(|(frame, selector)| (frame.saturating_add(raise_offset), selector)),
    );
    objects.extend(raise_objects.into_iter().map(|mut event| {
        event.frame = event.frame.saturating_add(raise_offset);
        event
    }));
    bg_effects.extend(raise_bg_effects.into_iter().map(|mut event| {
        event.frame = event.frame.saturating_add(raise_offset);
        event
    }));
    Some((
        "BattleCommand_LowerSub_MoveDelay_RaiseSub".to_string(),
        raise_offset.saturating_add(raise_frames),
        sounds,
        cries,
        objects,
        bg_effects,
    ))
}

fn visible_substitute_raise_after_delay_definition(
    snapshot: &RuntimeShellSnapshot,
) -> Option<(
    String,
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let (_, raise_frames, mut sounds, mut cries, mut objects, mut bg_effects) =
        visible_move_animation_definition(snapshot, "SUBSTITUTE", 2)?;
    for (frame, _) in &mut sounds {
        *frame = frame.saturating_add(40);
    }
    for (frame, _) in &mut cries {
        *frame = frame.saturating_add(40);
    }
    for event in &mut objects {
        event.frame = event.frame.saturating_add(40);
    }
    for event in &mut bg_effects {
        event.frame = event.frame.saturating_add(40);
    }
    Some((
        "BattleCommand_MoveDelay_RaiseSub".to_string(),
        40_u16.saturating_add(raise_frames),
        sounds,
        cries,
        objects,
        bg_effects,
    ))
}

#[derive(Default)]
struct VisibleBattleAnimationTimeline {
    frame: u16,
    sounds: Vec<(u16, String)>,
    cries: Vec<(u16, u8)>,
    objects: Vec<VisibleMoveObjectEvent>,
    bg_effects: Vec<VisibleMoveBgEvent>,
    loops: std::collections::BTreeMap<(String, usize), i32>,
    anim_var: i32,
    anim_param: i32,
    commands_executed: usize,
}

fn compile_visible_battle_animation_timeline(
    snapshot: &RuntimeShellSnapshot,
    root_label: &str,
    animation_param: i32,
) -> Option<(
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let mut timeline = VisibleBattleAnimationTimeline {
        anim_param: animation_param,
        ..Default::default()
    };
    execute_visible_battle_animation_script(snapshot, root_label, &mut timeline, 0)?;
    Some((
        timeline.frame,
        timeline.sounds,
        timeline.cries,
        timeline.objects,
        timeline.bg_effects,
    ))
}

fn execute_visible_battle_animation_script(
    snapshot: &RuntimeShellSnapshot,
    script_label: &str,
    timeline: &mut VisibleBattleAnimationTimeline,
    depth: usize,
) -> Option<()> {
    if depth > 32 {
        return None;
    }
    let source = snapshot.presentation.battle_animations.get(script_label)?;
    let mut labels = std::collections::BTreeMap::<String, usize>::new();
    let mut commands = Vec::<String>::new();
    for line in source {
        let trimmed = line.trim();
        if trimmed.starts_with('.') && !trimmed.contains(char::is_whitespace) {
            labels.insert(trimmed.to_string(), commands.len());
        } else {
            commands.push(trimmed.to_string());
        }
    }
    let mut pointer = 0_usize;
    while pointer < commands.len() {
        timeline.commands_executed = timeline.commands_executed.saturating_add(1);
        if timeline.commands_executed > 65_535 {
            return None;
        }
        let command_index = pointer;
        let command = &commands[pointer];
        pointer += 1;
        let (opcode, raw_arguments) = command
            .split_once(char::is_whitespace)
            .map_or((command.as_str(), ""), |(opcode, arguments)| (opcode, arguments));
        let arguments = raw_arguments
            .split(',')
            .map(str::trim)
            .filter(|argument| !argument.is_empty())
            .collect::<Vec<_>>();
        match opcode {
            "anim_wait" => {
                let frames = arguments
                    .first()
                    .and_then(|argument| parse_visible_battle_animation_int(argument))
                    .and_then(|frames| u16::try_from(frames).ok())?;
                timeline.frame = timeline.frame.saturating_add(frames);
            }
            "anim_sound" => {
                if let Some(sound) = arguments.get(2) {
                    timeline
                        .sounds
                        .push((timeline.frame.saturating_add(1), (*sound).to_string()));
                }
            }
            "playsound" => {
                if let Some(sound) = arguments.first() {
                    timeline
                        .sounds
                        .push((timeline.frame.saturating_add(1), (*sound).to_string()));
                }
            }
            "anim_cry" => {
                let selector = arguments
                    .first()
                    .and_then(|argument| parse_visible_battle_animation_int(argument))
                    .unwrap_or(0);
                timeline.cries.push((
                    timeline.frame.saturating_add(1),
                    u8::try_from(selector & 0x03).ok()?,
                ));
            }
            "anim_obj" if arguments.len() >= 4 => {
                let (x, y, param) = if arguments.len() >= 6 {
                    (
                        parse_visible_battle_animation_int(arguments[1])?
                            .saturating_mul(8)
                            .saturating_add(parse_visible_battle_animation_int(arguments[2])?),
                        parse_visible_battle_animation_int(arguments[3])?
                            .saturating_mul(8)
                            .saturating_add(parse_visible_battle_animation_int(arguments[4])?),
                        parse_visible_battle_animation_int(arguments[5])?,
                    )
                } else {
                    (
                        parse_visible_battle_animation_int(arguments[1])?,
                        parse_visible_battle_animation_int(arguments[2])?,
                        parse_visible_battle_animation_int(arguments[3])?,
                    )
                };
                timeline.objects.push(VisibleMoveObjectEvent {
                    frame: timeline.frame.saturating_add(1),
                    command: VisibleMoveObjectCommand::Spawn {
                        object_id: arguments[0].to_string(),
                        x: i16::try_from(x).ok()?,
                        y: i16::try_from(y).ok()?,
                        param: u8::try_from(param & 0xff).ok()?,
                    },
                });
            }
            "anim_clearobjs" => timeline.objects.push(VisibleMoveObjectEvent {
                frame: timeline.frame.saturating_add(1),
                command: VisibleMoveObjectCommand::Clear,
            }),
            "anim_incobj" => {
                let slot = parse_visible_battle_animation_int(arguments.first()?)?;
                timeline.objects.push(VisibleMoveObjectEvent {
                    frame: timeline.frame.saturating_add(1),
                    command: VisibleMoveObjectCommand::Increment {
                        slot: u8::try_from(slot).ok()?,
                    },
                });
            }
            "anim_setobj" => {
                let slot = parse_visible_battle_animation_int(arguments.first()?)?;
                let value = parse_visible_battle_animation_int(arguments.get(1)?)?;
                timeline.objects.push(VisibleMoveObjectEvent {
                    frame: timeline.frame.saturating_add(1),
                    command: VisibleMoveObjectCommand::Set {
                        slot: u8::try_from(slot).ok()?,
                        value: u8::try_from(value & 0xff).ok()?,
                    },
                });
            }
            "anim_transform" | "anim_raisesub" | "anim_dropsub" | "anim_minimize"
            | "anim_minimizeopp" | "anim_updateactorpic" => {
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: format!("BATTLE_ACTOR_{}", opcode.trim_start_matches("anim_").to_ascii_uppercase()),
                    duration: 0,
                    target: "BG_EFFECT_USER".to_string(),
                    param: 0,
                    incremented: false,
                });
            }
            "anim_bgp" | "anim_obp0" | "anim_obp1" => {
                let value = parse_visible_battle_animation_int(arguments.first()?)?;
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: format!("BATTLE_PALETTE_{}", opcode.trim_start_matches("anim_").to_ascii_uppercase()),
                    duration: 0,
                    target: String::new(),
                    param: u8::try_from(value & 0xff).ok()?,
                    incremented: false,
                });
            }
            "anim_resetobp0" => timeline.bg_effects.push(VisibleMoveBgEvent {
                frame: timeline.frame.saturating_add(1),
                effect_id: "BATTLE_PALETTE_OBP0".to_string(),
                duration: 0,
                target: String::new(),
                param: 0xe4,
                incremented: false,
            }),
            "anim_beatup" => timeline.bg_effects.push(VisibleMoveBgEvent {
                frame: timeline.frame.saturating_add(1),
                effect_id: "BATTLE_ACTOR_BEATUP".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: u8::try_from(timeline.anim_param & 0xff).ok()?,
                incremented: false,
            }),
            "anim_1gfx" | "anim_2gfx" | "anim_3gfx" | "anim_battlergfx_1row"
            | "anim_battlergfx_2row" | "anim_checkpokeball"
            | "anim_keepsprites" => {}
            "anim_bgeffect" if arguments.len() >= 4 => {
                let duration = parse_visible_battle_animation_int(arguments[1])?;
                let param = parse_visible_battle_animation_int(arguments[3])?;
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: arguments[0].to_string(),
                    duration: u16::try_from(duration & 0xffff).ok()?,
                    target: arguments[2].to_string(),
                    param: u8::try_from(param & 0xff).ok()?,
                    incremented: false,
                });
            }
            "anim_incbgeffect" => {
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: arguments.first()?.to_string(),
                    duration: 0,
                    target: String::new(),
                    param: 0,
                    incremented: true,
                });
            }
            "anim_call" => {
                let target = *arguments.first()?;
                if target.starts_with('.') {
                    return None;
                }
                execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
            }
            "anim_jump" => {
                let target = *arguments.first()?;
                if target.starts_with('.') {
                    pointer = *labels.get(target)?;
                } else {
                    execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
                    return Some(());
                }
            }
            "anim_loop" => {
                let count = parse_visible_battle_animation_int(arguments.first()?)?;
                let target = *arguments.get(1)?;
                let key = (script_label.to_string(), command_index);
                if let Some(remaining) = timeline.loops.get_mut(&key) {
                    if *remaining < 0 {
                        pointer = *labels.get(target)?;
                    } else if *remaining > 0 {
                        *remaining -= 1;
                        pointer = *labels.get(target)?;
                    } else {
                        timeline.loops.remove(&key);
                    }
                } else {
                    timeline.loops.insert(key, if count <= 0 { -1 } else { count - 1 });
                    pointer = *labels.get(target)?;
                }
            }
            "anim_setvar" => {
                timeline.anim_var = parse_visible_battle_animation_int(arguments.first()?)?;
            }
            "anim_incvar" => timeline.anim_var = (timeline.anim_var + 1) & 0xff,
            "anim_if_var_equal" => {
                let value = parse_visible_battle_animation_int(arguments.first()?)?;
                if timeline.anim_var == value {
                    let target = *arguments.get(1)?;
                    if target.starts_with('.') {
                        pointer = *labels.get(target)?;
                    } else {
                        execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
                        return Some(());
                    }
                }
            }
            "anim_if_param_equal" => {
                let value = parse_visible_battle_animation_int(arguments.first()?)?;
                if timeline.anim_param == value {
                    let target = *arguments.get(1)?;
                    if target.starts_with('.') {
                        pointer = *labels.get(target)?;
                    } else {
                        execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
                        return Some(());
                    }
                }
            }
            "anim_if_param_and" => {
                let mask = parse_visible_battle_animation_int(arguments.first()?)?;
                if timeline.anim_param & mask != 0 {
                    pointer = *labels.get(*arguments.get(1)?)?;
                }
            }
            "anim_jumpuntil" => {
                if timeline.anim_param > 0 {
                    timeline.anim_param -= 1;
                    pointer = *labels.get(*arguments.first()?)?;
                }
            }
            "anim_ret" => return Some(()),
            _ => {}
        }
    }
    Some(())
}

fn visible_null_battle_animation_object_lifetime(
    bundle: &serde_json::Value,
    frameset_name: &str,
) -> Option<u16> {
    let frames = bundle.get("framesets")?.get(frameset_name)?.as_array()?;
    let mut duration = 0_u16;
    for frame in frames {
        match frame.get("command")?.as_str()? {
            "frame" | "wait" => {
                let frames = frame.get("duration")?.as_u64()?.max(1);
                duration = duration.saturating_add(u16::try_from(frames).ok()?);
            }
            "delete" => return Some(duration.max(1)),
            "restart" | "end" => return None,
            _ => return None,
        }
    }
    None
}

fn parse_visible_battle_animation_int(token: &str) -> Option<i32> {
    let token = token.trim().replace('_', "");
    if let Some(hex) = token.strip_prefix('$') {
        i32::from_str_radix(hex, 16).ok()
    } else if let Some(binary) = token.strip_prefix('%') {
        i32::from_str_radix(binary, 2).ok()
    } else if let Some(hex) = token.strip_prefix("0x") {
        i32::from_str_radix(hex, 16).ok()
    } else if let Some(binary) = token.strip_prefix("0b") {
        i32::from_str_radix(binary, 2).ok()
    } else {
        token.parse::<i32>().ok()
    }
}

fn battle_stat_display_name(stat: &str) -> &str {
    match stat {
        "ATTACK" => "ATTACK",
        "DEFENSE" => "DEFENSE",
        "SPEED" => "SPEED",
        "SPECIAL_ATTACK" => "SPCL.ATK",
        "SPECIAL_DEFENSE" => "SPCL.DEF",
        "ACCURACY" => "ACCURACY",
        "EVASION" => "EVASION",
        "ABILITY" => "ABILITY",
        _ => "INVALID STAT",
    }
}

fn battle_event_changes_visible_scene(
    event: &crate::core::battle::turn::BattleEvent,
) -> bool {
    use crate::core::battle::turn::BattleEvent;

    matches!(
        event,
        BattleEvent::Switched { .. }
            | BattleEvent::Damage { .. }
            | BattleEvent::ResidualStatusDamage { .. }
            | BattleEvent::LeechSeedDamage { .. }
            | BattleEvent::CurseDamage { .. }
            | BattleEvent::NightmareDamage { .. }
            | BattleEvent::TrapDamage { .. }
            | BattleEvent::SpikesDamage { .. }
            | BattleEvent::FutureSightDamage { .. }
            | BattleEvent::SandstormDamage { .. }
            | BattleEvent::ConfusionSelfDamage { .. }
            | BattleEvent::HealApplied { .. }
            | BattleEvent::PresentHeal { .. }
            | BattleEvent::HpDrained { .. }
            | BattleEvent::LeechSeedDrain { .. }
            | BattleEvent::PainSplitApplied { .. }
            | BattleEvent::CounterDamage { .. }
            | BattleEvent::BideReleased { .. }
            | BattleEvent::SubstituteCreated { .. }
            | BattleEvent::SubstituteDamaged { .. }
            | BattleEvent::SubstituteBroken { .. }
            | BattleEvent::TransformApplied { .. }
            | BattleEvent::CurseApplied { .. }
            | BattleEvent::SelfdestructDamage { .. }
            | BattleEvent::Fainted { .. }
            | BattleEvent::StatusApplied { .. }
            | BattleEvent::StatusHealed { .. }
            | BattleEvent::HealBellChimed { .. }
            | BattleEvent::HeldItemStatusHealed { .. }
            | BattleEvent::HeldItemHpHealed { .. }
            | BattleEvent::HeldItemPpRestored { .. }
            | BattleEvent::WokeUp { .. }
            | BattleEvent::BattleItemEffect { .. }
            | BattleEvent::RecoilDamage { .. }
            | BattleEvent::JumpKickCrash { .. }
    )
}

fn retarget_visible_battle_hp_tween(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
) {
    let player_target_pixels = scene
        .battle
        .as_ref()
        .and_then(|battle| battle.active_player_party_index)
        .and_then(|index| scene.party.slots.iter().find(|slot| slot.index == index))
        .map(|slot| battle_hud_hp_pixels(slot.pokemon.hp, slot.pokemon.max_hp));
    let enemy_target_pixels = scene
        .battle
        .as_ref()
        .map(|battle| battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp));
    let Some(tween) = runtime_shell.battle_hp_tween.as_mut() else {
        return;
    };
    if let Some(pixels) = player_target_pixels {
        tween.player_target_pixels = pixels;
        tween.player_frames_until_step = 0;
    }
    if let Some(slot) = scene
        .battle
        .as_ref()
        .and_then(|battle| battle.active_player_party_index)
        .and_then(|index| scene.party.slots.iter().find(|slot| slot.index == index))
    {
        tween.player_target_hp = slot.pokemon.hp;
        tween.player_max_hp = slot.pokemon.max_hp;
    }
    if let Some(pixels) = enemy_target_pixels {
        tween.enemy_target_pixels = pixels;
        tween.enemy_frames_until_step = 0;
    }
}

fn apply_visible_battle_event_to_scene(
    scene: &mut RuntimeShellSnapshot,
    event: &crate::core::battle::turn::BattleEvent,
    pending_baton_pass_sides: &mut BTreeSet<crate::core::battle::turn::BattleSide>,
) {
    use crate::core::battle::turn::{BattleEvent, BattleSide};

    match event {
        BattleEvent::AirborneStarted { side, .. } => {
            if let Some(battle) = scene.battle.as_mut() {
                match side {
                    BattleSide::Player => battle.player_semi_invulnerable = true,
                    BattleSide::Enemy => battle.enemy_semi_invulnerable = true,
                }
            }
            return;
        }
        BattleEvent::AirborneEnded { side, .. } => {
            if let Some(battle) = scene.battle.as_mut() {
                match side {
                    BattleSide::Player => battle.player_semi_invulnerable = false,
                    BattleSide::Enemy => battle.enemy_semi_invulnerable = false,
                }
            }
            return;
        }
        BattleEvent::BatonPassed { side, .. } => {
            pending_baton_pass_sides.insert(*side);
            return;
        }
        BattleEvent::TransformApplied { side, species, .. } => {
            let Some(battle) = scene.battle.as_mut() else {
                return;
            };
            match side {
                BattleSide::Player => battle.player_transformed_species = Some(species.clone()),
                BattleSide::Enemy => battle.enemy_transformed_species = Some(species.clone()),
            }
            return;
        }
        BattleEvent::SubstituteCreated {
            side,
            substitute_hp,
            ..
        }
        | BattleEvent::SubstituteDamaged {
            target: side,
            substitute_hp_after: substitute_hp,
            ..
        } => {
            set_visible_battle_substitute_hp(scene, *side, *substitute_hp);
        }
        BattleEvent::SubstituteBroken { target, .. } => {
            set_visible_battle_substitute_hp(scene, *target, 0);
        }
        BattleEvent::BattleItemEffect { side, outcome } => {
            set_visible_battle_side_hp(scene, *side, outcome.hp_after);
            set_visible_battle_side_status(scene, *side, outcome.status_after.clone());
            return;
        }
        BattleEvent::HeldItemHpHealed { side, hp_after, .. } => {
            set_visible_battle_side_hp(scene, *side, *hp_after);
            return;
        }
        BattleEvent::StatusApplied { target, status, .. } => {
            set_visible_battle_side_status(scene, *target, Some(status.clone()));
            return;
        }
        BattleEvent::StatusHealed { target, .. } | BattleEvent::WokeUp { side: target, .. } => {
            set_visible_battle_side_status(scene, *target, None);
            return;
        }
        BattleEvent::HealBellChimed {
            side,
            active_status_before: Some(_),
        } => {
            set_visible_battle_side_status(scene, *side, None);
            return;
        }
        BattleEvent::HeldItemStatusHealed {
            side,
            status_before: Some(_),
            ..
        } => {
            set_visible_battle_side_status(scene, *side, None);
            return;
        }
        BattleEvent::Fainted { side } => {
            set_visible_battle_side_hp(scene, *side, 0);
            return;
        }
        _ => {}
    }

    if let BattleEvent::PainSplitApplied {
        side,
        target,
        user_hp_after,
        target_hp_after,
        ..
    } = event
    {
        set_visible_battle_side_hp(scene, *side, *user_hp_after);
        set_visible_battle_side_hp(scene, *target, *target_hp_after);
        return;
    }

    if let BattleEvent::HpDrained { side, hp_after, .. }
    | BattleEvent::LeechSeedDrain { side, hp_after, .. } = event
    {
        set_visible_battle_side_hp(scene, *side, *hp_after);
        return;
    }

    if let BattleEvent::PresentHeal {
        target, hp_after, ..
    }
    | BattleEvent::CounterDamage {
        target,
        defender_hp_after: hp_after,
        ..
    }
    | BattleEvent::BideReleased {
        target,
        target_hp_after: hp_after,
        ..
    } = event
    {
        set_visible_battle_side_hp(scene, *target, *hp_after);
        return;
    }

    if let BattleEvent::SubstituteCreated { side, hp_after, .. }
    | BattleEvent::CurseApplied { side, hp_after, .. } = event
    {
        set_visible_battle_side_hp(scene, *side, *hp_after);
        return;
    }

    if let BattleEvent::SelfdestructDamage { side, .. } = event {
        set_visible_battle_side_hp(scene, *side, 0);
        return;
    }

    if let BattleEvent::Switched { side, party_index } = event {
        let baton_pass_switch = pending_baton_pass_sides.remove(side);
        let Some(battle) = scene.battle.as_mut() else {
            return;
        };
        match side {
            BattleSide::Player => {
                battle.active_player_party_index = Some(*party_index);
                battle.player_transformed_species = None;
                if !baton_pass_switch {
                    battle.player_substitute_hp = 0;
                }
            }
            BattleSide::Enemy => {
                battle.active_enemy_party_index = Some(*party_index);
                battle.enemy_transformed_species = None;
                if !baton_pass_switch {
                    battle.enemy_substitute_hp = 0;
                }
                if let Some(pokemon) = battle.enemy_party.get(*party_index).cloned() {
                    battle.enemy_pokemon = pokemon;
                }
            }
        }
        return;
    }

    let hp_update = match event {
        BattleEvent::Damage {
            side,
            defender_hp_after,
            ..
        } => Some((
            match side {
                BattleSide::Player => BattleSide::Enemy,
                BattleSide::Enemy => BattleSide::Player,
            },
            *defender_hp_after,
        )),
        BattleEvent::ResidualStatusDamage { side, hp_after, .. }
        | BattleEvent::LeechSeedDamage { side, hp_after, .. }
        | BattleEvent::CurseDamage { side, hp_after, .. }
        | BattleEvent::NightmareDamage { side, hp_after, .. }
        | BattleEvent::TrapDamage { side, hp_after, .. }
        | BattleEvent::SpikesDamage { side, hp_after, .. }
        | BattleEvent::FutureSightDamage { side, hp_after, .. }
        | BattleEvent::SandstormDamage { side, hp_after, .. }
        | BattleEvent::ConfusionSelfDamage { side, hp_after, .. }
        | BattleEvent::HealApplied { side, hp_after, .. }
        | BattleEvent::RecoilDamage { side, hp_after, .. }
        | BattleEvent::JumpKickCrash { side, hp_after, .. } => Some((*side, *hp_after)),
        _ => None,
    };
    let Some((side, hp)) = hp_update else {
        return;
    };
    set_visible_battle_side_hp(scene, side, hp);
}

fn set_visible_battle_side_hp(
    scene: &mut RuntimeShellSnapshot,
    side: crate::core::battle::turn::BattleSide,
    hp: u16,
) {
    use crate::core::battle::turn::BattleSide;

    let Some(battle) = scene.battle.as_mut() else {
        return;
    };
    match side {
        BattleSide::Enemy => battle.enemy_pokemon.hp = hp,
        BattleSide::Player => {
            let Some(active_index) = battle.active_player_party_index else {
                return;
            };
            if let Some(slot) = scene
                .party
                .slots
                .iter_mut()
                .find(|slot| slot.index == active_index)
            {
                slot.pokemon.hp = hp;
            }
        }
    }
}

fn set_visible_battle_side_status(
    scene: &mut RuntimeShellSnapshot,
    side: crate::core::battle::turn::BattleSide,
    status: Option<String>,
) {
    use crate::core::battle::turn::BattleSide;

    let Some(battle) = scene.battle.as_mut() else {
        return;
    };
    match side {
        BattleSide::Enemy => battle.enemy_pokemon.status = status,
        BattleSide::Player => {
            let Some(active_index) = battle.active_player_party_index else {
                return;
            };
            if let Some(slot) = scene
                .party
                .slots
                .iter_mut()
                .find(|slot| slot.index == active_index)
            {
                slot.pokemon.status = status;
            }
        }
    }
}

fn set_visible_battle_substitute_hp(
    scene: &mut RuntimeShellSnapshot,
    side: crate::core::battle::turn::BattleSide,
    hp: u16,
) {
    use crate::core::battle::turn::BattleSide;

    let Some(battle) = scene.battle.as_mut() else {
        return;
    };
    match side {
        BattleSide::Player => battle.player_substitute_hp = hp,
        BattleSide::Enemy => battle.enemy_substitute_hp = hp,
    }
}

fn format_battle_turn_summary(outcome: &crate::core::battle::turn::BattleTurnOutcome) -> String {
    format!(
        "player_hp={}/{} enemy_hp={}/{} turn={}",
        outcome.state.player.hp,
        outcome.state.player.max_hp,
        outcome.state.enemy.hp,
        outcome.state.enemy.max_hp,
        outcome.state.turn
    )
}

fn use_visible_repel(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.repel_steps.is_some(),
        "selected item is not a repel",
    )?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:repel"))?;
    if snapshot.progression.repel_steps_remaining > 0 {
        runtime_shell.field_notice = Some(visible_asm_text(
            &snapshot,
            "RepelUsedEarlierIsStillInEffectText",
        )?);
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "REPEL STILL IN EFFECT");
        return Ok(());
    }
    let item_use = match runtime_shell.shell.use_bag_repel_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field repel item={} steps={} consumed={} checksum={:?}",
        item_id, item_use.repel_steps_after, item_use.item_use.consumed, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_bicycle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "bicycle")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:bicycle"))?;
    if snapshot.overworld.mode == MovementMode::Bike
        && snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_ALWAYS_ON_BIKE")
    {
        runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "CantGetOffBikeText")?);
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "CAN'T GET OFF HERE");
        return Ok(());
    }
    let item_use = match runtime_shell.shell.use_bag_bicycle_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field bicycle item={} mode={:?}->{:?} checksum={:?}",
        item_id, item_use.mode_before, item_use.mode_after, item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("BICYCLE MODE {:?}", item_use.mode_after),
    );
    if item_use.mode_after == MovementMode::Bike {
        // BikeFunction starts MUSIC_BICYCLE before Script_GetOnBike owns the
        // acknowledgement textbox. The ordinary music synchronizer pauses
        // while field text is open, so queue this source boundary explicitly.
        queue_visible_current_music(runtime_shell)?;
    }
    close_visible_field_pack_without_log(runtime_shell);
    // Script_GetOnBike/Script_GetOffBike changes VAR_MOVEMENT before its
    // textbox, but UpdatePlayerSprite runs only after the acknowledgement.
    // Retain the pre-toggle LCD scene beneath the text so the bike/normal
    // sprite does not appear one source boundary early.
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    let display_name = item_display_name(&snapshot, &item_id);
    runtime_shell.field_notice = Some(match item_use.mode_after {
        MovementMode::Bike => format!(
            "{} got on the\n{}.",
            snapshot.trainer.player_name, display_name
        ),
        MovementMode::Normal => format!(
            "{} got off\nthe {}.",
            snapshot.trainer.player_name, display_name
        ),
        MovementMode::Skate | MovementMode::Surf | MovementMode::SurfPika => {
            anyhow::bail!("bicycle use ended in invalid mode {:?}", item_use.mode_after)
        }
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_town_map(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "town_map")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:town_map"))?;
    let item_use = runtime_shell.shell.use_bag_town_map_in_field(&item_id)?;
    open_visible_pokegear_menu(runtime_shell)?;
    runtime_shell.last_audio_events.push(format!(
        "field town_map item={} landmark={:?} checksum={:?}",
        item_id, item_use.landmark, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_escape_rope(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "escape_rope")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:escape_rope"))?;
    let item_use = match runtime_shell.shell.use_bag_escape_rope_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field escape_rope item={} destination={} warp={} checksum={:?}",
        item_id, item_use.destination_map, item_use.destination_warp_index, item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "ESCAPE ROPE TO {} WARP {}",
            item_use.destination_map, item_use.destination_warp_index
        ),
    );
    runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "UseEscapeRopeText")?);
    runtime_shell.pending_field_travel_arrival = true;
    runtime_shell.pending_field_travel_delay_frames = None;
    Ok(())
}

fn use_visible_fishing_rod(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let scene = runtime_shell.shell.snapshot()?;
    let rods = runtime_shell.shell.fishing_rod_ids();
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| rods.contains(&item.item_id),
        "selected item is not a fishing rod",
    )?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:fishing_rod"))?;
    let item_use = match runtime_shell.shell.use_bag_fishing_rod_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if fishing_error_is_cant_fish_here(&error) => {
            close_visible_field_pack_without_log(runtime_shell);
            retain_visible_field_notice_scene(runtime_shell, &scene);
            runtime_shell.field_notice = Some(visible_fishing_cant_cast_text());
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                "NOT EVEN A NIBBLE",
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field fishing item={} rod={} group={:?} bite={:?} battle={:?} checksum={:?}",
        item_id,
        item_use.rod,
        item_use.cast.session.group,
        item_use.cast.bite,
        item_use.cast.wild_battle,
        item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "FISHING {:?} {:?}",
            item_use.cast.bite, item_use.cast.wild_battle
        ),
    );
    close_visible_field_pack_without_log(runtime_shell);
    present_visible_fishing_cast(
        runtime_shell,
        &scene,
        item_use.cast.bite,
        item_use.cast.wild_battle.is_some(),
    )?;
    Ok(())
}

fn fishing_error_is_cant_fish_here(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<FishingError>(),
        Some(
            FishingError::CannotFishWhileSurfing
                | FishingError::FacingTileOutOfBounds
                | FishingError::FacingTileIsNotWater
        )
    )
}

fn visible_fishing_cant_cast_text() -> String {
    "There's no water\nto fish here.".to_string()
}

fn present_visible_fishing_cast(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
    bite: Option<bool>,
    starts_battle: bool,
) -> Result<()> {
    retain_visible_field_notice_scene(runtime_shell, scene);
    runtime_shell.field_notice_queue.clear();
    runtime_shell.pending_sweet_scent_nothing_notice = false;
    runtime_shell.visible_strength_notice_phase = None;
    runtime_shell.field_notice = None;
    runtime_shell.pending_field_battle_entry = false;
    runtime_shell.visible_fishing_animation = Some(VisibleFishingAnimation {
        phase: VisibleFishingPhase::Cast,
        frame: 0,
        facing_up: scene.overworld.facing == Direction::Up,
        bite: bite == Some(true),
        starts_battle,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn use_visible_itemfinder(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "itemfinder")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:itemfinder"))?;
    let item_use = match runtime_shell.shell.use_bag_itemfinder_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field itemfinder item={} found={:?} cues={} consumed={} checksum={:?}",
        item_id,
        item_use.found,
        item_use.itemfinder_sound_cues,
        item_use.item_use.consumed,
        item_use.state_checksum
    ));
    set_shell_action_status(runtime_shell, format!("ITEMFINDER {:?}", item_use.found));
    close_visible_field_pack_without_log(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_squirtbottle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "squirtbottle")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:squirtbottle"))?;
    let item_use = match runtime_shell.shell.use_bag_squirtbottle_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field squirtbottle item={} target={:?} movement={} script={:?} checksum={:?}",
        item_id,
        item_use.target_object_identifier,
        item_use.target_movement,
        item_use.target_script,
        item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SQUIRTBOTTLE TARGET {:?}",
            item_use.target_object_identifier
        ),
    );
    close_visible_field_pack_without_log(runtime_shell);
    consume_visible_dispatched_field_script(runtime_shell)?;
    Ok(())
}

fn consume_visible_dispatched_field_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.script_events.next_script.is_some() {
        take_visible_next_script(runtime_shell)?;
    }
    Ok(())
}

fn use_visible_coin_case(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "coin_case")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:coin_case"))?;
    let item_use = runtime_shell.shell.use_bag_coin_case_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field coin_case item={} {}={} checksum={:?}",
        item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
    ));
    open_visible_field_balance_boundary(
        runtime_shell,
        "FieldCoinCase",
        &item_id,
        &item_use.balance_label,
        item_use.balance,
    );
    Ok(())
}

fn use_visible_blue_card(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "blue_card")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:blue_card"))?;
    let item_use = runtime_shell.shell.use_bag_blue_card_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field blue_card item={} {}={} checksum={:?}",
        item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
    ));
    open_visible_field_balance_boundary(
        runtime_shell,
        "FieldBlueCard",
        &item_id,
        &item_use.balance_label,
        item_use.balance,
    );
    Ok(())
}

fn open_visible_field_balance_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    label: &str,
    item_id: &str,
    balance_label: &str,
    balance: impl Display,
) {
    close_visible_field_pack_without_log(runtime_shell);
    let balance = balance.to_string();
    runtime_shell.field_notice = Some(match label {
        "FieldCoinCase" => format!("Coins:\n{balance}"),
        "FieldBlueCard" => format!("You now have\n{balance} points."),
        _ => unreachable!("unknown field balance surface {label}"),
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "opened field balance text {label} item={item_id} {balance_label}={balance}"
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn use_visible_surf(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "surf",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:surf:{party_index}"))?;
    let field_move = runtime_shell.shell.use_surf_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field surf party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("SURF PARTY #{} {:?}", party_index, field_move.outcome),
    );
    runtime_shell.pending_surf_start_from = Some(field_move.outcome.from_tile);
    runtime_shell.pending_field_notice_effect_frames =
        Some(WALK_FRAME_HOLD_TICKS.saturating_mul(2));
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "SURF")?);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn execute_visible_contextual_field_move(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    if !visible_field_shortcut_allowed(runtime_shell) {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(permission) = runtime_shell.shell.facing_tile_collision_permission()? else {
        return Ok(false);
    };
    if runtime_shell_has_object_movement_target(
        runtime_shell,
        &snapshot,
        "SPRITEMOVEDATA_STRENGTH_BOULDER",
    )? {
        if snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_STRENGTH_ACTIVE")
        {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "BouldersMoveText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        } else if snapshot_has_field_move_actor_and_badge(
            &snapshot,
            &runtime_shell.shell,
            "strength",
        )? {
            open_visible_contextual_field_move_prompt(
                runtime_shell,
                PartyFieldMove::Strength,
                "AskStrengthText",
            )?;
        } else {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "BouldersMayMoveText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(true);
    }
    if field_move_rule_contains_target_collision(&runtime_shell.shell, "waterfall", permission)? {
        if matches!(snapshot.overworld.mode, MovementMode::Surf | MovementMode::SurfPika)
            && snapshot.overworld.facing == crate::core::world::map::Direction::Up
            && snapshot_has_field_move_actor_and_badge(
                &snapshot,
                &runtime_shell.shell,
                "waterfall",
            )?
        {
            open_visible_contextual_field_move_prompt(
                runtime_shell,
                PartyFieldMove::Waterfall,
                "AskWaterfallText",
            )?;
        } else {
            runtime_shell.field_notice =
                Some(visible_asm_text(&snapshot, "HugeWaterfallText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(true);
    }
    if !matches!(snapshot.overworld.mode, MovementMode::Surf | MovementMode::SurfPika)
        && field_move_rule_allows_surf_target(&runtime_shell.shell, permission)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "surf")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Surf,
            "AskSurfText",
        )?;
        return Ok(true);
    }
    if field_move_rule_contains_target_collision(&runtime_shell.shell, "cut", permission)? {
        if snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "cut")? {
            open_visible_contextual_field_move_prompt(
                runtime_shell,
                PartyFieldMove::Cut,
                "AskCutText",
            )?;
        } else {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "CanCutText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(true);
    }
    if field_move_rule_contains_target_collision(&runtime_shell.shell, "whirlpool", permission)? {
        if snapshot_has_field_move_actor_and_badge(
            &snapshot,
            &runtime_shell.shell,
            "whirlpool",
        )? {
            open_visible_contextual_field_move_prompt(
                runtime_shell,
                PartyFieldMove::Whirlpool,
                "AskWhirlpoolText",
            )?;
        } else {
            runtime_shell.field_notice =
                Some(visible_asm_text(&snapshot, "MayPassWhirlpoolText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(true);
    }
    // Smashable rocks are OBJECTTYPE_SCRIPT interactions. Their A-button
    // path must continue through jumpstd SmashRockScript -> AskRockSmashScript;
    // only an explicit party-menu selection queues RockSmashFromMenuScript.
    if snapshot_has_headbutt_target(&snapshot)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "headbutt")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Headbutt,
            "AskHeadbuttText",
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn open_visible_contextual_field_move_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    field_move: PartyFieldMove,
    text_label: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.party_cursor = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        party_field_move_rule_id(field_move),
        usize::MAX,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "field_move:contextual:{}:prompt",
            party_field_move_rule_id(field_move)
        ),
    )?;
    runtime_shell.pending_contextual_field_move = Some(field_move);
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "field:move-confirm".to_string(),
        option_index: 0,
    });
    runtime_shell.field_notice = Some(visible_asm_text(&snapshot, text_label)?);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn resolve_visible_contextual_field_move_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    let Some(field_move) = runtime_shell.pending_contextual_field_move.take() else {
        anyhow::bail!("contextual field-move confirmation is not active");
    };
    runtime_shell.yes_no_cursor = None;
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.pending_sweet_scent_nothing_notice = false;
    runtime_shell.visible_strength_notice_phase = None;
    runtime_shell.field_notice_scene = None;
    if !accepted {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "field_move:contextual:{}:decline",
                party_field_move_rule_id(field_move)
            ),
        )?;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    execute_visible_party_field_move(runtime_shell, field_move)
}

fn field_move_rule_contains_target_collision(
    shell: &RuntimeGameShell,
    rule_id: &str,
    permission: u8,
) -> Result<bool> {
    let Some(key) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
    else {
        return Ok(false);
    };
    if !matches!(key.move_id.as_deref(), Some(move_id) if !move_id.is_empty()) {
        anyhow::bail!("compiled field move rule {rule_id} has no move_id");
    }
    require_visible_field_move_badge_shape(rule_id, key.badge_region.as_deref(), key.badge_index)?;
    Ok(key.target_collisions.contains(&permission))
}

fn require_visible_field_move_badge_shape(
    rule_id: &str,
    region: Option<&str>,
    index: Option<usize>,
) -> Result<()> {
    match (region, index) {
        (Some("johto" | "kanto"), Some(index)) if index < 8 => Ok(()),
        (region, index) => {
            anyhow::bail!(
                "compiled field move rule {rule_id} has invalid badge requirement {region:?} {index:?}"
            );
        }
    }
}

fn snapshot_has_field_move_actor_and_badge(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
) -> Result<bool> {
    let Some(rule) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
    else {
        return Ok(false);
    };
    let Some(move_id) = rule.move_id.as_deref() else {
        anyhow::bail!("compiled field move rule {rule_id} has no move_id");
    };
    if !snapshot.party.slots.iter().any(|slot| {
        slot.pokemon
            .moves
            .iter()
            .any(|learned| learned.name == move_id)
    }) {
        return Ok(false);
    }
    match (rule.badge_region.as_deref(), rule.badge_index) {
        (Some("johto"), Some(index)) => {
            visible_badge_at(&snapshot.progression.badges.johto, rule_id, index)
        }
        (Some("kanto"), Some(index)) => {
            visible_badge_at(&snapshot.progression.badges.kanto, rule_id, index)
        }
        (None, None) => Ok(true),
        (region, index) => {
            anyhow::bail!(
                "compiled field move rule {rule_id} has invalid badge requirement {region:?} {index:?}"
            )
        }
    }
}

fn visible_badge_at(badges: &[bool; 8], rule_id: &str, index: usize) -> Result<bool> {
    badges.get(index).copied().with_context(|| {
        format!("compiled field move rule {rule_id} badge index {index} is out of range")
    })
}

fn field_move_rule_allows_surf_target(shell: &RuntimeGameShell, permission: u8) -> Result<bool> {
    let Some(key) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == "surf")
    else {
        return Ok(false);
    };
    if !matches!(key.move_id.as_deref(), Some(move_id) if !move_id.is_empty()) {
        anyhow::bail!("compiled field move rule surf has no move_id");
    }
    require_visible_field_move_badge_shape("surf", key.badge_region.as_deref(), key.badge_index)?;
    Ok(
        crate::core::world::collision::describe_collision(permission).terrain
            == crate::core::world::collision::Terrain::Water
            && !key.blocked_collisions.contains(&permission),
    )
}

fn runtime_shell_has_object_movement_target(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    movement: &str,
) -> Result<bool> {
    let Some(interaction) = runtime_shell
        .shell
        .current_overworld_interaction_checked()?
    else {
        return Ok(false);
    };
    if interaction.map_name != snapshot.overworld.map_name {
        return Ok(false);
    }
    let crate::core::world::session::OverworldInteractionTarget::Object {
        object_identifier, ..
    } = &interaction.target
    else {
        return Ok(false);
    };
    for object in &snapshot.visible_objects {
        if object_identifier
            .as_ref()
            .is_some_and(|id| object.object_identifier.as_ref() == Some(id))
            || snapshot_object_tile_matches_checked(snapshot, object, interaction.target_tile)?
        {
            return Ok(object.spritemovedata == movement);
        }
    }
    Ok(false)
}

fn snapshot_has_headbutt_target(snapshot: &RuntimeShellSnapshot) -> Result<bool> {
    let Some(block) = facing_metatile_block(snapshot)? else {
        return Ok(false);
    };
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "active overworld map {} is missing from verified map catalog",
                snapshot.overworld.map_name
            )
        })?;
    let tileset = snapshot
        .tilesets
        .iter()
        .find(|tileset| tileset.tileset_id == map.attributes.tileset_name)
        .with_context(|| {
            format!(
                "active map {} references missing verified tileset {}",
                map.map_name, map.attributes.tileset_name
            )
        })?;
    Ok(
        tileset_collision_tokens(tileset, block).is_some_and(|tokens| {
            tokens
                .iter()
                .any(|token| matches!(token.as_str(), "HEADBUTT_TREE" | "HEADBUTT_TREE_1D"))
        }),
    )
}

fn facing_metatile_block(snapshot: &RuntimeShellSnapshot) -> Result<Option<u16>> {
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "active overworld map {} is missing from verified map catalog",
                snapshot.overworld.map_name
            )
        })?;
    let TilePosition {
        x: tile_x,
        y: tile_y,
    } = facing_runtime_tile(snapshot)?;
    if tile_x < 0 || tile_y < 0 {
        return Ok(None);
    }
    let Some((metatile_x, metatile_y)) = facing_metatile_coordinates(tile_x, tile_y)? else {
        return Ok(None);
    };
    if metatile_x >= map.attributes.width || metatile_y >= map.attributes.height {
        return Ok(None);
    }
    let index =
        usize::from(metatile_y) * usize::from(map.attributes.width) + usize::from(metatile_x);
    let block = map.blocks.get(index).copied().with_context(|| {
        format!(
            "active map {} block index {} is outside verified block count {}",
            map.map_name,
            index,
            map.blocks.len()
        )
    })?;
    Ok(Some(block))
}

fn facing_metatile_coordinates(tile_x: i16, tile_y: i16) -> Result<Option<(u16, u16)>> {
    if tile_x < 0 || tile_y < 0 {
        return Ok(None);
    }
    if tile_x % METATILE_WIDTH != 0 || tile_y % METATILE_WIDTH != 0 {
        return Ok(None);
    }
    runtime_tile_to_metatile_u16(tile_x, tile_y, "facing metatile block").map(Some)
}

fn use_visible_cut(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target_tile = facing_runtime_tile(&snapshot)?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "cut",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:cut:{party_index}:front"))?;
    let field_move = runtime_shell
        .shell
        .use_cut_field_move_in_front(party_index)?;
    let metatile_x = field_move.outcome.metatile_x;
    let metatile_y = field_move.outcome.metatile_y;
    runtime_shell.visible_cut_animation = Some(VisibleCutAnimation {
        target_tile,
        facing: snapshot.overworld.facing,
        variant: field_move.outcome.variant.clone(),
        frame: 0,
    });
    runtime_shell.pending_field_notice_sound =
        Some("SFX_PLACE_PUZZLE_PIECE_DOWN".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(32);
    runtime_shell.last_audio_events.push(format!(
        "field cut party_index={} target=({}, {}) outcome={:?} checksum={:?}",
        party_index, metatile_x, metatile_y, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "CUT ({}, {}) {:?}",
            metatile_x, metatile_y, field_move.outcome
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "CUT")?);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_whirlpool(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target_tile = facing_runtime_tile(&snapshot)?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "whirlpool",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:whirlpool:{party_index}:front"),
    )?;
    let field_move = runtime_shell
        .shell
        .use_whirlpool_field_move_in_front(party_index)?;
    let metatile_x = field_move.outcome.metatile_x;
    let metatile_y = field_move.outcome.metatile_y;
    runtime_shell.last_audio_events.push(format!(
        "field whirlpool party_index={} target=({}, {}) outcome={:?} checksum={:?}",
        party_index, metatile_x, metatile_y, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "WHIRLPOOL ({}, {}) {:?}",
            metatile_x, metatile_y, field_move.outcome
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice =
        Some(visible_field_move_use_text(&snapshot, party_index, "WHIRLPOOL")?);
    runtime_shell.pending_field_notice_sound = Some("SFX_SURF".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(32);
    runtime_shell.visible_whirlpool_animation = Some(VisibleWhirlpoolAnimation {
        target_tile,
        frame: 0,
    });
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_strength(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "strength",
        runtime_shell.party_cursor,
    )?;
    anyhow::ensure!(
        runtime_shell.field_notice_queue.is_empty(),
        "Strength started while another field notice sequence was queued"
    );
    record_visible_runtime_action(runtime_shell, format!("field_move:strength:{party_index}"))?;
    let field_move = runtime_shell.shell.use_strength_field_move(party_index)?;
    let strength_user = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .context("Strength user disappeared from the retained party")?;
    runtime_shell.last_audio_events.push(format!(
        "field strength party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("STRENGTH PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice =
        Some(visible_field_move_use_text(&snapshot, party_index, "STRENGTH")?);
    runtime_shell.pending_field_notice_cry = Some(strength_user.pokemon.species.id.clone());
    runtime_shell
        .field_notice_queue
        .push_back(format!("{} can\nmove boulders.", strength_user.pokemon.nickname));
    runtime_shell.visible_strength_notice_phase = Some(VisibleStrengthNoticePhase::UseText);
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_flash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "flash",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:flash:{party_index}"))?;
    let field_move = runtime_shell.shell.use_flash_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field flash party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("FLASH PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "FLASH")?);
    runtime_shell.visible_flash_animation = Some(VisibleFlashAnimation { frame: 0 });
    runtime_shell.pending_field_notice_sound = Some("SFX_FLASH".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(16);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_waterfall(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "waterfall",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:waterfall:{party_index}"))?;
    let field_move = runtime_shell.shell.use_waterfall_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field waterfall party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("WATERFALL PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice =
        Some(visible_field_move_use_text(&snapshot, party_index, "WATERFALL")?);
    runtime_shell.pending_field_notice_sound = Some("SFX_BUBBLEBEAM".to_string());
    runtime_shell.visible_waterfall_animation = Some(VisibleWaterfallAnimation {
        from_tile: field_move.outcome.from_tile,
        to_tile: field_move.outcome.to_tile,
        steps: field_move.outcome.steps,
        frame: 0,
    });
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_fly(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "fly",
        runtime_shell.party_cursor,
    )?;
    let destinations = active_fly_destinations(&snapshot, &runtime_shell.shell);
    if destinations.is_empty() {
        runtime_shell.fly_cursor = None;
        record_visible_runtime_action(runtime_shell, "field_move:fly:no_destinations")?;
        runtime_shell
            .last_audio_events
            .push("no active FLYPOINT engine flags".to_string());
        set_shell_action_status(runtime_shell, "NO FLY DESTINATIONS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let selected_index = visible_cursor_index(
        &mut runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    );
    let destination = destinations[selected_index].clone();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "field_move:fly:{}:{}:{}:{}",
            party_index,
            selected_index,
            destination.flypoint_flag,
            destination.destination_spawn_identifier
        ),
    )?;
    let field_move = runtime_shell.shell.use_fly_field_move(
        party_index,
        destination.destination_spawn_identifier,
        &destination.flypoint_flag,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "field fly destination {}/{} flag={} party_index={} spawn={} map={} tile=({}, {}) checksum={:?}",
        selected_index + 1,
        destinations.len(),
        field_move.flypoint_flag,
        party_index,
        field_move.destination_spawn_identifier,
        field_move.destination_map,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "FLY TO {} ({}, {})",
            field_move.destination_map,
            field_move.destination_tile.x,
            field_move.destination_tile.y
        ),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.pending_field_travel_arrival = false;
    runtime_shell.pending_field_travel_delay_frames = None;
    runtime_shell.visible_fly_animation = Some(VisibleFlyAnimation {
        phase: VisibleFlyAnimationPhase::From,
        frame: 0,
    });
    let BevyRuntimeShell {
        shell,
        pending_audio,
        last_audio_events,
        ..
    } = runtime_shell;
    queue_visible_sound_effect(
        shell.runtime().audio(),
        pending_audio,
        last_audio_events,
        "SFX_FLY",
    )?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn active_fly_destinations(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
) -> Vec<RuntimeFlyDestinationKey> {
    let use_kanto_map = visible_pokegear_region(snapshot) == "KANTO"
        && snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_FLYPOINT_INDIGO_PLATEAU");
    shell
        .fly_destination_keys()
        .into_iter()
        .filter(|destination| {
            let destination_is_kanto = snapshot
                .presentation
                .pokegear_landmarks
                .landmarks
                .iter()
                .find(|landmark| landmark.constant == destination.label)
                .is_some_and(|landmark| landmark.region == "KANTO");
            if destination_is_kanto != use_kanto_map {
                return false;
            }
            let is_default = if use_kanto_map {
                destination.label == "LANDMARK_INDIGO_PLATEAU"
            } else {
                destination.label == "LANDMARK_SILVER_CAVE"
            };
            is_default
                || snapshot
                    .progression
                    .active_engine_flags
                    .contains(&destination.flypoint_flag)
        })
        .collect()
}

fn fly_destination_label(destination: &RuntimeFlyDestinationKey) -> String {
    destination
        .label
        .strip_prefix("LANDMARK_")
        .unwrap_or(&destination.label)
        .replace('_', " ")
}

fn use_visible_dig(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "dig",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:dig:{party_index}"))?;
    let field_move = runtime_shell.shell.use_dig_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field dig party_index={} destination={} warp={} tile=({}, {}) checksum={:?}",
        party_index,
        field_move.destination_map,
        field_move.destination_warp_index,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "DIG TO {} WARP {}",
            field_move.destination_map, field_move.destination_warp_index
        ),
    );
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "DIG")?);
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.pending_field_travel_arrival = true;
    runtime_shell.pending_field_travel_delay_frames = None;
    runtime_shell.visible_field_travel_animation = Some(VisibleFieldTravelAnimation::DigOut);
    Ok(())
}

fn use_visible_teleport(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "teleport",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:teleport:{party_index}"))?;
    let field_move = runtime_shell.shell.use_teleport_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field teleport party_index={} destination={} spawn={} tile=({}, {}) checksum={:?}",
        party_index,
        field_move.destination_map,
        field_move.destination_spawn_identifier,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "TELEPORT TO {} ({}, {})",
            field_move.destination_map,
            field_move.destination_tile.x,
            field_move.destination_tile.y
        ),
    );
    runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "TeleportReturnText")?);
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.pending_field_travel_arrival = true;
    runtime_shell.pending_field_travel_delay_frames = Some(60);
    runtime_shell.visible_field_travel_animation =
        Some(VisibleFieldTravelAnimation::TeleportFrom);
    Ok(())
}

fn use_visible_headbutt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target_tile = facing_runtime_tile(&snapshot)?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "headbutt",
        runtime_shell.party_cursor,
    )?;
    let player_id = snapshot.trainer.player_id;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:headbutt:{party_index}:player:{player_id}"),
    )?;
    let field_move = runtime_shell
        .shell
        .use_headbutt_field_move(party_index, player_id)?;
    runtime_shell.visible_headbutt_animation = Some(VisibleHeadbuttAnimation {
        target_tile,
        facing: snapshot.overworld.facing,
        frame: 0,
    });
    runtime_shell.pending_field_notice_sound = Some("SFX_SANDSTORM".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(32);
    runtime_shell.last_audio_events.push(format!(
        "field headbutt party_index={} encounter={:?} battle={:?} checksum={:?}",
        party_index, field_move.field_encounter, field_move.wild_battle, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "HEADBUTT {:?} {:?}",
            field_move.field_encounter, field_move.wild_battle
        ),
    );
    settle_visible_field_move_after_possible_battle(
        runtime_shell,
        &snapshot,
        visible_field_move_use_text(&snapshot, party_index, "HEADBUTT")?,
    )?;
    Ok(())
}

fn use_visible_rock_smash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "rock_smash",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:rock_smash:{party_index}"),
    )?;
    let dispatch = runtime_shell
        .shell
        .queue_rock_smash_from_menu(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field rock_smash party_index={} script={} last_talked={:?} checksum={:?}",
        party_index,
        dispatch.next_script,
        dispatch.last_talked_object,
        dispatch.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("ROCK SMASH {}", dispatch.next_script),
    );
    consume_visible_dispatched_field_script(runtime_shell)
}

fn use_visible_sweet_scent(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "sweet_scent",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:sweet_scent:{party_index}"),
    )?;
    let field_move = runtime_shell
        .shell
        .use_sweet_scent_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field sweet_scent party_index={} encounter={:?} battle={:?} checksum={:?}",
        party_index,
        field_move.wild_encounter,
        field_move.wild_battle,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWEET SCENT {:?} {:?}",
            field_move.wild_encounter, field_move.wild_battle
        ),
    );
    if field_move
        .wild_encounter
        .as_ref()
        .and_then(|encounter| encounter.resolved.as_ref())
        .is_none()
    {
        runtime_shell.pending_sweet_scent_nothing_notice = true;
        runtime_shell
            .field_notice_queue
            .push_back(visible_asm_text(&snapshot, "SweetScentNothingText")?);
    } else {
        // TypeScript main holds the resolved encounter behind its twenty-frame
        // SWEET_SCENT field-animation timer and starts the sound at that
        // boundary. The battle must not replace the use textbox immediately.
        runtime_shell.pending_field_notice_sound = Some("SFX_SWEET_SCENT".to_string());
        runtime_shell.pending_field_notice_effect_frames = Some(20);
        runtime_shell.visible_sweet_scent_delay = true;
    }
    settle_visible_field_move_after_possible_battle(
        runtime_shell,
        &snapshot,
        visible_field_move_use_text(&snapshot, party_index, "SWEET SCENT")?,
    )?;
    Ok(())
}

fn use_visible_sweet_scent_current_surface(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_sweet_scent(runtime_shell)
}

fn settle_visible_field_action_after_possible_battle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    if runtime_shell.shell.snapshot()?.battle.is_some() {
        prepare_visible_battle_entry(runtime_shell)?;
        return settle_visible_battle_after_action(runtime_shell);
    }
    continue_visible_script_after_prompt(runtime_shell)
}

fn settle_visible_field_move_after_possible_battle(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
    notice: String,
) -> Result<()> {
    retain_visible_field_notice_scene(runtime_shell, scene);
    if runtime_shell.shell.snapshot()?.battle.is_some() {
        runtime_shell.pending_field_battle_entry = true;
        runtime_shell.field_notice = Some(notice);
        return Ok(());
    }
    runtime_shell.field_notice = Some(notice);
    continue_visible_script_after_prompt(runtime_shell)
}

fn retain_visible_field_notice_scene(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
) {
    runtime_shell.field_notice_scene = Some(Arc::new(scene.clone()));
}

fn carried_field_rule_item(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
) -> Result<String> {
    let Some(item_id) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.item_id)
    else {
        anyhow::bail!("compiled pack has no field item rule {rule_id}");
    };
    if carried_item_ids(snapshot).any(|carried| carried == item_id) {
        Ok(item_id)
    } else {
        anyhow::bail!("bag does not carry field item {item_id} for rule {rule_id}")
    }
}

fn field_rule_item_matches(shell: &RuntimeGameShell, rule_id: &str, item_id: &str) -> bool {
    shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.item_id)
        .is_some_and(|rule_item_id| rule_item_id == item_id)
}

fn carried_item_ids(snapshot: &RuntimeShellSnapshot) -> impl Iterator<Item = &str> {
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
        .chain(
            snapshot
                .bag
                .custom_pockets
                .values()
                .flat_map(|items| items.iter()),
        )
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.as_str())
}

fn carried_item_quantity(snapshot: &RuntimeShellSnapshot, item_id: &str) -> Option<u16> {
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
        .chain(
            snapshot
                .bag
                .custom_pockets
                .values()
                .flat_map(|items| items.iter()),
        )
        .find(|item| item.quantity > 0 && item.item_id == item_id)
        .map(|item| item.quantity)
}

fn item_catalog_detail_label(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    let Some(item) = snapshot.items.iter().find(|item| item.item_id == item_id) else {
        return "INVALID ITEM".to_string();
    };
    format!("{} ${}", item.pocket, item.price)
}

fn shop_buy_item_label(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    let Some(item) = snapshot.items.iter().find(|item| item.item_id == item_id) else {
        return format!("{item_id} INVALID ITEM");
    };
    format!("{} ${}", item.name.replace('_', " "), item.price)
}

fn shop_sell_item_label(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    let Some(quantity) = carried_item_quantity(snapshot, item_id) else {
        return format!("{item_id} INVALID INVENTORY");
    };
    let Some(item) = snapshot.items.iter().find(|item| item.item_id == item_id) else {
        return format!("{item_id} x{quantity} INVALID ITEM");
    };
    format!(
        "{} x{} ${}",
        item.name.replace('_', " "),
        quantity,
        u32::from(item.price / 2)
    )
}

fn sellable_carried_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    let mut item_ids = snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    item_ids.extend(
        snapshot
            .bag
            .custom_pockets
            .values()
            .flat_map(|items| items.iter())
            .filter(|item| item.quantity > 0)
            .map(|item| item.item_id.clone()),
    );
    item_ids
}

fn party_index_for_field_move_rule(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
    party_cursor: usize,
) -> Result<usize> {
    let Some(move_id) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.move_id)
    else {
        anyhow::bail!("compiled pack has no field move rule {rule_id}");
    };
    if let Some(selected) = snapshot.party.slots.get(party_cursor) {
        if selected
            .pokemon
            .moves
            .iter()
            .any(|learned| learned.name == move_id)
        {
            return Ok(selected.index);
        }
    }
    snapshot
        .party
        .slots
        .iter()
        .find(|slot| {
            slot.pokemon
                .moves
                .iter()
                .any(|learned| learned.name == move_id)
        })
        .map(|slot| slot.index)
        .with_context(|| {
            format!("party has no Pokemon with field move {move_id} for rule {rule_id}")
        })
}

fn visible_field_move_use_text(
    snapshot: &RuntimeShellSnapshot,
    party_index: usize,
    move_name: &str,
) -> Result<String> {
    let nickname = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .with_context(|| format!("field-move user party index {party_index} is missing"))?;
    Ok(format!("{nickname} used {move_name}!"))
}

fn facing_runtime_tile(snapshot: &RuntimeShellSnapshot) -> Result<TilePosition> {
    facing_runtime_tile_from(snapshot.overworld.tile, snapshot.overworld.facing)
}

fn facing_runtime_tile_from(
    tile: TilePosition,
    facing: crate::core::world::map::Direction,
) -> Result<TilePosition> {
    checked_move_by_stride(tile, facing, StepOptions::default().stride_tiles).with_context(|| {
        format!(
            "facing tile overflows runtime coordinates from ({}, {}) facing {:?}",
            tile.x, tile.y, facing
        )
    })
}

fn runtime_tile_to_metatile_u16(x: i16, y: i16, context: &str) -> Result<(u16, u16)> {
    if x < 0 || y < 0 {
        anyhow::bail!("runtime tile ({x}, {y}) is outside unsigned map coordinates for {context}");
    }
    if x % METATILE_WIDTH != 0 || y % METATILE_WIDTH != 0 {
        anyhow::bail!(
            "runtime tile ({x}, {y}) is not aligned to metatile width {METATILE_WIDTH} for {context}"
        );
    }
    let metatile_x = u16::try_from(x.div_euclid(METATILE_WIDTH)).with_context(|| {
        format!("runtime tile x {x} cannot convert to metatile coordinate for {context}")
    })?;
    let metatile_y = u16::try_from(y.div_euclid(METATILE_WIDTH)).with_context(|| {
        format!("runtime tile y {y} cannot convert to metatile coordinate for {context}")
    })?;
    Ok((metatile_x, metatile_y))
}

fn open_visible_battle_move_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "fight_open");
    };
    if battle.commands.player_move_slots.is_empty() {
        runtime_shell.battle_move_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:fight:no_moves")?;
        runtime_shell
            .last_audio_events
            .push("active battle has no available player moves".to_string());
        set_shell_action_status(runtime_shell, "NO MOVES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let move_menu_count = battle_move_menu_option_count(&snapshot, battle)?;
    visible_cursor_index(
        &mut runtime_shell.battle_move_cursor,
        "battle:moves",
        move_menu_count,
    );
    runtime_shell.battle_move_swap_origin = None;
    runtime_shell.battle_switch_cursor = None;
    runtime_shell.pending_battle_move_switch_slot = None;
    runtime_shell.battle_party_action_cursor = None;
    runtime_shell.battle_party_summary_open = false;
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened battle moves choices={:?}",
        battle.commands.player_move_slots
    ));
    set_shell_action_status(
        runtime_shell,
        format!("FIGHT MOVES {}", battle.commands.player_move_slots.len()),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_battle_switch_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "switch_open");
    };
    if snapshot.party.slots.is_empty() {
        runtime_shell.battle_switch_cursor = None;
        runtime_shell.battle_party_action_cursor = None;
        runtime_shell.battle_party_summary_open = false;
        record_visible_runtime_action(runtime_shell, "battle:pokemon:no_switches")?;
        runtime_shell
            .last_audio_events
            .push("active battle has no available party switches".to_string());
        set_shell_action_status(runtime_shell, "NO SWITCHES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    visible_cursor_index(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(&snapshot),
    );
    runtime_shell.battle_party_action_cursor = None;
    runtime_shell.battle_party_summary_open = false;
    runtime_shell.battle_move_cursor = None;
    runtime_shell.battle_move_swap_origin = None;
    runtime_shell.pending_battle_move_switch_slot = None;
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened battle switch choices={:?}",
        battle.commands.switch_party_indices
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWITCH CHOICES {}",
            battle.commands.switch_party_indices.len()
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_battle_move_switch_target(
    runtime_shell: &mut BevyRuntimeShell,
    move_slot: usize,
) -> Result<()> {
    open_visible_battle_switch_target(runtime_shell)?;
    if runtime_shell.battle_switch_cursor.is_some() {
        runtime_shell.pending_battle_move_switch_slot = Some(move_slot);
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:move_switch:{move_slot}:target_open"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "opened battle move-switch target for slot {move_slot}"
        ));
        set_shell_action_status(runtime_shell, "BATON PASS TARGET");
        trim_event_log(&mut runtime_shell.last_audio_events);
    }
    Ok(())
}

fn switch_visible_battle_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "switch_confirm");
    };
    let selected_index = visible_cursor_index(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(&snapshot),
    );
    if selected_index >= snapshot.party.slots.len() {
        return press_visible_battle_b_button(runtime_shell);
    }
    let slot = &snapshot.party.slots[selected_index];
    let party_index = slot.index;
    if battle.active_player_party_index == Some(party_index) {
        record_visible_runtime_action(runtime_shell, "battle:switch:already_active")?;
        runtime_shell
            .battle_messages
            .push_back(format!("{}\nis already out.", slot.pokemon.nickname));
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(
            runtime_shell,
            format!("{} IS ALREADY OUT", slot.pokemon.nickname),
        );
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let baton_pass_target = runtime_shell.pending_battle_move_switch_slot.is_some();
    let trainer_shift_target = trainer_shift_switch_pending(&snapshot, battle);
    if !baton_pass_target
        && !trainer_shift_target
        && !visible_active_battle_player_fainted(&snapshot)
        && (battle.player_cannot_escape || battle.player_wrapped)
    {
        let active_name = battle
            .active_player_party_index
            .and_then(|active| snapshot.party.slots.iter().find(|slot| slot.index == active))
            .map(|slot| slot.pokemon.nickname.as_str())
            .context("trapped battle switch requires the active player party Pokemon")?;
        record_visible_runtime_action(runtime_shell, "battle:switch:trapped")?;
        runtime_shell
            .battle_messages
            .push_back(format!("{active_name}\ncan't be recalled!"));
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "POKEMON CAN'T BE RECALLED");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if slot.pokemon.is_egg {
        record_visible_runtime_action(runtime_shell, "battle:switch:egg")?;
        runtime_shell
            .battle_messages
            .push_back("An EGG can't\nbattle!".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "AN EGG CAN'T BATTLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if slot.pokemon.hp == 0 {
        record_visible_runtime_action(runtime_shell, "battle:switch:no_will")?;
        runtime_shell
            .battle_messages
            .push_back("There's no will to\nbattle!".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "THERE'S NO WILL TO BATTLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if let Some(move_slot) = runtime_shell.pending_battle_move_switch_slot {
        return resolve_visible_battle_move_switch_to(runtime_shell, move_slot, party_index);
    }
    if trainer_shift_target {
        advance_visible_trainer_battle(runtime_shell)?;
        return switch_visible_trainer_shift_party_without_turn(runtime_shell, party_index);
    }
    switch_visible_battle_pokemon_to(runtime_shell, party_index)
}

fn battle_switch_option_count(snapshot: &RuntimeShellSnapshot) -> usize {
    snapshot.party.slots.len() + usize::from(!visible_active_battle_player_fainted(snapshot))
}

fn resolve_visible_battle_move_switch_to(
    runtime_shell: &mut BevyRuntimeShell,
    move_slot: usize,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "move_switch_to");
    };
    let battle_before_turn = battle.clone();
    if !battle.commands.player_move_slots.contains(&move_slot) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:move_switch:{move_slot}:unavailable"),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("player move slot {move_slot} is not available"));
        set_shell_action_status(runtime_shell, "MOVE UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if selected_battle_move_effect(&snapshot, move_slot)? != "BATON_PASS" {
        anyhow::bail!("move_switch action requires BATON_PASS effect at slot {move_slot}");
    }
    if !battle.commands.switch_party_indices.contains(&party_index) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:move_switch:{move_slot}:{party_index}:unavailable"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "party index {party_index} is not an available battle move switch"
        ));
        set_shell_action_status(runtime_shell, "SWITCH UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_slot = match &enemy_action {
        BattleAction::Move { slot } => Some(*slot),
        _ => None,
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:move_switch:{move_slot}:{party_index}:enemy:{enemy_action:?}"),
    )?;
    record_visible_battle_action_frame(
        runtime_shell,
        BattleAction::MoveSwitch {
            slot: move_slot,
            party_index,
        },
    )?;
    let turn = resolve_active_battle_turn_with_enemy_rng(
        runtime_shell,
        enemy_rng_seed_after,
        BattleAction::MoveSwitch {
            slot: move_slot,
            party_index,
        },
        enemy_action,
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle move-switch move_slot={} party_index={} enemy_slot={} {} events={} checksum={:?}",
        move_slot,
        party_index,
        enemy_slot.map_or_else(|| "switch".to_string(), |slot| slot.to_string()),
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    defer_visible_party_index_cry_after_send_out(
        runtime_shell,
        party_index,
        "battle_move_switch",
    )?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "BATON PASS PARTY #{} {}",
            party_index,
            format_battle_turn_summary(&turn.outcome)
        ),
    );
    settle_visible_resolved_battle_turn(runtime_shell, &battle_before_turn)
}

fn switch_visible_battle_pokemon_to(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "switch_to");
    };
    let battle_before_turn = battle.clone();
    if !battle.commands.switch_party_indices.contains(&party_index) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:switch:{party_index}:unavailable"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "party index {party_index} is not an available battle switch"
        ));
        set_shell_action_status(runtime_shell, "SWITCH UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if visible_active_battle_player_fainted(&snapshot) {
        return switch_visible_battle_party_without_turn(runtime_shell, party_index);
    }
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_slot = match &enemy_action {
        BattleAction::Move { slot } => Some(*slot),
        _ => None,
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:switch:{party_index}:enemy:{enemy_action:?}"),
    )?;
    record_visible_battle_action_frame(runtime_shell, BattleAction::Switch { party_index })?;
    let turn = resolve_active_battle_turn_with_enemy_rng(
        runtime_shell,
        enemy_rng_seed_after,
        BattleAction::Switch { party_index },
        enemy_action,
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle switch party_index={} enemy_slot={} {} events={} checksum={:?}",
        party_index,
        enemy_slot.map_or_else(|| "switch".to_string(), |slot| slot.to_string()),
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    defer_visible_party_index_cry_after_send_out(runtime_shell, party_index, "battle_switch")?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWITCH PARTY #{} {}",
            party_index,
            format_battle_turn_summary(&turn.outcome)
        ),
    );
    settle_visible_resolved_battle_turn(runtime_shell, &battle_before_turn)
}

fn settle_visible_battle_after_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MAX_BATTLE_SETTLE_STEPS: usize = 8;
    for _ in 0..MAX_BATTLE_SETTLE_STEPS {
        let snapshot = runtime_shell.shell.snapshot()?;
        let Some(battle) = snapshot.battle.as_ref() else {
            runtime_shell.trainer_items_used.clear();
            return Ok(());
        };
        if visible_active_battle_player_fainted(&snapshot) {
            handle_visible_player_fainted_battle_boundary(runtime_shell, &snapshot, battle)?;
            return Ok(());
        }
        if battle.enemy_pokemon.hp != 0 {
            return Ok(());
        }
        match battle.kind {
            RuntimeBattleKind::Trainer { .. } => {
                let Some(enemy_index) = battle.active_enemy_party_index else {
                    anyhow::bail!("active trainer battle has no active enemy party index");
                };
                if battle.rewarded_enemy_party_indices.contains(&enemy_index) {
                    advance_visible_trainer_battle(runtime_shell)?;
                } else {
                    claim_visible_battle_rewards(runtime_shell)?;
                    // Keep the defeated battler on screen for the complete
                    // EXP/level/move/evolution narration. The final text
                    // acknowledgement resumes settlement and reveals the
                    // replacement at its send-out boundary.
                    return Ok(());
                }
            }
            RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => {
                claim_visible_battle_rewards(runtime_shell)?;
            }
        }
    }
    anyhow::bail!(
        "battle settlement exceeded {MAX_BATTLE_SETTLE_STEPS} reward/advance steps before reaching player control"
    )
}

fn visible_active_battle_player_fainted(snapshot: &RuntimeShellSnapshot) -> bool {
    let active = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.is_active_battle_pokemon);
    if snapshot.battle.is_some() {
        active.is_some_and(|slot| slot.pokemon.hp == 0)
    } else {
        active.is_some_and(|slot| slot.pokemon.hp == 0)
    }
}

fn should_offer_trainer_shift_switch(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> bool {
    snapshot.trainer.options.battle_style == BattleStyle::Shift
        // A simultaneous knockout has no active player battler to keep or
        // recall. Crystal goes directly to the mandatory replacement flow;
        // presenting the optional Shift prompt here aliases that forced-party
        // cursor as a shift target and can trap the battle on a fainted row.
        && !visible_active_battle_player_fainted(snapshot)
        && trainer_shift_switch_pending(snapshot, battle)
        && !battle.commands.switch_party_indices.is_empty()
}

fn trainer_shift_switch_pending(
    _snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> bool {
    if !matches!(battle.kind, RuntimeBattleKind::Trainer { .. }) {
        return false;
    }
    if battle.enemy_pokemon.hp != 0 {
        return false;
    }
    let Some(enemy_index) = battle.active_enemy_party_index else {
        return false;
    };
    if !battle.rewarded_enemy_party_indices.contains(&enemy_index) {
        return false;
    }
    battle
        .enemy_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != enemy_index && pokemon.hp > 0)
}

fn confirm_visible_trainer_shift_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_shift_prompt_cursor,
        "battle:shift-prompt",
        2,
    )
    .context("trainer Shift prompt requires a valid YES/NO cursor")?;
    resolve_visible_trainer_shift_prompt(runtime_shell, selected == 0)
}

fn confirm_visible_wild_faint_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_faint_prompt_cursor,
        "battle:faint-prompt",
        2,
    )
    .context("wild faint prompt requires a valid YES/NO cursor")?;
    resolve_visible_wild_faint_prompt(runtime_shell, selected == 0)
}

fn resolve_visible_wild_faint_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    use_next_pokemon: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("wild faint prompt requires an active battle")?;
    if matches!(&battle.kind, RuntimeBattleKind::Trainer { .. })
        || !visible_active_battle_player_fainted(&snapshot)
        || battle.commands.switch_party_indices.is_empty()
    {
        anyhow::bail!("wild faint prompt is active outside a replaceable wild faint boundary");
    }
    runtime_shell.battle_faint_prompt_cursor = None;
    if use_next_pokemon {
        record_visible_runtime_action(runtime_shell, "battle:faint_prompt:yes")?;
        open_visible_battle_switch_target(runtime_shell)?;
        set_shell_action_status(runtime_shell, "CHOOSE NEXT POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let scripted_static_wild = visible_static_wild_source(&snapshot, battle);
    record_visible_runtime_action(runtime_shell, "battle:faint_prompt:no")?;
    let escape = runtime_shell.shell.attempt_escape_active_wild_battle()?;
    runtime_shell.last_audio_events.push(format!(
        "wild faint replacement escape outcome={:?} checksum={:?}",
        escape.outcome, escape.state_checksum
    ));
    runtime_shell.battle_message_scene = Some(Box::new(snapshot));
    if escape.outcome.escaped {
        runtime_shell
            .battle_messages
            .push_back("Got away safely!".to_string());
        finish_visible_wild_battle_exit(
            runtime_shell,
            scripted_static_wild,
            "wild_faint_replacement_escape",
        )
    } else {
        runtime_shell
            .battle_messages
            .push_back("Can't escape!".to_string());
        open_visible_battle_switch_target(runtime_shell)?;
        set_shell_action_status(runtime_shell, "CHOOSE NEXT POKEMON");
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        Ok(())
    }
}

fn resolve_visible_trainer_shift_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    change_pokemon: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("trainer Shift prompt requires an active battle")?;
    if !trainer_shift_switch_pending(&snapshot, battle) {
        anyhow::bail!("trainer Shift prompt is active outside the enemy replacement boundary");
    }
    runtime_shell.battle_shift_prompt_cursor = None;
    if change_pokemon {
        record_visible_runtime_action(runtime_shell, "battle:shift_prompt:yes")?;
        open_visible_battle_switch_target(runtime_shell)?;
        set_shell_action_status(runtime_shell, "SHIFT: CHOOSE POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "battle:shift_prompt:no")?;
    reset_visible_battle_action_cursors(runtime_shell);
    set_shell_action_status(runtime_shell, "SHIFT: KEEP CURRENT");
    trim_event_log(&mut runtime_shell.last_audio_events);
    advance_visible_trainer_battle(runtime_shell)
}

fn handle_visible_player_fainted_battle_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<()> {
    match battle.commands.switch_party_indices.as_slice() {
        [] => {
            reset_visible_battle_action_cursors(runtime_shell);
            if battle.battle_type == "BATTLETYPE_CANLOSE" {
                let RuntimeBattleKind::Trainer {
                    source_script,
                    loss_text,
                    ..
                } = &battle.kind
                else {
                    anyhow::bail!("BATTLETYPE_CANLOSE is only valid for a trainer battle");
                };
                let map_name = snapshot.overworld.map_name.clone();
                queue_visible_trainer_result_text(runtime_shell, snapshot, loss_text)?;
                reset_visible_battle_exit_state(runtime_shell);
                complete_visible_scripted_trainer_battle(
                    runtime_shell,
                    &map_name,
                    source_script,
                    false,
                    true,
                )
            } else {
                resolve_visible_blackout(runtime_shell)
            }
        }
        _ => {
            if !matches!(&battle.kind, RuntimeBattleKind::Trainer { .. }) {
                runtime_shell.battle_action_cursor = None;
                runtime_shell.battle_switch_cursor = None;
                visible_cursor_index(
                    &mut runtime_shell.battle_faint_prompt_cursor,
                    "battle:faint-prompt",
                    2,
                );
                set_shell_action_status(runtime_shell, "USE NEXT POKEMON?");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            let actions = visible_battle_action_ids(snapshot, battle);
            let pokemon_action_index = actions
                .iter()
                .position(|action| *action == VisibleBattleAction::Pokemon)
                .context("fainted active Pokemon has switch targets but no Pokemon action")?;
            runtime_shell.battle_action_cursor = Some(MenuCursor {
                surface_id: "battle:actions".to_string(),
                option_index: pokemon_action_index,
            });
            visible_cursor_index(
                &mut runtime_shell.battle_switch_cursor,
                "battle:switch",
                battle_switch_option_count(snapshot),
            );
            runtime_shell.last_audio_events.push(format!(
                "active Pokemon fainted; awaiting replacement choices={}",
                battle.commands.switch_party_indices.len()
            ));
            set_shell_action_status(
                runtime_shell,
                format!(
                    "CHOOSE REPLACEMENT {}",
                    battle.commands.switch_party_indices.len()
                ),
            );
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
    }
}

fn switch_visible_battle_party_without_turn(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("battle:replacement:{party_index}"))?;
    let switched = runtime_shell
        .shell
        .switch_active_battle_party(party_index)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle replacement party_index={} checksum={:?}",
        switched.party_index, switched.state_checksum
    ));
    let replacement = runtime_shell.shell.snapshot()?;
    let send_out_message = visible_player_send_out_message(&replacement, switched.party_index)?;
    runtime_shell.battle_messages.push_back(send_out_message.clone());
    runtime_shell.battle_player_send_out_pending = true;
    runtime_shell.battle_message_scene = Some(Box::new(replacement));
    mark_runtime_snapshot_dirty(runtime_shell);
    let species_id = runtime_shell
        .shell
        .snapshot()?
        .party
        .slots
        .iter()
        .find(|slot| slot.index == switched.party_index)
        .map(|slot| slot.pokemon.species.id.clone())
        .context("battle replacement is missing its selected party species")?;
    defer_visible_battle_cry_after_message(
        runtime_shell,
        species_id,
        "battle_replacement",
        send_out_message,
    );
    runtime_shell.battle_enemy_hp_at_player_send_out = runtime_shell
        .shell
        .snapshot()?
        .battle
        .as_ref()
        .map(|battle| battle.enemy_pokemon.hp);
    set_shell_action_status(
        runtime_shell,
        format!("SENT PARTY #{}", switched.party_index),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    settle_visible_battle_after_action(runtime_shell)?;
    Ok(())
}

fn switch_visible_trainer_shift_party_without_turn(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let enemy_send_out_scene = runtime_shell
        .battle_message_scene
        .clone()
        .context("trainer Shift switch is missing the staged enemy send-out scene")?;
    let enemy_send_out_message = runtime_shell
        .battle_messages
        .back()
        .cloned()
        .context("trainer Shift switch is missing the enemy send-out message")?;
    record_visible_runtime_action(runtime_shell, format!("battle:shift_switch:{party_index}"))?;
    let switched = runtime_shell
        .shell
        .switch_active_battle_party(party_index)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "trainer shift switch party_index={} checksum={:?}",
        switched.party_index, switched.state_checksum
    ));
    let replacement = runtime_shell.shell.snapshot()?;
    let outgoing_index = enemy_send_out_scene
        .battle
        .as_ref()
        .and_then(|battle| battle.active_player_party_index)
        .context("trainer Shift switch is missing the outgoing active party index")?;
    let outgoing_nickname = enemy_send_out_scene
        .party
        .slots
        .iter()
        .find(|slot| slot.index == outgoing_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .context("trainer Shift switch is missing the outgoing party member")?;
    let withdraw_message =
        visible_player_withdraw_message(runtime_shell, &enemy_send_out_scene, outgoing_nickname);
    let send_out_message = visible_player_send_out_message(&replacement, switched.party_index)?;
    runtime_shell
        .battle_messages
        .push_back(withdraw_message.clone());
    runtime_shell.battle_messages.push_back(send_out_message.clone());
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_message_scenes.push_back(enemy_send_out_scene.clone());
    runtime_shell.battle_message_scenes.push_back(enemy_send_out_scene.clone());
    runtime_shell
        .battle_message_scenes
        .push_back(Box::new(replacement.clone()));
    queue_visible_player_recall_animation(
        runtime_shell,
        &enemy_send_out_scene,
        &withdraw_message,
    );
    runtime_shell.battle_message_scene = Some(enemy_send_out_scene);
    mark_runtime_snapshot_dirty(runtime_shell);
    let species_id = runtime_shell
        .shell
        .snapshot()?
        .party
        .slots
        .iter()
        .find(|slot| slot.index == switched.party_index)
        .map(|slot| slot.pokemon.species.id.clone())
        .context("trainer Shift switch is missing its selected party species")?;
    defer_visible_battle_cry_after_message(
        runtime_shell,
        species_id,
        "trainer_shift_switch",
        send_out_message,
    );
    debug_assert!(visible_message_is_enemy_send_out(&enemy_send_out_message));
    runtime_shell.battle_enemy_hp_at_player_send_out = runtime_shell
        .shell
        .snapshot()?
        .battle
        .as_ref()
        .map(|battle| battle.enemy_pokemon.hp);
    set_shell_action_status(
        runtime_shell,
        format!("SHIFT SENT PARTY #{}", switched.party_index),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn throw_visible_battle_ball(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "ball_throw");
    }
    if carried_ball_item_ids(&snapshot).is_empty() {
        runtime_shell.ball_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:ball:throw:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no carried ball".to_string());
        runtime_shell
            .battle_messages
            .push_back("You don't have any BALLS.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "NO BALLS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let (ball_index, ball_id) = selected_battle_ball_id(runtime_shell)?;
    throw_visible_battle_ball_id(runtime_shell, ball_index, ball_id)
}

fn throw_visible_battle_ball_at(
    runtime_shell: &mut BevyRuntimeShell,
    ball_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "ball_throw_at");
    };
    if !battle.commands.can_use_items {
        record_visible_runtime_action(runtime_shell, "battle:ball:items_unavailable")?;
        runtime_shell
            .last_audio_events
            .push("active battle does not allow item use".to_string());
        runtime_shell
            .battle_messages
            .push_back("Items can't be used here.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "ITEMS UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let ball_ids = carried_ball_item_ids(&snapshot);
    let Some(ball_id) = ball_ids.get(ball_index).cloned() else {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:ball:index:{}:unavailable", ball_index + 1),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("bag has no ball at index {}", ball_index + 1));
        set_shell_action_status(runtime_shell, "BALL UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    throw_visible_battle_ball_id(runtime_shell, ball_index, ball_id)
}

fn throw_visible_battle_ball_id(
    runtime_shell: &mut BevyRuntimeShell,
    ball_index: usize,
    ball_id: String,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "ball_throw_id");
    }
    let scripted_static_wild = snapshot
        .battle
        .as_ref()
        .and_then(|battle| visible_static_wild_source(&snapshot, battle));
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:ball:{ball_id}:index:{}", ball_index + 1),
    )?;
    let capture = runtime_shell.shell.throw_ball_at_active_battle(&ball_id)?;
    if capture.outcome.as_ref().is_some_and(|outcome| outcome.storage_full) {
        runtime_shell
            .battle_messages
            .push_back("The <PKMN> BOX\nis full. That\ncan't be used now.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot));
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:capture_storage_full:{ball_id}:index:{}", ball_index + 1),
        )?;
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "BOX FULL");
        return Ok(());
    }
    record_visible_battle_item_action_frame(runtime_shell, &ball_id)?;
    let use_message = format!(
        "{} used the {}.",
        visible_battle_player_name(&snapshot),
        item_display_name(&snapshot, &ball_id)
    );
    runtime_shell.battle_messages.push_back(use_message.clone());
    runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
    mark_runtime_snapshot_dirty(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "threw ball_index={} ball={} outcome={:?} checksum={:?}",
        ball_index + 1,
        ball_id,
        capture.outcome,
        capture.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        visible_capture_attempt_status(&ball_id, capture.outcome.as_ref()),
    );
    reset_visible_battle_action_cursors(runtime_shell);
    if let Some(outcome) = capture.outcome.as_ref() {
        runtime_shell.visible_capture_animation = Some(VisibleCaptureAnimation {
            trigger_message: use_message,
            ball_id: ball_id.clone(),
            animation_shakes: outcome.animation_shakes,
            blocked: outcome.blocked,
            caught: outcome.caught,
            started: false,
            complete: false,
            sprites_cleared: false,
            frame: 0,
        });
        if outcome.blocked {
            runtime_shell
                .battle_messages
                .push_back("The trainer\nblocked the BALL!".to_string());
            runtime_shell
                .battle_messages
                .push_back("Don't be a thief!".to_string());
            runtime_shell.pending_enemy_response_after_capture =
                Some((ball_id.clone(), "Don't be a thief!".to_string()));
            record_visible_runtime_action(
                runtime_shell,
                format!("battle:capture_blocked:{ball_id}:index:{}", ball_index + 1),
            )?;
        } else if outcome.caught {
            let enemy_name = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.enemy_pokemon.nickname.as_str())
                .context("caught outcome is missing its active enemy")?;
            let enemy_species = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.enemy_pokemon.species.id.as_str())
                .context("caught outcome is missing its active enemy species")?;
            runtime_shell
                .battle_messages
                .push_back(format!("Gotcha! {enemy_name}\nwas caught!"));
            let battle_type = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.battle_type.as_str())
                .unwrap_or_default();
            if battle_type != "BATTLETYPE_TUTORIAL"
                && snapshot
                    .progression
                    .active_engine_flags
                    .contains("ENGINE_POKEDEX")
                && !snapshot
                    .progression
                    .pokedex_caught_species
                    .contains(enemy_species)
            {
                runtime_shell.battle_messages.push_back(format!(
                    "{enemy_name}'s data\nwas newly added to\nthe POKéDEX."
                ));
            }
            runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
            record_visible_runtime_action(
                runtime_shell,
                format!("battle:capture_complete:{ball_id}:index:{}", ball_index + 1),
            )?;
            if matches!(
                battle_type,
                "BATTLETYPE_TUTORIAL" | "BATTLETYPE_CONTEST"
            ) {
                complete_visible_standard_capture(
                    runtime_shell,
                    outcome.clone(),
                    None,
                    scripted_static_wild,
                )?;
            } else {
                runtime_shell.pending_standard_capture = Some(PendingStandardCapture {
                    outcome: outcome.clone(),
                    scripted_static_wild,
                    default_name: canonical_species_display_name(enemy_species),
                });
            }
        } else {
            let text = match outcome.wobble_count.min(3) {
                0 => "Oh no! The <PKMN>\nbroke free!".to_string(),
                1 => "Aww! It appeared\nto be caught!".to_string(),
                2 => "Aargh!\nAlmost had it!".to_string(),
                _ => "Shoot! It was so\nclose too!".to_string(),
            };
            runtime_shell.battle_messages.push_back(text.clone());
            runtime_shell.pending_enemy_response_after_capture = Some((ball_id.clone(), text));
        }
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn visible_capture_attempt_status(
    ball_id: &str,
    outcome: Option<&crate::core::battle::capture::CaptureOutcome>,
) -> String {
    let Some(outcome) = outcome else {
        return format!("THREW {ball_id}");
    };
    if outcome.blocked {
        if outcome.storage_full {
            return format!("{ball_id} STORAGE FULL");
        }
        return format!("{ball_id} BLOCKED");
    }
    if outcome.caught {
        return format!("{ball_id} CAUGHT");
    }
    format!("{ball_id} SHAKES {}", outcome.animation_shakes)
}

fn visible_capture_completion_status(completion: &crate::RuntimeCaptureCompletion) -> String {
    let Some(stored) = completion.stored.as_ref() else {
        return "CAPTURE COMPLETE".to_string();
    };
    let location = match stored.location {
        crate::core::models::CaptureStorageLocation::Party { slot } => {
            format!("PARTY #{}", slot + 1)
        }
        crate::core::models::CaptureStorageLocation::Pc { box_index, slot } => {
            format!("BOX {} #{}", box_index + 1, slot + 1)
        }
    };
    compact_scene_label(
        &format!("CAUGHT {} -> {location}", stored.pokemon.species.id),
        76,
    )
}

fn complete_visible_standard_capture(
    runtime_shell: &mut BevyRuntimeShell,
    outcome: crate::core::battle::capture::CaptureOutcome,
    nickname: Option<String>,
    scripted_static_wild: Option<VisibleStaticWildOrigin>,
) -> Result<()> {
    let battle_before_completion = runtime_shell.shell.snapshot()?;
    let completion = runtime_shell
        .shell
        .complete_active_wild_capture(&outcome, nickname)?;
    if let Some(stored) = completion.stored.as_ref() {
        if matches!(
            stored.location,
            crate::core::models::CaptureStorageLocation::Pc { .. }
        ) {
            runtime_shell.battle_messages.push_back(format!(
                "{} was\nsent to BILL's PC.",
                stored.pokemon.nickname
            ));
            runtime_shell.battle_message_scene = Some(Box::new(battle_before_completion.clone()));
        }
    }
    runtime_shell.last_audio_events.push(format!(
        "capture complete stored={:?} checksum={:?}",
        completion.stored, completion.state_checksum
    ));
    if let Some(caught_species) = completion
        .stored
        .as_ref()
        .map(|stored| stored.pokemon.species.id.clone())
        .or_else(|| {
            completion
                .contest_pokemon
                .as_ref()
                .map(|pokemon| pokemon.species.id.clone())
        })
    {
        queue_visible_pokemon_cry(runtime_shell, &caught_species, "battle_capture_complete")?;
    }
    set_shell_action_status(
        runtime_shell,
        visible_capture_completion_status(&completion),
    );
    queue_visible_pay_day_payout(runtime_shell, &battle_before_completion);
    finish_visible_wild_battle_exit(runtime_shell, scripted_static_wild, "battle_capture")
}

fn queue_visible_pay_day_payout(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) {
    let Some(battle) = snapshot.battle.as_ref() else {
        return;
    };
    queue_visible_pay_day_payout_for_battle(
        runtime_shell,
        battle,
        &snapshot.trainer.player_name,
    );
}

fn queue_visible_pay_day_payout_for_battle(
    runtime_shell: &mut BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    player_name: &str,
) {
    let mut payout = battle.pay_day_money.min(0x00ff_ffff);
    if battle.amulet_coin_active {
        payout = payout.saturating_mul(2).min(0x00ff_ffff);
    }
    if payout > 0 {
        runtime_shell
            .battle_messages
            .push_back(format!("{player_name} picked up\n¥{payout}!"));
    }
}

fn claim_visible_battle_rewards(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.clone() else {
        return handle_visible_no_active_battle(runtime_shell, "claim_rewards");
    };
    let reward_recipient_index = battle
        .active_player_party_index
        .context("battle rewards are missing the active player party index")?;
    let reward_recipient_name = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == reward_recipient_index)
        .map(|slot| slot.pokemon.nickname.clone())
        .context("battle rewards are missing the active player Pokemon")?;
    let map_name = snapshot.overworld.map_name.clone();
    let scripted_static_origin = visible_static_wild_source(&snapshot, &battle);
    let plain_wild_battle = matches!(battle.kind, crate::RuntimeBattleKind::Wild { .. });
    let trainer_battle = matches!(battle.kind, crate::RuntimeBattleKind::Trainer { .. });
    let reward_action = match &battle.kind {
        crate::RuntimeBattleKind::Wild { map_name, .. } => {
            format!("battle:claim_rewards:wild:{map_name}")
        }
        crate::RuntimeBattleKind::StaticWild {
            source_script,
            species,
            level,
            ..
        } => format!("battle:claim_rewards:static_wild:{source_script}:{species}:{level}"),
        crate::RuntimeBattleKind::Trainer {
            source_script,
            trainer_class,
            trainer_id,
            ..
        } => format!("battle:claim_rewards:trainer:{source_script}:{trainer_class}:{trainer_id}"),
    };
    record_visible_runtime_action(runtime_shell, reward_action)?;
    let message = match battle.kind {
        crate::RuntimeBattleKind::Wild { .. } => {
            let rewards = runtime_shell.shell.claim_active_wild_battle_rewards()?;
            stage_visible_battle_exp_tween(runtime_shell, &snapshot, &rewards.outcome)?;
            stage_visible_battle_level_stats(runtime_shell, &snapshot, &rewards.outcome)?;
            set_shell_action_status(
                runtime_shell,
                visible_battle_reward_status(&rewards.outcome),
            );
            push_visible_battle_reward_events(
                runtime_shell,
                &rewards.outcome,
                reward_recipient_index,
                &reward_recipient_name,
            )?;
            queue_visible_pay_day_payout(runtime_shell, &snapshot);
            retain_visible_pre_reward_battle_scene(runtime_shell, &snapshot);
            format!(
                "claimed wild rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            )
        }
        crate::RuntimeBattleKind::StaticWild { .. } => {
            let rewards = runtime_shell.shell.claim_active_wild_battle_rewards()?;
            stage_visible_battle_exp_tween(runtime_shell, &snapshot, &rewards.outcome)?;
            stage_visible_battle_level_stats(runtime_shell, &snapshot, &rewards.outcome)?;
            set_shell_action_status(
                runtime_shell,
                visible_battle_reward_status(&rewards.outcome),
            );
            push_visible_battle_reward_events(
                runtime_shell,
                &rewards.outcome,
                reward_recipient_index,
                &reward_recipient_name,
            )?;
            queue_visible_pay_day_payout(runtime_shell, &snapshot);
            retain_visible_pre_reward_battle_scene(runtime_shell, &snapshot);
            let message = format!(
                "claimed wild rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            );
            runtime_shell.last_audio_events.push(message);
            finish_visible_wild_battle_exit(
                runtime_shell,
                Some(scripted_static_origin.context(
                    "static wild reward completion lost its captured origin",
                )?),
                "wild_battle_victory",
            )?;
            return Ok(());
        }
        crate::RuntimeBattleKind::Trainer { trainer_name, .. } => {
            let rewards = runtime_shell.shell.claim_active_trainer_battle_rewards()?;
            stage_visible_battle_exp_tween(runtime_shell, &snapshot, &rewards.outcome)?;
            stage_visible_battle_level_stats(runtime_shell, &snapshot, &rewards.outcome)?;
            set_shell_action_status(
                runtime_shell,
                visible_battle_reward_status(&rewards.outcome),
            );
            push_visible_battle_reward_events(
                runtime_shell,
                &rewards.outcome,
                reward_recipient_index,
                &reward_recipient_name,
            )?;
            retain_visible_pre_reward_battle_scene(runtime_shell, &snapshot);
            let message = format!(
                "claimed trainer rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            );
            let latest = runtime_shell.shell.snapshot()?;
            if let Some(battle) = latest.battle.as_ref() {
                if should_offer_trainer_shift_switch(&latest, battle) {
                    runtime_shell.battle_action_cursor = None;
                    runtime_shell.battle_move_cursor = None;
                    runtime_shell.battle_move_swap_origin = None;
                    reset_visible_battle_item_cursors(runtime_shell);
                    visible_cursor_index(
                        &mut runtime_shell.battle_shift_prompt_cursor,
                        "battle:shift-prompt",
                        2,
                    );
                    runtime_shell.battle_switch_cursor = None;
                    let next_enemy = next_unresolved_trainer_enemy_label(battle)
                        .context("trainer Shift prompt requires the next enemy party member")?;
                    runtime_shell
                        .battle_messages
                        .push_back(format!("{trainer_name}\nis about to use\n{next_enemy}."));
                    runtime_shell.last_audio_events.push(format!(
                        "trainer shift switch prompt choices={}",
                        battle.commands.switch_party_indices.len()
                    ));
                    set_shell_action_status(runtime_shell, "SHIFT: SWITCH POKEMON?");
                }
            }
            message
        }
    };
    runtime_shell.last_audio_events.push(message);
    trim_event_log(&mut runtime_shell.last_audio_events);
    if trainer_battle {
        if runtime_shell.battle_shift_prompt_cursor.is_none()
            && runtime_shell.battle_switch_cursor.is_none()
        {
            reset_visible_battle_action_cursors(runtime_shell);
        }
        return Ok(());
    }
    if plain_wild_battle {
        finish_visible_wild_battle_exit(runtime_shell, None, "wild_battle_victory")?;
    } else {
        reset_visible_battle_exit_state(runtime_shell);
        queue_visible_current_music(runtime_shell)?;
    }
    Ok(())
}

fn retain_visible_pre_reward_battle_scene(
    runtime_shell: &mut BevyRuntimeShell,
    battle_before_rewards: &RuntimeShellSnapshot,
) {
    // Reward authority commits EXP, levels, moves, and evolution atomically,
    // but Crystal does not reveal the resulting battler before narrating it.
    // Keep the faint/reward boundary frame until every queued message clears.
    runtime_shell.battle_message_scene = Some(Box::new(battle_before_rewards.clone()));
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn visible_battle_reward_status(
    outcome: &crate::core::systems::battle_rewards::BattleRewardOutcome,
) -> String {
    let mut parts = vec![format!(
        "{} EXP {}",
        outcome.defeated_species, outcome.experience_awarded
    )];
    if outcome.level_after > outcome.level_before {
        parts.push(format!("LEVEL {}", outcome.level_after));
    }
    if !outcome.learned_moves.is_empty() {
        parts.push(format!("LEARNED {}", outcome.learned_moves.join(",")));
    }
    if !outcome.pending_move_learns.is_empty() {
        let pending = outcome
            .pending_move_learns
            .iter()
            .map(|learned| learned.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        parts.push(format!("WANTS {pending}"));
    }
    if let Some(target_species) = outcome.evolution.target_species.as_ref() {
        parts.push(format!("EVOLVED {target_species}"));
    }
    compact_scene_label(&parts.join(" / "), 76)
}

fn visible_battle_exp_pixels(
    runtime_shell: &BevyRuntimeShell,
    pokemon: &crate::core::models::pokemon::Pokemon,
) -> Result<u16> {
    if pokemon.level >= 100 {
        return Ok(0);
    }
    let level = pokemon.level.clamp(1, 99);
    let current = crate::core::systems::experience::calculate_experience(
        runtime_shell.shell.runtime().growth_rates(),
        &pokemon.species.growth_rate,
        level,
    )?;
    let next = crate::core::systems::experience::calculate_experience(
        runtime_shell.shell.runtime().growth_rates(),
        &pokemon.species.growth_rate,
        level + 1,
    )?;
    let span = (next - current).max(1);
    let capped_exp = pokemon.experience.clamp(current, next);
    let remaining = next - capped_exp;
    Ok((64 - ((remaining * 64) / span).clamp(0, 64)) as u16)
}

fn stage_visible_battle_exp_tween(
    runtime_shell: &mut BevyRuntimeShell,
    before: &RuntimeShellSnapshot,
    outcome: &crate::core::systems::battle_rewards::BattleRewardOutcome,
) -> Result<()> {
    let Some(active_index) = before
        .battle
        .as_ref()
        .and_then(|battle| battle.active_player_party_index)
    else {
        return Ok(());
    };
    let Some(before_mon) = before.party.slots.iter().find(|slot| slot.index == active_index).map(|slot| &slot.pokemon) else {
        return Ok(());
    };
    let after = runtime_shell.shell.snapshot()?;
    let Some(after_mon) = after.party.slots.iter().find(|slot| slot.index == active_index).map(|slot| &slot.pokemon) else {
        return Ok(());
    };
    let awards = if outcome.recipient_outcomes.is_empty() {
        vec![(
            outcome.experience_awarded,
            outcome.level_before,
            outcome.level_after,
            before_mon.nickname.clone(),
        )]
    } else {
        outcome
            .recipient_outcomes
            .iter()
            .filter(|recipient| recipient.party_index == active_index)
            .map(|recipient| (
                recipient.experience_awarded,
                recipient.level_before,
                recipient.level_after,
                recipient.nickname.clone(),
            ))
            .collect::<Vec<_>>()
    };
    let mut staged = VecDeque::new();
    let mut rolling_exp = before_mon.experience;
    for (experience_awarded, level_before, level_after, nickname) in awards {
        if experience_awarded <= 0 {
            continue;
        }
        let mut segment_before = before_mon.clone();
        segment_before.level = level_before;
        segment_before.experience = rolling_exp;
        rolling_exp = rolling_exp
            .saturating_add(experience_awarded)
            .min(after_mon.experience);
        let mut segment_after = after_mon.clone();
        segment_after.level = level_after;
        segment_after.experience = rolling_exp;
        let mut targets = VecDeque::new();
        for _ in level_before..level_after {
            targets.push_back(64);
        }
        targets.push_back(visible_battle_exp_pixels(runtime_shell, &segment_after)?);
        let target_pixels = targets
            .pop_front()
            .context("staged EXP animation has no initial bar target")?;
        staged.push_back(VisibleBattleExpTween {
            trigger_message: format!(
                "{nickname} gained\n{experience_awarded} EXP. Points!"
            ),
            started: false,
            pixels: visible_battle_exp_pixels(runtime_shell, &segment_before)?,
            level: level_before,
            target_pixels,
            remaining_targets: targets,
            frames_until_step: 0,
            steps_in_segment: 0,
        });
    }
    runtime_shell.battle_exp_tween = staged.pop_front();
    runtime_shell.pending_battle_exp_tweens = staged;
    Ok(())
}

fn stage_visible_battle_level_stats(
    runtime_shell: &mut BevyRuntimeShell,
    before: &RuntimeShellSnapshot,
    outcome: &crate::core::systems::battle_rewards::BattleRewardOutcome,
) -> Result<()> {
    let after = runtime_shell.shell.snapshot()?;
    let recipients = if outcome.recipient_outcomes.is_empty() {
        before
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|party_index| {
                before
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == party_index)
                    .map(|slot| vec![(party_index, outcome.level_before, outcome.level_after, slot.pokemon.nickname.clone())])
            })
            .unwrap_or_default()
    } else {
        outcome
            .recipient_outcomes
            .iter()
            .map(|recipient| (
                recipient.party_index,
                recipient.level_before,
                recipient.level_after,
                recipient.nickname.clone(),
            ))
            .collect::<Vec<_>>()
    };
    for (party_index, level_before, level_after, nickname) in recipients {
        if level_after <= level_before {
            continue;
        }
        let Some(pokemon) = after
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .map(|slot| &slot.pokemon)
        else {
            continue;
        };
        runtime_shell.battle_level_stats.push_back(VisibleBattleLevelStats {
            trigger_message: format!("{nickname} grew to\nlevel {level_after}!"),
            triggered: false,
            active: false,
            frames_before_input: 0,
            attack: pokemon.attack,
            defense: pokemon.defense,
            speed: pokemon.speed,
            special_attack: pokemon.special_attack,
            special_defense: pokemon.special_defense,
        });
    }
    Ok(())
}

fn push_visible_battle_reward_events(
    runtime_shell: &mut BevyRuntimeShell,
    outcome: &crate::core::systems::battle_rewards::BattleRewardOutcome,
    party_index: usize,
    recipient_name: &str,
) -> Result<()> {
    if !outcome.recipient_outcomes.is_empty() {
        for recipient in &outcome.recipient_outcomes {
            let projected = crate::core::systems::battle_rewards::BattleRewardOutcome {
                defeated_species: outcome.defeated_species.clone(),
                experience_awarded: recipient.experience_awarded,
                level_before: recipient.level_before,
                level_after: recipient.level_after,
                learned_moves: recipient.learned_moves.clone(),
                pending_move_learns: recipient.pending_move_learns.clone(),
                deferred_level_evolution: false,
                evolution: recipient.evolution.clone(),
                recipient_outcomes: Vec::new(),
            };
            push_visible_battle_reward_events(
                runtime_shell,
                &projected,
                recipient.party_index,
                &recipient.nickname,
            )?;
        }
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    if outcome.experience_awarded > 0 {
        runtime_shell.battle_messages.push_back(format!(
            "{} gained\n{} EXP. Points!",
            recipient_name, outcome.experience_awarded
        ));
    }
    runtime_shell.last_audio_events.push(format!(
        "battle reward exp defeated={} exp={}",
        outcome.defeated_species, outcome.experience_awarded
    ));
    if outcome.level_after > outcome.level_before {
        for level in outcome.level_before.saturating_add(1)..=outcome.level_after {
            let message = format!(
                "{} grew to\nlevel {}!",
                recipient_name, level
            );
            runtime_shell.battle_fanfare_messages.push_back(message.clone());
            runtime_shell.battle_messages.push_back(message);
        }
        runtime_shell.last_audio_events.push(format!(
            "battle reward level {}->{}",
            outcome.level_before, outcome.level_after
        ));
    }
    for move_id in &outcome.learned_moves {
        let move_name = battle_move_display_name(&snapshot, move_id);
        let message = format!("{} learned\n{}!", recipient_name, move_name);
        runtime_shell.battle_fanfare_messages.push_back(message.clone());
        runtime_shell.battle_messages.push_back(message);
        runtime_shell
            .last_audio_events
            .push(format!("battle reward learned move {move_id}"));
    }
    for learned in outcome.pending_move_learns.iter().filter(|learned| {
        !outcome
            .evolution
            .pending_move_learns
            .iter()
            .any(|evolution_move| evolution_move.name == learned.name)
    }) {
        let move_name = battle_move_display_name(&snapshot, &learned.name);
        runtime_shell.battle_messages.push_back(format!(
            "{} is\ntrying to learn\n{}.",
            recipient_name, move_name
        ));
        runtime_shell
            .last_audio_events
            .push(format!("battle reward pending move learn {}", learned.name));
    }
    if let Some(target_species) = outcome.evolution.target_species.as_ref() {
        let species_name = crate::core::models::pokemon_species_display_name(target_species);
        let evolving_message = format!("What? {} is evolving!", recipient_name);
        let evolved_message = format!(
            "Congratulations! {} evolved into {}!",
            recipient_name, species_name
        );
        runtime_shell.battle_messages.push_back(evolving_message.clone());
        runtime_shell.battle_messages.push_back(evolved_message.clone());
        let pending_move_messages = outcome
            .evolution
            .pending_move_learns
            .iter()
            .map(|learned| {
                format!(
                    "{} is\ntrying to learn\n{}.",
                    recipient_name,
                    battle_move_display_name(&snapshot, &learned.name)
                )
            })
            .collect::<Vec<_>>();
        runtime_shell
            .battle_messages
            .extend(pending_move_messages.iter().cloned());
        if outcome.evolution.cancel_snapshot.is_some() {
            runtime_shell.battle_evolution_cancellations.push_back(
                VisibleEvolutionCancellation {
                    party_index,
                    trigger_message: evolving_message.clone(),
                    evolved_message: evolved_message.clone(),
                    pending_move_messages,
                    report: outcome.evolution.clone(),
                },
            );
        }
        runtime_shell
            .battle_evolution_cries
            .push_back((target_species.clone(), evolving_message.clone()));
        runtime_shell
            .battle_sounds_after_messages
            .push_back(("SFX_CAUGHT_MON".to_string(), evolving_message.clone()));
        runtime_shell
            .last_audio_events
            .push(format!("battle reward evolved {target_species}"));
    }
    for event in &outcome.evolution.events {
        let label = match event {
            crate::core::systems::evolution::EvolutionEvent::Text(text) => {
                format!("battle reward evolution text {text}")
            }
            crate::core::systems::evolution::EvolutionEvent::ItemConsumed(item_id) => {
                format!("battle reward evolution consumed {item_id}")
            }
            crate::core::systems::evolution::EvolutionEvent::MoveLearned(move_id) => {
                format!("battle reward evolution learned move {move_id}")
            }
        };
        runtime_shell.last_audio_events.push(label);
    }
    Ok(())
}

fn advance_visible_trainer_battle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.clone() else {
        return handle_visible_no_active_battle(runtime_shell, "advance_trainer");
    };
    let crate::RuntimeBattleKind::Trainer {
        source_script,
        trainer_name,
        win_text,
        ..
    } = battle.kind
    else {
        anyhow::bail!("active battle is not a trainer battle");
    };
    let map_name = snapshot.overworld.map_name.clone();
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:advance_trainer:{map_name}:{source_script}"),
    )?;
    let advance = runtime_shell.shell.advance_active_trainer_battle()?;
    runtime_shell.last_audio_events.push(format!(
        "advanced trainer battle defeated={} next_enemy={:?} checksum={:?}",
        advance.trainer_defeated, advance.next_enemy, advance.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "TRAINER BATTLE defeated={} next={:?}",
            advance.trainer_defeated, advance.next_enemy
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    if advance.trainer_defeated {
        queue_visible_trainer_result_text(runtime_shell, &snapshot, &win_text)?;
        reset_visible_battle_exit_state(runtime_shell);
        complete_visible_scripted_trainer_battle(
            runtime_shell,
            &map_name,
            &source_script,
            true,
            false,
        )?;
    } else {
        reset_visible_battle_action_cursors(runtime_shell);
        let replacement = runtime_shell.shell.snapshot()?;
        let replacement_battle = replacement
            .battle
            .as_ref()
            .context("trainer replacement advance removed the active battle")?;
        runtime_shell.battle_enemy_hp_at_player_send_out =
            Some(replacement_battle.enemy_pokemon.hp);
        let send_out_message = format!(
            "{}\nsent out\n{}!",
            trainer_name, replacement_battle.enemy_pokemon.nickname
        );
        runtime_shell.battle_messages.push_back(send_out_message.clone());
        runtime_shell.battle_enemy_send_out_pending = true;
        defer_visible_battle_cry_after_message(
            runtime_shell,
            replacement_battle.enemy_pokemon.species.id.clone(),
            "trainer_replacement",
            send_out_message,
        );
        runtime_shell.battle_message_scene = Some(Box::new(replacement));
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn queue_visible_trainer_result_text(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    text_label: &str,
) -> Result<()> {
    if runtime_shell.battle_message_scene.is_none() {
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
    }
    let text = runtime_shell.shell.text_snapshot(text_label)?;
    let pages = if let Some(asm_text) = text.asm_text.as_deref() {
        vec![normalize_visible_script_text_with_context(
            asm_text,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )]
    } else if let Some(body) = text.body.as_ref() {
        render_visible_script_text_pages(
            body,
            &snapshot.script_events.named_buffers,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )
    } else {
        anyhow::bail!("trainer result text '{text_label}' has no renderable body");
    };
    runtime_shell
        .battle_messages
        .extend(pages.into_iter().filter(|page| !page.trim().is_empty()));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_shop_top_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.shop_top_cursor,
        "shop:top",
        3,
    )
    .context("shop top menu requires a valid cursor")?;
    match selected {
        0 => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let shop = snapshot.pending_shop.as_ref().context("shop top menu has no shop")?;
            runtime_shell.shop_top_cursor = None;
            runtime_shell.sell_cursor = None;
            visible_cursor_index(
                &mut runtime_shell.menu_cursor,
                &shop_cursor_surface_id(shop),
                shop.inventory.len(),
            );
            Ok(())
        }
        1 => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let sellable = sellable_carried_item_ids(&snapshot);
            if sellable.is_empty() {
                let notice = "You don't have anything to sell.".to_string();
                set_shell_action_status(runtime_shell, notice.clone());
                runtime_shell.shop_notice = Some(notice);
                runtime_shell.shop_return_to_top_after_notice = true;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            runtime_shell.shop_top_cursor = None;
            visible_cursor_index(&mut runtime_shell.sell_cursor, "sell:bag", sellable.len());
            Ok(())
        }
        _ => close_visible_shop(runtime_shell),
    }
}

fn buy_visible_shop_cursor_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(shop) = snapshot.pending_shop else {
        return handle_visible_no_active_shop(runtime_shell, "buy_confirm");
    };
    if shop.inventory.is_empty() {
        anyhow::bail!("shop {} has no compiled inventory", shop.mart_id);
    }
    let surface_id = shop_cursor_surface_id(&shop);
    let selected_index = visible_cursor_index(
        &mut runtime_shell.menu_cursor,
        &surface_id,
        shop.inventory.len(),
    );
    begin_visible_shop_quantity(runtime_shell, &shop, selected_index, false)
}

fn begin_visible_shop_quantity(
    runtime_shell: &mut BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    selected_index: usize,
    selling: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = if selling {
        sellable_carried_item_ids(&snapshot).get(selected_index).cloned()
    } else {
        shop.inventory.get(selected_index).cloned()
    }
    .context("shop quantity selection has no item")?;
    let item = snapshot.items.iter().find(|item| item.item_id == item_id)
        .with_context(|| format!("shop quantity item {item_id} is missing from catalog"))?;
    let unit_price = if selling { item.price / 2 } else { item.price };
    let max_quantity = if selling {
        carried_item_quantity(&snapshot, &item_id).unwrap_or(0).min(99)
    } else if unit_price == 0 {
        0
    } else {
        let owned = carried_item_quantity(&snapshot, &item_id).unwrap_or(0).min(99);
        let stack_limit: u16 = match item.pocket.as_str() {
            "KEY_ITEM" | "TM_HM" => 1,
            _ => 99,
        };
        let pocket_has_room = owned > 0
            || match item.pocket.as_str() {
                "ITEM" => snapshot.bag.items.iter().filter(|entry| entry.quantity > 0).count() < 20,
                "BALL" => snapshot.bag.balls.iter().filter(|entry| entry.quantity > 0).count() < 12,
                "KEY_ITEM" => snapshot.bag.key_items.iter().filter(|entry| entry.quantity > 0).count() < 25,
                "TM_HM" => true,
                _ => true,
            };
        let capacity = if pocket_has_room {
            stack_limit.saturating_sub(owned)
        } else {
            0
        };
        capacity
            .min((snapshot.trainer.money / u32::from(unit_price)).min(99) as u16)
    };
    if max_quantity == 0 {
        let notice = if selling { "You don't have any left." } else { "You can't buy any." };
        set_shell_action_status(runtime_shell, notice);
        runtime_shell.shop_notice = Some(notice.to_string());
        runtime_shell.shop_return_to_top_after_notice = true;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    runtime_shell.shop_quantity = Some(VisibleShopQuantity {
        item_id,
        selling,
        quantity: 1,
        max_quantity,
        unit_price,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn adjust_visible_shop_quantity(runtime_shell: &mut BevyRuntimeShell, delta: i16) -> Result<()> {
    let Some(quantity) = runtime_shell.shop_quantity.as_mut() else {
        return Ok(());
    };
    quantity.quantity = (i32::from(quantity.quantity) + i32::from(delta))
        .clamp(1, i32::from(quantity.max_quantity)) as u16;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_shop_quantity(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let quantity = runtime_shell.shop_quantity.take().context("shop quantity prompt is not open")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let shop = snapshot
        .pending_shop
        .clone()
        .context("shop quantity prompt has no active shop")?;
    if quantity.selling {
        let sellable = sellable_carried_item_ids(&snapshot);
        let selected_index = sellable.iter().position(|item| item == &quantity.item_id)
            .context("selected sell item disappeared during quantity prompt")?;
        sell_visible_shop_item_from_list(runtime_shell, &sellable, selected_index, quantity.quantity)
    } else {
        let selected_index = shop.inventory.iter().position(|item| item == &quantity.item_id)
            .context("selected buy item disappeared during quantity prompt")?;
        buy_visible_shop_item_from_snapshot(runtime_shell, &shop, selected_index, quantity.quantity)
    }
}

fn buy_visible_shop_item_from_snapshot(
    runtime_shell: &mut BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    selected_index: usize,
    quantity: u16,
) -> Result<()> {
    if shop.inventory.is_empty() {
        anyhow::bail!("shop {} has no compiled inventory", shop.mart_id);
    }
    let item_id = shop
        .inventory
        .get(selected_index)
        .cloned()
        .with_context(|| {
            format!(
                "shop {} selected item index {} outside compiled inventory length {}",
                shop.mart_id,
                selected_index,
                shop.inventory.len()
            )
        })?;
    record_visible_runtime_action(
        runtime_shell,
        format!("shop:buy:{}:{}:{quantity}", shop.mart_id, item_id),
    )?;
    let transaction = runtime_shell.shell.buy_shop_item(&item_id, quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "shop buy {}/{} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        shop.inventory.len(),
        item_id,
        transaction.outcome,
        transaction.state_checksum
    ));
    let notice = visible_shop_transaction_status("BOUGHT", &item_id, &transaction.outcome);
    set_shell_action_status(runtime_shell, notice.clone());
    runtime_shell.shop_notice = Some(notice);
    runtime_shell.shop_return_to_top_after_notice = true;
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(shop) = snapshot.pending_shop.as_ref() {
        if shop.inventory.is_empty() {
            runtime_shell.menu_cursor = None;
        } else {
            runtime_shell.menu_cursor = Some(MenuCursor {
                surface_id: shop_cursor_surface_id(shop),
                option_index: selected_index.min(shop.inventory.len().saturating_sub(1)),
            });
        }
    }
    Ok(())
}

fn shop_cursor_surface_id(shop: &crate::core::state::ScriptShopRequest) -> String {
    format!(
        "shop:{}:{}:{}",
        shop.source_script, shop.command_index, shop.mart_id
    )
}

fn sell_selected_bag_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_none() {
        return handle_visible_no_active_shop(runtime_shell, "sell_confirm");
    }
    let sellable = sellable_carried_item_ids(&snapshot);
    if sellable.is_empty() {
        runtime_shell.sell_cursor = None;
        record_visible_runtime_action(runtime_shell, "shop:sell:confirm:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no sellable carried item".to_string());
        let notice = "You don't have anything to sell.".to_string();
        set_shell_action_status(runtime_shell, notice.clone());
        runtime_shell.shop_notice = Some(notice);
        runtime_shell.shop_return_to_top_after_notice = true;
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let selected_index =
        visible_cursor_index(&mut runtime_shell.sell_cursor, "sell:bag", sellable.len());
    begin_visible_shop_quantity(
        runtime_shell,
        snapshot.pending_shop.as_ref().context("no active shop")?,
        selected_index,
        true,
    )
}

fn sell_visible_shop_item_from_list(
    runtime_shell: &mut BevyRuntimeShell,
    sellable: &[String],
    selected_index: usize,
    quantity: u16,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let mart_id = snapshot
        .pending_shop
        .as_ref()
        .map(|shop| shop.mart_id.clone())
        .context("no active shop")?;
    let item_id = sellable.get(selected_index).cloned().with_context(|| {
        format!(
            "sell cursor selected item index {} outside sellable item count {}",
            selected_index,
            sellable.len()
        )
    })?;
    record_visible_runtime_action(runtime_shell, format!("shop:sell:{mart_id}:{item_id}:{quantity}"))?;
    let transaction = runtime_shell.shell.sell_shop_item(&item_id, quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "shop sell {}/{} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        sellable.len(),
        item_id,
        transaction.outcome,
        transaction.state_checksum
    ));
    let notice = visible_shop_transaction_status("SOLD", &item_id, &transaction.outcome);
    set_shell_action_status(runtime_shell, notice.clone());
    runtime_shell.shop_notice = Some(notice);
    runtime_shell.shop_return_to_top_after_notice = true;
    let snapshot = runtime_shell.shell.snapshot()?;
    let sellable = sellable_carried_item_ids(&snapshot);
    if sellable.is_empty() {
        runtime_shell.sell_cursor = None;
    } else {
        runtime_shell.sell_cursor = Some(MenuCursor {
            surface_id: "sell:bag".to_string(),
            option_index: selected_index.min(sellable.len().saturating_sub(1)),
        });
    }
    Ok(())
}

fn close_visible_shop(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shop_notice = Some("Please come again!".to_string());
    runtime_shell.shop_return_to_top_after_notice = false;
    runtime_shell.shop_close_after_notice = true;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn finalize_visible_shop_close(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shop_quantity = None;
    runtime_shell.shop_notice = None;
    runtime_shell.shop_welcome_seen = false;
    runtime_shell.shop_return_to_top_after_notice = false;
    runtime_shell.shop_close_after_notice = false;
    runtime_shell.shop_top_cursor = Some(MenuCursor {
        surface_id: "shop:top".to_string(),
        option_index: 0,
    });
    let close = runtime_shell.shell.close_script_shop()?;
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "shop closed mart={} checksum={:?}",
        close.shop.mart_id, close.state_checksum
    ));
    set_shell_action_status(runtime_shell, format!("CLOSED SHOP {}", close.shop.mart_id));
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn dismiss_visible_shop_notice(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    runtime_shell.shop_notice = None;
    if runtime_shell.shop_close_after_notice {
        return finalize_visible_shop_close(runtime_shell);
    }
    if runtime_shell.shop_return_to_top_after_notice {
        runtime_shell.shop_return_to_top_after_notice = false;
        runtime_shell.menu_cursor = None;
        runtime_shell.sell_cursor = None;
        runtime_shell.shop_quantity = None;
        runtime_shell.shop_top_cursor = Some(MenuCursor {
            surface_id: "shop:top".to_string(),
            option_index: 0,
        });
        runtime_shell.shop_notice = Some("Can I do anything\nelse for you?".to_string());
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_shop_transaction_status(
    verb: &str,
    _item_id: &str,
    outcome: &crate::core::systems::shop::ShopResult,
) -> String {
    if outcome.success {
        match verb {
            "BOUGHT" => "Here you are.\nThank you!".to_string(),
            "SOLD" => format!("Sold for {}!", outcome.message),
            _ => outcome.message.clone(),
        }
    } else {
        outcome.message.clone()
    }
}

fn close_visible_pc_surface(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pc:submenu:close")?;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.pc_item_quantity = None;
    runtime_shell.player_pc_action_cursor = None;
    runtime_shell.mailbox_cursor = None;
    runtime_shell.mailbox_action_cursor = None;
    runtime_shell.mailbox_attach_index = None;
    runtime_shell.pc_confirmation = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.bill_pc_pokemon_action_cursor = None;
    runtime_shell.bill_pc_box_summary = None;
    runtime_shell.pending_pc_release = None;
    runtime_shell.pc_notice = None;
    runtime_shell.bill_pc_box_cursor = None;
    if runtime_shell.bill_pc_session_open {
        runtime_shell.party_menu_open = false;
        runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
            surface_id: "pc:bill-actions".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "BILL'S PC");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pc_hub_session_open {
        runtime_shell.pc_hub_cursor = Some(MenuCursor {
            surface_id: "pc:hub".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    runtime_shell
        .last_audio_events
        .push("closed PC surface".to_string());
    set_shell_action_status(runtime_shell, "CLOSED PC");
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn visible_pc_hub_actions(snapshot: &RuntimeShellSnapshot) -> Vec<VisiblePcHubAction> {
    let mut actions = vec![VisiblePcHubAction::BillsPc, VisiblePcHubAction::PlayerPc];
    if snapshot.progression.active_engine_flags.contains("ENGINE_POKEDEX") {
        actions.push(VisiblePcHubAction::OakPc);
    }
    if snapshot.progression.hall_of_fame.count > 0
        || !snapshot.progression.hall_of_fame.entries.is_empty()
    {
        if !actions.contains(&VisiblePcHubAction::OakPc) {
            actions.push(VisiblePcHubAction::OakPc);
        }
        actions.push(VisiblePcHubAction::HallOfFame);
    }
    actions.push(VisiblePcHubAction::TurnOff);
    actions
}

fn visible_pc_hub_action_label(
    snapshot: &RuntimeShellSnapshot,
    action: VisiblePcHubAction,
) -> String {
    match action {
        VisiblePcHubAction::BillsPc => "BILL'S PC".to_string(),
        VisiblePcHubAction::PlayerPc => format!("{}'S PC", snapshot.trainer.player_name),
        VisiblePcHubAction::OakPc => "PROF.OAK'S PC".to_string(),
        VisiblePcHubAction::HallOfFame => "HALL OF FAME".to_string(),
        VisiblePcHubAction::TurnOff => "TURN OFF".to_string(),
    }
}

fn visible_bill_pc_action_label(action: VisibleBillPcAction) -> &'static str {
    match action {
        VisibleBillPcAction::Withdraw => "WITHDRAW POKEMON",
        VisibleBillPcAction::Deposit => "DEPOSIT POKEMON",
        VisibleBillPcAction::ChangeBox => "CHANGE BOX",
        VisibleBillPcAction::MoveWithoutMail => "MOVE PKMN W/O MAIL",
        VisibleBillPcAction::SeeYa => "SEE YA!",
    }
}

fn confirm_visible_bill_pc_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_action_cursor,
        "pc:bill-actions",
        VISIBLE_BILL_PC_ACTIONS.len(),
    )
    .context("Bill's PC action menu is open without a valid cursor")?;
    let action = VISIBLE_BILL_PC_ACTIONS[selected];
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:bill:{}", visible_bill_pc_action_label(action)),
    )?;
    runtime_shell.bill_pc_action_cursor = None;
    match action {
        VisibleBillPcAction::Withdraw => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
                option_index: 0,
            });
            runtime_shell.party_menu_open = false;
            set_shell_action_status(runtime_shell, "WITHDRAW POKEMON");
        }
        VisibleBillPcAction::Deposit => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
                option_index: 0,
            });
            open_visible_party_menu(runtime_shell)?;
            set_shell_action_status(runtime_shell, "DEPOSIT POKEMON");
        }
        VisibleBillPcAction::ChangeBox => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.bill_pc_box_cursor = Some(MenuCursor {
                surface_id: "pc:bill-boxes".to_string(),
                option_index: snapshot.storage.current_pc_box,
            });
            set_shell_action_status(runtime_shell, "CHOOSE A BOX");
        }
        VisibleBillPcAction::MoveWithoutMail => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.bill_pc_move_open = true;
            runtime_shell.bill_pc_move_source = None;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "CHOOSE A POKEMON TO MOVE");
        }
        VisibleBillPcAction::SeeYa => return close_visible_bill_pc_actions(runtime_shell),
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_bill_pc_box(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_box_cursor,
        "pc:bill-boxes",
        crate::core::models::MAX_PC_BOXES,
    )
    .context("Bill's PC box menu is open without a valid cursor")?;
    let switched = runtime_shell.shell.switch_current_pc_box(selected)?;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
        surface_id: "pc:bill-actions".to_string(),
        option_index: 2,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(
        runtime_shell,
        format!("BOX {} SELECTED", switched.box_index_after + 1),
    );
    Ok(())
}

fn confirm_visible_bill_pc_move(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let box_index = snapshot.storage.current_pc_box;
    let slot = selected_current_box_slot_index(runtime_shell)?;
    let Some((source_box, source_slot)) = runtime_shell.bill_pc_move_source else {
        let occupied = current_storage_box(&snapshot)?
            .slots
            .iter()
            .any(|candidate| candidate.index == slot);
        if !occupied {
            set_shell_action_status(runtime_shell, "NO POKEMON THERE");
            return Ok(());
        }
        runtime_shell.bill_pc_move_source = Some((box_index, slot));
        set_shell_action_status(runtime_shell, "CHOOSE A DESTINATION");
        return Ok(());
    };
    if source_box == box_index && source_slot == slot {
        runtime_shell.bill_pc_move_source = None;
        set_shell_action_status(runtime_shell, "MOVE CANCELLED");
        return Ok(());
    }
    let moved = runtime_shell.shell.move_pc_pokemon_without_mail(
        source_box,
        source_slot,
        box_index,
        slot,
    )?;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.storage_cursor = Some(MenuCursor {
        surface_id: storage_cursor_surface_id(moved.target_box),
        option_index: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(
        runtime_shell,
        if moved.swapped {
            "POKEMON SWAPPED"
        } else {
            "POKEMON MOVED"
        },
    );
    Ok(())
}

fn close_visible_bill_pc_actions(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pc:bill:see_ya")?;
    runtime_shell.bill_pc_session_open = false;
    runtime_shell.bill_pc_action_cursor = None;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.party_menu_open = false;
    runtime_shell.pc_hub_cursor = Some(MenuCursor {
        surface_id: "pc:hub".to_string(),
        option_index: 0,
    });
    set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_pc_hub(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let actions = visible_pc_hub_actions(&snapshot);
    let selected = strict_readonly_cursor_index(&runtime_shell.pc_hub_cursor, "pc:hub", actions.len())
        .context("Pokemon Center PC hub is open without a valid cursor")?;
    let action = actions[selected];
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:hub:{}", visible_pc_hub_action_label(&snapshot, action)),
    )?;
    runtime_shell.pc_hub_cursor = None;
    match action {
        VisiblePcHubAction::BillsPc => {
            runtime_shell.bill_pc_session_open = true;
            runtime_shell.bill_pc_box_cursor = None;
            runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:bill-actions".to_string(),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "BILL'S PC");
        }
        VisiblePcHubAction::PlayerPc => {
            runtime_shell.bill_pc_session_open = false;
            runtime_shell.bill_pc_action_cursor = None;
            runtime_shell.bill_pc_box_cursor = None;
            runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:player-actions".to_string(),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "PLAYER'S PC");
        }
        VisiblePcHubAction::OakPc => {
            let oak = runtime_shell.shell.show_prof_oaks_pc_boot()?;
            let SpecialRoutineEffect::ProfOaksPcBoot {
                seen_count,
                caught_count,
                rating_label,
            } = &oak.outcome.effect
            else {
                anyhow::bail!("Prof. Oak PC returned the wrong special effect");
            };
            open_visible_prof_oak_rating(
                runtime_shell,
                *seen_count,
                *caught_count,
                rating_label,
            )?;
        }
        VisiblePcHubAction::HallOfFame => {
            runtime_shell.hall_of_fame_pc_index = Some(0);
            refresh_visible_hall_of_fame_pc(runtime_shell, &snapshot)?;
            set_shell_action_status(runtime_shell, "HALL OF FAME");
        }
        VisiblePcHubAction::TurnOff => return turn_off_visible_pc_hub(runtime_shell),
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn refresh_visible_hall_of_fame_pc(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<()> {
    let entries = &snapshot.progression.hall_of_fame.entries;
    anyhow::ensure!(!entries.is_empty(), "Hall of Fame PC opened without a record");
    let index = runtime_shell
        .hall_of_fame_pc_index
        .context("Hall of Fame PC is open without an entry index")?;
    let record = entries
        .get(index)
        .with_context(|| format!("Hall of Fame entry {index} is out of range"))?;
    let mut details = vec![format!("HALL OF FAME #{:02}", index + 1)];
    details.extend(record.team.iter().enumerate().map(|(slot, pokemon)| {
        let name = pokemon
            .as_ref()
            .map(|pokemon| {
                let nickname = pokemon.nickname.trim();
                if nickname.is_empty() {
                    pokemon.species.as_str()
                } else {
                    nickname
                }
            })
            .filter(|name| !name.is_empty())
            .unwrap_or("-----");
        format!("{:>2}. {name}", slot + 1)
    }));
    runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
        label: "HallOfFamePC".to_string(),
        details,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn move_visible_hall_of_fame_pc(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let len = snapshot.progression.hall_of_fame.entries.len();
    anyhow::ensure!(len > 0, "Hall of Fame PC has no records to browse");
    let current = runtime_shell
        .hall_of_fame_pc_index
        .context("Hall of Fame PC is open without an entry index")?;
    runtime_shell.hall_of_fame_pc_index = Some(wrapped_index(current, len, delta));
    queue_visible_shell_sound_effect(runtime_shell, "SFX_MENU")?;
    refresh_visible_hall_of_fame_pc(runtime_shell, &snapshot)
}

fn open_visible_prof_oak_rating(
    runtime_shell: &mut BevyRuntimeShell,
    seen_count: usize,
    caught_count: usize,
    rating_label: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let rating = snapshot
        .presentation
        .asm_text
        .get(rating_label)
        .with_context(|| format!("Prof. Oak rating text {rating_label} is missing"))?;
    let rating = normalize_visible_script_text_with_context(
        rating,
        &snapshot.trainer.player_name,
        visible_rival_name(&snapshot),
        snapshot.progression.time.day_of_week,
    );
    runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
        label: "ProfOaksPcBoot".to_string(),
        details: vec![
            "PROF.OAK'S PC".to_string(),
            format!("SEEN {seen_count}  OWN {caught_count}"),
            rating,
        ],
    });
    set_shell_action_status(runtime_shell, "PROF.OAK'S RATING");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_player_pc_action_label(action: VisiblePlayerPcAction) -> &'static str {
    match action {
        VisiblePlayerPcAction::WithdrawItem => "WITHDRAW ITEM",
        VisiblePlayerPcAction::DepositItem => "DEPOSIT ITEM",
        VisiblePlayerPcAction::TossItem => "TOSS ITEM",
        VisiblePlayerPcAction::MailBox => "MAIL BOX",
        VisiblePlayerPcAction::Decoration => "DECORATION",
        VisiblePlayerPcAction::LogOff => "LOG OFF",
        VisiblePlayerPcAction::TurnOff => "TURN OFF",
    }
}

fn visible_player_pc_actions(runtime_shell: &BevyRuntimeShell) -> &'static [VisiblePlayerPcAction] {
    if runtime_shell.pc_hub_session_open {
        &VISIBLE_PLAYER_PC_ACTIONS
    } else {
        &VISIBLE_PLAYERS_HOUSE_PC_ACTIONS
    }
}

fn confirm_visible_player_pc_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let actions = visible_player_pc_actions(runtime_shell);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.player_pc_action_cursor,
        "pc:player-actions",
        actions.len(),
    )
    .context("Player PC action menu requires a valid cursor")?;
    let action = actions[selected];
    record_visible_runtime_action(runtime_shell, format!("pc:player:{}", visible_player_pc_action_label(action)))?;
    runtime_shell.player_pc_action_cursor = None;
    match action {
        VisiblePlayerPcAction::WithdrawItem | VisiblePlayerPcAction::TossItem => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if carried_item_count(&snapshot.bag.pc_items) == 0 {
                runtime_shell.pc_notice = Some("No items here!".to_string());
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(), option_index: selected,
                });
            } else {
                runtime_shell.pc_item_action = Some(action);
                runtime_shell.pc_item_cursor = Some(MenuCursor { surface_id: "pc:items".to_string(), option_index: 0 });
            }
        }
        VisiblePlayerPcAction::DepositItem => {
            runtime_shell.pc_item_action = Some(action);
            open_visible_pc_item_deposit_pack(runtime_shell)?;
            if !visible_field_pack_is_open(runtime_shell) {
                runtime_shell.pc_item_action = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(), option_index: selected,
                });
                runtime_shell.pc_notice = Some("No items here!".to_string());
            }
        }
        VisiblePlayerPcAction::MailBox => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.mailbox.is_empty() {
                runtime_shell.pc_notice = Some("There's no MAIL here.".to_string());
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(), option_index: selected,
                });
            } else {
                runtime_shell.mailbox_cursor = Some(MenuCursor { surface_id: "pc:mailbox".to_string(), option_index: 0 });
            }
        }
        VisiblePlayerPcAction::Decoration => {
            runtime_shell.pc_notice = Some("DECORATION".to_string());
            runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:player-actions".to_string(),
                option_index: selected,
            });
        }
        VisiblePlayerPcAction::LogOff | VisiblePlayerPcAction::TurnOff => {
            return close_visible_player_pc(runtime_shell)
        }
    }
    Ok(())
}

fn close_visible_player_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.player_pc_action_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.pc_item_quantity = None;
    runtime_shell.mailbox_cursor = None;
    runtime_shell.mailbox_action_cursor = None;
    runtime_shell.mailbox_attach_index = None;
    if runtime_shell.pc_hub_session_open {
        runtime_shell.pc_hub_cursor = Some(MenuCursor { surface_id: "pc:hub".to_string(), option_index: 0 });
        set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
        return Ok(());
    }
    runtime_shell.pc_hub_cursor = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_some() {
        let _ = runtime_shell.shell.close_active_menu()?;
    } else if snapshot.ui.window_open {
        let _ = runtime_shell.shell.close_runtime_window()?;
    }
    set_shell_action_status(runtime_shell, "LOGGED OFF");
    continue_visible_script_after_prompt(runtime_shell)
}

fn confirm_visible_mailbox_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let selected = strict_readonly_cursor_index(&runtime_shell.mailbox_cursor, "pc:mailbox", snapshot.mailbox.len())
        .context("mailbox requires a valid cursor")?;
    runtime_shell.mailbox_action_cursor = Some(MenuCursor {
        surface_id: "pc:mailbox-actions".to_string(), option_index: 0,
    });
    record_visible_runtime_action(runtime_shell, format!("pc:mailbox:select:{selected}"))?;
    Ok(())
}

fn confirm_visible_mailbox_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let action = strict_readonly_cursor_index(
        &runtime_shell.mailbox_action_cursor,
        "pc:mailbox-actions",
        VISIBLE_MAILBOX_ACTIONS.len(),
    )
    .context("mailbox action menu requires a valid cursor")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let mailbox_index = strict_readonly_cursor_index(&runtime_shell.mailbox_cursor, "pc:mailbox", snapshot.mailbox.len())
        .context("mailbox action has no selected message")?;
    let entry = &snapshot.mailbox[mailbox_index];
    match action {
        0 => runtime_shell.pc_notice = Some(format!("{}\n\nFrom {}", entry.mail.message, entry.mail.author)),
        1 => {
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::PutMailInPack(mailbox_index));
            runtime_shell.yes_no_cursor = Some(MenuCursor { surface_id: "pc:confirmation".to_string(), option_index: 0 });
            runtime_shell.pc_notice = Some("The MAIL's message will be lost. Is that OK?".to_string());
            return Ok(());
        }
        2 => {
            runtime_shell.mailbox_attach_index = Some(mailbox_index);
            runtime_shell.mailbox_action_cursor = None;
            open_visible_party_menu(runtime_shell)?;
            set_shell_action_status(runtime_shell, "ATTACH MAIL TO WHICH POKEMON?");
            return Ok(());
        }
        _ => {}
    }
    runtime_shell.mailbox_action_cursor = None;
    let latest = runtime_shell.shell.snapshot()?;
    if latest.mailbox.is_empty() {
        runtime_shell.mailbox_cursor = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
    } else if let Some(cursor) = runtime_shell.mailbox_cursor.as_mut() {
        cursor.option_index = cursor.option_index.min(latest.mailbox.len() - 1);
    }
    Ok(())
}

fn resolve_visible_pc_confirmation(runtime_shell: &mut BevyRuntimeShell, accepted: bool) -> Result<()> {
    let confirmation = runtime_shell.pc_confirmation.take().context("no PC confirmation is active")?;
    runtime_shell.yes_no_cursor = None;
    runtime_shell.pc_notice = None;
    if !accepted
        && !matches!(
            &confirmation,
            VisiblePcConfirmation::NpcTrade(_)
                | VisiblePcConfirmation::ScriptPartyIntro(_)
                | VisiblePcConfirmation::MoveDeletion { .. }
                | VisiblePcConfirmation::MoveTutorForget { .. }
                | VisiblePcConfirmation::MoveTutorStop { .. }
                | VisiblePcConfirmation::DayCareWithdraw { .. }
        )
    {
        return Ok(());
    }
    match confirmation {
        VisiblePcConfirmation::TossItem { item_id, quantity } => {
            record_visible_runtime_action(
                runtime_shell,
                format!("pc:toss_item:{item_id}:{quantity}"),
            )?;
            let transfer = runtime_shell.shell.toss_pc_item(&item_id, quantity)?;
            runtime_shell.pc_notice = Some(format!(
                "Discarded\n{}(S).",
                item_display_name(&runtime_shell.shell.snapshot()?, &transfer.item_id)
            ));
            let snapshot = runtime_shell.shell.snapshot()?;
            let count = carried_item_count(&snapshot.bag.pc_items);
            if count == 0 {
                runtime_shell.pc_item_cursor = None;
                runtime_shell.pc_item_action = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(),
                    option_index: 2,
                });
            } else if let Some(cursor) = runtime_shell.pc_item_cursor.as_mut() {
                cursor.option_index = cursor.option_index.min(count - 1);
            }
        }
        VisiblePcConfirmation::PutMailInPack(mailbox_index) => {
            match runtime_shell.shell.move_mailbox_mail_to_bag(mailbox_index) {
                Ok(_) => runtime_shell.pc_notice = Some("The MAIL was put in the PACK.".to_string()),
                Err(error) if error.to_string().contains("bag") => runtime_shell.pc_notice = Some("The PACK is full.".to_string()),
                Err(error) => return Err(error),
            }
            runtime_shell.mailbox_action_cursor = None;
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.mailbox.is_empty() {
                runtime_shell.mailbox_cursor = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
            } else if let Some(cursor) = runtime_shell.mailbox_cursor.as_mut() {
                cursor.option_index = cursor.option_index.min(snapshot.mailbox.len() - 1);
            }
        }
        VisiblePcConfirmation::NpcTrade(pending) => {
            let PendingScriptPartySelection::NpcTrade {
                origin_map_name,
                source_script,
                command_index,
                trade_id,
            } = pending
            else {
                anyhow::bail!("NPC trade confirmation contains a non-trade party request");
            };
            if accepted {
                runtime_shell.pending_script_party_selection = Some(
                    PendingScriptPartySelection::NpcTrade {
                        origin_map_name,
                        source_script,
                        command_index,
                        trade_id,
                    },
                );
                open_visible_party_menu(runtime_shell)?;
                set_shell_action_status(runtime_shell, "CHOOSE A POKEMON");
            } else {
                apply_visible_npc_trade_selection(
                    runtime_shell,
                    origin_map_name,
                    source_script,
                    command_index,
                    trade_id,
                    None,
                )?;
            }
        }
        VisiblePcConfirmation::ScriptPartyIntro(pending) => {
            if accepted {
                let status = match &pending {
                    PendingScriptPartySelection::NameRater => "WHICH POKéMON'S NICKNAME?",
                    PendingScriptPartySelection::MoveDeletion { party_index: None } => {
                        "WHICH POKéMON?"
                    }
                    PendingScriptPartySelection::DayCareDeposit { .. } => "WHICH POKéMON?",
                    _ => anyhow::bail!(
                        "script party intro contains an unsupported selection request"
                    ),
                };
                runtime_shell.pending_script_party_selection = Some(pending);
                open_visible_party_menu(runtime_shell)?;
                set_shell_action_status(runtime_shell, status);
            } else {
                let (label, details) = match pending {
                    PendingScriptPartySelection::NameRater => (
                        "NameRaterComeAgainText",
                        vec!["OK, then. Come".to_string(), "again sometime.".to_string()],
                    ),
                    PendingScriptPartySelection::MoveDeletion { party_index: None } => (
                        "DeleterNoComeAgainText",
                        vec!["No? Come visit me".to_string(), "again.".to_string()],
                    ),
                    PendingScriptPartySelection::DayCareDeposit { .. } => (
                        "DayCareOhFineText",
                        vec!["Oh, fine then.".to_string(), "Come again.".to_string()],
                    ),
                    _ => anyhow::bail!(
                        "script party intro contains an unsupported selection request"
                    ),
                };
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: label.to_string(),
                    details,
                });
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::MoveDeletion {
            party_index,
            move_index,
        } => {
            if accepted {
                record_visible_runtime_action(
                    runtime_shell,
                    format!("script:special:move_deletion:{party_index}:{move_index}"),
                )?;
                let special = runtime_shell
                    .shell
                    .delete_party_move_special(party_index, move_index)?;
                runtime_shell.last_audio_events.push(format!(
                    "move deletion outcome={:?} checksum={:?}",
                    special.outcome.effect, special.state_checksum
                ));
                queue_visible_shell_sound_effect(runtime_shell, "SFX_MOVE_DELETED")?;
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DeleterForgotMoveText".to_string(),
                    details: vec![
                        "Done! Your POKéMON".to_string(),
                        "forgot the move.".to_string(),
                    ],
                });
            } else {
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DeleterNoComeAgainText".to_string(),
                    details: vec![
                        "No? Come visit me".to_string(),
                        "again.".to_string(),
                    ],
                });
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::MoveTutorForget {
            move_id,
            party_index,
        } => {
            if accepted {
                runtime_shell.pending_script_party_selection = Some(
                    PendingScriptPartySelection::MoveTutor {
                        move_id,
                        party_index: Some(party_index),
                    },
                );
                runtime_shell.party_move_cursor = Some(MenuCursor {
                    surface_id: party_move_cursor_surface_id(party_index),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "WHICH MOVE SHOULD BE FORGOTTEN?");
            } else {
                runtime_shell.pc_notice = Some("Stop learning this move?".to_string());
                runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::MoveTutorStop {
                    move_id,
                    party_index,
                });
                runtime_shell.yes_no_cursor = Some(MenuCursor {
                    surface_id: "pc:confirmation".to_string(),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "STOP LEARNING?");
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::MoveTutorStop {
            move_id,
            party_index,
        } => {
            if accepted {
                runtime_shell.party_move_cursor = None;
                runtime_shell.pending_script_party_selection = None;
                set_visible_script_numeric_value(runtime_shell, u8::MAX);
                close_visible_party_menu(runtime_shell);
                let snapshot = runtime_shell.shell.snapshot()?;
                let nickname = snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == party_index)
                    .map(|slot| {
                        if slot.pokemon.nickname.trim().is_empty() {
                            canonical_species_display_name(&slot.pokemon.species.id)
                        } else {
                            slot.pokemon.nickname.clone()
                        }
                    })
                    .context("Move Tutor party selection is no longer present")?;
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DidNotLearnMoveText".to_string(),
                    details: vec![format!(
                        "{nickname} did not learn\n{}.",
                        move_id.replace('_', " ")
                    )],
                });
            } else {
                let snapshot = runtime_shell.shell.snapshot()?;
                let nickname = snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == party_index)
                    .map(|slot| {
                        if slot.pokemon.nickname.trim().is_empty() {
                            canonical_species_display_name(&slot.pokemon.species.id)
                        } else {
                            slot.pokemon.nickname.clone()
                        }
                    })
                    .context("Move Tutor party selection is no longer present")?;
                runtime_shell.pc_notice = Some(format!(
                    "{nickname} is\ntrying to learn\n{}.\nIt can't learn\nover four moves.\nDelete a move?",
                    move_id.replace('_', " ")
                ));
                runtime_shell.pc_confirmation = Some(
                    VisiblePcConfirmation::MoveTutorForget {
                        move_id,
                        party_index,
                    },
                );
                runtime_shell.yes_no_cursor = Some(MenuCursor {
                    surface_id: "pc:confirmation".to_string(),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "DELETE A MOVE?");
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::DayCareWithdraw { caretaker } => {
            if !accepted {
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DayCareOhFineText".to_string(),
                    details: vec!["Oh, fine then.".to_string(), "Come again.".to_string()],
                });
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            let caretaker_kind = if caretaker == "man" {
                RuntimeDayCareCaretaker::Man
            } else {
                RuntimeDayCareCaretaker::Lady
            };
            let used = runtime_shell.shell.use_day_care(
                caretaker_kind,
                RuntimeDayCareAction::Withdraw,
                None,
            )?;
            let snapshot = runtime_shell.shell.snapshot()?;
            let interaction = snapshot.day_care.last_interaction.as_ref().context("Day-Care withdrawal produced no result")?;
            runtime_shell.special_boundary_queue.clear();
            runtime_shell.special_boundary = Some(match interaction.reason.as_deref() {
                None if interaction.success => {
                    let species = interaction
                        .pokemon
                        .as_deref()
                        .context("successful Day-Care withdrawal is missing its Pokémon")?;
                    SpecialBoundaryDisplay {
                        label: "DayCareWithdrawText".to_string(),
                        details: vec![format!(
                            "Perfect! Here's your\n{} back.",
                            canonical_species_display_name(species)
                        )],
                    }
                }
                Some("party_full") => SpecialBoundaryDisplay {
                    label: "DayCarePartyFullText".to_string(),
                    details: vec!["You have no room\nfor it.".to_string()],
                },
                Some("not_enough_money") => SpecialBoundaryDisplay {
                    label: "DayCareNotEnoughMoneyText".to_string(),
                    details: vec!["You don't have\nenough money.".to_string()],
                },
                _ => SpecialBoundaryDisplay {
                    label: "DayCareOhFineText".to_string(),
                    details: vec!["Oh, fine then.".to_string()],
                },
            });
            if interaction.success {
                runtime_shell.pending_special_cry = interaction.pokemon.clone();
                runtime_shell.special_boundary_queue.push_back(SpecialBoundaryDisplay {
                    label: "DayCareGotBackText".to_string(),
                    details: vec!["Come again.".to_string()],
                });
            }
            runtime_shell.last_audio_events.push(format!("day-care withdrawal outcome={:?}", used.outcome.effect));
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::BuenaPrize { item_id } => {
            if accepted {
                let balance_before = runtime_shell.shell.snapshot()?.trainer.blue_card_balance;
                let used = runtime_shell.shell.use_buena_prize(item_id.clone(), 1)?;
                runtime_shell.last_audio_events.push(format!(
                    "Buena prize item={item_id} outcome={:?} checksum={:?}",
                    used.outcome.effect, used.state_checksum
                ));
                let snapshot = runtime_shell.shell.snapshot()?;
                if snapshot.trainer.blue_card_balance < balance_before {
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_TRANSACTION")?;
                    runtime_shell.pc_notice = Some("Here you go!".to_string());
                } else {
                    let cost = snapshot
                        .special
                        .buena_prizes
                        .get(&item_id)
                        .copied()
                        .with_context(|| format!("Buena prize {item_id} disappeared"))?;
                    runtime_shell.pc_notice = Some(if u16::from(cost) > balance_before {
                        "You don't have\nenough points.".to_string()
                    } else {
                        "You have no room\nfor it.".to_string()
                    });
                }
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
    }
    Ok(())
}

fn attach_visible_mailbox_mail(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let mailbox_index = runtime_shell.mailbox_attach_index.context("no mailbox message selected")?;
    let party_index = selected_party_index(runtime_shell)?;
    match runtime_shell.shell.attach_mailbox_mail_to_party(mailbox_index, party_index) {
        Ok(_) => {
            runtime_shell.pc_notice = Some("The MAIL was attached.".to_string());
            runtime_shell.mailbox_attach_index = None;
            close_visible_party_menu(runtime_shell);
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.mailbox.is_empty() {
                runtime_shell.mailbox_cursor = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
            } else {
                runtime_shell.mailbox_cursor = Some(MenuCursor { surface_id: "pc:mailbox".to_string(), option_index: mailbox_index.min(snapshot.mailbox.len() - 1) });
            }
        }
        Err(error) if error.to_string().contains("Egg") => runtime_shell.pc_notice = Some("An EGG can't hold MAIL.".to_string()),
        Err(error) if error.to_string().contains("already holding") => runtime_shell.pc_notice = Some("That Pokemon is holding an item.".to_string()),
        Err(error) => return Err(error),
    }
    Ok(())
}

fn turn_off_visible_pc_hub(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pc:hub:turn_off")?;
    runtime_shell.pc_hub_session_open = false;
    runtime_shell.pc_hub_cursor = None;
    runtime_shell.hall_of_fame_pc_index = None;
    runtime_shell.bill_pc_session_open = false;
    runtime_shell.bill_pc_action_cursor = None;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.pc_item_quantity = None;
    runtime_shell.player_pc_action_cursor = None;
    runtime_shell.mailbox_cursor = None;
    runtime_shell.mailbox_action_cursor = None;
    runtime_shell.mailbox_attach_index = None;
    runtime_shell.pc_confirmation = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_some() {
        let _ = runtime_shell.shell.close_active_menu()?;
    } else if snapshot.ui.window_open {
        let _ = runtime_shell.shell.close_runtime_window()?;
    }
    set_shell_action_status(runtime_shell, "TURNED OFF THE PC");
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)
}

fn close_shop_or_teleport(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_some() {
        close_visible_shop(runtime_shell)
    } else {
        use_visible_teleport(runtime_shell)
    }
}

type VisibleStaticWildOrigin = crate::RuntimeStaticWildBattleOrigin;

fn visible_static_wild_source(
    _snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Option<VisibleStaticWildOrigin> {
    match &battle.kind {
        crate::RuntimeBattleKind::StaticWild {
            origin_map_name,
            source_script,
            startbattle_command_index,
            resume_command_index,
            species,
            level,
            ..
        } => Some(VisibleStaticWildOrigin {
            map_name: origin_map_name.clone(),
            source_script: source_script.clone(),
            startbattle_command_index: *startbattle_command_index,
            resume_command_index: *resume_command_index,
            battle_type: battle.battle_type.clone(),
            species: species.clone(),
            level: *level,
        }),
        _ => None,
    }
}

fn finish_visible_wild_battle_exit(
    runtime_shell: &mut BevyRuntimeShell,
    scripted_static_wild: Option<VisibleStaticWildOrigin>,
    plain_reason: &str,
) -> Result<()> {
    reset_visible_battle_exit_state(runtime_shell);
    if let Some(origin) = scripted_static_wild {
        runtime_shell.pending_plain_battle_map_reload = false;
        complete_visible_scripted_wild_battle(runtime_shell, &origin)
    } else {
        restore_visible_overworld_after_battle_exit(runtime_shell, plain_reason)
    }
}

fn restore_visible_overworld_after_battle_exit(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    runtime_shell.pending_plain_battle_map_reload = true;
    if runtime_shell.battle_messages.is_empty() {
        queue_visible_current_music(runtime_shell)?;
        begin_visible_plain_battle_map_reload(runtime_shell)?;
    }
    // When reward/capture text remains, keep the retained battle scene and
    // its A-button ownership until the queue drains. The final acknowledgement
    // starts this reload from deterministic_session's terminal-scene branch;
    // arming the input-locking fade here would make that text impossible to
    // dismiss.
    let snapshot = runtime_shell.shell.snapshot()?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:exit:{}:{}:{}:{}",
            reason,
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y
        ),
    )?;
    set_shell_action_status(
        runtime_shell,
        format!(
            "BATTLE EXIT {} ({},{})",
            snapshot.overworld.map_name, snapshot.overworld.tile.x, snapshot.overworld.tile.y
        ),
    );
    Ok(())
}

fn begin_visible_plain_battle_map_reload(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.pending_plain_battle_map_reload = false;
    // Random encounters execute WildBattleScript's reloadmapafterbattle even
    // though they do not have a compiled map-script cursor in the desktop
    // runtime. Preserve that same MAPSETUP_RELOADMAP tiles-callback and white
    // fade boundary before field input resumes.
    runtime_shell.pending_scene_script = None;
    runtime_shell.map_callback_return_cursor = None;
    runtime_shell.map_reload_return_cursor = None;
    runtime_shell.pending_map_callbacks = visible_current_map_callback_scripts(
        runtime_shell,
        Some("MAPCALLBACK_TILES"),
    )?;
    take_next_visible_map_callback(runtime_shell)?;
    continue_visible_script_after_prompt(runtime_shell)?;
    runtime_shell.visible_walk_warp_phase = Some(VisibleWalkWarpPhase::MapReloadFadeIn);
    runtime_shell.screen_fade = Some(VisibleScreenFade::new(
        ScriptFadeColor::White,
        ScriptFadeDirection::In,
        8,
    ));
    Ok(())
}
