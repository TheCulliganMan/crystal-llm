fn validate_render_map_event_coordinates(map: &crate::RuntimeMapCatalogSnapshot) -> Result<()> {
    for warp in &map.events.warps {
        if warp_tile_position_checked(warp).is_none() {
            anyhow::bail!(
                "warp {} on {} has out-of-range runtime coordinates ({}, {})",
                warp.index,
                map.map_name,
                warp.x,
                warp.y
            );
        }
    }
    for bg in &map.events.bg_events {
        if background_event_tile_position_checked(bg).is_none() {
            anyhow::bail!(
                "background event '{}' on {} has out-of-range runtime coordinates ({}, {})",
                bg.script,
                map.map_name,
                bg.x,
                bg.y
            );
        }
    }
    for coord in &map.events.coord_events {
        if coord_event_tile_position_checked(coord).is_none() {
            anyhow::bail!(
                "coord event '{}' on {} has out-of-range runtime coordinates ({}, {})",
                coord.script_name,
                map.map_name,
                coord.x,
                coord.y
            );
        }
    }
    Ok(())
}

fn record_visible_render_error(
    commands: &mut Commands,
    runtime_shell: &mut BevyRuntimeShell,
    error: anyhow::Error,
) {
    record_visible_runtime_system_error(runtime_shell, error);
    spawn_shell_error_banner(commands, runtime_shell);
}

fn player_facing_marker_size(dx: i16, dy: i16) -> Vec2 {
    if dx != 0 {
        Vec2::new(TILE_SIZE * 0.20, TILE_SIZE * 0.46)
    } else if dy != 0 {
        Vec2::new(TILE_SIZE * 0.46, TILE_SIZE * 0.20)
    } else {
        Vec2::splat(TILE_SIZE * 0.20)
    }
}

fn runtime_debug_overlays_enabled() -> bool {
    false
}

fn sync_visible_map_name_sign(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) {
    let landmarks = &snapshot.presentation.pokegear_landmarks;
    let national_park_gate = matches!(
        snapshot.overworld.map_name.as_str(),
        "Route35NationalParkGate" | "Route36NationalParkGate"
    );
    let current = if national_park_gate {
        Some("__MAP_NAME_SIGN_SENTINEL__".to_string())
    } else {
        landmarks
            .map_to_landmark
            .get(&snapshot.overworld.map_name)
            .cloned()
    };
    let previous = runtime_shell.previous_map_sign_landmark.clone();
    runtime_shell.previous_map_sign_landmark = current.clone();
    runtime_shell.visible_map_name_sign = None;

    let Some(landmark) = current else {
        return;
    };
    if previous.as_ref() == Some(&landmark)
        || previous.is_none()
        || previous.as_deref() == Some("LANDMARK_SPECIAL")
        || national_park_gate
        || matches!(
            landmark.as_str(),
            "LANDMARK_RADIO_TOWER"
                | "LANDMARK_LAV_RADIO_TOWER"
                | "LANDMARK_UNDERGROUND_PATH"
                | "LANDMARK_INDIGO_PLATEAU"
                | "LANDMARK_POWER_PLANT"
                | "LANDMARK_SPECIAL"
        )
    {
        return;
    }
    let Some(label) = landmarks
        .landmarks
        .iter()
        .find(|entry| entry.constant == landmark)
        .map(|entry| entry.name.clone())
    else {
        return;
    };
    runtime_shell.visible_map_name_sign = Some(VisibleMapNameSign {
        landmark,
        label,
        frames_remaining: 60,
    });
}

fn spawn_visible_map_name_sign(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    if snapshot.battle.is_some() {
        return Ok(());
    }
    let Some(sign) = runtime_shell.visible_map_name_sign.as_ref() else {
        return Ok(());
    };
    require_bitmap_font_art(rendered_art, asset_root, images)?;
    let palette_key = normalize_tileset_time_of_day(
        snapshot.progression.time.time_of_day.as_key(),
    );
    if !rendered_art.map_name_sign_cache.contains_key(&palette_key)
        && !rendered_art.map_name_sign_errors.contains_key(&palette_key)
    {
        let loaded = load_map_name_sign_frames(asset_root, &palette_key, images);
        match loaded {
            Ok(frames) => {
                rendered_art
                    .map_name_sign_cache
                    .insert(palette_key.clone(), frames);
            }
            Err(error) => {
                rendered_art
                    .map_name_sign_errors
                    .insert(palette_key.clone(), error.to_string());
            }
        }
    }
    let frames = rendered_art
        .map_name_sign_cache
        .get(&palette_key)
        .with_context(|| {
            rendered_art
                .map_name_sign_errors
                .get(&palette_key)
                .cloned()
                .unwrap_or_else(|| format!("map-name sign art is unavailable for {palette_key}"))
        })?;
    let tile_index = |row: usize, col: usize| -> usize {
        match (row, col) {
            (0, 0) => 1,
            (0, 19) => 4,
            (0, _) => if (col - 1) % 4 < 2 { 3 } else { 2 },
            (1, 0) => 5,
            (1, 19) => 11,
            (1, _) => 13,
            (2, 0) => 6,
            (2, 19) => 12,
            (2, _) => 13,
            (3, 0) => 7,
            (3, 19) => 10,
            (3, _) => if (col - 1) % 4 < 2 { 9 } else { 8 },
            _ => unreachable!("map-name sign is exactly four rows"),
        }
    };
    for row in 0..4 {
        for col in 0..20 {
            let frame = &frames[tile_index(row, col)];
            let (x, y) = battle_hud_tile_origin(col as f32, (14 + row) as f32);
            commands.spawn((
                SpriteBundle {
                    texture: frame.handle.clone(),
                    sprite: Sprite { custom_size: Some(frame.size), ..default() },
                    transform: Transform::from_xyz(x, y, 3.2),
                    ..default()
                },
                MapNameSignMarker,
                FieldCommandMarker,
            ));
        }
    }
    let glyphs = bitmap_text_frames(rendered_art, asset_root, images, &sign.label);
    let start_tile = ((20_usize.saturating_sub(glyphs.len())) / 2) as f32;
    for (index, frame) in glyphs.into_iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(start_tile + index as f32, 16.0);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite { custom_size: Some(frame.size), ..default() },
                transform: Transform::from_xyz(x, y, 3.3),
                ..default()
            },
            MapNameSignMarker,
            FieldCommandMarker,
        ));
    }
    Ok(())
}

fn load_map_name_sign_frames(
    asset_root: &AssetRoot,
    time_of_day: &str,
    images: &mut Assets<Image>,
) -> Result<Vec<SpriteFrame>> {
    let path = asset_root
        .runtime_assets()
        .join("gfx/frames/map_entry_sign.png");
    let source = image::open(&path)
        .with_context(|| format!("decode map-name sign PNG {}", path.display()))?
        .to_rgba8();
    if source.dimensions() != (56, 16) {
        anyhow::bail!(
            "map-name sign PNG {} must be 56x16, found {}x{}",
            path.display(),
            source.width(),
            source.height()
        );
    }
    let palettes = load_tileset_palette_bank(asset_root, "__map_name_sign__", time_of_day)?
        .context("map-name sign requires the time-of-day BG palette bank")?;
    let palette = palettes
        .get(7)
        .context("map-name sign requires BG palette 7")?;
    Ok((0..14)
        .map(|index| opaque_palette_tile_frame(&source, index, palette, images))
        .collect())
}

fn opaque_palette_tile_frame(
    source: &image::RgbaImage,
    frame_index: usize,
    palette: &Palette,
    images: &mut Assets<Image>,
) -> SpriteFrame {
    let columns = source.width() as usize / SOURCE_TILE_SIZE;
    let source_x = (frame_index % columns) * SOURCE_TILE_SIZE;
    let source_y = (frame_index / columns) * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
    for row in 0..SOURCE_TILE_SIZE {
        for col in 0..SOURCE_TILE_SIZE {
            let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            let palette_index = match pixel[0] {
                0xff => 0,
                0xaa => 1,
                0x55 => 2,
                0x00 => 3,
                value => palette_index_from_gray(value),
            };
            let offset = (row * SOURCE_TILE_SIZE + col) * 4;
            data[offset..offset + 3].copy_from_slice(&palette[palette_index]);
            data[offset + 3] = 255;
        }
    }
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
    SpriteFrame {
        handle: images.add(image),
        size: Vec2::splat(TILE_SIZE),
    }
}

fn spawn_field_prompt_marker(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    start_x: i16,
    start_y: i16,
) {
    let TilePosition {
        x: front_x,
        y: front_y,
    } = facing_runtime_tile(snapshot).expect("field prompt facing tile must be in runtime bounds");
    let Some((prompt_tile, label)) =
        field_prompt_context_checked(runtime_shell, snapshot, map, front_x, front_y)
            .expect("field prompt coordinates must be valid runtime coordinate state")
    else {
        return;
    };
    let Some((x, y)) = runtime_tile_playfield_position(prompt_tile, start_x, start_y) else {
        return;
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(1.0, 0.92, 0.22, 0.62),
                custom_size: Some(Vec2::new(TILE_SIZE * 0.88, TILE_SIZE * 0.88)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 2.7),
            ..default()
        },
        FieldPromptMarker,
    ));
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 13.0,
                    color: Color::rgb(1.0, 0.98, 0.80),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(x, y + TILE_SIZE * 0.82, 2.9),
            ..default()
        },
        FieldPromptMarker,
    ));
}

fn field_prompt_label(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    front_x: i16,
    front_y: i16,
) -> Option<String> {
    field_prompt_label_checked(runtime_shell, snapshot, map, front_x, front_y)
        .expect("field prompt coordinates must be valid runtime coordinate state")
}

fn field_prompt_label_checked(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    front_x: i16,
    front_y: i16,
) -> Result<Option<String>> {
    Ok(
        field_prompt_context_checked(runtime_shell, snapshot, map, front_x, front_y)?
            .map(|(_, label)| label),
    )
}

fn field_prompt_context(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    front_x: i16,
    front_y: i16,
) -> Option<(TilePosition, String)> {
    field_prompt_context_checked(runtime_shell, snapshot, map, front_x, front_y)
        .expect("field prompt coordinates must be valid runtime coordinate state")
}

fn field_prompt_context_checked(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    front_x: i16,
    front_y: i16,
) -> Result<Option<(TilePosition, String)>> {
    if let Some(interaction) = runtime_shell
        .shell
        .current_overworld_interaction_checked()?
    {
        if interaction.map_name == snapshot.overworld.map_name {
            match &interaction.target {
                crate::core::world::session::OverworldInteractionTarget::Object {
                    object_identifier,
                    ..
                } => {
                    for object in &snapshot.visible_objects {
                        if object_identifier
                            .as_ref()
                            .is_some_and(|id| object.object_identifier.as_ref() == Some(id))
                            || snapshot_object_tile_matches_checked(
                                snapshot,
                                object,
                                interaction.target_tile,
                            )?
                        {
                            return Ok(Some((
                                interaction.target_tile,
                                field_object_prompt_label(object),
                            )));
                        }
                    }
                }
                crate::core::world::session::OverworldInteractionTarget::Background {
                    event_type,
                } => {
                    for bg in &map.events.bg_events {
                        if bg.script == interaction.script
                            && bg.event_type == *event_type
                            && background_event_tile_matches_checked(bg, interaction.target_tile)?
                        {
                            return Ok(Some((interaction.target_tile, field_bg_prompt_label(bg))));
                        }
                    }
                }
                crate::core::world::session::OverworldInteractionTarget::Collision {
                    permission,
                } => {
                    return Ok(Some((
                        interaction.target_tile,
                        format!("collision:{permission:#04x}"),
                    )));
                }
            }
        }
    }
    for object in &snapshot.visible_objects {
        if snapshot_object_tile_matches_checked(
            snapshot,
            object,
            TilePosition::new(front_x, front_y),
        )? {
            return Ok(Some((
                TilePosition::new(front_x, front_y),
                field_object_prompt_label(object),
            )));
        }
    }
    for bg in &map.events.bg_events {
        if background_event_tile_matches_checked(bg, TilePosition::new(front_x, front_y))? {
            return Ok(Some((
                TilePosition::new(front_x, front_y),
                field_bg_prompt_label(bg),
            )));
        }
    }
    for warp in &map.events.warps {
        let Some(tile) = warp_tile_position_checked(warp) else {
            anyhow::bail!(
                "warp {} on {} has out-of-range runtime coordinates ({}, {})",
                warp.index,
                map.map_name,
                warp.x,
                warp.y
            );
        };
        if tile == snapshot.overworld.tile {
            return Ok(Some((
                snapshot.overworld.tile,
                field_warp_prompt_label(warp),
            )));
        }
    }
    Ok(None)
}

fn field_object_prompt_label(object: &crate::core::map::ObjectEvent) -> String {
    let label = object_scene_label(object);
    let kind = match visible_object_kind(object) {
        VisibleObjectKind::ItemBall => "ITEM",
        VisibleObjectKind::Trainer => "TRAINER",
        VisibleObjectKind::Script => "NPC",
        VisibleObjectKind::Invalid => "INVALID",
    };
    compact_scene_label(&format!("A {kind} {label}"), 26)
}

fn field_bg_prompt_label(bg: &crate::core::map::BackgroundEvent) -> String {
    compact_scene_label(&format!("A {} {}", bg.event_type, bg.script), 26)
}

fn field_warp_prompt_label(warp: &crate::core::map::WarpEvent) -> String {
    compact_scene_label(
        &format!("WARP {}#{}", warp.target_map, warp.target_warp_id),
        26,
    )
}

fn spawn_field_command_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    render_error: &mut Option<anyhow::Error>,
) {
    if runtime_shell.field_notice.is_some() && !visible_field_pack_is_open(runtime_shell) {
        if let Err(error) = require_bitmap_font_art(rendered_art, asset_root, images) {
            *render_error = Some(error);
            return;
        }
        spawn_field_notice(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        );
        return;
    }
    let entries = match visible_field_command_entries(snapshot, runtime_shell) {
        Ok(entries) => entries,
        Err(error) => {
            *render_error = Some(error);
            return;
        }
    };
    if entries.is_empty() {
        return;
    }
    if let Err(error) = require_bitmap_font_art(rendered_art, asset_root, images) {
        *render_error = Some(error);
        return;
    }

    if runtime_shell.hall_of_fame_pc_index.is_some() {
        spawn_visible_hall_of_fame_pc(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        );
        return;
    }
    if runtime_shell.visible_card_flip.is_some() {
        if let Err(error) =
            spawn_visible_card_flip(commands, runtime_shell, rendered_art, asset_root, images)
        {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.visible_slot_machine.is_some() {
        if let Err(error) =
            spawn_visible_slot_machine(commands, runtime_shell, rendered_art, asset_root, images)
        {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        if let Err(error) = spawn_visible_unown_puzzle(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        if let Err(error) = spawn_visible_kurt_apricorn_menu(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.start_menu_cursor.is_some() {
        if let Err(error) = spawn_start_menu_command_window(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.options_menu_open {
        if let Err(error) = spawn_options_menu_command_window(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.trainer_card_open {
        spawn_trainer_card_screen(commands, snapshot, runtime_shell, rendered_art, images);
        return;
    }
    if runtime_shell.pokedex_menu_open {
        if let Err(error) = spawn_field_pokedex_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.pokegear_menu_open {
        if let Err(error) = spawn_field_pokegear_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if visible_field_pack_is_open(runtime_shell)
        && runtime_shell.field_pack_target_mode.is_none()
        && runtime_shell.tmhm_teach_prompt_cursor.is_none()
        && runtime_shell.tmhm_decision_prompt_cursor.is_none()
        && !runtime_shell.tmhm_forget_menu_open
    {
        if let Err(error) = spawn_field_pack_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.party_summary_open {
        if let Err(error) = spawn_field_party_summary_screen(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.party_move_reorder_open {
        if let Err(error) = spawn_field_move_reorder_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.party_action_cursor.is_some() {
        if let Err(error) = spawn_field_party_menu(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            let (x, y) = battle_hud_tile_origin(1.0, 15.0);
            spawn_field_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &compact_scene_label(&format!("INVALID PARTY {error:#}"), 18),
                x,
                y,
                4.2,
            );
            return;
        }
        if let Err(error) = spawn_field_party_action_window(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.party_give_take_cursor.is_some() {
        if let Err(error) = spawn_field_party_menu(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            let (x, y) = battle_hud_tile_origin(1.0, 15.0);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &compact_scene_label(&format!("INVALID PARTY {error:#}"), 18), x, y, 4.2,
            );
            return;
        }
        if let Err(error) = spawn_field_party_give_take_window(
            commands, runtime_shell, rendered_art, asset_root, images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.party_switch_cursor.is_some() {
        if let Err(error) = spawn_field_party_menu(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            let (x, y) = battle_hud_tile_origin(1.0, 15.0);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &compact_scene_label(&format!("INVALID PARTY {error:#}"), 18), x, y, 4.2,
            );
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.fly_cursor.is_some() {
        if let Err(error) = spawn_field_fly_map_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        ) {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.party_menu_open
        && !runtime_shell.party_summary_open
        && !runtime_shell.party_move_reorder_open
        && runtime_shell.party_action_cursor.is_none()
        && runtime_shell.party_give_take_cursor.is_none()
        && runtime_shell.party_switch_cursor.is_none()
        && runtime_shell.fly_cursor.is_none()
    {
        if let Err(error) = spawn_field_party_menu(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        ) {
            commands.spawn((
                Text2dBundle {
                    text: Text::from_section(error.to_string(), TextStyle::default()),
                    transform: Transform::from_xyz(0.0, 0.0, 4.0),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
        spawn_pc_notice(commands, runtime_shell, rendered_art, asset_root, images);
        return;
    }
    let origin_x = PLAYFIELD_LEFT + TILE_SIZE * 11.0;
    let origin_y = PLAYFIELD_TOP - TILE_SIZE * 13.8;
    let two_columns = scene_menu_uses_two_columns(&entries);
    let panel_height = if two_columns {
        TILE_SIZE * 3.0
    } else {
        TILE_SIZE * (entries.len().max(1) as f32 * 0.66 + 1.0)
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.10, 0.11, 0.10),
                custom_size: Some(Vec2::new(TILE_SIZE * 7.35, panel_height + TILE_SIZE * 0.18)),
                ..default()
            },
            transform: Transform::from_xyz(origin_x + TILE_SIZE * 2.5, origin_y, 3.3),
            ..default()
        },
        FieldCommandMarker,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(TILE_SIZE * 7.0, panel_height - TILE_SIZE * 0.16)),
                ..default()
            },
            transform: Transform::from_xyz(origin_x + TILE_SIZE * 2.5, origin_y, 3.35),
            ..default()
        },
        FieldCommandMarker,
    ));
    for (index, entry) in entries.iter().take(FIELD_TEXT_BOX_VISIBLE_ROWS).enumerate() {
        let (x, y) = scene_menu_entry_position(origin_x, origin_y, index, two_columns);
        let display_entry = scene_menu_display_entry(entry, two_columns);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &display_entry,
            x,
            y,
            3.6,
        );
    }
}

fn spawn_visible_hall_of_fame_pc(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    let Some(boundary) = runtime_shell
        .special_boundary
        .as_ref()
        .filter(|boundary| boundary.label == "HallOfFamePC")
    else {
        return;
    };
    let (center_x, center_y) = field_window_center(0.0, 0.0, 20.0, 18.0);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.97, 0.97, 0.97),
                custom_size: Some(Vec2::new(TILE_SIZE * 20.0, TILE_SIZE * 18.0)),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 5.8),
            ..default()
        },
        FieldCommandMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(commands, frame, 0.0, 0.0, 20, 9, 5.9);
    }
    for (row, line) in boundary.details.iter().take(7).enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 1.0 + row as f32);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            line,
            x,
            y,
            6.0,
        );
    }
}

fn spawn_visible_slot_machine(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(machine) = runtime_shell.visible_slot_machine.as_ref() else { return Ok(()); };
    let frame = load_visible_slot_machine_frame(asset_root, machine, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(Vec2::new(TILE_SIZE * 20.0, TILE_SIZE * 18.0)),
                ..default()
            },
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + TILE_SIZE * 9.5,
                PLAYFIELD_TOP - TILE_SIZE * 8.5,
                3.3,
            ),
            ..default()
        },
        FieldCommandMarker,
    ));
    for (text, tile_x, tile_y) in [
        (format!("{:04}", machine.coins), 5.0, 1.0),
        (format!("{:04}", machine.payout), 11.0, 1.0),
        (machine.message.clone(), 1.0, 15.0),
        (format!("BET {} A:SPIN B:QUIT", machine.bet), 1.0, 17.0),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, &text, x, y, 3.7);
    }
    Ok(())
}

fn spawn_visible_card_flip(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(game) = runtime_shell.visible_card_flip.as_ref() else { return Ok(()); };
    let frame = load_visible_card_flip_frame(asset_root, game, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(Vec2::new(TILE_SIZE * 20.0, TILE_SIZE * 18.0)),
                ..default()
            },
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + TILE_SIZE * 9.5,
                PLAYFIELD_TOP - TILE_SIZE * 8.5,
                3.3,
            ),
            ..default()
        },
        FieldCommandMarker,
    ));
    let (message_x, message_y) = battle_hud_tile_origin(1.0, 13.0);
    spawn_field_command_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &game.message,
        message_x,
        message_y,
        3.6,
    );
    let (coin_x, coin_y) = battle_hud_tile_origin(10.0, 16.0);
    spawn_field_command_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &format!("COIN {:04}", game.coins),
        coin_x,
        coin_y,
        3.6,
    );
    if matches!(game.phase, VisibleCardFlipPhase::ChooseCard) {
        for index in 0..2 {
            let label = if index == game.which_card { ">[CARD]<" } else { " [CARD] " };
            let (x, y) = battle_hud_tile_origin(1.0, 2.0 + index as f32 * 6.0);
            spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, &label, x, y, 3.6);
        }
    } else if matches!(game.phase, VisibleCardFlipPhase::PlaceBet) {
        let (x, y) = battle_hud_tile_origin(
            9.0 + game.bet_x as f32 * 1.75,
            game.bet_y as f32 * 1.45,
        );
        spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, ">", x, y, 3.8);
    } else if matches!(
        game.phase,
        VisibleCardFlipPhase::AskPlay | VisibleCardFlipPhase::PlayAgain
    ) {
        for (index, option) in ["YES", "NO"].iter().enumerate() {
            let label = if index == game.yes_no_index {
                format!(">{option}")
            } else {
                format!(" {option}")
            };
            let (x, y) = battle_hud_tile_origin(13.0, 8.0 + index as f32 * 2.0);
            spawn_field_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &label,
                x,
                y,
                3.6,
            );
        }
    }
    let (x, y) = battle_hud_tile_origin(1.0, 15.0);
    let instructions = match game.phase {
        VisibleCardFlipPhase::ChooseCard => "A FLIP",
        VisibleCardFlipPhase::PlaceBet => "A BET",
        VisibleCardFlipPhase::AskPlay | VisibleCardFlipPhase::PlayAgain => "A OK   B NO",
        VisibleCardFlipPhase::NotEnoughCoins => "A OK",
    };
    spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, instructions, x, y, 3.6);
    Ok(())
}

fn load_visible_card_flip_frame(
    asset_root: &AssetRoot,
    game: &VisibleCardFlip,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    let root = asset_root.resolve_vendor("gfx/card_flip");
    let sheet_1 = image::open(root.join("card_flip_1.png"))
        .context("decode Card Flip primary sheet")?
        .to_rgba8();
    let sheet_2 = image::open(root.join("card_flip_2.png"))
        .context("decode Card Flip secondary sheet")?
        .to_rgba8();
    let light_off = image::open(root.join("off.png"))
        .context("decode Card Flip off light")?
        .to_rgba8();
    let light_on = image::open(root.join("on.png"))
        .context("decode Card Flip on light")?
        .to_rgba8();
    anyhow::ensure!(sheet_1.dimensions() == (128, 32), "invalid Card Flip primary sheet");
    anyhow::ensure!(sheet_2.dimensions() == (24, 160), "invalid Card Flip secondary sheet");
    anyhow::ensure!(light_off.dimensions() == (8, 8), "invalid Card Flip off light");
    anyhow::ensure!(light_on.dimensions() == (8, 8), "invalid Card Flip on light");
    let tilemap = std::fs::read(root.join("card_flip.tilemap"))
        .context("read Card Flip tilemap")?;
    anyhow::ensure!(tilemap.len() == 12 * 11, "invalid Card Flip tilemap length");
    let palettes = load_card_flip_palettes(&root.join("card_flip.pal"))?;
    let [red, green, blue] = palettes[0][2];
    let mut target = vec![0_u8; WIDTH * HEIGHT * 4];
    for pixel in target.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[red, green, blue, 255]);
    }
    for tile_y in 0..18 {
        for tile_x in 0..9 {
            blit_paletted_slot_tile(
                &sheet_1,
                0x29,
                &palettes[0],
                tile_x * TILE,
                tile_y * TILE,
                false,
                &mut target,
            );
        }
    }
    for (offset, tile_id) in tilemap.into_iter().enumerate() {
        let linear_tile = (offset / 12) * 20 + 9 + offset % 12;
        let tile_x = linear_tile % 20;
        let tile_y = linear_tile / 20;
        let palette_index = card_flip_background_palette(tile_x, tile_y);
        let (sheet, source_tile) = match tile_id {
            0xef => (&light_off, 0),
            0x00..=0x3d => (&sheet_1, usize::from(tile_id)),
            0x3e..=0x79 => (&sheet_2, usize::from(tile_id - 0x3e)),
            _ => anyhow::bail!("Card Flip tilemap references unsupported tile {tile_id:#04x}"),
        };
        blit_paletted_slot_tile(
            sheet,
            source_tile,
            &palettes[palette_index],
            tile_x * TILE,
            tile_y * TILE,
            false,
            &mut target,
        );
    }
    let completed_rounds = game.revealed.iter().filter(|flag| **flag).count().min(12);
    for round in 0..completed_rounds {
        let linear_tile = 9 + round;
        blit_paletted_slot_tile(
            &light_on,
            0,
            &palettes[card_flip_background_palette(linear_tile % 20, linear_tile / 20)],
            (linear_tile % 20) * TILE,
            (linear_tile / 20) * TILE,
            false,
            &mut target,
        );
    }
    let mut image = Image::new(
        Extent3d { width: WIDTH as u32, height: HEIGHT as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        target,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::new(WIDTH as f32, HEIGHT as f32) })
}

