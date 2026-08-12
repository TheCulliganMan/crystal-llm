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
    let unown =
        visible_intro_frameset_steps("SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1").expect("Unown frameset");
    assert_eq!(
        unown.iter().map(|step| step.duration).collect::<Vec<_>>(),
        vec![3, 3, 7, 0]
    );
    let pichu =
        visible_intro_frameset_steps("SPRITE_ANIM_FRAMESET_INTRO_PICHU").expect("Pichu frameset");
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
    let crystal =
        load_title_frame(&asset_root, "crystal", 1, true, &mut images).expect("load title crystal");
    let suicune =
        load_title_frame(&asset_root, "suicune", 8, true, &mut images).expect("load title suicune");
    let copyright = load_title_frame(&asset_root, "copyright", 1, true, &mut images)
        .expect("load title copyright");

    assert_eq!(logo.size, Vec2::new(160.0, 64.0));
    assert_eq!(crystal.size, Vec2::new(48.0, 80.0));
    assert_eq!(suicune.size, Vec2::new(128.0, 128.0));
    assert_eq!(copyright.size, Vec2::new(232.0, 8.0));
    assert_eq!(images.len(), 4);
}

#[test]
fn title_suicune_bg_tiles_keep_palette_color_zero_opaque() {
    let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
    let palette = [[11, 22, 33], [44, 55, 66], [77, 88, 99], [111, 122, 133]];
    let mut target = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];

    blit_native_title_tile(
        &source,
        0,
        &palette,
        false,
        0,
        0,
        NativeTitleScroll::None,
        &mut target,
        None,
    );

    assert_eq!(
        &target[..4],
        &[11, 22, 33, 255],
        "ASM draws Suicune through the opaque BG layer; color zero must not punch holes in its head"
    );
}

#[test]
fn intro_suicune_bg_tiles_preserve_exported_alpha_like_typescript() {
    let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
    let palette = [[13, 24, 35], [44, 55, 66], [77, 88, 99], [111, 122, 133]];
    let mut target = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE * 4];

    blit_intro_source_tile(
        &source,
        8,
        0,
        &palette,
        false,
        false,
        false,
        0,
        0,
        &mut target,
    );

    assert_eq!(
        &target[..4],
        &[0, 0, 0, 0],
        "the TypeScript compositor leaves exported alpha transparent instead of painting a palette-zero tile rectangle"
    );
}

