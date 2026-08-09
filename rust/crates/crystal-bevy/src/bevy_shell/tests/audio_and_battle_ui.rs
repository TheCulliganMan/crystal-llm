fn assert_audio_sources_contain_non_silent_wav(world: &World, expected_count: usize) {
    let audio_sources = world.resource::<Assets<AudioSource>>();
    assert_eq!(audio_sources.len(), expected_count);
    for (_, source) in audio_sources.iter() {
        let bytes = source.bytes.as_ref();
        assert!(
            bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE"),
            "Bevy AudioSource must contain WAV bytes"
        );
        let data_header = bytes
            .windows(4)
            .position(|window| window == b"data")
            .expect("WAV data chunk");
        let pcm_start = data_header + 8;
        assert!(
            pcm_start < bytes.len(),
            "WAV data chunk should include PCM payload"
        );
        assert!(
            bytes[pcm_start..]
                .chunks_exact(2)
                .any(|sample| i16::from_le_bytes([sample[0], sample[1]]).unsigned_abs() > 32),
            "generated Bevy audio source must not be silent"
        );
    }
}
#[test]
fn title_music_queues_and_spawns_bevy_audio_source() {
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

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .add_plugins(MinimalPlugins)
        .insert_resource(VisibleSequenceTickClock::deterministic_test())
        .init_resource::<Assets<AudioSource>>()
        .add_systems(Update, tick_visible_title_screen)
        .add_systems(
            Update,
            sync_runtime_title_music.after(tick_visible_title_screen),
        )
        .add_systems(Update, play_pending_audio.after(sync_runtime_title_music));
    advance_title_to_press_start_for_test(&mut app);
    app.update();

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert_eq!(
        runtime_shell.active_music.as_deref(),
        Some(title_music.as_str())
    );
    assert!(
        runtime_shell.pending_audio.is_empty(),
        "title music queue should be drained into Bevy audio playback"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("played Music")),
        "title music should reach the Bevy audio playback system"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("played SoundEffect SFX_TITLE_SCREEN_ENTRANCE")),
        "title entrance should reach the Bevy audio playback system"
    );
    assert_audio_sources_contain_non_silent_wav(world, 2);
    let world = app.world_mut();
    let mut music_entities = world.query_filtered::<Entity, With<MusicAudioMarker>>();
    assert_eq!(music_entities.iter(world).count(), 1);
    let mut transient_entities = world.query_filtered::<Entity, With<TransientAudioMarker>>();
    assert_eq!(
        transient_entities.iter(world).count(),
        0,
        "a replacement music command must stop stale transient audio"
    );

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.pending_audio.push(BevyAudioCommand {
            audio_id: title_music.clone(),
            kind: ModpackAudioKind::Music,
            mode: ModpackAudioPlaybackMode::SequencedMidi,
            looped: true,
        });
        runtime_shell.pending_audio.push(BevyAudioCommand {
            audio_id: title_music.clone(),
            kind: ModpackAudioKind::Music,
            mode: ModpackAudioPlaybackMode::SequencedMidi,
            looped: true,
        });
    }
    app.update();
    let world = app.world();
    assert_eq!(
        world.resource::<Assets<AudioSource>>().len(),
        2,
        "replaying the same title music must reuse the cached decoded Bevy AudioSource instead of synchronously re-rendering MIDI"
    );
    assert_audio_sources_contain_non_silent_wav(world, 2);
    let world = app.world_mut();
    let mut music_entities = world.query_filtered::<Entity, With<MusicAudioMarker>>();
    assert_eq!(music_entities.iter(world).count(), 1);
}

#[test]
fn overworld_current_music_queues_and_spawns_bevy_audio_source() {
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
    let expected_music = runtime_shell.shell.audio_state().current_music.clone();
    assert_eq!(expected_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .init_resource::<Assets<AudioSource>>()
        .add_systems(Update, play_pending_audio.after(sync_runtime_current_music))
        .add_systems(Update, sync_runtime_current_music);
    app.update();

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    assert_eq!(runtime_shell.active_music, expected_music);
    assert!(
        runtime_shell.pending_audio.is_empty(),
        "current map music queue should be drained into Bevy audio playback"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("played Music MUSIC_NEW_BARK_TOWN")),
        "current map music should reach the Bevy audio playback system"
    );
    assert_audio_sources_contain_non_silent_wav(world, 1);
    let world = app.world_mut();
    let mut music_entities = world.query_filtered::<Entity, With<MusicAudioMarker>>();
    assert_eq!(music_entities.iter(world).count(), 1);
}

