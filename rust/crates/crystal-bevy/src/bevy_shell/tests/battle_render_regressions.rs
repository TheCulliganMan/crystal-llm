#[test]
fn battle_dialogue_uses_player_input_drains_once_and_returns_menu_control() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    runtime_shell.battle_text_reveal = None;
    sync_visible_battle_action_cursor(&mut runtime_shell);

    let original_cursor = runtime_shell
        .battle_action_cursor
        .clone()
        .expect("active battle action cursor");
    let messages = [
        "CYNDAQUIL used\nTACKLE!".to_string(),
        "A deliberately long battle message that occupies more than two native textbox lines and must advance through every page exactly once.".to_string(),
        "It's not very\neffective...".to_string(),
    ];
    runtime_shell.battle_messages = messages.clone().into_iter().collect();
    runtime_shell.battle_message_scene = Some(Box::new(
        runtime_shell.shell.snapshot().expect("battle dialogue scene"),
    ));

    dispatch_visible_ui_direction(&mut runtime_shell, GameButton::Down);
    assert_eq!(
        runtime_shell.battle_action_cursor.as_ref(),
        Some(&original_cursor),
        "directional input during battle text must not navigate the hidden command menu"
    );

    let mut dismissed = Vec::new();
    for step in 0..64 {
        let Some(message) = runtime_shell.battle_messages.front().cloned() else {
            break;
        };
        let snapshot = runtime_shell
            .shell
            .presentation_snapshot()
            .expect("battle dialogue presentation snapshot");
        while !visible_battle_message_is_complete(&runtime_shell, &message) {
            assert!(
                advance_visible_battle_text_reveal(&mut runtime_shell, &snapshot, true),
                "battle dialogue reveal stalled at step {step}: {message:?}"
            );
        }
        let page_before = runtime_shell
            .battle_text_reveal
            .as_ref()
            .expect("completed battle text reveal")
            .page_index;
        press_visible_a_button(&mut runtime_shell).expect("player A advances battle dialogue");
        if runtime_shell.battle_messages.front() != Some(&message) {
            dismissed.push(message);
        } else {
            let page_after = runtime_shell
                .battle_text_reveal
                .as_ref()
                .expect("next battle text page reveal")
                .page_index;
            assert_eq!(
                page_after,
                page_before + 1,
                "A on complete battle text must advance a page or consume the message"
            );
        }
    }

    assert_eq!(dismissed, messages, "battle messages repeated or changed order");
    assert!(runtime_shell.battle_messages.is_empty());
    assert!(runtime_shell.battle_text_reveal.is_none());
    assert!(runtime_shell.battle_message_scene.is_none());
    sync_visible_battle_action_cursor(&mut runtime_shell);
    assert!(
        runtime_shell.battle_action_cursor.is_some(),
        "closing the last battle message must return command-menu control"
    );
    assert_eq!(runtime_shell.last_error, None);
}

