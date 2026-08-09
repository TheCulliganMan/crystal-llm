#[test]
fn integrated_title_launch_schedule_renders_menu_and_starts_music() {
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
    let title_music = runtime
        .title_music_id()
        .expect("title music id")
        .to_string();
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert!(runtime_shell.title_menu.is_some());
    assert_eq!(
        runtime_shell.active_music.as_deref(),
        Some(title_music.as_str())
    );
    assert!(
        runtime_shell.pending_audio.is_empty(),
        "launch schedule should drain title music into Bevy audio playback"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("played Music")),
        "title launch should reach Bevy audio playback"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("played SoundEffect SFX_TITLE_SCREEN_ENTRANCE")),
        "title launch should play the TypeScript/ASM title entrance sound"
    );

    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(!rendered_art.title_screen_cache.is_empty());
    assert!(rendered_art.title_screen_errors.is_empty());
    assert_eq!(rendered_art.font_error, None);
    assert_eq!(world.resource::<Assets<AudioSource>>().len(), 2);
    assert!(
        !world.resource::<Assets<Image>>().is_empty(),
        "title launch should load the composed native title/menu image"
    );

    let world = app.world_mut();
    let mut music_entities = world.query_filtered::<Entity, With<MusicAudioMarker>>();
    assert_eq!(music_entities.iter(world).count(), 1);
    let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
    assert_eq!(
        title_entities.iter(world).count(),
        1,
        "main menu phase should spawn one composed TypeScript-style menu surface"
    );
}

#[test]
fn integrated_title_to_overworld_schedule_accepts_name_renders_music_and_movement() {
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
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut app);

    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.title_menu.is_none());
        assert!(runtime_shell.pending_name_input.is_none());
        assert!(runtime_shell.pending_oak_intro.is_none());
        assert_eq!(
            runtime_shell.active_music.as_deref(),
            Some("MUSIC_NEW_BARK_TOWN")
        );
        assert!(
            runtime_shell.pending_audio.is_empty(),
            "new-game map music should be drained through the launch schedule"
        );
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event.contains("played Music MUSIC_NEW_BARK_TOWN")),
            "new-game map music should reach Bevy audio playback"
        );
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after name confirm");
        assert_eq!(snapshot.trainer.player_name, "AB");
        assert_eq!(snapshot.overworld.map_name, "PlayersHouse2F");
        // ASM's PlayersHouse2F spawn is (3,3), preserved from the
        // exporter instead of the previous approximated (2,2).
        assert_eq!(snapshot.overworld.tile, TilePosition { x: 3, y: 3 });
    }

    {
        let world = app.world();
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(!rendered_art.cache.is_empty());
        assert!(!rendered_art.sprite_cache.is_empty());
        assert!(rendered_art.errors.is_empty());
        assert!(rendered_art.sprite_errors.is_empty());
        assert!(
            world.resource::<Assets<Image>>().len() > METATILE_TILE_COUNT,
            "title-to-overworld schedule should keep real tile and sprite art loaded"
        );
    }
    {
        let world = app.world_mut();
        let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
        assert_eq!(
            title_entities.iter(world).count(),
            0,
            "title entities should be despawned after entering overworld"
        );
        let _surfaces = retained_map_surface_pair(world);
        let mut players = world.query_filtered::<Entity, With<PlayerMarker>>();
        assert_eq!(players.iter(world).count(), 1);
        assert!(
            world.resource::<RenderedViewport>().map_texture.is_some(),
            "overworld must retain a composited map texture"
        );
    }
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let text_label = runtime_shell
            .shell
            .runtime()
            .script_text_body_keys()
            .into_iter()
            .find(|key| key.map_name == "PlayersHouse2F")
            .map(|key| key.body_key)
            .expect("PlayersHouse2F script text body");
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .text_window_open = true;
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .pending_text_label = Some(text_label.clone());
        // Mutating the authoritative fixture directly bypasses the
        // normal input/mutation boundary, so explicitly invalidate the
        // renderer's semantic revision before ticking the app.
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    let dialogue_frame_before = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot before dialogue printer update")
        .state_checksum
        .frame();
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let snapshot = runtime_shell.shell.snapshot().expect("dialog snapshot");
        assert!(snapshot.ui.text.is_some());
        assert_eq!(
            snapshot.state_checksum.frame(),
            dialogue_frame_before,
            "a field textbox must not advance overworld/autonomous frames while its text prints"
        );
    }
    {
        let world = app.world_mut();
        let mut frame_tiles = world.query_filtered::<Entity, With<SceneDialogWindowFrameMarker>>();
        assert_eq!(
            frame_tiles.iter(world).count(),
            battle_window_frame_tile_count(
                FIELD_TEXT_BOX_WIDTH_TILES as usize,
                FIELD_TEXT_BOX_HEIGHT_TILES as usize,
            ),
            "runtime dialogue should render the full ASM 20x6 textbox frame, not arbitrary Rust rectangles"
        );
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(!rendered_art.window_frame_cache.is_empty());
        assert!(rendered_art.window_frame_errors.is_empty());
    }
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .text_window_open = false;
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .pending_text_label = None;
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();

    let initial_playfield_entity = {
        let world = app.world_mut();
        let mut tiles =
            world.query_filtered::<Entity, (With<PlayfieldTile>, Without<PlayfieldPriorityTile>)>();
        tiles
            .get_single(world)
            .expect("overworld should have one retained playfield entity before movement")
    };

    let start_tile = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot before movement")
        .overworld
        .tile;
    {
        let mut frames = app.world_mut().resource_mut::<HeldArrowRightTestFrames>();
        frames.0 = 6;
    }
    for _ in 0..6 {
        app.update();
    }
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert!(
        runtime_shell.last_overworld_input.is_some(),
        "held ArrowRight should be recorded after title-to-overworld transition"
    );
    let end_tile = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot after movement")
        .overworld
        .tile;
    assert!(
        end_tile.x > start_tile.x,
        "held ArrowRight should move player right from {start_tile:?} to {end_tile:?}"
    );
    assert!(
        app.world()
            .resource::<RenderedViewport>()
            .map_texture
            .is_some(),
        "map transitions must retain the composited viewport layer"
    );
    let world = app.world_mut();
    let mut tiles =
        world.query_filtered::<Entity, (With<PlayfieldTile>, Without<PlayfieldPriorityTile>)>();
    assert_eq!(
        tiles
            .get_single(world)
            .expect("overworld should retain one playfield entity after movement"),
        initial_playfield_entity,
        "walking must update the active LCD texture in place instead of exposing a blank frame between despawn and respawn"
    );
}

