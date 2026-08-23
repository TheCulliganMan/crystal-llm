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
const BOOT_UI_WHITE: [u8; 4] = [255, 255, 255, 255];

const PACK_SCREEN_WIDTH_TILES: usize = 20;
const PACK_SCREEN_HEIGHT_TILES: usize = 18;
const PACK_BACKGROUND_TILE: u8 = 0x24;
const PACK_TOP_BAR_FIRST_TILE: u8 = 0x28;
const PACK_ICON_FIRST_TILE: u8 = 0x50;
const PACK_ICON_TILES_PER_POCKET: usize = 15;

fn load_visible_field_pack_frame(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
    items: &[(String, u16)],
    selected: usize,
    list_start: usize,
    description: &str,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = runtime_shell.asset_root.runtime_assets();
    let root = assets.join("gfx/pack");
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode Pack font PNG")?
        .to_rgba8();
    let frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
        .context("decode Pack textbox frame PNG")?
        .to_rgba8();
    let menu_tiles = crate::read_runtime_asset(root.join("pack_menu.2bpp"))
        .context("read canonical Pack menu tiles")?;
    let up_arrow = crate::read_runtime_asset(assets.join("gfx/font/up_arrow.2bpp"))
        .context("read canonical Pack scroll-up glyph")?;
    let female = snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE;
    let icon_tiles = crate::read_runtime_asset(root.join(if female {
        "pack_f.2bpp"
    } else {
        "pack.2bpp"
    }))
    .context("read canonical Pack icon tiles")?;
    let palette_source = crate::read_runtime_asset_to_string(root.join(if female {
        "pack_f.pal"
    } else {
        "pack.pal"
    }))
    .context("read canonical Pack palettes")?;
    let mut palettes = parse_palette_file(&palette_source, None)?;
    anyhow::ensure!(palettes.len() >= 6, "Pack palette must contain six palettes");
    while palettes.len() < 8 {
        palettes.push(*palettes.last().expect("six palettes checked"));
    }
    anyhow::ensure!(menu_tiles.len() == 80 * 16, "Pack menu art must contain 80 tiles");
    anyhow::ensure!(icon_tiles.len() == 60 * 16, "Pack icon art must contain four 15-tile pockets");
    let label_map = crate::read_runtime_asset(root.join("pack_menu.tilemap"))
        .context("read canonical Pack pocket-label tilemap")?;
    anyhow::ensure!(label_map.len() == 60, "Pack pocket-label tilemap must contain 60 bytes");

    let pocket_index = match pocket {
        FieldPackPocket::TmHm => 0,
        FieldPackPocket::Items => 1,
        FieldPackPocket::KeyItems => 2,
        FieldPackPocket::Balls => 3,
        FieldPackPocket::Custom(pocket_id) => {
            anyhow::bail!("custom Pack pocket {pocket_id} has no canonical ASM pocket art")
        }
    };
    let icon_chunk = [1_usize, 3, 0, 2][pocket_index];
    let mut tilemap = vec![NAME_ENTRY_SPACE_TILE; PACK_SCREEN_WIDTH_TILES * PACK_SCREEN_HEIGHT_TILES];
    let mut attrmap = vec![0_u8; tilemap.len()];
    for y in 1..12 {
        for x in 0..PACK_SCREEN_WIDTH_TILES {
            tilemap[y * PACK_SCREEN_WIDTH_TILES + x] = PACK_BACKGROUND_TILE;
        }
    }
    for y in 1..12 {
        for x in 5..PACK_SCREEN_WIDTH_TILES {
            tilemap[y * PACK_SCREEN_WIDTH_TILES + x] = NAME_ENTRY_SPACE_TILE;
        }
    }
    for x in 0..PACK_SCREEN_WIDTH_TILES {
        tilemap[x] = PACK_TOP_BAR_FIRST_TILE + x as u8;
        attrmap[x] = if x < 10 { 1 } else { 2 };
    }
    let label_offset = pocket_index * 15;
    for y in 0..3 {
        for x in 0..5 {
            tilemap[(7 + y) * 20 + x] = label_map[label_offset + y * 5 + x];
            attrmap[(7 + y) * 20 + x] = 4;
        }
    }
    for y in 0..3 {
        for x in 0..5 {
            tilemap[(3 + y) * 20 + x] = PACK_ICON_FIRST_TILE + (y * 5 + x) as u8;
            attrmap[(3 + y) * 20 + x] = 5;
        }
    }
    for y in 2..11 {
        attrmap[y * 20 + 7] = 3;
    }

    let mut write = |x: usize, y: usize, text: &str| -> Result<()> {
        for (offset, token) in tokenize_name_entry_string(text).into_iter().enumerate() {
            if x + offset >= 20 || y >= 18 {
                break;
            }
            tilemap[y * 20 + x + offset] = name_entry_token_tile(&token)
                .with_context(|| format!("unsupported Pack glyph {token:?}"))?;
        }
        Ok(())
    };
    for visible_index in 0..7 {
        let index = list_start + visible_index;
        if index > items.len() {
            break;
        }
        let row = 2 + visible_index;
        write(7, row, if index == selected { "▶" } else { " " })?;
        if let Some((item_id, quantity)) = items.get(index) {
            let name = compact_scene_label(&item_display_name(snapshot, item_id).to_uppercase(), 8);
            write(8, row, &name)?;
            if !matches!(pocket, FieldPackPocket::KeyItems) {
                write(16, row, &format!("×{:02}", (*quantity).min(99)))?;
            }
        } else {
            write(8, row, "CANCEL")?;
        }
    }
    if list_start > 0 {
        write(19, 2, "▲")?;
    }
    if list_start + 7 < items.len() + 1 {
        write(19, 8, "▼")?;
    }

    let width = 20 * SOURCE_TILE_SIZE;
    let height = 18 * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for (index, tile_id) in tilemap.iter().copied().enumerate() {
        let x = index % 20;
        let y = index / 20;
        let palette = &palettes[usize::from(attrmap[index].min(7))];
        if tile_id == NAME_ENTRY_SPACE_TILE {
            fill_native_tile(&mut data, x, y, palette[0]);
        } else if tile_id >= 0x80 {
            draw_paletted_png_tile(&font, usize::from(tile_id - 0x80), palette, x, y, &mut data)?;
        } else if tile_id == 0x61 {
            draw_paletted_2bpp_tile(&up_arrow, 0, palette, x, y, &mut data)?;
        } else if (PACK_ICON_FIRST_TILE..PACK_ICON_FIRST_TILE + 15).contains(&tile_id) {
            let icon_index = icon_chunk * PACK_ICON_TILES_PER_POCKET
                + usize::from(tile_id - PACK_ICON_FIRST_TILE);
            draw_paletted_2bpp_tile(&icon_tiles, icon_index, palette, x, y, &mut data)?;
        } else {
            draw_paletted_2bpp_tile(&menu_tiles, usize::from(tile_id), palette, x, y, &mut data)?;
        }
    }
    draw_time_set_window(&frame, 0, 12, 20, 6, &mut data)?;
    for (line_index, line) in wrap_boot_text_for_box(description, 18, 3).iter().enumerate() {
        draw_time_set_text(&font, line, 8, (14 + line_index) * 8, &mut data)?;
    }
    if let Some(cursor) = runtime_shell.field_pack_action_cursor.as_ref() {
        let actions = visible_selected_pack_item_actions(snapshot, runtime_shell, pocket, false)?;
        let choice = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pack:actions",
            actions.len(),
        )
        .context("Pack action cursor is invalid")?;
        let top = match actions.len() { 5 => 1, 4 => 3, 3 => 5, 2 => 7, _ => 9 };
        draw_time_set_window(&frame, 13, top, 7, actions.len() + 2, &mut data)?;
        for (index, action) in actions.iter().enumerate() {
            draw_time_set_text(
                &font,
                &format!(
                    "{}{}",
                    if index == choice { "▶" } else { " " },
                    visible_field_pack_action_label(*action)
                ),
                14 * 8,
                (top + 1 + index) * 8,
                &mut data,
            )?;
        }
    }
    let mut image = Image::new(
        Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::new(width as f32, height as f32) })
}

fn fill_native_tile(target: &mut [u8], tile_x: usize, tile_y: usize, rgb: [u8; 3]) {
    for y in tile_y * 8..tile_y * 8 + 8 {
        for x in tile_x * 8..tile_x * 8 + 8 {
            let offset = (y * 160 + x) * 4;
            target[offset..offset + 3].copy_from_slice(&rgb);
            target[offset + 3] = 255;
        }
    }
}

fn draw_paletted_2bpp_tile(
    source: &[u8], tile_index: usize, palette: &Palette, tile_x: usize, tile_y: usize, target: &mut [u8],
) -> Result<()> {
    let start = tile_index * 16;
    let tile = source.get(start..start + 16).with_context(|| format!("2bpp tile {tile_index} is missing"))?;
    for row in 0..8 {
        let lo = tile[row * 2];
        let hi = tile[row * 2 + 1];
        for col in 0..8 {
            let bit = 7 - col;
            let level = usize::from((lo >> bit) & 1 | (((hi >> bit) & 1) << 1));
            let offset = (((tile_y * 8 + row) * 160) + tile_x * 8 + col) * 4;
            target[offset..offset + 3].copy_from_slice(&palette[level]);
            target[offset + 3] = 255;
        }
    }
    Ok(())
}

fn draw_paletted_png_tile(
    source: &image::RgbaImage, tile_index: usize, palette: &Palette, tile_x: usize, tile_y: usize, target: &mut [u8],
) -> Result<()> {
    let columns = source.width() as usize / 8;
    anyhow::ensure!(columns > 0, "font tile sheet has no columns");
    let source_x = tile_index % columns * 8;
    let source_y = tile_index / columns * 8;
    anyhow::ensure!(source_y + 8 <= source.height() as usize, "font tile {tile_index} is missing");
    for row in 0..8 {
        for col in 0..8 {
            let gray = source.get_pixel((source_x + col) as u32, (source_y + row) as u32)[0];
            let level = palette_index_from_gray(gray);
            let offset = (((tile_y * 8 + row) * 160) + tile_x * 8 + col) * 4;
            target[offset..offset + 3].copy_from_slice(&palette[level]);
            target[offset + 3] = 255;
        }
    }
    Ok(())
}

fn load_visible_field_party_frame(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    selected: usize,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = runtime_shell.asset_root.runtime_assets();
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode party-menu font PNG")?
        .to_rgba8();
    let window = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
        .context("decode party-menu window frame PNG")?
        .to_rgba8();
    let battle_extra = crate::read_runtime_asset(assets.join("gfx/font/font_battle_extra.2bpp"))
        .context("read party-menu level glyphs")?;
    let text_palette: Palette = [
        [255, 255, 255],
        [170, 170, 170],
        [85, 85, 85],
        [0, 0, 0],
    ];
    let width = 160;
    let height = 144;
    let mut data = vec![255_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    for (row_index, slot) in snapshot.party.slots.iter().take(6).enumerate() {
        let name_row = 1 + row_index * 2;
        draw_time_set_text(
            &font,
            if selected == row_index { "▶" } else { " " },
            0,
            name_row * 8,
            &mut data,
        )?;
        if runtime_shell.party_switch_cursor.is_some() && runtime_shell.party_cursor == row_index {
            draw_time_set_text(&font, "▷", 16, name_row * 8, &mut data)?;
        }
        if !slot.pokemon.is_egg {
            draw_time_set_text(
                &font,
                &compact_scene_label(&slot.pokemon.nickname, 10),
                3 * 8,
                name_row * 8,
                &mut data,
            )?;
            draw_time_set_text(
                &font,
                &format!("{:>3}/{:>3}", slot.pokemon.hp.min(999), slot.pokemon.max_hp.min(999)),
                13 * 8,
                name_row * 8,
                &mut data,
            )?;
            let status_row = name_row + 1;
            draw_time_set_text(&font, party_status_token(&slot.pokemon), 5 * 8, status_row * 8, &mut data)?;
            draw_paletted_2bpp_tile(
                &battle_extra,
                usize::from(0x6e_u8 - 0x60),
                &text_palette,
                8,
                status_row,
                &mut data,
            )?;
            draw_time_set_text(
                &font,
                &format!("{:>2}", slot.pokemon.level.min(100)),
                9 * 8,
                status_row * 8,
                &mut data,
            )?;
            draw_native_hp_bar(
                &mut data,
                11 * 8,
                status_row * 8 + 2,
                slot.pokemon.hp,
                slot.pokemon.max_hp,
            );
        }
    }
    let cancel_row = 1 + snapshot.party.slots.len().min(6) * 2;
    draw_time_set_text(
        &font,
        if selected >= snapshot.party.slots.len() { "▶CANCEL" } else { " CANCEL" },
        8,
        cancel_row * 8,
        &mut data,
    )?;
    draw_time_set_window(&window, 0, 14, 20, 4, &mut data)?;
    let prompt = if runtime_shell.party_hp_transfer_source.is_some() {
        "Use on which"
    } else if runtime_shell.party_switch_cursor.is_some() {
        "Move to where?"
    } else {
        "Choose a POKéMON."
    };
    draw_time_set_text(&font, prompt, 8, 16 * 8, &mut data)?;
    if runtime_shell.party_hp_transfer_source.is_some() {
        draw_time_set_text(&font, "POKéMON?", 8, 17 * 8, &mut data)?;
    }

    if let Some(cursor) = runtime_shell.party_action_cursor.as_ref() {
        let actions = visible_party_actions(snapshot, runtime_shell)?;
        let action_selected = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "party:actions",
            actions.len(),
        )
        .context("party action cursor is invalid")?;
        draw_time_set_window(&window, 6, 0, 14, 18, &mut data)?;
        for (index, action) in actions.iter().enumerate() {
            let label = party_submenu_action_entry(
                *action,
                if index == action_selected { "▶" } else { " " },
            );
            draw_time_set_text(&font, &label, 7 * 8, (1 + index * 2) * 8, &mut data)?;
        }
    }
    if let Some(cursor) = runtime_shell.party_give_take_cursor.as_ref() {
        let mail_actions = cursor.surface_id == "party:mail-actions";
        let (surface, labels): (&str, &[&str]) = if mail_actions {
            ("party:mail-actions", &["READ", "TAKE", "QUIT"])
        } else {
            ("party:give-take", &["GIVE", "TAKE"])
        };
        let choice = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            surface,
            labels.len(),
        )
        .with_context(|| format!("{surface} cursor is invalid"))?;
        let top = 18_usize.saturating_sub(labels.len() + 2);
        draw_time_set_window(&window, 12, top, 8, labels.len() + 2, &mut data)?;
        for (index, label) in labels.iter().enumerate() {
            draw_time_set_text(
                &font,
                &format!("{}{}", if index == choice { "▶" } else { " " }, label),
                13 * 8,
                (top + 1 + index) * 8,
                &mut data,
            )?;
        }
    }

    let mut image = Image::new(
        Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::new(width as f32, height as f32) })
}

fn draw_native_hp_bar(target: &mut [u8], x: usize, y: usize, hp: u16, max_hp: u16) {
    let pixels = if max_hp == 0 || hp == 0 {
        0
    } else {
        ((usize::from(hp.min(max_hp)) * 48) / usize::from(max_hp)).max(1)
    };
    let color = if pixels >= 24 {
        [0, 189, 0]
    } else if pixels >= 10 {
        [255, 189, 0]
    } else {
        [255, 0, 0]
    };
    for row in 0..4 {
        for col in 0..48 {
            let offset = ((y + row) * 160 + x + col) * 4;
            target[offset..offset + 3].copy_from_slice(if col < pixels { &color } else { &[180, 180, 180] });
            target[offset + 3] = 255;
        }
    }
}