#[test]
fn b_cancels_visible_evolution_before_success_and_restores_exact_pokemon() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "DRAGONAIR",
            55,
            Some("BERRY".to_string()),
            None,
            "EVOLUTION_CANCEL_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Dragonair");
    let dragonite = runtime_shell
        .runtime
        .data
        .pokemon
        .get("DRAGONITE")
        .expect("Dragonite in compiled pack")
        .clone();
    let original = runtime_shell.shell.session().state.storage.party.pokemon[0]
        .as_ref()
        .expect("Dragonair in party")
        .clone();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        let evolved = state.storage.party.pokemon[0]
            .as_mut()
            .expect("Dragonair in party");
        evolved.species = dragonite;
        evolved.nickname = "DRAGONITE".to_string();
        evolved.item = None;
        evolved.hp = evolved.hp.saturating_add(17);
        evolved.max_hp = evolved.max_hp.saturating_add(17);
        evolved.attack = evolved.attack.saturating_add(29);
        let evolution_move = crate::core::models::LearnedMove {
            name: "WING_ATTACK".to_string(),
            current_pp: 35,
            pp_ups: 0,
        };
        if let Some(first_move) = evolved.moves.first_mut() {
            *first_move = evolution_move;
        } else {
            evolved.moves.push(evolution_move);
        }
        state.pending_move_learn = Some(crate::core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "DRAGONITE".to_string(),
            level: 55,
            learned_move: crate::core::models::LearnedMove {
                name: "WING_ATTACK".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        state.sync_party_from_storage();
    }
    let evolving = "What? DRAGONAIR is evolving!".to_string();
    let evolved = "Congratulations! DRAGONAIR evolved into DRAGONITE!".to_string();
    let pending = "DRAGONAIR is\ntrying to learn\nWING ATTACK.".to_string();
    let report = EvolutionReport {
        target_species: Some("DRAGONITE".to_string()),
        events: vec![crate::core::systems::evolution::EvolutionEvent::Text(
            "EvolvingText",
        )],
        pending_move_learns: vec![crate::core::models::LearnedMove {
            name: "WING_ATTACK".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }],
        cancel_snapshot: Some(Box::new(original.clone())),
    };
    runtime_shell.battle_messages = [evolving.clone(), evolved.clone(), pending.clone()]
        .into_iter()
        .collect();
    runtime_shell.battle_evolution_cries = [("DRAGONITE".to_string(), evolving.clone())]
        .into_iter()
        .collect();
    runtime_shell.battle_sounds_after_messages = [("SFX_CAUGHT_MON".to_string(), evolving.clone())]
        .into_iter()
        .collect();
    runtime_shell
        .battle_evolution_cancellations
        .push_back(VisibleEvolutionCancellation {
            party_index: 0,
            trigger_message: evolving,
            evolved_message: evolved.clone(),
            pending_move_messages: vec![pending.clone()],
            report,
        });
    finish_current_battle_message_for_regression(&mut runtime_shell);

    press_visible_b_button(&mut runtime_shell).expect("cancel evolution with B");

    assert_eq!(
        runtime_shell.shell.session().state.storage.party.pokemon[0].as_ref(),
        Some(&original)
    );
    assert!(
        runtime_shell
            .shell
            .session()
            .state
            .pending_move_learn
            .is_none()
    );
    assert!(
        runtime_shell
            .shell
            .session()
            .state
            .pending_move_learn_queue
            .is_empty()
    );
    assert_eq!(
        runtime_shell.battle_messages.front().map(String::as_str),
        Some("Huh? DRAGONAIR\nstopped evolving!")
    );
    assert!(!runtime_shell.battle_messages.contains(&evolved));
    assert!(!runtime_shell.battle_messages.contains(&pending));
    assert!(runtime_shell.battle_evolution_cancellations.is_empty());
    assert!(runtime_shell.battle_evolution_cries.is_empty());
    assert!(runtime_shell.battle_sounds_after_messages.is_empty());
}

fn capture_ball_sprite_count(world: &mut World) -> usize {
    let mut commands = world.query_filtered::<&Transform, With<BattleCommandMarker>>();
    commands
        .iter(world)
        .filter(|transform| (transform.translation.z - 4.1).abs() < f32::EPSILON)
        .count()
}

fn assert_caught_capture_render_state(world: &mut World, ball_visible: bool) {
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    let capture = runtime_shell
        .visible_capture_animation
        .as_ref()
        .expect("caught capture presentation must remain retained");
    assert!(capture.complete && capture.caught);
    assert!(
        capture.enemy_hidden(),
        "the still-live core enemy must stay hidden until capture commit"
    );
    assert_eq!(capture.ball_visible(), ball_visible);
    let _ = runtime_shell;

    let mut battlers = world.query_filtered::<Entity, With<BattleBattlerMarker>>();
    assert_eq!(
        battlers.iter(world).count(),
        1,
        "only the player battler may remain while the caught enemy is pending commit"
    );
    assert_eq!(
        capture_ball_sprite_count(world),
        ball_visible as usize,
        "the retained Poké Ball entity must follow the source ClearSprites boundary"
    );
}

