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
    let palette_key = normalize_tileset_time_of_day(snapshot.progression.time.time_of_day.as_key());
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
            (0, _) => {
                if (col - 1) % 4 < 2 {
                    3
                } else {
                    2
                }
            }
            (1, 0) => 5,
            (1, 19) => 11,
            (1, _) => 13,
            (2, 0) => 6,
            (2, 19) => 12,
            (2, _) => 13,
            (3, 0) => 7,
            (3, 19) => 10,
            (3, _) => {
                if (col - 1) % 4 < 2 {
                    9
                } else {
                    8
                }
            }
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
                    sprite: Sprite {
                        custom_size: Some(frame.size),
                        ..default()
                    },
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
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
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
    let source = crate::open_runtime_image(&path)
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
                color: Color::srgba(1.0, 0.92, 0.22, 0.62),
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
                    color: Color::srgb(1.0, 0.98, 0.80),
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

/// Field-owned screens that present a complete LCD through the retained
/// full-screen surface. The presenter is retired only after these states
/// close and the complete overworld replacement has been staged.
fn retained_field_fullscreen_active(runtime_shell: &BevyRuntimeShell) -> bool {
    (runtime_shell.pending_name_choice.is_some()
        && runtime_shell.pending_standard_capture.is_none()
        && runtime_shell.pending_gift_pokemon_nickname.is_none()
        && runtime_shell.pending_egg_hatch_nickname.is_none())
        || runtime_shell.pending_name_input.is_some()
        || runtime_shell.pending_mail_input.is_some()
        || runtime_shell.pending_mail_read.is_some()
        || runtime_shell.pending_egg_hatch_nickname.is_some()
        || runtime_shell.visible_slot_machine.is_some()
        || runtime_shell.visible_card_flip.is_some()
        || runtime_shell.visible_unown_puzzle.is_some()
        || runtime_shell.visible_unown_printer.is_some()
        || runtime_shell.visible_diploma.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.hall_of_fame_pc_index.is_some()
        || runtime_shell.pokedex_menu_open
        || runtime_shell.pokegear_menu_open
        || runtime_shell.trainer_card_open
        || (runtime_shell.party_menu_open && runtime_shell.fly_cursor.is_some())
        || (runtime_shell.storage_cursor.is_some() && !runtime_shell.party_menu_open)
        || (runtime_shell.pc_item_cursor.is_some() && !visible_field_pack_is_open(runtime_shell))
        || runtime_shell.bill_pc_box_cursor.is_some()
        || runtime_shell.visible_egg_hatch.as_ref().is_some_and(|hatch| {
            hatch.phase != VisibleEggHatchPhase::HuhText
        })
}

fn textbox_frame_id(frame: crate::core::state::FrameType) -> u8 {
    use crate::core::state::FrameType;
    match frame {
        FrameType::Frame1 => 1,
        FrameType::Frame2 => 2,
        FrameType::Frame3 => 3,
        FrameType::Frame4 => 4,
        FrameType::Frame5 => 5,
        FrameType::Frame6 => 6,
        FrameType::Frame7 => 7,
        FrameType::Frame8 => 8,
    }
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
        spawn_field_notice(commands, runtime_shell, rendered_art, asset_root, images);
        return;
    }
    // The Town Map is a complete raster surface and intentionally has no
    // command-entry rows. Render Pokégear ownership before the generic empty
    // entry guard, or TownMapScript opens modal state while drawing nothing.
    if runtime_shell.pokegear_menu_open {
        if let Err(error) = spawn_field_pokegear_screen(
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
    if runtime_shell.visible_buena_password.is_some() {
        if let Err(error) = spawn_visible_buena_password_menu(
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
    if runtime_shell.visible_battle_tower_challenge_menu.is_some() {
        if let Err(error) = spawn_visible_battle_tower_challenge_menu(
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
    if runtime_shell.visible_battle_tower_room_menu.is_some() {
        if let Err(error) = spawn_visible_battle_tower_room_menu(
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
        if let Err(error) =
            spawn_visible_hall_of_fame_pc(commands, runtime_shell, rendered_art, asset_root, images)
        {
            *render_error = Some(error);
        }
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
        if let Err(error) =
            spawn_visible_unown_puzzle(commands, runtime_shell, rendered_art, asset_root, images)
        {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.visible_unown_printer.is_some() {
        if let Err(error) = spawn_visible_unown_printer(
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
        if let Err(error) =
            spawn_trainer_card_screen(commands, snapshot, runtime_shell, rendered_art, images)
        {
            *render_error = Some(error);
        }
        return;
    }
    if runtime_shell.pokedex_menu_open {
        if let Err(error) = spawn_field_pokedex_screen(
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
    if visible_field_pack_is_open(runtime_shell)
        && runtime_shell.field_pack_target_mode.is_none()
        && runtime_shell.tmhm_teach_prompt_cursor.is_none()
        && runtime_shell.tmhm_decision_prompt_cursor.is_none()
        && !runtime_shell.tmhm_forget_menu_open
    {
        if let Err(error) = spawn_field_pack_screen(
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
        if let Err(error) = spawn_field_party_give_take_window(
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
    if runtime_shell.party_menu_open && runtime_shell.party_switch_cursor.is_some() {
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
        }
        return;
    }
    if runtime_shell.party_menu_open && runtime_shell.fly_cursor.is_some() {
        if let Err(error) = spawn_field_fly_map_screen(
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
    let window_height = if two_columns { 5 } else { 7 };
    let window_left = 10.0;
    let window_top = 11.0;
    let window_width = 8;
    let (center_x, center_y) = field_window_center(
        window_left,
        window_top,
        window_width as f32,
        window_height as f32,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (window_width - 2) as f32,
                    TILE_SIZE * (window_height - 2) as f32,
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 3.3),
            ..default()
        },
        FieldCommandMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_field_command_window_frame_tiles(
            commands,
            frame,
            window_left,
            window_top,
            window_width,
            window_height,
            3.35,
        );
    }
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

fn spawn_visible_buena_password_menu(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let menu = runtime_shell
        .visible_buena_password
        .as_ref()
        .context("no Buena password menu is open")?;
    anyhow::ensure!(menu.options.len() == 3, "Buena password menu must have three choices");
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let labels = menu
        .options
        .iter()
        .map(|option| match menu.category_type.as_str() {
            "BUENA_MON" => Ok(crate::core::models::pokemon_species_display_name(option)),
            "BUENA_ITEM" => Ok(item_display_name(&snapshot, option)),
            "BUENA_MOVE" => Ok(battle_move_display_name(&snapshot, option)),
            "BUENA_STRING" => Ok(option.clone()),
            other => anyhow::bail!("unknown Buena password category type {other}"),
        })
        .collect::<Result<Vec<_>>>()?;
    let selected = strict_readonly_cursor_index(
        &Some(menu.cursor.clone()),
        "script:buena-password",
        menu.options.len(),
    )
    .context("Buena password menu has no valid cursor")?;
    let width = labels
        .iter()
        .map(|option| option.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(3)
        .clamp(11, 20);
    let (center_x, center_y) = field_window_center(0.0, 0.0, width as f32, 8.0);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (width - 2) as f32,
                    TILE_SIZE * 6.0,
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 3.3),
            ..default()
        },
        FieldCommandMarker,
    ));
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("Buena password window frame art is unavailable")?;
    spawn_field_command_window_frame_tiles(commands, frame, 0.0, 0.0, width, 8, 3.35);
    for (index, option) in labels.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 2.0 + index as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{}{}", if index == selected { ">" } else { " " }, option),
            x,
            y,
            3.6,
        );
    }
    Ok(())
}

fn spawn_visible_battle_tower_challenge_menu(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let menu = runtime_shell
        .visible_battle_tower_challenge_menu
        .as_ref()
        .context("no Battle Tower challenge menu is open")?;
    let options: &[&str] = if menu.english {
        &["CHALLENGE", "EXPLANATION", "CANCEL"]
    } else {
        &["NEWS", "NEWS", "???", "CANCEL"]
    };
    let selected = strict_readonly_cursor_index(
        &Some(menu.cursor.clone()),
        "script:battle-tower-challenge",
        options.len(),
    )
    .context("Battle Tower challenge menu has no valid cursor")?;
    let (center_x, center_y) = field_window_center(0.0, 0.0, 14.0, 8.0);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(TILE_SIZE * 12.0, TILE_SIZE * 6.0)),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 3.3),
            ..default()
        },
        FieldCommandMarker,
    ));
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("Battle Tower challenge menu frame art is unavailable")?;
    spawn_field_command_window_frame_tiles(commands, frame, 0.0, 0.0, 14, 8, 3.35);
    for (index, option) in options.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 2.0 + index as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{}{}", if index == selected { ">" } else { " " }, option),
            x,
            y,
            3.6,
        );
    }
    Ok(())
}

fn spawn_visible_battle_tower_room_menu(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let menu = runtime_shell
        .visible_battle_tower_room_menu
        .as_ref()
        .context("no Battle Tower room menu is open")?;
    commit_presented_fullscreen_solid(commands, rendered_art, [247, 247, 247, 255], 3.2, images)?;
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("Battle Tower room menu frame art is unavailable")?;
    match &menu.phase {
        VisibleBattleTowerRoomMenuPhase::PickLevel => {
            let selected = strict_readonly_cursor_index(
                &Some(menu.cursor.clone()),
                "script:battle-tower-room",
                menu.level_groups.len() + 1,
            )
            .context("Battle Tower room menu has no valid cursor")?;
            let choice = menu
                .level_groups
                .get(selected)
                .map(|group| format!("L:{:>3}", u16::from(*group) * 10))
                .unwrap_or_else(|| "CANCEL".to_string());
            spawn_field_command_window_frame_tiles(commands, frame, 12.0, 7.0, 8, 7, 3.35);
            for (text, tile_x, tile_y) in [
                ("What level do you\nwant to challenge?".to_string(), 1.0, 2.0),
                ("▲".to_string(), 16.0, 8.0),
                (choice, 13.0, 10.0),
                ("▼".to_string(), 16.0, 12.0),
            ] {
                let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
                spawn_field_command_bitmap_text(
                    commands, rendered_art, asset_root, images, &text, x, y, 3.6,
                );
            }
        }
        VisibleBattleTowerRoomMenuPhase::ConfirmCancel { yes_no_index } => {
            spawn_field_command_window_frame_tiles(commands, frame, 13.0, 7.0, 7, 6, 3.35);
            for (text, tile_x, tile_y) in [
                ("Cancel your BATTLE\nROOM challenge?".to_string(), 1.0, 2.0),
                (
                    format!("{}YES", if *yes_no_index == 0 { ">" } else { " " }),
                    14.0,
                    8.0,
                ),
                (
                    format!("{}NO", if *yes_no_index == 1 { ">" } else { " " }),
                    14.0,
                    10.0,
                ),
            ] {
                let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
                spawn_field_command_bitmap_text(
                    commands, rendered_art, asset_root, images, &text, x, y, 3.6,
                );
            }
        }
        VisibleBattleTowerRoomMenuPhase::Rejection { message } => {
            let (x, y) = battle_hud_tile_origin(1.0, 2.0);
            spawn_field_command_bitmap_text(
                commands, rendered_art, asset_root, images, message, x, y, 3.6,
            );
        }
    }
    Ok(())
}

fn spawn_visible_hall_of_fame_pc(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(boundary) = runtime_shell
        .special_boundary
        .as_ref()
        .filter(|boundary| boundary.label == "HallOfFamePC")
    else {
        return Ok(());
    };
    commit_presented_fullscreen_solid(commands, rendered_art, [247, 247, 247, 255], 5.8, images)?;
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(commands, frame, 0.0, 0.0, 20, 9, 5.9);
    }
    for (row, line) in boundary.details.iter().take(7).enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 1.0 + row as f32);
        spawn_scene_dialog_bitmap_text(commands, rendered_art, asset_root, images, line, x, y, 6.0);
    }
    Ok(())
}

fn spawn_visible_slot_machine(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(machine) = runtime_shell.visible_slot_machine.as_ref() else {
        return Ok(());
    };
    if rendered_art.slot_machine_sources.is_none()
        && rendered_art.slot_machine_source_error.is_none()
    {
        match load_slot_machine_render_sources(asset_root) {
            Ok(sources) => rendered_art.slot_machine_sources = Some(sources),
            Err(error) => rendered_art.slot_machine_source_error = Some(format!("{error:#}")),
        }
    }
    if let Some(error) = rendered_art.slot_machine_source_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let sources = rendered_art
        .slot_machine_sources
        .as_ref()
        .context("slot machine render sources are unavailable")?;
    let frame = render_visible_slot_machine_frame(sources, machine, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        3.3,
        images,
    )?;
    for (text, tile_x, tile_y) in [
        (format!("{:04}", machine.coins), 5.0, 1.0),
        (format!("{:04}", machine.payout), 11.0, 1.0),
        (machine.message.clone(), 1.0, 15.0),
        (format!("BET {} A:SPIN B:QUIT", machine.bet), 1.0, 17.0),
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
            3.7,
        );
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
    let Some(game) = runtime_shell.visible_card_flip.as_ref() else {
        return Ok(());
    };
    if rendered_art.card_flip_sources.is_none() && rendered_art.card_flip_source_error.is_none() {
        match load_card_flip_render_sources(asset_root) {
            Ok(sources) => rendered_art.card_flip_sources = Some(sources),
            Err(error) => rendered_art.card_flip_source_error = Some(format!("{error:#}")),
        }
    }
    if let Some(error) = rendered_art.card_flip_source_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let sources = rendered_art
        .card_flip_sources
        .as_ref()
        .context("Card Flip render sources are unavailable")?;
    let frame = render_visible_card_flip_frame(sources, game, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        3.3,
        images,
    )?;
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
    if matches!(
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
    Ok(())
}

fn load_card_flip_render_sources(asset_root: &AssetRoot) -> Result<CardFlipRenderSources> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    let root = asset_root.resolve_vendor("gfx/card_flip");
    let sheet_1 = crate::open_runtime_image(root.join("card_flip_1.png"))
        .context("decode Card Flip primary sheet")?
        .to_rgba8();
    let sheet_2 = crate::open_runtime_image(root.join("card_flip_2.png"))
        .context("decode Card Flip secondary sheet")?
        .to_rgba8();
    let sheet_3 = crate::open_runtime_image(root.join("card_flip_3.png"))
        .context("decode Card Flip object sheet")?
        .to_rgba8();
    let font = crate::open_runtime_image(asset_root.runtime_assets().join("gfx/font/font.png"))
        .context("decode Card Flip font")?
        .to_rgba8();
    let light_off = crate::open_runtime_image(root.join("off.png"))
        .context("decode Card Flip off light")?
        .to_rgba8();
    let light_on = crate::open_runtime_image(root.join("on.png"))
        .context("decode Card Flip on light")?
        .to_rgba8();
    anyhow::ensure!(
        sheet_1.dimensions() == (128, 32),
        "invalid Card Flip primary sheet"
    );
    anyhow::ensure!(
        sheet_2.dimensions() == (24, 160),
        "invalid Card Flip secondary sheet"
    );
    anyhow::ensure!(
        sheet_3.dimensions() == (8, 56),
        "invalid Card Flip object sheet"
    );
    anyhow::ensure!(
        light_off.dimensions() == (8, 8),
        "invalid Card Flip off light"
    );
    anyhow::ensure!(
        light_on.dimensions() == (8, 8),
        "invalid Card Flip on light"
    );
    let tilemap = crate::read_runtime_asset(root.join("card_flip.tilemap"))
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
    Ok(CardFlipRenderSources {
        base: target,
        background_tiles: sheet_1,
        face_tiles: sheet_2,
        object_tiles: sheet_3,
        font,
        light_on,
        palettes,
    })
}

fn render_visible_card_flip_frame(
    sources: &CardFlipRenderSources,
    game: &VisibleCardFlip,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    let mut target = sources.base.clone();
    let completed_rounds = game.revealed.iter().filter(|flag| **flag).count().min(12);
    for round in 0..completed_rounds {
        let linear_tile = 9 + round;
        blit_paletted_slot_tile(
            &sources.light_on,
            0,
            &sources.palettes[card_flip_background_palette(linear_tile % 20, linear_tile / 20)],
            (linear_tile % 20) * TILE,
            (linear_tile / 20) * TILE,
            false,
            &mut target,
        );
    }
    match game.phase {
        VisibleCardFlipPhase::ChooseCard => {
            match game.animation {
                VisibleCardFlipAnimation::Deal { frame } if frame < 20 => {}
                VisibleCardFlipAnimation::Deal { .. } => {
                    draw_card_flip_face_down(sources, 2, 0, &mut target);
                }
                VisibleCardFlipAnimation::Cycle { .. } => {
                    draw_card_flip_face_down(sources, 2, 0, &mut target);
                    draw_card_flip_face_down(sources, 2, 6, &mut target);
                    draw_card_flip_border(sources, game.which_card, &mut target);
                }
                VisibleCardFlipAnimation::SelectFlash { frame } => {
                    draw_card_flip_face_down(sources, 2, 0, &mut target);
                    draw_card_flip_face_down(sources, 2, 6, &mut target);
                    if frame / 4 % 2 == 0 {
                        draw_card_flip_border(sources, game.which_card, &mut target);
                    }
                }
                VisibleCardFlipAnimation::None
                | VisibleCardFlipAnimation::WaitStake
                | VisibleCardFlipAnimation::WaitBeforeReveal
                | VisibleCardFlipAnimation::WaitReveal
                | VisibleCardFlipAnimation::WaitResult { .. }
                | VisibleCardFlipAnimation::Payout { .. }
                | VisibleCardFlipAnimation::AwaitResult
                | VisibleCardFlipAnimation::QuitWaitBefore
                | VisibleCardFlipAnimation::QuitWaitAfter => {}
            }
        }
        VisibleCardFlipPhase::PlaceBet => {
            draw_card_flip_face_down(sources, 2, game.which_card * 6, &mut target);
            draw_card_flip_bet_cursor(sources, game.bet_x, game.bet_y, &mut target);
        }
        VisibleCardFlipPhase::Result | VisibleCardFlipPhase::PlayAgain => {
            if let Some((species, level)) = game.face_card.as_ref() {
                draw_card_flip_face_up(
                    sources,
                    2,
                    game.which_card * 6,
                    species,
                    *level,
                    &mut target,
                )?;
            }
        }
        VisibleCardFlipPhase::AskPlay
        | VisibleCardFlipPhase::Shuffled
        | VisibleCardFlipPhase::NotEnoughCoins => {}
    }
    let mut image = Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        target,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(WIDTH as f32, HEIGHT as f32),
    })
}

const CARD_FLIP_FACE_DOWN: [[u8; 5]; 6] = [
    [0x08, 0x09, 0x09, 0x09, 0x0a],
    [0x0b, 0x28, 0x2b, 0x28, 0x0c],
    [0x0b, 0x2c, 0x2d, 0x2e, 0x0c],
    [0x0b, 0x2f, 0x30, 0x31, 0x0c],
    [0x0b, 0x32, 0x33, 0x34, 0x0c],
    [0x0d, 0x0e, 0x0e, 0x0e, 0x0f],
];

const CARD_FLIP_FACE_UP: [[u8; 5]; 6] = [
    [0x18, 0x19, 0x19, 0x19, 0x1a],
    [0x1b, 0x35, 0x7f, 0x7f, 0x1c],
    [0x0b, 0x28, 0x28, 0x28, 0x0c],
    [0x0b, 0x28, 0x28, 0x28, 0x0c],
    [0x0b, 0x28, 0x28, 0x28, 0x0c],
    [0x1d, 0x1e, 0x1e, 0x1e, 0x1f],
];

fn draw_card_flip_bg_box(
    sources: &CardFlipRenderSources,
    tilemap: &[[u8; 5]; 6],
    tile_x: usize,
    tile_y: usize,
    palette_index: usize,
    target: &mut [u8],
) {
    for (row, tiles) in tilemap.iter().enumerate() {
        for (column, tile) in tiles.iter().copied().enumerate() {
            if tile == 0x7f {
                continue;
            }
            blit_paletted_slot_tile(
                &sources.background_tiles,
                usize::from(tile),
                &sources.palettes[palette_index],
                (tile_x + column) * 8,
                (tile_y + row) * 8,
                false,
                target,
            );
        }
    }
}

fn draw_card_flip_face_down(
    sources: &CardFlipRenderSources,
    tile_x: usize,
    tile_y: usize,
    target: &mut [u8],
) {
    draw_card_flip_bg_box(sources, &CARD_FLIP_FACE_DOWN, tile_x, tile_y, 0, target);
}

fn draw_card_flip_face_up(
    sources: &CardFlipRenderSources,
    tile_x: usize,
    tile_y: usize,
    species: &str,
    level: u8,
    target: &mut [u8],
) -> Result<()> {
    let (palette_index, source_anchor): (usize, usize) = match species {
        "PIKACHU" => (1, 0x4e),
        "JIGGLYPUFF" => (2, 0x57),
        "POLIWAG" => (3, 0x69),
        "ODDISH" => (4, 0x60),
        other => anyhow::bail!("unknown Card Flip face species {other}"),
    };
    anyhow::ensure!((1..=6).contains(&level), "invalid Card Flip face level {level}");
    draw_card_flip_bg_box(
        sources,
        &CARD_FLIP_FACE_UP,
        tile_x,
        tile_y,
        palette_index,
        target,
    );
    // The source adds SCREEN_HEIGHT (18), not SCREEN_WIDTH, to the card's
    // tilemap address before writing its 3x3 portrait.
    let portrait_linear = tile_y * 20 + tile_x + 18;
    let portrait_x = portrait_linear % 20;
    let portrait_y = portrait_linear / 20;
    for row in 0..3 {
        for column in 0..3 {
            blit_paletted_slot_tile(
                &sources.face_tiles,
                source_anchor - 0x3e + row * 3 + column,
                &sources.palettes[palette_index],
                (portrait_x + column) * 8,
                (portrait_y + row) * 8,
                false,
                target,
            );
        }
    }
    draw_card_flip_level(
        sources,
        level,
        (tile_x + 3) * 8,
        (tile_y + 1) * 8,
        palette_index,
        target,
    );
    Ok(())
}

fn draw_card_flip_level(
    sources: &CardFlipRenderSources,
    level: u8,
    dest_x: usize,
    dest_y: usize,
    palette_index: usize,
    target: &mut [u8],
) {
    let tile_index = usize::from(0xf6 + level - 0x80);
    let columns = sources.font.width() as usize / 8;
    let source_x = tile_index % columns * 8;
    let source_y = tile_index / columns * 8;
    for y in 0..7 {
        for x in 0..8 {
            let pixel = sources.font.get_pixel((source_x + x) as u32, (source_y + y + 1) as u32);
            let color = palette_index_from_gray(pixel[0]);
            let [red, green, blue] = sources.palettes[palette_index][color];
            let offset = ((dest_y + y) * 160 + dest_x + x) * 4;
            target[offset..offset + 4].copy_from_slice(&[red, green, blue, 255]);
        }
    }
}