#[test]
fn integrated_players_house_first_floor_renders_after_bedroom_warp() {
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
            map_name: "PlayersHouse1F".to_string(),
            tile_x: 9,
            tile_y: 0,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize first-floor shell");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("first-floor snapshot");
    assert_eq!(snapshot.overworld.map_name, "PlayersHouse1F");
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<PlayerMarker>>()
            .iter(world)
            .count(),
        1,
        "first-floor arrival must render the player"
    );
    let _surfaces = retained_map_surface_pair(world);
}

#[test]
fn integrated_players_house_pc_opens_its_menu_from_the_live_compiled_pack() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "PlayersHouse2F".to_string(),
            tile_x: 2,
            tile_y: 2,
        },
        BevyShellConfig {
            smoke_player_name: Some("TEST".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize player-bedroom shell");
    runtime_shell.shell.session.overworld.player.facing = Direction::Up;

    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    app.update();
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    // The boot message owns A until it is visibly printed and dismissed.
    // Replaying actual presses here catches the live failure where the PC
    // menu appeared underneath a retained text lock and ignored navigation.
    for _ in 0..256 {
        let boot_notice_open = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            shell.field_notice.is_some() || shell.pc_notice.is_some()
        };
        if !boot_notice_open {
            break;
        }
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    }

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert!(
        runtime_shell.player_pc_action_cursor.is_some(),
        "A on the bedroom PC must open the player's PC menu; error={:?}, status={:?}, events={:?}",
        runtime_shell.last_error,
        runtime_shell.last_action_status,
        runtime_shell.last_audio_events
    );
    let initial_pc_option = runtime_shell
        .player_pc_action_cursor
        .as_ref()
        .expect("player PC cursor")
        .option_index;
    drop(runtime_shell);
    app.update();
    let rendered_before = {
        let world = app.world_mut();
        let background = world
            .query_filtered::<&Sprite, With<SceneDialogTextBoxBackgroundMarker>>()
            .iter(world)
            .next()
            .expect("Player PC must render its canonical window paper");
        assert_eq!(
            background.custom_size,
            Some(Vec2::new(14.0 * TILE_SIZE, 11.0 * TILE_SIZE)),
            "TypeScript/ASM Player PC action window must be 16x13 tiles"
        );
        let mut glyphs = world
            .query::<(&DialogGlyphMarker, &Handle<Image>)>()
            .iter(world)
            .map(|(marker, texture)| (marker.key, format!("{:?}", texture.id())))
            .collect::<Vec<_>>();
        glyphs.sort();
        glyphs
    };
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    app.update();
    assert_ne!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .player_pc_action_cursor
            .as_ref()
            .expect("player PC cursor after navigation")
            .option_index,
        initial_pc_option,
        "the rendered PC menu must accept navigation input"
    );
    let rendered_after = {
        let world = app.world_mut();
        let mut glyphs = world
            .query::<(&DialogGlyphMarker, &Handle<Image>)>()
            .iter(world)
            .map(|(marker, texture)| (marker.key, format!("{:?}", texture.id())))
            .collect::<Vec<_>>();
        glyphs.sort();
        glyphs
    };
    assert_ne!(
        rendered_after, rendered_before,
        "Down must visibly move the Player PC arrow, not only mutate an internal cursor"
    );

    // Return to WITHDRAW, enter the real nested item surface, and back out.
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowUp);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    for _ in 0..256 {
        let shell = app.world().resource::<BevyRuntimeShell>();
        if shell.pc_item_cursor.is_some() || shell.pc_notice.is_some() {
            break;
        }
        let _ = shell;
        app.update();
    }
    assert!(
        app.world().resource::<BevyRuntimeShell>().pc_item_cursor.is_some()
            || app.world().resource::<BevyRuntimeShell>().pc_notice.is_some(),
        "WITHDRAW must visibly enter the item list or display the canonical empty notice"
    );
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyX);
    for _ in 0..8 {
        app.update();
    }
    if app.world().resource::<BevyRuntimeShell>().pc_notice.is_some() {
        for _ in 0..256 {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
            if app.world().resource::<BevyRuntimeShell>().pc_notice.is_none() {
                break;
            }
        }
    }
    assert!(
        app.world().resource::<BevyRuntimeShell>().player_pc_action_cursor.is_some(),
        "B from a nested Player PC surface must return to the six-action menu"
    );
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyX);
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .player_pc_action_cursor
            .is_none(),
        "B must turn off the PC and close its action UI"
    );
    assert_overworld_control_returns_and_player_moves(&mut app, "PlayersHousePCScript");
}