#[test]
fn battle_screen_offset_moves_battlers_and_commands_but_not_fixed_canvas_and_restores() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .visible_move_animations
        .push_back(VisibleMoveAnimation {
            trigger_message: "screen shake".to_string(),
            move_id: "TEST_SHAKE".to_string(),
            animation_label: "BattleAnim_TestShake".to_string(),
            player_move: true,
            started: true,
            waiting_for_hp: false,
            frame: 0,
            total_frames: 4,
            sound_events: Vec::new(),
            next_sound_event: 0,
            cry_events: Vec::new(),
            next_cry_event: 0,
            object_events: Vec::new(),
            bg_events: vec![
                VisibleMoveBgEvent {
                    frame: 0,
                    effect_id: "BATTLE_BG_EFFECT_SHAKE_SCREEN_X".to_string(),
                    duration: 4,
                    target: "3".to_string(),
                    param: 0x12,
                    incremented: false,
                },
                VisibleMoveBgEvent {
                    frame: 0,
                    effect_id: "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y".to_string(),
                    duration: 4,
                    target: "2".to_string(),
                    param: 0x12,
                    incremented: false,
                },
            ],
            actor_species_override: None,
            actor_shiny_override: None,
        });
    let expected_offset = visible_move_screen_offset(runtime_shell.visible_move_animations.front());
    assert_ne!(
        expected_offset,
        Vec3::ZERO,
        "fixture must produce a screen shake"
    );

    let battler_origin = Vec3::new(10.0, 20.0, 3.0);
    let command_origin = Vec3::new(-4.0, 7.0, 4.0);
    let canvas_origin = Vec3::new(1.0, 2.0, 2.7);
    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .add_systems(Update, apply_visible_battle_screen_offset);
    let battler = app
        .world_mut()
        .spawn((
            Transform::from_translation(battler_origin),
            BattleBattlerMarker,
        ))
        .id();
    let command = app
        .world_mut()
        .spawn((
            Transform::from_translation(command_origin),
            BattleCommandMarker,
        ))
        .id();
    let canvas = app
        .world_mut()
        .spawn((
            Transform::from_translation(canvas_origin),
            BattleCommandMarker,
            FixedBattleCanvasMarker,
        ))
        .id();

    app.update();
    assert_eq!(
        app.world()
            .entity(battler)
            .get::<Transform>()
            .unwrap()
            .translation,
        battler_origin + expected_offset
    );
    assert_eq!(
        app.world()
            .entity(command)
            .get::<Transform>()
            .unwrap()
            .translation,
        command_origin + expected_offset
    );
    assert_eq!(
        app.world()
            .entity(canvas)
            .get::<Transform>()
            .unwrap()
            .translation,
        canvas_origin,
        "the full-screen battle canvas must remain anchored to the LCD"
    );

    app.world_mut()
        .resource_mut::<BevyRuntimeShell>()
        .visible_move_animations
        .clear();
    app.update();
    assert_eq!(
        app.world()
            .entity(battler)
            .get::<Transform>()
            .unwrap()
            .translation,
        battler_origin,
        "battler offset must be removed when the shake ends"
    );
    assert_eq!(
        app.world()
            .entity(command)
            .get::<Transform>()
            .unwrap()
            .translation,
        command_origin,
        "command/HUD offset must be removed when the shake ends"
    );
    assert_eq!(
        app.world()
            .entity(canvas)
            .get::<Transform>()
            .unwrap()
            .translation,
        canvas_origin
    );
}

