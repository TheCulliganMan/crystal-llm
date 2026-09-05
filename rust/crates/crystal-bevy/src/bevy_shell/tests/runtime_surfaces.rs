#[test]
fn native_gameplay_tick_journals_vblank_before_the_injected_rtc_sample() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.shell.set_runtime_journal_enabled(true);
    runtime_shell
        .shell
        .set_game_timer_counting(true)
        .expect("arm FinishContinue game timer");
    let state_before = runtime_shell.shell.session().state().clone();
    let retained_before = runtime_shell.shell.retained_runtime_commands().len();
    let sample = RuntimeRtcSample {
        date: GameDate::new(2000, 1, 2),
        hour: 13,
        minute: 14,
        second: 15,
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(NativeRtcSource::fixed(sample))
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(RuntimeTickTimer::new(0.0))
        .add_systems(Update, apply_keyboard_input);

    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert_eq!(
        runtime_shell.shell.session().state().time.registers.hours,
        13
    );
    assert_eq!(
        runtime_shell.shell.session().state().time.registers.minutes,
        14
    );
    assert_eq!(
        runtime_shell.shell.session().state().time.registers.seconds,
        15
    );
    assert_eq!(
        runtime_shell.shell.session().state().time.game_time_frames,
        1
    );
    let timer_frame = &runtime_shell.shell.retained_runtime_commands()[retained_before];
    let timer_command =
        crystal_assets::decode_runtime_mutation_command_frame(timer_frame, &state_before)
            .expect("decode native VBlank command against the pre-tick state");
    let crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(timer_command) =
        timer_command
    else {
        panic!("native tick must journal a VBlank batch first");
    };
    assert_eq!(timer_command.vblanks, 1);
    assert_eq!(timer_command.normal_divider_trace.samples.len(), 2);

    let mut state_after_vblank = state_before;
    let mut divider = crystal_core::random::ReplayDivider::new(
        timer_command.normal_divider_trace.samples.iter().copied(),
    );
    let mut rng =
        crystal_core::random::CrystalRandom::new(state_after_vblank.random_state, &mut divider);
    rng.random(false).expect("replay native VBlank_Normal");
    state_after_vblank.random_state = rng.state();
    state_after_vblank.vblank_counter = state_after_vblank.vblank_counter.wrapping_add(1);
    state_after_vblank.advance_game_timer_vblank();
    let clock_frame = &runtime_shell.shell.retained_runtime_commands()[retained_before + 1];
    let command =
        crystal_assets::decode_runtime_mutation_command_frame(clock_frame, &state_after_vblank)
            .expect("decode native RTC command against the post-VBlank state");
    let crystal_assets::RuntimeMutationCommand::UpdateClockFromDatetime(command) = command else {
        panic!("native tick must journal its RTC sample after GameTimer and before input");
    };
    assert_eq!(
        (command.date, command.hour, command.minute, command.second),
        (sample.date, sample.hour, sample.minute, sample.second)
    );
}

#[test]
fn vblank_play_timer_counts_presentation_held_text_menu_and_battle_frames() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .set_game_timer_counting(true);
    let overworld_frame = runtime_shell.shell.session().state().frame_counter;
    runtime_shell.field_notice = Some("HELD TEXT".to_string());
    runtime_shell.field_text_reveal = Some(VisibleFieldTextReveal {
        text: "HELD TEXT".to_string(),
        page_index: 0,
        visible_chars: 0,
        frames_until_next_char: 1,
    });
    let mut app = integrated_shell_test_app(runtime_shell);

    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_frames,
            1
        );
        assert_eq!(
            runtime_shell.shell.session().state().frame_counter,
            overworld_frame
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.field_notice = None;
        runtime_shell.field_text_reveal = None;
        runtime_shell.start_menu_cursor = Some(MenuCursor {
            surface_id: "start".to_string(),
            option_index: 0,
        });
    }
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_frames,
            2
        );
        assert_eq!(
            runtime_shell.shell.session().state().frame_counter,
            overworld_frame
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.start_menu_cursor = None;
        runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
            frame: 0,
            stronger_enemy: false,
            cave_environment: false,
            trainer_battle: true,
        });
    }
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_frames,
            3
        );
        assert_eq!(
            runtime_shell.shell.session().state().frame_counter,
            overworld_frame
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell
            .shell
            .session_mut()
            .state_mut()
            .set_game_logic_paused(true);
    }
    app.update();
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(
        runtime_shell.shell.session().state().time.game_time_frames,
        3
    );
    assert_eq!(
        runtime_shell.shell.session().state().frame_counter,
        overworld_frame
    );
}

#[test]
fn native_title_main_menu_keeps_game_timer_counting_clear() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let title = runtime_shell.title_menu.as_mut().expect("title menu");
    title
        .presentation_machine
        .memory
        .insert("wJumptableIndex".to_string(), 0x82);
    title
        .presentation_machine
        .memory
        .insert("wTitleScreenSelectedOption".to_string(), 0);
    title
        .presentation_machine
        .memory
        .insert("hSCX".to_string(), 0);
    let mut app = integrated_shell_test_app(runtime_shell);

    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert!(!runtime_shell.shell.session().state().game_timer_counting);
    assert_eq!(
        runtime_shell.shell.session().state().time.game_time_frames,
        0
    );
}