fn draw_card_flip_border(
    sources: &CardFlipRenderSources,
    which_card: usize,
    target: &mut [u8],
) {
    let origin_x = 2 * 8;
    let origin_y = which_card * 6 * 8;
    for (x, y, tile, flip_x, flip_y) in [
        (0, 0, 4, false, false), (1, 0, 6, false, false),
        (2, 0, 6, false, false), (3, 0, 6, false, false), (4, 0, 4, true, false),
        (0, 1, 5, false, false), (4, 1, 5, true, false),
        (0, 2, 5, false, false), (4, 2, 5, true, false),
        (0, 3, 5, false, false), (4, 3, 5, true, false),
        (0, 4, 5, false, false), (4, 4, 5, true, false),
        (0, 5, 4, false, true), (1, 5, 6, false, true),
        (2, 5, 6, false, true), (3, 5, 6, false, true), (4, 5, 4, true, true),
    ] {
        blit_card_flip_object_tile(
            sources,
            tile,
            (origin_x + x * 8) as i32,
            (origin_y + y * 8) as i32,
            flip_x,
            flip_y,
            target,
        );
    }
}

fn draw_card_flip_bet_cursor(
    sources: &CardFlipRenderSources,
    cursor_x: usize,
    cursor_y: usize,
    target: &mut [u8],
) {
    #[derive(Clone, Copy)]
    enum Shape {
        Impossible,
        SingleTile,
        PokeGroup,
        PokeGroupPair,
        NumGroup,
        NumGroupPair,
    }

    let Some((anchor_x, anchor_y, shape)) = (match (cursor_x, cursor_y) {
        (0, 0) => Some((88, 16, Shape::Impossible)),
        (1, 0) => Some((96, 16, Shape::Impossible)),
        (2 | 3, 0) => Some((104, 16, Shape::PokeGroupPair)),
        (4 | 5, 0) => Some((136, 16, Shape::PokeGroupPair)),
        (0, 1) => Some((88, 24, Shape::Impossible)),
        (1, 1) => Some((96, 24, Shape::Impossible)),
        (2..=5, 1) => Some((104 + (cursor_x - 2) as i32 * 16, 24, Shape::PokeGroup)),
        (0, 2 | 3) => Some((88, 40, Shape::NumGroupPair)),
        (1, 2) => Some((96, 40, Shape::NumGroup)),
        (1, 3) => Some((96, 52, Shape::NumGroup)),
        (2..=5, 2) => Some((104 + (cursor_x - 2) as i32 * 16, 40, Shape::SingleTile)),
        (2..=5, 3) => Some((104 + (cursor_x - 2) as i32 * 16, 52, Shape::SingleTile)),
        (0, 4 | 5) => Some((88, 64, Shape::NumGroupPair)),
        (1, 4) => Some((96, 64, Shape::NumGroup)),
        (1, 5) => Some((96, 76, Shape::NumGroup)),
        (2..=5, 4) => Some((104 + (cursor_x - 2) as i32 * 16, 64, Shape::SingleTile)),
        (2..=5, 5) => Some((104 + (cursor_x - 2) as i32 * 16, 76, Shape::SingleTile)),
        (0, 6 | 7) => Some((88, 88, Shape::NumGroupPair)),
        (1, 6) => Some((96, 88, Shape::NumGroup)),
        (1, 7) => Some((96, 100, Shape::NumGroup)),
        (2..=5, 6) => Some((104 + (cursor_x - 2) as i32 * 16, 88, Shape::SingleTile)),
        (2..=5, 7) => Some((104 + (cursor_x - 2) as i32 * 16, 100, Shape::SingleTile)),
        _ => None,
    }) else {
        return;
    };
    let mut sprite = |x_tile, y_tile, x_pixel, y_pixel, tile, flip_x, flip_y| {
        blit_card_flip_object_tile(
            sources,
            tile,
            anchor_x + x_tile * 8 + x_pixel,
            anchor_y + y_tile * 8 + y_pixel,
            flip_x,
            flip_y,
            target,
        );
    };

    match shape {
        Shape::Impossible => {
            sprite(0, 0, 0, 0, 0, false, false);
            sprite(1, 0, 0, 0, 0, true, false);
            sprite(0, 1, 0, 0, 0, false, true);
            sprite(1, 1, 0, 0, 0, true, true);
        }
        Shape::SingleTile => {
            sprite(-1, 0, 7, 0, 0, false, false);
            sprite(0, 0, 0, 0, 2, false, false);
            sprite(1, 0, 0, 0, 3, false, false);
            sprite(-1, 0, 7, 5, 0, false, true);
            sprite(0, 0, 0, 5, 2, false, true);
            sprite(1, 0, 0, 5, 3, false, false);
        }
        Shape::PokeGroup => {
            sprite(-1, 0, 7, 0, 0, false, false);
            sprite(0, 0, 0, 0, 2, false, false);
            sprite(1, 0, 0, 0, 0, true, false);
            sprite(-1, 1, 7, 0, 1, false, false);
            sprite(1, 1, 0, 0, 1, true, false);
            for row in 2..=10 {
                sprite(-1, row, 7, 0, 1, false, false);
                sprite(1, row, 0, 0, 3, false, false);
            }
            sprite(-1, 10, 7, 1, 0, false, true);
            sprite(0, 10, 0, 1, 2, false, true);
            sprite(1, 10, 0, 1, 3, false, false);
        }
        Shape::NumGroup => {
            sprite(-1, 0, 7, 0, 0, false, false);
            for column in 0..=8 {
                let tile = if column == 0 || column % 2 == 1 { 2 } else { 3 };
                sprite(column, 0, 0, 0, tile, false, false);
            }
            sprite(-1, 0, 7, 5, 0, false, true);
            for column in 0..=8 {
                let tile = if column == 0 || column % 2 == 1 { 2 } else { 3 };
                sprite(column, 0, 0, 5, tile, false, tile == 2);
            }
        }
        Shape::NumGroupPair => {
            for column in 0..=9 {
                let tile = if column == 0 { 0 } else if column <= 2 || column % 2 == 0 { 2 } else { 3 };
                sprite(column, 0, 0, 0, tile, false, false);
            }
            for row in 1..=2 {
                sprite(0, row, 0, 0, 1, false, false);
                for column in [3, 5, 7, 9] {
                    sprite(column, row, 0, 0, 3, false, false);
                }
            }
            sprite(0, 2, 0, 1, 0, false, true);
            sprite(1, 2, 0, 1, 2, false, true);
            sprite(2, 2, 0, 1, 2, false, true);
            for column in 3..=9 {
                sprite(column, 2, 0, 1, 3, false, false);
            }
        }
        Shape::PokeGroupPair => {
            sprite(-1, 0, 7, 0, 0, false, false);
            sprite(3, 0, 0, 0, 0, true, false);
            for row in 1..=2 {
                sprite(-1, row, 7, 0, 1, false, false);
                sprite(3, row, 0, 0, 1, true, false);
            }
            for row in 3..=11 {
                sprite(-1, row, 7, 0, 1, false, false);
                sprite(1, row, 0, 0, 3, false, false);
                sprite(3, row, 0, 0, 3, false, false);
            }
            sprite(-1, 11, 7, 1, 0, false, true);
            sprite(0, 11, 0, 1, 2, false, true);
            sprite(1, 11, 0, 1, 3, false, true);
            sprite(2, 11, 0, 1, 2, false, true);
            sprite(3, 11, 0, 1, 3, true, true);
        }
    }
}

fn blit_card_flip_object_tile(
    sources: &CardFlipRenderSources,
    tile: usize,
    dest_x: i32,
    dest_y: i32,
    flip_x: bool,
    flip_y: bool,
    target: &mut [u8],
) {
    for y in 0..8 {
        for x in 0..8 {
            let source_x = if flip_x { 7 - x } else { x };
            let source_y = if flip_y { 7 - y } else { y };
            let pixel = sources.object_tiles.get_pixel(source_x as u32, (tile * 8 + source_y) as u32);
            let color = palette_index_from_gray(pixel[0]);
            let output_x = dest_x + x as i32;
            let output_y = dest_y + y as i32;
            if color == 0 || !(0..160).contains(&output_x) || !(0..144).contains(&output_y) {
                continue;
            }
            let [red, green, blue] = sources.palettes[0][color];
            let offset = (output_y as usize * 160 + output_x as usize) * 4;
            target[offset..offset + 4].copy_from_slice(&[red, green, blue, 255]);
        }
    }
}

fn load_card_flip_palettes(path: &Path) -> Result<Vec<Palette>> {
    let source = crate::read_runtime_asset_to_string(path)
        .with_context(|| format!("read Card Flip palette {}", path.display()))?;
    let colors = source
        .lines()
        .filter_map(|raw_line| {
            let line = raw_line.split(';').next().unwrap_or("").trim();
            line.to_ascii_uppercase().starts_with("RGB").then_some(line)
        })
        .map(|line| rgb_triplet_to_u8(&parse_rgb_values(line)?))
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(
        colors.len() == 36,
        "Card Flip palette must contain 36 colors"
    );
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

fn load_slot_machine_render_sources(asset_root: &AssetRoot) -> Result<SlotMachineRenderSources> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    let root = asset_root.resolve_vendor("gfx/slots");
    let ui = crate::open_runtime_image(root.join("slots_1.png"))
        .context("decode slots UI sheet")?
        .to_rgba8();
    let symbols = crate::open_runtime_image(root.join("slots_2.png"))
        .context("decode slots symbol sheet")?
        .to_rgba8();
    let actors = crate::open_runtime_image(root.join("slots_3.png"))
        .context("decode slots actor sheet")?
        .to_rgba8();
    anyhow::ensure!(
        ui.dimensions() == (16, 152),
        "invalid slots UI sheet dimensions"
    );
    anyhow::ensure!(
        symbols.dimensions() == (16, 256),
        "invalid slots symbol sheet dimensions"
    );
    anyhow::ensure!(
        actors.dimensions() == (24, 240),
        "invalid slots actor sheet dimensions"
    );
    let tilemap =
        crate::read_runtime_asset(root.join("slots.tilemap")).context("read slots tilemap")?;
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
            sheet,
            source_tile,
            &palettes[palette],
            tile_x * TILE,
            tile_y * TILE,
            false,
            &mut target,
        );
    }
    Ok(SlotMachineRenderSources {
        base: target,
        symbols,
        actors,
        palettes,
    })
}

fn render_visible_slot_machine_frame(
    sources: &SlotMachineRenderSources,
    machine: &VisibleSlotMachine,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    let mut target = sources.base.clone();
    let flash_inverted = matches!(
        machine.animation,
        VisibleSlotMachineAnimation::FlashResult { frames_remaining }
            if frames_remaining % 2 == 1
    );
    for reel in 0..3 {
        for row in 0..3 {
            let symbol_index = slot_symbol_palette_index(&machine.windows[reel][2 - row])?;
            let base_tile = symbol_index * 4;
            let mut palette = sources.palettes[symbol_index];
            if flash_inverted {
                palette.reverse();
            }
            for icon_y in 0..2 {
                for icon_x in 0..2 {
                    blit_paletted_slot_tile(
                        &sources.symbols,
                        base_tile + icon_y * 2 + icon_x,
                        &palette,
                        [5, 9, 13][reel] * TILE + icon_x * TILE,
                        [4, 6, 8][row] * TILE + icon_y * TILE,
                        true,
                        &mut target,
                    );
                }
            }
        }
    }
    if machine.background_y_offset != 0 {
        target = scroll_slot_background(&target, machine.background_y_offset);
    }
    if let Some(actor) = machine.actor {
        draw_visible_slot_actor(sources, actor, &mut target);
    }
    if let Some(actor) = machine.secondary_actor {
        draw_visible_slot_actor(sources, actor, &mut target);
    }
    let mut image = Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        target,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(WIDTH as f32, HEIGHT as f32),
    })
}

fn scroll_slot_background(source: &[u8], y_offset: i8) -> Vec<u8> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    let mut target = vec![255; source.len()];
    for dest_y in 0..HEIGHT {
        let source_y = dest_y as isize + isize::from(y_offset);
        if !(0..HEIGHT as isize).contains(&source_y) {
            continue;
        }
        let source_y = source_y as usize;
        let source_start = source_y * WIDTH * 4;
        let dest_start = dest_y * WIDTH * 4;
        target[dest_start..dest_start + WIDTH * 4]
            .copy_from_slice(&source[source_start..source_start + WIDTH * 4]);
    }
    target
}

fn draw_visible_slot_actor(
    sources: &SlotMachineRenderSources,
    actor: VisibleSlotActor,
    target: &mut [u8],
) {
    const GOLEM_PIECES: [(i16, i16, usize, bool); 6] = [
        (-12, -12, 0, false),
        (-4, -12, 2, false),
        (4, -12, 0, true),
        (-12, 4, 4, false),
        (-4, 4, 6, false),
        (4, 4, 4, true),
    ];
    const CHANSEY_TOP: [(i16, i16, usize); 3] =
        [(-12, -12, 0), (-4, -12, 2), (4, -12, 4)];
    const CHANSEY_BOTTOMS: [[usize; 3]; 5] = [
        [6, 8, 10],
        [12, 14, 16],
        [18, 20, 22],
        [24, 26, 28],
        [36, 38, 40],
    ];
    let palette = &sources.palettes[9];
    match actor {
        VisibleSlotActor::Golem {
            x,
            y_offset,
            frame,
            flip_x,
            flip_y,
            ..
        } => {
            let frame_base = if frame % 2 == 0 { 0 } else { 8 };
            for (relative_x, relative_y, tile, piece_flip_x) in GOLEM_PIECES {
                blit_paletted_slot_object(
                    &sources.actors,
                    frame_base + tile,
                    palette,
                    x + if flip_x { -8 - relative_x } else { relative_x } - 8,
                    104 + y_offset
                        + if flip_y { -8 - relative_y } else { relative_y }
                        - 16,
                    piece_flip_x ^ flip_x,
                    flip_y,
                    target,
                );
            }
        }
        VisibleSlotActor::Chansey {
            x,
            frame,
            finishing,
            ..
        } => {
            let set = if finishing {
                [0, 3, 4, 3, 0][usize::from(frame).min(4)]
            } else {
                [0, 1, 0, 2][usize::from(frame) % 4]
            };
            let top = if set == 4 {
                [(-12, -12, 30), (-4, -12, 32), (4, -12, 34)]
            } else {
                CHANSEY_TOP
            };
            for (relative_x, relative_y, tile) in top {
                blit_paletted_slot_object(
                    &sources.actors,
                    16 + tile,
                    palette,
                    x + relative_x - 8,
                    relative_y - 16,
                    false,
                    false,
                    target,
                );
            }
            for (column, tile) in CHANSEY_BOTTOMS[set].into_iter().enumerate() {
                blit_paletted_slot_object(
                    &sources.actors,
                    16 + tile,
                    palette,
                    x - 12 + column as i16 * 8 - 8,
                    4 - 16,
                    false,
                    false,
                    target,
                );
            }
        }
        VisibleSlotActor::Egg { x, y_offset } => blit_paletted_slot_object(
            &sources.actors,
            58,
            palette,
            x - 4 - 8,
            108 + y_offset - 4 - 16,
            false,
            false,
            target,
        ),
    }
}

fn blit_paletted_slot_object(
    source: &image::RgbaImage,
    tile_index: usize,
    palette: &Palette,
    dest_x: i16,
    dest_y: i16,
    flip_x: bool,
    flip_y: bool,
    target: &mut [u8],
) {
    const TARGET_WIDTH: i16 = 160;
    const TARGET_HEIGHT: i16 = 144;
    let columns = source.width() as usize / 8;
    for y in 0..16 {
        for x in 0..8 {
            let output_x = dest_x + x;
            let output_y = dest_y + y;
            if !(0..TARGET_WIDTH).contains(&output_x) || !(0..TARGET_HEIGHT).contains(&output_y) {
                continue;
            }
            let source_pixel_x = if flip_x { 7 - x } else { x };
            let source_pixel_y = if flip_y { 15 - y } else { y };
            let source_tile = tile_index + usize::from(source_pixel_y >= 8);
            let source_x = (source_tile % columns) * 8 + source_pixel_x as usize;
            let source_y = (source_tile / columns) * 8 + (source_pixel_y as usize % 8);
            let pixel = source.get_pixel(source_x as u32, source_y as u32);
            let palette_index = palette_index_from_gray(pixel[0]);
            if palette_index == 0 {
                continue;
            }
            let [red, green, blue] = palette[palette_index];
            let offset = (usize::from(output_y as u16) * TARGET_WIDTH as usize
                + usize::from(output_x as u16))
                * 4;
            target[offset] = red;
            target[offset + 1] = green;
            target[offset + 2] = blue;
            target[offset + 3] = 255;
        }
    }
}

fn load_slot_machine_palettes(path: &Path) -> Result<Vec<Palette>> {
    let source = crate::read_runtime_asset_to_string(path)
        .with_context(|| format!("read slot palette {}", path.display()))?;
    let mut colors = Vec::new();
    for raw_line in source.lines() {
        let line = raw_line.split(';').next().unwrap_or("").trim();
        if !line.to_ascii_uppercase().starts_with("RGB") {
            continue;
        }
        colors.push(rgb_triplet_to_u8(&parse_rgb_values(line)?)?);
    }
    anyhow::ensure!(colors.len() == 64, "slot palette must contain 64 colors");
    Ok(colors
        .chunks_exact(4)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect())
}

fn slot_machine_background_palette(tile_x: usize, tile_y: usize) -> usize {
    let mut palette = 0;
    for (x, y, width, height, value) in [
        (0, 2, 3, 10, 2),
        (17, 2, 3, 10, 2),
        (0, 4, 3, 6, 3),
        (17, 4, 3, 6, 3),
        (0, 6, 3, 2, 4),
        (17, 6, 3, 2, 4),
        (4, 2, 12, 2, 1),
        (3, 2, 1, 10, 1),
        (16, 2, 1, 10, 1),
    ] {
        if tile_x >= x && tile_x < x + width && tile_y >= y && tile_y < y + height {
            palette = value;
        }
    }
    if (12..18).contains(&tile_y) {
        7
    } else {
        palette
    }
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
            if transparent_zero && palette_index == 0 {
                continue;
            }
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
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(puzzle) = runtime_shell.visible_unown_puzzle.as_ref() else {
        return Ok(());
    };
    if rendered_art.unown_puzzle_sources.is_none()
        && rendered_art.unown_puzzle_source_error.is_none()
    {
        match load_unown_puzzle_render_sources(asset_root) {
            Ok(sources) => rendered_art.unown_puzzle_sources = Some(sources),
            Err(error) => rendered_art.unown_puzzle_source_error = Some(format!("{error:#}")),
        }
    }
    if let Some(error) = rendered_art.unown_puzzle_source_error.as_deref() {
        anyhow::bail!(error.to_string());
    }
    let sources = rendered_art
        .unown_puzzle_sources
        .as_ref()
        .context("Unown puzzle render sources are unavailable")?;
    let frame = render_visible_unown_puzzle_frame(sources, puzzle, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        3.6,
        images,
    )?;
    Ok(())
}