fn load_visible_party_summary_frame(
    runtime_shell: &BevyRuntimeShell,
    rows: &[(usize, f32, String)],
    tint: [u8; 4],
    page: u8,
    hp: Option<(u16, u16)>,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let font = crate::open_runtime_image(
        runtime_shell.asset_root.runtime_assets().join("gfx/font/font.png"),
    )
    .context("decode status-screen font PNG")?
    .to_rgba8();
    let width = 160;
    let height = 144;
    let mut data = vec![0_u8; width * height * 4];
    for y in 0..height {
        let color = if y < 8 * 8 { BOOT_UI_WHITE } else { tint };
        for x in 0..width {
            let offset = (y * width + x) * 4;
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
    // stats_screen.asm divides the fixed identity header from the active page.
    for x in 0..width {
        let offset = (7 * 8 * width + x) * 4;
        data[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
    }
    for (index, color) in [
        [255, 158, 255, 255],
        [173, 255, 115, 255],
        [140, 255, 255, 255],
    ]
    .into_iter()
    .enumerate()
    {
        let left = (13 + index * 2) * 8;
        for y in 5 * 8..7 * 8 {
            for x in left..left + 2 * 8 {
                let offset = (y * width + x) * 4;
                data[offset..offset + 4].copy_from_slice(&color);
            }
        }
        if usize::from(page.saturating_sub(1)) == index {
            for y in 5 * 8..7 * 8 {
                for x in left..left + 2 * 8 {
                    if y == 5 * 8 || y == 7 * 8 - 1 || x == left || x == left + 2 * 8 - 1 {
                        let offset = (y * width + x) * 4;
                        data[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
                    }
                }
            }
        }
    }
    draw_native_page_arrow(&mut data, 12 * 8, 6 * 8, false);
    draw_native_page_arrow(&mut data, 19 * 8, 6 * 8, true);
    if page == 1
        && let Some((current_hp, max_hp)) = hp
    {
        draw_native_hp_bar(&mut data, 2 * 8, 9 * 8 + 2, current_hp, max_hp);
    }
    for (column, row, text) in rows {
        draw_time_set_text(
            &font,
            &compact_scene_label(text, 18),
            column * 8,
            (*row as usize) * 8,
            &mut data,
        )?;
    }
    let mut image = Image::new(
        Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::new(width as f32, height as f32) })
}

fn draw_native_page_arrow(target: &mut [u8], x: usize, y: usize, right: bool) {
    for row in 0..7 {
        let half_width = if row <= 3 { row } else { 6 - row };
        for offset in 0..=half_width {
            let column = if right { offset } else { 6 - offset };
            let pixel = ((y + row) * 160 + x + column) * 4;
            target[pixel..pixel + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
    }
}

fn load_visible_title_main_menu_frame(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = runtime_shell.asset_root.runtime_assets();
    if rendered_art.title_menu_font_source.is_none() {
        rendered_art.title_menu_font_source = Some(
            crate::open_runtime_image(assets.join("gfx/font/font.png"))
                .context("decode title main-menu font PNG")?
                .to_rgba8(),
        );
    }
    if rendered_art.title_menu_frame_source.is_none() {
        rendered_art.title_menu_frame_source = Some(
            crate::open_runtime_image(assets.join("gfx/frames/1.png"))
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

fn load_visible_continue_screen_frame(
    runtime_shell: &BevyRuntimeShell,
    continue_screen: &VisibleContinueScreen,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = runtime_shell.asset_root.runtime_assets();
    if rendered_art.title_menu_font_source.is_none() {
        rendered_art.title_menu_font_source = Some(
            crate::open_runtime_image(assets.join("gfx/font/font.png"))
                .context("decode Continue-screen font PNG")?
                .to_rgba8(),
        );
    }
    if rendered_art.title_menu_frame_source.is_none() {
        rendered_art.title_menu_frame_source = Some(
            crate::open_runtime_image(assets.join("gfx/frames/1.png"))
                .context("decode Continue-screen frame PNG")?
                .to_rgba8(),
        );
    }
    let font = rendered_art.title_menu_font_source.as_ref().unwrap();
    let frame = rendered_art.title_menu_frame_source.as_ref().unwrap();
    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![255_u8; width * height * 4];
    draw_time_set_window(frame, 0, 0, 16, 10, &mut data)?;
    for (label, x, y) in [("PLAYER", 1, 2), ("BADGES", 1, 4), ("TIME", 1, 8)] {
        draw_time_set_text(
            font,
            label,
            x * SOURCE_TILE_SIZE,
            y * SOURCE_TILE_SIZE,
            &mut data,
        )?;
    }
    if continue_screen.pokedex_count.is_some() {
        draw_time_set_text(
            font,
            "#DEX",
            SOURCE_TILE_SIZE,
            6 * SOURCE_TILE_SIZE,
            &mut data,
        )?;
    }
    draw_time_set_text(
        font,
        &continue_screen.player_name,
        8 * SOURCE_TILE_SIZE,
        2 * SOURCE_TILE_SIZE,
        &mut data,
    )?;
    draw_time_set_text(
        font,
        &format!("{:>2}", continue_screen.badge_count),
        13 * SOURCE_TILE_SIZE,
        4 * SOURCE_TILE_SIZE,
        &mut data,
    )?;
    if let Some(caught) = continue_screen.pokedex_count {
        draw_time_set_text(
            font,
            &format!("{:>3}", caught.min(999)),
            12 * SOURCE_TILE_SIZE,
            6 * SOURCE_TILE_SIZE,
            &mut data,
        )?;
    }
    draw_time_set_text(
        font,
        &format!(
            "{:>3}:{:02}",
            continue_screen.hours.min(999),
            continue_screen.minutes.min(59)
        ),
        9 * SOURCE_TILE_SIZE,
        8 * SOURCE_TILE_SIZE,
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
    let hour = time.registers.hours;
    let minute = time.registers.minutes;
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
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    if rendered_art.credits_sources.is_none() && rendered_art.credits_source_error.is_none() {
        match load_visible_credits_sources(&runtime_shell.asset_root) {
            Ok(sources) => rendered_art.credits_sources = Some(sources),
            Err(error) => rendered_art.credits_source_error = Some(format!("{error:#}")),
        }
    }
    if let Some(error) = rendered_art.credits_source_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let sources = rendered_art
        .credits_sources
        .as_ref()
        .context("credits render sources are unavailable")?;
    let frame = render_visible_credits_frame_from_sources(sources, credits, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
    Ok(())
}

fn spawn_visible_delete_save_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    delete_save: &VisibleDeleteSaveScreen,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = load_delete_save_frame(&runtime_shell.asset_root, delete_save, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
    Ok(())
}

fn spawn_visible_mystery_gift_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    mystery_gift: &VisibleMysteryGiftScreen,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = load_mystery_gift_frame(&runtime_shell.asset_root, mystery_gift, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode delete-save font PNG")?
        .to_rgba8();
    let frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode Mystery Gift font PNG")?
        .to_rgba8();
    let frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
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
) -> Result<()> {
    let key = trainer_card_art_key(
        snapshot,
        runtime_shell.trainer_card_page,
        runtime_shell.trainer_card_colon_visible,
        runtime_shell.trainer_card_badge_frame,
    );
    if !rendered_art.trainer_card_cache.contains_key(&key)
        && !rendered_art.trainer_card_errors.contains_key(&key)
    {
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
        retain_bounded_fullscreen_art_key(
            &mut rendered_art.trainer_card_cache,
            &mut rendered_art.trainer_card_errors,
            &mut rendered_art.trainer_card_cache_order,
            key.clone(),
            images,
        );
    }
    let Some(frame) = rendered_art.trainer_card_cache.get(&key).cloned() else {
        let error = rendered_art
            .trainer_card_errors
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "unknown Trainer Card render error".to_string());
        anyhow::bail!("required Trainer Card frame could not be rendered: {error}");
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        4.2,
        images,
    )?;
    Ok(())
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode Trainer Card font PNG")?
        .to_rgba8();
    let full_font = crate::open_runtime_image(assets.join("gfx/font/english.png"))
        .context("decode Trainer Card full font PNG")?
        .to_rgba8();
    let portrait_stem = if snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE {
        "kris_card"
    } else {
        "chris_card"
    };
    let portrait = crate::open_runtime_image(trainer_card_dir.join(format!("{portrait_stem}.png")))
        .with_context(|| format!("decode Trainer Card portrait PNG {portrait_stem}"))?
        .to_rgba8();
    let trainer_tiles = crate::open_runtime_image(trainer_card_dir.join("trainer_card.png"))
        .context("decode Trainer Card tiles PNG")?
        .to_rgba8();
    let right_corner = crate::open_runtime_image(trainer_card_dir.join("card_right_corner.png"))
        .context("decode Trainer Card right-corner PNG")?
        .to_rgba8();
    let status_tiles = crate::open_runtime_image(trainer_card_dir.join("card_status.png"))
        .context("decode Trainer Card status PNG")?
        .to_rgba8();
    let leaders = if page == VisibleTrainerCardPage::JohtoBadges {
        Some(
            crate::open_runtime_image(trainer_card_dir.join("leaders.png"))
                .context("decode Trainer Card leader portraits PNG")?
                .to_rgba8(),
        )
    } else {
        None
    };
    let badges = if page == VisibleTrainerCardPage::JohtoBadges {
        Some(
            crate::open_runtime_image(trainer_card_dir.join("badges.png"))
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
    let background = palettes.get(background_palette).with_context(|| {
        format!("Trainer Card background palette {background_palette} is missing")
    })?[0];
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
            let palette = palettes.get(palette_index).with_context(|| {
                format!("Trainer Card tile ({x}, {y}) requires missing palette {palette_index}")
            })?;
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
        draw_owned_trainer_card_badges(snapshot, badges, asset_root, badge_frame, &mut data)?;
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
    for (index, owned) in snapshot
        .progression
        .badges
        .johto
        .iter()
        .copied()
        .enumerate()
    {
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
            let pixel =
                source.get_pixel((source_x + source_column) as u32, (source_y + row) as u32);
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
    load_named_predef_palette(asset_root, "PREDEFPAL_CGB_BADGE")
}

fn load_named_predef_palette(asset_root: &AssetRoot, palette_name: &str) -> Result<Palette> {
    let path = asset_root.runtime_assets().join("gfx/sgb/predef.pal");
    let text = crate::read_runtime_asset_to_string(&path)
        .with_context(|| format!("read predefined palette {}", path.display()))?;
    let line = text
        .lines()
        .find(|line| {
            line.split_once(';')
                .is_some_and(|(_, comment)| comment.split_whitespace().next() == Some(palette_name))
        })
        .with_context(|| format!("{} has no {palette_name} entry", path.display()))?;
    let rgb = line
        .split_once(';')
        .map(|(rgb, _)| rgb)
        .with_context(|| format!("{palette_name} entry has no comment separator"))?
        .trim()
        .strip_prefix("RGB")
        .with_context(|| format!("{palette_name} entry does not begin with RGB"))?;
    let components = rgb
        .split(',')
        .map(|component| component.trim().parse::<u8>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("parse {palette_name} components"))?;
    if components.len() != 12 || components.iter().any(|component| *component > 31) {
        anyhow::bail!("{palette_name} must contain four RGB5 colors, got {components:?}");
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
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = load_clock_reset_frame(&runtime_shell.asset_root, clock_reset, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode clock-reset font PNG")?
        .to_rgba8();
    let frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
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
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode gender-selection font PNG")?
        .to_rgba8();
    let frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
        .context("decode gender-selection textbox frame PNG")?
        .to_rgba8();
    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&BOOT_UI_WHITE);
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

fn load_gender_selection_background(
    palette_path: &Path,
    tile_path: &Path,
) -> Result<[[u8; 4]; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE]> {
    let text = crate::read_runtime_asset_to_string(palette_path)
        .with_context(|| format!("read {}", palette_path.display()))?;
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
                255,
            ]);
        }
    }
    let palette: [[u8; 4]; 4] = colors.try_into().map_err(|colors: Vec<[u8; 4]>| {
        anyhow::anyhow!(
            "gender palette {} must contain exactly four colors, found {}",
            palette_path.display(),
            colors.len()
        )
    })?;
    let tile = crate::read_runtime_asset(tile_path).with_context(|| format!("read {}", tile_path.display()))?;
    if tile.len() != SOURCE_TILE_SIZE * 2 {
        anyhow::bail!(
            "gender background tile {} must contain exactly {} bytes, found {}",
            tile_path.display(),
            SOURCE_TILE_SIZE * 2,
            tile.len()
        );
    }

    let mut pixels = [[0_u8; 4]; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE];
    for row in 0..SOURCE_TILE_SIZE {
        let lo = tile[row * 2];
        let hi = tile[row * 2 + 1];
        for column in 0..SOURCE_TILE_SIZE {
            let bit = 1 << (7 - column);
            let color_index = (((hi & bit != 0) as usize) << 1) | (lo & bit != 0) as usize;
            pixels[row * SOURCE_TILE_SIZE + column] = palette[color_index];
        }
    }
    Ok(pixels)
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
    if !rendered_art.time_set_cache.contains_key(&key)
        && !rendered_art.time_set_errors.contains_key(&key)
    {
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
        retain_bounded_fullscreen_art_key(
            &mut rendered_art.time_set_cache,
            &mut rendered_art.time_set_errors,
            &mut rendered_art.time_set_cache_order,
            key.clone(),
            images,
        );
    }
    let Some(frame) = rendered_art.time_set_cache.get(&key).cloned() else {
        return Ok(());
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode time-set font PNG")?
        .to_rgba8();
    let frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
        .context("decode time-set textbox frame PNG")?
        .to_rgba8();
    let up_arrow = crate::open_runtime_image(assets.join("gfx/new_game/up_arrow.png"))
        .context("decode time-set up arrow PNG")?
        .to_rgba8();
    let down_arrow = crate::open_runtime_image(assets.join("gfx/new_game/down_arrow.png"))
        .context("decode time-set down arrow PNG")?
        .to_rgba8();

    let width = TIME_SET_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = TIME_SET_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&BOOT_UI_WHITE);
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
    if max_chars_per_line == 0 || max_lines == 0 {
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
            } else if boot_text_tile_len(&word) > max_chars_per_line {
                if !current.is_empty() {
                    let available = max_chars_per_line
                        .saturating_sub(boot_text_tile_len(&current) + 1);
                    if available > 0 {
                        let (prefix, remainder) = split_boot_word_prefix(&word, available);
                        current.push(' ');
                        current.push_str(&prefix);
                        word = remainder;
                    }
                    lines.push(current);
                    current = String::new();
                    if lines.len() >= max_lines {
                        break;
                    }
                }
                while boot_text_tile_len(&word) > max_chars_per_line {
                    let (prefix, remainder) =
                        split_boot_word_prefix(&word, max_chars_per_line);
                    lines.push(prefix);
                    word = remainder;
                    if lines.len() >= max_lines {
                        break;
                    }
                }
                if lines.len() >= max_lines {
                    current.clear();
                    break;
                }
                current = word;
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

fn split_boot_word_prefix(word: &str, max_tiles: usize) -> (String, String) {
    let mut split_at = 0;
    for (index, ch) in word.char_indices() {
        let end = index + ch.len_utf8();
        if boot_text_tile_len(&word[..end]) > max_tiles {
            break;
        }
        split_at = end;
    }
    if split_at == 0 {
        split_at = word.chars().next().map(char::len_utf8).unwrap_or(0);
    }
    (word[..split_at].to_string(), word[split_at..].to_string())
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
            put_time_set_pixel(target, x, y, BOOT_UI_WHITE);
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
    if !rendered_art.oak_intro_cache.contains_key(&key)
        && !rendered_art.oak_intro_errors.contains_key(&key)
    {
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
        retain_bounded_fullscreen_art_key(
            &mut rendered_art.oak_intro_cache,
            &mut rendered_art.oak_intro_errors,
            &mut rendered_art.oak_intro_cache_order,
            key.clone(),
            images,
        );
    }
    let Some(frame) = rendered_art.oak_intro_cache.get(&key).cloned() else {
        let error = rendered_art
            .oak_intro_errors
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "unknown Oak intro frame render error".to_string());
        anyhow::bail!("required Oak intro frame could not be rendered: {error}");
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode Oak intro font PNG")?
        .to_rgba8();
    let textbox_frame = crate::open_runtime_image(assets.join("gfx/frames/1.png"))
        .context("decode Oak intro textbox frame PNG")?
        .to_rgba8();
    let down_arrow = crate::open_runtime_image(assets.join("gfx/new_game/down_arrow.png"))
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
    let sources = load_visible_credits_sources(asset_root)?;
    render_visible_credits_frame_from_sources(&sources, credits, images)
}

fn render_visible_credits_frame_from_sources(
    sources: &CreditsRenderSources,
    credits: &VisibleCreditsScreen,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let palette_set = sources
        .palette_sets
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
    draw_visible_credits_mon_strip(sources, credits, bg_palette, &mut data)?;
    fill_visible_credits_rect(
        &mut data,
        0,
        5 * SOURCE_TILE_SIZE,
        CREDITS_SCREEN_WIDTH,
        12 * SOURCE_TILE_SIZE,
        text_palette[0],
    );
    draw_visible_credits_border_rows(sources, border_palette, &mut data);
    draw_visible_credits_text(sources, credits, text_palette, &mut data)?;
    if credits.show_the_end || credits.awaiting_exit {
        draw_visible_credits_the_end(sources, text_palette, &mut data);
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

fn load_visible_credits_sources(asset_root: &AssetRoot) -> Result<CreditsRenderSources> {
    let mut mon_frames = Vec::with_capacity(4 * CREDITS_FRAMES_PER_SCENE);
    for mon_index in 0..4 {
        for frame_index in 0..CREDITS_FRAMES_PER_SCENE {
            mon_frames.push(load_visible_credits_mon_frame_levels(
                asset_root,
                VisibleCreditsBorderFrame {
                    mon_index,
                    frame_index: frame_index as u8,
                },
            )?);
        }
    }
    Ok(CreditsRenderSources {
        palette_sets: load_credits_palette_sets(asset_root)?,
        mon_frames,
        border_tiles: load_visible_credits_border_tiles(asset_root)?,
        font: load_visible_credits_font_tiles(asset_root)?,
        copyright_tiles: load_visible_credits_copyright_tiles(asset_root)?,
        the_end_levels: load_visible_credits_the_end_levels(asset_root)?,
    })
}

fn load_credits_palette_sets(asset_root: &AssetRoot) -> Result<Vec<[Palette; 3]>> {
    let palette_path = asset_root.runtime_assets().join("gfx/credits/credits.pal");
    let content = crate::read_runtime_asset_to_string(&palette_path)
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
    sources: &CreditsRenderSources,
    credits: &VisibleCreditsScreen,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    let frame_levels = visible_credits_mon_frame_levels(sources, credits)?;
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
    sources: &CreditsRenderSources,
    credits: &VisibleCreditsScreen,
) -> Result<Vec<u8>> {
    let mut levels = vec![2_u8; CREDITS_MON_FRAME_SIZE * CREDITS_MON_FRAME_SIZE];
    if let Some(frame) = credits.border_frame_top {
        let frame_levels = cached_visible_credits_mon_frame_levels(sources, frame)?;
        copy_visible_credits_frame_half(&mut levels, &frame_levels, 0);
    }
    if let Some(frame) = credits.border_frame_bottom {
        let frame_levels = cached_visible_credits_mon_frame_levels(sources, frame)?;
        copy_visible_credits_frame_half(&mut levels, &frame_levels, CREDITS_MON_FRAME_SIZE / 2);
    }
    Ok(levels)
}

fn cached_visible_credits_mon_frame_levels(
    sources: &CreditsRenderSources,
    frame: VisibleCreditsBorderFrame,
) -> Result<&[u8]> {
    let mon_index = usize::from(frame.mon_index % 4);
    let frame_index = usize::from(frame.frame_index) % CREDITS_FRAMES_PER_SCENE;
    sources
        .mon_frames
        .get(mon_index * CREDITS_FRAMES_PER_SCENE + frame_index)
        .map(Vec::as_slice)
        .context("cached credits mon frame is unavailable")
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
    let source = crate::open_runtime_image(&path)
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
    sources: &CreditsRenderSources,
    palette: &Palette,
    target: &mut [u8],
) {
    draw_visible_credits_border_row(target, &sources.border_tiles, 4, 4, palette);
    draw_visible_credits_border_row(target, &sources.border_tiles, 17, 0, palette);
}

fn load_visible_credits_border_tiles(asset_root: &AssetRoot) -> Result<Vec<Vec<u8>>> {
    let path = asset_root.runtime_assets().join("gfx/credits/border.png");
    let source = crate::open_runtime_image(&path)
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
    (0..9)
        .map(|index| {
            extract_visible_credits_levels(
                &source,
                (index % 3) * SOURCE_TILE_SIZE,
                (index / 3) * SOURCE_TILE_SIZE,
                SOURCE_TILE_SIZE,
                SOURCE_TILE_SIZE,
            )
        })
        .collect::<Result<Vec<_>>>()
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
    sources: &CreditsRenderSources,
    credits: &VisibleCreditsScreen,
    palette: &Palette,
    target: &mut [u8],
) -> Result<()> {
    for line in &credits.lines {
        if line.token == "COPYRIGHT" {
            draw_visible_credits_copyright(sources, credits, palette, target);
            continue;
        }
        for (line_offset, tile_ids) in line.tiles.iter().enumerate() {
            let mut draw_x = 0;
            let draw_y = (6 + usize::from(line.line_index) * 2) * SOURCE_TILE_SIZE
                + line_offset * SOURCE_TILE_SIZE;
            for tile_id in tile_ids {
                if *tile_id != 0x7f {
                    let levels = sources.font.levels.get(tile_id).with_context(|| {
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
    sources: &CreditsRenderSources,
    credits: &VisibleCreditsScreen,
    palette: &Palette,
    target: &mut [u8],
) {
    let draw_y = (6 + usize::from(
        credits
            .lines
            .iter()
            .find(|line| line.token == "COPYRIGHT")
            .map(|line| line.line_index)
            .unwrap_or(0),
    ) * 2)
        * SOURCE_TILE_SIZE;
    for (tile_index, levels) in sources.copyright_tiles.iter().enumerate() {
        blit_visible_credits_levels(
            target,
            levels,
            SOURCE_TILE_SIZE,
            SOURCE_TILE_SIZE,
            (2 + tile_index) * SOURCE_TILE_SIZE,
            draw_y,
            palette,
            false,
        );
    }
}

fn load_visible_credits_copyright_tiles(asset_root: &AssetRoot) -> Result<Vec<Vec<u8>>> {
    let path = asset_root.runtime_assets().join("gfx/splash/copyright.png");
    let source = crate::open_runtime_image(&path)
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
    (0..29)
        .map(|tile_index| {
            extract_visible_credits_levels(
                &source,
                tile_index * SOURCE_TILE_SIZE,
                0,
                SOURCE_TILE_SIZE,
                SOURCE_TILE_SIZE,
            )
        })
        .collect()
}

fn draw_visible_credits_the_end(
    sources: &CreditsRenderSources,
    palette: &Palette,
    target: &mut [u8],
) {
    blit_visible_credits_levels(
        target,
        &sources.the_end_levels,
        64,
        16,
        6 * SOURCE_TILE_SIZE,
        9 * SOURCE_TILE_SIZE,
        palette,
        true,
    );
}

fn load_visible_credits_the_end_levels(asset_root: &AssetRoot) -> Result<Vec<u8>> {
    let path = asset_root.runtime_assets().join("gfx/credits/theend.png");
    let source = crate::open_runtime_image(&path)
        .with_context(|| format!("decode credits The End {}", path.display()))?
        .to_rgba8();
    extract_visible_credits_levels(&source, 0, 0, 64, 16)
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
    let source = crate::open_runtime_image(path)
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
    let source = crate::open_runtime_image(path)
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
    let source = crate::open_runtime_image(path)
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
    let source = crate::open_runtime_image(&path)
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

fn should_despawn_player_facing_entity(
    retain_player_sprite: bool,
    is_retained_player_sprite: bool,
) -> bool {
    !retain_player_sprite || !is_retained_player_sprite
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
    map_base_surfaces: Query<'w, 's, (), (With<PlayfieldTile>, Without<PlayfieldPriorityTile>)>,
    map_priority_surfaces: Query<'w, 's, (), (With<PlayfieldTile>, With<PlayfieldPriorityTile>)>,
    map_sprites: Query<
        'w,
        's,
        &'static mut Transform,
        (
            With<PlayfieldTile>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<DialogGlyphMarker>,
            Without<LedgeShadowMarker>,
        ),
    >,
    players: Query<'w, 's, Entity, Or<(With<PlayerMarker>, With<PlayerFacingMarker>)>>,
    player_sprites: Query<
        'w,
        's,
        (
            &'static mut Handle<Image>,
            &'static mut Transform,
            &'static mut Sprite,
            &'static mut PlayerSpriteFrames,
        ),
        (
            With<PlayerMarker>,
            Without<DialogGlyphMarker>,
            Without<VisibleIntroSurface>,
        ),
    >,
    ledge_shadows: Query<
        'w,
        's,
        (&'static mut Transform, &'static Sprite),
        (
            With<LedgeShadowMarker>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<PlayfieldTile>,
            Without<DialogGlyphMarker>,
        ),
    >,
    objects: Query<'w, 's, Entity, With<ObjectMarker>>,
    object_sprites: Query<
        'w,
        's,
        (
            Entity,
            &'static mut VisibleObjectSprite,
            &'static mut Handle<Image>,
            &'static mut Transform,
            &'static mut Sprite,
        ),
        (Without<PlayerMarker>, Without<VisibleIntroSurface>),
    >,
    events: Query<'w, 's, Entity, With<EventMarker>>,
    prompts: Query<'w, 's, Entity, With<FieldPromptMarker>>,
    field_commands: Query<'w, 's, Entity, With<FieldCommandMarker>>,
    scene_dialogs: Query<'w, 's, Entity, With<SceneDialogMarker>>,
    dialog_glyphs: Query<
        'w,
        's,
        (
            Entity,
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
    yes_no_prompts: Query<'w, 's, Entity, With<YesNoPromptMarker>>,
    intro_surfaces: Query<
        'w,
        's,
        (Entity, &'static mut Handle<Image>),
        (With<VisibleIntroSurface>, Without<PlayerMarker>),
    >,
    pokemon_pictures: Query<'w, 's, Entity, With<PokemonPictureMarker>>,
    // Generic title cleanup must never remove the one retained LCD entity.
    // Its pixels are committed in place across every full-screen handoff and
    // it is retired explicitly only after a valid overworld frame is staged.
    title_markers: Query<'w, 's, Entity, (With<TitleScreenMarker>, Without<VisibleIntroSurface>)>,
    battlers: Query<'w, 's, Entity, With<BattleBattlerMarker>>,
    battle_commands: Query<'w, 's, Entity, With<BattleCommandMarker>>,
    fixed_battle_canvases: Query<'w, 's, Entity, With<FixedBattleCanvasMarker>>,
}

fn set_overworld_map_scroll(
    map_sprites: &mut Query<
        &mut Transform,
        (
            With<PlayfieldTile>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<DialogGlyphMarker>,
            Without<LedgeShadowMarker>,
        ),
    >,
    offset: Vec2,
) {
    // The composited viewport has a base layer and a priority layer.  Both
    // are PlayfieldTile entities and must move as one LCD background during
    // camera interpolation.  `get_single_mut` silently stopped scrolling as
    // soon as the priority layer was introduced because this query then
    // matched two entities.
    for mut transform in map_sprites.iter_mut() {
        // The composite is already the complete 640x576 playfield. Centering
        // it at a half-tile offset exposed a 16px strip of ClearColor on the
        // right and bottom edges; only the live camera interpolation belongs
        // in this transform.
        transform.translation.x = offset.x;
        transform.translation.y = offset.y;
    }
}

fn visible_effective_map_time_of_day<'a>(
    map: &'a crate::RuntimeMapCatalogSnapshot,
    live_time_of_day: &'a str,
    flash_active: bool,
) -> &'a str {
    let declared = map.attributes.time_of_day.as_deref();
    if declared.is_some_and(|value| {
        value.eq_ignore_ascii_case("dark") || value.eq_ignore_ascii_case("darkness")
    }) {
        return if flash_active { "nite" } else { "dark" };
    }
    if map.attributes.environment.as_deref().is_some_and(|value| {
        value.eq_ignore_ascii_case("indoor") || value.eq_ignore_ascii_case("gate")
    }) {
        return "indoor";
    }
    declared.unwrap_or(live_time_of_day)
}

fn queue_existing_entity_despawn(
    commands: &mut Commands,
    queued: &mut std::collections::HashSet<Entity>,
    entity: Entity,
) {
    // Render markers intentionally overlap (a glyph can also be a scene or
    // prompt entity). Queue each entity once across the entire render pass so
    // deferred cleanup is idempotent instead of emitting B0003 storms.
    if queued.insert(entity) {
        commands.entity(entity).despawn();
    }
}

fn visual_world_grid_dimensions(_enabled: bool) -> (i16, i16, i16) {
    #[cfg(feature = "voxel-view")]
    if _enabled {
        return (
            VISUAL_WORLD_HALO_TILES,
            VISUAL_WORLD_TILES_X,
            VISUAL_WORLD_TILES_Y,
        );
    }
    (
        CLASSIC_SCROLL_HALO_TILES,
        CLASSIC_SCROLL_TILES_X,
        CLASSIC_SCROLL_TILES_Y,
    )
}

fn retained_field_dialog_structure_matches(
    text_box_background_count: usize,
    frame_tile_count: usize,
    retained_yes_no_prompt: bool,
    desired_yes_no_prompt: bool,
) -> bool {
    let expected_frame_tile_count = battle_window_frame_tile_count(
        FIELD_TEXT_BOX_WIDTH_TILES as usize,
        FIELD_TEXT_BOX_HEIGHT_TILES as usize,
    ) + usize::from(desired_yes_no_prompt)
        * battle_window_frame_tile_count(
            FIELD_YES_NO_WIDTH_TILES as usize,
            FIELD_YES_NO_HEIGHT_TILES as usize,
        );
    text_box_background_count == 1
        && frame_tile_count == expected_frame_tile_count
        && retained_yes_no_prompt == desired_yes_no_prompt
}

fn acknowledge_rendered_field_text(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) {
    runtime_shell.rendered_field_text_identity = visible_field_dialog_pages(snapshot, runtime_shell)
        .and_then(|pages| {
            let reveal = runtime_shell.field_text_reveal.as_ref()?;
            visible_field_dialogue_is_entirely_consumed(runtime_shell, snapshot)
                .then(|| (reveal.text.clone(), reveal.page_index.min(pages.len().saturating_sub(1))))
        });
}

fn render_playfield(
    mut commands: Commands,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut rendered: ResMut<RenderedViewport>,
    mut tileset_art: ResMut<RenderedTilesetArt>,
    mut images: ResMut<Assets<Image>>,
    entity_queries: RenderEntityQueries,
    tick_timer: Option<Res<RuntimeTickTimer>>,
    #[cfg(feature = "voxel-view")] voxel_settings: Option<
        Res<crystal_voxel_view::VoxelViewSettings>,
    >,
) {
    let movement_subframe = tick_timer.map_or(0.0, |timer| timer.presentation_subframe());
    #[cfg(feature = "voxel-view")]
    // Production always installs the setting. Feature-gated extraction tests
    // that invoke this system in isolation intentionally request the complete
    // optional frame by omitting it.
    let visual_world_enabled = voxel_settings.map_or(true, |settings| settings.enabled);
    #[cfg(not(feature = "voxel-view"))]
    let visual_world_enabled = false;
    #[cfg(feature = "voxel-view")]
    let visual_world_mode_unchanged = rendered.visual_world_enabled == visual_world_enabled;
    #[cfg(not(feature = "voxel-view"))]
    let visual_world_mode_unchanged = true;
    let RenderEntityQueries {
        tiles,
        map_base_surfaces,
        map_priority_surfaces,
        mut map_sprites,
        players,
        mut player_sprites,
        mut ledge_shadows,
        objects,
        mut object_sprites,
        events,
        prompts,
        field_commands,
        scene_dialogs,
        mut dialog_glyphs,
        dialog_frame_tiles,
        dialog_text_box_backgrounds,
        yes_no_prompts,
        mut intro_surfaces,
        pokemon_pictures,
        title_markers,
        battlers,
        battle_commands,
        fixed_battle_canvases,
    } = entity_queries;
    let mut queued_despawns = std::collections::HashSet::new();
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
                    if let Ok((_, mut texture)) = intro_surfaces.get_single_mut() {
                        *texture = frame.handle;
                        // A title/options/debug overlay can still be present
                        // on the first retained title -> intro handoff. Clear
                        // every predecessor except the retained LCD entity;
                        // subsequent intro frames normally find no work here.
                        let mut despawned = std::collections::HashSet::new();
                        for entity in tiles
                            .iter()
                            .chain(players.iter())
                            .chain(objects.iter())
                            .chain(events.iter())
                            .chain(prompts.iter())
                            .chain(field_commands.iter())
                            .chain(scene_dialogs.iter())
                            .chain(pokemon_pictures.iter())
                            .chain(title_markers.iter())
                            .chain(battlers.iter())
                            .chain(battle_commands.iter())
                        {
                            if despawned.insert(entity) {
                                queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
                            }
                        }
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
        let mut despawned = std::collections::HashSet::new();
        for entity in tiles
            .iter()
            .chain(players.iter())
            .chain(objects.iter())
            .chain(events.iter())
            .chain(prompts.iter())
            .chain(field_commands.iter())
            .chain(scene_dialogs.iter())
            .chain(pokemon_pictures.iter())
            .chain(title_markers.iter())
            .chain(battlers.iter())
            .chain(battle_commands.iter())
        {
            if despawned.insert(entity) {
                queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
            }
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        if let Err(error) = spawn_visible_credits_screen(
            &mut commands,
            &runtime_shell,
            &credits,
            &mut tileset_art,
            &mut images,
        ) {
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
            &mut tileset_art,
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
            &mut tileset_art,
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
            &mut tileset_art,
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
                Ok(snapshot) => {
                    if let Err(error) = spawn_options_menu_command_window(
                        &mut commands,
                        &snapshot,
                        &runtime_shell,
                        &mut tileset_art,
                        &runtime_shell.asset_root,
                        &mut images,
                    ) {
                        record_visible_render_error(&mut commands, &mut runtime_shell, error);
                    }
                }
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in players.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in events.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in title_markers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battlers.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in battle_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
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
    let player_name_choice = runtime_shell
        .pending_name_choice
        .as_ref()
        .filter(|_| {
            runtime_shell.pending_standard_capture.is_none()
                && runtime_shell.pending_gift_pokemon_nickname.is_none()
                && runtime_shell.pending_egg_hatch_nickname.is_none()
        })
        .cloned();
    let player_name_input = runtime_shell
        .pending_name_input
        .as_ref()
        .filter(|input| input.label == "YOUR NAME?")
        .cloned();
    if player_name_choice.is_some() || player_name_input.is_some() {
        let shell_render_key = shell_render_key(&runtime_shell);
        if rendered.title_active && rendered.shell_render_key == Some(shell_render_key) {
            return;
        }
        for entity in tiles
            .iter()
            .chain(players.iter())
            .chain(objects.iter())
            .chain(events.iter())
            .chain(prompts.iter())
            .chain(field_commands.iter())
            .chain(scene_dialogs.iter())
            .chain(pokemon_pictures.iter())
            .chain(title_markers.iter())
            .chain(battlers.iter())
            .chain(battle_commands.iter())
        {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        rendered.map_name = None;
        rendered.tile = None;
        rendered.state_hash = None;
        rendered.dialog_key = None;
        rendered.shell_render_key = Some(shell_render_key);
        rendered.title_active = true;
        let result = if let Some(choice) = player_name_choice.as_ref() {
            spawn_visible_name_choice_screen(
                &mut commands,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
                choice,
            )
        } else {
            spawn_visible_name_entry_screen(
                &mut commands,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
                player_name_input
                    .as_ref()
                    .expect("player name input was checked above"),
            )
        };
        if let Err(error) = result {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
        }
        return;
    }
    // A freshly spawned base/priority pair is deferred until this system
    // completes. Keep the previous complete LCD presenter through that first
    // field extraction, then retire it on the next update once both layers
    // are independently query-visible. This check precedes the idle fast path
    // so a visually stable field cannot strand the pending presenter.
    if tileset_art.presented_fullscreen_release_pending
        && !retained_field_fullscreen_active(&runtime_shell)
        && map_base_surfaces.iter().next().is_some()
        && map_priority_surfaces.iter().next().is_some()
    {
        remove_presented_fullscreen_entity(&mut commands, &mut tileset_art);
    }
    // Avoid snapshot/checksum work on idle frames. Gameplay actions bump the
    // revision; walking animation and queued audio are the only visual work
    // that can legitimately require a refresh without a new snapshot.
    if rendered.snapshot_revision == Some(runtime_shell.snapshot_revision)
        && visual_world_mode_unchanged
        && runtime_shell.pending_audio.is_empty()
        && runtime_shell.player_walk_frame_ticks == 0
        && runtime_shell.object_walk_frame_ticks == 0
        && runtime_shell.object_walk_frame_ticks_by_id.is_empty()
    {
        return;
    }
    let Ok(current_snapshot) = cached_runtime_snapshot(&mut runtime_shell) else {
        return;
    };
    let scripted_movement_snapshot = runtime_shell
        .visible_script_movement_scene
        .clone()
        .unwrap_or(current_snapshot);
    let arrival_snapshot = runtime_shell
        .pending_overworld_warp_scene
        .clone()
        .unwrap_or(scripted_movement_snapshot);
    let field_snapshot = runtime_shell
        .field_notice_scene
        .as_ref()
        .filter(|_| {
            runtime_shell.field_notice.is_some()
                || runtime_shell.pending_field_notice_effect_frames.is_some()
                || matches!(
                    runtime_shell.visible_fly_animation,
                    Some(VisibleFlyAnimation {
                        phase: VisibleFlyAnimationPhase::From,
                        ..
                    })
                )
                || (runtime_shell.visible_waterfall_animation.is_some()
                    && runtime_shell.field_notice.is_none())
        })
        .cloned()
        .unwrap_or(arrival_snapshot);
    let retained_battle_presentation = !runtime_shell.battle_messages.is_empty()
        || runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active);
    if retained_battle_presentation && runtime_shell.battle_message_scene.is_none() {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!("retained battle presentation has no retained battle scene"),
        );
        return;
    }
    let mut snapshot = if retained_battle_presentation
        || matches!(
            runtime_shell.visible_blackout_phase,
            Some(VisibleBlackoutPhase::FadeOut | VisibleBlackoutPhase::WhiteHold { .. })
        ) {
        runtime_shell
            .battle_message_scene
            .as_ref()
            .map(|scene| Arc::new(scene.as_ref().clone()))
            .unwrap_or(field_snapshot)
    } else {
        field_snapshot
    };
    if !runtime_shell.follower_visible_tile_overrides.is_empty() {
        let snapshot = Arc::make_mut(&mut snapshot);
        for (object_id, tile) in &runtime_shell.follower_visible_tile_overrides {
            snapshot
                .visible_object_runtime_tiles
                .insert(object_id.clone(), *tile);
        }
    }
    tileset_art.selected_window_frame_id = textbox_frame_id(snapshot.trainer.options.frame);
    if rendered
        .map_name
        .as_ref()
        .is_some_and(|map_name| map_name != &snapshot.overworld.map_name)
    {
        sync_visible_map_name_sign(&mut runtime_shell, &snapshot);
    }
    let terminal_battle_scene = retained_battle_presentation
        .then(|| runtime_shell.battle_message_scene.as_deref().cloned())
        .flatten();
    let battle_transition_surface_active = runtime_shell.visible_battle_transition.is_some()
        && !matches!(
            runtime_shell.pending_overworld_step_boundary,
            Some(PendingOverworldStepBoundary::WildBattle)
        );
    let battle_canvas_active = !battle_transition_surface_active
        && (snapshot.battle.is_some() || terminal_battle_scene.is_some());
    let state_hash = snapshot.visual_state_hash;
    runtime_shell.battle_lcd_animation_active = snapshot.battle.is_some();
    let shell_render_key = battle_animated_shell_render_key(&snapshot, &runtime_shell);
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
        && visual_world_mode_unchanged
        && rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.tile == Some(snapshot.overworld.tile)
        && rendered.world_key == Some(world_key)
        && snapshot.battle.is_none()
        && snapshot.ui.menu.is_none()
        && snapshot.pending_shop.is_none()
        && snapshot.ui.active_pokemon_picture.is_none()
        && snapshot.pending_move_learn.is_none()
        && runtime_shell.pending_name_input.is_none()
        && runtime_shell.pending_mail_input.is_none()
        && runtime_shell.pending_name_choice.is_none()
        && runtime_shell.pending_time_set.is_none()
        && runtime_shell.pending_oak_intro.is_none()
        && runtime_shell.pending_gender_selection.is_none()
        && runtime_shell.pending_day_of_week.is_none()
        && runtime_shell.visible_balance_overlay.is_none()
        && runtime_shell.visible_mom_bank.is_none()
        && (snapshot.ui.text.is_some()
            || snapshot.ui.pending_yes_no.is_some()
            || runtime_shell.field_notice.is_some()
            || runtime_shell.pc_notice.is_some());
    if dialog_only_update && rendered.dialog_key != dialog_key {
        let retained_yes_no_prompt = yes_no_prompts.iter().next().is_some();
        let desired_yes_no_prompt = scene_dialog_yes_no_active(&snapshot, &runtime_shell);
        let retained_dialog_frame_tile_count = dialog_frame_tiles.iter().count();
        let has_retained_dialog_frame = retained_field_dialog_structure_matches(
            dialog_text_box_backgrounds.iter().count(),
            retained_dialog_frame_tile_count,
            retained_yes_no_prompt,
            desired_yes_no_prompt,
        );
        // Every field dialog owns one 20x6 frame and, while yes/no is active,
        // exactly one additional 6x4 frame. Looking only for any retained
        // frame mistook orphaned YesNoBox tiles for the next field textbox;
        // its blank surface then survived while the text fast path updated no
        // usable glyphs. Validate the complete layer before mutating it.
        if !has_retained_dialog_frame {
            for entity in scene_dialogs.iter() {
                queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
            }
            if let Err(error) = spawn_scene_dialog(
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
            rendered.state_hash = Some(state_hash);
            rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
            rendered.dialog_key = dialog_key;
            rendered.shell_render_key = Some(shell_render_key);
            acknowledge_rendered_field_text(&mut runtime_shell, &snapshot);
            return;
        }
        if has_retained_dialog_frame
            && update_scene_dialog_text_content_in_place(
                &mut commands,
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
            acknowledge_rendered_field_text(&mut runtime_shell, &snapshot);
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
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in field_commands.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in prompts.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        for entity in pokemon_pictures.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
        let dialog_result = if has_retained_dialog_frame {
            spawn_scene_dialog_text_content(
                &mut commands,
                &snapshot,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            )
        } else {
            spawn_scene_dialog(
                &mut commands,
                &snapshot,
                &runtime_shell,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            )
        };
        if let Err(error) = dialog_result {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.dialog_key = dialog_key;
        rendered.shell_render_key = Some(shell_render_key);
        acknowledge_rendered_field_text(&mut runtime_shell, &snapshot);
        return;
    }
    if dialog_only_update && rendered.dialog_key == dialog_key {
        // The runtime checksum can advance while text/script bookkeeping is
        // unchanged visually.  Keep every retained world entity and only
        // acknowledge the new semantic revision.
        rendered.state_hash = Some(state_hash);
        rendered.snapshot_revision = Some(runtime_shell.snapshot_revision);
        rendered.shell_render_key = Some(shell_render_key);
        acknowledge_rendered_field_text(&mut runtime_shell, &snapshot);
        return;
    }
    if visible_scene_dialog_entries(&snapshot, &runtime_shell)
        .is_ok_and(|entries| entries.is_empty())
    {
        for entity in scene_dialogs.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
    }
    // Save/menu/script bookkeeping can change the semantic checksum without
    // changing any pixels in the overworld. Retain the existing map, player,
    // and object entities in that case; rebuilding them here was the other
    // major source of frame stalls after the tile layer was composited.
    let world_only_update = !rendered.title_active
        && visual_world_mode_unchanged
        && rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.tile == Some(snapshot.overworld.tile)
        && rendered.world_key == Some(world_key)
        && snapshot.battle.is_none()
        && snapshot.ui.text.is_none()
        && snapshot.ui.pending_yes_no.is_none()
        // Closing a dialog changes no world geometry. Do not let the cheap
        // world-only path retain its now-orphaned textbox entities.
        && scene_dialogs.iter().next().is_none()
        && pokemon_pictures.iter().next().is_none()
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
        && runtime_shell.pending_mail_input.is_none()
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
        && (runtime_shell.player_walk_frame_ticks > 0
            || runtime_shell.object_walk_frame_ticks > 0
            || !runtime_shell.object_walk_frame_ticks_by_id.is_empty()
            || rendered.walk_viewport_origin.is_some())
    {
        // The semantic snapshot stays at the destination tile while the LCD
        // is still showing the in-between walking frames. Move the retained
        // sprites only; rebuilding the map here would reintroduce the black
        // flash this fast path exists to prevent.
        if let Some((start_x, start_y)) = rendered.viewport_origin
            && update_overworld_sprite_positions(
                &snapshot,
                movement_subframe,
                runtime_shell.visible_ledge_jump,
                runtime_shell.player_walk_from,
                runtime_shell.player_walk_frame_ticks,
                runtime_shell.player_walk_total_ticks,
                &runtime_shell.object_walk_from,
                &runtime_shell.object_walk_frame_ticks_by_id,
                &runtime_shell.object_walk_total_ticks_by_id,
                runtime_shell.trainer_walk_from.as_ref(),
                runtime_shell.object_walk_frame_ticks,
                runtime_shell.object_walk_total_ticks,
                visible_overworld_camera_offset(&rendered, &runtime_shell, movement_subframe),
                start_x,
                start_y,
                &mut player_sprites,
                &mut ledge_shadows,
                &mut object_sprites,
            )
        {
            let camera_offset =
                visible_overworld_camera_offset(&rendered, &runtime_shell, movement_subframe);
            set_overworld_map_scroll(&mut map_sprites, camera_offset);
            if camera_offset == Vec2::ZERO {
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
        && visual_world_mode_unchanged
        && rendered.tile == Some(snapshot.overworld.tile)
        && rendered.world_key == Some(world_key)
        && rendered.state_hash == Some(state_hash)
        && rendered.shell_render_key == Some(shell_render_key)
        && snapshot.ui.active_pokemon_picture.is_none()
    {
        return;
    }

    // Never remove a drawable map merely because an overworld snapshot has
    // changed. A turn, walking commit, or camera scroll updates this same
    // retained LCD surface; tearing it down exposes Bevy's clear colour for
    // a frame before its replacement is available.
    let retain_walking_viewport = rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name);
    // The two composited map entities are the LCD's persistent background
    // surfaces, including across map-name changes.  Their image handles are
    // updated in place below once the complete destination viewport has been
    // validated and composed.  Queuing their despawn here is not safe: Bevy
    // queries still see deferred-despawn entities later in this system, so the
    // spawn-if-missing branch would observe the old pair and create no
    // replacements.  After command application the destination could then be
    // left with no map surface at all, exposing the window clear colour.
    // A same-map redraw updates the one retained player entity below. Never
    // despawn it merely because facing/mode changed: Commands are deferred,
    // so the voxel/classic camera handoff otherwise sees a frame with no
    // player and repeated steps visibly alternate between present/absent.
    let retain_player_sprite = retain_walking_viewport && player_sprites.iter().count() == 1;
    // UI glyphs and frames intentionally carry several marker components
    // (for example both FieldCommandMarker and SceneDialogMarker). Bevy
    // queries are snapshots, and despawns are deferred, so independently
    // clearing every marker query queued the same entity more than once.
    // Native dialogue then emitted thousands of B0003 warnings and slowed
    // down further on every page. Queue each entity exactly once per frame.
    // PlayerFacingMarker also owns short-lived OAM effects (grass rustle,
    // ledge shadow, fishing frames, and the optional facing overlay). Keep
    // only the real PlayerMarker entity on same-map redraws. Retaining every
    // PlayerFacingMarker caused one new grass tile to accumulate on every
    // animation tick until the overlapping sprites formed a giant smear.
    for entity in players.iter() {
        if should_despawn_player_facing_entity(
            retain_player_sprite,
            player_sprites.contains(entity),
        ) {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
    }
    // Same-map object sprites are part of the retained LCD just like the
    // player and map surfaces. Ambient tile animation, follower-only walks,
    // and script bookkeeping all reach this full-render boundary; replacing
    // every NPC there makes the extracted 2D frame intermittently contain no
    // objects while the new entities/textures are prepared. Reconcile the
    // existing entities in place below and only add/remove objects whose
    // actual visible identity changed.
    let retain_object_sprites = retain_walking_viewport;
    if !retain_object_sprites {
        for entity in objects.iter() {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
    } else {
        // Temporary effects (including Fly leaves and the bird) also carry
        // ObjectMarker but not the stable VisibleObjectSprite identity used
        // by the reconciler. Retire those every redraw so a multi-frame field
        // animation cannot accumulate prior-frame OAM while the map NPCs stay
        // retained beneath it.
        for entity in objects.iter() {
            if object_sprites.get(entity).is_err() {
                queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
            }
        }
    }
    for entity in events.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    for entity in prompts.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    for entity in field_commands.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    for entity in scene_dialogs.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    for entity in pokemon_pictures.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    for entity in title_markers.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    // The title/intro handoff intentionally retains one opaque LCD entity to
    // avoid a macOS clear-colour flash between boot frames. Once a complete
    // overworld viewport is ready, that continuity surface must be retired as
    // well; it is excluded from `title_markers` above so generic modal cleanup
    // cannot accidentally tear it down during the intro itself.
    let map_layers_were_query_visible =
        map_base_surfaces.iter().next().is_some() && map_priority_surfaces.iter().next().is_some();
    for (entity, _) in intro_surfaces.iter_mut() {
        // The same retained entity may already have been adopted as the
        // overworld LCD surface later in this system. Retire only its
        // full-screen ownership markers; despawning it would invalidate the
        // queued map texture/transform updates applied at the command flush.
        let mut retained_surface = commands.entity(entity);
        retained_surface.remove::<TitleScreenMarker>();
        if map_layers_were_query_visible && !retained_field_fullscreen_active(&runtime_shell) {
            retained_surface.remove::<VisibleIntroSurface>();
        }
    }
    for entity in battlers.iter() {
        queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
    }
    // The opaque LCD canvas is the battle renderer's continuity anchor. Keep
    // one instance alive while battlers, HUD, text, and animation objects are
    // rebuilt around it. A deferred despawn/spawn pair is normally applied in
    // one command flush, but retaining the canvas also protects an already
    // presented battle if required art fails later in this update.
    let retained_fixed_battle_canvas = battle_canvas_active
        .then(|| fixed_battle_canvases.iter().next())
        .flatten();
    for entity in battle_commands.iter() {
        if Some(entity) != retained_fixed_battle_canvas {
            queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
        }
    }
    if battle_canvas_active && retained_fixed_battle_canvas.is_none() {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    // Pokemon palette color zero expands to full white. Keep
                    // the battle canvas identical so enclosed white sprite
                    // details do not show as a mismatched rectangle/halo.
                    color: Color::WHITE,
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
    let Some((start_x, start_y)) = connection_composite_viewport_origin(
        &snapshot,
        map,
        player_render_x,
        player_render_y,
        width,
        height,
    ) else {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "connection composite bounds for {} exceed supported render coordinates",
                map.map_name,
            ),
        );
        return;
    };
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
    // PALETTE_AUTO is exported as no map-level override.  In Crystal that
    // means the outdoor palette follows the live clock; indoor/gate maps use
    // their separate environment palette even though maps.asm calls them
    // PALETTE_DAY.
    let live_time_of_day = match snapshot.progression.time.time_of_day {
        crystal_core::world::encounters::TimeOfDay::Morning => "morn",
        crystal_core::world::encounters::TimeOfDay::Day => "day",
        crystal_core::world::encounters::TimeOfDay::Night => "nite",
    };
    let flash_active = snapshot
        .progression
        .active_engine_flags
        .contains("STATUSFLAGS_FLASH");
    let effective_time_of_day =
        visible_effective_map_time_of_day(map, live_time_of_day, flash_active);
    let objects_above_priority = visible_object_indices_above_priority(&snapshot, map, tileset);
    let tileset_art_key = TilesetArtKey {
        tileset_id: tileset.tileset_id.clone(),
        time_of_day: effective_time_of_day.to_string(),
    };
    let map_visual_key =
        {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // Palette tables are immutable pack data, but map blocks are not:
            // callbacks, script changeblock commands, and field moves replace
            // them at runtime. Hash every map that can contribute viewport pixels
            // so the retained 2D surface is recomposed after such a write.
            map.map_name.hash(&mut hasher);
            visible_map_block_identity(&snapshot).hash(&mut hasher);
            map.attributes.tileset_name.hash(&mut hasher);
            tileset_art_key.time_of_day.hash(&mut hasher);
            map.attributes.border_block.hash(&mut hasher);
            tileset.palette_map.hash(&mut hasher);
            for connection in &map.attributes.connections {
                connection.direction.hash(&mut hasher);
                connection.target_map.hash(&mut hasher);
                connection.offset.hash(&mut hasher);
                if let Some(target_map) = snapshot
                    .maps
                    .iter()
                    .find(|candidate| candidate.map_name == connection.target_map)
                {
                    target_map.attributes.tileset_name.hash(&mut hasher);
                    target_map.attributes.border_block.hash(&mut hasher);
                    visible_effective_map_time_of_day(target_map, live_time_of_day, flash_active)
                        .hash(&mut hasher);
                    if let Some(target_tileset) = snapshot.tilesets.iter().find(|candidate| {
                        candidate.tileset_id == target_map.attributes.tileset_name
                    }) {
                        target_tileset.palette_map.hash(&mut hasher);
                    }
                }
            }
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
        // Position-only retention cannot choose new directional sprite art.
        // A turn must fall through to the retained-player redraw below, which
        // swaps standing/walking image handles without despawning the entity.
        && rendered.player_sprite_facing == Some(snapshot.overworld.facing)
        && rendered.player_sprite_mode == Some(snapshot.overworld.mode)
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
        && runtime_shell.pending_mail_input.is_none()
        && runtime_shell.pending_name_choice.is_none()
        // These effects add independent OAM sprites. The position-only fast
        // path updates player/NPC transforms and returns before spawning
        // them, which otherwise drops their first frame when movement and
        // effect creation occur in the same authoritative tick.
        && runtime_shell.visible_ledge_jump.is_none()
        && runtime_shell.visible_grass_rustle.is_none()
        && runtime_shell.visible_strength_boulder_dust.is_none()
        && !runtime_debug_overlays_enabled();
    // A changed camera origin also changes the pixels behind every sprite.
    // Let the retained texture update below before moving the sprites;
    // returning here left the old viewport in place and caused the subsequent
    // render to tear down the visible playfield.
    if can_update_positions_in_place
        && rendered.viewport_origin == Some((start_x, start_y))
        && update_overworld_sprite_positions(
            &snapshot,
            movement_subframe,
            runtime_shell.visible_ledge_jump,
            runtime_shell.player_walk_from,
            runtime_shell.player_walk_frame_ticks,
            runtime_shell.player_walk_total_ticks,
            &runtime_shell.object_walk_from,
            &runtime_shell.object_walk_frame_ticks_by_id,
            &runtime_shell.object_walk_total_ticks_by_id,
            runtime_shell.trainer_walk_from.as_ref(),
            runtime_shell.object_walk_frame_ticks,
            runtime_shell.object_walk_total_ticks,
            visible_overworld_camera_offset(&rendered, &runtime_shell, movement_subframe),
            start_x,
            start_y,
            &mut player_sprites,
            &mut ledge_shadows,
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
    for connection in &map.attributes.connections {
        let Some(target_map) = snapshot
            .maps
            .iter()
            .find(|candidate| candidate.map_name == connection.target_map)
        else {
            record_visible_render_error(
                &mut commands,
                &mut runtime_shell,
                anyhow::anyhow!(
                    "connection target {} referenced by {} is missing from the map catalog",
                    connection.target_map,
                    map.map_name,
                ),
            );
            return;
        };
        let Some(target_tileset) = snapshot
            .tilesets
            .iter()
            .find(|candidate| candidate.tileset_id == target_map.attributes.tileset_name)
        else {
            record_visible_render_error(
                &mut commands,
                &mut runtime_shell,
                anyhow::anyhow!(
                    "connection target {} references missing tileset {}",
                    target_map.map_name,
                    target_map.attributes.tileset_name,
                ),
            );
            return;
        };
        let target_art_key = TilesetArtKey {
            tileset_id: target_tileset.tileset_id.clone(),
            time_of_day: visible_effective_map_time_of_day(
                target_map,
                live_time_of_day,
                flash_active,
            )
            .to_string(),
        };
        if !tileset_art.cache.contains_key(&target_art_key) {
            match load_tileset_art(
                &runtime_shell.asset_root,
                &target_art_key.tileset_id,
                &target_art_key.time_of_day,
                &target_tileset.palette_map,
                &mut images,
            ) {
                Ok(art) => {
                    tileset_art.errors.remove(&target_art_key);
                    tileset_art.cache.insert(target_art_key.clone(), art);
                }
                Err(error) => {
                    tileset_art
                        .errors
                        .insert(target_art_key.clone(), error.to_string());
                }
            }
        }
        if !tileset_art.cache.contains_key(&target_art_key) {
            let error = tileset_art
                .errors
                .get(&target_art_key)
                .cloned()
                .unwrap_or_else(|| "unknown connection tileset art load error".to_string());
            record_visible_render_error(
                &mut commands,
                &mut runtime_shell,
                anyhow::anyhow!(
                    "required connection tileset art {} ({}) could not be loaded: {}",
                    target_art_key.tileset_id,
                    target_art_key.time_of_day,
                    error,
                ),
            );
            return;
        }
    }

    // Keep the map layer as one retained sprite instead of 360 independent
    // Bevy entities.  The old per-tile entity churn was the dominant cost on
    // every camera step and was the reason the shell could fall to one FPS.
    // Objects, player, dialog, and battle overlays remain separate layers.
    let mut viewport_tile_handles = Vec::with_capacity(
        usize::try_from(CLASSIC_SCROLL_TILES_X * CLASSIC_SCROLL_TILES_Y).unwrap_or_default(),
    );
    let mut priority_viewport_tiles = Vec::with_capacity(
        usize::try_from(CLASSIC_SCROLL_TILES_X * CLASSIC_SCROLL_TILES_Y).unwrap_or_default(),
    );
    let (visual_world_halo, visual_world_tiles_x, visual_world_tiles_y) =
        visual_world_grid_dimensions(visual_world_enabled);
    #[cfg(any(test, feature = "voxel-view"))]
    let mut visual_tiles = Vec::with_capacity(
        usize::try_from(visual_world_tiles_x * visual_world_tiles_y).unwrap_or_default(),
    );
    #[cfg(feature = "voxel-view")]
    let mut visual_world_tile_handles = Vec::with_capacity(
        if visual_world_enabled {
            usize::try_from(visual_world_tiles_x * visual_world_tiles_y).unwrap_or_default()
        } else {
            0
        },
    );
    #[cfg(any(test, feature = "voxel-view"))]
    let mut visual_tileset_ids = HashMap::<&str, Arc<str>>::new();
    for visual_y in 0..visual_world_tiles_y {
        for visual_x in 0..visual_world_tiles_x {
            let x = visual_x - visual_world_halo;
            let y = visual_y - visual_world_halo;
            let inside_scroll_surface = x >= -CLASSIC_SCROLL_HALO_TILES
                && y >= -CLASSIC_SCROLL_HALO_TILES
                && x < VIEWPORT_TILES_X + CLASSIC_SCROLL_HALO_TILES
                && y < VIEWPORT_TILES_Y + CLASSIC_SCROLL_HALO_TILES;
            let map_x = i32::from(start_x) + i32::from(x);
            let map_y = i32::from(start_y) + i32::from(y);
            let (source_map, source_x, source_y) =
                connection_render_source(&snapshot, map, map_x, map_y)
                    .unwrap_or((map, map_x, map_y));
            let Some(source_tileset) = snapshot
                .tilesets
                .iter()
                .find(|candidate| candidate.tileset_id == source_map.attributes.tileset_name)
            else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "render source map {} references missing tileset {}",
                        source_map.map_name,
                        source_map.attributes.tileset_name,
                    ),
                );
                return;
            };
            let source_art_key = TilesetArtKey {
                tileset_id: source_tileset.tileset_id.clone(),
                time_of_day: visible_effective_map_time_of_day(
                    source_map,
                    live_time_of_day,
                    flash_active,
                )
                .to_string(),
            };
            let source_width =
                i32::from(source_map.attributes.width) * i32::from(RENDER_METATILE_WIDTH);
            let source_height =
                i32::from(source_map.attributes.height) * i32::from(RENDER_METATILE_WIDTH);
            let (block, sub_x, sub_y) = if source_x >= 0
                && source_y >= 0
                && source_x < source_width
                && source_y < source_height
            {
                let block_x = source_x.div_euclid(i32::from(RENDER_METATILE_WIDTH));
                let block_y = source_y.div_euclid(i32::from(RENDER_METATILE_WIDTH));
                let sub_x = source_x.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                let sub_y = source_y.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                let index =
                    (block_y as usize * source_map.attributes.width as usize) + block_x as usize;
                (source_map.blocks[index], sub_x, sub_y)
            } else {
                let sub_x = source_x.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                let sub_y = source_y.rem_euclid(i32::from(RENDER_METATILE_WIDTH)) as usize;
                (source_map.attributes.border_block as u16, sub_x, sub_y)
            };
            let Some((source_tile_index, tile_handle)) =
                tileset_art.cache.get(&source_art_key).and_then(|art| {
                    let offset = usize::from(block)
                        .checked_mul(METATILE_TILE_COUNT)?
                        .checked_add(sub_y.checked_mul(RENDER_METATILE_WIDTH as usize)?)?
                        .checked_add(sub_x)?;
                    let tile_index = *art.metatile_layout.get(offset)?;
                    let handle = art.tile_handle_at_frame(
                        block,
                        sub_x,
                        sub_y,
                        runtime_shell.lcd_animation_frame,
                        snapshot
                            .progression
                            .active_engine_flags
                            .contains("ENGINE_FOREST_IS_RESTLESS"),
                    )?;
                    Some((tile_index, handle))
                })
            else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "required tileset art {} missing metatile {} sub-tile ({}, {})",
                        source_art_key.tileset_id,
                        block,
                        sub_x,
                        sub_y
                    ),
                );
                return;
            };
            #[cfg(not(any(test, feature = "voxel-view")))]
            let _ = source_tile_index;
            if inside_scroll_surface {
                viewport_tile_handles.push(tile_handle.clone());
            }
            #[cfg(feature = "voxel-view")]
            if visual_world_enabled {
                visual_world_tile_handles.push(tile_handle.clone());
            }
            // Collision data remains private to the faithful 2D compositor:
            // it selects the classic foreground-priority layer, but is never
            // exported as optional-renderer height or shape information.
            let Some(collision) = tileset_collision_tokens(source_tileset, block) else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "required tileset {} collision row {} is missing",
                        source_tileset.tileset_id,
                        block,
                    ),
                );
                return;
            };
            if collision.len() != 4 {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "tileset {} collision row {} has {} quadrants instead of 4",
                        source_tileset.tileset_id,
                        block,
                        collision.len(),
                    ),
                );
                return;
            }
            let foreground_bottom = collision[0] == "FLOOR"
                && collision[1] == "FLOOR"
                && collision[2..4]
                    .iter()
                    .all(|token| token == "WALL" || priority_collision_token(token));
            let collision_index = (if sub_y < 2 { 0 } else { 2 }) + if sub_x < 2 { 0 } else { 1 };
            let priority = priority_collision_token(&collision[collision_index])
                || (foreground_bottom && sub_y >= 1);
            #[cfg(any(test, feature = "voxel-view"))]
            {
                let visual_tileset_id = visual_tileset_ids
                    .entry(source_tileset.tileset_id.as_str())
                    .or_insert_with(|| Arc::from(source_tileset.tileset_id.as_str()))
                    .clone();
                visual_tiles.push(crystal_render_api::VisualTile {
                    column: visual_x as u32,
                    row: visual_y as u32,
                    source: crystal_render_api::VisualTileSource {
                        tileset_id: visual_tileset_id,
                        metatile_id: block,
                        subtile_column: u8::try_from(sub_x)
                            .expect("4x4 metatile column always fits u8"),
                        subtile_row: u8::try_from(sub_y)
                            .expect("4x4 metatile row always fits u8"),
                        tile_index: u16::from(source_tile_index),
                    },
                    texture: tile_handle.clone(),
                    priority,
                });
            }
            let priority_spec = if priority {
                let Some(handle) = tileset_art
                    .cache
                    .get(&source_art_key)
                    .and_then(|art| art.priority_tile_handle(block, sub_x, sub_y))
                else {
                    record_visible_render_error(
                        &mut commands,
                        &mut runtime_shell,
                        anyhow::anyhow!(
                            "required priority art {} missing metatile {} sub-tile ({}, {})",
                            source_art_key.tileset_id,
                            block,
                            sub_x,
                            sub_y,
                        ),
                    );
                    return;
                };
                Some((
                    handle,
                    if foreground_bottom && sub_y == 1 {
                        SOURCE_TILE_SIZE / 2
                    } else {
                        0
                    },
                ))
            } else {
                None
            };
            if inside_scroll_surface {
                priority_viewport_tiles.push(priority_spec);
            }
        }
    }
    let previous_viewport_origin = rendered.viewport_origin;
    let tile_frame_key = {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for handle in &viewport_tile_handles {
            handle.id().hash(&mut hasher);
        }
        for spec in &priority_viewport_tiles {
            spec.as_ref().map(|(handle, clip_top)| (handle.id(), *clip_top)).hash(&mut hasher);
        }
        #[cfg(feature = "voxel-view")]
        for handle in &visual_world_tile_handles {
            handle.id().hash(&mut hasher);
        }
        hasher.finish()
    };
    let ambient_tile_frame_due = runtime_shell.ambient_tileset_animation_active
        && runtime_shell
            .ambient_tileset_animation_schedule
            .iter()
            .any(|(period, offset)| {
                runtime_shell.lcd_animation_frame >= *offset
                    && (runtime_shell.lcd_animation_frame - *offset) % (*period).max(1) == 0
            });
    let facing_only_redraw = rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && previous_viewport_origin == Some((start_x, start_y))
        && rendered.map_visual_key == Some(map_visual_key)
        && rendered.position_key == Some(position_key)
        && rendered.appearance_key == Some(appearance_key)
        && rendered.player_sprite_mode == Some(snapshot.overworld.mode)
        && rendered.player_sprite_facing != Some(snapshot.overworld.facing)
        && !ambient_tile_frame_due;
    let retained_texture_content = rendered.map_texture.is_some()
        && rendered.map_priority_texture.is_some()
        && rendered.tile_frame_key == Some(tile_frame_key);
    let origin_changing_walk = rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && (runtime_shell.player_walk_frame_ticks > 0
            || runtime_shell.visible_ledge_jump.is_some())
        && previous_viewport_origin != Some((start_x, start_y));
    // The active base and priority surfaces include a complete four-tile
    // halo, which covers the largest ordinary camera step without exposing
    // an edge or requiring a duplicate texture pair.
    // There is exactly one mutable base-layer LCD image. Keeping its handle on
    // the visible entity lets Bevy draw the previous complete viewport until
    // the replacement upload is ready. A former origin cache stored this same
    // active handle and then mutated it for later camera positions, so every
    // cached entry silently aliased and returned whichever viewport was drawn
    // last. Recompose into the sole active image instead of caching aliases.
    let viewport_texture = if facing_only_redraw || retained_texture_content {
        rendered
            .map_texture
            .clone()
            .expect("facing-only redraw retains an initialized base texture")
    } else {
        compose_tile_grid(
            &viewport_tile_handles,
            CLASSIC_SCROLL_TILES_X as usize,
            CLASSIC_SCROLL_TILES_Y as usize,
            rendered.map_texture.clone(),
            &mut images,
        )
    };
    let priority_viewport_texture = if facing_only_redraw || retained_texture_content {
        rendered
            .map_priority_texture
            .clone()
            .expect("facing-only redraw retains an initialized priority texture")
    } else {
        compose_priority_tile_grid(
            &priority_viewport_tiles,
            CLASSIC_SCROLL_TILES_X as usize,
            CLASSIC_SCROLL_TILES_Y as usize,
            rendered.map_priority_texture.clone(),
            &mut images,
        )
    };
    rendered.map_texture = Some(viewport_texture.clone());
    rendered.map_priority_texture = Some(priority_viewport_texture.clone());
    #[cfg(feature = "voxel-view")]
    {
        if visual_world_enabled && !facing_only_redraw && !retained_texture_content {
            rendered.visual_world_texture = Some(compose_visual_world_tiles(
                &visual_world_tile_handles,
                visual_world_tiles_x as usize,
                visual_world_tiles_y as usize,
                rendered.visual_world_texture.clone(),
                &mut images,
            ));
        }
        rendered.visual_world_grid_size = UVec2::new(
            visual_world_tiles_x as u32,
            visual_world_tiles_y as u32,
        );
        rendered.visual_world_enabled = visual_world_enabled;
    }
    #[cfg(any(test, feature = "voxel-view"))]
    {
        rendered.visual_tiles = visual_tiles;
    }
    rendered.viewport_origin = Some((start_x, start_y));
    rendered.walk_viewport_origin = if origin_changing_walk {
        previous_viewport_origin
    } else {
        None
    };
    rendered.map_visual_key = Some(map_visual_key);
    rendered.tile_frame_key = Some(tile_frame_key);
    let mut visible_tileset_art_keys = vec![tileset_art_key.clone()];
    for connection in &map.attributes.connections {
        if let Some(target_map) = snapshot
            .maps
            .iter()
            .find(|candidate| candidate.map_name == connection.target_map)
            && let Some(target_tileset) = snapshot
                .tilesets
                .iter()
                .find(|candidate| candidate.tileset_id == target_map.attributes.tileset_name)
        {
            visible_tileset_art_keys.push(TilesetArtKey {
                tileset_id: target_tileset.tileset_id.clone(),
                time_of_day: visible_effective_map_time_of_day(
                    target_map,
                    live_time_of_day,
                    flash_active,
                )
                .to_string(),
            });
        }
    }
    visible_tileset_art_keys.sort_by(|left, right| {
        left.tileset_id
            .cmp(&right.tileset_id)
            .then_with(|| left.time_of_day.cmp(&right.time_of_day))
    });
    visible_tileset_art_keys.dedup();
    runtime_shell.ambient_tileset_animation_active = visible_tileset_art_keys.iter().any(|key| {
        tileset_art
            .cache
            .get(key)
            .is_some_and(|art| !art.animated_tiles.is_empty())
    });
    let mut schedule = visible_tileset_art_keys
        .iter()
        .filter_map(|key| tileset_art.cache.get(key))
        .flat_map(|art| {
            art.animated_tiles
                .values()
                .filter(|animation| {
                    !animation.requires_forest_restless
                        || snapshot
                            .progression
                            .active_engine_flags
                            .contains("ENGINE_FOREST_IS_RESTLESS")
                })
                .flat_map(|animation| {
                    std::iter::once((animation.frame_ticks.max(1), animation.phase_offset))
                        .chain(animation.additional_schedule.iter().copied())
                })
        })
        .collect::<Vec<_>>();
    schedule.sort_unstable();
    schedule.dedup();
    runtime_shell.ambient_tileset_animation_schedule = schedule;
    let camera_offset =
        visible_overworld_camera_offset(&rendered, &runtime_shell, movement_subframe);
    set_overworld_map_scroll(&mut map_sprites, camera_offset);
    if can_update_positions_in_place
        && tiles.iter().count() == 2
        && update_overworld_sprite_positions(
            &snapshot,
            movement_subframe,
            runtime_shell.visible_ledge_jump,
            runtime_shell.player_walk_from,
            runtime_shell.player_walk_frame_ticks,
            runtime_shell.player_walk_total_ticks,
            &runtime_shell.object_walk_from,
            &runtime_shell.object_walk_frame_ticks_by_id,
            &runtime_shell.object_walk_total_ticks_by_id,
            runtime_shell.trainer_walk_from.as_ref(),
            runtime_shell.object_walk_frame_ticks,
            runtime_shell.object_walk_total_ticks,
            visible_overworld_camera_offset(&rendered, &runtime_shell, movement_subframe),
            start_x,
            start_y,
            &mut player_sprites,
            &mut ledge_shadows,
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
                    custom_size: Some(Vec2::new(CLASSIC_SCROLL_WIDTH, CLASSIC_SCROLL_HEIGHT)),
                    ..default()
                },
                // Image pixel (0, 0) is already the playfield's top-left;
                // center the complete 640x576 composite on the camera.
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
            PlayfieldTile,
        ));
        commands.spawn((
            SpriteBundle {
                texture: priority_viewport_texture,
                sprite: Sprite {
                    custom_size: Some(Vec2::new(CLASSIC_SCROLL_WIDTH, CLASSIC_SCROLL_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 2.4),
                ..default()
            },
            PlayfieldTile,
            PlayfieldPriorityTile,
        ));
    }
    {
        let mut reconciled_object_entities = std::collections::HashSet::new();
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
            let Some(source_object_slot) = snapshot.visible_object_slots.get(index).copied() else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!("visible object {index} has no source object slot"),
                );
                return;
            };
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
            let Some((view_x, view_y)) = runtime_event_view_tile(object_tile, start_x, start_y)
            else {
                continue;
            };
            let walking_from = object.object_identifier.as_ref().and_then(|object_id| {
                runtime_shell
                    .trainer_walk_from
                    .as_ref()
                    .filter(|(walking_id, _)| walking_id == object_id)
                    .map(|(_, from)| *from)
                    .or_else(|| runtime_shell.object_walk_from.get(object_id).copied())
            });
            let destination_visible = overworld_object_in_scroll_region(view_x, view_y);
            let origin_visible = walking_from
                .and_then(|from| runtime_event_view_tile(from, start_x, start_y))
                .is_some_and(|(x, y)| overworld_object_in_scroll_region(x, y));
            if !destination_visible && !origin_visible {
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
                .or_else(|| object_event_initial_facing(&object.spritemovedata));
            let Some(direction) = direction else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "visible object {:?} on {} has no runtime or source-facing direction",
                        object.object_identifier,
                        snapshot.overworld.map_name,
                    ),
                );
                return;
            };
            let render_sprite_id = sprite_id.clone();
            let sprite_frame = sprite_frame_for_art(
                &mut tileset_art,
                &runtime_shell.asset_root,
                &render_sprite_id,
                palette_id,
                effective_time_of_day,
                direction,
                false,
                &mut images,
            );
            let walking_frame =
                if object_sprite_is_animated(&object.spritemovedata) && sprite_frame.is_some() {
                    sprite_frame_for_art(
                        &mut tileset_art,
                        &runtime_shell.asset_root,
                        &render_sprite_id,
                        palette_id,
                        effective_time_of_day,
                        direction,
                        true,
                        &mut images,
                    )
                } else {
                    None
                };
            if object_sprite_is_animated(&object.spritemovedata)
                && sprite_frame.is_some()
                && walking_frame.is_none()
            {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "animated overworld object {:?} ({render_sprite_id}) has no required action frame",
                        object.object_identifier,
                    ),
                );
                return;
            }
            if let Some(frame) = sprite_frame {
                let animated = object_sprite_is_animated(&object.spritemovedata);
                let walk_phase = object
                    .object_identifier
                    .as_ref()
                    .and_then(|object_id| runtime_shell.object_walk_phases.get(object_id).copied());
                let uses_action_frame = walking_from.is_some()
                    && walk_phase.map_or(
                        !runtime_shell.object_walk_stride,
                        object_walk_uses_action_frame,
                    );
                let display_frame = if uses_action_frame {
                    walking_frame.as_ref().unwrap_or(&frame)
                } else {
                    &frame
                };
                let (object_x, object_y) = if let Some(from) = walking_from {
                    let trainer_is_walking =
                        object.object_identifier.as_ref().is_some_and(|object_id| {
                            runtime_shell
                                .trainer_walk_from
                                .as_ref()
                                .is_some_and(|(walking_id, _)| walking_id == object_id)
                        });
                    let (remaining, total_ticks) = if trainer_is_walking {
                        (
                            runtime_shell.object_walk_frame_ticks,
                            runtime_shell.object_walk_total_ticks,
                        )
                    } else {
                        let Some(object_id) = object.object_identifier.as_ref() else {
                            record_visible_render_error(
                                &mut commands,
                                &mut runtime_shell,
                                anyhow::anyhow!("retained object walk has no object identifier"),
                            );
                            return;
                        };
                        let Some(remaining) = runtime_shell
                            .object_walk_frame_ticks_by_id
                            .get(object_id)
                            .copied()
                        else {
                            record_visible_render_error(
                                &mut commands,
                                &mut runtime_shell,
                                anyhow::anyhow!(
                                    "retained object walk {object_id} has no remaining-frame timer"
                                ),
                            );
                            return;
                        };
                        let Some(total) = runtime_shell
                            .object_walk_total_ticks_by_id
                            .get(object_id)
                            .copied()
                        else {
                            record_visible_render_error(
                                &mut commands,
                                &mut runtime_shell,
                                anyhow::anyhow!(
                                    "retained object walk {object_id} has no total-frame timer"
                                ),
                            );
                            return;
                        };
                        (remaining, total)
                    };
                    let target = render_tile_playfield_position(view_x, view_y);
                    if total_ticks == 0 {
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!("retained object walk has a zero-frame duration"),
                        );
                        return;
                    }
                    let Some((from_view_x, from_view_y)) =
                        runtime_event_view_tile(from, start_x, start_y)
                    else {
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!(
                                "retained object walk origin ({}, {}) overflows viewport coordinates",
                                from.x,
                                from.y,
                            ),
                        );
                        return;
                    };
                    // Moving OAM may begin just outside the LCD and enter during
                    // this stride. A static-position viewport clamp here would
                    // collapse that valid offscreen origin to the destination.
                    let from = render_tile_playfield_position(from_view_x, from_view_y);
                    let progress = visible_movement_progress(remaining, total_ticks);
                    overworld_sprite_position_from_base(
                        from.0 + (target.0 - from.0) * progress,
                        from.1 + (target.1 - from.1) * progress,
                        display_frame.size,
                    )
                } else {
                    overworld_sprite_position(view_x, view_y, display_frame.size)
                };
                let next_sprite = Sprite {
                    custom_size: Some(display_frame.size),
                    flip_x: uses_action_frame
                        && matches!(direction, Direction::Up | Direction::Down)
                        && walk_phase.map_or(
                            !runtime_shell.object_walk_stride,
                            object_walk_uses_mirrored_action_frame,
                        ),
                    ..default()
                };
                // Match LoadAndSortSprites: objects farther down the map are
                // nearer the camera and must draw over objects above them.
                let next_transform = Transform::from_xyz(
                    object_x + camera_offset.x,
                    object_y + camera_offset.y,
                    if objects_above_priority.contains(&index) {
                        2.41
                    } else {
                        overworld_entity_depth(
                            object_tile,
                            Some(source_object_slot),
                            (start_x, start_y),
                        )
                    },
                );
                let next_visible = VisibleObjectSprite {
                    object_index: index,
                    object_identifier: object.object_identifier.clone(),
                    source_id: Arc::from(render_sprite_id.as_str()),
                    above_priority: objects_above_priority.contains(&index),
                    standing: frame.handle.clone(),
                    walking: walking_frame.as_ref().map(|frame| frame.handle.clone()),
                    mirror_walking: matches!(direction, Direction::Up | Direction::Down),
                    animated,
                };
                let retained = retain_object_sprites
                    .then(|| {
                        object_sprites
                            .iter_mut()
                            .find(|(_, rendered, _, _, _)| rendered.object_index == index)
                    })
                    .flatten();
                if let Some((entity, mut rendered, mut texture, mut transform, mut sprite)) =
                    retained
                {
                    *rendered = next_visible;
                    *texture = display_frame.handle.clone();
                    *transform = next_transform;
                    *sprite = next_sprite;
                    reconciled_object_entities.insert(entity);
                } else {
                    commands.spawn((
                        SpriteBundle {
                            texture: display_frame.handle.clone(),
                            sprite: next_sprite,
                            transform: next_transform,
                            ..default()
                        },
                        ObjectMarker,
                        next_visible,
                    ));
                }
            } else {
                let key = SpriteArtKey {
                    sprite_id: render_sprite_id.clone(),
                    palette_id,
                    time_of_day: normalize_tileset_time_of_day(effective_time_of_day),
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
                let (object_tile_x, object_tile_y) = render_tile_playfield_position(view_x, view_y);
                spawn_object_label(&mut commands, object, object_tile_x, object_tile_y);
            }
        }
        if retain_object_sprites {
            for (entity, _, _, _, _) in object_sprites.iter_mut() {
                if !reconciled_object_entities.contains(&entity) {
                    queue_existing_entity_despawn(&mut commands, &mut queued_despawns, entity);
                }
            }
        }
    }

    if let Some(dust) = runtime_shell.visible_strength_boulder_dust.as_ref() {
        let time_of_day = effective_time_of_day;
        let Some(frames) = boulder_dust_frames_for_art(
            &mut tileset_art,
            &runtime_shell.asset_root,
            time_of_day,
            &mut images,
        ) else {
            let key = normalize_tileset_time_of_day(time_of_day);
            let error = tileset_art
                .boulder_dust_errors
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "unknown boulder dust load error".to_string());
            record_visible_render_error(
                &mut commands,
                &mut runtime_shell,
                anyhow::anyhow!("required boulder dust could not be rendered: {error}"),
            );
            return;
        };
        if let Some(target) = snapshot
            .visible_object_runtime_tiles
            .get(&dust.object_id)
            .copied()
        {
            let Some((target_view_x, target_view_y)) =
                runtime_event_view_tile(target, start_x, start_y)
            else {
                record_visible_render_error(
                    &mut commands,
                    &mut runtime_shell,
                    anyhow::anyhow!("Strength boulder target overflows viewport coordinates"),
                );
                return;
            };
            let retained_origin = runtime_shell.object_walk_from.get(&dust.object_id).copied();
            let visible = [Some(target), retained_origin]
                .into_iter()
                .flatten()
                .any(|tile| {
                    runtime_event_view_tile(tile, start_x, start_y).is_some_and(|(x, y)| {
                        (0..VIEWPORT_TILES_X).contains(&x) && (0..VIEWPORT_TILES_Y).contains(&y)
                    })
                });
            if visible {
                let mut position = render_tile_playfield_position(target_view_x, target_view_y);
                if let Some(from) = retained_origin {
                    let Some(remaining) = runtime_shell
                        .object_walk_frame_ticks_by_id
                        .get(&dust.object_id)
                        .copied()
                    else {
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!("Strength boulder dust has no remaining-frame timer"),
                        );
                        return;
                    };
                    let Some(total) = runtime_shell
                        .object_walk_total_ticks_by_id
                        .get(&dust.object_id)
                        .copied()
                    else {
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!("Strength boulder dust has no total-frame timer"),
                        );
                        return;
                    };
                    if total == 0 {
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!("Strength boulder dust has a zero-frame duration"),
                        );
                        return;
                    }
                    let Some((from_view_x, from_view_y)) =
                        runtime_event_view_tile(from, start_x, start_y)
                    else {
                        record_visible_render_error(
                            &mut commands,
                            &mut runtime_shell,
                            anyhow::anyhow!(
                                "Strength boulder origin overflows viewport coordinates"
                            ),
                        );
                        return;
                    };
                    let from = render_tile_playfield_position(from_view_x, from_view_y);
                    let progress =
                        (f32::from(total) - f32::from(remaining.min(total))) / f32::from(total);
                    position.0 = from.0 + (position.0 - from.0) * progress;
                    position.1 = from.1 + (position.1 - from.1) * progress;
                }
                let (offset_x, offset_y) = match dust.direction {
                    Direction::Down => (0.0, -4.0),
                    Direction::Up => (0.0, 8.0),
                    Direction::Left => (6.0, 2.0),
                    Direction::Right => (-6.0, 2.0),
                };
                let frame = &frames[usize::from((dust.age / 2) % 2)];
                let (x, y) = overworld_sprite_position_from_base(
                    position.0 + offset_x * 4.0,
                    position.1 - offset_y * 4.0,
                    frame.size,
                );
                commands.spawn((
                    SpriteBundle {
                        texture: frame.handle.clone(),
                        sprite: Sprite {
                            custom_size: Some(frame.size),
                            ..default()
                        },
                        // SPRITEMOVEDATA_BOULDERDUST carries LOW_PRIORITY, so
                        // keep it immediately behind its tracked boulder.
                        transform: Transform::from_xyz(
                            x,
                            y,
                            overworld_entity_depth(target, None, (start_x, start_y)) - 0.000_001,
                        ),
                        ..default()
                    },
                    ObjectMarker,
                    BoulderDustMarker,
                ));
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
        effective_time_of_day,
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
        effective_time_of_day,
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
        effective_time_of_day,
    ) {
        record_visible_render_error(&mut commands, &mut runtime_shell, error);
        return;
    }
    if let Some(emote) = runtime_shell.visible_overworld_emote.clone() {
        let target = if matches!(
            emote.object.as_str(),
            "PLAYER" | "PLAYER_OBJECT" | "LAST_TALKED"
        ) && (emote.object != "LAST_TALKED"
            || snapshot.script_events.last_talked_object.is_none())
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
                        let (emote_x, emote_y) =
                            overworld_emote_position_from_base(x, y, frame.size);
                        commands.spawn((
                            SpriteBundle {
                                texture: frame.handle,
                                sprite: Sprite {
                                    custom_size: Some(frame.size),
                                    ..default()
                                },
                                transform: Transform::from_xyz(emote_x, emote_y, 3.2),
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
                Color::srgb(0.18, 0.42, 0.96),
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
                Color::srgb(0.92, 0.92, 0.86),
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
                Color::srgb(0.74, 0.42, 0.94),
                TILE_SIZE * 0.42,
                1.3,
            );
            spawn_coord_event_label(&mut commands, start_x, start_y, coord);
        }
    }

    let (movement_from, movement_remaining, movement_total) = runtime_shell
        .visible_ledge_jump
        .map(|jump| (Some(jump.from), 16_u8.saturating_sub(jump.frame), 16))
        .unwrap_or((
            runtime_shell.player_walk_from,
            runtime_shell.player_walk_frame_ticks,
            runtime_shell.player_walk_total_ticks,
        ));
    let Some((mut player_x, mut player_y_base)) =
        visible_player_playfield_position_for_duration_with_subframe(
            snapshot.overworld.tile,
            movement_from,
            movement_remaining,
            movement_total,
            movement_subframe,
            start_x,
            start_y,
        )
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
    player_x += camera_offset.x;
    player_y_base += camera_offset.y;
    if let Err(error) = spawn_visible_fly_animation(
        &mut commands,
        &runtime_shell,
        &mut tileset_art,
        &mut images,
        player_x,
        player_y_base,
        effective_time_of_day,
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
            effective_time_of_day,
            snapshot.overworld.facing,
            false,
            &mut images,
        ),
        sprite_frame_for_art(
            &mut tileset_art,
            &runtime_shell.asset_root,
            player_sprite_id,
            player_palette_id,
            effective_time_of_day,
            snapshot.overworld.facing,
            true,
            &mut images,
        ),
    );
    if player_art.0.is_some() && player_art.1.is_none() {
        record_visible_render_error(
            &mut commands,
            &mut runtime_shell,
            anyhow::anyhow!(
                "player overworld sprite {player_sprite_id} has no required action frame"
            ),
        );
        return;
    }
    if snapshot.overworld_player_hidden {
        // `hide_object PLAYER`/special presentation owns this state. The
        // sprite is intentionally absent; it is not an art-load failure.
    } else if let Some(standing_frame) = player_art.0 {
        let walking_frame = player_art.1;
        let fishing_frame = if runtime_shell.visible_fishing_animation.is_some() {
            match fishing_player_frame(
                &mut tileset_art,
                &runtime_shell.asset_root,
                female,
                snapshot.overworld.facing,
                player_palette_id,
                effective_time_of_day,
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
        let player_is_moving = !runtime_shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| {
                (movement.object_id == "PLAYER" && movement.active_uses_standing_frame)
                    || (movement.follower_object_id.as_deref() == Some("PLAYER")
                        && movement.follower_active_uses_standing_frame
                        && runtime_shell.player_walk_frame_ticks > 0)
            })
            && (runtime_shell.visible_ledge_jump.is_some()
                || runtime_shell.player_walk_frame_ticks > 0);
        let player_uses_action_frame =
            player_is_moving && player_walk_uses_action_frame(runtime_shell.player_walk_stride);
        let frame = if let Some(frame) = fishing_frame.as_ref() {
            frame
        } else if player_uses_action_frame {
            walking_frame.as_ref().unwrap_or(&standing_frame)
        } else {
            &standing_frame
        };
        let (player_x, player_ground_y) =
            overworld_sprite_position_from_base(player_x, player_y_base, frame.size);
        let fishing_bob = runtime_shell
            .visible_fishing_animation
            .filter(|animation| {
                animation.phase == VisibleFishingPhase::Hook
                    && animation.frame < 32
                    && animation.frame & 1 == 1
            })
            .map_or(0.0, |_| -(TILE_SIZE / SOURCE_TILE_SIZE as f32));
        let player_y = player_ground_y
            + visible_ledge_jump_y_offset(runtime_shell.visible_ledge_jump)
            + fishing_bob;
        let player_depth_tile = runtime_shell.visible_ledge_jump.map_or_else(
            || {
                runtime_shell
                    .player_walk_from
                    .filter(|_| runtime_shell.player_walk_frame_ticks > 0)
                    .unwrap_or(snapshot.overworld.tile)
            },
            |jump| {
                if jump.frame < WALK_FRAME_HOLD_TICKS {
                    jump.from
                } else {
                    TilePosition {
                        x: jump.from.x + (jump.to.x - jump.from.x) / 2,
                        y: jump.from.y + (jump.to.y - jump.from.y) / 2,
                    }
                }
            },
        );
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
                        player_ground_y - frame.size.y * 0.5 + shadow.size.y * 0.5,
                        overworld_entity_depth(player_depth_tile, None, (start_x, start_y))
                            - 0.000_001,
                    ),
                    ..default()
                },
                PlayerFacingMarker,
                LedgeShadowMarker,
            ));
        }
        let player_flip_x = player_uses_action_frame
            && runtime_shell.player_walk_mirror_stride
            && matches!(snapshot.overworld.facing, Direction::Up | Direction::Down);
        if retain_player_sprite && runtime_shell.visible_fly_animation.is_none() {
            if let Ok((mut texture, mut transform, mut sprite, mut frames)) =
                player_sprites.get_single_mut()
            {
                *texture = frame.handle.clone();
                *transform = Transform::from_xyz(
                    player_x,
                    player_y,
                    overworld_entity_depth(player_depth_tile, None, (start_x, start_y)),
                );
                sprite.custom_size = Some(frame.size);
                sprite.flip_x = player_flip_x;
                frames.standing = standing_frame.handle.clone();
                frames.walking = walking_frame.as_ref().map(|frame| frame.handle.clone());
                frames.mirror_walking = matches!(
                    snapshot.overworld.facing,
                    Direction::Up | Direction::Down
                );
                rendered.player_sprite_facing = Some(snapshot.overworld.facing);
                rendered.player_sprite_mode = Some(snapshot.overworld.mode);
            }
        } else if !retain_player_sprite && runtime_shell.visible_fly_animation.is_none() {
            commands.spawn((
                SpriteBundle {
                    texture: frame.handle.clone(),
                    sprite: Sprite {
                        custom_size: Some(frame.size),
                        flip_x: player_flip_x,
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        player_x,
                        player_y,
                        overworld_entity_depth(player_depth_tile, None, (start_x, start_y)),
                    ),
                    ..default()
                },
                PlayerMarker,
                PlayerSpriteFrames {
                    standing: standing_frame.handle.clone(),
                    walking: walking_frame.as_ref().map(|frame| frame.handle.clone()),
                    mirror_walking: matches!(
                        snapshot.overworld.facing,
                        Direction::Up | Direction::Down
                    ),
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
        if let Some(rustle) = runtime_shell.visible_grass_rustle.filter(|_| {
            runtime_shell
                .visible_ledge_jump
                .map_or(true, |jump| jump.frame >= WALK_FRAME_HOLD_TICKS)
        }) {
            let time_of_day = effective_time_of_day;
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
            commands.spawn((
                SpriteBundle {
                    texture: rustle_frame.handle.clone(),
                    sprite: Sprite {
                        custom_size: Some(rustle_frame.size),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        player_x,
                        player_y - frame.size.y * 0.5 + rustle_frame.size.y * 0.5,
                        2.5 + f32::from(rustle.tile.y) * 0.001,
                    ),
                    ..default()
                },
                PlayerFacingMarker,
                GrassRustleMarker,
            ));
        }
    } else {
        let key = SpriteArtKey {
            sprite_id: player_sprite_id.to_string(),
            palette_id: player_palette_id,
            time_of_day: normalize_tileset_time_of_day(effective_time_of_day),
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
                    color: Color::srgb(1.0, 0.95, 0.32),
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

    if let Some(transition) = runtime_shell.visible_battle_transition
        && !matches!(
            runtime_shell.pending_overworld_step_boundary,
            Some(PendingOverworldStepBoundary::WildBattle)
        )
    {
        spawn_visible_battle_transition(
            &mut commands,
            transition,
            rendered.map_texture.clone(),
            rendered.map_priority_texture.clone(),
        );
    }

    if runtime_shell.visible_battle_transition.is_none()
        && let Some(battle) = &snapshot.battle
    {
        let player_send_out_pending = runtime_shell.battle_player_send_out_pending
            || (runtime_shell.battle_entry_messages_remaining == 0
                && runtime_shell
                    .battle_messages
                    .front()
                    .is_some_and(|message| visible_message_is_player_send_out(message)))
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
        let capture_throw_active = runtime_shell
            .visible_capture_animation
            .as_ref()
            .is_some_and(VisibleCaptureAnimation::throw_active);
        if let Err(error) = spawn_battle_battler_markers(
            &mut commands,
            &snapshot,
            battle,
            runtime_shell.battle_entry_messages_remaining,
            runtime_shell.battle_enemy_send_out_pending,
            player_send_out_pending,
            capture_enemy_hidden,
            capture_enemy_clip_tiles,
            capture_throw_active,
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
            runtime_shell.visible_trainer_exit_animation.is_some(),
            runtime_shell.battle_hp_tween.as_ref(),
            runtime_shell.battle_exp_tween.as_ref(),
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
        if let Err(error) = spawn_battle_command_menu(
            &mut commands,
            &snapshot,
            &runtime_shell,
            battle,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
        ) {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
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
                runtime_shell
                    .visible_capture_animation
                    .as_ref()
                    .is_some_and(VisibleCaptureAnimation::throw_active),
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
                runtime_shell.visible_trainer_exit_animation.is_some(),
                runtime_shell.battle_hp_tween.as_ref(),
                runtime_shell.battle_exp_tween.as_ref(),
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
            if let Err(error) = spawn_battle_command_menu(
                &mut commands,
                scene,
                &runtime_shell,
                battle,
                &mut tileset_art,
                &runtime_shell.asset_root,
                &mut images,
            ) {
                record_visible_render_error(&mut commands, &mut runtime_shell, error);
                return;
            }
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
    } else if runtime_shell.pokegear_menu_open
        || !scene_dialog_surface_active(&snapshot, &runtime_shell)
    {
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
        let mut field_command_render_error = None;
        spawn_field_command_menu(
            &mut commands,
            &snapshot,
            &runtime_shell,
            &mut tileset_art,
            &runtime_shell.asset_root,
            &mut images,
            &mut field_command_render_error,
        );
        if let Some(error) = field_command_render_error {
            record_visible_render_error(&mut commands, &mut runtime_shell, error);
            return;
        }
    }
    if runtime_shell.pokedex_scripted_entry && runtime_shell.pending_standard_capture.is_some() {
        if let Err(error) = spawn_field_pokedex_screen(
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
    if let Err(error) = spawn_scene_dialog(
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

    // Every fallible overworld compositor and overlay has succeeded. Retire
    // the title/full-screen presenter only now, after the destination map,
    // player, objects, and UI have all been staged into this command buffer.
    // Field-owned full-LCD screens keep the presenter until their UI closes.
    // A cold map pair is deferred, so retain one extra extraction unless both
    // layers were already visible at system start. The image allocation stays
    // alive for the next title/credits sequence; only its ECS entity retires.
    if !retained_field_fullscreen_active(&runtime_shell) {
        if map_base_surfaces.iter().next().is_some()
            && map_priority_surfaces.iter().next().is_some()
        {
            remove_presented_fullscreen_entity(&mut commands, &mut tileset_art);
        } else if tileset_art.presented_fullscreen_entity.is_some() {
            tileset_art.presented_fullscreen_release_pending = true;
        }
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
    acknowledge_rendered_field_text(&mut runtime_shell, &snapshot);
    #[cfg(feature = "voxel-view")]
    {
        rendered.visual_world_enabled = visual_world_enabled;
    }
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
        rendered_art
            .party_icon_cache
            .insert("ICON_BIRD".to_string(), frames.clone());
        frames
    };
    // TypeScript's fly animation deliberately builds one stateless 16x16
    // surface from the first four icon tiles; it does not run the party-menu
    // two-frame icon cycle here.
    let icon = icon_frames[0].clone();
    let (player_origin_x, player_origin_y) =
        overworld_sprite_position_from_base(player_origin_x, player_origin_y, icon.size);
    let snapshot = runtime_shell.shell.snapshot()?;
    let (mut player, origin_y) = match animation.phase {
        VisibleFlyAnimationPhase::From => (
            VisibleFlyObjectState {
                x: 84,
                y: 80,
                x_offset: 0,
                angle: 0,
            },
            80_u8,
        ),
        VisibleFlyAnimationPhase::To => (
            VisibleFlyObjectState {
                x: 84,
                y: 248,
                x_offset: 0,
                angle: 0,
            },
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
                y: player
                    .y
                    .wrapping_add(selector.wrapping_mul(2))
                    .wrapping_add(64),
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
                    player.x_offset =
                        visible_battle_anim_sine(player.angle.wrapping_add(0x10), amplitude);
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
            leaf.x_offset = visible_battle_anim_sine(leaf.angle.wrapping_add(0x10), 0x40);
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

    let x = player_origin_x + (i32::from(player.x) + player.x_offset - 84) as f32 * scale;
    let y = player_origin_y - i32::from(player.y.wrapping_sub(origin_y) as i8) as f32 * scale;
    commands.spawn((
        SpriteBundle {
            texture: icon.handle,
            sprite: Sprite {
                custom_size: Some(icon.size),
                ..default()
            },
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
            0x32 | 0x33 | 0x42 | 0x43 => {
                visible_whirlpool_tile_frame(
                    rendered_art,
                    &runtime_shell.asset_root,
                    images,
                    &runtime_tileset,
                    time_of_day,
                    tile_id,
                    phase,
                )?
                .handle
            }
            _ => base_handles
                .get(usize::from(tile_id))
                .cloned()
                .with_context(|| {
                    format!("WHIRLPOOL metatile references missing tile ${tile_id:02x}")
                })?,
        };
        let sub_x = (index % usize::from(RENDER_METATILE_WIDTH as u16)) as f32;
        let sub_y = (index / usize::from(RENDER_METATILE_WIDTH as u16)) as f32;
        commands.spawn((
            SpriteBundle {
                texture: handle,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
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
    let cache_graphic = format!("whirlpool:{}:{tile_id:02x}:{phase}", tileset.tileset_id);
    let cache_key = (cache_graphic, time.clone(), 0);
    if let Some(frame) = rendered_art.field_move_tile_cache.get(&cache_key) {
        return Ok(frame.clone());
    }
    let path = asset_root
        .runtime_assets()
        .join("gfx/tilesets/whirlpool")
        .join(format!("{source_number}.2bpp"));
    let data = crate::read_runtime_asset(&path)
        .with_context(|| format!("read WHIRLPOOL animation tile {}", path.display()))?;
    let offset = usize::from(phase) * 16;
    let tile = data
        .get(offset..offset + 16)
        .with_context(|| format!("WHIRLPOOL source {} lacks phase {phase}", path.display()))?;
    let palette_value = tileset
        .palette_map
        .get(usize::from(tile_id))
        .copied()
        .with_context(|| {
            format!("WHIRLPOOL tile ${tile_id:02x} has no tileset palette-map entry")
        })?;
    let palette_index = usize::from(palette_value & 7);
    let palette_bank = load_tileset_palette_bank(asset_root, &tileset.tileset_id, &time)?
        .context("WHIRLPOOL requires a tileset palette bank")?;
    let palette = palette_bank
        .get(palette_index)
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
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let frame = SpriteFrame {
        handle: images.add(image),
        size: Vec2::splat(TILE_SIZE),
    };
    rendered_art
        .field_move_tile_cache
        .insert(cache_key, frame.clone());
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
    else {
        return Ok(());
    };
    let Some((target_x, target_y)) =
        runtime_tile_playfield_position(animation.target_tile, start_x, start_y)
    else {
        return Ok(());
    };
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
    let grass = rendered_art
        .cache
        .get(&tileset_key)
        .and_then(|art| art.tile_handles.get(5))
        .cloned()
        .context("HEADBUTT requires source background tile $05")?;
    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    for (piece_x, piece_y) in [(-8_i32, -8_i32), (0, -8), (-8, 0), (0, 0)] {
        let x = source_origin_x + (spawn_x + piece_x + 4) as f32 * scale;
        let y = source_origin_y - (spawn_y + piece_y + 4) as f32 * scale;
        commands.spawn((
            SpriteBundle {
                texture: grass.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 3.05),
                ..default()
            },
            ObjectMarker,
        ));
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
        commands,
        snapshot,
        rendered_art,
        &runtime_shell.asset_root,
        images,
        time_of_day,
        "headbutt_tree",
        oam_name,
        xflip,
        source_origin_x,
        source_origin_y,
        spawn_x,
        spawn_y,
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
    else {
        return Ok(());
    };
    let Some((target_x, target_y)) =
        runtime_tile_playfield_position(animation.target_tile, start_x, start_y)
    else {
        return Ok(());
    };
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
        let Some(oam_name) = oam_name else {
            return Ok(());
        };
        let (spawn_x, spawn_y) = match animation.facing {
            Direction::Right => (12_i32, 11_i32),
            Direction::Left => (8, 11),
            Direction::Down => (10, 13),
            Direction::Up => (10, 9),
        };
        return spawn_visible_field_move_oam(
            commands,
            snapshot,
            rendered_art,
            &runtime_shell.asset_root,
            images,
            time_of_day,
            "cut_tree",
            oam_name,
            false,
            source_origin_x,
            source_origin_y,
            spawn_x,
            spawn_y,
        );
    }
    if !animation.variant.eq_ignore_ascii_case("grass") {
        anyhow::bail!(
            "unknown exported CUT animation variant {}",
            animation.variant
        );
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
        (11, 12),
        (9, 12),
        (11, 14),
        (9, 14),
        (11, 8),
        (9, 8),
        (11, 10),
        (9, 10),
        (7, 12),
        (9, 12),
        (7, 10),
        (9, 10),
        (11, 12),
        (13, 12),
        (11, 10),
        (13, 10),
    ];
    let parity = usize::from((player_x & 1) != 0) + usize::from((player_y & 1) != 0) * 2;
    let (base_x, base_y) = LEAF_COORDS[direction_index + parity];
    let amplitude = ((u16::from(age) + 1) / 2) as u8;
    for initial_angle in [0_u8, 0x10, 0x20, 0x30] {
        let angle = initial_angle.wrapping_add(age.wrapping_mul(3));
        spawn_visible_field_move_oam(
            commands,
            snapshot,
            rendered_art,
            &runtime_shell.asset_root,
            images,
            time_of_day,
            "cut_grass",
            "SPRITE_ANIM_OAMSET_LEAF",
            false,
            source_origin_x,
            source_origin_y,
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
        rendered_art.intro_sprite_bundle_cache = Some(
            serde_json::from_str(&snapshot.presentation.sprite_anim_bundle)
                .context("parse packed sprite animation bundle for CUT")?,
        );
    }
    let oam_set = rendered_art
        .intro_sprite_bundle_cache
        .as_ref()
        .and_then(|bundle| bundle.get("oam_sets"))
        .and_then(|sets| sets.get(oam_name))
        .with_context(|| {
            format!("field-move OAM set {oam_name} is missing from the packed bundle")
        })?;
    let tile_offset = oam_set
        .get("tile_offset")
        .and_then(serde_json::Value::as_u64)
        .and_then(|offset| u8::try_from(offset).ok())
        .with_context(|| format!("field-move OAM set {oam_name} has an invalid tile offset"))?;
    let pieces = oam_set
        .get("pieces")
        .and_then(serde_json::Value::as_array)
        .with_context(|| format!("field-move OAM set {oam_name} has no pieces"))?
        .clone();
    for piece in pieces {
        let piece_x = piece
            .get("x")
            .and_then(serde_json::Value::as_i64)
            .with_context(|| format!("CUT OAM set {oam_name} has a piece without X"))?
            as i32;
        let piece_y = piece
            .get("y")
            .and_then(serde_json::Value::as_i64)
            .with_context(|| format!("CUT OAM set {oam_name} has a piece without Y"))?
            as i32;
        let tile = piece
            .get("tile")
            .and_then(serde_json::Value::as_u64)
            .and_then(|tile| u8::try_from(tile).ok())
            .with_context(|| format!("CUT OAM set {oam_name} has an invalid tile"))?
            .checked_add(tile_offset)
            .context("field-move OAM tile overflow")?;
        let attributes = piece
            .get("attributes")
            .and_then(serde_json::Value::as_u64)
            .with_context(|| format!("CUT OAM set {oam_name} has a piece without attributes"))?;
        if attributes & 0x7 != 6 {
            anyhow::bail!(
                "CUT OAM set {oam_name} requires unexpected palette {}",
                attributes & 0x7
            );
        }
        let frame = visible_field_move_tile_frame(
            rendered_art,
            asset_root,
            images,
            time_of_day,
            graphic,
            tile,
        )?;
        let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
        let piece_x = if frame_xflip { -8 - piece_x } else { piece_x };
        let x = source_origin_x + (object_x + piece_x + 4) as f32 * scale;
        let y = source_origin_y - (object_y + piece_y + 4) as f32 * scale;
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 3.1),
                ..default()
            },
            ObjectMarker,
        ));
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
    if let Some(frame) = rendered_art.field_move_tile_cache.get(&key) {
        return Ok(frame.clone());
    }
    let path = asset_root
        .runtime_assets()
        .join("gfx/overworld")
        .join(format!("{graphic}.2bpp"));
    let data = crate::read_runtime_asset(&path)
        .with_context(|| format!("read field-move graphics {}", path.display()))?;
    let offset = usize::from(tile_index)
        .checked_mul(16)
        .context("field-move tile offset overflow")?;
    let tile = data.get(offset..offset + 16).with_context(|| {
        format!(
            "field-move tile {tile_index} is missing from {}",
            path.display()
        )
    })?;
    let palettes = load_npc_sprite_palette_bank(asset_root, &time)?;
    let palette = palettes
        .get(6)
        .context("field-move object requires PAL_OW_TREE palette 6")?;
    let mut pixels = vec![0_u8; 8 * 8 * 4];
    for row in 0..8_usize {
        for column in 0..8_usize {
            let bit = 1 << (7 - column);
            let colour = (((tile[row * 2 + 1] & bit != 0) as usize) << 1)
                | (tile[row * 2] & bit != 0) as usize;
            if colour == 0 {
                continue;
            }
            let target = (row * 8 + column) * 4;
            pixels[target..target + 3].copy_from_slice(&palette[colour]);
            pixels[target + 3] = 255;
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let frame = SpriteFrame {
        handle: images.add(image),
        size: Vec2::splat(TILE_SIZE),
    };
    rendered_art
        .field_move_tile_cache
        .insert(key, frame.clone());
    Ok(frame)
}