fn load_card_flip_palettes(path: &Path) -> Result<Vec<Palette>> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("read Card Flip palette {}", path.display()))?;
    let colors = source
        .lines()
        .filter_map(|raw_line| {
            let line = raw_line.split(';').next().unwrap_or("").trim();
            line.to_ascii_uppercase().starts_with("RGB").then_some(line)
        })
        .map(|line| rgb_triplet_to_u8(&parse_rgb_values(line)?))
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(colors.len() == 36, "Card Flip palette must contain 36 colors");
    Ok(colors
        .chunks_exact(4)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect())
}

fn card_flip_background_palette(tile_x: usize, tile_y: usize) -> usize {
    if tile_y == 0 && tile_x >= 9 {
        return 1;
    }
    if (1..3).contains(&tile_y) {
        match tile_x {
            12..=13 => return 1,
            14..=15 => return 2,
            16..=17 => return 3,
            18..=19 => return 4,
            _ => {}
        }
    }
    0
}

fn compact_card_flip_symbol(symbol: &str) -> &'static str {
    match symbol {
        "ODDISH" => "O",
        "POLIWAG" => "W",
        "PIKACHU" => "P",
        "JIGGLYPUFF" => "J",
        "RATTATA" => "R",
        "VOLTORB" => "V",
        _ => "?",
    }
}

fn load_visible_slot_machine_frame(
    asset_root: &AssetRoot,
    machine: &VisibleSlotMachine,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    let root = asset_root.resolve_vendor("gfx/slots");
    let ui = image::open(root.join("slots_1.png")).context("decode slots UI sheet")?.to_rgba8();
    let symbols = image::open(root.join("slots_2.png")).context("decode slots symbol sheet")?.to_rgba8();
    anyhow::ensure!(ui.dimensions() == (16, 152), "invalid slots UI sheet dimensions");
    anyhow::ensure!(symbols.dimensions() == (16, 256), "invalid slots symbol sheet dimensions");
    let tilemap = std::fs::read(root.join("slots.tilemap")).context("read slots tilemap")?;
    anyhow::ensure!(tilemap.len() == 20 * 12, "invalid slots tilemap length");
    let palettes = load_slot_machine_palettes(&root.join("slots.pal"))?;
    let mut target = vec![255_u8; WIDTH * HEIGHT * 4];

    for (offset, tile_id) in tilemap.into_iter().enumerate() {
        let tile_x = offset % 20;
        let tile_y = offset / 20;
        let palette = slot_machine_background_palette(tile_x, tile_y);
        let (sheet, source_tile) = if tile_id < 0x25 {
            (&ui, usize::from(tile_id))
        } else {
            (&symbols, usize::from(tile_id - 0x25))
        };
        blit_paletted_slot_tile(
            sheet, source_tile, &palettes[palette], tile_x * TILE, tile_y * TILE, false, &mut target,
        );
    }
    for reel in 0..3 {
        for row in 0..3 {
            let symbol_index = slot_symbol_palette_index(&machine.windows[reel][2 - row])?;
            let base_tile = symbol_index * 4;
            for icon_y in 0..2 {
                for icon_x in 0..2 {
                    blit_paletted_slot_tile(
                        &symbols,
                        base_tile + icon_y * 2 + icon_x,
                        &palettes[symbol_index],
                        [5, 9, 13][reel] * TILE + icon_x * TILE,
                        [4, 6, 8][row] * TILE + icon_y * TILE,
                        true,
                        &mut target,
                    );
                }
            }
        }
    }
    let mut image = Image::new(
        Extent3d { width: WIDTH as u32, height: HEIGHT as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        target,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::new(WIDTH as f32, HEIGHT as f32) })
}

fn load_slot_machine_palettes(path: &Path) -> Result<Vec<Palette>> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("read slot palette {}", path.display()))?;
    let mut colors = Vec::new();
    for raw_line in source.lines() {
        let line = raw_line.split(';').next().unwrap_or("").trim();
        if !line.to_ascii_uppercase().starts_with("RGB") { continue; }
        colors.push(rgb_triplet_to_u8(&parse_rgb_values(line)?)?);
    }
    anyhow::ensure!(colors.len() == 64, "slot palette must contain 64 colors");
    Ok(colors.chunks_exact(4).map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]]).collect())
}

fn slot_machine_background_palette(tile_x: usize, tile_y: usize) -> usize {
    let mut palette = 0;
    for (x, y, width, height, value) in [
        (0, 2, 3, 10, 2), (17, 2, 3, 10, 2), (0, 4, 3, 6, 3),
        (17, 4, 3, 6, 3), (0, 6, 3, 2, 4), (17, 6, 3, 2, 4),
        (4, 2, 12, 2, 1), (3, 2, 1, 10, 1), (16, 2, 1, 10, 1),
    ] {
        if tile_x >= x && tile_x < x + width && tile_y >= y && tile_y < y + height {
            palette = value;
        }
    }
    if (12..18).contains(&tile_y) { 7 } else { palette }
}

fn slot_symbol_palette_index(symbol: &str) -> Result<usize> {
    match symbol {
        "SEVEN" => Ok(0),
        "POKEBALL" => Ok(1),
        "CHERRY" => Ok(2),
        "PIKACHU" => Ok(3),
        "SQUIRTLE" => Ok(4),
        "STARYU" => Ok(5),
        _ => anyhow::bail!("unknown slot symbol {symbol}"),
    }
}

fn blit_paletted_slot_tile(
    source: &image::RgbaImage,
    tile_index: usize,
    palette: &Palette,
    dest_x: usize,
    dest_y: usize,
    transparent_zero: bool,
    target: &mut [u8],
) {
    const TARGET_WIDTH: usize = 160;
    let columns = source.width() as usize / 8;
    let source_x = (tile_index % columns) * 8;
    let source_y = (tile_index / columns) * 8;
    for y in 0..8 {
        for x in 0..8 {
            let pixel = source.get_pixel((source_x + x) as u32, (source_y + y) as u32);
            let palette_index = palette_index_from_gray(pixel[0]);
            if transparent_zero && palette_index == 0 { continue; }
            let [red, green, blue] = palette[palette_index];
            let offset = ((dest_y + y) * TARGET_WIDTH + dest_x + x) * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn spawn_visible_unown_puzzle(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    _rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(puzzle) = runtime_shell.visible_unown_puzzle.as_ref() else {
        return Ok(());
    };
    let frame = load_visible_unown_puzzle_frame(asset_root, puzzle, images)?;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(Vec2::new(TILE_SIZE * 20.0, TILE_SIZE * 18.0)),
                ..default()
            },
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + TILE_SIZE * 9.5,
                PLAYFIELD_TOP - TILE_SIZE * 8.5,
                3.6,
            ),
            ..default()
        },
        FieldCommandMarker,
    ));
    Ok(())
}

fn load_visible_unown_puzzle_frame(
    asset_root: &AssetRoot,
    puzzle: &VisibleUnownPuzzle,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    const PIECE: usize = 24;
    let root = asset_root.resolve_vendor("gfx/unown_puzzle");
    let puzzle_path = root.join(format!("{}.png", puzzle.puzzle_id.to_ascii_lowercase()));
    let source = image::open(&puzzle_path)
        .with_context(|| format!("decode Unown puzzle PNG {}", puzzle_path.display()))?
        .to_rgba8();
    let border = image::open(root.join("tile_borders.png"))
        .context("decode Unown puzzle piece borders")?
        .to_rgba8();
    let mut cursor = image::open(root.join("cursor.png"))
        .context("decode Unown puzzle cursor")?
        .to_rgba8();
    let start_cancel = image::open(root.join("start_cancel.png"))
        .context("decode Unown puzzle START/CANCEL graphics")?
        .to_rgba8();
    anyhow::ensure!(source.dimensions() == (48, 48), "invalid Unown puzzle dimensions");
    anyhow::ensure!(border.dimensions() == (64, 8), "invalid Unown border dimensions");
    anyhow::ensure!(cursor.dimensions() == (16, 16), "invalid Unown cursor dimensions");
    anyhow::ensure!(start_cancel.dimensions() == (152, 8), "invalid Unown START/CANCEL dimensions");
    // OBJ palette colour zero is transparent on the Game Boy. The source PNG
    // represents it as opaque white because PNG has no OBJ/BG distinction.
    for pixel in cursor.pixels_mut() {
        if pixel[0] >= 248 && pixel[1] >= 248 && pixel[2] >= 248 {
            pixel[3] = 0;
        }
    }

    let mut target = vec![248_u8; WIDTH * HEIGHT * 4];
    for pixel in target.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    let draw_tile = |tile: usize, x: usize, y: usize, target: &mut [u8]| {
        blit_unown_rgba(&start_cancel, tile * TILE, 0, TILE, TILE, x, y, false, false, target);
    };
    // The ASM fills the LCD with PUZZLE_BORDER ($ee), then carves the 12x12
    // center from PUZZLE_VOID ($ef). Both tiles live immediately after the
    // start/cancel base tile in the shipped source strip.
    for tile_y in 0..18 {
        for tile_x in 0..20 {
            draw_tile(1, tile_x * TILE, tile_y * TILE, &mut target);
        }
    }
    for tile_y in 3..15 {
        for tile_x in 4..16 {
            draw_tile(2, tile_x * TILE, tile_y * TILE, &mut target);
        }
    }

    let mut pieces = Vec::with_capacity(16);
    for piece_index in 0..16 {
        let mut piece = image::RgbaImage::new(PIECE as u32, PIECE as u32);
        let source_x = (piece_index % 4) * 12;
        let source_y = (piece_index / 4) * 12;
        for y in 0..PIECE {
            for x in 0..PIECE {
                piece.put_pixel(x as u32, y as u32, *source.get_pixel((source_x + x / 2) as u32, (source_y + y / 2) as u32));
            }
        }
        for (border_index, (x, y)) in [
            (0, 0), (8, 0), (16, 0), (0, 8), (16, 8), (0, 16), (8, 16), (16, 16),
        ].into_iter().enumerate() {
            overlay_unown_border(&border, border_index * TILE, &mut piece, x, y);
        }
        pieces.push(piece);
    }

    for y in 0..6 {
        for x in 0..6 {
            let dest_x = TILE + x * PIECE;
            let dest_y = y * PIECE;
            match puzzle.layout[y][x] {
                1..=16 => blit_unown_rgba(
                    &pieces[usize::from(puzzle.layout[y][x] - 1)], 0, 0, PIECE, PIECE,
                    dest_x, dest_y, false, false, &mut target,
                ),
                _ => {
                    let vacant_tile = if (1..=4).contains(&x) && (1..=4).contains(&y) { 2 } else { 1 };
                    for block_y in 0..3 {
                        for block_x in 0..3 {
                            draw_tile(vacant_tile, dest_x + block_x * TILE, dest_y + block_y * TILE, &mut target);
                        }
                    }
                }
            }
        }
    }

    // Source tilemap box at (4,15), including the exact START/CANCEL lettering.
    draw_tile(3, 32, 120, &mut target);
    for x in 5..15 { draw_tile(4, x * TILE, 120, &mut target); }
    draw_tile(5, 120, 120, &mut target);
    draw_tile(6, 32, 128, &mut target);
    if !puzzle.solved {
        for offset in 0..10 { draw_tile(9 + offset, (5 + offset) * TILE, 128, &mut target); }
    }
    draw_tile(6, 120, 128, &mut target);
    draw_tile(7, 32, 136, &mut target);
    for x in 5..15 { draw_tile(4, x * TILE, 136, &mut target); }
    draw_tile(8, 120, 136, &mut target);

    let cursor_x = TILE + puzzle.cursor_x * PIECE;
    let cursor_y = puzzle.cursor_y * PIECE;
    if let Some(piece) = puzzle.holding_piece.filter(|piece| (1..=16).contains(piece)) {
        blit_unown_rgba(&pieces[usize::from(piece - 1)], 0, 0, PIECE, PIECE, cursor_x, cursor_y, false, false, &mut target);
    } else {
        let cursor_tiles = [
            (0, false, false), (1, false, false), (0, true, false),
            (2, false, false), (3, false, false), (2, true, false),
            (0, false, true), (1, false, true), (0, true, true),
        ];
        for (index, (tile, flip_x, flip_y)) in cursor_tiles.into_iter().enumerate() {
            blit_unown_rgba(
                &cursor, (tile % 2) * TILE, (tile / 2) * TILE, TILE, TILE,
                cursor_x + (index % 3) * TILE, cursor_y + (index / 3) * TILE,
                flip_x, flip_y, &mut target,
            );
        }
    }

    let mut image = Image::new(
        Extent3d { width: WIDTH as u32, height: HEIGHT as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        target,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::new(WIDTH as f32, HEIGHT as f32) })
}

fn overlay_unown_border(
    border: &image::RgbaImage,
    source_x: usize,
    piece: &mut image::RgbaImage,
    dest_x: usize,
    dest_y: usize,
) {
    for y in 0..8 {
        for x in 0..8 {
            let overlay = border.get_pixel((source_x + x) as u32, y as u32);
            let base = piece.get_pixel_mut((dest_x + x) as u32, (dest_y + y) as u32);
            for channel in 0..3 { base[channel] = base[channel].min(overlay[channel]); }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_unown_rgba(
    source: &image::RgbaImage,
    source_x: usize,
    source_y: usize,
    width: usize,
    height: usize,
    dest_x: usize,
    dest_y: usize,
    flip_x: bool,
    flip_y: bool,
    target: &mut [u8],
) {
    const TARGET_WIDTH: usize = 160;
    const TARGET_HEIGHT: usize = 144;
    for y in 0..height {
        for x in 0..width {
            let sx = source_x + if flip_x { width - 1 - x } else { x };
            let sy = source_y + if flip_y { height - 1 - y } else { y };
            let dx = dest_x + x;
            let dy = dest_y + y;
            if dx >= TARGET_WIDTH || dy >= TARGET_HEIGHT { continue; }
            let pixel = source.get_pixel(sx as u32, sy as u32);
            if pixel[3] == 0 { continue; }
            let offset = (dy * TARGET_WIDTH + dx) * 4;
            target[offset..offset + 4].copy_from_slice(&pixel.0);
        }
    }
}

fn spawn_visible_kurt_apricorn_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let choices = visible_kurt_apricorn_choices(snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.kurt_apricorn_cursor,
        "script:kurt-apricorn",
        choices.len(),
    )
    .with_context(|| {
        format!(
            "Kurt Apricorn cursor is invalid for {} choices",
            choices.len()
        )
    })?;
    let (left, top, width, height) = if runtime_shell.kurt_apricorn_quantity.is_some() {
        (6.0, 9.0, 14.0, 4.0)
    } else {
        (1.0, 1.0, 13.0, 10.0)
    };
    let (center_x, center_y) = field_window_center(left, top, width, height);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(TILE_SIZE * (width - 2.0), TILE_SIZE * (height - 2.0))),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.1),
            ..default()
        },
        FieldCommandMarker,
    ));
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("Kurt Apricorn menu requires window-frame art")?;
    spawn_field_command_window_frame_tiles(
        commands,
        frame,
        left,
        top,
        width as usize,
        height as usize,
        4.2,
    );
    if let Some(quantity) = runtime_shell.kurt_apricorn_quantity {
        let (item_id, _) = choices
            .get(selected)
            .context("selected Kurt Apricorn choice is missing")?;
        let (x, y) = battle_hud_tile_origin(7.0, 10.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &item_display_name(snapshot, item_id),
            x,
            y,
            4.3,
        );
        let (x, y) = battle_hud_tile_origin(16.0, 11.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("×{quantity:02}"),
            x,
            y,
            4.3,
        );
        return Ok(());
    }
    let first = selected.saturating_sub(3).min(choices.len().saturating_sub(4));
    for (row, (index, (item_id, quantity))) in choices
        .iter()
        .enumerate()
        .skip(first)
        .take(4)
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(2.0, 2.0 + row as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{}",
                if index == selected { ">" } else { " " },
                item_display_name(snapshot, item_id)
            ),
            x,
            y,
            4.3,
        );
        let (x, y) = battle_hud_tile_origin(10.0, 2.0 + row as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("×{:02}", (*quantity).min(99)),
            x,
            y,
            4.3,
        );
    }
    Ok(())
}

fn spawn_field_pokedex_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let selected = runtime_shell.pokedex_cursor;
    let species = snapshot.pokemon.get(selected).with_context(|| {
        format!(
            "Pokédex cursor {selected} is outside {} species",
            snapshot.pokemon.len()
        )
    })?;
    let scripted_capture_entry = runtime_shell.pokedex_scripted_entry
        && runtime_shell.pending_standard_capture.is_some();
    let seen = scripted_capture_entry
        || snapshot.progression.pokedex_seen_species.contains(&species.species_id);
    let caught = scripted_capture_entry
        || snapshot.progression.pokedex_caught_species.contains(&species.species_id);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.12, 0.18, 0.24),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    if runtime_shell.pokedex_detail_open && seen {
        spawn_field_pokedex_detail(
            commands, snapshot, runtime_shell, species, caught, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    for (row, text) in [
        (10.0, "SEEN".to_string()),
        (11.0, format!("{:03}", snapshot.progression.pokedex_seen)),
        (13.0, "OWN".to_string()),
        (14.0, format!("{:03}", snapshot.progression.pokedex_owned)),
        (16.0, "SELECT>OPTION".to_string()),
    ] {
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, &text, x, y, 3.8,
        );
    }
    if seen {
        if let Some(frame) = pokemon_frame_for_art(
            rendered_art, asset_root, &species.species_id,
            PokemonSpriteSide::Front, false, images,
        ) {
            let (x, y) = battle_hud_tile_origin(3.5, 4.0);
            commands.spawn((
                SpriteBundle {
                    texture: frame.handle,
                    sprite: Sprite { custom_size: Some(frame.size), ..default() },
                    transform: Transform::from_xyz(x, y, 3.8),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
    } else {
        let (x, y) = battle_hud_tile_origin(3.0, 4.0);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, "?", x, y, 3.8,
        );
    }
    let scroll = visible_window_start(selected, snapshot.pokemon.len(), 7);
    for (visible_index, entry) in snapshot.pokemon.iter().skip(scroll).take(7).enumerate() {
        let index = scroll + visible_index;
        let entry_seen = snapshot.progression.pokedex_seen_species.contains(&entry.species_id);
        let entry_caught = snapshot.progression.pokedex_caught_species.contains(&entry.species_id);
        let name = if entry_seen {
            crate::core::models::pokemon_species_display_name(&entry.species_id)
        } else {
            "-----".to_string()
        };
        let (x, y) = battle_hud_tile_origin(9.0, 1.0 + visible_index as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{}{}{:03} {}", if index == selected { ">" } else { " " }, if entry_caught { "C" } else { " " }, entry.int_id, compact_scene_label(&name, 8)),
            x, y, 3.8,
        );
    }
    Ok(())
}

fn spawn_field_pokegear_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    match runtime_shell.pokegear_page {
        PokegearPage::Clock => {}
        PokegearPage::Map => anyhow::ensure!(
            snapshot.progression.active_engine_flags.contains("ENGINE_MAP_CARD"),
            "Pokégear MAP card is selected before it is unlocked"
        ),
        PokegearPage::Phone => anyhow::ensure!(
            snapshot.progression.active_engine_flags.contains("ENGINE_PHONE_CARD"),
            "Pokégear PHONE card is selected before it is unlocked"
        ),
        PokegearPage::Radio => anyhow::ensure!(
            snapshot.progression.active_engine_flags.contains("ENGINE_RADIO_CARD"),
            "Pokégear RADIO card is selected before it is unlocked"
        ),
    }
    if runtime_shell.pokegear_page == PokegearPage::Phone {
        let contacts = visible_pokegear_phone_contact_ids(snapshot);
        if !contacts.is_empty() {
            anyhow::ensure!(
                runtime_shell.pokegear_phone_cursor < contacts.len(),
                "Pokégear phone cursor {} is outside {} contacts",
                runtime_shell.pokegear_phone_cursor,
                contacts.len()
            );
        }
    }
    if runtime_shell.pokegear_page == PokegearPage::Radio {
        if let Some(station) = runtime_shell.pokegear_radio_station.as_deref() {
            let transcript = visible_map_radio_transcript(station);
            let label = transcript.get(runtime_shell.pokegear_radio_segment).with_context(|| {
                format!(
                    "Pokégear radio segment {} is outside {} transcript segments for {station}",
                    runtime_shell.pokegear_radio_segment,
                    transcript.len()
                )
            })?;
            snapshot
                .presentation
                .asm_text
                .get(*label)
                .with_context(|| format!("Pokégear radio transcript text {label} is missing"))?;
        } else {
            anyhow::ensure!(
                runtime_shell.pokegear_radio_index < VISIBLE_POKEGEAR_RADIO_FREQUENCIES.len(),
                "Pokégear radio index {} is outside {} frequencies",
                runtime_shell.pokegear_radio_index,
                VISIBLE_POKEGEAR_RADIO_FREQUENCIES.len()
            );
        }
    }
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.90, 0.82, 0.55),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    let cards = [
        (PokegearPage::Clock, "CLOCK"),
        (PokegearPage::Map, "MAP"),
        (PokegearPage::Phone, "PHONE"),
        (PokegearPage::Radio, "RADIO"),
    ];
    for (index, (page, label)) in cards.iter().enumerate() {
        let unlocked = match page {
            PokegearPage::Clock => true,
            PokegearPage::Map => snapshot.progression.active_engine_flags.contains("ENGINE_MAP_CARD"),
            PokegearPage::Phone => snapshot.progression.active_engine_flags.contains("ENGINE_PHONE_CARD"),
            PokegearPage::Radio => snapshot.progression.active_engine_flags.contains("ENGINE_RADIO_CARD"),
        };
        if !unlocked {
            continue;
        }
        let (x, y) = battle_hud_tile_origin(1.0 + index as f32 * 4.5, 0.5);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{}{}", if *page == runtime_shell.pokegear_page { ">" } else { " " }, label),
            x, y, 3.8,
        );
    }
    if runtime_shell.pokegear_page == PokegearPage::Map {
        let landmarks = &snapshot.presentation.pokegear_landmarks.landmarks;
        let selected = runtime_shell.pokegear_cursor;
        let landmark = selected_pokegear_landmark(snapshot, runtime_shell.pokegear_cursor)?;
        let (x, y) = battle_hud_tile_origin(9.0, 0.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(&landmark.name.replace('\n', " "), 11),
            x,
            y,
            3.8,
        );
        for index in visible_pokegear_landmark_indices(snapshot) {
            let landmark = &landmarks[index];
            let x = (landmark.x as f32 - 8.0).clamp(0.0, 159.0);
            let y = (landmark.y as f32 - 16.0).clamp(0.0, 143.0);
            let is_current = snapshot.presentation.pokegear_landmarks.map_to_landmark
                .get(&snapshot.overworld.map_name).is_some_and(|id| id == &landmark.constant);
            let is_selected = index == selected;
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: if is_selected { Color::rgb(0.95, 0.12, 0.12) } else if is_current { Color::rgb(0.15, 0.35, 0.95) } else { Color::rgb(0.35, 0.55, 0.25) },
                        custom_size: Some(Vec2::splat(if is_selected { 5.0 } else { 2.0 })),
                        ..default()
                    },
                    transform: Transform::from_xyz(PLAYFIELD_LEFT + x, PLAYFIELD_TOP - y, 3.7),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
    }
    let entries = visible_pokegear_menu_entries(snapshot, runtime_shell);
    let start_row = if runtime_shell.pokegear_page == PokegearPage::Map { 13.0 } else { 4.0 };
    for (index, line) in entries.iter().take(10).enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, start_row + index as f32);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &compact_scene_label(line, 18), x, y, 3.8,
        );
    }
    Ok(())
}

fn spawn_field_fly_map_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.68, 0.82, 0.58),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    let destinations = active_fly_destinations(snapshot, &runtime_shell.shell);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    )
    .with_context(|| {
        format!(
            "FLY destination cursor is invalid for {} destinations",
            destinations.len()
        )
    })?;
    for (index, destination) in destinations.iter().enumerate() {
        let landmark = snapshot
            .presentation
            .pokegear_landmarks
            .landmarks
            .iter()
            .find(|landmark| landmark.constant == destination.label)
            .with_context(|| {
                format!(
                    "FLY destination {} has no Pokégear landmark",
                    destination.label
                )
            })?;
        let x = (landmark.x as f32 - 8.0).clamp(0.0, 159.0);
        let y = (landmark.y as f32 - 16.0).clamp(0.0, 143.0);
        let is_selected = selected == index;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: if is_selected { Color::rgb(0.95, 0.12, 0.12) } else { Color::rgb(0.18, 0.42, 0.82) },
                    custom_size: Some(Vec2::splat(if is_selected { 6.0 } else { 3.0 })),
                    ..default()
                },
                transform: Transform::from_xyz(PLAYFIELD_LEFT + x, PLAYFIELD_TOP - y, 3.7),
                ..default()
            },
            FieldCommandMarker,
        ));
    }
    let label = destinations
        .get(selected)
        .map(fly_destination_label)
        .context("selected FLY destination is missing")?;
    let (x, y) = battle_hud_tile_origin(1.0, 15.0);
    spawn_field_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!(">{}", compact_scene_label(&label, 17)), x, y, 3.8,
    );
    Ok(())
}