#[test]
fn caught_capture_retains_then_clears_sprites_without_revealing_enemy_before_commit() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    let battle_scene = runtime_shell
        .shell
        .snapshot()
        .expect("active capture battle snapshot");
    let rng_seed_after = runtime_shell.shell.session().state.rng_seed;
    let unrelated = "An unrelated queued message.".to_string();
    let gotcha = "Gotcha! SUDOWOODO\nwas caught!".to_string();
    let pokedex = "SUDOWOODO's data\nwas newly added to\nthe POKéDEX.".to_string();
    runtime_shell.battle_messages.push_back(unrelated);
    runtime_shell.battle_messages.push_back(gotcha.clone());
    runtime_shell.battle_messages.push_back(pokedex.clone());
    runtime_shell.battle_message_scene = Some(Box::new(battle_scene));
    runtime_shell.visible_capture_animation = Some(VisibleCaptureAnimation {
        trigger_message: "Player used POKé BALL!".to_string(),
        ball_id: "POKE_BALL".to_string(),
        animation_shakes: 3,
        blocked: false,
        caught: true,
        started: false,
        complete: true,
        sprites_cleared: false,
        frame: 228 + 48 * 3,
    });
    let outcome = crate::core::battle::capture::CaptureOutcome {
        caught: true,
        blocked: false,
        storage_full: false,
        wobble_count: 3,
        animation_shakes: 3,
        final_catch_rate: u8::MAX,
        rng_seed_after,
        ball_id: Some("POKE_BALL".to_string()),
    };
    runtime_shell.pending_standard_capture = Some(PendingStandardCapture {
        outcome,
        scripted_static_wild: None,
        default_name: "SUDOWOODO".to_string(),
    });
    finish_current_battle_message_for_regression(&mut runtime_shell);

    let mut app = battle_render_regression_app(runtime_shell);
    app.update();
    assert_caught_capture_render_state(app.world_mut(), true);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        press_visible_a_button(&mut runtime_shell).expect("dismiss unrelated battle page");
        assert_eq!(runtime_shell.battle_messages.front(), Some(&gotcha));
        assert!(
            runtime_shell
                .visible_capture_animation
                .as_ref()
                .is_some_and(|capture| !capture.sprites_cleared),
            "an unrelated page must not clear the retained caught ball"
        );
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), true);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        finish_current_battle_message_for_regression(&mut runtime_shell);
        press_visible_a_button(&mut runtime_shell).expect("dismiss Gotcha page");
        assert_eq!(runtime_shell.battle_messages.front(), Some(&pokedex));
        assert!(
            runtime_shell
                .visible_capture_animation
                .as_ref()
                .is_some_and(|capture| capture.sprites_cleared),
            "Gotcha dismissal must execute the source ClearSprites boundary"
        );
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        finish_current_battle_message_for_regression(&mut runtime_shell);
        press_visible_a_button(&mut runtime_shell).expect("open scripted Pokedex entry");
        assert!(runtime_shell.pokedex_scripted_entry);
        assert!(runtime_shell.pokedex_detail_open);
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);
    assert!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .battle_messages
            .is_empty(),
        "opening the scripted Pokedex entry must consume the final capture text"
    );

    for step in 0..8 {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        if !runtime_shell.pokedex_menu_open {
            break;
        }
        let capture_pending_before = runtime_shell.pending_standard_capture.is_some();
        press_visible_a_button(&mut runtime_shell).expect("advance scripted Pokedex entry");
        assert!(
            runtime_shell.battle_messages.is_empty(),
            "scripted Pokedex step {step} resumed the unfinished battle: capture_before={} capture_after={} name_choice={} {:?}",
            capture_pending_before,
            runtime_shell.pending_standard_capture.is_some(),
            runtime_shell.pending_name_choice.is_some(),
            runtime_shell.battle_messages,
        );
    }
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            !runtime_shell.pokedex_menu_open,
            "scripted Pokedex entry did not close: detail={} page={} battle_messages={:?} action={:?}",
            runtime_shell.pokedex_detail_open,
            runtime_shell.pokedex_detail_page,
            runtime_shell.battle_messages,
            runtime_shell.last_runtime_action
        );
        assert!(runtime_shell.pending_name_choice.is_some());
        assert!(runtime_shell.visible_capture_animation.is_some());
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        confirm_visible_name_choice(&mut runtime_shell).expect("choose to nickname capture");
        assert!(runtime_shell.pending_name_input.is_some());
        assert!(runtime_shell.visible_capture_animation.is_some());
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        confirm_visible_player_name_input(&mut runtime_shell)
            .expect("commit captured Pokemon nickname");
        assert!(runtime_shell.pending_standard_capture.is_none());
        assert!(runtime_shell.shell.snapshot().unwrap().battle.is_none());
        assert!(
            runtime_shell.visible_capture_animation.is_none(),
            "capture presentation must clear only after authoritative capture commit"
        );
    }
    app.update();
    let world = app.world_mut();
    let mut battlers = world.query_filtered::<Entity, With<BattleBattlerMarker>>();
    assert_eq!(battlers.iter(world).count(), 0);
    assert_eq!(capture_ball_sprite_count(world), 0);
}

