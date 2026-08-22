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
    assert_eq!(
        timer_command,
        crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(
            crystal_assets::RuntimeGameTimerAdvanceCommand { vblanks: 1 },
        )
    );

    let mut state_after_vblank = state_before;
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
    title.phase = VisibleTitlePhase::MainMenu;
    title.scx = 0;
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
        assert_eq!(
            command,
            crystal_assets::RuntimeMutationCommand::AdvanceGameTimerVBlanks(
                crystal_assets::RuntimeGameTimerAdvanceCommand { vblanks: 120 },
            )
        );
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
            crystal_assets::RuntimeGameTimerAdvanceCommand { vblanks: u32::MAX },
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
        finish_visible_intro_screen(&mut runtime_shell, "retained-lcd-regression")
            .expect("handoff intro to title");
    }
    app.update();
    assert_retained_fullscreen_surface(app.world_mut(), &retained);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let title = runtime_shell.title_menu.as_mut().expect("title menu");
        title.phase = VisibleTitlePhase::PressStart;
        title.scx = 0;
        title.frame = 0;
        title.title_timer = 10_000;
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
            0,
            "a field overlay releases immediately when both retained map layers were already staged"
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
        label: "NAME YOUR POKéMON?".to_string(),
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
fn new_game_name_choice_covers_the_complete_lcd_with_white() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell.pending_name_choice = Some(VisibleNameChoice {
        options: vec![
            "NEW NAME".to_string(),
            "CHRIS".to_string(),
            "MAT".to_string(),
            "ALLAN".to_string(),
            "JON".to_string(),
        ],
        selected: 0,
    });
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
        menu_sizes.contains(&Vec2::new(12.0 * TILE_SIZE, 14.0 * TILE_SIZE)),
        "preset-name menu must be wide enough for NEW NAME and no taller than its five choices; sizes={menu_sizes:?}"
    );

    let retained = retained_fullscreen_surface(app.world_mut());
    let images = app.world().resource::<Assets<Image>>();
    let image = images
        .get(&retained.texture)
        .expect("retained name-choice backdrop image");
    assert!(
        image
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]),
        "the uncovered half of the preset-name screen must be white, not retained overworld pixels"
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
