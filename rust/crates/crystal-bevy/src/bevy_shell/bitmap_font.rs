fn bitmap_font_2bpp_tile_handle(
    data: &[u8],
    tile_index: usize,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>> {
    let offset = tile_index
        .checked_mul(16)
        .context("font tile offset overflow")?;
    let tile = data
        .get(offset..offset + 16)
        .with_context(|| format!("font 2bpp tile {tile_index} is missing"))?;
    let mut pixels = vec![0_u8; BITMAP_FONT_TILE_SIZE * BITMAP_FONT_TILE_SIZE * 4];
    for row in 0..BITMAP_FONT_TILE_SIZE {
        let lo = tile[row * 2];
        let hi = tile[row * 2 + 1];
        for col in 0..BITMAP_FONT_TILE_SIZE {
            let bit = 1 << (7 - col);
            let level = ((hi & bit != 0) as u8) << 1 | (lo & bit != 0) as u8;
            if level == 0 {
                continue;
            }
            let target = (row * BITMAP_FONT_TILE_SIZE + col) * 4;
            pixels[target] = 20;
            pixels[target + 1] = 24;
            pixels[target + 2] = 20;
            pixels[target + 3] = 255;
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: BITMAP_FONT_TILE_SIZE as u32,
            height: BITMAP_FONT_TILE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(images.add(image))
}

fn bitmap_font_tile_handle(
    source: &image::RgbaImage,
    tile_index: usize,
    tiles_per_row: usize,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>> {
    let source_width = source.width() as usize;
    let source_height = source.height() as usize;
    let source_x = (tile_index % tiles_per_row) * BITMAP_FONT_TILE_SIZE;
    let source_y = (tile_index / tiles_per_row) * BITMAP_FONT_TILE_SIZE;
    if source_x + BITMAP_FONT_TILE_SIZE > source_width
        || source_y + BITMAP_FONT_TILE_SIZE > source_height
    {
        anyhow::bail!(
            "bitmap font tile {} is outside source dimensions {}x{}",
            tile_index,
            source_width,
            source_height
        );
    }
    let mut data = vec![0_u8; BITMAP_FONT_TILE_SIZE * BITMAP_FONT_TILE_SIZE * 4];
    for row in 0..BITMAP_FONT_TILE_SIZE {
        for col in 0..BITMAP_FONT_TILE_SIZE {
            let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            let [r, g, b, a] = pixel.0;
            let target = (row * BITMAP_FONT_TILE_SIZE + col) * 4;
            // Crystal's font PNG is a 2-bit grayscale sheet: white is the
            // transparent paper/background and dark pixels are the glyph.
            // Treating bright pixels as opaque (the old condition) produced
            // black rectangles around every letter in dialogue and YES/NO
            // prompts.  Match the TypeScript font normalizer by discarding
            // near-white pixels and retaining the glyph levels.
            if bitmap_font_glyph_pixel(r, g, b, a) {
                data[target] = 20;
                data[target + 1] = 24;
                data[target + 2] = 20;
                data[target + 3] = 255;
            } else {
                data[target + 3] = 0;
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: BITMAP_FONT_TILE_SIZE as u32,
            height: BITMAP_FONT_TILE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(images.add(image))
}

fn load_window_frame_art(
    asset_root: &AssetRoot,
    frame_id: u8,
    images: &mut Assets<Image>,
) -> Result<WindowFrameArt> {
    anyhow::ensure!((1..=8).contains(&frame_id), "invalid textbox frame {frame_id}");
    let frame_path = asset_root
        .runtime_assets()
        .join(format!("gfx/frames/{frame_id}.png"));
    let source = crate::open_runtime_image(&frame_path)
        .with_context(|| format!("load battle window frame {}", frame_path.display()))?
        .to_rgba8();
    let palette_path = asset_root
        .runtime_assets()
        .join("gfx/stats/party_menu_bg.pal");
    let palette_source = crate::read_runtime_asset_to_string(&palette_path)
        .with_context(|| format!("read textbox palette {}", palette_path.display()))?;
    let palette = parse_palette_file(&palette_source, None)?
        .into_iter()
        .next()
        .with_context(|| format!("textbox palette {} is empty", palette_path.display()))?;
    let expected_width = BITMAP_FONT_TILE_SIZE * 3;
    let expected_height = BITMAP_FONT_TILE_SIZE * 2;
    if source.width() as usize != expected_width || source.height() as usize != expected_height {
        anyhow::bail!(
            "battle window frame {} has invalid dimensions {}x{}; expected {}x{}",
            frame_path.display(),
            source.width(),
            source.height(),
            expected_width,
            expected_height
        );
    }
    Ok(WindowFrameArt {
        top_left: window_frame_tile(&source, &palette, 0, 0, images)?,
        top_edge: window_frame_tile(&source, &palette, 1, 0, images)?,
        top_right: window_frame_tile(&source, &palette, 2, 0, images)?,
        side_edge: window_frame_tile(&source, &palette, 0, 1, images)?,
        bottom_left: window_frame_tile(&source, &palette, 1, 1, images)?,
        bottom_right: window_frame_tile(&source, &palette, 2, 1, images)?,
    })
}

fn window_frame_tile(
    source: &image::RgbaImage,
    palette: &Palette,
    tile_x: usize,
    tile_y: usize,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let mut data = vec![0_u8; BITMAP_FONT_TILE_SIZE * BITMAP_FONT_TILE_SIZE * 4];
    let source_x = tile_x * BITMAP_FONT_TILE_SIZE;
    let source_y = tile_y * BITMAP_FONT_TILE_SIZE;
    for row in 0..BITMAP_FONT_TILE_SIZE {
        for col in 0..BITMAP_FONT_TILE_SIZE {
            let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            let target = (row * BITMAP_FONT_TILE_SIZE + col) * 4;
            let color = palette[palette_index_from_gray(pixel[0])];
            data[target..target + 3].copy_from_slice(&color);
            data[target + 3] = 255;
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: BITMAP_FONT_TILE_SIZE as u32,
            height: BITMAP_FONT_TILE_SIZE as u32,
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
        size: Vec2::splat(TILE_SIZE),
    })
}

fn bitmap_font_char_map() -> HashMap<char, u16> {
    let mut map = HashMap::new();
    map.insert(' ', 0x7f);
    for (index, ch) in "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().enumerate() {
        map.insert(ch, 0x80 + index as u16);
    }
    for (index, ch) in "abcdefghijklmnopqrstuvwxyz".chars().enumerate() {
        map.insert(ch, 0xa0 + index as u16);
    }
    for index in 0..10 {
        map.insert(char::from(b'0' + index as u8), 0xf6 + index as u16);
    }
    for (ch, tile_id) in [
        ('(', 0x9a),
        (')', 0x9b),
        (':', 0x9c),
        (';', 0x9d),
        ('[', 0x9e),
        (']', 0x9f),
        ('\'', 0xe0),
        ('-', 0xe3),
        ('?', 0xe6),
        ('!', 0xe7),
        ('.', 0xe8),
        ('&', 0xe9),
        ('é', 0xea),
        ('É', 0xea),
        ('Ä', 0xc0),
        ('Ö', 0xc1),
        ('Ü', 0xc2),
        ('ä', 0xc3),
        ('ö', 0xc4),
        ('ü', 0xc5),
        ('×', 0xf1),
        ('/', 0xf3),
        (',', 0xf4),
        ('>', 0xed),
        ('<', 0x71),
        ('◀', 0x71),
        ('▷', 0xec),
        ('▶', 0xed),
        ('▼', 0xee),
        ('▲', 0x61),
        ('♂', 0xef),
        ('♀', 0xf5),
        ('¥', 0xf0),
        ('☎', 0x62),
        ('…', 0x75),
        ('—', 0x7a),
        ('–', 0x7a),
        ('┌', 0x79),
        ('─', 0x7a),
        ('┐', 0x7b),
        ('│', 0x7c),
        ('└', 0x7d),
        ('┘', 0x7e),
        ('#', 0x54),
        ('_', 0x5f),
        ('=', 0x3d),
        ('+', 0x2b),
        ('"', 0x73),
    ] {
        map.insert(ch, tile_id);
    }
    for (ch, tile_id) in [
        ('\u{e100}', 0x4a), // <PKMN>
        ('\u{e101}', 0x5b), // <PC>
        ('\u{e102}', 0x5c), // <TM>
        ('\u{e103}', 0x5d), // <TRAINER>
        ('\u{e104}', 0x5e), // <ROCKET>
        ('\u{e105}', 0xe1), // <PK>
        ('\u{e106}', 0xe2), // <MN>
        ('\u{e107}', 0xf2), // <DOT>
        ('\u{e108}', 0x70), // <PO>
        ('\u{e109}', 0x71), // <KE>
        ('\u{e10a}', 0x6e), // <LV>
        ('\u{e10b}', 0x73), // <ID>
        ('\u{e120}', 0xd0), // 'd
        ('\u{e121}', 0xd1), // 'l
        ('\u{e122}', 0xd2), // 'm
        ('\u{e123}', 0xd3), // 'r
        ('\u{e124}', 0xd4), // 's
        ('\u{e125}', 0xd5), // 't
        ('\u{e126}', 0xd6), // 'v
    ] {
        map.insert(ch, tile_id);
    }
    map
}

/// Exact control-token normalization from TypeScript's
/// `ui/text/constants.ts`.  Private-use characters are intentional: they
/// preserve one-tile control glyphs until the bitmap font lookup.
fn normalize_bitmap_font_text(text: &str) -> String {
    [
        ("<TRAINER>", "\u{e103}"),
        ("<ROCKET>", "\u{e104}"),
        ("<PKMN>", "\u{e105}\u{e106}"),
        ("<POKE>", "#"),
        ("<PC>", "\u{e101}"),
        ("<TM>", "\u{e102}"),
        ("<PK>", "\u{e105}"),
        ("<MN>", "\u{e106}"),
        ("<DOT>", "\u{e107}"),
        ("<PO>", "\u{e108}"),
        ("<KE>", "\u{e109}"),
        ("<LV>", "\u{e10a}"),
        ("<ID>", "\u{e10b}"),
        ("<……>", "……"),
    ]
    .into_iter()
    .fold(text.to_string(), |normalized, (token, replacement)| {
        normalized.replace(token, replacement)
    })
    .replace('#', "POKé")
}

type Palette = [[u8; 3]; 4];

fn compose_viewport_tiles(
    tile_handles: &[Handle<Image>],
    existing: Option<Handle<Image>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    compose_tile_grid(
        tile_handles,
        VIEWPORT_TILES_X as usize,
        VIEWPORT_TILES_Y as usize,
        existing,
        images,
    )
}

fn compose_tile_grid(
    tile_handles: &[Handle<Image>],
    grid_width: usize,
    grid_height: usize,
    existing: Option<Handle<Image>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    // Keep the retained LCD at its native 160x144 resolution. The sprite is
    // displayed at PLAYFIELD_WIDTH x PLAYFIELD_HEIGHT with nearest sampling,
    // so expanding every source texel 4x4 on the CPU only creates sixteen
    // times more composition and upload work without changing a visible pixel.
    let width = grid_width * SOURCE_TILE_SIZE;
    let height = grid_height * SOURCE_TILE_SIZE;
    let data_len = width * height * 4;
    let mut data = existing
        .as_ref()
        .and_then(|handle| images.get_mut(handle))
        .map(|image| std::mem::take(&mut image.data))
        .unwrap_or_default();
    data.resize(data_len, 0);
    data.fill(0);
    for (tile_index, handle) in tile_handles.iter().enumerate() {
        let Some(tile) = images.get(handle) else {
            continue;
        };
        let tile_x = (tile_index % grid_width) * SOURCE_TILE_SIZE;
        let tile_y = (tile_index / grid_width) * SOURCE_TILE_SIZE;
        for source_y in 0..SOURCE_TILE_SIZE {
            let source_row_start = source_y * SOURCE_TILE_SIZE * 4;
            let source_row_end = source_row_start + SOURCE_TILE_SIZE * 4;
            if source_row_end > tile.data.len() {
                continue;
            }

            let destination_offset = ((tile_y + source_y) * width + tile_x) * 4;
            data[destination_offset..destination_offset + SOURCE_TILE_SIZE * 4]
                .copy_from_slice(&tile.data[source_row_start..source_row_end]);
        }
    }
    if let Some(handle) = existing {
        if let Some(image) = images.get_mut(&handle) {
            image.data = data;
            image.sampler = ImageSampler::nearest();
            return handle;
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
    images.add(image)
}

#[cfg(feature = "voxel-view")]
fn compose_visual_world_tiles(
    tile_handles: &[Handle<Image>],
    grid_width: usize,
    grid_height: usize,
    existing: Option<Handle<Image>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let width = grid_width * SOURCE_TILE_SIZE;
    let height = grid_height * SOURCE_TILE_SIZE;
    let data_len = width * height * 4;
    let mut data = existing
        .as_ref()
        .and_then(|handle| images.get_mut(handle))
        .filter(|image| {
            image.texture_descriptor.size.width == width as u32
                && image.texture_descriptor.size.height == height as u32
        })
        .map(|image| std::mem::take(&mut image.data))
        .unwrap_or_default();
    data.resize(data_len, 0);
    data.fill(0);
    for (tile_index, handle) in tile_handles.iter().enumerate() {
        let Some(tile) = images.get(handle) else {
            continue;
        };
        let tile_x = (tile_index % grid_width) * SOURCE_TILE_SIZE;
        let tile_y = (tile_index / grid_width) * SOURCE_TILE_SIZE;
        for source_y in 0..SOURCE_TILE_SIZE {
            let source_start = source_y * SOURCE_TILE_SIZE * 4;
            let source_end = source_start + SOURCE_TILE_SIZE * 4;
            if source_end > tile.data.len() {
                continue;
            }
            let destination = ((tile_y + source_y) * width + tile_x) * 4;
            data[destination..destination + SOURCE_TILE_SIZE * 4]
                .copy_from_slice(&tile.data[source_start..source_end]);
        }
    }
    if let Some(handle) = existing {
        if let Some(image) = images.get_mut(&handle)
            && image.texture_descriptor.size.width == width as u32
            && image.texture_descriptor.size.height == height as u32
        {
            image.data = data;
            image.sampler = ImageSampler::nearest();
            return handle;
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
    images.add(image)
}

fn compose_priority_viewport_tiles(
    tile_specs: &[Option<(Handle<Image>, usize)>],
    existing: Option<Handle<Image>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    compose_priority_tile_grid(
        tile_specs,
        VIEWPORT_TILES_X as usize,
        VIEWPORT_TILES_Y as usize,
        existing,
        images,
    )
}

fn compose_priority_tile_grid(
    tile_specs: &[Option<(Handle<Image>, usize)>],
    grid_width: usize,
    grid_height: usize,
    existing: Option<Handle<Image>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let width = grid_width * SOURCE_TILE_SIZE;
    let height = grid_height * SOURCE_TILE_SIZE;
    let data_len = width * height * 4;
    let mut data = existing
        .as_ref()
        .and_then(|handle| images.get_mut(handle))
        .map(|image| std::mem::take(&mut image.data))
        .unwrap_or_default();
    data.resize(data_len, 0);
    data.fill(0);
    for (tile_index, spec) in tile_specs.iter().enumerate() {
        let Some((handle, clip_top)) = spec else {
            continue;
        };
        let Some(tile) = images.get(handle) else {
            continue;
        };
        let tile_x = (tile_index % grid_width) * SOURCE_TILE_SIZE;
        let tile_y = (tile_index / grid_width) * SOURCE_TILE_SIZE;
        for source_y in (*clip_top).min(SOURCE_TILE_SIZE)..SOURCE_TILE_SIZE {
            let source_row_start = source_y * SOURCE_TILE_SIZE * 4;
            let source_row_end = source_row_start + SOURCE_TILE_SIZE * 4;
            if source_row_end > tile.data.len() {
                continue;
            }
            let destination_offset = ((tile_y + source_y) * width + tile_x) * 4;
            data[destination_offset..destination_offset + SOURCE_TILE_SIZE * 4]
                .copy_from_slice(&tile.data[source_row_start..source_row_end]);
        }
    }
    if let Some(handle) = existing {
        if let Some(image) = images.get_mut(&handle) {
            image.data = data;
            image.sampler = ImageSampler::nearest();
            return handle;
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
    images.add(image)
}

fn load_tileset_art(
    asset_root: &AssetRoot,
    tileset_id: &str,
    time_of_day: &str,
    palette_map: &[u8],
    images: &mut Assets<Image>,
) -> Result<TilesetArt> {
    let runtime_assets = asset_root.runtime_assets();
    let metatile_path = runtime_assets
        .join("data/tilesets")
        .join(format!("{tileset_id}_metatiles.bin"));
    let image_path = runtime_assets
        .join("gfx/tilesets")
        .join(format!("{tileset_id}.png"));
    let metatile_layout = crate::read_runtime_asset(&metatile_path)
        .with_context(|| format!("read tileset metatile layout {}", metatile_path.display()))?;
    if metatile_layout.len() % METATILE_TILE_COUNT != 0 {
        anyhow::bail!(
            "tileset {} metatile layout length {} is not divisible by {}",
            tileset_id,
            metatile_layout.len(),
            METATILE_TILE_COUNT
        );
    }
    let mut source = crate::open_runtime_image(&image_path)
        .with_context(|| format!("decode tileset PNG {}", image_path.display()))?
        .to_rgba8();
    if tileset_id == "battle_tower_outside" {
        apply_battle_tower_outside_roof(&runtime_assets, &mut source)?;
    }
    let (width, height) = source.dimensions();
    if width % SOURCE_TILE_SIZE as u32 != 0 || height % SOURCE_TILE_SIZE as u32 != 0 {
        anyhow::bail!(
            "tileset {} PNG has invalid dimensions {}x{}",
            tileset_id,
            width,
            height
        );
    }
    let source_tile_count =
        (width as usize / SOURCE_TILE_SIZE) * (height as usize / SOURCE_TILE_SIZE);
    let palette_bank = load_tileset_palette_bank(asset_root, tileset_id, time_of_day)?;
    let renderable_tile_count = source_tile_count.max(palette_map.len()).max(1);
    let mut tile_handles = Vec::with_capacity(renderable_tile_count);
    let mut priority_tile_handles = Vec::with_capacity(renderable_tile_count);
    for tile_index in 0..renderable_tile_count {
        let palette_value = palette_map.get(tile_index).copied().unwrap_or(0);
        let palette_index = usize::from(palette_value & 0x07);
        let vram_bank = (palette_value >> 3) & 0x01;
        let source_tile_index =
            resolve_tileset_tile_index(source_tile_count, tile_index, vram_bank);
        let palette = palette_bank
            .as_ref()
            .and_then(|bank| bank.get(palette_index).or_else(|| bank.first()));
        let mut data = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
        copy_source_tile_rgba(
            &source,
            width as usize,
            source_tile_index,
            palette,
            &mut data,
        );
        let mut priority_data = data.clone();
        clear_source_tile_palette_zero_alpha(
            &source,
            width as usize,
            source_tile_index,
            &mut priority_data,
        );
        let mut image = Image::new(
            Extent3d {
                width: SOURCE_TILE_SIZE as u32,
                height: SOURCE_TILE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        tile_handles.push(images.add(image));
        let mut priority_image = Image::new(
            Extent3d {
                width: SOURCE_TILE_SIZE as u32,
                height: SOURCE_TILE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            priority_data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        priority_image.sampler = ImageSampler::nearest();
        priority_tile_handles.push(images.add(priority_image));
    }
    let animated_tiles = load_common_tileset_animations(
        asset_root,
        tileset_id,
        palette_map,
        palette_bank.as_ref(),
        &tile_handles,
        images,
    )?;
    Ok(TilesetArt {
        metatile_layout,
        tile_handles,
        priority_tile_handles,
        animated_tiles,
    })
}

fn apply_battle_tower_outside_roof(
    runtime_assets: &std::path::Path,
    source: &mut image::RgbaImage,
) -> Result<()> {
    const FIRST_ROOF_TILE: u32 = 0x0a;
    const ROOF_TILE_COUNT: u32 = 9;
    let roof_path = runtime_assets.join("gfx/tilesets/roofs/olivine.png");
    let roof = crate::open_runtime_image(&roof_path)
        .with_context(|| format!("decode Battle Tower map-group roof {}", roof_path.display()))?
        .to_rgba8();
    if roof.width() * roof.height()
        != ROOF_TILE_COUNT * SOURCE_TILE_SIZE as u32 * SOURCE_TILE_SIZE as u32
        || roof.width() % SOURCE_TILE_SIZE as u32 != 0
        || roof.height() % SOURCE_TILE_SIZE as u32 != 0
    {
        anyhow::bail!(
            "Battle Tower map-group roof {} must contain exactly {} aligned tiles",
            roof_path.display(),
            ROOF_TILE_COUNT,
        );
    }
    let source_tiles_wide = source.width() / SOURCE_TILE_SIZE as u32;
    let roof_tiles_wide = roof.width() / SOURCE_TILE_SIZE as u32;
    for tile in 0..ROOF_TILE_COUNT {
        let source_tile = FIRST_ROOF_TILE + tile;
        let source_x = (source_tile % source_tiles_wide) * SOURCE_TILE_SIZE as u32;
        let source_y = (source_tile / source_tiles_wide) * SOURCE_TILE_SIZE as u32;
        let roof_x = (tile % roof_tiles_wide) * SOURCE_TILE_SIZE as u32;
        let roof_y = (tile / roof_tiles_wide) * SOURCE_TILE_SIZE as u32;
        for y in 0..SOURCE_TILE_SIZE as u32 {
            for x in 0..SOURCE_TILE_SIZE as u32 {
                source.put_pixel(
                    source_x + x,
                    source_y + y,
                    *roof.get_pixel(roof_x + x, roof_y + y),
                );
            }
        }
    }
    Ok(())
}

fn load_common_tileset_animations(
    asset_root: &AssetRoot,
    tileset_id: &str,
    palette_map: &[u8],
    palette_bank: Option<&Vec<Palette>>,
    tile_handles: &[Handle<Image>],
    images: &mut Assets<Image>,
) -> Result<HashMap<usize, TilesetAnimatedTile>> {
    const FLOWER_TILE: usize = 0x03;
    const WATER_TILE: usize = 0x14;
    const FLOWER_TILESETS: [&str; 5] = ["johto", "johto_modern", "kanto", "park", "forest"];
    const WATER_TILESETS: [&str; 9] = [
        "johto", "johto_modern", "kanto", "park", "forest", "port", "cave",
        "dark_cave", "ice_path",
    ];
    let mut animated = HashMap::new();
    let root = asset_root.runtime_assets().join("gfx/tilesets");
    if FLOWER_TILESETS.contains(&tileset_id) {
        let palette = tileset_animation_palette(FLOWER_TILE, palette_map, palette_bank);
        let mut frames = Vec::with_capacity(2);
        for name in ["cgb_1.2bpp", "cgb_2.2bpp"] {
            frames.extend(load_2bpp_animation_frames(
                &root.join("flower").join(name),
                1,
                palette,
                images,
            )?);
        }
        animated.insert(FLOWER_TILE, sequential_tileset_animation(frames, 60, 0));
    }
    if WATER_TILESETS.contains(&tileset_id) {
        let palette = tileset_animation_palette(WATER_TILE, palette_map, palette_bank);
        let frames = load_2bpp_animation_frames(
            &root.join("water/water.2bpp"),
            4,
            palette,
            images,
        )?;
        // ASM/TypeScript: ((wTileAnimationTimer / 11) >> 1) & 3.
        animated.insert(WATER_TILE, sequential_tileset_animation(frames, 22, 0));
    }
    if tileset_id == "park" {
        const FOUNTAIN_TILE: usize = 0x5f;
        let palette = tileset_animation_palette(FOUNTAIN_TILE, palette_map, palette_bank);
        let source = load_numbered_2bpp_animation_frames(
            &root.join("fountain"), 5, palette, images,
        )?;
        let frames = [0_usize, 1, 2, 3, 2, 3, 4, 0]
            .into_iter()
            .map(|index| source[index].clone())
            .collect();
        animated.insert(FOUNTAIN_TILE, sequential_tileset_animation(frames, 11, 0));
    }
    if tileset_id == "elite_four_room" {
        const LAVA_TILE_1: usize = 0x5b;
        const LAVA_TILE_2: usize = 0x38;
        let source_1 = load_numbered_2bpp_animation_frames(
            &root.join("lava"),
            4,
            tileset_animation_palette(LAVA_TILE_1, palette_map, palette_bank),
            images,
        )?;
        let source_2 = load_numbered_2bpp_animation_frames(
            &root.join("lava"),
            4,
            tileset_animation_palette(LAVA_TILE_2, palette_map, palette_bank),
            images,
        )?;
        let offset_frames = [2_usize, 3, 0, 1]
            .into_iter()
            .map(|index| source_1[index].clone())
            .collect();
        animated.insert(
            LAVA_TILE_1,
            sequential_tileset_animation(offset_frames, 2, 0),
        );
        animated.insert(
            LAVA_TILE_2,
            sequential_tileset_animation(source_2, 2, 0),
        );
    }
    if tileset_id == "tower" {
        const DESTINATIONS: [usize; 10] = [
            0x2d, 0x2f, 0x3d, 0x3f, 0x3c, 0x2c, 0x4d, 0x4f, 0x5d, 0x5f,
        ];
        const UPDATE_SEQUENCE: [usize; 10] = [
            0x5d, 0x5f, 0x4d, 0x4f, 0x3c, 0x2c, 0x3d, 0x3f, 0x2d, 0x2f,
        ];
        const FRAME_ORDER: [usize; 8] = [0, 1, 2, 3, 4, 3, 2, 1];
        for (file_index, destination) in DESTINATIONS.into_iter().enumerate() {
            let source = load_2bpp_animation_frames(
                &root.join("tower-pillar").join(format!("{}.2bpp", file_index + 1)),
                5,
                tileset_animation_palette(destination, palette_map, palette_bank),
                images,
            )?;
            let frames = FRAME_ORDER
                .into_iter()
                .map(|index| source[index].clone())
                .collect();
            let phase_offset = UPDATE_SEQUENCE
                .iter()
                .position(|candidate| *candidate == destination)
                .with_context(|| {
                    format!("tower animation destination ${destination:02x} has no command slot")
                })? as u64;
            animated.insert(
                destination,
                sequential_tileset_animation(frames, 16, phase_offset),
            );
        }
    }
    if tileset_id == "forest" {
        const LEFT_TILE: usize = 0x0c;
        const RIGHT_TILE: usize = 0x0f;
        let directory = root.join("forest-tree");
        let left_frames = load_numbered_2bpp_animation_frames(
            &directory,
            2,
            tileset_animation_palette(LEFT_TILE, palette_map, palette_bank),
            images,
        )?;
        let mut right_frames = Vec::with_capacity(2);
        for index in 3..=4 {
            right_frames.extend(load_2bpp_animation_frames(
                &directory.join(format!("{index}.2bpp")),
                1,
                tileset_animation_palette(RIGHT_TILE, palette_map, palette_bank),
                images,
            )?);
        }
        animated.insert(
            LEFT_TILE,
            TilesetAnimatedTile {
                frames: left_frames,
                frame_ticks: 1,
                phase_offset: 0,
                requires_forest_restless: true,
                cave_water_composite: false,
                advance_on_phase_offset: false,
                additional_schedule: Vec::new(),
            },
        );
        animated.insert(
            RIGHT_TILE,
            TilesetAnimatedTile {
                frames: right_frames,
                frame_ticks: 1,
                phase_offset: 0,
                requires_forest_restless: true,
                cave_water_composite: false,
                advance_on_phase_offset: false,
                additional_schedule: Vec::new(),
            },
        );
    }
    if matches!(tileset_id, "cave" | "dark_cave") {
        const HORIZONTAL_TILE: usize = 0x14;
        const VERTICAL_TILE: usize = 0x40;
        let water_sources = animated
            .get(&HORIZONTAL_TILE)
            .map(|animation| animation.frames.clone())
            .context("cave scroll animation requires the four water frames")?;
        let mut composite_frames = Vec::with_capacity(32);
        for source in &water_sources {
            for shift in 0..8 {
                composite_frames.push(shifted_tileset_tile_handle(
                    source, shift, 0, images,
                )?);
            }
        }
        animated.insert(
            HORIZONTAL_TILE,
            TilesetAnimatedTile {
                frames: composite_frames,
                frame_ticks: 22,
                phase_offset: 0,
                requires_forest_restless: false,
                cave_water_composite: true,
                advance_on_phase_offset: false,
                additional_schedule: vec![(19, 4)],
            },
        );
        let vertical_source = tile_handles
            .get(VERTICAL_TILE)
            .context("cave waterfall animation requires tile $40")?;
        animated.insert(
            VERTICAL_TILE,
            scrolled_tileset_animation(vertical_source, false, 3, 19, 16, images)?,
        );
    }
    if tileset_id == "ice_path" {
        const HORIZONTAL_VISIBLE_TILE: usize = 0xb5;
        const VERTICAL_VISIBLE_TILE: usize = 0xb1;
        let horizontal_source = tile_handles
            .get(HORIZONTAL_VISIBLE_TILE)
            .context("Ice Path water animation requires visible tile $b5")?;
        let vertical_source = tile_handles
            .get(VERTICAL_VISIBLE_TILE)
            .context("Ice Path waterfall animation requires visible tile $b1")?;
        animated.insert(
            HORIZONTAL_VISIBLE_TILE,
            scrolled_tileset_animation(horizontal_source, true, 1, 19, 4, images)?,
        );
        animated.insert(
            VERTICAL_VISIBLE_TILE,
            scrolled_tileset_animation(vertical_source, false, 3, 19, 16, images)?,
        );
    }
    Ok(animated)
}

fn sequential_tileset_animation(
    frames: Vec<Handle<Image>>,
    frame_ticks: u64,
    phase_offset: u64,
) -> TilesetAnimatedTile {
    TilesetAnimatedTile {
        frames,
        frame_ticks,
        phase_offset,
        requires_forest_restless: false,
        cave_water_composite: false,
        advance_on_phase_offset: false,
        additional_schedule: Vec::new(),
    }
}

fn scrolled_tileset_animation(
    source: &Handle<Image>,
    horizontal: bool,
    pixels_per_step: usize,
    frame_ticks: u64,
    phase_offset: u64,
    images: &mut Assets<Image>,
) -> Result<TilesetAnimatedTile> {
    let mut frames = Vec::with_capacity(8);
    for stage in 0..8 {
        let shift = (stage * pixels_per_step) % 8;
        frames.push(shifted_tileset_tile_handle(
            source,
            if horizontal { shift } else { 0 },
            if horizontal { 0 } else { shift },
            images,
        )?);
    }
    Ok(TilesetAnimatedTile {
        frames,
        frame_ticks,
        phase_offset,
        requires_forest_restless: false,
        cave_water_composite: false,
        advance_on_phase_offset: true,
        additional_schedule: Vec::new(),
    })
}

fn shifted_tileset_tile_handle(
    source: &Handle<Image>,
    left_pixels: usize,
    down_pixels: usize,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>> {
    let source_data = images
        .get(source)
        .context("tileset scroll animation source image is unavailable")?
        .data
        .clone();
    if source_data.len() != SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4 {
        anyhow::bail!(
            "tileset scroll animation source must be one 8x8 RGBA tile, found {} bytes",
            source_data.len()
        );
    }
    let mut shifted = vec![0_u8; source_data.len()];
    for y in 0..SOURCE_TILE_SIZE {
        for x in 0..SOURCE_TILE_SIZE {
            let source_x = (x + left_pixels) % SOURCE_TILE_SIZE;
            let source_y = (y + SOURCE_TILE_SIZE - (down_pixels % SOURCE_TILE_SIZE))
                % SOURCE_TILE_SIZE;
            let source_offset = (source_y * SOURCE_TILE_SIZE + source_x) * 4;
            let target_offset = (y * SOURCE_TILE_SIZE + x) * 4;
            shifted[target_offset..target_offset + 4]
                .copy_from_slice(&source_data[source_offset..source_offset + 4]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: SOURCE_TILE_SIZE as u32,
            height: SOURCE_TILE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        shifted,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(images.add(image))
}

fn load_numbered_2bpp_animation_frames(
    directory: &std::path::Path,
    frame_count: usize,
    palette: Option<&Palette>,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut frames = Vec::with_capacity(frame_count);
    for index in 1..=frame_count {
        frames.extend(load_2bpp_animation_frames(
            &directory.join(format!("{index}.2bpp")),
            1,
            palette,
            images,
        )?);
    }
    Ok(frames)
}

fn tileset_animation_palette<'a>(
    tile_index: usize,
    palette_map: &[u8],
    palette_bank: Option<&'a Vec<Palette>>,
) -> Option<&'a Palette> {
    let palette_index = usize::from(palette_map.get(tile_index).copied().unwrap_or(0) & 0x07);
    palette_bank.and_then(|bank| bank.get(palette_index).or_else(|| bank.first()))
}

fn load_2bpp_animation_frames(
    path: &std::path::Path,
    frame_count: usize,
    palette: Option<&Palette>,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let bytes = crate::read_runtime_asset(path)
        .with_context(|| format!("read tileset animation {}", path.display()))?;
    let expected = frame_count * SOURCE_TILE_SIZE * 2;
    if bytes.len() != expected {
        anyhow::bail!(
            "tileset animation {} must contain {} bytes ({} frames), found {}",
            path.display(), expected, frame_count, bytes.len()
        );
    }
    let mut frames = Vec::with_capacity(frame_count);
    for frame_index in 0..frame_count {
        let frame = &bytes[frame_index * 16..frame_index * 16 + 16];
        let mut rgba = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
        for row in 0..SOURCE_TILE_SIZE {
            let lo = frame[row * 2];
            let hi = frame[row * 2 + 1];
            for col in 0..SOURCE_TILE_SIZE {
                let bit = 1 << (7 - col);
                let color_index = (((hi & bit != 0) as usize) << 1)
                    | (lo & bit != 0) as usize;
                let color = palette
                    .map(|colors| colors[color_index])
                    .unwrap_or_else(|| {
                        let gray = [255_u8, 170, 85, 0][color_index];
                        [gray, gray, gray]
                    });
                let offset = (row * SOURCE_TILE_SIZE + col) * 4;
                rgba[offset..offset + 3].copy_from_slice(&color);
                rgba[offset + 3] = 255;
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: SOURCE_TILE_SIZE as u32,
                height: SOURCE_TILE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        frames.push(images.add(image));
    }
    Ok(frames)
}

fn copy_source_tile_rgba(
    source: &image::RgbaImage,
    source_width: usize,
    tile_index: usize,
    palette: Option<&Palette>,
    target: &mut [u8],
) {
    let tiles_per_row = (source_width / SOURCE_TILE_SIZE).max(1);
    let source_x = (tile_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (tile_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        for col in 0..SOURCE_TILE_SIZE {
            let source_pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            let offset = (row * SOURCE_TILE_SIZE + col) * 4;
            let alpha = source_pixel[3];
            if let Some(palette) = palette {
                // Exported PNG alpha represents Game Boy colour zero. BG
                // pixels are never transparent on hardware, so the base map
                // layer must paint palette[0]; only the separately composed
                // priority/OBJ layer clears colour zero below.
                let palette_index = if alpha == 0 {
                    0
                } else {
                    palette_index_from_gray(source_pixel[0])
                };
                let [red, green, blue] = palette[palette_index];
                target[offset] = red;
                target[offset + 1] = green;
                target[offset + 2] = blue;
                target[offset + 3] = 255;
            } else {
                // Some source tilesets use their already-coloured PNG
                // directly and therefore have no separate runtime palette
                // bank. PNG alpha still marks Game Boy colour zero in that
                // path; it is not transparency for a BG tile. Preserve the
                // exported RGB value and make every base-layer pixel opaque.
                target[offset] = source_pixel[0];
                target[offset + 1] = source_pixel[1];
                target[offset + 2] = source_pixel[2];
                target[offset + 3] = 255;
            }
        }
    }
}

fn clear_source_tile_palette_zero_alpha(
    source: &image::RgbaImage,
    source_width: usize,
    tile_index: usize,
    target: &mut [u8],
) {
    let tiles_per_row = (source_width / SOURCE_TILE_SIZE).max(1);
    let source_x = (tile_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (tile_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        for col in 0..SOURCE_TILE_SIZE {
            let source_pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            if source_pixel[3] == 0 || palette_index_from_gray(source_pixel[0]) == 0 {
                target[(row * SOURCE_TILE_SIZE + col) * 4 + 3] = 0;
            }
        }
    }
}

fn resolve_tileset_tile_index(source_tile_count: usize, tile_index: usize, vram_bank: u8) -> usize {
    if source_tile_count == 0 {
        return 0;
    }
    if vram_bank == 1 && source_tile_count % 2 == 0 {
        let half = source_tile_count / 2;
        let candidate = (tile_index & 0x7f) + half;
        if candidate < source_tile_count {
            return candidate;
        }
    }
    if vram_bank == 1 {
        let bank_one_tile = (tile_index & 0x7f) + 0x80;
        if bank_one_tile < source_tile_count {
            return bank_one_tile;
        }
    }
    if tile_index < source_tile_count {
        return tile_index;
    }
    if tile_index >= 0x80 {
        let mirrored = tile_index - 0x80;
        if mirrored < source_tile_count {
            return mirrored;
        }
    }
    0
}

fn load_tileset_palette_bank(
    asset_root: &AssetRoot,
    tileset_id: &str,
    time_of_day: &str,
) -> Result<Option<Vec<Palette>>> {
    let runtime_assets = asset_root.runtime_assets();
    let tileset_palette_path = runtime_assets
        .join("gfx/tilesets")
        .join(format!("{tileset_id}.pal"));
    if crate::runtime_asset_exists(&tileset_palette_path) {
        let content = crate::read_runtime_asset_to_string(&tileset_palette_path)
            .with_context(|| format!("read tileset palette {}", tileset_palette_path.display()))?;
        let palettes = parse_palette_file(&content, None)?;
        if !palettes.is_empty() {
            return Ok(Some(palettes.into_iter().take(8).collect()));
        }
    }
    let bg_palette_path = runtime_assets.join("gfx/tilesets/bg_tiles.pal");
    if !crate::runtime_asset_exists(&bg_palette_path) {
        return Ok(None);
    }
    let content = crate::read_runtime_asset_to_string(&bg_palette_path)
        .with_context(|| format!("read bg tiles palette {}", bg_palette_path.display()))?;
    let normalized = normalize_tileset_time_of_day(time_of_day);
    for group in [normalized.as_str(), "day", "morn", "indoor"] {
        let palettes = parse_palette_file(&content, Some(group))?;
        if !palettes.is_empty() {
            return Ok(Some(palettes.into_iter().take(8).collect()));
        }
    }
    Ok(None)
}

fn title_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    asset_id: &str,
    palette_id: u8,
    transparent_zero: bool,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let key = TitleArtKey {
        asset_id: asset_id.to_string(),
        palette_id,
        transparent_zero,
    };
    if !rendered_art.title_cache.contains_key(&key) {
        match load_title_frame(asset_root, asset_id, palette_id, transparent_zero, images) {
            Ok(frame) => {
                rendered_art.title_errors.remove(&key);
                rendered_art.title_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .title_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art.title_cache.get(&key).cloned()
}

fn title_screen_art_key(title: &TitleMenu) -> TitleScreenArtKey {
    let animation_frame = match title.phase {
        // Entrance crystal placement changes continuously and SCX identifies
        // each finite scroll step, so preserve its native frame counter.
        VisibleTitlePhase::Entrance => title.frame,
        // Once settled, the only moving title art is Suicune's four-frame
        // loop. Bound the source cache to those four images instead of
        // retaining a new 160x144 asset every eight ticks until timeout.
        _ => ((title.frame / 8) % 4) * 8,
    };
    TitleScreenArtKey {
        scx: title.scx,
        frame: animation_frame,
        show_version_window: !matches!(title.phase, VisibleTitlePhase::Entrance),
    }
}

fn title_screen_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    title: &TitleMenu,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let key = title_screen_art_key(title);
    if !rendered_art.title_screen_cache.contains_key(&key) {
        match load_title_screen_frame(asset_root, title, images) {
            Ok(frame) => {
                rendered_art.title_screen_errors.remove(&key);
                rendered_art.title_screen_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .title_screen_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art.title_screen_cache.get(&key).cloned()
}

/// Key the persistent intro surface by the entire visible LCD state.  The
/// texture is updated in place, so retaining the exact frame does not require
/// creating one GPU image per Game Boy frame.
fn intro_scene_art_key(intro: &VisibleIntroScreen) -> IntroSceneArtKey {
    let mut sprite_hasher = std::collections::hash_map::DefaultHasher::new();
    intro.sprites.hash(&mut sprite_hasher);
    IntroSceneArtKey {
        scene_index: intro.jumptable_index,
        scene_frame_counter: intro.scene_frame_counter,
        scene_timer: intro.scene_timer,
        scroll_x: intro.scroll_x,
        scroll_y: intro.scroll_y,
        global_anim_x_offset: intro.global_anim_x_offset,
        sprite_hash: sprite_hasher.finish(),
        palette_effect: intro.palette_effect,
    }
}

fn intro_scene_frame_for_art_with_bundle(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    sprite_anim_bundle: &str,
    intro: &VisibleIntroScreen,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    intro_renderer::compose_frame(rendered_art, asset_root, sprite_anim_bundle, intro, images)
}

#[cfg(test)]
fn intro_scene_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    intro: &VisibleIntroScreen,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let path = asset_root
        .runtime_assets()
        .join("data/sprite_anim_bundle.json");
    let sprite_anim_bundle = crate::read_runtime_asset_to_string(&path).ok()?;
    intro_scene_frame_for_art_with_bundle(
        rendered_art,
        asset_root,
        &sprite_anim_bundle,
        intro,
        images,
    )
}

fn load_intro_scene_frame(
    asset_root: &AssetRoot,
    sprite_anim_bundle: &str,
    intro: &VisibleIntroScreen,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const INTRO_SURFACE_TILES: usize = 32;
    const INTRO_SURFACE_SIZE: usize = INTRO_SURFACE_TILES * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; INTRO_SURFACE_SIZE * INTRO_SURFACE_SIZE * 4];
    ensure_intro_effect_palette_banks(asset_root, rendered_art)?;
    if let Some(map_name) = visible_intro_bg_tilemap_name(intro.jumptable_index) {
        draw_intro_tilemap(
            asset_root,
            intro,
            map_name,
            intro.scroll_x,
            intro.scroll_y,
            rendered_art,
            &mut data,
        )?;
    }
    if intro.jumptable_index == 9 {
        draw_intro_grass_rustle(
            asset_root,
            intro,
            intro.scene_frame_counter,
            rendered_art,
            &mut data,
        )?;
    }
    draw_visible_intro_sprites(
        sprite_anim_bundle,
        asset_root,
        intro,
        rendered_art,
        &mut data,
    )?;
    // The intro uses a 32x32 BG map as its backing store, but the LCD exposes
    // only its top-left 20x18-tile viewport.  Never present the backing store:
    // doing so leaks the offscreen rows/columns and OAM staging area.
    let data = crop_intro_lcd_viewport(&data);
    let mut image = Image::new(
        Extent3d {
            width: TITLE_SCREEN_WIDTH as u32,
            height: TITLE_SCREEN_HEIGHT as u32,
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
        size: Vec2::new(TITLE_SCREEN_WIDTH as f32, TITLE_SCREEN_HEIGHT as f32),
    })
}

fn ensure_intro_effect_palette_banks(
    asset_root: &AssetRoot,
    rendered_art: &mut RenderedTilesetArt,
) -> Result<()> {
    let intro_root = asset_root.runtime_assets().join("gfx/intro");
    for (palette_name, transparent_zero) in [
        ("unowns", false),
        ("unown_1", true),
        ("unown_2", true),
        ("fade", false),
    ] {
        let key = format!("{palette_name}:{transparent_zero}");
        if !rendered_art.intro_palette_cache.contains_key(&key) {
            rendered_art.intro_palette_cache.insert(
                key,
                load_intro_palette_bank_for_kind(&intro_root, palette_name, transparent_zero)?,
            );
        }
    }
    Ok(())
}

fn crop_intro_lcd_viewport(backing: &[u8]) -> Vec<u8> {
    const INTRO_BACKING_WIDTH: usize = 32 * SOURCE_TILE_SIZE;
    let mut viewport = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];
    for row in 0..TITLE_SCREEN_HEIGHT {
        let source_start = row * INTRO_BACKING_WIDTH * 4;
        let target_start = row * TITLE_SCREEN_WIDTH * 4;
        viewport[target_start..target_start + TITLE_SCREEN_WIDTH * 4]
            .copy_from_slice(&backing[source_start..source_start + TITLE_SCREEN_WIDTH * 4]);
    }
    viewport
}

fn visible_intro_bg_tilemap_name(scene_index: usize) -> Option<&'static str> {
    match scene_index {
        0 | 1 => Some("unown_a"),
        2 | 3 | 6 | 7 | 8 | 9 | 12 | 13 => Some("background"),
        4 | 5 => Some("unown_hi"),
        10 | 11 => Some("unowns"),
        14 | 15 => Some("suicune_jump"),
        16 | 17 => Some("suicune_close"),
        18 | 19 | 20 | 21 | 22 | 23 | 24 => Some("suicune_back"),
        25 | 26 | 27 => Some("crystal_unowns"),
        _ => None,
    }
}

fn draw_intro_tilemap(
    asset_root: &AssetRoot,
    intro: &VisibleIntroScreen,
    map_name: &str,
    scroll_x: u8,
    scroll_y: u8,
    rendered_art: &mut RenderedTilesetArt,
    target: &mut [u8],
) -> Result<()> {
    const INTRO_SURFACE_TILES: usize = 32;
    let intro_root = asset_root.runtime_assets().join("gfx/intro");
    let tilemap_path = intro_root.join(format!("{map_name}.tilemap"));
    let attrmap_path = intro_root.join(format!("{map_name}.attrmap"));
    let tilemap =
        crate::read_runtime_asset(&tilemap_path).with_context(|| format!("read {}", tilemap_path.display()))?;
    let attrmap =
        crate::read_runtime_asset(&attrmap_path).with_context(|| format!("read {}", attrmap_path.display()))?;
    let tile_count = tilemap
        .len()
        .min(attrmap.len())
        .min(INTRO_SURFACE_TILES * INTRO_SURFACE_TILES);
    for tile_offset in 0..tile_count {
        let tile_x = tile_offset % INTRO_SURFACE_TILES;
        let tile_y = tile_offset / INTRO_SURFACE_TILES;
        let tile_id = tilemap[tile_offset];
        let attr = if intro.jumptable_index == 9 && map_name == "background" {
            let row = tile_y;
            let palette = if row < 12 {
                1
            } else if row < 15 {
                2
            } else {
                3
            };
            (attrmap[tile_offset] & !0x07) | palette
        } else {
            attrmap[tile_offset]
        };
        let mut graphic_name = visible_intro_graphic_name(map_name);
        let mut tile_shift = visible_intro_tile_shift(map_name);
        let mut tile_mode = visible_intro_tile_index_mode(map_name);
        let mut palette_name = visible_intro_palette_name(graphic_name);
        if map_name == "suicune_back" && tile_id >= 0x80 {
            graphic_name = "unowns";
            tile_shift = 0x80;
            tile_mode = IntroTileIndexMode::Offset;
            palette_name = "suicune";
        }
        draw_intro_tile(
            &intro_root,
            intro,
            graphic_name,
            palette_name,
            tile_id,
            attr,
            tile_shift,
            tile_mode,
            false,
            rendered_art,
            ((tile_x * SOURCE_TILE_SIZE) as i16 - i16::from(scroll_x)).rem_euclid(256) as usize,
            ((tile_y * SOURCE_TILE_SIZE) as i16 - i16::from(scroll_y)).rem_euclid(256) as usize,
            target,
        )?;
    }
    Ok(())
}

fn draw_intro_grass_rustle(
    asset_root: &AssetRoot,
    intro: &VisibleIntroScreen,
    scene_frame_counter: u8,
    rendered_art: &mut RenderedTilesetArt,
    target: &mut [u8],
) -> Result<()> {
    if scene_frame_counter >= 0x24 {
        return Ok(());
    }
    let frame = usize::from((scene_frame_counter & 0x0c) >> 2);
    let grass_name = ["grass1", "grass2", "grass3", "grass2"][frame];
    let intro_root = asset_root.runtime_assets().join("gfx/intro");
    for tile_index in 0..4_u8 {
        draw_intro_tile(
            &intro_root,
            intro,
            grass_name,
            "background",
            tile_index,
            2,
            0,
            IntroTileIndexMode::Offset,
            false,
            rendered_art,
            9 * SOURCE_TILE_SIZE + usize::from(tile_index) * SOURCE_TILE_SIZE,
            12 * SOURCE_TILE_SIZE,
            target,
        )?;
    }
    Ok(())
}

fn draw_visible_intro_sprites(
    sprite_anim_bundle: &str,
    asset_root: &AssetRoot,
    intro: &VisibleIntroScreen,
    rendered_art: &mut RenderedTilesetArt,
    target: &mut [u8],
) -> Result<()> {
    if rendered_art.intro_sprite_bundle_cache.is_none() {
        rendered_art.intro_sprite_bundle_cache =
            Some(load_intro_sprite_anim_bundle(sprite_anim_bundle)?);
    }
    let bundle = rendered_art
        .intro_sprite_bundle_cache
        .as_ref()
        .expect("intro sprite bundle cache initialized")
        .clone();
    let intro_root = asset_root.runtime_assets().join("gfx/intro");
    for sprite in &intro.sprites {
        if sprite.start_delay > 0 {
            continue;
        }
        let Some(oam_set_name) = visible_intro_sprite_oam_set(sprite, &bundle) else {
            continue;
        };
        let Some(oam_set) = bundle
            .get("oam_sets")
            .and_then(|sets| sets.get(oam_set_name.as_str()))
        else {
            anyhow::bail!(
                "intro OAM set {} missing from sprite_anim_bundle",
                oam_set_name
            );
        };
        let tile_offset = oam_set
            .get("tile_offset")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0) as i16;
        let Some(pieces) = oam_set.get("pieces").and_then(serde_json::Value::as_array) else {
            continue;
        };
        let frame_flags = sprite.attr_flags & 0xe0;
        for piece in pieces {
            let piece_x = piece
                .get("x")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i16;
            let piece_y = piece
                .get("y")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i16;
            let piece_tile = piece
                .get("tile")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i16;
            let piece_attr = piece
                .get("attributes")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as u8;
            let base_attr = piece_attr | sprite.oam_attr;
            let flipped_attr = (base_attr ^ frame_flags) & 0xe0;
            let attr = (base_attr & !0xe0) | flipped_attr;
            let offset_x = apply_visible_intro_frame_flip(piece_x, (frame_flags & 0x20) != 0);
            let offset_y = apply_visible_intro_frame_flip(piece_y, (frame_flags & 0x40) != 0);
            let global_x = i16::from(intro.global_anim_x_offset as i8);
            let draw_x = sprite.x + sprite.x_offset + global_x + offset_x - 8;
            let draw_y = sprite.y + sprite.y_offset + offset_y - 16;
            let tile_id = (piece_tile + tile_offset).rem_euclid(256) as u8;
            draw_intro_sprite_tile(
                &intro_root,
                intro,
                &sprite.gfx_name,
                visible_intro_palette_name(&sprite.gfx_name),
                tile_id,
                attr,
                0,
                IntroTileIndexMode::Offset,
                true,
                rendered_art,
                draw_x,
                draw_y,
                target,
            )?;
        }
    }
    Ok(())
}

/// OBJ tiles are composited directly onto the LCD in the TypeScript renderer.
/// Unlike the 32x32 BG tilemap, coordinates outside the 160x144 viewport must
/// be clipped, never wrapped onto the opposite side of the staging surface.
fn draw_intro_sprite_tile(
    intro_root: &Path,
    intro: &VisibleIntroScreen,
    graphic_name: &str,
    palette_name: &str,
    tile_id: u8,
    attr: u8,
    tile_shift: u8,
    tile_mode: IntroTileIndexMode,
    transparent_zero: bool,
    rendered_art: &mut RenderedTilesetArt,
    dest_x: i16,
    dest_y: i16,
    target: &mut [u8],
) -> Result<()> {
    let image_path = intro_root.join(format!("{graphic_name}.png"));
    let source_key = graphic_name.to_string();
    if !rendered_art.intro_source_cache.contains_key(&source_key) {
        let source = crate::open_runtime_image(&image_path)
            .with_context(|| format!("decode intro PNG {}", image_path.display()))?
            .to_rgba8();
        rendered_art
            .intro_source_cache
            .insert(source_key.clone(), source);
    }
    let source = rendered_art
        .intro_source_cache
        .get(&source_key)
        .expect("intro source cache initialized");
    let (width, height) = source.dimensions();
    if width % SOURCE_TILE_SIZE as u32 != 0 || height % SOURCE_TILE_SIZE as u32 != 0 {
        anyhow::bail!(
            "intro PNG {} has invalid tile dimensions {}x{}",
            image_path.display(),
            width,
            height
        );
    }
    let source_tile_count =
        (width as usize / SOURCE_TILE_SIZE) * (height as usize / SOURCE_TILE_SIZE);
    let palette_key = format!("{palette_name}:{transparent_zero}");
    if !rendered_art.intro_palette_cache.contains_key(&palette_key) {
        rendered_art.intro_palette_cache.insert(
            palette_key.clone(),
            load_intro_palette_bank_for_kind(intro_root, palette_name, transparent_zero)?,
        );
    }
    let palette_bank = rendered_art
        .intro_palette_cache
        .get(&palette_key)
        .expect("intro palette cache initialized");
    let palette_index = usize::from(attr & 0x07);
    let palette = palette_bank
        .get(palette_index)
        .or_else(|| palette_bank.first())
        .with_context(|| format!("intro palette {palette_name} is empty"))?;
    let palette_override = visible_intro_effective_palette_cached(
        intro,
        rendered_art,
        palette_name,
        palette_index,
        palette,
        true,
    )?;
    let source_tile_index =
        resolve_intro_tile_index(tile_id, tile_shift, tile_mode, source_tile_count);
    blit_intro_sprite_source_tile(
        source,
        width as usize,
        source_tile_index,
        &palette_override,
        transparent_zero,
        (attr & 0x20) != 0,
        (attr & 0x40) != 0,
        dest_x,
        dest_y,
        target,
    );
    Ok(())
}

fn load_intro_sprite_anim_bundle(sprite_anim_bundle: &str) -> Result<serde_json::Value> {
    serde_json::from_str(sprite_anim_bundle).context("parse packed sprite animation bundle")
}

fn visible_intro_sprite_oam_set(
    sprite: &VisibleIntroSprite,
    bundle: &serde_json::Value,
) -> Option<String> {
    if let Some(oam_set) = &sprite.current_oam_set {
        return Some(oam_set.clone());
    }
    let frame_index = usize::try_from(sprite.frameset_step.max(0)).ok()?;
    bundle
        .get("framesets")
        .and_then(|framesets| framesets.get(&sprite.frameset_name))
        .and_then(|frameset| frameset.get("steps"))
        .and_then(serde_json::Value::as_array)
        .and_then(|steps| steps.get(frame_index))
        .and_then(|step| step.get("oam_set"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn apply_visible_intro_frame_flip(offset: i16, flip: bool) -> i16 {
    if flip { -8 - offset } else { offset }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntroTileIndexMode {
    Offset,
    Signed,
}

fn visible_intro_graphic_name(map_name: &str) -> &str {
    match map_name {
        "unown_a" | "unown_hi" | "unowns" => "unowns",
        "background" => "background",
        "suicune_jump" => "suicune_jump",
        "suicune_close" => "suicune_close",
        "suicune_back" => "suicune_back",
        "crystal_unowns" => "crystal_unowns",
        other => other,
    }
}

fn visible_intro_palette_name(graphic_name: &str) -> &str {
    if graphic_name == "suicune_close" {
        // IntroScene17 loads IntroSuicuneClosePalette, whose banks 2-4
        // provide the light-blue and purple shading for the close-up.  The
        // generic Suicune palette intentionally leaves those banks orange.
        "suicune_close"
    } else if graphic_name.starts_with("unown") || graphic_name == "pulse" {
        "unowns"
    } else if graphic_name.starts_with("suicune") {
        "suicune"
    } else if graphic_name.starts_with("grass")
        || graphic_name == "background"
        || graphic_name == "pichu_wooper"
    {
        "background"
    } else if graphic_name == "crystal_unowns" {
        "crystal_unowns"
    } else {
        graphic_name
    }
}

fn visible_intro_tile_shift(map_name: &str) -> u8 {
    match map_name {
        "unowns" | "unown_a" | "unown_hi" | "crystal_unowns" | "suicune_close" => 0x80,
        _ => 0,
    }
}

fn visible_intro_tile_index_mode(map_name: &str) -> IntroTileIndexMode {
    match map_name {
        "suicune_close" => IntroTileIndexMode::Signed,
        _ => IntroTileIndexMode::Offset,
    }
}

fn draw_intro_tile(
    intro_root: &Path,
    intro: &VisibleIntroScreen,
    graphic_name: &str,
    palette_name: &str,
    tile_id: u8,
    attr: u8,
    tile_shift: u8,
    tile_mode: IntroTileIndexMode,
    transparent_zero: bool,
    rendered_art: &mut RenderedTilesetArt,
    dest_x: usize,
    dest_y: usize,
    target: &mut [u8],
) -> Result<()> {
    let image_path = intro_root.join(format!("{graphic_name}.png"));
    let source_key = graphic_name.to_string();
    if !rendered_art.intro_source_cache.contains_key(&source_key) {
        let source = crate::open_runtime_image(&image_path)
            .with_context(|| format!("decode intro PNG {}", image_path.display()))?
            .to_rgba8();
        rendered_art
            .intro_source_cache
            .insert(source_key.clone(), source);
    }
    let source = rendered_art
        .intro_source_cache
        .get(&source_key)
        .expect("intro source cache initialized");
    let (width, height) = source.dimensions();
    if width % SOURCE_TILE_SIZE as u32 != 0 || height % SOURCE_TILE_SIZE as u32 != 0 {
        anyhow::bail!(
            "intro PNG {} has invalid tile dimensions {}x{}",
            image_path.display(),
            width,
            height
        );
    }
    let source_tile_count =
        (width as usize / SOURCE_TILE_SIZE) * (height as usize / SOURCE_TILE_SIZE);
    let palette_key = format!("{palette_name}:{transparent_zero}");
    if !rendered_art.intro_palette_cache.contains_key(&palette_key) {
        rendered_art.intro_palette_cache.insert(
            palette_key.clone(),
            load_intro_palette_bank_for_kind(intro_root, palette_name, transparent_zero)?,
        );
    }
    let palette_bank = rendered_art
        .intro_palette_cache
        .get(&palette_key)
        .expect("intro palette cache initialized");
    let palette_index = usize::from(attr & 0x07);
    let palette = palette_bank
        .get(palette_index)
        .or_else(|| palette_bank.first())
        .with_context(|| format!("intro palette {palette_name} is empty"))?;
    let palette_override = visible_intro_effective_palette_cached(
        intro,
        rendered_art,
        palette_name,
        palette_index,
        palette,
        false,
    )?;
    let source_tile_index =
        resolve_intro_tile_index(tile_id, tile_shift, tile_mode, source_tile_count);
    let xflip = (attr & 0x20) != 0;
    let yflip = (attr & 0x40) != 0;
    blit_intro_source_tile(
        &source,
        width as usize,
        source_tile_index,
        &palette_override,
        transparent_zero,
        xflip,
        yflip,
        dest_x,
        dest_y,
        target,
    );
    Ok(())
}

fn visible_intro_effective_palette_cached(
    intro: &VisibleIntroScreen,
    rendered_art: &RenderedTilesetArt,
    palette_name: &str,
    palette_index: usize,
    base_palette: &Palette,
    is_obj_palette: bool,
) -> Result<Palette> {
    let black = [[0_u8, 0, 0]; 4];
    Ok(match intro.palette_effect {
        VisibleIntroPaletteEffect::None => *base_palette,
        VisibleIntroPaletteEffect::ClearBg if !is_obj_palette => black,
        VisibleIntroPaletteEffect::ClearBg => *base_palette,
        VisibleIntroPaletteEffect::UnownFade { palette_idx, timer }
            if !is_obj_palette && palette_name == "unowns" =>
        {
            let target = usize::from(palette_idx & 0x07);
            if palette_index != target {
                black
            } else {
                let source_palette = rendered_art
                    .intro_palette_cache
                    .get("unowns:false")
                    .context("intro palette cache missing unowns")?;
                let mut palette = source_palette.get(target).copied().with_context(|| {
                    format!("intro palette unowns has no source entry for index {target}")
                })?;
                let fade_timer = {
                    let value = timer & 0x3f;
                    if value > 0x1f { 0x3f - value } else { value }
                };
                palette[1] = intro_bw_fade_color(fade_timer);
                palette[2] = intro_black_light_blue_fade_color(fade_timer);
                palette[3] = intro_black_blue_fade_color(fade_timer);
                palette
            }
        }
        VisibleIntroPaletteEffect::UnownFade { .. } => *base_palette,
        VisibleIntroPaletteEffect::AppearUnown {
            palette_set_idx,
            revealed,
        } if !is_obj_palette
            && (palette_name == "unowns" || palette_name == "suicune")
            && palette_index >= 2
            && palette_index <= usize::from(revealed) =>
        {
            let palette_set = if palette_set_idx == 0 {
                "unown_1"
            } else {
                "unown_2"
            };
            rendered_art
                .intro_palette_cache
                .get(&format!("{palette_set}:true"))
                .with_context(|| format!("intro palette cache missing {palette_set}"))?
                .first()
                .copied()
                .with_context(|| format!("intro palette {palette_set} has no OBJ palette"))?
        }
        VisibleIntroPaletteEffect::AppearUnown { .. } => *base_palette,
        VisibleIntroPaletteEffect::Scene24Fade { fade_index } if !is_obj_palette => {
            let fade_palettes = rendered_art
                .intro_palette_cache
                .get("fade:false")
                .context("intro palette cache missing fade")?;
            fade_palettes
                .get(usize::from(fade_index))
                .copied()
                .with_context(|| {
                    format!("intro fade palette has no source entry for index {fade_index}")
                })?
        }
        VisibleIntroPaletteEffect::Scene24Fade { .. } => *base_palette,
        VisibleIntroPaletteEffect::CrystalWordFade { fade_level, timer }
            if palette_name == "crystal_unowns" && usize::from(fade_level) == palette_index =>
        {
            let mut palette = *base_palette;
            palette[2] = intro_fast_fade_color(timer);
            palette[3] = intro_slow_fade_color(timer);
            palette
        }
        VisibleIntroPaletteEffect::CrystalWordFade { .. } => *base_palette,
    })
}

#[cfg(test)]
fn visible_intro_effective_palette(
    intro: &VisibleIntroScreen,
    intro_root: &Path,
    palette_name: &str,
    palette_index: usize,
    base_palette: &Palette,
) -> Result<Palette> {
    let mut rendered_art = RenderedTilesetArt::default();
    for name in [palette_name, "unowns", "unown_1", "unown_2", "fade"] {
        for transparent_zero in [false, true] {
            let key = format!("{name}:{transparent_zero}");
            if !rendered_art.intro_palette_cache.contains_key(&key) {
                rendered_art.intro_palette_cache.insert(
                    key,
                    load_intro_palette_bank_for_kind(intro_root, name, transparent_zero)?,
                );
            }
        }
    }
    visible_intro_effective_palette_cached(
        intro,
        &rendered_art,
        palette_name,
        palette_index,
        base_palette,
        false,
    )
}

fn intro_bw_fade_color(timer: u8) -> [u8; 3] {
    let hue = timer.min(31).saturating_mul(8);
    [hue, hue, hue]
}

fn intro_black_light_blue_fade_color(timer: u8) -> [u8; 3] {
    let hue = timer.min(31);
    [0, (hue / 2).saturating_mul(8), hue.saturating_mul(8)]
}

fn intro_black_blue_fade_color(timer: u8) -> [u8; 3] {
    let hue = timer.min(31).saturating_mul(8);
    [0, 0, hue]
}

fn intro_fast_fade_color(timer: u8) -> [u8; 3] {
    let mut hue = 31_i16;
    let target = usize::from(timer).min(15);
    let mut values = [[0_u8; 3]; 16];
    let mut index = 0;
    for _ in 0..8 {
        values[index] = intro_fade_gray(hue);
        index += 1;
        hue -= 1;
        values[index] = intro_fade_gray(hue);
        index += 1;
        hue -= 2;
    }
    values[target]
}

fn intro_slow_fade_color(timer: u8) -> [u8; 3] {
    intro_fade_gray(31_i16 - i16::from(timer.min(15)))
}

fn intro_fade_gray(hue: i16) -> [u8; 3] {
    let value = hue.clamp(0, 31) as u8 * 8;
    [value, value, value]
}

fn resolve_intro_tile_index(
    tile_id: u8,
    tile_shift: u8,
    tile_mode: IntroTileIndexMode,
    source_tile_count: usize,
) -> usize {
    let mut resolved = tile_id;
    if tile_shift != 0 {
        resolved = match tile_mode {
            IntroTileIndexMode::Signed => resolved.wrapping_sub(tile_shift),
            IntroTileIndexMode::Offset if resolved >= tile_shift => {
                resolved.wrapping_sub(tile_shift)
            }
            IntroTileIndexMode::Offset => resolved,
        };
    }
    if source_tile_count == 0 {
        0
    } else {
        usize::from(resolved) % source_tile_count
    }
}

#[cfg(test)]
fn load_intro_palette_bank(intro_root: &Path, palette_name: &str) -> Result<Vec<Palette>> {
    let palette_path = intro_root.join(format!("{palette_name}.pal"));
    let content = crate::read_runtime_asset_to_string(&palette_path)
        .with_context(|| format!("read intro palette {}", palette_path.display()))?;
    let palettes = parse_palette_file(&content, None)?;
    if palettes.is_empty() {
        anyhow::bail!(
            "intro palette {} produced no palettes",
            palette_path.display()
        );
    }
    Ok(palettes.into_iter().take(8).collect())
}

fn load_intro_palette_bank_for_kind(
    intro_root: &Path,
    palette_name: &str,
    transparent_zero: bool,
) -> Result<Vec<Palette>> {
    let palette_path = intro_root.join(format!("{palette_name}.pal"));
    let content = crate::read_runtime_asset_to_string(&palette_path)
        .with_context(|| format!("read intro palette {}", palette_path.display()))?;
    let palettes = parse_palette_file(&content, None)?;
    let selected = if transparent_zero && palettes.len() >= 16 {
        palettes.into_iter().skip(8).take(8).collect::<Vec<_>>()
    } else {
        palettes.into_iter().take(8).collect::<Vec<_>>()
    };
    if selected.is_empty() {
        anyhow::bail!(
            "intro palette {} produced no {} palettes",
            palette_path.display(),
            if transparent_zero {
                "OBJ"
            } else {
                "background"
            }
        );
    }
    Ok(selected)
}

fn blit_intro_source_tile(
    source: &image::RgbaImage,
    source_width: usize,
    tile_index: usize,
    palette: &Palette,
    transparent_zero: bool,
    xflip: bool,
    yflip: bool,
    dest_x: usize,
    dest_y: usize,
    target: &mut [u8],
) {
    const INTRO_SURFACE_SIZE: usize = 32 * SOURCE_TILE_SIZE;
    let tiles_per_row = (source_width / SOURCE_TILE_SIZE).max(1);
    let source_x = (tile_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (tile_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        for col in 0..SOURCE_TILE_SIZE {
            let source_col = if xflip {
                SOURCE_TILE_SIZE - 1 - col
            } else {
                col
            };
            let source_row = if yflip {
                SOURCE_TILE_SIZE - 1 - row
            } else {
                row
            };
            let source_pixel = source.get_pixel(
                (source_x + source_col) as u32,
                (source_y + source_row) as u32,
            );
            // Match the TypeScript intro compositor: exported alpha remains
            // transparent when the recolored tile is blitted.  Forcing these
            // pixels to palette color zero paints opaque 8x8 rectangles over
            // Suicune's close-up.
            if source_pixel[3] == 0 {
                continue;
            }
            let palette_index = palette_index_from_gray(source_pixel[0]);
            if transparent_zero && palette_index == 0 {
                continue;
            }
            let [red, green, blue] = palette[palette_index];
            let target_x = (dest_x + col) % INTRO_SURFACE_SIZE;
            let target_y = (dest_y + row) % INTRO_SURFACE_SIZE;
            let offset = (target_y * INTRO_SURFACE_SIZE + target_x) * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn blit_intro_sprite_source_tile(
    source: &image::RgbaImage,
    source_width: usize,
    tile_index: usize,
    palette: &Palette,
    transparent_zero: bool,
    xflip: bool,
    yflip: bool,
    dest_x: i16,
    dest_y: i16,
    target: &mut [u8],
) {
    const INTRO_SURFACE_SIZE: usize = 32 * SOURCE_TILE_SIZE;
    const LCD_WIDTH: i16 = 20 * SOURCE_TILE_SIZE as i16;
    const LCD_HEIGHT: i16 = 18 * SOURCE_TILE_SIZE as i16;
    let tiles_per_row = (source_width / SOURCE_TILE_SIZE).max(1);
    let source_x = (tile_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (tile_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        let target_y = dest_y + row as i16;
        if !(0..LCD_HEIGHT).contains(&target_y) {
            continue;
        }
        for col in 0..SOURCE_TILE_SIZE {
            let target_x = dest_x + col as i16;
            if !(0..LCD_WIDTH).contains(&target_x) {
                continue;
            }
            let source_col = if xflip {
                SOURCE_TILE_SIZE - 1 - col
            } else {
                col
            };
            let source_row = if yflip {
                SOURCE_TILE_SIZE - 1 - row
            } else {
                row
            };
            let source_pixel = source.get_pixel(
                (source_x + source_col) as u32,
                (source_y + source_row) as u32,
            );
            if source_pixel[3] == 0 {
                continue;
            }
            let palette_index = palette_index_from_gray(source_pixel[0]);
            if transparent_zero && palette_index == 0 {
                continue;
            }
            let [red, green, blue] = palette[palette_index];
            let target_index = ((target_y as usize * INTRO_SURFACE_SIZE) + target_x as usize) * 4;
            target[target_index] = red;
            target[target_index + 1] = green;
            target[target_index + 2] = blue;
            target[target_index + 3] = 255;
        }
    }
}

fn load_title_frame(
    asset_root: &AssetRoot,
    asset_id: &str,
    palette_id: u8,
    transparent_zero: bool,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let image_path = title_asset_path(asset_root, asset_id);
    let source = crate::open_runtime_image(&image_path)
        .with_context(|| format!("decode title PNG {}", image_path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width == 0 || height == 0 {
        anyhow::bail!(
            "title asset {} has invalid dimensions {}x{}",
            asset_id,
            width,
            height
        );
    }
    let palette_bank = load_title_palette_bank(asset_root)?;
    let palette = palette_bank
        .get(usize::from(palette_id))
        .or_else(|| palette_bank.first())
        .context("title palette bank is empty")?;
    let mut data = vec![0_u8; width as usize * height as usize * 4];
    copy_source_image_rgba(
        &source,
        width as usize,
        height as usize,
        palette,
        transparent_zero,
        &mut data,
    );
    let mut image = Image::new(
        Extent3d {
            width,
            height,
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

const TITLE_SCREEN_WIDTH: usize = 20 * SOURCE_TILE_SIZE;
const TITLE_SCREEN_HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
const TITLE_LOGO_ROWS: usize = 7;
const TITLE_LOGO_COLS: usize = 20;
const TITLE_BG_SCY: usize = 8;
const TITLE_LOGO_ASM_Y_TILE: usize = 3;
const TITLE_SUICUNE_ASM_Y_TILE: usize = 12;
const TITLE_VERSION_WINDOW_Y: usize = 0x88;
const TITLE_VERSION_TEXT_START_TILE: u8 = 0x0c;
const TITLE_VERSION_TEXT_START_COLUMN: usize = 3;
const TITLE_VERSION_TEXT_COLUMNS: usize = 13;

#[derive(Clone, Copy)]
enum NativeTitleScroll {
    None,
    EntranceInterlaced(u8),
}

impl NativeTitleScroll {
    fn at_scanline(self, y: usize) -> i16 {
        match self {
            Self::None => 0,
            Self::EntranceInterlaced(scx) if y < 80 && y % 2 == 0 => i16::from(scx),
            Self::EntranceInterlaced(scx) if y < 80 => i16::from(0_u8.wrapping_sub(scx)),
            Self::EntranceInterlaced(_) => 0,
        }
    }
}

fn load_title_screen_frame(
    asset_root: &AssetRoot,
    title: &TitleMenu,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let title_root = asset_root.runtime_assets().join("gfx/title");
    let logo = crate::open_runtime_image(title_root.join("logo.png"))
        .context("decode native title logo PNG")?
        .to_rgba8();
    let suicune = crate::open_runtime_image(title_root.join("suicune.png"))
        .context("decode native title Suicune PNG")?
        .to_rgba8();
    let crystal = crate::open_runtime_image(title_root.join("crystal.png"))
        .context("decode native title crystal PNG")?
        .to_rgba8();
    let palette_bank = load_title_palette_bank(asset_root)?;
    let mut data = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];
    let mut priority_map = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    draw_native_title_background(
        &logo,
        &suicune,
        &palette_bank,
        title,
        &mut data,
        &mut priority_map,
    )?;
    if !matches!(title.phase, VisibleTitlePhase::Entrance) {
        draw_native_title_version_window(&logo, &palette_bank, &mut data, &mut priority_map)?;
    }
    draw_native_title_crystal_sprites(&crystal, &palette_bank, title, &priority_map, &mut data)?;
    let mut image = Image::new(
        Extent3d {
            width: TITLE_SCREEN_WIDTH as u32,
            height: TITLE_SCREEN_HEIGHT as u32,
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
        size: Vec2::new(TITLE_SCREEN_WIDTH as f32, TITLE_SCREEN_HEIGHT as f32),
    })
}

fn draw_native_title_background(
    logo: &image::RgbaImage,
    suicune: &image::RgbaImage,
    palette_bank: &[Palette],
    title: &TitleMenu,
    target: &mut [u8],
    priority_map: &mut [u8],
) -> Result<()> {
    let background_scroll = if matches!(title.phase, VisibleTitlePhase::Entrance) {
        NativeTitleScroll::EntranceInterlaced(title.scx)
    } else {
        NativeTitleScroll::None
    };
    let suicune_frame_start = [0x80_u8, 0x88, 0x00, 0x08][((title.frame / 8) as usize) % 4];
    for row in 0..6 {
        for col in 0..8 {
            let tile_id = suicune_frame_start.wrapping_add((row * 16 + col) as u8);
            let tile_index = if tile_id >= 0x80 {
                usize::from(tile_id - 0x80)
            } else {
                usize::from(tile_id) + 0x80
            };
            blit_native_title_tile(
                suicune,
                tile_index,
                palette_bank
                    .first()
                    .context("title palette bank missing BG palette 0")?,
                false,
                (6 + col) * SOURCE_TILE_SIZE,
                (TITLE_SUICUNE_ASM_Y_TILE + row) * SOURCE_TILE_SIZE - TITLE_BG_SCY,
                background_scroll,
                target,
                Some(priority_map),
            );
        }
    }

    for row in 0..TITLE_LOGO_ROWS {
        for col in 0..TITLE_LOGO_COLS {
            let tile_index = row * TITLE_LOGO_COLS + col;
            let palette_index = title_logo_palette_index(row + 3, col);
            let palette = palette_bank
                .get(palette_index)
                .with_context(|| format!("title palette {palette_index} missing"))?;
            blit_native_title_tile(
                logo,
                tile_index,
                palette,
                true,
                col * SOURCE_TILE_SIZE,
                (TITLE_LOGO_ASM_Y_TILE + row) * SOURCE_TILE_SIZE - TITLE_BG_SCY,
                background_scroll,
                target,
                Some(priority_map),
            );
        }
    }
    Ok(())
}

fn title_logo_palette_index(tile_row: usize, tile_col: usize) -> usize {
    match tile_row {
        3 | 4 => 2,
        5 => 3,
        6 => 4,
        7 => 5,
        8 => 6,
        9 if (5..16).contains(&tile_col) => 1,
        9 => 6,
        _ => 0,
    }
}

fn draw_native_title_version_window(
    logo: &image::RgbaImage,
    palette_bank: &[Palette],
    target: &mut [u8],
    priority_map: &mut [u8],
) -> Result<()> {
    let palette = palette_bank
        .get(7)
        .context("title palette bank missing version window palette 7")?;
    for col in 0..TITLE_VERSION_TEXT_COLUMNS {
        let tile_id = TITLE_VERSION_TEXT_START_TILE.wrapping_add(col as u8);
        let tile_index = if tile_id >= 0x80 {
            usize::from(tile_id - 0x80)
        } else {
            usize::from(tile_id) + 0x80
        };
        blit_native_title_tile(
            logo,
            tile_index,
            palette,
            true,
            (TITLE_VERSION_TEXT_START_COLUMN + col) * SOURCE_TILE_SIZE,
            TITLE_VERSION_WINDOW_Y,
            NativeTitleScroll::None,
            target,
            Some(priority_map),
        );
    }
    Ok(())
}

fn draw_native_title_crystal_sprites(
    crystal: &image::RgbaImage,
    palette_bank: &[Palette],
    title: &TitleMenu,
    priority_map: &[u8],
    target: &mut [u8],
) -> Result<()> {
    let palette = palette_bank
        .get(8)
        .context("title palette bank missing OBJ palette 0")?;
    let base_y = match title.phase {
        VisibleTitlePhase::Entrance => {
            let advanced = ((title.frame.min(122) as i16) * 2).min(244);
            -50 + advanced
        }
        _ => 6,
    };
    blit_native_title_image_with_priority(
        crystal,
        palette,
        true,
        56,
        base_y,
        0,
        priority_map,
        target,
    );
    Ok(())
}

fn blit_native_title_tile(
    source: &image::RgbaImage,
    tile_index: usize,
    palette: &Palette,
    transparent_zero: bool,
    dest_x: usize,
    dest_y: usize,
    scroll: NativeTitleScroll,
    target: &mut [u8],
    mut priority_map: Option<&mut [u8]>,
) {
    let tiles_per_row = (source.width() as usize / SOURCE_TILE_SIZE).max(1);
    let source_x = (tile_index % tiles_per_row) * SOURCE_TILE_SIZE;
    let source_y = (tile_index / tiles_per_row) * SOURCE_TILE_SIZE;
    for row in 0..SOURCE_TILE_SIZE {
        let draw_y = dest_y + row;
        if draw_y >= TITLE_SCREEN_HEIGHT {
            continue;
        }
        for col in 0..SOURCE_TILE_SIZE {
            let sx = source_x + col;
            let sy = source_y + row;
            if sx >= source.width() as usize || sy >= source.height() as usize {
                continue;
            }
            let source_pixel = source.get_pixel(sx as u32, sy as u32);
            if source_pixel[3] == 0 {
                continue;
            }
            let palette_index = palette_index_from_gray(source_pixel[0]);
            if transparent_zero && palette_index == 0 {
                continue;
            }
            let draw_x = (dest_x as i16 + col as i16 - scroll.at_scanline(draw_y))
                .rem_euclid(TITLE_SCREEN_WIDTH as i16) as usize;
            let [red, green, blue] = palette[palette_index];
            let offset = (draw_y * TITLE_SCREEN_WIDTH + draw_x) * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
            if let Some(map) = priority_map.as_deref_mut() {
                map[draw_y * TITLE_SCREEN_WIDTH + draw_x] = palette_index as u8;
            }
        }
    }
}

fn blit_native_title_image_with_priority(
    source: &image::RgbaImage,
    palette: &Palette,
    transparent_zero: bool,
    dest_x: i16,
    dest_y: i16,
    scroll_x: i16,
    priority_map: &[u8],
    target: &mut [u8],
) {
    for row in 0..source.height() as i16 {
        let draw_y = dest_y + row;
        if draw_y < 0 || draw_y >= TITLE_SCREEN_HEIGHT as i16 {
            continue;
        }
        for col in 0..source.width() as i16 {
            let source_pixel = source.get_pixel(col as u32, row as u32);
            if source_pixel[3] == 0 {
                continue;
            }
            let palette_index = palette_index_from_gray(source_pixel[0]);
            if transparent_zero && palette_index == 0 {
                continue;
            }
            let draw_x = (dest_x + col - scroll_x).rem_euclid(TITLE_SCREEN_WIDTH as i16) as usize;
            let priority_offset = draw_y as usize * TITLE_SCREEN_WIDTH + draw_x;
            if priority_map.get(priority_offset).copied().unwrap_or(0) != 0 {
                continue;
            }
            let [red, green, blue] = palette[palette_index];
            let offset = priority_offset * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn title_asset_path(asset_root: &AssetRoot, asset_id: &str) -> PathBuf {
    let runtime_assets = asset_root.runtime_assets();
    match asset_id {
        "copyright" => runtime_assets.join("gfx/splash/copyright.png"),
        other => runtime_assets
            .join("gfx/title")
            .join(format!("{other}.png")),
    }
}

fn load_title_palette_bank(asset_root: &AssetRoot) -> Result<Vec<Palette>> {
    let palette_path = asset_root.runtime_assets().join("gfx/title/title.pal");
    let content = crate::read_runtime_asset_to_string(&palette_path)
        .with_context(|| format!("read title palette {}", palette_path.display()))?;
    let palettes = parse_palette_file(&content, None)?;
    if palettes.is_empty() {
        anyhow::bail!(
            "title palette {} produced no palettes",
            palette_path.display()
        );
    }
    Ok(palettes)
}

fn pokemon_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    species_id: &str,
    side: PokemonSpriteSide,
    shiny: bool,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    pokemon_animation_frame_for_art(
        rendered_art,
        asset_root,
        species_id,
        side,
        shiny,
        0,
        images,
    )
}

fn pokemon_animation_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    species_id: &str,
    side: PokemonSpriteSide,
    shiny: bool,
    frame: u16,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let key = PokemonArtKey {
        species_id: normalize_pokemon_asset_id(species_id),
        side,
        shiny,
        frame,
    };
    if !rendered_art.pokemon_cache.contains_key(&key) {
        match load_pokemon_animation_frame(
            asset_root,
            &key.species_id,
            side,
            shiny,
            frame,
            images,
        ) {
            Ok(frame) => {
                rendered_art.pokemon_errors.remove(&key);
                rendered_art.pokemon_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .pokemon_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art.pokemon_cache.get(&key).cloned()
}

fn pokemon_art_error(
    rendered_art: &RenderedTilesetArt,
    species_id: &str,
    side: PokemonSpriteSide,
    shiny: bool,
) -> String {
    let key = PokemonArtKey {
        species_id: normalize_pokemon_asset_id(species_id),
        side,
        shiny,
        frame: 0,
    };
    rendered_art
        .pokemon_errors
        .get(&key)
        .cloned()
        .unwrap_or_else(|| "unknown Pokemon art load error".to_string())
}

fn oak_intro_sprite_frame(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    oak_intro: &VisibleOakIntroSequence,
    trainer: &crate::RuntimeTrainerSnapshot,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let sprite = oak_intro.current_sprite.as_deref()?;
    match sprite {
        "WOOPER" => pokemon_frame_for_art(
            rendered_art,
            asset_root,
            "WOOPER",
            PokemonSpriteSide::Front,
            false,
            images,
        ),
        "OAK" | "PLAYER" => {
            let key = IntroArtKey {
                asset_id: oak_intro_intro_asset_id(sprite, trainer),
            };
            if !rendered_art.intro_cache.contains_key(&key) {
                match load_oak_intro_frame(asset_root, &key.asset_id, images) {
                    Ok(frame) => {
                        rendered_art.intro_errors.remove(&key);
                        rendered_art.intro_cache.insert(key.clone(), frame);
                    }
                    Err(error) => {
                        rendered_art
                            .intro_errors
                            .insert(key.clone(), error.to_string());
                    }
                }
            }
            rendered_art.intro_cache.get(&key).cloned()
        }
        _ => None,
    }
}

fn oak_intro_art_error(
    rendered_art: &RenderedTilesetArt,
    oak_intro: &VisibleOakIntroSequence,
    trainer: &crate::RuntimeTrainerSnapshot,
) -> String {
    let Some(sprite) = oak_intro.current_sprite.as_deref() else {
        return "no Oak intro sprite is active".to_string();
    };
    if sprite == "WOOPER" {
        return pokemon_art_error(rendered_art, "WOOPER", PokemonSpriteSide::Front, false);
    }
    let key = IntroArtKey {
        asset_id: oak_intro_intro_asset_id(sprite, trainer),
    };
    rendered_art
        .intro_errors
        .get(&key)
        .cloned()
        .unwrap_or_else(|| "unknown Oak intro art load error".to_string())
}

fn oak_intro_intro_asset_id(sprite: &str, trainer: &crate::RuntimeTrainerSnapshot) -> String {
    match sprite {
        "OAK" => "trainer:oak".to_string(),
        "PLAYER" if trainer.player_gender == PLAYER_GENDER_FEMALE => "player:kris".to_string(),
        "PLAYER" => "player:chris".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

fn load_oak_intro_frame(
    asset_root: &AssetRoot,
    asset_id: &str,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let battle_sprite = asset_id.starts_with("battle-trainer:")
        || asset_id.starts_with("battle-player:");
    let (image_path, palette_path, player_palette) = oak_intro_asset_paths(asset_root, asset_id)?;
    let source = crate::open_runtime_image(&image_path)
        .with_context(|| format!("decode Oak intro PNG {}", image_path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width == 0 || height == 0 {
        anyhow::bail!(
            "Oak intro asset {asset_id} has invalid dimensions {}x{}",
            width,
            height
        );
    }
    let palette = if player_palette {
        load_oak_intro_player_palette(&palette_path)?
    } else {
        load_gbcpal_palette(&palette_path)?
    };
    let frame_width = width as usize;
    let frame_height = height as usize;
    let mut data = vec![0_u8; frame_width * frame_height * 4];
    copy_source_image_rgba(
        &source,
        frame_width,
        frame_height,
        &palette,
        battle_sprite,
        &mut data,
    );
    let mut image = Image::new(
        Extent3d {
            width,
            height,
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

fn oak_intro_asset_paths(
    asset_root: &AssetRoot,
    asset_id: &str,
) -> Result<(PathBuf, PathBuf, bool)> {
    let runtime_assets = asset_root.runtime_assets();
    if let Some(stem) = asset_id.strip_prefix("battle-trainer:") {
        if stem.is_empty() {
            anyhow::bail!("battle trainer asset id has no trainer sprite stem");
        }
        return Ok((
            runtime_assets.join(format!("gfx/trainers/{stem}.png")),
            runtime_assets.join(format!("gfx/trainers/{stem}.gbcpal")),
            false,
        ));
    }
    if let Some(stem) = asset_id.strip_prefix("battle-player:") {
        if stem == "dude" {
            return Ok((
                runtime_assets.join("gfx/battle/dude.png"),
                runtime_assets.join("gfx/trainers/cal.gbcpal"),
                true,
            ));
        }
        let palette = match stem {
            "kris_back" => "falkner",
            "chris_back" => "cal",
            other => anyhow::bail!("unknown battle player backpic stem {other}"),
        };
        return Ok((
            runtime_assets.join(format!("gfx/player/{stem}.png")),
            runtime_assets.join(format!("gfx/trainers/{palette}.gbcpal")),
            true,
        ));
    }
    Ok(match asset_id {
        "trainer:oak" => (
            runtime_assets.join("gfx/trainers/oak.png"),
            runtime_assets.join("gfx/trainers/oak.gbcpal"),
            false,
        ),
        "player:chris" => (
            runtime_assets.join("gfx/player/chris.png"),
            runtime_assets.join("gfx/trainers/cal.gbcpal"),
            true,
        ),
        "player:kris" => (
            runtime_assets.join("gfx/player/kris.png"),
            runtime_assets.join("gfx/trainers/falkner.gbcpal"),
            true,
        ),
        other => anyhow::bail!("unknown Oak intro asset id {other}"),
    })
}

fn load_oak_intro_player_palette(path: &Path) -> Result<Palette> {
    let trainer_palette = load_gbcpal_palette(path)?;
    Ok([
        [255, 255, 255],
        trainer_palette[1],
        trainer_palette[2],
        [0, 0, 0],
    ])
}

fn load_pokemon_frame(
    asset_root: &AssetRoot,
    species_id: &str,
    side: PokemonSpriteSide,
    shiny: bool,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    load_pokemon_animation_frame(asset_root, species_id, side, shiny, 0, images)
}

fn load_pokemon_animation_frame(
    asset_root: &AssetRoot,
    species_id: &str,
    side: PokemonSpriteSide,
    shiny: bool,
    frame: u16,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let species_id = normalize_pokemon_asset_id(species_id);
    let image_path = pokemon_asset_path(asset_root, &species_id, side, "png");
    let source = crate::open_runtime_image(&image_path)
        .with_context(|| format!("decode Pokemon sprite PNG {}", image_path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width == 0 || height == 0 {
        anyhow::bail!(
            "Pokemon sprite {} {:?} has invalid dimensions {}x{}",
            species_id,
            side,
            width,
            height
        );
    }
    let frame_width = width as usize;
    let frame_height = width.min(height) as usize;
    let source_y = usize::from(frame) * frame_height;
    if source_y + frame_height > height as usize {
        anyhow::bail!(
            "Pokemon sprite {} {:?} animation frame {} exceeds {} source frames",
            species_id,
            side,
            frame,
            height as usize / frame_height
        );
    }
    // Exported Pokemon PNGs are already colourized with the normal species
    // palette. Resolve their pixels against that palette before applying the
    // requested normal/shiny palette; inferring indices from only the red
    // channel turns colours such as Chikorita's dark green into black.
    let source_palette = load_pokemon_palette(asset_root, &species_id, side, false)?;
    let palette = if shiny {
        load_pokemon_palette(asset_root, &species_id, side, true)?
    } else {
        source_palette
    };
    let mut data = vec![0_u8; frame_width * frame_height * 4];
    copy_pokemon_frame_rgba_at(
        &source,
        frame_width,
        frame_height,
        source_y,
        &source_palette,
        &palette,
        &mut data,
    );
    let mut image = Image::new(
        Extent3d {
            width,
            height: frame_height as u32,
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
        size: Vec2::new(frame_width as f32, frame_height as f32),
    })
}

fn pokemon_asset_path(
    asset_root: &AssetRoot,
    species_id: &str,
    side: PokemonSpriteSide,
    extension: &str,
) -> PathBuf {
    let sprite_name = match side {
        PokemonSpriteSide::Front => "front",
        PokemonSpriteSide::Back => "back",
    };
    asset_root
        .runtime_assets()
        .join("gfx/pokemon")
        .join(species_id)
        .join(format!("{sprite_name}.{extension}"))
}

fn load_pokemon_palette(
    asset_root: &AssetRoot,
    species_id: &str,
    side: PokemonSpriteSide,
    shiny: bool,
) -> Result<Palette> {
    if shiny {
        let palette_path = asset_root
            .runtime_assets()
            .join("gfx/pokemon")
            .join(species_id)
            .join("shiny.pal");
        if crate::runtime_asset_exists(&palette_path) {
            let content = crate::read_runtime_asset_to_string(&palette_path).with_context(|| {
                format!("read shiny Pokemon palette {}", palette_path.display())
            })?;
            if let Some(palette) = parse_palette_file(&content, None)?.into_iter().next() {
                return Ok(palette);
            }
        }
    }
    let side_palette_path = pokemon_asset_path(asset_root, species_id, side, "gbcpal");
    if crate::runtime_asset_exists(&side_palette_path) {
        return load_pokemon_gbcpal(&side_palette_path);
    }
    let normal_palette_path = asset_root
        .runtime_assets()
        .join("gfx/pokemon")
        .join(species_id)
        .join("normal.gbcpal");
    load_pokemon_gbcpal(&normal_palette_path)
}

fn load_pokemon_gbcpal(path: &Path) -> Result<Palette> {
    load_gbcpal_palette(path)
}

fn load_gbcpal_palette(path: &Path) -> Result<Palette> {
    let bytes =
        crate::read_runtime_asset(path).with_context(|| format!("read GBC palette {}", path.display()))?;
    if bytes.len() < 8 {
        anyhow::bail!(
            "GBC palette {} has {} bytes; expected at least 8",
            path.display(),
            bytes.len()
        );
    }
    let mut colors = [[0_u8; 3]; 4];
    for index in 0..4 {
        let offset = index * 2;
        let value = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        colors[index] = gbc_color_to_rgb(value);
    }
    Ok(colors)
}

fn gbc_color_to_rgb(value: u16) -> [u8; 3] {
    let red = (value & 0x1f) as u8;
    let green = ((value >> 5) & 0x1f) as u8;
    let blue = ((value >> 10) & 0x1f) as u8;
    [
        normalize_palette_component(red),
        normalize_palette_component(green),
        normalize_palette_component(blue),
    ]
}

fn copy_pokemon_frame_rgba(
    source: &image::RgbaImage,
    frame_width: usize,
    frame_height: usize,
    source_palette: &Palette,
    palette: &Palette,
    target: &mut [u8],
) {
    copy_pokemon_frame_rgba_at(
        source,
        frame_width,
        frame_height,
        0,
        source_palette,
        palette,
        target,
    );
}

fn copy_pokemon_frame_rgba_at(
    source: &image::RgbaImage,
    frame_width: usize,
    frame_height: usize,
    source_y: usize,
    source_palette: &Palette,
    palette: &Palette,
    target: &mut [u8],
) {
    let mut palette_indices = vec![0usize; frame_width * frame_height];
    let mut source_opaque = vec![false; frame_width * frame_height];
    for row in 0..frame_height {
        for col in 0..frame_width {
            let source_pixel = source.get_pixel(col as u32, (source_y + row) as u32);
            let index = row * frame_width + col;
            source_opaque[index] = source_pixel[3] != 0;
            palette_indices[index] = pokemon_palette_index(source_pixel, source_palette);
        }
    }

    // Match TypeScript's _apply_colorkey_transparency: only background pixels
    // connected to the frame border are transparent. Palette color 0 inside a
    // sprite is real artwork and must not be discarded (several Crystal
    // sprites use enclosed white highlights/details).
    let background_index = palette_indices[0];
    let mut transparent = vec![false; frame_width * frame_height];
    let mut pending = std::collections::VecDeque::new();
    let enqueue = |row: usize,
                   col: usize,
                   transparent: &mut Vec<bool>,
                   pending: &mut std::collections::VecDeque<(usize, usize)>| {
        let index = row * frame_width + col;
        if source_opaque[index] && palette_indices[index] == background_index && !transparent[index]
        {
            transparent[index] = true;
            pending.push_back((row, col));
        }
    };
    for col in 0..frame_width {
        enqueue(0, col, &mut transparent, &mut pending);
        enqueue(frame_height - 1, col, &mut transparent, &mut pending);
    }
    for row in 1..frame_height.saturating_sub(1) {
        enqueue(row, 0, &mut transparent, &mut pending);
        enqueue(row, frame_width - 1, &mut transparent, &mut pending);
    }
    while let Some((row, col)) = pending.pop_front() {
        if row > 0 {
            enqueue(row - 1, col, &mut transparent, &mut pending);
        }
        if row + 1 < frame_height {
            enqueue(row + 1, col, &mut transparent, &mut pending);
        }
        if col > 0 {
            enqueue(row, col - 1, &mut transparent, &mut pending);
        }
        if col + 1 < frame_width {
            enqueue(row, col + 1, &mut transparent, &mut pending);
        }
    }

    for row in 0..frame_height {
        for col in 0..frame_width {
            let index = row * frame_width + col;
            let offset = index * 4;
            if !source_opaque[index] || transparent[index] {
                target[offset + 3] = 0;
                continue;
            }
            let [red, green, blue] = palette[palette_indices[index]];
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn pokemon_palette_index(pixel: &image::Rgba<u8>, source_palette: &Palette) -> usize {
    let rgb = [pixel[0], pixel[1], pixel[2]];
    if let Some(index) = source_palette.iter().position(|colour| *colour == rgb) {
        return index;
    }
    match pixel[0] {
        0xff => 0,
        0xaa => 1,
        0x55 => 2,
        0x00 => 3,
        value => palette_index_from_gray(value),
    }
}

fn normalize_pokemon_asset_id(species_id: &str) -> String {
    species_id.trim().to_ascii_lowercase().replace('_', "-")
}

fn copy_source_image_rgba(
    source: &image::RgbaImage,
    width: usize,
    height: usize,
    palette: &Palette,
    transparent_zero: bool,
    target: &mut [u8],
) {
    for row in 0..height {
        for col in 0..width {
            let source_pixel = source.get_pixel(col as u32, row as u32);
            let offset = (row * width + col) * 4;
            if source_pixel[3] == 0 {
                target[offset + 3] = 0;
                continue;
            }
            let palette_index = palette_index_from_gray(source_pixel[0]);
            if transparent_zero && palette_index == 0 {
                target[offset + 3] = 0;
                continue;
            }
            let [red, green, blue] = palette[palette_index];
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn town_map_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    region: &str,
    player_gender: u8,
    tile_palettes: &[String],
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let region = if region.eq_ignore_ascii_case("KANTO") { "kanto" } else { "johto" };
    let key = (region.to_string(), player_gender);
    if !rendered_art.town_map_cache.contains_key(&key)
        && !rendered_art.town_map_errors.contains_key(&key)
    {
        match load_town_map_frame(asset_root, region, player_gender, tile_palettes, images) {
            Ok(frame) => {
                rendered_art.town_map_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art.town_map_errors.insert(key.clone(), format!("{error:#}"));
            }
        }
    }
    rendered_art.town_map_cache.get(&key).cloned()
}

fn load_town_map_frame(
    asset_root: &AssetRoot,
    region: &str,
    player_gender: u8,
    tile_palettes: &[String],
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH_TILES: usize = 20;
    const HEIGHT_TILES: usize = 18;
    const TILE_PIXELS: usize = 8;
    let root = asset_root.runtime_assets().join("gfx/pokegear");
    let source_path = root.join("town_map.png");
    let source = crate::open_runtime_image(&source_path)
        .with_context(|| format!("decode Town Map tiles {}", source_path.display()))?
        .to_rgba8();
    anyhow::ensure!(
        source.width() == 128 && source.height() == 24,
        "Town Map tiles {} have dimensions {}x{}, expected 128x24",
        source_path.display(),
        source.width(),
        source.height()
    );
    anyhow::ensure!(
        tile_palettes.len() >= 48,
        "Town Map palette map has {} entries, expected at least 48",
        tile_palettes.len()
    );
    let palette_path = root.join(if player_gender == 0 {
        "pokegear.pal"
    } else {
        "pokegear_f.pal"
    });
    let palette_source = crate::read_runtime_asset_to_string(&palette_path)
        .with_context(|| format!("read Town Map palette {}", palette_path.display()))?;
    let palettes = parse_palette_file(&palette_source, None)?;
    anyhow::ensure!(palettes.len() >= 6, "Town Map palette bank is incomplete");
    let tilemap_path = root.join(format!("{region}.bin"));
    let mut tilemap = crate::read_runtime_asset(&tilemap_path)
        .with_context(|| format!("read Town Map tilemap {}", tilemap_path.display()))?;
    if tilemap.last() == Some(&0xff) {
        tilemap.pop();
    }
    anyhow::ensure!(
        tilemap.len() == WIDTH_TILES * HEIGHT_TILES,
        "Town Map tilemap {} has {} tiles, expected {}",
        tilemap_path.display(),
        tilemap.len(),
        WIDTH_TILES * HEIGHT_TILES
    );
    let width = WIDTH_TILES * TILE_PIXELS;
    let height = HEIGHT_TILES * TILE_PIXELS;
    let mut data = vec![0_u8; width * height * 4];
    for (map_index, tile_id) in tilemap.into_iter().enumerate() {
        let tile = usize::from(tile_id);
        anyhow::ensure!(tile < 48, "Town Map tile id {tile} is outside the 48-tile atlas");
        let palette_index = match tile_palettes[tile].as_str() {
            "BORDER" => 0,
            "EARTH" => 1,
            "MOUNTAIN" => 2,
            "CITY" => 3,
            "POI" => 4,
            "POI_MTN" => 5,
            token => anyhow::bail!("unknown Town Map palette token {token}"),
        };
        let source_x = (tile % 16) * TILE_PIXELS;
        let source_y = (tile / 16) * TILE_PIXELS;
        let target_x = (map_index % WIDTH_TILES) * TILE_PIXELS;
        let target_y = (map_index / WIDTH_TILES) * TILE_PIXELS;
        for row in 0..TILE_PIXELS {
            for col in 0..TILE_PIXELS {
                let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
                let colour = palettes[palette_index][palette_index_from_gray(pixel[0])];
                let offset = ((target_y + row) * width + target_x + col) * 4;
                data[offset..offset + 3].copy_from_slice(&colour);
                data[offset + 3] = 255;
            }
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

fn sprite_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    sprite_id: &str,
    palette_id: u8,
    time_of_day: &str,
    direction: Direction,
    walking: bool,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let key = SpriteArtKey {
        sprite_id: sprite_id.to_string(),
        palette_id: palette_id & 0x7,
        time_of_day: normalize_tileset_time_of_day(time_of_day),
    };
    if !rendered_art.sprite_cache.contains_key(&key) {
        match load_sprite_art(
            asset_root,
            &key.sprite_id,
            key.palette_id,
            &key.time_of_day,
            images,
        ) {
            Ok(art) => {
                rendered_art.sprite_errors.remove(&key);
                rendered_art.sprite_cache.insert(key.clone(), art);
            }
            Err(error) => {
                rendered_art
                    .sprite_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    let art = rendered_art.sprite_cache.get(&key)?;
    Some(
        match direction {
            Direction::Up => &art.up,
            Direction::Down => &art.down,
            Direction::Left => &art.left,
            Direction::Right => &art.right,
        }
        .frame(walking),
    )
}

fn load_sprite_art(
    asset_root: &AssetRoot,
    sprite_id: &str,
    palette_id: u8,
    time_of_day: &str,
    images: &mut Assets<Image>,
) -> Result<SpriteArt> {
    let sprite_path = sprite_asset_path(asset_root, sprite_id);
    let source = crate::open_runtime_image(&sprite_path)
        .with_context(|| format!("decode overworld sprite PNG {}", sprite_path.display()))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    if width == 0 || height == 0 || height % width != 0 {
        anyhow::bail!(
            "overworld sprite {} has invalid frame sheet dimensions {}x{}",
            sprite_id,
            width,
            height
        );
    }
    let frame_size = width as usize;
    let frame_count = (height / width) as usize;
    let palette_bank = load_npc_sprite_palette_bank(asset_root, time_of_day)?;
    let palette = palette_bank
        .get(usize::from(palette_id & 0x7))
        .or_else(|| palette_bank.first())
        .context("NPC sprite palette bank is empty")?;
    let effective_frame_count = if is_padded_static_sprite_sheet(&source, frame_size, frame_count) {
        1
    } else {
        frame_count
    };
    map_sprite_sheet_frames(
        sprite_id,
        &source,
        frame_size,
        effective_frame_count,
        palette,
        images,
    )
}

fn sprite_asset_path(asset_root: &AssetRoot, sprite_id: &str) -> PathBuf {
    let runtime_assets = asset_root.runtime_assets();
    if let Some(icon_id) = sprite_id.strip_prefix("icon_") {
        runtime_assets
            .join("gfx/icons")
            .join(format!("{icon_id}.png"))
    } else {
        runtime_assets
            .join("gfx/sprites")
            .join(format!("{sprite_id}.png"))
    }
}

fn emote_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    emote_id: &str,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    if !rendered_art.emote_cache.contains_key(emote_id)
        && !rendered_art.emote_errors.contains_key(emote_id)
    {
        let asset_name = match emote_id {
            "EMOTE_BOLT" => "bolt",
            "EMOTE_FISH" => "fish",
            "EMOTE_HAPPY" => "happy",
            "EMOTE_HEART" => "heart",
            "EMOTE_QUESTION" => "question",
            "EMOTE_SAD" => "sad",
            "EMOTE_SHOCK" => "shock",
            "EMOTE_SLEEP" => "sleep",
            other => {
                rendered_art
                    .emote_errors
                    .insert(other.to_string(), format!("unknown emote id {other}"));
                return None;
            }
        };
        let path = asset_root
            .runtime_assets()
            .join("gfx/emotes")
            .join(format!("{asset_name}.png"));
        let loaded = (|| -> Result<SpriteFrame> {
            let mut source = crate::open_runtime_image(&path)
                .with_context(|| format!("decode emote PNG {}", path.display()))?
                .to_rgba8();
            let (width, height) = source.dimensions();
            if width == 0 || height == 0 {
                anyhow::bail!("emote PNG {} is empty", path.display());
            }
            // Match TypeScript's EmoteSurfaceCache: the color in the source
            // image's top-left pixel is the sprite background, not visible
            // art. The checked-in 2-bit PNGs are opaque, so uploading them
            // directly produces a large white square around every emote.
            let background = *source.get_pixel(0, 0);
            if background[3] != 0 {
                for pixel in source.pixels_mut() {
                    if *pixel == background {
                        pixel[3] = 0;
                    }
                }
            }
            let mut image = Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                source.into_raw(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::nearest();
            Ok(SpriteFrame {
                handle: images.add(image),
                size: Vec2::new(width as f32 * 4.0, height as f32 * 4.0),
            })
        })();
        match loaded {
            Ok(frame) => {
                rendered_art.emote_cache.insert(emote_id.to_string(), frame);
            }
            Err(error) => {
                rendered_art
                    .emote_errors
                    .insert(emote_id.to_string(), error.to_string());
            }
        }
    }
    rendered_art.emote_cache.get(emote_id).cloned()
}

fn ledge_shadow_frame_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    if rendered_art.ledge_shadow.is_none() && rendered_art.ledge_shadow_error.is_none() {
        let path = asset_root.runtime_assets().join("gfx/overworld/shadow.png");
        let loaded = (|| -> Result<SpriteFrame> {
            let source = crate::open_runtime_image(&path)
                .with_context(|| format!("decode ledge shadow PNG {}", path.display()))?
                .to_rgba8();
            let (width, height) = source.dimensions();
            if width == 0 || height == 0 {
                anyhow::bail!("ledge shadow PNG {} is empty", path.display());
            }
            let overlap = width / 2;
            let combined_width = width
                .checked_add(overlap)
                .context("ledge shadow combined width overflow")?;
            let mut base = image::RgbaImage::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let pixel = source.get_pixel(width - 1 - x, height - 1 - y);
                    let transparent = pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240;
                    base.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            pixel[0],
                            pixel[1],
                            pixel[2],
                            if transparent { 0 } else { 255 },
                        ]),
                    );
                }
            }
            let mut combined = image::RgbaImage::new(combined_width, height);
            for y in 0..height {
                for x in 0..width {
                    let flipped = *base.get_pixel(width - 1 - x, y);
                    if flipped[3] != 0 {
                        combined.put_pixel(x, y, flipped);
                    }
                    let pixel = *base.get_pixel(x, y);
                    if pixel[3] != 0 {
                        combined.put_pixel(x + overlap, y, pixel);
                    }
                }
            }
            let mut image = Image::new(
                Extent3d {
                    width: combined_width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                combined.into_raw(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::nearest();
            Ok(SpriteFrame {
                handle: images.add(image),
                size: Vec2::new(combined_width as f32 * 4.0, height as f32 * 4.0),
            })
        })();
        match loaded {
            Ok(frame) => rendered_art.ledge_shadow = Some(frame),
            Err(error) => rendered_art.ledge_shadow_error = Some(error.to_string()),
        }
    }
    rendered_art.ledge_shadow.clone()
}

fn grass_rustle_frames_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    time_of_day: &str,
    images: &mut Assets<Image>,
) -> Option<[SpriteFrame; 2]> {
    let key = normalize_tileset_time_of_day(time_of_day);
    if !rendered_art.grass_rustle_cache.contains_key(&key)
        && !rendered_art.grass_rustle_errors.contains_key(&key)
    {
        let path = asset_root
            .runtime_assets()
            .join("gfx/overworld/grass_rustle.png");
        let loaded = (|| -> Result<[SpriteFrame; 2]> {
            let source = crate::open_runtime_image(&path)
                .with_context(|| format!("decode grass rustle PNG {}", path.display()))?
                .to_rgba8();
            let (width, height) = source.dimensions();
            if width == 0 || height == 0 {
                anyhow::bail!("grass rustle PNG {} is empty", path.display());
            }
            let background = *source.get_pixel(0, 0);
            let palette_bank = load_npc_sprite_palette_bank(asset_root, &key)?;
            let palette = palette_bank
                .get(6)
                .context("grass rustle palette bank has no palette 6")?;
            let build = |flipped: bool, images: &mut Assets<Image>| -> SpriteFrame {
                let mut pixels = vec![0; width as usize * height as usize * 4];
                for y in 0..height {
                    for x in 0..width {
                        let source_x = if flipped { width - 1 - x } else { x };
                        let pixel = source.get_pixel(source_x, y);
                        let offset = (y as usize * width as usize + x as usize) * 4;
                        if *pixel == background {
                            continue;
                        }
                        let color = palette[palette_index_from_gray(pixel[0])];
                        pixels[offset] = color[0];
                        pixels[offset + 1] = color[1];
                        pixels[offset + 2] = color[2];
                        pixels[offset + 3] = 255;
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                SpriteFrame {
                    handle: images.add(image),
                    size: Vec2::new(width as f32 * 4.0, height as f32 * 4.0),
                }
            };
            Ok([build(false, images), build(true, images)])
        })();
        match loaded {
            Ok(frames) => {
                rendered_art.grass_rustle_cache.insert(key.clone(), frames);
            }
            Err(error) => {
                rendered_art
                    .grass_rustle_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art.grass_rustle_cache.get(&key).cloned()
}

fn boulder_dust_frames_for_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    time_of_day: &str,
    images: &mut Assets<Image>,
) -> Option<[SpriteFrame; 2]> {
    let key = normalize_tileset_time_of_day(time_of_day);
    if !rendered_art.boulder_dust_cache.contains_key(&key)
        && !rendered_art.boulder_dust_errors.contains_key(&key)
    {
        let path = asset_root
            .runtime_assets()
            .join("gfx/overworld/boulder_dust.png");
        let loaded = (|| -> Result<[SpriteFrame; 2]> {
            let source = crate::open_runtime_image(&path)
                .with_context(|| format!("decode boulder dust PNG {}", path.display()))?
                .to_rgba8();
            let (width, height) = source.dimensions();
            if width != 8 || height != 16 {
                anyhow::bail!(
                    "boulder dust PNG {} must be 8x16, found {width}x{height}",
                    path.display()
                );
            }
            let background = *source.get_pixel(0, 0);
            let palette_bank = load_npc_sprite_palette_bank(asset_root, &key)?;
            let palette = palette_bank
                .get(5)
                .context("boulder dust palette bank has no PAL_OW_EMOTE palette 5")?;
            let build = |tile: u32, images: &mut Assets<Image>| -> SpriteFrame {
                let mut pixels = vec![0; 16 * 16 * 4];
                for y in 0..16_u32 {
                    for x in 0..16_u32 {
                        let pixel = source.get_pixel(x % 8, tile * 8 + y % 8);
                        if *pixel == background {
                            continue;
                        }
                        let offset = (y as usize * 16 + x as usize) * 4;
                        let color = palette[palette_index_from_gray(pixel[0])];
                        pixels[offset] = color[0];
                        pixels[offset + 1] = color[1];
                        pixels[offset + 2] = color[2];
                        pixels[offset + 3] = 255;
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width: 16,
                        height: 16,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                SpriteFrame {
                    handle: images.add(image),
                    size: Vec2::splat(64.0),
                }
            };
            Ok([build(0, images), build(1, images)])
        })();
        match loaded {
            Ok(frames) => {
                rendered_art.boulder_dust_cache.insert(key.clone(), frames);
            }
            Err(error) => {
                rendered_art
                    .boulder_dust_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art.boulder_dust_cache.get(&key).cloned()
}

fn resolve_visible_object_sprite_asset_id(
    asset_root: &AssetRoot,
    sprite: &str,
    variable_sprites: &BTreeMap<String, String>,
    menu_icons: &BTreeMap<String, String>,
) -> String {
    let requested = variable_sprites
        .get(sprite)
        .map(String::as_str)
        .unwrap_or(sprite);
    let normalized = requested
        .trim()
        .to_ascii_lowercase()
        .strip_prefix("sprite_")
        .unwrap_or(requested.trim())
        .to_ascii_lowercase();
    let mut candidates = vec![normalized.replace('-', "_")];
    if candidates[0] == "pokeball" {
        candidates.push("poke_ball".to_string());
    }
    // The TS renderer resolves species-only object sprites through menu
    // icons.  Keep the same useful path for Rust packs that contain the
    // corresponding icon asset.
    candidates.push(format!("icon_{normalized}"));
    if let Some(icon_token) = menu_icons.get(&normalized.to_ascii_uppercase()) {
        let icon = icon_token.trim().to_ascii_lowercase();
        if !icon.is_empty() {
            candidates.push(if icon.starts_with("icon_") {
                icon
            } else {
                format!("icon_{icon}")
            });
        }
    }
    candidates
        .into_iter()
        .find(|candidate| crate::runtime_asset_exists(sprite_asset_path(asset_root, candidate)))
        .unwrap_or_else(|| normalized.replace('-', "_"))
}

fn object_sprite_is_animated(spritemovedata: &str) -> bool {
    matches!(
        spritemovedata,
        "SPRITEMOVEDATA_WALK_LEFT_RIGHT"
            | "SPRITEMOVEDATA_WALK_UP_DOWN"
            | "SPRITEMOVEDATA_WANDER"
            | "SPRITEMOVEDATA_SWIM_WANDER"
            | "SPRITEMOVEDATA_SPINCLOCKWISE"
            | "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE"
            | "SPRITEMOVEDATA_SPINRANDOM_SLOW"
            | "SPRITEMOVEDATA_SPINRANDOM_FAST"
    )
}

fn resolve_visible_object_palette(
    sprite: &str,
    palette_override: u8,
    defaults: &BTreeMap<String, i64>,
) -> u8 {
    if palette_override != 0 {
        return palette_override & 0x7;
    }
    let normalized = sprite.trim().to_ascii_uppercase();
    defaults
        .get(sprite)
        .or_else(|| defaults.get(&normalized))
        .copied()
        .and_then(|palette| u8::try_from(palette).ok())
        .unwrap_or(0)
        & 0x7
}

fn load_npc_sprite_palette_bank(asset_root: &AssetRoot, time_of_day: &str) -> Result<Vec<Palette>> {
    let palette_path = asset_root
        .runtime_assets()
        .join("gfx/overworld/npc_sprites.pal");
    let content = crate::read_runtime_asset_to_string(&palette_path)
        .with_context(|| format!("read NPC sprite palette {}", palette_path.display()))?;
    let normalized = normalize_tileset_time_of_day(time_of_day);
    for group in [normalized.as_str(), "day", "morn", "nite", "dark"] {
        let palettes = parse_palette_file(&content, Some(group))?;
        if palettes.len() >= 8 {
            return Ok(palettes.into_iter().take(8).collect());
        }
    }
    anyhow::bail!(
        "NPC sprite palette {} did not contain a complete time-of-day palette group",
        palette_path.display()
    )
}

fn create_sprite_frame(
    source: &image::RgbaImage,
    frame_size: usize,
    frame_index: usize,
    palette: &Palette,
    mirror_x: bool,
    images: &mut Assets<Image>,
) -> SpriteFrame {
    let mut data = vec![0_u8; frame_size * frame_size * 4];
    copy_source_sprite_rgba(
        source,
        frame_size,
        frame_index,
        palette,
        mirror_x,
        &mut data,
    );
    let mut image = Image::new(
        Extent3d {
            width: frame_size as u32,
            height: frame_size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    SpriteFrame {
        handle: images.add(image),
        size: Vec2::splat(frame_size as f32 * (TILE_SIZE / SOURCE_TILE_SIZE as f32)),
    }
}

fn copy_source_sprite_rgba(
    source: &image::RgbaImage,
    frame_size: usize,
    frame_index: usize,
    palette: &Palette,
    mirror_x: bool,
    target: &mut [u8],
) {
    let mut palette_indices = vec![0usize; frame_size * frame_size];
    let mut source_opaque = vec![false; frame_size * frame_size];
    for row in 0..frame_size {
        for col in 0..frame_size {
            let source_col = if mirror_x { frame_size - 1 - col } else { col };
            let source_pixel =
                source.get_pixel(source_col as u32, (frame_index * frame_size + row) as u32);
            let index = row * frame_size + col;
            source_opaque[index] = source_pixel[3] != 0;
            palette_indices[index] = match source_pixel[0] {
                0xff => 0,
                0xaa => 1,
                0x55 => 2,
                0x00 => 3,
                value => palette_index_from_gray(value),
            };
        }
    }

    // Match the TypeScript renderer: only background pixels connected to the
    // frame border are transparent. White pixels enclosed by a character are
    // real artwork (coats, eyes, highlights) and must remain opaque.
    let background_index = palette_indices[0];
    let mut transparent = vec![false; frame_size * frame_size];
    let mut pending = VecDeque::new();
    macro_rules! enqueue {
        ($row:expr, $col:expr) => {{
            let row = $row;
            let col = $col;
            let index = row * frame_size + col;
            if source_opaque[index]
                && palette_indices[index] == background_index
                && !transparent[index]
            {
                transparent[index] = true;
                pending.push_back((row, col));
            }
        }};
    }
    for col in 0..frame_size {
        enqueue!(0, col);
        enqueue!(frame_size - 1, col);
    }
    for row in 1..frame_size.saturating_sub(1) {
        enqueue!(row, 0);
        enqueue!(row, frame_size - 1);
    }
    while let Some((row, col)) = pending.pop_front() {
        if row > 0 {
            enqueue!(row - 1, col);
        }
        if row + 1 < frame_size {
            enqueue!(row + 1, col);
        }
        if col > 0 {
            enqueue!(row, col - 1);
        }
        if col + 1 < frame_size {
            enqueue!(row, col + 1);
        }
    }

    for row in 0..frame_size {
        for col in 0..frame_size {
            let index = row * frame_size + col;
            let offset = index * 4;
            if !source_opaque[index] || transparent[index] {
                target[offset + 3] = 0;
                continue;
            }
            let [red, green, blue] = palette[palette_indices[index]];
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn is_padded_static_sprite_sheet(
    source: &image::RgbaImage,
    frame_size: usize,
    frame_count: usize,
) -> bool {
    frame_count > 1
        && !source_sprite_frame_is_uniform(source, frame_size, 0)
        && (1..frame_count)
            .all(|frame_index| source_sprite_frame_is_uniform(source, frame_size, frame_index))
}

fn source_sprite_frame_is_uniform(
    source: &image::RgbaImage,
    frame_size: usize,
    frame_index: usize,
) -> bool {
    let first = *source.get_pixel(0, (frame_index * frame_size) as u32);
    for row in 0..frame_size {
        for col in 0..frame_size {
            if *source.get_pixel(col as u32, (frame_index * frame_size + row) as u32) != first {
                return false;
            }
        }
    }
    true
}

fn map_sprite_sheet_frames(
    sprite_id: &str,
    source: &image::RgbaImage,
    frame_size: usize,
    frame_count: usize,
    palette: &Palette,
    images: &mut Assets<Image>,
) -> Result<SpriteArt> {
    match frame_count {
        6 => Ok(SpriteArt {
            down: OverworldDirectionArt {
                standing: create_sprite_frame(source, frame_size, 0, palette, false, images),
                walking: Some(create_sprite_frame(
                    source, frame_size, 3, palette, false, images,
                )),
            },
            up: OverworldDirectionArt {
                standing: create_sprite_frame(source, frame_size, 1, palette, false, images),
                walking: Some(create_sprite_frame(
                    source, frame_size, 4, palette, false, images,
                )),
            },
            left: OverworldDirectionArt {
                standing: create_sprite_frame(source, frame_size, 2, palette, false, images),
                walking: Some(create_sprite_frame(
                    source, frame_size, 5, palette, false, images,
                )),
            },
            right: OverworldDirectionArt {
                standing: create_sprite_frame(source, frame_size, 2, palette, true, images),
                walking: Some(create_sprite_frame(
                    source, frame_size, 5, palette, true, images,
                )),
            },
        }),
        3 => Ok(SpriteArt {
            down: static_overworld_direction(source, frame_size, 0, palette, false, images),
            up: static_overworld_direction(source, frame_size, 1, palette, false, images),
            left: static_overworld_direction(source, frame_size, 2, palette, false, images),
            right: static_overworld_direction(source, frame_size, 2, palette, true, images),
        }),
        2 => Ok(SpriteArt {
            down: static_overworld_direction(source, frame_size, 0, palette, false, images),
            up: static_overworld_direction(source, frame_size, 1, palette, false, images),
            left: static_overworld_direction(source, frame_size, 0, palette, false, images),
            right: static_overworld_direction(source, frame_size, 0, palette, true, images),
        }),
        1 => Ok(SpriteArt {
            down: static_overworld_direction(source, frame_size, 0, palette, false, images),
            up: static_overworld_direction(source, frame_size, 0, palette, false, images),
            left: static_overworld_direction(source, frame_size, 0, palette, false, images),
            right: static_overworld_direction(source, frame_size, 0, palette, true, images),
        }),
        count => anyhow::bail!(
            "overworld sprite {} has unsupported frame count {}",
            sprite_id,
            count
        ),
    }
}

fn static_overworld_direction(
    source: &image::RgbaImage,
    frame_size: usize,
    frame_index: usize,
    palette: &Palette,
    mirror_x: bool,
    images: &mut Assets<Image>,
) -> OverworldDirectionArt {
    OverworldDirectionArt {
        standing: create_sprite_frame(source, frame_size, frame_index, palette, mirror_x, images),
        walking: None,
    }
}

fn normalize_tileset_time_of_day(value: &str) -> String {
    let normalized = value.to_ascii_lowercase();
    match normalized.as_str() {
        "morning" | "morn" => "morn".to_string(),
        "night" | "nite" => "nite".to_string(),
        "darkness" | "dark" => "dark".to_string(),
        "indoors" | "indoor" => "indoor".to_string(),
        "" => "day".to_string(),
        _ => normalized,
    }
}

fn parse_palette_file(content: &str, group_filter: Option<&str>) -> Result<Vec<Palette>> {
    let mut current_group = "default".to_string();
    let mut palettes = Vec::new();
    let mut pending: Vec<[u8; 3]> = Vec::new();
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with(';') && !trimmed.to_ascii_uppercase().contains("RGB") {
            current_group = trimmed
                .trim_start_matches(';')
                .trim()
                .to_ascii_lowercase()
                .replace(' ', "_");
            pending.clear();
            continue;
        }
        let line = raw_line.split(';').next().unwrap_or("").trim();
        if !line.to_ascii_uppercase().starts_with("RGB") {
            continue;
        }
        if let Some(group) = group_filter {
            if current_group != group {
                continue;
            }
        }
        let values = parse_rgb_values(line)?;
        match values.len() {
            3 => {
                pending.push(rgb_triplet_to_u8(&values[0..3])?);
                if pending.len() == 4 {
                    palettes.push([pending[0], pending[1], pending[2], pending[3]]);
                    pending.clear();
                }
            }
            12 => palettes.push([
                rgb_triplet_to_u8(&values[0..3])?,
                rgb_triplet_to_u8(&values[3..6])?,
                rgb_triplet_to_u8(&values[6..9])?,
                rgb_triplet_to_u8(&values[9..12])?,
            ]),
            count if count % 3 == 0 => {
                for chunk in values.chunks(3) {
                    pending.push(rgb_triplet_to_u8(chunk)?);
                    if pending.len() == 4 {
                        palettes.push([pending[0], pending[1], pending[2], pending[3]]);
                        pending.clear();
                    }
                }
            }
            _ => anyhow::bail!("malformed RGB palette line '{line}'"),
        }
    }
    Ok(palettes)
}
