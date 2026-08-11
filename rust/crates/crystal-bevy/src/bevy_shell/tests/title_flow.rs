fn press_key_for_runtime_hotkey_app(app: &mut App, key: KeyCode) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(key);
    }
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset(key);
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
    assert_eq!(intro.jumptable_index, 1);
    assert_eq!(intro.scene_frame_counter, 8);
    assert_eq!(intro.scene_name(), "unown_fade");
    assert_eq!(
        intro.palette_effect,
        VisibleIntroPaletteEffect::UnownFade {
            palette_idx: 0,
            timer: 8,
        },
        "packaged launch must begin with visible pixels, not an ambiguous black window"
    );
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
            VisibleTitlePhase::Entrance | VisibleTitlePhase::Timer | VisibleTitlePhase::PressStart
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
    let backup_path = PathBuf::from(format!("{}.bak", save_path.display()));
    let _ = std::fs::remove_file(&save_path);
    let _ = std::fs::remove_file(&backup_path);

    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .save(&save_path)
        .expect("write primary and recovery save artifacts");
    assert!(save_path.exists());
    assert!(backup_path.exists());
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
    assert!(
        !backup_path.exists(),
        "ErasePreviousSave must also delete the recovery copy so Continue cannot resurrect it"
    );
    assert!(runtime_shell.pending_delete_save.is_none());
    assert!(
        runtime_shell
            .title_menu
            .as_ref()
            .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::Entrance)),
        "delete-save confirmation should return through a fresh title entrance"
    );
    let title = runtime_shell.title_menu.as_ref().expect("restarted title");
    assert!(
        title_continue_save_path(&runtime_shell, title).is_none(),
        "the next title entry must not offer Continue after ErasePreviousSave"
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
    runtime_shell.latest_rtc_sample = Some(native_rtc_source_for_test().sample());
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
    assert_eq!(snapshot.progression.time.registers.hours, 1);
    assert_eq!(snapshot.progression.time.registers.minutes, 1);
    assert_eq!(snapshot.progression.time.game_time_hours, 0);
    assert_eq!(snapshot.progression.time.game_time_minutes, 0);
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
fn visible_clock_reset_refuses_to_invent_an_epoch_without_a_native_sample() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.latest_rtc_sample = None;
    runtime_shell.pending_clock_reset = Some(VisibleClockResetScreen {
        phase: VisibleClockResetPhase::SetMinute,
        confirm_selection: 0,
        day: 1,
        hour: 20,
        minute: 30,
    });

    let error = confirm_visible_clock_reset_screen(&mut runtime_shell)
        .expect_err("manual clock changes require an explicit native/replay RTC sample");

    assert!(
        format!("{error:#}").contains("native RTC sample is required"),
        "{error:#}"
    );
    assert!(runtime_shell.pending_clock_reset.is_some());
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
                press_visible_oak_intro_a_button(&mut runtime_shell).expect("advance Oak intro");
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct RetainedMapSurfacePair {
    base_entity: Entity,
    base_texture: Handle<Image>,
    priority_entity: Entity,
    priority_texture: Handle<Image>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RetainedFullscreenSurface {
    entity: Entity,
    texture: Handle<Image>,
}

fn retained_fullscreen_surface(world: &mut World) -> RetainedFullscreenSurface {
    let (entity, texture, size, transform, visibility) = {
        let mut surfaces = world.query_filtered::<
            (Entity, &Handle<Image>, &Sprite, &Transform, &Visibility),
            With<VisibleIntroSurface>,
        >();
        let (entity, texture, sprite, transform, visibility) = surfaces
            .get_single(world)
            .expect("one retained full-screen LCD presenter");
        (
            entity,
            texture.clone(),
            sprite.custom_size,
            *transform,
            *visibility,
        )
    };
    assert_eq!(
        size,
        Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
        "the retained presenter must fill the 640x576 integer-scaled LCD"
    );
    assert_eq!(
        transform.translation.truncate(),
        Vec2::ZERO,
        "the complete LCD must stay centered on the main camera"
    );
    assert_ne!(
        visibility,
        Visibility::Hidden,
        "the retained LCD texture is not proof of a visible Bevy presentation"
    );

    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert_eq!(
        rendered_art.presented_fullscreen_entity,
        Some(entity),
        "RenderedTilesetArt must own the one visible presenter entity"
    );
    assert_eq!(
        rendered_art
            .intro_presented_surface
            .as_ref()
            .map(|surface| &surface.handle),
        Some(&texture),
        "the visible sprite must use the retained LCD texture"
    );

    let image = world
        .resource::<Assets<Image>>()
        .get(&texture)
        .expect("retained full-screen LCD image");
    let descriptor = &image.texture_descriptor;
    assert_eq!(
        (descriptor.size.width, descriptor.size.height),
        (TITLE_SCREEN_WIDTH as u32, TITLE_SCREEN_HEIGHT as u32),
        "the retained texture must contain exactly one native 160x144 LCD"
    );
    assert_eq!(descriptor.size.depth_or_array_layers, 1);
    assert_eq!(descriptor.dimension, TextureDimension::D2);
    assert_eq!(descriptor.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(
        image.data.len(),
        TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4,
        "the live LCD texture must contain one RGBA texel per native pixel"
    );
    assert!(
        image.data.chunks_exact(4).all(|pixel| pixel[3] == 255),
        "a full-LCD owner must be opaque so the window clear color can never flash through"
    );
    RetainedFullscreenSurface { entity, texture }
}

fn assert_retained_fullscreen_surface_nonblack(
    world: &World,
    surface: &RetainedFullscreenSurface,
    screen: &str,
) {
    let image = world
        .resource::<Assets<Image>>()
        .get(&surface.texture)
        .expect("retained full-screen LCD image");
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "the texture attached to the visible {screen} presenter must not be an all-black LCD"
    );
}

fn assert_main_camera_presents_one_logical_lcd(world: &mut World) {
    let mut cameras = world
        .query_filtered::<(&Camera, &OrthographicProjection, &Transform), With<MainCameraMarker>>();
    let (camera, projection, transform) = cameras
        .get_single(world)
        .expect("one active 2D presentation camera");
    assert!(camera.is_active, "the main LCD camera must be active");
    assert_eq!(
        camera.order, 0,
        "the main LCD camera must own the window pass"
    );
    assert_eq!(
        transform.translation.truncate(),
        Vec2::ZERO,
        "modal LCD screens must not inherit an overworld camera displacement"
    );
    assert!(
        matches!(
            projection.scaling_mode,
            bevy::render::camera::ScalingMode::WindowSize(scale)
                if (scale - 1.0).abs() < f32::EPSILON
        ),
        "the TypeScript renderer scales one logical LCD to the host target; a fixed physical-pixel projection can show only part of it on Retina displays: {:?}",
        projection.scaling_mode
    );
    assert!(
        projection.near <= -1000.0 && projection.far >= 1000.0,
        "the main camera must retain Bevy's 2D depth range so positive-z LCD sprites are not culled: near={}, far={}",
        projection.near,
        projection.far
    );
}

#[test]
fn shell_camera_keeps_every_lcd_sprite_depth_visible() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, setup_shell_view);
    app.update();
    assert_main_camera_presents_one_logical_lcd(app.world_mut());
}