fn spawn_visible_unown_printer(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(printer) = runtime_shell.visible_unown_printer.as_ref() else {
        return Ok(());
    };
    anyhow::ensure!(
        !printer.letters.is_empty(),
        "Unown Printer is visible without any registered forms"
    );
    commit_presented_fullscreen_solid(commands, rendered_art, [247, 247, 247, 255], 3.4, images)?;
    let frame = battle_window_frame_art(rendered_art, asset_root, images)
        .context("Unown Printer window frame art is unavailable")?;
    // These are the three Textbox calls in _UnownPrinter: a full-width title,
    // the 7x7 front-picture box, and the full-width prompt at the bottom.
    spawn_field_command_window_frame_tiles(commands, frame, 0.0, 0.0, 20, 5, 3.5);
    spawn_field_command_window_frame_tiles(commands, frame, 0.0, 5.0, 9, 9, 3.5);
    spawn_field_command_window_frame_tiles(commands, frame, 0.0, 14.0, 20, 4, 3.5);

    for (text, tile_x, tile_y) in [
        (" ALPH RUINS STAMP", 1.0, 2.0),
        (" PRINT", 11.0, 6.0),
        (" CANCEL", 11.0, 7.0),
        ("← PREVIOUS", 10.0, 8.0),
        ("→ NEXT", 10.0, 9.0),
        ("Do what?", 1.0, 16.0),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            text,
            x,
            y,
            3.7,
        );
    }
    spawn_unown_printer_button(
        commands,
        images,
        asset_root.resolve_vendor("gfx/printer/bold_a.png"),
        10.0,
        6.0,
    )?;
    spawn_unown_printer_button(
        commands,
        images,
        asset_root.resolve_vendor("gfx/printer/bold_b.png"),
        10.0,
        7.0,
    )?;

    if printer.selected < 26 {
        let letter = char::from(b'A' + printer.selected);
        if let Some(frontpic) = pokemon_frame_for_art(
            rendered_art,
            asset_root,
            &format!("UNOWN_{letter}"),
            PokemonSpriteSide::Front,
            false,
            images,
        ) {
            let (x, y) = battle_hud_tile_origin(4.0, 9.0);
            commands.spawn((
                SpriteBundle {
                    texture: frontpic.handle,
                    sprite: Sprite {
                        custom_size: Some(frontpic.size),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, 3.7),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
    } else {
        let (x, y) = battle_hud_tile_origin(1.0, 9.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "VACANT",
            x,
            y,
            3.7,
        );
    }
    Ok(())
}

fn spawn_unown_printer_button(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    path: PathBuf,
    tile_x: f32,
    tile_y: f32,
) -> Result<()> {
    let source = crate::open_runtime_image(&path)
        .with_context(|| format!("decode Unown Printer button {}", path.display()))?
        .to_rgba8();
    anyhow::ensure!(
        source.dimensions() == (8, 8),
        "invalid Unown Printer button dimensions for {}",
        path.display()
    );
    let mut image = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        source.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    commands.spawn((
        SpriteBundle {
            texture: images.add(image),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(8.0)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 3.7),
            ..default()
        },
        FieldCommandMarker,
    ));
    Ok(())
}

fn load_unown_puzzle_render_sources(asset_root: &AssetRoot) -> Result<UnownPuzzleRenderSources> {
    const TILE: usize = 8;
    const PIECE: usize = 24;
    let root = asset_root.resolve_vendor("gfx/unown_puzzle");
    let border = crate::open_runtime_image(root.join("tile_borders.png"))
        .context("decode Unown puzzle piece borders")?
        .to_rgba8();
    let mut cursor = crate::open_runtime_image(root.join("cursor.png"))
        .context("decode Unown puzzle cursor")?
        .to_rgba8();
    let start_cancel = crate::open_runtime_image(root.join("start_cancel.png"))
        .context("decode Unown puzzle START/CANCEL graphics")?
        .to_rgba8();
    anyhow::ensure!(
        border.dimensions() == (64, 8),
        "invalid Unown border dimensions"
    );
    anyhow::ensure!(
        cursor.dimensions() == (16, 16),
        "invalid Unown cursor dimensions"
    );
    anyhow::ensure!(
        start_cancel.dimensions() == (152, 8),
        "invalid Unown START/CANCEL dimensions"
    );
    // OBJ palette colour zero is transparent on the Game Boy. The source PNG
    // represents it as opaque white because PNG has no OBJ/BG distinction.
    for pixel in cursor.pixels_mut() {
        if pixel[0] >= 248 && pixel[1] >= 248 && pixel[2] >= 248 {
            pixel[3] = 0;
        }
    }

    let mut puzzle_pieces = HashMap::new();
    for puzzle_id in ["aerodactyl", "hooh", "kabuto", "omanyte"] {
        let puzzle_path = root.join(format!("{puzzle_id}.png"));
        let source = crate::open_runtime_image(&puzzle_path)
            .with_context(|| format!("decode Unown puzzle PNG {}", puzzle_path.display()))?
            .to_rgba8();
        anyhow::ensure!(
            source.dimensions() == (48, 48),
            "invalid Unown puzzle dimensions for {puzzle_id}"
        );
        let mut pieces = Vec::with_capacity(16);
        for piece_index in 0..16 {
            let mut piece = image::RgbaImage::new(PIECE as u32, PIECE as u32);
            let source_x = (piece_index % 4) * 12;
            let source_y = (piece_index / 4) * 12;
            for y in 0..PIECE {
                for x in 0..PIECE {
                    piece.put_pixel(
                        x as u32,
                        y as u32,
                        *source.get_pixel((source_x + x / 2) as u32, (source_y + y / 2) as u32),
                    );
                }
            }
            for (border_index, (x, y)) in [
                (0, 0),
                (8, 0),
                (16, 0),
                (0, 8),
                (16, 8),
                (0, 16),
                (8, 16),
                (16, 16),
            ]
            .into_iter()
            .enumerate()
            {
                overlay_unown_border(&border, border_index * TILE, &mut piece, x, y);
            }
            pieces.push(piece);
        }
        puzzle_pieces.insert(puzzle_id.to_string(), pieces);
    }
    Ok(UnownPuzzleRenderSources {
        pieces: puzzle_pieces,
        cursor,
        start_cancel,
    })
}

fn render_visible_unown_puzzle_frame(
    sources: &UnownPuzzleRenderSources,
    puzzle: &VisibleUnownPuzzle,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const TILE: usize = 8;
    const PIECE: usize = 24;
    let puzzle_id = puzzle.puzzle_id.to_ascii_lowercase();
    let pieces = sources
        .pieces
        .get(&puzzle_id)
        .with_context(|| format!("unknown Unown puzzle art {puzzle_id}"))?;

    let mut target = vec![248_u8; WIDTH * HEIGHT * 4];
    for pixel in target.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    let draw_tile = |tile: usize, x: usize, y: usize, target: &mut [u8]| {
        blit_unown_rgba(
            &sources.start_cancel,
            tile * TILE,
            0,
            TILE,
            TILE,
            x,
            y,
            false,
            false,
            target,
        );
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

    for y in 0..6 {
        for x in 0..6 {
            let dest_x = TILE + x * PIECE;
            let dest_y = y * PIECE;
            match puzzle.layout[y][x] {
                1..=16 => blit_unown_rgba(
                    &pieces[usize::from(puzzle.layout[y][x] - 1)],
                    0,
                    0,
                    PIECE,
                    PIECE,
                    dest_x,
                    dest_y,
                    false,
                    false,
                    &mut target,
                ),
                _ => {
                    let vacant_tile = if (1..=4).contains(&x) && (1..=4).contains(&y) {
                        2
                    } else {
                        1
                    };
                    for block_y in 0..3 {
                        for block_x in 0..3 {
                            draw_tile(
                                vacant_tile,
                                dest_x + block_x * TILE,
                                dest_y + block_y * TILE,
                                &mut target,
                            );
                        }
                    }
                }
            }
        }
    }

    // Source tilemap box at (4,15), including the exact START/CANCEL lettering.
    draw_tile(3, 32, 120, &mut target);
    for x in 5..15 {
        draw_tile(4, x * TILE, 120, &mut target);
    }
    draw_tile(5, 120, 120, &mut target);
    draw_tile(6, 32, 128, &mut target);
    if !puzzle.solved {
        for offset in 0..10 {
            draw_tile(9 + offset, (5 + offset) * TILE, 128, &mut target);
        }
    }
    draw_tile(6, 120, 128, &mut target);
    draw_tile(7, 32, 136, &mut target);
    for x in 5..15 {
        draw_tile(4, x * TILE, 136, &mut target);
    }
    draw_tile(8, 120, 136, &mut target);

    let cursor_x = TILE + puzzle.cursor_x * PIECE;
    let cursor_y = puzzle.cursor_y * PIECE;
    if let Some(piece) = puzzle
        .holding_piece
        .filter(|piece| (1..=16).contains(piece))
    {
        blit_unown_rgba(
            &pieces[usize::from(piece - 1)],
            0,
            0,
            PIECE,
            PIECE,
            cursor_x,
            cursor_y,
            false,
            false,
            &mut target,
        );
    } else {
        let cursor_tiles = [
            (0, false, false),
            (1, false, false),
            (0, true, false),
            (2, false, false),
            (3, false, false),
            (2, true, false),
            (0, false, true),
            (1, false, true),
            (0, true, true),
        ];
        for (index, (tile, flip_x, flip_y)) in cursor_tiles.into_iter().enumerate() {
            blit_unown_rgba(
                &sources.cursor,
                (tile % 2) * TILE,
                (tile / 2) * TILE,
                TILE,
                TILE,
                cursor_x + (index % 3) * TILE,
                cursor_y + (index / 3) * TILE,
                flip_x,
                flip_y,
                &mut target,
            );
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        target,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(WIDTH as f32, HEIGHT as f32),
    })
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
            for channel in 0..3 {
                base[channel] = base[channel].min(overlay[channel]);
            }
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
            if dx >= TARGET_WIDTH || dy >= TARGET_HEIGHT {
                continue;
            }
            let pixel = source.get_pixel(sx as u32, sy as u32);
            if pixel[3] == 0 {
                continue;
            }
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
                color: Color::WHITE,
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
    let first = selected
        .saturating_sub(3)
        .min(choices.len().saturating_sub(4));
    for (row, (index, (item_id, quantity))) in
        choices.iter().enumerate().skip(first).take(4).enumerate()
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
    let scripted_capture_entry =
        runtime_shell.pokedex_scripted_entry && runtime_shell.pending_standard_capture.is_some();
    let seen = scripted_capture_entry
        || snapshot
            .progression
            .pokedex_seen_species
            .contains(&species.species_id);
    let caught = scripted_capture_entry
        || snapshot
            .progression
            .pokedex_caught_species
            .contains(&species.species_id);
    commit_presented_fullscreen_solid(commands, rendered_art, [31, 46, 61, 255], 3.4, images)?;
    if runtime_shell.pokedex_detail_open && seen {
        spawn_field_pokedex_detail(
            commands,
            snapshot,
            runtime_shell,
            species,
            caught,
            rendered_art,
            asset_root,
            images,
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
            commands,
            rendered_art,
            asset_root,
            images,
            &text,
            x,
            y,
            3.8,
        );
    }
    if seen {
        if let Some(frame) = pokemon_frame_for_art(
            rendered_art,
            asset_root,
            &species.species_id,
            PokemonSpriteSide::Front,
            false,
            images,
        ) {
            let (x, y) = battle_hud_tile_origin(3.5, 4.0);
            commands.spawn((
                SpriteBundle {
                    texture: frame.handle,
                    sprite: Sprite {
                        custom_size: Some(frame.size),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, 3.8),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
    } else {
        let (x, y) = battle_hud_tile_origin(3.0, 4.0);
        spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, "?", x, y, 3.8);
    }
    let scroll = visible_window_start(selected, snapshot.pokemon.len(), 7);
    for (visible_index, entry) in snapshot.pokemon.iter().skip(scroll).take(7).enumerate() {
        let index = scroll + visible_index;
        let entry_seen = snapshot
            .progression
            .pokedex_seen_species
            .contains(&entry.species_id);
        let entry_caught = snapshot
            .progression
            .pokedex_caught_species
            .contains(&entry.species_id);
        let name = if entry_seen {
            crate::core::models::pokemon_species_display_name(&entry.species_id)
        } else {
            "-----".to_string()
        };
        let (x, y) = battle_hud_tile_origin(9.0, 1.0 + visible_index as f32 * 2.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{}{:03} {}",
                if index == selected { ">" } else { " " },
                if entry_caught { "C" } else { " " },
                entry.int_id,
                compact_scene_label(&name, 8)
            ),
            x,
            y,
            3.8,
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
        // Standard furniture specials (TownMapScript/MapRadio) use these
        // visible pages before the corresponding portable Pokégear card is
        // unlocked. Normal Pokégear navigation already filters locked pages.
        PokegearPage::Map => {}
        PokegearPage::Phone => anyhow::ensure!(
            snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_PHONE_CARD"),
            "Pokégear PHONE card is selected before it is unlocked"
        ),
        PokegearPage::Radio => {}
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
            if !transcript.is_empty() {
                let label = transcript
                    .get(runtime_shell.pokegear_radio_segment)
                    .with_context(|| {
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
                    .with_context(|| {
                        format!("Pokégear radio transcript text {label} is missing")
                    })?;
            }
        } else {
            anyhow::ensure!(
                runtime_shell.pokegear_radio_tuning_knob <= 80
                    && runtime_shell.pokegear_radio_tuning_knob % 2 == 0,
                "Pokégear radio tuning knob {} is outside the even range 0..=80",
                runtime_shell.pokegear_radio_tuning_knob,
            );
        }
    }
    if runtime_shell.pokegear_page != PokegearPage::Map {
        commit_presented_fullscreen_solid(
            commands,
            rendered_art,
            [230, 209, 140, 255],
            3.4,
            images,
        )?;
    }
    let cards = [
        (PokegearPage::Clock, "CLOCK"),
        (PokegearPage::Map, "MAP"),
        (PokegearPage::Phone, "PHONE"),
        (PokegearPage::Radio, "RADIO"),
    ];
    for (index, (page, label)) in cards
        .iter()
        .enumerate()
        .filter(|_| runtime_shell.pokegear_page != PokegearPage::Map)
    {
        let unlocked = match page {
            PokegearPage::Clock => true,
            PokegearPage::Map => snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_MAP_CARD"),
            PokegearPage::Phone => snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_PHONE_CARD"),
            PokegearPage::Radio => snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_RADIO_CARD"),
        };
        if !unlocked {
            continue;
        }
        let (x, y) = battle_hud_tile_origin(1.0 + index as f32 * 4.5, 0.5);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{}",
                if *page == runtime_shell.pokegear_page {
                    ">"
                } else {
                    " "
                },
                label
            ),
            x,
            y,
            3.8,
        );
    }
    if runtime_shell.pokegear_page == PokegearPage::Map {
        let region = visible_pokegear_region(snapshot)?;
        let tile_palettes = snapshot
            .presentation
            .pokegear_town_map_palette_map
            .get("town_map")
            .context("compiled pack has no Town Map palette assignment")?;
        let frame = town_map_frame_for_art(
            rendered_art,
            asset_root,
            region,
            snapshot.trainer.player_gender,
            tile_palettes,
            images,
        )
        .with_context(|| {
            let key = (region.to_ascii_lowercase(), snapshot.trainer.player_gender);
            format!(
                "render Town Map: {}",
                rendered_art
                    .town_map_errors
                    .get(&key)
                    .map(String::as_str)
                    .unwrap_or("Town Map art is unavailable")
            )
        })?;
        commit_presented_fullscreen_frame(
            commands,
            rendered_art,
            &frame,
            PresentedFullscreenFrameSource::Cached,
            3.4,
            images,
        )?;
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
        for index in visible_pokegear_landmark_indices(snapshot)? {
            let landmark = &landmarks[index];
            let x = (landmark.x as f32 - 8.0).clamp(0.0, 159.0);
            let y = (landmark.y as f32 - 16.0).clamp(0.0, 143.0);
            let is_current = snapshot
                .presentation
                .pokegear_landmarks
                .map_to_landmark
                .get(&snapshot.overworld.map_name)
                .is_some_and(|id| id == &landmark.constant);
            let is_selected = index == selected;
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: if is_selected {
                            Color::srgb(0.95, 0.12, 0.12)
                        } else if is_current {
                            Color::srgb(0.15, 0.35, 0.95)
                        } else {
                            Color::srgb(0.35, 0.55, 0.25)
                        },
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
    if runtime_shell.pokegear_page != PokegearPage::Map {
        let entries = visible_pokegear_menu_entries(snapshot, runtime_shell)?;
        for (index, line) in entries.iter().take(10).enumerate() {
            let (x, y) = battle_hud_tile_origin(1.0, 4.0 + index as f32);
            spawn_field_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &compact_scene_label(line, 18),
                x,
                y,
                3.8,
            );
        }
    }
    Ok(())
}

/// Render the same 20x18 Johto tilemap and palette-selected town-map tiles
/// used by `TownMapOverlay` and `_TownMap`. The furniture special is a bare
/// town map, not a fabricated Pokégear card menu on a flat backdrop.
fn visible_overworld_town_map_frame(
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let gfx = assets.join("gfx/pokegear");
    let sheet_path = gfx.join("town_map.png");
    let sheet = crate::open_runtime_image(&sheet_path)
        .with_context(|| format!("decode town-map tiles {}", sheet_path.display()))?
        .to_rgba8();
    anyhow::ensure!(
        sheet.width() % 8 == 0 && sheet.height() % 8 == 0,
        "town-map tiles {} are not 8x8 aligned",
        sheet_path.display()
    );
    let mut tilemap = crate::read_runtime_asset(gfx.join("johto.bin"))
        .context("read canonical Johto town-map tilemap")?;
    if tilemap.last() == Some(&0xff) {
        tilemap.pop();
    }
    anyhow::ensure!(
        tilemap.len() == 20 * 18,
        "Johto town-map tilemap must contain 360 tiles"
    );
    let palette_tokens: serde_json::Value = serde_json::from_slice(
        &crate::read_runtime_asset(assets.join("data/pokegear_town_map_palette_map.json"))
            .context("read town-map palette assignments")?,
    )
    .context("decode town-map palette assignments")?;
    let tokens = palette_tokens["town_map"]
        .as_array()
        .context("town-map palette assignments lack town_map")?;
    let palettes = parse_visible_pokegear_palette_bank(&gfx.join("pokegear.pal"))?;
    let columns = sheet.width() / 8;
    let mut pixels = vec![0_u8; 160 * 144 * 4];
    for (map_index, tile_id) in tilemap.into_iter().enumerate() {
        let tile_index = usize::from(tile_id);
        let token = tokens
            .get(tile_index)
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("town-map tile {tile_index} lacks a palette token"))?;
        let palette_key = match token {
            "BORDER" => "border",
            "EARTH" => "earth",
            "MOUNTAIN" => "mountain",
            "CITY" => "city",
            "POI" => "point_of_interest",
            "POI_MTN" => "mountain_point_of_interest",
            other => anyhow::bail!("unknown town-map palette token {other}"),
        };
        let palette = palettes
            .get(palette_key)
            .with_context(|| format!("Pokégear palette {palette_key} is missing"))?;
        let source_x = u32::from(tile_id) % columns * 8;
        let source_y = u32::from(tile_id) / columns * 8;
        anyhow::ensure!(
            source_y + 8 <= sheet.height(),
            "town-map tile {tile_id} is outside its sheet"
        );
        let target_x = map_index % 20 * 8;
        let target_y = map_index / 20 * 8;
        for row in 0..8_usize {
            for col in 0..8_usize {
                let gray = sheet.get_pixel(source_x + col as u32, source_y + row as u32)[0];
                // Match TypeScript's PNG -> Game Boy level inversion before
                // applying the four-color CGB palette.
                let level = if gray >= 192 {
                    0
                } else if gray >= 128 {
                    1
                } else if gray >= 64 {
                    2
                } else {
                    3
                };
                let target = ((target_y + row) * 160 + target_x + col) * 4;
                pixels[target..target + 3].copy_from_slice(&palette[level]);
                pixels[target + 3] = 255;
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: 160,
            height: 144,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(160.0, 144.0),
    })
}

fn parse_visible_pokegear_palette_bank(
    path: &std::path::Path,
) -> Result<HashMap<String, [[u8; 3]; 4]>> {
    let source = crate::read_runtime_asset_to_string(path)
        .with_context(|| format!("read Pokégear palette {}", path.display()))?;
    let mut palettes = HashMap::new();
    let mut label = None;
    let mut colors = Vec::new();
    for raw in source.lines() {
        let line = raw.trim();
        if let Some(comment) = line.strip_prefix(';') {
            label = Some(
                comment
                    .trim()
                    .to_ascii_lowercase()
                    .replace(" (boy)", "")
                    .replace(' ', "_"),
            );
            colors.clear();
        } else if let Some(rgb) = line.strip_prefix("RGB") {
            let values = rgb
                .split(',')
                .map(|value| value.trim().parse::<u8>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            anyhow::ensure!(values.len() == 3, "malformed Pokégear RGB line {line}");
            colors.push([
                values[0] << 3 | values[0] >> 2,
                values[1] << 3 | values[1] >> 2,
                values[2] << 3 | values[2] >> 2,
            ]);
            if colors.len() == 4 {
                let key = label
                    .take()
                    .context("Pokégear RGB values have no palette label")?;
                palettes.insert(key, [colors[0], colors[1], colors[2], colors[3]]);
            }
        }
    }
    Ok(palettes)
}

fn spawn_field_fly_map_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    commit_presented_fullscreen_solid(commands, rendered_art, [173, 209, 148, 255], 3.4, images)?;
    let destinations = active_fly_destinations(snapshot, &runtime_shell.shell)?;
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
                    color: if is_selected {
                        Color::srgb(0.95, 0.12, 0.12)
                    } else {
                        Color::srgb(0.18, 0.42, 0.82)
                    },
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
        commands,
        rendered_art,
        asset_root,
        images,
        &format!(">{}", compact_scene_label(&label, 17)),
        x,
        y,
        3.8,
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
        rendered_art,
        asset_root,
        &species.species_id,
        PokemonSpriteSide::Front,
        false,
        images,
    ) {
        let (x, y) = battle_hud_tile_origin(3.5, 4.0);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 3.8),
                ..default()
            },
            FieldCommandMarker,
        ));
    }
    let height = if caught {
        entry.height_digits.to_string()
    } else {
        "????".to_string()
    };
    let weight = if caught {
        entry.weight_digits.to_string()
    } else {
        "????".to_string()
    };
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
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(&text, 11),
            x,
            y,
            3.8,
        );
    }
    let page_index = runtime_shell.pokedex_detail_page;
    let page = entry
        .pages
        .get(page_index)
        .map(String::as_str)
        .with_context(|| {
            format!(
                "Pokédex detail page {page_index} is outside {} pages for {}",
                entry.pages.len(),
                species.species_id
            )
        })?;
    for (index, line) in wrap_boot_text_for_box(page, 18, 5).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 11.0 + index as f32);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            line,
            x,
            y,
            3.8,
        );
    }
    if entry.pages.len() > 1 {
        let (x, y) = battle_hud_tile_origin(14.0, 16.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{}/{}", page_index + 1, entry.pages.len()),
            x,
            y,
            3.8,
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
    let (items, cursor, surface_id): (Vec<(String, u16)>, &Option<MenuCursor>, String) =
        match &pocket {
            FieldPackPocket::Items => (
                snapshot
                    .bag
                    .items
                    .iter()
                    .filter(|item| item.quantity > 0)
                    .map(|item| (item.item_id.clone(), item.quantity))
                    .collect(),
                &runtime_shell.bag_cursor,
                "bag:items".to_string(),
            ),
            FieldPackPocket::Balls => (
                snapshot
                    .bag
                    .balls
                    .iter()
                    .filter(|item| item.quantity > 0)
                    .map(|item| (item.item_id.clone(), item.quantity))
                    .collect(),
                &runtime_shell.ball_cursor,
                "bag:balls".to_string(),
            ),
            FieldPackPocket::KeyItems => (
                snapshot
                    .bag
                    .key_items
                    .iter()
                    .filter(|item| item.quantity > 0)
                    .map(|item| (item.item_id.clone(), item.quantity))
                    .collect(),
                &runtime_shell.key_item_cursor,
                "bag:key-items".to_string(),
            ),
            FieldPackPocket::TmHm => (
                snapshot
                    .bag
                    .tm_hm
                    .iter()
                    .filter(|item| item.quantity > 0)
                    .map(|item| (item.item_id.clone(), item.quantity))
                    .collect(),
                &runtime_shell.tmhm_cursor,
                "bag:tmhm".to_string(),
            ),
            FieldPackPocket::Custom(pocket_id) => (
                snapshot
                    .bag
                    .custom_pockets
                    .get(pocket_id)
                    .into_iter()
                    .flatten()
                    .filter(|item| item.quantity > 0)
                    .map(|item| (item.item_id.clone(), item.quantity))
                    .collect(),
                &runtime_shell.custom_item_cursor,
                custom_pack_surface_id(pocket_id),
            ),
        };
    let row_count = field_pack_selectable_count(items.len());
    let selected =
        strict_readonly_cursor_index(cursor, &surface_id, row_count).with_context(|| {
            format!("field PACK cursor is invalid for {surface_id} with {row_count} rows")
        })?;
    let list_start = visible_window_start(selected, row_count, 7);
    let description = if let Some((item_id, _)) = items.get(selected) {
        snapshot
            .items
            .iter()
            .find(|item| item.item_id == *item_id)
            .with_context(|| format!("field PACK item {item_id} is missing"))?
            .description
            .as_str()
    } else {
        ""
    };
    let frame = load_visible_field_pack_frame(
        snapshot,
        runtime_shell,
        &pocket,
        &items,
        selected,
        list_start,
        description,
        images,
    )?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        3.4,
        images,
    )?;
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
            commands,
            rendered_art,
            asset_root,
            images,
            1.0,
            10.0,
            18.0,
            8.0,
            4.5,
        );
        for (index, line) in wrap_boot_text_for_box(&visible_notice, 16, 6)
            .iter()
            .enumerate()
        {
            let (x, y) = battle_hud_tile_origin(2.0, 11.0 + index as f32);
            spawn_field_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                line,
                x,
                y,
                4.8,
            );
        }
        if visible_field_text_reveal_is_complete_for_text(runtime_shell, notice)
            && visible_field_notice_uses_prompt_arrow(runtime_shell)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_field_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "▼",
                x,
                y,
                4.8,
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
    let slot = snapshot
        .party
        .slots
        .get(runtime_shell.party_cursor)
        .with_context(|| {
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
        [255, 240, 199, 255]
    } else {
        match page {
            1 => [255, 219, 227, 255],
            2 => [214, 245, 214, 255],
            _ => [214, 232, 255, 255],
        }
    };
    let mut rows = vec![
        (
            8,
            0.0,
            format!(
                "No.{:03}  L{:>2}",
                pokemon.species.int_id, pokemon.level
            ),
        ),
        (8, 2.0, compact_scene_label(&pokemon.nickname, 10)),
        (
            9,
            4.0,
            format!(
                "/{}",
                crate::core::models::pokemon_species_display_name(&pokemon.species.id)
            ),
        ),
    ];
    if pokemon.is_egg {
        rows.push((8, 1.0, "EGG".to_string()));
        let hatch_lines = if pokemon.happiness < 6 {
            [
                "It's making sounds",
                "inside. It's going",
                "to hatch soon!",
                "",
            ]
        } else if pokemon.happiness < 11 {
            [
                "It moves around",
                "inside sometimes.",
                "It must be close",
                "to hatching.",
            ]
        } else if pokemon.happiness < 41 {
            [
                "Wonder what's",
                "inside? It needs",
                "more time, though.",
                "",
            ]
        } else {
            ["This EGG needs a", "lot more time to", "hatch.", ""]
        };
        rows.extend(
            hatch_lines
                .into_iter()
                .enumerate()
                .filter(|(_, line)| !line.is_empty())
                .map(|(index, line)| (1, 9.0 + index as f32 * 2.0, line.to_string())),
        );
    } else {
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
                let type1 = species.type1.clone();
                let type2 = (species.type1 != species.type2).then(|| species.type2.clone());
                rows.extend([
                    (0, 9.0, "HP".to_string()),
                    (1, 10.0, format!("{:>3}/{:>3}", pokemon.hp, pokemon.max_hp)),
                    (0, 12.0, "STATUS/".to_string()),
                    (6, 13.0, party_status_token(pokemon).to_string()),
                    (0, 14.0, "TYPE/".to_string()),
                    (1, 15.0, type1),
                    (1, 16.0, type2.unwrap_or_default()),
                    (10, 9.0, "EXP POINTS".to_string()),
                    (13, 10.0, format!("{:>7}", pokemon.experience.max(0))),
                ]);
            }
            2 => {
                rows.push((0, 8.0, "ITEM".to_string()));
                rows.push((8, 8.0, held_item_label));
                if pokemon.moves.is_empty() {
                    rows.push((8, 10.0, "NO MOVES".to_string()));
                } else {
                    for (index, learned) in pokemon.moves.iter().take(4).enumerate() {
                        let row = 8.0 + index as f32 * 2.0;
                        rows.push((8, row + 2.0, battle_move_display_name(snapshot, &learned.name)));
                        rows.push((12, row + 3.0, visible_move_pp_text(snapshot, learned)));
                    }
                }
            }
            _ => rows.extend([
                (0, 9.0, format!("IDNo.{:05}", pokemon.original_trainer_id)),
                (0, 12.0, format!("OT/{}", pokemon.original_trainer_name)),
                (11, 8.0, "ATTACK".to_string()),
                (17, 9.0, format!("{:>3}", pokemon.attack)),
                (11, 10.0, "DEFENSE".to_string()),
                (17, 11.0, format!("{:>3}", pokemon.defense)),
                (11, 12.0, "SPCL.ATK".to_string()),
                (17, 13.0, format!("{:>3}", pokemon.special_attack)),
                (11, 14.0, "SPCL.DEF".to_string()),
                (17, 15.0, format!("{:>3}", pokemon.special_defense)),
                (11, 16.0, "SPEED".to_string()),
                (17, 17.0, format!("{:>3}", pokemon.speed)),
            ]),
        }
    }
    let frame = load_visible_party_summary_frame(
        runtime_shell,
        &rows,
        tint,
        page,
        (!pokemon.is_egg).then_some((pokemon.hp, pokemon.max_hp)),
        images,
    )?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        3.4,
        images,
    )?;
    let sprite_species = if pokemon.is_egg { "EGG" } else { &pokemon.species.id };
    let shiny = !pokemon.is_egg && visible_pokemon_is_shiny(pokemon);
    let sprite = pokemon_frame_for_art(
        rendered_art,
        asset_root,
        sprite_species,
        PokemonSpriteSide::Front,
        shiny,
        images,
    )
    .with_context(|| format!("party summary front sprite {sprite_species} is unavailable"))?;
    let (x, y) = battle_hud_tile_origin(3.5, 3.5);
    commands.spawn((
        SpriteBundle {
            texture: sprite.handle,
            sprite: Sprite { custom_size: Some(sprite.size * 4.0), ..default() },
            transform: Transform::from_xyz(x, y, 3.8),
            ..default()
        },
        FieldCommandMarker,
    ));
    Ok(())
}

