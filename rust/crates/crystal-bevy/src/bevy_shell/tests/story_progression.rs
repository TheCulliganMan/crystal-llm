#[test]
fn real_pack_bug_contest_timeout_warps_to_national_park_gate() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let (tile_x, tile_y) = (0..80)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .find(|(x, y)| {
            runtime
                .start_overworld_session_at_runtime_tile(
                    &asset_root,
                    "NationalParkBugContest",
                    *x,
                    *y,
                )
                .is_ok()
        })
        .expect("compiled Bug Contest map must have a walkable tile");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "NationalParkBugContest".to_string(),
            tile_x,
            tile_y,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Bug Contest map shell");
    let state = runtime_shell.shell.session_mut().state_mut();
    state.bug_contest.timer_active = true;
    state.bug_contest.timer_minutes_remaining = 20;
    state.bug_contest.timer_seconds_remaining = 0;
    state.bug_contest.timer_start_time = Some(crate::core::systems::time::ClockTime {
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
    });
    state.time.current_day = 1;
    state.time.day_of_week = 1;
    state
        .flags
        .set_engine_flag("ENGINE_BUG_CONTEST_TIMER", true)
        .expect("compiled pack Bug Contest timer flag");

    runtime_shell
        .shell
        .tick(std::iter::empty::<GameButton>())
        .expect("advance expired Bug Contest frame");
    let snapshot = runtime_shell.shell.snapshot().expect("timeout snapshot");
    assert_eq!(snapshot.overworld.map_name, "Route36NationalParkGate");
    assert_eq!(snapshot.overworld.tile, TilePosition::new(0, 4));
    assert!(!snapshot.bug_contest.timer_active);
    assert_eq!(
        runtime_shell
            .shell
            .session()
            .state()
            .flags
            .is_event_flag_set("EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_CONTEST_DAY")
            .expect("contest-day flag"),
        true
    );
}

#[test]
fn visible_overworld_normal_inputs_trigger_mom_coord_event() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &[
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
        ],
        None,
    )
    .expect("normal inputs trigger Mom's first-floor coord event");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(smoke.final_map, "PlayersHouse1F");
    assert_eq!((smoke.final_tile_x, smoke.final_tile_y), (9, 4));
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_PLAYERSHOUSE1F_NOOP")
    );
    assert_eq!(smoke.warps, 1);
    assert_eq!(smoke.coord_events, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("warp=true"))
    );
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("coord=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("MUSIC_MOM"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag ENGINE_POKEGEAR=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag ENGINE_PHONE_CARD=true"))
    );
}