fn spawn_field_pokedex_detail(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    species: &crate::RuntimePokemonCatalogSnapshot,
    caught: bool,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let entry = snapshot
        .presentation
        .pokedex_entries
        .get(&species.species_id)
        .with_context(|| format!("Pokédex entry is missing species {}", species.species_id))?;
    if let Some(frame) = pokemon_frame_for_art(
        rendered_art, asset_root, &species.species_id,
        PokemonSpriteSide::Front, false, images,
    ) {
        let (x, y) = battle_hud_tile_origin(3.5, 4.0);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite { custom_size: Some(frame.size), ..default() },
                transform: Transform::from_xyz(x, y, 3.8),
                ..default()
            },
            FieldCommandMarker,
        ));
    }
    let height = if caught { entry.height_digits.to_string() } else { "????".to_string() };
    let weight = if caught { entry.weight_digits.to_string() } else { "????".to_string() };
    for (row, text) in [
        (
            2.0,
            crate::core::models::pokemon_species_display_name(&species.species_id),
        ),
        (4.0, entry.classification.clone()),
        (7.0, format!("HT {height}")),
        (8.0, format!("WT {weight}")),
        (9.0, format!("No.{:03}", species.int_id)),
    ] {
        let (x, y) = battle_hud_tile_origin(9.0, row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, &compact_scene_label(&text, 11), x, y, 3.8,
        );
    }
    let page_index = runtime_shell.pokedex_detail_page;
    let page = entry.pages.get(page_index).map(String::as_str).with_context(|| {
        format!(
            "Pokédex detail page {page_index} is outside {} pages for {}",
            entry.pages.len(),
            species.species_id
        )
    })?;
    for (index, line) in wrap_boot_text_for_box(page, 18, 5).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 11.0 + index as f32);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 3.8,
        );
    }
    if entry.pages.len() > 1 {
        let (x, y) = battle_hud_tile_origin(14.0, 16.0);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{}/{}", page_index + 1, entry.pages.len()), x, y, 3.8,
        );
    }
    Ok(())
}

fn spawn_field_pack_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    let (items, cursor, surface_id): (Vec<(String, u16)>, &Option<MenuCursor>, String) = match &pocket {
        FieldPackPocket::Items => (
            snapshot.bag.items.iter().filter(|item| item.quantity > 0)
                .map(|item| (item.item_id.clone(), item.quantity)).collect(),
            &runtime_shell.bag_cursor,
            "bag:items".to_string(),
        ),
        FieldPackPocket::Balls => (
            snapshot.bag.balls.iter().filter(|item| item.quantity > 0)
                .map(|item| (item.item_id.clone(), item.quantity)).collect(),
            &runtime_shell.ball_cursor,
            "bag:balls".to_string(),
        ),
        FieldPackPocket::KeyItems => (
            snapshot.bag.key_items.iter().filter(|item| item.quantity > 0)
                .map(|item| (item.item_id.clone(), item.quantity)).collect(),
            &runtime_shell.key_item_cursor,
            "bag:key-items".to_string(),
        ),
        FieldPackPocket::TmHm => (
            snapshot.bag.tm_hm.iter().filter(|item| item.quantity > 0)
                .map(|item| (item.item_id.clone(), item.quantity)).collect(),
            &runtime_shell.tmhm_cursor,
            "bag:tmhm".to_string(),
        ),
        FieldPackPocket::Custom(pocket_id) => (
            snapshot.bag.custom_pockets.get(pocket_id).into_iter().flatten()
                .filter(|item| item.quantity > 0)
                .map(|item| (item.item_id.clone(), item.quantity)).collect(),
            &runtime_shell.custom_item_cursor,
            custom_pack_surface_id(pocket_id),
        ),
    };
    let row_count = field_pack_selectable_count(items.len());
    let selected = strict_readonly_cursor_index(cursor, &surface_id, row_count)
        .with_context(|| format!("field PACK cursor is invalid for {surface_id} with {row_count} rows"))?;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.96, 0.96, 0.91),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    let pocket_label = field_pack_pocket_label(&pocket).to_uppercase();
    for (row, text) in [
        (1.0, "< PACK >".to_string()),
        (4.0, "[BAG]".to_string()),
        (7.0, "<     >".to_string()),
        (8.0, compact_scene_label(&pocket_label, 5)),
    ] {
        let (x, y) = battle_hud_tile_origin(0.0, row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, &text, x, y, 3.8,
        );
    }
    let list_start = visible_window_start(selected, row_count, 7);
    for visible_index in 0..7 {
        let index = list_start + visible_index;
        if index >= row_count {
            break;
        }
        let row = 2.0 + visible_index as f32;
        let marker = if index == selected { ">" } else { " " };
        let (name, quantity) = if index >= items.len() {
            ("CANCEL".to_string(), None)
        } else {
            (item_display_name(snapshot, &items[index].0), Some(items[index].1))
        };
        let (x, y) = battle_hud_tile_origin(7.0, row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{marker}{}", compact_scene_label(&name, 8)), x, y, 3.8,
        );
        if let Some(quantity) = quantity {
            let (x, y) = battle_hud_tile_origin(16.0, row);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("×{:02}", quantity.min(99)), x, y, 3.8,
            );
        }
    }
    let description = if let Some((item_id, _)) = items.get(selected) {
        snapshot
            .items
            .iter()
            .find(|item| item.item_id == *item_id)
            .with_context(|| format!("field PACK item {item_id} is missing"))?
            .description
            .as_str()
    } else {
        "Close the PACK."
    };
    for (index, line) in wrap_boot_text_for_box(description, 18, 4).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 13.0 + index as f32);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 3.8,
        );
    }
    if let Some(action_cursor) = &runtime_shell.field_pack_action_cursor {
        let actions = visible_selected_pack_item_actions(
            snapshot,
            runtime_shell,
            &pocket,
            false,
        )?;
        let action_selected = strict_readonly_cursor_index(
            &Some(action_cursor.clone()), "pack:actions", actions.len(),
        )
        .with_context(|| {
            format!(
                "field PACK action cursor is invalid for {} actions",
                actions.len()
            )
        })?;
        let top = match actions.len() { 5 => 1.0, 4 => 3.0, 3 => 5.0, 2 => 7.0, _ => 9.0 };
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            13.0,
            top,
            7.0,
            actions.len() as f32 * 2.0 + 1.0,
            4.1,
        );
        for (index, action) in actions.iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(14.0, top + 1.0 + index as f32 * 2.0);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("{}{}", if index == action_selected { ">" } else { " " }, visible_field_pack_action_label(*action)),
                x, y, 4.3,
            );
        }
    }
    spawn_field_notice(commands, runtime_shell, rendered_art, asset_root, images);
    Ok(())
}

fn spawn_field_notice(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    if let Some(notice) = runtime_shell.field_notice.as_deref() {
        let visible_notice = visible_revealed_shell_notice_text(runtime_shell, notice);
        spawn_battle_window(
            commands, rendered_art, asset_root, images,
            1.0, 10.0, 18.0, 8.0, 4.5,
        );
        for (index, line) in wrap_boot_text_for_box(&visible_notice, 16, 6).iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(2.0, 11.0 + index as f32);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images, line, x, y, 4.8,
            );
        }
        if visible_field_text_reveal_is_complete_for_text(runtime_shell, notice)
            && visible_field_notice_uses_prompt_arrow(runtime_shell)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images, "▼", x, y, 4.8,
            );
        }
    }
}

fn spawn_field_party_summary_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let slot = snapshot.party.slots.get(runtime_shell.party_cursor).with_context(|| {
        format!(
            "party summary cursor {} is outside {} slots",
            runtime_shell.party_cursor,
            snapshot.party.slots.len()
        )
    })?;
    let pokemon = &slot.pokemon;
    let page = runtime_shell.party_summary_page;
    anyhow::ensure!(
        (1..=3).contains(&page),
        "party summary page {page} is outside pages 1 through 3"
    );
    let held_item_label = if let Some(item_id) = pokemon.item.as_deref() {
        snapshot
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .with_context(|| format!("party summary held item {item_id} is missing"))?
            .name
            .replace('_', " ")
    } else {
        "NONE".to_string()
    };
    for learned in &pokemon.moves {
        snapshot
            .moves
            .iter()
            .find(|move_data| move_data.move_id == learned.name)
            .with_context(|| {
                format!(
                    "party summary is missing move metadata for {}",
                    learned.name
                )
            })?;
    }
    let tint = if pokemon.is_egg {
        Color::rgb(0.98, 0.94, 0.78)
    } else {
        match page {
            1 => Color::rgb(1.0, 0.86, 0.89),
            2 => Color::rgb(0.84, 0.96, 0.84),
            _ => Color::rgb(0.84, 0.91, 1.0),
        }
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: tint,
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    let mut rows = vec![
        (1.0, compact_scene_label(&pokemon.nickname, 10)),
        (2.0, format!("No.{:03}  \u{e10a}{:>2}", pokemon.species.int_id, pokemon.level)),
        (
            3.0,
            crate::core::models::pokemon_species_display_name(&pokemon.species.id),
        ),
    ];
    if pokemon.is_egg {
        rows.push((7.0, "EGG".to_string()));
        let hatch_lines = if pokemon.happiness < 6 {
            ["It's making sounds", "inside. It's going", "to hatch soon!", ""]
        } else if pokemon.happiness < 11 {
            ["It moves around", "inside sometimes.", "It must be close", "to hatching."]
        } else if pokemon.happiness < 41 {
            ["Wonder what's", "inside? It needs", "more time, though.", ""]
        } else {
            ["This EGG needs a", "lot more time to", "hatch.", ""]
        };
        rows.extend(
            hatch_lines
                .into_iter()
                .enumerate()
                .filter(|(_, line)| !line.is_empty())
                .map(|(index, line)| (9.0 + index as f32 * 2.0, line.to_string())),
        );
    } else {
        rows.push((5.0, format!("< {}  {}  {} >", if page == 1 { "[1]" } else { "1" }, if page == 2 { "[2]" } else { "2" }, if page == 3 { "[3]" } else { "3" })));
        match page {
            1 => {
                let species = snapshot
                    .pokemon
                    .iter()
                    .find(|entry| entry.species_id == pokemon.species.id)
                    .with_context(|| {
                        format!(
                            "party summary is missing species data for {}",
                            pokemon.species.id
                        )
                    })?;
                let type_label = if species.type1 == species.type2 {
                    species.type1.clone()
                } else {
                    format!("{}/{}", species.type1, species.type2)
                };
                rows.extend([
                    (7.0, format!("HP  {:>3}/{:>3}", pokemon.hp, pokemon.max_hp)),
                    (8.0, format!("STATUS  {}", party_status_token(pokemon))),
                    (10.0, format!("TYPE/ {type_label}")),
                    (12.0, format!("EXP POINTS {:>7}", pokemon.experience.max(0))),
                    (14.0, format!("ITEM {held_item_label}")),
                ]);
            }
            2 => {
                rows.push((6.0, format!("ITEM {held_item_label}")));
                if pokemon.moves.is_empty() {
                    rows.push((9.0, "NO MOVES".to_string()));
                } else {
                    for (index, learned) in pokemon.moves.iter().take(4).enumerate() {
                        let row = 8.0 + index as f32 * 2.0;
                        rows.push((row, battle_move_display_name(snapshot, &learned.name)));
                        rows.push((row + 1.0, visible_move_pp_text(snapshot, learned)));
                    }
                }
            }
            _ => rows.extend([
                (7.0, format!("OT/{}", pokemon.original_trainer_name)),
                (8.0, format!("IDNo.{:05}", pokemon.original_trainer_id)),
                (10.0, format!("ATTACK  {:>3}", pokemon.attack)),
                (11.0, format!("DEFENSE {:>3}", pokemon.defense)),
                (12.0, format!("SPCL.ATK{:>3}", pokemon.special_attack)),
                (13.0, format!("SPCL.DEF{:>3}", pokemon.special_defense)),
                (14.0, format!("SPEED   {:>3}", pokemon.speed)),
            ]),
        }
    }
    for (row, text) in rows {
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(&text, 18),
            x,
            y,
            3.8,
        );
    }
    Ok(())
}

fn spawn_field_move_reorder_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let slot = snapshot.party.slots.get(runtime_shell.party_cursor).with_context(|| {
        format!(
            "move-reorder party cursor {} is outside {} slots",
            runtime_shell.party_cursor,
            snapshot.party.slots.len()
        )
    })?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_reorder_surface_id(slot.index),
        slot.pokemon.moves.len(),
    )
    .with_context(|| {
        format!(
            "move-reorder cursor is invalid for party slot {} with {} moves",
            slot.index,
            slot.pokemon.moves.len()
        )
    })?;
    if let Some(origin) = runtime_shell.party_move_reorder_origin {
        anyhow::ensure!(
            origin < slot.pokemon.moves.len(),
            "move-reorder origin {origin} is outside {} moves",
            slot.pokemon.moves.len()
        );
    }
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    let (x, y) = battle_hud_tile_origin(1.0, 1.0);
    spawn_field_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("< {} \u{e10a}{:>2} >", compact_scene_label(&slot.pokemon.nickname, 10), slot.pokemon.level),
        x, y, 3.8,
    );
    for (index, learned) in slot.pokemon.moves.iter().enumerate().take(4) {
        let row = 3.0 + index as f32 * 2.0;
        let marker = if runtime_shell.party_move_reorder_origin == Some(index) {
            "▷"
        } else if selected == index {
            ">"
        } else {
            " "
        };
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{marker}{}", battle_move_display_name(snapshot, &learned.name)),
            x, y, 3.8,
        );
        let (x, y) = battle_hud_tile_origin(10.0, row + 1.0);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &visible_move_pp_text(snapshot, learned), x, y, 3.8,
        );
    }
    let selected_move = &slot.pokemon.moves[selected];
    if runtime_shell.party_move_reorder_origin.is_some() {
        let (x, y) = battle_hud_tile_origin(1.0, 12.0);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, "Where?", x, y, 3.8,
        );
        return Ok(());
    }
    let move_data = snapshot
        .moves
        .iter()
        .find(|entry| entry.move_id == selected_move.name)
        .with_context(|| {
            format!(
                "move-reorder screen is missing move metadata for {}",
                selected_move.name
            )
        })?;
    let move_type = move_data.move_type.replace("_TYPE", "").replace('_', " ");
    let power = move_data.power;
    let (x, y) = battle_hud_tile_origin(2.0, 12.0);
    spawn_field_command_bitmap_text(
        commands, rendered_art, asset_root, images, &format!("TYPE/{move_type}"), x, y, 3.8,
    );
    let (x, y) = battle_hud_tile_origin(12.0, 12.0);
    spawn_field_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("ATK/{:>3}", if power == 0 { "---".to_string() } else { power.to_string() }),
        x, y, 3.8,
    );
    if rendered_art.move_description_cache.is_none()
        && rendered_art.move_description_error.is_none()
    {
        match load_asm_move_descriptions(asset_root, snapshot) {
            Ok(descriptions) => rendered_art.move_description_cache = Some(descriptions),
            Err(error) => rendered_art.move_description_error = Some(error.to_string()),
        }
    }
    let description = rendered_art
        .move_description_cache
        .as_ref()
        .and_then(|descriptions| descriptions.get(&selected_move.name))
        .cloned()
        .with_context(|| {
            rendered_art.move_description_error.clone().unwrap_or_else(|| {
                format!("move description {} is missing", selected_move.name)
            })
        })?;
    for (index, line) in wrap_boot_text_for_box(&description, 18, 3).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 14.0 + index as f32);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 3.8,
        );
    }
    Ok(())
}

fn load_asm_move_descriptions(
    asset_root: &AssetRoot,
    snapshot: &RuntimeShellSnapshot,
) -> Result<HashMap<String, String>> {
    let path = asset_root.resolve_vendor("data/moves/descriptions.asm");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read ASM move descriptions {}", path.display()))?;
    let mut table = Vec::new();
    let mut descriptions = HashMap::<String, String>::new();
    let mut in_table = false;
    let mut labels = Vec::<String>::new();
    let mut parts = Vec::<String>::new();
    let flush = |labels: &mut Vec<String>,
                 parts: &mut Vec<String>,
                 descriptions: &mut HashMap<String, String>| {
        let text = parts.join("").trim().to_string();
        for label in labels.drain(..) {
            descriptions.insert(label, text.clone());
        }
        parts.clear();
    };
    for raw in content.lines() {
        let line = raw.split(';').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line == "MoveDescriptions::" {
            in_table = true;
            continue;
        }
        if in_table {
            if line.starts_with("assert_table_length") {
                in_table = false;
            } else if let Some(label) = line.strip_prefix("dw ") {
                table.push(label.trim().to_string());
            }
            continue;
        }
        if line.ends_with(':') {
            if !parts.is_empty() {
                flush(&mut labels, &mut parts, &mut descriptions);
            }
            labels.push(line.trim_end_matches(':').to_string());
            continue;
        }
        if labels.is_empty() {
            continue;
        }
        let Some(first_quote) = line.find('"') else {
            continue;
        };
        let Some(last_quote) = line.rfind('"') else {
            continue;
        };
        if last_quote <= first_quote {
            continue;
        }
        if line.starts_with("next ") && !parts.is_empty() {
            parts.push("\n".to_string());
        }
        if line.starts_with("db ") || line.starts_with("next ") {
            parts.push(line[first_quote + 1..last_quote].trim_end_matches('@').to_string());
        }
    }
    flush(&mut labels, &mut parts, &mut descriptions);
    let constants_path = asset_root.resolve_vendor("constants/move_constants.asm");
    let constants = std::fs::read_to_string(&constants_path)
        .with_context(|| format!("read ASM move constants {}", constants_path.display()))?;
    let mut move_order = Vec::new();
    for raw in constants.lines() {
        let line = raw.split(';').next().unwrap_or_default().trim();
        if line.starts_with("DEF NUM_ATTACKS ") {
            break;
        }
        let Some(token) = line.strip_prefix("const ") else {
            continue;
        };
        let move_id = token.split_whitespace().next().unwrap_or_default();
        if move_id != "NO_MOVE" && !move_id.is_empty() {
            move_order.push(move_id.to_string());
        }
    }
    if table.len() < move_order.len() {
        anyhow::bail!(
            "ASM move description table has {} entries for {} move constants",
            table.len(),
            move_order.len()
        );
    }
    let compiled_moves = snapshot
        .moves
        .iter()
        .map(|entry| entry.move_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let ordered_moves = move_order.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    if compiled_moves != ordered_moves {
        anyhow::bail!("compiled move catalog does not exactly match ASM move constants");
    }
    move_order
        .into_iter()
        .enumerate()
        .map(|(index, move_id)| {
            let label = &table[index];
            let description = descriptions
                .get(label)
                .filter(|text| !text.is_empty())
                .with_context(|| {
                    format!("ASM move description {label} missing for {move_id}")
                })?;
            Ok((move_id, description.clone()))
        })
        .collect()
}

fn spawn_field_party_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let row_count = normal_visible_party_menu_row_count(snapshot);
    let source = runtime_shell.party_cursor;
    anyhow::ensure!(
        source < row_count,
        "party cursor {source} is outside {row_count} rows"
    );
    let selected = if let Some(switch_cursor) = &runtime_shell.party_switch_cursor {
        let source_slot = snapshot
            .party
            .slots
            .get(source)
            .context("party switch source is not backed by a party slot")?;
        strict_readonly_cursor_index(
            &Some(switch_cursor.clone()),
            &party_switch_cursor_surface_id(source_slot.index),
            snapshot.party.slots.len(),
        )
        .context("party switch target requires a valid cursor")?
    } else {
        source
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        FieldCommandMarker,
    ));
    for (row_index, slot) in snapshot.party.slots.iter().enumerate().take(6) {
        let name_row = 1.0 + row_index as f32 * 2.0;
        let status_row = name_row + 1.0;
        spawn_battle_party_icon(
            commands, snapshot, slot, row_index, selected == row_index, true,
            rendered_art, asset_root, images,
        ).context("field party selection has no valid icon")?;
        let (x, y) = battle_hud_tile_origin(0.0, name_row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            if selected == row_index { ">" } else { " " }, x, y, 3.8,
        );
        if runtime_shell.party_switch_cursor.is_some() && source == row_index {
            let (x, y) = battle_hud_tile_origin(2.0, name_row);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images, "▷", x, y, 3.8,
            );
        }
        let (x, y) = battle_hud_tile_origin(3.0, name_row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &compact_scene_label(&slot.pokemon.nickname, 10), x, y, 3.8,
        );
        if slot.pokemon.is_egg {
            continue;
        }
        let (x, y) = battle_hud_tile_origin(13.0, name_row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{:>3}/{:>3}", slot.pokemon.hp.min(999), slot.pokemon.max_hp.min(999)),
            x, y, 3.8,
        );
        let (x, y) = battle_hud_tile_origin(5.0, status_row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            party_status_token(&slot.pokemon), x, y, 3.8,
        );
        let (x, y) = battle_hud_tile_origin(8.0, status_row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("\u{e10a}{:>2}", slot.pokemon.level.min(100)), x, y, 3.8,
        );
        let hp_tiles = ((u32::from(slot.pokemon.hp.min(slot.pokemon.max_hp)) * 6)
            / u32::from(slot.pokemon.max_hp.max(1))) as usize;
        let (x, y) = battle_hud_tile_origin(11.0, status_row);
        spawn_field_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{}{}", "|".repeat(hp_tiles), ".".repeat(6 - hp_tiles)), x, y, 3.8,
        );
    }
    let cancel_row = 1.0 + snapshot.party.slots.len().min(6) as f32 * 2.0;
    let (x, y) = battle_hud_tile_origin(1.0, cancel_row);
    spawn_field_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("{}CANCEL", if selected >= snapshot.party.slots.len() { ">" } else { " " }),
        x, y, 3.8,
    );
    let (x, y) = battle_hud_tile_origin(1.0, 15.0);
    spawn_field_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        if runtime_shell.party_hp_transfer_source.is_some() {
            "Use on which <PKMN>?"
        } else if runtime_shell.party_switch_cursor.is_some() {
            "Move to where?"
        } else {
            "Choose a <PKMN>."
        },
        x, y, 4.1,
    );
    Ok(())
}

fn spawn_field_party_action_window(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let actions = visible_party_actions(snapshot, runtime_shell)?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_action_cursor,
        "party:actions",
        actions.len(),
    )
    .with_context(|| format!("party action cursor is invalid for {} actions", actions.len()))?;
    let (left, top, width, height) = (6.0, 0.0, 14.0, 18.0);
    let (center_x, center_y) = battle_hud_tile_origin(
        left + width / 2.0 - 0.5,
        top + height / 2.0 - 0.5,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(TILE_SIZE * width, TILE_SIZE * height)),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.0),
            ..default()
        },
        FieldCommandMarker,
    ));
    for (index, action) in actions.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(7.0, 1.0 + index as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &party_submenu_action_entry(*action, if index == selected { ">" } else { " " }),
            x,
            y,
            4.2,
        );
    }
    Ok(())
}

fn spawn_field_party_give_take_window(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let mail_actions = runtime_shell.party_give_take_cursor.as_ref()
        .is_some_and(|cursor| cursor.surface_id == "party:mail-actions");
    let surface = if mail_actions { "party:mail-actions" } else { "party:give-take" };
    let labels: &[&str] = if mail_actions { &["READ", "TAKE", "QUIT"] } else { &["GIVE", "TAKE"] };
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_give_take_cursor,
        surface,
        labels.len(),
    )
    .with_context(|| format!("{surface} cursor is invalid for {} actions", labels.len()))?;
    let (center_x, center_y) = battle_hud_tile_origin(15.5, 14.5);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(TILE_SIZE * 8.0, TILE_SIZE * (labels.len() as f32 * 2.0 + 2.0))),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.3),
            ..default()
        },
        FieldCommandMarker,
    ));
    for (index, label) in labels.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(13.0, 13.0 + index as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{}{label}", if selected == index { ">" } else { " " }),
            x,
            y,
            4.5,
        );
    }
    Ok(())
}

fn spawn_start_menu_command_window(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let options = visible_start_menu_options(runtime_shell, snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.start_menu_cursor,
        START_MENU_SURFACE_ID,
        options.len(),
    )
    .with_context(|| format!("START menu cursor is invalid for {} options", options.len()))?;
    let width_tiles = (START_MENU_RIGHT_TILE - START_MENU_LEFT_TILE + 1.0) as usize;
    let height_tiles = START_MENU_MIN_HEIGHT_TILES.max(options.len() as f32 + 2.0) as usize;
    let contest_active = snapshot.bug_contest.timer_active
        || snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_BUG_CONTEST_TIMER");
    let top_tile = if contest_active { 2.0 } else { START_MENU_TOP_TILE };
    spawn_start_menu_command_window_fill(commands, top_tile, width_tiles, height_tiles, 3.3);
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("START menu requires window-frame art")?;
    spawn_field_command_window_frame_tiles(
        commands,
        frame,
        START_MENU_LEFT_TILE,
        top_tile,
        width_tiles,
        height_tiles,
        3.4,
    );
    if contest_active {
        spawn_bug_contest_start_menu_status(
            commands,
            snapshot,
            rendered_art,
            asset_root,
            images,
        )?;
    }
    for (index, option) in options.iter().enumerate() {
        let row_tile_y = top_tile + 1.0 + index as f32;
        if index == selected {
            let (cursor_x, cursor_y) = battle_hud_tile_origin(START_MENU_CURSOR_TILE_X, row_tile_y);
            spawn_field_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                ">",
                cursor_x,
                cursor_y,
                3.6,
            );
        }
        let (label_x, label_y) = battle_hud_tile_origin(START_MENU_LABEL_TILE_X, row_tile_y);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &start_menu_option_display_label(*option, snapshot),
            label_x,
            label_y,
            3.6,
        );
    }
    Ok(())
}

fn spawn_start_menu_command_window_fill(
    commands: &mut Commands,
    top_tile: f32,
    width_tiles: usize,
    height_tiles: usize,
    z: f32,
) {
    let (x, y) = field_window_center(
        START_MENU_LEFT_TILE,
        top_tile,
        width_tiles as f32,
        height_tiles as f32,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(
                    width_tiles as f32 * TILE_SIZE,
                    height_tiles as f32 * TILE_SIZE,
                )),
                ..default()
            },
            transform: Transform::from_xyz(x, y, z),
            ..default()
        },
        FieldCommandMarker,
    ));
}

