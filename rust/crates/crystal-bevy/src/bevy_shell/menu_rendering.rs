const TITLE_MAIN_MENU_BOX_X: usize = 0;
const TITLE_MAIN_MENU_BOX_Y: usize = 0;
const TITLE_MAIN_MENU_BOX_WIDTH: usize = 17;
const TITLE_MAIN_MENU_BOX_HEIGHT: usize = 8;
const TITLE_MAIN_MENU_TIME_BOX_X: usize = 0;
const TITLE_MAIN_MENU_TIME_BOX_Y: usize = 14;
const TITLE_MAIN_MENU_TIME_BOX_WIDTH: usize = 20;
const TITLE_MAIN_MENU_TIME_BOX_HEIGHT: usize = 4;
const TITLE_MAIN_MENU_CURSOR_PERIOD: u32 = 16;
const TITLE_MAIN_MENU_CURSOR_OFFSET: usize = 1;
const TITLE_MAIN_MENU_FADE_SPEED: u32 = 24;
const TITLE_MAIN_MENU_DAY_STRINGS: [&str; 7] =
    ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"];

fn load_visible_title_main_menu_frame(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = runtime_shell.asset_root.runtime_assets();
    if rendered_art.title_menu_font_source.is_none() {
        rendered_art.title_menu_font_source = Some(
            image::open(assets.join("gfx/font/font.png"))
                .context("decode title main-menu font PNG")?
                .to_rgba8(),
        );
    }
    if rendered_art.title_menu_frame_source.is_none() {
        rendered_art.title_menu_frame_source = Some(
            image::open(assets.join("gfx/frames/1.png"))
                .context("decode title main-menu textbox frame PNG")?
                .to_rgba8(),
        );
    }
    let font = rendered_art
        .title_menu_font_source
        .as_ref()
        .expect("title menu font source initialized");
    let frame = rendered_art
        .title_menu_frame_source
        .as_ref()
        .expect("title menu frame source initialized");

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[255, 255, 255, 255]);
    }

    draw_time_set_window(
        &frame,
        TITLE_MAIN_MENU_BOX_X,
        TITLE_MAIN_MENU_BOX_Y,
        TITLE_MAIN_MENU_BOX_WIDTH,
        TITLE_MAIN_MENU_BOX_HEIGHT,
        &mut data,
    )?;
    let options = visible_title_menu_options(runtime_shell, title);
    let selected = title
        .cursor
        .option_index
        .min(options.len().saturating_sub(1));
    let cursor_bob = visible_title_main_menu_cursor_bob(title.main_menu_frame);
    for (index, option) in options.iter().enumerate() {
        let y = (TITLE_MAIN_MENU_BOX_Y + 2 + index) * SOURCE_TILE_SIZE;
        if index == selected {
            draw_time_set_text(
                &font,
                "▶",
                (TITLE_MAIN_MENU_BOX_X + 1) * SOURCE_TILE_SIZE,
                y + cursor_bob,
                &mut data,
            )?;
        }
        draw_time_set_text(
            &font,
            visible_title_menu_option_label(*option),
            (TITLE_MAIN_MENU_BOX_X + 2) * SOURCE_TILE_SIZE,
            y,
            &mut data,
        )?;
    }

    if title_continue_save_path(runtime_shell, title).is_some() {
        draw_time_set_window(
            &frame,
            TITLE_MAIN_MENU_TIME_BOX_X,
            TITLE_MAIN_MENU_TIME_BOX_Y,
            TITLE_MAIN_MENU_TIME_BOX_WIDTH,
            TITLE_MAIN_MENU_TIME_BOX_HEIGHT,
            &mut data,
        )?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let (day_text, time_text) = visible_title_main_menu_clock_strings(&snapshot);
        draw_time_set_text(
            &font,
            &day_text,
            (TITLE_MAIN_MENU_TIME_BOX_X + 1) * SOURCE_TILE_SIZE,
            (TITLE_MAIN_MENU_TIME_BOX_Y + 1) * SOURCE_TILE_SIZE,
            &mut data,
        )?;
        draw_time_set_text(
            &font,
            &time_text,
            (TITLE_MAIN_MENU_TIME_BOX_X + 4) * SOURCE_TILE_SIZE,
            (TITLE_MAIN_MENU_TIME_BOX_Y + 2) * SOURCE_TILE_SIZE,
            &mut data,
        )?;
    }

    let fade_alpha = visible_title_main_menu_fade_alpha(title.main_menu_frame);
    if fade_alpha > 0 {
        for pixel in data.chunks_exact_mut(4) {
            let alpha = fade_alpha;
            let inv_alpha = 255_u16.saturating_sub(alpha);
            pixel[0] = ((u16::from(pixel[0]) * inv_alpha) / 255) as u8;
            pixel[1] = ((u16::from(pixel[1]) * inv_alpha) / 255) as u8;
            pixel[2] = ((u16::from(pixel[2]) * inv_alpha) / 255) as u8;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn visible_title_menu_option_label(option: TitleMenuOption) -> &'static str {
    match option {
        TitleMenuOption::Continue => "CONTINUE",
        TitleMenuOption::NewGame => "NEW GAME",
        TitleMenuOption::Options => "OPTION",
        TitleMenuOption::MysteryGift => "MYSTERY GIFT",
    }
}

fn visible_title_main_menu_cursor_bob(frame: u32) -> usize {
    if frame % TITLE_MAIN_MENU_CURSOR_PERIOD < TITLE_MAIN_MENU_CURSOR_PERIOD / 2 {
        0
    } else {
        TITLE_MAIN_MENU_CURSOR_OFFSET
    }
}

fn visible_title_main_menu_fade_alpha(frame: u32) -> u16 {
    255_u16.saturating_sub(frame.saturating_mul(TITLE_MAIN_MENU_FADE_SPEED).min(255) as u16)
}

fn visible_title_main_menu_clock_strings(snapshot: &RuntimeShellSnapshot) -> (String, String) {
    let time = &snapshot.progression.time;
    let day = TITLE_MAIN_MENU_DAY_STRINGS[usize::from(time.day_of_week % 7)];
    let hour = time.game_time_hours % 24;
    let minute = time.game_time_minutes.min(59);
    let period = if hour < MORN_HOUR {
        "NITE"
    } else if hour < DAY_HOUR {
        "MORN"
    } else if hour < NITE_HOUR {
        "DAY"
    } else {
        "NITE"
    };
    let hour12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    (
        format!("{day}DAY"),
        format!("{period}{hour12:<2}:{minute:02}"),
    )
}

fn spawn_visible_credits_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    credits: &VisibleCreditsScreen,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = render_visible_credits_frame(&runtime_shell.asset_root, credits, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * 4.0),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 0.1),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

