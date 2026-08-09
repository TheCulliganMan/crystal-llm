fn bevy_shell_source() -> &'static str {
    concat!(
        include_str!("../../bevy_shell.rs"),
        include_str!("../deterministic_session.rs"),
        include_str!("../trainer_card.rs"),
        include_str!("../title_menu.rs"),
        include_str!("../credits.rs"),
        include_str!("../script_callbacks.rs"),
        include_str!("../economy.rs"),
        include_str!("../battle_messages.rs"),
        include_str!("../battle_results.rs"),
        include_str!("../battle_entry.rs"),
        include_str!("../menu_rendering.rs"),
        include_str!("../render_mod.rs"),
        include_str!("../overworld_rendering.rs"),
        include_str!("../start_menu.rs"),
        include_str!("../bitmap_font.rs"),
        include_str!("../graphics_assets.rs"),
        include_str!("../field_pack.rs"),
    )
}

/// Match the desktop executable: the game data comes from the explicit
/// compiled pack, while artwork resolves from the workspace asset root.
/// `load_from_compiled_pack` intentionally addresses web runtime data and
/// therefore cannot exercise the native desktop pack in this test.
fn workspace_desktop_runtime(asset_root: &AssetRoot) -> CrystalRuntime {
    let pack_path = std::env::var_os("CRYSTAL_RENDER_TEST_PACK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            asset_root
                .repository_root
                .join("content-packs/core-modular.crystalpack")
        });
    let loaded = crystal_assets::read_loaded_verified_compiled_game_pack(&pack_path)
        .expect("load desktop compiled pack");
    CrystalRuntime::from_loaded_compiled_pack(asset_root, loaded)
        .expect("construct desktop runtime from compiled pack")
}

#[test]
fn release_shell_contains_no_developer_shortcut_dispatcher() {
    let source = bevy_shell_source();
    let dispatcher = format!("{}_{}_{}", "runtime", "developer", "shortcuts_enabled");
    assert!(
        !source.contains(&dispatcher),
        "release shell must not compile a developer keyboard dispatcher"
    );
}

#[test]
fn release_hotkey_mapper_has_no_space_or_escape_aliases() {
    let source = bevy_shell_source();
    let start = source
        .find("fn apply_runtime_hotkeys")
        .expect("runtime hotkey mapper");
    let end = source[start..]
        .find("fn drain_unused_runtime_ticks")
        .map(|offset| start + offset)
        .expect("end of runtime hotkey mapper");
    let mapper = &source[start..end];

    assert!(
        !mapper.contains("KeyCode::Escape") && !mapper.contains("KeyCode::Space"),
        "only configured Game Boy controls may drive the runtime hotkey mapper"
    );
}

#[test]
fn release_name_input_accepts_only_configured_game_boy_controls() {
    let source = bevy_shell_source();
    let start = source
        .find("fn apply_visible_name_input_keys")
        .expect("name input mapper");
    let end = source[start..]
        .find("fn apply_visible_name_input_smoke_char")
        .map(|offset| start + offset)
        .expect("end of name input mapper");
    let mapper = &source[start..end];

    assert!(!mapper.contains("KeyCode::Backspace"));
    assert!(!mapper.contains("KeyCode::ShiftLeft"));
}

#[test]
fn release_runtime_contains_no_partial_idle_frame_path() {
    let fast_path = format!("{}_{}_{}", "advance", "idle_frame", "fast");
    assert!(
        !bevy_shell_source().contains(&fast_path)
            && !include_str!("../../lib.rs").contains(&fast_path),
        "release runtime must advance every gameplay frame through the authoritative path"
    );
}

#[test]
fn release_shell_cannot_invoke_a_special_outside_script_execution() {
    let direct_special = format!("{}_{}_{}", "apply", "noop", "special");
    assert!(
        !bevy_shell_source().contains(&direct_special)
            && !include_str!("../../lib.rs").contains(&direct_special),
        "a source special must be reached by the script interpreter, never a shell action"
    );
}

#[test]
fn release_shell_has_no_host_happiness_service_actions() {
    let direct_service = format!("{}_{}_{}", "apply", "visible", "happiness_service");
    assert!(
        !bevy_shell_source().contains(&direct_service),
        "happiness must execute only through its exported script special, not a host action"
    );
}