fn spawn_bug_contest_start_menu_status(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    const LEFT: f32 = 0.0;
    const TOP: f32 = 0.0;
    const WIDTH: usize = 17;
    const HEIGHT: usize = 6;
    let (center_x, center_y) = field_window_center(LEFT, TOP, WIDTH as f32, HEIGHT as f32);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(
                    WIDTH as f32 * TILE_SIZE,
                    HEIGHT as f32 * TILE_SIZE,
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 3.3),
            ..default()
        },
        FieldCommandMarker,
    ));
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("Bug Contest status requires window-frame art")?;
    spawn_field_command_window_frame_tiles(
        commands, frame, LEFT, TOP, WIDTH, HEIGHT, 3.4,
    );
    let caught = snapshot
        .bug_contest
        .caught_species
        .as_deref()
        .unwrap_or("None");
    let mut rows = vec![format!("CAUGHT  {caught}")];
    if snapshot.bug_contest.caught_species.is_some() {
        let caught_level = snapshot
            .bug_contest
            .caught_level
            .context("Bug Contest caught species is missing its level")?;
        rows.push(format!(
            "LEVEL   {}",
            caught_level
        ));
    }
    rows.push(format!(
        "BALLS:  {}",
        snapshot.bug_contest.park_balls_remaining
    ));
    for (index, row) in rows.iter().enumerate() {
        let row_y = if rows.len() == 3 {
            1.0 + index as f32 * 2.0
        } else {
            1.0 + index as f32 * 4.0
        };
        let (x, y) = battle_hud_tile_origin(1.0, row_y);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            row,
            x,
            y,
            3.6,
        );
    }
    Ok(())
}

fn spawn_options_menu_command_window(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let selected = runtime_shell.options_cursor;
    anyhow::ensure!(
        selected < OPTIONS_MENU_ITEMS.len(),
        "OPTIONS cursor {selected} is outside {} entries",
        OPTIONS_MENU_ITEMS.len()
    );
    spawn_options_menu_window_fill(commands, 3.3);
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("OPTIONS menu requires window-frame art")?;
    spawn_field_command_window_frame_tiles(
        commands,
        frame,
        OPTIONS_MENU_LEFT_TILE,
        OPTIONS_MENU_TOP_TILE,
        OPTIONS_MENU_WIDTH_TILES,
        OPTIONS_MENU_HEIGHT_TILES,
        3.4,
    );
    for (index, item) in OPTIONS_MENU_ITEMS.iter().copied().enumerate() {
        let row_tile_y = options_menu_row_tile_y(index);
        if index == selected {
            spawn_options_menu_text(
                commands,
                rendered_art,
                asset_root,
                images,
                ">",
                OPTIONS_MENU_CURSOR_TILE_X,
                row_tile_y,
                3.6,
            );
        }
        spawn_options_menu_text(
            commands,
            rendered_art,
            asset_root,
            images,
            options_menu_item_label(item),
            OPTIONS_MENU_LABEL_TILE_X,
            row_tile_y,
            3.6,
        );
        if item == OptionsMenuItem::Cancel {
            continue;
        }
        let value_row_tile_y = row_tile_y + 1.0;
        spawn_options_menu_text(
            commands,
            rendered_art,
            asset_root,
            images,
            ":",
            OPTIONS_MENU_VALUE_TILE_X - 1.0,
            value_row_tile_y,
            3.6,
        );
        if item == OptionsMenuItem::Frame {
            spawn_options_menu_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "TYPE",
                OPTIONS_MENU_VALUE_TILE_X,
                value_row_tile_y,
                3.6,
            );
        }
        let value_x = if item == OptionsMenuItem::Frame {
            OPTIONS_MENU_FRAME_VALUE_TILE_X
        } else {
            OPTIONS_MENU_VALUE_TILE_X
        };
        spawn_options_menu_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &option_value_for_item(&snapshot.trainer.options, item),
            value_x,
            value_row_tile_y,
            3.6,
        );
    }
    Ok(())
}

fn spawn_options_menu_window_fill(commands: &mut Commands, z: f32) {
    let (x, y) = field_window_center(
        OPTIONS_MENU_LEFT_TILE,
        OPTIONS_MENU_TOP_TILE,
        OPTIONS_MENU_WIDTH_TILES as f32,
        OPTIONS_MENU_HEIGHT_TILES as f32,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(
                    OPTIONS_MENU_WIDTH_TILES as f32 * TILE_SIZE,
                    OPTIONS_MENU_HEIGHT_TILES as f32 * TILE_SIZE,
                )),
                ..default()
            },
            transform: Transform::from_xyz(x, y, z),
            ..default()
        },
        FieldCommandMarker,
    ));
}

fn options_menu_row_tile_y(index: usize) -> f32 {
    OPTIONS_MENU_FIRST_ROW_TILE_Y + index as f32 * OPTIONS_MENU_ROW_SPACING_TILES
}

fn spawn_options_menu_text(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
    tile_x: f32,
    tile_y: f32,
    z: f32,
) {
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, text, x, y, z);
}

fn spawn_scene_dialog(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    if scene_dialog_surface_active(snapshot, runtime_shell) {
        require_bitmap_font_art(rendered_art, asset_root, images)?;
    }
    if runtime_shell.visible_diploma.is_some() {
        spawn_visible_diploma(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if let Some(word) = runtime_shell.visible_unown_words.as_deref() {
        spawn_visible_unown_words(commands, rendered_art, asset_root, images, word);
        return Ok(());
    }
    if runtime_shell.visible_magnet_train.is_some() {
        spawn_visible_magnet_train(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    spawn_visible_heal_machine(
        commands,
        runtime_shell,
        rendered_art,
        asset_root,
        images,
    )?;
    spawn_visible_balance_overlay(
        commands,
        runtime_shell,
        rendered_art,
        asset_root,
        images,
    );
    if runtime_shell
        .visible_mom_bank
        .as_ref()
        .is_some_and(|bank| {
            bank.messages.is_empty()
                && matches!(
                    bank.phase,
                    VisibleMomBankPhase::Menu
                        | VisibleMomBankPhase::Withdraw
                        | VisibleMomBankPhase::Deposit
                )
        })
    {
        spawn_visible_mom_bank_screen(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        );
        return Ok(());
    }
    if runtime_shell.field_notice.is_some() {
        spawn_field_notice(commands, runtime_shell, rendered_art, asset_root, images);
        return Ok(());
    }
    let entries = visible_scene_dialog_entries(snapshot, runtime_shell)?;
    if entries.is_empty() {
        return Ok(());
    }

    if let Some(choice) = runtime_shell.pending_name_choice.as_ref() {
        spawn_visible_name_choice_screen(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
            choice,
        );
        return Ok(());
    }
    if let Some(input) = runtime_shell.pending_name_input.as_ref() {
        spawn_visible_name_entry_screen(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
            input,
        );
        return Ok(());
    }
    if let Some(shop) = snapshot.pending_shop.as_ref() {
        spawn_field_shop_screen(
            commands, snapshot, runtime_shell, shop, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if runtime_shell.storage_cursor.is_some() && !runtime_shell.party_menu_open {
        spawn_field_storage_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if runtime_shell.pc_item_cursor.is_some() && !visible_field_pack_is_open(runtime_shell) {
        spawn_field_pc_item_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        spawn_field_pc_box_selection_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if let Some(menu) = snapshot.ui.menu.as_ref()
        && menu.menu_2d_requested
        && spawn_runtime_2d_menu(
            commands,
            runtime_shell,
            menu,
            rendered_art,
            asset_root,
            images,
        )
    {
        return Ok(());
    }

    spawn_scene_dialog_text_box(commands, rendered_art, asset_root, images, 4.0);
    spawn_scene_dialog_text_content(
        commands,
        snapshot,
        runtime_shell,
        rendered_art,
        asset_root,
        images,
    )?;
    Ok(())
}

fn spawn_visible_diploma(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    const WIDTH: usize = 20 * SOURCE_TILE_SIZE;
    const HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
    if rendered_art.diploma_base_cache.is_none() {
        match load_visible_diploma_base(asset_root) {
            Ok(data) => rendered_art.diploma_base_cache = Some(data),
            Err(error) => rendered_art.diploma_base_error = Some(format!("{error:#}")),
        }
    }
    if let Some(error) = rendered_art.diploma_base_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let mut image = Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rendered_art.diploma_base_cache.as_ref().unwrap().clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let (x, y) = field_window_center(0.0, 0.0, 20.0, 18.0);
    commands.spawn((
        SpriteBundle {
            texture: images.add(image),
            sprite: Sprite {
                custom_size: Some(Vec2::new(20.0 * TILE_SIZE, 18.0 * TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 20.0),
            ..default()
        },
        FieldCommandMarker,
    ));
    for (text, tile_x, tile_y) in [
        ("PLAYER".to_string(), 2.0, 5.0),
        (snapshot.trainer.player_name.clone(), 9.0, 5.0),
        ("This certifies".to_string(), 2.0, 8.0),
        ("that you have".to_string(), 2.0, 9.0),
        ("completed the".to_string(), 2.0, 10.0),
        ("new POKéDEX.".to_string(), 2.0, 11.0),
        ("Congratulations!".to_string(), 2.0, 12.0),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &text,
            x,
            y,
            20.2,
        );
    }
    if runtime_shell
        .visible_diploma
        .is_some_and(|frame| frame & 0x10 != 0)
    {
        let (x, y) = battle_hud_tile_origin(18.0, 17.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "▼",
            x,
            y,
            20.2,
        );
    }
    Ok(())
}

fn load_visible_diploma_base(asset_root: &AssetRoot) -> Result<Vec<u8>> {
    const WIDTH: usize = 20 * SOURCE_TILE_SIZE;
    const HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
    let source = image::open(asset_root.resolve_vendor("gfx/diploma/diploma.png"))
        .context("decode diploma graphics")?
        .to_rgba8();
    let tilemap = std::fs::read(asset_root.resolve_vendor("gfx/diploma/page1.tilemap"))
        .context("read diploma page-one tilemap")?;
    anyhow::ensure!(tilemap.len() == 20 * 18, "diploma page-one tilemap must be 360 bytes");
    anyhow::ensure!(
        source.width() % SOURCE_TILE_SIZE as u32 == 0
            && source.height() % SOURCE_TILE_SIZE as u32 == 0,
        "diploma graphics must contain complete 8x8 tiles"
    );
    let columns = source.width() as usize / SOURCE_TILE_SIZE;
    let palette_text = std::fs::read_to_string(
        asset_root.resolve_vendor("gfx/diploma/diploma.pal"),
    )
    .context("read diploma palette")?;
    let palette = parse_palette_file(&palette_text, None)?
        .into_iter()
        .next()
        .context("diploma palette has no color set")?;
    let mut data = vec![0_u8; WIDTH * HEIGHT * 4];
    for tile_y in 0..18 {
        for tile_x in 0..20 {
            let tile_id = usize::from(tilemap[tile_y * 20 + tile_x]);
            let source_x = (tile_id % columns) * SOURCE_TILE_SIZE;
            let source_y = (tile_id / columns) * SOURCE_TILE_SIZE;
            anyhow::ensure!(
                source_y + SOURCE_TILE_SIZE <= source.height() as usize,
                "diploma tile {tile_id} lies outside diploma graphics"
            );
            for row in 0..SOURCE_TILE_SIZE {
                for col in 0..SOURCE_TILE_SIZE {
                    let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
                    let color = palette[palette_index_from_gray(pixel[0])];
                    let target = (((tile_y * SOURCE_TILE_SIZE + row) * WIDTH)
                        + tile_x * SOURCE_TILE_SIZE
                        + col)
                        * 4;
                    data[target..target + 3].copy_from_slice(&color);
                    data[target + 3] = 255;
                }
            }
        }
    }
    Ok(data)
}

fn spawn_visible_unown_words(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    word: &str,
) {
    let length = word.chars().count() as f32;
    let left = (9.0 - length).max(0.0);
    let right = (10.0 + length).min(19.0);
    let width = (right - left + 1.0).max(2.0);
    let top = 4.0;
    let height = 6.0;
    let (center_x, center_y) = field_window_center(left, top, width, height);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (width - 2.0),
                    TILE_SIZE * (height - 2.0),
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.1),
            ..default()
        },
        FieldCommandMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_field_command_window_frame_tiles(
            commands,
            frame,
            left,
            top,
            width as usize,
            height as usize,
            4.2,
        );
    }
    let (x, y) = battle_hud_tile_origin(left + 1.0, top + 2.0);
    spawn_field_command_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        word,
        x,
        y,
        4.3,
    );
}

fn spawn_visible_magnet_train(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    const WIDTH: usize = 20 * SOURCE_TILE_SIZE;
    const HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
    const TOP_BAND: usize = 6 * SOURCE_TILE_SIZE - 1;
    const MID_BAND: usize = 6 * SOURCE_TILE_SIZE;
    let Some(animation) = runtime_shell.visible_magnet_train.as_ref() else {
        return Ok(());
    };
    if rendered_art.magnet_train_base_cache.is_none() {
        match load_visible_magnet_train_base(asset_root) {
            Ok(data) => rendered_art.magnet_train_base_cache = Some(data),
            Err(error) => rendered_art.magnet_train_base_error = Some(format!("{error:#}")),
        }
    }
    if let Some(error) = rendered_art.magnet_train_base_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let base = rendered_art.magnet_train_base_cache.as_ref().unwrap();
    let background_shift = i16::from(((animation.offset * 2) & 0xff) as u8);
    let background_shift = if background_shift < 128 {
        background_shift
    } else {
        background_shift - 256
    };
    let train_shift = i16::from((animation.position & 0xff) as u8);
    let train_shift = if train_shift < 128 { train_shift } else { train_shift - 256 };
    let mut data = vec![0_u8; WIDTH * HEIGHT * 4];
    for y in 0..HEIGHT {
        let shift = if (TOP_BAND..TOP_BAND + MID_BAND).contains(&y) {
            train_shift
        } else {
            background_shift
        };
        for x in 0..WIDTH {
            let source_x = (x as i16 + shift).rem_euclid(WIDTH as i16) as usize;
            let source_offset = (y * WIDTH + source_x) * 4;
            let target_offset = (y * WIDTH + x) * 4;
            data[target_offset..target_offset + 4]
                .copy_from_slice(&base[source_offset..source_offset + 4]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let (x, y) = field_window_center(0.0, 0.0, 20.0, 18.0);
    commands.spawn((
        SpriteBundle {
            texture: images.add(image),
            sprite: Sprite {
                custom_size: Some(Vec2::new(20.0 * TILE_SIZE, 18.0 * TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 20.0),
            ..default()
        },
        FieldCommandMarker,
    ));
    Ok(())
}

fn load_visible_magnet_train_base(asset_root: &AssetRoot) -> Result<Vec<u8>> {
    const WIDTH: usize = 20 * SOURCE_TILE_SIZE;
    const HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
    let tileset = image::open(asset_root.resolve_vendor("gfx/tilesets/train_station.png"))
        .context("decode magnet-train station tileset")?
        .to_rgba8();
    let background = std::fs::read(
        asset_root.resolve_vendor("gfx/overworld/magnet_train_bg.tilemap"),
    )
    .context("read magnet-train background tilemap")?;
    let foreground = std::fs::read(
        asset_root.resolve_vendor("gfx/overworld/magnet_train_fg.tilemap"),
    )
    .context("read magnet-train foreground tilemap")?;
    anyhow::ensure!(background.len() == 36, "magnet-train background tilemap must be 36 bytes");
    anyhow::ensure!(foreground.len() == 80, "magnet-train foreground tilemap must be 80 bytes");
    let mut data = vec![0_u8; WIDTH * HEIGHT * 4];
    let mut draw_tile = |tile_id: u8, tile_x: usize, tile_y: usize| -> Result<()> {
        let source_x = usize::from(tile_id % 16) * SOURCE_TILE_SIZE;
        let source_y = usize::from(tile_id / 16) * SOURCE_TILE_SIZE;
        anyhow::ensure!(
            source_x + SOURCE_TILE_SIZE <= tileset.width() as usize
                && source_y + SOURCE_TILE_SIZE <= tileset.height() as usize,
            "magnet-train tile {tile_id} lies outside the station tileset"
        );
        for row in 0..SOURCE_TILE_SIZE {
            for col in 0..SOURCE_TILE_SIZE {
                let pixel = tileset.get_pixel((source_x + col) as u32, (source_y + row) as u32);
                let target = (((tile_y * SOURCE_TILE_SIZE + row) * WIDTH)
                    + tile_x * SOURCE_TILE_SIZE
                    + col)
                    * 4;
                data[target..target + 4].copy_from_slice(&pixel.0);
            }
        }
        Ok(())
    };
    for tile_y in 0..18 {
        for tile_x in (0..20).step_by(2) {
            draw_tile(background[tile_y * 2], tile_x, tile_y)?;
            draw_tile(background[tile_y * 2 + 1], tile_x + 1, tile_y)?;
        }
    }
    for row in 0..4 {
        for col in 0..20 {
            draw_tile(foreground[row * 20 + col], col, row + 6)?;
        }
    }
    Ok(data)
}

fn spawn_visible_heal_machine(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(animation) = runtime_shell.visible_heal_machine.as_ref() else {
        return Ok(());
    };
    if rendered_art.heal_machine_ball_cache.is_none() {
        let frames = load_visible_heal_machine_ball_frames(asset_root, images);
        match frames {
            Ok((balls, lamps)) => {
                rendered_art.heal_machine_ball_cache = Some(balls);
                rendered_art.heal_machine_lamp_cache = Some(lamps);
            }
            Err(error) => {
                let message = format!("{error:#}");
                rendered_art.heal_machine_ball_error = Some(message.clone());
                anyhow::bail!(message);
            }
        }
    }
    if let Some(error) = rendered_art.heal_machine_ball_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let frames = rendered_art.heal_machine_ball_cache.as_ref().unwrap();
    let lamps = rendered_art.heal_machine_lamp_cache.as_ref().unwrap();
    let ball_frames = u16::from(animation.party_count) * 30;
    let visible_count = if animation.frame < ball_frames {
        usize::from((animation.frame / 30 + 1).min(u16::from(animation.party_count)))
    } else {
        usize::from(animation.party_count)
    };
    let palette_phase = if animation.frame >= ball_frames {
        usize::from((1 + (animation.frame - ball_frames) / 10) % 4)
    } else {
        0
    };
    // dbsprite stores (tile y, tile x, pixel y, pixel x), then hardware OAM
    // applies the usual x-8/y-16 origin. Keep these as exact source-screen
    // pixels rather than approximate tile centers.
    let positions: &[(f32, f32, bool)] = if animation.kind == 2 {
        &[
            (52.0, 65.0, false),
            (52.0, 70.0, false),
            (51.0, 61.0, false),
            (51.0, 74.0, false),
            (49.0, 57.0, false),
            (49.0, 77.0, false),
        ]
    } else {
        &[
            (30.0, 16.0, false),
            (30.0, 24.0, true),
            (35.0, 16.0, false),
            (35.0, 24.0, true),
            (40.0, 16.0, false),
            (40.0, 24.0, true),
        ]
    };
    if animation.kind != 2 {
        let elm_x = if animation.kind == 1 { 16.0 } else { 0.0 };
        let elm_y = if animation.kind == 1 { 32.0 } else { 0.0 };
        for (source_x, source_y) in [(24.0, 18.0), (24.0, 22.0)] {
            let (x, y) = battle_hud_tile_origin(
                (source_x + elm_x) / SOURCE_TILE_SIZE as f32,
                (source_y + elm_y) / SOURCE_TILE_SIZE as f32,
            );
            commands.spawn((
                SpriteBundle {
                    texture: lamps[palette_phase].handle.clone(),
                    sprite: Sprite { custom_size: Some(Vec2::splat(TILE_SIZE)), ..default() },
                    transform: Transform::from_xyz(x, y, 3.8),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
    }
    let elm_x = if animation.kind == 1 { 16.0 } else { 0.0 };
    let elm_y = if animation.kind == 1 { 32.0 } else { 0.0 };
    for &(source_x, source_y, flip_x) in
        positions.iter().take(visible_count.min(positions.len()))
    {
        let (x, y) = battle_hud_tile_origin(
            (source_x + elm_x) / SOURCE_TILE_SIZE as f32,
            (source_y + elm_y) / SOURCE_TILE_SIZE as f32,
        );
        commands.spawn((
            SpriteBundle {
                texture: frames[palette_phase].handle.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    flip_x,
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 3.8),
                ..default()
            },
            FieldCommandMarker,
        ));
    }
    Ok(())
}

fn load_visible_heal_machine_ball_frames(
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<([SpriteFrame; 4], [SpriteFrame; 4])> {
    let source = image::open(asset_root.resolve_vendor("gfx/overworld/heal_machine.png"))
        .context("decode heal-machine source art")?
        .to_rgba8();
    anyhow::ensure!(source.dimensions() == (8, 16), "heal-machine source art must be 8x16");
    let palette_source = std::fs::read_to_string(
        asset_root.resolve_vendor("gfx/overworld/heal_machine.pal"),
    )
    .context("read heal-machine palette")?;
    let colors = palette_source
        .lines()
        .filter_map(|line| {
            let line = line.split(';').next().unwrap_or("").trim();
            line.to_ascii_uppercase().starts_with("RGB").then_some(line)
        })
        .map(|line| rgb_triplet_to_u8(&parse_rgb_values(line)?))
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(colors.len() == 4, "heal-machine palette must contain four colors");
    let make_frame = |source_y: usize, palette: [[u8; 3]; 4], images: &mut Assets<Image>| {
        let mut data = vec![0_u8; 8 * 8 * 4];
        for row in 0..8 {
            for col in 0..8 {
                let gray = source.get_pixel(col as u32, (row + source_y) as u32)[0];
                let index = palette_index_from_gray(gray);
                let offset = (row * 8 + col) * 4;
                data[offset..offset + 3].copy_from_slice(&palette[index]);
                data[offset + 3] = if index == 0 { 0 } else { 255 };
            }
        }
        let mut image = Image::new(
            Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        SpriteFrame { handle: images.add(image), size: Vec2::splat(TILE_SIZE) }
    };
    let balls = std::array::from_fn(|phase| {
        let palette = std::array::from_fn(|index| colors[(index + phase) % 4]);
        make_frame(8, palette, images)
    });
    let lamps = std::array::from_fn(|phase| {
        let palette = std::array::from_fn(|index| colors[(index + phase) % 4]);
        make_frame(0, palette, images)
    });
    Ok((balls, lamps))
}

fn spawn_visible_balance_overlay(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    let Some(overlay) = runtime_shell.visible_balance_overlay.as_ref() else {
        return;
    };
    match overlay {
        VisibleBalanceOverlay::MoneyTopRight { money } => {
            spawn_visible_balance_window(
                commands, rendered_art, asset_root, images, 11.0, 0.0, 9, 3,
            );
            let (x, y) = battle_hud_tile_origin(12.0, 1.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images, &format!("¥{money:>6}"), x, y, 3.9,
            );
        }
        VisibleBalanceOverlay::CoinsTopRight { coins } => {
            spawn_visible_balance_window(
                commands, rendered_art, asset_root, images, 11.0, 0.0, 9, 3,
            );
            let (label_x, label_y) = battle_hud_tile_origin(12.0, 0.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images, "COIN", label_x, label_y, 3.9,
            );
            let (amount_x, amount_y) = battle_hud_tile_origin(13.0, 1.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("{coins:>4}"),
                amount_x,
                amount_y,
                3.9,
            );
        }
        VisibleBalanceOverlay::MoneyAndCoins { money, coins } => {
            spawn_visible_balance_window(
                commands, rendered_art, asset_root, images, 5.0, 0.0, 15, 5,
            );
            for (text, tile_x, tile_y) in [
                ("MONEY".to_string(), 6.0, 1.0),
                (format!("¥{money:>6}"), 12.0, 1.0),
                ("COIN".to_string(), 6.0, 3.0),
                (format!("{coins:>4}"), 15.0, 3.0),
            ] {
                let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
                spawn_scene_dialog_bitmap_text(
                    commands, rendered_art, asset_root, images, &text, x, y, 3.9,
                );
            }
        }
    }
}

fn spawn_visible_balance_window(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    tile_x: f32,
    tile_y: f32,
    width_tiles: usize,
    height_tiles: usize,
) {
    let (center_x, center_y) = field_window_center(
        tile_x,
        tile_y,
        width_tiles as f32,
        height_tiles as f32,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (width_tiles.saturating_sub(2)) as f32,
                    TILE_SIZE * (height_tiles.saturating_sub(2)) as f32,
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 3.6),
            ..default()
        },
        SceneDialogMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            tile_x,
            tile_y,
            width_tiles,
            height_tiles,
            3.7,
        );
    }
}

fn spawn_visible_mom_bank_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    let Some(bank) = runtime_shell.visible_mom_bank.as_ref() else {
        return;
    };
    let (width, height) = if bank.phase == VisibleMomBankPhase::Menu {
        (11, 11)
    } else {
        (20, 8)
    };
    spawn_visible_balance_window(
        commands, rendered_art, asset_root, images, 0.0, 0.0, width, height,
    );
    let lines = if bank.phase == VisibleMomBankPhase::Menu {
        vec![
            "What do you want".to_string(),
            "to do?".to_string(),
            format!("{}GET", if bank.menu_index == 0 { ">" } else { " " }),
            format!("{}SAVE", if bank.menu_index == 1 { ">" } else { " " }),
            format!("{}CHANGE", if bank.menu_index == 2 { ">" } else { " " }),
            format!("{}CANCEL", if bank.menu_index == 3 { ">" } else { " " }),
        ]
    } else {
        let action = if bank.phase == VisibleMomBankPhase::Withdraw {
            "WITHDRAW"
        } else {
            "DEPOSIT"
        };
        vec![
            format!("SAVED      ¥{:>6}", snapshot.trainer.moms_money),
            format!("HELD       ¥{:>6}", snapshot.trainer.money),
            format!("{action}   ¥{:06}", bank.amount),
            format!("             {}▲", " ".repeat(usize::from(bank.digit))),
        ]
    };
    for (index, line) in lines.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 1.0 + index as f32);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 4.0,
        );
    }
}

fn spawn_runtime_2d_menu(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    menu: &crate::RuntimeMenuSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> bool {
    let Some(grid) = menu
        .layout
        .vertical_menus
        .iter()
        .find(|grid| grid.two_dimensional && !grid.options.is_empty())
    else {
        return false;
    };
    let Some([left, top, right, bottom]) = menu.coords.or(menu.layout.declared_coords) else {
        return false;
    };
    let (Some(columns), Some(spacing)) = (grid.columns, grid.spacing) else {
        return false;
    };
    if right < left || bottom < top {
        return false;
    }
    let surface_id = vertical_menu_surface_id(menu, grid);
    let Some(selected) = strict_readonly_cursor_index(
        &runtime_shell.menu_cursor,
        &surface_id,
        grid.options.len(),
    ) else {
        return false;
    };
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        left as f32,
        top as f32,
        (right - left + 1) as f32,
        (bottom - top + 1) as f32,
        4.1,
    );
    for (index, option) in grid.options.iter().enumerate() {
        let row = index / columns;
        let column = index % columns;
        let (x, y) = battle_hud_tile_origin(
            left as f32 + 1.0 + column as f32 * spacing as f32,
            top as f32 + 1.0 + row as f32 * 2.0,
        );
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{}{}", if index == selected { ">" } else { " " }, option),
            x,
            y,
            4.4,
        );
    }
    true
}

fn spawn_field_shop_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.96, 0.96, 0.91),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 4.0),
            ..default()
        },
        SceneDialogMarker,
    ));
    let shop_notice = if runtime_shell.shop_welcome_seen {
        runtime_shell.shop_notice.as_deref()
    } else {
        Some("Welcome! How may I\nhelp you?")
    };
    if let Some(notice) = shop_notice {
        let visible_notice = visible_revealed_shell_notice_text(runtime_shell, notice);
        for (index, line) in wrap_boot_text_for_box(&visible_notice, 18, 4).iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(1.0, 12.0 + index as f32);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images, line, x, y, 4.5,
            );
        }
        if visible_field_text_reveal_is_complete_for_text(runtime_shell, notice)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images, "▼", x, y, 4.5,
            );
        }
        return Ok(());
    }
    if let Some(cursor) = runtime_shell.shop_top_cursor.as_ref() {
        let selected = strict_readonly_cursor_index(
            &Some(cursor.clone()), "shop:top", 3,
        )
        .context("shop top-menu cursor is invalid")?;
        let (x, y) = battle_hud_tile_origin(12.0, 0.5);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("¥{}", snapshot.trainer.money), x, y, 4.2,
        );
        for (index, option) in ["BUY", "SELL", "QUIT"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(13.0, 3.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("{}{}", if selected == index { ">" } else { " " }, option),
                x, y, 4.2,
            );
        }
        return Ok(());
    }
    let selling = runtime_shell.sell_cursor.is_some();
    let item_ids = if selling {
        sellable_carried_item_ids(snapshot)
    } else {
        shop.inventory.clone()
    };
    let (cursor, surface_id) = if selling {
        (&runtime_shell.sell_cursor, "sell:bag".to_string())
    } else {
        (&runtime_shell.menu_cursor, shop_cursor_surface_id(shop))
    };
    let selected = strict_readonly_cursor_index(cursor, &surface_id, item_ids.len())
        .with_context(|| {
            format!(
                "shop item cursor is invalid for {surface_id} with {} items",
                item_ids.len()
            )
        })?;
    for (tile_x, tile_y, text) in [
        (1.0, 0.5, if selling { "SELL".to_string() } else { "BUY".to_string() }),
        (12.0, 0.5, format!("¥{}", snapshot.trainer.money)),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, &text, x, y, 4.2,
        );
    }
    let scroll = visible_window_start(selected, item_ids.len(), 5);
    for (visible_index, item_id) in item_ids.iter().skip(scroll).take(5).enumerate() {
        let index = scroll + visible_index;
        let row = 3.0 + visible_index as f32 * 2.0;
        let item = snapshot
            .items
            .iter()
            .find(|item| item.item_id == *item_id)
            .with_context(|| format!("shop item {item_id} is missing"))?;
        let name = item.name.replace('_', " ");
        let line = if selling {
            let quantity = carried_item_quantity(snapshot, item_id)
                .with_context(|| format!("sell item {item_id} has no carried quantity"))?;
            format!(
                "{}{} ×{:02}",
                if selected == index { ">" } else { " " },
                compact_scene_label(&name, 10),
                quantity.min(99)
            )
        } else {
            format!(
                "{}{}",
                if selected == index { ">" } else { " " },
                compact_scene_label(&name, 10)
            )
        };
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, &line, x, y, 4.2,
        );
        let price = if selling { item.price / 2 } else { item.price };
        let (x, y) = battle_hud_tile_origin(14.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, &format!("¥{price}"), x, y, 4.2,
        );
    }
    let selected_item_id = item_ids
        .get(selected)
        .context("selected shop item is missing from its inventory")?;
    let description = snapshot
        .items
        .iter()
        .find(|item| item.item_id == *selected_item_id)
        .with_context(|| format!("selected shop item {selected_item_id} is missing"))?
        .description
        .as_str();
    for (index, line) in wrap_boot_text_for_box(description, 18, 3).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 14.0 + index as f32);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 4.2,
        );
    }
    if let Some(quantity) = runtime_shell.shop_quantity.as_ref() {
        anyhow::ensure!(quantity.quantity > 0, "shop quantity prompt has zero quantity");
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(1.0, 1.0, 1.0),
                    custom_size: Some(Vec2::new(TILE_SIZE * 13.0, TILE_SIZE * 3.0)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    battle_hud_tile_origin(13.5, 15.5).0,
                    battle_hud_tile_origin(13.5, 15.5).1,
                    4.3,
                ),
                ..default()
            },
            SceneDialogMarker,
        ));
        let total = u32::from(quantity.unit_price) * u32::from(quantity.quantity);
        let (x, y) = battle_hud_tile_origin(8.0, 15.0);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("×{:02} ¥{}", quantity.quantity, total), x, y, 4.5,
        );
    }
    Ok(())
}