fn spawn_visible_egg_hatch_tile(
    commands: &mut Commands,
    tile: &SpriteFrame,
    screen_x: f32,
    screen_y: f32,
    flip_x: bool,
    flip_y: bool,
    z: f32,
) {
    let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    commands.spawn((
        SpriteBundle {
            texture: tile.handle.clone(),
            sprite: Sprite {
                custom_size: Some(tile.size * source_scale),
                flip_x,
                flip_y,
                ..default()
            },
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + (screen_x - 4.0) * source_scale,
                PLAYFIELD_TOP - (screen_y - 12.0) * source_scale,
                z,
            ),
            ..default()
        },
        FieldCommandMarker,
    ));
}

fn visible_egg_hatch_tiles<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a [SpriteFrame; 2]> {
    if rendered_art.egg_hatch_tile_cache.is_none()
        && rendered_art.egg_hatch_tile_error.is_none()
    {
        let loaded = (|| -> Result<[SpriteFrame; 2]> {
            let bytes = crate::read_runtime_asset(
                asset_root.runtime_assets().join("gfx/evo/egg_hatch.2bpp"),
            )
            .context("read exact egg hatch crack/fragment graphics")?;
            if bytes.len() != 32 {
                anyhow::bail!("egg_hatch.2bpp must contain exactly two tiles");
            }
            let mut frames = Vec::with_capacity(2);
            for tile_index in 0..2 {
                let tile = &bytes[tile_index * 16..tile_index * 16 + 16];
                let mut rgba = vec![0_u8; 8 * 8 * 4];
                for y in 0..8_usize {
                    for x in 0..8_usize {
                        let bit = 1 << (7 - x);
                        let level = (((tile[y * 2 + 1] & bit != 0) as u8) << 1)
                            | (tile[y * 2] & bit != 0) as u8;
                        if level == 0 {
                            continue;
                        }
                        let shade = [255_u8, 170, 85, 0][usize::from(level)];
                        let offset = (y * 8 + x) * 4;
                        rgba[offset..offset + 3].fill(shade);
                        rgba[offset + 3] = 255;
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width: 8,
                        height: 8,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    rgba,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                frames.push(SpriteFrame {
                    handle: images.add(image),
                    size: Vec2::splat(8.0),
                });
            }
            frames
                .try_into()
                .map_err(|_| anyhow::anyhow!("egg hatch tile decode count drifted"))
        })();
        match loaded {
            Ok(frames) => rendered_art.egg_hatch_tile_cache = Some(frames),
            Err(error) => rendered_art.egg_hatch_tile_error = Some(error.to_string()),
        }
    }
    rendered_art.egg_hatch_tile_cache.as_ref().with_context(|| {
        rendered_art
            .egg_hatch_tile_error
            .clone()
            .unwrap_or_else(|| "egg hatch graphics are unavailable".to_string())
    })
}

fn spawn_field_move_reorder_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let slot = snapshot
        .party
        .slots
        .get(runtime_shell.party_cursor)
        .with_context(|| {
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
                color: Color::srgb(1.0, 1.0, 1.0),
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
        commands,
        rendered_art,
        asset_root,
        images,
        &format!(
            "< {} \u{e10a}{:>2} >",
            compact_scene_label(&slot.pokemon.nickname, 10),
            slot.pokemon.level
        ),
        x,
        y,
        3.8,
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
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{marker}{}",
                battle_move_display_name(snapshot, &learned.name)
            ),
            x,
            y,
            3.8,
        );
        let (x, y) = battle_hud_tile_origin(10.0, row + 1.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &visible_move_pp_text(snapshot, learned),
            x,
            y,
            3.8,
        );
    }
    let selected_move = &slot.pokemon.moves[selected];
    if runtime_shell.party_move_reorder_origin.is_some() {
        let (x, y) = battle_hud_tile_origin(1.0, 12.0);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "Where?",
            x,
            y,
            3.8,
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
        commands,
        rendered_art,
        asset_root,
        images,
        &format!("TYPE/{move_type}"),
        x,
        y,
        3.8,
    );
    let (x, y) = battle_hud_tile_origin(12.0, 12.0);
    spawn_field_command_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &format!(
            "ATK/{:>3}",
            if power == 0 {
                "---".to_string()
            } else {
                power.to_string()
            }
        ),
        x,
        y,
        3.8,
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
            rendered_art
                .move_description_error
                .clone()
                .unwrap_or_else(|| format!("move description {} is missing", selected_move.name))
        })?;
    for (index, line) in wrap_boot_text_for_box(&description, 18, 3)
        .iter()
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(1.0, 14.0 + index as f32);
        spawn_field_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            line,
            x,
            y,
            3.8,
        );
    }
    Ok(())
}

fn load_asm_move_descriptions(
    asset_root: &AssetRoot,
    snapshot: &RuntimeShellSnapshot,
) -> Result<HashMap<String, String>> {
    let path = asset_root.resolve_vendor("data/moves/descriptions.asm");
    let content = crate::read_runtime_asset_to_string(&path)
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
            parts.push(
                line[first_quote + 1..last_quote]
                    .trim_end_matches('@')
                    .to_string(),
            );
        }
    }
    flush(&mut labels, &mut parts, &mut descriptions);
    let constants_path = asset_root.resolve_vendor("constants/move_constants.asm");
    let constants = crate::read_runtime_asset_to_string(&constants_path)
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
    let ordered_moves = move_order
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
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
                .with_context(|| format!("ASM move description {label} missing for {move_id}"))?;
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
    let frame = load_visible_field_party_frame(snapshot, runtime_shell, selected, images)?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        3.4,
        images,
    )?;
    for (row_index, slot) in snapshot.party.slots.iter().enumerate().take(6) {
        spawn_battle_party_icon(
            commands,
            snapshot,
            slot,
            row_index,
            selected == row_index,
            true,
            rendered_art,
            asset_root,
            images,
        )
        .context("field party selection has no valid icon")?;
    }
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
    let _ = (commands, snapshot, runtime_shell, rendered_art, asset_root, images);
    Ok(())
}

fn spawn_field_party_give_take_window(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let _ = (commands, runtime_shell, rendered_art, asset_root, images);
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
    let top_tile = if contest_active {
        2.0
    } else {
        START_MENU_TOP_TILE
    };
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
        spawn_bug_contest_start_menu_status(commands, snapshot, rendered_art, asset_root, images)?;
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
                color: Color::srgb(1.0, 1.0, 1.0),
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
                color: Color::srgb(1.0, 1.0, 1.0),
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
    spawn_field_command_window_frame_tiles(commands, frame, LEFT, TOP, WIDTH, HEIGHT, 3.4);
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
        rows.push(format!("LEVEL   {}", caught_level));
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
        spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, row, x, y, 3.6);
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
                color: Color::srgb(1.0, 1.0, 1.0),
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
    if runtime_shell.visible_egg_hatch.as_ref().is_some_and(|hatch| {
        hatch.phase != VisibleEggHatchPhase::HuhText
    }) {
        let hatch_text = runtime_shell
            .visible_egg_hatch
            .as_ref()
            .is_some_and(|hatch| hatch.phase == VisibleEggHatchPhase::HatchText);
        spawn_visible_egg_hatch(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        if !hatch_text {
            return Ok(());
        }
    }
    if let Some(word) = runtime_shell.visible_unown_words.as_deref() {
        spawn_visible_unown_words(commands, rendered_art, asset_root, images, word);
        return Ok(());
    }
    if runtime_shell.visible_magnet_train.is_some() {
        spawn_visible_magnet_train(commands, runtime_shell, rendered_art, asset_root, images)?;
        return Ok(());
    }
    spawn_visible_heal_machine(commands, runtime_shell, rendered_art, asset_root, images)?;
    spawn_visible_balance_overlay(commands, runtime_shell, rendered_art, asset_root, images);
    if runtime_shell.visible_mom_bank.as_ref().is_some_and(|bank| {
        bank.messages.is_empty()
            && matches!(
                bank.phase,
                VisibleMomBankPhase::Menu
                    | VisibleMomBankPhase::Withdraw
                    | VisibleMomBankPhase::Deposit
            )
    }) {
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
    if let Some(choice) = runtime_shell.pending_name_choice.as_ref() {
        spawn_visible_name_choice_screen(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
            choice,
        )?;
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
        )?;
        return Ok(());
    }
    if let Some(input) = runtime_shell.pending_mail_input.as_ref() {
        spawn_visible_mail_entry_screen(
            commands,
            rendered_art,
            asset_root,
            images,
            input,
        )?;
        return Ok(());
    }
    if let Some(reader) = runtime_shell.pending_mail_read.as_ref() {
        spawn_visible_mail_read_screen(
            commands,
            rendered_art,
            asset_root,
            images,
            reader,
        )?;
        return Ok(());
    }
    if let Some(shop) = snapshot.pending_shop.as_ref() {
        spawn_field_shop_screen(
            commands,
            snapshot,
            runtime_shell,
            shop,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.storage_cursor.is_some() && !runtime_shell.party_menu_open {
        spawn_field_storage_screen(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.pc_item_cursor.is_some() && !visible_field_pack_is_open(runtime_shell) {
        spawn_field_pc_item_screen(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.decoration_menu.is_some() {
        spawn_visible_decoration_screen(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        spawn_visible_player_pc_action_screen(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        spawn_field_pc_box_selection_screen(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
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

    if let Some(day_prompt) = runtime_shell.pending_day_of_week.as_ref() {
        if day_prompt.confirming {
            spawn_visible_day_of_week_confirmation(
                commands,
                snapshot,
                runtime_shell,
                rendered_art,
                asset_root,
                images,
            )?;
        } else {
            spawn_visible_day_of_week_selection(
                commands,
                runtime_shell,
                rendered_art,
                asset_root,
                images,
            );
        }
        return Ok(());
    }

    let entries = visible_scene_dialog_entries(snapshot, runtime_shell)?;
    if entries.is_empty() {
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

fn spawn_visible_egg_hatch(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let hatch = runtime_shell
        .visible_egg_hatch
        .as_ref()
        .context("egg hatch renderer requires active hatch state")?;
    commit_presented_fullscreen_solid(
        commands,
        rendered_art,
        [247, 247, 247, 255],
        5.8,
        images,
    )?;
    let (species_id, x_offset, animation_frame) = match hatch.phase {
        VisibleEggHatchPhase::EggHold => ("EGG", 0, 0),
        VisibleEggHatchPhase::Wobble => ("EGG", visible_egg_wobble_x(hatch.frame), 0),
        VisibleEggHatchPhase::Shell => (hatch.species_id.as_str(), 0, 0),
        VisibleEggHatchPhase::Reveal => (
            hatch.species_id.as_str(),
            0,
            runtime_shell
                .visible_frontpic_animation
                .as_ref()
                .map_or(0, |animation| animation.frame),
        ),
        VisibleEggHatchPhase::HatchText => (hatch.species_id.as_str(), 0, 0),
        VisibleEggHatchPhase::HuhText => return Ok(()),
    };
    let sprite = pokemon_animation_frame_for_art(
        rendered_art,
        asset_root,
        species_id,
        PokemonSpriteSide::Front,
        false,
        animation_frame,
        images,
    )
    .with_context(|| format!("egg hatch front sprite {species_id} is unavailable"))?;
    let (center_x, center_y) = battle_hud_tile_origin(10.0, 8.0);
    commands.spawn((
        SpriteBundle {
            texture: sprite.handle,
            sprite: Sprite {
                custom_size: Some(sprite.size * 4.0),
                ..default()
            },
            transform: Transform::from_xyz(
                center_x + f32::from(x_offset) * TILE_SIZE / SOURCE_TILE_SIZE as f32,
                center_y,
                6.0,
            ),
            ..default()
        },
        FieldCommandMarker,
    ));
    let tiles = visible_egg_hatch_tiles(rendered_art, asset_root, images)?;
    if hatch.phase == VisibleEggHatchPhase::Wobble {
        let crack_count = [50_u16, 124, 222]
            .into_iter()
            .filter(|boundary| hatch.frame >= *boundary)
            .count();
        for crack_index in 0..crack_count {
            spawn_visible_egg_hatch_tile(
                commands,
                &tiles[0],
                88.0,
                84.0 + crack_index as f32 * 16.0,
                false,
                false,
                6.2,
            );
        }
    }
    if hatch.phase == VisibleEggHatchPhase::Shell && hatch.frame < 16 {
        const FRAGMENTS: [(i16, i16, bool, bool, u8); 10] = [
            (84, 72, false, false, 0x3c),
            (92, 72, true, false, 0x04),
            (84, 80, false, false, 0x30),
            (92, 80, true, false, 0x10),
            (84, 88, false, true, 0x24),
            (92, 88, true, true, 0x1c),
            (80, 76, false, false, 0x36),
            (96, 76, true, false, 0x0a),
            (80, 84, false, true, 0x2a),
            (96, 84, true, true, 0x16),
        ];
        let amplitude = (hatch.frame as u8).wrapping_mul(8);
        for (base_y, base_x, flip_x, flip_y, angle) in FRAGMENTS {
            let phase = angle ^ if hatch.frame % 2 == 0 { 0x20 } else { 0 };
            let y_offset = visible_battle_anim_sine(phase, amplitude);
            let x_offset = visible_battle_anim_sine(phase.wrapping_add(0x10), amplitude);
            spawn_visible_egg_hatch_tile(
                commands,
                &tiles[1],
                f32::from(base_x) + x_offset as f32,
                f32::from(base_y) + y_offset as f32,
                flip_x,
                flip_y,
                6.3,
            );
        }
    }
    Ok(())
}

fn spawn_visible_player_pc_action_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    // PlayerPCMenu.drawActions and PlayersPCMenuData both define this as the
    // 16x13 window at (0,0). It is not a four-row field textbox: the house PC
    // must visibly expose all six canonical actions at once.
    const LEFT: f32 = 0.0;
    const TOP: f32 = 0.0;
    const WIDTH: f32 = 16.0;
    const HEIGHT: f32 = 13.0;
    let actions = visible_player_pc_actions(runtime_shell);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.player_pc_action_cursor,
        "pc:player-actions",
        actions.len(),
    )
    .context("Player PC action screen requires a valid cursor")?;
    let (center_x, center_y) = field_window_center(LEFT, TOP, WIDTH, HEIGHT);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (WIDTH - 2.0),
                    TILE_SIZE * (HEIGHT - 2.0),
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.0),
            ..default()
        },
        SceneDialogMarker,
        SceneDialogTextBoxBackgroundMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            LEFT,
            TOP,
            WIDTH as usize,
            HEIGHT as usize,
            4.05,
        );
    }
    for (index, action) in actions.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 2.0 + index as f32);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{}",
                if index == selected { ">" } else { " " },
                visible_player_pc_action_label(*action)
            ),
            x,
            y,
            4.2,
        );
    }
    Ok(())
}

fn spawn_visible_decoration_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let menu = runtime_shell
        .decoration_menu
        .as_ref()
        .context("decoration screen requires an active menu")?;
    if let VisibleDecorationMenuPhase::Side { decoration_id, .. } = &menu.phase {
        spawn_scene_dialog_text_box(commands, rendered_art, asset_root, images, 4.0);
        let (question_x, question_y) = battle_hud_tile_origin(1.0, 13.0);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            if decoration_id.is_some() {
                "Which side do you\nwant to put it on?"
            } else {
                "Which side do you\nwant to put away?"
            },
            question_x,
            question_y,
            4.2,
        );
    }
    let (left, top, width, height, entries, selected, surface_id, visible_rows) = match &menu.phase {
        VisibleDecorationMenuPhase::Categories { categories, cursor } => (
            5.0,
            0.0,
            15.0,
            18.0,
            categories
                .iter()
                .map(|category| visible_decoration_category_label(*category).to_string())
                .chain(std::iter::once("EXIT".to_string()))
                .collect::<Vec<_>>(),
            cursor.option_index,
            "pc:decorations:categories",
            16usize,
        ),
        VisibleDecorationMenuPhase::Decorations {
            decorations,
            cursor,
            ..
        } => (
            0.0,
            0.0,
            20.0,
            18.0,
            decorations
                .iter()
                .map(|decoration| decoration.display_name.clone())
                .chain(["PUT IT AWAY".to_string(), "CANCEL".to_string()])
                .collect::<Vec<_>>(),
            cursor.option_index,
            "pc:decorations:items",
            8usize,
        ),
        VisibleDecorationMenuPhase::Side { cursor, .. } => (
            0.0,
            0.0,
            14.0,
            8.0,
            vec![
                "RIGHT SIDE".to_string(),
                "LEFT SIDE".to_string(),
                "CANCEL".to_string(),
            ],
            cursor.option_index,
            "pc:decorations:side",
            3usize,
        ),
    };
    anyhow::ensure!(
        selected < entries.len(),
        "{surface_id} cursor {selected} exceeds {} entries",
        entries.len()
    );
    let (center_x, center_y) = field_window_center(left, top, width, height);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (width - 2.0),
                    TILE_SIZE * (height - 2.0),
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.0),
            ..default()
        },
        SceneDialogMarker,
        SceneDialogTextBoxBackgroundMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            left,
            top,
            width as usize,
            height as usize,
            4.05,
        );
    }
    let scroll = if entries.len() > visible_rows {
        selected
            .saturating_sub(visible_rows.saturating_sub(1))
            .min(entries.len() - visible_rows)
    } else {
        0
    };
    for (row, (index, label)) in entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_rows)
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(left + 1.0, top + 2.0 + row as f32);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{}{}", if index == selected { ">" } else { " " }, label),
            x,
            y,
            4.2,
        );
    }
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
    let frame = SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(WIDTH as f32, HEIGHT as f32),
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        20.0,
        images,
    )?;
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
    let source = crate::open_runtime_image(asset_root.resolve_vendor("gfx/diploma/diploma.png"))
        .context("decode diploma graphics")?
        .to_rgba8();
    let tilemap = crate::read_runtime_asset(asset_root.resolve_vendor("gfx/diploma/page1.tilemap"))
        .context("read diploma page-one tilemap")?;
    anyhow::ensure!(
        tilemap.len() == 20 * 18,
        "diploma page-one tilemap must be 360 bytes"
    );
    anyhow::ensure!(
        source.width() % SOURCE_TILE_SIZE as u32 == 0
            && source.height() % SOURCE_TILE_SIZE as u32 == 0,
        "diploma graphics must contain complete 8x8 tiles"
    );
    let columns = source.width() as usize / SOURCE_TILE_SIZE;
    let palette_text =
        crate::read_runtime_asset_to_string(asset_root.resolve_vendor("gfx/diploma/diploma.pal"))
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
                color: Color::WHITE,
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
    if let Some(frame) = window_frame_art(rendered_art, asset_root, images, 2) {
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
    spawn_field_command_bitmap_text(commands, rendered_art, asset_root, images, word, x, y, 4.3);
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
    let train_shift = if train_shift < 128 {
        train_shift
    } else {
        train_shift - 256
    };
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
    let frame = SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(WIDTH as f32, HEIGHT as f32),
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        20.0,
        images,
    )?;
    Ok(())
}