#[test]
fn empty_terminal_battle_reward_starts_plain_map_reload_before_releasing_frame() {
    let mut runtime_shell = route36_overworld_shell_for_battle_render_regression();
    let terminal_scene = runtime_shell
        .shell
        .snapshot()
        .expect("terminal retained battle frame fixture");
    assert!(terminal_scene.battle.is_none());
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_exp_tween = None;
    runtime_shell.pending_battle_exp_tweens.clear();
    runtime_shell.battle_level_stats.clear();
    runtime_shell.battle_message_scene = Some(Box::new(terminal_scene));
    runtime_shell.pending_plain_battle_map_reload = true;

    assert!(
        finish_visible_empty_battle_reward_presentation(&mut runtime_shell)
            .expect("finish empty terminal reward presentation")
    );
    assert!(runtime_shell.battle_message_scene.is_none());
    assert!(!runtime_shell.pending_plain_battle_map_reload);
    assert_eq!(
        runtime_shell.visible_walk_warp_phase,
        Some(VisibleWalkWarpPhase::MapReloadFadeIn)
    );
    let fade = runtime_shell
        .screen_fade
        .expect("plain battle exit must arm its white reload fade");
    assert_eq!(fade.color, ScriptFadeColor::White);
    assert_eq!(fade.direction, ScriptFadeDirection::In);
    assert_eq!(fade.alpha, 255);
}