fn spawn_field_storage_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let pc_box = snapshot
        .storage
        .boxes
        .iter()
        .find(|pc_box| pc_box.index == snapshot.storage.current_pc_box)
        .with_context(|| {
            format!(
                "current PC box {} is missing",
                snapshot.storage.current_pc_box
            )
        })?;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.83, 0.91, 0.96),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 4.0),
            ..default()
        },
        SceneDialogMarker,
    ));
    if runtime_shell.pc_notice.is_some() {
        spawn_pc_notice(commands, runtime_shell, rendered_art, asset_root, images);
        return Ok(());
    }
    if let Some(summary) = runtime_shell.bill_pc_box_summary.as_ref() {
        let summary_box = snapshot
            .storage
            .boxes
            .iter()
            .find(|entry| entry.index == summary.box_index)
            .with_context(|| format!("PC summary box {} is missing", summary.box_index))?;
        let slot = summary_box
            .slots
            .iter()
            .find(|entry| entry.index == summary.box_slot)
            .with_context(|| {
                format!(
                    "PC summary slot {} is missing from box {}",
                    summary.box_slot,
                    summary.box_index
                )
            })?;
        anyhow::ensure!(
            (1..=3).contains(&summary.page),
            "PC summary page {} is outside pages 1 through 3",
            summary.page
        );
        let pokemon = &slot.pokemon;
        let held_item_label = if let Some(item_id) = pokemon.item.as_deref() {
            snapshot
                .items
                .iter()
                .find(|item| item.item_id == item_id)
                .with_context(|| format!("PC summary held item {item_id} is missing"))?
                .name
                .replace('_', " ")
        } else {
            "NONE".to_string()
        };
        for learned in &pokemon.moves {
            snapshot
                .moves
                .iter()
                .find(|move_data| move_data.move_id == learned.name)
                .with_context(|| {
                    format!("PC summary move metadata {} is missing", learned.name)
                })?;
        }
        let mut rows = vec![
            (1.0, compact_scene_label(&pokemon.nickname, 10)),
            (2.0, format!("No.{:03}  \u{e10a}{:>2}", pokemon.species.int_id, pokemon.level)),
            (4.0, format!("< {}  {}  {} >", if summary.page == 1 { "[1]" } else { "1" }, if summary.page == 2 { "[2]" } else { "2" }, if summary.page == 3 { "[3]" } else { "3" })),
        ];
        match summary.page {
            1 => rows.extend([
                (6.0, format!("HP  {:>3}/{:>3}", pokemon.hp, pokemon.max_hp)),
                (7.0, format!("STATUS  {}", party_status_token(pokemon))),
                (9.0, format!("EXP POINTS {:>7}", pokemon.experience.max(0))),
                (11.0, format!("ITEM {held_item_label}")),
            ]),
            2 => {
                if pokemon.moves.is_empty() {
                    rows.push((7.0, "NO MOVES".to_string()));
                } else {
                    for (index, learned) in pokemon.moves.iter().take(4).enumerate() {
                        let row = 6.0 + index as f32 * 2.0;
                        rows.push((row, battle_move_display_name(snapshot, &learned.name)));
                        rows.push((row + 1.0, visible_move_pp_text(snapshot, learned)));
                    }
                }
            }
            _ => rows.extend([
                (6.0, format!("OT/{}", pokemon.original_trainer_name)),
                (7.0, format!("IDNo.{:05}", pokemon.original_trainer_id)),
                (9.0, format!("ATTACK  {:>3}", pokemon.attack)),
                (10.0, format!("DEFENSE {:>3}", pokemon.defense)),
                (11.0, format!("SPCL.ATK{:>3}", pokemon.special_attack)),
                (12.0, format!("SPCL.DEF{:>3}", pokemon.special_defense)),
                (13.0, format!("SPEED   {:>3}", pokemon.speed)),
            ]),
        }
        rows.push((16.0, "A NEXT  B BACK".to_string()));
        for (row, text) in rows {
            let (x, y) = battle_hud_tile_origin(1.0, row);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images,
                &compact_scene_label(&text, 18), x, y, 4.2,
            );
        }
        return Ok(());
    }
    let option_count = if runtime_shell.bill_pc_move_open {
        crate::core::models::MAX_BOX_MONS
    } else {
        pc_box.slots.len()
    };
    let selected = strict_readonly_cursor_index(
        &runtime_shell.storage_cursor,
        &storage_cursor_surface_id(pc_box.index),
        option_count,
    )
    .with_context(|| {
        format!(
            "PC storage cursor is invalid for box {} with {option_count} entries",
            pc_box.index
        )
    })?;
    for (tile_x, text) in [
        (1.0, format!("< {} >", compact_scene_label(&pc_box.name, 10))),
        (14.0, format!("{:02}/20", pc_box.count)),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, 0.5);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, &text, x, y, 4.2,
        );
    }
    let scroll = visible_window_start(selected, option_count, 7);
    for visible_index in 0..7 {
        let slot_index = scroll + visible_index;
        if slot_index >= option_count {
            break;
        }
        let row = 2.5 + visible_index as f32 * 1.5;
        let slot = pc_box.slots.iter().find(|slot| slot.index == slot_index);
        let label = slot
            .map(|slot| compact_scene_label(&slot.pokemon.nickname, 10))
            .unwrap_or_else(|| "---".to_string());
        let level = slot.map(|slot| format!("L{:02}", slot.pokemon.level)).unwrap_or_default();
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{}{:02} {}", if selected == slot_index { ">" } else { " " }, slot_index + 1, label),
            x, y, 4.2,
        );
        let (x, y) = battle_hud_tile_origin(15.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, &level, x, y, 4.2,
        );
    }
    let action = if runtime_shell.bill_pc_move_open {
        "A PLACE  B CANCEL"
    } else {
        "A WITHDRAW  SELECT RELEASE"
    };
    let (x, y) = battle_hud_tile_origin(1.0, 14.0);
    spawn_scene_dialog_bitmap_text(
        commands, rendered_art, asset_root, images,
        &compact_scene_label(action, 18), x, y, 4.2,
    );
    let (x, y) = battle_hud_tile_origin(1.0, 16.0);
    spawn_scene_dialog_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("PARTY {:02}/6", snapshot.storage.party_count), x, y, 4.2,
    );
    if let Some(prompt) = runtime_shell.pending_pc_release.as_ref() {
        spawn_battle_window(
            commands, rendered_art, asset_root, images,
            1.0, 10.0, 18.0, 8.0, 4.5,
        );
        let selected = strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "pc:release-confirm",
            2,
        )
        .context("PC release confirmation cursor is invalid")?;
        let (x, y) = battle_hud_tile_origin(2.0, 11.0);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("RELEASE {}?", compact_scene_label(&prompt.nickname, 9)), x, y, 4.8,
        );
        for (index, label) in ["YES", "NO"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(12.0, 13.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("{}{}", if selected == index { ">" } else { " " }, label),
                x, y, 4.8,
            );
        }
    } else if let Some(cursor) = runtime_shell.bill_pc_pokemon_action_cursor.as_ref() {
        spawn_battle_window(
            commands, rendered_art, asset_root, images,
            10.0, 8.0, 9.0, 9.0, 4.5,
        );
        let selected = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pc:pokemon-actions",
            4,
        )
        .context("PC Pokémon action cursor is invalid")?;
        for (index, label) in ["WITHDRAW", "STATS", "RELEASE", "CANCEL"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(11.0, 9.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("{}{}", if selected == index { ">" } else { " " }, label),
                x, y, 4.8,
            );
        }
    }
    Ok(())
}

fn spawn_pc_notice(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    if let Some(notice) = runtime_shell.pc_notice.as_deref() {
        let visible_notice = visible_revealed_shell_notice_text(runtime_shell, notice);
        spawn_battle_window(
            commands, rendered_art, asset_root, images,
            1.0, 10.0, 18.0, 8.0, 4.5,
        );
        for (index, line) in wrap_boot_text_for_box(&visible_notice, 16, 6).iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(2.0, 11.0 + index as f32);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images, line, x, y, 4.8,
            );
        }
        if visible_field_text_reveal_is_complete_for_text(runtime_shell, notice)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_scene_dialog_bitmap_text(
                commands, rendered_art, asset_root, images, "▼", x, y, 4.8,
            );
        }
    }
}

fn spawn_field_pc_item_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.90, 0.93, 0.84),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 4.0),
            ..default()
        },
        SceneDialogMarker,
    ));
    let items = snapshot
        .bag
        .pc_items
        .iter()
        .filter(|item| item.quantity > 0)
        .collect::<Vec<_>>();
    let selected = strict_readonly_cursor_index(
        &runtime_shell.pc_item_cursor,
        "pc:items",
        items.len(),
    )
    .with_context(|| format!("PC item cursor is invalid for {} items", items.len()))?;
    let (x, y) = battle_hud_tile_origin(1.0, 0.5);
    spawn_scene_dialog_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("ITEM STORAGE {:02}", items.len()), x, y, 4.2,
    );
    let scroll = visible_window_start(selected, items.len(), 7);
    for (visible_index, item) in items.iter().skip(scroll).take(7).enumerate() {
        let index = scroll + visible_index;
        let row = 2.5 + visible_index as f32 * 1.5;
        let catalog = snapshot
            .items
            .iter()
            .find(|catalog| catalog.item_id == item.item_id)
            .with_context(|| format!("PC item {} is missing", item.item_id))?;
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!(
                "{}{}",
                if selected == index { ">" } else { " " },
                compact_scene_label(&catalog.name.replace('_', " "), 11)
            ),
            x, y, 4.2,
        );
        let (x, y) = battle_hud_tile_origin(16.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("×{:02}", item.quantity.min(99)), x, y, 4.2,
        );
    }
    let selected_item = items.get(selected).context("selected PC item is missing")?;
    let description = snapshot
        .items
        .iter()
        .find(|catalog| catalog.item_id == selected_item.item_id)
        .with_context(|| format!("selected PC item {} is missing", selected_item.item_id))?
        .description
        .as_str();
    for (index, line) in wrap_boot_text_for_box(description, 18, 3).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 14.0 + index as f32);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 4.2,
        );
    }
    Ok(())
}

fn spawn_field_pc_box_selection_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.83, 0.91, 0.96),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 4.0),
            ..default()
        },
        SceneDialogMarker,
    ));
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_box_cursor,
        "pc:bill-boxes",
        crate::core::models::MAX_PC_BOXES,
    )
    .context("PC box-selection cursor is invalid")?;
    let (x, y) = battle_hud_tile_origin(1.0, 0.5);
    spawn_scene_dialog_bitmap_text(
        commands, rendered_art, asset_root, images, "CHANGE BOX", x, y, 4.2,
    );
    let scroll = visible_window_start(selected, crate::core::models::MAX_PC_BOXES, 7);
    for visible_index in 0..7 {
        let index = scroll + visible_index;
        if index >= crate::core::models::MAX_PC_BOXES {
            break;
        }
        let pc_box = snapshot.storage.boxes.iter()
            .find(|pc_box| pc_box.index == index)
            .with_context(|| format!("PC box {index} is missing from box selection"))?;
        let (name, count) = (pc_box.name.as_str(), pc_box.count);
        let row = 2.5 + visible_index as f32 * 2.0;
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!(
                "{}{:02} {}",
                if selected == index { ">" } else { " " },
                index + 1,
                compact_scene_label(name, 10)
            ),
            x, y, 4.2,
        );
        let (x, y) = battle_hud_tile_origin(16.0, row);
        spawn_scene_dialog_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{:02}", count), x, y, 4.2,
        );
    }
    Ok(())
}

/// Update only the changing dialog content. The textbox background and its
/// 48 frame tiles are retained between character advances; rebuilding those
/// entities for every glyph was the dominant visible-dialog CPU spike.
fn spawn_scene_dialog_text_content(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let entries = visible_scene_dialog_entries(snapshot, runtime_shell)?;
    let prompt_active = scene_dialog_yes_no_active(snapshot, runtime_shell);
    for (index, entry) in entries
        .iter()
        .filter(|entry| !(prompt_active && is_visible_yes_no_prompt_entry(entry)))
        .take(FIELD_TEXT_BOX_VISIBLE_ROWS)
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(
            FIELD_TEXT_BOX_TEXT_LEFT_TILE,
            FIELD_TEXT_BOX_TEXT_TOP_TILE + index as f32 * FIELD_TEXT_BOX_ROW_SPACING_TILES,
        );
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            entry,
            x,
            y,
            4.2,
        );
    }
    if prompt_active {
        spawn_visible_yes_no_prompt_box(
            snapshot,
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        );
    } else if field_dialogue_prompt_arrow_visible(snapshot, runtime_shell) {
        let (x, y) = battle_hud_tile_origin(
            FIELD_TEXT_BOX_LEFT_TILE + FIELD_TEXT_BOX_WIDTH_TILES - 2.0,
            FIELD_TEXT_BOX_TOP_TILE + FIELD_TEXT_BOX_HEIGHT_TILES - 2.0,
        );
        spawn_scene_dialog_bitmap_text(commands, rendered_art, asset_root, images, "▼", x, y, 4.2);
    }
    Ok(())
}

fn field_dialogue_prompt_arrow_visible(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> bool {
    snapshot.ui.pending_text_wait.is_some()
        && pending_text_wait_uses_prompt_button(runtime_shell)
        && visible_field_dialogue_is_fully_revealed(runtime_shell, snapshot)
        && runtime_shell.lcd_animation_frame & (1 << 4) != 0
}

fn pending_text_wait_uses_prompt_button(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .pending_text_wait
        .as_ref()
        .is_some_and(|wait| !wait.command.eq_ignore_ascii_case("waitbutton"))
}

fn is_visible_yes_no_prompt_entry(entry: &str) -> bool {
    let normalized = entry
        .trim()
        .trim_start_matches('>')
        .trim()
        .to_ascii_uppercase();
    normalized == "YES"
        || normalized == "NO"
        || (normalized.contains("YES") && normalized.contains("NO"))
}

fn scene_dialog_yes_no_active(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> bool {
    snapshot.ui.pending_yes_no.is_some()
        || runtime_shell.pending_phone_prompt.is_some()
        || runtime_shell.pending_contextual_field_move.is_some()
        || runtime_shell.held_item_swap_prompt
        || runtime_shell.party_mail_take_stage.is_some()
        || runtime_shell.pc_confirmation.is_some()
        || runtime_shell
            .pack_toss
            .as_ref()
            .is_some_and(|toss| toss.confirming)
        || runtime_shell
            .save_flow
            .as_ref()
            .is_some_and(|flow| matches!(
                flow.stage,
                VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt
            ))
        || runtime_shell.tmhm_teach_prompt_cursor.is_some()
        || runtime_shell.tmhm_decision_prompt_cursor.is_some()
}

fn scene_dialog_yes_no_cursor_index(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> usize {
    if snapshot.ui.pending_yes_no.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
            .unwrap_or(0);
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:phone-number", 2)
            .unwrap_or(0);
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "field:move-confirm",
            2,
        )
        .unwrap_or(0);
    }
    if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
        return strict_readonly_cursor_index(
            &runtime_shell.tmhm_teach_prompt_cursor,
            "pack:tmhm:teach-prompt",
            2,
        )
        .unwrap_or(0);
    }
    if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
        return strict_readonly_cursor_index(
            &runtime_shell.tmhm_decision_prompt_cursor,
            "pack:tmhm:decision",
            2,
        )
        .unwrap_or(0);
    }
    if runtime_shell.held_item_swap_prompt {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "party:held-item-swap",
            2,
        )
        .unwrap_or(0);
    }
    if let Some(stage) = runtime_shell.party_mail_take_stage {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            if stage == 1 { "party:mail-send-pc" } else { "party:mail-lose-message" },
            2,
        )
        .unwrap_or(0);
    }
    if runtime_shell.pc_confirmation.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "pc:confirmation", 2)
            .unwrap_or(1);
    }
    if runtime_shell
        .pack_toss
        .as_ref()
        .is_some_and(|toss| toss.confirming)
    {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "pack:toss-confirm",
            2,
        )
        .unwrap_or(0);
    }
    runtime_shell
        .save_flow
        .as_ref()
        .filter(|flow| matches!(
            flow.stage,
            VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt
        ))
        .map(|flow| flow.yes_no_index.min(1))
        .unwrap_or(0)
}

fn spawn_visible_yes_no_prompt_box(
    snapshot: &RuntimeShellSnapshot,
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    // `YesNoBox` in home/menu.asm sets b=SCREEN_WIDTH-6 and c=7, then
    // `_YesNoBox` produces border coordinates (14,7)..(19,11).  It is a
    // five-tile menu above the field textbox, not a four-tile subwindow
    // inside it.
    let (center_x, center_y) = field_window_center(
        FIELD_YES_NO_LEFT_TILE,
        FIELD_YES_NO_TOP_TILE,
        FIELD_YES_NO_WIDTH_TILES,
        FIELD_YES_NO_HEIGHT_TILES,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (FIELD_YES_NO_WIDTH_TILES - 2.0),
                    TILE_SIZE * (FIELD_YES_NO_HEIGHT_TILES - 2.0),
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.25),
            ..default()
        },
        SceneDialogMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE,
            FIELD_YES_NO_WIDTH_TILES as usize,
            FIELD_YES_NO_HEIGHT_TILES as usize,
            4.3,
        );
    }
    let selected = scene_dialog_yes_no_cursor_index(snapshot, runtime_shell);
    for (index, label) in ["YES", "NO"].into_iter().enumerate() {
        let marker = if index == selected { ">" } else { " " };
        let (x, y) = battle_hud_tile_origin(
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE + 1.0 + index as f32 * 2.0,
        );
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{marker}{label}"),
            x,
            y,
            4.4,
        );
    }
}

fn spawn_scene_dialog_text_box(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    z: f32,
) {
    let (center_x, center_y) = field_window_center(
        FIELD_TEXT_BOX_LEFT_TILE,
        FIELD_TEXT_BOX_TOP_TILE,
        FIELD_TEXT_BOX_WIDTH_TILES,
        FIELD_TEXT_BOX_HEIGHT_TILES,
    );
    // The text plane must not depend on the frame image cache. Otherwise an
    // asset-cache miss exposes map pixels through a dialog for a frame.
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (FIELD_TEXT_BOX_WIDTH_TILES - 2.0),
                    TILE_SIZE * (FIELD_TEXT_BOX_HEIGHT_TILES - 2.0),
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, z),
            ..default()
        },
        SceneDialogMarker,
        SceneDialogTextBoxBackgroundMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            FIELD_TEXT_BOX_LEFT_TILE,
            FIELD_TEXT_BOX_TOP_TILE,
            FIELD_TEXT_BOX_WIDTH_TILES as usize,
            FIELD_TEXT_BOX_HEIGHT_TILES as usize,
            z + 0.05,
        );
    }
}

fn spawn_visible_name_entry_screen(
    commands: &mut Commands,
    _runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    input: &PendingNameInput,
) {
    let key = NameEntryArtKey {
        label: input.label.clone(),
        value: input.value.clone(),
        cursor_column: input.cursor_column,
        cursor_row: input.cursor_row,
        case: input.case,
    };
    if !rendered_art.name_entry_cache.contains_key(&key) {
        match load_name_entry_frame(asset_root, input, images) {
            Ok(frame) => {
                rendered_art.name_entry_errors.remove(&key);
                rendered_art.name_entry_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .name_entry_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    let Some(frame) = rendered_art.name_entry_cache.get(&key).cloned() else {
        return;
    };
    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    let screen_size = Vec2::new(
        NAME_ENTRY_SCREEN_TILE_WIDTH as f32 * TILE_SIZE,
        NAME_ENTRY_SCREEN_TILE_HEIGHT as f32 * TILE_SIZE,
    );
    let screen_center = name_entry_screen_center();
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(screen_size),
                ..default()
            },
            transform: Transform::from_xyz(screen_center.x, screen_center.y, 5.9),
            ..default()
        },
        SceneDialogMarker,
    ));
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * scale),
                ..default()
            },
            transform: Transform::from_xyz(screen_center.x, screen_center.y, 6.0),
            ..default()
        },
        SceneDialogMarker,
    ));
}