#[test]
fn native_title_screen_frame_uses_title_palettes_and_window_layer() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let title = runtime_shell.title_menu.clone().expect("title menu");
    let mut images = Assets::<Image>::default();

    let entrance_frame = load_title_screen_frame(&runtime_shell.asset_root, &title, &mut images)
        .expect("render title entrance frame");
    let entrance_data = images
        .get(&entrance_frame.handle)
        .expect("title entrance image")
        .data
        .clone();
    assert_eq!(entrance_frame.size, Vec2::new(160.0, 144.0));
    assert_opaque_nonblack_lcd_pixels(&entrance_data, "title entrance");
    assert!(
        entrance_data
            .chunks_exact(4)
            .any(|pixel| pixel[3] == 255 && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)),
        "native title frame must contain real palette-colored title art"
    );

    let mut main_title = title.clone();
    main_title.phase = VisibleTitlePhase::MainMenu;
    main_title.scx = 0;
    let main_frame = load_title_screen_frame(&runtime_shell.asset_root, &main_title, &mut images)
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
fn native_title_layers_use_asm_scy_and_wy_coordinates() {
    let entrance_scroll = NativeTitleScroll::EntranceInterlaced(112);
    assert_eq!(entrance_scroll.at_scanline(0), 112);
    assert_eq!(entrance_scroll.at_scanline(1), 144);
    assert_eq!(entrance_scroll.at_scanline(80), 0);

    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let mut title = runtime_shell.title_menu.take().expect("title menu");
    title.phase = VisibleTitlePhase::PressStart;
    title.frame = 0;
    title.scx = 0;

    let logo = image::RgbaImage::from_pixel(160, 64, image::Rgba([0, 0, 0, 255]));
    let suicune = image::RgbaImage::from_pixel(128, 128, image::Rgba([0, 0, 0, 255]));
    let palette_bank = (0_u8..9)
        .map(|palette| {
            [
                [palette, 0, 0],
                [palette, 1, 0],
                [palette, 2, 0],
                [palette, 3, 0],
            ]
        })
        .collect::<Vec<Palette>>();
    let mut target = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];
    let mut priority_map = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];

    draw_native_title_background(
        &logo,
        &suicune,
        &palette_bank,
        &title,
        &mut target,
        &mut priority_map,
    )
    .expect("draw title BG");

    let logo_top =
        (TITLE_LOGO_ASM_Y_TILE * SOURCE_TILE_SIZE - TITLE_BG_SCY) * TITLE_SCREEN_WIDTH * 4;
    assert_eq!(
        &target[logo_top..logo_top + 4],
        &[2, 3, 0, 255],
        "ASM hlcoord 0,3 must appear at y=16 after SCY=8"
    );
    let suicune_top = ((TITLE_SUICUNE_ASM_Y_TILE * SOURCE_TILE_SIZE - TITLE_BG_SCY)
        * TITLE_SCREEN_WIDTH
        + 6 * SOURCE_TILE_SIZE)
        * 4;
    assert_eq!(
        &target[suicune_top..suicune_top + 4],
        &[0, 3, 0, 255],
        "ASM hlcoord 6,12 must place Suicune at visible y=88 after SCY=8"
    );

    target.fill(0);
    priority_map.fill(0);
    draw_native_title_version_window(&logo, &palette_bank, &mut target, &mut priority_map)
        .expect("draw title window");
    let copyright_top = TITLE_VERSION_WINDOW_Y * TITLE_SCREEN_WIDTH * 4
        + TITLE_VERSION_TEXT_START_COLUMN * SOURCE_TILE_SIZE * 4;
    assert_eq!(
        &target[copyright_top..copyright_top + 4],
        &[7, 3, 0, 255],
        "ASM WY=$88 must place the copyright window on the bottom scanline row"
    );
    assert_eq!(
        &target[TITLE_VERSION_TEXT_START_COLUMN * SOURCE_TILE_SIZE * 4
            ..TITLE_VERSION_TEXT_START_COLUMN * SOURCE_TILE_SIZE * 4 + 4],
        &[0, 0, 0, 0],
        "copyright must not be drawn at the top of the title"
    );
}

#[test]
fn native_title_preserves_every_suicune_pixel_including_the_head() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let mut title = runtime_shell.title_menu.take().expect("title menu");
    title.phase = VisibleTitlePhase::PressStart;
    title.frame = 0;
    title.scx = 0;
    let mut images = Assets::<Image>::default();
    let source = image::open(
        runtime_shell
            .asset_root
            .runtime_assets()
            .join("gfx/title/suicune.png"),
    )
    .expect("load Suicune source")
    .to_rgba8();
    let palette =
        load_title_palette_bank(&runtime_shell.asset_root).expect("load title palettes")[0];

    let origin_x = 6 * SOURCE_TILE_SIZE;
    let origin_y = TITLE_SUICUNE_ASM_Y_TILE * SOURCE_TILE_SIZE - TITLE_BG_SCY;
    for (phase, scx, label) in [
        (VisibleTitlePhase::PressStart, 0, "settled"),
        (VisibleTitlePhase::Entrance, 112, "entrance"),
    ] {
        title.phase = phase;
        title.scx = scx;
        let frame = load_title_screen_frame(&runtime_shell.asset_root, &title, &mut images)
            .expect("render title frame");
        let actual = &images.get(&frame.handle).expect("title image").data;
        for y in 0..6 * SOURCE_TILE_SIZE {
            for x in 0..8 * SOURCE_TILE_SIZE {
                let source_pixel = source.get_pixel(x as u32, y as u32);
                let expected = palette[palette_index_from_gray(source_pixel[0])];
                let offset = ((origin_y + y) * TITLE_SCREEN_WIDTH + origin_x + x) * 4;
                assert_eq!(
                    &actual[offset..offset + 4],
                    &[expected[0], expected[1], expected[2], 255],
                    "{label} Suicune frame-0 pixel ({x},{y}) changed during title composition"
                );
            }
        }
    }
}