fn load_visible_magnet_train_base(asset_root: &AssetRoot) -> Result<Vec<u8>> {
    const WIDTH: usize = 20 * SOURCE_TILE_SIZE;
    const HEIGHT: usize = 18 * SOURCE_TILE_SIZE;
    let tileset =
        crate::open_runtime_image(asset_root.resolve_vendor("gfx/tilesets/train_station.png"))
            .context("decode magnet-train station tileset")?
            .to_rgba8();
    let background = crate::read_runtime_asset(
        asset_root.resolve_vendor("gfx/overworld/magnet_train_bg.tilemap"),
    )
    .context("read magnet-train background tilemap")?;
    let foreground = crate::read_runtime_asset(
        asset_root.resolve_vendor("gfx/overworld/magnet_train_fg.tilemap"),
    )
    .context("read magnet-train foreground tilemap")?;
    anyhow::ensure!(
        background.len() == 36,
        "magnet-train background tilemap must be 36 bytes"
    );
    anyhow::ensure!(
        foreground.len() == 80,
        "magnet-train foreground tilemap must be 80 bytes"
    );
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
                let target =
                    (((tile_y * SOURCE_TILE_SIZE + row) * WIDTH) + tile_x * SOURCE_TILE_SIZE + col)
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
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(TILE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, 3.8),
                    ..default()
                },
                FieldCommandMarker,
            ));
        }
    }
    let elm_x = if animation.kind == 1 { 16.0 } else { 0.0 };
    let elm_y = if animation.kind == 1 { 32.0 } else { 0.0 };
    for &(source_x, source_y, flip_x) in positions.iter().take(visible_count.min(positions.len())) {
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
    let source =
        crate::open_runtime_image(asset_root.resolve_vendor("gfx/overworld/heal_machine.png"))
            .context("decode heal-machine source art")?
            .to_rgba8();
    anyhow::ensure!(
        source.dimensions() == (8, 16),
        "heal-machine source art must be 8x16"
    );
    let palette_source = crate::read_runtime_asset_to_string(
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
    anyhow::ensure!(
        colors.len() == 4,
        "heal-machine palette must contain four colors"
    );
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
            Extent3d {
                width: 8,
                height: 8,
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
                commands,
                rendered_art,
                asset_root,
                images,
                11.0,
                0.0,
                9,
                3,
            );
            let (x, y) = battle_hud_tile_origin(12.0, 1.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("¥{money:>6}"),
                x,
                y,
                3.9,
            );
        }
        VisibleBalanceOverlay::CoinsTopRight { coins } => {
            spawn_visible_balance_window(
                commands,
                rendered_art,
                asset_root,
                images,
                11.0,
                0.0,
                9,
                3,
            );
            let (label_x, label_y) = battle_hud_tile_origin(12.0, 0.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "COIN",
                label_x,
                label_y,
                3.9,
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
                commands,
                rendered_art,
                asset_root,
                images,
                5.0,
                0.0,
                15,
                5,
            );
            for (text, tile_x, tile_y) in [
                ("MONEY".to_string(), 6.0, 1.0),
                (format!("¥{money:>6}"), 12.0, 1.0),
                ("COIN".to_string(), 6.0, 3.0),
                (format!("{coins:>4}"), 15.0, 3.0),
            ] {
                let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
                spawn_scene_dialog_bitmap_text(
                    commands,
                    rendered_art,
                    asset_root,
                    images,
                    &text,
                    x,
                    y,
                    3.9,
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
    let (center_x, center_y) =
        field_window_center(tile_x, tile_y, width_tiles as f32, height_tiles as f32);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
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
        SceneDialogTextBoxBackgroundMarker,
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
        commands,
        rendered_art,
        asset_root,
        images,
        0.0,
        0.0,
        width,
        height,
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
        spawn_scene_dialog_bitmap_text(commands, rendered_art, asset_root, images, line, x, y, 4.0);
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
    let Some(selected) =
        strict_readonly_cursor_index(&runtime_shell.menu_cursor, &surface_id, grid.options.len())
    else {
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

const SHOP_TOP_MENU_LEFT: f32 = 0.0;
const SHOP_TOP_MENU_WIDTH: f32 = 8.0;
const SHOP_TOP_MENU_OPTION_LEFT: f32 = SHOP_TOP_MENU_LEFT + 1.0;
const SHOP_MONEY_WINDOW_LEFT: f32 = 11.0;
const SHOP_MONEY_WINDOW_WIDTH: f32 = 9.0;
const SHOP_MONEY_TEXT_LEFT: f32 = SHOP_MONEY_WINDOW_LEFT + 1.0;

fn spawn_field_shop_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let shop_notice = if runtime_shell.shop_welcome_seen {
        runtime_shell.shop_notice.as_deref()
    } else {
        Some("Welcome! How may I\nhelp you?")
    };
    if let Some(notice) = shop_notice {
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            0.0,
            12.0,
            20.0,
            6.0,
            4.1,
        );
        let visible_notice = visible_revealed_shell_notice_text(runtime_shell, notice);
        for (index, line) in wrap_boot_text_for_box(&visible_notice, 18, 2)
            .iter()
            .enumerate()
        {
            let (x, y) = battle_hud_tile_origin(1.0, 14.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                line,
                x,
                y,
                4.5,
            );
        }
        if visible_field_text_reveal_is_complete_for_text(runtime_shell, notice)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "▼",
                x,
                y,
                4.5,
            );
        }
        return Ok(());
    }
    if let Some(cursor) = runtime_shell.shop_top_cursor.as_ref() {
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            SHOP_TOP_MENU_LEFT,
            0.0,
            SHOP_TOP_MENU_WIDTH,
            9.0,
            4.1,
        );
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            SHOP_MONEY_WINDOW_LEFT,
            0.0,
            SHOP_MONEY_WINDOW_WIDTH,
            3.0,
            4.1,
        );
        let selected = strict_readonly_cursor_index(&Some(cursor.clone()), "shop:top", 3)
            .context("shop top-menu cursor is invalid")?;
        let (x, y) = battle_hud_tile_origin(SHOP_MONEY_TEXT_LEFT, 1.0);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format_price(snapshot.trainer.money),
            x,
            y,
            4.2,
        );
        for (index, option) in ["BUY", "SELL", "QUIT"].iter().enumerate() {
            let (x, y) =
                battle_hud_tile_origin(SHOP_TOP_MENU_OPTION_LEFT, 1.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("{}{}", if selected == index { ">" } else { " " }, option),
                x,
                y,
                4.2,
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
    let selected =
        strict_readonly_cursor_index(cursor, &surface_id, item_ids.len()).with_context(|| {
            format!(
                "shop item cursor is invalid for {surface_id} with {} items",
                item_ids.len()
            )
        })?;
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        1.0,
        3.0,
        19.0,
        10.0,
        4.1,
    );
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        11.0,
        0.0,
        9.0,
        3.0,
        4.1,
    );
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        0.0,
        12.0,
        20.0,
        6.0,
        4.1,
    );
    for (tile_x, tile_y, text) in [
        (
            1.0,
            0.5,
            if selling {
                "SELL".to_string()
            } else {
                "BUY".to_string()
            },
        ),
        (12.0, 1.0, format_price(snapshot.trainer.money)),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &text,
            x,
            y,
            4.2,
        );
    }
    let scroll = visible_window_start(selected, item_ids.len(), 4);
    for (visible_index, item_id) in item_ids.iter().skip(scroll).take(4).enumerate() {
        let index = scroll + visible_index;
        let row = 4.0 + visible_index as f32 * 2.0;
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
        let (x, y) = battle_hud_tile_origin(2.0, row);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &line,
            x,
            y,
            4.2,
        );
        if !selling {
            let (x, y) = battle_hud_tile_origin(10.0, row + 1.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format_price(u32::from(item.price)),
                x,
                y,
                4.2,
            );
        }
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
    for (index, line) in wrap_boot_text_for_box(description, 18, 4)
        .iter()
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(1.0, 13.0 + index as f32);
        spawn_scene_dialog_bitmap_text(commands, rendered_art, asset_root, images, line, x, y, 4.2);
    }
    if let Some(quantity) = runtime_shell.shop_quantity.as_ref() {
        anyhow::ensure!(
            quantity.quantity > 0,
            "shop quantity prompt has zero quantity"
        );
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            7.0,
            15.0,
            13.0,
            3.0,
            4.3,
        );
        let total = u32::from(quantity.unit_price) * u32::from(quantity.quantity);
        let (x, y) = battle_hud_tile_origin(8.0, 16.0);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("×{:02} {}", quantity.quantity, format_price(total)),
            x,
            y,
            4.5,
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
    commit_presented_fullscreen_solid(commands, rendered_art, [212, 232, 245, 255], 4.0, images)?;
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
                    summary.box_slot, summary.box_index
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
                .with_context(|| format!("PC summary move metadata {} is missing", learned.name))?;
        }
        let mut rows = vec![
            (1.0, compact_scene_label(&pokemon.nickname, 10)),
            (
                2.0,
                format!(
                    "No.{:03}  \u{e10a}{:>2}",
                    pokemon.species.int_id, pokemon.level
                ),
            ),
            (
                4.0,
                format!(
                    "< {}  {}  {} >",
                    if summary.page == 1 { "[1]" } else { "1" },
                    if summary.page == 2 { "[2]" } else { "2" },
                    if summary.page == 3 { "[3]" } else { "3" }
                ),
            ),
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
                commands,
                rendered_art,
                asset_root,
                images,
                &compact_scene_label(&text, 18),
                x,
                y,
                4.2,
            );
        }
        return Ok(());
    }
    let party_move_view = runtime_shell.bill_pc_move_open && runtime_shell.bill_pc_move_party_open;
    let entries = if party_move_view {
        snapshot
            .party
            .slots
            .iter()
            .map(|slot| (slot.index, &slot.pokemon))
            .collect::<Vec<_>>()
    } else {
        pc_box
            .slots
            .iter()
            .map(|slot| (slot.index, &slot.pokemon))
            .collect::<Vec<_>>()
    };
    let option_count = if runtime_shell.bill_pc_move_open {
        if runtime_shell.bill_pc_move_source.is_some() {
            entries.len() + 1
        } else {
            entries.len().max(1)
        }
    } else {
        entries.len()
    };
    let surface_id = if party_move_view {
        pc_move_party_surface_id().to_string()
    } else {
        storage_cursor_surface_id(pc_box.index)
    };
    let selected =
        strict_readonly_cursor_index(&runtime_shell.storage_cursor, &surface_id, option_count)
            .with_context(|| {
                format!(
                    "PC storage cursor is invalid for box {} with {option_count} entries",
                    pc_box.index
                )
            })?;
    for (tile_x, text) in [
        (
            1.0,
            format!(
                "< {} >",
                if party_move_view {
                    "PARTY".to_string()
                } else {
                    compact_scene_label(&pc_box.name, 10)
                }
            ),
        ),
        (
            14.0,
            format!(
                "{:02}/{}",
                entries.len(),
                if party_move_view { 6 } else { 20 }
            ),
        ),
    ] {
        let (x, y) = battle_hud_tile_origin(tile_x, 0.5);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &text,
            x,
            y,
            4.2,
        );
    }
    let scroll = visible_window_start(selected, option_count, 7);
    for visible_index in 0..7 {
        let slot_index = scroll + visible_index;
        if slot_index >= option_count {
            break;
        }
        let row = 2.5 + visible_index as f32 * 1.5;
        let slot = entries.iter().find(|(index, _)| *index == slot_index);
        let label = slot
            .map(|(_, pokemon)| compact_scene_label(&pokemon.nickname, 10))
            .unwrap_or_else(|| "---".to_string());
        let level = slot
            .map(|(_, pokemon)| format!("L{:02}", pokemon.level))
            .unwrap_or_default();
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{:02} {}",
                if selected == slot_index { ">" } else { " " },
                slot_index + 1,
                label
            ),
            x,
            y,
            4.2,
        );
        let (x, y) = battle_hud_tile_origin(15.0, row);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &level,
            x,
            y,
            4.2,
        );
    }
    if runtime_shell.bill_pc_move_save.is_some() {
        let (x, y) = battle_hud_tile_origin(1.0, 16.0);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "Saving… Leave ON!",
            x,
            y,
            4.2,
        );
        return Ok(());
    }
    let action = if runtime_shell.bill_pc_move_open {
        "A PLACE  B CANCEL"
    } else {
        "A WITHDRAW  SELECT RELEASE"
    };
    let (x, y) = battle_hud_tile_origin(1.0, 14.0);
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &compact_scene_label(action, 18),
        x,
        y,
        4.2,
    );
    let (x, y) = battle_hud_tile_origin(1.0, 16.0);
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &format!("PARTY {:02}/6", snapshot.storage.party_count),
        x,
        y,
        4.2,
    );
    if runtime_shell.pending_pc_release.is_some() {
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            1.0,
            10.0,
            18.0,
            8.0,
            4.5,
        );
        let selected =
            strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "pc:release-confirm", 2)
                .context("PC release confirmation cursor is invalid")?;
        let (x, y) = battle_hud_tile_origin(2.0, 11.0);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "Release <PK><MN>?",
            x,
            y,
            4.8,
        );
        for (index, label) in ["YES", "NO"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(12.0, 13.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("{}{}", if selected == index { ">" } else { " " }, label),
                x,
                y,
                4.8,
            );
        }
    } else if let Some(cursor) = runtime_shell.bill_pc_pokemon_action_cursor.as_ref() {
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            10.0,
            8.0,
            9.0,
            9.0,
            4.5,
        );
        let selected = strict_readonly_cursor_index(&Some(cursor.clone()), "pc:pokemon-actions", 4)
            .context("PC Pokémon action cursor is invalid")?;
        for (index, label) in ["WITHDRAW", "STATS", "RELEASE", "CANCEL"]
            .iter()
            .enumerate()
        {
            let (x, y) = battle_hud_tile_origin(11.0, 9.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("{}{}", if selected == index { ">" } else { " " }, label),
                x,
                y,
                4.8,
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
        let visible_notice = if runtime_shell.pc_release_sequence.is_some()
            || runtime_shell.pc_transfer_sequence.is_some()
        {
            notice.to_string()
        } else {
            visible_revealed_shell_notice_text(runtime_shell, notice)
        };
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            1.0,
            10.0,
            18.0,
            8.0,
            4.5,
        );
        for (index, line) in wrap_boot_text_for_box(&visible_notice, 16, 6)
            .iter()
            .enumerate()
        {
            let (x, y) = battle_hud_tile_origin(2.0, 11.0 + index as f32);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                line,
                x,
                y,
                4.8,
            );
        }
        if visible_field_text_reveal_is_complete_for_text(runtime_shell, notice)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "▼",
                x,
                y,
                4.8,
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
    commit_presented_fullscreen_solid(commands, rendered_art, [230, 237, 214, 255], 4.0, images)?;
    let items = snapshot
        .bag
        .pc_items
        .iter()
        .filter(|item| item.quantity > 0)
        .collect::<Vec<_>>();
    let selected =
        strict_readonly_cursor_index(&runtime_shell.pc_item_cursor, "pc:items", items.len())
            .with_context(|| format!("PC item cursor is invalid for {} items", items.len()))?;
    let (x, y) = battle_hud_tile_origin(1.0, 0.5);
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &format!("ITEM STORAGE {:02}", items.len()),
        x,
        y,
        4.2,
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
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{}",
                if selected == index { ">" } else { " " },
                compact_scene_label(&catalog.name.replace('_', " "), 11)
            ),
            x,
            y,
            4.2,
        );
        let (x, y) = battle_hud_tile_origin(16.0, row);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("×{:02}", item.quantity.min(99)),
            x,
            y,
            4.2,
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
    for (index, line) in wrap_boot_text_for_box(description, 18, 3)
        .iter()
        .enumerate()
    {
        let (x, y) = battle_hud_tile_origin(1.0, 14.0 + index as f32);
        spawn_scene_dialog_bitmap_text(commands, rendered_art, asset_root, images, line, x, y, 4.2);
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
    commit_presented_fullscreen_solid(commands, rendered_art, [212, 232, 245, 255], 4.0, images)?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_box_cursor,
        "pc:bill-boxes",
        crate::core::models::MAX_PC_BOXES,
    )
    .context("PC box-selection cursor is invalid")?;
    let (x, y) = battle_hud_tile_origin(1.0, 0.5);
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        "CHANGE BOX",
        x,
        y,
        4.2,
    );
    let scroll = visible_window_start(selected, crate::core::models::MAX_PC_BOXES, 7);
    for visible_index in 0..7 {
        let index = scroll + visible_index;
        if index >= crate::core::models::MAX_PC_BOXES {
            break;
        }
        let pc_box = snapshot
            .storage
            .boxes
            .iter()
            .find(|pc_box| pc_box.index == index)
            .with_context(|| format!("PC box {index} is missing from box selection"))?;
        let (name, count) = (pc_box.name.as_str(), pc_box.count);
        let row = 2.5 + visible_index as f32 * 2.0;
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{}{:02} {}",
                if selected == index { ">" } else { " " },
                index + 1,
                compact_scene_label(name, 10)
            ),
            x,
            y,
            4.2,
        );
        let (x, y) = battle_hud_tile_origin(16.0, row);
        spawn_scene_dialog_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{:02}", count),
            x,
            y,
            4.2,
        );
    }
    if let Some(cursor) = runtime_shell.bill_pc_box_action_cursor.as_ref() {
        let selected_action = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pc:bill-box-actions",
            4,
        )
        .context("PC box-action cursor is invalid")?;
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            10.0,
            4.0,
            9.0,
            10.0,
            4.5,
        );
        for (index, label) in ["SWITCH", "NAME", "PRINT", "QUIT"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(11.0, 5.0 + index as f32 * 2.0);
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("{}{}", if index == selected_action { ">" } else { " " }, label),
                x,
                y,
                4.2,
            );
        }
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
        )?;
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
        && runtime_shell
            .shell
            .session()
            .state()
            .script_runtime
            .pending_text_wait
            .as_ref()
            .is_some_and(|wait| pending_text_wait_command_shows_prompt_arrow(&wait.command))
        && visible_field_dialogue_is_entirely_consumed(runtime_shell, snapshot)
        && runtime_shell.lcd_animation_frame & (1 << 4) != 0
}

fn pending_text_wait_command_shows_prompt_arrow(command: &str) -> bool {
    command.eq_ignore_ascii_case("promptbutton") || command.eq_ignore_ascii_case("waitbutton")
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
    (snapshot.ui.pending_yes_no.is_some()
        && visible_field_dialogue_is_entirely_consumed(runtime_shell, snapshot))
        || runtime_shell
            .pending_day_of_week
            .as_ref()
            .is_some_and(|prompt| prompt.confirming)
        || runtime_shell.pending_phone_prompt.is_some()
        || runtime_shell.pending_remember_password.is_some()
        || runtime_shell.pending_contextual_field_move.is_some()
        || runtime_shell.held_item_swap_prompt
        || runtime_shell.party_mail_take_stage.is_some()
        || runtime_shell.pc_confirmation.is_some()
        || runtime_shell
            .pack_toss
            .as_ref()
            .is_some_and(|toss| toss.confirming)
        || runtime_shell.save_flow.as_ref().is_some_and(|flow| {
            matches!(
                flow.stage,
                VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt
            )
        })
        || runtime_shell.tmhm_teach_prompt_cursor.is_some()
        || runtime_shell.tmhm_decision_prompt_cursor.is_some()
}

fn scene_dialog_yes_no_cursor_index(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<usize> {
    if let Some(prompt) = runtime_shell
        .pending_day_of_week
        .as_ref()
        .filter(|prompt| prompt.confirming)
    {
        anyhow::ensure!(prompt.yes_no_index < 2, "day-of-week YES/NO cursor is invalid");
        return Ok(prompt.yes_no_index);
    }
    if snapshot.ui.pending_yes_no.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
            .context("script YES/NO prompt has no valid ui:yes-no cursor");
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:phone-number", 2)
            .context("phone-number prompt has no valid ui:phone-number cursor");
    }
    if runtime_shell.pending_remember_password.is_some() {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "script:remember-password",
            2,
        )
        .context("password prompt has no valid script:remember-password cursor");
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "field:move-confirm", 2)
            .context("field-move prompt has no valid field:move-confirm cursor");
    }
    if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
        return strict_readonly_cursor_index(
            &runtime_shell.tmhm_teach_prompt_cursor,
            "pack:tmhm:teach-prompt",
            2,
        )
        .context("TM/HM teach prompt has no valid pack:tmhm:teach-prompt cursor");
    }
    if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
        return strict_readonly_cursor_index(
            &runtime_shell.tmhm_decision_prompt_cursor,
            "pack:tmhm:decision",
            2,
        )
        .context("TM/HM decision has no valid pack:tmhm:decision cursor");
    }
    if runtime_shell.held_item_swap_prompt {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "party:held-item-swap",
            2,
        )
        .context("held-item swap has no valid party:held-item-swap cursor");
    }
    if let Some(stage) = runtime_shell.party_mail_take_stage {
        return strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            if stage == 1 {
                "party:mail-send-pc"
            } else {
                "party:mail-lose-message"
            },
            2,
        )
        .context("party Mail prompt has no valid cursor");
    }
    if runtime_shell.pc_confirmation.is_some() {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "pc:confirmation", 2)
            .context("PC confirmation has no valid pc:confirmation cursor");
    }
    if runtime_shell
        .pack_toss
        .as_ref()
        .is_some_and(|toss| toss.confirming)
    {
        return strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "pack:toss-confirm", 2)
            .context("Pack toss confirmation has no valid pack:toss-confirm cursor");
    }
    let flow = runtime_shell
        .save_flow
        .as_ref()
        .filter(|flow| {
            matches!(
                flow.stage,
                VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt
            )
        })
        .context("no active YES/NO prompt owns the visible cursor")?;
    anyhow::ensure!(flow.yes_no_index < 2, "save YES/NO cursor is invalid");
    Ok(flow.yes_no_index)
}