#[test]
fn ordinary_cave_transition_wave_uses_old_offset_accumulator_and_native_scanline_angles() {
    const PREFIX_FRAMES: u16 = 3;
    const FLASH_FRAMES: u16 = 75;
    const BETWEEN_FRAMES: u16 = 2;

    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + BETWEEN_FRAMES,
        stronger_enemy: false,
        cave_environment: true,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = battle_render_regression_app(runtime_shell);

    for outro in [0_u16, 1, 2, 5, 14] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + BETWEEN_FRAMES + outro,
                stronger_enemy: false,
                cave_environment: true,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();

        let mut counter = 0_u8;
        let mut offset = 0_u8;
        let mut amplitude = 0_u8;
        for _ in 0..=outro {
            amplitude = counter;
            let old_offset = offset;
            counter = counter.wrapping_add(old_offset);
            offset = offset.wrapping_add(1);
        }

        let world = app.world_mut();
        let mut strips = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let base_strips = strips
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.rect.is_some() && (transform.translation.z - 2.65).abs() < f32::EPSILON
            })
            .collect::<Vec<_>>();
        let mut source_rows = vec![Vec::<f32>::new(); 144];
        for (sprite, transform) in base_strips {
            assert_eq!(
                sprite.custom_size,
                Some(Vec2::new(PLAYFIELD_WIDTH, TILE_SIZE / 8.0)),
                "each native scanline must display as one 640x4 strip"
            );
            let rect = sprite.rect.expect("wave strip source rectangle");
            let source_y = (rect.min.y / (TILE_SIZE / 8.0)) as u8;
            assert_eq!(
                rect.max.y - rect.min.y,
                TILE_SIZE / 8.0,
                "wave strips must sample exactly one native scanline"
            );
            source_rows[usize::from(source_y)].push(transform.translation.x);
            let expected_y =
                PLAYFIELD_HEIGHT * 0.5 - (f32::from(source_y) + 0.5) * (TILE_SIZE / 8.0);
            assert_eq!(
                transform.translation.y, expected_y,
                "wave row {source_y} must stay centered on its native scanline"
            );
        }
        for (source_y, actual_positions) in source_rows.iter_mut().enumerate() {
            let expected_shift =
                visible_battle_anim_sine((source_y as u8).wrapping_mul(2), amplitude) as f32
                    * (TILE_SIZE / 8.0);
            let mut expected_positions = vec![expected_shift];
            if expected_shift > 0.0 {
                expected_positions.push(expected_shift - PLAYFIELD_WIDTH);
            } else if expected_shift < 0.0 {
                expected_positions.push(expected_shift + PLAYFIELD_WIDTH);
            }
            actual_positions.sort_by(f32::total_cmp);
            expected_positions.sort_by(f32::total_cmp);
            assert_eq!(
                *actual_positions, expected_positions,
                "outro {outro} row {source_y} must wrap its shifted SCX scanline"
            );
        }

        let mut priority_strips =
            world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let priority_count = priority_strips
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.rect.is_some() && (transform.translation.z - 2.66).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(
            priority_count,
            source_rows.iter().map(Vec::len).sum::<usize>(),
            "priority scanlines must use the same wrap copies as the base layer"
        );
    }
}