#[test]
fn moms_coord_event_keeps_the_written_dialogue_in_the_textbox() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize new-game shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell).expect("settle new-game scripts");

    let inputs = [
        GameButton::Right,
        GameButton::Right,
        GameButton::Right,
        GameButton::Right,
        GameButton::Right,
        GameButton::Right,
        GameButton::Up,
        GameButton::Up,
        GameButton::Up,
        GameButton::Up,
        GameButton::Down,
        GameButton::Down,
        GameButton::Down,
        GameButton::Down,
        GameButton::Down,
    ];
    'walk_to_mom: for button in inputs {
        let outcome = apply_visible_shell_smoke_frame(&mut runtime_shell, &[button])
            .expect("advance Player's House input");
        if let Some(frame) = outcome.frame {
            if frame.coord_event.is_some() {
                execute_last_coord_event_script(&mut runtime_shell)
                    .expect("run Mom coord-event script");
                break 'walk_to_mom;
            }
            if frame.warp.is_some() {
                settle_visible_overworld_frame_arrival(&mut runtime_shell)
                    .expect("settle bedroom warp");
            }
        }
        settle_visible_shell_smoke_until_idle(&mut runtime_shell)
            .expect("settle Player's House scripts");
    }

    let initial_snapshot = runtime_shell.shell.snapshot().expect("Mom emote snapshot");
    assert_ne!(
        initial_snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
        Some("ElmsLookingForYouText"),
        "Mom script fell through to text before its blocking visual phases: action={:?} cursor={:?} movement={:?} emote={:?} events={:?}",
        runtime_shell.last_runtime_action,
        runtime_shell.active_script_cursor,
        runtime_shell.visible_script_movement,
        runtime_shell.visible_overworld_emote,
        runtime_shell.last_audio_events
    );
    assert!(runtime_shell.visible_script_movement.is_none());
    assert_eq!(
        runtime_shell
            .visible_overworld_emote
            .as_ref()
            .map(|emote| emote.frames_remaining),
        Some(15),
        "showemote must hold MeetMomRightScript for its authored duration"
    );

    let mut app = integrated_shell_test_app(runtime_shell);
    for frame in 0..14 {
        app.update();
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = runtime_shell.shell.snapshot().expect("Mom emote frame");
        assert_eq!(runtime_shell.last_error, None, "emote frame {frame}");
        assert!(
            runtime_shell.visible_overworld_emote.is_some(),
            "emote frame {frame}"
        );
        assert!(
            runtime_shell.visible_script_movement.is_none(),
            "emote frame {frame}"
        );
        assert_ne!(
            snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
            Some("ElmsLookingForYouText"),
            "emote frame {frame}"
        );
    }

    let mut saw_mom_movement = false;
    let mut dialogue_opened = false;
    for _ in 0..64 {
        app.update();
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        saw_mom_movement |= runtime_shell.visible_script_movement.is_some();
        let snapshot = runtime_shell.shell.snapshot().expect("Mom sequence frame");
        if snapshot
            .ui
            .text
            .as_ref()
            .is_some_and(|text| text.label == "ElmsLookingForYouText")
        {
            dialogue_opened = true;
            break;
        }
    }
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert!(
        saw_mom_movement,
        "Mom must walk after the emote finishes: action={:?} cursor={:?} movement={:?} emote={:?} events={:?}",
        runtime_shell.last_runtime_action,
        runtime_shell.active_script_cursor,
        runtime_shell.visible_script_movement,
        runtime_shell.visible_overworld_emote,
        runtime_shell.last_audio_events
    );
    assert!(
        dialogue_opened,
        "Mom's dialogue must open after her walk finishes: action={:?} cursor={:?} movement={:?} emote={:?} text={:?} events={:?}",
        runtime_shell.last_runtime_action,
        runtime_shell.active_script_cursor,
        runtime_shell.visible_script_movement,
        runtime_shell.visible_overworld_emote,
        runtime_shell.shell.snapshot().ok().and_then(|snapshot| snapshot.ui.text),
        runtime_shell.last_audio_events
    );

    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("Mom dialogue snapshot");
    let text = snapshot.ui.text.as_ref().expect("Mom textbox is active");
    assert_eq!(text.label, "ElmsLookingForYouText");
    let rendered = render_visible_script_text_body(
        text.body.as_ref().expect("Mom text is a map text body"),
        &snapshot.script_events.named_buffers,
        &snapshot.trainer.player_name,
        visible_rival_name(&snapshot),
        snapshot.progression.time.day_of_week,
    );
    assert!(rendered.contains("Oh, AB…! Our"));
    assert!(rendered.contains("neighbor, PROF."));
    assert!(!rendered.contains("MeetMomScript"));
    assert!(!rendered.contains('"'));
}