#[test]
fn native_catch_up_journals_one_exact_batched_game_timer_command() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.shell.set_runtime_journal_enabled(true);
    runtime_shell
        .shell
        .set_game_timer_counting(true)
        .expect("arm FinishContinue game timer");
    runtime_shell.field_notice = Some("HELD TEXT".to_string());
    runtime_shell.field_text_reveal = Some(VisibleFieldTextReveal {
        text: "HELD TEXT".to_string(),
        page_index: 0,
        visible_chars: 0,
        frames_until_next_char: 1,
    });
    let state_before = runtime_shell.shell.session().state().clone();
    let retained_before = runtime_shell.shell.retained_runtime_commands().len();
    let mut app = integrated_shell_test_app(runtime_shell);
    {
        let mut timer = app.world_mut().resource_mut::<RuntimeTickTimer>();
        timer.step_seconds = 999.0;
        timer.finished_vblanks = 120;
        timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    }

    app.update();

    let overworld_frame = {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_seconds,
            2
        );
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_frames,
            0
        );
        let retained = &runtime_shell.shell.retained_runtime_commands()[retained_before..];
        assert_eq!(retained.len(), 1);
        let command =
            crystal_assets::decode_runtime_mutation_command_frame(&retained[0], &state_before)
                .expect("decode batched catch-up VBlank command against pre-update state");
        let crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(command) = command
        else {
            panic!("catch-up tick must journal one VBlank batch");
        };
        assert_eq!(command.vblanks, 120);
        assert_eq!(command.normal_divider_trace.samples.len(), 240);
        runtime_shell.shell.session().state().frame_counter
    };
    {
        let timer = app.world().resource::<RuntimeTickTimer>();
        assert_eq!(timer.finished_ticks, 0);
        assert_eq!(timer.presentation_ticks, 0);
    }
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.field_notice = None;
        runtime_shell.field_text_reveal = None;
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .session()
            .state()
            .frame_counter,
        overworld_frame,
        "the capped input tick must be discarded while the modal owns the update"
    );
}

#[test]
fn battle_transition_consumes_every_bounded_catch_up_tick() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: 0,
        stronger_enemy: false,
        cave_environment: true,
        trainer_battle: false,
    });
    let mut timer = RuntimeTickTimer::new(999.0);
    timer.finished_vblanks = MAX_RUNTIME_CATCH_UP_TICKS;
    timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(native_rtc_source_for_test())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(timer)
        .add_systems(Update, apply_keyboard_input);

    app.update();

    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .visible_battle_transition
            .expect("battle transition remains active")
            .frame,
        MAX_RUNTIME_CATCH_UP_TICKS as u16,
        "battle entry must retain its 60 Hz duration when rendering falls below 60 FPS"
    );

    let terminal_frame = {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let transition = runtime_shell
            .visible_battle_transition
            .as_mut()
            .expect("battle transition remains active");
        let terminal_frame = visible_battle_transition_total_frames(transition) - 1;
        transition.frame = terminal_frame - 1;
        terminal_frame
    };
    {
        let mut timer = app.world_mut().resource_mut::<RuntimeTickTimer>();
        timer.finished_vblanks = MAX_RUNTIME_CATCH_UP_TICKS;
        timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .visible_battle_transition
            .expect("terminal black frame must be presented before battle setup")
            .frame,
        terminal_frame,
        "catch-up must not skip the transition's terminal black presentation"
    );
}

#[test]
fn retained_animations_consume_every_bounded_catch_up_tick() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.visible_fishing_animation = Some(VisibleFishingAnimation {
        phase: VisibleFishingPhase::Cast,
        frame: 10,
        facing_up: false,
        bite: false,
        starts_battle: false,
    });
    let mut timer = RuntimeTickTimer::new(999.0);
    timer.finished_vblanks = MAX_RUNTIME_CATCH_UP_TICKS;
    timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(native_rtc_source_for_test())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(timer)
        .add_systems(Update, apply_keyboard_input);

    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .visible_fishing_animation
            .expect("fishing cast remains active")
            .frame,
        15
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.visible_fishing_animation = None;
        runtime_shell.visible_heal_machine = Some(VisibleHealMachine {
            kind: 0,
            party_count: 1,
            frame: 1,
        });
    }
    {
        let mut timer = app.world_mut().resource_mut::<RuntimeTickTimer>();
        timer.finished_vblanks = MAX_RUNTIME_CATCH_UP_TICKS;
        timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .visible_heal_machine
            .as_ref()
            .expect("heal animation remains active")
            .frame,
        6
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.visible_heal_machine = None;
        runtime_shell.battle_hp_tween = Some(VisibleBattleHpTween {
            player_hp: 10,
            player_target_hp: 15,
            player_max_hp: 64,
            player_pixels: 10,
            player_target_pixels: 15,
            player_frames_until_step: 0,
            enemy_pixels: 20,
            enemy_target_pixels: 20,
            enemy_frames_until_step: 0,
        });
    }
    {
        let mut timer = app.world_mut().resource_mut::<RuntimeTickTimer>();
        timer.finished_vblanks = MAX_RUNTIME_CATCH_UP_TICKS;
        timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .battle_hp_tween
            .as_ref()
            .expect("HP tween remains retained")
            .player_pixels,
        13,
        "the two-frame HP pixel cadence must consume all five elapsed frames"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.battle_hp_tween = None;
        runtime_shell.pending_field_notice_effect_frames = Some(32);
        runtime_shell.visible_map_name_sign = Some(VisibleMapNameSign {
            landmark: "TEST".to_string(),
            label: "TEST".to_string(),
            frames_remaining: 10,
        });
    }
    {
        let mut timer = app.world_mut().resource_mut::<RuntimeTickTimer>();
        timer.finished_vblanks = MAX_RUNTIME_CATCH_UP_TICKS;
        timer.finished_ticks = MAX_RUNTIME_CATCH_UP_TICKS;
    }
    app.update();
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.pending_field_notice_effect_frames, Some(27));
    assert_eq!(
        runtime_shell
            .visible_map_name_sign
            .as_ref()
            .expect("map sign remains visible")
            .frames_remaining,
        5
    );
}