#[test]
fn overworld_music_is_not_queued_during_name_entry() {
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
    .expect("initialize shell");
    runtime_shell.pending_audio.clear();
    runtime_shell.active_music = None;
    runtime_shell.pending_name_input = Some(PendingNameInput {
        label: "YOUR NAME?".to_string(),
        value: String::new(),
        max_length: VISIBLE_NAME_ENTRY_MAX_LENGTH,
        cursor_column: 0,
        cursor_row: 0,
        case: NameInputCase::Upper,
    });

    queue_visible_current_music(&mut runtime_shell).expect("sync current music");

    assert!(runtime_shell.pending_audio.is_empty());
    assert_eq!(runtime_shell.active_music, None);
}

#[test]
fn overworld_scene_spawns_real_tiles_and_player_from_compiled_pack() {
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

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app.update();

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    let snapshot = runtime_shell.shell.snapshot().expect("rendered snapshot");
    for sprite in [
        "SPRITE_CONSOLE",
        "SPRITE_DOLL_1",
        "SPRITE_DOLL_2",
        "SPRITE_BIG_DOLL",
    ] {
        assert!(
            !snapshot
                .visible_objects
                .iter()
                .any(|object| object.sprite == sprite),
            "{sprite} must not render without a live decoration replacement"
        );
        assert!(
            !snapshot.script_events.variable_sprites.contains_key(sprite),
            "{sprite} must not retain a decoration variable sprite without decoration state"
        );
    }
    for event_flag in [
        "EVENT_PLAYERS_HOUSE_2F_CONSOLE",
        "EVENT_PLAYERS_HOUSE_2F_DOLL_1",
        "EVENT_PLAYERS_HOUSE_2F_DOLL_2",
        "EVENT_PLAYERS_HOUSE_2F_BIG_DOLL",
    ] {
        assert_eq!(
            runtime_shell
                .shell
                .session
                .state
                .flags
                .event_flags
                .get(event_flag),
            Some(&true),
            "{event_flag} must hide the unset decoration object"
        );
    }
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(!rendered_art.cache.is_empty());
    assert!(!rendered_art.sprite_cache.is_empty());
    assert!(rendered_art.errors.is_empty());
    assert!(rendered_art.sprite_errors.is_empty());
    assert!(world.resource::<Assets<Image>>().len() > METATILE_TILE_COUNT);

    let world = app.world_mut();
    let _surfaces = retained_map_surface_pair(world);
    let mut players = world.query_filtered::<Entity, With<PlayerMarker>>();
    assert_eq!(players.iter(world).count(), 1);
}

#[test]
fn every_compiled_map_character_has_renderable_sprite_art() {
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
    let mut sprite_keys = BTreeSet::new();
    let mut variable_sprites = runtime.data().initialize_events.variable_sprites.clone();
    for map in runtime.data().maps.values() {
        for command in &map.script_runtime_commands {
            if command.command.eq_ignore_ascii_case("variablesprite") && command.args.len() == 2 {
                variable_sprites.insert(command.args[0].clone(), command.args[1].clone());
            }
        }
    }
    for map in runtime.data().maps.values() {
        for object in &map.objects {
            if object.sprite == "SPRITE_NONE" || object.sprite == "SPRITE_NULL" {
                continue;
            }
            // These are ASM variable-sprite slots.  Their visible art is
            // supplied by a `variablesprite`/decoration script at run
            // time (or by the Day-Care species state), so the raw object
            // constant is deliberately not an asset filename.
            if matches!(
                object.sprite.as_str(),
                "SPRITE_CONSOLE"
                    | "SPRITE_DOLL_1"
                    | "SPRITE_DOLL_2"
                    | "SPRITE_BIG_DOLL"
                    | "SPRITE_WEIRD_TREE"
                    | "SPRITE_OLIVINE_RIVAL"
                    | "SPRITE_AZALEA_ROCKET"
                    | "SPRITE_FUCHSIA_GYM_1"
                    | "SPRITE_FUCHSIA_GYM_2"
                    | "SPRITE_FUCHSIA_GYM_3"
                    | "SPRITE_FUCHSIA_GYM_4"
                    | "SPRITE_COPYCAT"
                    | "SPRITE_JANINE_IMPERSONATOR"
                    | "SPRITE_DAY_CARE_MON_1"
                    | "SPRITE_DAY_CARE_MON_2"
            ) {
                continue;
            }
            let sprite_id = resolve_visible_object_sprite_asset_id(
                &asset_root,
                &object.sprite,
                &variable_sprites,
                &runtime.data().menu_icons,
            );
            sprite_keys.insert((sprite_id, object.pal & 0x7));
        }
    }

    let mut images = Assets::<Image>::default();
    let mut failures = Vec::new();
    for (sprite_id, palette_id) in sprite_keys {
        if let Err(error) = load_sprite_art(&asset_root, &sprite_id, palette_id, "day", &mut images)
        {
            failures.push(format!("{sprite_id} palette={palette_id}: {error:#}"));
        }
    }
    assert!(
        failures.is_empty(),
        "compiled character sprites must all have renderable art:\n{}",
        failures.join("\n")
    );
}