#[test]
fn integrated_players_house_bookshelf_renders_dialogue_from_the_live_compiled_pack() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    runtime
        .data()
        .script_text_command("PlayersHouse2F", "PictureBookshelfScript", 0)
        .expect("live pack must materialize the standard bookshelf text command");
    assert!(
        runtime.data().asm_text.contains_key("PictureBookshelfText"),
        "live pack must contain the standard bookshelf's canonical ASM text"
    );
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "PlayersHouse2F".to_string(),
            tile_x: 5,
            tile_y: 2,
        },
        BevyShellConfig {
            smoke_player_name: Some("TEST".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize player-bedroom shell");
    runtime_shell.shell.session.overworld.player.facing = Direction::Up;
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    app.update();
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(
        runtime_shell.last_error, None,
        "bookshelf interaction failed: action={:?} events={:?}",
        runtime_shell.last_runtime_action, runtime_shell.last_audio_events
    );
    let snapshot = runtime_shell.shell.snapshot().expect("bookshelf snapshot");
    assert_eq!(
        snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
        Some("PictureBookshelfText")
    );
    let world = app.world_mut();
    let background = world
        .query_filtered::<&Sprite, With<SceneDialogTextBoxBackgroundMarker>>()
        .iter(world)
        .next()
        .expect("bookshelf dialogue must spawn a rendered textbox entity");
    assert_eq!(
        background.color,
        Color::WHITE,
        "field dialogue must use TypeScript's first textbox palette color"
    );
    assert!(
        world
            .query_filtered::<Entity, With<DialogGlyphMarker>>()
            .iter(world)
            .count()
            > 0,
        "bookshelf text must render glyph entities, not only mutate runtime state"
    );
    for _ in 0..128 {
        app.update();
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    for _ in 0..3 {
        app.update();
    }
    let close_diagnostic = {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        format!(
            "snapshot={:?} action={:?} cursor={:?} events={:?}",
            runtime_shell.shell.snapshot().ok().map(|snapshot| snapshot.ui),
            runtime_shell.last_runtime_action,
            runtime_shell.active_script_cursor,
            runtime_shell.last_audio_events
        )
    };
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<SceneDialogTextBoxBackgroundMarker>>()
            .iter(world)
            .count(),
        0,
        "acknowledging the bookshelf text must close its rendered textbox: {close_diagnostic}"
    );
    assert_overworld_control_returns_and_player_moves(&mut app, "PictureBookshelfScript");
}

fn assert_overworld_control_returns_and_player_moves(app: &mut App, script: &str) {
    {
        let shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = shell.shell.snapshot().expect("post-interaction snapshot");
        assert_eq!(shell.last_error, None, "{script} left a runtime error");
        assert_eq!(
            shell.active_script_cursor,
            None,
            "{script} left its script cursor armed; ui={:?} action={:?} events={:?}",
            snapshot.ui,
            shell.last_runtime_action,
            shell.last_audio_events
        );
        assert!(!snapshot.ui.text_window_open, "{script} left its textbox open");
        assert!(snapshot.ui.pending_text_wait.is_none(), "{script} left a text wait pending");
    }
    let start = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .unwrap()
        .overworld
        .tile;
    for key in [KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp] {
        // Crystal turns toward a newly pressed direction first. A second
        // press performs the walk, exactly like the real joypad path.
        for _ in 0..2 {
            press_key_for_runtime_hotkey_app(app, key);
            for _ in 0..6 {
                app.update();
            }
        }
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(shell.last_error, None, "{script} errored when movement resumed");
        if shell.shell.snapshot().unwrap().overworld.tile != start {
            return;
        }
    }
    let shell = app.world().resource::<BevyRuntimeShell>();
    panic!(
        "{script} finished visually but never returned movement control from {start:?}; action={:?} input={:?} cursor={:?} events={:?}",
        shell.last_runtime_action,
        shell.last_overworld_input,
        shell.active_script_cursor,
        shell.last_audio_events
    );
}

fn find_live_standard_script_approach(
    runtime: &CrystalRuntime,
    expected_script: &str,
) -> (String, TilePosition, Direction) {
    let mut found_scripts = std::collections::BTreeSet::new();
    for map_name in ["PlayersHouse2F", "PlayersHouse1F", "ElmsLab"] {
        if !runtime.data().maps.contains_key(map_name) {
            continue;
        }
        let module = runtime.data().map_module(map_name).unwrap();
        let width = i16::try_from(module.attributes.width).unwrap() * 2;
        let height = i16::try_from(module.attributes.height).unwrap() * 2;
        for y in 0..height {
            for x in 0..width {
                let tile = TilePosition::new(x, y);
                let Ok(mut session) = runtime.data().overworld_session(map_name, tile, 0) else { continue };
                for facing in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
                    session.player.facing = facing;
                if let Some(interaction) = session
                    .check_interaction_checked(1)
                    .expect("scan live collision interaction")
                {
                    found_scripts.insert(interaction.script.clone());
                    if interaction.script != expected_script {
                        continue;
                    }
                    return (map_name.to_string(), tile, facing);
                }
            }
        }
    }
    }
    panic!("live pack has no walkable collision interaction for {expected_script}; found {found_scripts:?}");
}

#[test]
fn integrated_house_tv_map_and_radio_render_and_progress_from_live_collision_scripts() {
    for (script, initial_label, expected_page) in [
        ("TVScript", Some("TVText"), None),
        (
            "PlayersHousePosterScript",
            Some("LookTownMapText"),
            Some(PokegearPage::Map),
        ),
        ("PlayersHouseRadioScript", Some("PlayersRadioText1"), None),
    ] {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let runtime = workspace_desktop_runtime(&asset_root);
        let spawn_identifier = runtime.title_new_game_spawn_identifier().expect("new-game spawn");
        let (map_name, interaction_tile, interaction_facing) =
            find_live_standard_script_approach(&runtime, script);
        let mut runtime_shell = initialize_bevy_runtime_shell(
            asset_root,
            runtime,
            BevyShellStart::NewGameAtRuntimeTile {
                spawn_identifier,
                map_name,
                tile_x: interaction_tile.x,
                tile_y: interaction_tile.y,
            },
            BevyShellConfig { smoke_player_name: Some("TEST".to_string()), ..Default::default() },
        )
        .expect("standard interaction must be reachable from a walkable live tile");
        runtime_shell.shell.session.overworld.player.facing = interaction_facing;
        let mut app = integrated_shell_test_app(runtime_shell);
        app.update();
        app.update();
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

        if let Some(label) = initial_label {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(runtime_shell.last_error, None, "{script} failed");
            assert_eq!(
                runtime_shell.shell.snapshot().unwrap().ui.text.as_ref().map(|text| text.label.as_str()),
                Some(label),
                "{script} must expose its canonical text; cursor={:?} overrides={:?} blocks={:?} events={:?}",
                runtime_shell.active_script_cursor,
                runtime_shell.shell.session.state.map_block_overrides.get("PlayersHouse2F"),
                &runtime_shell.shell.session.overworld.map.metatile_ids[..4],
                runtime_shell.last_audio_events,
            );
            let world = app.world_mut();
            assert!(world.query_filtered::<Entity, With<SceneDialogTextBoxBackgroundMarker>>().iter(world).next().is_some(), "{script} must render a textbox");
            assert!(world.query_filtered::<Entity, With<DialogGlyphMarker>>().iter(world).next().is_some(), "{script} must render glyphs");
            if script == "PlayersHouseRadioScript" {
                let mut labels = Vec::new();
                for _ in 0..4096 {
                    app.update();
                    let snapshot = app
                        .world()
                        .resource::<BevyRuntimeShell>()
                        .shell
                        .snapshot()
                        .unwrap();
                    if let Some(label) = snapshot.ui.text.as_ref().map(|text| text.label.clone())
                        && labels.last() != Some(&label)
                    {
                        labels.push(label);
                    }
                    if !snapshot.ui.text_window_open {
                        break;
                    }
                }
                assert_eq!(
                    labels,
                    ["PlayersRadioText1", "PlayersRadioText2", "PlayersRadioText3", "PlayersRadioText4"],
                    "the initial radio broadcast must display Oak, Mary, and the closing segment in canonical order; cursor={:?} delay={:?} reveal={:?} ui={:?} events={:?}",
                    app.world().resource::<BevyRuntimeShell>().active_script_cursor,
                    app.world().resource::<BevyRuntimeShell>().visible_script_delay_frames,
                    app.world().resource::<BevyRuntimeShell>().field_text_reveal,
                    app.world().resource::<BevyRuntimeShell>().shell.snapshot().unwrap().ui,
                    app.world().resource::<BevyRuntimeShell>().last_audio_events,
                );
            } else {
                for _ in 0..512 {
                    let finished = {
                        let shell = app.world().resource::<BevyRuntimeShell>();
                        expected_page.is_some_and(|page| {
                            shell.pokegear_menu_open && shell.pokegear_page == page
                        }) || (expected_page.is_none()
                            && !shell.shell.snapshot().unwrap().ui.text_window_open)
                    };
                    if finished {
                        break;
                    }
                    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
                    app.update();
                }
            }
        }

        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None, "{script} progression failed");
        if let Some(page) = expected_page {
            assert!(
                runtime_shell.pokegear_menu_open,
                "{script} must open its modal UI; ui={:?} action={:?} events={:?}",
                runtime_shell.shell.snapshot().unwrap().ui,
                runtime_shell.last_runtime_action,
                runtime_shell.last_audio_events
            );
            assert_eq!(runtime_shell.pokegear_page, page, "{script} opened the wrong UI page");
            drop(runtime_shell);
            app.update();
            assert_eq!(
                app.world().resource::<BevyRuntimeShell>().last_error,
                None,
                "{script} modal render failed"
            );
            let world = app.world_mut();
            let glyph_sprites = world
                .query::<(&Sprite, &Transform)>()
                .iter(world)
                .filter(|(sprite, transform)| {
                    transform.translation.z >= 3.8 && sprite.custom_size.is_some()
                })
                .count();
            assert!(glyph_sprites > 0, "{script} modal must render bitmap glyph sprites");
            let _ = world;
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyX);
            for _ in 0..3 {
                app.update();
            }
        } else {
            assert!(!runtime_shell.shell.snapshot().unwrap().ui.text_window_open, "{script} dialogue must close after acknowledgement");
        }
        assert_overworld_control_returns_and_player_moves(&mut app, script);
    }
}