use super::*;
use crate::core::systems::script_text::{ScriptTextBody, ScriptTextBodyCommand};

#[test]
fn real_pack_trainer_battle_starts_and_resolves_from_route_30() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root.clone());
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load core pack for Red battle");
    let party = [VisibleShellSmokePokemon {
        species_id: "TYPHLOSION".to_string(),
        level: 100,
        held_item_id: None,
    }];
    let smoke = smoke_visible_shell_trainer_battle(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 26,
            map_name: "Route30".to_string(),
            tile_x: 8,
            tile_y: 8,
        },
        BevyShellConfig {
            smoke_player_name: Some("TEST".to_string()),
            ..Default::default()
        },
        VisibleShellBattleSmokeRef {
            map_name: "Route30".to_string(),
            source_script: "TrainerYoungsterJoey".to_string(),
            command_index: 39,
        },
        &party,
    )
    .expect("trainer battle should resolve through the public Bevy shell bridge");
    assert_eq!(smoke.trainer_class, "YOUNGSTER");
    assert_eq!(smoke.trainer_id, "JOEY1");
    assert!(smoke.trainer_defeated);
    assert!(!smoke.active_battle_after);
    assert!(smoke.turns > 0);
}

fn oak_intro_prompt_arrow_dark_pixels(image: &Image) -> usize {
    let width = image.texture_descriptor.size.width as usize;
    (OAK_INTRO_PROMPT_ARROW_Y..OAK_INTRO_PROMPT_ARROW_Y + SOURCE_TILE_SIZE)
        .flat_map(|y| {
            (OAK_INTRO_PROMPT_ARROW_X..OAK_INTRO_PROMPT_ARROW_X + SOURCE_TILE_SIZE)
                .map(move |x| (x, y))
        })
        .filter(|(x, y)| {
            let offset = (y * width + x) * 4;
            offset + 3 < image.data.len()
                && image.data[offset] < 32
                && image.data[offset + 1] < 32
                && image.data[offset + 2] < 32
                && image.data[offset + 3] == 255
        })
        .count()
}

#[test]
fn boot_textbox_wraps_and_normalizes_like_typescript_bitmap_font() {
    assert_eq!(
        normalize_boot_text("Welcome to the\nworld of #MON!"),
        "Welcome to the\nworld of POKéMON!"
    );
    assert_eq!(
        wrap_boot_text_for_box("Welcome to the world of #MON!", 18, 4),
        vec![
            "Welcome to the".to_string(),
            "world of POKéMON!".to_string()
        ]
    );
    assert_eq!(
        wrap_boot_text_for_box("one two three four five six seven", 7, 2),
        vec!["one two".to_string(), "three".to_string()],
        "boot textboxes must clamp to the inner textbox line count instead of overflowing"
    );
}

#[test]
fn special_boundary_entries_do_not_expose_rust_effect_details() {
    let generic_boundary = special_boundary_display(&SpecialRoutineEffect::Noop);
    assert_eq!(generic_boundary.label, "SpecialRoutine");
    assert!(
        generic_boundary.details.is_empty(),
        "generic special boundaries must not expose Rust enum dumps: {generic_boundary:?}"
    );

    let boundary = SpecialBoundaryDisplay {
        label: "FadeOutMusic".to_string(),
        details: vec![
            "effect=FadeOutMusic { audio_id: MUSIC_NONE }".to_string(),
            "audio=MUSIC_NONE".to_string(),
        ],
    };

    let entries = visible_special_boundary_display_entries(&boundary);
    assert_eq!(entries, vec![">FadeOutMusic".to_string()]);

    let mut context = Vec::new();
    append_special_boundary_display_context(&boundary, &mut context);
    assert_eq!(context, vec!["special_boundary=FadeOutMusic".to_string()]);
}

#[test]
fn runtime_tick_timer_preserves_all_vblanks_while_bounding_input_catch_up() {
    let mut timer = RuntimeTickTimer::new(1.0 / 60.0);
    timer.tick(1.0 / 30.0);
    assert_eq!(timer.take_vblanks(), 2);
    assert_eq!(timer.take_ticks(), MAX_RUNTIME_CATCH_UP_TICKS);
    assert!(!timer.has_tick());
}

