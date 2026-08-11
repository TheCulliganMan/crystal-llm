#[test]
fn visible_start_menu_responds_to_normal_button_inputs() {
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
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before opening the start menu");

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Start])
        .expect("Start opens the start menu through normal input dispatch");
    assert_eq!(
        visible_start_menu_entries(&runtime_shell).expect("start menu entries"),
        vec![">PACK", " AB", " SAVE", " OPTION", " EXIT"]
    );

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
        .expect("Down moves the start menu cursor through normal input dispatch");
    assert_eq!(
        visible_start_menu_entries(&runtime_shell).expect("moved start menu entries"),
        vec![" PACK", ">AB", " SAVE", " OPTION", " EXIT"]
    );

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A selects the highlighted start menu option through normal input dispatch");
    assert!(runtime_shell.start_menu_cursor.is_none());
    assert!(runtime_shell.trainer_card_open);
    assert_eq!(
        runtime_shell.last_action_status.as_deref(),
        Some("TRAINER CARD")
    );

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::B])
        .expect("B closes the opened start menu panel through normal input dispatch");
    assert!(!runtime_shell.trainer_card_open);
}
#[test]
fn visible_pokedex_and_pokegear_overlays_do_not_render_debug_detail_rows() {
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
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before opening the start menu");
    runtime_shell
        .shell
        .set_script_flag_for_smoke(ENGINE_POKEDEX_FLAG)
        .expect("unlock Pokedex through engine flag storage");
    runtime_shell
        .shell
        .set_script_flag_for_smoke(ENGINE_POKEGEAR_FLAG)
        .expect("unlock Pokegear through engine flag storage");

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Start])
        .expect("Start opens the start menu through normal input dispatch");
    assert_eq!(
        visible_start_menu_entries(&runtime_shell).expect("start menu entries"),
        vec![">#DEX", " PACK", " #GEAR", " AB", " SAVE", " OPTION", " EXIT"]
    );

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens Pokedex from the highlighted Start menu row");
    {
        let snapshot = runtime_shell.shell.snapshot().expect("Pokedex snapshot");
        let expected = visible_pokedex_menu_entries(&snapshot, &runtime_shell);
        let mut overlay_lines = Vec::new();
        append_visible_shell_surface_overlay(&snapshot, &runtime_shell, &mut overlay_lines);
        assert_eq!(
            overlay_lines, expected,
            "Pokedex overlay should be the same visible menu rows as the command panel"
        );
        assert_no_visible_pokedex_or_pokegear_debug_rows(&overlay_lines);
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens Pokedex detail from the highlighted species");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pokedex detail snapshot");
        let expected = visible_pokedex_menu_entries(&snapshot, &runtime_shell);
        let mut overlay_lines = Vec::new();
        append_visible_shell_surface_overlay(&snapshot, &runtime_shell, &mut overlay_lines);
        assert_eq!(
            overlay_lines, expected,
            "Pokedex detail overlay should be the same visible rows as the command panel"
        );
        assert_no_visible_pokedex_or_pokegear_debug_rows(&overlay_lines);
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::B])
        .expect("B closes Pokedex detail");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::B])
        .expect("B closes Pokedex");

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Start])
        .expect("Start reopens the start menu through normal input dispatch");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
        .expect("Down moves from Pokedex to Pack");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
        .expect("Down moves from Pack to Pokegear");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens Pokegear from the highlighted Start menu row");
    {
        let snapshot = runtime_shell.shell.snapshot().expect("Pokegear snapshot");
        let expected = visible_pokegear_menu_entries(&snapshot, &runtime_shell);
        let mut overlay_lines = Vec::new();
        append_visible_shell_surface_overlay(&snapshot, &runtime_shell, &mut overlay_lines);
        assert_eq!(
            overlay_lines, expected,
            "Pokegear overlay should be the same visible menu rows as the command panel"
        );
        assert_no_visible_pokedex_or_pokegear_debug_rows(&overlay_lines);
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens Pokegear landmark detail");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pokegear detail snapshot");
        let expected = visible_pokegear_menu_entries(&snapshot, &runtime_shell);
        let mut overlay_lines = Vec::new();
        append_visible_shell_surface_overlay(&snapshot, &runtime_shell, &mut overlay_lines);
        assert_eq!(
            overlay_lines, expected,
            "Pokegear detail overlay should be the same visible rows as the command panel"
        );
        assert_no_visible_pokedex_or_pokegear_debug_rows(&overlay_lines);
    }
}