#[test]
fn integrated_title_to_start_menu_schedule_renders_and_selects_with_live_keys() {
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
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut app);

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert_eq!(
            visible_start_menu_entries(runtime_shell).expect("start menu entries"),
            vec![">PACK", " AB", " OPTION", " EXIT"]
        );
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("START MENU")
        );
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        assert!(
            field_commands.iter(world).count() > 2,
            "live Start key should render the start menu with bitmap glyph sprites"
        );
        let mut field_command_frame_tiles =
            world.query_filtered::<Entity, With<FieldCommandWindowFrameMarker>>();
        assert_eq!(
            field_command_frame_tiles.iter(world).count(),
            battle_window_frame_tile_count(
                (START_MENU_RIGHT_TILE - START_MENU_LEFT_TILE + 1.0) as usize,
                START_MENU_MIN_HEIGHT_TILES.max(4.0 + 2.0) as usize,
            ),
            "live Start key should render the TypeScript/ASM start menu frame"
        );
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(rendered_art.font_cache.is_some());
        assert_eq!(rendered_art.font_error, None);
        assert!(!rendered_art.window_frame_cache.is_empty());
        assert!(rendered_art.window_frame_errors.is_empty());
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert_eq!(
            visible_start_menu_entries(runtime_shell).expect("moved start menu entries"),
            vec![" PACK", ">AB", " OPTION", " EXIT"]
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.start_menu_cursor.is_none());
        assert!(runtime_shell.trainer_card_open);
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("TRAINER CARD")
        );
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        let field_command_count = field_commands.iter(world).count();
        let rendered_art = world.resource::<RenderedTilesetArt>();
        let trainer_card_error_report = rendered_art
            .trainer_card_errors
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        let trainer_card_cache_len = rendered_art.trainer_card_cache.len();
        assert_eq!(
            field_command_count, 1,
            "live menu selection should render the composited Trainer Card screen; cache={trainer_card_cache_len} errors={trainer_card_error_report}"
        );
        assert_eq!(rendered_art.trainer_card_errors.len(), 0);
        assert_eq!(rendered_art.trainer_card_cache.len(), 1);
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.trainer_card_open);
        assert_eq!(
            runtime_shell.trainer_card_page,
            VisibleTrainerCardPage::JohtoBadges
        );
    }
    {
        let rendered_art = app.world().resource::<RenderedTilesetArt>();
        assert_eq!(rendered_art.trainer_card_errors.len(), 0);
        assert!(
            rendered_art.trainer_card_cache.len() >= 2,
            "Trainer Card A should render the source Johto badge page"
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyX);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(!runtime_shell.trainer_card_open);
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        assert_eq!(
            field_commands.iter(world).count(),
            0,
            "closing Trainer Card should remove the rendered field-command panel"
        );
    }
}