#[test]
fn active_pokemon_picture_renders_the_asm_window_and_grayscale_frontpic() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route36 overworld shell");
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .active_pokemon_picture = Some("CHIKORITA".to_string());

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app.update();

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(
        rendered_art.pokepic_cache.contains_key("chikorita"),
        "active Pokemon picture must decode the required grayscale frontpic"
    );
    assert_eq!(rendered_art.pokepic_errors.get("chikorita"), None);

    let world = app.world_mut();
    let mut picture_entities = world.query_filtered::<Entity, With<PokemonPictureMarker>>();
    assert!(
        picture_entities.iter(world).count() > 2,
        "active Pokemon picture should include its window frame and 7x7 frontpic"
    );
}

#[test]
fn battle_scene_renders_real_art_and_asm_hud_without_rust_only_battler_labels() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route36 overworld shell");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BEVY_BATTLE_RENDER",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add battle party Pokemon");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before battle");
    runtime_shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start Sudowoodo battle");
    prepare_visible_battle_entry(&mut runtime_shell).expect("prepare battle entry");
    assert_eq!(
        runtime_shell
            .battle_messages
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            "Wild SUDOWOODO\nappeared!".to_string(),
            "Go! CYNDAQUIL!".to_string(),
        ]
    );
    assert!(
        runtime_shell.battle_message_scene.is_some(),
        "battle entry messages must retain their battle-start render scene"
    );
    // This test inspects the post-transition battle canvas itself. The
    // transition renderer has separate timing/shape coverage.
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_messages.clear();
    sync_visible_battle_action_cursor(&mut runtime_shell);

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app.update();

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(
        rendered_art.pokemon_cache.contains_key(&PokemonArtKey {
            species_id: "sudowoodo".to_string(),
            side: PokemonSpriteSide::Front,
            shiny: false,
            frame: 0,
        }),
        "enemy battle Pokemon must render real front art"
    );
    assert!(
        rendered_art
            .pokemon_cache
            .keys()
            .any(|key| { key.species_id == "cyndaquil" && key.side == PokemonSpriteSide::Back }),
        "player battle Pokemon must render real back art"
    );
    assert_eq!(rendered_art.font_error, None);
    assert!(
        rendered_art.font_cache.is_some(),
        "battle HUD must render with the runtime bitmap font"
    );
    assert!(
        !rendered_art.window_frame_cache.is_empty(),
        "battle command windows must render from the real textbox frame tile sheet"
    );
    assert!(rendered_art.window_frame_errors.is_empty());

    let world = app.world_mut();
    let mut battlers = world.query_filtered::<Entity, With<BattleBattlerMarker>>();
    assert_eq!(
        battlers.iter(world).count(),
        2,
        "battle scene should include only the two battler art sprites here; HUD text must come from the ASM battle HUD path, not Rust-only bitmap labels"
    );
    let mut battle_huds = world.query_filtered::<Entity, With<BattleHudMarker>>();
    assert!(
        battle_huds.iter(world).count() >= 20,
        "battle HUD should render enemy/player names, level/status, HP labels, bars, and player HP digits from ASM layout coordinates"
    );
    let mut battle_commands = world.query_filtered::<Entity, With<BattleCommandMarker>>();
    assert!(
        battle_commands.iter(world).count() > 2,
        "battle command menu should render"
    );
    let mut battle_window_frames = world.query_filtered::<Entity, With<BattleWindowFrameMarker>>();
    assert_eq!(
        battle_window_frames.iter(world).count(),
        battle_window_frame_tile_count(
            BATTLE_TEXT_BOX_WIDTH_TILES as usize,
            BATTLE_TEXT_BOX_HEIGHT_TILES as usize,
        ) + battle_window_frame_tile_count(
            BATTLE_MAIN_MENU_WIDTH_TILES as usize,
            BATTLE_MAIN_MENU_HEIGHT_TILES as usize,
        ),
        "the post-entry text and main-menu windows should each render one frame sprite per ASM border tile"
    );
}