fn spawn_visible_day_of_week_selection(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    const DAY_BOX_LEFT: f32 = 9.0;
    const DAY_BOX_TOP: f32 = 3.0;
    const DAY_BOX_WIDTH: f32 = 11.0;
    const DAY_BOX_HEIGHT: f32 = 4.0;
    const DAYS: [&str; 7] = [
        " SUNDAY",
        " MONDAY",
        " TUESDAY",
        "WEDNESDAY",
        "THURSDAY",
        " FRIDAY",
        "SATURDAY",
    ];
    let Some(prompt) = runtime_shell.pending_day_of_week.as_ref() else {
        return;
    };

    // TypeScript DayOfWeekScreen and engine/rtc/timeset.asm both keep the
    // ordinary 20x6 question box while drawing a separate 11x4 day picker at
    // (9,3). This is not a generic full-width menu.
    spawn_scene_dialog_text_box(commands, rendered_art, asset_root, images, 4.0);
    let (question_x, question_y) = battle_hud_tile_origin(1.0, 13.0);
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        "What day is it?",
        question_x,
        question_y,
        4.2,
    );

    let (center_x, center_y) =
        field_window_center(DAY_BOX_LEFT, DAY_BOX_TOP, DAY_BOX_WIDTH, DAY_BOX_HEIGHT);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(
                    TILE_SIZE * (DAY_BOX_WIDTH - 2.0),
                    TILE_SIZE * (DAY_BOX_HEIGHT - 2.0),
                )),
                ..default()
            },
            transform: Transform::from_xyz(center_x, center_y, 4.0),
            ..default()
        },
        SceneDialogMarker,
        SceneDialogTextBoxBackgroundMarker,
    ));
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        spawn_scene_dialog_window_frame_tiles(
            commands,
            frame,
            DAY_BOX_LEFT,
            DAY_BOX_TOP,
            DAY_BOX_WIDTH as usize,
            DAY_BOX_HEIGHT as usize,
            4.05,
        );
    }
    for (text, x, y) in [
        ("▲", 14.0, 3.0),
        (DAYS[usize::from(prompt.selected_day % 7)], 10.0, 5.0),
        ("▼", 14.0, 6.0),
    ] {
        let (x, y) = battle_hud_tile_origin(x, y);
        spawn_scene_dialog_bitmap_text(commands, rendered_art, asset_root, images, text, x, y, 4.2);
    }
}

fn spawn_visible_day_of_week_confirmation(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
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
        return Ok(());
    };
    spawn_scene_dialog_text_box(commands, rendered_art, asset_root, images, 4.0);
    let (x, y) = battle_hud_tile_origin(1.0, 13.0);
    spawn_scene_dialog_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &format!("{}, is it?", DAYS[usize::from(prompt.selected_day % 7)]),
        x,
        y,
        4.2,
    );
    spawn_visible_yes_no_prompt_box(
        snapshot,
        commands,
        runtime_shell,
        rendered_art,
        asset_root,
        images,
    )?;
    Ok(())
}

fn spawn_visible_yes_no_prompt_box(
    snapshot: &RuntimeShellSnapshot,
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    // `YesNoBox` in home/menu.asm sets b=SCREEN_WIDTH-6 and c=7, then
    // `_YesNoBox` anchors at (14,7), and the TypeScript reference renders a
    // 6x4-tile surface with YES/NO on consecutive interior rows 8 and 9.
    let (center_x, center_y) = field_window_center(
        FIELD_YES_NO_LEFT_TILE,
        FIELD_YES_NO_TOP_TILE,
        FIELD_YES_NO_WIDTH_TILES,
        FIELD_YES_NO_HEIGHT_TILES,
    );
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
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
        YesNoPromptMarker,
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
    let selected = scene_dialog_yes_no_cursor_index(snapshot, runtime_shell)?;
    for (index, label) in ["YES", "NO"].into_iter().enumerate() {
        let marker = if index == selected { ">" } else { " " };
        let (x, y) = battle_hud_tile_origin(
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE + 1.0 + index as f32,
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
    Ok(())
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
                color: Color::WHITE,
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
) -> Result<()> {
    let key = NameEntryArtKey {
        label: input.label.clone(),
        value: input.value.clone(),
        cursor_column: input.cursor_column,
        cursor_row: input.cursor_row,
        case: input.case,
    };
    if !rendered_art.name_entry_cache.contains_key(&key)
        && !rendered_art.name_entry_errors.contains_key(&key)
    {
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
        retain_bounded_fullscreen_art_key(
            &mut rendered_art.name_entry_cache,
            &mut rendered_art.name_entry_errors,
            &mut rendered_art.name_entry_cache_order,
            key.clone(),
            images,
        );
    }
    let Some(frame) = rendered_art.name_entry_cache.get(&key).cloned() else {
        return Ok(());
    };
    // Naming is a true 20x18 LCD screen, not an overworld dialog overlay.
    // Commit it into the same retained allocation as Oak/title so the first
    // naming frame and every cursor move cannot expose the staged field below.
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        6.0,
        images,
    )?;
    Ok(())
}

fn spawn_visible_mail_entry_screen(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    input: &PendingMailInput,
) -> Result<()> {
    let key = MailEntryArtKey {
        value: input.value.clone(),
        cursor_column: input.cursor_column,
        cursor_row: input.cursor_row,
        case: input.case,
    };
    if !rendered_art.mail_entry_cache.contains_key(&key)
        && !rendered_art.mail_entry_errors.contains_key(&key)
    {
        match load_mail_entry_frame(asset_root, input, images) {
            Ok(frame) => {
                rendered_art.mail_entry_errors.remove(&key);
                rendered_art.mail_entry_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .mail_entry_errors
                    .insert(key.clone(), error.to_string());
            }
        }
        retain_bounded_fullscreen_art_key(
            &mut rendered_art.mail_entry_cache,
            &mut rendered_art.mail_entry_errors,
            &mut rendered_art.mail_entry_cache_order,
            key.clone(),
            images,
        );
    }
    let Some(frame) = rendered_art.mail_entry_cache.get(&key).cloned() else {
        return Ok(());
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        6.0,
        images,
    )?;
    Ok(())
}

#[derive(Clone)]
struct MailReadTile {
    set_pixels: [bool; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE],
    color_index: usize,
}

fn spawn_visible_mail_read_screen(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    reader: &VisibleMailRead,
) -> Result<()> {
    if !rendered_art.mail_read_cache.contains_key(reader)
        && !rendered_art.mail_read_errors.contains_key(reader)
    {
        match load_mail_read_frame(asset_root, reader, images) {
            Ok(frame) => {
                rendered_art.mail_read_errors.remove(reader);
                rendered_art.mail_read_cache.insert(reader.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .mail_read_errors
                    .insert(reader.clone(), error.to_string());
            }
        }
        retain_bounded_fullscreen_art_key(
            &mut rendered_art.mail_read_cache,
            &mut rendered_art.mail_read_errors,
            &mut rendered_art.mail_read_cache_order,
            reader.clone(),
            images,
        );
    }
    let Some(frame) = rendered_art.mail_read_cache.get(reader).cloned() else {
        return Ok(());
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        6.0,
        images,
    )?;
    Ok(())
}

fn load_mail_read_frame(
    asset_root: &AssetRoot,
    reader: &VisibleMailRead,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let palette_source = std::fs::read_to_string(
        asset_root.runtime_assets().join("gfx/mail/mail.pal"),
    )
    .context("read Mail palette bank")?;
    let palettes = parse_palette_file(&palette_source, None)?;
    let mail_index = crate::core::models::item::MAIL_ITEM_IDS
        .iter()
        .position(|item| *item == reader.mail.mail_type)
        .with_context(|| format!("unknown Mail stationery {}", reader.mail.mail_type))?;
    let palette = palettes
        .get(mail_index)
        .copied()
        .with_context(|| format!("Mail palette bank has no row {mail_index}"))?;
    let mut tilemap = [[0x7f_u8; NAME_ENTRY_SCREEN_TILE_WIDTH]; NAME_ENTRY_SCREEN_TILE_HEIGHT];
    let mut tiles = BTreeMap::<u8, MailReadTile>::new();
    build_mail_read_stationery(
        asset_root,
        &reader.mail.mail_type,
        &mut tilemap,
        &mut tiles,
    )?;
    place_mail_read_text(&reader.mail, mail_index, &mut tilemap)?;

    let font = crate::open_runtime_image(asset_root.runtime_assets().join("gfx/font/font.png"))
        .context("decode Mail reader font PNG")?
        .to_rgba8();
    let font_extra =
        crate::open_runtime_image(asset_root.runtime_assets().join("gfx/font/font_extra.png"))
            .context("decode Mail reader extra-font PNG")?
            .to_rgba8();
    let width = NAME_ENTRY_SCREEN_TILE_WIDTH * SOURCE_TILE_SIZE;
    let height = NAME_ENTRY_SCREEN_TILE_HEIGHT * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel[..3].copy_from_slice(&palette[0]);
        pixel[3] = 255;
    }
    for (tile_y, row) in tilemap.iter().enumerate() {
        for (tile_x, tile_id) in row.iter().copied().enumerate() {
            if let Some(tile) = tiles.get(&tile_id) {
                draw_mail_read_tile(
                    tile,
                    &palette,
                    tile_x * SOURCE_TILE_SIZE,
                    tile_y * SOURCE_TILE_SIZE,
                    &mut data,
                );
            } else if tile_id != 0x7f {
                draw_mail_read_font_tile(
                    tile_id,
                    &font,
                    &font_extra,
                    &palette,
                    tile_x * SOURCE_TILE_SIZE,
                    tile_y * SOURCE_TILE_SIZE,
                    &mut data,
                )?;
            }
        }
    }
    if reader.mail.mail_type == "PORTRAITMAIL" {
        draw_mail_portrait(asset_root, &reader.mail.species, &palette, &mut data)?;
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

fn load_mail_1bpp_tiles(asset_root: &AssetRoot, name: &str) -> Result<Vec<[bool; 64]>> {
    let path = asset_root
        .runtime_assets()
        .join("gfx/mail")
        .join(format!("{name}.1bpp"));
    let source = std::fs::read(&path)
        .with_context(|| format!("read canonical Mail art {}", path.display()))?;
    anyhow::ensure!(
        source.len() % 8 == 0,
        "Mail art {} has non-tile byte length {}",
        path.display(),
        source.len()
    );
    let mut result = Vec::with_capacity(source.len() / 8);
    for rows in source.chunks_exact(8) {
        let mut tile = [false; 64];
        for (y, row) in rows.iter().copied().enumerate() {
            for x in 0..8 {
                tile[y * 8 + x] = row & (0x80 >> x) != 0;
            }
        }
        result.push(tile);
    }
    Ok(result)
}

fn register_mail_tiles(
    tiles: &mut BTreeMap<u8, MailReadTile>,
    start: u8,
    source: &[[bool; 64]],
    offset: usize,
    count: usize,
    color_index: usize,
) -> Result<u8> {
    let selected = source
        .get(offset..offset + count)
        .with_context(|| format!("Mail tile slice {offset}..{} is outside source", offset + count))?;
    for (index, set_pixels) in selected.iter().enumerate() {
        let tile_id = start
            .checked_add(u8::try_from(index).context("Mail tile index exceeds byte")?)
            .context("Mail tile id overflow")?;
        tiles.insert(
            tile_id,
            MailReadTile {
                set_pixels: *set_pixels,
                color_index,
            },
        );
    }
    start
        .checked_add(u8::try_from(count).context("Mail tile count exceeds byte")?)
        .context("Mail tile pointer overflow")
}

fn register_solid_mail_tiles(
    tiles: &mut BTreeMap<u8, MailReadTile>,
    start: u8,
    count: usize,
    color_index: usize,
) -> Result<u8> {
    let source = vec![[true; 64]; count];
    register_mail_tiles(tiles, start, &source, 0, count, color_index)
}

fn build_mail_read_stationery(
    asset_root: &AssetRoot,
    mail_type: &str,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    match mail_type {
        "FLOWER_MAIL" => build_flower_mail_stationery(asset_root, map, tiles),
        "SURF_MAIL" => build_surf_or_liteblue_mail_stationery(asset_root, map, tiles, false),
        "LITEBLUEMAIL" => build_surf_or_liteblue_mail_stationery(asset_root, map, tiles, true),
        "PORTRAITMAIL" => build_portrait_mail_stationery(asset_root, map, tiles),
        "LOVELY_MAIL" => build_lovely_mail_stationery(asset_root, map, tiles),
        "EON_MAIL" => build_eon_mail_stationery(asset_root, map, tiles),
        "MORPH_MAIL" => build_morph_mail_stationery(asset_root, map, tiles),
        "BLUESKY_MAIL" => build_bluesky_mail_stationery(asset_root, map, tiles),
        "MUSIC_MAIL" => build_music_mail_stationery(asset_root, map, tiles),
        "MIRAGE_MAIL" => build_mirage_mail_stationery(asset_root, map, tiles),
        other => anyhow::bail!("unknown Mail stationery {other}"),
    }
}

fn mail_draw_row(map: &mut [[u8; 20]; 18], x: usize, y: usize, tile: u8, count: usize) {
    for column in x..x + count {
        map[y][column] = tile;
    }
}

fn mail_draw_column(
    map: &mut [[u8; 20]; 18],
    x: usize,
    y: usize,
    tile: u8,
    count: usize,
) {
    for row in y..y + count {
        map[row][x] = tile;
    }
}

fn mail_draw_2x2(map: &mut [[u8; 20]; 18], x: usize, y: usize, tile: u8) {
    map[y][x] = tile;
    map[y][x + 1] = tile + 1;
    map[y + 1][x] = tile + 2;
    map[y + 1][x + 1] = tile + 3;
}

fn mail_draw_3x2(map: &mut [[u8; 20]; 18], x: usize, y: usize, tile: u8) {
    for column in 0..3 {
        map[y][x + column] = tile + column as u8;
        map[y + 1][x + column] = tile + 3 + column as u8;
    }
}

fn mail_draw_alternating_row(
    map: &mut [[u8; 20]; 18],
    x: usize,
    y: usize,
    tile: u8,
    pairs: usize,
) {
    for index in 0..=pairs * 2 {
        map[y][x + index] = tile + (index % 2) as u8;
    }
}

fn mail_draw_alternating_column(
    map: &mut [[u8; 20]; 18],
    x: usize,
    y: usize,
    tile: u8,
    pairs: usize,
) {
    for index in 0..=pairs * 2 {
        map[y + index][x] = tile + (index % 2) as u8;
    }
}

fn draw_mail_border(map: &mut [[u8; 20]; 18], variant_two: bool) {
    map[0][0] = 0x31;
    mail_draw_row(map, 1, 0, 0x32, 18);
    map[0][19] = if variant_two { 0x31 } else { 0x33 };
    mail_draw_column(map, 0, 1, if variant_two { 0x33 } else { 0x34 }, 16);
    map[17][0] = if variant_two { 0x31 } else { 0x36 };
    mail_draw_row(map, 1, 17, if variant_two { 0x34 } else { 0x37 }, 18);
    mail_draw_column(map, 19, 1, 0x35, 16);
    map[17][19] = if variant_two { 0x31 } else { 0x38 };
}

fn load_mail_tiles_named(
    asset_root: &AssetRoot,
    tiles: &mut BTreeMap<u8, MailReadTile>,
    start: u8,
    name: &str,
    offset: usize,
    count: usize,
    color_index: usize,
) -> Result<u8> {
    let source = load_mail_1bpp_tiles(asset_root, name)?;
    register_mail_tiles(tiles, start, &source, offset, count, color_index)
}

fn place_lovely_eon_icons(map: &mut [[u8; 20]; 18]) {
    for (x, y) in [(2, 2), (16, 2), (9, 4), (2, 11), (6, 12), (12, 11)] {
        mail_draw_2x2(map, x, y, 0x3d);
    }
    for (x, y) in [(5, 4), (6, 2), (12, 4), (14, 2), (3, 13), (9, 11), (16, 12)] {
        map[y][x] = 0x41;
    }
}

fn build_flower_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    let mut next = load_mail_tiles_named(asset_root, tiles, 0x31, "flower_mail_border", 0, 8, 1)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "oddish", 0, 4, 3)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "flower_mail_border", 6, 1, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "flower_1", 0, 4, 1)?;
    let _ = load_mail_tiles_named(asset_root, tiles, next, "flower_2", 0, 4, 2)?;
    draw_mail_border(map, false);
    mail_draw_row(map, 2, 15, 0x3d, 16);
    for (x, y) in [(16, 13), (2, 13)] {
        mail_draw_2x2(map, x, y, 0x39);
    }
    for (x, y) in [(2, 2), (5, 3), (10, 2), (16, 3), (5, 11), (16, 10)] {
        mail_draw_2x2(map, x, y, 0x3e);
    }
    for (x, y) in [(3, 4), (12, 3), (14, 2), (2, 10), (14, 11)] {
        mail_draw_2x2(map, x, y, 0x42);
    }
    Ok(())
}

fn build_surf_or_liteblue_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
    lite_blue: bool,
) -> Result<()> {
    let border = if lite_blue {
        "litebluemail_border"
    } else {
        "surf_mail_border"
    };
    let pokemon = if lite_blue { "dratini" } else { "lapras" };
    let mut next = load_mail_tiles_named(asset_root, tiles, 0x31, border, 0, 8, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, next, pokemon, 0, 6, 3)?;
    next = load_mail_tiles_named(
        asset_root,
        tiles,
        next,
        if lite_blue { "portraitmail_underline" } else { "wave" },
        0,
        1,
        2,
    )?;
    next = load_mail_tiles_named(asset_root, tiles, next, "small_triangle", 0, 1, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "eon_mail_border_1", 0, 1, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "eon_mail_border_1", 1, 1, 1)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "eon_mail_border_2", 0, 1, 1)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "large_triangle", 0, 4, 1)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "large_heart", 0, 4, 1)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "morph_mail_corner", 0, 4, 2)?;
    let _ = load_mail_tiles_named(asset_root, tiles, next, "large_circle", 0, 4, 2)?;

    draw_mail_border(map, false);
    mail_draw_row(map, 2, 15, 0x3f, 16);
    mail_draw_3x2(map, 15, 14, 0x39);
    for (x, y) in [(2, 2), (15, 11)] {
        mail_draw_2x2(map, x, y, 0x44);
    }
    for (x, y) in [(3, 12), (15, 2)] {
        mail_draw_2x2(map, x, y, 0x4c);
    }
    mail_draw_2x2(map, 6, 3, 0x50);
    for (tile, positions) in [
        (0x40, &[(13, 2), (6, 14)][..]),
        (0x41, &[(4, 5), (17, 5), (13, 12)][..]),
        (0x42, &[(9, 2), (14, 5), (3, 10)][..]),
        (0x43, &[(6, 11)][..]),
    ] {
        for &(x, y) in positions {
            map[y][x] = tile;
        }
    }
    Ok(())
}

fn build_portrait_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    let mut next = load_mail_tiles_named(asset_root, tiles, 0x31, "portraitmail_border", 0, 5, 2)?;
    let _ = load_mail_tiles_named(asset_root, tiles, next, "portraitmail_underline", 0, 1, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, 0x3d, "large_pokeball", 0, 4, 1)?;
    let _ = load_mail_tiles_named(asset_root, tiles, next, "small_pokeball", 0, 1, 2)?;
    draw_mail_border(map, true);
    mail_draw_row(map, 8, 15, 0x36, 10);
    place_lovely_eon_icons(map);
    Ok(())
}

fn build_lovely_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    let mut next = load_mail_tiles_named(asset_root, tiles, 0x31, "lovely_mail_border", 0, 5, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "poliwag", 0, 6, 3)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "lovely_mail_underline", 0, 1, 2)?;
    next = load_mail_tiles_named(asset_root, tiles, next, "large_heart", 0, 4, 2)?;
    let _ = load_mail_tiles_named(asset_root, tiles, next, "small_heart", 0, 1, 1)?;
    draw_mail_border(map, true);
    mail_draw_row(map, 2, 15, 0x3c, 16);
    mail_draw_3x2(map, 15, 14, 0x36);
    place_lovely_eon_icons(map);
    Ok(())
}

fn build_eon_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    load_mail_tiles_named(asset_root, tiles, 0x31, "eon_mail_border_1", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x32, "eon_mail_border_2", 0, 1, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x33, "eon_mail_border_2", 0, 1, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x34, "eon_mail_border_1", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x35, "surf_mail_border", 6, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x36, "eevee", 0, 6, 3)?;
    load_mail_tiles_named(asset_root, tiles, 0x3d, "large_circle", 0, 4, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x41, "eon_mail_border_2", 0, 1, 2)?;
    mail_draw_alternating_row(map, 0, 0, 0x31, 9);
    mail_draw_alternating_row(map, 1, 17, 0x31, 9);
    mail_draw_alternating_column(map, 0, 1, 0x33, 8);
    mail_draw_alternating_column(map, 19, 0, 0x33, 8);
    mail_draw_row(map, 2, 15, 0x35, 16);
    mail_draw_3x2(map, 15, 14, 0x36);
    place_lovely_eon_icons(map);
    Ok(())
}

fn build_morph_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    register_solid_mail_tiles(tiles, 0x31, 5, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x36, "morph_mail_corner", 3, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x37, "morph_mail_corner", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x38, "morph_mail_border", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x39, "eon_mail_border_1", 0, 1, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x3a, "morph_mail_divider", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x3b, "ditto", 0, 6, 3)?;
    draw_mail_border(map, true);
    for (x, y) in [(1, 1), (17, 15)] {
        mail_draw_2x2(map, x, y, 0x31);
    }
    for (x, y) in [(1, 3), (3, 1), (16, 16), (18, 14)] {
        map[y][x] = 0x31;
    }
    for (x, y) in [(1, 4), (2, 3), (3, 2), (4, 1)] {
        map[y][x] = 0x36;
    }
    for (x, y) in [(15, 16), (16, 15), (17, 14), (18, 13)] {
        map[y][x] = 0x37;
    }
    mail_draw_row(map, 2, 15, 0x38, 14);
    mail_draw_row(map, 2, 11, 0x39, 16);
    mail_draw_row(map, 2, 5, 0x39, 16);
    mail_draw_row(map, 6, 1, 0x3a, 13);
    mail_draw_row(map, 1, 16, 0x3a, 13);
    mail_draw_3x2(map, 3, 13, 0x3b);
    Ok(())
}