fn spawn_visible_delete_save_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    delete_save: &VisibleDeleteSaveScreen,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = load_delete_save_frame(&runtime_shell.asset_root, delete_save, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * (TILE_SIZE / SOURCE_TILE_SIZE as f32)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

fn spawn_visible_mystery_gift_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    mystery_gift: &VisibleMysteryGiftScreen,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = load_mystery_gift_frame(&runtime_shell.asset_root, mystery_gift, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * (TILE_SIZE / SOURCE_TILE_SIZE as f32)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

const DELETE_SAVE_PROMPT_BOX_X: usize = 1;
const DELETE_SAVE_PROMPT_BOX_Y: usize = 6;
const DELETE_SAVE_PROMPT_BOX_WIDTH: usize = 18;
const DELETE_SAVE_PROMPT_BOX_HEIGHT: usize = 4;
const DELETE_SAVE_OPTION_BOX_X: usize = 11;
const DELETE_SAVE_OPTION_BOX_Y: usize = 9;
const DELETE_SAVE_OPTION_BOX_WIDTH: usize = 6;
const DELETE_SAVE_OPTION_BOX_HEIGHT: usize = 4;

fn load_delete_save_frame(
    asset_root: &AssetRoot,
    delete_save: &VisibleDeleteSaveScreen,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode delete-save font PNG")?
        .to_rgba8();
    let frame = image::open(assets.join("gfx/frames/1.png"))
        .context("decode delete-save textbox frame PNG")?
        .to_rgba8();

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    draw_time_set_textbox(
        &font,
        &frame,
        "Delete all saved data?",
        DELETE_SAVE_PROMPT_BOX_X,
        DELETE_SAVE_PROMPT_BOX_Y,
        DELETE_SAVE_PROMPT_BOX_WIDTH,
        DELETE_SAVE_PROMPT_BOX_HEIGHT,
        &mut data,
    )?;
    draw_time_set_window(
        &frame,
        DELETE_SAVE_OPTION_BOX_X,
        DELETE_SAVE_OPTION_BOX_Y,
        DELETE_SAVE_OPTION_BOX_WIDTH,
        DELETE_SAVE_OPTION_BOX_HEIGHT,
        &mut data,
    )?;
    let selected = delete_save.selected_index.min(1);
    draw_time_set_text(
        &font,
        if selected == 0 { "▶YES" } else { " YES" },
        (DELETE_SAVE_OPTION_BOX_X + 1) * SOURCE_TILE_SIZE,
        (DELETE_SAVE_OPTION_BOX_Y + 1) * SOURCE_TILE_SIZE,
        &mut data,
    )?;
    draw_time_set_text(
        &font,
        if selected == 1 { "▶NO" } else { " NO" },
        (DELETE_SAVE_OPTION_BOX_X + 1) * SOURCE_TILE_SIZE,
        (DELETE_SAVE_OPTION_BOX_Y + 2) * SOURCE_TILE_SIZE,
        &mut data,
    )?;

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

const MYSTERY_GIFT_BOX_X: usize = 1;
const MYSTERY_GIFT_BOX_Y: usize = 6;
const MYSTERY_GIFT_BOX_WIDTH: usize = 18;
const MYSTERY_GIFT_BOX_MIN_HEIGHT: usize = 6;

fn load_mystery_gift_frame(
    asset_root: &AssetRoot,
    mystery_gift: &VisibleMysteryGiftScreen,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode Mystery Gift font PNG")?
        .to_rgba8();
    let frame = image::open(assets.join("gfx/frames/1.png"))
        .context("decode Mystery Gift textbox frame PNG")?
        .to_rgba8();

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    let line_count = mystery_gift.message.lines().count().max(1);
    let box_height = (line_count + 2).max(MYSTERY_GIFT_BOX_MIN_HEIGHT);
    draw_time_set_window(
        &frame,
        MYSTERY_GIFT_BOX_X,
        MYSTERY_GIFT_BOX_Y,
        MYSTERY_GIFT_BOX_WIDTH,
        box_height,
        &mut data,
    )?;
    draw_time_set_text(
        &font,
        &mystery_gift.message,
        (MYSTERY_GIFT_BOX_X + 1) * SOURCE_TILE_SIZE,
        (MYSTERY_GIFT_BOX_Y + 1) * SOURCE_TILE_SIZE,
        &mut data,
    )?;

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

const TRAINER_CARD_TILE_WIDTH: usize = 20;
const TRAINER_CARD_TILE_HEIGHT: usize = 18;
const TRAINER_CARD_TOP_BORDER_ROWS: usize = 5;
const TRAINER_CARD_BOTTOM_BORDER_TOP: usize = 8;
const TRAINER_CARD_BOTTOM_BORDER_ROWS: usize = 6;
const TRAINER_CARD_PORTRAIT_X: usize = 14;
const TRAINER_CARD_PORTRAIT_Y: usize = 1;
const TRAINER_CARD_PORTRAIT_WIDTH: usize = 5;
const TRAINER_CARD_PORTRAIT_HEIGHT: usize = 7;
const TRAINER_CARD_RIGHT_CORNER_INDEX: u8 = 0x1c;
const TRAINER_CARD_TILE_BASE: u8 = 0x23;
const TRAINER_CARD_STATUS_TILE_BASE: u8 = 0x29;
const TRAINER_CARD_SMALL_COLON_TILE: u8 = TRAINER_CARD_STATUS_TILE_BASE + 5;

fn trainer_card_art_key(
    snapshot: &RuntimeShellSnapshot,
    page: VisibleTrainerCardPage,
    colon_visible: bool,
    badge_frame: u8,
) -> TrainerCardArtKey {
    TrainerCardArtKey {
        page,
        badge_frame: if page == VisibleTrainerCardPage::JohtoBadges {
            badge_frame & 0x07
        } else {
            0
        },
        player_name: snapshot.trainer.player_name.clone(),
        player_id: snapshot.trainer.player_id,
        player_gender: snapshot.trainer.player_gender,
        money: snapshot.trainer.money,
        has_pokedex: snapshot
            .progression
            .active_engine_flags
            .contains(ENGINE_POKEDEX_FLAG),
        pokedex_owned: snapshot.progression.pokedex_owned,
        game_time_hours: snapshot.progression.time.game_time_hours,
        game_time_minutes: snapshot.progression.time.game_time_minutes,
        colon_visible: page == VisibleTrainerCardPage::Info && colon_visible,
    }
}

fn spawn_trainer_card_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) {
    let key = trainer_card_art_key(
        snapshot,
        runtime_shell.trainer_card_page,
        runtime_shell.trainer_card_colon_visible,
        runtime_shell.trainer_card_badge_frame,
    );
    if !rendered_art.trainer_card_cache.contains_key(&key) {
        match load_trainer_card_frame(
            &runtime_shell.asset_root,
            snapshot,
            key.page,
            key.badge_frame,
            key.colon_visible,
            images,
        ) {
            Ok(frame) => {
                rendered_art.trainer_card_errors.remove(&key);
                rendered_art.trainer_card_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .trainer_card_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    let Some(frame) = rendered_art.trainer_card_cache.get(&key).cloned() else {
        let error = rendered_art
            .trainer_card_errors
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "unknown Trainer Card render error".to_string());
        error!("required Trainer Card frame could not be rendered: {error}");
        return;
    };
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 4.2),
            ..default()
        },
        FieldCommandMarker,
    ));
}

fn load_trainer_card_frame(
    asset_root: &AssetRoot,
    snapshot: &RuntimeShellSnapshot,
    page: VisibleTrainerCardPage,
    badge_frame: u8,
    colon_visible: bool,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let trainer_card_dir = assets.join("gfx/trainer_card");
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode Trainer Card font PNG")?
        .to_rgba8();
    let full_font = image::open(assets.join("gfx/font/english.png"))
        .context("decode Trainer Card full font PNG")?
        .to_rgba8();
    let portrait_stem = if snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE {
        "kris_card"
    } else {
        "chris_card"
    };
    let portrait = image::open(trainer_card_dir.join(format!("{portrait_stem}.png")))
        .with_context(|| format!("decode Trainer Card portrait PNG {portrait_stem}"))?
        .to_rgba8();
    let trainer_tiles = image::open(trainer_card_dir.join("trainer_card.png"))
        .context("decode Trainer Card tiles PNG")?
        .to_rgba8();
    let right_corner = image::open(trainer_card_dir.join("card_right_corner.png"))
        .context("decode Trainer Card right-corner PNG")?
        .to_rgba8();
    let status_tiles = image::open(trainer_card_dir.join("card_status.png"))
        .context("decode Trainer Card status PNG")?
        .to_rgba8();
    let leaders = if page == VisibleTrainerCardPage::JohtoBadges {
        Some(
            image::open(trainer_card_dir.join("leaders.png"))
                .context("decode Trainer Card leader portraits PNG")?
                .to_rgba8(),
        )
    } else {
        None
    };
    let badges = if page == VisibleTrainerCardPage::JohtoBadges {
        Some(
            image::open(trainer_card_dir.join("badges.png"))
                .context("decode Trainer Card badge sprites PNG")?
                .to_rgba8(),
        )
    } else {
        None
    };
    validate_trainer_card_source(&portrait, portrait_stem, 40, 56)?;
    validate_trainer_card_source(&trainer_tiles, "trainer_card", 16, 24)?;
    validate_trainer_card_source(&right_corner, "card_right_corner", 8, 8)?;
    validate_trainer_card_source(&status_tiles, "card_status", 48, 8)?;
    if let Some(leaders) = &leaders {
        validate_trainer_card_source(leaders, "leaders", 80, 72)?;
    }
    if let Some(badges) = &badges {
        validate_trainer_card_source(badges, "badges", 16, 176)?;
    }

    let palettes = load_trainer_card_palettes(asset_root)?;
    let background_palette = trainer_card_tile_palette(snapshot.trainer.player_gender, 0, 0);
    let background = palettes
        .get(background_palette)
        .or_else(|| palettes.first())
        .context("Trainer Card palettes are empty")?[0];
    let width = TRAINER_CARD_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TRAINER_CARD_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[0] = background[0];
        pixel[1] = background[1];
        pixel[2] = background[2];
        pixel[3] = 255;
    }

    let mut tilemap =
        vec![NAME_ENTRY_SPACE_TILE; TRAINER_CARD_TILE_WIDTH * TRAINER_CARD_TILE_HEIGHT];
    seed_trainer_card_page_one_tilemap(&mut tilemap, colon_visible);
    if page == VisibleTrainerCardPage::JohtoBadges {
        seed_trainer_card_badge_tilemap(&mut tilemap);
    }
    let has_pokedex = snapshot
        .progression
        .active_engine_flags
        .contains(ENGINE_POKEDEX_FLAG);
    if !has_pokedex && page == VisibleTrainerCardPage::Info {
        for y in 9..=10 {
            for x in 1..=17 {
                set_trainer_card_tile(&mut tilemap, x, y, NAME_ENTRY_SPACE_TILE);
            }
        }
    }
    draw_trainer_card_static_text(&font, &full_font, &mut data, 2, 2, "NAME/", 5)?;
    draw_trainer_card_static_text(&font, &full_font, &mut data, 2, 6, "MONEY", 5)?;
    if has_pokedex && page == VisibleTrainerCardPage::Info {
        draw_trainer_card_static_text(&font, &full_font, &mut data, 2, 10, "#DEX", 4)?;
    }
    if page == VisibleTrainerCardPage::Info {
        draw_trainer_card_static_text(&font, &full_font, &mut data, 2, 12, "PLAY TIME", 9)?;
        draw_trainer_card_static_text(&font, &full_font, &mut data, 10, 15, "  BADGES▶", 9)?;
    }
    write_trainer_card_text(
        &font,
        &full_font,
        &mut data,
        7,
        2,
        &snapshot
            .trainer
            .player_name
            .chars()
            .take(10)
            .collect::<String>(),
        10,
    )?;
    write_trainer_card_text(
        &font,
        &full_font,
        &mut data,
        5,
        4,
        &format!("{:05}", snapshot.trainer.player_id),
        5,
    )?;
    write_trainer_card_text(
        &font,
        &full_font,
        &mut data,
        7,
        6,
        &format_trainer_card_money(snapshot.trainer.money),
        7,
    )?;
    if has_pokedex && page == VisibleTrainerCardPage::Info {
        write_trainer_card_text(
            &font,
            &full_font,
            &mut data,
            15,
            10,
            &format!("{:>3}", snapshot.progression.pokedex_owned.min(999)),
            3,
        )?;
    }
    if page == VisibleTrainerCardPage::Info {
        write_trainer_card_text(
            &font,
            &full_font,
            &mut data,
            11,
            12,
            &format!("{:>4}", snapshot.progression.time.game_time_hours),
            4,
        )?;
        write_trainer_card_text(
            &font,
            &full_font,
            &mut data,
            16,
            12,
            &format!("{:02}", snapshot.progression.time.game_time_minutes.min(59)),
            2,
        )?;
    }

    for y in 0..TRAINER_CARD_TILE_HEIGHT {
        for x in 0..TRAINER_CARD_TILE_WIDTH {
            let tile = tilemap[y * TRAINER_CARD_TILE_WIDTH + x];
            if tile == NAME_ENTRY_SPACE_TILE {
                continue;
            }
            let palette_index = trainer_card_tile_palette(snapshot.trainer.player_gender, x, y);
            let palette = palettes
                .get(palette_index)
                .or_else(|| palettes.first())
                .context("Trainer Card palettes are empty")?;
            draw_trainer_card_tile(
                &portrait,
                &trainer_tiles,
                &right_corner,
                &status_tiles,
                leaders.as_ref(),
                tile,
                x,
                y,
                palette,
                &mut data,
            )?;
        }
    }

    if let Some(badges) = &badges {
        draw_owned_trainer_card_badges(
            snapshot,
            badges,
            asset_root,
            badge_frame,
            &mut data,
        )?;
    }

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn validate_trainer_card_source(
    image: &image::RgbaImage,
    label: &str,
    expected_width: u32,
    expected_height: u32,
) -> Result<()> {
    if image.width() != expected_width || image.height() != expected_height {
        anyhow::bail!(
            "Trainer Card {label} PNG has invalid dimensions {}x{}, expected {}x{}",
            image.width(),
            image.height(),
            expected_width,
            expected_height
        );
    }
    Ok(())
}

fn seed_trainer_card_page_one_tilemap(tilemap: &mut [u8], colon_visible: bool) {
    draw_trainer_card_border(tilemap, 0, TRAINER_CARD_TOP_BORDER_ROWS);
    draw_trainer_card_border(
        tilemap,
        TRAINER_CARD_BOTTOM_BORDER_TOP,
        TRAINER_CARD_BOTTOM_BORDER_ROWS,
    );
    let mut tile_id = 0_u8;
    for dy in 0..TRAINER_CARD_PORTRAIT_HEIGHT {
        for dx in 0..TRAINER_CARD_PORTRAIT_WIDTH {
            set_trainer_card_tile(
                tilemap,
                TRAINER_CARD_PORTRAIT_X + dx,
                TRAINER_CARD_PORTRAIT_Y + dy,
                tile_id,
            );
            tile_id = tile_id.wrapping_add(1);
        }
    }
    set_trainer_card_tiles(tilemap, 2, 4, &[0x27, 0x28]);
    set_trainer_card_tiles(
        tilemap,
        1,
        3,
        &[
            0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x25, 0x26,
        ],
    );
    set_trainer_card_tiles(tilemap, 2, 8, &[0x29, 0x2a, 0x2b, 0x2c, 0x2d]);
    if colon_visible {
        set_trainer_card_tile(tilemap, 15, 12, TRAINER_CARD_SMALL_COLON_TILE);
    }
}

fn seed_trainer_card_badge_tilemap(tilemap: &mut [u8]) {
    for y in TRAINER_CARD_BOTTOM_BORDER_TOP..TRAINER_CARD_TILE_HEIGHT {
        for x in 0..TRAINER_CARD_TILE_WIDTH {
            set_trainer_card_tile(tilemap, x, y, NAME_ENTRY_SPACE_TILE);
        }
    }
    draw_trainer_card_border(
        tilemap,
        TRAINER_CARD_BOTTOM_BORDER_TOP,
        TRAINER_CARD_BOTTOM_BORDER_ROWS,
    );
    set_trainer_card_tiles(tilemap, 2, 8, &[0x79, 0x7a, 0x7b, 0x7c, 0x7d]);

    let mut tile = TRAINER_CARD_STATUS_TILE_BASE;
    for &(start_x, start_y) in &[(2, 10), (6, 10), (10, 10), (14, 10)] {
        tile = place_trainer_card_leader_face(tilemap, start_x, start_y, tile);
    }
    tile = 0x51;
    for &(start_x, start_y) in &[(2, 13), (6, 13), (10, 13), (14, 13)] {
        tile = place_trainer_card_leader_face(tilemap, start_x, start_y, tile);
    }
}

fn place_trainer_card_leader_face(
    tilemap: &mut [u8],
    start_x: usize,
    start_y: usize,
    mut tile: u8,
) -> u8 {
    for (row, width) in [4_usize, 3, 3].into_iter().enumerate() {
        for column in 0..width {
            set_trainer_card_tile(tilemap, start_x + column, start_y + row, tile);
            tile = tile.wrapping_add(1);
        }
    }
    tile
}

fn draw_owned_trainer_card_badges(
    snapshot: &RuntimeShellSnapshot,
    badges: &image::RgbaImage,
    asset_root: &AssetRoot,
    badge_frame: u8,
    target: &mut [u8],
) -> Result<()> {
    const BADGE_POSITIONS: [(usize, usize); 8] = [
        (16, 88),
        (48, 88),
        (80, 88),
        (112, 88),
        (48, 112),
        (16, 112),
        (80, 112),
        (112, 112),
    ];
    const BADGE_FRAMES: [[u8; 8]; 8] = [
        [0x00, 0x20, 0x24, 0xa0, 0x00, 0x20, 0x24, 0xa0],
        [0x04, 0x20, 0x24, 0xa0, 0x04, 0x20, 0x24, 0xa0],
        [0x08, 0x20, 0x24, 0xa0, 0x08, 0x20, 0x24, 0xa0],
        [0x0c, 0x20, 0x24, 0xa0, 0x0c, 0x20, 0x24, 0xa0],
        [0x10, 0x20, 0x24, 0xa0, 0x10, 0x20, 0x24, 0xa0],
        [0x14, 0x20, 0x24, 0xa0, 0x14, 0x20, 0x24, 0xa0],
        [0x18, 0x20, 0x24, 0xa0, 0x18, 0x20, 0x24, 0xa0],
        [0x1c, 0x20, 0x24, 0xa0, 0x9c, 0x20, 0x24, 0xa0],
    ];
    let palette = load_trainer_card_badge_palette(asset_root)?;
    for (index, owned) in snapshot.progression.badges.johto.iter().copied().enumerate() {
        if !owned {
            continue;
        }
        let (dest_x, dest_y) = BADGE_POSITIONS[index];
        let raw_base = BADGE_FRAMES[index][usize::from(badge_frame & 0x07)];
        let flipped = raw_base & 0x80 != 0;
        let base = usize::from(raw_base & 0x7f);
        let offsets = if flipped { [1, 0, 3, 2] } else { [0, 1, 2, 3] };
        for row in 0..2 {
            for column in 0..2 {
                blit_trainer_card_badge_tile(
                    badges,
                    base + offsets[row * 2 + column],
                    dest_x + column * SOURCE_TILE_SIZE,
                    dest_y + row * SOURCE_TILE_SIZE,
                    &palette,
                    flipped,
                    target,
                );
            }
        }
    }
    Ok(())
}

fn blit_trainer_card_badge_tile(
    source: &image::RgbaImage,
    source_index: usize,
    dest_x: usize,
    dest_y: usize,
    palette: &Palette,
    xflip: bool,
    target: &mut [u8],
) {
    const TARGET_WIDTH: usize = TRAINER_CARD_TILE_WIDTH * SOURCE_TILE_SIZE;
    let tiles_per_row = (source.width() as usize / SOURCE_TILE_SIZE).max(1);
    let source_x = (source_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (source_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        for column in 0..SOURCE_TILE_SIZE {
            let source_column = if xflip {
                SOURCE_TILE_SIZE - 1 - column
            } else {
                column
            };
            let pixel = source.get_pixel(
                (source_x + source_column) as u32,
                (source_y + row) as u32,
            );
            let palette_index = palette_index_from_gray(pixel[0]);
            if pixel[3] == 0 || palette_index == 0 {
                continue;
            }
            let [red, green, blue] = palette[palette_index];
            let offset = ((dest_y + row) * TARGET_WIDTH + dest_x + column) * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn draw_trainer_card_border(tilemap: &mut [u8], top: usize, rows: usize) {
    for col in 0..TRAINER_CARD_TILE_WIDTH {
        set_trainer_card_tile(tilemap, col, top, TRAINER_CARD_TILE_BASE);
    }
    let mut row = top + 1;
    set_trainer_card_tile(tilemap, 0, row, TRAINER_CARD_TILE_BASE);
    set_trainer_card_tile(
        tilemap,
        TRAINER_CARD_TILE_WIDTH - 2,
        row,
        TRAINER_CARD_RIGHT_CORNER_INDEX,
    );
    set_trainer_card_tile(
        tilemap,
        TRAINER_CARD_TILE_WIDTH - 1,
        row,
        TRAINER_CARD_TILE_BASE,
    );
    row += 1;
    for _ in 0..rows {
        set_trainer_card_tile(tilemap, 0, row, TRAINER_CARD_TILE_BASE);
        set_trainer_card_tile(
            tilemap,
            TRAINER_CARD_TILE_WIDTH - 1,
            row,
            TRAINER_CARD_TILE_BASE,
        );
        row += 1;
    }
    set_trainer_card_tile(tilemap, 0, row, TRAINER_CARD_TILE_BASE);
    set_trainer_card_tile(tilemap, 1, row, TRAINER_CARD_TILE_BASE + 1);
    set_trainer_card_tile(
        tilemap,
        TRAINER_CARD_TILE_WIDTH - 1,
        row,
        TRAINER_CARD_TILE_BASE,
    );
    row += 1;
    for col in 0..TRAINER_CARD_TILE_WIDTH {
        set_trainer_card_tile(tilemap, col, row, TRAINER_CARD_TILE_BASE);
    }
}

fn set_trainer_card_tiles(tilemap: &mut [u8], x: usize, y: usize, tiles: &[u8]) {
    for (index, tile) in tiles.iter().copied().enumerate() {
        set_trainer_card_tile(tilemap, x + index, y, tile);
    }
}

fn set_trainer_card_tile(tilemap: &mut [u8], x: usize, y: usize, tile: u8) {
    if x >= TRAINER_CARD_TILE_WIDTH || y >= TRAINER_CARD_TILE_HEIGHT {
        return;
    }
    tilemap[y * TRAINER_CARD_TILE_WIDTH + x] = tile;
}

fn write_trainer_card_text(
    font: &image::RgbaImage,
    full_font: &image::RgbaImage,
    target: &mut [u8],
    x: usize,
    y: usize,
    text: &str,
    max_len: usize,
) -> Result<()> {
    let mut padded = text.to_string();
    if padded.chars().count() > max_len {
        padded = padded.chars().take(max_len).collect();
    }
    while padded.chars().count() < max_len {
        padded.push(' ');
    }
    draw_trainer_card_text_tiles(
        font,
        full_font,
        &padded,
        x * SOURCE_TILE_SIZE,
        y * SOURCE_TILE_SIZE,
        target,
    )
}

fn draw_trainer_card_static_text(
    font: &image::RgbaImage,
    full_font: &image::RgbaImage,
    target: &mut [u8],
    x: usize,
    y: usize,
    text: &str,
    max_len: usize,
) -> Result<()> {
    write_trainer_card_text(font, full_font, target, x, y, text, max_len)
}

fn draw_trainer_card_text_tiles(
    font: &image::RgbaImage,
    full_font: &image::RgbaImage,
    text: &str,
    x_px: usize,
    y_px: usize,
    target: &mut [u8],
) -> Result<()> {
    let mut cursor_x = x_px;
    for token in tokenize_name_entry_string(text) {
        let tile_id = name_entry_token_tile(&token)
            .with_context(|| format!("unsupported Trainer Card glyph {token:?}"))?;
        if tile_id != NAME_ENTRY_SPACE_TILE {
            draw_trainer_card_font_tile(font, full_font, tile_id, cursor_x, y_px, target)?;
        }
        cursor_x += SOURCE_TILE_SIZE;
    }
    Ok(())
}

fn draw_trainer_card_font_tile(
    font: &image::RgbaImage,
    full_font: &image::RgbaImage,
    tile_id: u8,
    dest_x: usize,
    dest_y: usize,
    target: &mut [u8],
) -> Result<()> {
    if tile_id >= 0x80 {
        let tile_index = usize::from(tile_id - 0x80);
        blit_time_set_tile_image(
            font,
            (tile_index % 16) * SOURCE_TILE_SIZE,
            (tile_index / 16) * SOURCE_TILE_SIZE,
            dest_x,
            dest_y,
            false,
            false,
            true,
            target,
        );
        return Ok(());
    }
    let tile_index = usize::from(tile_id);
    let tiles_per_row = full_font.width() as usize / SOURCE_TILE_SIZE;
    if tiles_per_row == 0 {
        anyhow::bail!(
            "Trainer Card full font has invalid width {}",
            full_font.width()
        );
    }
    blit_time_set_tile_image(
        full_font,
        (tile_index % tiles_per_row) * SOURCE_TILE_SIZE,
        (tile_index / tiles_per_row) * SOURCE_TILE_SIZE,
        dest_x,
        dest_y,
        false,
        false,
        true,
        target,
    );
    Ok(())
}

fn format_trainer_card_money(amount: u32) -> String {
    let digits = format!("{:>6}", amount.min(999_999));
    let first_digit = digits
        .find(|character: char| character.is_ascii_digit())
        .expect("formatted Trainer Card money always contains a digit");
    format!("{}¥{}", &digits[..first_digit], &digits[first_digit..])
}

fn draw_trainer_card_tile(
    portrait: &image::RgbaImage,
    trainer_tiles: &image::RgbaImage,
    right_corner: &image::RgbaImage,
    status_tiles: &image::RgbaImage,
    leaders: Option<&image::RgbaImage>,
    tile: u8,
    dest_tile_x: usize,
    dest_tile_y: usize,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let (source, source_index) = if tile < 35 {
        (portrait, usize::from(tile))
    } else if tile == TRAINER_CARD_RIGHT_CORNER_INDEX {
        (right_corner, 0)
    } else if (TRAINER_CARD_TILE_BASE..TRAINER_CARD_TILE_BASE + 6).contains(&tile) {
        (trainer_tiles, usize::from(tile - TRAINER_CARD_TILE_BASE))
    } else if let Some(leaders) = leaders.filter(|_| tile >= TRAINER_CARD_STATUS_TILE_BASE) {
        (leaders, usize::from(tile - TRAINER_CARD_STATUS_TILE_BASE))
    } else if (TRAINER_CARD_STATUS_TILE_BASE..TRAINER_CARD_STATUS_TILE_BASE + 6).contains(&tile) {
        (
            status_tiles,
            usize::from(tile - TRAINER_CARD_STATUS_TILE_BASE),
        )
    } else {
        anyhow::bail!("unsupported Trainer Card tile id 0x{tile:02x}");
    };
    blit_trainer_card_palette_tile(
        source,
        source_index,
        dest_tile_x * SOURCE_TILE_SIZE,
        dest_tile_y * SOURCE_TILE_SIZE,
        palette,
        target,
    );
    Ok(())
}

fn blit_trainer_card_palette_tile(
    source: &image::RgbaImage,
    source_index: usize,
    dest_x: usize,
    dest_y: usize,
    palette: &Palette,
    target: &mut [u8],
) {
    const TARGET_WIDTH: usize = TRAINER_CARD_TILE_WIDTH * SOURCE_TILE_SIZE;
    let tiles_per_row = (source.width() as usize / SOURCE_TILE_SIZE).max(1);
    let source_x = (source_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (source_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        for col in 0..SOURCE_TILE_SIZE {
            let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            if pixel[3] == 0 {
                continue;
            }
            let palette_index = palette_index_from_gray(pixel[0]);
            let [red, green, blue] = palette[palette_index];
            let offset = ((dest_y + row) * TARGET_WIDTH + dest_x + col) * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn trainer_card_tile_palette(player_gender: u8, x: usize, y: usize) -> usize {
    if (TRAINER_CARD_PORTRAIT_X..TRAINER_CARD_PORTRAIT_X + TRAINER_CARD_PORTRAIT_WIDTH).contains(&x)
        && (TRAINER_CARD_PORTRAIT_Y..TRAINER_CARD_PORTRAIT_Y + TRAINER_CARD_PORTRAIT_HEIGHT)
            .contains(&y)
    {
        return if player_gender == PLAYER_GENDER_FEMALE {
            1
        } else {
            0
        };
    }
    if x == 18 && y == 1 {
        return if player_gender == PLAYER_GENDER_MALE {
            1
        } else {
            0
        };
    }
    if (11..13).contains(&y) {
        if (2..6).contains(&x) {
            return 1;
        }
        if (6..10).contains(&x) {
            return 2;
        }
        if (10..14).contains(&x) {
            return 3;
        }
        if (14..18).contains(&x) {
            return 4;
        }
    }
    if (14..16).contains(&y) {
        if (2..6).contains(&x) {
            return 5;
        }
        if (6..10).contains(&x) {
            return 6;
        }
        if (10..14).contains(&x) {
            return 7;
        }
        if player_gender == PLAYER_GENDER_FEMALE && (14..18).contains(&x) {
            return 1;
        }
    }
    if player_gender == PLAYER_GENDER_MALE {
        1
    } else {
        0
    }
}

fn load_trainer_card_palettes(asset_root: &AssetRoot) -> Result<Vec<Palette>> {
    const TRAINER_PALETTE_FILES: [&str; 8] = [
        "cal", "falkner", "bugsy", "whitney", "morty", "chuck", "jasmine", "pryce",
    ];
    let trainer_palette_dir = asset_root.runtime_assets().join("gfx/trainers");
    let mut palettes = Vec::with_capacity(TRAINER_PALETTE_FILES.len());
    for stem in TRAINER_PALETTE_FILES {
        let path = trainer_palette_dir.join(format!("{stem}.gbcpal"));
        palettes.push(
            load_gbcpal_palette(&path)
                .with_context(|| format!("load Trainer Card palette {}", path.display()))?,
        );
    }
    Ok(palettes)
}

fn load_trainer_card_badge_palette(asset_root: &AssetRoot) -> Result<Palette> {
    let path = asset_root.runtime_assets().join("gfx/sgb/predef.pal");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read Trainer Card badge palette {}", path.display()))?;
    let line = text
        .lines()
        .find(|line| line.contains("; PREDEFPAL_CGB_BADGE"))
        .with_context(|| format!("{} has no PREDEFPAL_CGB_BADGE entry", path.display()))?;
    let rgb = line
        .split_once(';')
        .map(|(rgb, _)| rgb)
        .context("Trainer Card badge palette entry has no comment separator")?
        .trim()
        .strip_prefix("RGB")
        .context("Trainer Card badge palette entry does not begin with RGB")?;
    let components = rgb
        .split(',')
        .map(|component| component.trim().parse::<u8>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("parse PREDEFPAL_CGB_BADGE components")?;
    if components.len() != 12 || components.iter().any(|component| *component > 31) {
        anyhow::bail!(
            "PREDEFPAL_CGB_BADGE must contain four RGB5 colors, got {components:?}"
        );
    }
    let mut palette = [[0_u8; 3]; 4];
    for (index, color) in components.chunks_exact(3).enumerate() {
        palette[index] = [
            gbc5_to_u8(color[0]),
            gbc5_to_u8(color[1]),
            gbc5_to_u8(color[2]),
        ];
    }
    Ok(palette)
}

fn spawn_visible_clock_reset_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    clock_reset: &VisibleClockResetScreen,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = load_clock_reset_frame(&runtime_shell.asset_root, clock_reset, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * (TILE_SIZE / SOURCE_TILE_SIZE as f32)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

const CLOCK_RESET_PROMPT_BOX_X: usize = 1;
const CLOCK_RESET_PROMPT_BOX_Y: usize = 6;
const CLOCK_RESET_PROMPT_BOX_WIDTH: usize = 18;
const CLOCK_RESET_PROMPT_BOX_HEIGHT: usize = 4;
const CLOCK_RESET_YES_NO_BOX_X: usize = 14;
const CLOCK_RESET_YES_NO_BOX_Y: usize = 7;
const CLOCK_RESET_YES_NO_BOX_WIDTH: usize = 6;
const CLOCK_RESET_YES_NO_BOX_HEIGHT: usize = 4;
const CLOCK_RESET_VALUE_BOX_X: usize = 11;
const CLOCK_RESET_VALUE_BOX_Y: usize = 9;
const CLOCK_RESET_VALUE_BOX_WIDTH: usize = 6;
const CLOCK_RESET_VALUE_BOX_HEIGHT: usize = 4;
const CLOCK_RESET_DAY_NAMES: [&str; 7] = ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"];

fn load_clock_reset_frame(
    asset_root: &AssetRoot,
    clock_reset: &VisibleClockResetScreen,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode clock-reset font PNG")?
        .to_rgba8();
    let frame = image::open(assets.join("gfx/frames/1.png"))
        .context("decode clock-reset textbox frame PNG")?
        .to_rgba8();

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    match clock_reset.phase {
        VisibleClockResetPhase::Confirm => {
            draw_time_set_textbox(
                &font,
                &frame,
                "Reset clock?",
                CLOCK_RESET_PROMPT_BOX_X,
                CLOCK_RESET_PROMPT_BOX_Y,
                CLOCK_RESET_PROMPT_BOX_WIDTH,
                CLOCK_RESET_PROMPT_BOX_HEIGHT,
                &mut data,
            )?;
            draw_time_set_window(
                &frame,
                CLOCK_RESET_YES_NO_BOX_X,
                CLOCK_RESET_YES_NO_BOX_Y,
                CLOCK_RESET_YES_NO_BOX_WIDTH,
                CLOCK_RESET_YES_NO_BOX_HEIGHT,
                &mut data,
            )?;
            let selected = clock_reset.confirm_selection.min(1);
            draw_time_set_text(
                &font,
                if selected == 0 { "▶YES" } else { " YES" },
                (CLOCK_RESET_YES_NO_BOX_X + 1) * SOURCE_TILE_SIZE,
                (CLOCK_RESET_YES_NO_BOX_Y + 1) * SOURCE_TILE_SIZE,
                &mut data,
            )?;
            draw_time_set_text(
                &font,
                if selected == 1 { "▶NO" } else { " NO" },
                (CLOCK_RESET_YES_NO_BOX_X + 1) * SOURCE_TILE_SIZE,
                (CLOCK_RESET_YES_NO_BOX_Y + 2) * SOURCE_TILE_SIZE,
                &mut data,
            )?;
        }
        VisibleClockResetPhase::SetDay => {
            draw_clock_reset_value_prompt(
                &font,
                &frame,
                "What day is it?",
                &format!(
                    "{}DAY",
                    CLOCK_RESET_DAY_NAMES[usize::from(clock_reset.day % 7)]
                ),
                &mut data,
            )?;
        }
        VisibleClockResetPhase::SetHour => {
            draw_clock_reset_value_prompt(
                &font,
                &frame,
                "What hour is it?",
                &format!("{:02}", clock_reset.hour % 24),
                &mut data,
            )?;
        }
        VisibleClockResetPhase::SetMinute => {
            draw_clock_reset_value_prompt(
                &font,
                &frame,
                "What minute?",
                &format!("{:02}", clock_reset.minute % 60),
                &mut data,
            )?;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn draw_clock_reset_value_prompt(
    font: &image::RgbaImage,
    frame: &image::RgbaImage,
    prompt: &str,
    value: &str,
    target: &mut [u8],
) -> Result<()> {
    draw_time_set_textbox(
        font,
        frame,
        prompt,
        CLOCK_RESET_PROMPT_BOX_X,
        CLOCK_RESET_PROMPT_BOX_Y,
        CLOCK_RESET_PROMPT_BOX_WIDTH,
        CLOCK_RESET_PROMPT_BOX_HEIGHT,
        target,
    )?;
    draw_time_set_window(
        frame,
        CLOCK_RESET_VALUE_BOX_X,
        CLOCK_RESET_VALUE_BOX_Y,
        CLOCK_RESET_VALUE_BOX_WIDTH,
        CLOCK_RESET_VALUE_BOX_HEIGHT,
        target,
    )?;
    draw_time_set_text(
        font,
        value,
        (CLOCK_RESET_VALUE_BOX_X + 1) * SOURCE_TILE_SIZE,
        (CLOCK_RESET_VALUE_BOX_Y + 1) * SOURCE_TILE_SIZE,
        target,
    )
}

fn spawn_visible_gender_selection_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    gender: &VisibleGenderSelection,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let key = GenderArtKey {
        selected_index: gender.selected_index.min(1),
        confirmed: gender.confirmed,
        fade_counter: gender.fade_counter,
    };
    if !rendered_art.gender_cache.contains_key(&key) {
        match load_gender_selection_frame(&runtime_shell.asset_root, gender, images) {
            Ok(frame) => {
                rendered_art.gender_errors.remove(&key);
                rendered_art.gender_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .gender_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    let Some(frame) = rendered_art.gender_cache.get(&key).cloned() else {
        return Ok(());
    };
    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * scale),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

const GENDER_QUESTION_TEXT: &str = "ARE YOU A BOY? OR\nARE YOU A GIRL?";
const GENDER_BOX_X: usize = 6;
const GENDER_BOX_Y: usize = 4;
const GENDER_BOX_WIDTH: usize = 7;
const GENDER_BOX_HEIGHT: usize = 6;

fn load_gender_selection_frame(
    asset_root: &AssetRoot,
    gender: &VisibleGenderSelection,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode gender-selection font PNG")?
        .to_rgba8();
    let frame = image::open(assets.join("gfx/frames/1.png"))
        .context("decode gender-selection textbox frame PNG")?
        .to_rgba8();
    let background =
        load_gender_selection_background(&assets.join("gfx/new_game/gender_screen.pal"))
            .context("load gender-selection background palette")?;

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[0] = background[0];
        pixel[1] = background[1];
        pixel[2] = background[2];
        pixel[3] = 255;
    }

    draw_time_set_textbox(
        &font,
        &frame,
        GENDER_QUESTION_TEXT,
        0,
        TIME_SET_TEXTBOX_Y,
        TIME_SET_SCREEN_TILE_WIDTH,
        TIME_SET_TEXTBOX_HEIGHT,
        &mut data,
    )?;
    draw_time_set_window(
        &frame,
        GENDER_BOX_X,
        GENDER_BOX_Y,
        GENDER_BOX_WIDTH,
        GENDER_BOX_HEIGHT,
        &mut data,
    )?;
    let selected = gender.selected_index.min(1);
    draw_time_set_text(
        &font,
        if selected == 0 { "▶BOY" } else { " BOY" },
        (GENDER_BOX_X + 1) * SOURCE_TILE_SIZE,
        (GENDER_BOX_Y + 1) * SOURCE_TILE_SIZE,
        &mut data,
    )?;
    draw_time_set_text(
        &font,
        if selected == 1 { "▶GIRL" } else { " GIRL" },
        (GENDER_BOX_X + 1) * SOURCE_TILE_SIZE,
        (GENDER_BOX_Y + 3) * SOURCE_TILE_SIZE,
        &mut data,
    )?;
    apply_gender_selection_fade(gender.fade_counter, &mut data);

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn load_gender_selection_background(path: &Path) -> Result<[u8; 3]> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut colors = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        let Some(rest) = line.strip_prefix("RGB") else {
            continue;
        };
        let parts = rest
            .trim()
            .split(',')
            .map(|part| part.trim().parse::<u8>())
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("parse gender palette line {line:?}"))?;
        if parts.len() == 3 {
            colors.push([
                gbc5_to_u8(parts[0]),
                gbc5_to_u8(parts[1]),
                gbc5_to_u8(parts[2]),
            ]);
        }
    }
    colors
        .get(1)
        .copied()
        .with_context(|| format!("gender palette {} is missing color index 1", path.display()))
}

fn gbc5_to_u8(value: u8) -> u8 {
    (value << 3) | (value >> 2)
}

fn apply_gender_selection_fade(fade_counter: u8, target: &mut [u8]) {
    if fade_counter >= VISIBLE_GENDER_FADE_IN_FRAMES {
        return;
    }
    let alpha = u16::from(VISIBLE_GENDER_FADE_IN_FRAMES - fade_counter);
    let denom = u16::from(VISIBLE_GENDER_FADE_IN_FRAMES);
    for pixel in target.chunks_exact_mut(4) {
        for channel in &mut pixel[0..3] {
            let base = u16::from(*channel);
            *channel = ((base * (denom - alpha) + 255 * alpha) / denom) as u8;
        }
    }
}

fn spawn_visible_time_set_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    time_set: &VisibleTimeSetScreen,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let key = TimeSetArtKey {
        phase: time_set.phase,
        hour: time_set.hour,
        minute: time_set.minute,
        visible_dialog: visible_time_set_visible_dialog(time_set),
        yes_no_index: time_set.yes_no_index,
    };
    if !rendered_art.time_set_cache.contains_key(&key) {
        match load_time_set_frame(&runtime_shell.asset_root, time_set, images) {
            Ok(frame) => {
                rendered_art.time_set_errors.remove(&key);
                rendered_art.time_set_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .time_set_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    let Some(frame) = rendered_art.time_set_cache.get(&key).cloned() else {
        return Ok(());
    };
    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * scale),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

const TIME_SET_SCREEN_TILE_WIDTH: usize = 20;
const TIME_SET_SCREEN_TILE_HEIGHT: usize = 18;
const TIME_SET_TEXTBOX_Y: usize = 12;
const TIME_SET_TEXTBOX_HEIGHT: usize = 6;
const TIME_SET_HOUR_BOX_X: usize = 3;
const TIME_SET_HOUR_BOX_Y: usize = 7;
const TIME_SET_HOUR_BOX_WIDTH: usize = 17;
const TIME_SET_HOUR_BOX_HEIGHT: usize = 4;
const TIME_SET_MINUTE_BOX_X: usize = 11;
const TIME_SET_MINUTE_BOX_Y: usize = 7;
const TIME_SET_MINUTE_BOX_WIDTH: usize = 9;
const TIME_SET_MINUTE_BOX_HEIGHT: usize = 4;
const TIME_SET_YES_NO_BOX_X: usize = 14;
const TIME_SET_YES_NO_BOX_Y: usize = 7;
const TIME_SET_YES_NO_BOX_WIDTH: usize = 6;
const TIME_SET_YES_NO_BOX_HEIGHT: usize = 4;
const TIME_SET_HOUR_TEXT_X: usize = 4;
const TIME_SET_HOUR_TEXT_Y: usize = 9;
const TIME_SET_MINUTE_TEXT_X: usize = 12;
const TIME_SET_MINUTE_TEXT_Y: usize = 9;
const TIME_SET_HOUR_ARROW_X: usize = 11;
const TIME_SET_MINUTE_ARROW_X: usize = 15;
const TIME_SET_ARROW_TOP_Y: usize = 7;
const TIME_SET_ARROW_BOTTOM_Y: usize = 10;

fn load_time_set_frame(
    asset_root: &AssetRoot,
    time_set: &VisibleTimeSetScreen,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode time-set font PNG")?
        .to_rgba8();
    let frame = image::open(assets.join("gfx/frames/1.png"))
        .context("decode time-set textbox frame PNG")?
        .to_rgba8();
    let up_arrow = image::open(assets.join("gfx/new_game/up_arrow.png"))
        .context("decode time-set up arrow PNG")?
        .to_rgba8();
    let down_arrow = image::open(assets.join("gfx/new_game/down_arrow.png"))
        .context("decode time-set down arrow PNG")?
        .to_rgba8();

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![248_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[1] = 248;
        pixel[2] = 248;
        pixel[3] = 255;
    }

    match time_set.phase {
        VisibleTimeSetPhase::SetHour => {
            draw_time_set_textbox(
                &font,
                &frame,
                "What time is it?",
                0,
                TIME_SET_TEXTBOX_Y,
                20,
                TIME_SET_TEXTBOX_HEIGHT,
                &mut data,
            )?;
            draw_time_set_window(
                &frame,
                TIME_SET_HOUR_BOX_X,
                TIME_SET_HOUR_BOX_Y,
                TIME_SET_HOUR_BOX_WIDTH,
                TIME_SET_HOUR_BOX_HEIGHT,
                &mut data,
            )?;
            draw_time_set_arrows(&up_arrow, &down_arrow, TIME_SET_HOUR_ARROW_X, &mut data);
            draw_time_set_text(
                &font,
                &visible_time_set_hour_display(time_set),
                TIME_SET_HOUR_TEXT_X * SOURCE_TILE_SIZE,
                TIME_SET_HOUR_TEXT_Y * SOURCE_TILE_SIZE,
                &mut data,
            )?;
        }
        VisibleTimeSetPhase::SetMinute => {
            draw_time_set_textbox(
                &font,
                &frame,
                "How many minutes?",
                0,
                TIME_SET_TEXTBOX_Y,
                20,
                TIME_SET_TEXTBOX_HEIGHT,
                &mut data,
            )?;
            draw_time_set_window(
                &frame,
                TIME_SET_MINUTE_BOX_X,
                TIME_SET_MINUTE_BOX_Y,
                TIME_SET_MINUTE_BOX_WIDTH,
                TIME_SET_MINUTE_BOX_HEIGHT,
                &mut data,
            )?;
            draw_time_set_arrows(&up_arrow, &down_arrow, TIME_SET_MINUTE_ARROW_X, &mut data);
            draw_time_set_text(
                &font,
                &visible_time_set_minute_display(time_set),
                TIME_SET_MINUTE_TEXT_X * SOURCE_TILE_SIZE,
                TIME_SET_MINUTE_TEXT_Y * SOURCE_TILE_SIZE,
                &mut data,
            )?;
        }
        VisibleTimeSetPhase::HourConfirm | VisibleTimeSetPhase::MinuteConfirm => {
            draw_time_set_textbox(
                &font,
                &frame,
                &visible_time_set_visible_dialog(time_set),
                0,
                TIME_SET_TEXTBOX_Y,
                20,
                TIME_SET_TEXTBOX_HEIGHT,
                &mut data,
            )?;
            draw_time_set_window(
                &frame,
                TIME_SET_YES_NO_BOX_X,
                TIME_SET_YES_NO_BOX_Y,
                TIME_SET_YES_NO_BOX_WIDTH,
                TIME_SET_YES_NO_BOX_HEIGHT,
                &mut data,
            )?;
            let cursor = time_set.yes_no_index.min(1);
            draw_time_set_text(
                &font,
                if cursor == 0 { "▶YES" } else { " YES" },
                (TIME_SET_YES_NO_BOX_X + 1) * SOURCE_TILE_SIZE,
                (TIME_SET_YES_NO_BOX_Y + 1) * SOURCE_TILE_SIZE,
                &mut data,
            )?;
            draw_time_set_text(
                &font,
                if cursor == 1 { "▶NO" } else { " NO" },
                (TIME_SET_YES_NO_BOX_X + 1) * SOURCE_TILE_SIZE,
                (TIME_SET_YES_NO_BOX_Y + 2) * SOURCE_TILE_SIZE,
                &mut data,
            )?;
        }
        VisibleTimeSetPhase::WakeDialogue | VisibleTimeSetPhase::FinalReaction => {
            draw_time_set_textbox(
                &font,
                &frame,
                &visible_time_set_visible_dialog(time_set),
                0,
                TIME_SET_TEXTBOX_Y,
                20,
                TIME_SET_TEXTBOX_HEIGHT,
                &mut data,
            )?;
        }
        VisibleTimeSetPhase::Complete => {}
    }

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn draw_time_set_textbox(
    font: &image::RgbaImage,
    frame: &image::RgbaImage,
    text: &str,
    x_tiles: usize,
    y_tiles: usize,
    width_tiles: usize,
    height_tiles: usize,
    target: &mut [u8],
) -> Result<()> {
    draw_time_set_window(frame, x_tiles, y_tiles, width_tiles, height_tiles, target)?;
    let max_chars_per_line = width_tiles.saturating_sub(2).max(1);
    let max_lines = height_tiles.saturating_sub(2);
    let lines = wrap_boot_text_for_box(text, max_chars_per_line, max_lines);
    let clip_x = (x_tiles + 1) * SOURCE_TILE_SIZE;
    let clip_y = (y_tiles + 1) * SOURCE_TILE_SIZE;
    let clip_width = width_tiles.saturating_sub(2) * SOURCE_TILE_SIZE;
    let clip_height = height_tiles.saturating_sub(2) * SOURCE_TILE_SIZE;
    for (line_index, line) in lines.iter().enumerate() {
        draw_time_set_text_clipped(
            font,
            line,
            (x_tiles + 1) * SOURCE_TILE_SIZE,
            (y_tiles + 2 + line_index) * SOURCE_TILE_SIZE,
            Some((clip_x, clip_y, clip_width, clip_height)),
            target,
        )?;
    }
    Ok(())
}

fn wrap_boot_text_for_box(text: &str, max_chars_per_line: usize, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    let normalized = normalize_boot_text(text);
    let mut lines = Vec::new();
    for raw_line in normalized.split('\n') {
        if raw_line.is_empty() {
            lines.push(String::new());
            if lines.len() >= max_lines {
                break;
            }
            continue;
        }
        let mut current = String::new();
        for raw_word in raw_line.split_whitespace() {
            let mut word = raw_word.to_string();
            if word.contains('@') {
                if !current.is_empty() {
                    lines.push(current.trim_end().to_string());
                    current.clear();
                    if lines.len() >= max_lines {
                        break;
                    }
                }
                word = word.replace('@', "");
                if word.is_empty() {
                    continue;
                }
            }
            let candidate = if current.is_empty() {
                word.clone()
            } else {
                format!("{current} {word}")
            };
            if boot_text_tile_len(&candidate) <= max_chars_per_line {
                current = candidate;
            } else {
                if !current.is_empty() {
                    lines.push(current);
                    if lines.len() >= max_lines {
                        current = String::new();
                        break;
                    }
                }
                current = word;
            }
        }
        if lines.len() >= max_lines {
            break;
        }
        if !current.is_empty() {
            lines.push(current);
            if lines.len() >= max_lines {
                break;
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines.truncate(max_lines);
    lines
}

fn normalize_boot_text(text: &str) -> String {
    text.replace("<……>", "……")
        .replace("<POKE>", "#")
        .replace('#', "POKé")
}

fn boot_text_tile_len(text: &str) -> usize {
    tokenize_name_entry_string(text)
        .into_iter()
        .filter(|token| token != "@")
        .count()
}

fn draw_time_set_window(
    frame: &image::RgbaImage,
    x_tiles: usize,
    y_tiles: usize,
    width_tiles: usize,
    height_tiles: usize,
    target: &mut [u8],
) -> Result<()> {
    if frame.width() != (SOURCE_TILE_SIZE * 3) as u32
        || frame.height() != (SOURCE_TILE_SIZE * 2) as u32
    {
        anyhow::bail!(
            "time-set frame must be 24x16, got {}x{}",
            frame.width(),
            frame.height()
        );
    }
    let x_px = x_tiles * SOURCE_TILE_SIZE;
    let y_px = y_tiles * SOURCE_TILE_SIZE;
    for y in y_px + SOURCE_TILE_SIZE..y_px + (height_tiles - 1) * SOURCE_TILE_SIZE {
        for x in x_px + SOURCE_TILE_SIZE..x_px + (width_tiles - 1) * SOURCE_TILE_SIZE {
            put_time_set_pixel(target, x, y, [255, 255, 255, 255]);
        }
    }
    blit_time_set_tile_image(frame, 0, 0, x_px, y_px, false, false, false, target);
    blit_time_set_tile_image(
        frame,
        SOURCE_TILE_SIZE * 2,
        0,
        x_px + (width_tiles - 1) * SOURCE_TILE_SIZE,
        y_px,
        false,
        false,
        false,
        target,
    );
    blit_time_set_tile_image(
        frame,
        SOURCE_TILE_SIZE,
        SOURCE_TILE_SIZE,
        x_px,
        y_px + (height_tiles - 1) * SOURCE_TILE_SIZE,
        false,
        false,
        false,
        target,
    );
    blit_time_set_tile_image(
        frame,
        SOURCE_TILE_SIZE * 2,
        SOURCE_TILE_SIZE,
        x_px + (width_tiles - 1) * SOURCE_TILE_SIZE,
        y_px + (height_tiles - 1) * SOURCE_TILE_SIZE,
        false,
        false,
        false,
        target,
    );
    for column in 1..width_tiles - 1 {
        let x = x_px + column * SOURCE_TILE_SIZE;
        blit_time_set_tile_image(
            frame,
            SOURCE_TILE_SIZE,
            0,
            x,
            y_px,
            false,
            false,
            false,
            target,
        );
        blit_time_set_tile_image(
            frame,
            SOURCE_TILE_SIZE,
            0,
            x,
            y_px + (height_tiles - 1) * SOURCE_TILE_SIZE,
            false,
            false,
            false,
            target,
        );
    }
    for row in 1..height_tiles - 1 {
        let y = y_px + row * SOURCE_TILE_SIZE;
        blit_time_set_tile_image(
            frame,
            0,
            SOURCE_TILE_SIZE,
            x_px,
            y,
            false,
            false,
            false,
            target,
        );
        blit_time_set_tile_image(
            frame,
            0,
            SOURCE_TILE_SIZE,
            x_px + (width_tiles - 1) * SOURCE_TILE_SIZE,
            y,
            false,
            false,
            false,
            target,
        );
    }
    Ok(())
}

fn draw_time_set_arrows(
    up_arrow: &image::RgbaImage,
    down_arrow: &image::RgbaImage,
    x_tiles: usize,
    target: &mut [u8],
) {
    blit_time_set_tile_image(
        up_arrow,
        0,
        0,
        x_tiles * SOURCE_TILE_SIZE,
        TIME_SET_ARROW_TOP_Y * SOURCE_TILE_SIZE,
        false,
        false,
        true,
        target,
    );
    blit_time_set_tile_image(
        down_arrow,
        0,
        0,
        x_tiles * SOURCE_TILE_SIZE,
        TIME_SET_ARROW_BOTTOM_Y * SOURCE_TILE_SIZE,
        false,
        false,
        true,
        target,
    );
}

fn draw_time_set_text(
    font: &image::RgbaImage,
    text: &str,
    x_px: usize,
    y_px: usize,
    target: &mut [u8],
) -> Result<()> {
    draw_time_set_text_clipped(font, text, x_px, y_px, None, target)
}

fn draw_time_set_text_clipped(
    font: &image::RgbaImage,
    text: &str,
    x_px: usize,
    y_px: usize,
    clip: Option<(usize, usize, usize, usize)>,
    target: &mut [u8],
) -> Result<()> {
    let mut cursor_x = x_px;
    let mut cursor_y = y_px;
    for token in tokenize_name_entry_string(text) {
        if token == "\n" {
            cursor_x = x_px;
            cursor_y += SOURCE_TILE_SIZE;
            continue;
        }
        let tile_id = name_entry_token_tile(&token)
            .with_context(|| format!("unsupported time-set glyph {token:?}"))?;
        draw_time_set_font_tile_clipped(font, tile_id, cursor_x, cursor_y, clip, target)?;
        cursor_x += SOURCE_TILE_SIZE;
    }
    Ok(())
}

fn draw_time_set_font_tile_clipped(
    font: &image::RgbaImage,
    tile_id: u8,
    dest_x: usize,
    dest_y: usize,
    clip: Option<(usize, usize, usize, usize)>,
    target: &mut [u8],
) -> Result<()> {
    if tile_id == 0 || tile_id == NAME_ENTRY_SPACE_TILE {
        return Ok(());
    }
    if tile_id < 0x80 {
        anyhow::bail!("time-set glyph tile 0x{tile_id:02x} is outside font.png");
    }
    let font_index = usize::from(tile_id - 0x80);
    let tiles_per_row = font.width() as usize / SOURCE_TILE_SIZE;
    if tiles_per_row == 0 {
        anyhow::bail!("time-set font has invalid width {}", font.width());
    }
    let source_x = (font_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (font_index / tiles_per_row) * SOURCE_TILE_SIZE;
    blit_time_set_tile_image_clipped(
        font, source_x, source_y, dest_x, dest_y, false, false, true, clip, target,
    );
    Ok(())
}

fn put_time_set_pixel(target: &mut [u8], x: usize, y: usize, rgba: [u8; 4]) {
    const TARGET_WIDTH: usize = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    const TARGET_HEIGHT: usize = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    if x >= TARGET_WIDTH || y >= TARGET_HEIGHT {
        return;
    }
    let offset = (y * TARGET_WIDTH + x) * 4;
    target[offset] = rgba[0];
    target[offset + 1] = rgba[1];
    target[offset + 2] = rgba[2];
    target[offset + 3] = rgba[3];
}

fn blit_time_set_tile_image(
    source: &image::RgbaImage,
    source_x: usize,
    source_y: usize,
    dest_x: usize,
    dest_y: usize,
    xflip: bool,
    yflip: bool,
    white_transparent: bool,
    target: &mut [u8],
) {
    blit_time_set_tile_image_clipped(
        source,
        source_x,
        source_y,
        dest_x,
        dest_y,
        xflip,
        yflip,
        white_transparent,
        None,
        target,
    );
}

fn blit_time_set_tile_image_clipped(
    source: &image::RgbaImage,
    source_x: usize,
    source_y: usize,
    dest_x: usize,
    dest_y: usize,
    xflip: bool,
    yflip: bool,
    white_transparent: bool,
    clip: Option<(usize, usize, usize, usize)>,
    target: &mut [u8],
) {
    const TARGET_WIDTH: usize = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    const TARGET_HEIGHT: usize = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let clip_right = clip.map(|(x, _, width, _)| x.saturating_add(width));
    let clip_bottom = clip.map(|(_, y, _, height)| y.saturating_add(height));
    for row in 0..SOURCE_TILE_SIZE {
        for col in 0..SOURCE_TILE_SIZE {
            let sample_x = if xflip {
                SOURCE_TILE_SIZE - 1 - col
            } else {
                col
            };
            let sample_y = if yflip {
                SOURCE_TILE_SIZE - 1 - row
            } else {
                row
            };
            let sx = source_x + sample_x;
            let sy = source_y + sample_y;
            if sx >= source.width() as usize || sy >= source.height() as usize {
                continue;
            }
            let dx = dest_x + col;
            let dy = dest_y + row;
            if dx >= TARGET_WIDTH || dy >= TARGET_HEIGHT {
                continue;
            }
            if let Some((clip_x, clip_y, _, _)) = clip {
                if dx < clip_x
                    || dy < clip_y
                    || clip_right.is_some_and(|right| dx >= right)
                    || clip_bottom.is_some_and(|bottom| dy >= bottom)
                {
                    continue;
                }
            }
            let pixel = source.get_pixel(sx as u32, sy as u32);
            if pixel[3] == 0 {
                continue;
            }
            if white_transparent && pixel[0] > 248 && pixel[1] > 248 && pixel[2] > 248 {
                continue;
            }
            let offset = (dy * TARGET_WIDTH + dx) * 4;
            target[offset] = pixel[0];
            target[offset + 1] = pixel[1];
            target[offset + 2] = pixel[2];
            target[offset + 3] = pixel[3];
        }
    }
}

fn spawn_visible_oak_intro_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    oak_intro: &VisibleOakIntroSequence,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let trainer = &snapshot.trainer;
    let key = oak_intro_art_key(oak_intro, trainer.player_gender);
    if !rendered_art.oak_intro_cache.contains_key(&key) {
        match load_oak_intro_screen_frame(
            &runtime_shell.asset_root,
            oak_intro,
            trainer,
            rendered_art,
            images,
        ) {
            Ok(frame) => {
                rendered_art.oak_intro_errors.remove(&key);
                rendered_art.oak_intro_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .oak_intro_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    let Some(frame) = rendered_art.oak_intro_cache.get(&key).cloned() else {
        let error = rendered_art
            .oak_intro_errors
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "unknown Oak intro frame render error".to_string());
        anyhow::bail!("required Oak intro frame could not be rendered: {error}");
    };
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        TitleScreenMarker,
    ));
    Ok(())
}

const OAK_INTRO_SPRITE_X: usize = 48;
const OAK_INTRO_SPRITE_Y: usize = 32;
const OAK_INTRO_TEXTBOX_Y: usize = 12;
const OAK_INTRO_TEXTBOX_HEIGHT: usize = 6;
const OAK_INTRO_PROMPT_ARROW_X: usize = 18 * SOURCE_TILE_SIZE;
const OAK_INTRO_PROMPT_ARROW_Y: usize = 16 * SOURCE_TILE_SIZE;

fn oak_intro_art_key(oak_intro: &VisibleOakIntroSequence, player_gender: u8) -> OakIntroArtKey {
    OakIntroArtKey {
        mode: oak_intro.mode,
        scene_state: oak_intro.scene_state.clone(),
        scene_phase: oak_intro.scene_phase,
        current_sprite: oak_intro.current_sprite.clone(),
        player_gender,
        current_text: oak_intro.current_text.clone(),
        visible_chars: oak_intro.visible_chars,
        waiting_for_input: oak_intro.waiting_for_input,
        blink_visible: oak_intro.waiting_for_input && oak_intro.blink_timer < 30,
        wipe_active: oak_intro.wipe_active,
        wipe_window_x: oak_intro.wipe_window_x,
        fade_active: oak_intro.fade_active,
        fade_alpha: oak_intro.fade_alpha,
    }
}

fn load_oak_intro_screen_frame(
    asset_root: &AssetRoot,
    oak_intro: &VisibleOakIntroSequence,
    trainer: &crate::RuntimeTrainerSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode Oak intro font PNG")?
        .to_rgba8();
    let textbox_frame = image::open(assets.join("gfx/frames/1.png"))
        .context("decode Oak intro textbox frame PNG")?
        .to_rgba8();
    let down_arrow = image::open(assets.join("gfx/new_game/down_arrow.png"))
        .context("decode Oak intro down-arrow PNG")?
        .to_rgba8();
    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![255_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    if let Some(sprite_frame) =
        oak_intro_sprite_frame(rendered_art, asset_root, oak_intro, trainer, images)
    {
        let sprite_image = images
            .get(&sprite_frame.handle)
            .context("Oak intro sprite frame image missing from Bevy assets")?;
        blit_sprite_frame_image(
            sprite_image,
            OAK_INTRO_SPRITE_X,
            OAK_INTRO_SPRITE_Y,
            width,
            height,
            &mut data,
        );
    } else if let Some(sprite) = oak_intro.current_sprite.as_deref() {
        let error = oak_intro_art_error(rendered_art, oak_intro, trainer);
        anyhow::bail!("required Oak intro sprite {sprite} could not be rendered: {error}");
    }

    let dialog = format_oak_intro_dialog(oak_intro);
    if !dialog.is_empty() {
        draw_time_set_textbox(
            &font,
            &textbox_frame,
            &dialog,
            0,
            OAK_INTRO_TEXTBOX_Y,
            TIME_SET_SCREEN_TILE_WIDTH,
            OAK_INTRO_TEXTBOX_HEIGHT,
            &mut data,
        )?;
        if oak_intro.waiting_for_input && oak_intro.blink_timer < 30 {
            blit_time_set_tile_image(
                &down_arrow,
                0,
                0,
                OAK_INTRO_PROMPT_ARROW_X,
                OAK_INTRO_PROMPT_ARROW_Y,
                false,
                false,
                true,
                &mut data,
            );
        }
    }

    if oak_intro.wipe_active {
        let wipe_x = usize::from(oak_intro.wipe_window_x.min(VISIBLE_OAK_WIPE_END_X));
        fill_native_rect(
            &mut data,
            width,
            wipe_x,
            0,
            width.saturating_sub(wipe_x),
            height,
            255,
        );
    }
    if oak_intro.fade_active || oak_intro.fade_alpha > 0 {
        fade_native_to_white(&mut data, oak_intro.fade_alpha);
    }

    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn blit_sprite_frame_image(
    source: &Image,
    dest_x: usize,
    dest_y: usize,
    target_width: usize,
    target_height: usize,
    target: &mut [u8],
) {
    let source_width = source.texture_descriptor.size.width as usize;
    let source_height = source.texture_descriptor.size.height as usize;
    for row in 0..source_height {
        for col in 0..source_width {
            let dx = dest_x + col;
            let dy = dest_y + row;
            if dx >= target_width || dy >= target_height {
                continue;
            }
            let source_offset = (row * source_width + col) * 4;
            if source_offset + 3 >= source.data.len() || source.data[source_offset + 3] == 0 {
                continue;
            }
            let target_offset = (dy * target_width + dx) * 4;
            target[target_offset] = source.data[source_offset];
            target[target_offset + 1] = source.data[source_offset + 1];
            target[target_offset + 2] = source.data[source_offset + 2];
            target[target_offset + 3] = 255;
        }
    }
}

fn fill_native_rect(
    target: &mut [u8],
    target_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    value: u8,
) {
    for row in y..y.saturating_add(height) {
        for col in x..x.saturating_add(width) {
            let offset = (row * target_width + col) * 4;
            if offset + 3 >= target.len() {
                continue;
            }
            target[offset] = value;
            target[offset + 1] = value;
            target[offset + 2] = value;
            target[offset + 3] = 255;
        }
    }
}

fn fade_native_to_white(target: &mut [u8], alpha: u8) {
    let alpha = u16::from(alpha);
    let inv_alpha = 255_u16.saturating_sub(alpha);
    for pixel in target.chunks_exact_mut(4) {
        pixel[0] = ((u16::from(pixel[0]) * inv_alpha + 255 * alpha) / 255) as u8;
        pixel[1] = ((u16::from(pixel[1]) * inv_alpha + 255 * alpha) / 255) as u8;
        pixel[2] = ((u16::from(pixel[2]) * inv_alpha + 255 * alpha) / 255) as u8;
        pixel[3] = 255;
    }
}

const CREDITS_SCREEN_WIDTH: usize = 20 * SOURCE_TILE_SIZE;
const CREDITS_SCREEN_HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
const CREDITS_MON_FRAME_SIZE: usize = 32;
const CREDITS_FRAMES_PER_SCENE: usize = 4;
fn render_visible_credits_frame(
    asset_root: &AssetRoot,
    credits: &VisibleCreditsScreen,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let palettes = load_credits_palette_sets(asset_root)?;
    let palette_set = palettes
        .get(usize::from(credits.scene_index & 0x03))
        .context("credits palette set missing")?;
    let bg_palette = &palette_set[0];
    let border_palette = &palette_set[1];
    let text_palette = &palette_set[2];
    let mut data = vec![0_u8; CREDITS_SCREEN_WIDTH * CREDITS_SCREEN_HEIGHT * 4];
    fill_visible_credits_rect(
        &mut data,
        0,
        0,
        CREDITS_SCREEN_WIDTH,
        CREDITS_SCREEN_HEIGHT,
        bg_palette[0],
    );
    draw_visible_credits_mon_strip(asset_root, credits, bg_palette, &mut data)?;
    fill_visible_credits_rect(
        &mut data,
        0,
        5 * SOURCE_TILE_SIZE,
        CREDITS_SCREEN_WIDTH,
        12 * SOURCE_TILE_SIZE,
        text_palette[0],
    );
    draw_visible_credits_border_rows(asset_root, border_palette, &mut data)?;
    draw_visible_credits_text(asset_root, credits, text_palette, &mut data)?;
    if credits.show_the_end || credits.awaiting_exit {
        draw_visible_credits_the_end(asset_root, text_palette, &mut data)?;
    }
    apply_visible_credits_line_scroll(credits, &mut data);
    let mut image = Image::new(
        Extent3d {
            width: CREDITS_SCREEN_WIDTH as u32,
            height: CREDITS_SCREEN_HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(CREDITS_SCREEN_WIDTH as f32, CREDITS_SCREEN_HEIGHT as f32),
    })
}

fn load_credits_palette_sets(asset_root: &AssetRoot) -> Result<Vec<[Palette; 3]>> {
    let palette_path = asset_root.runtime_assets().join("gfx/credits/credits.pal");
    let content = std::fs::read_to_string(&palette_path)
        .with_context(|| format!("read credits palette {}", palette_path.display()))?;
    let palettes = parse_palette_file(&content, None)?;
    if palettes.len() % 3 != 0 {
        anyhow::bail!(
            "credits palette {} should be grouped in threes, got {} palettes",
            palette_path.display(),
            palettes.len()
        );
    }
    let mut sets = Vec::new();
    for chunk in palettes.chunks(3) {
        sets.push([chunk[0], chunk[1], chunk[2]]);
    }
    if sets.len() != 4 {
        anyhow::bail!(
            "credits palettes should contain exactly 4 scene sets, got {}",
            sets.len()
        );
    }
    Ok(sets)
}

fn draw_visible_credits_mon_strip(
    asset_root: &AssetRoot,
    credits: &VisibleCreditsScreen,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let frame_levels = visible_credits_mon_frame_levels(asset_root, credits)?;
    for x in (0..CREDITS_SCREEN_WIDTH).step_by(CREDITS_MON_FRAME_SIZE) {
        blit_visible_credits_levels(
            target,
            &frame_levels,
            CREDITS_MON_FRAME_SIZE,
            CREDITS_MON_FRAME_SIZE,
            x,
            0,
            palette,
            false,
        );
    }
    Ok(())
}

fn visible_credits_mon_frame_levels(
    asset_root: &AssetRoot,
    credits: &VisibleCreditsScreen,
) -> Result<Vec<u8>> {
    let mut levels = vec![2_u8; CREDITS_MON_FRAME_SIZE * CREDITS_MON_FRAME_SIZE];
    if let Some(frame) = credits.border_frame_top {
        let frame_levels = load_visible_credits_mon_frame_levels(asset_root, frame)?;
        copy_visible_credits_frame_half(&mut levels, &frame_levels, 0);
    }
    if let Some(frame) = credits.border_frame_bottom {
        let frame_levels = load_visible_credits_mon_frame_levels(asset_root, frame)?;
        copy_visible_credits_frame_half(&mut levels, &frame_levels, CREDITS_MON_FRAME_SIZE / 2);
    }
    Ok(levels)
}

fn load_visible_credits_mon_frame_levels(
    asset_root: &AssetRoot,
    frame: VisibleCreditsBorderFrame,
) -> Result<Vec<u8>> {
    let mon = match frame.mon_index % 4 {
        0 => "pichu",
        1 => "smoochum",
        2 => "ditto",
        3 => "igglybuff",
        _ => unreachable!(),
    };
    let path = asset_root
        .runtime_assets()
        .join("gfx/credits")
        .join(format!("{mon}.png"));
    let source = image::open(&path)
        .with_context(|| format!("decode credits mon frame {}", path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    let expected_height = (CREDITS_MON_FRAME_SIZE * CREDITS_FRAMES_PER_SCENE) as u32;
    if width != CREDITS_MON_FRAME_SIZE as u32 || height != expected_height {
        anyhow::bail!(
            "credits mon frame {} must be {}x{}, got {}x{}",
            path.display(),
            CREDITS_MON_FRAME_SIZE,
            expected_height,
            width,
            height
        );
    }
    let frame_index = usize::from(frame.frame_index) % CREDITS_FRAMES_PER_SCENE;
    extract_visible_credits_levels(
        &source,
        0,
        frame_index * CREDITS_MON_FRAME_SIZE,
        CREDITS_MON_FRAME_SIZE,
        CREDITS_MON_FRAME_SIZE,
    )
}

fn copy_visible_credits_frame_half(target: &mut [u8], source: &[u8], target_y: usize) {
    for row in 0..(CREDITS_MON_FRAME_SIZE / 2) {
        for col in 0..CREDITS_MON_FRAME_SIZE {
            let source_index = (target_y + row) * CREDITS_MON_FRAME_SIZE + col;
            let target_index = (target_y + row) * CREDITS_MON_FRAME_SIZE + col;
            if let (Some(target_level), Some(source_level)) =
                (target.get_mut(target_index), source.get(source_index))
            {
                *target_level = *source_level;
            }
        }
    }
}

fn draw_visible_credits_border_rows(
    asset_root: &AssetRoot,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let path = asset_root.runtime_assets().join("gfx/credits/border.png");
    let source = image::open(&path)
        .with_context(|| format!("decode credits border {}", path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width != 24 || height != 24 {
        anyhow::bail!(
            "credits border {} must be 24x24, got {}x{}",
            path.display(),
            width,
            height
        );
    }
    let tiles = (0..9)
        .map(|index| {
            extract_visible_credits_levels(
                &source,
                (index % 3) * SOURCE_TILE_SIZE,
                (index / 3) * SOURCE_TILE_SIZE,
                SOURCE_TILE_SIZE,
                SOURCE_TILE_SIZE,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    draw_visible_credits_border_row(target, &tiles, 4, 4, palette);
    draw_visible_credits_border_row(target, &tiles, 17, 0, palette);
    Ok(())
}

fn draw_visible_credits_border_row(
    target: &mut [u8],
    tiles: &[Vec<u8>],
    row: usize,
    base_index: usize,
    palette: &Palette,
) {
    let mut tile_x = 0;
    for _ in 0..(CREDITS_SCREEN_WIDTH / (SOURCE_TILE_SIZE * 4)) {
        for offset in 0..4 {
            if let Some(tile) = tiles.get(base_index + offset) {
                blit_visible_credits_levels(
                    target,
                    tile,
                    SOURCE_TILE_SIZE,
                    SOURCE_TILE_SIZE,
                    tile_x * SOURCE_TILE_SIZE,
                    row * SOURCE_TILE_SIZE,
                    palette,
                    false,
                );
            }
            tile_x += 1;
        }
    }
}

fn draw_visible_credits_text(
    asset_root: &AssetRoot,
    credits: &VisibleCreditsScreen,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let font = load_visible_credits_font_tiles(asset_root)?;
    for line in &credits.lines {
        if line.token == "COPYRIGHT" {
            draw_visible_credits_copyright(asset_root, credits, palette, target)?;
            continue;
        }
        for (line_offset, tile_ids) in line.tiles.iter().enumerate() {
            let mut draw_x = 0;
            let draw_y = (6 + usize::from(line.line_index) * 2) * SOURCE_TILE_SIZE
                + line_offset * SOURCE_TILE_SIZE;
            for tile_id in tile_ids {
                if *tile_id != 0x7f {
                    let levels = font.levels.get(tile_id).with_context(|| {
                        format!("credits font tile 0x{tile_id:02x} unavailable")
                    })?;
                    blit_visible_credits_levels(
                        target,
                        levels,
                        SOURCE_TILE_SIZE,
                        SOURCE_TILE_SIZE,
                        draw_x,
                        draw_y,
                        palette,
                        false,
                    );
                }
                draw_x += SOURCE_TILE_SIZE;
            }
        }
    }
    Ok(())
}

fn draw_visible_credits_copyright(
    asset_root: &AssetRoot,
    credits: &VisibleCreditsScreen,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let path = asset_root.runtime_assets().join("gfx/splash/copyright.png");
    let source = image::open(&path)
        .with_context(|| format!("decode credits copyright {}", path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width != 232 || height != SOURCE_TILE_SIZE as u32 {
        anyhow::bail!(
            "credits copyright {} must be 232x8, got {}x{}",
            path.display(),
            width,
            height
        );
    }
    let draw_y = (6 + usize::from(
        credits
            .lines
            .iter()
            .find(|line| line.token == "COPYRIGHT")
            .map(|line| line.line_index)
            .unwrap_or(0),
    ) * 2)
        * SOURCE_TILE_SIZE;
    for tile_index in 0..29 {
        let levels = extract_visible_credits_levels(
            &source,
            tile_index * SOURCE_TILE_SIZE,
            0,
            SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
        )?;
        blit_visible_credits_levels(
            target,
            &levels,
            SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
            (2 + tile_index) * SOURCE_TILE_SIZE,
            draw_y,
            palette,
            false,
        );
    }
    Ok(())
}

fn draw_visible_credits_the_end(
    asset_root: &AssetRoot,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let path = asset_root.runtime_assets().join("gfx/credits/theend.png");
    let source = image::open(&path)
        .with_context(|| format!("decode credits The End {}", path.display()))?
        .to_rgba8();
    let levels = extract_visible_credits_levels(&source, 0, 0, 64, 16)?;
    blit_visible_credits_levels(
        target,
        &levels,
        64,
        16,
        6 * SOURCE_TILE_SIZE,
        9 * SOURCE_TILE_SIZE,
        palette,
        true,
    );
    Ok(())
}

fn load_visible_credits_font_tiles(asset_root: &AssetRoot) -> Result<CreditsFontTiles> {
    let font_root = asset_root.runtime_assets().join("gfx/font");
    let mut levels = BTreeMap::new();
    load_visible_credits_font_png_tiles(&font_root.join("font.png"), 0x80, true, &mut levels)?;
    load_visible_credits_single_font_tile(&font_root.join("space.png"), 0x7f, &mut levels)?;
    load_visible_credits_font_png_tiles(
        &font_root.join("font_battle_extra.png"),
        0x60,
        false,
        &mut levels,
    )?;
    load_visible_credits_font_extra_tiles(&font_root.join("font_extra.png"), &mut levels)?;
    load_visible_credits_single_font_tile(&font_root.join("up_arrow.png"), 0x61, &mut levels)?;
    load_visible_credits_single_font_tile(&font_root.join("phone_icon.png"), 0x62, &mut levels)?;
    load_visible_credits_frame_tiles(asset_root, &mut levels)?;
    Ok(CreditsFontTiles { levels })
}

fn load_visible_credits_font_png_tiles(
    path: &Path,
    base_tile_id: u16,
    store_zero_based_aliases: bool,
    target: &mut BTreeMap<u16, Vec<u8>>,
) -> Result<()> {
    let source = image::open(path)
        .with_context(|| format!("decode credits font tiles {}", path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width % SOURCE_TILE_SIZE as u32 != 0 || height % SOURCE_TILE_SIZE as u32 != 0 {
        anyhow::bail!(
            "credits font {} has invalid dimensions {}x{}",
            path.display(),
            width,
            height
        );
    }
    let tiles_wide = width as usize / SOURCE_TILE_SIZE;
    let tile_count = tiles_wide * (height as usize / SOURCE_TILE_SIZE);
    for tile_index in 0..tile_count {
        let levels = extract_visible_credits_levels(
            &source,
            (tile_index % tiles_wide) * SOURCE_TILE_SIZE,
            (tile_index / tiles_wide) * SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
        )?;
        target.insert(base_tile_id + tile_index as u16, levels.clone());
        if store_zero_based_aliases {
            target.entry(tile_index as u16).or_insert(levels);
        }
    }
    Ok(())
}

fn load_visible_credits_font_extra_tiles(
    path: &Path,
    target: &mut BTreeMap<u16, Vec<u8>>,
) -> Result<()> {
    let source = image::open(path)
        .with_context(|| format!("decode credits font extra {}", path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width != 128 || height != 16 {
        anyhow::bail!(
            "credits font extra {} must be 128x16, got {}x{}",
            path.display(),
            width,
            height
        );
    }
    let tiles_wide = width as usize / SOURCE_TILE_SIZE;
    for offset in 0..22 {
        let tile_index = 3 + offset;
        let levels = extract_visible_credits_levels(
            &source,
            (tile_index % tiles_wide) * SOURCE_TILE_SIZE,
            (tile_index / tiles_wide) * SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
        )?;
        target.insert(0x63 + offset as u16, levels);
    }
    Ok(())
}

fn load_visible_credits_single_font_tile(
    path: &Path,
    tile_id: u16,
    target: &mut BTreeMap<u16, Vec<u8>>,
) -> Result<()> {
    let source = image::open(path)
        .with_context(|| format!("decode credits font tile {}", path.display()))?
        .to_rgba8();
    let levels = extract_visible_credits_levels(&source, 0, 0, SOURCE_TILE_SIZE, SOURCE_TILE_SIZE)?;
    target.insert(tile_id, levels);
    Ok(())
}

fn load_visible_credits_frame_tiles(
    asset_root: &AssetRoot,
    target: &mut BTreeMap<u16, Vec<u8>>,
) -> Result<()> {
    let path = asset_root.runtime_assets().join("gfx/frames/1.png");
    let source = image::open(&path)
        .with_context(|| format!("decode credits frame tiles {}", path.display()))?
        .to_rgba8();
    let tile_ids = [0x79_u16, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e];
    for (index, tile_id) in tile_ids.iter().copied().enumerate() {
        let levels = extract_visible_credits_levels(
            &source,
            (index % 3) * SOURCE_TILE_SIZE,
            (index / 3) * SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
        )?;
        target.insert(tile_id, levels);
    }
    Ok(())
}

fn extract_visible_credits_levels(
    source: &image::RgbaImage,
    source_x: usize,
    source_y: usize,
    width: usize,
    height: usize,
) -> Result<Vec<u8>> {
    if source_x + width > source.width() as usize || source_y + height > source.height() as usize {
        anyhow::bail!("credits source rect exceeds source image dimensions");
    }
    let mut levels = Vec::with_capacity(width * height);
    for row in 0..height {
        for col in 0..width {
            let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            levels.push(visible_credits_gray_level(
                pixel[0], pixel[1], pixel[2], pixel[3],
            ));
        }
    }
    Ok(levels)
}

fn visible_credits_gray_level(red: u8, green: u8, blue: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        return 0;
    }
    let value = ((u16::from(red) + u16::from(green) + u16::from(blue)) / 3) as u8;
    if value > 213 {
        0
    } else if value > 160 {
        1
    } else if value > 96 {
        2
    } else {
        3
    }
}

fn fill_visible_credits_rect(
    target: &mut [u8],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 3],
) {
    for row in y..(y + height).min(CREDITS_SCREEN_HEIGHT) {
        for col in x..(x + width).min(CREDITS_SCREEN_WIDTH) {
            let offset = (row * CREDITS_SCREEN_WIDTH + col) * 4;
            target[offset] = color[0];
            target[offset + 1] = color[1];
            target[offset + 2] = color[2];
            target[offset + 3] = 255;
        }
    }
}

fn blit_visible_credits_levels(
    target: &mut [u8],
    levels: &[u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    palette: &Palette,
    transparent_zero: bool,
) {
    for row in 0..height {
        for col in 0..width {
            let target_x = x + col;
            let target_y = y + row;
            if target_x >= CREDITS_SCREEN_WIDTH || target_y >= CREDITS_SCREEN_HEIGHT {
                continue;
            }
            let level = usize::from(*levels.get(row * width + col).unwrap_or(&0)).min(3);
            if transparent_zero && level == 0 {
                continue;
            }
            let color = palette[level];
            let offset = (target_y * CREDITS_SCREEN_WIDTH + target_x) * 4;
            target[offset] = color[0];
            target[offset + 1] = color[1];
            target[offset + 2] = color[2];
            target[offset + 3] = 255;
        }
    }
}

fn apply_visible_credits_line_scroll(credits: &VisibleCreditsScreen, target: &mut [u8]) {
    if credits.ly_override == 0 {
        return;
    }
    let source = target.to_vec();
    let shift = if credits.ly_override < 128 {
        i16::from(credits.ly_override)
    } else {
        i16::from(credits.ly_override) - 256
    };
    if shift == 0 {
        return;
    }
    for (start, count) in [(0x1f_usize, 8_usize), (0x87_usize, 8_usize)] {
        for y in start..(start + count).min(CREDITS_SCREEN_HEIGHT) {
            for x in 0..CREDITS_SCREEN_WIDTH {
                let source_x = (x as i16 + shift).rem_euclid(CREDITS_SCREEN_WIDTH as i16) as usize;
                let source_offset = (y * CREDITS_SCREEN_WIDTH + source_x) * 4;
                let target_offset = (y * CREDITS_SCREEN_WIDTH + x) * 4;
                target[target_offset..target_offset + 4]
                    .copy_from_slice(&source[source_offset..source_offset + 4]);
            }
        }
    }
}

fn visible_credits_screen_lines(credits: &VisibleCreditsScreen) -> Vec<String> {
    let mut lines = credits
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    if credits.show_the_end || credits.awaiting_exit {
        lines.push("THE END".to_string());
    }
    lines
}

/// Query bundle for the retained viewport renderer. Keeping these queries in
/// one SystemParam avoids Bevy's function-system parameter limit while still
/// allowing the incremental path to update player/NPC transforms in place.
#[derive(SystemParam)]
struct RenderEntityQueries<'w, 's> {
    tiles: Query<'w, 's, Entity, With<PlayfieldTile>>,
    map_sprites: Query<
        'w,
        's,
        &'static mut Transform,
        (
            With<PlayfieldTile>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<DialogGlyphMarker>,
        ),
    >,
    players: Query<'w, 's, Entity, Or<(With<PlayerMarker>, With<PlayerFacingMarker>)>>,
    player_sprites: Query<'w, 's, (&'static mut Transform, &'static Sprite), With<PlayerMarker>>,
    objects: Query<'w, 's, Entity, With<ObjectMarker>>,
    object_sprites: Query<
        'w,
        's,
        (
            &'static VisibleObjectSprite,
            &'static mut Transform,
            &'static Sprite,
        ),
        Without<PlayerMarker>,
    >,
    events: Query<'w, 's, Entity, With<EventMarker>>,
    prompts: Query<'w, 's, Entity, With<FieldPromptMarker>>,
    field_commands: Query<'w, 's, Entity, With<FieldCommandMarker>>,
    scene_dialogs: Query<'w, 's, Entity, With<SceneDialogMarker>>,
    dialog_glyphs: Query<
        'w,
        's,
        (
            &'static DialogGlyphMarker,
            &'static mut Handle<Image>,
            &'static mut Transform,
            &'static mut Sprite,
        ),
        (
            With<DialogGlyphMarker>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<VisibleIntroSurface>,
        ),
    >,
    dialog_frame_tiles: Query<'w, 's, Entity, With<SceneDialogWindowFrameMarker>>,
    dialog_text_box_backgrounds: Query<'w, 's, Entity, With<SceneDialogTextBoxBackgroundMarker>>,
    intro_surfaces: Query<'w, 's, &'static mut Handle<Image>, With<VisibleIntroSurface>>,
    pokemon_pictures: Query<'w, 's, Entity, With<PokemonPictureMarker>>,
    title_markers: Query<'w, 's, Entity, With<TitleScreenMarker>>,
    battlers: Query<'w, 's, Entity, With<BattleBattlerMarker>>,
    battle_commands: Query<'w, 's, Entity, With<BattleCommandMarker>>,
}

fn set_overworld_map_scroll(
    map_sprites: &mut Query<
        &mut Transform,
        (
            With<PlayfieldTile>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<DialogGlyphMarker>,
        ),
    >,
    offset: Vec2,
) {
    let Ok(mut transform) = map_sprites.get_single_mut() else {
        return;
    };
    transform.translation.x = -TILE_SIZE * 0.5 + offset.x;
    transform.translation.y = TILE_SIZE * 0.5 + offset.y;
}

fn render_playfield(
    mut commands: Commands,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut rendered: ResMut<RenderedViewport>,
    mut tileset_art: ResMut<RenderedTilesetArt>,
    mut images: ResMut<Assets<Image>>,
    entity_queries: RenderEntityQueries,
) {
    let RenderEntityQueries {
        tiles,
        mut map_sprites,
        players,
        mut player_sprites,
        objects,
        mut object_sprites,
        events,
        prompts,
        field_commands,
        scene_dialogs,
        mut dialog_glyphs,
        dialog_frame_tiles,
        dialog_text_box_backgrounds,
        mut intro_surfaces,
        pokemon_pictures,
        title_markers,
        battlers,
        battle_commands,
    } = entity_queries;
    if let Some(intro) = runtime_shell.intro_screen.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        if rendered.title_active {
            // Do not despawn the LCD surface between intro frames.  On macOS
            // the queued despawn/spawn pair can be displayed as a black frame
            // before the new texture upload completes.  Replacing the handle
            // leaves the last complete frame on screen until the next one is
            // ready.
            match intro_scene_frame_for_art_with_bundle(
                &mut tileset_art,
                &runtime_shell.asset_root,
                runtime_shell
                    .shell
                    .runtime()
                    .data()
                    .sprite_anim_bundle
                    .as_str(),
                &intro,
                &mut images,
            ) {
                Some(frame) => {
                    if let Ok(mut texture) = intro_surfaces.get_single_mut() {
                        *texture = frame.handle;
                        rendered.shell_render_key = Some(shell_render_key);
                        return;
                    }
                }
                None => {
                    let key = intro_scene_art_key(&intro);
                    let error = tileset_art
                        .intro_scene_errors
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| "unknown intro art load error".to_string());
                    record_visible_render_error(
                        &mut commands,
                        &mut runtime_shell,
                        anyhow::anyhow!(
                            "required intro scene {} frame {} could not be rendered: {}",
                            intro.scene_name(),
                            intro.scene_frame_counter,
                            error
                        ),
                    );
                    return;
                }
            }
            // A transition from another title surface (or a stale entity) can
            // legitimately leave no intro entity.  Fall through once to make
            // a fresh surface; subsequent animation updates stay retained.
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_intro_screen(
            &mut commands,
            &runtime_shell,
            &intro,
            &mut tileset_art,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(credits) = runtime_shell.credits_screen.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) =
            spawn_visible_credits_screen(&mut commands, &runtime_shell, &credits, &mut images)
        {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(delete_save) = runtime_shell.pending_delete_save.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_delete_save_screen(
            &mut commands,
            &runtime_shell,
            &delete_save,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(mystery_gift) = runtime_shell.pending_mystery_gift.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_mystery_gift_screen(
            &mut commands,
            &runtime_shell,
            &mystery_gift,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(clock_reset) = runtime_shell.pending_clock_reset.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_clock_reset_screen(
            &mut commands,
            &runtime_shell,
            &clock_reset,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(title) = runtime_shell.title_menu.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_title_screen(
            &mut commands,
            &mut runtime_shell,
            &title,
            &mut tileset_art,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        if runtime_shell.options_menu_open {
            match runtime_shell.shell.snapshot() {
                Ok(snapshot) => spawn_options_menu_command_window(
                    &mut commands,
                    &snapshot,
                    &runtime_shell,
                    &mut tileset_art,
                    &runtime_shell.asset_root,
                    &mut images,
                ),
                Err(error) => record_visible_render_error(&mut commands, &mut runtime_shell, error),
            }
        }
        return;
    }
    if let Some(oak_intro) = runtime_shell.pending_oak_intro.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_oak_intro_screen(
            &mut commands,
            &runtime_shell,
            &oak_intro,
            &mut tileset_art,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(gender) = runtime_shell.pending_gender_selection.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_gender_selection_screen(
            &mut commands,
            &runtime_shell,
            &gender,
            &mut tileset_art,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    if let Some(time_set) = runtime_shell.pending_time_set.clone() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
        for entity in events.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in scene_dialogs.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in title_markers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battlers.iter() {
            commands.entity(entity).despawn();
        }
        for entity in battle_commands.iter() {
            commands.entity(entity).despawn();
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_time_set_screen(
            &mut commands,
            &runtime_shell,
            &time_set,
            &mut tileset_art,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    // Avoid snapshot/checksum work on idle frames. Gameplay actions bump the
    // revision; walking animation and queued audio are the only visual work
    // that can legitimately require a refresh without a new snapshot.
    if rendered.snapshot_revision == Some(runtime_shell.snapshot_revision)
        && runtime_shell.pending_audio.is_empty()
        && runtime_shell.player_walk_frame_ticks == 0
    {
        return;
    }
    let Ok(current_snapshot) = cached_runtime_snapshot(&mut runtime_shell) else {
        return;
    };
    let field_snapshot = runtime_shell
        .field_notice_scene
        .as_ref()
        .filter(|_| {
            runtime_shell.field_notice.is_some()
                || runtime_shell.pending_field_notice_effect_frames.is_some()
                || matches!(
                    runtime_shell.visible_fly_animation,
                    Some(VisibleFlyAnimation { phase: VisibleFlyAnimationPhase::From, .. })
                )
                || (runtime_shell.visible_waterfall_animation.is_some()
                    && runtime_shell.field_notice.is_none())
        })
        .cloned()
        .unwrap_or(current_snapshot);
    let snapshot = if !runtime_shell.battle_messages.is_empty()
        || matches!(
            runtime_shell.visible_blackout_phase,
            Some(VisibleBlackoutPhase::FadeOut | VisibleBlackoutPhase::WhiteHold { .. })
        )
    {
        runtime_shell
            .battle_message_scene
            .as_ref()
            .map(|scene| Arc::new(scene.as_ref().clone()))
            .unwrap_or(field_snapshot)
    } else {
        field_snapshot
    };
    if rendered
        .map_name
        .as_ref()
        .is_some_and(|map_name| map_name != &snapshot.overworld.map_name)
    {
        sync_visible_map_name_sign(&mut runtime_shell, &snapshot);
    }
    let terminal_battle_scene = (!runtime_shell.battle_messages.is_empty())
        .then(|| runtime_shell.battle_message_scene.as_deref().cloned())
        .flatten();
    let state_hash = snapshot.visual_state_hash;
    let shell_render_key = shell_render_key(&runtime_shell);
    let world_key = overworld_render_world_key(&snapshot);
    let dialog_key = visible_scene_dialog_entries(&snapshot, &runtime_shell)
        .ok()
        .map(|entries| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            entries.hash(&mut hasher);
            strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
                .hash(&mut hasher);
            hasher.finish()
        });
    // Script text advances one character at a time and changes the semantic
    // checksum every frame. Rebuilding the entire 20x18 map (tiles, NPC
    // sprites, and images) for each character was the source of the 100%+
    // CPU spikes and the apparent one-FPS runtime. Refresh only the dialog
    // layer when the world itself is unchanged.
    let dialog_only_update = rendered.title_active == false
        && rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.tile == Some(snapshot.overworld.tile)
        && snapshot.battle.is_none()
        && snapshot.ui.menu.is_none()
        && snapshot.pending_shop.is_none()
        && snapshot.ui.active_pokemon_picture.is_none()
        && snapshot.pending_move_learn.is_none()
        && runtime_shell.pending_name_input.is_none()
        && runtime_shell.pending_name_choice.is_none()
        && runtime_shell.pending_time_set.is_none()
        && runtime_shell.pending_oak_intro.is_none()
        && runtime_shell.pending_gender_selection.is_none()
        && runtime_shell.visible_balance_overlay.is_none()
        && runtime_shell.visible_mom_bank.is_none()
        && (snapshot.ui.text.is_some() || snapshot.ui.pending_yes_no.is_some());
    if dialog_only_update && rendered.dialog_key != dialog_key {
        let has_retained_dialog_frame = dialog_frame_tiles.iter().next().is_some();
        if has_retained_dialog_frame
            && update_scene_dialog_text_content_in_place(
                &snapshot,
                &runtime_shell,
                &mut dialog_glyphs,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            )
        {
            rendered.state_hash = Some(state_hash);
            rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
            rendered.dialog_key = dialog_key;
            rendered.shell_render_key = Some(shell_render_key);
            return;
        }
        let retained_dialog_frames = dialog_frame_tiles
            .iter()
            .chain(dialog_text_box_backgrounds.iter())
            .collect::<HashSet<_>>();
        for entity in scene_dialogs
            .iter()
            .filter(|entity| !retained_dialog_frames.contains(entity))
        {
            commands.entity(entity).despawn();
        }
        for entity in field_commands.iter() {
            commands.entity(entity).despawn();
        }
        for entity in prompts.iter() {
            commands.entity(entity).despawn();
        }
        for entity in pokemon_pictures.iter() {
            commands.entity(entity).despawn();
        }
        if has_retained_dialog_frame {
            spawn_scene_dialog_text_content(
                &mut commands,
                &snapshot,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            );
        } else {
            spawn_scene_dialog(
                &mut commands,
                &snapshot,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            );
        }
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.dialog_key = dialog_key;
        rendered.shell_render_key = Some(shell_render_key);
        return;
    }
    if dialog_only_update && rendered.dialog_key == dialog_key {
        // The runtime checksum can advance while text/script bookkeeping is
        // unchanged visually.  Keep every retained world entity and only
        // acknowledge the new semantic revision.
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.shell_render_key = Some(shell_render_key);
        return;
    }
    // Save/menu/script bookkeeping can change the semantic checksum without
    // changing any pixels in the overworld. Retain the existing map, player,
    // and object entities in that case; rebuilding them here was the other
    // major source of frame stalls after the tile layer was composited.
    let world_only_update = !rendered.title_active
        && rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.tile == Some(snapshot.overworld.tile)
        && rendered.world_key == Some(world_key)
        && snapshot.battle.is_none()
        && snapshot.ui.text.is_none()
        && snapshot.ui.pending_yes_no.is_none()
        && snapshot.ui.menu.is_none()
        && snapshot.pending_shop.is_none()
        && snapshot.ui.active_pokemon_picture.is_none()
        // These menus are shell-owned overlays and are not represented in
        // the core snapshot UI.  They must force a render so opening Options
        // (or the start/party/gear menus) cannot be swallowed by the cheap
        // world-only acknowledgement path.
        && runtime_shell.start_menu_cursor.is_none()
        && !runtime_shell.options_menu_open
        && !runtime_shell.party_menu_open
        && !runtime_shell.pokedex_menu_open
        && !runtime_shell.pokegear_menu_open
        && !runtime_shell.trainer_card_open
        && !visible_field_pack_is_open(&runtime_shell)
        && !runtime_shell.save_menu_open
        && runtime_shell.special_boundary.is_none()
        && snapshot.pending_move_learn.is_none()
        && runtime_shell.pending_name_input.is_none()
        && runtime_shell.pending_name_choice.is_none()
        && runtime_shell.pending_time_set.is_none()
        && runtime_shell.pending_oak_intro.is_none()
        && runtime_shell.pending_gender_selection.is_none()
        && runtime_shell.last_error.is_none()
        && !runtime_debug_overlays_enabled()
        // A changed shell key means an overlay was opened or closed.  Let the
        // normal rebuild path remove stale command-window entities.
        && rendered.shell_render_key == Some(shell_render_key);
    if world_only_update
        && rendered.player_sprite_facing == Some(snapshot.overworld.facing)
        && rendered.player_sprite_mode == Some(snapshot.overworld.mode)
        && (runtime_shell.player_walk_frame_ticks > 0 || rendered.walk_viewport_origin.is_some())
    {
        // The semantic snapshot stays at the destination tile while the LCD
        // is still showing the in-between walking frames. Move the retained
        // sprites only; rebuilding the map here would reintroduce the black
        // flash this fast path exists to prevent.
        if let Some((start_x, start_y)) = rendered.viewport_origin
            && update_overworld_sprite_positions(
                &snapshot,
                runtime_shell.player_walk_from,
                runtime_shell.player_walk_frame_ticks,
                runtime_shell.trainer_walk_from.as_ref(),
                runtime_shell.object_walk_frame_ticks,
                overworld_walk_camera_offset(&rendered, runtime_shell.player_walk_frame_ticks),
                start_x,
                start_y,
                &mut player_sprites,
                &mut object_sprites,
            )
        {
            set_overworld_map_scroll(
                &mut map_sprites,
                overworld_walk_camera_offset(&rendered, runtime_shell.player_walk_frame_ticks),
            );
            if runtime_shell.player_walk_frame_ticks == 0 {
                rendered.walk_viewport_origin = None;
            }
            rendered.state_hash = Some(state_hash);
            rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
            rendered.shell_render_key = Some(shell_render_key);
            return;
        }
    }
    if world_only_update
        && rendered.player_sprite_facing == Some(snapshot.overworld.facing)
        && rendered.player_sprite_mode == Some(snapshot.overworld.mode)
    {
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.shell_render_key = Some(shell_render_key);
        return;
    }
    if rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.tile == Some(snapshot.overworld.tile)
        && rendered.state_hash == Some(state_hash)
        && rendered.shell_render_key == Some(shell_render_key)
    {
        return;
    }

    // Never remove a drawable map merely because an overworld snapshot has
    // changed. A turn, walking commit, or camera scroll updates this same
    // retained LCD surface; tearing it down exposes Bevy's clear colour for
    // a frame before its replacement is available.
    let retain_walking_viewport = rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name);
    if !retain_walking_viewport {
        for entity in tiles.iter() {
            commands.entity(entity).despawn();
        }
    }
    let retain_player_sprite = retain_walking_viewport
        && runtime_shell.player_walk_frame_ticks > 0
        && rendered.player_sprite_facing == Some(snapshot.overworld.facing)
        && rendered.player_sprite_mode == Some(snapshot.overworld.mode);
    if !retain_player_sprite {
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }
    }
    let retain_object_sprites = retain_walking_viewport && runtime_shell.player_walk_frame_ticks > 0;
    if !retain_object_sprites && runtime_shell.visible_fly_animation.is_none() {
        for entity in objects.iter() {
            commands.entity(entity).despawn();
        }
    }
    for entity in events.iter() {
        commands.entity(entity).despawn();
    }
    for entity in prompts.iter() {
        commands.entity(entity).despawn();
    }
    for entity in field_commands.iter() {
        commands.entity(entity).despawn();
    }
    for entity in scene_dialogs.iter() {
        commands.entity(entity).despawn();
    }
    for entity in pokemon_pictures.iter() {
        commands.entity(entity).despawn();
    }
    for entity in title_markers.iter() {
        commands.entity(entity).despawn();
    }
    for entity in battlers.iter() {
        commands.entity(entity).despawn();
    }
    for entity in battle_commands.iter() {
        commands.entity(entity).despawn();
    }

    let Some(map) = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
    else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "active overworld map {} is missing from verified map catalog",
                snapshot.overworld.map_name
            ),
        );
        return;
    };

    let Some(player_render_x) = snapshot
        .overworld
        .tile
        .x
        .checked_mul(RENDER_TILES_PER_RUNTIME_TILE)
    else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "visible viewport x origin overflows render coordinates from player tile {}",
                snapshot.overworld.tile.x
            ),
        );
        return;
    };
    let Some(player_render_y) = snapshot
        .overworld
        .tile
        .y
        .checked_mul(RENDER_TILES_PER_RUNTIME_TILE)
    else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "visible viewport y origin overflows render coordinates from player tile {}",
                snapshot.overworld.tile.y
            ),
        );
        return;
    };
    let Ok((width, height)) =
        render_tile_bounds_i16(&map.map_name, map.attributes.width, map.attributes.height)
    else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "active map {} dimensions {}x{} overflow supported runtime tile bounds",
                map.map_name,
                map.attributes.width,
                map.attributes.height
            ),
        );
        return;
    };
    let start_x = render_viewport_origin(player_render_x, width, VIEWPORT_TILES_X);
    let start_y = render_viewport_origin(player_render_y, height, VIEWPORT_TILES_Y);
    let expected_block_count =
        usize::from(map.attributes.width) * usize::from(map.attributes.height);
    if map.blocks.len() != expected_block_count {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "active map {} block count {} does not match declared dimensions {}x{}",
                map.map_name,
                map.blocks.len(),
                map.attributes.width,
                map.attributes.height
            ),
        );
        return;
    }
    let Some(tileset) = snapshot
        .tilesets
        .iter()
        .find(|tileset| tileset.tileset_id == map.attributes.tileset_name)
    else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "active map {} references missing verified tileset {}",
                map.map_name,
                map.attributes.tileset_name
            ),
        );
        return;
    };
    if let Err(error) = validate_render_map_event_coordinates(map) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    let tileset_art_key = TilesetArtKey {
        tileset_id: tileset.tileset_id.clone(),
        time_of_day: map
            .attributes
            .time_of_day
            .as_deref()
            .unwrap_or("day")
            .to_string(),
    };
    let map_visual_key = {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Compiled map blocks and palette tables are immutable pack data.
        // Hashing the complete block array here made every walking frame scan
        // the whole map even though only the player transform changed.  The
        // map name plus its immutable rendering parameters is the identity
        // needed by the retained viewport cache.
        map.map_name.hash(&mut hasher);
        map.attributes.tileset_name.hash(&mut hasher);
        tileset_art_key.time_of_day.hash(&mut hasher);
        map.attributes.border_block.hash(&mut hasher);
        tileset.palette_map.hash(&mut hasher);
        hasher.finish()
    };
    let position_key = overworld_render_position_key(&snapshot);
    let appearance_key = overworld_render_appearance_key(&snapshot);

    // A normal walking step changes the player/NPC transforms and can shift
    // the camera by a tile.  Retain the already-created LCD sprite in both
    // cases: despawning it before a replacement GPU texture is available
    // makes macOS present the clear (black) surface for one frame.
    let can_update_positions_in_place = !rendered.title_active
        && rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.map_visual_key == Some(map_visual_key)
        && rendered.world_key != Some(world_key)
        && rendered.position_key != Some(position_key)
        && rendered.appearance_key == Some(appearance_key)
        && snapshot.battle.is_none()
        && snapshot.ui.text.is_none()
        && snapshot.ui.pending_yes_no.is_none()
        && snapshot.ui.menu.is_none()
        && snapshot.pending_shop.is_none()
        && snapshot.ui.active_pokemon_picture.is_none()
        && snapshot.pending_move_learn.is_none()
        && runtime_shell.last_error.is_none()
        && runtime_shell.pending_name_input.is_none()
        && runtime_shell.pending_name_choice.is_none()
        && !runtime_debug_overlays_enabled();
    // A changed camera origin also changes the pixels behind every sprite.
    // Let the retained texture update below before moving the sprites;
    // returning here left the old viewport in place and caused the subsequent
    // render to tear down the visible playfield.
    if can_update_positions_in_place
        && rendered.viewport_origin == Some((start_x, start_y))
        && update_overworld_sprite_positions(
            &snapshot,
            runtime_shell.player_walk_from,
            runtime_shell.player_walk_frame_ticks,
            runtime_shell.trainer_walk_from.as_ref(),
            runtime_shell.object_walk_frame_ticks,
            overworld_walk_camera_offset(&rendered, runtime_shell.player_walk_frame_ticks),
            start_x,
            start_y,
            &mut player_sprites,
            &mut object_sprites,
        )
    {
        rendered.tile = Some(snapshot.overworld.tile);
        rendered.world_key = Some(world_key);
        rendered.position_key = Some(position_key);
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.shell_render_key = Some(shell_render_key);
        return;
    }
    if !tileset_art.cache.contains_key(&tileset_art_key) {
        match load_tileset_art(
            &runtime_shell.asset_root,
            &tileset_art_key.tileset_id,
            &tileset_art_key.time_of_day,
            &tileset.palette_map,
            &mut images,
        ) {
            Ok(art) => {
                tileset_art.errors.remove(&tileset_art_key);
                tileset_art.cache.insert(tileset_art_key.clone(), art);
            }
            Err(error) => {
                tileset_art
                    .errors
                    .insert(tileset_art_key.clone(), error.to_string());
            }
        }
    }
    if !tileset_art.cache.contains_key(&tileset_art_key) {
        let error = tileset_art
            .errors
            .get(&tileset_art_key)
            .cloned()
            .unwrap_or_else(|| "unknown tileset art load error".to_string());
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "required tileset art {} ({}) could not be rendered: {}",
                tileset_art_key.tileset_id,
                tileset_art_key.time_of_day,
                error
            ),
        );
        return;
    }

    // Keep the map layer as one retained sprite instead of 360 independent
    // Bevy entities.  The old per-tile entity churn was the dominant cost on
    // every camera step and was the reason the shell could fall to one FPS.
    // Objects, player, dialog, and battle overlays remain separate layers.
    let mut viewport_tile_handles = Vec::with_capacity(
        usize::try_from(VIEWPORT_TILES_X * VIEWPORT_TILES_Y).unwrap_or_default(),
    );
    for y in 0..VIEWPORT_TILES_Y {
        for x in 0..VIEWPORT_TILES_X {
            let map_x = i32::from(start_x) + i32::from(x);
            let map_y = i32::from(start_y) + i32::from(y);
            let (block, sub_x, sub_y) = if map_x >= 0
                && map_y >= 0
                && map_x < i32::from(width)
                && map_y < i32::from(height)
            {
                let block_x = map_x.div_euclid(i32::from(RENDER_METATILE_WIDTH));
                let block_y = map_y.div_euclid(i32::from(RENDER_METATILE_WIDTH));
                let sub_x = map_x.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                let sub_y = map_y.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                let index = (block_y as usize * map.attributes.width as usize) + block_x as usize;
                (map.blocks[index], sub_x, sub_y)
            } else {
                let sub_x = map_x.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                let sub_y = map_y.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                (map.attributes.border_block as u16, sub_x, sub_y)
            };
            let Some(tile_handle) = tileset_art
                .cache
                .get(&tileset_art_key)
                .and_then(|art| art.tile_handle(block, sub_x, sub_y))
            else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "required tileset art {} missing metatile {} sub-tile ({}, {})",
                        tileset_art_key.tileset_id,
                        block,
                        sub_x,
                        sub_y
                    ),
                );
                return;
            };
            viewport_tile_handles.push(tile_handle.clone());
        }
    }
    let viewport_texture = if let Some(active_texture) = rendered.map_texture.clone() {
        // Keep the handle used by the visible playfield entity. Updating its
        // pixels in place leaves the last complete frame drawable while the
        // renderer uploads the new camera origin, rather than exposing a
        // black clear frame between despawn and spawn.
        compose_viewport_tiles(&viewport_tile_handles, Some(active_texture), &mut images)
    } else {
        let composite_key = (map_visual_key, start_x, start_y);
        if let Some((_, handle)) = tileset_art
            .viewport_composites
            .iter()
            .find(|(key, _)| *key == composite_key)
        {
            handle.clone()
        } else {
            let handle = compose_viewport_tiles(
                &viewport_tile_handles,
                // Cached composites are immutable entries. Reusing the previous
                // image here would mutate an image already stored for another
                // camera origin and make the cache return the wrong viewport.
                None,
                &mut images,
            );
            tileset_art
                .viewport_composites
                .push_back((composite_key, handle.clone()));
            while tileset_art.viewport_composites.len() > 4 {
                if let Some((_, evicted)) = tileset_art.viewport_composites.pop_front() {
                    // Bevy does not reclaim an image merely because its
                    // Handle was dropped. Explicitly remove evicted camera
                    // composites so walking through a large map cannot grow
                    // the GPU asset store without bound.
                    images.remove(&evicted);
                }
            }
            handle
        }
    };
    let previous_viewport_origin = rendered.viewport_origin;
    rendered.map_texture = Some(viewport_texture.clone());
    rendered.viewport_origin = Some((start_x, start_y));
    rendered.walk_viewport_origin = if runtime_shell.player_walk_frame_ticks > 0
        && previous_viewport_origin != Some((start_x, start_y))
    {
        previous_viewport_origin
    } else {
        None
    };
    rendered.map_visual_key = Some(map_visual_key);
    set_overworld_map_scroll(
        &mut map_sprites,
        overworld_walk_camera_offset(&rendered, runtime_shell.player_walk_frame_ticks),
    );
    if can_update_positions_in_place
        && tiles.iter().count() == 1
        && update_overworld_sprite_positions(
            &snapshot,
            runtime_shell.player_walk_from,
            runtime_shell.player_walk_frame_ticks,
            runtime_shell.trainer_walk_from.as_ref(),
            runtime_shell.object_walk_frame_ticks,
            overworld_walk_camera_offset(&rendered, runtime_shell.player_walk_frame_ticks),
            start_x,
            start_y,
            &mut player_sprites,
            &mut object_sprites,
        )
    {
        rendered.tile = Some(snapshot.overworld.tile);
        rendered.world_key = Some(world_key);
        rendered.position_key = Some(position_key);
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.shell_render_key = Some(shell_render_key);
        return;
    }
    if tiles.iter().next().is_none() {
        commands.spawn((
            SpriteBundle {
                texture: viewport_texture,
                sprite: Sprite {
                    custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                    ..default()
                },
                // Match the center of the old per-tile layer: tile (0, 0) was
                // centered at (-320, 288), so the 20x18 composite is centered at
                // (-16, 16).
                transform: Transform::from_xyz(-TILE_SIZE * 0.5, TILE_SIZE * 0.5, 0.0),
                ..default()
            },
            PlayfieldTile,
        ));
    }

    if !retain_object_sprites {
    // `LoadAndSortSprites` in the ASM/TypeScript renderer orders objects by
    // their live runtime Y coordinate, then X. Declaration order is not a
    // rendering order: using it makes NPCs draw in front of characters they
    // should be behind (especially after scripted movement).
    let mut visible_object_indices: Vec<usize> = (0..snapshot.visible_objects.len()).collect();
    visible_object_indices.sort_by_key(|index| {
        let object = &snapshot.visible_objects[*index];
        let tile = object
            .object_identifier
            .as_ref()
            .and_then(|object_id| {
                snapshot
                    .visible_object_runtime_tiles
                    .get(object_id)
                    .copied()
            })
            .or_else(|| object_tile_position_checked(object));
        tile.map(|tile| (tile.y, tile.x))
            .unwrap_or((i16::MAX, i16::MAX))
    });
    for index in visible_object_indices {
        let object = &snapshot.visible_objects[index];
        let object_tile = object
            .object_identifier
            .as_ref()
            .and_then(|object_id| {
                snapshot
                    .visible_object_runtime_tiles
                    .get(object_id)
                    .copied()
            })
            .or_else(|| object_tile_position_checked(object));
        let Some(object_tile) = object_tile else {
            record_visible_render_error(
                &mut commands,
                &mut runtime_shell,
                anyhow::anyhow!(
                    "visible object {:?} on {} has out-of-range runtime coordinates ({}, {})",
                    object.object_identifier,
                    snapshot.overworld.map_name,
                    object.x,
                    object.y
                ),
            );
            return;
        };
        let Some((view_x, view_y)) = runtime_event_view_tile(object_tile, start_x, start_y) else {
            continue;
        };
        if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
            continue;
        }

        let sprite_id = resolve_visible_object_sprite_asset_id(
            &runtime_shell.asset_root,
            &object.sprite,
            &snapshot.script_events.variable_sprites,
            &snapshot.presentation.menu_icons,
        );
        // ASM object_event palette 0 means "use the sprite's compiled
        // default", not palette bank 0.  The TS renderer resolves this
        // before instantiating the animation; doing a raw `pal & 7` here
        // makes most NPCs use the wrong colors.
        let palette_id = resolve_visible_object_palette(
            &object.sprite,
            object.pal,
            &snapshot.presentation.sprite_palette_defaults,
        );
        let direction = object
            .object_identifier
            .as_ref()
            .and_then(|object_id| snapshot.visible_object_facings.get(object_id).copied())
            .or_else(|| object_event_initial_facing(&object.spritemovedata))
            .unwrap_or(Direction::Down);
        let render_sprite_id = sprite_id.clone();
        let sprite_frame = sprite_frame_for_art(
            &mut tileset_art,
            &runtime_shell.asset_root,
            &render_sprite_id,
            palette_id,
            map.attributes.time_of_day.as_deref().unwrap_or("day"),
            direction,
            false,
            &mut images,
        );
        let rock_smash_target = runtime_shell.visible_rock_smash_target == Some(object_tile);
        let walking_frame =
            if (object_sprite_is_animated(&object.spritemovedata) || rock_smash_target)
                && sprite_frame.is_some()
            {
                sprite_frame_for_art(
                    &mut tileset_art,
                    &runtime_shell.asset_root,
                    &render_sprite_id,
                    palette_id,
                    map.attributes.time_of_day.as_deref().unwrap_or("day"),
                    direction,
                    true,
                    &mut images,
                )
            } else {
                None
            };
        if let Some(frame) = sprite_frame {
            let animated = object_sprite_is_animated(&object.spritemovedata);
            let (object_x, object_y) = overworld_sprite_position(view_x, view_y, frame.size);
            let rock_smash_action_frame = rock_smash_target
                && runtime_shell
                    .pending_field_notice_effect_frames
                    .is_some_and(|remaining| remaining > 0 && remaining <= 10 && remaining % 2 == 1);
            let visible_handle = if rock_smash_action_frame {
                let Some(action_frame) = walking_frame.as_ref() else {
                    record_visible_render_error(
                        &mut commands,
                        &mut runtime_shell,
                        anyhow::anyhow!("Rock Smash target has no required action frame"),
                    );
                    return;
                };
                action_frame.handle.clone()
            } else {
                frame.handle.clone()
            };
            commands.spawn((
                SpriteBundle {
                    texture: visible_handle,
                    sprite: Sprite {
                        custom_size: Some(frame.size),
                        ..default()
                    },
                    // Match LoadAndSortSprites: objects farther down the map
                    // are nearer the camera and must draw over objects above
                    // them.  The tiny epsilon keeps the 2-D layer ordering
                    // deterministic without changing screen coordinates.
                    transform: Transform::from_xyz(
                        object_x,
                        object_y,
                        1.0 + f32::from(object_tile.y) * 0.001,
                    ),
                    ..default()
                },
                ObjectMarker,
                VisibleObjectSprite {
                    object_index: index,
                    object_identifier: object.object_identifier.clone(),
                    standing: frame.handle.clone(),
                    walking: walking_frame.map(|frame| frame.handle),
                    animated,
                },
            ));
        } else {
            let key = SpriteArtKey {
                sprite_id: render_sprite_id.clone(),
                palette_id,
                time_of_day: normalize_tileset_time_of_day(
                    map.attributes.time_of_day.as_deref().unwrap_or("day"),
                ),
            };
            let error = tileset_art
                .sprite_errors
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "unknown overworld sprite load error".to_string());
            record_visible_render_error(
                &mut commands,
                &mut runtime_shell,
                anyhow::anyhow!(
                    "required overworld sprite art {} palette {} could not be rendered: {}",
                    sprite_id,
                    palette_id,
                    error
                ),
            );
            return;
        }
        if runtime_debug_overlays_enabled() {
            spawn_object_label(
                &mut commands,
                object,
                PLAYFIELD_LEFT + view_x as f32 * TILE_SIZE,
                PLAYFIELD_TOP - view_y as f32 * TILE_SIZE,
            );
        }
    }

    }

    if let Err(error) = spawn_visible_cut_animation(
        &mut commands,
        &runtime_shell,
        &snapshot,
        &mut tileset_art,
        &mut images,
        start_x,
        start_y,
        map.attributes.time_of_day.as_deref().unwrap_or("day"),
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    if let Err(error) = spawn_visible_whirlpool_animation(
        &mut commands,
        &runtime_shell,
        &snapshot,
        &mut tileset_art,
        &mut images,
        start_x,
        start_y,
        map.attributes.time_of_day.as_deref().unwrap_or("day"),
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    if let Err(error) = spawn_visible_headbutt_animation(
        &mut commands,
        &runtime_shell,
        &snapshot,
        map,
        &mut tileset_art,
        &mut images,
        start_x,
        start_y,
        map.attributes.time_of_day.as_deref().unwrap_or("day"),
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    if let Some(emote) = runtime_shell.visible_overworld_emote.clone() {
        let target = if matches!(
            emote.object.as_str(),
            "PLAYER" | "PLAYER_OBJECT" | "LAST_TALKED"
        ) && (emote.object != "LAST_TALKED" || snapshot.script_events.last_talked_object.is_none())
        {
            Some(snapshot.overworld.tile)
        } else {
            let object_id = if emote.object == "LAST_TALKED" {
                snapshot.script_events.last_talked_object.as_deref()
            } else {
                Some(emote.object.as_str())
            };
            object_id.and_then(|object_id| {
                snapshot
                    .visible_object_runtime_tiles
                    .get(object_id)
                    .copied()
                    .or_else(|| {
                        snapshot.visible_objects.iter().find_map(|object| {
                            (object.object_identifier.as_deref() == Some(object_id))
                                .then(|| object_tile_position_checked(object))
                                .flatten()
                        })
                    })
            })
        };
        if let Some(target) = target {
            if let Some((x, y)) = runtime_tile_playfield_position(target, start_x, start_y) {
                match emote_frame_for_art(
                    &mut tileset_art,
                    &runtime_shell.asset_root,
                    &emote.emote,
                    &mut images,
                ) {
                    Some(frame) => {
                        commands.spawn((
                            SpriteBundle {
                                texture: frame.handle,
                                sprite: Sprite {
                                    custom_size: Some(frame.size),
                                    ..default()
                                },
                                transform: Transform::from_xyz(x, y + TILE_SIZE * 1.35, 3.2),
                                ..default()
                            },
                            ObjectMarker,
                        ));
                    }
                    None => {
                        let error = tileset_art
                            .emote_errors
                            .get(&emote.emote)
                            .cloned()
                            .unwrap_or_else(|| "unknown emote load error".to_string());
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!(
                                "required emote art {} could not be rendered: {}",
                                emote.emote,
                                error
                            ),
                        );
                        return;
                    }
                };
            }
        }
    }

    if runtime_debug_overlays_enabled() {
        for warp in &map.events.warps {
            let Some(tile) = warp_tile_position_checked(warp) else {
                continue;
            };
            spawn_event_marker(
                &mut commands,
                start_x,
                start_y,
                tile,
                Color::rgb(0.18, 0.42, 0.96),
                TILE_SIZE * 0.34,
                1.1,
            );
            spawn_warp_event_label(&mut commands, start_x, start_y, warp);
        }
        for bg in &map.events.bg_events {
            let Some(tile) = background_event_tile_position_checked(bg) else {
                continue;
            };
            spawn_event_marker(
                &mut commands,
                start_x,
                start_y,
                tile,
                Color::rgb(0.92, 0.92, 0.86),
                TILE_SIZE * 0.26,
                1.2,
            );
            spawn_bg_event_label(&mut commands, start_x, start_y, bg);
        }
        for coord in &map.events.coord_events {
            let Some(tile) = coord_event_tile_position_checked(coord) else {
                continue;
            };
            spawn_event_marker(
                &mut commands,
                start_x,
                start_y,
                tile,
                Color::rgb(0.74, 0.42, 0.94),
                TILE_SIZE * 0.42,
                1.3,
            );
            spawn_coord_event_label(&mut commands, start_x, start_y, coord);
        }
    }

    let Some((player_x, player_y_base)) =
        runtime_tile_playfield_position(snapshot.overworld.tile, start_x, start_y)
    else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "player tile ({}, {}) is outside the visible render viewport origin ({start_x}, {start_y})",
                snapshot.overworld.tile.x,
                snapshot.overworld.tile.y
            ),
        );
        return;
    };
    if let Err(error) = spawn_visible_fly_animation(
        &mut commands,
        &runtime_shell,
        &mut tileset_art,
        &mut images,
        player_x,
        player_y_base,
        map.attributes.time_of_day.as_deref().unwrap_or("day"),
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    let female = snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE;
    let (player_sprite_id, player_sprite_token, player_palette_override) =
        match snapshot.overworld.mode {
            MovementMode::Normal | MovementMode::Skate => {
                if female {
                    ("kris", "SPRITE_KRIS", snapshot.trainer.player_palette_id)
                } else {
                    ("chris", "SPRITE_CHRIS", snapshot.trainer.player_palette_id)
                }
            }
            MovementMode::Bike => {
                if female {
                    (
                        "kris_bike",
                        "SPRITE_KRIS_BIKE",
                        snapshot.trainer.player_palette_id,
                    )
                } else {
                    (
                        "chris_bike",
                        "SPRITE_CHRIS_BIKE",
                        snapshot.trainer.player_palette_id,
                    )
                }
            }
            MovementMode::Surf => ("surf", "SPRITE_SURF", 1),
            MovementMode::SurfPika => ("surfing_pikachu", "SPRITE_SURFING_PIKACHU", 0),
        };
    let player_palette_id = resolve_visible_object_palette(
        player_sprite_token,
        player_palette_override,
        &snapshot.presentation.sprite_palette_defaults,
    );
    let player_art = (
        sprite_frame_for_art(
            &mut tileset_art,
            &runtime_shell.asset_root,
            player_sprite_id,
            player_palette_id,
            map.attributes.time_of_day.as_deref().unwrap_or("day"),
            snapshot.overworld.facing,
            false,
            &mut images,
        ),
        sprite_frame_for_art(
            &mut tileset_art,
            &runtime_shell.asset_root,
            player_sprite_id,
            player_palette_id,
            map.attributes.time_of_day.as_deref().unwrap_or("day"),
            snapshot.overworld.facing,
            true,
            &mut images,
        ),
    );
    if let Some(standing_frame) = player_art.0 {
        let walking_frame = player_art.1;
        let fishing_frame = if runtime_shell.visible_fishing_animation.is_some() {
            match fishing_player_frame(
                &mut tileset_art,
                &runtime_shell.asset_root,
                female,
                snapshot.overworld.facing,
                player_palette_id,
                map.attributes.time_of_day.as_deref().unwrap_or("day"),
                &mut images,
            ) {
                Ok(frame) => Some(frame),
                Err(error) => {
                    record_visible_render_error(&mut commands, &mut runtime_shell, error);
                    return;
                }
            }
        } else {
            None
        };
        let frame = if let Some(frame) = fishing_frame.as_ref() {
            frame
        } else if runtime_shell.player_walk_frame_ticks > 0 && runtime_shell.player_walk_stride {
            walking_frame.as_ref().unwrap_or(&standing_frame)
        } else {
            &standing_frame
        };
        let (player_x, player_y) =
            overworld_sprite_position_from_base(player_x, player_y_base, frame.size);
        if runtime_shell.visible_ledge_jump.is_some() {
            let Some(shadow) = ledge_shadow_frame_for_art(
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) else {
                let error = tileset_art
                    .ledge_shadow_error
                    .clone()
                    .unwrap_or_else(|| "unknown ledge shadow load error".to_string());
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!("required ledge shadow could not be rendered: {error}"),
                );
                return;
            };
            commands.spawn((
                SpriteBundle {
                    texture: shadow.handle,
                    sprite: Sprite {
                        custom_size: Some(shadow.size),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        player_x,
                        player_y - frame.size.y * 0.5,
                        1.0 + f32::from(snapshot.overworld.tile.y) * 0.001,
                    ),
                    ..default()
                },
                PlayerFacingMarker,
                LedgeShadowMarker,
            ));
        }
        if !retain_player_sprite && runtime_shell.visible_fly_animation.is_none() {
            commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(
                    player_x,
                    player_y,
                    1.0 + f32::from(snapshot.overworld.tile.y) * 0.001 + 0.0005,
                ),
                ..default()
            },
            PlayerMarker,
                PlayerSpriteFrames {
                standing: standing_frame.handle.clone(),
                walking: walking_frame.as_ref().map(|frame| frame.handle.clone()),
            },
            ));
            rendered.player_sprite_facing = Some(snapshot.overworld.facing);
            rendered.player_sprite_mode = Some(snapshot.overworld.mode);
        }
        if let Err(error) = spawn_visible_fishing_animation(
            &mut commands,
            &runtime_shell,
            snapshot.overworld.facing,
            player_x,
            player_y,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
        if let Some(rustle) = runtime_shell.visible_grass_rustle {
            let time_of_day = map.attributes.time_of_day.as_deref().unwrap_or("day");
            let Some(frames) = grass_rustle_frames_for_art(
                &mut tileset_art,
                &runtime_shell.asset_root,
                time_of_day,
                &mut images,
            ) else {
                let key = normalize_tileset_time_of_day(time_of_day);
                let error = tileset_art
                    .grass_rustle_errors
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| "unknown grass rustle load error".to_string());
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!("required grass rustle could not be rendered: {error}"),
                );
                return;
            };
            let rustle_frame = &frames[usize::from((rustle.age / 4) % 2)];
            if let Some((base_x, base_y)) =
                runtime_tile_playfield_position(rustle.tile, start_x, start_y)
            {
                let (target_x, target_y) =
                    overworld_sprite_position_from_base(base_x, base_y, frame.size);
                commands.spawn((
                    SpriteBundle {
                        texture: rustle_frame.handle.clone(),
                        sprite: Sprite {
                            custom_size: Some(rustle_frame.size),
                            ..default()
                        },
                        transform: Transform::from_xyz(
                            target_x,
                            target_y - frame.size.y * 0.5 + rustle_frame.size.y * 0.5,
                            2.5 + f32::from(rustle.tile.y) * 0.001,
                        ),
                        ..default()
                    },
                    PlayerFacingMarker,
                    GrassRustleMarker,
                ));
            }
        }
    } else {
        let key = SpriteArtKey {
            sprite_id: player_sprite_id.to_string(),
            palette_id: player_palette_id,
            time_of_day: normalize_tileset_time_of_day(
                map.attributes.time_of_day.as_deref().unwrap_or("day"),
            ),
        };
        let error = tileset_art
            .sprite_errors
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "unknown player sprite load error".to_string());
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "required player sprite art {} palette {} could not be rendered: {}",
                player_sprite_id,
                player_palette_id,
                error
            ),
        );
        return;
    }
    if runtime_debug_overlays_enabled() {
        let (facing_dx, facing_dy) = snapshot.overworld.facing.delta();
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(1.0, 0.95, 0.32),
                    custom_size: Some(player_facing_marker_size(facing_dx, facing_dy)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    player_x + facing_dx as f32 * TILE_SIZE * 0.28,
                    player_y_base - facing_dy as f32 * TILE_SIZE * 0.28,
                    2.2,
                ),
                ..default()
            },
            PlayerFacingMarker,
        ));
    }

    if runtime_debug_overlays_enabled() {
        spawn_map_status_label(&mut commands, &snapshot);
        spawn_map_connection_labels(&mut commands, map);
    }

    if let Some(transition) = runtime_shell.visible_battle_transition {
        spawn_visible_battle_transition(&mut commands, transition, rendered.map_texture.clone());
    } else if snapshot.battle.is_some() || terminal_battle_scene.is_some() {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(248.0 / 255.0, 248.0 / 255.0, 248.0 / 255.0),
                    custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 2.7),
                ..default()
            },
            BattleCommandMarker,
            FixedBattleCanvasMarker,
        ));
    }

    if runtime_shell.visible_battle_transition.is_none()
        && let Some(battle) = &snapshot.battle
    {
        let player_send_out_pending = runtime_shell.battle_player_send_out_pending
            || (runtime_shell.battle_entry_messages_remaining == 0
                && runtime_shell
                    .battle_messages
                    .front()
                    .is_some_and(|message| message.starts_with("Go! ")))
            || runtime_shell
                .visible_trainer_exit_animation
                .as_ref()
                .is_some_and(|animation| {
                    animation.side == crate::core::battle::turn::BattleSide::Player
                });
        let capture_enemy_hidden = runtime_shell
            .visible_capture_animation
            .as_ref()
            .is_some_and(VisibleCaptureAnimation::enemy_hidden);
        let capture_enemy_clip_tiles = runtime_shell
            .visible_capture_animation
            .as_ref()
            .and_then(VisibleCaptureAnimation::enemy_clip_tiles);
        if let Err(error) = spawn_battle_battler_markers(
            &mut commands,
            &snapshot,
            battle,
            runtime_shell.battle_entry_messages_remaining,
            runtime_shell.battle_enemy_send_out_pending,
            player_send_out_pending,
            capture_enemy_hidden,
            capture_enemy_clip_tiles,
            runtime_shell.visible_send_out_animation.as_ref(),
            runtime_shell.visible_trainer_exit_animation.as_ref(),
            runtime_shell.visible_frontpic_animation.as_ref(),
            runtime_shell.visible_move_animations.front(),
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
        if let Err(error) = spawn_battle_hud(
            &mut commands,
            &snapshot,
            battle,
            runtime_shell.battle_entry_messages_remaining,
            runtime_shell.battle_enemy_send_out_pending,
            player_send_out_pending,
            runtime_shell.battle_hp_tween.as_ref(),
            runtime_shell.shell.runtime().growth_rates(),
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
        if let Err(error) = spawn_visible_move_animation_objects(
            &mut commands,
            &snapshot,
            &runtime_shell,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
        spawn_visible_move_animation_overlay(&mut commands, &runtime_shell);
        spawn_battle_command_menu(
            &mut commands,
            &snapshot,
            &runtime_shell,
            battle,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        );
        if let Err(error) = spawn_visible_capture_animation(
            &mut commands,
            &snapshot,
            &runtime_shell,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
        if let Err(error) = spawn_visible_send_out_poof(
            &mut commands,
            &runtime_shell,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
    } else if runtime_shell.visible_battle_transition.is_none()
        && let Some(scene) = terminal_battle_scene.as_ref()
    {
        if let Some(battle) = scene.battle.as_ref() {
            if let Err(error) = spawn_battle_battler_markers(
                &mut commands,
                scene,
                battle,
                runtime_shell.battle_entry_messages_remaining,
                runtime_shell.battle_enemy_send_out_pending,
                runtime_shell.battle_player_send_out_pending,
                runtime_shell
                    .visible_capture_animation
                    .as_ref()
                    .is_some_and(VisibleCaptureAnimation::enemy_hidden),
                runtime_shell
                    .visible_capture_animation
                    .as_ref()
                    .and_then(VisibleCaptureAnimation::enemy_clip_tiles),
                runtime_shell.visible_send_out_animation.as_ref(),
                runtime_shell.visible_trainer_exit_animation.as_ref(),
                runtime_shell.visible_frontpic_animation.as_ref(),
                runtime_shell.visible_move_animations.front(),
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) {
                record_visible_render_error(&mut commands, &mut runtime_shell, error);
                return;
            }
            if let Err(error) = spawn_battle_hud(
                &mut commands,
                scene,
                battle,
                runtime_shell.battle_entry_messages_remaining,
                runtime_shell.battle_enemy_send_out_pending,
                runtime_shell.battle_player_send_out_pending,
                runtime_shell.battle_hp_tween.as_ref(),
                runtime_shell.shell.runtime().growth_rates(),
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) {
                record_visible_render_error(&mut commands, &mut runtime_shell, error);
                return;
            }
            if let Err(error) = spawn_visible_move_animation_objects(
                &mut commands,
                scene,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) {
                record_visible_render_error(&mut commands, &mut runtime_shell, error);
                return;
            }
            spawn_visible_move_animation_overlay(&mut commands, &runtime_shell);
            spawn_battle_command_menu(
                &mut commands,
                scene,
                &runtime_shell,
                battle,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            );
            if let Err(error) = spawn_visible_capture_animation(
                &mut commands,
                scene,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) {
                record_visible_render_error(&mut commands, &mut runtime_shell, error);
                return;
            }
            if let Err(error) = spawn_visible_send_out_poof(
                &mut commands,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) {
                record_visible_render_error(&mut commands, &mut runtime_shell, error);
                return;
            }
        }
    } else if !scene_dialog_surface_active(&snapshot, &runtime_shell) {
        if runtime_debug_overlays_enabled() {
            spawn_field_prompt_marker(
                &mut commands,
                &runtime_shell,
                &snapshot,
                map,
                start_x,
                start_y,
            );
        }
        spawn_field_command_menu(
            &mut commands,
            &snapshot,
            &runtime_shell,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        );
    }
    if let Err(error) = spawn_visible_map_name_sign(
        &mut commands,
        &snapshot,
        &runtime_shell,
        &mut tileset_art,
        &runtime_shell.asset_root,
        &mut images,
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    spawn_scene_dialog(
        &mut commands,
        &snapshot,
        &runtime_shell,
        &mut tileset_art,
        &runtime_shell.asset_root,
        &mut images,
    );
    if let Err(error) = spawn_active_pokemon_picture(
        &mut commands,
        &snapshot,
        &mut tileset_art,
        &runtime_shell.asset_root,
        &mut images,
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    if runtime_shell.last_error.is_some() {
        spawn_shell_error_banner(&mut commands, &runtime_shell);
    } else if runtime_debug_overlays_enabled() {
        spawn_shell_status_banner(&mut commands, &runtime_shell);
        spawn_audio_status_label(&mut commands, &runtime_shell);
    }

    rendered.map_name = Some(snapshot.overworld.map_name.clone());
    rendered.tile = Some(snapshot.overworld.tile);
    rendered.world_key = Some(world_key);
    rendered.position_key = Some(position_key);
    rendered.appearance_key = Some(appearance_key);
    rendered.state_hash = Some(state_hash);
    rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
    rendered.dialog_key = dialog_key;
    rendered.shell_render_key = Some(shell_render_key);
rendered.title_active = false;
}

#[derive(Clone, Copy)]
struct VisibleFlyObjectState {
    x: u8,
    y: u8,
    x_offset: i32,
    angle: u8,
}

#[allow(clippy::too_many_arguments)]
fn spawn_visible_fly_animation(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
    player_origin_x: f32,
    player_origin_y: f32,
    time_of_day: &str,
) -> Result<()> {
    let Some(animation) = runtime_shell.visible_fly_animation else {
        return Ok(());
    };
    let icon_frames = if let Some(frames) = rendered_art.party_icon_cache.get("ICON_BIRD") {
        frames.clone()
    } else {
        let frames = load_party_icon_frame(&runtime_shell.asset_root, "ICON_BIRD", images)?;
        rendered_art.party_icon_cache.insert("ICON_BIRD".to_string(), frames.clone());
        frames
    };
    // TypeScript's fly animation deliberately builds one stateless 16x16
    // surface from the first four icon tiles; it does not run the party-menu
    // two-frame icon cycle here.
    let icon = icon_frames[0].clone();
    let (player_origin_x, player_origin_y) = overworld_sprite_position_from_base(
        player_origin_x,
        player_origin_y,
        icon.size,
    );
    let snapshot = runtime_shell.shell.snapshot()?;
    let (mut player, origin_y) = match animation.phase {
        VisibleFlyAnimationPhase::From => (
            VisibleFlyObjectState { x: 84, y: 80, x_offset: 0, angle: 0 },
            80_u8,
        ),
        VisibleFlyAnimationPhase::To => (
            VisibleFlyObjectState { x: 84, y: 248, x_offset: 0, angle: 0 },
            84_u8,
        ),
    };
    let mut player_delay = 0_u8;
    let mut player_amplitude = match animation.phase {
        VisibleFlyAnimationPhase::From => 0_u8,
        VisibleFlyAnimationPhase::To => 88_u8,
    };
    let mut leaf_counter = 0_u8;
    let mut leaves: Vec<VisibleFlyObjectState> = Vec::new();
    for _ in 0..=animation.frame {
        let previous_counter = leaf_counter;
        leaf_counter = leaf_counter.wrapping_add(1);
        if previous_counter & 7 == 0 {
            let selector = leaf_counter & 24;
            leaves.push(VisibleFlyObjectState {
                x: player.x,
                y: player.y.wrapping_add(selector.wrapping_mul(2)).wrapping_add(64),
                x_offset: 0,
                angle: 0,
            });
        }
        match animation.phase {
            VisibleFlyAnimationPhase::From => {
                if player.y != 0 {
                    let previous_delay = player_delay;
                    player_delay = player_delay.wrapping_add(1);
                    if previous_delay >= 0x40 {
                        player.y = player.y.wrapping_sub(2);
                        if player_amplitude < 0x40 {
                            player_amplitude = player_amplitude.wrapping_add(8);
                        }
                        player.x_offset = visible_battle_anim_sine(
                            player.angle.wrapping_add(0x10),
                            player_amplitude,
                        );
                        player.angle = player.angle.wrapping_add(1);
                    }
                }
            }
            VisibleFlyAnimationPhase::To => {
                if player.y != 84 {
                    player.y = player.y.wrapping_add(2);
                    let amplitude = player_amplitude;
                    if player_amplitude != 0 {
                        player_amplitude = player_amplitude.wrapping_sub(2);
                    }
                    player.x_offset = visible_battle_anim_sine(
                        player.angle.wrapping_add(0x10),
                        amplitude,
                    );
                    player.angle = player.angle.wrapping_add(1);
                }
            }
        }
        leaves.retain_mut(|leaf| {
            if leaf.x >= 184 {
                return false;
            }
            leaf.x = leaf.x.wrapping_add(2);
            leaf.y = leaf.y.wrapping_sub(1);
            leaf.x_offset = visible_battle_anim_sine(
                leaf.angle.wrapping_add(0x10),
                0x40,
            );
            leaf.angle = leaf.angle.wrapping_add(1);
            true
        });
    }

    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    for leaf in leaves {
        spawn_visible_field_move_oam(
            commands,
            &snapshot,
            rendered_art,
            &runtime_shell.asset_root,
            images,
            time_of_day,
            "cut_grass",
            "SPRITE_ANIM_OAMSET_LEAF",
            false,
            player_origin_x,
            player_origin_y,
            i32::from(leaf.x) + leaf.x_offset - 84,
            i32::from(leaf.y.wrapping_sub(origin_y) as i8),
        )?;
    }

    let x = player_origin_x
        + (i32::from(player.x) + player.x_offset - 84) as f32 * scale;
    let y = player_origin_y
        - i32::from(player.y.wrapping_sub(origin_y) as i8) as f32 * scale;
    commands.spawn((
        SpriteBundle {
            texture: icon.handle,
            sprite: Sprite { custom_size: Some(icon.size), ..default() },
            transform: Transform::from_xyz(x, y, 3.2),
            ..default()
        },
        ObjectMarker,
    ));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_visible_whirlpool_animation(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
    start_x: i16,
    start_y: i16,
    time_of_day: &str,
) -> Result<()> {
    let Some(animation) = runtime_shell
        .visible_whirlpool_animation
        .filter(|_| runtime_shell.field_notice.is_none())
    else {
        return Ok(());
    };
    let Some((origin_x, origin_y)) =
        runtime_tile_playfield_position(animation.target_tile, start_x, start_y)
    else {
        return Ok(());
    };
    // TypeScript's WhirlpoolTileAnimation intentionally composes the fixed
    // Johto metatile $07, independent of the replacement block's tileset.
    let johto_tileset = snapshot
        .tilesets
        .iter()
        .find(|candidate| candidate.tileset_id.eq_ignore_ascii_case("johto"))
        .context("WHIRLPOOL requires the exported Johto tileset")?;
    let key = TilesetArtKey {
        tileset_id: johto_tileset.tileset_id.clone(),
        time_of_day: normalize_tileset_time_of_day(time_of_day),
    };
    let runtime_tileset = RuntimeTilesetKey {
        tileset_id: johto_tileset.tileset_id.clone(),
        collision: johto_tileset.collision.clone(),
        palette_map: johto_tileset.palette_map.clone(),
    };
    if !rendered_art.cache.contains_key(&key) {
        let art = load_tileset_art(
            &runtime_shell.asset_root,
            &key.tileset_id,
            &key.time_of_day,
            &johto_tileset.palette_map,
            images,
        )?;
        rendered_art.cache.insert(key.clone(), art);
    }
    let art = rendered_art
        .cache
        .get(&key)
        .with_context(|| format!("WHIRLPOOL requires loaded tileset art {}", key.tileset_id))?;
    let layout_start = 0x07_usize * METATILE_TILE_COUNT;
    let layout = art
        .metatile_layout
        .get(layout_start..layout_start + METATILE_TILE_COUNT)
        .context("WHIRLPOOL requires Johto metatile $07")?
        .to_vec();
    let base_handles = art.tile_handles.clone();
    let phase = animation.frame & 3;
    for (index, tile_id) in layout.into_iter().enumerate() {
        let handle = match tile_id {
            0x32 | 0x33 | 0x42 | 0x43 => visible_whirlpool_tile_frame(
                rendered_art,
                &runtime_shell.asset_root,
                images,
                &runtime_tileset,
                time_of_day,
                tile_id,
                phase,
            )?
            .handle,
            _ => base_handles
                .get(usize::from(tile_id))
                .cloned()
                .with_context(|| format!("WHIRLPOOL metatile references missing tile ${tile_id:02x}"))?,
        };
        let sub_x = (index % usize::from(RENDER_METATILE_WIDTH as u16)) as f32;
        let sub_y = (index / usize::from(RENDER_METATILE_WIDTH as u16)) as f32;
        commands.spawn((
            SpriteBundle {
                texture: handle,
                sprite: Sprite { custom_size: Some(Vec2::splat(TILE_SIZE)), ..default() },
                transform: Transform::from_xyz(
                    origin_x + sub_x * TILE_SIZE,
                    origin_y - sub_y * TILE_SIZE,
                    2.95,
                ),
                ..default()
            },
            ObjectMarker,
        ));
    }
    Ok(())
}

fn visible_whirlpool_tile_frame(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    tileset: &RuntimeTilesetKey,
    time_of_day: &str,
    tile_id: u8,
    phase: u8,
) -> Result<SpriteFrame> {
    let source_number = match tile_id {
        0x32 => 1_u8,
        0x33 => 2,
        0x42 => 3,
        0x43 => 4,
        _ => anyhow::bail!("unsupported WHIRLPOOL animated tile ${tile_id:02x}"),
    };
    let time = normalize_tileset_time_of_day(time_of_day);
    let cache_graphic = format!(
        "whirlpool:{}:{tile_id:02x}:{phase}",
        tileset.tileset_id
    );
    let cache_key = (cache_graphic, time.clone(), 0);
    if let Some(frame) = rendered_art.field_move_tile_cache.get(&cache_key) {
        return Ok(frame.clone());
    }
    let path = asset_root
        .runtime_assets()
        .join("gfx/tilesets/whirlpool")
        .join(format!("{source_number}.2bpp"));
    let data = std::fs::read(&path)
        .with_context(|| format!("read WHIRLPOOL animation tile {}", path.display()))?;
    let offset = usize::from(phase) * 16;
    let tile = data
        .get(offset..offset + 16)
        .with_context(|| format!("WHIRLPOOL source {} lacks phase {phase}", path.display()))?;
    let palette_value = tileset.palette_map.get(usize::from(tile_id)).copied().unwrap_or(0);
    let palette_index = usize::from(palette_value & 7);
    let palette_bank = load_tileset_palette_bank(asset_root, &tileset.tileset_id, &time)?
        .context("WHIRLPOOL requires a tileset palette bank")?;
    let palette = palette_bank
        .get(palette_index)
        .or_else(|| palette_bank.first())
        .with_context(|| format!("WHIRLPOOL palette {palette_index} is missing"))?;
    let mut pixels = vec![0_u8; 8 * 8 * 4];
    for row in 0..8_usize {
        for column in 0..8_usize {
            let bit = 1 << (7 - column);
            let colour = (((tile[row * 2 + 1] & bit != 0) as usize) << 1)
                | (tile[row * 2] & bit != 0) as usize;
            let target = (row * 8 + column) * 4;
            pixels[target..target + 3].copy_from_slice(&palette[colour]);
            pixels[target + 3] = 255;
        }
    }
    let mut image = Image::new(
        Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let frame = SpriteFrame { handle: images.add(image), size: Vec2::splat(TILE_SIZE) };
    rendered_art.field_move_tile_cache.insert(cache_key, frame.clone());
    Ok(frame)
}

#[allow(clippy::too_many_arguments)]
fn spawn_visible_headbutt_animation(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &RuntimeMapCatalogSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
    start_x: i16,
    start_y: i16,
    time_of_day: &str,
) -> Result<()> {
    let Some(animation) = runtime_shell
        .visible_headbutt_animation
        .as_ref()
        .filter(|_| runtime_shell.field_notice.is_none())
    else { return Ok(()); };
    let Some((target_x, target_y)) = runtime_tile_playfield_position(animation.target_tile, start_x, start_y) else { return Ok(()); };
    let source_origin_x = target_x - TILE_SIZE * 0.5;
    let source_origin_y = target_y + TILE_SIZE * 0.5;
    let (spawn_x, spawn_y) = match animation.facing {
        Direction::Right => (12_i32, 11_i32),
        Direction::Left => (8, 11),
        Direction::Down => (10, 13),
        Direction::Up => (10, 9),
    };

    let tileset_key = TilesetArtKey {
        tileset_id: map.attributes.tileset_name.clone(),
        time_of_day: time_of_day.to_string(),
    };
    let grass = rendered_art.cache.get(&tileset_key)
        .and_then(|art| art.tile_handles.get(5)).cloned()
        .context("HEADBUTT requires source background tile $05")?;
    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    for (piece_x, piece_y) in [(-8_i32, -8_i32), (0, -8), (-8, 0), (0, 0)] {
        let x = source_origin_x + (spawn_x + piece_x + 4) as f32 * scale;
        let y = source_origin_y - (spawn_y + piece_y + 4) as f32 * scale;
        commands.spawn((SpriteBundle {
            texture: grass.clone(),
            sprite: Sprite { custom_size: Some(Vec2::splat(TILE_SIZE)), ..default() },
            transform: Transform::from_xyz(x, y, 3.05),
            ..default()
        }, ObjectMarker));
    }

    let mut frame_index = 0_u8;
    let mut duration = 2_u8;
    for _ in 0..animation.frame.saturating_sub(1) {
        if duration > 0 {
            duration -= 1;
        } else {
            frame_index = (frame_index + 1) % 4;
            duration = 2;
        }
    }
    let (oam_name, xflip) = match frame_index {
        0 | 2 => ("SPRITE_ANIM_OAMSET_TREE_1", false),
        1 => ("SPRITE_ANIM_OAMSET_HEADBUTT_TREE_2", false),
        3 => ("SPRITE_ANIM_OAMSET_HEADBUTT_TREE_2", true),
        _ => unreachable!("HEADBUTT frameset index is modulo four"),
    };
    spawn_visible_field_move_oam(
        commands, snapshot, rendered_art, &runtime_shell.asset_root, images, time_of_day,
        "headbutt_tree", oam_name, xflip, source_origin_x, source_origin_y, spawn_x, spawn_y,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_visible_cut_animation(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
    start_x: i16,
    start_y: i16,
    time_of_day: &str,
) -> Result<()> {
    let Some(animation) = runtime_shell
        .visible_cut_animation
        .as_ref()
        .filter(|_| runtime_shell.field_notice.is_none())
    else { return Ok(()); };
    let Some((target_x, target_y)) = runtime_tile_playfield_position(animation.target_tile, start_x, start_y) else { return Ok(()); };
    let source_origin_x = target_x - TILE_SIZE * 0.5;
    let source_origin_y = target_y + TILE_SIZE * 0.5;
    let age = animation.frame.saturating_sub(1);
    if animation.variant.eq_ignore_ascii_case("tree") {
        let oam_name = match age {
            0..=1 => Some("SPRITE_ANIM_OAMSET_TREE_1"),
            2..=18 => Some("SPRITE_ANIM_OAMSET_CUT_TREE_2"),
            19..=20 => Some("SPRITE_ANIM_OAMSET_CUT_TREE_3"),
            21 => Some("SPRITE_ANIM_OAMSET_CUT_TREE_4"),
            _ => None,
        };
        let Some(oam_name) = oam_name else { return Ok(()); };
        let (spawn_x, spawn_y) = match animation.facing {
            Direction::Right => (12_i32, 11_i32),
            Direction::Left => (8, 11),
            Direction::Down => (10, 13),
            Direction::Up => (10, 9),
        };
        return spawn_visible_field_move_oam(commands, snapshot, rendered_art, &runtime_shell.asset_root, images, time_of_day, "cut_tree", oam_name, false, source_origin_x, source_origin_y, spawn_x, spawn_y);
    }
    if !animation.variant.eq_ignore_ascii_case("grass") {
        anyhow::bail!("unknown exported CUT animation variant {}", animation.variant);
    }
    let metatile_x = animation.target_tile.x / METATILE_WIDTH;
    let metatile_y = animation.target_tile.y / METATILE_WIDTH;
    let (player_x, player_y, direction_index) = match animation.facing {
        Direction::Down => (metatile_x, metatile_y - 1, 0_usize),
        Direction::Up => (metatile_x, metatile_y + 1, 4),
        Direction::Left => (metatile_x + 1, metatile_y, 8),
        Direction::Right => (metatile_x - 1, metatile_y, 12),
    };
    const LEAF_COORDS: [(i32, i32); 16] = [
        (11, 12), (9, 12), (11, 14), (9, 14), (11, 8), (9, 8), (11, 10), (9, 10),
        (7, 12), (9, 12), (7, 10), (9, 10), (11, 12), (13, 12), (11, 10), (13, 10),
    ];
    let parity = usize::from((player_x & 1) != 0) + usize::from((player_y & 1) != 0) * 2;
    let (base_x, base_y) = LEAF_COORDS[direction_index + parity];
    let amplitude = ((u16::from(age) + 1) / 2) as u8;
    for initial_angle in [0_u8, 0x10, 0x20, 0x30] {
        let angle = initial_angle.wrapping_add(age.wrapping_mul(3));
        spawn_visible_field_move_oam(
            commands, snapshot, rendered_art, &runtime_shell.asset_root, images, time_of_day,
            "cut_grass", "SPRITE_ANIM_OAMSET_LEAF", false, source_origin_x, source_origin_y,
            base_x + visible_battle_anim_sine(angle.wrapping_add(0x10), amplitude),
            base_y + visible_battle_anim_sine(angle, amplitude),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_visible_field_move_oam(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    time_of_day: &str,
    graphic: &str,
    oam_name: &str,
    frame_xflip: bool,
    source_origin_x: f32,
    source_origin_y: f32,
    object_x: i32,
    object_y: i32,
) -> Result<()> {
    if rendered_art.intro_sprite_bundle_cache.is_none() {
        rendered_art.intro_sprite_bundle_cache = Some(serde_json::from_str(&snapshot.presentation.sprite_anim_bundle).context("parse packed sprite animation bundle for CUT")?);
    }
    let oam_set = rendered_art.intro_sprite_bundle_cache.as_ref()
        .and_then(|bundle| bundle.get("oam_sets")).and_then(|sets| sets.get(oam_name))
        .with_context(|| format!("field-move OAM set {oam_name} is missing from the packed bundle"))?;
    let tile_offset = oam_set.get("tile_offset").and_then(serde_json::Value::as_u64)
        .and_then(|offset| u8::try_from(offset).ok())
        .with_context(|| format!("field-move OAM set {oam_name} has an invalid tile offset"))?;
    let pieces = oam_set.get("pieces").and_then(serde_json::Value::as_array)
        .with_context(|| format!("field-move OAM set {oam_name} has no pieces"))?.clone();
    for piece in pieces {
        let piece_x = piece.get("x").and_then(serde_json::Value::as_i64).with_context(|| format!("CUT OAM set {oam_name} has a piece without X"))? as i32;
        let piece_y = piece.get("y").and_then(serde_json::Value::as_i64).with_context(|| format!("CUT OAM set {oam_name} has a piece without Y"))? as i32;
        let tile = piece.get("tile").and_then(serde_json::Value::as_u64).and_then(|tile| u8::try_from(tile).ok()).with_context(|| format!("CUT OAM set {oam_name} has an invalid tile"))?.checked_add(tile_offset).context("field-move OAM tile overflow")?;
        let attributes = piece.get("attributes").and_then(serde_json::Value::as_u64).with_context(|| format!("CUT OAM set {oam_name} has a piece without attributes"))?;
        if attributes & 0x7 != 6 {
            anyhow::bail!("CUT OAM set {oam_name} requires unexpected palette {}", attributes & 0x7);
        }
        let frame = visible_field_move_tile_frame(rendered_art, asset_root, images, time_of_day, graphic, tile)?;
        let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
        let piece_x = if frame_xflip { -8 - piece_x } else { piece_x };
        let x = source_origin_x + (object_x + piece_x + 4) as f32 * scale;
        let y = source_origin_y - (object_y + piece_y + 4) as f32 * scale;
        commands.spawn((SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite { custom_size: Some(frame.size), ..default() },
            transform: Transform::from_xyz(x, y, 3.1),
            ..default()
        }, ObjectMarker));
    }
    Ok(())
}

fn visible_field_move_tile_frame(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    time_of_day: &str,
    graphic: &str,
    tile_index: u8,
) -> Result<SpriteFrame> {
    let time = normalize_tileset_time_of_day(time_of_day);
    let key = (graphic.to_string(), time.clone(), tile_index);
    if let Some(frame) = rendered_art.field_move_tile_cache.get(&key) { return Ok(frame.clone()); }
    let path = asset_root.runtime_assets().join("gfx/overworld").join(format!("{graphic}.2bpp"));
    let data = std::fs::read(&path).with_context(|| format!("read field-move graphics {}", path.display()))?;
    let offset = usize::from(tile_index).checked_mul(16).context("field-move tile offset overflow")?;
    let tile = data.get(offset..offset + 16).with_context(|| format!("field-move tile {tile_index} is missing from {}", path.display()))?;
    let palettes = load_npc_sprite_palette_bank(asset_root, &time)?;
    let palette = palettes.get(6).context("field-move object requires PAL_OW_TREE palette 6")?;
    let mut pixels = vec![0_u8; 8 * 8 * 4];
    for row in 0..8_usize {
        for column in 0..8_usize {
            let bit = 1 << (7 - column);
            let colour = (((tile[row * 2 + 1] & bit != 0) as usize) << 1) | (tile[row * 2] & bit != 0) as usize;
            if colour == 0 { continue; }
            let target = (row * 8 + column) * 4;
            pixels[target..target + 3].copy_from_slice(&palette[colour]);
            pixels[target + 3] = 255;
        }
    }
    let mut image = Image::new(
        Extent3d { width: 8, height: 8, depth_or_array_layers: 1 }, TextureDimension::D2,
        pixels, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let frame = SpriteFrame { handle: images.add(image), size: Vec2::splat(TILE_SIZE) };
    rendered_art.field_move_tile_cache.insert(key, frame.clone());
    Ok(frame)
}
