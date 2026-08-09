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
        "Mom's dialogue must open after her walk finishes"
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