#[test]
fn integrated_title_option_entry_opens_options_before_new_game() {
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
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let title = runtime_shell
            .title_menu
            .as_ref()
            .expect("title menu should be open");
        assert_eq!(
            visible_title_menu_entries(runtime_shell, title).expect("title entries"),
            vec![">NEW GAME", " OPTION"]
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.title_menu.is_some());
        assert!(runtime_shell.options_menu_open);
        assert_eq!(runtime_shell.last_action_status.as_deref(), Some("OPTIONS"));
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("title Options snapshot");
        assert_eq!(
            visible_options_menu_entries(&snapshot, runtime_shell).first(),
            Some(&format!(
                ">TEXT SPEED: {}",
                option_value_for_item(&snapshot.trainer.options, OptionsMenuItem::TextSpeed)
            ))
        );
        let entries = visible_options_menu_entries(&snapshot, runtime_shell);
        assert!(
            !entries
                .iter()
                .any(|entry| entry.contains("NO TEXT SCROLL") || entry.contains("No Text Scroll")),
            "Options menu must not expose non-ASM no_text_scroll as a visible row"
        );
        assert_eq!(
            OPTIONS_MENU_ITEMS
                .iter()
                .map(|item| options_menu_item_label(*item))
                .collect::<Vec<_>>(),
            vec![
                "TEXT SPEED",
                "BATTLE SCENE",
                "BATTLE STYLE",
                "SOUND",
                "PRINT",
                "MENU ACCOUNT",
                "FRAME",
                "CANCEL",
            ]
        );
    }
    {
        let world = app.world_mut();
        let mut options_frame_tiles =
            world.query_filtered::<Entity, With<FieldCommandWindowFrameMarker>>();
        assert_eq!(
            options_frame_tiles.iter(world).count(),
            battle_window_frame_tile_count(OPTIONS_MENU_WIDTH_TILES, OPTIONS_MENU_HEIGHT_TILES),
            "title Options should render the full-screen TypeScript/ASM options frame"
        );
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(rendered_art.font_cache.is_some());
        assert_eq!(rendered_art.font_error, None);
        assert!(!rendered_art.window_frame_cache.is_empty());
        assert!(rendered_art.window_frame_errors.is_empty());
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    let before = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot before title Options change")
        .trainer
        .options
        .battle_scene;
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.options_menu_open);
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after title Options change");
        assert_ne!(snapshot.trainer.options.battle_scene, before);
        assert!(
            visible_options_menu_entries(&snapshot, runtime_shell)
                .iter()
                .any(|entry| entry
                    == &format!(
                        ">BATTLE SCENE: {}",
                        option_value_for_item(
                            &snapshot.trainer.options,
                            OptionsMenuItem::BattleScene
                        )
                    )),
            "title Options should keep live Options cursor focus"
        );
        let mut overlay_lines = Vec::new();
        append_visible_shell_surface_overlay(&snapshot, runtime_shell, &mut overlay_lines);
        assert!(
            overlay_lines
                .iter()
                .any(|entry| entry.starts_with(">BATTLE SCENE: ")),
            "Options overlay should render the TypeScript-style selected row: {overlay_lines:?}"
        );
        assert!(
            !overlay_lines.iter().any(|entry| entry == "OPTIONS DETAIL"
                || entry.starts_with("options_menu ")
                || entry.starts_with("VALUES ")),
            "Options overlay must not expose Rust-only detail/debug rows: {overlay_lines:?}"
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyX);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.title_menu.is_some());
        assert!(!runtime_shell.options_menu_open);
        let title = runtime_shell
            .title_menu
            .as_ref()
            .expect("title menu should remain after closing Options");
        assert_eq!(
            visible_title_menu_entries(runtime_shell, title).expect("title entries"),
            vec![" NEW GAME", ">OPTION"]
        );
    }
}

#[test]
fn integrated_party_menu_renders_and_confirms_cancel_row() {
    let runtime_shell = core_modular_title_shell_for_test();
    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut app);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell
            .shell
            .add_party_pokemon(
                "CYNDAQUIL",
                5,
                None,
                None,
                "BEVY_PARTY_CANCEL_TEST",
                1,
                Dv::from_non_hp(10, 10, 10, 10),
            )
            .expect("add party Pokemon for menu test");
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.party_menu_open);
        let snapshot = runtime_shell.shell.snapshot().expect("party snapshot");
        let entries = visible_party_menu_entries(&snapshot, runtime_shell);
        assert!(
            entries
                .first()
                .is_some_and(|entry| entry.starts_with(">CYNDAQUIL \u{e10a}5 ")),
            "Pokemon row should be selected before moving to CANCEL: {entries:?}"
        );
        assert_eq!(
            entries.get(1).map(String::as_str),
            Some(" CANCEL"),
            "Pokemon menu should render the trailing TypeScript CANCEL row"
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.party_menu_open);
        assert!(runtime_shell.party_action_cursor.is_some());
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("party action snapshot");
        let entries = visible_party_menu_entries(&snapshot, runtime_shell);
        assert!(
            entries
                .first()
                .is_some_and(|entry| entry.starts_with(">CYNDAQUIL \u{e10a}5 ")),
            "Pokemon submenu should keep the selected party row visible: {entries:?}"
        );
        assert!(
            entries.iter().any(|entry| entry == " CANCEL"),
            "Pokemon submenu should keep the party CANCEL row visible: {entries:?}"
        );
        assert!(
            entries.iter().any(|entry| entry == "SUBMENU:"),
            "Pokemon submenu should render the TypeScript SUBMENU label: {entries:?}"
        );
        assert!(
            entries.iter().any(|entry| entry == ">STATS"),
            "Pokemon submenu should use the TypeScript STATS label: {entries:?}"
        );
        assert!(
            entries.iter().any(|entry| entry == " CANCEL"),
            "Pokemon submenu should expose a CANCEL action: {entries:?}"
        );
    }

    let submenu_action_count = {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("party submenu action count snapshot");
        visible_party_actions(&snapshot, runtime_shell)
            .expect("party submenu actions")
            .len()
    };
    let mut reached_submenu_cancel = false;
    for _ in 1..submenu_action_count {
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("party submenu traversal snapshot");
        reached_submenu_cancel = visible_party_menu_entries(&snapshot, runtime_shell)
            .iter()
            .any(|entry| entry == ">CANCEL");
        if reached_submenu_cancel {
            break;
        }
    }
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("party submenu cancel snapshot");
        assert!(
            reached_submenu_cancel,
            "walking the live submenu should reach CANCEL: {:?}",
            visible_party_menu_entries(&snapshot, runtime_shell)
        );
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.party_menu_open);
        assert!(
            runtime_shell.party_action_cursor.is_none(),
            "Z on submenu CANCEL should close only the Pokemon submenu"
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let snapshot = runtime_shell.shell.snapshot().expect("cancel snapshot");
        assert!(
            visible_party_menu_entries(&snapshot, runtime_shell)
                .iter()
                .any(|entry| entry == ">CANCEL"),
            "ArrowDown should reach the live Pokemon CANCEL row"
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(
            !runtime_shell.party_menu_open,
            "Z on CANCEL should close Pokemon"
        );
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("POKEMON CLOSED")
        );
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        assert_eq!(
            field_commands.iter(world).count(),
            0,
            "closing Pokemon should remove the rendered field-command panel"
        );
    }
}

