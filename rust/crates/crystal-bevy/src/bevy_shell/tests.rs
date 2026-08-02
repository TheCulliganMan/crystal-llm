    fn bevy_shell_source() -> &'static str {
        concat!(
            include_str!("../bevy_shell.rs"),
            include_str!("deterministic_session.rs"),
            include_str!("trainer_card.rs"),
            include_str!("title_menu.rs"),
            include_str!("credits.rs"),
            include_str!("script_callbacks.rs"),
            include_str!("economy.rs"),
            include_str!("battle_messages.rs"),
            include_str!("battle_results.rs"),
            include_str!("battle_entry.rs"),
            include_str!("menu_rendering.rs"),
            include_str!("overworld_rendering.rs"),
            include_str!("start_menu.rs"),
            include_str!("bitmap_font.rs"),
            include_str!("graphics_assets.rs"),
            include_str!("field_pack.rs"),
        )
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
                && !include_str!("../lib.rs").contains(&fast_path),
            "release runtime must advance every gameplay frame through the authoritative path"
        );
    }

    #[test]
    fn release_shell_cannot_invoke_a_special_outside_script_execution() {
        let direct_special = format!("{}_{}_{}", "apply", "noop", "special");
        assert!(
            !bevy_shell_source().contains(&direct_special)
                && !include_str!("../lib.rs").contains(&direct_special),
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
    fn real_pack_red_battle_starts_and_resolves_from_silver_cave() {
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
        let smoke = smoke_visible_shell_trainer_battle(
            asset_root,
            runtime,
            BevyShellStart::NewGameAtRuntimeTile {
                spawn_identifier: 26,
                map_name: "SilverCaveRoom3".to_string(),
                tile_x: 9,
                tile_y: 33,
            },
            BevyShellConfig {
                smoke_player_name: Some("REDTEST".to_string()),
                ..Default::default()
            },
            VisibleShellBattleSmokeRef {
                map_name: "SilverCaveRoom3".to_string(),
                source_script: "Red".to_string(),
                command_index: 8,
            },
            &[VisibleShellSmokePokemon {
                species_id: "RATTATA".to_string(),
                level: 100,
                held_item_id: None,
            }],
        )
        .expect("Red battle should resolve through the public Bevy shell bridge");
        assert_eq!(smoke.trainer_class, "RED");
        assert_eq!(smoke.trainer_id, "RED1");
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
    fn runtime_tick_timer_drops_excess_elapsed_frames_after_render_stall() {
        let mut timer = RuntimeTickTimer::new(1.0 / 60.0);
        timer.tick(1.0 / 30.0);
        assert_eq!(timer.take_ticks(), MAX_RUNTIME_CATCH_UP_TICKS);
        assert!(!timer.has_tick());
    }

    #[test]
    fn runtime_tick_timer_bounds_stale_catch_up_after_render_stall() {
        let mut timer = RuntimeTickTimer::new(1.0 / 60.0);
        timer.tick(2.0);
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
        let runtime = CrystalRuntime::load_from_compiled_pack(
            &asset_root,
            "content-packs/core-modular.crystalpack",
        )
        .expect("load compiled pack");
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
        let runtime = CrystalRuntime::load_from_compiled_pack(
            &asset_root,
            "content-packs/core-modular.crystalpack",
        )
        .expect("load compiled pack");
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

    #[test]
    fn field_move_play_refusals_do_not_swallow_unsupported_pack_coverage() {
        assert!(party_field_move_error_is_play_refusal(&anyhow::anyhow!(
            FieldMoveError::MissingBadge {
                move_id: "SURF".to_string(),
                region: "johto".to_string(),
                badge_index: 5,
            }
        )));
        assert!(!party_field_move_error_is_play_refusal(&anyhow::anyhow!(
            FieldMoveError::UnsupportedReplacement {
                move_id: "CUT".to_string(),
                tileset_name: "johto".to_string(),
                block_id: 0x5b,
            }
        )));
        assert!(!party_field_move_error_is_play_refusal(&anyhow::anyhow!(
            FieldMoveError::UnsupportedCollision {
                move_id: "WHIRLPOOL".to_string(),
                block_id: 0x30,
            }
        )));
    }

    #[test]
    fn runtime_tile_playfield_position_preserves_runtime_tile_offsets() {
        assert_eq!(
            runtime_event_view_tile(TilePosition::new(1, 0), 0, 0),
            Some((2, 0))
        );
        assert_eq!(
            runtime_event_view_tile(TilePosition::new(i16::MIN, 0), 1, 0),
            None
        );
        assert_eq!(
            runtime_tile_playfield_position(TilePosition::new(1, 0), 0, 0),
            Some((PLAYFIELD_LEFT + TILE_SIZE * 2.0, PLAYFIELD_TOP))
        );
        assert_eq!(
            runtime_tile_playfield_position(TilePosition::new(-1, 0), 0, 0),
            None
        );
    }

    #[test]
    fn overworld_sprites_anchor_to_their_complete_oam_footprint() {
        let base = runtime_tile_playfield_position(TilePosition::new(0, 0), 0, 0)
            .expect("visible origin tile");
        assert_eq!(
            overworld_sprite_position_from_base(base.0, base.1, Vec2::splat(TILE_SIZE * 2.0)),
            (base.0 + TILE_SIZE * 0.5, base.1 - TILE_SIZE * 0.5),
            "a 16x16 Game Boy sprite must cover its two-by-two render-tile footprint"
        );
        assert_eq!(
            overworld_sprite_position_from_base(base.0, base.1, Vec2::splat(TILE_SIZE)),
            base,
            "single-tile icon sprites remain anchored to their addressed render tile"
        );
    }

    #[test]
    fn finite_pcm_music_does_not_restart_when_its_playback_plan_requests_looping() {
        let pcm_music = BevyAudioCommand {
            audio_id: "MUSIC_CRYSTAL_OPENING".to_string(),
            kind: ModpackAudioKind::Music,
            mode: ModpackAudioPlaybackMode::RawPcm,
            looped: true,
        };
        assert!(
            !native_audio_repeats_without_pcm_loop(&pcm_music),
            "a PCM asset without explicit loop bounds must end at its exported endpoint"
        );

        let midi_music = BevyAudioCommand {
            mode: ModpackAudioPlaybackMode::SequencedMidi,
            ..pcm_music
        };
        assert!(native_audio_repeats_without_pcm_loop(&midi_music));
    }

    #[test]
    fn map_debug_details_report_runtime_tiles_and_raw_event_coordinates() {
        let warp = crate::core::map::WarpEvent {
            index: 3,
            x: 2,
            y: 3,
            target_map_constant: "ROUTE_29".to_string(),
            target_map: "ROUTE_29".to_string(),
            target_warp_id: 1,
        };
        assert_eq!(
            format_warp_event_detail_line(&warp),
            "warp 3 runtime_tile=(2, 3) raw=(2, 3) target=ROUTE_29 target_warp=1"
        );

        let object = crate::core::map::ObjectEvent {
            sprite: "SPRITE_TEACHER".to_string(),
            x: 2,
            y: 3,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "TeacherScript".to_string(),
            label: None,
            event_flag: "EVENT_TEACHER".to_string(),
            object_identifier: Some("ROUTE29_TEACHER".to_string()),
            sightline_direction_override: None,
        };
        assert_eq!(
            format_visible_object_detail_line(&object),
            "visible_object Some(\"ROUTE29_TEACHER\") sprite=SPRITE_TEACHER runtime_tile=(2, 3) raw=(2, 3) script=TeacherScript flag=EVENT_TEACHER"
        );
    }

    #[test]
    fn runtime_tile_bounds_use_checked_runtime_metatile_width() {
        assert_eq!(
            runtime_tile_bounds_i16("ROUTE_29", 20, 18).expect("map bounds"),
            (40, 36)
        );
        assert_eq!(
            render_tile_bounds_i16("ROUTE_29", 20, 18).expect("render bounds"),
            (80, 72)
        );
        assert!(
            runtime_tile_bounds_i16("OVERFLOW_WIDTH", 16_384, 1)
                .expect_err("overflowing width rejects")
                .to_string()
                .contains("width 16384 overflows supported runtime tile coordinate bounds")
        );
        assert!(
            runtime_tile_bounds_i16("OVERFLOW_HEIGHT", 1, 16_384)
                .expect_err("overflowing height rejects")
                .to_string()
                .contains("height 16384 overflows supported runtime tile coordinate bounds")
        );
    }

    #[test]
    fn render_viewport_origin_clamps_like_typescript_camera() {
        assert_eq!(render_viewport_origin(4, 16, VIEWPORT_TILES_X), 0);
        assert_eq!(render_viewport_origin(20, 80, VIEWPORT_TILES_X), 10);
        assert_eq!(render_viewport_origin(79, 80, VIEWPORT_TILES_X), 60);
        assert_eq!(render_viewport_origin(4, 12, VIEWPORT_TILES_Y), 0);
        assert_eq!(render_viewport_origin(18, 72, VIEWPORT_TILES_Y), 9);
        assert_eq!(render_viewport_origin(71, 72, VIEWPORT_TILES_Y), 54);
    }

    #[test]
    fn johto_tileset_art_loads_real_runtime_assets() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let palette_map_path = asset_root
            .runtime_assets()
            .join("data/tilesets/johto_palette_map.json");
        let palette_map: Vec<u8> = serde_json::from_slice(
            &std::fs::read(&palette_map_path).expect("read johto palette map"),
        )
        .expect("parse johto palette map");
        let mut images = Assets::<Image>::default();
        let art = load_tileset_art(&asset_root, "johto", "day", &palette_map, &mut images)
            .expect("load johto tileset art");

        assert!(art.metatile_layout.len() >= METATILE_TILE_COUNT);
        assert!(art.tile_handles.len() > 0x40);
        assert!(art.tile_handle(0, 0, 0).is_some());
        assert!(art.tile_handle(0, 1, 0).is_some());
        assert!(art.tile_handle(0, 3, 3).is_some());
    }

    #[test]
    fn viewport_tile_composite_preserves_scaled_tile_grid() {
        let mut images = Assets::<Image>::default();
        let tile = images.add(Image::new(
            Extent3d {
                width: SOURCE_TILE_SIZE as u32,
                height: SOURCE_TILE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0xff, 0x00, 0x00, 0xff]
                .into_iter()
                .cycle()
                .take(SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4)
                .collect(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let handles = vec![tile; (VIEWPORT_TILES_X * VIEWPORT_TILES_Y) as usize];
        let composite = compose_viewport_tiles(&handles, None, &mut images);
        let image_count = images.len();
        let reused = compose_viewport_tiles(&handles, Some(composite.clone()), &mut images);
        assert_eq!(reused, composite);
        assert_eq!(images.len(), image_count);
        let image = images.get(&composite).expect("composited viewport image");
        assert_eq!(image.texture_descriptor.size.width, 640);
        assert_eq!(image.texture_descriptor.size.height, 576);
        assert_eq!(&image.data[0..4], &[0xff, 0x00, 0x00, 0xff]);
        let last = (576 * 640 - 1) * 4;
        assert_eq!(&image.data[last..last + 4], &[0xff, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn bitmap_font_art_loads_runtime_menu_glyphs() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut images = Assets::<Image>::default();
        let art = load_bitmap_font_art(&asset_root, &mut images).expect("load bitmap font art");

        for glyph in ['A', 'Z', '0', '9', '>', '?', ' '] {
            assert!(
                art.glyphs.contains_key(&glyph),
                "expected bitmap font glyph {glyph:?}"
            );
            assert_eq!(
                art.glyphs.get(&glyph).map(|frame| frame.size),
                Some(Vec2::splat(TILE_SIZE)),
                "dialogue glyph {glyph:?} must occupy one scaled 8x8 Game Boy tile"
            );
        }
    }

    #[test]
    fn field_dialogue_uses_the_game_boy_text_grid() {
        assert_eq!(SCENE_DIALOG_TEXT_CHARS, 18);
        assert_eq!(FIELD_TEXT_BOX_TEXT_LEFT_TILE, 1.0);
        assert_eq!(FIELD_TEXT_BOX_TEXT_TOP_TILE, 13.0);
        assert_eq!(FIELD_TEXT_BOX_VISIBLE_ROWS, 4);
        assert_eq!(BITMAP_FONT_ADVANCE, TILE_SIZE);
    }

    #[test]
    fn field_dialogue_wraps_pokegear_text_inside_the_four_row_box() {
        assert_eq!(
            wrap_scene_dialog_line(
                "POKéMON GEAR, or just POKéGEAR. It's essential information.",
                SCENE_DIALOG_TEXT_CHARS,
            ),
            vec![
                "POKéMON GEAR, or",
                "just POKéGEAR.",
                "It's essential",
                "information.",
            ],
        );
    }

    #[test]
    fn player_walk_stride_alternates_on_consecutive_steps() {
        let first = next_player_walk_stride(0, false);
        let second = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, first);
        let third = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, second);

        assert!(first, "the first step starts on the walking frame");
        assert!(!second, "the second consecutive step returns to standing");
        assert!(
            third,
            "the third consecutive step resumes the walking frame"
        );
    }

    #[test]
    fn player_walk_interpolates_each_lcd_frame_between_committed_tiles() {
        for (from, to) in [
            (TilePosition { x: 2, y: 2 }, TilePosition { x: 3, y: 2 }),
            (TilePosition { x: 2, y: 2 }, TilePosition { x: 1, y: 2 }),
            (TilePosition { x: 2, y: 2 }, TilePosition { x: 2, y: 1 }),
            (TilePosition { x: 2, y: 2 }, TilePosition { x: 2, y: 3 }),
        ] {
            let positions = (0..=WALK_FRAME_HOLD_TICKS)
                .rev()
                .map(|remaining| {
                    visible_player_playfield_position(to, Some(from), remaining, 0, 0)
                        .expect("walk position")
                })
                .collect::<Vec<_>>();
            let final_position = visible_player_playfield_position(to, None, 0, 0, 0)
                .expect("final walk position");
            assert_eq!(positions[0], visible_player_playfield_position(from, None, 0, 0, 0).expect("initial tile"));
            assert_eq!(positions.last().copied(), Some(final_position));

            let dx = (final_position.0 - positions[0].0) / f32::from(WALK_FRAME_HOLD_TICKS);
            let dy = (final_position.1 - positions[0].1) / f32::from(WALK_FRAME_HOLD_TICKS);
            for (frame, position) in positions.iter().enumerate() {
                assert_eq!(position.0, positions[0].0 + dx * frame as f32);
                assert_eq!(position.1, positions[0].1 + dy * frame as f32);
            }
        }
    }

    #[test]
    fn walking_camera_scroll_is_interpolated_with_the_player() {
        let rendered = RenderedViewport {
            walk_viewport_origin: Some((10, 8)),
            viewport_origin: Some((12, 8)),
            ..default()
        };

        let initial = overworld_walk_camera_offset(&rendered, WALK_FRAME_HOLD_TICKS);
        let middle = overworld_walk_camera_offset(&rendered, WALK_FRAME_HOLD_TICKS / 2);
        let final_offset = overworld_walk_camera_offset(&rendered, 0);

        assert_eq!(initial, Vec2::new(TILE_SIZE * 2.0, 0.0));
        assert_eq!(middle, Vec2::new(TILE_SIZE, 0.0));
        assert_eq!(final_offset, Vec2::ZERO);
    }

    #[test]
    fn live_walk_retains_the_viewport_texture_and_updates_every_lcd_frame() {
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
        let runtime_shell = initialize_bevy_runtime_shell(
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
        .expect("initialize walkable New Bark Town fixture");
        let mut app = integrated_shell_test_app(runtime_shell);
        app.update();

        let start_tile = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .expect("snapshot before walking")
            .overworld
            .tile;
        let (viewport_entity, viewport_texture) = {
            let world = app.world_mut();
            let mut viewport = world.query_filtered::<(Entity, &Handle<Image>), With<PlayfieldTile>>();
            let (entity, texture) = viewport
                .get_single(world)
                .expect("initial overworld has one composited viewport");
            (entity, texture.clone())
        };
        let image_count = app.world().resource::<Assets<Image>>().len();

        app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 16;
        for _ in 0..16 {
            app.update();
            let shell = app.world().resource::<BevyRuntimeShell>();
            if shell.player_walk_frame_ticks == WALK_FRAME_HOLD_TICKS
                && shell
                    .shell
                    .snapshot()
                    .expect("snapshot after walking starts")
                    .overworld
                    .tile
                    .x
                    > start_tile.x
            {
                break;
            }
        }
        assert_eq!(
            app.world().resource::<BevyRuntimeShell>().player_walk_frame_ticks,
            WALK_FRAME_HOLD_TICKS,
            "fixture must execute an authoritative walking step"
        );

        let mut player_x_positions = Vec::new();
        let mut map_x_positions = Vec::new();
        for _ in 0..WALK_FRAME_HOLD_TICKS {
            {
                let world = app.world_mut();
                let mut viewport = world.query_filtered::<
                    (Entity, &Handle<Image>, &Transform),
                    With<PlayfieldTile>,
                >();
                let (entity, texture, transform) = viewport
                    .get_single(world)
                    .expect("walking keeps one composited viewport");
                assert_eq!(entity, viewport_entity, "walking must not despawn the LCD sprite");
                assert_eq!(texture, &viewport_texture, "walking must not replace the LCD texture handle");
                map_x_positions.push(transform.translation.x);
            }
            assert_eq!(
                app.world().resource::<Assets<Image>>().len(),
                image_count,
                "walking frames must not allocate replacement viewport textures"
            );
            let x = {
                let world = app.world_mut();
                let mut players = world.query_filtered::<&Transform, With<PlayerMarker>>();
                players
                    .get_single(world)
                    .expect("player sprite remains retained while walking")
                    .translation
                    .x
            };
            player_x_positions.push(x);
            app.update();
        }
        let player_moves = player_x_positions.windows(2).any(|frame| frame[0] != frame[1]);
        let map_moves = map_x_positions.windows(2).any(|frame| frame[0] != frame[1]);
        assert!(
            player_moves || map_moves,
            "each LCD walk must visibly advance the player or retained camera: player={player_x_positions:?}, map={map_x_positions:?}"
        );
    }

    #[test]
    fn reversing_during_a_walk_refreshes_facing_without_replacing_the_lcd() {
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
        let runtime_shell = initialize_bevy_runtime_shell(
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
        .expect("initialize walkable New Bark Town fixture");
        let mut app = integrated_shell_test_app(runtime_shell);
        app.update();

        let viewport_entity = {
            let world = app.world_mut();
            let mut tiles = world.query_filtered::<Entity, With<PlayfieldTile>>();
            tiles.get_single(world).expect("initial LCD viewport")
        };
        app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 16;
        for _ in 0..16 {
            app.update();
            if app.world().resource::<BevyRuntimeShell>().player_walk_frame_ticks
                == WALK_FRAME_HOLD_TICKS
            {
                break;
            }
        }
        let right_texture = {
            let world = app.world_mut();
            let mut players = world.query_filtered::<&Handle<Image>, With<PlayerMarker>>();
            players
                .get_single(world)
                .expect("rightward walking player")
                .clone()
        };

        // Reverse before the prior eight LCD walking frames end. This is the
        // old moonwalking path: the semantic facing changed but the retained
        // sprite kept the right-facing texture.
        app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 0;
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::ArrowRight);
            keys.clear_just_pressed(KeyCode::ArrowRight);
            keys.press(KeyCode::ArrowLeft);
        }
        for _ in 0..2 {
            app.update();
            if app
                .world()
                .resource::<BevyRuntimeShell>()
                .shell
                .snapshot()
                .expect("snapshot while reversing")
                .overworld
                .facing
                == Direction::Left
            {
                break;
            }
        }

        let shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = shell.shell.snapshot().expect("snapshot after turn");
        assert_eq!(snapshot.overworld.facing, Direction::Left);
        assert!(
            shell.player_walk_frame_ticks > 0,
            "the reversal must happen inside the retained walking interval"
        );
        let _ = shell;
        let (viewport_after_turn, left_texture) = {
            let world = app.world_mut();
            let mut tiles = world.query_filtered::<Entity, With<PlayfieldTile>>();
            let viewport = tiles.get_single(world).expect("retained LCD viewport");
            let mut players = world.query_filtered::<&Handle<Image>, With<PlayerMarker>>();
            let player = players.get_single(world).expect("turned player sprite").clone();
            (viewport, player)
        };
        assert_eq!(
            viewport_after_turn, viewport_entity,
            "a turn must retain the LCD/map entity rather than flash or rebuild it"
        );
        assert_ne!(
            left_texture, right_texture,
            "reversing during a walk must replace the stale right-facing frame"
        );
        assert_eq!(
            app.world().resource::<RenderedViewport>().player_sprite_facing,
            Some(Direction::Left),
            "the retained-player identity must match its authoritative facing"
        );
    }

    #[test]
    fn autonomous_character_movement_selects_walking_sprite_frames() {
        assert!(object_sprite_is_animated("SPRITEMOVEDATA_WALK_LEFT_RIGHT"));
        assert!(object_sprite_is_animated("SPRITEMOVEDATA_WANDER"));
        assert!(object_sprite_is_animated("SPRITEMOVEDATA_SPINCLOCKWISE"));
        assert!(!object_sprite_is_animated("SPRITEMOVEDATA_STANDING_DOWN"));
        assert!(!object_sprite_is_animated("SPRITEMOVEDATA_STILL"));
    }

    #[test]
    fn chris_sprite_art_loads_real_runtime_assets() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut images = Assets::<Image>::default();
        let art =
            load_sprite_art(&asset_root, "chris", 0, "day", &mut images).expect("load chris art");

        for direction in [&art.down, &art.up, &art.left, &art.right] {
            assert_eq!(direction.standing.size, Vec2::splat(64.0));
            assert_eq!(
                direction.walking.as_ref().map(|frame| frame.size),
                Some(Vec2::splat(64.0)),
                "six-frame player sheets must preserve their walking frame"
            );
        }
        assert!(images.len() >= 8);
    }

    #[test]
    fn kris_sprite_art_loads_for_female_player_selection() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut images = Assets::<Image>::default();
        let art =
            load_sprite_art(&asset_root, "kris", 0, "day", &mut images).expect("load kris art");
        assert_eq!(art.down.standing.size, Vec2::splat(64.0));
        assert_eq!(
            art.down.walking.as_ref().map(|frame| frame.size),
            Some(Vec2::splat(64.0))
        );
    }

    #[test]
    fn overworld_colorkey_preserves_enclosed_white_character_pixels() {
        let mut source = image::RgbaImage::from_pixel(5, 5, image::Rgba([255, 255, 255, 255]));
        for row in 1..4 {
            for col in 1..4 {
                source.put_pixel(col, row, image::Rgba([0, 0, 0, 255]));
            }
        }
        let palette = [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]];
        let mut target = vec![0_u8; 5 * 5 * 4];
        copy_source_sprite_rgba(&source, 5, 0, &palette, false, &mut target);

        assert_eq!(
            target[3], 0,
            "border-connected white background is transparent"
        );
        let center_alpha = target[(2 * 5 + 2) * 4 + 3];
        assert_eq!(center_alpha, 255, "enclosed white artwork remains opaque");
    }

    #[test]
    fn intro_trigonometry_matches_the_asm_fixed_point_wave() {
        assert_eq!(visible_intro_sine(0x00, 0x20), 0);
        assert_eq!(visible_intro_sine(0x10, 0x20), 0x20);
        assert_eq!(visible_intro_sine(0x30, 0x20), -0x20);
        assert_eq!(visible_intro_cosine(0x00, 0x20), 0x20);
        assert_eq!(visible_intro_cosine(0x20, 0x20), -0x20);
        // The ASM's truncating 8.8 multiply differs from rounded floating
        // point math at this intermediate pulse position.
        assert_eq!(visible_intro_sine(0x08, 0x18), 16);
    }

    #[test]
    fn intro_framesets_preserve_asm_durations() {
        let unown = visible_intro_frameset_steps("SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1")
            .expect("Unown frameset");
        assert_eq!(
            unown.iter().map(|step| step.duration).collect::<Vec<_>>(),
            vec![3, 3, 7, 0]
        );
        let pichu = visible_intro_frameset_steps("SPRITE_ANIM_FRAMESET_INTRO_PICHU")
            .expect("Pichu frameset");
        assert_eq!(
            pichu.iter().map(|step| step.duration).collect::<Vec<_>>(),
            vec![32, 7, 7, 0]
        );
        assert!(matches!(
            pichu.last().map(|step| step.command),
            Some(IntroFrameCommand::End)
        ));
        let unown_f = visible_intro_frameset_steps("SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2")
            .expect("Unown F frameset");
        assert_eq!(
            unown_f.iter().map(|step| step.duration).collect::<Vec<_>>(),
            vec![3, 3, 3, 7, 7, 0]
        );
    }

    #[test]
    fn title_art_loads_real_runtime_assets() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut images = Assets::<Image>::default();

        let logo =
            load_title_frame(&asset_root, "logo", 1, true, &mut images).expect("load title logo");
        let crystal = load_title_frame(&asset_root, "crystal", 1, true, &mut images)
            .expect("load title crystal");
        let suicune = load_title_frame(&asset_root, "suicune", 8, true, &mut images)
            .expect("load title suicune");
        let copyright = load_title_frame(&asset_root, "copyright", 1, true, &mut images)
            .expect("load title copyright");

        assert_eq!(logo.size, Vec2::new(160.0, 64.0));
        assert_eq!(crystal.size, Vec2::new(48.0, 80.0));
        assert_eq!(suicune.size, Vec2::new(128.0, 128.0));
        assert_eq!(copyright.size, Vec2::new(232.0, 8.0));
        assert_eq!(images.len(), 4);
    }

    #[test]
    fn native_title_screen_frame_uses_title_palettes_and_window_layer() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        let title = runtime_shell.title_menu.clone().expect("title menu");
        let mut images = Assets::<Image>::default();

        let entrance_frame =
            load_title_screen_frame(&runtime_shell.asset_root, &title, &mut images)
                .expect("render title entrance frame");
        let entrance_data = images
            .get(&entrance_frame.handle)
            .expect("title entrance image")
            .data
            .clone();
        assert_eq!(entrance_frame.size, Vec2::new(160.0, 144.0));
        assert!(
            entrance_data
                .chunks_exact(4)
                .any(|pixel| pixel[3] == 255 && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)),
            "native title frame must contain real palette-colored title art"
        );

        let mut main_title = title.clone();
        main_title.phase = VisibleTitlePhase::MainMenu;
        main_title.scx = 0;
        let main_frame =
            load_title_screen_frame(&runtime_shell.asset_root, &main_title, &mut images)
                .expect("render title main frame");
        let main_data = images
            .get(&main_frame.handle)
            .expect("title main image")
            .data
            .clone();
        assert_ne!(
            entrance_data, main_data,
            "main title frame must include the version window layer absent during entrance"
        );
    }

    #[test]
    fn native_title_crystal_pixels_respect_bg_window_priority() {
        let mut crystal = image::RgbaImage::new(1, 1);
        crystal.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
        let palette: Palette = [[0, 0, 0], [80, 80, 80], [160, 160, 160], [240, 16, 32]];
        let mut target = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];
        target[0..4].copy_from_slice(&[1, 2, 3, 255]);
        let mut priority_map = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
        priority_map[0] = 2;

        blit_native_title_image_with_priority(
            &crystal,
            &palette,
            true,
            0,
            0,
            0,
            &priority_map,
            &mut target,
        );
        assert_eq!(
            &target[0..4],
            &[1, 2, 3, 255],
            "Title crystal OAM priority must not draw over non-zero BG/WIN pixels"
        );

        priority_map[0] = 0;
        blit_native_title_image_with_priority(
            &crystal,
            &palette,
            true,
            0,
            0,
            0,
            &priority_map,
            &mut target,
        );
        assert_eq!(
            &target[0..4],
            &[240, 16, 32, 255],
            "Title crystal pixels should draw over BG/WIN color index zero"
        );
    }

    #[test]
    fn visible_title_screen_spawns_only_native_frame_without_status_text_overlay() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        let mut title = runtime_shell.title_menu.clone().expect("title menu");
        title.phase = VisibleTitlePhase::PressStart;
        title.scx = 0;
        runtime_shell.title_menu = Some(title);

        let mut app = App::new();
        app.insert_resource(runtime_shell)
            .insert_resource(RenderedViewport::default())
            .insert_resource(RenderedTilesetArt::default())
            .init_resource::<Assets<Image>>()
            .add_systems(Update, render_playfield);

        app.update();

        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let mut query = app
            .world_mut()
            .query_filtered::<&Sprite, With<TitleScreenMarker>>();
        let sizes = query
            .iter(app.world())
            .filter_map(|sprite| sprite.custom_size)
            .collect::<Vec<_>>();
        assert_eq!(
            sizes,
            vec![Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)],
            "title screen should render only the native 160x144 title frame scaled to the playfield, with no Rust-only status text overlay"
        );
    }

    #[test]
    fn title_main_menu_frame_uses_typescript_window_layout_without_title_overlay() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        finish_visible_intro_screen(&mut runtime_shell, "test").expect("finish intro");
        advance_visible_title_to_press_start(&mut runtime_shell);
        open_visible_title_main_menu(&mut runtime_shell).expect("open main menu");
        let mut title = runtime_shell.title_menu.clone().expect("title menu");
        title.main_menu_frame = TITLE_MAIN_MENU_CURSOR_PERIOD;
        let mut images = Assets::<Image>::default();
        let mut rendered_art = RenderedTilesetArt::default();

        let frame = load_visible_title_main_menu_frame(
            &runtime_shell,
            &title,
            &mut rendered_art,
            &mut images,
        )
        .expect("render title main menu frame");
        assert!(rendered_art.title_menu_font_source.is_some());
        assert!(rendered_art.title_menu_frame_source.is_some());
        let image = images.get(&frame.handle).expect("main menu image");
        assert_eq!(
            image.texture_descriptor.size.width,
            (20 * SOURCE_TILE_SIZE) as u32
        );
        assert_eq!(
            image.texture_descriptor.size.height,
            (18 * SOURCE_TILE_SIZE) as u32
        );
        assert_eq!(
            &image.data[0..4],
            &[255, 255, 255, 255],
            "MainMenu draws over a white background, not the animated title art"
        );
        assert!(
            image.data.chunks_exact(4).any(|pixel| {
                pixel[3] == 255 && (pixel[0] < 64 || pixel[1] < 64 || pixel[2] < 64)
            }),
            "main menu frame should include window borders and bitmap glyph pixels"
        );
        let time_box_sample =
            ((TITLE_MAIN_MENU_TIME_BOX_Y * SOURCE_TILE_SIZE + 1) * TITLE_SCREEN_WIDTH + 1) * 4;
        assert_eq!(
            &image.data[time_box_sample..time_box_sample + 4],
            &[255, 255, 255, 255],
            "without a continue save, the TypeScript main menu does not draw the time box"
        );
    }

    #[test]
    fn title_main_menu_cursor_bobs_on_typescript_frame_period() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        finish_visible_intro_screen(&mut runtime_shell, "test").expect("finish intro");
        advance_visible_title_to_press_start(&mut runtime_shell);
        open_visible_title_main_menu(&mut runtime_shell).expect("open main menu");
        let mut title = runtime_shell.title_menu.clone().expect("title menu");
        title.main_menu_frame = TITLE_MAIN_MENU_CURSOR_PERIOD;
        let mut images = Assets::<Image>::default();
        let mut rendered_art = RenderedTilesetArt::default();

        let still = load_visible_title_main_menu_frame(
            &runtime_shell,
            &title,
            &mut rendered_art,
            &mut images,
        )
        .expect("render unbobbed main menu");
        let still_data = images
            .get(&still.handle)
            .expect("unbobbed image")
            .data
            .clone();
        title.main_menu_frame = TITLE_MAIN_MENU_CURSOR_PERIOD + TITLE_MAIN_MENU_CURSOR_PERIOD / 2;
        let bobbed = load_visible_title_main_menu_frame(
            &runtime_shell,
            &title,
            &mut rendered_art,
            &mut images,
        )
        .expect("render bobbed main menu");
        let bobbed_data = images
            .get(&bobbed.handle)
            .expect("bobbed image")
            .data
            .clone();

        assert_ne!(
            still_data, bobbed_data,
            "MainMenu cursor should bob one pixel after half the 16-frame period"
        );
        assert_eq!(visible_title_main_menu_cursor_bob(0), 0);
        assert_eq!(
            visible_title_main_menu_cursor_bob(TITLE_MAIN_MENU_CURSOR_PERIOD / 2),
            TITLE_MAIN_MENU_CURSOR_OFFSET
        );
        assert_eq!(
            visible_title_main_menu_cursor_bob(TITLE_MAIN_MENU_CURSOR_PERIOD),
            0
        );
    }

    #[test]
    fn title_main_menu_fades_in_on_typescript_speed() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        finish_visible_intro_screen(&mut runtime_shell, "test").expect("finish intro");
        advance_visible_title_to_press_start(&mut runtime_shell);
        open_visible_title_main_menu(&mut runtime_shell).expect("open main menu");
        let mut title = runtime_shell.title_menu.clone().expect("title menu");
        let mut images = Assets::<Image>::default();
        let mut rendered_art = RenderedTilesetArt::default();

        title.main_menu_frame = 0;
        let black = load_visible_title_main_menu_frame(
            &runtime_shell,
            &title,
            &mut rendered_art,
            &mut images,
        )
        .expect("render initial fade frame");
        let black_image = images.get(&black.handle).expect("initial fade image");
        assert_eq!(
            &black_image.data[0..4],
            &[0, 0, 0, 255],
            "MainMenu starts with the TypeScript fade-in overlay fully black"
        );

        title.main_menu_frame = 11;
        let clear = load_visible_title_main_menu_frame(
            &runtime_shell,
            &title,
            &mut rendered_art,
            &mut images,
        )
        .expect("render cleared fade frame");
        let clear_image = images.get(&clear.handle).expect("cleared fade image");
        assert_eq!(
            &clear_image.data[0..4],
            &[255, 255, 255, 255],
            "MainMenu fade-in clears after repeated 24-alpha steps"
        );
        assert_eq!(visible_title_main_menu_fade_alpha(0), 255);
        assert_eq!(visible_title_main_menu_fade_alpha(10), 15);
        assert_eq!(visible_title_main_menu_fade_alpha(11), 0);
    }

    #[test]
    fn title_scene_spawns_real_art_entities_from_compiled_pack() {
        fn spawn_title_scene_once(
            mut commands: Commands,
            mut runtime_shell: ResMut<BevyRuntimeShell>,
            mut rendered_art: ResMut<RenderedTilesetArt>,
            mut images: ResMut<Assets<Image>>,
        ) {
            let title = runtime_shell
                .title_menu
                .clone()
                .expect("title menu is active");
            spawn_title_screen(
                &mut commands,
                &mut runtime_shell,
                &title,
                &mut rendered_art,
                &mut images,
            )
            .expect("spawn real title screen art");
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
        let runtime_shell = initialize_bevy_runtime_shell(
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
        .expect("initialize title shell");

        let mut app = App::new();
        app.insert_resource(runtime_shell)
            .insert_resource(RenderedTilesetArt::default())
            .init_resource::<Assets<Image>>()
            .add_systems(Update, spawn_title_scene_once);
        app.update();

        let world = app.world();
        let runtime_shell = world.resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert_eq!(rendered_art.title_screen_cache.len(), 1);
        assert!(rendered_art.title_screen_errors.is_empty());
        assert_eq!(rendered_art.font_error, None);
        assert!(
            !world.resource::<Assets<Image>>().is_empty(),
            "title scene should include the composed native title frame"
        );

        let world = app.world_mut();
        let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
        assert_eq!(
            title_entities.iter(world).count(),
            1,
            "title scene should spawn one composed native title surface, with no Rust-only glyph overlay"
        );
    }

    #[test]
    fn intro_surface_preserves_the_native_lcd_aspect_at_integer_scale() {
        assert_eq!(visible_intro_display_size(), Vec2::new(640.0, 576.0));
        assert_eq!(
            visible_intro_display_size(),
            Vec2::new(
                TITLE_SCREEN_WIDTH as f32 * (TILE_SIZE / SOURCE_TILE_SIZE as f32),
                TITLE_SCREEN_HEIGHT as f32 * (TILE_SIZE / SOURCE_TILE_SIZE as f32),
            ),
            "the 160x144 LCD must be shown at four-times integer scale, never as a square"
        );
    }

    #[test]
    fn intro_scene_renderer_uses_real_asm_tilemap_art_not_debug_text() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let intro = VisibleIntroScreen::new();
        let mut rendered_art = RenderedTilesetArt::default();
        let mut images = Assets::<Image>::default();

        let frame = intro_scene_frame_for_art(&mut rendered_art, &asset_root, &intro, &mut images)
            .expect("render first intro scene from ASM tilemap art");
        let image = images.get(&frame.handle).expect("intro image asset");

        assert_eq!(
            image.texture_descriptor.size.width,
            20 * SOURCE_TILE_SIZE as u32
        );
        assert_eq!(
            image.texture_descriptor.size.height,
            18 * SOURCE_TILE_SIZE as u32
        );
        assert!(
            image.data.chunks_exact(4).any(|pixel| pixel[3] != 0),
            "intro renderer should produce visible tile pixels"
        );
        assert!(
            rendered_art.intro_presented_surface.is_some(),
            "intro rendering must retain one LCD texture instead of allocating a frame cache"
        );
        assert!(rendered_art.intro_scene_errors.is_empty());
        assert!(
            rendered_art.font_cache.is_none(),
            "intro scene rendering must not fall back to bitmap debug text"
        );

        // The opening frame is intentionally black while the first palette
        // fades in.  A settled background scene must not remain black: this
        // catches a broken palette/tilemap path rather than treating alpha as
        // evidence that the player can see the intro.
        let mut settled = intro;
        settled.jumptable_index = 3;
        settled.scene_frame_counter = 0x20;
        let settled_frame =
            intro_scene_frame_for_art(&mut rendered_art, &asset_root, &settled, &mut images)
                .expect("render settled intro background scene");
        let settled_image = images
            .get(&settled_frame.handle)
            .expect("settled intro image asset");
        assert!(
            settled_image
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[0] > 12 || pixel[1] > 12 || pixel[2] > 12),
            "a settled intro scene must contain lit Game Boy pixels"
        );
    }

    #[test]
    fn intro_scene_renderer_composites_real_oam_sprites_from_bundle() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut rendered_art = RenderedTilesetArt::default();
        let mut images = Assets::<Image>::default();

        let mut background_only = VisibleIntroScreen::new();
        background_only.jumptable_index = 6;
        let background_frame = intro_scene_frame_for_art(
            &mut rendered_art,
            &asset_root,
            &background_only,
            &mut images,
        )
        .expect("render background-only intro scene");
        let background_data = images
            .get(&background_frame.handle)
            .expect("background intro image")
            .data
            .clone();

        let mut with_sprite = background_only.clone();
        spawn_visible_intro_sprite(
            &mut with_sprite,
            "SPRITE_ANIM_OBJ_INTRO_SUICUNE",
            10 * 8,
            9 * 8,
        )
        .expect("spawn Suicune intro sprite");
        apply_visible_intro_sprite_pipeline(&mut with_sprite);
        let sprite_frame =
            intro_scene_frame_for_art(&mut rendered_art, &asset_root, &with_sprite, &mut images)
                .expect("render intro scene with sprite OAM");
        let sprite_data = &images
            .get(&sprite_frame.handle)
            .expect("sprite intro image")
            .data;

        assert_eq!(
            background_frame.handle, sprite_frame.handle,
            "intro animation should update the retained LCD texture instead of allocating a new image"
        );
        assert!(
            background_data
                .iter()
                .zip(sprite_data.iter())
                .any(|(left, right)| left != right),
            "intro sprite OAM should visibly alter the rendered frame"
        );
    }

    #[test]
    fn intro_oam_tiles_clip_at_the_lcd_edge_instead_of_wrapping() {
        let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
        let mut target = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE * 4];
        let palette = [[0, 0, 0], [10, 20, 30], [40, 50, 60], [70, 80, 90]];

        blit_intro_sprite_source_tile(
            &source,
            8,
            0,
            &palette,
            false,
            false,
            false,
            -8,
            0,
            &mut target,
        );

        let left_edge = 0;
        let wrapped_right_edge = ((32 * SOURCE_TILE_SIZE - 1) * 4) as usize;
        assert_eq!(&target[left_edge..left_edge + 4], &[0, 0, 0, 0]);
        assert_eq!(
            &target[wrapped_right_edge..wrapped_right_edge + 4],
            &[0, 0, 0, 0],
            "an offscreen OAM tile must not reappear at the opposite BG edge"
        );
    }

    #[test]
    fn intro_scene_renderer_applies_asm_palette_effects() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut rendered_art = RenderedTilesetArt::default();
        let mut images = Assets::<Image>::default();

        let mut base = VisibleIntroScreen::new();
        base.jumptable_index = 0;
        let base_frame =
            intro_scene_frame_for_art(&mut rendered_art, &asset_root, &base, &mut images)
                .expect("render base Unown intro frame");
        let base_data = images
            .get(&base_frame.handle)
            .expect("base intro image")
            .data
            .clone();

        let mut faded = base.clone();
        faded.jumptable_index = 1;
        faded.palette_effect = VisibleIntroPaletteEffect::UnownFade {
            palette_idx: 0,
            timer: 0x1f,
        };
        let faded_frame =
            intro_scene_frame_for_art(&mut rendered_art, &asset_root, &faded, &mut images)
                .expect("render faded Unown intro frame");
        let faded_data = images
            .get(&faded_frame.handle)
            .expect("faded intro image")
            .data
            .clone();
        assert!(
            base_data
                .iter()
                .zip(faded_data.iter())
                .any(|(left, right)| left != right),
            "Unown palette fade should visibly alter the rendered frame"
        );

        let intro_root = asset_root.runtime_assets().join("gfx/intro");
        let suicune_palettes =
            load_intro_palette_bank(&intro_root, "suicune").expect("load Suicune intro palettes");
        let suicune_base = suicune_palettes[0];
        let faded_suicune =
            visible_intro_effective_palette(&faded, &intro_root, "suicune", 0, &suicune_base)
                .expect("resolve faded Suicune palette");
        assert_eq!(
            faded_suicune, suicune_base,
            "unownFade only overrides the Unown palette bank; Suicune changes happen in appearUnown"
        );

        let mut appear = base.clone();
        appear.palette_effect = VisibleIntroPaletteEffect::AppearUnown {
            palette_set_idx: 0,
            revealed: 3,
        };
        let unrevealed_suicune = visible_intro_effective_palette(
            &appear,
            &intro_root,
            "suicune",
            1,
            &suicune_palettes[1],
        )
        .expect("resolve unrevealed Suicune palette");
        let revealed_suicune =
            visible_intro_effective_palette(&appear, &intro_root, "suicune", 3, &suicune_base)
                .expect("resolve revealed Suicune palette");
        let reveal_palette = load_intro_palette_bank(&intro_root, "unown_1")
            .expect("load appearUnown source palette")[0];
        assert_eq!(
            unrevealed_suicune, suicune_palettes[1],
            "appearUnown must not rewrite palettes before the first ASM target index"
        );
        assert_eq!(
            revealed_suicune, reveal_palette,
            "appearUnown writes the same hardware palette to Suicune-backed attrs"
        );

        let mut cleared = base.clone();
        cleared.palette_effect = VisibleIntroPaletteEffect::ClearBg;
        let cleared_frame =
            intro_scene_frame_for_art(&mut rendered_art, &asset_root, &cleared, &mut images)
                .expect("render cleared intro frame");
        let cleared_data = &images
            .get(&cleared_frame.handle)
            .expect("cleared intro image")
            .data;
        assert!(
            cleared_data
                .chunks_exact(4)
                .filter(|pixel| pixel[3] != 0)
                .all(|pixel| pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0),
            "clearBgPalettes should render all nontransparent BG pixels black"
        );
    }

    #[test]
    fn intro_unown_fade_does_not_recolor_obj_pulses() {
        let base_palette = [[8, 16, 24], [32, 40, 48], [56, 64, 72], [80, 88, 96]];
        let mut rendered_art = RenderedTilesetArt::default();
        rendered_art
            .intro_palette_cache
            .insert("unowns:false".to_string(), vec![base_palette]);
        let mut intro = VisibleIntroScreen::new();
        intro.palette_effect = VisibleIntroPaletteEffect::UnownFade {
            palette_idx: 0,
            timer: 0x1f,
        };

        let pulse_palette = visible_intro_effective_palette_cached(
            &intro,
            &rendered_art,
            "unowns",
            0,
            &base_palette,
            true,
        )
        .expect("resolve pulse OBJ palette");

        assert_eq!(
            pulse_palette, base_palette,
            "the BG-only Unown fade must not recolor the pulse OBJ palette"
        );
    }

    #[test]
    fn credits_parser_reads_asm_script_and_strings() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);

        let constants =
            load_visible_credit_constant_indices(&asset_root).expect("load credits constants");
        let strings = load_visible_credits_strings(&asset_root).expect("load credits strings");
        let string_tiles =
            load_visible_credits_string_tiles(&asset_root).expect("load credits string tiles");
        let script = load_visible_credits_script(&asset_root).expect("load credits script");

        let staff_index = constants.get("STAFF").copied().expect("STAFF constant");
        assert_eq!(
            strings.get(staff_index).map(String::as_str),
            Some("      #MON\n  CRYSTAL VERSION\n       STAFF")
        );
        let staff_tiles = string_tiles.get(staff_index).expect("STAFF tile rows");
        assert_eq!(staff_tiles.len(), 3);
        assert!(
            staff_tiles[0].contains(&0x54),
            "STAFF first row must preserve the ASM #MON glyph tile"
        );
        assert_eq!(
            staff_tiles[0].len(),
            "      #MON".len(),
            "credits tile parser should preserve ASM spacing"
        );
        assert!(matches!(script.first(), Some(VisibleCreditsOp::Clear)));
        assert!(
            script
                .iter()
                .any(|op| matches!(op, VisibleCreditsOp::Music))
        );
        assert!(
            script
                .iter()
                .any(|op| matches!(op, VisibleCreditsOp::TheEnd))
        );
        assert!(matches!(script.last(), Some(VisibleCreditsOp::End)));
    }

    #[test]
    fn credits_screen_opens_from_asm_and_reaches_music_opcode_by_tick() {
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
            BevyShellStart::Title {
                spawn_identifier,
                save_path: None,
            },
            BevyShellConfig {
                smoke_player_name: Some("AB".to_string()),
                ..Default::default()
            },
        )
        .expect("initialize title shell");

        open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
        {
            let credits = runtime_shell
                .credits_screen
                .as_ref()
                .expect("credits screen active");
            assert!(credits.lines.is_empty());
            assert_eq!(credits.timer, 0);
            assert_eq!(visible_credits_step_index(credits), 0);
            assert_eq!(runtime_shell.active_music.as_deref(), None);
        }

        tick_visible_credits_screen(&mut runtime_shell);
        {
            let credits = runtime_shell
                .credits_screen
                .as_ref()
                .expect("credits screen active after first tick");
            assert_eq!(
                credits.lines.first().map(|line| line.text.as_str()),
                Some("      #MON\n  CRYSTAL VERSION\n       STAFF")
            );
            assert_eq!(
                credits.lines.first().map(|line| line.tiles.len()),
                Some(3),
                "credits rendering should carry parsed ASM tile rows"
            );
            assert!(
                credits
                    .lines
                    .first()
                    .and_then(|line| line.tiles.first())
                    .is_some_and(|tiles| tiles.contains(&0x54)),
                "credits rendering should preserve the parsed #MON tile"
            );
            assert_eq!(credits.timer, 8);
            assert_eq!(visible_credits_step_index(credits), 1);
            assert_eq!(runtime_shell.active_music.as_deref(), None);
        }

        for _ in 0..7 {
            tick_visible_credits_screen(&mut runtime_shell);
        }
        {
            let credits = runtime_shell
                .credits_screen
                .as_ref()
                .expect("credits screen active after first jumptable pass");
            assert_eq!(
                credits.timer, 8,
                "wait counters should only decrement on the parse step, not every frame"
            );
            assert_eq!(credits.ly_override, 0xfe);
            assert_eq!(runtime_shell.active_music.as_deref(), None);
        }

        for _ in 0..140 {
            if runtime_shell.active_music.as_deref() == Some("MUSIC_CREDITS") {
                break;
            }
            tick_visible_credits_screen(&mut runtime_shell);
        }

        let credits = runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen still active");
        assert_eq!(credits.timer, 10);
        assert_eq!(visible_credits_step_index(credits), 1);
        assert_eq!(runtime_shell.active_music.as_deref(), Some("MUSIC_CREDITS"));
        assert!(
            runtime_shell
                .pending_audio
                .iter()
                .any(|command| command.audio_id == "MUSIC_CREDITS"),
            "credits music should be queued from the parsed CREDITS_MUSIC opcode"
        );
    }

    #[test]
    fn credits_frame_renders_real_assets_and_special_font_tiles() {
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
            asset_root.clone(),
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
        .expect("initialize title shell");

        open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
        tick_visible_credits_screen(&mut runtime_shell);
        let credits = runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen active");
        let font = load_visible_credits_font_tiles(&asset_root).expect("load credits font");
        assert!(
            font.levels.contains_key(&0x54),
            "credits font must include the #MON Poke glyph tile used by STAFF"
        );

        let mut images = Assets::<Image>::default();
        let frame = render_visible_credits_frame(&asset_root, credits, &mut images)
            .expect("render credits");
        let image = images.get(&frame.handle).expect("credits image handle");
        assert_eq!(
            image.texture_descriptor.size.width,
            CREDITS_SCREEN_WIDTH as u32
        );
        assert_eq!(
            image.texture_descriptor.size.height,
            CREDITS_SCREEN_HEIGHT as u32
        );
        let unique_colors = image
            .data
            .chunks_exact(4)
            .filter(|rgba| rgba[3] != 0)
            .map(|rgba| [rgba[0], rgba[1], rgba[2]])
            .collect::<BTreeSet<_>>();
        assert!(
            unique_colors.len() >= 3,
            "first credits frame should contain tinted background, border, and text colors"
        );

        let mut staged_credits = credits.clone();
        staged_credits.border_frame_top = Some(VisibleCreditsBorderFrame {
            mon_index: 0,
            frame_index: 0,
        });
        staged_credits.border_frame_bottom = Some(VisibleCreditsBorderFrame {
            mon_index: 0,
            frame_index: 0,
        });
        let staged_frame = render_visible_credits_frame(&asset_root, &staged_credits, &mut images)
            .expect("render staged credits mon frame");
        let staged_image = images
            .get(&staged_frame.handle)
            .expect("staged credits image handle");
        let staged_unique_colors = staged_image
            .data
            .chunks_exact(4)
            .filter(|rgba| rgba[3] != 0)
            .map(|rgba| [rgba[0], rgba[1], rgba[2]])
            .collect::<BTreeSet<_>>();
        assert!(
            staged_unique_colors.len() >= 4,
            "staged credits frame should include tinted mon strip colors from real assets"
        );
    }

    fn press_key_for_runtime_hotkey_app(app: &mut App, key: KeyCode) {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(key);
        }
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(key);
            keys.clear_just_pressed(key);
        }
    }

    fn skip_intro_for_test(app: &mut App) {
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .intro_screen
            .is_some()
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            skip_visible_intro_screen(&mut runtime_shell, GameButton::Start)
                .expect("skip intro for title test setup");
        }
        assert!(
            app.world()
                .resource::<BevyRuntimeShell>()
                .intro_screen
                .is_none(),
            "intro should be skipped before title-only test setup"
        );
    }

    fn advance_title_to_press_start_for_test(app: &mut App) {
        skip_intro_for_test(app);
        for _ in 0..40 {
            app.update();
            let ready = app
                .world()
                .resource::<BevyRuntimeShell>()
                .title_menu
                .as_ref()
                .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::PressStart));
            if ready {
                return;
            }
        }
        panic!("title did not reach PRESS START phase");
    }

    fn open_title_main_menu_for_test(app: &mut App) {
        advance_title_to_press_start_for_test(app);
        press_key_for_runtime_hotkey_app(app, KeyCode::Enter);
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            runtime_shell
                .title_menu
                .as_ref()
                .is_some_and(visible_title_main_menu_ready),
            "title Start should open the title main menu"
        );
    }

    #[test]
    fn visible_title_launch_starts_with_crystal_intro_before_title() {
        let runtime_shell = core_modular_title_shell_for_test();

        assert!(runtime_shell.intro_screen.is_some());
        let intro = runtime_shell.intro_screen.as_ref().expect("intro screen");
        assert_eq!(intro.jumptable_index, 0);
        assert_eq!(intro.scene_frame_counter, 0);
        assert_eq!(intro.scene_name(), "unown_a");
        assert!(
            runtime_shell
                .title_menu
                .as_ref()
                .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::Entrance)),
            "title state should be staged behind the intro"
        );
    }

    #[test]
    fn visible_intro_live_skip_enters_title_entrance() {
        let runtime_shell = core_modular_title_shell_for_test();
        let mut app = integrated_shell_test_app(runtime_shell);

        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert!(runtime_shell.intro_screen.is_some());
            assert!(runtime_shell.title_menu.is_some());
        }
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(runtime_shell.last_error, None);
            let title = runtime_shell
                .title_menu
                .as_ref()
                .expect("title should remain after intro skip");
            assert!(matches!(
                title.phase,
                VisibleTitlePhase::Entrance
                    | VisibleTitlePhase::Timer
                    | VisibleTitlePhase::PressStart
            ));
            assert!(
                runtime_shell
                    .last_audio_events
                    .iter()
                    .any(|event| event.contains("intro skip")),
                "intro skip should be logged"
            );
        }
    }

    #[test]
    fn intro_title_handoff_clears_fade_and_old_audio_before_title_cue() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.screen_fade = Some(VisibleScreenFade::new(
            ScriptFadeColor::Black,
            ScriptFadeDirection::Out,
            8,
        ));
        runtime_shell.pending_audio.push(BevyAudioCommand {
            audio_id: "MUSIC_CRYSTAL_OPENING".to_string(),
            kind: ModpackAudioKind::Music,
            mode: ModpackAudioPlaybackMode::RawPcm,
            looped: true,
        });

        finish_visible_intro_screen(&mut runtime_shell, "test").expect("finish intro");

        assert!(runtime_shell.screen_fade.is_none());
        assert!(runtime_shell.pending_music_stop);
        assert_eq!(runtime_shell.active_music.as_deref(), Some("MUSIC_NONE"));
        assert_eq!(
            runtime_shell
                .pending_audio
                .iter()
                .map(|command| (command.kind, command.audio_id.as_str()))
                .collect::<Vec<_>>(),
            vec![(ModpackAudioKind::SoundEffect, "SFX_TITLE_SCREEN_ENTRANCE")],
            "only the title entrance cue may survive the intro audio boundary"
        );
    }

    #[test]
    fn visible_title_delete_save_combo_opens_prompt_from_press_start() {
        let runtime_shell = core_modular_title_shell_for_test();
        let mut app = integrated_shell_test_app(runtime_shell);
        advance_title_to_press_start_for_test(&mut app);

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftRight);
            keys.press(KeyCode::KeyX);
            keys.press(KeyCode::ArrowUp);
        }
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::ShiftRight);
            keys.release(KeyCode::KeyX);
            keys.release(KeyCode::ArrowUp);
            keys.clear_just_pressed(KeyCode::ShiftRight);
            keys.clear_just_pressed(KeyCode::KeyX);
            keys.clear_just_pressed(KeyCode::ArrowUp);
        }

        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell
                .pending_delete_save
                .as_ref()
                .map(|screen| screen.selected_index),
            Some(1),
            "Up+B+Select should open the delete-save prompt with NO selected"
        );
        assert!(
            runtime_shell
                .title_menu
                .as_ref()
                .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::PressStart)),
            "delete-save prompt should sit over the press-start title state"
        );
    }

    #[test]
    fn visible_delete_save_confirm_removes_configured_save_and_restarts_title() {
        let save_path = std::env::temp_dir().join(format!(
            "crystal-bevy-delete-save-{}.crystalsave",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&save_path);
        std::fs::write(&save_path, b"delete me").expect("write test save file");

        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell
            .title_menu
            .as_mut()
            .expect("title menu")
            .save_path = Some(save_path.clone());
        finish_visible_intro_screen(&mut runtime_shell, "test").expect("finish intro");
        advance_visible_title_to_press_start(&mut runtime_shell);

        open_visible_delete_save_screen(&mut runtime_shell).expect("open delete save");
        move_visible_delete_save_cursor(&mut runtime_shell).expect("select YES");
        confirm_visible_delete_save_screen(&mut runtime_shell).expect("confirm delete save");

        assert!(
            !save_path.exists(),
            "YES on delete-save prompt should delete the configured .crystalsave"
        );
        assert!(runtime_shell.pending_delete_save.is_none());
        assert!(
            runtime_shell
                .title_menu
                .as_ref()
                .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::Entrance)),
            "delete-save confirmation should return through a fresh title entrance"
        );
    }

    #[test]
    fn delete_save_prompt_frame_uses_boot_window_and_default_no_cursor() {
        let runtime_shell = core_modular_title_shell_for_test();
        let mut images = Assets::<Image>::default();
        let frame = load_delete_save_frame(
            &runtime_shell.asset_root,
            &VisibleDeleteSaveScreen { selected_index: 1 },
            &mut images,
        )
        .expect("render delete-save prompt");
        let image = images.get(&frame.handle).expect("delete-save image");
        assert_eq!(
            image.texture_descriptor.size.width,
            (20 * SOURCE_TILE_SIZE) as u32
        );
        assert_eq!(
            image.texture_descriptor.size.height,
            (18 * SOURCE_TILE_SIZE) as u32
        );
        assert!(
            image
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[3] != 0 && (pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0)),
            "delete-save prompt should render real window/text pixels over the black background"
        );
    }

    #[test]
    fn visible_title_clock_reset_combo_opens_prompt_after_select_release() {
        let runtime_shell = core_modular_title_shell_for_test();
        let mut app = integrated_shell_test_app(runtime_shell);
        advance_title_to_press_start_for_test(&mut app);

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftRight);
            keys.press(KeyCode::KeyX);
            keys.press(KeyCode::ArrowDown);
        }
        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert!(
                runtime_shell
                    .title_menu
                    .as_ref()
                    .is_some_and(|title| title.clock_reset_trigger),
                "Down+B+Select should arm clock reset on the title screen"
            );
            assert!(runtime_shell.pending_clock_reset.is_none());
        }

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::ShiftRight);
            keys.release(KeyCode::KeyX);
            keys.release(KeyCode::ArrowDown);
            keys.clear_just_pressed(KeyCode::ShiftRight);
            keys.clear_just_pressed(KeyCode::KeyX);
            keys.clear_just_pressed(KeyCode::ArrowDown);
            keys.press(KeyCode::ArrowLeft);
            keys.press(KeyCode::ArrowUp);
        }
        app.update();

        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            runtime_shell
                .pending_clock_reset
                .as_ref()
                .is_some_and(|screen| matches!(screen.phase, VisibleClockResetPhase::Confirm)),
            "releasing Select while holding Left+Up should open the clock-reset prompt"
        );
    }

    #[test]
    fn visible_clock_reset_flow_commits_manual_time_and_restarts_title() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        finish_visible_intro_screen(&mut runtime_shell, "test").expect("finish intro");
        advance_visible_title_to_press_start(&mut runtime_shell);

        open_visible_clock_reset_screen(&mut runtime_shell).expect("open clock reset");
        move_visible_clock_reset_cursor(&mut runtime_shell, 1).expect("select YES");
        confirm_visible_clock_reset_screen(&mut runtime_shell).expect("enter day phase");
        assert!(
            runtime_shell
                .pending_clock_reset
                .as_ref()
                .is_some_and(|screen| matches!(screen.phase, VisibleClockResetPhase::SetDay))
        );
        move_visible_clock_reset_cursor(&mut runtime_shell, 1).expect("increment day");
        confirm_visible_clock_reset_screen(&mut runtime_shell).expect("enter hour phase");
        move_visible_clock_reset_cursor(&mut runtime_shell, 1).expect("increment hour");
        confirm_visible_clock_reset_screen(&mut runtime_shell).expect("enter minute phase");
        move_visible_clock_reset_cursor(&mut runtime_shell, 1).expect("increment minute");
        confirm_visible_clock_reset_screen(&mut runtime_shell).expect("commit clock reset");

        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after clock reset");
        assert_eq!(snapshot.progression.time.day_of_week, 1);
        assert_eq!(snapshot.progression.time.game_time_hours, 1);
        assert_eq!(snapshot.progression.time.game_time_minutes, 1);
        assert!(runtime_shell.pending_clock_reset.is_none());
        assert!(
            runtime_shell
                .title_menu
                .as_ref()
                .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::Entrance)),
            "clock reset confirmation should return through a fresh title entrance"
        );
    }

    #[test]
    fn clock_reset_prompt_frame_uses_boot_window_and_value_phases() {
        let runtime_shell = core_modular_title_shell_for_test();
        let mut images = Assets::<Image>::default();
        let confirm = load_clock_reset_frame(
            &runtime_shell.asset_root,
            &VisibleClockResetScreen {
                phase: VisibleClockResetPhase::Confirm,
                confirm_selection: 1,
                day: 0,
                hour: 0,
                minute: 0,
            },
            &mut images,
        )
        .expect("render clock confirm prompt");
        let day = load_clock_reset_frame(
            &runtime_shell.asset_root,
            &VisibleClockResetScreen {
                phase: VisibleClockResetPhase::SetDay,
                confirm_selection: 0,
                day: 2,
                hour: 3,
                minute: 4,
            },
            &mut images,
        )
        .expect("render clock day prompt");
        let confirm_image = images.get(&confirm.handle).expect("confirm image");
        let day_image = images.get(&day.handle).expect("day image");
        assert_eq!(
            confirm_image.texture_descriptor.size.width,
            (20 * SOURCE_TILE_SIZE) as u32
        );
        assert_eq!(
            confirm_image.texture_descriptor.size.height,
            (18 * SOURCE_TILE_SIZE) as u32
        );
        assert_ne!(
            confirm_image.data, day_image.data,
            "clock reset value phases should render different prompt contents"
        );
    }

    #[test]
    fn visible_intro_completes_to_title_within_typescript_frame_budget() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        let mut saw_opening_music_command = false;
        let mut saw_final_whoosh_command = false;

        for _ in 0..2400 {
            tick_visible_intro_screen(&mut runtime_shell).expect("tick intro state machine");
            saw_opening_music_command |= runtime_shell.pending_audio.iter().any(|command| {
                command.audio_id == "MUSIC_CRYSTAL_OPENING"
                    && matches!(command.kind, ModpackAudioKind::Music)
            });
            saw_final_whoosh_command |= runtime_shell.pending_audio.iter().any(|command| {
                command.audio_id == "SFX_INTRO_WHOOSH"
                    && matches!(command.kind, ModpackAudioKind::SoundEffect)
            });
            if runtime_shell.intro_screen.is_none() {
                assert!(runtime_shell.title_menu.is_some());
                assert!(
                    runtime_shell
                        .last_audio_events
                        .iter()
                        .any(|event| event.contains("intro complete")),
                    "intro completion should be logged"
                );
                assert!(
                    saw_final_whoosh_command,
                    "final intro whoosh must be queued through compiled-pack sound playback"
                );
                assert!(
                    saw_opening_music_command,
                    "crystal opening music must be queued through compiled-pack music playback"
                );
                return;
            }
        }

        let intro = runtime_shell
            .intro_screen
            .as_ref()
            .expect("intro stalled before title");
        panic!(
            "intro stalled at scene {} {} frame {}",
            intro.jumptable_index,
            intro.scene_name(),
            intro.scene_frame_counter
        );
    }

    #[test]
    fn visible_intro_never_loses_its_lcd_surface_during_a_full_sequence() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        let mut rendered_art = RenderedTilesetArt::default();
        let mut images = Assets::<Image>::default();
        let mut saw_lit_lcd_pixel = false;

        for _ in 0..2400 {
            if let Some(intro) = runtime_shell.intro_screen.as_ref() {
                let frame = intro_scene_frame_for_art_with_bundle(
                    &mut rendered_art,
                    &runtime_shell.asset_root,
                    runtime_shell
                        .shell
                        .runtime()
                        .data()
                        .sprite_anim_bundle
                        .as_str(),
                    intro,
                    &mut images,
                )
                .unwrap_or_else(|| {
                    let key = intro_scene_art_key(intro);
                    let error = rendered_art
                        .intro_scene_errors
                        .get(&key)
                        .map(String::as_str)
                        .unwrap_or("no composition error was recorded");
                    panic!(
                        "intro scene {} ({}) frame {} failed to compose: {error:#}",
                        intro.jumptable_index,
                        intro.scene_name(),
                        intro.scene_frame_counter,
                    )
                });
                let image = images
                    .get(&frame.handle)
                    .expect("composed intro frame must retain its image handle");
                saw_lit_lcd_pixel |= image
                    .data
                    .chunks_exact(4)
                    .any(|rgba| rgba[0] > 12 || rgba[1] > 12 || rgba[2] > 12);
                assert!(
                    rendered_art.intro_scene_errors.is_empty(),
                    "intro composition must not degrade into an error/black surface: {:?}",
                    rendered_art.intro_scene_errors
                );
            }
            tick_visible_intro_screen(&mut runtime_shell).expect("tick intro state machine");
            if runtime_shell.intro_screen.is_none() {
                assert!(
                    saw_lit_lcd_pixel,
                    "the complete intro must show visible LCD pixels"
                );
                return;
            }
        }
        panic!("intro did not finish within its source sequence budget");
    }

    fn complete_time_set_for_test(app: &mut App) {
        for _ in 0..64 {
            if app
                .world()
                .resource::<BevyRuntimeShell>()
                .pending_time_set
                .is_none()
            {
                return;
            }
            {
                let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
                if let Some(time_set) = runtime_shell.pending_time_set.as_mut() {
                    if !visible_time_set_dialog_text(time_set).is_empty()
                        && !visible_time_set_dialog_complete(time_set)
                    {
                        advance_visible_time_set_dialog(time_set);
                    }
                }
            }
            press_key_for_runtime_hotkey_app(app, KeyCode::Enter);
        }
        panic!("time set screen did not complete");
    }

    fn complete_oak_intro_for_test(app: &mut App) {
        for _ in 0..256 {
            if app
                .world()
                .resource::<BevyRuntimeShell>()
                .pending_oak_intro
                .is_none()
            {
                if app
                    .world()
                    .resource::<BevyRuntimeShell>()
                    .pending_name_choice
                    .is_some()
                {
                    press_key_for_runtime_hotkey_app(app, KeyCode::Enter);
                }
                return;
            }
            {
                let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
                tick_visible_oak_intro(&mut runtime_shell).expect("tick Oak intro");
                if let Some(oak_intro) = runtime_shell.pending_oak_intro.as_mut() {
                    if !oak_intro.current_text.is_empty()
                        && !visible_oak_intro_dialog_complete(oak_intro)
                    {
                        oak_intro.visible_chars = oak_intro.current_text.chars().count();
                        oak_intro.waiting_for_input = true;
                    }
                }
                let should_press = runtime_shell
                    .pending_oak_intro
                    .as_ref()
                    .is_some_and(|oak_intro| oak_intro.waiting_for_input || oak_intro.finished);
                if should_press {
                    press_visible_oak_intro_a_button(&mut runtime_shell)
                        .expect("advance Oak intro");
                }
            }
            app.update();
        }
        panic!("Oak intro did not complete");
    }

    fn finish_current_oak_intro_page_for_test(runtime_shell: &mut BevyRuntimeShell) {
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro pending");
            assert!(
                !oak_intro.current_text.is_empty(),
                "expected an Oak intro text page"
            );
            oak_intro.visible_chars = oak_intro.current_text.chars().count();
            oak_intro.waiting_for_input = true;
        }
        press_visible_oak_intro_a_button(runtime_shell).expect("advance Oak intro page");
    }

    fn confirm_gender_for_test(app: &mut App, expected: VisiblePlayerGender) {
        press_key_for_runtime_hotkey_app(app, KeyCode::Enter);
        for _ in 0..=usize::from(VISIBLE_GENDER_CONFIRM_DELAY_FRAMES) + 1 {
            app.update();
            if app
                .world()
                .resource::<BevyRuntimeShell>()
                .pending_gender_selection
                .is_none()
            {
                break;
            }
        }
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.selected_player_gender, Some(expected));
        let snapshot = runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after gender confirmation");
        assert_eq!(
            snapshot.trainer.player_gender,
            visible_player_gender_value(expected)
        );
        assert!(
            runtime_shell.pending_gender_selection.is_none(),
            "confirmed gender should close before the Oak intro clock"
        );
        assert_eq!(runtime_shell.last_error, None);
        let _ = runtime_shell;
        complete_time_set_for_test(app);
        complete_oak_intro_for_test(app);
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            runtime_shell.pending_name_input.is_some(),
            "completed Oak intro should open player name input"
        );
    }

    #[derive(Resource)]
    struct HeldArrowRightTestFrames(u8);

    fn inject_held_arrow_right_for_test(
        mut keys: ResMut<ButtonInput<KeyCode>>,
        mut frames: ResMut<HeldArrowRightTestFrames>,
    ) {
        if frames.0 == 0 {
            keys.release(KeyCode::ArrowRight);
            return;
        }
        keys.press(KeyCode::ArrowRight);
        frames.0 -= 1;
    }

    fn integrated_shell_test_app(runtime_shell: BevyRuntimeShell) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(ClearColor(Color::rgb(0.05, 0.07, 0.06)))
            .insert_resource(runtime_shell)
            // Each MinimalPlugins update is an explicit deterministic Game
            // Boy frame in these integration tests.  Using wall-clock delta
            // here makes confirmations advance only on some updates and
            // turns the title flow into a flaky, visibly slow UI.
            .insert_resource(RuntimeTickTimer::new(0.0))
            .insert_resource(VisibleSequenceTickClock::deterministic_test())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(HeldArrowRightTestFrames(0))
            .init_resource::<Assets<AudioSource>>()
            .init_resource::<Assets<Image>>()
            .insert_resource(RenderedViewport::default())
            .insert_resource(RenderedTilesetArt::default())
            .insert_resource(HudMode::Status)
            .add_systems(Startup, setup_shell_view)
            .add_systems(Update, inject_held_arrow_right_for_test)
            .add_systems(
                Update,
                apply_keyboard_input.after(inject_held_arrow_right_for_test),
            )
            .add_systems(Update, apply_runtime_hotkeys.after(apply_keyboard_input))
            .add_systems(
                Update,
                drain_runtime_audio_events.after(apply_runtime_hotkeys),
            )
            .add_systems(
                Update,
                tick_visible_title_screen.after(drain_runtime_audio_events),
            )
            .add_systems(
                Update,
                sync_runtime_title_music.after(tick_visible_title_screen),
            )
            .add_systems(
                Update,
                sync_runtime_battle_music.after(sync_runtime_title_music),
            )
            .add_systems(
                Update,
                sync_runtime_current_music.after(sync_runtime_battle_music),
            )
            .add_systems(
                Update,
                queue_battle_intro_cry.after(sync_runtime_current_music),
            )
            .add_systems(Update, play_pending_audio.after(queue_battle_intro_cry))
            .add_systems(Update, render_playfield.after(play_pending_audio))
            .add_systems(Update, refresh_status_text.after(render_playfield))
            .add_systems(Update, refresh_dialog_text.after(refresh_status_text))
            .add_systems(Update, refresh_battle_text.after(refresh_dialog_text))
            .add_systems(Update, refresh_shell_panels.after(refresh_battle_text));
        app
    }

    fn core_modular_title_shell_for_test() -> BevyRuntimeShell {
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
    fn compiled_pack_contains_complete_npc_trade_payloads() {
        let shell = core_modular_title_shell_for_test();
        let snapshot = shell.shell.snapshot().expect("snapshot compiled pack");
        let mike = snapshot
            .special
            .npc_trades
            .get("NPC_TRADE_MIKE")
            .expect("Mike NPC trade");
        assert_eq!(mike.requested_species, "ABRA");
        assert_eq!(mike.offered_species, "MACHOP");
        assert_eq!(mike.nickname, "MUSCLE");
        assert_eq!(mike.dvs, vec![0x37, 0x66]);
        assert_eq!(mike.held_item, "GOLD_BERRY");
        assert_eq!(mike.original_trainer_id, 37460);
        assert_eq!(mike.original_trainer_name, "MIKE");
        let tower = shell
            .shell
            .runtime()
            .data()
            .battle_tower_rules
            .as_ref()
            .expect("compiled Battle Tower roster");
        assert_eq!(tower.trainers.len(), 70);
        assert_eq!(tower.mon_groups.len(), 10);
        assert!(tower.mon_groups.iter().all(|group| group.len() == 21));
        assert_eq!(tower.mon_groups[0][0].species, "JOLTEON");
    }

    #[test]
    fn real_pack_battle_tower_loads_canonical_roster_and_party() {
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
        let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
            asset_root,
            runtime,
            1,
            "BattleTower1F",
            8,
            5,
        )
        .expect("start Battle Tower map shell");
        for species in ["CYNDAQUIL", "TOTODILE", "CHIKORITA"] {
            shell
                .add_party_pokemon(
                    species,
                    10,
                    None,
                    None,
                    "PLAYER",
                    1,
                    Dv::from_non_hp(10, 10, 10, 10),
                )
                .expect("add Battle Tower party member");
        }
        shell
            .apply_battle_tower_action(
                "BATTLETOWERACTION_SAVELEVELGROUP".to_string(),
                Some(1),
                None,
            )
            .expect("select level group");
        shell
            .load_battle_tower_opponent_special(
                "caller_supplied_id".to_string(),
                "caller_supplied_sprite".to_string(),
                "OBJECT_EVENT_1".to_string(),
            )
            .expect("load canonical Battle Tower opponent");
        let snapshot = shell.snapshot().expect("snapshot Battle Tower battle");
        let battle = snapshot.battle.expect("active Battle Tower battle");
        assert_eq!(battle.battle_type, "BATTLETYPE_BATTLE_TOWER");
        assert_eq!(battle.enemy_party.len(), 3);
        assert!(battle.enemy_party.iter().all(|pokemon| pokemon.level == 10));
        let RuntimeBattleKind::Trainer { trainer_id, .. } = battle.kind else {
            panic!("Battle Tower must use trainer battle kind");
        };
        assert_ne!(trainer_id, "caller_supplied_id");
        assert!(trainer_id.starts_with("BATTLE_TOWER_"));
    }

    #[test]
    fn npc_trade_runtime_exchanges_matching_party_pokemon() {
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
        let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
            asset_root,
            runtime,
            1,
            "GoldenrodDeptStore5F",
            5,
            5,
        )
        .expect("start trade map shell");
        shell
            .add_party_pokemon(
                "ABRA",
                18,
                None,
                None,
                "PLAYER",
                1,
                Dv::from_non_hp(10, 10, 10, 10),
            )
            .expect("add requested Abra");
        shell
            .add_party_pokemon(
                "CHIKORITA",
                12,
                None,
                None,
                "PLAYER",
                1,
                Dv::from_non_hp(10, 10, 10, 10),
            )
            .expect("add party member after requested Abra");
        let command = shell
            .script_runtime_command_keys()
            .into_iter()
            .find(|key| key.command == "trade" && key.args == vec!["NPC_TRADE_MIKE"])
            .expect("Mike trade command");
        shell
            .apply_compiled_script_command(
                &command.source_script,
                command.command_index,
                ScriptRuntimeInputs {
                    selected_party_index: Some(0),
                    ..ScriptRuntimeInputs::default()
                },
                ScriptPhoneInputs::default(),
            )
            .expect("execute Mike trade");
        let snapshot = shell.snapshot().expect("snapshot exchanged party");
        let traded = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.pokemon.species.id == "MACHOP")
            .map(|slot| &slot.pokemon)
            .expect("traded party slot");
        assert_eq!(traded.species.id, "MACHOP");
        assert_eq!(traded.nickname, "MUSCLE");
        assert_eq!(traded.item.as_deref(), Some("GOLD_BERRY"));
        assert_eq!(traded.original_trainer_name, "MIKE");
        assert_eq!(traded.original_trainer_id, 37460);
        assert_eq!(snapshot.party.slots.len(), 2);
        assert_eq!(snapshot.party.slots[0].pokemon.species.id, "CHIKORITA");
        assert_eq!(snapshot.party.slots[1].pokemon.species.id, "MACHOP");
        shell
            .apply_compiled_script_command(
                &command.source_script,
                command.command_index,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("completed trade should use post-trade path");
        let second = shell.snapshot().expect("snapshot repeated trade");
        assert_eq!(second.party.slots.len(), 2);
        assert_eq!(second.party.slots[0].pokemon.species.id, "CHIKORITA");
        assert_eq!(second.party.slots[1].pokemon.species.id, "MACHOP");
    }

    #[test]
    fn time_set_directional_input_matches_typescript_phase_rules() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        open_visible_time_set_screen(&mut runtime_shell, VisibleTimeSetNext::OakIntro)
            .expect("open time set");
        {
            let time_set = runtime_shell
                .pending_time_set
                .as_mut()
                .expect("time set pending");
            time_set.phase = VisibleTimeSetPhase::SetHour;
            time_set.hour = 10;
        }

        move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Right)
            .expect("right ignored on hour");
        assert_eq!(
            runtime_shell
                .pending_time_set
                .as_ref()
                .expect("time set pending")
                .hour,
            10,
            "TypeScript only changes the hour on up/down"
        );
        move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Up)
            .expect("up increments hour");
        assert_eq!(
            runtime_shell
                .pending_time_set
                .as_ref()
                .expect("time set pending")
                .hour,
            11
        );
        move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Left)
            .expect("left ignored on hour");
        assert_eq!(
            runtime_shell
                .pending_time_set
                .as_ref()
                .expect("time set pending")
                .hour,
            11
        );

        {
            let time_set = runtime_shell
                .pending_time_set
                .as_mut()
                .expect("time set pending");
            time_set.phase = VisibleTimeSetPhase::SetMinute;
            time_set.minute = 0;
        }
        move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Right)
            .expect("right increments minute");
        assert_eq!(
            runtime_shell
                .pending_time_set
                .as_ref()
                .expect("time set pending")
                .minute,
            1,
            "TypeScript changes the minute on right"
        );
        move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Left)
            .expect("left decrements minute");
        assert_eq!(
            runtime_shell
                .pending_time_set
                .as_ref()
                .expect("time set pending")
                .minute,
            0,
            "TypeScript changes the minute on left"
        );
    }

    #[test]
    fn time_set_renders_real_boot_window_font_and_arrow_assets() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        open_visible_time_set_screen(&mut runtime_shell, VisibleTimeSetNext::OakIntro)
            .expect("open time set");
        {
            let time_set = runtime_shell
                .pending_time_set
                .as_mut()
                .expect("time set pending");
            time_set.phase = VisibleTimeSetPhase::SetHour;
            time_set.visible_chars = 0;
        }
        let mut images = Assets::<Image>::default();
        let frame = load_time_set_frame(
            &runtime_shell.asset_root,
            runtime_shell
                .pending_time_set
                .as_ref()
                .expect("time set pending"),
            &mut images,
        )
        .expect("render time-set frame from boot assets");
        let image = images.get(&frame.handle).expect("time-set image");

        assert_eq!(image.texture_descriptor.size.width, 160);
        assert_eq!(image.texture_descriptor.size.height, 144);
        assert!(
            image
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[0] < 16 && pixel[1] < 16 && pixel[2] < 16 && pixel[3] == 255),
            "time-set screen must contain black font/frame pixels from real assets"
        );
        assert!(
            image
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[0] == 255 && pixel[1] == 255 && pixel[2] == 255),
            "time-set screen must contain boot window fill from the real textbox layout"
        );
    }

    #[test]
    fn oak_intro_renders_real_oak_wooper_and_player_portraits() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
        assert!(
            runtime_shell
                .pending_audio
                .iter()
                .any(|command| command.audio_id == "MUSIC_ROUTE_30"
                    && matches!(command.kind, ModpackAudioKind::Music)),
            "Oak intro must queue TypeScript/ASM Route 30 music, not Route 29 or overworld music"
        );
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event.contains("queued Oak intro music MUSIC_ROUTE_30")),
            "{:?}",
            runtime_shell.last_audio_events
        );
        let mut app = App::new();
        app.insert_resource(runtime_shell)
            .insert_resource(RenderedViewport::default())
            .insert_resource(RenderedTilesetArt::default())
            .init_resource::<Assets<Image>>()
            .add_systems(Update, render_playfield);

        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(runtime_shell.last_error, None);
            let rendered_art = app.world().resource::<RenderedTilesetArt>();
            assert!(
                rendered_art.intro_cache.contains_key(&IntroArtKey {
                    asset_id: "trainer:oak".to_string(),
                }),
                "Oak intro must render Professor Oak from gfx/trainers/oak.png, not a label"
            );
            let mut query = app
                .world_mut()
                .query_filtered::<&Sprite, With<TitleScreenMarker>>();
            let sizes = query
                .iter(app.world())
                .filter_map(|sprite| sprite.custom_size)
                .collect::<Vec<_>>();
            assert_eq!(
                sizes,
                vec![Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)],
                "Oak intro should spawn one composed native 160x144 surface scaled to the shared playfield"
            );
        }

        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro pending");
            oak_intro.scene_phase = VisibleOakIntroPhase::Text;
            oak_intro.fade_active = false;
            oak_intro.fade_alpha = 0;
            oak_intro.current_text = "Hello! Sorry to\nkeep you waiting!".to_string();
            oak_intro.visible_chars = oak_intro.current_text.chars().count();
            oak_intro.waiting_for_input = true;
            oak_intro.blink_timer = 0;
        }
        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Oak intro pending");
            let trainer = runtime_shell
                .shell
                .snapshot()
                .expect("Oak intro snapshot")
                .trainer;
            let key = oak_intro_art_key(oak_intro, trainer.player_gender);
            let rendered_art = app.world().resource::<RenderedTilesetArt>();
            let oak_frame = rendered_art
                .oak_intro_cache
                .get(&key)
                .expect("Oak intro frame cache");
            assert_eq!(oak_frame.size, Vec2::new(160.0, 144.0));
            let handle = oak_frame.handle.clone();
            let image = app
                .world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .expect("Oak intro composed image");
            let has_textbox_text = (OAK_INTRO_TEXTBOX_Y * SOURCE_TILE_SIZE..144).any(|y| {
                (0..160).any(|x| {
                    let offset = (y * 160 + x) * 4;
                    image.data[offset] < 24
                        && image.data[offset + 1] < 24
                        && image.data[offset + 2] < 24
                        && image.data[offset + 3] == 255
                })
            });
            assert!(
                has_textbox_text,
                "Oak intro dialog must be drawn into the composed native frame, not loose Bevy glyph sprites"
            );
            let visible_arrow_pixels = oak_intro_prompt_arrow_dark_pixels(image);
            assert!(
                visible_arrow_pixels > 0,
                "Oak intro prompt arrow must render during the visible half of the TypeScript blink cycle"
            );
        }

        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro pending");
            oak_intro.blink_timer = 30;
        }
        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Oak intro pending");
            let trainer = runtime_shell
                .shell
                .snapshot()
                .expect("Oak intro snapshot")
                .trainer;
            let key = oak_intro_art_key(oak_intro, trainer.player_gender);
            let rendered_art = app.world().resource::<RenderedTilesetArt>();
            let oak_frame = rendered_art
                .oak_intro_cache
                .get(&key)
                .expect("Oak intro frame cache after blink toggle");
            let handle = oak_frame.handle.clone();
            let image = app
                .world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .expect("Oak intro blink-hidden image");
            assert_eq!(
                oak_intro_prompt_arrow_dark_pixels(image),
                0,
                "Oak intro prompt arrow must be hidden for blinkTimer >= 30 like TypeScript"
            );
        }

        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro pending");
            oak_intro.scene_index = 1;
            start_visible_oak_intro_scene(oak_intro);
        }
        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(runtime_shell.last_error, None);
            let rendered_art = app.world().resource::<RenderedTilesetArt>();
            assert!(
                rendered_art.pokemon_cache.contains_key(&PokemonArtKey {
                    species_id: "wooper".to_string(),
                    side: PokemonSpriteSide::Front,
                    shiny: false,
                    frame: 0,
                }),
                "Oak intro must render Wooper from the real Pokemon frontpic"
            );
        }

        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell
                .shell
                .set_player_gender(PLAYER_GENDER_FEMALE)
                .expect("set Kris gender");
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro pending");
            oak_intro.scene_index = 3;
            start_visible_oak_intro_scene(oak_intro);
        }
        app.update();
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(runtime_shell.last_error, None);
            let rendered_art = app.world().resource::<RenderedTilesetArt>();
            assert!(
                rendered_art.intro_cache.contains_key(&IntroArtKey {
                    asset_id: "player:kris".to_string(),
                }),
                "Oak intro must render the selected female player portrait"
            );
        }
    }

    #[test]
    fn oak_intro_fade_phases_gate_text_and_clear_sprite_on_fade_out() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Oak intro pending");
            assert_eq!(oak_intro.scene_state, "oak_intro_1");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::FadeIn);
            assert_eq!(oak_intro.fade_alpha, 255);
            assert!(oak_intro.current_text.is_empty());
        }

        for _ in 0..(4 * VISIBLE_OAK_INTRO_FADE_FRAME_DELAY) {
            tick_visible_oak_intro(&mut runtime_shell).expect("tick Oak fade-in");
        }
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Oak intro pending after fade-in");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::Text);
            assert_eq!(oak_intro.fade_alpha, 0);
            assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[0].2[0]);
        }

        for _ in 0..VISIBLE_OAK_INTRO_SCENES[0].2.len() {
            finish_current_oak_intro_page_for_test(&mut runtime_shell);
        }
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Oak intro pending during fade-out");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::FadeOut);
            assert_eq!(oak_intro.current_sprite.as_deref(), Some("OAK"));
            assert_eq!(oak_intro.fade_alpha, 0);
        }

        for _ in 0..(3 * VISIBLE_OAK_INTRO_FADE_FRAME_DELAY) {
            tick_visible_oak_intro(&mut runtime_shell).expect("tick Oak fade-out");
        }
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Oak intro pending after fade-out");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::Complete);
            assert!(oak_intro.finished);
            assert_eq!(oak_intro.fade_alpha, 255);
            assert_eq!(oak_intro.current_sprite, None);
        }
    }

    #[test]
    fn oak_intro_queues_wooper_cry_from_pack_metadata() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_mut()
                .expect("Oak intro pending");
            oak_intro.scene_index = 1;
            start_visible_oak_intro_scene(oak_intro);
        }

        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Wooper intro pending");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::WipeIn);
            assert_eq!(oak_intro.wipe_window_x, 0);
            assert!(oak_intro.current_text.is_empty());
        }
        assert!(
            !runtime_shell
                .pending_audio
                .iter()
                .any(|command| command.audio_id == "CRY_WOOPER"),
            "Wooper cry must not be queued before the wipe and first text group"
        );

        for _ in 0..=VISIBLE_OAK_WIPE_END_X / VISIBLE_OAK_WIPE_STEP_PIXELS {
            tick_visible_oak_intro(&mut runtime_shell).expect("tick Wooper wipe");
        }
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Wooper intro pending after wipe");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextOne);
            assert!(!oak_intro.wipe_active);
            assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[1].2[0]);
        }

        finish_current_oak_intro_page_for_test(&mut runtime_shell);
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Wooper second page pending");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextOne);
            assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[1].2[1]);
        }
        finish_current_oak_intro_page_for_test(&mut runtime_shell);
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Wooper cry phase pending");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::Cry);
            assert!(!oak_intro.wooper_cry_queued);
            assert!(oak_intro.current_text.is_empty());
        }
        assert!(
            !runtime_shell
                .pending_audio
                .iter()
                .any(|command| command.audio_id == "CRY_WOOPER"),
            "Wooper cry must wait until the cry phase"
        );

        tick_visible_oak_intro(&mut runtime_shell).expect("tick Wooper cry phase");

        assert!(
            runtime_shell
                .pending_audio
                .iter()
                .any(|command| command.audio_id == "CRY_WOOPER"
                    && matches!(command.kind, ModpackAudioKind::Cry)),
            "Wooper showcase must queue CRY_WOOPER from compiled pack metadata"
        );
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event.contains("queued oak_intro cry CRY_WOOPER species=WOOPER")),
            "{:?}",
            runtime_shell.last_audio_events
        );
        {
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("Wooper text-two pending");
            assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextTwo);
            assert!(oak_intro.wooper_cry_queued);
            assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[1].2[2]);
        }
    }

    #[test]
    fn empty_player_name_confirmation_uses_gender_default_player_name() {
        fn confirm_empty_name_for_gender(player_gender: u8) -> String {
            let mut runtime_shell = core_modular_title_shell_for_test();
            runtime_shell
                .shell
                .set_player_gender(player_gender)
                .expect("set player gender");
            runtime_shell.selected_player_gender = if player_gender == PLAYER_GENDER_FEMALE {
                Some(VisiblePlayerGender::Girl)
            } else {
                Some(VisiblePlayerGender::Boy)
            };
            runtime_shell.intro_screen = None;
            runtime_shell.title_menu = None;
            open_visible_player_name_input(&mut runtime_shell).expect("open player name input");
            assert_eq!(
                runtime_shell
                    .pending_name_input
                    .as_ref()
                    .map(|input| input.label.as_str()),
                Some("YOUR NAME?")
            );
            move_visible_player_name_cursor_to_end(&mut runtime_shell).expect("move to END");
            select_visible_player_name_grid_key(&mut runtime_shell)
                .expect("empty name should use the selected-gender default-name path");
            assert!(runtime_shell.pending_name_input.is_none());
            assert!(runtime_shell.pending_oak_intro.is_some());

            runtime_shell
                .shell
                .snapshot()
                .expect("snapshot after empty name")
                .trainer
                .player_name
        }

        assert_eq!(
            confirm_empty_name_for_gender(PLAYER_GENDER_MALE),
            DEFAULT_MALE_PLAYER_NAME,
            "blank END confirmation must use the male ASM default name"
        );
        assert_eq!(
            confirm_empty_name_for_gender(PLAYER_GENDER_FEMALE),
            DEFAULT_FEMALE_PLAYER_NAME,
            "blank END confirmation must use the female ASM default name"
        );
    }

    #[test]
    fn player_name_entry_matches_typescript_navigation_groups() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        open_visible_player_name_input(&mut runtime_shell).expect("open player name input");

        for _ in 0..visible_name_input_bottom_row_index() {
            move_visible_player_name_cursor_down(&mut runtime_shell).expect("move down");
        }
        {
            let input = runtime_shell
                .pending_name_input
                .as_ref()
                .expect("name input open");
            assert_eq!(
                (input.cursor_column, input.cursor_row),
                (0, visible_name_input_bottom_row_index())
            );
        }
        select_visible_player_name_grid_key(&mut runtime_shell).expect("toggle lower");
        assert_eq!(
            runtime_shell
                .pending_name_input
                .as_ref()
                .map(|input| input.case),
            Some(NameInputCase::Lower)
        );

        move_visible_player_name_cursor_right(&mut runtime_shell).expect("move to DEL");
        {
            let input = runtime_shell
                .pending_name_input
                .as_ref()
                .expect("name input open");
            assert_eq!(
                (input.cursor_column, input.cursor_row),
                (3, visible_name_input_bottom_row_index())
            );
        }
        select_visible_player_name_grid_key(&mut runtime_shell).expect("delete empty is allowed");
        assert_eq!(
            runtime_shell
                .pending_name_input
                .as_ref()
                .map(|input| input.value.as_str()),
            Some("")
        );

        move_visible_player_name_cursor_right(&mut runtime_shell).expect("move to END group");
        {
            let input = runtime_shell
                .pending_name_input
                .as_ref()
                .expect("name input open");
            assert_eq!(
                (input.cursor_column, input.cursor_row),
                (6, visible_name_input_bottom_row_index())
            );
        }

        move_visible_player_name_cursor_up(&mut runtime_shell).expect("move to symbol row");
        move_visible_player_name_cursor_up(&mut runtime_shell).expect("move to lower letter row");
        select_visible_player_name_grid_key(&mut runtime_shell).expect("append y");
        assert_eq!(
            runtime_shell
                .pending_name_input
                .as_ref()
                .map(|input| input.value.as_str()),
            Some("y")
        );
    }

    #[test]
    fn player_name_entry_moves_to_end_at_max_length_like_typescript() {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        open_visible_player_name_input(&mut runtime_shell).expect("open player name input");

        for ch in "KRYSTAL".chars() {
            append_visible_player_name_char(&mut runtime_shell, ch).expect("append name char");
        }

        let input = runtime_shell
            .pending_name_input
            .as_ref()
            .expect("name input open");
        assert_eq!(input.value, "KRYSTAL");
        assert_eq!(
            (input.cursor_column, input.cursor_row),
            (8, visible_name_input_bottom_row_index())
        );
    }

    #[test]
    fn player_name_entry_renders_real_naming_screen_assets() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let input = PendingNameInput {
            label: "YOUR NAME?".to_string(),
            value: "KRIS".to_string(),
            max_length: VISIBLE_NAME_ENTRY_MAX_LENGTH,
            cursor_column: 6,
            cursor_row: visible_name_input_bottom_row_index(),
            case: NameInputCase::Lower,
        };
        let mut images = Assets::<Image>::default();

        let frame =
            load_name_entry_frame(&asset_root, &input, &mut images).expect("render naming screen");
        let image = images.get(&frame.handle).expect("naming screen image");

        assert_eq!(image.texture_descriptor.size.width, 160);
        assert_eq!(image.texture_descriptor.size.height, 144);
        assert_eq!(frame.size, Vec2::new(160.0, 144.0));
        assert_eq!(name_entry_screen_center(), Vec3::ZERO);
        assert!(
            image
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[0] < 248 || pixel[1] < 248 || pixel[2] < 248),
            "naming screen should contain real font, border, underline, and cursor pixels"
        );
        assert!(
            image.data.chunks_exact(4).all(|pixel| pixel[3] == 255),
            "naming screen must be fully opaque so the overworld cannot bleed through UI"
        );
    }

    #[test]
    fn live_runtime_hotkeys_start_new_game_from_title_and_accept_name() {
        let runtime_shell = core_modular_title_shell_for_test();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(runtime_shell)
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(RuntimeTickTimer::new(0.0))
            .insert_resource(VisibleSequenceTickClock::deterministic_test())
            .add_systems(Update, apply_keyboard_input)
            .add_systems(
                Update,
                tick_visible_title_screen.after(apply_keyboard_input),
            )
            .add_systems(
                Update,
                apply_runtime_hotkeys.after(tick_visible_title_screen),
            );

        open_title_main_menu_for_test(&mut app);
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert!(runtime_shell.title_menu.is_none());
            assert!(
                runtime_shell.pending_gender_selection.is_some(),
                "Enter on New Game should open gender before Oak intro and name input"
            );
            assert!(runtime_shell.pending_time_set.is_none());
            assert!(runtime_shell.pending_oak_intro.is_none());
            assert!(runtime_shell.pending_name_input.is_none());
            assert_eq!(runtime_shell.last_error, None);
        }
        confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
        complete_time_set_for_test(&mut app);
        complete_oak_intro_for_test(&mut app);

        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(
                runtime_shell
                    .pending_name_input
                    .as_ref()
                    .map(|input| input.value.as_str()),
                Some("AB")
            );
        }

        press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert!(
                runtime_shell.pending_oak_intro.is_some(),
                "confirmed name should open Oak's final encouragement before gameplay"
            );
        }
        complete_oak_intro_for_test(&mut app);
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(runtime_shell.pending_name_input.is_none());
        assert!(runtime_shell.pending_oak_intro.is_none());
        assert_eq!(runtime_shell.last_error, None);
        let snapshot = runtime_shell.shell.snapshot().expect("snapshot after name");
        assert_eq!(snapshot.trainer.player_name, "AB");
        assert_eq!(snapshot.overworld.map_name, "PlayersHouse2F");
    }

    #[test]
    fn title_new_game_opens_gender_then_oak_clock_intro_before_name_input() {
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
            BevyShellConfig {
                smoke_player_name: Some("AB".to_string()),
                ..Default::default()
            },
        )
        .expect("initialize title shell");

        let mut app = integrated_shell_test_app(runtime_shell);
        open_title_main_menu_for_test(&mut app);
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            let gender = runtime_shell
                .pending_gender_selection
                .as_ref()
                .expect("New Game should open gender selection before Oak intro");
            assert_eq!(
                format_gender_dialog(runtime_shell),
                "ARE YOU A BOY?\nOR ARE YOU A GIRL?\n> BOY\n  GIRL"
            );
            assert_eq!(visible_gender_entries(gender), vec!["> BOY", "  GIRL"]);
            assert!(runtime_shell.pending_time_set.is_none());
            assert!(runtime_shell.pending_oak_intro.is_none());
            assert!(runtime_shell.pending_name_input.is_none());
            assert_eq!(runtime_shell.last_error, None);
        }

        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            let gender = runtime_shell
                .pending_gender_selection
                .as_ref()
                .expect("gender selection should remain active after cursor move");
            assert_eq!(visible_gender_entries(gender), vec!["  BOY", "> GIRL"]);
        }

        press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
        for _ in 0..=usize::from(VISIBLE_GENDER_CONFIRM_DELAY_FRAMES) + 1 {
            app.update();
            if app
                .world()
                .resource::<BevyRuntimeShell>()
                .pending_gender_selection
                .is_none()
            {
                break;
            }
        }
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert_eq!(
                runtime_shell.selected_player_gender,
                Some(VisiblePlayerGender::Girl)
            );
            let time_set = runtime_shell
                .pending_time_set
                .as_ref()
                .expect("confirmed gender should open Oak intro time set");
            assert_eq!(time_set.phase, VisibleTimeSetPhase::WakeDialogue);
            assert_eq!(time_set.next, VisibleTimeSetNext::OakIntro);
            assert_eq!(time_set.wake_index, 0);
            assert_eq!(time_set.hour, 10);
            assert_eq!(time_set.minute, 0);
            assert!(runtime_shell.pending_oak_intro.is_none());
            assert!(runtime_shell.pending_name_input.is_none());
            assert_eq!(runtime_shell.last_error, None);
        }

        complete_time_set_for_test(&mut app);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert!(runtime_shell.pending_time_set.is_none());
            let oak_intro = runtime_shell
                .pending_oak_intro
                .as_ref()
                .expect("completed clock should open Oak intro");
            assert_eq!(oak_intro.mode, VisibleOakIntroMode::Intro);
            assert_eq!(oak_intro.scene_state, "oak_intro_1");
            assert_eq!(oak_intro.current_sprite.as_deref(), Some("OAK"));
            assert!(
                runtime_shell
                    .last_audio_events
                    .iter()
                    .any(|event| event.contains("time set 10:00 tod=Day game=10:0")),
                "default clock confirmation should commit the selected manual time"
            );
            assert!(runtime_shell.pending_name_input.is_none());
            assert_eq!(runtime_shell.last_error, None);
        }

        complete_oak_intro_for_test(&mut app);
        {
            let runtime_shell = app.world().resource::<BevyRuntimeShell>();
            assert!(runtime_shell.pending_oak_intro.is_none());
            assert!(runtime_shell.pending_name_input.is_some());
        }
    }

    #[test]
    fn gender_selection_renders_real_boot_window_palette_and_options() {
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
        complete_time_set_for_test(&mut app);
        app.update();

        let world = app.world_mut();
        let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
        assert!(
            title_entities.iter(world).count() >= 1,
            "gender selection should render a native boot UI frame"
        );
        let rendered_art = world.resource::<RenderedTilesetArt>();
        assert!(
            !rendered_art.gender_cache.is_empty(),
            "gender selection render must use the real boot window/font/palette compositor"
        );
        assert!(rendered_art.gender_errors.is_empty());

        let mut images = Assets::<Image>::default();
        let mut gender = world
            .resource::<BevyRuntimeShell>()
            .pending_gender_selection
            .as_ref()
            .expect("gender selection pending")
            .clone();
        gender.fade_counter = VISIBLE_GENDER_FADE_IN_FRAMES;
        let frame = load_gender_selection_frame(
            &world.resource::<BevyRuntimeShell>().asset_root,
            &gender,
            &mut images,
        )
        .expect("render gender selection frame directly");
        let image = images.get(&frame.handle).expect("gender image");
        assert_eq!(image.texture_descriptor.size.width, 160);
        assert_eq!(image.texture_descriptor.size.height, 144);
        assert_eq!(
            &image.data[0..4],
            &[0x4a, 0xf7, 0xff, 0xff],
            "gender screen background must use palette index 1 from gender_screen.2bpp"
        );
        assert!(
            image
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[0] < 16 && pixel[1] < 16 && pixel[2] < 16 && pixel[3] == 255),
            "gender screen must contain black font/frame pixels from real assets"
        );
    }

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
            let mut tiles = world.query_filtered::<Entity, With<PlayfieldTile>>();
            assert_eq!(
                tiles.iter(world).count(),
                1,
                "the viewport map layer is composited into one retained sprite"
            );
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
            let mut frame_tiles =
                world.query_filtered::<Entity, With<SceneDialogWindowFrameMarker>>();
            assert_eq!(
                frame_tiles.iter(world).count(),
                battle_window_frame_tile_count(
                    FIELD_TEXT_BOX_WIDTH_TILES as usize,
                    FIELD_TEXT_BOX_HEIGHT_TILES as usize,
                ),
                "runtime dialogue should render the full ASM 20x6 textbox frame, not arbitrary Rust rectangles"
            );
            let rendered_art = world.resource::<RenderedTilesetArt>();
            assert!(rendered_art.window_frame_cache.is_some());
            assert_eq!(rendered_art.window_frame_error, None);
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
            let mut tiles = world.query_filtered::<Entity, With<PlayfieldTile>>();
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
        let mut tiles = world.query_filtered::<Entity, With<PlayfieldTile>>();
        assert_eq!(
            tiles
                .get_single(world)
                .expect("overworld should retain one playfield entity after movement"),
            initial_playfield_entity,
            "walking must update the active LCD texture in place instead of exposing a blank frame between despawn and respawn"
        );
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
            assert!(rendered_art.window_frame_cache.is_some());
            assert_eq!(rendered_art.window_frame_error, None);
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
                    .any(|entry| entry.contains("NO TEXT SCROLL")
                        || entry.contains("No Text Scroll")),
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
            assert!(rendered_art.window_frame_cache.is_some());
            assert_eq!(rendered_art.window_frame_error, None);
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
            assert!(reached_submenu_cancel, "walking the live submenu should reach CANCEL: {:?}", visible_party_menu_entries(&snapshot, runtime_shell));
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
            assert!(rendered_art.window_frame_cache.is_some());
            assert_eq!(rendered_art.window_frame_error, None);
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
            assert_eq!(
                runtime_shell.active_music.as_deref(),
                Some("MUSIC_NEW_BARK_TOWN")
            );
        }
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
        let mut tiles = world.query_filtered::<Entity, With<PlayfieldTile>>();
        assert_eq!(
            tiles.iter(world).count(),
            1,
            "the viewport map layer is composited into one retained sprite"
        );
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
                if command.command.eq_ignore_ascii_case("variablesprite") && command.args.len() == 2
                {
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
            if let Err(error) =
                load_sprite_art(&asset_root, &sprite_id, palette_id, "day", &mut images)
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
        prepare_visible_battle_entry(&mut runtime_shell);
        assert_eq!(
            runtime_shell
                .battle_messages
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                "Wild SUDOWOODO appeared!".to_string(),
                "Go! CYNDAQUIL!".to_string(),
            ]
        );
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
            rendered_art.pokemon_cache.contains_key(&PokemonArtKey {
                species_id: "cyndaquil".to_string(),
                side: PokemonSpriteSide::Back,
                shiny: false,
                frame: 0,
            }),
            "player battle Pokemon must render real back art"
        );
        assert_eq!(rendered_art.font_error, None);
        assert!(
            rendered_art.font_cache.is_some(),
            "battle HUD must render with the runtime bitmap font"
        );
        assert!(
            rendered_art.window_frame_cache.is_some(),
            "battle command windows must render from the real textbox frame tile sheet"
        );
        assert_eq!(rendered_art.window_frame_error, None);

        let world = app.world_mut();
        let mut battlers = world.query_filtered::<Entity, With<BattleBattlerMarker>>();
        assert_eq!(
            battlers.iter(world).count(),
            4,
            "battle scene should include only battler art and grounding shadows here; HUD text must come from the ASM battle HUD path, not Rust-only bitmap labels"
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
        let mut battle_window_frames =
            world.query_filtered::<Entity, With<BattleWindowFrameMarker>>();
        assert_eq!(
            battle_window_frames.iter(world).count(),
            battle_window_frame_tile_count(
                BATTLE_TEXT_BOX_WIDTH_TILES as usize,
                BATTLE_TEXT_BOX_HEIGHT_TILES as usize,
            ) +
            battle_window_frame_tile_count(
                BATTLE_MAIN_MENU_WIDTH_TILES as usize,
                BATTLE_MAIN_MENU_HEIGHT_TILES as usize,
            ),
            "battle text and main-menu windows should each render one frame sprite per ASM border tile"
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
                species_id: "CYNDAQUIL".to_string(),
                level: 20,
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
        prepare_visible_battle_entry(&mut runtime_shell);
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
            rendered_art.window_frame_cache.is_some(),
            "battle submenu should load the real textbox frame tile sheet"
        );
        assert_eq!(rendered_art.window_frame_error, None);
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
        prepare_visible_battle_entry(&mut runtime_shell);
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
        prepare_visible_battle_entry(&mut runtime_shell);
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
            vec![">PACK", " AB", " OPTION", " EXIT"]
        );

        apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
            .expect("Down moves the start menu cursor through normal input dispatch");
        assert_eq!(
            visible_start_menu_entries(&runtime_shell).expect("moved start menu entries"),
            vec![" PACK", ">AB", " OPTION", " EXIT"]
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
            vec![">#DEX", " PACK", " #GEAR", " AB", " OPTION", " EXIT"]
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
            .expect("A tosses one item from the Pack action menu");
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

        apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Down])
            .expect("Down reaches Pack CANCEL through normal input dispatch");
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
            let buttons =
                collect_overworld_keyboard_buttons(&keys, false, false, false, false, false);
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
            runtime_shell.shell.snapshot().expect("movement diagnostic snapshot").overworld.facing
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
            vec![">PACK", " AB", " OPTION", " EXIT"]
        );

        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ArrowDown);
        apply_visible_runtime_controls(&keys, &mut runtime_shell);

        assert_eq!(
            visible_start_menu_entries(&runtime_shell).expect("moved start menu entries"),
            vec![" PACK", ">AB", " OPTION", " EXIT"]
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
        assert_eq!((smoke.final_tile_x, smoke.final_tile_y), (5, 8));
        assert_eq!(
            smoke.final_scene.as_deref(),
            Some("SCENE_NEWBARKTOWN_TEACHER_STOPS_YOU")
        );
        assert_eq!(smoke.warps, 2);
        assert_eq!(smoke.coord_events, 2);
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
                .any(|event| event.contains("closed special boundary InitialSetDstFlag"))
        );
        assert!(
            smoke
                .audio_events
                .iter()
                .any(|event| event.contains("script flag ENGINE_FLYPOINT_NEW_BARK=true"))
        );
        assert!(smoke
            .audio_events
            .iter()
            .any(|event| event.contains("coord event script=NewBarkTown_TeacherStopsYouScene2")));
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
        push_frames(&mut input_frames, GameButton::Up, 5);
        push_frames(&mut input_frames, GameButton::Right, 2);
        push_frames(&mut input_frames, GameButton::Up, 2);
        push_frames(&mut input_frames, GameButton::Right, 2);
        push_frames(&mut input_frames, GameButton::Up, 1);
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
        assert_eq!((smoke.final_tile_x, smoke.final_tile_y), (5, 3));
        assert_eq!(
            smoke.final_scene.as_deref(),
            Some("SCENE_ELMSLAB_AIDE_GIVES_POTION")
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
            smoke.audio_events.iter().any(
                |event| event.contains("script scene setscene SCENE_ELMSLAB_AIDE_GIVES_POTION")
            )
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
        push_frames(&mut input_frames, GameButton::Right, 2);
        push_frames(&mut input_frames, GameButton::Up, 1);
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
        assert_eq!((smoke.final_tile_x, smoke.final_tile_y), (6, 3));
        assert_eq!(smoke.final_scene.as_deref(), Some("SCENE_NEWBARKTOWN_NOOP"));
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
        assert!(smoke.frame_events.iter().any(|event| {
            event.contains("62:[Down]:ElmsLab@(5,8)") && event.contains("coord=true")
        }));
        assert!(
            smoke
                .frame_events
                .iter()
                .any(|event| event.contains("65:[Down]:NewBarkTown@(6,3)"))
        );
    }

    #[test]
    fn chikorita_battle_art_loads_real_runtime_assets() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut images = Assets::<Image>::default();

        let front = load_pokemon_frame(
            &asset_root,
            "CHIKORITA",
            PokemonSpriteSide::Front,
            false,
            &mut images,
        )
        .expect("load Chikorita front art");
        let back = load_pokemon_frame(
            &asset_root,
            "CHIKORITA",
            PokemonSpriteSide::Back,
            false,
            &mut images,
        )
        .expect("load Chikorita back art");

        assert_eq!(front.size, Vec2::new(40.0, 40.0));
        assert_eq!(back.size, Vec2::new(48.0, 48.0));
        assert_eq!(images.len(), 2);
    }

    #[test]
    fn wooper_frontpic_retains_black_outline_like_typescript_renderer() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let asset_root = AssetRoot::new(repo_root);
        let mut images = Assets::<Image>::default();
        let frame = load_pokemon_frame(
            &asset_root,
            "WOOPER",
            PokemonSpriteSide::Front,
            false,
            &mut images,
        )
        .expect("load Wooper front art");
        let image = images.get(&frame.handle).expect("Wooper image");
        let black_outline_pixels = image
            .data
            .chunks_exact(4)
            .filter(|pixel| pixel[3] == 255 && pixel[0] < 24 && pixel[1] < 24 && pixel[2] < 24)
            .count();
        let transparent_background_pixels = image
            .data
            .chunks_exact(4)
            .filter(|pixel| pixel[3] == 0)
            .count();
        assert!(
            black_outline_pixels > 0,
            "Wooper frontpic lost its black outline"
        );
        assert!(
            transparent_background_pixels > 0,
            "Wooper frontpic background was not keyed transparent"
        );
    }

    #[test]
    fn object_palette_zero_uses_compiled_sprite_default() {
        let defaults = BTreeMap::from([(String::from("SPRITE_ELM"), 5_i64)]);
        assert_eq!(
            resolve_visible_object_palette("SPRITE_ELM", 0, &defaults),
            5
        );
        assert_eq!(
            resolve_visible_object_palette("SPRITE_ELM", 3, &defaults),
            3
        );
    }

    #[test]
    fn pokemon_colorkey_preserves_enclosed_palette_zero_pixels() {
        let mut source = image::RgbaImage::from_pixel(5, 5, image::Rgba([255, 255, 255, 255]));
        for row in 1..4 {
            for col in 1..4 {
                source.put_pixel(col, row, image::Rgba([0, 0, 0, 255]));
            }
        }
        source.put_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
        let palette = [[255, 255, 255], [132, 165, 206], [140, 66, 115], [0, 0, 0]];
        let mut target = vec![0_u8; 5 * 5 * 4];
        copy_pokemon_frame_rgba(&source, 5, 5, &palette, &mut target);
        assert_eq!(
            target[3], 0,
            "border-connected background must be transparent"
        );
        let center = (2 * 5 + 2) * 4;
        assert_eq!(
            target[center + 3],
            255,
            "enclosed palette zero must remain opaque"
        );
        let outline = (1 * 5 + 1) * 4;
        assert_eq!(&target[outline..outline + 4], &[0, 0, 0, 255]);
    }

    #[test]
    fn retained_frame_pair_rejects_either_frame_outside_session_range() {
        validate_retained_frame_pair("test", 10, 12, 10, 12).expect("boundary frames are valid");
        for (first, second) in [(9, 11), (13, 11), (11, 9), (11, 13)] {
            let error = validate_retained_frame_pair("test", first, second, 10, 12)
                .expect_err("out-of-range retained frame must reject");
            assert!(
                error
                    .to_string()
                    .contains("outside session frame range 10..=12"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn retained_menu_choice_order_is_strictly_increasing() {
        validate_retained_menu_choice_order(None, 10).expect("first choice frame is valid");
        validate_retained_menu_choice_order(Some(10), 11).expect("later choice frame is valid");
        for current in [9, 10] {
            let error = validate_retained_menu_choice_order(Some(10), current)
                .expect_err("non-increasing menu choice frame must reject");
            assert!(
                error
                    .to_string()
                    .contains("not strictly after previous choice frame 10"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn retained_input_frames_fill_missing_frames_with_zero_masks() {
        let masks = BTreeMap::from([(10, 0x10), (12, 0x80)]);
        let inputs =
            retained_input_frames_from_masks(10, 13, &masks).expect("retained input frames");

        assert_eq!(
            inputs
                .iter()
                .map(|input| (input.frame(), input.joypad_mask()))
                .collect::<Vec<_>>(),
            vec![(10, 0x10), (11, 0x00), (12, 0x80)]
        );

        let lockstep =
            retained_lockstep_frames_from_masks(10, 13, &masks).expect("retained lockstep frames");
        assert_eq!(lockstep.len(), 3);
        assert_eq!(lockstep[1].joypad_mask_for(LOCAL_PLAYER_ID), Some(0));
    }

    #[test]
    fn deterministic_input_frame_uses_pre_tick_frame_from_post_tick_checksum() {
        assert_eq!(
            deterministic_input_frame_from_post_tick_checksum(&StateChecksum::new(
                145,
                0xaabb_ccdd
            ))
            .expect("input frame"),
            144
        );
        let error =
            deterministic_input_frame_from_post_tick_checksum(&StateChecksum::new(0, 0xaabb_ccdd))
                .expect_err("post-tick frame zero cannot produce input frame");
        assert!(
            error.to_string().contains("before post-tick frame 0"),
            "{error:#}"
        );
    }

    #[test]
    fn retained_battle_action_order_is_strictly_increasing() {
        validate_retained_battle_action_order(None, 10).expect("first action frame is valid");
        validate_retained_battle_action_order(Some(10), 11).expect("later action frame is valid");

        for turn in [9, 10] {
            let error = validate_retained_battle_action_order(Some(10), turn)
                .expect_err("stale or duplicate battle action frame must reject");
            assert!(
                error
                    .to_string()
                    .contains("not strictly after previous turn 10"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn retained_state_hash_stream_rejects_terminal_before_start() {
        let start = StateChecksumFrame::new(LOCAL_PLAYER_ID, Frame(10), 0x1111_1111);
        let current = StateChecksumFrame::new(LOCAL_PLAYER_ID, Frame(9), 0x2222_2222);

        let error = validate_retained_state_hash_stream(&start, &current)
            .expect_err("terminal state hash before start must reject");
        assert!(
            error
                .to_string()
                .contains("current frame 9 is before start frame 10"),
            "{error:#}"
        );
    }

    #[test]
    fn runtime_tile_to_metatile_u16_rejects_negative_coordinates() {
        let error = runtime_tile_to_metatile_u16(-1, 0, "test")
            .expect_err("negative runtime x must reject");
        assert!(
            error
                .to_string()
                .contains("outside unsigned map coordinates"),
            "{error:#}"
        );
    }

    #[test]
    fn script_text_renderer_resolves_runtime_named_buffers() {
        let body = ScriptTextBody {
            label: "RuntimeBufferText".to_string(),
            commands: vec![
                ScriptTextBodyCommand {
                    command: "text".to_string(),
                    args: vec!["Caught".to_string()],
                    command_index: 0,
                },
                ScriptTextBodyCommand {
                    command: "text_ram".to_string(),
                    args: vec!["STRING_BUFFER_3".to_string()],
                    command_index: 1,
                },
                ScriptTextBodyCommand {
                    command: "text_decimal".to_string(),
                    args: vec![
                        "STRING_BUFFER_4".to_string(),
                        "1".to_string(),
                        "3".to_string(),
                    ],
                    command_index: 2,
                },
            ],
        };
        let named_buffers = BTreeMap::from([
            ("STRING_BUFFER_3".to_string(), "CHIKORITA".to_string()),
            ("STRING_BUFFER_4".to_string(), "152".to_string()),
        ]);

        assert_eq!(
            render_script_text_body(&body, &named_buffers),
            "Caught\nCHIKORITA\n152"
        );
    }

    #[test]
    fn missing_script_text_buffers_render_blank_instead_of_host_diagnostics() {
        let body = ScriptTextBody {
            label: "MissingRuntimeBufferText".to_string(),
            commands: vec![ScriptTextBodyCommand {
                command: "text_ram".to_string(),
                args: vec!["STRING_BUFFER_3".to_string()],
                command_index: 0,
            }],
        };

        assert_eq!(render_script_text_body(&body, &BTreeMap::new()), "");
        assert_eq!(
            render_visible_script_text_pages(&body, &BTreeMap::new(), "CHRIS", "RIVAL", 0),
            vec![String::new()]
        );
    }

    #[test]
    fn visible_script_text_keeps_asm_paragraphs_as_player_advanced_pages() {
        let body = ScriptTextBody {
            label: "MomGivesPokegearText".to_string(),
            commands: vec![
                ScriptTextBodyCommand {
                    command: "text".to_string(),
                    args: vec!["#MON GEAR, or".to_string()],
                    command_index: 0,
                },
                ScriptTextBodyCommand {
                    command: "line".to_string(),
                    args: vec!["just #GEAR.".to_string()],
                    command_index: 1,
                },
                ScriptTextBodyCommand {
                    command: "para".to_string(),
                    args: vec!["It's essential if".to_string()],
                    command_index: 2,
                },
                ScriptTextBodyCommand {
                    command: "line".to_string(),
                    args: vec!["you want to be a".to_string()],
                    command_index: 3,
                },
                ScriptTextBodyCommand {
                    command: "cont".to_string(),
                    args: vec!["good trainer.".to_string()],
                    command_index: 4,
                },
                ScriptTextBodyCommand {
                    command: "done".to_string(),
                    args: Vec::new(),
                    command_index: 5,
                },
            ],
        };

        assert_eq!(
            render_visible_script_text_pages(&body, &BTreeMap::new(), "CHRIS", "RIVAL", 0),
            vec![
                "POKéMON GEAR, or\n\njust POKéGEAR.",
                "It's essential if\n\nyou want to be a",
                "you want to be a\n\ngood trainer.",
            ],
            "ASM para and cont must preserve their player-acknowledged clear/scroll boundaries"
        );
    }

    #[test]
    fn field_dialogue_prompt_waits_for_the_current_page_to_finish_printing() {
        let reveal = VisibleFieldTextReveal {
            // The stored value is the identity of every page, not the text of
            // this one page.  A multi-page dialogue must therefore be judged
            // against `current_page`, never this value.
            text: "first page\u{1e}second page".to_string(),
            page_index: 0,
            visible_chars: 3,
            frames_until_next_char: 0,
        };

        assert!(!visible_field_text_reveal_is_complete(
            &reveal,
            "first page"
        ));

        let finished = VisibleFieldTextReveal {
            visible_chars: "first page".chars().count(),
            ..reveal
        };
        assert!(visible_field_text_reveal_is_complete(
            &finished,
            "first page"
        ));
    }

    #[test]
    fn visible_script_text_normalizes_asm_quotes_and_control_tokens() {
        assert_eq!(
            normalize_visible_script_text("\"Oh, <PLAYER>! Your #MON\"", "CHRIS"),
            "\"Oh, CHRIS! Your POKéMON\""
        );
        assert_eq!(render_script_text_args(&["\"Hello\"".to_string()]), "Hello");
        assert_eq!(bitmap_font_char_map().get(&'é'), Some(&0xea));
    }

    #[test]
    fn visible_script_text_uses_saved_rival_name_and_current_weekday() {
        assert_eq!(
            normalize_visible_script_text_with_context(
                "Meet <RIVAL> on <TODAY>.",
                "CHRIS",
                "SILVER",
                3,
            ),
            "Meet SILVER on WED."
        );
    }

    #[test]
    fn bitmap_font_normalization_and_glyph_map_match_typescript() {
        assert_eq!(
            normalize_bitmap_font_text(
                "<TRAINER> <ROCKET> <PKMN> <POKE> <PC> <TM> <PK><MN><DOT><PO><KE><LV><ID><……>#"
            ),
            "\u{e103} \u{e104} \u{e105}\u{e106} POKé \u{e101} \u{e102} \u{e105}\u{e106}\u{e107}\u{e108}\u{e109}\u{e10a}\u{e10b}……POKé"
        );

        let glyphs = bitmap_font_char_map();
        for (glyph, tile) in [
            ('Ä', 0xc0),
            ('Ö', 0xc1),
            ('Ü', 0xc2),
            ('ä', 0xc3),
            ('ö', 0xc4),
            ('ü', 0xc5),
            ('☎', 0x62),
            ('▲', 0x61),
            ('…', 0x75),
            ('—', 0x7a),
            ('–', 0x7a),
            ('\u{e100}', 0x4a),
            ('\u{e101}', 0x5b),
            ('\u{e102}', 0x5c),
            ('\u{e103}', 0x5d),
            ('\u{e104}', 0x5e),
            ('\u{e105}', 0xe1),
            ('\u{e106}', 0xe2),
            ('\u{e107}', 0xf2),
            ('\u{e108}', 0x70),
            ('\u{e109}', 0x71),
            ('\u{e10a}', 0x6e),
            ('\u{e10b}', 0x73),
        ] {
            assert_eq!(glyphs.get(&glyph), Some(&tile), "glyph {glyph:?}");
        }
    }