#[test]
fn visible_new_game_completes_mom_walks_to_elms_lab_and_gets_rendered_starter() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize new game");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("finish player naming");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell).expect("settle bedroom arrival");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    // Real joypad route from the bedroom spawn, through the stair warp, and
    // onto Mom's authored coordinate event.
    for key in [
        KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowRight,
        KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowRight,
        KeyCode::ArrowUp, KeyCode::ArrowUp, KeyCode::ArrowUp, KeyCode::ArrowUp,
    ] {
        press_key_for_runtime_hotkey_app(&mut app, key);
        for _ in 0..8 {
            app.update();
        }
    }
    for _ in 0..64 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        if shell.visible_walk_warp_phase.is_none()
            && shell.shell.snapshot().unwrap().overworld.map_name == "PlayersHouse1F"
        {
            break;
        }
    }
    for _ in 0..5 {
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
        for _ in 0..8 {
            app.update();
        }
    }

    let mut seen_labels = Vec::new();
    let mut saw_rendered_mom_text = false;
    let mut saw_rendered_yes_no = false;
    let mut saw_canonical_day_selector = false;
    let mut saw_dynamic_dst_confirmation = false;
    let mut completed_mom = false;
    let mut mom_frames = 0usize;
    for frame in 0..1024 {
        mom_frames = frame + 1;
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(shell.last_error, None, "Mom lifecycle failed: {:?}", shell.last_audio_events);
        let snapshot = shell.shell.snapshot().expect("Mom lifecycle snapshot");
        let visible_label = snapshot.ui.text.as_ref().map(|text| text.label.clone());
        let pending_yes_no = snapshot.ui.pending_yes_no.is_some();
        if pending_yes_no
            && matches!(
                snapshot.script_events.last_special_routine.as_deref(),
                Some("InitialSetDSTFlag" | "InitialClearDSTFlag")
            )
        {
            let entries = visible_scene_dialog_entries(&snapshot, shell)
                .expect("render Mom DST confirmation entries");
            saw_dynamic_dst_confirmation |= entries.iter().any(|line| line.contains(':'))
                && entries.iter().any(|line| line.contains("is that OK?"));
        }
        if let Some(label) = visible_label.clone()
            && seen_labels.last() != Some(&label)
        {
            seen_labels.push(label);
        }
        let current_scene = shell
            .shell
            .current_scene_script()
            .expect("current Player's House scene")
            .map(|scene| scene.scene_id);
        completed_mom = snapshot.overworld.map_name == "PlayersHouse1F"
            && current_scene.as_deref() == Some("SCENE_PLAYERSHOUSE1F_NOOP")
            && snapshot.progression.active_engine_flags.contains("ENGINE_POKEGEAR")
            && snapshot.progression.active_engine_flags.contains("ENGINE_PHONE_CARD")
            && shell.active_script_cursor.is_none()
            && shell.visible_script_movement.is_none()
            && shell.visible_overworld_emote.is_none()
            && shell.player_walk_frame_ticks == 0
            && shell.object_walk_frame_ticks == 0
            && shell.object_walk_frame_ticks_by_id.is_empty()
            && shell.special_boundary.is_none()
            && shell.pending_day_of_week.is_none()
            && !snapshot.ui.text_window_open;
        if completed_mom {
            break;
        }
        let pending_day_of_week = shell.pending_day_of_week.is_some();
        let pending_day_confirming = shell
            .pending_day_of_week
            .as_ref()
            .is_some_and(|prompt| prompt.confirming);
        let _ = shell;
        if visible_label.is_some() || pending_yes_no {
            let world = app.world_mut();
            let has_textbox = world
                .query_filtered::<Entity, With<SceneDialogTextBoxBackgroundMarker>>()
                .iter(world)
                .next()
                .is_some();
            let has_glyphs = world
                .query_filtered::<Entity, With<DialogGlyphMarker>>()
                .iter(world)
                .next()
                .is_some();
            saw_rendered_mom_text |= visible_label.is_some() && has_textbox && has_glyphs;
            saw_rendered_yes_no |= pending_yes_no && has_textbox && has_glyphs;
            if pending_day_of_week {
                let custom_sizes = world
                    .query_filtered::<&Sprite, With<SceneDialogTextBoxBackgroundMarker>>()
                    .iter(world)
                    .filter_map(|sprite| sprite.custom_size)
                    .collect::<Vec<_>>();
                saw_canonical_day_selector |= custom_sizes
                    .contains(&Vec2::new(9.0 * TILE_SIZE, 2.0 * TILE_SIZE));
                if pending_day_confirming {
                    assert!(
                        !custom_sizes.contains(&Vec2::new(9.0 * TILE_SIZE, 2.0 * TILE_SIZE)),
                        "day confirmation must replace the selector instead of stacking both windows"
                    );
                }
            }
        }
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    }
    let failed_shell = app.world().resource::<BevyRuntimeShell>();
    let failed_snapshot = failed_shell.shell.snapshot().expect("failed Mom snapshot");
    assert!(
        completed_mom,
        "Mom never completed; labels={seen_labels:?} map={} tile={:?} cursor={:?} action={:?} events={:?}",
        failed_snapshot.overworld.map_name,
        failed_snapshot.overworld.tile,
        failed_shell.active_script_cursor,
        failed_shell.last_runtime_action,
        failed_shell.last_audio_events
    );
    assert!(
        mom_frames < 1024,
        "Mom's complete visible interaction exceeded the real-input frame budget"
    );
    let _ = failed_shell;
    assert_eq!(
        seen_labels.first().map(String::as_str),
        Some("ElmsLookingForYouText"),
        "Mom must begin with her canonical dialogue, not a script/pre-text label"
    );
    let canonical_mom_labels = [
        "ElmsLookingForYouText",
        "MomGivesPokegearText",
        "IsItDSTText",
        "ComeHomeForDSTText",
        "KnowTheInstructionsText",
        "InstructionsNextText",
    ];
    let mut next_expected = 0usize;
    for label in &seen_labels {
        if canonical_mom_labels.get(next_expected) == Some(&label.as_str()) {
            next_expected += 1;
        }
    }
    assert_eq!(
        next_expected,
        canonical_mom_labels.len(),
        "Mom must render every ASM-authored dialogue label in order; saw {seen_labels:?}"
    );
    assert!(
        saw_canonical_day_selector,
        "SetDayOfWeek must render its separate canonical 11x4 selector window"
    );
    assert!(
        saw_dynamic_dst_confirmation,
        "DST setup must visibly render the live HH:MM confirmation before its yes/no"
    );
    assert!(saw_rendered_mom_text, "Mom executed text without rendering its textbox and bitmap glyphs");
    assert!(saw_rendered_yes_no, "Mom's yes/no executed without rendering an interactive prompt");
    let direction_still_captured = {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        has_visible_shell_direction_action(&mut shell)
    };
    assert!(
        !direction_still_captured,
        "Mom finished but a modal/script surface still captures directional input"
    );
    assert_overworld_control_returns_and_player_moves(&mut app, "MeetMomScript");

    // Derive the front-door route from the compiled collision map, then
    // replay every tile through the production Bevy keyboard path.
    let exit_path = {
        let shell = app.world().resource::<BevyRuntimeShell>();
        let start = shell.shell.session().overworld.clone();
        let mut queue = std::collections::VecDeque::from([(start.clone(), Vec::new())]);
        let mut visited = std::collections::BTreeSet::from([(
            start.player.tile.x,
            start.player.tile.y,
        )]);
        let mut found = None;
        while let Some((session, path)) = queue.pop_front() {
            for direction in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
                let mut next = session.clone();
                let Ok(mut step) = next.step_and_check_warp_checked(
                    direction,
                    crate::core::world::movement::StepOptions::default(),
                ) else {
                    continue;
                };
                if matches!(step.outcome, crate::core::world::movement::StepOutcome::Turned { .. }) {
                    let Ok(second) = next.step_and_check_warp_checked(
                        direction,
                        crate::core::world::movement::StepOptions::default(),
                    ) else {
                        continue;
                    };
                    step = second;
                }
                if !matches!(step.outcome, crate::core::world::movement::StepOutcome::Moved { .. }) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(direction);
                if let Some(warp) = step.warp.as_ref() {
                    if warp.warp.target_map == "NEW_BARK_TOWN"
                        || warp.warp.target_map == "NewBarkTown"
                    {
                        found = Some(next_path);
                        break;
                    }
                    continue;
                }
                if visited.insert((next.player.tile.x, next.player.tile.y)) {
                    queue.push_back((next, next_path));
                }
            }
            if found.is_some() {
                break;
            }
        }
        found.expect("compiled Player's House collision must reach its front-door warp")
    };
    for direction in exit_path {
        let key = match direction {
            Direction::Up => KeyCode::ArrowUp,
            Direction::Down => KeyCode::ArrowDown,
            Direction::Left => KeyCode::ArrowLeft,
            Direction::Right => KeyCode::ArrowRight,
        };
        let before = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap();
        for _ in 0..2 {
            press_key_for_runtime_hotkey_app(&mut app, key);
            for _ in 0..9 {
                app.update();
            }
            let after = app
                .world()
                .resource::<BevyRuntimeShell>()
                .shell
                .snapshot()
                .unwrap();
            if after.overworld.map_name != before.overworld.map_name
                || after.overworld.tile != before.overworld.tile
            {
                break;
            }
        }
    }
    for _ in 0..256 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name
            == "NewBarkTown"
        {
            break;
        }
    }
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("house-exit snapshot");
    assert_eq!(shell.last_error, None);
    assert_eq!(
        snapshot.overworld.map_name,
        "NewBarkTown",
        "Mom completed but live movement still could not exit the house; tile={:?} pending_reason={:?} objects={:?} script_locks={:?} locks={:?} events={:?}",
        snapshot.overworld.tile,
        shell.shell.pending_script_work_reason(),
        (&shell.shell.session().overworld.object_runtime_tiles, &shell.shell.session().overworld.object_last_runtime_tiles),
        (shell.shell.session().state().script_runtime.player_input_locked, shell.shell.session().state().script_runtime.all_input_locked, shell.shell.session().state().script_runtime.script_stop_requested),
        (&shell.field_text_reveal, shell.visible_script_delay_frames, &shell.visible_walk_warp_phase, &shell.pending_overworld_step_boundary, &shell.visible_script_movement, &shell.visible_overworld_emote),
        (shell.last_runtime_action.clone(), shell.last_overworld_input.clone(), shell.recent_overworld_inputs.clone(), shell.last_audio_events.clone())
    );
    let _ = shell;

    let mut saw_route_text = false;
    for _ in 0..96 {
        let current_map = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name;
        if current_map == "ElmsLab" {
            break;
        }
        let path = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            collision_path_to_map_warp(&shell.shell.session().overworld, "ELMS_LAB")
        };
        press_visible_direction_until_tile_changes(&mut app, path[0]);
        saw_route_text |= settle_visible_story_boundary(&mut app).0;
    }
    assert_eq!(
        app.world().resource::<BevyRuntimeShell>().shell.snapshot().unwrap().overworld.map_name,
        "ElmsLab",
        "live keyboard route never reached Elm's lab"
    );

    for _ in 0..64 {
        let tile = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .tile;
        if tile == (TilePosition { x: 5, y: 3 }) {
            break;
        }
        let path = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            collision_path_to_tile(
                &shell.shell.session().overworld,
                TilePosition { x: 5, y: 3 },
            )
        };
        press_visible_direction_until_tile_changes(&mut app, path[0]);
        saw_route_text |= settle_visible_story_boundary(&mut app).0;
    }
    assert_eq!(
        app.world().resource::<BevyRuntimeShell>().shell.snapshot().unwrap().overworld.tile,
        TilePosition { x: 5, y: 3 },
        "live keyboard route never reached the Cyndaquil ball"
    );
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    let (starter_text_rendered, starter_picture_rendered) = settle_visible_story_boundary(&mut app);
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("post-starter snapshot");
    assert_eq!(shell.last_error, None);
    assert!(saw_route_text, "the New Bark/Elm story executed without rendered dialogue");
    assert!(starter_text_rendered, "the starter script executed without rendered text glyphs");
    assert!(starter_picture_rendered, "the starter script executed without rendering Cyndaquil's picture");
    assert_eq!(snapshot.party.slots.len(), 1);
    assert_eq!(snapshot.party.slots[0].pokemon.species.id, "CYNDAQUIL");
    assert!(snapshot.progression.active_event_flags.contains("EVENT_GOT_CYNDAQUIL_FROM_ELM"));
    assert!(snapshot.progression.active_event_flags.contains("EVENT_GOT_A_POKEMON_FROM_ELM"));
    assert_eq!(shell.active_script_cursor, None, "starter script did not finish");
}