#[test]
fn map_name_sign_show_boundary_invalidates_the_idle_renderer() {
    let mut sign = Some(VisibleMapNameSign {
        landmark: "TEST".to_string(),
        label: "TEST".to_string(),
        // PlaceMapNameSign's old value 59 initializes the text and exposes
        // the window after decrementing the WRAM timer to 58.
        frames_remaining: 59,
    });
    assert!(
        advance_visible_map_name_sign(&mut sign, 1),
        "the 59-to-58 window-show boundary must invalidate the idle renderer"
    );
    assert_eq!(
        sign
            .as_ref()
            .expect("map sign becomes visible")
            .frames_remaining,
        58
    );

    assert!(
        !advance_visible_map_name_sign(&mut sign, 1),
        "an already-visible countdown frame does not alter the retained surface"
    );

    sign.as_mut().expect("map sign remains retained").frames_remaining = 1;
    assert!(
        !advance_visible_map_name_sign(&mut sign, 1),
        "the old-1 pass leaves the window visible with timer zero"
    );
    assert_eq!(
        sign.as_ref()
            .expect("timer zero still owns the final visible frame")
            .frames_remaining,
        0
    );
    assert!(
        advance_visible_map_name_sign(&mut sign, 1),
        "the following old-zero pass hides the window"
    );
    assert!(sign.is_none());
}

#[test]
fn modal_early_return_does_not_replay_an_already_consumed_vblank_next_update() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .set_game_timer_counting(true);
    runtime_shell.field_notice = Some("HELD TEXT".to_string());
    runtime_shell.field_text_reveal = Some(VisibleFieldTextReveal {
        text: "HELD TEXT".to_string(),
        page_index: 0,
        visible_chars: 0,
        frames_until_next_char: 1,
    });
    let mut timer = RuntimeTickTimer::new(999.0);
    timer.finished_vblanks = 1;
    timer.finished_ticks = 1;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(native_rtc_source_for_test())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(timer)
        .add_systems(Update, apply_keyboard_input);

    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .session()
            .state()
            .time
            .game_time_frames,
        1
    );

    // Both budgets were consumed atomically before the modal return. The
    // following update has neither an old VBlank nor queued input work.
    app.update();
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .session()
            .state()
            .time
            .game_time_frames,
        1
    );

    let overworld_frame = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .session()
        .state()
        .frame_counter;
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.field_notice = None;
        runtime_shell.field_text_reveal = None;
    }
    app.update();
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(
        runtime_shell.shell.session().state().frame_counter,
        overworld_frame,
        "closing a modal must not execute input frames queued while it was held"
    );
}

#[test]
fn game_timer_batch_preserves_large_count_and_caps_in_one_command() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.shell.set_runtime_journal_enabled(true);
    runtime_shell
        .shell
        .set_game_timer_counting(true)
        .expect("arm FinishContinue game timer");
    let state_before = runtime_shell.shell.session().state().clone();
    let retained_before = runtime_shell.shell.retained_runtime_commands().len();

    let outcome = runtime_shell
        .shell
        .advance_game_timer_vblanks(u32::MAX)
        .expect("advance one large exact VBlank batch");

    assert!(outcome.counted);
    assert_eq!(
        (
            outcome.hours,
            outcome.minutes,
            outcome.seconds,
            outcome.frames
        ),
        (999, 59, 59, 0)
    );
    let retained = &runtime_shell.shell.retained_runtime_commands()[retained_before..];
    assert_eq!(retained.len(), 1);
    assert_eq!(
        crystal_assets::decode_runtime_mutation_command_frame(&retained[0], &state_before)
            .expect("decode large batched VBlank command"),
        crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(
            crystal_assets::RuntimeGameTimerAdvanceCommand {
                vblanks: u32::MAX,
                normal_divider_trace: crystal_assets::RuntimeDividerTrace::new([]),
            },
        )
    );
    assert!(
        runtime_shell
            .shell
            .advance_game_timer_vblanks(0)
            .unwrap_err()
            .to_string()
            .contains("nonzero VBlank count")
    );
    assert_eq!(
        runtime_shell.shell.retained_runtime_commands().len(),
        retained_before + 1,
        "a rejected zero batch must not enter the journal"
    );
}