fn spawn_visible_name_choice_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    choice: &VisibleNameChoice,
) {
    if let Some(capture) = runtime_shell.pending_standard_capture.as_ref() {
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            BATTLE_TEXT_BOX_LEFT_TILE,
            BATTLE_TEXT_BOX_TOP_TILE,
            BATTLE_TEXT_BOX_WIDTH_TILES,
            BATTLE_TEXT_BOX_HEIGHT_TILES,
            5.9,
        );
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE,
            FIELD_YES_NO_WIDTH_TILES,
            FIELD_YES_NO_HEIGHT_TILES,
            6.1,
        );
        for (line_index, line) in [
            "Give a nickname to".to_string(),
            capture.default_name.clone(),
        ]
        .into_iter()
        .enumerate()
        {
            let (x, y) = battle_hud_tile_origin(1.0, 13.0 + line_index as f32);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &line,
                x,
                y,
                6.2,
            );
        }
        for (index, label) in choice.options.iter().take(2).enumerate() {
            let (x, y) = battle_hud_tile_origin(
                FIELD_YES_NO_LEFT_TILE,
                FIELD_YES_NO_TOP_TILE + 1.0 + index as f32 * 2.0,
            );
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("{}{label}", if index == choice.selected { ">" } else { " " }),
                x,
                y,
                6.3,
            );
        }
        return;
    }
    const LEFT_TILE: f32 = 0.0;
    const TOP_TILE: f32 = 0.0;
    const WIDTH_TILES: usize = 10;
    const HEIGHT_TILES: usize = 15;
    let (center_x, center_y) =
        field_window_center(LEFT_TILE, TOP_TILE, WIDTH_TILES as f32, HEIGHT_TILES as f32);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.91, 0.96, 0.86),
                custom_size: Some(Vec2::new(
                    WIDTH_TILES as f32 * TILE_SIZE,
                    HEIGHT_TILES as f32 * TILE_SIZE,
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 5.9),
            ..default()
        },
        SceneDialogMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            LEFT_TILE,
            TOP_TILE,
            WIDTH_TILES,
            HEIGHT_TILES,
            6.0,
        );
    }
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        "NAME",
        battle_hud_tile_origin(1.0, 1.0).0,
        battle_hud_tile_origin(1.0, 1.0).1,
        6.1,
    );
    for (index, label) in choice.options.iter().enumerate() {
        let row_y = 3.0 + index as f32 * 2.0;
        if index == choice.selected {
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                ">",
                battle_hud_tile_origin(1.0, row_y).0,
                battle_hud_tile_origin(1.0, row_y).1,
                6.1,
            );
        }
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            label,
            battle_hud_tile_origin(2.0, row_y).0,
            battle_hud_tile_origin(2.0, row_y).1,
            6.1,
        );
    }
}

fn spawn_scene_dialog_bitmap_text(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
    x: f32,
    y: f32,
    z: f32,
) {
    for (index, frame) in bitmap_text_frames(rendered_art, asset_root, images, text)
        .into_iter()
        .enumerate()
    {
        let key = dialog_glyph_key(x, y, index);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x + index as f32 * BITMAP_FONT_ADVANCE, y, z),
                ..default()
            },
            SceneDialogMarker,
            DialogGlyphMarker { key },
        ));
    }
}

fn dialog_glyph_key(x: f32, y: f32, index: usize) -> u64 {
    // All dialog coordinates are tile-derived half-pixel values. Quantizing
    // them keeps the identity stable without hashing floating-point bytes.
    let x = (x * 16.0).round() as i32 as u32 as u64;
    let y = (y * 16.0).round() as i32 as u32 as u64;
    (x << 32) ^ (y << 8) ^ index as u64
}

/// Update the retained glyph sprites when the dialog's layout is unchanged.
/// Returning false asks the caller to use the normal rebuild path (for
/// example when a second line appears or a yes/no window is opened).
fn update_scene_dialog_text_content_in_place<F: QueryFilter>(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    glyphs: &mut Query<
        (
            &DialogGlyphMarker,
            &mut Handle<Image>,
            &mut Transform,
            &mut Sprite,
        ),
        F,
    >,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> bool {
    let Ok(entries) = visible_scene_dialog_entries(snapshot, runtime_shell) else {
        return false;
    };
    let prompt_active = scene_dialog_yes_no_active(snapshot, runtime_shell);
    let mut desired = Vec::new();
    for (index, entry) in entries
        .iter()
        .filter(|entry| !(prompt_active && is_visible_yes_no_prompt_entry(entry)))
        .take(FIELD_TEXT_BOX_VISIBLE_ROWS)
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(
            FIELD_TEXT_BOX_TEXT_LEFT_TILE,
            FIELD_TEXT_BOX_TEXT_TOP_TILE + index as f32 * FIELD_TEXT_BOX_ROW_SPACING_TILES,
        );
        for (glyph_index, frame) in bitmap_text_frames(rendered_art, asset_root, images, entry)
            .into_iter()
            .enumerate()
        {
            desired.push((
                dialog_glyph_key(x, y, glyph_index),
                frame.handle,
                frame.size,
                Transform::from_xyz(x + glyph_index as f32 * BITMAP_FONT_ADVANCE, y, 4.2),
            ));
        }
    }
    if prompt_active {
        for (index, label) in ["YES", "NO"].into_iter().enumerate() {
            let selected = scene_dialog_yes_no_cursor_index(snapshot, runtime_shell);
            let marker = if index == selected { ">" } else { " " };
            let (x, y) = battle_hud_tile_origin(
                FIELD_YES_NO_LEFT_TILE,
                FIELD_YES_NO_TOP_TILE + 1.0 + index as f32 * 2.0,
            );
            let text = format!("{marker}{label}");
            for (glyph_index, frame) in bitmap_text_frames(rendered_art, asset_root, images, &text)
                .into_iter()
                .enumerate()
            {
                desired.push((
                    dialog_glyph_key(x, y, glyph_index),
                    frame.handle,
                    frame.size,
                    Transform::from_xyz(x + glyph_index as f32 * BITMAP_FONT_ADVANCE, y, 4.4),
                ));
            }
        }
    } else if field_dialogue_prompt_arrow_visible(snapshot, runtime_shell) {
        let (x, y) = battle_hud_tile_origin(
            FIELD_TEXT_BOX_LEFT_TILE + FIELD_TEXT_BOX_WIDTH_TILES - 2.0,
            FIELD_TEXT_BOX_TOP_TILE + FIELD_TEXT_BOX_HEIGHT_TILES - 2.0,
        );
        for (glyph_index, frame) in bitmap_text_frames(rendered_art, asset_root, images, "▼")
            .into_iter()
            .enumerate()
        {
            desired.push((
                dialog_glyph_key(x, y, glyph_index),
                frame.handle,
                frame.size,
                Transform::from_xyz(x + glyph_index as f32 * BITMAP_FONT_ADVANCE, y, 4.2),
            ));
        }
    }
    let mut existing = glyphs.iter_mut().collect::<Vec<_>>();
    if existing.len() != desired.len() {
        return false;
    }
    existing.sort_by_key(|(marker, _, _, _)| marker.key);
    desired.sort_by_key(|(key, _, _, _)| *key);
    for ((marker, mut texture, mut transform, mut sprite), (key, handle, size, next_transform)) in
        existing.into_iter().zip(desired)
    {
        if marker.key != key {
            return false;
        }
        if texture.id() != handle.id() {
            *texture = handle;
        }
        *transform = next_transform;
        sprite.custom_size = Some(size);
    }
    true
}

const NAME_ENTRY_SCREEN_TILE_WIDTH: usize = 20;
const NAME_ENTRY_SCREEN_TILE_HEIGHT: usize = 18;
const NAME_ENTRY_KEYBOARD_COLUMNS: usize = 17;
const NAME_ENTRY_KEYBOARD_START_X: usize = 2;
const NAME_ENTRY_KEYBOARD_START_Y: usize = 8;
const NAME_ENTRY_KEYBOARD_ROW_SPACING: usize = 2;
const NAME_ENTRY_NAME_X: usize = 5;
const NAME_ENTRY_NAME_Y: usize = 6;
const NAME_ENTRY_CURSOR_BASE_X: i16 = 24;
const NAME_ENTRY_CURSOR_BASE_Y: i16 = 80;
const NAME_ENTRY_ROW_PIXEL_STEP: i16 = 0x10;
const NAME_ENTRY_BORDER_TILE: u8 = 0x60;
const NAME_ENTRY_SPACE_TILE: u8 = 0x7f;
const NAME_ENTRY_UNDERLINE_TILE: u8 = 0xf2;
const NAME_ENTRY_MIDDLE_LINE_TILE: u8 = 0xeb;
const NAME_ENTRY_LETTER_X_OFFSETS: [i16; 9] =
    [0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
const NAME_ENTRY_CASE_ROW_X_OFFSETS: [i16; 9] =
    [0x00, 0x00, 0x00, 0x30, 0x30, 0x30, 0x60, 0x60, 0x60];
const NAME_ENTRY_SMALL_CURSOR_OAM: [(i16, i16, i16, i16, usize, bool, bool); 4] = [
    (-1, -1, 7, 7, 0, false, false),
    (0, -1, 0, 7, 0, true, false),
    (-1, 0, 7, 0, 0, false, true),
    (0, 0, 0, 0, 0, true, true),
];
const NAME_ENTRY_BIG_CURSOR_OAM: [(i16, i16, i16, i16, usize, bool, bool); 10] = [
    (0, -1, 0, 7, 0, false, false),
    (1, -1, 0, 7, 1, false, false),
    (2, -1, 0, 7, 1, false, false),
    (3, -1, 0, 7, 1, false, false),
    (4, -1, 0, 7, 0, true, false),
    (0, 0, 0, 0, 0, false, true),
    (1, 0, 0, 0, 1, false, true),
    (2, 0, 0, 0, 1, false, true),
    (3, 0, 0, 0, 1, false, true),
    (4, 0, 0, 0, 0, true, true),
];

fn name_entry_screen_center() -> Vec3 {
    Vec3::new(
        PLAYFIELD_LEFT + NAME_ENTRY_SCREEN_TILE_WIDTH as f32 * TILE_SIZE * 0.5,
        PLAYFIELD_TOP - NAME_ENTRY_SCREEN_TILE_HEIGHT as f32 * TILE_SIZE * 0.5,
        0.0,
    )
}

fn load_name_entry_frame(
    asset_root: &AssetRoot,
    input: &PendingNameInput,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = image::open(assets.join("gfx/font/font.png"))
        .context("decode naming screen font PNG")?
        .to_rgba8();
    let cursor = image::open(assets.join("gfx/naming_screen/cursor.png"))
        .context("decode naming screen cursor PNG")?
        .to_rgba8();
    let border = image::open(assets.join("gfx/naming_screen/border.png"))
        .context("decode naming screen border PNG")?
        .to_rgba8();
    let underline = image::open(assets.join("gfx/naming_screen/underline.png"))
        .context("decode naming screen underline PNG")?
        .to_rgba8();
    let middle_line = image::open(assets.join("gfx/naming_screen/middle_line.png"))
        .context("decode naming screen middle-line PNG")?
        .to_rgba8();

    let tilemap = build_name_entry_tilemap(input)?;
    let width = NAME_ENTRY_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = NAME_ENTRY_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![255_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    for (tile_y, row) in tilemap.iter().enumerate() {
        for (tile_x, tile_id) in row.iter().copied().enumerate() {
            draw_name_entry_tile(
                tile_id,
                &font,
                &border,
                &underline,
                &middle_line,
                tile_x * SOURCE_TILE_SIZE,
                tile_y * SOURCE_TILE_SIZE,
                &mut data,
            )?;
        }
    }
    draw_name_entry_cursor(input, &cursor, &mut data)?;

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

fn build_name_entry_tilemap(input: &PendingNameInput) -> Result<Vec<Vec<u8>>> {
    let mut tilemap = vec![
        vec![NAME_ENTRY_BORDER_TILE; NAME_ENTRY_SCREEN_TILE_WIDTH];
        NAME_ENTRY_SCREEN_TILE_HEIGHT
    ];
    clear_name_entry_box(&mut tilemap, 1, 1, 18, 6);
    clear_name_entry_box(&mut tilemap, 1, 8, 18, 7);
    clear_name_entry_box(&mut tilemap, 1, 16, 18, 1);
    write_name_entry_string(
        &mut tilemap,
        &input.label.to_uppercase(),
        NAME_ENTRY_NAME_X,
        2,
    )?;

    let mut name_tiles = vec![NAME_ENTRY_MIDDLE_LINE_TILE; input.max_length];
    if !name_tiles.is_empty() {
        name_tiles[0] = NAME_ENTRY_UNDERLINE_TILE;
    }
    for (index, ch) in input
        .value
        .chars()
        .take(input.max_length)
        .enumerate()
    {
        name_tiles[index] = name_entry_char_tile(ch)
            .with_context(|| format!("unsupported naming-screen value char {ch:?}"))?;
        if index + 1 < input.max_length {
            name_tiles[index + 1] = NAME_ENTRY_UNDERLINE_TILE;
        }
    }
    for (index, tile_id) in name_tiles.into_iter().enumerate() {
        tilemap[NAME_ENTRY_NAME_Y][NAME_ENTRY_NAME_X + index] = tile_id;
    }

    for (row_index, row_text) in visible_name_input_layout(input.case).iter().enumerate() {
        let row_tiles = name_entry_string_tiles(row_text)?;
        let tile_y = NAME_ENTRY_KEYBOARD_START_Y + row_index * NAME_ENTRY_KEYBOARD_ROW_SPACING;
        for (column, tile_id) in row_tiles.into_iter().enumerate() {
            tilemap[tile_y][NAME_ENTRY_KEYBOARD_START_X + column] = tile_id;
        }
    }
    Ok(tilemap)
}

fn clear_name_entry_box(tilemap: &mut [Vec<u8>], x: usize, y: usize, width: usize, height: usize) {
    for row in y..y + height {
        for col in x..x + width {
            if let Some(tile) = tilemap.get_mut(row).and_then(|line| line.get_mut(col)) {
                *tile = 0;
            }
        }
    }
}

fn write_name_entry_string(tilemap: &mut [Vec<u8>], text: &str, x: usize, y: usize) -> Result<()> {
    let mut tile_x = x;
    for token in tokenize_name_entry_string(text) {
        if token == "@" {
            break;
        }
        if let Some(tile_id) = name_entry_token_tile(&token) {
            if let Some(tile) = tilemap.get_mut(y).and_then(|line| line.get_mut(tile_x)) {
                *tile = tile_id;
            }
        }
        tile_x += 1;
    }
    Ok(())
}

fn name_entry_string_tiles(text: &str) -> Result<Vec<u8>> {
    let mut tiles = Vec::new();
    for token in tokenize_name_entry_string(text) {
        if token == "@" {
            break;
        }
        let tile_id = name_entry_token_tile(&token)
            .with_context(|| format!("unsupported naming-screen glyph {token:?}"))?;
        tiles.push(tile_id);
    }
    if tiles.len() != NAME_ENTRY_KEYBOARD_COLUMNS {
        anyhow::bail!(
            "naming-screen keyboard row {text:?} produced {} tiles, expected {}",
            tiles.len(),
            NAME_ENTRY_KEYBOARD_COLUMNS
        );
    }
    Ok(tiles)
}

fn tokenize_name_entry_string(text: &str) -> Vec<String> {
    const MULTI: [&str; 9] = ["<PK>", "<MN>", "'d", "'l", "'m", "'r", "'s", "'t", "'v"];
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < text.len() {
        if let Some(token) = MULTI
            .iter()
            .find(|token| text[index..].starts_with(**token))
        {
            tokens.push((*token).to_string());
            index += token.len();
            continue;
        }
        let ch = text[index..].chars().next().expect("valid char boundary");
        tokens.push(ch.to_string());
        index += ch.len_utf8();
    }
    tokens
}

fn name_entry_char_tile(ch: char) -> Option<u8> {
    name_entry_token_tile(&ch.to_string())
}

fn name_entry_token_tile(token: &str) -> Option<u8> {
    if token.len() == 1 {
        let byte = token.as_bytes()[0];
        if byte.is_ascii_uppercase() {
            return Some(0x80 + (byte - b'A'));
        }
        if byte.is_ascii_lowercase() {
            return Some(0xa0 + (byte - b'a'));
        }
        if byte.is_ascii_digit() {
            return Some(0xf6 + (byte - b'0'));
        }
    }
    Some(match token {
        " " => NAME_ENTRY_SPACE_TILE,
        "(" => 0x9a,
        ")" => 0x9b,
        ":" => 0x9c,
        ";" => 0x9d,
        "[" => 0x9e,
        "]" => 0x9f,
        "'d" => 0xd0,
        "'l" => 0xd1,
        "'m" => 0xd2,
        "'r" => 0xd3,
        "'s" => 0xd4,
        "'t" => 0xd5,
        "'v" => 0xd6,
        "'" => 0xe0,
        "<PK>" => 0xe1,
        "<MN>" => 0xe2,
        "-" => 0xe3,
        "?" => 0xe6,
        "!" => 0xe7,
        "." => 0xe8,
        "&" => 0xe9,
        "é" => 0xea,
        "→" => 0xeb,
        "▷" => 0xec,
        "▶" => 0xed,
        "▼" => 0xee,
        "♂" => 0xef,
        "¥" => 0xf0,
        "×" => 0xf1,
        "<DOT>" => 0xf2,
        "/" => 0xf3,
        "," => 0xf4,
        "♀" => 0xf5,
        "#" => 0x54,
        "<" => 0x71,
        ">" => 0xed,
        _ => return None,
    })
}

fn draw_name_entry_tile(
    tile_id: u8,
    font: &image::RgbaImage,
    border: &image::RgbaImage,
    underline: &image::RgbaImage,
    middle_line: &image::RgbaImage,
    dest_x: usize,
    dest_y: usize,
    target: &mut [u8],
) -> Result<()> {
    if tile_id == 0 || tile_id == NAME_ENTRY_SPACE_TILE {
        return Ok(());
    }
    if tile_id == NAME_ENTRY_BORDER_TILE {
        blit_name_entry_tile_image(border, 0, 0, dest_x, dest_y, false, false, false, target);
        return Ok(());
    }
    if tile_id == NAME_ENTRY_UNDERLINE_TILE {
        blit_name_entry_tile_image(underline, 0, 0, dest_x, dest_y, false, false, false, target);
        return Ok(());
    }
    if tile_id == NAME_ENTRY_MIDDLE_LINE_TILE {
        blit_name_entry_tile_image(
            middle_line,
            0,
            0,
            dest_x,
            dest_y,
            false,
            false,
            false,
            target,
        );
        return Ok(());
    }
    if tile_id >= 0x80 {
        let font_index = usize::from(tile_id - 0x80);
        let tiles_per_row = font.width() as usize / SOURCE_TILE_SIZE;
        if tiles_per_row == 0 {
            anyhow::bail!("naming-screen font has invalid width {}", font.width());
        }
        let source_x = (font_index % tiles_per_row) * SOURCE_TILE_SIZE;
        let source_y = (font_index / tiles_per_row) * SOURCE_TILE_SIZE;
        blit_name_entry_tile_image(
            font, source_x, source_y, dest_x, dest_y, false, false, false, target,
        );
    }
    Ok(())
}

fn draw_name_entry_cursor(
    input: &PendingNameInput,
    cursor: &image::RgbaImage,
    target: &mut [u8],
) -> Result<()> {
    if cursor.width() < SOURCE_TILE_SIZE as u32 || cursor.height() < (SOURCE_TILE_SIZE * 2) as u32 {
        anyhow::bail!(
            "naming-screen cursor PNG has invalid dimensions {}x{}",
            cursor.width(),
            cursor.height()
        );
    }
    let offsets = if input.cursor_row == visible_name_input_bottom_row_index() {
        NAME_ENTRY_CASE_ROW_X_OFFSETS
    } else {
        NAME_ENTRY_LETTER_X_OFFSETS
    };
    let x_offset = offsets
        .get(input.cursor_column)
        .copied()
        .unwrap_or_else(|| *offsets.last().expect("cursor offsets"));
    let anchor_x = NAME_ENTRY_CURSOR_BASE_X + x_offset;
    let anchor_y = NAME_ENTRY_CURSOR_BASE_Y
        + i16::try_from(input.cursor_row).unwrap_or(0) * NAME_ENTRY_ROW_PIXEL_STEP;
    let tile_x =
        ((anchor_x - SOURCE_TILE_SIZE as i16) / SOURCE_TILE_SIZE as i16) * SOURCE_TILE_SIZE as i16;
    let tile_y = ((anchor_y - 16) / SOURCE_TILE_SIZE as i16) * SOURCE_TILE_SIZE as i16;
    let pieces: &[(i16, i16, i16, i16, usize, bool, bool)] =
        if input.cursor_row == visible_name_input_bottom_row_index() {
            &NAME_ENTRY_BIG_CURSOR_OAM
        } else {
            &NAME_ENTRY_SMALL_CURSOR_OAM
        };
    for (x_tile, y_tile, x_px, y_px, tile_index, xflip, yflip) in pieces {
        let dest_x = tile_x + x_tile * SOURCE_TILE_SIZE as i16 + x_px;
        let dest_y = tile_y + y_tile * SOURCE_TILE_SIZE as i16 + y_px;
        blit_name_entry_tile_image(
            cursor,
            0,
            tile_index * SOURCE_TILE_SIZE,
            dest_x.max(0) as usize,
            dest_y.max(0) as usize,
            *xflip,
            *yflip,
            true,
            target,
        );
    }
    Ok(())
}

fn blit_name_entry_tile_image(
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
    const TARGET_WIDTH: usize = NAME_ENTRY_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    const TARGET_HEIGHT: usize = NAME_ENTRY_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
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

fn spawn_field_command_bitmap_text(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
    x: f32,
    y: f32,
    z: f32,
) {
    for (index, frame) in bitmap_text_frames(rendered_art, asset_root, images, text)
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x + index as f32 * BITMAP_FONT_ADVANCE, y, z),
                ..default()
            },
            FieldCommandMarker,
        ));
    }
}

fn spawn_battle_command_bitmap_text(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
    x: f32,
    y: f32,
    z: f32,
) {
    for (index, frame) in bitmap_text_frames(rendered_art, asset_root, images, text)
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x + index as f32 * BITMAP_FONT_ADVANCE, y, z),
                ..default()
            },
            BattleCommandMarker,
        ));
    }
}

fn bitmap_text_frames(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
) -> Vec<SpriteFrame> {
    if rendered_art.font_cache.is_none() && rendered_art.font_error.is_none() {
        match load_bitmap_font_art(asset_root, images) {
            Ok(font) => rendered_art.font_cache = Some(font),
            Err(error) => rendered_art.font_error = Some(error.to_string()),
        }
    }
    let Some(font) = rendered_art.font_cache.as_ref() else {
        return Vec::new();
    };
    // Keep the visible renderer on the same character stream as the
    // TypeScript BitmapFont.  In particular, exported ASM text still carries
    // control tokens such as <PKMN>; letting those fall through to '?' hides
    // content errors and draws the wrong number of tiles.
    normalize_bitmap_font_text(text)
        .chars()
        .map(|ch| font.glyphs.get(&ch).or_else(|| font.glyphs.get(&'?')))
        .map(|frame| {
            frame.cloned().unwrap_or_else(|| SpriteFrame {
                handle: Handle::default(),
                size: Vec2::splat(0.0),
            })
        })
        .collect()
}

fn require_bitmap_font_art(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    if rendered_art.font_cache.is_none() && rendered_art.font_error.is_none() {
        match load_bitmap_font_art(asset_root, images) {
            Ok(font) => rendered_art.font_cache = Some(font),
            Err(error) => rendered_art.font_error = Some(error.to_string()),
        }
    }
    let font = rendered_art.font_cache.as_ref().with_context(|| {
        rendered_art
            .font_error
            .clone()
            .unwrap_or_else(|| "bitmap font art is unavailable".to_string())
    })?;
    for required in [' ', '?'] {
        if !font.glyphs.contains_key(&required) {
            anyhow::bail!("bitmap font is missing required glyph {required:?}");
        }
    }
    Ok(())
}

fn spawn_active_pokemon_picture(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(species_id) = snapshot.ui.active_pokemon_picture.as_deref() else {
        return Ok(());
    };
    const LEFT: f32 = 6.0;
    const TOP: f32 = 4.0;
    const WIDTH: f32 = 9.0;
    const HEIGHT: f32 = 10.0;
    let (window_x, window_y) = field_window_center(LEFT, TOP, WIDTH, HEIGHT);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(TILE_SIZE * 7.0, TILE_SIZE * 8.0)),
                ..default()
            },
            transform: Transform::from_xyz(window_x, window_y, 3.9),
            ..default()
        },
        PokemonPictureMarker,
    ));
    let window = battle_window_frame_art(rendered_art, asset_root, images)
        .context("required Pokepic window frame is unavailable")?;
    spawn_pokepic_window_frame_tiles(
        commands, window, LEFT, TOP, WIDTH as usize, HEIGHT as usize, 4.0,
    );
    let frame = pokepic_frame_for_art(rendered_art, asset_root, species_id, images)?.clone();
    let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    let picture_x = PLAYFIELD_LEFT + (LEFT + 1.0 + 3.5) * TILE_SIZE;
    let picture_y = PLAYFIELD_TOP - (TOP + 1.0 + 3.5) * TILE_SIZE;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle,
            sprite: Sprite {
                custom_size: Some(frame.size * source_scale),
                ..default()
            },
            transform: Transform::from_xyz(picture_x, picture_y, 4.1),
            ..default()
        },
        PokemonPictureMarker,
    ));
    Ok(())
}

fn spawn_pokepic_window_frame_tiles(
    commands: &mut Commands,
    frame: &WindowFrameArt,
    tile_x: f32,
    tile_y: f32,
    width: usize,
    height: usize,
    z: f32,
) {
    let mut spawn = |art: &SpriteFrame, x: f32, y: f32| {
        let (x, y) = battle_hud_tile_origin(x, y);
        commands.spawn((
            SpriteBundle {
                texture: art.handle.clone(),
                sprite: Sprite { custom_size: Some(art.size), ..default() },
                transform: Transform::from_xyz(x, y, z),
                ..default()
            },
            PokemonPictureMarker,
        ));
    };
    let right = tile_x + width.saturating_sub(1) as f32;
    let bottom = tile_y + height.saturating_sub(1) as f32;
    spawn(&frame.top_left, tile_x, tile_y);
    spawn(&frame.top_right, right, tile_y);
    spawn(&frame.bottom_left, tile_x, bottom);
    spawn(&frame.bottom_right, right, bottom);
    for col in 1..width.saturating_sub(1) {
        spawn(&frame.top_edge, tile_x + col as f32, tile_y);
        spawn(&frame.top_edge, tile_x + col as f32, bottom);
    }
    for row in 1..height.saturating_sub(1) {
        spawn(&frame.side_edge, tile_x, tile_y + row as f32);
        spawn(&frame.side_edge, right, tile_y + row as f32);
    }
}