#[test]
fn public_visible_shell_wild_battle_smoke_runs_from_overworld_to_turn_resolution() {
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
    let smoke = smoke_visible_shell_wild_battle(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 2,
            tile_y: 2,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        VisibleShellBattleSmokeRef {
            map_name: "Route36".to_string(),
            source_script: "WateredWeirdTreeScript".to_string(),
            command_index: 12,
        },
        &[VisibleShellSmokePokemon {
            species_id: "TYPHLOSION".to_string(),
            // Keep this a UI/turn-resolution smoke rather than making its
            // outcome depend on the canonical Sudowoodo damage race.
            level: 100,
            held_item_id: None,
        }],
        &[],
    )
    .expect("public shell battle smoke should complete");

    assert!(!smoke.wild_species.is_empty());
    assert!(
        smoke
            .action_entries
            .iter()
            .any(|entry| entry.contains("FIGHT"))
    );
    assert!(
        smoke
            .action_entries
            .iter()
            .any(|entry| entry.contains("PACK"))
    );
    assert!(!smoke.move_entries.is_empty());
    assert_ne!(
        smoke.state_hash.hash(),
        0,
        "smoke must return a committed state checksum"
    );
}

#[test]
fn battle_hud_hp_pixels_and_status_tokens_match_typescript_helpers() {
    assert_eq!(battle_hud_hp_pixels(0, 100), 0);
    assert_eq!(battle_hud_hp_pixels(1, 100), 1);
    assert_eq!(battle_hud_hp_pixels(50, 100), 24);
    assert_eq!(battle_hud_hp_pixels(100, 100), BATTLE_HUD_HP_BAR_LENGTH_PX);
    assert_eq!(battle_hud_hp_pixels(200, 100), BATTLE_HUD_HP_BAR_LENGTH_PX);
    assert_eq!(battle_status_token(Some("POISON")), Some("PSN"));
    assert_eq!(battle_status_token(Some("BAD_POISON")), Some("PSN"));
    assert_eq!(battle_status_token(Some("SLEEP")), Some("SLP"));
    assert_eq!(battle_status_token(Some("PARALYSIS")), Some("PAR"));
    assert_eq!(battle_status_token(Some("BURN")), Some("BRN"));
    assert_eq!(battle_status_token(Some("FREEZE")), Some("FRZ"));
    assert_eq!(battle_status_token(Some("UNKNOWN")), None);
    assert_eq!(battle_status_token(None), None);
}

#[test]
fn battle_main_command_menu_uses_asm_labels_and_grid_order() {
    let entries = |selected| battle_main_menu_entries_for_type("BATTLETYPE_NORMAL", 0, selected);
    assert_eq!(
        entries(battle_main_menu_index_for_action(
            VisibleBattleAction::Fight
        )),
        vec![">FIGHT", " <PKMN>", " PACK", " RUN"]
    );
    assert_eq!(
        entries(battle_main_menu_index_for_action(
            VisibleBattleAction::Pokemon
        )),
        vec![" FIGHT", "><PKMN>", " PACK", " RUN"]
    );
    assert_eq!(
        entries(battle_main_menu_index_for_action(VisibleBattleAction::Pack)),
        vec![" FIGHT", " <PKMN>", ">PACK", " RUN"]
    );
    assert_eq!(
        entries(battle_main_menu_index_for_action(VisibleBattleAction::Run)),
        vec![" FIGHT", " <PKMN>", " PACK", ">RUN"]
    );
}