#[test]
fn integrated_title_to_options_menu_schedule_renders_and_changes_with_live_keys() {
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
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut app);

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.start_menu_cursor.is_none());
        assert!(runtime_shell.options_menu_open);
        assert_eq!(runtime_shell.last_action_status.as_deref(), Some("OPTIONS"));
        let snapshot = runtime_shell.shell.snapshot().expect("options snapshot");
        let entries = visible_options_menu_entries(&snapshot, runtime_shell);
        assert_eq!(
            entries.first(),
            Some(&format!(
                ">TEXT SPEED: {}",
                option_value_for_item(&snapshot.trainer.options, OptionsMenuItem::TextSpeed)
            ))
        );
        assert_eq!(
            OPTIONS_MENU_ITEMS
                .iter()
                .map(|item| {
                    let value = option_value_for_item(&snapshot.trainer.options, *item);
                    if value.is_empty() {
                        options_menu_item_label(*item).to_string()
                    } else {
                        format!("{}: {}", options_menu_item_label(*item), value)
                            .trim()
                            .to_string()
                    }
                })
                .collect::<Vec<_>>(),
            vec![
                "TEXT SPEED: FAST".to_string(),
                "BATTLE SCENE: ON".to_string(),
                "BATTLE STYLE: SHIFT".to_string(),
                "SOUND: STEREO".to_string(),
                "PRINT: NORMAL".to_string(),
                "MENU ACCOUNT: ON".to_string(),
                "FRAME: 1".to_string(),
                "CANCEL".to_string(),
            ]
        );
        let mut overlay_lines = Vec::new();
        append_visible_shell_surface_overlay(&snapshot, runtime_shell, &mut overlay_lines);
        assert_eq!(
            overlay_lines, entries,
            "Options overlay should render the same TypeScript-style menu rows as the command panel"
        );
        assert!(
            !overlay_lines.iter().any(|entry| entry == "OPTIONS DETAIL"
                || entry.starts_with("options_menu ")
                || entry.starts_with("VALUES ")),
            "Options overlay must not expose Rust-only detail/debug rows: {overlay_lines:?}"
        );
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        assert!(
            field_commands.iter(world).count() > 2,
            "live Options menu should render with bitmap glyph sprites"
        );
        let mut options_frame_tiles =
            world.query_filtered::<Entity, With<FieldCommandWindowFrameMarker>>();
        assert_eq!(
            options_frame_tiles.iter(world).count(),
            battle_window_frame_tile_count(OPTIONS_MENU_WIDTH_TILES, OPTIONS_MENU_HEIGHT_TILES),
            "live Options menu should render the full-screen TypeScript/ASM options frame"
        );
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(rendered_art.font_cache.is_some());
        assert_eq!(rendered_art.font_error, None);
        assert!(!rendered_art.window_frame_cache.is_empty());
        assert!(rendered_art.window_frame_errors.is_empty());
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.options_menu_open);
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("moved options snapshot");
        assert!(
            visible_options_menu_entries(&snapshot, runtime_shell)
                .iter()
                .any(|entry| entry
                    == &format!(
                        ">BATTLE SCENE: {}",
                        option_value_for_item(
                            &snapshot.trainer.options,
                            OptionsMenuItem::BattleScene
                        )
                    )),
            "ArrowDown should move the live Options cursor"
        );
    }

    let before = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot before options change")
        .trainer
        .options
        .battle_scene;
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after options change");
        assert_ne!(
            snapshot.trainer.options.battle_scene, before,
            "Right should change the highlighted Options value through the live schedule"
        );
        assert!(
            visible_options_menu_entries(&snapshot, runtime_shell)
                .iter()
                .any(|entry| entry
                    == &format!(
                        ">BATTLE SCENE: {}",
                        option_value_for_item(
                            &snapshot.trainer.options,
                            OptionsMenuItem::BattleScene
                        )
                    )),
            "changed Options value should be rendered"
        );
    }

    for _ in 0..6 {
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    }
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.options_menu_open);
        let snapshot = runtime_shell.shell.snapshot().expect("cancel row snapshot");
        assert!(
            visible_options_menu_entries(&snapshot, runtime_shell)
                .iter()
                .any(|entry| entry == ">CANCEL"),
            "ArrowDown should reach the live Options CANCEL row"
        );
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(
            !runtime_shell.options_menu_open,
            "Z on CANCEL should close Options"
        );
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("OPTIONS CLOSED")
        );
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        assert_eq!(
            field_commands.iter(world).count(),
            0,
            "closing Options should remove the rendered field-command panel"
        );
    }
}

#[test]
fn integrated_title_to_save_menu_schedule_renders_and_writes_with_live_keys() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let save_path = std::env::temp_dir().join(format!(
        "crystal-bevy-integrated-save-{}.crystalsave",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&save_path);
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
        BevyShellStart::Title {
            spawn_identifier,
            save_path: Some(save_path.clone()),
        },
        BevyShellConfig {
            quick_save_path: Some(save_path.clone()),
            ..Default::default()
        },
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut app);

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert_eq!(
            visible_start_menu_entries(runtime_shell).expect("start menu entries"),
            vec![">PACK", " AB", " SAVE", " OPTION", " EXIT"]
        );
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert_eq!(
            visible_start_menu_entries(runtime_shell).expect("moved start menu entries"),
            vec![" PACK", " AB", ">SAVE", " OPTION", " EXIT"]
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.start_menu_cursor.is_none());
        assert!(runtime_shell.save_menu_open);
        assert_eq!(
            runtime_shell.save_flow.as_ref().map(|flow| flow.stage),
            Some(VisibleSaveFlowStage::Prompt)
        );
        let save_entries = visible_scene_dialog_entries(
            &runtime_shell.shell.snapshot().expect("save menu snapshot"),
            runtime_shell,
        )
        .expect("save dialog entries");
        assert_eq!(
            save_entries,
            vec!["Would you like to", "save the game?", "> YES    NO"]
        );
        assert_eq!(
            std::fs::exists(&save_path).expect("stat save before write"),
            false
        );
    }
    {
        let world = app.world_mut();
        let mut field_commands = world.query_filtered::<Entity, With<FieldCommandMarker>>();
        assert_eq!(
            field_commands.iter(world).count(),
            0,
            "Save should render as the Crystal dialog textbox, not a field-command panel"
        );
        let mut scene_dialogs = world.query_filtered::<Entity, With<SceneDialogMarker>>();
        assert!(
            scene_dialogs.iter(world).count() > 2,
            "Save prompt should render with scene dialog sprites"
        );
        let mut window_frames =
            world.query_filtered::<&Transform, With<SceneDialogWindowFrameMarker>>();
        let frame_positions = window_frames
            .iter(world)
            .map(|transform| transform.translation)
            .collect::<Vec<_>>();
        assert_eq!(
            frame_positions.len(),
            battle_window_frame_tile_count(
                FIELD_TEXT_BOX_WIDTH_TILES as usize,
                FIELD_TEXT_BOX_HEIGHT_TILES as usize,
            ) + battle_window_frame_tile_count(
                FIELD_YES_NO_WIDTH_TILES as usize,
                FIELD_YES_NO_HEIGHT_TILES as usize,
            ),
            "the save YesNoBox must retain both the field textbox and ASM 6x5 prompt frame"
        );
        let (yes_no_x, yes_no_y) =
            battle_hud_tile_origin(FIELD_YES_NO_LEFT_TILE, FIELD_YES_NO_TOP_TILE);
        assert!(frame_positions.iter().any(|position| {
            (position.x - yes_no_x).abs() < f32::EPSILON
                && (position.y - yes_no_y).abs() < f32::EPSILON
        }));
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(rendered_art.font_cache.is_some());
        assert_eq!(rendered_art.font_error, None);
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.save_menu_open);
        assert_eq!(
            runtime_shell.save_flow.as_ref().map(|flow| flow.stage),
            Some(VisibleSaveFlowStage::Saved)
        );
        let save_entries = visible_scene_dialog_entries(
            &runtime_shell
                .shell
                .snapshot()
                .expect("saved dialog snapshot"),
            runtime_shell,
        )
        .expect("saved dialog entries");
        assert_eq!(save_entries, vec!["AB saved", "the game."]);
        assert!(
            runtime_shell
                .last_action_status
                .as_deref()
                .is_some_and(|status| status.starts_with("SAVED FRAME")),
            "Save confirmation should report the written frame"
        );
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event.contains("saved ")
                    && event.contains(save_path.to_string_lossy().as_ref())),
            "Save confirmation should log the written save path"
        );
        let summary = runtime_shell
            .shell
            .runtime()
            .load_save_summary(&save_path)
            .expect("load written save summary");
        let snapshot = runtime_shell.shell.snapshot().expect("snapshot after save");
        assert_eq!(summary.saved_frame(), summary.state_frame());
        assert_eq!(summary.pack_content_hash(), snapshot.boot.pack_content_hash);
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(!runtime_shell.save_menu_open);
        assert!(runtime_shell.save_flow.is_none());
    }

    let _ = std::fs::remove_file(&save_path);
}