#[test]
fn runtime_tick_timer_long_stall_does_not_drop_elapsed_vblanks() {
    let mut timer = RuntimeTickTimer::new(1.0 / 60.0);
    timer.tick(2.0);
    assert_eq!(timer.take_vblanks(), 120);
    assert_eq!(timer.take_ticks(), MAX_RUNTIME_CATCH_UP_TICKS);
}

#[test]
fn visible_sequence_clock_recovers_normal_low_refresh_cadence_without_unbounded_skip() {
    let mut clock = VisibleSequenceTickClock::realtime();
    assert_eq!(
        clock.consume_frames(3.0 * GAME_TICK_SECONDS),
        3,
        "a 20 Hz host must preserve the 60 Hz title/intro wall-clock cadence"
    );
    assert_eq!(
        clock.consume_frames(2.0),
        MAX_VISIBLE_SEQUENCE_CATCH_UP_FRAMES,
        "a stalled host frame must not skip an entire visible sequence"
    );
}

#[test]
fn transient_audio_queue_keeps_only_the_latest_sfx_or_cry() {
    let command = |audio_id: &str, kind: ModpackAudioKind| BevyAudioCommand {
        audio_id: audio_id.to_string(),
        kind,
        mode: ModpackAudioPlaybackMode::RawPcm,
        looped: false,
    };
    let queue = coalesce_pending_transient_audio(vec![
        command("MUSIC_OLD", ModpackAudioKind::Music),
        command("SFX_OLD", ModpackAudioKind::SoundEffect),
        command("CRY_OLD", ModpackAudioKind::Cry),
        command("MUSIC_ROUTE_29", ModpackAudioKind::Music),
        command("SFX_NEW", ModpackAudioKind::SoundEffect),
    ]);
    assert_eq!(
        queue
            .iter()
            .map(|command| command.audio_id.as_str())
            .collect::<Vec<_>>(),
        vec!["MUSIC_ROUTE_29", "SFX_NEW"]
    );
}

#[test]
fn pending_music_guard_rejects_duplicate_track_but_not_transition() {
    let command = |audio_id: &str, kind: ModpackAudioKind| BevyAudioCommand {
        audio_id: audio_id.to_string(),
        kind,
        mode: ModpackAudioPlaybackMode::RawPcm,
        looped: true,
    };
    let pending = vec![command("MUSIC_NEW_BARK_TOWN", ModpackAudioKind::Music)];
    assert!(pending_music_command_is(&pending, "MUSIC_NEW_BARK_TOWN"));
    assert!(!pending_music_command_is(&pending, "MUSIC_ROUTE_29"));
}

#[test]
fn bitmap_font_background_is_transparent_and_dark_glyph_is_opaque() {
    assert!(!bitmap_font_glyph_pixel(255, 255, 255, 255));
    assert!(!bitmap_font_glyph_pixel(255, 255, 255, 0));
    assert!(bitmap_font_glyph_pixel(0, 0, 0, 255));
    assert!(bitmap_font_glyph_pixel(170, 170, 170, 255));
}

#[test]
fn yes_no_prompt_owns_all_yes_no_text_variants() {
    assert_eq!(
        (
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE,
            FIELD_YES_NO_WIDTH_TILES,
            FIELD_YES_NO_HEIGHT_TILES,
        ),
        (14.0, 7.0, 6.0, 5.0),
        "ASM YesNoBox uses menu_coords 14, 7, 19, 11"
    );
    assert!(is_visible_yes_no_prompt_entry("YES"));
    assert!(is_visible_yes_no_prompt_entry(">NO"));
    assert!(is_visible_yes_no_prompt_entry("YES / NO"));
    assert!(!is_visible_yes_no_prompt_entry("Would you like to save?"));
}

#[test]
fn phone_number_prompt_uses_the_standard_yes_no_window_cursor() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        state.script_runtime.text_window_open = false;
        state.script_runtime.pending_text_label = None;
    }
    runtime_shell.pending_phone_prompt = Some(PendingPhonePrompt {
        source_script: "PhonePromptFixture".to_string(),
        command_index: 0,
        contact_id: "PHONE_ELM".to_string(),
    });
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "ui:phone-number".to_string(),
        option_index: 1,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("phone prompt snapshot");
    assert!(scene_dialog_yes_no_active(&snapshot, &runtime_shell));
    assert_eq!(
        scene_dialog_yes_no_cursor_index(&snapshot, &runtime_shell),
        1
    );
}