#[test]
fn normal_vblank_batch_records_div_and_replays_rng_with_the_timer() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.shell.set_runtime_journal_enabled(true);
    runtime_shell.shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([0x12, 0x34, 0x56, 0x78]);
    let state_before = runtime_shell.shell.session().state().clone();
    let retained_before = runtime_shell.shell.retained_runtime_commands().len();

    runtime_shell
        .shell
        .advance_vblanks(2, 2)
        .expect("advance two VBlank_Normal frames");
    assert_eq!(
        runtime_shell.shell.session().state().vblank_counter,
        state_before.vblank_counter.wrapping_add(2),
    );

    let command_frame = &runtime_shell.shell.retained_runtime_commands()[retained_before];
    let command =
        crystal_assets::decode_runtime_mutation_command_frame(command_frame, &state_before)
            .expect("decode VBlank_Normal command against pre-update state");
    assert_eq!(
        command,
        crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(
            crystal_assets::RuntimeGameTimerAdvanceCommand {
                vblanks: 2,
                normal_divider_trace: crystal_assets::RuntimeDividerTrace::new([
                    0x12, 0x34, 0x56, 0x78,
                ]),
            },
        )
    );

    let mut expected_state = state_before.clone();
    let mut divider = crystal_core::random::ReplayDivider::new([0x12, 0x34, 0x56, 0x78]);
    let mut rng =
        crystal_core::random::CrystalRandom::new(expected_state.random_state, &mut divider);
    rng.random(false).expect("first VBlank_Normal Random");
    rng.random(false).expect("second VBlank_Normal Random");
    expected_state.random_state = rng.state();
    expected_state.advance_game_timer_vblanks(2);
    assert_eq!(
        runtime_shell.shell.session().state().random_state,
        expected_state.random_state,
    );

    let mut replay = core_modular_title_shell_for_test();
    replay.intro_screen = None;
    replay.title_menu = None;
    *replay.shell.session_mut().state_mut() = state_before;
    replay.shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([]);
    replay
        .shell
        .apply_runtime_command_frame(command_frame)
        .expect("replay recorded VBlank_Normal batch without host DIV");
    assert_eq!(
        replay.shell.session().state().random_state,
        expected_state.random_state,
    );
}

#[test]
fn battle_transition_vblank_uses_cutscene_handler_without_advancing_rng() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.shell.set_runtime_journal_enabled(true);
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: 0,
        stronger_enemy: false,
        cave_environment: false,
        trainer_battle: false,
    });
    runtime_shell.shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([]);
    let state_before = runtime_shell.shell.session().state().clone();
    let retained_before = runtime_shell.shell.retained_runtime_commands().len();
    let sample = RuntimeRtcSample {
        date: GameDate::new(2000, 1, 1),
        hour: 12,
        minute: 0,
        second: 0,
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(NativeRtcSource::fixed(sample))
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(RuntimeTickTimer::new(0.0))
        .add_systems(Update, apply_keyboard_input);

    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert_eq!(
        runtime_shell.shell.session().state().random_state,
        state_before.random_state,
    );
    assert_eq!(
        runtime_shell.shell.session().state().vblank_counter,
        state_before.vblank_counter,
        "VBlank_Cutscene does not increment hVBlankCounter"
    );
    let command = crystal_assets::decode_runtime_mutation_command_frame(
        &runtime_shell.shell.retained_runtime_commands()[retained_before],
        &state_before,
    )
    .expect("decode battle-transition VBlank command");
    let crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(command) = command else {
        panic!("battle transition must still advance the VBlank timer");
    };
    assert_eq!(command.vblanks, 1);
    assert!(command.normal_divider_trace.samples.is_empty());
}

#[test]
fn only_battle_anim_engine_frames_replace_vblank_normal() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;

    runtime_shell.visible_frontpic_animation = Some(VisibleFrontpicAnimation {
        species_id: "UNOWN".to_string(),
        speed: 1,
        pointer: 0,
        repeat: 0,
        wait: 0,
        frame: 0,
    });
    assert!(
        !visible_special_vblank_handler_active(&runtime_shell),
        "AnimateFrontpic runs under VBlank_Normal"
    );
    runtime_shell.visible_frontpic_animation = None;
    runtime_shell.visible_trainer_exit_animation = Some(VisibleTrainerExitAnimation {
        side: crate::core::battle::turn::BattleSide::Enemy,
        frame: 0,
        send_out_after: false,
    });
    assert!(
        !visible_special_vblank_handler_active(&runtime_shell),
        "SlideBattlePicOut runs under VBlank_Normal"
    );
    runtime_shell.visible_trainer_exit_animation = None;
    runtime_shell.visible_send_out_animation = Some(VisibleSendOutAnimation {
        side: crate::core::battle::turn::BattleSide::Enemy,
        frame: 0,
        shiny: false,
    });
    assert!(
        visible_special_vblank_handler_active(&runtime_shell),
        "ANIM_SEND_OUT_MON installs VBlank_Cutscene"
    );
}

#[test]
fn trainer_card_phase_reads_hvblankcounter_not_gameplay_frame() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.shell.session_mut().state_mut().frame_counter = 63;
    runtime_shell.shell.session_mut().state_mut().vblank_counter = 16;

    open_visible_trainer_card(&mut runtime_shell).expect("open Trainer Card at VBlank 16");

    assert!(!runtime_shell.trainer_card_colon_visible);
    assert_eq!(runtime_shell.trainer_card_colon_ticks, 16);
}