fn assert_no_visible_pokedex_or_pokegear_debug_rows(lines: &[String]) {
    assert!(
        !lines.iter().any(|entry| entry == "POKEDEX DETAIL"
            || entry == "POKEGEAR DETAIL"
            || entry.starts_with("pokedex selected=")
            || entry.starts_with("pokedex_entry ")
            || entry.starts_with("pokegear selected=")
            || entry.starts_with("pokegear phone_contacts=")),
        "Pokedex/Pokegear overlay must not expose Rust-only detail/debug rows: {lines:?}"
    );
}

#[test]
fn visible_pack_menu_renders_and_confirms_cancel_row_from_normal_inputs() {
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
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before opening the Pack");
    runtime_shell
        .shell
        .add_bag_item("POTION", 1)
        .expect("add Potion for Pack test");

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Start])
        .expect("Start opens the start menu through normal input dispatch");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens Pack from the start menu through normal input dispatch");
    {
        let snapshot = runtime_shell.shell.snapshot().expect("Pack snapshot");
        let entries = visible_field_pack_entries(&snapshot, &runtime_shell);
        assert_eq!(entries.first().map(String::as_str), Some("POCKET: ITEMS"));
        assert!(
            entries.iter().any(|entry| entry == ">POTION x01"),
            "Pack should render TypeScript-style quantity rows: {entries:?}"
        );
        assert!(
            entries.iter().any(|entry| entry == " CANCEL"),
            "Pack should render the trailing TypeScript CANCEL row: {entries:?}"
        );
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens the Pack action menu for the selected item");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pack action snapshot");
        let entries = visible_field_pack_entries(&snapshot, &runtime_shell);
        assert_eq!(
            entries,
            vec!["ACTION POTION x01", ">USE", " GIVE", " TOSS", " QUIT"],
            "Pack action menu should match the TypeScript item action options"
        );
    }
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
        .expect("Down moves inside the Pack action menu");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pack action moved snapshot");
        assert!(
            visible_field_pack_entries(&snapshot, &runtime_shell)
                .iter()
                .any(|entry| entry == ">GIVE"),
            "Pack action cursor should move before item-list cursor movement"
        );
    }
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::B])
        .expect("B closes the Pack action menu without closing Pack");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pack list restored snapshot");
        assert!(
            visible_field_pack_entries(&snapshot, &runtime_shell)
                .iter()
                .any(|entry| entry == ">POTION x01"),
            "B in the Pack action menu should return to the item list"
        );
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A reopens the Pack action menu before tossing");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
        .expect("Down moves from USE to GIVE");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
        .expect("Down moves from GIVE to TOSS");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A opens the ASM toss quantity prompt");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A confirms the toss quantity");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A accepts the toss confirmation");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pack tossed snapshot");
        let entries = visible_field_pack_entries(&snapshot, &runtime_shell);
        assert!(
            entries.iter().any(|entry| entry == ">CANCEL"),
            "Tossing the only item should leave the Pack on CANCEL: {entries:?}"
        );
        assert_eq!(
            runtime_shell.last_action_status.as_deref(),
            Some("TOSSED POTION x1")
        );
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A dismisses the canonical threw-away notice");
    {
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("Pack cancel snapshot");
        assert!(
            visible_field_pack_entries(&snapshot, &runtime_shell)
                .iter()
                .any(|entry| entry == ">CANCEL"),
            "Pack cursor should reach the CANCEL row"
        );
    }

    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("A closes Pack from the CANCEL row through normal input dispatch");
    assert!(!visible_field_pack_is_open(&runtime_shell));
    assert_eq!(
        runtime_shell.last_action_status.as_deref(),
        Some("PACK CLOSED")
    );
}

#[test]
fn start_menu_labels_match_typescript_asm_glyph_entries() {
    assert_eq!(start_menu_option_label(StartMenuOption::Pokedex), "#DEX");
    assert_eq!(start_menu_option_label(StartMenuOption::Pokemon), "#MON");
    assert_eq!(start_menu_option_label(StartMenuOption::Pack), "PACK");
    assert_eq!(start_menu_option_label(StartMenuOption::Pokegear), "#GEAR");
    assert_eq!(
        start_menu_option_label(StartMenuOption::TrainerCard),
        "STATUS"
    );
    assert_eq!(start_menu_option_label(StartMenuOption::Save), "SAVE");
    assert_eq!(start_menu_option_label(StartMenuOption::Options), "OPTION");
    assert_eq!(start_menu_option_label(StartMenuOption::Exit), "EXIT");
}