#[test]
fn bug_contest_menu_accepts_only_the_canonical_battle_type() {
    assert_eq!(
        battle_main_menu_entries_for_type("BATTLETYPE_CONTEST", 7, 2),
        vec![" FIGHT", " <PKMN>", ">PARKBALL× 7", " RUN"]
    );
    for alias in ["CONTEST", "BATTLETYPE_BUG_CONTEST", "BATTLETYPE_PARK"] {
        assert_eq!(
            battle_main_menu_entries_for_type(alias, 7, 2),
            vec![" FIGHT", " <PKMN>", ">PACK", " RUN"],
            "noncanonical battle type {alias} must not enter the contest menu"
        );
        assert!(
            !bevy_shell_source().contains(&format!("\"{alias}\"")),
            "Bevy production paths must not accept {alias}"
        );
    }
}

#[test]
fn battle_main_command_menu_uses_asm_header_tile_coordinates() {
    assert_eq!(battle_main_menu_panel_center(), (96.0, -144.0));
    assert_eq!(battle_main_menu_entry_tile(0), (9.0, 13.0));
    assert_eq!(battle_main_menu_entry_tile(1), (15.0, 13.0));
    assert_eq!(battle_main_menu_entry_tile(2), (9.0, 15.0));
    assert_eq!(battle_main_menu_entry_tile(3), (15.0, 15.0));
    assert!(battle_command_entries_are_main_menu(&[
        ">FIGHT".to_string(),
        " <PKMN>".to_string(),
        " PACK".to_string(),
        " RUN".to_string(),
    ]));
    assert!(!battle_command_entries_are_main_menu(&[
        ">TACKLE".to_string(),
        " GROWL".to_string(),
        " SMOKESCREEN".to_string(),
        " EMBER".to_string(),
    ]));
}

#[test]
fn battle_move_menu_uses_asm_windows_and_cancel_row() {
    assert_eq!(
        battle_window_center(
            BATTLE_TEXT_BOX_LEFT_TILE,
            BATTLE_TEXT_BOX_TOP_TILE,
            BATTLE_TEXT_BOX_WIDTH_TILES,
            BATTLE_TEXT_BOX_HEIGHT_TILES,
        ),
        (0.0, -144.0)
    );
    assert_eq!(battle_submenu_entry_tile(0, false), (1.0, 13.0));
    assert_eq!(battle_submenu_entry_tile(1, false), (1.0, 14.0));
    assert_eq!(battle_submenu_entry_tile(0, true), (1.0, 13.0));
    assert_eq!(battle_submenu_entry_tile(1, true), (10.0, 13.0));
    assert_eq!(battle_window_frame_tile_count(20, 6), 48);
    assert_eq!(
        battle_window_center(
            BATTLE_MOVE_SELECTION_LEFT_TILE,
            BATTLE_MOVE_SELECTION_TOP_TILE,
            BATTLE_MOVE_SELECTION_WIDTH_TILES,
            BATTLE_MOVE_SELECTION_HEIGHT_TILES,
        ),
        (48.0, -144.0)
    );
    assert_eq!(
        battle_window_center(
            BATTLE_MOVE_INFO_LEFT_TILE,
            BATTLE_MOVE_INFO_TOP_TILE,
            BATTLE_MOVE_INFO_WIDTH_TILES,
            BATTLE_MOVE_INFO_HEIGHT_TILES,
        ),
        (-108.0, -36.0)
    );
    assert_eq!(battle_move_menu_entry_tile(0), (6.0, 13.0));
    assert_eq!(battle_move_menu_entry_tile(1), (6.0, 14.0));
    assert_eq!(battle_move_menu_entry_tile(3), (6.0, 16.0));
    assert_eq!(battle_move_visible_rows(5), 4);
    assert_eq!(battle_window_frame_tile_count(12, 6), 32);
    assert_eq!(battle_window_frame_tile_count(16, 6), 40);
    assert_eq!(battle_window_frame_tile_count(11, 5), 28);
    assert_eq!(battle_type_display_name("FIRE_TYPE"), "FIRE");
    assert_eq!(battle_type_display_name("SPECIAL_ATTACK"), "SPECIAL ATTACK");
}