fn build_bluesky_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    load_mail_tiles_named(asset_root, tiles, 0x31, "eon_mail_border_1", 0, 1, 2)?;
    register_solid_mail_tiles(tiles, 0x32, 1, 3)?;
    load_mail_tiles_named(asset_root, tiles, 0x33, "grass", 0, 1, 3)?;
    let mut pokemon = load_mail_1bpp_tiles(asset_root, "dragonite")?;
    pokemon.extend(load_mail_1bpp_tiles(asset_root, "sentret")?);
    register_mail_tiles(tiles, 0x34, &pokemon, 0, 23, 3)?;
    load_mail_tiles_named(asset_root, tiles, 0x4b, "cloud", 0, 6, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x51, "flower_mail_border", 6, 1, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x52, "cloud", 0, 1, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x53, "cloud", 2, 2, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x55, "cloud", 5, 1, 1)?;

    mail_draw_row(map, 0, 0, 0x31, 20);
    mail_draw_column(map, 0, 1, 0x31, 16);
    mail_draw_column(map, 19, 1, 0x31, 16);
    mail_draw_row(map, 0, 17, 0x32, 20);
    mail_draw_row(map, 0, 16, 0x33, 20);
    for (row, x) in [(2, 2), (3, 3), (4, 4)] {
        for offset in 0..6 {
            map[row][x + offset] = 0x34 + ((row - 2) * 6 + offset) as u8;
        }
    }
    map[4][9] = 0x7f;
    mail_draw_2x2(map, 15, 14, 0x45);
    map[16][15] = 0x49;
    map[16][16] = 0x4a;
    mail_draw_3x2(map, 12, 1, 0x4b);
    mail_draw_3x2(map, 15, 4, 0x4b);
    mail_draw_row(map, 2, 11, 0x51, 16);
    mail_draw_2x2(map, 10, 3, 0x52);
    Ok(())
}

fn build_music_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    load_mail_tiles_named(asset_root, tiles, 0x31, "music_mail_border", 0, 4, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x35, "morph_mail_border", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x36, "small_note", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x37, "natu", 0, 6, 3)?;
    tiles.insert(
        0x3d,
        MailReadTile {
            set_pixels: [false; 64],
            color_index: 0,
        },
    );
    load_mail_tiles_named(asset_root, tiles, 0x3e, "large_note", 0, 3, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x41, "small_note", 0, 1, 1)?;
    mail_draw_alternating_row(map, 0, 0, 0x31, 9);
    mail_draw_alternating_row(map, 1, 17, 0x31, 9);
    mail_draw_alternating_column(map, 0, 1, 0x33, 8);
    mail_draw_alternating_column(map, 19, 0, 0x33, 8);
    mail_draw_alternating_row(map, 2, 15, 0x35, 7);
    mail_draw_3x2(map, 15, 14, 0x37);
    place_lovely_eon_icons(map);
    Ok(())
}

fn build_mirage_mail_stationery(
    asset_root: &AssetRoot,
    map: &mut [[u8; 20]; 18],
    tiles: &mut BTreeMap<u8, MailReadTile>,
) -> Result<()> {
    register_solid_mail_tiles(tiles, 0x31, 5, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x36, "grass", 0, 1, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x37, "mew", 0, 18, 2)?;
    load_mail_tiles_named(asset_root, tiles, 0x49, "litebluemail_border", 1, 1, 1)?;
    load_mail_tiles_named(asset_root, tiles, 0x4a, "litebluemail_border", 6, 1, 1)?;
    draw_mail_border(map, true);
    mail_draw_row(map, 1, 16, 0x36, 18);
    mail_draw_3x2(map, 15, 14, 0x37);
    map[16][15] = 0x3d;
    map[16][16] = 0x3e;
    mail_draw_alternating_row(map, 1, 1, 0x3f, 9);
    mail_draw_alternating_column(map, 0, 2, 0x41, 7);
    mail_draw_alternating_column(map, 19, 2, 0x43, 7);
    for (tile, x, y) in [(0x45, 0, 1), (0x46, 19, 1), (0x47, 0, 16), (0x48, 19, 16)] {
        map[y][x] = tile;
    }
    mail_draw_row(map, 2, 5, 0x49, 16);
    mail_draw_row(map, 2, 11, 0x4a, 16);
    Ok(())
}

fn place_mail_read_text(
    mail: &crate::core::models::pokemon::MailData,
    mail_index: usize,
    map: &mut [[u8; 20]; 18],
) -> Result<()> {
    let mut lines = mail.message.split('\n');
    let first = lines.next().unwrap_or_default();
    let explicit_second = lines.next();
    let (first, second) = if let Some(second) = explicit_second {
        (first.to_string(), second.to_string())
    } else {
        let chars = first.chars().collect::<Vec<_>>();
        (
            chars.iter().take(16).collect(),
            chars.iter().skip(16).take(16).collect(),
        )
    };
    for (row, text) in [(7, first.as_str()), (9, second.as_str())] {
        for (offset, character) in text.chars().take(16).enumerate() {
            map[row][2 + offset] = mail_entry_char_tile(character)
                .with_context(|| format!("unsupported Mail reader character {character:?}"))?;
        }
    }
    if !mail.author.is_empty() {
        let x = match mail_index {
            3 => 8,
            6 => 6,
            _ => 5,
        };
        for (offset, character) in mail.author.chars().take(10).enumerate() {
            map[14][x + offset] = mail_entry_char_tile(character)
                .with_context(|| format!("unsupported Mail author character {character:?}"))?;
        }
    }
    Ok(())
}

fn draw_mail_read_tile(
    tile: &MailReadTile,
    palette: &[[u8; 3]; 4],
    dest_x: usize,
    dest_y: usize,
    target: &mut [u8],
) {
    for y in 0..8 {
        for x in 0..8 {
            if tile.set_pixels[y * 8 + x] {
                let target_index = ((dest_y + y) * 160 + dest_x + x) * 4;
                target[target_index..target_index + 3]
                    .copy_from_slice(&palette[tile.color_index]);
            }
        }
    }
}

fn draw_mail_read_font_tile(
    tile_id: u8,
    font: &image::RgbaImage,
    font_extra: &image::RgbaImage,
    palette: &[[u8; 3]; 4],
    dest_x: usize,
    dest_y: usize,
    target: &mut [u8],
) -> Result<()> {
    let (source, index) = if (0x60..0x80).contains(&tile_id) {
        (font_extra, usize::from(tile_id - 0x60))
    } else if tile_id >= 0x80 {
        (font, usize::from(tile_id - 0x80))
    } else {
        return Ok(());
    };
    let tiles_per_row = source.width() as usize / 8;
    anyhow::ensure!(tiles_per_row > 0, "Mail font has invalid width {}", source.width());
    let source_x = (index % tiles_per_row) * 8;
    let source_y = (index / tiles_per_row) * 8;
    anyhow::ensure!(source_y + 8 <= source.height() as usize, "Mail font tile {tile_id:#x} is absent");
    for y in 0..8 {
        for x in 0..8 {
            let pixel = source.get_pixel((source_x + x) as u32, (source_y + y) as u32);
            if pixel[3] != 0 && pixel[0] < 128 {
                let target_index = ((dest_y + y) * 160 + dest_x + x) * 4;
                target[target_index..target_index + 3].copy_from_slice(&palette[3]);
            }
        }
    }
    Ok(())
}

fn draw_mail_portrait(
    asset_root: &AssetRoot,
    species: &str,
    palette: &[[u8; 3]; 4],
    target: &mut [u8],
) -> Result<()> {
    let path = asset_root
        .runtime_assets()
        .join("gfx/pokemon")
        .join(species.to_ascii_lowercase().replace('_', "-"))
        .join("front.png");
    let source = crate::open_runtime_image(&path)
        .with_context(|| format!("decode Portrait Mail frontpic {}", path.display()))?
        .to_rgba8();
    let frame_size = source.width().min(source.height()) as usize;
    anyhow::ensure!(frame_size > 0 && frame_size <= 56, "invalid Portrait Mail frontpic dimensions {}x{}", source.width(), source.height());
    let dest_x = 8 + (56 - frame_size) / 2;
    let dest_y = 80 + (56 - frame_size) / 2;
    for y in 0..frame_size {
        for x in 0..frame_size {
            let pixel = source.get_pixel(x as u32, y as u32);
            if pixel[3] == 0 {
                continue;
            }
            let luminance = (u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2])) / 3;
            let color_index = match luminance {
                0..=63 => 3,
                64..=159 => 2,
                160..=239 => 1,
                _ => 0,
            };
            if color_index != 0 {
                let target_index = ((dest_y + y) * 160 + dest_x + x) * 4;
                target[target_index..target_index + 3].copy_from_slice(&palette[color_index]);
            }
        }
    }
    Ok(())
}

fn load_mail_entry_frame(
    asset_root: &AssetRoot,
    input: &PendingMailInput,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode Mail composer font PNG")?
        .to_rgba8();
    let font_extra = crate::open_runtime_image(assets.join("gfx/font/font_extra.png"))
        .context("decode Mail composer extra-font PNG")?
        .to_rgba8();
    let cursor = crate::open_runtime_image(assets.join("gfx/naming_screen/cursor.png"))
        .context("decode Mail composer cursor PNG")?
        .to_rgba8();
    let border = crate::open_runtime_image(assets.join("gfx/naming_screen/border.png"))
        .context("decode Mail composer border PNG")?
        .to_rgba8();
    let underline = crate::open_runtime_image(assets.join("gfx/naming_screen/underline.png"))
        .context("decode Mail composer underline PNG")?
        .to_rgba8();
    let middle_line = crate::open_runtime_image(assets.join("gfx/naming_screen/middle_line.png"))
        .context("decode Mail composer middle-line PNG")?
        .to_rgba8();
    let mail_icon = crate::open_runtime_image(assets.join("gfx/naming_screen/mail.png"))
        .context("decode Mail composer icon PNG")?
        .to_rgba8();
    let tilemap = build_mail_entry_tilemap(input)?;
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
                &font_extra,
                &border,
                &underline,
                &middle_line,
                tile_x * SOURCE_TILE_SIZE,
                tile_y * SOURCE_TILE_SIZE,
                &mut data,
            )?;
        }
    }
    blit_name_entry_tile_image(&mail_icon, 0, 0, 8, 8, false, false, true, &mut data);
    blit_name_entry_tile_image(&mail_icon, 8, 0, 16, 8, false, false, true, &mut data);
    blit_name_entry_tile_image(&mail_icon, 0, 8, 8, 16, false, false, true, &mut data);
    blit_name_entry_tile_image(&mail_icon, 8, 8, 16, 16, false, false, true, &mut data);
    draw_mail_entry_cursor(input, &cursor, &mut data)?;
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

fn build_mail_entry_tilemap(input: &PendingMailInput) -> Result<Vec<Vec<u8>>> {
    let mut tilemap = vec![vec![0_u8; NAME_ENTRY_SCREEN_TILE_WIDTH]; NAME_ENTRY_SCREEN_TILE_HEIGHT];
    for row in tilemap.iter_mut().take(6) {
        row.fill(NAME_ENTRY_BORDER_TILE);
    }
    clear_name_entry_box(&mut tilemap, 1, 1, 18, 4);
    for (index, character) in input.value.chars().take(MAIL_INPUT_MESSAGE_LENGTH).enumerate() {
        let (x, y) = if index < MAIL_INPUT_LINE_LENGTH {
            (2 + index, 2)
        } else {
            (2 + index - MAIL_INPUT_LINE_LENGTH, 4)
        };
        tilemap[y][x] = mail_entry_char_tile(character)
            .with_context(|| format!("unsupported Mail composer character {character:?}"))?;
    }
    let next = input.value.chars().count().min(MAIL_INPUT_MESSAGE_LENGTH);
    if next < MAIL_INPUT_MESSAGE_LENGTH {
        let (x, y) = if next < MAIL_INPUT_LINE_LENGTH {
            (2 + next, 2)
        } else {
            (2 + next - MAIL_INPUT_LINE_LENGTH, 4)
        };
        tilemap[y][x] = NAME_ENTRY_UNDERLINE_TILE;
    }
    for row in 0..5 {
        for column in 0..MAIL_INPUT_COLUMNS {
            if let Some(character) = visible_mail_input_row_chars(input.case, row)[column] {
                tilemap[7 + row * 2][column * 2] = mail_entry_char_tile(character)
                    .with_context(|| format!("unsupported Mail keyboard character {character:?}"))?;
            }
        }
    }
    let bottom = visible_mail_input_layout(input.case)[5];
    for (column, tile) in name_entry_string_tiles_19(bottom)?.into_iter().enumerate() {
        tilemap[17][column] = tile;
    }
    Ok(tilemap)
}

fn name_entry_string_tiles_19(text: &str) -> Result<Vec<u8>> {
    let mut tiles = Vec::new();
    for token in tokenize_name_entry_string(text) {
        let tile = name_entry_token_tile(&token)
            .with_context(|| format!("unsupported Mail keyboard glyph {token:?}"))?;
        tiles.push(tile);
    }
    if tiles.len() != 19 {
        anyhow::bail!("Mail keyboard command row has {} tiles, expected 19", tiles.len());
    }
    Ok(tiles)
}

fn mail_entry_char_tile(character: char) -> Option<u8> {
    Some(match character {
        '\u{e105}' => 0xe1,
        '\u{e106}' => 0xe2,
        '\u{e108}' => 0x70,
        '\u{e109}' => 0x71,
        '\u{e120}' => 0xd0,
        '\u{e121}' => 0xd1,
        '\u{e122}' => 0xd2,
        '\u{e123}' => 0xd3,
        '\u{e124}' => 0xd4,
        '\u{e125}' => 0xd5,
        '\u{e126}' => 0xd6,
        '“' => 0x72,
        '”' => 0x73,
        '…' => 0x75,
        other => name_entry_char_tile(other)?,
    })
}

fn draw_mail_entry_cursor(
    input: &PendingMailInput,
    cursor: &image::RgbaImage,
    target: &mut [u8],
) -> Result<()> {
    let x_offset = if input.cursor_row == MAIL_INPUT_ROWS - 1 {
        [0, 0, 0, 0x30, 0x30, 0x30, 0x60, 0x60, 0x60, 0x60][input.cursor_column]
    } else {
        (input.cursor_column as i16) * 0x10
    };
    // `depixel 9, 2` becomes OAM x=16/y=72 because sprite init consumes
    // RGBDS's DE pair in y/x order, matching the ordinary naming compositor.
    let anchor_x = 16 + x_offset;
    let anchor_y = 72 + input.cursor_row as i16 * 0x10;
    let tile_x = ((anchor_x - 8) / 8) * 8;
    let tile_y = ((anchor_y - 16) / 8) * 8;
    let pieces: &[(i16, i16, i16, i16, usize, bool, bool)] =
        if input.cursor_row == MAIL_INPUT_ROWS - 1 {
            &NAME_ENTRY_BIG_CURSOR_OAM
        } else {
            &NAME_ENTRY_SMALL_CURSOR_OAM
        };
    for (x_tile, y_tile, x_px, y_px, tile_index, xflip, yflip) in pieces {
        blit_name_entry_tile_image(
            cursor,
            0,
            tile_index * SOURCE_TILE_SIZE,
            (tile_x + x_tile * 8 + x_px).max(0) as usize,
            (tile_y + y_tile * 8 + y_px).max(0) as usize,
            *xflip,
            *yflip,
            true,
            target,
        );
    }
    Ok(())
}