#[test]
fn unown_puzzle_cursor_blinks_from_hvblankcounter_unless_holding_piece() {
    let mut puzzle = VisibleUnownPuzzle {
        puzzle_id: "hooh".to_string(),
        layout: [[0; 6]; 6],
        holding_piece: None,
        cursor_x: 0,
        cursor_y: 0,
        solved: false,
    };
    assert!(!visible_unown_puzzle_cursor_visible(&puzzle, 0x0f));
    assert!(visible_unown_puzzle_cursor_visible(&puzzle, 0x10));
    puzzle.holding_piece = Some(1);
    assert!(visible_unown_puzzle_cursor_visible(&puzzle, 0));
}

fn core_modular_title_shell_for_test() -> BevyRuntimeShell {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize title shell")
}

#[test]
fn dontrestartmapmusic_is_not_auto_consumed_before_map_reload() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .session
        .state
        .script_runtime
        .map_music_restart_disabled = true;

    let snapshot = runtime_shell.shell.snapshot().expect("runtime snapshot");
    assert_eq!(visible_auto_runtime_flag(&snapshot), None);

    runtime_shell
        .shell
        .session
        .state
        .script_runtime
        .map_music_requested = true;
    let snapshot = runtime_shell.shell.snapshot().expect("runtime snapshot");
    assert_eq!(
        visible_auto_runtime_flag(&snapshot),
        Some(RuntimeScriptRuntimeFlag::MapMusicRequested)
    );
    assert!(snapshot.script_events.map_music_restart_disabled);
}