fn collision_path_to_map_warp(
    start: &crate::core::world::session::OverworldSession,
    target_map: &str,
) -> Vec<Direction> {
    let mut queue = std::collections::VecDeque::from([(start.clone(), Vec::new())]);
    let mut visited = std::collections::BTreeSet::from([(start.player.tile.x, start.player.tile.y)]);
    while let Some((session, path)) = queue.pop_front() {
        for direction in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
            let mut next = session.clone();
            let mut step = next
                .step_and_check_warp_checked(
                    direction,
                    crate::core::world::movement::StepOptions::default(),
                )
                .expect("probe compiled collision");
            if matches!(step.outcome, crate::core::world::movement::StepOutcome::Turned { .. }) {
                step = next
                    .step_and_check_warp_checked(
                        direction,
                        crate::core::world::movement::StepOptions::default(),
                    )
                    .expect("probe compiled collision after turn");
            }
            if !matches!(step.outcome, crate::core::world::movement::StepOutcome::Moved { .. }) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(direction);
            if let Some(warp) = step.warp {
                if warp.warp.target_map == target_map
                    || warp.warp.target_map_constant == target_map
                {
                    return next_path;
                }
                continue;
            }
            if visited.insert((next.player.tile.x, next.player.tile.y)) {
                queue.push_back((next, next_path));
            }
        }
    }
    panic!("no collision path from {} to warp {target_map}", start.map.name)
}