#[test]
fn field_windows_share_the_overworld_glyph_origin_not_the_battle_origin() {
    assert_eq!(
        field_window_center(
            FIELD_TEXT_BOX_LEFT_TILE,
            FIELD_TEXT_BOX_TOP_TILE,
            FIELD_TEXT_BOX_WIDTH_TILES,
            FIELD_TEXT_BOX_HEIGHT_TILES,
        ),
        (0.0, -192.0)
    );
    assert_eq!(
        battle_hud_tile_origin(FIELD_TEXT_BOX_TEXT_LEFT_TILE, FIELD_TEXT_BOX_TEXT_TOP_TILE).1,
        -144.0,
        "the field text baseline must sit inside its field textbox"
    );
    assert_eq!(
        battle_window_center(
            FIELD_TEXT_BOX_LEFT_TILE,
            FIELD_TEXT_BOX_TOP_TILE,
            FIELD_TEXT_BOX_WIDTH_TILES,
            FIELD_TEXT_BOX_HEIGHT_TILES,
        ),
        (0.0, -144.0),
        "battle coordinates are intentionally different and must not drive field UI"
    );
}

#[test]
fn battle_move_display_name_does_not_invent_missing_move_labels() {
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
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize runtime shell");
    let snapshot = runtime_shell.shell.snapshot().expect("snapshot");
    assert_eq!(
        battle_move_display_name(&snapshot, "MADE_UP_MOVE"),
        "INVALID MOVE MADE_UP_MOVE"
    );
    assert_eq!(
        battle_move_display_name(&snapshot, "QUICK_ATTACK"),
        "QUICK ATTACK"
    );
}

#[test]
fn battle_submenu_renders_asm_text_box_frame() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route36 overworld shell");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BEVY_BATTLE_SUBMENU_FRAME",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add lead Pokemon");
    runtime_shell
        .shell
        .add_party_pokemon(
            "TOTODILE",
            10,
            None,
            None,
            "BEVY_BATTLE_SUBMENU_FRAME",
            2,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add switch target Pokemon");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before battle");
    runtime_shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start Sudowoodo battle");
    prepare_visible_battle_entry(&mut runtime_shell).expect("prepare battle entry");
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    sync_visible_battle_action_cursor(&mut runtime_shell);
    select_visible_battle_action(&mut runtime_shell, VisibleBattleAction::Pokemon)
        .expect("select Pokemon");
    open_visible_battle_switch_target(&mut runtime_shell).expect("open switch menu");

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app.update();

    let world = app.world();
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(
        !rendered_art.window_frame_cache.is_empty(),
        "battle submenu should load the real textbox frame tile sheet"
    );
    assert!(rendered_art.window_frame_errors.is_empty());
    let world = app.world_mut();
    let mut frame_tiles = world.query_filtered::<Entity, With<BattleWindowFrameMarker>>();
    assert_eq!(
        frame_tiles.iter(world).count(),
        battle_window_frame_tile_count(
            BATTLE_TEXT_BOX_WIDTH_TILES as usize,
            BATTLE_TEXT_BOX_HEIGHT_TILES as usize,
        ),
        "battle submenus should render the full ASM text box frame"
    );
}

#[test]
fn battle_move_menu_entries_drop_rust_only_instruction_row() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route36 overworld shell");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BEVY_BATTLE_MOVE_MENU",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add battle party Pokemon");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before battle");
    runtime_shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start Sudowoodo battle");
    prepare_visible_battle_entry(&mut runtime_shell).expect("prepare battle entry");
    sync_visible_battle_action_cursor(&mut runtime_shell);
    select_visible_battle_action(&mut runtime_shell, VisibleBattleAction::Fight)
        .expect("select fight");
    open_visible_battle_move_target(&mut runtime_shell).expect("open moves");

    let snapshot = runtime_shell.shell.snapshot().expect("snapshot");
    let battle = snapshot.battle.as_ref().expect("battle");
    let entries = visible_battle_move_entries(&snapshot, &runtime_shell, battle);
    assert!(
        entries.iter().all(|entry| !entry.contains("A USE MOVE")),
        "battle move menu should not render Rust-only instruction rows"
    );
    assert!(
        entries.iter().any(|entry| entry.trim_start() == "CANCEL"),
        "battle move menu should include the TypeScript CANCEL row"
    );

    let active_index = battle.active_player_party_index.expect("active party");
    let active_slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_index)
        .expect("active slot");
    runtime_shell.battle_move_cursor = Some(MenuCursor {
        surface_id: "battle:moves".to_string(),
        option_index: active_slot.pokemon.moves.len(),
    });
    let cancel_snapshot = runtime_shell.shell.snapshot().expect("cancel snapshot");
    let cancel_battle = cancel_snapshot.battle.as_ref().expect("cancel battle");
    let cancel_entries =
        visible_battle_move_entries(&cancel_snapshot, &runtime_shell, cancel_battle);
    assert!(
        cancel_entries.iter().any(|entry| entry == ">CANCEL"),
        "move menu should render the selected TypeScript CANCEL row"
    );
    press_visible_battle_a_button(&mut runtime_shell).expect("cancel move menu with A");
    assert_eq!(
        runtime_shell.battle_move_cursor, None,
        "selecting the move menu CANCEL row should close the move menu"
    );
}