#[test]
fn battle_transition_outros_render_their_first_and_final_source_mutations() {
    const PREFIX_FRAMES: u16 = 3;
    const FLASH_FRAMES: u16 = 75;

    // Strong cave zoom: the nine source WaitBGMap calls are the nine
    // visible boxes, from 4x2 through the complete 20x18 LCD.
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + 1,
        stronger_enemy: true,
        cave_environment: true,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut zoom_app = battle_render_regression_app(runtime_shell);
    zoom_app.update();
    for (outro, expected_size) in [
        (0_u16, Vec2::new(TILE_SIZE * 4.0, TILE_SIZE * 2.0)),
        (8_u16, Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
    ] {
        if outro != 0 {
            let mut runtime_shell = zoom_app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + 1 + outro,
                stronger_enemy: true,
                cave_environment: true,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            drop(runtime_shell);
            zoom_app.update();
        }
        let world = zoom_app.world_mut();
        let mut black = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let surfaces = black
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.color == Color::BLACK && (transform.translation.z - 2.7).abs() < f32::EPSILON
            })
            .filter_map(|(sprite, transform)| {
                sprite.custom_size.map(|size| (size, transform.translation))
            })
            .collect::<Vec<_>>();
        assert_eq!(surfaces.len(), 1, "zoom outro {outro}");
        assert_eq!(surfaces[0].0, expected_size, "zoom outro {outro}");
        assert_eq!(
            surfaces[0].1,
            Vec3::new(0.0, 0.0, 2.7),
            "zoom boxes must expand about the exact LCD centre"
        );
    }

    // Strong outdoor scatter writes exactly twelve fresh cells per source
    // call. The first and sixteenth calls must therefore expose 12 and 192
    // black tiles respectively before the terminal hold.
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + 2,
        stronger_enemy: true,
        cave_environment: false,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut scatter_app = battle_render_regression_app(runtime_shell);
    scatter_app.update();
    for (outro, expected_tiles) in [(0_u16, 12_usize), (15_u16, 192_usize)] {
        if outro != 0 {
            let mut runtime_shell = scatter_app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + 2 + outro,
                stronger_enemy: true,
                cave_environment: false,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            drop(runtime_shell);
            scatter_app.update();
        }
        let world = scatter_app.world_mut();
        let mut black = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let count = black
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.color == Color::BLACK
                    && sprite.custom_size == Some(Vec2::splat(TILE_SIZE))
                    && (transform.translation.z - 2.7).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(count, expected_tiles, "scatter outro {outro}");
    }

    // Ordinary outdoor spin writes one wedge every three displayed frames.
    // Its five wedge shapes repeat over four quadrants and cover all 360
    // LCD cells after the twentieth write.
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + 2,
        stronger_enemy: false,
        cave_environment: false,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut spin_app = battle_render_regression_app(runtime_shell);
    spin_app.update();
    for (outro, expected_tiles) in [(0_u16, 16_usize), (57_u16, 360_usize)] {
        if outro != 0 {
            let mut runtime_shell = spin_app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + 2 + outro,
                stronger_enemy: false,
                cave_environment: false,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            drop(runtime_shell);
            spin_app.update();
        }
        let world = spin_app.world_mut();
        let mut black = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let count = black
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.color == Color::BLACK
                    && sprite.custom_size == Some(Vec2::splat(TILE_SIZE))
                    && (transform.translation.z - 2.7).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(count, expected_tiles, "spin outro {outro}");
        if outro == 57 {
            let mut tiles =
                world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
            let centers = tiles
                .iter(world)
                .filter(|(sprite, transform)| {
                    sprite.color == Color::BLACK
                        && sprite.custom_size == Some(Vec2::splat(TILE_SIZE))
                        && (transform.translation.z - 2.7).abs() < f32::EPSILON
                })
                .map(|(_, transform)| transform.translation.truncate())
                .collect::<Vec<_>>();
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.x)
                    .fold(f32::INFINITY, f32::min),
                PLAYFIELD_LEFT + TILE_SIZE * 0.5,
            );
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.x)
                    .fold(f32::NEG_INFINITY, f32::max),
                -PLAYFIELD_LEFT - TILE_SIZE * 0.5,
            );
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.y)
                    .fold(f32::INFINITY, f32::min),
                -PLAYFIELD_TOP + TILE_SIZE * 0.5,
            );
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.y)
                    .fold(f32::NEG_INFINITY, f32::max),
                PLAYFIELD_TOP - TILE_SIZE * 0.5,
            );
        }
    }
}

#[test]
fn every_battle_transition_variant_finishes_on_full_black_before_battle_canvas() {
    for (cave_environment, stronger_enemy, outro_frames, finish_frames) in [
        (true, false, 15_u16, 1_u16),
        (true, true, 9, 2),
        (false, false, 61, 4),
        (false, true, 21, 1),
    ] {
        let prefix_frames = 3_u16;
        let between_frames = if cave_environment && stronger_enemy {
            1
        } else {
            2
        };
        let total_frames = prefix_frames + 75 + between_frames + outro_frames + finish_frames;
        let mut runtime_shell = route36_battle_shell_for_render_regression();
        runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
            frame: total_frames - 1,
            stronger_enemy,
            cave_environment,
            trainer_battle: false,
        });
        let mut app = battle_render_regression_app(runtime_shell);
        app.update();

        let world = app.world_mut();
        let mut transition_surfaces =
            world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let surfaces = transition_surfaces
            .iter(world)
            .map(|(sprite, transform)| (sprite.custom_size, sprite.color, transform.translation.z))
            .collect::<Vec<_>>();
        let terminal_black = surfaces
            .iter()
            .filter(|(size, color, z)| {
                *size == Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
                    && *color == Color::BLACK
                    && (*z - 2.76).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(
            terminal_black, 1,
            "variant cave={cave_environment} stronger={stronger_enemy} must hold one opaque full-LCD black finish frame"
        );
        assert_eq!(
            surfaces
                .iter()
                .map(|(_, _, z)| *z)
                .fold(f32::NEG_INFINITY, f32::max),
            2.76,
            "terminal black must be the top transition surface"
        );
        let mut fixed_canvas = world.query_filtered::<Entity, With<FixedBattleCanvasMarker>>();
        assert_eq!(fixed_canvas.iter(world).count(), 0);
        let _ = world;

        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            advance_visible_battle_transition(&mut runtime_shell);
            assert!(runtime_shell.visible_battle_transition.is_none());
        }
        app.update();
        let world = app.world_mut();
        let mut fixed_canvas =
            world.query_filtered::<(&Sprite, &Transform), With<FixedBattleCanvasMarker>>();
        let canvases = fixed_canvas.iter(world).collect::<Vec<_>>();
        assert_eq!(
            canvases.len(),
            1,
            "variant cave={cave_environment} stronger={stronger_enemy} must hand off directly to one fixed battle canvas"
        );
        assert_eq!(
            canvases[0].0.custom_size,
            Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
        );
        assert_eq!(canvases[0].1.translation, Vec3::new(0.0, 0.0, 2.7));
    }
}