fn assert_retained_fullscreen_surface(world: &mut World, expected: &RetainedFullscreenSurface) {
    assert_eq!(
        retained_fullscreen_surface(world),
        *expected,
        "full-screen handoffs and animation updates must retain one Entity and Handle<Image>"
    );
}

fn retained_map_surface_pair(world: &mut World) -> RetainedMapSurfacePair {
    let (base_entity, base_texture, base_transform, base_size) = {
        let mut base = world.query_filtered::<
                (Entity, &Handle<Image>, &Transform, &Sprite),
                (With<PlayfieldTile>, Without<PlayfieldPriorityTile>),
            >();
        let (entity, texture, transform, sprite) = base
            .get_single(world)
            .expect("one retained base map surface");
        (
            entity,
            texture.clone(),
            transform.translation,
            sprite.custom_size,
        )
    };
    let (priority_entity, priority_texture, priority_transform, priority_size) = {
        let mut priority = world.query_filtered::<
                (Entity, &Handle<Image>, &Transform, &Sprite),
                (With<PlayfieldTile>, With<PlayfieldPriorityTile>),
            >();
        let (entity, texture, transform, sprite) = priority
            .get_single(world)
            .expect("one retained priority map surface");
        (
            entity,
            texture.clone(),
            transform.translation,
            sprite.custom_size,
        )
    };

    assert_eq!(
        base_size,
        Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
        "base map surface must fill the LCD"
    );
    assert_eq!(
        priority_size,
        Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
        "priority map surface must fill the LCD"
    );
    assert_eq!(
        (base_transform.x, base_transform.y),
        (priority_transform.x, priority_transform.y),
        "base and priority surfaces must stay pixel-aligned"
    );
    assert!(
        priority_transform.z > base_transform.z,
        "priority surface must render above the base surface"
    );

    let rendered = world.resource::<RenderedViewport>();
    let expected_offset =
        visible_overworld_camera_offset(rendered, world.resource::<BevyRuntimeShell>());
    if expected_offset == Vec2::ZERO {
        assert_eq!(
            (base_transform.x, base_transform.y),
            (0.0, 0.0),
            "a 640x576 composite at rest must be centered on the camera, with no clear-color edge strip"
        );
    }
    assert_eq!(
        rendered.map_texture.as_ref(),
        Some(&base_texture),
        "RenderedViewport must own the visible base texture handle"
    );
    assert_eq!(
        rendered.map_priority_texture.as_ref(),
        Some(&priority_texture),
        "RenderedViewport must own the visible priority texture handle"
    );
    let images = world.resource::<Assets<Image>>();
    assert!(
        images.get(&base_texture).is_some(),
        "base map texture handle must resolve to a live image"
    );
    assert!(
        images.get(&priority_texture).is_some(),
        "priority map texture handle must resolve to a live image"
    );

    RetainedMapSurfacePair {
        base_entity,
        base_texture,
        priority_entity,
        priority_texture,
    }
}