#[test]
fn battle_submenus_drop_rust_only_instruction_rows() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route36 overworld shell");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BEVY_BATTLE_SUBMENU",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add lead Pokemon");
    runtime_shell
        .shell
        .add_party_pokemon(
            "TOTODILE",
            10,
            None,
            None,
            "BEVY_BATTLE_SUBMENU",
            2,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add switch target Pokemon");
    runtime_shell
        .shell
        .add_bag_item("POTION", 2)
        .expect("add battle item");
    runtime_shell
        .shell
        .add_bag_item("POKE_BALL", 2)
        .expect("add battle ball");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before battle");
    runtime_shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start Sudowoodo battle");
    prepare_visible_battle_entry(&mut runtime_shell).expect("prepare battle entry");
    sync_visible_battle_action_cursor(&mut runtime_shell);

    select_visible_battle_action(&mut runtime_shell, VisibleBattleAction::Pokemon)
        .expect("select Pokemon");
    open_visible_battle_switch_target(&mut runtime_shell).expect("open switch menu");
    let snapshot = runtime_shell.shell.snapshot().expect("switch snapshot");
    let battle = snapshot.battle.as_ref().expect("switch battle");
    let switch_entries = visible_battle_switch_entries(&snapshot, &runtime_shell, battle);
    assert!(
        switch_entries
            .iter()
            .all(|entry| !entry.contains("A SWITCH") && !entry.contains("B BACK")),
        "battle switch menu should not render Rust-only instruction rows: {switch_entries:?}"
    );

    press_visible_battle_b_button(&mut runtime_shell).expect("close switch menu");
    select_visible_battle_action(&mut runtime_shell, VisibleBattleAction::Pack)
        .expect("select Pack");
    open_visible_battle_pack(&mut runtime_shell).expect("open battle pack");
    let snapshot = runtime_shell.shell.snapshot().expect("item snapshot");
    let item_entries = visible_battle_item_entries(&snapshot, &runtime_shell);
    assert!(
        item_entries
            .iter()
            .all(|entry| !entry.contains("A USE ITEM") && !entry.contains("B BACK")),
        "battle item menu should not render Rust-only instruction rows: {item_entries:?}"
    );
    assert!(
        item_entries
            .iter()
            .all(|entry| !entry.contains("effect=") && !entry.contains("use=")),
        "battle item menu should not expose internal item catalog fields: {item_entries:?}"
    );

    move_visible_ball_cursor(&mut runtime_shell, 0).expect("open ball pocket");
    let snapshot = runtime_shell.shell.snapshot().expect("ball snapshot");
    let ball_entries = visible_battle_ball_entries(&snapshot, &runtime_shell);
    assert!(
        ball_entries
            .iter()
            .all(|entry| !entry.contains("A THROW") && !entry.contains("B BACK")),
        "battle ball menu should not render Rust-only instruction rows: {ball_entries:?}"
    );

    move_visible_battle_bag_cursor(&mut runtime_shell, 0).expect("return to battle items");
    open_visible_battle_pack_target(&mut runtime_shell, BattlePackTargetMode::PartyPokemon)
        .expect("open battle item target");
    let snapshot = runtime_shell.shell.snapshot().expect("target snapshot");
    let target_entries = visible_battle_pack_target_entries(
        &snapshot,
        &runtime_shell,
        BattlePackTargetMode::PartyPokemon,
    );
    assert!(
        target_entries
            .iter()
            .all(|entry| !entry.contains("A TARGET") && !entry.contains("B BACK")),
        "battle item target menu should not render Rust-only instruction rows: {target_entries:?}"
    );
}