#[test]
fn start_menu_field_command_renders_bitmap_glyph_sprites() {
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
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before opening the start menu");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Start])
        .expect("open start menu");

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app.update();

    let world = app.world_mut();
    let mut field_command_entities = world.query_filtered::<Entity, With<FieldCommandMarker>>();
    assert!(
        field_command_entities.iter(world).count() > 2,
        "start menu field-command surface must include bitmap glyph sprites, not only panel sprites"
    );
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(
        rendered_art.font_cache.is_some(),
        "start menu render must load bitmap font art"
    );
    assert_eq!(rendered_art.font_error, None);
}

#[test]
fn arrow_key_mapping_moves_overworld_player_when_held() {
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
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before moving");
    let routing_snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot settled movement routing");
    let shell_routes_direction = has_visible_shell_direction_action(&mut runtime_shell);
    assert!(
        !shell_routes_direction,
        "settled overworld direction must route to joypad; cursor={:?} start={:?} special={:?} events={:?}",
        runtime_shell.active_script_cursor,
        runtime_shell.start_menu_cursor,
        runtime_shell.special_boundary,
        routing_snapshot.script_events
    );
    let start_tile = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot before movement")
        .overworld
        .tile;

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ArrowRight);
    for _ in 0..6 {
        let buttons = collect_overworld_keyboard_buttons(&keys, false, false, false, false, false);
        assert_eq!(buttons, vec![GameButton::Right]);
        apply_visible_shell_smoke_frame(&mut runtime_shell, &buttons)
            .expect("ArrowRight frame advances overworld input");
    }

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
}

#[test]
fn live_runtime_hotkeys_move_overworld_player_when_arrow_key_is_held() {
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
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before moving");
    let routing_snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot settled live movement routing");
    let shell_routes_direction = has_visible_shell_direction_action(&mut runtime_shell);
    assert!(
        !shell_routes_direction,
        "settled live direction must route to joypad; cursor={:?} start={:?} special={:?} events={:?}",
        runtime_shell.active_script_cursor,
        runtime_shell.start_menu_cursor,
        runtime_shell.special_boundary,
        routing_snapshot.script_events
    );
    let start_tile = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot before movement")
        .overworld
        .tile;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(native_rtc_source_for_test())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(RuntimeTickTimer::new(0.0))
        .insert_resource(HeldArrowRightTestFrames(6))
        .add_systems(
            Update,
            (
                inject_held_arrow_right_for_test,
                apply_keyboard_input,
                apply_runtime_hotkeys,
            )
                .chain(),
        );
    for _ in 0..6 {
        app.update();
    }

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert!(
        runtime_shell.last_overworld_input.is_some(),
        "held ArrowRight should be recorded as overworld input"
    );
    let end_tile = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot after live movement")
        .overworld
        .tile;
    assert_eq!(runtime_shell.last_error, None);
    assert!(
        end_tile.x > start_tile.x,
        "held ArrowRight through the live hotkey system should move player right from {start_tile:?} to {end_tile:?}; input={:?} status={:?} facing={:?}",
        runtime_shell.last_overworld_input,
        runtime_shell.last_action_status,
        runtime_shell
            .shell
            .snapshot()
            .expect("movement diagnostic snapshot")
            .overworld
            .facing
    );
    assert!(
        runtime_shell.player_walk_frame_ticks > 0,
        "a successful live movement step must hold the walking sprite frame"
    );
}

#[test]
fn arrow_key_dispatch_moves_visible_start_menu_cursor() {
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
    .expect("initialize overworld shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name and run arrival callbacks");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before opening the start menu");
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Start])
        .expect("open start menu");
    assert_eq!(
        visible_start_menu_entries(&runtime_shell).expect("start menu entries"),
        vec![">PACK", " AB", " SAVE", " OPTION", " EXIT"]
    );

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ArrowDown);
    apply_visible_runtime_controls(&keys, &mut runtime_shell, true);

    assert_eq!(
        visible_start_menu_entries(&runtime_shell).expect("moved start menu entries"),
        vec![" PACK", ">AB", " SAVE", " OPTION", " EXIT"]
    );
}

#[test]
fn visible_overworld_normal_inputs_walk_through_bedroom_warp() {
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
        ],
        None,
    )
    .expect("normal inputs walk through bedroom warp");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(smoke.final_map, "PlayersHouse1F");
    assert_eq!((smoke.final_tile_x, smoke.final_tile_y), (9, 0));
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_PLAYERSHOUSE1F_MEET_MOM")
    );
    assert_eq!(smoke.warps, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("warp=true"))
    );
}
