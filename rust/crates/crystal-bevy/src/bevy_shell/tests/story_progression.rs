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

    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    // Follow the same keyboard/update path available to a player and let the
    // authored stair fade finish before stepping onto Mom's coordinate event.
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
    for _ in 0..4 {
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
        for _ in 0..8 {
            app.update();
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for _ in 0..16 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .visible_overworld_emote
            .is_some()
        {
            break;
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::ArrowDown);

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
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
    let emote_frames = runtime_shell
        .visible_overworld_emote
        .as_ref()
        .map(|emote| emote.frames_remaining)
        .expect("the real input path must trigger Mom's emote");
    assert_eq!(
        emote_frames, 14,
        "the first rendered frame consumes one of showemote's 15 authored frames"
    );

    for frame in 0..emote_frames.saturating_sub(1) {
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
    let mut dialogue_activations = Vec::new();
    let mut previous_visible_label = None;
    let mut saw_rendered_mom_text = false;
    let mut saw_rendered_yes_no = false;
    let mut saw_canonical_day_selector = false;
    let mut saw_dynamic_dst_confirmation = false;
    let mut longest_dst_confirmation = Vec::new();
    let mut completed_mom = false;
    let mut mom_frames = 0usize;
    let mut dialogue_mom_tile = None;
    let mut dialogue_mom_render_position = None;
    let mut yes_no_boundaries = Vec::new();
    let mut yes_no_render_trace = Vec::new();
    let mut was_pending_yes_no = false;
    let mut saw_yes_no_prompt_cleared = false;
    let mut saw_yes_no_frame_cleared = false;
    let baseline_live_entities = app.world().entities().len();
    let mut peak_live_entities = baseline_live_entities;
    let mut previous_progress_signature = None;
    let mut previous_semantic_trace = None;
    let mut stationary_script_frames = 0usize;
    let mut saw_dst_question = false;
    let mut saw_final_phone_text = false;
    let mut final_phone_text_closed = false;
    let mut saw_authored_mom_return = false;
    for frame in 0..1024 {
        mom_frames = frame + 1;
        app.update();
        peak_live_entities = peak_live_entities.max(app.world().entities().len());
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(shell.last_error, None, "Mom lifecycle failed: {:?}", shell.last_audio_events);
        let snapshot = shell.shell.snapshot().expect("Mom lifecycle snapshot");
        let progress_signature = format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            shell.active_script_cursor,
            snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
            shell.field_text_reveal.as_ref().map(|reveal| (
                reveal.page_index,
                reveal.visible_chars,
                reveal.frames_until_next_char,
            )),
            snapshot
                .ui
                .pending_yes_no
                .as_ref()
                .and_then(|_| shell.yes_no_cursor.as_ref().map(|cursor| cursor.option_index)),
            shell.visible_script_delay_frames,
            shell.visible_script_movement,
            shell.pending_day_of_week,
        );
        let semantic_trace = format!(
            "cursor={:?} label={:?} page={:?} yes_no={} special={:?} script_value={:?} day_prompt={:?}",
            shell.active_script_cursor,
            snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
            shell.field_text_reveal.as_ref().map(|reveal| reveal.page_index),
            snapshot.ui.pending_yes_no.is_some(),
            snapshot.script_events.last_special_routine,
            snapshot.script_events.script_value,
            shell.pending_day_of_week.as_ref().map(|prompt| (
                prompt.selected_day,
                prompt.confirming,
                prompt.yes_no_index,
            )),
        );
        if previous_semantic_trace.as_ref() != Some(&semantic_trace) {
            eprintln!("mom_dialogue_trace frame={frame} {semantic_trace}");
            previous_semantic_trace = Some(semantic_trace);
        }
        if previous_progress_signature.as_ref() == Some(&progress_signature) {
            stationary_script_frames += 1;
        } else {
            previous_progress_signature = Some(progress_signature.clone());
            stationary_script_frames = 0;
        }
        assert!(
            stationary_script_frames < 120,
            "Mom script stopped making progress for {stationary_script_frames} frames: {progress_signature}"
        );
        let visible_label = snapshot.ui.text.as_ref().map(|text| text.label.clone());
        saw_dst_question |= visible_label.as_deref() == Some("IsItDSTText");
        saw_final_phone_text |= visible_label.as_deref() == Some("InstructionsNextText");
        final_phone_text_closed |= saw_final_phone_text
            && visible_label.is_none()
            && !snapshot.ui.text_window_open
            && snapshot.ui.pending_yes_no.is_none()
            && shell.active_script_cursor.as_ref().is_some_and(|cursor| {
                cursor.source_script != ".FinishPhone@MeetMomScript"
                    || cursor.next_command_index >= 3
            });
        let mom_movement_active = shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| movement.object_id == "PLAYERSHOUSE1F_MOM1");
        assert!(
            !(saw_dst_question && !final_phone_text_closed && mom_movement_active),
            "Mom began her return movement before the complete date/DST/phone dialogue closed; label={visible_label:?} cursor={:?}",
            shell.active_script_cursor
        );
        saw_authored_mom_return |= final_phone_text_closed && mom_movement_active;
        if visible_label != previous_visible_label {
            if let Some(label) = visible_label.clone() {
                dialogue_activations.push(label);
            }
            previous_visible_label = visible_label.clone();
        }
        let pending_yes_no = snapshot.ui.pending_yes_no.is_some();
        if was_pending_yes_no && !pending_yes_no {
            saw_yes_no_prompt_cleared = !app
                .world()
                .iter_entities()
                .any(|entity| entity.contains::<YesNoPromptMarker>());
            let (prompt_left_x, prompt_top_y) = battle_hud_tile_origin(
                FIELD_YES_NO_LEFT_TILE,
                FIELD_YES_NO_TOP_TILE,
            );
            saw_yes_no_frame_cleared = !app.world().iter_entities().any(|entity| {
                entity.contains::<SceneDialogWindowFrameMarker>()
                    && entity.get::<Transform>().is_some_and(|transform| {
                        (transform.translation.x - prompt_left_x).abs() < f32::EPSILON
                            && (transform.translation.y - prompt_top_y).abs() < f32::EPSILON
                    })
            });
        }
        was_pending_yes_no = pending_yes_no;
        if pending_yes_no {
            let boundary = format!("{:?}", shell.active_script_cursor);
            if yes_no_boundaries.last() != Some(&boundary) {
                yes_no_boundaries.push(boundary);
                yes_no_render_trace.push((
                    snapshot.script_events.last_special_routine.clone(),
                    visible_scene_dialog_entries(&snapshot, shell)
                        .expect("render Mom yes/no trace"),
                ));
            }
        }
        if snapshot.ui.text_window_open
            || pending_yes_no
            || shell.pending_day_of_week.is_some()
        {
            let mom_tile = snapshot
                .visible_object_runtime_tiles
                .get("PLAYERSHOUSE1F_MOM1")
                .copied()
                .expect("Mom must have a live runtime tile during her dialogue");
            let expected = *dialogue_mom_tile.get_or_insert(mom_tile);
            assert_eq!(
                mom_tile, expected,
                "Mom moved while dialogue still owned the scene; label={visible_label:?} cursor={:?}",
                shell.active_script_cursor
            );
            let rendered_dialog_visible = shell
                .visible_script_movement_scene
                .as_ref()
                .map_or(snapshot.ui.text_window_open, |scene| scene.ui.text_window_open);
            if rendered_dialog_visible
                && let Some(render_position) = app.world().iter_entities().find_map(|entity| {
                    entity
                        .get::<VisibleObjectSprite>()
                        .filter(|sprite| {
                            sprite.object_identifier.as_deref() == Some("PLAYERSHOUSE1F_MOM1")
                        })
                        .and_then(|_| entity.get::<Transform>())
                        .map(|transform| transform.translation.truncate())
                })
            {
                let expected = *dialogue_mom_render_position.get_or_insert(render_position);
                assert_eq!(
                    render_position, expected,
                    "Mom's rendered sprite moved while dialogue was visible; label={visible_label:?} cursor={:?}",
                    shell.active_script_cursor
                );
            }
        }
        if pending_yes_no
            && matches!(
                snapshot.script_events.last_special_routine.as_deref(),
                Some("InitialSetDSTFlag" | "InitialClearDSTFlag")
            )
        {
            let entries = visible_scene_dialog_entries(&snapshot, shell)
                .expect("render Mom DST confirmation entries");
            if entries.iter().map(String::len).sum::<usize>()
                > longest_dst_confirmation.iter().map(String::len).sum::<usize>()
            {
                longest_dst_confirmation = entries.clone();
            }
            saw_dynamic_dst_confirmation |= entries.iter().any(|line| line.contains(':'))
                && entries.iter().any(|line| line.contains("is that OK"));
        }
        if let Some(label) = visible_label.clone()
            && seen_labels.last() != Some(&label)
        {
            seen_labels.push(label);
        }
        let shell = app.world().resource::<BevyRuntimeShell>();
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
    assert_eq!(
        seen_labels.iter().map(String::as_str).collect::<Vec<_>>(),
        canonical_mom_labels,
        "Mom must render every ASM-authored dialogue label exactly once and in order"
    );
    assert_eq!(
        dialogue_activations
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        canonical_mom_labels,
        "Mom must not reactivate a completed text body while entering its prompt boundary"
    );
    assert_eq!(
        yes_no_boundaries.len(),
        3,
        "Mom's ASM path has exactly three YesNoBox boundaries; saw {yes_no_boundaries:?}"
    );
    assert!(
        saw_authored_mom_return,
        "Mom never performed her ASM-authored return movement after the phone dialogue closed"
    );
    let final_live_entities = app.world().entities().len();
    assert!(
        peak_live_entities < 1024 && final_live_entities <= baseline_live_entities + 32,
        "Mom dialogue leaked live render entities; baseline={baseline_live_entities} peak={peak_live_entities} final={final_live_entities}"
    );
    assert!(
        saw_canonical_day_selector,
        "SetDayOfWeek must render its separate canonical 11x4 selector window"
    );
    assert!(
        saw_dynamic_dst_confirmation,
        "DST setup must visibly render the live HH:MM confirmation before its yes/no; last_special={:?} reveal={:?}",
        failed_snapshot.script_events.last_special_routine,
        (
            failed_shell.field_text_reveal.as_ref(),
            yes_no_render_trace,
            longest_dst_confirmation
        )
    );
    assert!(saw_rendered_mom_text, "Mom executed text without rendering its textbox and bitmap glyphs");
    assert!(saw_rendered_yes_no, "Mom's yes/no executed without rendering an interactive prompt");
    assert!(
        saw_yes_no_prompt_cleared,
        "Mom's resolved YesNoBox survived into the following script frame"
    );
    assert!(
        saw_yes_no_frame_cleared,
        "Mom's resolved YesNoBox left its window frame rendered over the overworld"
    );
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
    let mut previous_label = None;
    let mut completed_labels = std::collections::BTreeSet::new();
    let mut dialogue_activations = Vec::new();
    // This is a budget for real 60 Hz frames, including typewriter frames,
    // scripted walks, prompt release frames, and every multi-page Elm line.
    // A small loop can time out while the game is still progressing and then
    // falsely report a movement lock.
    for _ in 0..2048 {
        app.update();
        let (busy, has_text, has_picture, party_nonempty, visible_label, cursor) = {
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
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                shell.active_script_cursor.clone(),
            )
        };
        if visible_label != previous_label {
            if let Some(previous) = previous_label.take() {
                completed_labels.insert(previous);
            }
            if let Some(label) = visible_label.clone() {
                assert!(
                    !completed_labels.contains(&label),
                    "compiled dialogue reactivated a completed text body {label}; activations={dialogue_activations:?} cursor={cursor:?}"
                );
                dialogue_activations.push(label);
            }
            previous_label = visible_label;
        }
        if has_text {
            let world = app.world_mut();
            rendered_text |= world
                .query_filtered::<Entity, With<DialogGlyphMarker>>()
                .iter(world)
                .next()
                .is_some();
        }
        if has_picture && !rendered_picture {
            // The script snapshot becomes authoritative before Bevy's render
            // systems consume it. Give the newly opened picture a real
            // display frame before a player A press can dismiss it.
            app.update();
            let world = app.world_mut();
            rendered_picture |= world
                .query_filtered::<Entity, With<PokemonPictureMarker>>()
                .iter(world)
                .next()
                .is_some();
            continue;
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
fn elm_dialogue_keeps_the_optional_world_frame_active_on_every_render_update() {
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
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 4,
            tile_y: 4,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab beside Elm");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.init_resource::<crystal_render_api::VisualWorldFrame>()
        .add_systems(
            Update,
            publish_visual_world_frame.after(render_playfield),
        );

    app.update();
    app.update();
    assert!(
        app.world()
            .resource::<crystal_render_api::VisualWorldFrame>()
            .active,
        "Elm's Lab must publish a valid optional-renderer frame before interaction"
    );

    // ElmsLabWalkUpToElmScript shows EMOTE_SHOCK over Elm immediately before
    // opening his first textbox. This screen-space effect used to invalidate
    // the optional world frame for its full 15-frame duration, producing the
    // visible 2.5D -> 2D -> 2.5D flicker.
    app.world_mut()
        .resource_mut::<BevyRuntimeShell>()
        .visible_overworld_emote = Some(VisibleOverworldEmote {
        emote: "EMOTE_SHOCK".to_string(),
        object: "ELMSLAB_ELM".to_string(),
        frames_remaining: 15,
    });
    app.update();
    assert!(
        app.world()
            .resource::<crystal_render_api::VisualWorldFrame>()
            .active,
        "Elm's EMOTE_SHOCK must overlay the manually selected world view without changing it"
    );

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowLeft);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    for update in 0..240 {
        app.update();
        let world_frame = app
            .world()
            .resource::<crystal_render_api::VisualWorldFrame>();
        if !world_frame.active {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("Elm dialogue snapshot");
            panic!(
                "optional world frame dropped on Elm render update {update}: text={:?} movement={:?} emote={:?} map={} tile={:?}",
                snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
                shell.visible_script_movement,
                shell.visible_overworld_emote,
                snapshot.overworld.map_name,
                snapshot.overworld.tile,
            );
        }
    }
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
    push_frames(&mut input_frames, GameButton::Up, 5);
    push_frames(&mut input_frames, GameButton::Right, 2);
    push_frames(&mut input_frames, GameButton::Up, 2);
    // NewBarkTown.asm places the Elm's Lab warp at (6, 3). The preceding
    // turn-and-step inputs leave the player at (6, 4), directly below it.
    push_frames(&mut input_frames, GameButton::Up, 9);
    // ElmsLab.asm places the Cyndaquil ball at (6, 3). From (4, 3), the
    // first Right turns, the second steps to (5, 3), and the third faces it.
    push_frames(&mut input_frames, GameButton::Right, 3);
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
    assert_eq!(
        smoke.final_map, "ElmsLab",
        "starter route stopped at ({}, {}) with events {:?}",
        smoke.final_tile_x, smoke.final_tile_y, smoke.frame_events
    );
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
    assert_eq!(smoke.interactions, 2);
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
    push_frames(&mut input_frames, GameButton::Up, 5);
    push_frames(&mut input_frames, GameButton::Right, 2);
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Up, 9);
    push_frames(&mut input_frames, GameButton::Right, 3);
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
    assert_eq!(smoke.interactions, 2);
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