#[test]
fn battle_redraw_retains_fixed_canvas_without_image_growth_even_if_overlay_rebuild_fails() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_text_reveal = None;
    sync_visible_battle_action_cursor(&mut runtime_shell);

    let mut app = battle_render_regression_app(runtime_shell);
    app.update();

    let first_canvas = {
        let world = app.world_mut();
        let mut canvases = world.query_filtered::<Entity, With<FixedBattleCanvasMarker>>();
        let canvases = canvases.iter(world).collect::<Vec<_>>();
        assert_eq!(
            canvases.len(),
            1,
            "battle must stage exactly one opaque canvas"
        );
        canvases[0]
    };
    let stable_image_count = app.world().resource::<Assets<Image>>().len();

    // Exercise both eight-frame party-icon phases and the sixteen-frame
    // cursor phase. These redraw the transient battle layers but must not
    // allocate replacement textures or replace the continuity canvas.
    for _ in 0..4 {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.lcd_animation_frame = runtime_shell.lcd_animation_frame.wrapping_add(8);
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();
        let world = app.world_mut();
        let mut canvases = world.query_filtered::<Entity, With<FixedBattleCanvasMarker>>();
        assert_eq!(canvases.iter(world).collect::<Vec<_>>(), vec![first_canvas]);
        assert_eq!(
            world.resource::<Assets<Image>>().len(),
            stable_image_count,
            "steady battle redraws must reuse cached image assets"
        );
    }

    // Fail a stage that runs after battlers and HUD have begun rebuilding.
    // The prior full-LCD canvas must survive the early return and continue
    // hiding the retained overworld rather than exposing it for a frame.
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.battle_action_cursor = Some(MenuCursor {
            surface_id: "invalid:battle-actions".to_string(),
            option_index: usize::MAX,
        });
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();

    let world = app.world_mut();
    assert!(
        world
            .resource::<BevyRuntimeShell>()
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("battle main-action cursor is invalid")),
        "fixture must reach the forced post-canvas render failure"
    );
    let mut canvases =
        world.query_filtered::<(Entity, &Sprite, &Transform), With<FixedBattleCanvasMarker>>();
    let canvases = canvases.iter(world).collect::<Vec<_>>();
    assert_eq!(canvases.len(), 1);
    assert_eq!(canvases[0].0, first_canvas);
    assert_eq!(
        canvases[0].1.custom_size,
        Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
    );
    assert_eq!(canvases[0].2.translation, Vec3::new(0.0, 0.0, 2.7));
    assert_eq!(
        world.resource::<Assets<Image>>().len(),
        stable_image_count,
        "the failed redraw must not leak image assets"
    );
}