#[test]
fn retained_fullscreen_lcd_survives_title_setup_and_hands_off_to_complete_overworld() {
    let mut app = integrated_shell_test_app(core_modular_title_shell_for_test());

    app.update();
    let retained = retained_fullscreen_surface(app.world_mut());
    let intro_image_count = app.world().resource::<Assets<Image>>().len();
    for _ in 0..4 {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            tick_visible_intro_screen(&mut runtime_shell).expect("advance intro LCD");
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
        assert_eq!(
            app.world().resource::<Assets<Image>>().len(),
            intro_image_count,
            "intro frames must update the retained image instead of accumulating textures"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        finish_and_drain_visible_intro_for_test(&mut runtime_shell, "retained-lcd-regression")
            .expect("handoff intro to title");
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let title = runtime_shell.title_menu.as_mut().expect("title menu");
        title
            .presentation_machine
            .memory
            .insert("hSCX".to_string(), 0);
        title
            .presentation_machine
            .values
            .insert("title_suicune_frame".to_string(), 0);
        title
            .presentation_machine
            .memory
            .insert("wTitleScreenTimer".to_string(), 10_000);
        title
            .presentation_machine
            .memory
            .insert("wJumptableIndex".to_string(), 2);
        title
            .presentation_machine
            .memory
            .insert("wTitleScreenTimer".to_string(), 10_000);
    }
    for _ in 0..40 {
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    let (bounded_title_cache, bounded_title_images) = {
        let world = app.world();
        let rendered_art = world.resource::<RenderedTilesetArt>();
        let settled_keys = rendered_art
            .title_screen_cache
            .keys()
            .filter(|key| key.scx == 0 && key.show_version_window)
            .collect::<Vec<_>>();
        assert!(
            settled_keys.len() <= 4,
            "settled title animation must cache only its four Suicune frames"
        );
        assert!(
            settled_keys
                .iter()
                .all(|key| matches!(key.frame, 0 | 8 | 16 | 24))
        );
        (
            rendered_art.title_screen_cache.len(),
            world.resource::<Assets<Image>>().len(),
        )
    };
    for _ in 0..40 {
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    assert_eq!(
        app.world()
            .resource::<RenderedTilesetArt>()
            .title_screen_cache
            .len(),
        bounded_title_cache,
        "settled title cache must be modulo its finite animation cycle"
    );
    assert_eq!(
        app.world().resource::<Assets<Image>>().len(),
        bounded_title_images,
        "repeating settled title animation must not allocate more GPU images"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        open_visible_title_main_menu(&mut runtime_shell).expect("open title main menu");
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    let main_menu_image_count = app.world().resource::<Assets<Image>>().len();
    for _ in 0..24 {
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
        assert_eq!(
            app.world().resource::<Assets<Image>>().len(),
            main_menu_image_count,
            "main-menu cursor/fade redraws must consume their transient frame"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.title_menu = None;
        open_visible_gender_selection(&mut runtime_shell).expect("open gender screen");
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    for _ in 0..VISIBLE_GENDER_FADE_IN_FRAMES {
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    for delta in [1, -1] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            move_visible_gender_selection(&mut runtime_shell, delta).expect("move gender cursor");
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    let gender_image_count = app.world().resource::<Assets<Image>>().len();
    for delta in [1, -1] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            move_visible_gender_selection(&mut runtime_shell, delta)
                .expect("repeat gender cursor state");
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    assert_eq!(
        app.world().resource::<Assets<Image>>().len(),
        gender_image_count,
        "revisiting cached gender cursor states must not allocate images"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.pending_gender_selection = None;
        open_visible_time_set_screen(&mut runtime_shell, VisibleTimeSetNext::OakIntro)
            .expect("open time-set screen");
        let time_set = runtime_shell.pending_time_set.as_mut().expect("time set");
        time_set.phase = VisibleTimeSetPhase::HourConfirm;
        time_set.visible_chars = visible_time_set_dialog_text(time_set).chars().count();
        time_set.text_timer = 0;
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    for _ in 0..2 {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Right)
                .expect("toggle time confirmation cursor");
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    let time_image_count = app.world().resource::<Assets<Image>>().len();
    for _ in 0..2 {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Left)
                .expect("repeat time confirmation cursor");
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    assert_eq!(
        app.world().resource::<Assets<Image>>().len(),
        time_image_count,
        "revisiting cached clock cursor states must not allocate images"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.pending_time_set = None;
        open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let oak_intro = runtime_shell.pending_oak_intro.as_mut().expect("Oak intro");
        oak_intro.scene_phase = VisibleOakIntroPhase::Text;
        oak_intro.fade_active = false;
        oak_intro.current_text = "HELLO!".to_string();
        oak_intro.visible_chars = oak_intro.current_text.chars().count();
        oak_intro.waiting_for_input = true;
        oak_intro.blink_timer = 30;
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell
            .pending_oak_intro
            .as_mut()
            .expect("Oak intro")
            .blink_timer = 0;
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    let oak_image_count = app.world().resource::<Assets<Image>>().len();
    for blink_timer in [30, 0] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro")
                .blink_timer = blink_timer;
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    assert_eq!(
        app.world().resource::<Assets<Image>>().len(),
        oak_image_count,
        "revisiting cached Oak blink states must not allocate images"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.pending_oak_intro = None;
        open_visible_player_name_input(&mut runtime_shell).expect("open naming screen");
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    for move_cursor in [
        move_visible_player_name_cursor_right as fn(&mut BevyRuntimeShell) -> Result<()>,
        move_visible_player_name_cursor_left,
    ] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            move_cursor(&mut runtime_shell).expect("move naming cursor");
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    let naming_image_count = app.world().resource::<Assets<Image>>().len();
    for move_cursor in [
        move_visible_player_name_cursor_right as fn(&mut BevyRuntimeShell) -> Result<()>,
        move_visible_player_name_cursor_left,
    ] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            move_cursor(&mut runtime_shell).expect("repeat naming cursor state");
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    assert_eq!(
        app.world().resource::<Assets<Image>>().len(),
        naming_image_count,
        "revisiting cached naming cursor states must not allocate images"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.pending_name_input = None;
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();
    {
        let world = app.world_mut();
        let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
        assert_eq!(
            presenters.iter(world).count(),
            1,
            "the retained LCD must cover the deferred frame that stages the replacement map layers",
        );
        assert!(
            world
                .resource::<RenderedTilesetArt>()
                .presented_fullscreen_release_pending
        );
    }
    app.update();
    {
        let world = app.world_mut();
        let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
        assert_eq!(
            presenters.iter(world).count(),
            0,
            "the retained LCD must release once both deferred map layers are query-visible",
        );
        let surfaces = retained_map_surface_pair(world);
        assert_base_map_surface_is_fully_opaque(world, &surfaces);
    }
    assert!(
        app.world()
            .resource::<RenderedTilesetArt>()
            .intro_presented_surface
            .is_some(),
        "the retained image allocation should remain ready for the next full-screen sequence"
    );
}

#[test]
fn cold_fullscreen_to_field_handoff_waits_until_both_map_layers_are_query_visible() {
    let mut app = integrated_shell_test_app(core_modular_title_shell_for_test());
    app.update();
    let retained = retained_fullscreen_surface(app.world_mut());
    {
        let world = app.world_mut();
        let mut map_layers = world.query_filtered::<Entity, With<PlayfieldTile>>();
        assert_eq!(
            map_layers.iter(world).count(),
            0,
            "intro fixture must begin with no staged field surface"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);
    {
        let world = app.world_mut();
        let surfaces = retained_map_surface_pair(world);
        assert_base_map_surface_is_fully_opaque(world, &surfaces);
    }

    app.update();
    let world = app.world_mut();
    let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
    assert_eq!(
        presenters.iter(world).count(),
        0,
        "the presenter may retire only on an update that begins with both map layers query-visible"
    );
    let surfaces = retained_map_surface_pair(world);
    assert_base_map_surface_is_fully_opaque(world, &surfaces);
}

#[test]
fn credits_redraws_reuse_one_presenter_one_image_and_one_decoded_source_bundle() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    let mut app = integrated_shell_test_app(runtime_shell);

    app.update();
    let retained = retained_fullscreen_surface(app.world_mut());
    let (source_address, source_shape, image_count) = {
        let world = app.world();
        let rendered_art = world.resource::<RenderedTilesetArt>();
        let sources = rendered_art
            .credits_sources
            .as_ref()
            .expect("credits decoded source bundle");
        assert_eq!(rendered_art.credits_source_error, None);
        (
            std::ptr::from_ref(sources).addr(),
            (
                sources.palette_sets.len(),
                sources.mon_frames.len(),
                sources.border_tiles.len(),
                sources.font.levels.len(),
                sources.copyright_tiles.len(),
                sources.the_end_levels.len(),
            ),
            world.resource::<Assets<Image>>().len(),
        )
    };

    for _ in 0..48 {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            tick_visible_credits_screen(&mut runtime_shell);
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
        assert_eq!(
            app.world().resource::<Assets<Image>>().len(),
            image_count,
            "animated credits frames must be consumed into the retained image"
        );
    }

    let rendered_art = app.world().resource::<RenderedTilesetArt>();
    let sources = rendered_art
        .credits_sources
        .as_ref()
        .expect("credits source bundle must remain cached");
    assert_eq!(
        std::ptr::from_ref(sources).addr(),
        source_address,
        "credits animation must reuse the original decoded source bundle"
    );
    assert_eq!(
        (
            sources.palette_sets.len(),
            sources.mon_frames.len(),
            sources.border_tiles.len(),
            sources.font.levels.len(),
            sources.copyright_tiles.len(),
            sources.the_end_levels.len(),
        ),
        source_shape
    );
    assert_eq!(rendered_art.credits_source_error, None);
}

#[test]
fn field_fullscreen_owner_reuses_presenter_and_releases_only_after_map_is_staged() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.visible_slot_machine = Some(VisibleSlotMachine {
        phase: VisibleSlotMachinePhase::Betting,
        animation: VisibleSlotMachineAnimation::None,
        yes_no_index: 0,
        bet: 1,
        coins: 1234,
        payout: 0,
        offsets: [14; 3],
        spin_ticks: [0; 3],
        spinning: [false; 3],
        next_reel: 1,
        actor: None,
        secondary_actor: None,
        background_y_offset: 0,
        windows: [
            ["CHERRY".into(), "SEVEN".into(), "SQUIRTLE".into()],
            ["PIKACHU".into(), "SEVEN".into(), "STARYU".into()],
            ["PIKACHU".into(), "SEVEN".into(), "PIKACHU".into()],
        ],
        message: "BET 1".to_string(),
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = integrated_shell_test_app(runtime_shell);

    app.update();
    let retained = retained_fullscreen_surface(app.world_mut());
    assert!(
        app.world()
            .resource::<RenderedTilesetArt>()
            .slot_machine_sources
            .is_some(),
        "slot renderer must retain its decoded source art"
    );
    assert_eq!(
        app.world()
            .resource::<RenderedTilesetArt>()
            .slot_machine_source_error,
        None
    );

    for bet in [2, 1] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            let machine = runtime_shell
                .visible_slot_machine
                .as_mut()
                .expect("slot machine open");
            machine.bet = bet;
            machine.message = format!("BET {bet}");
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
    }
    let bounded_image_count = app.world().resource::<Assets<Image>>().len();
    for bet in [2, 1, 2, 1, 2, 1] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            let machine = runtime_shell
                .visible_slot_machine
                .as_mut()
                .expect("slot machine open");
            machine.bet = bet;
            machine.message = format!("BET {bet}");
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();
        assert_retained_fullscreen_surface(app.world_mut(), &retained);
        assert_eq!(
            app.world().resource::<Assets<Image>>().len(),
            bounded_image_count,
            "slot animation/control redraws must not accumulate full-screen image assets"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.visible_slot_machine = None;
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();
    let world = app.world_mut();
    let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
    assert_eq!(
        presenters.iter(world).count(),
        0,
        "closing the last full-LCD field owner must release its presenter"
    );
    let surfaces = retained_map_surface_pair(world);
    assert_base_map_surface_is_fully_opaque(world, &surfaces);
}

#[test]
fn retained_field_fullscreen_ownership_distinguishes_new_game_and_capture_name_choices() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.pokedex_scripted_entry = true;
    assert!(
        !retained_field_fullscreen_active(&runtime_shell),
        "the sticky scripted-entry flag alone must not cover the battle with a stale Dex LCD"
    );

    runtime_shell.pending_name_choice = Some(VisibleNameChoice {
        options: vec!["YES".to_string(), "NO".to_string()],
        selected: 0,
        player_menu: None,
        player_phase: None,
        motion_step: 0,
        motion_frames_remaining: 0,
        pending_player_name: None,
    });
    assert!(
        retained_field_fullscreen_active(&runtime_shell),
        "the new-game preset-name menu must own a complete LCD background"
    );

    runtime_shell.pending_standard_capture = Some(PendingStandardCapture {
        outcome: crate::core::battle::capture::CaptureOutcome {
            caught: true,
            blocked: false,
            storage_full: false,
            wobble_count: 3,
            animation_shakes: 3,
            final_catch_rate: u8::MAX,
            ball_id: Some("POKE_BALL".to_string()),
        },
        scripted_static_wild: None,
        default_name: "SUDOWOODO".to_string(),
        prompt_for_nickname: true,
    });
    assert!(
        !retained_field_fullscreen_active(&runtime_shell),
        "the capture nickname YES/NO must reveal its retained battle background"
    );

    runtime_shell.pending_name_choice = None;
    runtime_shell.pending_standard_capture = None;
    runtime_shell.pokedex_menu_open = true;
    assert!(retained_field_fullscreen_active(&runtime_shell));
    runtime_shell.pokedex_detail_open = true;
    assert!(
        retained_field_fullscreen_active(&runtime_shell),
        "the nested Dex detail screen remains owned while the Dex menu is active"
    );

    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    assert!(!retained_field_fullscreen_active(&runtime_shell));
    runtime_shell.pending_name_input = Some(PendingNameInput {
        label: "SUDOWOODO'S\nNICKNAME?".to_string(),
        value: "".to_string(),
        max_length: 10,
        cursor_column: 0,
        cursor_row: 0,
        case: NameInputCase::Upper,
    });
    assert!(
        retained_field_fullscreen_active(&runtime_shell),
        "accepting the nickname prompt must hand ownership to the full-screen naming LCD"
    );
}

#[test]
fn new_game_name_choice_uses_source_menu_over_player_portrait_lcd() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_name_choice(&mut runtime_shell).expect("open source player-name menu");
    for _ in 0..40 {
        tick_visible_player_name_choice(&mut runtime_shell).expect("finish MovePlayerPicRight");
    }
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = integrated_shell_test_app(runtime_shell);

    app.update();

    let world = app.world_mut();
    let menu_sizes = world
        .query_filtered::<&Sprite, With<SceneDialogMarker>>()
        .iter(world)
        .filter_map(|sprite| sprite.custom_size)
        .collect::<Vec<_>>();
    assert!(
        menu_sizes.contains(&Vec2::new(11.0 * TILE_SIZE, 12.0 * TILE_SIZE)),
        "preset-name menu must use the exact menu_coords 0,0,10,11 extent; sizes={menu_sizes:?}"
    );

    let retained = retained_fullscreen_surface(app.world_mut());
    let images = app.world().resource::<Assets<Image>>();
    let image = images
        .get(&retained.texture)
        .expect("retained name-choice backdrop image");
    assert!(image.data.chunks_exact(4).all(|pixel| pixel[3] == 255));
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel != [255, 255, 255, 255]),
        "NamePlayer must retain the shifted player portrait and OakText6 behind its menu"
    );
    let top_right = (19 * SOURCE_TILE_SIZE) * 4;
    assert_eq!(
        &image.data[top_right..top_right + 4],
        &[255, 255, 255, 255],
        "the untouched LCD background must remain source-white"
    );
}

#[test]
fn custom_player_name_return_retains_naming_then_clears_and_redraws_portrait() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_name_choice(&mut runtime_shell).expect("open source player-name menu");
    for _ in 0..40 {
        tick_visible_player_name_choice(&mut runtime_shell).expect("finish MovePlayerPicRight");
    }
    confirm_visible_name_choice(&mut runtime_shell).expect("choose NEW NAME");
    runtime_shell.pending_name_input.as_mut().expect("NamingScreen").value = "GOLD".to_string();
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    let naming_texture = retained_fullscreen_surface(app.world_mut()).texture;

    {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        confirm_visible_player_name_input(&mut shell).expect("finish NamingScreen");
    }
    app.update();
    assert_eq!(
        retained_fullscreen_surface(app.world_mut()).texture,
        naming_texture,
        "RotateThreePalettesRight must fade the retained NamingScreen"
    );

    {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        for _ in 0..24 {
            tick_visible_player_name_choice(&mut shell).expect("finish fade out");
        }
    }
    app.update();
    let blank_texture = retained_fullscreen_surface(app.world_mut()).texture;
    let images = app.world().resource::<Assets<Image>>();
    assert!(
        images
            .get(&blank_texture)
            .expect("cleared custom-name LCD")
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]),
        "ClearTilemap and WaitBGMap must expose the source-white LCD beneath the white palette"
    );

    {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        for _ in 0..4 {
            tick_visible_player_name_choice(&mut shell).expect("finish WaitBGMap");
        }
    }
    app.update();
    let portrait_texture = retained_fullscreen_surface(app.world_mut()).texture;
    let images = app.world().resource::<Assets<Image>>();
    let image = images.get(&portrait_texture).expect("redrawn player LCD");
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel != [255, 255, 255, 255]),
        "DrawIntroPlayerPic must redraw the source player portrait before fade-in"
    );
    let textbox_start = OAK_INTRO_TEXTBOX_Y * SOURCE_TILE_SIZE * 160 * 4;
    assert!(
        image.data[textbox_start..]
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]),
        "ClearTilemap means OakText6 must not be invented after custom naming"
    );
}

#[test]
fn transparent_tileset_color_zero_is_opaque_on_base_and_clear_on_priority() {
    let source = image::RgbaImage::from_pixel(
        SOURCE_TILE_SIZE as u32,
        SOURCE_TILE_SIZE as u32,
        image::Rgba([255, 255, 255, 0]),
    );
    let palette: Palette = [
        [17, 34, 51],
        [68, 85, 102],
        [119, 136, 153],
        [170, 187, 204],
    ];
    let mut base = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
    copy_source_tile_rgba(&source, SOURCE_TILE_SIZE, 0, Some(&palette), &mut base);
    assert!(
        base.chunks_exact(4).all(|pixel| pixel == [17, 34, 51, 255]),
        "exported alpha-zero BG pixels are hardware color zero, not holes to ClearColor"
    );

    let mut unpaletted_base = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
    copy_source_tile_rgba(&source, SOURCE_TILE_SIZE, 0, None, &mut unpaletted_base);
    assert!(
        unpaletted_base
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]),
        "already-coloured tilesets must also keep Game Boy color zero opaque"
    );

    let mut priority = base.clone();
    clear_source_tile_palette_zero_alpha(&source, SOURCE_TILE_SIZE, 0, &mut priority);
    assert!(
        priority.chunks_exact(4).all(|pixel| pixel[3] == 0),
        "the separately composed priority layer must still clear color-zero pixels"
    );
}