// This is only a source-image invariant. Presentation tests below separately
// prove that Bevy attaches these pixels to a visible, correctly scaled entity.
fn assert_opaque_nonblack_lcd_pixels(data: &[u8], screen: &str) {
    assert_eq!(
        data.len(),
        160 * 144 * 4,
        "{screen} must compose one native 160x144 LCD"
    );
    assert!(
        data.chunks_exact(4).all(|pixel| pixel[3] == 255),
        "{screen} must be opaque so no previous scene can bleed through"
    );
    assert!(
        data.chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "{screen} must not present an all-black frame"
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

    let frame =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
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
        image
            .data
            .chunks_exact(4)
            .any(|pixel| { pixel[3] == 255 && (pixel[0] < 64 || pixel[1] < 64 || pixel[2] < 64) }),
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

    let still =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
            .expect("render unbobbed main menu");
    let still_data = images
        .get(&still.handle)
        .expect("unbobbed image")
        .data
        .clone();
    title.main_menu_frame = TITLE_MAIN_MENU_CURSOR_PERIOD + TITLE_MAIN_MENU_CURSOR_PERIOD / 2;
    let bobbed =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
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
    let black =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
            .expect("render initial fade frame");
    let black_image = images.get(&black.handle).expect("initial fade image");
    assert_eq!(
        &black_image.data[0..4],
        &[0, 0, 0, 255],
        "MainMenu starts with the TypeScript fade-in overlay fully black"
    );

    title.main_menu_frame = 11;
    let clear =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
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
fn intro_suicune_close_head_uses_its_asm_palette_banks() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut intro = VisibleIntroScreen::new();
    intro.jumptable_index = 17;
    intro.scroll_x = 0x60;
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    let frame = intro_scene_frame_for_art(&mut rendered_art, &asset_root, &intro, &mut images)
        .expect("render Suicune close-up head frame");
    let image = images.get(&frame.handle).expect("Suicune close-up image");
    let colors = image
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<BTreeSet<_>>();

    // `IntroScene17` loads IntroSuicuneClosePalette. These two colors come
    // from its palette banks 2-4 and do not exist in IntroSuicunePalette.
    // Their presence proves that the colored head is not flattened into the
    // generic orange Suicune background palette.
    assert!(
        colors.contains(&[99, 165, 255]),
        "Suicune's head must contain the close-up palette's light blue"
    );
    assert!(
        colors.contains(&[156, 66, 255]),
        "Suicune's head must contain the close-up palette's purple"
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

    let base = VisibleIntroScreen::new_for_presentation();
    let base_frame = intro_scene_frame_for_art(&mut rendered_art, &asset_root, &base, &mut images)
        .expect("render base Unown intro frame");
    let base_data = images
        .get(&base_frame.handle)
        .expect("base intro image")
        .data
        .clone();
    assert_opaque_nonblack_lcd_pixels(&base_data, "initial packaged-game intro");

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
    let unrevealed_suicune =
        visible_intro_effective_palette(&appear, &intro_root, "suicune", 1, &suicune_palettes[1])
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
fn hall_of_fame_credits_restore_game_timer_counting_on_return() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .set_game_timer_counting(false);
    open_visible_credits_screen(&mut runtime_shell, true).expect("open Hall of Fame credits");
    runtime_shell
        .credits_screen
        .as_mut()
        .expect("credits screen")
        .resume_game_timer_on_exit = true;

    close_visible_credits_screen(&mut runtime_shell, "test-return")
        .expect("return from Hall of Fame credits");

    assert!(runtime_shell.shell.session().state().game_timer_counting);
    assert!(!runtime_shell.shell.session().state().game_logic_paused);
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
    let frame =
        render_visible_credits_frame(&asset_root, credits, &mut images).expect("render credits");
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