fn pokepic_frame_for_art<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    species_id: &str,
    images: &mut Assets<Image>,
) -> Result<&'a SpriteFrame> {
    let key = normalize_pokemon_asset_id(species_id);
    if !rendered_art.pokepic_cache.contains_key(&key)
        && !rendered_art.pokepic_errors.contains_key(&key)
    {
        let loaded = (|| -> Result<SpriteFrame> {
            let root = asset_root.runtime_assets().join("gfx/pokemon").join(&key);
            let dimensions = std::fs::read(root.join("front.dimensions"))
                .with_context(|| format!("read Pokepic dimensions for {species_id}"))?;
            let dimension = dimensions.first().map(|value| usize::from(value & 0x0f))
                .context("Pokepic dimensions file is empty")?;
            if !(5..=7).contains(&dimension) {
                anyhow::bail!("Pokepic {species_id} has invalid {dimension}x{dimension} dimensions");
            }
            let data = std::fs::read(root.join("front.2bpp"))
                .with_context(|| format!("read Pokepic graphics for {species_id}"))?;
            let byte_count = dimension * dimension * 16;
            let frame = data.get(..byte_count)
                .with_context(|| format!("Pokepic {species_id} is shorter than one frame"))?;
            let mut rgba = vec![0_u8; 56 * 56 * 4];
            let left = if dimension == 7 { 0 } else { 8 };
            let top = if dimension == 5 { 8 } else { 0 };
            for tile in 0..dimension * dimension {
                let tile_x = tile % dimension;
                let tile_y = tile / dimension;
                for y in 0..8 {
                    let lo = frame[tile * 16 + y * 2];
                    let hi = frame[tile * 16 + y * 2 + 1];
                    for x in 0..8 {
                        let bit = 1 << (7 - x);
                        let level = ((hi & bit != 0) as u8) << 1 | (lo & bit != 0) as u8;
                        if level == 0 { continue; }
                        let shade = [255_u8, 170, 85, 0][usize::from(level)];
                        let dx = left + tile_x * 8 + x;
                        let dy = top + tile_y * 8 + y;
                        let offset = (dy * 56 + dx) * 4;
                        rgba[offset..offset + 3].fill(shade);
                        rgba[offset + 3] = 255;
                    }
                }
            }
            let mut image = Image::new(
                Extent3d { width: 56, height: 56, depth_or_array_layers: 1 },
                TextureDimension::D2,
                rgba,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::nearest();
            Ok(SpriteFrame { handle: images.add(image), size: Vec2::splat(56.0) })
        })();
        match loaded {
            Ok(frame) => { rendered_art.pokepic_cache.insert(key.clone(), frame); }
            Err(error) => { rendered_art.pokepic_errors.insert(key.clone(), error.to_string()); }
        }
    }
    rendered_art.pokepic_cache.get(&key).with_context(|| {
        rendered_art.pokepic_errors.get(&key).cloned()
            .unwrap_or_else(|| format!("Pokepic {species_id} is unavailable"))
    })
}

fn visible_field_dialog_pages(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Option<Vec<String>> {
    if let Some(notice) = runtime_shell.field_notice.as_ref() {
        return Some(vec![notice.clone()]);
    }
    if let Some(notice) = runtime_shell.pc_notice.as_ref() {
        return Some(vec![notice.clone()]);
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            return Some(vec!["Welcome! How may I\nhelp you?".to_string()]);
        }
        if let Some(notice) = runtime_shell.shop_notice.as_ref() {
            return Some(vec![notice.clone()]);
        }
    }
    let text = snapshot.ui.text.as_ref()?;
    let pages = if let Some(asm_text) = &text.asm_text {
        normalize_visible_script_text_with_context(
            asm_text,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )
            .split("\n\n")
            .map(str::to_owned)
            .collect()
    } else if let Some(body) = &text.body {
        render_visible_script_text_pages(
            body,
            &snapshot.script_events.named_buffers,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )
    } else {
        Vec::new()
    };
    Some(if pages.is_empty() {
        vec![String::new()]
    } else {
        pages
    })
}