fn spawn_visible_name_choice_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    choice: &VisibleNameChoice,
) -> Result<()> {
    let nickname_target = runtime_shell
        .pending_standard_capture
        .as_ref()
        .map(|capture| capture.default_name.as_str())
        .or_else(|| {
            runtime_shell
                .pending_gift_pokemon_nickname
                .as_ref()
                .map(|gift| gift.default_name.as_str())
        })
        .or_else(|| {
            runtime_shell
                .pending_egg_hatch_nickname
                .as_ref()
                .map(|hatch| hatch.default_name.as_str())
        });
    if let Some(default_name) = nickname_target {
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
            default_name.to_string(),
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
                FIELD_YES_NO_TOP_TILE + 1.0 + index as f32,
            );
            spawn_scene_dialog_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!(
                    "{}{label}",
                    if index == choice.selected { ">" } else { " " }
                ),
                x,
                y,
                6.3,
            );
        }
        return Ok(());
    }
    // The new-game preset-name menu is a complete LCD scene. Its 12x14
    // window intentionally occupies only the left half, but the remainder is
    // blank background—not the already initialized bedroom. Keep capture
    // nickname YES/NO prompts as battle overlays via the branch above.
    commit_presented_fullscreen_solid(commands, rendered_art, [255, 255, 255, 255], 5.8, images)?;
    const LEFT_TILE: f32 = 0.0;
    const TOP_TILE: f32 = 0.0;
    // NEW NAME occupies eight glyph cells beginning at column two. Twelve
    // tiles leave its final glyph clear of the right frame, while fourteen
    // tiles tightly contain the five double-spaced choices.
    const WIDTH_TILES: usize = 12;
    const HEIGHT_TILES: usize = 14;
    let (center_x, center_y) =
        field_window_center(LEFT_TILE, TOP_TILE, WIDTH_TILES as f32, HEIGHT_TILES as f32);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
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
    Ok(())
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
        let Ok(selected) = scene_dialog_yes_no_cursor_index(snapshot, runtime_shell) else {
            return false;
        };
        for (index, label) in ["YES", "NO"].into_iter().enumerate() {
            let marker = if index == selected { ">" } else { " " };
            let (x, y) = battle_hud_tile_origin(
                FIELD_YES_NO_LEFT_TILE,
                FIELD_YES_NO_TOP_TILE + 1.0 + index as f32,
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

#[cfg(test)]
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
    let font = crate::open_runtime_image(assets.join("gfx/font/font.png"))
        .context("decode naming screen font PNG")?
        .to_rgba8();
    let font_extra = crate::open_runtime_image(assets.join("gfx/font/font_extra.png"))
        .context("decode naming screen extra-font PNG")?
        .to_rgba8();
    let cursor = crate::open_runtime_image(assets.join("gfx/naming_screen/cursor.png"))
        .context("decode naming screen cursor PNG")?
        .to_rgba8();
    let border = crate::open_runtime_image(assets.join("gfx/naming_screen/border.png"))
        .context("decode naming screen border PNG")?
        .to_rgba8();
    let underline = crate::open_runtime_image(assets.join("gfx/naming_screen/underline.png"))
        .context("decode naming screen underline PNG")?
        .to_rgba8();
    let middle_line = crate::open_runtime_image(assets.join("gfx/naming_screen/middle_line.png"))
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
                &font_extra,
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
    for (index, ch) in input.value.chars().take(input.max_length).enumerate() {
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
        "▲" => 0x61,
        "◀" => 0x71,
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
    font_extra: &image::RgbaImage,
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
    if (0x60..0x80).contains(&tile_id) {
        let font_index = usize::from(tile_id - 0x60);
        let tiles_per_row = font_extra.width() as usize / SOURCE_TILE_SIZE;
        let source_x = (font_index % tiles_per_row) * SOURCE_TILE_SIZE;
        let source_y = (font_index / tiles_per_row) * SOURCE_TILE_SIZE;
        blit_name_entry_tile_image(
            font_extra,
            source_x,
            source_y,
            dest_x,
            dest_y,
            false,
            false,
            false,
            target,
        );
    } else if tile_id >= 0x80 {
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
        commands,
        window,
        LEFT,
        TOP,
        WIDTH as usize,
        HEIGHT as usize,
        4.0,
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
                sprite: Sprite {
                    custom_size: Some(art.size),
                    ..default()
                },
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
            let dimensions = crate::read_runtime_asset(root.join("front.dimensions"))
                .with_context(|| format!("read Pokepic dimensions for {species_id}"))?;
            let dimension = dimensions
                .first()
                .map(|value| usize::from(value & 0x0f))
                .context("Pokepic dimensions file is empty")?;
            if !(5..=7).contains(&dimension) {
                anyhow::bail!(
                    "Pokepic {species_id} has invalid {dimension}x{dimension} dimensions"
                );
            }
            let data = crate::read_runtime_asset(root.join("front.2bpp"))
                .with_context(|| format!("read Pokepic graphics for {species_id}"))?;
            let byte_count = dimension * dimension * 16;
            let frame = data
                .get(..byte_count)
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
                        if level == 0 {
                            continue;
                        }
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
                Extent3d {
                    width: 56,
                    height: 56,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                rgba,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::nearest();
            Ok(SpriteFrame {
                handle: images.add(image),
                size: Vec2::splat(56.0),
            })
        })();
        match loaded {
            Ok(frame) => {
                rendered_art.pokepic_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art
                    .pokepic_errors
                    .insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art.pokepic_cache.get(&key).with_context(|| {
        rendered_art
            .pokepic_errors
            .get(&key)
            .cloned()
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
    // `last_special_routine` is execution history, not the identity of the
    // current YesNoBox. Mom's later PHONE question is pending while
    // `ComeHomeForDSTText` prints, so a stale Initial*DSTFlag value must not
    // replace those four pages with the earlier clock confirmation.
    if snapshot.ui.pending_yes_no.is_some()
        && snapshot
            .ui
            .text
            .as_ref()
            .is_some_and(|text| text.label == "IsItDSTText")
    {
        match snapshot.script_events.last_special_routine.as_deref() {
            Some("InitialSetDSTFlag") => {
                return Some(vec![format!(
                    "{:02}:{:02} DST,\nis that OK?",
                    snapshot.progression.time.game_time_hours % 24,
                    snapshot.progression.time.game_time_minutes.min(59)
                )]);
            }
            Some("InitialClearDSTFlag") => {
                return Some(vec![format!(
                    "{:02}:{:02},\nis that OK?",
                    snapshot.progression.time.game_time_hours % 24,
                    snapshot.progression.time.game_time_minutes.min(59)
                )]);
            }
            _ => {}
        }
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            return Some(vec!["Welcome! How may I\nhelp you?".to_string()]);
        }
        if let Some(notice) = runtime_shell.shop_notice.as_ref() {
            return Some(vec![notice.clone()]);
        }
    }
    // Script text events are retained as execution history after `closetext`.
    // They are not a visible textbox. Requiring the authoritative window bit
    // prevents the previous line from being rendered as pre-text and stops a
    // stale typewriter from owning input after the script has completed.
    if !snapshot.ui.text_window_open {
        return None;
    }
    let text = snapshot.ui.text.as_ref()?;
    let mut resolved_asm_text = text.asm_text.clone();
    let mut resolved_body = text.body.clone();
    // TX_FAR prints the body at its pointer; the pointer symbol itself is not
    // text. Exported wrappers are `text_far Target` followed by `text_end`.
    // Resolve through the authoritative runtime catalog, with a hard depth
    // bound to reject malformed cycles rather than displaying debug labels.
    for _ in 0..8 {
        let far_target = resolved_body.as_ref().and_then(|body| {
            body.commands
                .iter()
                .find(|command| command.command == "text_far")
                .and_then(|command| command.args.first())
                .cloned()
        });
        let Some(far_target) = far_target else {
            break;
        };
        let far_text = runtime_shell.shell.text_snapshot(&far_target).ok()?;
        resolved_asm_text = far_text.asm_text;
        resolved_body = far_text.body;
    }
    if resolved_body.as_ref().is_some_and(|body| {
        body.commands
            .iter()
            .any(|command| command.command == "text_far")
    }) {
        return None;
    }
    let pages = if let Some(asm_text) = &resolved_asm_text {
        render_visible_asm_text_pages(
            asm_text,
            &snapshot.script_events.named_buffers,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )
    } else if let Some(body) = &resolved_body {
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

fn visible_revealed_shell_notice_text(runtime_shell: &BevyRuntimeShell, full_text: &str) -> String {
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
    let Some(pages) = visible_field_dialog_pages(snapshot, runtime_shell) else {
        return true;
    };
    let Some(reveal) = runtime_shell.field_text_reveal.as_ref() else {
        // The authoritative script can publish `writetext` and its following
        // wait in one update, before the presentation system has initialized
        // the typewriter.  That is still an entirely unread page, not a
        // completed one.  Reporting it complete lets the same A press consume
        // `promptbutton`/`waitbutton` and enter the next modal command; Mom's
        // weekday selector then opens while her previous text starts printing
        // underneath it.
        return false;
    };
    let text_identity = pages.join("\u{1e}");
    if reveal.text != text_identity {
        return false;
    }
    let full_text = &pages[reveal.page_index.min(pages.len().saturating_sub(1))];
    visible_field_text_reveal_is_complete(reveal, &full_text)
}

fn visible_field_dialogue_is_entirely_consumed(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> bool {
    let Some(pages) = visible_field_dialog_pages(snapshot, runtime_shell) else {
        return true;
    };
    let Some(reveal) = runtime_shell.field_text_reveal.as_ref() else {
        return false;
    };
    let text_identity = pages.join("\u{1e}");
    if reveal.text != text_identity || reveal.page_index + 1 != pages.len() {
        return false;
    }
    visible_field_text_reveal_is_complete(reveal, &pages[reveal.page_index])
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
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let Some(pages) = visible_field_dialog_pages(&snapshot, runtime_shell) else {
        if runtime_shell.active_script_cursor.is_some() {
            // A completed writetext can have a one-frame gap after its
            // TextLabel is consumed and before yesorno/promptbutton publishes
            // the same last Write again. Retain the completed printer across
            // that transient gap; clearing it reprints the full dialogue.
            return Ok(false);
        }
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
    let previous_page_index = reveal.page_index;
    let text_len = pages[previous_page_index].chars().count();
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
    // `<CONT>` scrolls the existing bottom line to the top and then prints
    // only the new bottom line. Starting the next page at zero retyped that
    // carried line, which players saw as Mom repeating her last words.
    reveal.visible_chars =
        visible_field_page_initial_chars(&pages[previous_page_index], &pages[reveal.page_index]);
    reveal.frames_until_next_char = 0;
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(true)
}

fn visible_field_page_initial_chars(previous_page: &str, next_page: &str) -> usize {
    let Some(previous_bottom_line) = previous_page.lines().last() else {
        return 0;
    };
    let Some(next_top_line) = next_page.lines().next() else {
        return 0;
    };
    if previous_bottom_line.is_empty() || previous_bottom_line != next_top_line {
        return 0;
    }
    next_top_line.chars().count() + usize::from(next_page.contains('\n'))
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
    if runtime_shell.decoration_menu.is_some() {
        entries.extend(visible_decoration_command_entries(runtime_shell)?);
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
            format!(
                "player PC action cursor is invalid for {} actions",
                actions.len()
            )
        })?;
        entries.extend(actions.iter().enumerate().map(|(index, action)| {
            format!(
                "{}{}",
                if index == selected { ">" } else { " " },
                visible_player_pc_action_label(*action)
            )
        }));
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.mailbox_cursor.is_some() {
        push_visible_mailbox_dialog_entries(&mut entries, snapshot, runtime_shell)?;
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
        push_visible_shop_dialog_entries(&mut entries, snapshot, runtime_shell, shop)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if let Some(menu) = &snapshot.ui.menu {
        push_visible_runtime_menu_dialog_entries(&mut entries, runtime_shell, menu)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    // ASM `pokepic` owns only its menu_coords 6,4,14,13 picture window.
    // It does not implicitly open the field textbox or print species data;
    // the surrounding script supplies any authored text after `closepokepic`.
    if snapshot.ui.active_pokemon_picture.is_some() {
        return Ok(entries);
    }
    if snapshot.pending_move_learn.is_some() {
        push_visible_pending_move_learn_entries(&mut entries, snapshot, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    if runtime_shell.elevator_cursor.is_some() {
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
    if runtime_shell.pending_mail_input.is_some() {
        push_visible_mail_input_entries(&mut entries, runtime_shell);
        return Ok(entries);
    }
    if runtime_shell.save_flow.is_some() {
        push_visible_save_dialog_entries(&mut entries, snapshot, runtime_shell)?;
        entries.truncate(SCENE_MENU_VISIBLE_ROWS);
        return Ok(entries);
    }
    let field_dialog_text = visible_field_dialog_text(snapshot, runtime_shell);
    if let Some(full_text) = field_dialog_text.as_deref() {
        push_wrapped_scene_dialog_lines(
            &mut entries,
            &visible_revealed_field_dialog_text(runtime_shell, &full_text),
        );
    }
    if snapshot.ui.pending_yes_no.is_some()
        && visible_field_dialogue_is_entirely_consumed(runtime_shell, snapshot)
    {
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

    if field_dialog_text.is_some() {
        // The field box has two printable baselines. A `promptbutton`
        // indicator is an arrow in its bottom-right tile, never another row (and
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
    let selected =
        strict_readonly_cursor_index(&runtime_shell.pc_hub_cursor, "pc:hub", actions.len())
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
) -> Result<()> {
    entries.push("MAIL BOX".to_string());
    let selected = strict_readonly_cursor_index(
        &runtime_shell.mailbox_cursor,
        "pc:mailbox",
        snapshot.mailbox.len(),
    )
    .context("mailbox is open without a valid nonempty cursor")?;
    for (index, entry) in snapshot.mailbox.iter().enumerate() {
        entries.push(compact_scene_label(
            &format!(
                "{}{}",
                if index == selected { ">" } else { " " },
                entry.mail.author
            ),
            SCENE_DIALOG_TEXT_CHARS,
        ));
    }
    if let Some(cursor) = &runtime_shell.mailbox_action_cursor {
        let action = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pc:mailbox-actions",
            VISIBLE_MAILBOX_ACTIONS.len(),
        )
        .context("mailbox action menu is open without a valid cursor")?;
        entries.clear();
        entries.push(compact_scene_label(
            &snapshot.mailbox[selected].mail.author,
            SCENE_DIALOG_TEXT_CHARS,
        ));
        entries.extend(
            VISIBLE_MAILBOX_ACTIONS
                .iter()
                .enumerate()
                .map(|(index, label)| {
                    format!("{}{}", if index == action { ">" } else { " " }, label)
                }),
        );
    }
    Ok(())
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
        || runtime_shell.elevator_cursor.is_some()
        || runtime_shell.pending_day_of_week.is_some()
        || runtime_shell.pending_phone_prompt.is_some()
        || runtime_shell.pending_remember_password.is_some()
        || runtime_shell.pending_name_input.is_some()
        || runtime_shell.pending_mail_input.is_some()
        || runtime_shell.pending_mail_read.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.save_flow.is_some()
        || snapshot.ui.pending_yes_no.is_some()
        || snapshot.ui.pending_text_wait.is_some()
        || runtime_shell.pc_hub_cursor.is_some()
        || runtime_shell.decoration_menu.is_some()
        || runtime_shell.player_pc_action_cursor.is_some()
        || runtime_shell.mailbox_cursor.is_some()
        || runtime_shell.pc_confirmation.is_some()
        || runtime_shell.storage_cursor.is_some()
        || runtime_shell.pc_item_cursor.is_some()
        || (runtime_shell.active_script_cursor.is_some()
            && runtime_shell.visible_buena_password.is_none()
            && runtime_shell.visible_battle_tower_challenge_menu.is_none()
            && runtime_shell.visible_battle_tower_room_menu.is_none())
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
        VisibleSaveFlowStage::Prompt => match flow.origin {
            VisibleSaveFlowOrigin::StartMenu => SAVE_TEXT_WOULD_YOU_LIKE,
            VisibleSaveFlowOrigin::BillsPcMove => SAVE_TEXT_MOVE_MON_WITHOUT_MAIL,
            VisibleSaveFlowOrigin::BillsPcChangeBox { .. } => SAVE_TEXT_CHANGE_BOX,
        },
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

fn push_visible_day_of_week_entries(entries: &mut Vec<String>, runtime_shell: &BevyRuntimeShell) {
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

fn push_visible_mail_input_entries(entries: &mut Vec<String>, runtime_shell: &BevyRuntimeShell) {
    let Some(input) = runtime_shell.pending_mail_input.as_ref() else {
        return;
    };
    entries.push("MAIL COMPOSER".to_string());
    let first = input.value.chars().take(MAIL_INPUT_LINE_LENGTH).collect::<String>();
    let second = input
        .value
        .chars()
        .skip(MAIL_INPUT_LINE_LENGTH)
        .collect::<String>();
    entries.push(first);
    entries.push(second);
    for (row, line) in visible_mail_input_layout(input.case).iter().enumerate() {
        entries.push((*line).to_string());
        if row == input.cursor_row {
            entries.push(format!("{}^", " ".repeat(input.cursor_column * 2)));
        }
    }
}

fn push_visible_name_choice_entries(entries: &mut Vec<String>, runtime_shell: &BevyRuntimeShell) {
    let Some(choice) = runtime_shell.pending_name_choice.as_ref() else {
        return;
    };
    if let Some(default_name) = runtime_shell
        .pending_standard_capture
        .as_ref()
        .map(|capture| capture.default_name.as_str())
        .or_else(|| {
            runtime_shell
                .pending_gift_pokemon_nickname
                .as_ref()
                .map(|gift| gift.default_name.as_str())
        })
        .or_else(|| {
            runtime_shell
                .pending_egg_hatch_nickname
                .as_ref()
                .map(|hatch| hatch.default_name.as_str())
        })
    {
        entries.push(compact_scene_label(
            &format!("Give a nickname to {default_name}?"),
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
        let surface_id = runtime_shell
            .elevator_cursor
            .as_ref()
            .map(|cursor| cursor.surface_id.as_str())
            .context("elevator prompt is active without a cursor")?;
        anyhow::bail!(
            "retained elevator surface {surface_id} has no matching compiled floors on map {}",
            snapshot.overworld.map_name
        );
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
) -> Result<()> {
    let Some(pending) = snapshot.pending_move_learn.as_ref() else {
        return Ok(());
    };
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .with_context(|| {
            format!(
                "pending move learn party index {} is not in the party",
                pending.party_index
            )
        })?;
    let text_target = if runtime_shell.move_learn_forget_menu_open {
        "_MoveAskForgetText"
    } else if runtime_shell.move_learn_decision == Some(VisibleTmHmDecision::StopLearning) {
        "_StopLearningMoveText"
    } else {
        "_AskForgetMoveText"
    };
    let mut pages = visible_move_learning_text_pages(
        runtime_shell,
        text_target,
        &slot.pokemon.nickname,
        &slot.pokemon.nickname,
        &pending.learned_move.name,
    )?;
    entries.push(
        pages
            .pop()
            .with_context(|| format!("move-learning text {text_target} has no final page"))?,
    );
    if let Some(cursor) = &runtime_shell.move_learn_decision_cursor {
        let selected = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "move-learn:decision",
            2,
        )
        .context("pending move-learn decision has no valid cursor")?;
        entries.push(format!("{}YES", if selected == 0 { ">" } else { " " }));
        entries.push(format!("{}NO", if selected == 1 { ">" } else { " " }));
        return Ok(());
    }
    if !runtime_shell.move_learn_forget_menu_open {
        return Ok(());
    }
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(pending.party_index),
        5,
    )
    .context("pending move-learn forget menu has no valid cursor")?;
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
    Ok(())
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
    let party_move_view = runtime_shell.bill_pc_move_open && runtime_shell.bill_pc_move_party_open;
    let visible_slots = if party_move_view {
        snapshot
            .party
            .slots
            .iter()
            .map(|slot| (slot.index, &slot.pokemon))
            .collect::<Vec<_>>()
    } else {
        box_snapshot
            .slots
            .iter()
            .map(|slot| (slot.index, &slot.pokemon))
            .collect::<Vec<_>>()
    };
    let surface_id = if party_move_view {
        pc_move_party_surface_id().to_string()
    } else {
        storage_cursor_surface_id(box_snapshot.index)
    };
    entries.push(compact_scene_label(
        &format!(
            "{} {}/{}",
            if party_move_view {
                "PARTY".to_string()
            } else {
                format!("BOX {} {}", box_snapshot.index, box_snapshot.name)
            },
            visible_slots.len(),
            if party_move_view { 6 } else { 20 }
        ),
        SCENE_DIALOG_TEXT_CHARS,
    ));
    entries.push(compact_scene_label(
        if runtime_shell.bill_pc_move_save.is_some() {
            "Saving… Leave ON!"
        } else if runtime_shell.party_menu_open {
            "A DEPOSIT  B CLOSE"
        } else {
            "A WITHDRAW SELECT RELEASE"
        },
        SCENE_DIALOG_TEXT_CHARS,
    ));
    if visible_slots.is_empty() && !runtime_shell.bill_pc_move_open {
        entries.push("EMPTY".to_string());
        return;
    }
    let option_count = if runtime_shell.bill_pc_move_open {
        if runtime_shell.bill_pc_move_source.is_some() {
            visible_slots.len() + 1
        } else {
            visible_slots.len().max(1)
        }
    } else {
        visible_slots.len()
    };
    let cursor_index =
        strict_readonly_cursor_index(&runtime_shell.storage_cursor, &surface_id, option_count);
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
            let Some((slot_index, pokemon)) =
                visible_slots.iter().find(|(index, _)| *index == offset)
            else {
                return compact_scene_label(
                    &format!("{marker}{offset} EMPTY"),
                    SCENE_DIALOG_TEXT_CHARS,
                );
            };
            let held = pokemon
                .item
                .as_deref()
                .map(|item| format!(" item={item}"))
                .unwrap_or_default();
            compact_scene_label(
                &format!(
                    "{marker}{} {} L{} HP {}/{}{}",
                    slot_index, pokemon.species.id, pokemon.level, pokemon.hp, pokemon.max_hp, held
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
) -> Result<()> {
    for vertical in &menu.layout.vertical_menus {
        let surface_id = vertical_menu_surface_id(menu, vertical);
        let cursor_index = strict_readonly_cursor_index(
            &runtime_shell.menu_cursor,
            &surface_id,
            vertical.options.len(),
        )
        .with_context(|| format!("runtime menu {surface_id} has no valid nonempty cursor"))?;
        if vertical.two_dimensional {
            let columns = vertical.columns.with_context(|| {
                format!("two-dimensional runtime menu {surface_id} has no column count")
            })?;
            anyhow::ensure!(
                columns > 0,
                "two-dimensional runtime menu {surface_id} has zero columns"
            );
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
            return Ok(());
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
            return Ok(());
        }
    }
    Ok(())
}

fn push_visible_shop_dialog_entries(
    entries: &mut Vec<String>,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
) -> Result<()> {
    if !runtime_shell.shop_welcome_seen {
        entries.push("Welcome! How may I".to_string());
        entries.push("help you?".to_string());
        return Ok(());
    }
    if let Some(notice) = runtime_shell.shop_notice.as_deref() {
        entries.extend(notice.lines().map(str::to_string));
        return Ok(());
    }
    if let Some(cursor) = runtime_shell.shop_top_cursor.as_ref() {
        let selected = strict_readonly_cursor_index(&Some(cursor.clone()), "shop:top", 3)
            .context("shop top menu is open without a valid shop:top cursor")?;
        entries.push(format_price(snapshot.trainer.money));
        entries.extend(
            ["BUY", "SELL", "QUIT"]
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    format!("{}{}", if index == selected { ">" } else { " " }, option)
                }),
        );
        return Ok(());
    }
    if runtime_shell.sell_cursor.is_some() {
        let sellable = sellable_carried_item_ids(snapshot);
        entries.push(compact_scene_label(
            &format!("SELL {}", format_price(snapshot.trainer.money)),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        let cursor_index =
            strict_readonly_cursor_index(&runtime_shell.sell_cursor, "sell:bag", sellable.len())
                .context("shop sell menu is open without a valid nonempty cursor")?;
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
        return Ok(());
    }
    let surface_id = shop_cursor_surface_id(shop);
    let cursor_index = strict_readonly_cursor_index(
        &runtime_shell.menu_cursor,
        &surface_id,
        shop.inventory.len(),
    );
    entries.push(compact_scene_label(
        &format!("BUY {}", format_price(snapshot.trainer.money)),
        SCENE_DIALOG_TEXT_CHARS,
    ));
    let cursor_index = cursor_index.with_context(|| {
        format!("shop buy menu is open without a valid nonempty cursor for {surface_id}")
    })?;
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
    Ok(())
}

fn spawn_shell_status_banner(commands: &mut Commands, runtime_shell: &BevyRuntimeShell) {
    let Some(message) = shell_status_banner_text(runtime_shell) else {
        return;
    };
    spawn_shell_banner(commands, message, Color::srgb(0.98, 0.96, 0.82));
}

fn spawn_shell_error_banner(commands: &mut Commands, runtime_shell: &BevyRuntimeShell) {
    let Some(error) = &runtime_shell.last_error else {
        return;
    };
    spawn_shell_banner(
        commands,
        format!("ERR {}", compact_scene_label(error, 72)),
        Color::srgb(1.0, 0.82, 0.68),
    );
}

fn spawn_shell_banner(commands: &mut Commands, message: String, text_color: Color) {
    let origin_x = PLAYFIELD_LEFT + TILE_SIZE * 0.75;
    let origin_y = PLAYFIELD_TOP + TILE_SIZE * 0.45;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(0.03, 0.04, 0.05, 0.82),
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
                    color: Color::srgb(0.82, 0.94, 1.0),
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
                    color: Color::srgb(0.92, 0.96, 0.86),
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
                    color: Color::srgb(0.78, 0.88, 0.96),
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
    if let Some(printer) = runtime_shell.visible_unown_printer.as_ref() {
        let form = if printer.selected < 26 {
            char::from(b'A' + printer.selected).to_string()
        } else {
            "VACANT".to_string()
        };
        return Ok(vec![
            " ALPH RUINS STAMP".to_string(),
            form,
            "A PRINT".to_string(),
            "B CANCEL".to_string(),
            "← PREVIOUS".to_string(),
            "→ NEXT".to_string(),
            "Do what?".to_string(),
        ]);
    }
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
        )
        .context("Kurt Apricorn selection has no valid cursor")?;
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
        )
        .context("Kurt Apricorn selection has no valid cursor")?;
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
    if runtime_shell.decoration_menu.is_some() {
        return visible_decoration_command_entries(runtime_shell);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        let actions = visible_player_pc_actions(runtime_shell);
        let selected = strict_readonly_cursor_index(
            &runtime_shell.player_pc_action_cursor,
            "pc:player-actions",
            actions.len(),
        )
        .with_context(|| {
            format!(
                "player PC action cursor is invalid for {} actions",
                actions.len()
            )
        })?;
        return Ok(actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                format!(
                    "{}{}",
                    if index == selected { ">" } else { " " },
                    visible_player_pc_action_label(*action)
                )
            })
            .collect());
    }
    if runtime_shell.mailbox_cursor.is_some() {
        let mut entries = Vec::new();
        push_visible_mailbox_dialog_entries(&mut entries, snapshot, runtime_shell)?;
        return Ok(entries);
    }
    if runtime_shell.start_menu_cursor.is_some() {
        return visible_start_menu_entries(runtime_shell);
    }
    if visible_field_pack_is_open(runtime_shell) {
        return visible_field_pack_entries(snapshot, runtime_shell);
    }
    if runtime_shell.party_menu_open {
        return visible_party_menu_entries(snapshot, runtime_shell);
    }
    if runtime_shell.pokedex_menu_open {
        return visible_pokedex_menu_entries(snapshot, runtime_shell);
    }
    if runtime_shell.pokegear_menu_open {
        return visible_pokegear_menu_entries(snapshot, runtime_shell);
    }
    if runtime_shell.trainer_card_open {
        return Ok(visible_trainer_card_entries(snapshot, runtime_shell));
    }
    if runtime_shell.options_menu_open {
        return visible_options_menu_entries(snapshot, runtime_shell);
    }
    if runtime_shell.save_menu_open {
        return Ok(Vec::new());
    }
    if runtime_shell.special_boundary.is_some() {
        return Ok(visible_special_boundary_entries(runtime_shell));
    }
    visible_field_idle_entries(snapshot, runtime_shell)
}

fn visible_decoration_command_entries(
    runtime_shell: &BevyRuntimeShell,
) -> Result<Vec<String>> {
    let phase = &runtime_shell
        .decoration_menu
        .as_ref()
        .context("decoration entries require an active menu")?
        .phase;
    let (labels, selected) = match phase {
        VisibleDecorationMenuPhase::Categories { categories, cursor } => (
            categories
                .iter()
                .map(|category| visible_decoration_category_label(*category).to_string())
                .chain(std::iter::once("EXIT".to_string()))
                .collect::<Vec<_>>(),
            cursor.option_index,
        ),
        VisibleDecorationMenuPhase::Decorations {
            decorations,
            cursor,
            ..
        } => (
            decorations
                .iter()
                .map(|decoration| decoration.display_name.clone())
                .chain(["PUT IT AWAY".to_string(), "CANCEL".to_string()])
                .collect::<Vec<_>>(),
            cursor.option_index,
        ),
        VisibleDecorationMenuPhase::Side { cursor, .. } => (
            vec![
                "RIGHT SIDE".to_string(),
                "LEFT SIDE".to_string(),
                "CANCEL".to_string(),
            ],
            cursor.option_index,
        ),
    };
    anyhow::ensure!(selected < labels.len(), "decoration menu cursor is invalid");
    Ok(labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| format!("{}{}", if index == selected { ">" } else { " " }, label))
        .collect())
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
    let mut entries: Vec<String> =
        windowed_index_range(selected, crate::core::models::MAX_PC_BOXES)
            .map(|index| -> Result<String> {
                let pc_box = snapshot
                    .storage
                    .boxes
                    .iter()
                    .find(|pc_box| pc_box.index == index)
                    .with_context(|| format!("Bill's PC box {index} is missing from storage"))?;
                Ok(compact_scene_label(
                    &format!(
                        "{}{} {} {}/{}",
                        if index == selected { ">" } else { " " },
                        index + 1,
                        pc_box.name,
                        pc_box.count,
                        crate::core::models::MAX_BOX_MONS
                    ),
                    SCENE_DIALOG_TEXT_CHARS,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
    if let Some(cursor) = runtime_shell.bill_pc_box_action_cursor.as_ref() {
        let action = strict_readonly_cursor_index(&Some(cursor.clone()), "pc:bill-box-actions", 4)
            .context("Bill's PC box action cursor is invalid")?;
        entries.extend(
            ["SWITCH", "NAME", "PRINT", "QUIT"]
                .iter()
                .enumerate()
                .map(|(index, label)| format!("{}{}", if index == action { ">" } else { " " }, label)),
        );
    }
    Ok(entries)
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