#[test]
fn integrated_title_continue_schedule_loads_saved_game_with_live_keys() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let save_path = std::env::temp_dir().join(format!(
        "crystal-bevy-integrated-continue-{}.crystalsave",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&save_path);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root.clone()),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack for save");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        AssetRoot::new(repo_root.clone()),
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: Some(save_path.clone()),
        },
        BevyShellConfig {
            quick_save_path: Some(save_path.clone()),
            ..Default::default()
        },
    )
    .expect("initialize title shell for save");

    let mut save_app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut save_app);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::Enter);
    confirm_gender_for_test(&mut save_app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut save_app);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    let saved_snapshot = save_app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot after live save");
    assert!(
        save_path.exists(),
        "live Save menu should create the save file"
    );
    assert_eq!(saved_snapshot.trainer.player_name, "AB");
    let saved_map = saved_snapshot.overworld.map_name.clone();
    let saved_tile = saved_snapshot.overworld.tile;
    let saved_pack_hash = saved_snapshot.boot.pack_content_hash.clone();
    drop(save_app);

    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root.clone()),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack for continue");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        AssetRoot::new(repo_root),
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: Some(save_path.clone()),
        },
        BevyShellConfig {
            quick_save_path: Some(save_path.clone()),
            ..Default::default()
        },
    )
    .expect("initialize title shell for continue");

    let mut continue_app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut continue_app);
    {
        let runtime_shell = continue_app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let title = runtime_shell
            .title_menu
            .as_ref()
            .expect("title menu should remain open before Continue");
        assert_eq!(
            visible_title_menu_entries(runtime_shell, title).expect("title entries"),
            vec![">CONTINUE", " NEW GAME", " OPTION"]
        );
        assert!(
            visible_title_continue_entries(runtime_shell, title)
                .iter()
                .any(|entry| entry.starts_with("EXISTS F")),
            "title Continue should render a readable save preview"
        );
    }
    {
        let world = continue_app.world_mut();
        let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
        assert_eq!(
            title_entities.iter(world).count(),
            1,
            "title Continue launch should render one composed TypeScript-style menu surface"
        );
    }

    press_key_for_runtime_hotkey_app(&mut continue_app, KeyCode::Enter);
    {
        let runtime_shell = continue_app.world().resource::<BevyRuntimeShell>();
        assert!(runtime_shell.title_menu.is_some());
        assert!(runtime_shell.visible_continue_screen.is_some());
    }
    press_key_for_runtime_hotkey_app(&mut continue_app, KeyCode::Enter);
    {
        let runtime_shell = continue_app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(runtime_shell.title_menu.is_none());
        assert!(
            runtime_shell
                .last_action_status
                .as_deref()
                .is_some_and(|status| status.starts_with("RESTORED ")),
            "title Continue should report the restored overworld location"
        );
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event.contains("title continue loaded")
                    && event.contains(save_path.to_string_lossy().as_ref())),
            "title Continue should log the exact loaded save path"
        );
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after Continue");
        assert_eq!(snapshot.trainer.player_name, "AB");
        assert_eq!(snapshot.overworld.map_name, saved_map);
        assert_eq!(snapshot.overworld.tile, saved_tile);
        assert_eq!(snapshot.boot.pack_content_hash, saved_pack_hash);
        assert!(
            runtime_shell.shell.session().state().game_timer_counting,
            "FinishContinueFunction must arm GAME_TIMER_COUNTING_F after loading"
        );
        assert!(!runtime_shell.shell.session().state().game_logic_paused);
        assert_eq!(
            runtime_shell.active_music.as_deref(),
            Some("MUSIC_NEW_BARK_TOWN")
        );
    }
    // The retained fullscreen title surface is released only after an
    // update begins with both overworld map layers query-visible.
    continue_app.update();
    {
        let world = continue_app.world_mut();
        let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
        assert_eq!(
            title_entities.iter(world).count(),
            0,
            "loading Continue should remove title art entities"
        );
        let mut players = world.query_filtered::<Entity, With<PlayerMarker>>();
        assert!(
            players.iter(world).count() >= 1,
            "loading Continue should render the saved player sprite"
        );
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(
            !rendered_art.cache.is_empty(),
            "loading Continue should populate real tileset art for the saved overworld"
        );
    }

    let _ = std::fs::remove_file(&save_path);
}