fn collision_path_to_tile(
    start: &crate::core::world::session::OverworldSession,
    target: TilePosition,
) -> Vec<Direction> {
    let mut queue = std::collections::VecDeque::from([(start.clone(), Vec::new())]);
    let mut visited = std::collections::BTreeSet::from([(start.player.tile.x, start.player.tile.y)]);
    while let Some((session, path)) = queue.pop_front() {
        if session.player.tile == target {
            return path;
        }
        for direction in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
            let mut next = session.clone();
            let mut step = next
                .step_and_check_warp_checked(direction, crate::core::world::movement::StepOptions::default())
                .expect("probe compiled collision");
            if matches!(step.outcome, crate::core::world::movement::StepOutcome::Turned { .. }) {
                step = next
                    .step_and_check_warp_checked(direction, crate::core::world::movement::StepOptions::default())
                    .expect("probe compiled collision after turn");
            }
            if matches!(step.outcome, crate::core::world::movement::StepOutcome::Moved { .. })
                && step.warp.is_none()
                && visited.insert((next.player.tile.x, next.player.tile.y))
            {
                let mut next_path = path.clone();
                next_path.push(direction);
                queue.push_back((next, next_path));
            }
        }
    }
    panic!("no collision path from {} to {target:?}", start.map.name)
}