#[test]
#[ignore = "performance probe; run explicitly with --ignored --nocapture"]
fn runtime_snapshot_performance_benchmark() {
    use std::hint::black_box;
    use std::time::Instant;

    const SAMPLES: usize = 120;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 2,
            tile_y: 2,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize benchmark shell");
    let benchmark_text_label = shell
        .shell
        .runtime()
        .script_text_body_keys()
        .into_iter()
        .filter(|key| key.map_name == "Route36")
        .map(|key| key.body_key)
        .find(|key| !key.starts_with('.') && !key.contains('@'))
        .expect("benchmark pack contains at least one valid text body");

    let mut timed = |label: &str, step: &mut dyn FnMut() -> Result<()>| {
        let start = Instant::now();
        for _ in 0..SAMPLES {
            step().expect("benchmark step");
        }
        let elapsed = start.elapsed();
        eprintln!(
            "runtime_perf {label} samples={SAMPLES} total_us={} per_sample_us={:.2}",
            elapsed.as_micros(),
            elapsed.as_secs_f64() * 1_000_000.0 / SAMPLES as f64
        );
    };

    timed("idle_cached", &mut || {
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
    timed("idle_full_snapshot", &mut || {
        black_box(shell.shell.snapshot()?);
        Ok(())
    });
    timed("overworld_movement", &mut || {
        shell.shell.tick([GameButton::Right])?;
        mark_runtime_snapshot_dirty(&mut shell);
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
    timed("dialog_dirty", &mut || {
        let state = shell.shell.session_mut().state_mut();
        state.script_runtime.text_window_open = true;
        state.script_runtime.pending_text_label = Some(benchmark_text_label.clone());
        mark_runtime_snapshot_dirty(&mut shell);
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
    {
        let state = shell.shell.session_mut().state_mut();
        state.script_runtime.text_window_open = false;
        state.script_runtime.pending_text_label = None;
    }
    mark_runtime_snapshot_dirty(&mut shell);

    shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BEVY_PERF_BATTLE",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add benchmark party Pokemon");
    shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start benchmark battle");
    mark_runtime_snapshot_dirty(&mut shell);
    timed("battle_cached", &mut || {
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
}

#[test]
#[ignore = "interactive Bevy schedule probe; run explicitly with --ignored --nocapture"]
fn interactive_bevy_schedule_performance_benchmark() {
    use std::time::Instant;

    const SAMPLES: usize = 120;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize interactive benchmark shell");
    runtime_shell.shell.set_runtime_journal_enabled(false);
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let idle_start = Instant::now();
    for _ in 0..SAMPLES {
        app.update();
    }
    let idle_elapsed = idle_start.elapsed();

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ArrowRight);
    app.insert_resource(keys);
    let movement_start = Instant::now();
    for _ in 0..SAMPLES {
        app.update();
    }
    let movement_elapsed = movement_start.elapsed();
    eprintln!(
        "interactive_bevy_perf samples={SAMPLES} idle_us={} idle_per_update_us={:.2} movement_us={} movement_per_update_us={:.2}",
        idle_elapsed.as_micros(),
        idle_elapsed.as_secs_f64() * 1_000_000.0 / SAMPLES as f64,
        movement_elapsed.as_micros(),
        movement_elapsed.as_secs_f64() * 1_000_000.0 / SAMPLES as f64,
    );
}

#[test]
fn screen_fade_reaches_black_in_exactly_eight_gameboy_frames() {
    let mut fade = VisibleScreenFade::new(ScriptFadeColor::Black, ScriptFadeDirection::Out, 8);
    for _ in 0..8 {
        fade.advance(GAME_TICK_SECONDS);
    }
    assert_eq!(fade.elapsed_frames, 8);
    assert_eq!(fade.alpha, 255);
}

#[test]
fn intro_renderer_preserves_every_semantic_frame() {
    let mut intro = VisibleIntroScreen::new();
    intro.jumptable_index = 13;
    intro.scene_frame_counter = 7;
    intro.scene_timer = 7;
    intro.scroll_x = 11;
    intro.scroll_y = 15;
    intro.global_anim_x_offset = 13;
    let render = intro_renderer::exact_presentation_state(&intro);
    assert_eq!(render.jumptable_index, intro.jumptable_index);
    assert_eq!(render.scene_frame_counter, 7);
    assert_eq!(render.scene_timer, 7);
    assert_eq!(render.scroll_x, 11);
    assert_eq!(render.scroll_y, 15);
    assert_eq!(render.global_anim_x_offset, 13);
    assert_eq!(
        intro.scene_frame_counter, 7,
        "simulation state is untouched"
    );
}

#[test]
fn title_art_cache_tracks_native_eight_frame_animation_cadence() {
    let mut title = TitleMenu {
        spawn_identifier: 0,
        save_path: None,
        cursor: MenuCursor {
            surface_id: "title".to_string(),
            option_index: 0,
        },
        phase: VisibleTitlePhase::PressStart,
        frame: 15,
        main_menu_frame: 0,
        scx: 0,
        title_timer: 0,
        clock_reset_trigger: false,
    };
    assert_eq!(title_screen_art_key(&title).frame, 8);
    title.frame = 16;
    assert_eq!(title_screen_art_key(&title).frame, 16);
}

#[test]
fn completed_fade_out_releases_after_one_terminal_frame() {
    let mut fade = VisibleScreenFade::new(ScriptFadeColor::Black, ScriptFadeDirection::Out, 1);
    fade.advance(GAME_TICK_SECONDS);
    assert!(
        !completed_screen_fade_should_clear(&fade),
        "the terminal black palette must be rendered once"
    );
    fade.terminal_frame_presented = true;
    assert!(
        completed_screen_fade_should_clear(&fade),
        "a completed FadeOut must not black out later field frames"
    );
}

#[test]
fn field_dialogue_reveal_uses_the_selected_text_speed() {
    assert_eq!(visible_text_frames_per_char(TextSpeed::Fast), 1);
    assert_eq!(visible_text_frames_per_char(TextSpeed::Mid), 3);
    assert_eq!(visible_text_frames_per_char(TextSpeed::Slow), 5);

    let reveal = VisibleFieldTextReveal {
        text: "PROF. ELM".to_string(),
        page_index: 0,
        visible_chars: 5,
        frames_until_next_char: 0,
    };
    assert_eq!(
        reveal
            .text
            .chars()
            .take(reveal.visible_chars)
            .collect::<String>(),
        "PROF."
    );
}

#[test]
fn runtime_tile_to_metatile_u16_uses_runtime_metatile_width() {
    assert_eq!(
        runtime_tile_to_metatile_u16(2, 0, "test").expect("runtime metatile coordinate"),
        (1, 0)
    );
    assert_eq!(
        runtime_tile_to_metatile_u16(4, 6, "test").expect("runtime metatile coordinate"),
        (2, 3)
    );
    assert!(
        runtime_tile_to_metatile_u16(3, 1, "test")
            .expect_err("unaligned runtime tile must reject")
            .to_string()
            .contains("not aligned to metatile width")
    );
}

#[test]
fn visible_shell_uses_the_game_boy_frame_cadence() {
    assert!(
        (f64::from(GAME_TICK_SECONDS) - GB_FRAME_DURATION_SECONDS).abs() < 0.000_001,
        "visible title, intro, and input timing must use the core Game Boy frame duration"
    );
}

#[test]
fn facing_runtime_tile_uses_field_interaction_stride_before_metatile_conversion() {
    let front = facing_runtime_tile_from(
        TilePosition::new(2, 2),
        crate::core::world::map::Direction::Right,
    )
    .expect("facing runtime tile stays inside runtime coordinate bounds");

    assert_eq!(front, TilePosition::new(3, 2));
    let error = runtime_tile_to_metatile_u16(front.x, front.y, "test")
        .expect_err("odd runtime tile does not convert to metatile");
    assert!(error.to_string().contains("not aligned to metatile width"));
}

#[test]
fn facing_metatile_coordinates_skip_unaligned_runtime_tiles() {
    assert_eq!(
        facing_metatile_coordinates(4, 6).expect("aligned facing tile"),
        Some((2, 3))
    );
    assert_eq!(
        facing_metatile_coordinates(39, 6).expect("odd object tile is not a metatile block"),
        None
    );
    assert_eq!(
        facing_metatile_coordinates(-1, 6).expect("negative front tile is outside map"),
        None
    );
}