fn assert_opaque_base_surface_covers_camera(world: &mut World) {
    let mut candidates = {
        let mut bases = world.query_filtered::<
                (&Handle<Image>, &Sprite, &Transform),
                (With<PlayfieldTile>, Without<PlayfieldPriorityTile>),
            >();
        bases
            .iter(world)
            .filter_map(|(handle, sprite, transform)| {
                sprite
                    .custom_size
                    .map(|size| (handle.clone(), size, transform.translation.truncate()))
            })
            .collect::<Vec<_>>()
    };
    {
        let mut backing = world
            .query_filtered::<(&Handle<Image>, &Sprite, &Transform), With<PlayfieldMapBackingBase>>(
            );
        candidates.extend(
            backing
                .iter(world)
                .filter_map(|(handle, sprite, transform)| {
                    sprite
                        .custom_size
                        .map(|size| (handle.clone(), size, transform.translation.truncate()))
                }),
        );
    }
    let images = world.resource::<Assets<Image>>();
    let viewport_left = -PLAYFIELD_WIDTH * 0.5;
    let viewport_right = PLAYFIELD_WIDTH * 0.5;
    let viewport_bottom = -PLAYFIELD_HEIGHT * 0.5;
    let viewport_top = PLAYFIELD_HEIGHT * 0.5;
    assert!(
        candidates.iter().any(|(handle, size, center)| {
            center.x - size.x * 0.5 <= viewport_left
                && center.x + size.x * 0.5 >= viewport_right
                && center.y - size.y * 0.5 <= viewport_bottom
                && center.y + size.y * 0.5 >= viewport_top
                && images
                    .get(handle)
                    .is_some_and(|image| image.data.chunks_exact(4).all(|pixel| pixel[3] == 255))
        }),
        "the live or retained backing base must cover every camera pixel: {candidates:?}"
    );
}

fn assert_base_map_surface_is_fully_opaque(world: &World, surfaces: &RetainedMapSurfacePair) {
    let image = world
        .resource::<Assets<Image>>()
        .get(&surfaces.base_texture)
        .expect("retained base map image");
    let size = image.texture_descriptor.size;
    assert_eq!(
        (size.width, size.height),
        (PLAYFIELD_WIDTH as u32, PLAYFIELD_HEIGHT as u32),
        "base map image must contain the complete 640x576 playfield"
    );
    assert_eq!(
        image.data.len(),
        PLAYFIELD_WIDTH as usize * PLAYFIELD_HEIGHT as usize * 4,
        "base map image must contain one RGBA texel for every playfield pixel"
    );
    assert!(
        image.data.chunks_exact(4).all(|pixel| pixel[3] == 255),
        "dark border blocks must remain opaque map pixels, never exposed ClearColor"
    );
}

fn inject_held_arrow_right_for_test(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut frames: ResMut<HeldArrowRightTestFrames>,
) {
    if frames.0 == 0 {
        // Ownership of release belongs to the caller. Mutating ArrowRight
        // here erased genuine held/tapped player input in unrelated tests.
        return;
    }
    keys.press(KeyCode::ArrowRight);
    frames.0 -= 1;
}

fn native_rtc_source_for_test() -> NativeRtcSource {
    NativeRtcSource::fixed(RuntimeRtcSample {
        date: GameDate::new(2000, 1, 1),
        hour: 6,
        minute: 0,
        second: 0,
    })
}

fn integrated_shell_test_app(runtime_shell: BevyRuntimeShell) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(f64::from(GAME_TICK_SECONDS)),
        ))
        .insert_resource(ClearColor(Color::rgb(0.05, 0.07, 0.06)))
        .insert_resource(runtime_shell)
        .insert_resource(native_rtc_source_for_test())
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
            drain_unused_runtime_ticks.after(apply_runtime_hotkeys),
        )
        .add_systems(
            Update,
            tick_visible_screen_fade
                .after(drain_unused_runtime_ticks)
                .before(render_playfield),
        )
        .add_systems(
            Update,
            drain_runtime_audio_events.after(drain_unused_runtime_ticks),
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