fn press_visible_direction_until_tile_changes(app: &mut App, direction: Direction) {
    let key = match direction {
        Direction::Up => KeyCode::ArrowUp,
        Direction::Down => KeyCode::ArrowDown,
        Direction::Left => KeyCode::ArrowLeft,
        Direction::Right => KeyCode::ArrowRight,
    };
    let before = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("pre-step snapshot")
        .overworld;
    for _ in 0..3 {
        press_key_for_runtime_hotkey_app(app, key);
        for _ in 0..9 {
            app.update();
        }
        let after = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .expect("post-step snapshot")
            .overworld;
        if after.map_name != before.map_name || after.tile != before.tile {
            return;
        }
    }
}

fn settle_visible_story_boundary(app: &mut App) -> (bool, bool) {
    let mut rendered_text = false;
    let mut rendered_picture = false;
    // This is a budget for real 60 Hz frames, including typewriter frames,
    // scripted walks, prompt release frames, and every multi-page Elm line.
    // A small loop can time out while the game is still progressing and then
    // falsely report a movement lock.
    for _ in 0..2048 {
        app.update();
        let (busy, has_text, has_picture, party_nonempty) = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("story-boundary snapshot");
            (
                shell.active_script_cursor.is_some()
                    || shell.visible_script_movement.is_some()
                    || shell.visible_overworld_emote.is_some()
                    || shell.shell.has_pending_script_work()
                    || snapshot.ui.text_window_open
                    || snapshot.ui.active_pokemon_picture.is_some(),
                snapshot.ui.text_window_open,
                snapshot.ui.active_pokemon_picture.is_some(),
                !snapshot.party.slots.is_empty(),
            )
        };
        if has_text {
            let world = app.world_mut();
            rendered_text |= world
                .query_filtered::<Entity, With<DialogGlyphMarker>>()
                .iter(world)
                .next()
                .is_some();
        }
        if has_picture {
            let world = app.world_mut();
            rendered_picture |= world
                .query_filtered::<Entity, With<PokemonPictureMarker>>()
                .iter(world)
                .next()
                .is_some();
        }
        if !busy {
            return (rendered_text, rendered_picture);
        }
        let pending_yes_no = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .ui
            .pending_yes_no
            .is_some();
        let key = if pending_yes_no && party_nonempty {
            KeyCode::KeyX
        } else {
            KeyCode::KeyZ
        };
        press_key_for_runtime_hotkey_app(app, key);
    }
    let a_action = {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        has_visible_shell_a_action(&mut shell)
    };
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("timed-out story snapshot");
    panic!(
        "visible story boundary did not return control; map={} tile={:?} pending={:?} a_action={:?} reveal={:?} movement={:?} player_walk={} object_walk={} cursor={:?} ui={:?} action={:?}",
        snapshot.overworld.map_name,
        snapshot.overworld.tile,
        shell.shell.pending_script_work_reason(),
        a_action,
        shell.field_text_reveal,
        shell.visible_script_movement,
        shell.player_walk_frame_ticks,
        shell.object_walk_frame_ticks,
        shell.active_script_cursor,
        snapshot.ui,
        shell.last_runtime_action
    )
}