#[test]
fn continue_consumes_post_credits_marker_and_warps_to_new_bark() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let save_path = std::env::temp_dir().join(format!(
        "crystal-bevy-post-credits-{}.crystalsave",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&save_path);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root.clone()),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        AssetRoot::new(repo_root),
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "PlayersHouse2F".to_string(),
            tile_x: 3,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize runtime shell");
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .hall_of_fame
        .spawn_after_champion = Some(1);
    runtime_shell
        .shell
        .save(&save_path)
        .expect("save post-credits marker");

    load_visible_runtime_save(&mut runtime_shell, &save_path, "title_continue")
        .expect("continue post-credits save");
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("post-credits snapshot");
    assert_eq!(snapshot.overworld.map_name, "NewBarkTown");
    assert_eq!(snapshot.progression.last_spawn_identifier, Some(14));
    assert_eq!(snapshot.progression.hall_of_fame.spawn_after_champion, None);
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("post-credits continue spawn=14"))
    );

    let _ = std::fs::remove_file(&save_path);
}

#[test]
fn continue_consumes_red_post_credits_marker_and_warps_to_mt_silver() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let save_path = std::env::temp_dir().join(format!(
        "crystal-bevy-red-post-credits-{}.crystalsave",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&save_path);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root.clone()),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        AssetRoot::new(repo_root),
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 26,
            map_name: "SilverCaveRoom3".to_string(),
            tile_x: 9,
            tile_y: 33,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize runtime shell");
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .hall_of_fame
        .spawn_after_champion = Some(2);
    runtime_shell
        .shell
        .save(&save_path)
        .expect("save Red marker");

    load_visible_runtime_save(&mut runtime_shell, &save_path, "title_continue")
        .expect("continue Red post-credits save");
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("post-credits snapshot");
    assert_eq!(snapshot.overworld.map_name, "SilverCaveOutside");
    assert_eq!(snapshot.progression.last_spawn_identifier, Some(26));
    assert_eq!(snapshot.progression.hall_of_fame.spawn_after_champion, None);
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("post-credits continue spawn=26"))
    );

    let _ = std::fs::remove_file(&save_path);
}

#[test]
fn integrated_title_mystery_gift_entry_requires_unlocked_save() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let save_path = std::env::temp_dir().join(format!(
        "crystal-bevy-integrated-mystery-gift-{}.crystalsave",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&save_path);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root.clone()),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack for save");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        AssetRoot::new(repo_root.clone()),
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: Some(save_path.clone()),
        },
        BevyShellConfig {
            quick_save_path: Some(save_path.clone()),
            ..Default::default()
        },
    )
    .expect("initialize title shell for save");

    let mut save_app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut save_app);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::Enter);
    confirm_gender_for_test(&mut save_app, VisiblePlayerGender::Boy);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    complete_oak_intro_for_test(&mut save_app);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut save_app, KeyCode::KeyZ);

    {
        let runtime_shell = save_app.world_mut().resource_mut::<BevyRuntimeShell>();
        assert!(
            save_path.exists(),
            "live Save menu should create the locked save file"
        );
        assert!(
            !runtime_shell
                .shell
                .runtime()
                .load_save(&save_path)
                .expect("load locked save")
                .mystery_gift_unlocked,
            "fresh New Game save should not expose Mystery Gift"
        );
    }

    {
        let mut runtime_shell = save_app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell
            .shell
            .use_mystery_gift(RuntimeMysteryGiftAction::Unlock)
            .expect("unlock Mystery Gift");
        runtime_shell
            .shell
            .save(&save_path)
            .expect("save unlocked Mystery Gift");
        assert!(
            runtime_shell
                .shell
                .runtime()
                .load_save(&save_path)
                .expect("load unlocked save")
                .mystery_gift_unlocked,
            "unlocked save should expose Mystery Gift"
        );
    }
    drop(save_app);

    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root.clone()),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack for title");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        AssetRoot::new(repo_root),
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: Some(save_path.clone()),
        },
        BevyShellConfig {
            quick_save_path: Some(save_path.clone()),
            ..Default::default()
        },
    )
    .expect("initialize title shell for Mystery Gift");

    let mut title_app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut title_app);
    {
        let runtime_shell = title_app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let title = runtime_shell
            .title_menu
            .as_ref()
            .expect("title menu should remain open before Mystery Gift");
        assert_eq!(
            visible_title_menu_entries(runtime_shell, title).expect("title entries"),
            vec![">CONTINUE", " NEW GAME", " OPTION", " MYSTERY GIFT"]
        );
    }

    press_key_for_runtime_hotkey_app(&mut title_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut title_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut title_app, KeyCode::ArrowDown);
    press_key_for_runtime_hotkey_app(&mut title_app, KeyCode::KeyZ);
    {
        let runtime_shell = title_app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        assert!(
            runtime_shell.title_menu.is_some(),
            "Mystery Gift selection should keep the title menu open"
        );
        let mystery_gift = runtime_shell
            .pending_mystery_gift
            .as_ref()
            .expect("Mystery Gift selection should open the Mystery Gift prompt");
        assert_eq!(mystery_gift.message, MYSTERY_GIFT_PRESS_TO_LINK_TEXT);
        assert!(mystery_gift.awaiting_exchange);
        assert!(!runtime_shell.options_menu_open);
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("MYSTERY GIFT")
        );
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event == "title opened Mystery Gift"),
            "Mystery Gift selection should be logged"
        );
        let title = runtime_shell.title_menu.as_ref().expect("title menu open");
        assert_eq!(
            visible_title_menu_entries(runtime_shell, title).expect("title entries"),
            vec![" CONTINUE", " NEW GAME", " OPTION", ">MYSTERY GIFT"]
        );
    }

    press_key_for_runtime_hotkey_app(&mut title_app, KeyCode::KeyZ);
    {
        let runtime_shell = title_app.world().resource::<BevyRuntimeShell>();
        let mystery_gift = runtime_shell
            .pending_mystery_gift
            .as_ref()
            .expect("Mystery Gift error should remain visible");
        assert_eq!(mystery_gift.message, MYSTERY_GIFT_COMMUNICATION_ERROR_TEXT);
        assert!(!mystery_gift.awaiting_exchange);
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("COMMUNICATION ERROR")
        );
    }

    press_key_for_runtime_hotkey_app(&mut title_app, KeyCode::KeyX);
    {
        let runtime_shell = title_app.world().resource::<BevyRuntimeShell>();
        assert!(runtime_shell.pending_mystery_gift.is_none());
        assert!(runtime_shell.title_menu.is_some());
        assert_eq!(runtime_shell.last_action_status.as_deref(), Some("TITLE"));
    }

    let _ = std::fs::remove_file(&save_path);
}