fn visible_field_dialog_text(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Option<String> {
    let pages = visible_field_dialog_pages(snapshot, runtime_shell)?;
    let page_index = runtime_shell
        .field_text_reveal
        .as_ref()
        .map(|reveal| reveal.page_index)
        .unwrap_or(0)
        .min(pages.len().saturating_sub(1));
    pages.get(page_index).cloned()
}

fn visible_revealed_field_dialog_text(runtime_shell: &BevyRuntimeShell, full_text: &str) -> String {
    let Some(reveal) = runtime_shell.field_text_reveal.as_ref() else {
        return full_text.to_string();
    };
    full_text.chars().take(reveal.visible_chars).collect()
}

fn visible_revealed_shell_notice_text(
    runtime_shell: &BevyRuntimeShell,
    full_text: &str,
) -> String {
    runtime_shell
        .field_text_reveal
        .as_ref()
        .filter(|reveal| reveal.text == full_text)
        .map(|reveal| full_text.chars().take(reveal.visible_chars).collect())
        .unwrap_or_default()
}

fn visible_field_dialogue_is_fully_revealed(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> bool {
    let Some(full_text) = visible_field_dialog_text(snapshot, runtime_shell) else {
        return true;
    };
    let Some(reveal) = runtime_shell.field_text_reveal.as_ref() else {
        return true;
    };
    visible_field_text_reveal_is_complete(reveal, &full_text)
}

fn visible_field_text_reveal_is_complete(
    reveal: &VisibleFieldTextReveal,
    current_page: &str,
) -> bool {
    reveal.visible_chars >= current_page.chars().count()
}

fn visible_field_text_reveal_is_complete_for_text(
    runtime_shell: &BevyRuntimeShell,
    text: &str,
) -> bool {
    runtime_shell
        .field_text_reveal
        .as_ref()
        .is_some_and(|reveal| reveal.text == text && reveal.visible_chars >= text.chars().count())
}

/// Advance only the visual text printer.  The script remains at its existing
/// text/wait command until the player confirms the fully printed page.
fn tick_visible_field_text_reveal(
    runtime_shell: &mut BevyRuntimeShell,
    acceleration_requested: bool,
) -> Result<bool> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(pages) = visible_field_dialog_pages(&snapshot, runtime_shell) else {
        let changed = runtime_shell.field_text_reveal.take().is_some();
        return Ok(changed);
    };
    let text_identity = pages.join("\u{1e}");
    let reveal = runtime_shell
        .field_text_reveal
        .get_or_insert_with(|| VisibleFieldTextReveal {
            text: text_identity.clone(),
            page_index: 0,
            visible_chars: 0,
            frames_until_next_char: 0,
        });
    if reveal.text != text_identity {
        *reveal = VisibleFieldTextReveal {
            text: text_identity,
            page_index: 0,
            visible_chars: 0,
            frames_until_next_char: 0,
        };
    }
    reveal.page_index = reveal.page_index.min(pages.len().saturating_sub(1));
    let full_text = &pages[reveal.page_index];
    let text_len = full_text.chars().count();
    if snapshot.trainer.options.no_text_scroll {
        let changed = reveal.visible_chars < text_len;
        reveal.visible_chars = text_len;
        reveal.frames_until_next_char = 0;
        return Ok(changed);
    }
    let frames_per_char = if acceleration_requested {
        1
    } else {
        visible_text_frames_per_char(snapshot.trainer.options.text_speed)
    };
    if reveal.visible_chars >= text_len {
        return Ok(false);
    }
    if acceleration_requested {
        reveal.frames_until_next_char = 0;
    }
    if reveal.frames_until_next_char > 0 {
        reveal.frames_until_next_char -= 1;
        return Ok(false);
    }
    reveal.visible_chars = reveal.visible_chars.saturating_add(1).min(text_len);
    reveal.frames_until_next_char = frames_per_char.saturating_sub(1);
    Ok(true)
}

fn advance_visible_completed_field_text_page(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<bool> {
    let Some(pages) = visible_field_dialog_pages(snapshot, runtime_shell) else {
        return Ok(false);
    };
    let Some(reveal) = runtime_shell.field_text_reveal.as_mut() else {
        return Ok(false);
    };
    let text_identity = pages.join("\u{1e}");
    if reveal.text != text_identity {
        return Ok(false);
    }
    reveal.page_index = reveal.page_index.min(pages.len().saturating_sub(1));
    let text_len = pages[reveal.page_index].chars().count();
    if reveal.visible_chars < text_len {
        return Ok(false);
    }
    if reveal.page_index + 1 >= pages.len() {
        return Ok(false);
    }
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    let reveal = runtime_shell
        .field_text_reveal
        .as_mut()
        .context("field text reveal disappeared while advancing its page")?;
    reveal.page_index += 1;
    reveal.visible_chars = 0;
    reveal.frames_until_next_char = 0;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn visible_scene_dialog_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    if let Some(bank) = runtime_shell.visible_mom_bank.as_ref() {
        if let Some(message) = bank.messages.front() {
            push_wrapped_scene_dialog_lines(&mut entries, message);
        } else {
            match bank.phase {
                VisibleMomBankPhase::InitializeQuestion => {
                    entries.push("Do you want me to".to_string());
                    entries.push("save your money?".to_string());
                    entries.push(if bank.yes_no_index == 0 {
                        "> YES    NO".to_string()
                    } else {
                        "  YES  > NO".to_string()
                    });
                }
                VisibleMomBankPhase::AccessQuestion => {
                    entries.push("Or is this about".to_string());
                    entries.push("your money?".to_string());
                    entries.push(if bank.yes_no_index == 0 {
                        "> YES    NO".to_string()
                    } else {
                        "  YES  > NO".to_string()
                    });
                }
                VisibleMomBankPhase::ChangeQuestion => {
                    entries.push("Do you want to".to_string());
                    entries.push("save some money?".to_string());
                    entries.push(if bank.yes_no_index == 0 {
                        "> YES    NO".to_string()
                    } else {
                        "  YES  > NO".to_string()
                    });
                }
                VisibleMomBankPhase::Menu => entries.extend([
                    "What do you want to do?".to_string(),
                    "GET / SAVE / CHANGE / CANCEL".to_string(),
                ]),
                VisibleMomBankPhase::Withdraw | VisibleMomBankPhase::Deposit => entries.extend([
                    format!("SAVED ¥{}", snapshot.trainer.moms_money),
                    format!("HELD ¥{}", snapshot.trainer.money),
                    format!("AMOUNT ¥{:06}", bank.amount),
                ]),
            }
        }
        return Ok(entries);
    }
    if let Some(toss) = runtime_shell.pack_toss.as_ref() {
        let display_name = item_display_name(snapshot, &toss.item_id);
        if toss.confirming {
            entries.push(compact_scene_label(
                &format!("Throw away {}", toss.quantity),
                SCENE_DIALOG_TEXT_CHARS,
            ));
            entries.push(compact_scene_label(
                &format!("{display_name}(S)?"),
                SCENE_DIALOG_TEXT_CHARS,
            ));
        } else {
            entries.push("Throw away how".to_string());
            entries.push("many?".to_string());
            entries.push(compact_scene_label(
                &format!("{display_name} ×{:02}", toss.quantity),
                SCENE_DIALOG_TEXT_CHARS,
            ));
        }
        return Ok(entries);
    }
    if runtime_shell.held_item_swap_prompt {
        let party_index = runtime_shell
            .party_held_item_give_target
            .context("held-item swap prompt is missing its party target")?;
        let pokemon = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .with_context(|| format!("held-item swap target {party_index} is not in the party"))?;
        let old_item = pokemon
            .pokemon
            .item
            .as_deref()
            .context("held-item swap prompt target is no longer holding an item")?;
        entries.push(compact_scene_label(
            &format!("{} is already holding", pokemon.pokemon.nickname),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        entries.push(compact_scene_label(
            &format!("{}.", item_display_name(snapshot, old_item)),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        entries.push("Switch items?".to_string());
        return Ok(entries);
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        entries.push("BILL'S PC".to_string());
        entries.extend(visible_bill_pc_action_entries(runtime_shell)?);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        entries.push("CHANGE BOX".to_string());
        entries.extend(visible_bill_pc_box_entries(snapshot, runtime_shell)?);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.pc_hub_cursor.is_some() {
        push_visible_pc_hub_dialog_entries(&mut entries, snapshot, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        entries.push("PLAYER'S PC".to_string());
        let actions = visible_player_pc_actions(runtime_shell);
        let selected = strict_readonly_cursor_index(
            &runtime_shell.player_pc_action_cursor,
            "pc:player-actions",
            actions.len(),
        )
        .with_context(|| {
            format!("player PC action cursor is invalid for {} actions", actions.len())
        })?;
        entries.extend(actions.iter().enumerate().map(|(index, action)| {
            format!("{}{}", if index == selected { ">" } else { " " }, visible_player_pc_action_label(*action))
        }));
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.mailbox_cursor.is_some() {
        push_visible_mailbox_dialog_entries(&mut entries, snapshot, runtime_shell);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.storage_cursor.is_some() {
        push_visible_storage_dialog_entries(&mut entries, snapshot, runtime_shell);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.pc_item_cursor.is_some() {
        push_visible_pc_item_dialog_entries(&mut entries, snapshot, runtime_shell);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if let Some(shop) = &snapshot.pending_shop {
        push_visible_shop_dialog_entries(&mut entries, snapshot, runtime_shell, shop);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if let Some(menu) = &snapshot.ui.menu {
        push_visible_runtime_menu_dialog_entries(&mut entries, runtime_shell, menu);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if let Some(picture) = &snapshot.ui.active_pokemon_picture {
        push_visible_pokemon_picture_entries(&mut entries, snapshot, picture);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if snapshot.pending_move_learn.is_some() {
        push_visible_pending_move_learn_entries(&mut entries, snapshot, runtime_shell);
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if has_visible_gift_pokemon_prompt(snapshot, runtime_shell) {
        push_visible_gift_pokemon_entries(&mut entries, snapshot, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if has_visible_elevator_prompt(snapshot, runtime_shell) {
        push_visible_elevator_entries(&mut entries, snapshot, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.pending_day_of_week.is_some() {
        push_visible_day_of_week_entries(&mut entries, runtime_shell);
        return Ok(entries);
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        push_visible_phone_prompt_entries(&mut entries, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.pending_name_choice.is_some() {
        push_visible_name_choice_entries(&mut entries, runtime_shell);
        return Ok(entries);
    }
    if runtime_shell.pending_name_input.is_some() {
        push_visible_name_input_entries(&mut entries, runtime_shell);
        return Ok(entries);
    }
    if runtime_shell.save_flow.is_some() {
        push_visible_save_dialog_entries(&mut entries, snapshot, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if let Some(full_text) = visible_field_dialog_text(snapshot, runtime_shell) {
        push_wrapped_scene_dialog_lines(
            &mut entries,
            &visible_revealed_field_dialog_text(runtime_shell, &full_text),
        );
    }
    if snapshot.ui.pending_yes_no.is_some() {
        let selected = strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
            .context("yes/no prompt is active without a valid cursor")?;
        push_priority_scene_dialog_entry(
            &mut entries,
            if selected == 0 {
                "> YES    NO".to_string()
            } else {
                "  YES  > NO".to_string()
            },
        );
    }

    if visible_field_dialog_text(snapshot, runtime_shell).is_some() {
        // The field box has four interior rows. A `promptbutton` indicator is
        // an arrow in its bottom-right tile, never a fifth textual row (and
        // never the debug word "NEXT").
        entries.truncate(FIELD_TEXT_BOX_VISIBLE_ROWS);
    }

    entries.truncate(SCENE_MENU_VISIBLE_ROWS);
    Ok(entries)
}

fn push_visible_pc_hub_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<()> {
    let actions = visible_pc_hub_actions(snapshot);
    let selected = strict_readonly_cursor_index(&runtime_shell.pc_hub_cursor, "pc:hub", actions.len())
        .context("Pokemon Center PC hub is open without a valid cursor")?;
    entries.push("ACCESS WHOSE PC?".to_string());
    entries.extend(actions.into_iter().enumerate().map(|(index, action)| {
        compact_scene_label(
            &format!(
                "{}{}",
                if index == selected { ">" } else { " " },
                visible_pc_hub_action_label(snapshot, action)
            ),
            SCENE_DIALOG_TEXT_CHARS,
        )
    }));
    Ok(())
}

fn push_visible_mailbox_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) {
    entries.push("MAIL BOX".to_string());
    let selected = strict_readonly_cursor_index(
        &runtime_shell.mailbox_cursor,
        "pc:mailbox",
        snapshot.mailbox.len(),
    )
    .unwrap_or(0);
    for (index, entry) in snapshot.mailbox.iter().enumerate() {
        entries.push(compact_scene_label(
            &format!("{}{}", if index == selected { ">" } else { " " }, entry.mail.author),
            SCENE_DIALOG_TEXT_CHARS,
        ));
    }
    if let Some(cursor) = &runtime_shell.mailbox_action_cursor {
        let action = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pc:mailbox-actions",
            VISIBLE_MAILBOX_ACTIONS.len(),
        )
        .unwrap_or(0);
        entries.clear();
        entries.push(compact_scene_label(&snapshot.mailbox[selected].mail.author, SCENE_DIALOG_TEXT_CHARS));
        entries.extend(VISIBLE_MAILBOX_ACTIONS.iter().enumerate().map(|(index, label)| {
            format!("{}{}", if index == action { ">" } else { " " }, label)
        }));
    }
}

fn scene_dialog_surface_active(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> bool {
    runtime_shell.visible_mom_bank.is_some()
        || snapshot.pending_shop.is_some()
        || snapshot.ui.menu.is_some()
        || snapshot.ui.text.is_some()
        || snapshot.ui.active_pokemon_picture.is_some()
        || has_visible_gift_pokemon_prompt(snapshot, runtime_shell)
        || has_visible_elevator_prompt(snapshot, runtime_shell)
        || runtime_shell.pending_day_of_week.is_some()
        || runtime_shell.pending_phone_prompt.is_some()
        || runtime_shell.pending_name_input.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.save_flow.is_some()
        || snapshot.ui.pending_yes_no.is_some()
        || snapshot.ui.pending_text_wait.is_some()
        || runtime_shell.pc_hub_cursor.is_some()
        || runtime_shell.player_pc_action_cursor.is_some()
        || runtime_shell.mailbox_cursor.is_some()
        || runtime_shell.pc_confirmation.is_some()
        || runtime_shell.storage_cursor.is_some()
        || runtime_shell.pc_item_cursor.is_some()
        || runtime_shell.active_script_cursor.is_some()
        || runtime_shell.tmhm_teach_prompt_cursor.is_some()
        || runtime_shell.tmhm_decision_prompt_cursor.is_some()
}

fn push_visible_save_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<()> {
    let Some(flow) = runtime_shell.save_flow.as_ref() else {
        return Ok(());
    };
    let label = match flow.stage {
        VisibleSaveFlowStage::Prompt => SAVE_TEXT_WOULD_YOU_LIKE,
        VisibleSaveFlowStage::OverwritePrompt => SAVE_TEXT_ALREADY_EXISTS,
        VisibleSaveFlowStage::Saving => SAVE_TEXT_SAVING,
        VisibleSaveFlowStage::Saved => SAVE_TEXT_SAVED,
        VisibleSaveFlowStage::Error => SAVE_TEXT_CORRUPTED,
    };
    let text = visible_asm_text(snapshot, label)?;
    push_wrapped_scene_dialog_lines(entries, &text);
    if matches!(
        flow.stage,
        VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt
    ) {
        push_priority_scene_dialog_entry(
            entries,
            if flow.yes_no_index == 0 {
                "> YES    NO".to_string()
            } else {
                "  YES  > NO".to_string()
            },
        );
    }
    Ok(())
}

fn visible_asm_text(snapshot: &RuntimeShellSnapshot, label: &str) -> Result<String> {
    let text = snapshot
        .presentation
        .asm_text
        .get(label)
        .with_context(|| {
            format!("ASM text label {label} is missing from the presentation catalog")
        })?
        .replace("<PLAYER>", &snapshot.trainer.player_name)
        .replace("<PLAY_G>", &snapshot.trainer.player_name);
    Ok(text)
}

fn push_visible_pokemon_picture_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    species_id: &str,
) {
    let Some(species) = snapshot
        .pokemon
        .iter()
        .find(|species| species.species_id == species_id)
    else {
        entries.push(compact_scene_label(
            &format!("INVALID SPECIES {species_id}"),
            30,
        ));
        return;
    };
    entries.push(compact_scene_label(
        &crate::core::models::pokemon_species_display_name(species_id),
        30,
    ));
    entries.push(compact_scene_label(
        &format!("#{:03} {} {}", species.int_id, species.type1, species.type2),
        30,
    ));
    if runtime_debug_overlays_enabled() {
        entries.push(compact_scene_label(
            &format!(
                "catch={} exp={} growth={}",
                species.catch_rate, species.base_exp, species.growth_rate
            ),
            30,
        ));
    }
}

fn push_visible_gift_pokemon_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<()> {
    let gifts = visible_gift_pokemon_prompt_options(snapshot, runtime_shell);
    let gift_count = gifts.len();
    if gift_count == 0 {
        entries.push("GIFT POKEMON EMPTY".to_string());
        return Ok(());
    }
    let cursor = runtime_shell
        .gift_pokemon_cursor
        .as_ref()
        .context("gift Pokemon prompt is active without a cursor")?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.gift_pokemon_cursor,
        &cursor.surface_id,
        gift_count,
    )
    .context("gift Pokemon prompt is active without a valid cursor")?;
    entries.push(compact_scene_label(
        &format!("GIFT POKEMON {}/{}", selected + 1, gift_count),
        30,
    ));
    entries.extend(windowed_index_range(selected, gift_count).map(|index| {
        let gift = gifts[index];
        let marker = if index == selected { ">" } else { " " };
        let held = gift
            .held_item_id
            .as_deref()
            .map(|item| format!(" {}", item_display_name(snapshot, item)))
            .unwrap_or_default();
        let species_name = if gift.egg {
            "EGG".to_string()
        } else {
            crate::core::models::pokemon_species_display_name(&gift.species_id)
        };
        compact_scene_label(
            &format!("{marker}{species_name} L{}{held}", gift.level),
            30,
        )
    }));
    Ok(())
}

fn push_visible_phone_prompt_entries(
    entries: &mut Vec<String>,
    runtime_shell: &BevyRuntimeShell,
) -> Result<()> {
    let Some(prompt) = runtime_shell.pending_phone_prompt.as_ref() else {
        return Ok(());
    };
    let selected = strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:phone-number", 2)
        .context("phone prompt is active without a valid cursor")?;
    entries.push(compact_scene_label(
        &format!("PHONE {}", prompt.contact_id),
        30,
    ));
    entries.push(if selected == 0 {
        "> YES    NO".to_string()
    } else {
        "  YES  > NO".to_string()
    });
    Ok(())
}

fn push_visible_day_of_week_entries(
    entries: &mut Vec<String>,
    runtime_shell: &BevyRuntimeShell,
) {
    const DAYS: [&str; 7] = [
        "SUNDAY",
        "MONDAY",
        "TUESDAY",
        "WEDNESDAY",
        "THURSDAY",
        "FRIDAY",
        "SATURDAY",
    ];
    let Some(prompt) = runtime_shell.pending_day_of_week.as_ref() else {
        return;
    };
    let day = DAYS[usize::from(prompt.selected_day % 7)];
    if prompt.confirming {
        entries.push(format!("{day}, is it?"));
        entries.push(if prompt.yes_no_index == 0 {
            "> YES    NO".to_string()
        } else {
            "  YES  > NO".to_string()
        });
    } else {
        entries.push("What day is it?".to_string());
        entries.push("      ▲".to_string());
        entries.push(format!("    {day}"));
        entries.push("      ▼".to_string());
    }
}

fn push_visible_name_input_entries(entries: &mut Vec<String>, runtime_shell: &BevyRuntimeShell) {
    let Some(input) = runtime_shell.pending_name_input.as_ref() else {
        return;
    };
    entries.push(compact_scene_label("NAME ENTRY", 30));
    entries.push(compact_scene_label(&input.label, 30));
    let cursor = if input.value.chars().count() < input.max_length {
        "_"
    } else {
        ""
    };
    entries.push(compact_scene_label(
        &format!("NAME {}{cursor}", input.value),
        30,
    ));
    let layout = visible_name_input_layout(input.case);
    for (row, line) in layout.iter().enumerate() {
        entries.push(compact_scene_label(line, 30));
        if row == input.cursor_row {
            let pointer_index = (input.cursor_column * 2).min(16);
            entries.push(compact_scene_label(
                &format!("{}^", " ".repeat(pointer_index)),
                30,
            ));
        }
    }
    entries.push(compact_scene_label(
        &format!(
            "CASE {:?} LEN {}/{}",
            input.case,
            input.value.chars().count(),
            input.max_length
        ),
        30,
    ));
}

fn push_visible_name_choice_entries(entries: &mut Vec<String>, runtime_shell: &BevyRuntimeShell) {
    let Some(choice) = runtime_shell.pending_name_choice.as_ref() else {
        return;
    };
    if let Some(capture) = runtime_shell.pending_standard_capture.as_ref() {
        entries.push(compact_scene_label(
            &format!("Give a nickname to {}?", capture.default_name),
            30,
        ));
    } else {
        entries.push("NAME".to_string());
    }
    entries.extend(choice.options.iter().enumerate().map(|(index, option)| {
        if index == choice.selected {
            format!("> {option}")
        } else {
            format!("  {option}")
        }
    }));
}

fn push_visible_elevator_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<()> {
    let elevators = visible_elevator_prompt_options(snapshot, runtime_shell);
    let option_count = visible_elevator_option_count(snapshot, runtime_shell);
    if option_count == 0 {
        entries.push("ELEVATOR EMPTY".to_string());
        for elevator in &elevators {
            entries.push(compact_scene_label(
                &format!("{} has no floors", elevator.data_label),
                30,
            ));
        }
        return Ok(());
    }
    let cursor = runtime_shell
        .elevator_cursor
        .as_ref()
        .context("elevator prompt is active without a cursor")?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.elevator_cursor,
        &cursor.surface_id,
        option_count,
    )
    .context("elevator prompt is active without a valid cursor")?;
    entries.push(compact_scene_label(
        &format!("ELEVATOR {}/{}", selected + 1, option_count),
        30,
    ));
    let mut flat_index = 0usize;
    for elevator in elevators {
        for floor in &elevator.floors {
            if windowed_index_range(selected, option_count).contains(&flat_index) {
                let marker = if flat_index == selected { ">" } else { " " };
                entries.push(compact_scene_label(
                    &format!(
                        "{marker}{} -> {} warp{}",
                        floor.floor, floor.target_map, floor.warp
                    ),
                    30,
                ));
            }
            flat_index += 1;
        }
    }
    Ok(())
}

fn push_visible_pending_move_learn_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) {
    let Some(pending) = snapshot.pending_move_learn.as_ref() else {
        return;
    };
    let move_name = battle_move_display_name(snapshot, &pending.learned_move.name);
    entries.push(compact_scene_label(
        &format!(
            "{} WANTS {}",
            crate::core::models::pokemon_species_display_name(&pending.species_id),
            move_name
        ),
        SCENE_DIALOG_TEXT_CHARS,
    ));
    if let Some(cursor) = &runtime_shell.move_learn_decision_cursor {
        let selected = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "move-learn:decision",
            2,
        );
        let Some(selected) = selected else {
            entries.push(compact_scene_label(
                "INVALID CURSOR move-learn:decision",
                SCENE_DIALOG_TEXT_CHARS,
            ));
            return;
        };
        let prompt = match runtime_shell.move_learn_decision {
            Some(VisibleTmHmDecision::ForgetMove) => "DELETE A MOVE TO MAKE ROOM?",
            Some(VisibleTmHmDecision::StopLearning) => "STOP LEARNING THIS MOVE?",
            None => "INVALID MOVE-LEARN DECISION",
        };
        entries.push(compact_scene_label(prompt, SCENE_DIALOG_TEXT_CHARS));
        entries.push(format!("{}YES", if selected == 0 { ">" } else { " " }));
        entries.push(format!("{}NO", if selected == 1 { ">" } else { " " }));
        return;
    }
    if !runtime_shell.move_learn_forget_menu_open {
        entries.push(compact_scene_label(
            "IS TRYING TO LEARN IT!",
            SCENE_DIALOG_TEXT_CHARS,
        ));
        entries.push(compact_scene_label(
            "A/B CONTINUE",
            SCENE_DIALOG_TEXT_CHARS,
        ));
        return;
    }
    entries.push(compact_scene_label(
        "WHICH MOVE SHOULD BE FORGOTTEN?",
        SCENE_DIALOG_TEXT_CHARS,
    ));
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(pending.party_index),
        5,
    );
    let Some(selected) = selected else {
        entries.push(compact_scene_label(
            "INVALID CURSOR move-learn",
            SCENE_DIALOG_TEXT_CHARS,
        ));
        return;
    };
    let Some(slot) = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
    else {
        entries.push(compact_scene_label(
            &format!("INVALID PARTY {}", pending.party_index),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        return;
    };
    for (index, learned) in slot.pokemon.moves.iter().enumerate() {
        let marker = if index == selected { ">" } else { " " };
        entries.push(move_menu_entry(snapshot, learned, marker));
    }
    entries.push(format!(
        "{}CANCEL",
        if selected == slot.pokemon.moves.len() {
            ">"
        } else {
            " "
        }
    ));
}

fn push_visible_storage_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) {
    let Some(box_snapshot) = snapshot
        .storage
        .boxes
        .iter()
        .find(|pc_box| pc_box.index == snapshot.storage.current_pc_box)
    else {
        entries.push(compact_scene_label(
            &format!("INVALID PC BOX {}", snapshot.storage.current_pc_box),
            30,
        ));
        return;
    };
    let surface_id = storage_cursor_surface_id(box_snapshot.index);
    entries.push(compact_scene_label(
        &format!(
            "BOX {} {} {}/{}",
            box_snapshot.index, box_snapshot.name, box_snapshot.count, snapshot.storage.party_count
        ),
        SCENE_DIALOG_TEXT_CHARS,
    ));
    entries.push(compact_scene_label(
        if runtime_shell.party_menu_open {
            "A DEPOSIT  B CLOSE"
        } else {
            "A WITHDRAW SELECT RELEASE"
        },
        SCENE_DIALOG_TEXT_CHARS,
    ));
    if box_snapshot.slots.is_empty() && !runtime_shell.bill_pc_move_open {
        entries.push("EMPTY".to_string());
        return;
    }
    let option_count = if runtime_shell.bill_pc_move_open {
        crate::core::models::MAX_BOX_MONS
    } else {
        box_snapshot.slots.len()
    };
    let cursor_index = strict_readonly_cursor_index(
        &runtime_shell.storage_cursor,
        &surface_id,
        option_count,
    );
    let Some(cursor_index) = cursor_index else {
        entries.push(compact_scene_label(
            &format!("INVALID CURSOR {surface_id}"),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        return;
    };
    entries.extend(
        windowed_index_range(cursor_index, option_count).map(|offset| {
            let marker = if offset == cursor_index { ">" } else { " " };
            let Some(slot) = box_snapshot.slots.iter().find(|slot| slot.index == offset) else {
                return compact_scene_label(
                    &format!("{marker}{offset} EMPTY"),
                    SCENE_DIALOG_TEXT_CHARS,
                );
            };
            let held = slot
                .pokemon
                .item
                .as_deref()
                .map(|item| format!(" item={item}"))
                .unwrap_or_default();
            compact_scene_label(
                &format!(
                    "{marker}{} {} L{} HP {}/{}{}",
                    slot.index,
                    slot.pokemon.species.id,
                    slot.pokemon.level,
                    slot.pokemon.hp,
                    slot.pokemon.max_hp,
                    held
                ),
                SCENE_DIALOG_TEXT_CHARS,
            )
        }),
    );
}

fn push_visible_pc_item_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) {
    let carried_offsets = snapshot
        .bag
        .pc_items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.quantity > 0)
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>();
    entries.push(compact_scene_label(
        &format!("PC ITEMS {}", carried_offsets.len()),
        SCENE_DIALOG_TEXT_CHARS,
    ));
    entries.push(compact_scene_label(
        if visible_field_pack_is_open(runtime_shell) {
            "A DEPOSIT ITEM  B CLOSE"
        } else if runtime_shell.pc_item_action == Some(VisiblePlayerPcAction::TossItem) {
            "A TOSS ITEM  B CLOSE"
        } else {
            "A WITHDRAW ITEM  B CLOSE"
        },
        SCENE_DIALOG_TEXT_CHARS,
    ));
    if carried_offsets.is_empty() {
        entries.push("EMPTY".to_string());
        return;
    }
    let cursor_index = strict_readonly_cursor_index(
        &runtime_shell.pc_item_cursor,
        "pc:items",
        carried_offsets.len(),
    );
    let Some(cursor_index) = cursor_index else {
        entries.push(compact_scene_label(
            "INVALID CURSOR pc:items",
            SCENE_DIALOG_TEXT_CHARS,
        ));
        return;
    };
    entries.extend(
        windowed_index_range(cursor_index, carried_offsets.len()).map(|visible_index| {
            let item = &snapshot.bag.pc_items[carried_offsets[visible_index]];
            let marker = if visible_index == cursor_index {
                ">"
            } else {
                " "
            };
            compact_scene_label(
                &format!(
                    "{marker}{} x{}",
                    item_display_name(snapshot, &item.item_id),
                    item.quantity
                ),
                SCENE_DIALOG_TEXT_CHARS,
            )
        }),
    );
}

fn push_visible_runtime_menu_dialog_entries(
    entries: &mut Vec<String>,
    runtime_shell: &BevyRuntimeShell,
    menu: &crate::RuntimeMenuSnapshot,
) {
    for vertical in &menu.layout.vertical_menus {
        let surface_id = vertical_menu_surface_id(menu, vertical);
        let cursor_index = strict_readonly_cursor_index(
            &runtime_shell.menu_cursor,
            &surface_id,
            vertical.options.len(),
        );
        let Some(cursor_index) = cursor_index else {
            entries.push(compact_scene_label(
                &format!("INVALID CURSOR {surface_id}"),
                SCENE_DIALOG_TEXT_CHARS,
            ));
            return;
        };
        if vertical.two_dimensional {
            let Some(columns) = vertical.columns else {
                entries.push("INVALID 2D MENU".to_string());
                return;
            };
            for (row_index, row) in vertical.options.chunks(columns).enumerate() {
                let first_index = row_index * columns;
                let line = row
                    .iter()
                    .enumerate()
                    .map(|(column_index, option)| {
                        let marker = if first_index + column_index == cursor_index {
                            ">"
                        } else {
                            " "
                        };
                        format!("{marker}{option}")
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                entries.push(compact_scene_label(&line, SCENE_DIALOG_TEXT_CHARS));
            }
            return;
        }
        entries.push(compact_scene_label(&menu.menu_id, SCENE_DIALOG_TEXT_CHARS));
        entries.extend(
            windowed_index_range(cursor_index, vertical.options.len()).map(|index| {
                let option = &vertical.options[index];
                let marker = if index == cursor_index { ">" } else { " " };
                compact_scene_label(&format!("{marker}{option}"), SCENE_DIALOG_TEXT_CHARS)
            }),
        );
        if entries.len() >= SCENE_MENU_VISIBLE_ROWS {
            return;
        }
    }
}

fn push_visible_shop_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
) {
    if runtime_shell.sell_cursor.is_some() {
        let sellable = sellable_carried_item_ids(snapshot);
        entries.push(compact_scene_label(
            &format!("SELL {} ${}", shop.mart_id, snapshot.trainer.money),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        let Some(cursor_index) =
            strict_readonly_cursor_index(&runtime_shell.sell_cursor, "sell:bag", sellable.len())
        else {
            entries.push(compact_scene_label(
                "INVALID CURSOR sell:bag",
                SCENE_DIALOG_TEXT_CHARS,
            ));
            return;
        };
        entries.extend(
            windowed_index_range(cursor_index, sellable.len()).map(|index| {
                let item = &sellable[index];
                let marker = if index == cursor_index { ">" } else { " " };
                compact_scene_label(
                    &format!("{marker}{}", shop_sell_item_label(snapshot, item)),
                    SCENE_DIALOG_TEXT_CHARS,
                )
            }),
        );
        return;
    }
    let surface_id = shop_cursor_surface_id(shop);
    let cursor_index = strict_readonly_cursor_index(
        &runtime_shell.menu_cursor,
        &surface_id,
        shop.inventory.len(),
    );
    entries.push(compact_scene_label(
        &format!(
            "{} {} ${}",
            shop.mart_type, shop.mart_id, snapshot.trainer.money
        ),
        SCENE_DIALOG_TEXT_CHARS,
    ));
    let Some(cursor_index) = cursor_index else {
        entries.push(compact_scene_label(
            &format!("INVALID CURSOR {surface_id}"),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        return;
    };
    entries.extend(
        windowed_index_range(cursor_index, shop.inventory.len()).map(|index| {
            let item = &shop.inventory[index];
            let marker = if index == cursor_index { ">" } else { " " };
            compact_scene_label(
                &format!("{marker}{}", shop_buy_item_label(snapshot, item)),
                SCENE_DIALOG_TEXT_CHARS,
            )
        }),
    );
}

fn spawn_shell_status_banner(commands: &mut Commands, runtime_shell: &BevyRuntimeShell) {
    let Some(message) = shell_status_banner_text(runtime_shell) else {
        return;
    };
    spawn_shell_banner(commands, message, Color::rgb(0.98, 0.96, 0.82));
}

fn spawn_shell_error_banner(commands: &mut Commands, runtime_shell: &BevyRuntimeShell) {
    let Some(error) = &runtime_shell.last_error else {
        return;
    };
    spawn_shell_banner(
        commands,
        format!("ERR {}", compact_scene_label(error, 72)),
        Color::rgb(1.0, 0.82, 0.68),
    );
}

fn spawn_shell_banner(commands: &mut Commands, message: String, text_color: Color) {
    let origin_x = PLAYFIELD_LEFT + TILE_SIZE * 0.75;
    let origin_y = PLAYFIELD_TOP + TILE_SIZE * 0.45;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.03, 0.04, 0.05, 0.82),
                custom_size: Some(Vec2::new(TILE_SIZE * 13.2, TILE_SIZE * 0.86)),
                ..default()
            },
            transform: Transform::from_xyz(origin_x + TILE_SIZE * 6.6, origin_y, 4.6),
            ..default()
        },
        SceneDialogMarker,
    ));
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                message,
                TextStyle {
                    font_size: 14.0,
                    color: text_color,
                    ..default()
                },
            ),
            transform: Transform::from_xyz(origin_x + TILE_SIZE * 0.25, origin_y, 4.8),
            ..default()
        },
        SceneDialogMarker,
    ));
}

fn shell_status_banner_text(runtime_shell: &BevyRuntimeShell) -> Option<String> {
    if let Some(error) = &runtime_shell.last_error {
        return Some(format!("ERR {}", compact_scene_label(error, 72)));
    }
    if let Some(status) = &runtime_shell.last_action_status {
        return Some(compact_scene_label(status, 76));
    }
    let latest = runtime_shell.last_audio_events.last()?;
    if latest.starts_with("saved ")
        || latest.starts_with("loaded ")
        || latest.starts_with("title continue loaded ")
        || latest.starts_with("title new game ")
        || latest.starts_with("battle ")
        || latest.starts_with("queued title music ")
        || latest.starts_with("queued current music ")
        || latest.starts_with("queued battle music ")
        || latest.starts_with("queued battle cry ")
        || latest.starts_with("queued audio preview ")
        || latest.starts_with("queued music fade ")
        || latest.starts_with("queued music stop")
        || latest.starts_with("claimed wild rewards ")
        || latest.starts_with("claimed trainer rewards ")
        || latest.starts_with("scripted wild complete ")
        || latest.starts_with("scripted trainer complete ")
    {
        return Some(compact_scene_label(strip_status_checksum_tail(latest), 76));
    }
    None
}

fn set_shell_action_status(runtime_shell: &mut BevyRuntimeShell, message: impl Into<String>) {
    runtime_shell.last_action_status = Some(message.into());
}

fn strip_status_checksum_tail(message: &str) -> &str {
    message
        .split_once(" checksum=")
        .map(|(head, _)| head)
        .unwrap_or(message)
}

fn spawn_audio_status_label(commands: &mut Commands, runtime_shell: &BevyRuntimeShell) {
    let Some(label) = audio_status_label(runtime_shell) else {
        return;
    };
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 12.0,
                    color: Color::rgb(0.82, 0.94, 1.0),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + TILE_SIZE * 10.2,
                PLAYFIELD_TOP + TILE_SIZE * 0.92,
                4.8,
            ),
            ..default()
        },
        SceneDialogMarker,
    ));
}

fn spawn_map_status_label(commands: &mut Commands, snapshot: &RuntimeShellSnapshot) {
    let label = compact_scene_label(
        &format!(
            "{} ({},{}) {:?}",
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y,
            snapshot.overworld.facing
        ),
        46,
    );
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 12.0,
                    color: Color::rgb(0.92, 0.96, 0.86),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + TILE_SIZE * 0.1,
                PLAYFIELD_TOP + TILE_SIZE * 0.92,
                4.8,
            ),
            ..default()
        },
        SceneDialogMarker,
    ));
}

fn spawn_map_connection_labels(commands: &mut Commands, map: &crate::RuntimeMapCatalogSnapshot) {
    if map.attributes.connections.is_empty() {
        return;
    }
    let label = compact_scene_label(
        &map.attributes
            .connections
            .iter()
            .map(|connection| format!("{}:{}", connection.direction, connection.target_map))
            .collect::<Vec<_>>()
            .join(" "),
        62,
    );
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 10.5,
                    color: Color::rgb(0.78, 0.88, 0.96),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + TILE_SIZE * 0.1,
                PLAYFIELD_TOP + TILE_SIZE * 0.46,
                4.8,
            ),
            ..default()
        },
        SceneDialogMarker,
    ));
}

fn audio_status_label(runtime_shell: &BevyRuntimeShell) -> Option<String> {
    if let Some(music) = &runtime_shell.active_music {
        let mut label = format!("MUSIC {}", compact_scene_label(music, 22));
        if !runtime_shell.pending_audio.is_empty() {
            label.push_str(&format!(" +{}", runtime_shell.pending_audio.len()));
        }
        return Some(label);
    }
    if let Some(music) = &runtime_shell.faded_music {
        return Some(format!("FADE {}", compact_scene_label(music, 24)));
    }
    if runtime_shell.pending_music_stop {
        return Some("MUSIC STOP".to_string());
    }
    None
}

fn visible_field_command_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<Vec<String>> {
    if let Some(game) = runtime_shell.visible_card_flip.as_ref() {
        return Ok(vec![
            "CARD FLIP".to_string(),
            format!("COINS {:04} PAY {:02}", game.coins, game.payout),
            game.message.clone(),
        ]);
    }
    if let Some(machine) = runtime_shell.visible_slot_machine.as_ref() {
        return Ok(vec![
            "SLOT MACHINE".to_string(),
            format!("COINS {:04}", machine.coins),
            format!("BET {} PAY {:04}", machine.bet, machine.payout),
            machine.message.clone(),
        ]);
    }
    if let Some(puzzle) = runtime_shell.visible_unown_puzzle.as_ref() {
        let piece_char = |piece: u8| -> char {
            match piece {
                0 => '·',
                1..=9 => char::from(b'0' + piece),
                10..=16 => char::from(b'A' + (piece - 10)),
                _ => '?',
            }
        };
        return Ok(puzzle
            .layout
            .iter()
            .enumerate()
            .map(|(y, row)| {
                let mut line = String::new();
                for (x, piece) in row.iter().enumerate() {
                    if !line.is_empty() {
                        line.push(' ');
                    }
                    if x == puzzle.cursor_x && y == puzzle.cursor_y {
                        line.push('[');
                        line.push(piece_char(*piece));
                        line.push(']');
                    } else {
                        line.push(' ');
                        line.push(piece_char(*piece));
                        line.push(' ');
                    }
                }
                if y == 5 {
                    line.push_str(&format!(
                        " H:{}",
                        puzzle.holding_piece.map(piece_char).unwrap_or('·')
                    ));
                }
                line
            })
            .collect());
    }
    if runtime_shell.buena_prize_cursor.is_some() {
        let choices = visible_buena_prize_choices(snapshot)?;
        let selected = strict_readonly_cursor_index(
            &runtime_shell.buena_prize_cursor,
            "script:buena-prize",
            choices.len(),
        ).context("Kurt Apricorn selection has no valid cursor")?;
        let start = selected
            .saturating_sub(3)
            .min(choices.len().saturating_sub(4));
        let mut entries = choices
            .iter()
            .enumerate()
            .skip(start)
            .take(4)
            .map(|(index, (item_id, cost))| {
                format!(
                    "{}{} {}",
                    if index == selected { ">" } else { " " },
                    item_display_name(snapshot, item_id),
                    cost
                )
            })
            .collect::<Vec<_>>();
        entries.push(format!("POINTS {:02}", snapshot.trainer.blue_card_balance));
        return Ok(entries);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        let choices = visible_kurt_apricorn_choices(snapshot);
        let selected = strict_readonly_cursor_index(
            &runtime_shell.kurt_apricorn_cursor,
            "script:kurt-apricorn",
            choices.len(),
        ).context("Kurt Apricorn selection has no valid cursor")?;
        if let Some(quantity) = runtime_shell.kurt_apricorn_quantity {
            let (item_id, maximum) = choices
                .get(selected)
                .context("Kurt quantity selection has no Apricorn type")?;
            return Ok(vec![
                "HOW MANY?".to_string(),
                item_display_name(snapshot, item_id),
                format!("×{quantity:02} / {maximum:02}"),
            ]);
        }
        let entries = choices
            .iter()
            .enumerate()
            .map(|(index, (item_id, quantity))| {
                format!(
                    "{}{} x{}",
                    if index == selected { ">" } else { " " },
                    item_display_name(snapshot, item_id),
                    quantity
                )
            })
            .collect::<Vec<_>>();
        return Ok(entries);
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        return visible_bill_pc_action_entries(runtime_shell);
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        return visible_bill_pc_box_entries(snapshot, runtime_shell);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        let actions = visible_player_pc_actions(runtime_shell);
        let selected = strict_readonly_cursor_index(
            &runtime_shell.player_pc_action_cursor,
            "pc:player-actions",
            actions.len(),
        )
        .with_context(|| {
            format!("player PC action cursor is invalid for {} actions", actions.len())
        })?;
        return Ok(actions.iter().enumerate().map(|(index, action)| {
            format!("{}{}", if index == selected { ">" } else { " " }, visible_player_pc_action_label(*action))
        }).collect());
    }
    if runtime_shell.mailbox_cursor.is_some() {
        let mut entries = Vec::new();
        push_visible_mailbox_dialog_entries(&mut entries, snapshot, runtime_shell);
        return Ok(entries);
    }
    if runtime_shell.start_menu_cursor.is_some() {
        return visible_start_menu_entries(runtime_shell);
    }
    if visible_field_pack_is_open(runtime_shell) {
        return Ok(visible_field_pack_entries(snapshot, runtime_shell));
    }
    if runtime_shell.party_menu_open {
        return Ok(visible_party_menu_entries(snapshot, runtime_shell));
    }
    if runtime_shell.pokedex_menu_open {
        return Ok(visible_pokedex_menu_entries(snapshot, runtime_shell));
    }
    if runtime_shell.pokegear_menu_open {
        return Ok(visible_pokegear_menu_entries(snapshot, runtime_shell));
    }
    if runtime_shell.trainer_card_open {
        return Ok(visible_trainer_card_entries(snapshot, runtime_shell));
    }
    if runtime_shell.options_menu_open {
        return Ok(visible_options_menu_entries(snapshot, runtime_shell));
    }
    if runtime_shell.save_menu_open {
        return Ok(Vec::new());
    }
    if runtime_shell.special_boundary.is_some() {
        return Ok(visible_special_boundary_entries(runtime_shell));
    }
    visible_field_idle_entries(snapshot, runtime_shell)
}

fn visible_bill_pc_action_entries(runtime_shell: &BevyRuntimeShell) -> Result<Vec<String>> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_action_cursor,
        "pc:bill-actions",
        VISIBLE_BILL_PC_ACTIONS.len(),
    )
    .context("Bill's PC action menu is open without a valid cursor")?;
    Ok(VISIBLE_BILL_PC_ACTIONS
        .iter()
        .enumerate()
        .map(|(index, action)| {
            compact_scene_label(
                &format!(
                    "{}{}",
                    if index == selected { ">" } else { " " },
                    visible_bill_pc_action_label(*action)
                ),
                SCENE_DIALOG_TEXT_CHARS,
            )
        })
        .collect())
}

fn visible_bill_pc_box_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<Vec<String>> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_box_cursor,
        "pc:bill-boxes",
        crate::core::models::MAX_PC_BOXES,
    )
    .context("Bill's PC box menu is open without a valid cursor")?;
    Ok(windowed_index_range(selected, crate::core::models::MAX_PC_BOXES)
        .map(|index| {
            let (name, count) = snapshot
                .storage
                .boxes
                .iter()
                .find(|pc_box| pc_box.index == index)
                .map(|pc_box| (pc_box.name.as_str(), pc_box.count))
                .unwrap_or(("BOX", 0));
            compact_scene_label(
                &format!(
                    "{}{} {} {}/{}",
                    if index == selected { ">" } else { " " },
                    index + 1,
                    name,
                    count,
                    crate::core::models::MAX_BOX_MONS
                ),
                SCENE_DIALOG_TEXT_CHARS,
            )
        })
        .collect())
}

fn visible_field_idle_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<Vec<String>> {
    if !runtime_debug_overlays_enabled() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    if let Some(map) = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
    {
        let TilePosition {
            x: front_x,
            y: front_y,
        } = facing_runtime_tile(snapshot)?;
        if let Some(prompt) =
            field_prompt_label_checked(runtime_shell, snapshot, map, front_x, front_y)?
        {
            entries.push(compact_scene_label(&prompt, 30));
        } else {
            entries.push(compact_scene_label(
                &format!(
                    "{} ({}, {}) {:?}",
                    snapshot.overworld.map_name,
                    snapshot.overworld.tile.x,
                    snapshot.overworld.tile.y,
                    snapshot.overworld.facing
                ),
                30,
            ));
        }
    } else {
        entries.push(compact_scene_label(
            &format!("MISSING MAP {}", snapshot.overworld.map_name),
            30,
        ));
    }
    entries.push("START MENU".to_string());
    if snapshot.party.slots.is_empty() {
        entries.push("PARTY EMPTY".to_string());
    } else if let Some(lead) = snapshot.party.slots.first() {
        entries.push(compact_scene_label(
            &format!(
                "PARTY {} L{} {}/{}",
                lead.pokemon.species.id, lead.pokemon.level, lead.pokemon.hp, lead.pokemon.max_hp
            ),
            30,
        ));
    }
    let carried_items = carried_item_count(&snapshot.bag.items)
        + carried_item_count(&snapshot.bag.balls)
        + carried_item_count(&snapshot.bag.key_items)
        + snapshot.bag.tm_hm.len();
    entries.push(format!("PACK {carried_items}"));
    if let Some(registered) = snapshot.progression.registered_key_item.as_deref() {
        entries.push(compact_scene_label(&format!("SELECT {registered}"), 30));
    }
    if runtime_shell.quick_save_path.is_some() {
        entries.push("SAVE READY".to_string());
    }
    entries.truncate(SCENE_MENU_VISIBLE_ROWS);
    Ok(entries)
}