#[test]
fn visible_overworld_normal_inputs_exit_house_and_trigger_new_bark_teacher_stop() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &[
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
            vec![GameButton::Left],
        ],
        None,
    )
    .expect("normal inputs exit the house and trigger New Bark's teacher stop");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(
        smoke.final_map,
        "NewBarkTown",
        "house route stopped at ({}, {}) scene={:?} warps={} coord_events={} events={:?}",
        smoke.final_tile_x,
        smoke.final_tile_y,
        smoke.final_scene,
        smoke.warps,
        smoke.coord_events,
        smoke.frame_events
    );
    assert_eq!(
        (smoke.final_tile_x, smoke.final_tile_y),
        (5, 8),
        "normal-input route ended with events {:?}",
        smoke.frame_events
    );
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_NEWBARKTOWN_TEACHER_STOPS_YOU")
    );
    assert_eq!(smoke.warps, 2);
    assert_eq!(
        smoke.coord_events, 2,
        "teacher coordinate event did not fire: {:?}",
        smoke.frame_events
    );
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("warp=true"))
    );
    assert_eq!(
        smoke
            .frame_events
            .iter()
            .filter(|event| event.contains("coord=true"))
            .count(),
        2
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag ENGINE_FLYPOINT_NEW_BARK=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("coord event script=NewBarkTown_TeacherStopsYouScene2"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("Text_ItsDangerousToGoAlone"))
    );
    assert!(smoke.audio_events.iter().any(|event| {
        event.contains(
            "script movement NEWBARKTOWN_TEACHER NewBarkTown_TeacherBringsYouBackMovement2",
        )
    }));
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("consumed runtime flag MapMusicRequested"))
    );
}

#[test]
fn elms_lab_callback_places_elm_at_asm_intro_position() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab");
    let snapshot = runtime_shell.shell.snapshot().expect("snapshot Elm's Lab");
    assert_eq!(
        snapshot.visible_object_runtime_tiles.get("ELMSLAB_ELM"),
        Some(&TilePosition { x: 3, y: 4 }),
        "ElmsLabMoveElmCallback must apply moveobject ELMSLAB_ELM, 3, 4 before the intro scene"
    );
    // Keep the cache path exercised here too: the callback is a gameplay
    // mutation, not a renderer-only correction.
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let cached = cached_runtime_snapshot(&mut runtime_shell).expect("cached callback snapshot");
    assert_eq!(
        cached.visible_object_runtime_tiles.get("ELMSLAB_ELM"),
        Some(&TilePosition { x: 3, y: 4 })
    );
}

#[test]
fn adjacent_scripted_npc_dispatches_when_the_player_faces_it() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "ElmsLab".to_string(),
            tile_x: 4,
            tile_y: 4,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab beside scripted NPC");

    // The NPC occupies (3, 4). A blocked Left press turns the player
    // toward it; the subsequent A must dispatch the object's compiled
    // script instead of falling through to a no-interaction frame.
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Left])
        .expect("face adjacent scripted NPC");
    let faced = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot after facing adjacent NPC");
    assert_eq!(
        faced.overworld.facing,
        Direction::Left,
        "blocked direction must turn the player before A routing; script_events={:?} cursor={:?}",
        faced.script_events,
        runtime_shell.active_script_cursor
    );
    assert!(
        runtime_shell
            .shell
            .current_overworld_interaction_checked()
            .expect("resolve faced NPC interaction")
            .is_some(),
        "faced NPC must be visible to authoritative interaction lookup"
    );
    let outcome = apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("interact with adjacent scripted NPC");
    assert!(outcome.interaction, "facing NPC must receive A interaction");
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("interaction script=ProfElmScript")),
        "interaction must dispatch the object event's compiled script"
    );
}

#[test]
fn visible_overworld_normal_inputs_enter_elms_lab_and_choose_cyndaquil() {
    fn push_frames(frames: &mut Vec<Vec<GameButton>>, button: GameButton, count: usize) {
        frames.extend(std::iter::repeat_n(vec![button], count));
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut input_frames = Vec::new();
    push_frames(&mut input_frames, GameButton::Right, 6);
    push_frames(&mut input_frames, GameButton::Up, 4);
    push_frames(&mut input_frames, GameButton::Down, 5);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 4);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 5);
    push_frames(&mut input_frames, GameButton::Down, 2);
    push_frames(&mut input_frames, GameButton::Left, 9);
    push_frames(&mut input_frames, GameButton::Left, 4);
    push_frames(&mut input_frames, GameButton::Up, 5);
    push_frames(&mut input_frames, GameButton::Right, 2);
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Right, 2);
    push_frames(&mut input_frames, GameButton::Up, 9);
    push_frames(&mut input_frames, GameButton::Right, 1);
    push_frames(&mut input_frames, GameButton::A, 1);

    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &input_frames,
        None,
    )
    .expect("normal inputs enter Elm's lab and choose Cyndaquil");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(smoke.final_map, "ElmsLab");
    assert_eq!(
        (smoke.final_tile_x, smoke.final_tile_y),
        (5, 3),
        "starter route events were {:?}",
        smoke.frame_events
    );
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_ELMSLAB_AIDE_GIVES_POTION"),
        "starter route party={:?} events={:?}",
        smoke.final_party_species,
        smoke.frame_events
    );
    assert_eq!(smoke.warps, 3);
    assert_eq!(smoke.coord_events, 2);
    assert_eq!(smoke.interactions, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_PROF_ELM"));
    assert!(smoke.pending_audio > 0);
    assert_eq!(smoke.final_party_species, vec!["CYNDAQUIL"]);
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("interaction script=CyndaquilPokeBallScript"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("CRY_CYNDAQUIL"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("scripted_gift_pokemon_granted"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag EVENT_GOT_CYNDAQUIL_FROM_ELM=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag EVENT_GOT_A_POKEMON_FROM_ELM=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script runtime addcellnum args=[\"PHONE_ELM\"]"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script scene setmapscene SCENE_NEWBARKTOWN_NOOP"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script scene setscene SCENE_ELMSLAB_AIDE_GIVES_POTION"))
    );
}

#[test]
fn visible_overworld_normal_inputs_get_aide_potion_and_exit_elms_lab() {
    fn push_frames(frames: &mut Vec<Vec<GameButton>>, button: GameButton, count: usize) {
        frames.extend(std::iter::repeat_n(vec![button], count));
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut input_frames = Vec::new();
    push_frames(&mut input_frames, GameButton::Right, 6);
    push_frames(&mut input_frames, GameButton::Up, 4);
    push_frames(&mut input_frames, GameButton::Down, 5);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 4);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 5);
    push_frames(&mut input_frames, GameButton::Down, 2);
    push_frames(&mut input_frames, GameButton::Left, 9);
    push_frames(&mut input_frames, GameButton::Left, 4);
    push_frames(&mut input_frames, GameButton::Up, 5);
    push_frames(&mut input_frames, GameButton::Right, 2);
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Right, 2);
    push_frames(&mut input_frames, GameButton::Up, 9);
    push_frames(&mut input_frames, GameButton::Right, 1);
    push_frames(&mut input_frames, GameButton::A, 1);
    push_frames(&mut input_frames, GameButton::Down, 6);
    push_frames(&mut input_frames, GameButton::Down, 3);

    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &input_frames,
        None,
    )
    .expect("normal inputs get the aide Potion and exit Elm's lab");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(smoke.final_map, "NewBarkTown");
    assert_eq!(
        (smoke.final_tile_x, smoke.final_tile_y),
        (6, 3),
        "aide route events were {:?}",
        smoke.frame_events
    );
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_NEWBARKTOWN_NOOP"),
        "aide route party={:?} events={:?}",
        smoke.final_party_species,
        smoke.frame_events
    );
    assert_eq!(smoke.warps, 4);
    assert_eq!(smoke.coord_events, 3);
    assert_eq!(smoke.interactions, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert_eq!(smoke.final_party_species, vec!["CYNDAQUIL"]);
    assert_eq!(
        smoke
            .final_bag_items
            .iter()
            .find(|item| item.item_id == "POTION")
            .map(|item| item.quantity),
        Some(1)
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("coord event script=AideScript_WalkPotion2"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script item grant Granted { item_id: \"POTION\""))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("AideText_GiveYouPotion"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script scene setscene SCENE_ELMSLAB_NOOP"))
    );
    assert!(
        smoke.frame_events.iter().any(|event| {
            event.contains(":[Down]:ElmsLab@(5,8)") && event.contains("coord=true")
        })
    );
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains(":[Down]:NewBarkTown@(6,3)"))
    );
}
