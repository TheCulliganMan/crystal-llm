fn parse_rgb_values(line: &str) -> Result<Vec<u8>> {
    line.replace("RGB", "")
        .replace("rgb", "")
        .replace(',', " ")
        .split_whitespace()
        .map(|value| {
            value
                .parse::<u8>()
                .with_context(|| format!("parse palette component '{value}'"))
        })
        .collect()
}

fn rgb_triplet_to_u8(values: &[u8]) -> Result<[u8; 3]> {
    if values.len() != 3 {
        anyhow::bail!("RGB triplet requires 3 components, got {}", values.len());
    }
    Ok([
        normalize_palette_component(values[0]),
        normalize_palette_component(values[1]),
        normalize_palette_component(values[2]),
    ])
}

fn normalize_palette_component(value: u8) -> u8 {
    if value <= 31 {
        (value << 3) | (value >> 2)
    } else {
        value
    }
}

fn palette_index_from_gray(gray: u8) -> usize {
    if gray >= 213 {
        0
    } else if gray >= 128 {
        1
    } else if gray >= 43 {
        2
    } else {
        3
    }
}

fn tileset_collision_tokens<'a>(
    tileset: &'a crate::RuntimeTilesetCatalogSnapshot,
    block: u16,
) -> Option<&'a [String]> {
    // Exported ASM metatile ids are canonical lowercase hexadecimal (for
    // example block 15 is keyed as "0f", not decimal "15" or "0F").
    let key = format!("{block:02x}");
    tileset.collision.get(&key).map(|tokens| tokens.as_slice())
}

fn spawn_object_label(
    commands: &mut Commands,
    object: &crate::core::map::ObjectEvent,
    x: f32,
    y: f32,
) {
    let label = compact_scene_label(&object_scene_label(object), 18);
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 10.5,
                    color: Color::srgb(0.98, 0.98, 0.86),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(x - TILE_SIZE * 0.62, y + TILE_SIZE * 0.78, 2.6),
            ..default()
        },
        ObjectMarker,
    ));
}

fn object_scene_label(object: &crate::core::map::ObjectEvent) -> String {
    match visible_object_kind(object) {
        VisibleObjectKind::ItemBall | VisibleObjectKind::Trainer => object
            .object_identifier
            .as_ref()
            .or(object.label.as_ref())
            .cloned()
            .unwrap_or_else(|| object.script.clone()),
        VisibleObjectKind::Script => object
            .label
            .as_ref()
            .or(object.object_identifier.as_ref())
            .cloned()
            .unwrap_or_else(|| object.sprite.clone()),
        VisibleObjectKind::Invalid => object
            .object_identifier
            .as_ref()
            .or(object.label.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("INVALID {}", object.object_type)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleObjectKind {
    ItemBall,
    Trainer,
    Script,
    Invalid,
}

fn visible_object_kind(object: &crate::core::map::ObjectEvent) -> VisibleObjectKind {
    match object.object_type.as_str() {
        "OBJECTTYPE_ITEMBALL" => VisibleObjectKind::ItemBall,
        "OBJECTTYPE_TRAINER" => VisibleObjectKind::Trainer,
        "OBJECTTYPE_SCRIPT" => VisibleObjectKind::Script,
        _ => VisibleObjectKind::Invalid,
    }
}

fn spawn_bg_event_label(
    commands: &mut Commands,
    start_x: i16,
    start_y: i16,
    bg: &crate::core::map::BackgroundEvent,
) {
    let Some(tile) = background_event_tile_position_checked(bg) else {
        return;
    };
    let Some((view_x, view_y)) = runtime_event_view_tile(tile, start_x, start_y) else {
        return;
    };
    if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
        return;
    }
    let label = compact_scene_label(&bg.script, 18);
    let (tile_x, tile_y) = render_tile_playfield_position(view_x, view_y);
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 9.5,
                    color: Color::srgb(0.92, 0.92, 0.78),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(
                tile_x - TILE_SIZE * 0.56,
                tile_y + TILE_SIZE * 0.52,
                2.4,
            ),
            ..default()
        },
        EventMarker,
    ));
}

fn spawn_warp_event_label(
    commands: &mut Commands,
    start_x: i16,
    start_y: i16,
    warp: &crate::core::map::WarpEvent,
) {
    let Some(tile) = warp_tile_position_checked(warp) else {
        return;
    };
    let Some((view_x, view_y)) = runtime_event_view_tile(tile, start_x, start_y) else {
        return;
    };
    if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
        return;
    }
    let label = compact_scene_label(
        &format!("-> {}#{}", warp.target_map, warp.target_warp_id),
        18,
    );
    let (tile_x, tile_y) = render_tile_playfield_position(view_x, view_y);
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 9.5,
                    color: Color::srgb(0.72, 0.84, 1.0),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(
                tile_x - TILE_SIZE * 0.58,
                tile_y - TILE_SIZE * 0.58,
                2.4,
            ),
            ..default()
        },
        EventMarker,
    ));
}

fn spawn_coord_event_label(
    commands: &mut Commands,
    start_x: i16,
    start_y: i16,
    coord: &crate::core::map::CoordEvent,
) {
    let Some(tile) = coord_event_tile_position_checked(coord) else {
        return;
    };
    let Some((view_x, view_y)) = runtime_event_view_tile(tile, start_x, start_y) else {
        return;
    };
    if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
        return;
    }
    let label = compact_scene_label(&coord.script_name, 18);
    let (tile_x, tile_y) = render_tile_playfield_position(view_x, view_y);
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 9.5,
                    color: Color::srgb(0.90, 0.74, 1.0),
                    ..default()
                },
            ),
            transform: Transform::from_xyz(tile_x - TILE_SIZE * 0.58, tile_y, 2.4),
            ..default()
        },
        EventMarker,
    ));
}

fn compact_scene_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    label
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>()
        + "~"
}

fn canonical_species_display_name(species_id: &str) -> String {
    species_id.replace('_', " ")
}

fn push_wrapped_scene_dialog_lines(entries: &mut Vec<String>, text: &str) {
    for line in text.split('\n') {
        if line.trim().is_empty() {
            entries.push(String::new());
            if entries.len() >= SCENE_MENU_VISIBLE_ROWS {
                return;
            }
            continue;
        }
        for wrapped in wrap_scene_dialog_line(line.trim(), SCENE_DIALOG_TEXT_CHARS) {
            entries.push(wrapped);
            if entries.len() >= SCENE_MENU_VISIBLE_ROWS {
                return;
            }
        }
    }
}

fn push_priority_scene_dialog_entry(entries: &mut Vec<String>, entry: String) {
    if entries.len() >= SCENE_MENU_VISIBLE_ROWS {
        entries.truncate(SCENE_MENU_VISIBLE_ROWS.saturating_sub(1));
    }
    entries.push(entry);
}

fn wrap_scene_dialog_line(line: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let current_len = current.chars().count();
        let word_len = word.chars().count();
        if current_len == 0 {
            if word_len <= max_chars {
                current.push_str(word);
            } else {
                push_split_dialog_word(&mut rows, word, max_chars);
            }
            continue;
        }
        if current_len + 1 + word_len <= max_chars {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        rows.push(std::mem::take(&mut current));
        if word_len <= max_chars {
            current.push_str(word);
        } else {
            push_split_dialog_word(&mut rows, word, max_chars);
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

fn push_split_dialog_word(rows: &mut Vec<String>, word: &str, max_chars: usize) {
    let mut chunk = String::new();
    for ch in word.chars() {
        if chunk.chars().count() >= max_chars {
            rows.push(std::mem::take(&mut chunk));
        }
        chunk.push(ch);
    }
    if !chunk.is_empty() {
        rows.push(chunk);
    }
}

fn spawn_event_marker(
    commands: &mut Commands,
    start_x: i16,
    start_y: i16,
    tile: TilePosition,
    color: Color,
    size: f32,
    z: f32,
) {
    let Some((x, y)) = runtime_tile_playfield_position(tile, start_x, start_y) else {
        return;
    };

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::new(size, size)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, z),
            ..default()
        },
        EventMarker,
    ));
}

fn runtime_event_view_tile(tile: TilePosition, start_x: i16, start_y: i16) -> Option<(i16, i16)> {
    let render_x = tile.x.checked_mul(RENDER_TILES_PER_RUNTIME_TILE)?;
    let render_y = tile.y.checked_mul(RENDER_TILES_PER_RUNTIME_TILE)?;
    Some((
        render_x.checked_sub(start_x)?,
        render_y.checked_sub(start_y)?,
    ))
}

fn overworld_object_in_scroll_region(view_x: i16, view_y: i16) -> bool {
    // The map surface carries one runtime tile beyond the LCD so a camera
    // step can reveal it continuously. Keep character OAM in that same region;
    // culling against only the settled LCD made edge characters appear or
    // disappear as a complete sprite at the end of the scroll.
    (-CLASSIC_SCROLL_HALO_TILES..VIEWPORT_TILES_X + CLASSIC_SCROLL_HALO_TILES)
        .contains(&view_x)
        && (-CLASSIC_SCROLL_HALO_TILES..VIEWPORT_TILES_Y + CLASSIC_SCROLL_HALO_TILES)
            .contains(&view_y)
}

fn runtime_tile_playfield_position(
    tile: TilePosition,
    start_x: i16,
    start_y: i16,
) -> Option<(f32, f32)> {
    let (view_x, view_y) = runtime_event_view_tile(tile, start_x, start_y)?;
    if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
        return None;
    }
    Some(render_tile_playfield_position(view_x, view_y))
}

fn render_tile_playfield_position(view_x: i16, view_y: i16) -> (f32, f32) {
    // PLAYFIELD_LEFT/TOP are the LCD edges. The composited map is centered on
    // the camera, so runtime coordinates address the centre of the first
    // 32x32 render tile one half-tile inward from those edges.
    (
        PLAYFIELD_LEFT + (view_x as f32 + 0.5) * TILE_SIZE,
        PLAYFIELD_TOP - (view_y as f32 + 0.5) * TILE_SIZE,
    )
}

fn overworld_sprite_position(view_x: i16, view_y: i16, sprite_size: Vec2) -> (f32, f32) {
    let (tile_x, tile_y) = render_tile_playfield_position(view_x, view_y);
    overworld_sprite_position_from_base(tile_x, tile_y, sprite_size)
}

fn overworld_sprite_position_from_base(
    tile_center_x: f32,
    tile_center_y: f32,
    sprite_size: Vec2,
) -> (f32, f32) {
    // Runtime map coordinates address the top-left render tile of an object.
    // Bevy sprites are center-origin, so translate from that 8x8 pixel tile to
    // the centre of the object's complete 16x16 (or larger) OAM footprint.
    (
        tile_center_x - TILE_SIZE * 0.5 + sprite_size.x * 0.5,
        tile_center_y + TILE_SIZE * 0.5 - sprite_size.y * 0.5,
    )
}

fn overworld_emote_position_from_base(
    tile_center_x: f32,
    tile_center_y: f32,
    emote_size: Vec2,
) -> (f32, f32) {
    // ASM positions the 16x16 emote immediately above the object's 16x16 OAM
    // footprint. Runtime coordinates point at the footprint's top-left 8x8
    // tile, while Bevy sprites use their centre, so account for both origins.
    (
        tile_center_x + TILE_SIZE * 0.5,
        tile_center_y + TILE_SIZE * 0.5 + emote_size.y * 0.5,
    )
}

fn render_viewport_origin(player_render_tile: i16, render_extent: i16, viewport_tiles: i16) -> i16 {
    let max_origin = render_extent.saturating_sub(viewport_tiles).max(0);
    player_render_tile
        .saturating_sub(viewport_tiles / 2)
        .clamp(0, max_origin)
}

fn runtime_tile_bounds_i16(map_name: &str, width: u16, height: u16) -> Result<(i16, i16)> {
    let metatile_width = i16::try_from(METATILE_WIDTH)
        .with_context(|| format!("map {map_name} has invalid runtime metatile width"))?;
    let runtime_width = i16::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(metatile_width))
        .with_context(|| {
            format!(
                "map {map_name} width {width} overflows supported runtime tile coordinate bounds"
            )
        })?;
    let runtime_height = i16::try_from(height)
        .ok()
        .and_then(|height| height.checked_mul(metatile_width))
        .with_context(|| {
            format!(
                "map {map_name} height {height} overflows supported runtime tile coordinate bounds"
            )
        })?;
    Ok((runtime_width, runtime_height))
}

fn render_tile_bounds_i16(map_name: &str, width: u16, height: u16) -> Result<(i16, i16)> {
    let render_metatile_width = RENDER_METATILE_WIDTH;
    let render_width = i16::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(render_metatile_width))
        .with_context(|| {
            format!(
                "map {map_name} width {width} overflows supported render tile coordinate bounds"
            )
        })?;
    let render_height = i16::try_from(height)
        .ok()
        .and_then(|height| height.checked_mul(render_metatile_width))
        .with_context(|| {
            format!(
                "map {map_name} height {height} overflows supported render tile coordinate bounds"
            )
        })?;
    Ok((render_width, render_height))
}

fn bevy_audio_action(kind: &RuntimeResolvedAudioPlaybackKind) -> Option<BevyAudioAction> {
    match kind {
        RuntimeResolvedAudioPlaybackKind::Play { audio_id, playback } => {
            Some(BevyAudioAction::Play(BevyAudioCommand {
                audio_id: audio_id.clone(),
                kind: playback.kind,
                mode: playback.mode,
                looped: matches!(
                    playback.loop_policy,
                    crate::assets::ModpackAudioLoopPolicy::Loop
                ),
            }))
        }
        RuntimeResolvedAudioPlaybackKind::FadeMusic {
            audio_id,
            fade_frames,
            ..
        } => Some(BevyAudioAction::FadeMusic {
            audio_id: audio_id.clone(),
            fade_frames: *fade_frames,
        }),
        RuntimeResolvedAudioPlaybackKind::WaitForSoundEffect => {
            Some(BevyAudioAction::WaitForSoundEffect)
        }
    }
}

fn play_pending_audio(
    mut commands: Commands,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    music_audio: Query<Entity, With<MusicAudioMarker>>,
    transient_audio: Query<Entity, With<TransientAudioMarker>>,
    #[cfg(all(not(test), not(target_arch = "wasm32")))] mut native_audio: NonSendMut<
        NativeAudioBackend,
    >,
    #[cfg(all(not(test), target_arch = "wasm32"))] mut browser_audio_unlocked: Local<bool>,
    #[cfg(all(not(test), target_arch = "wasm32"))] mut browser_audio: NonSendMut<
        BrowserAudioBackend,
    >,
    #[cfg(all(not(test), target_arch = "wasm32"))] keyboard: Res<ButtonInput<KeyCode>>,
    #[cfg(all(not(test), target_arch = "wasm32"))] mouse: Res<ButtonInput<MouseButton>>,
    #[cfg(all(not(test), target_arch = "wasm32"))] touches: Res<Touches>,
) {
    #[cfg(all(not(test), target_arch = "wasm32"))]
    if !*browser_audio_unlocked {
        let unlock_requested = browser_audio_unlock_requested(
            keyboard.get_just_pressed().next().is_some(),
            mouse.get_just_pressed().next().is_some(),
            touches.any_just_pressed(),
        );
        if !unlock_requested {
            // Keep the initial music/SFX queued until this system is running
            // in the browser's user-input event turn. Creating the WebAudio
            // stream before a gesture leaves its AudioContext suspended and
            // produces permanent silence even after the player clicks.
            return;
        }
        *browser_audio_unlocked = true;
    }
    #[cfg(all(not(test), not(target_arch = "wasm32")))]
    {
        runtime_shell.transient_audio_playing = !native_audio.transient_finished();
    }
    #[cfg(all(not(test), target_arch = "wasm32"))]
    {
        runtime_shell.transient_audio_playing = !browser_audio.transient_finished();
    }
    // Bevy queries are snapshots for the duration of a system invocation, so
    // a newly spawned music entity is not visible to `music_audio.iter()`
    // later in this same loop. Track the entities we spawn locally as well;
    // otherwise two queued music transitions can overlap for an entire song.
    let mut active_music_entities = music_audio.iter().collect::<Vec<_>>();
    let mut active_transient_entities = transient_audio.iter().collect::<Vec<_>>();
    if runtime_shell.pending_music_stop {
        for entity in active_music_entities.drain(..) {
            commands.entity(entity).despawn();
        }
        // A map/title/battle transition is an audio boundary.  Do not let a
        // queued cry or sound effect from the previous surface continue over
        // the replacement track; that was the source of audible multi-song
        // and stale-cue overlap on macOS.
        for entity in active_transient_entities.drain(..) {
            commands.entity(entity).despawn();
        }
        #[cfg(all(not(test), not(target_arch = "wasm32")))]
        {
            native_audio.stop_music();
            native_audio.stop_transient();
        }
        #[cfg(all(not(test), target_arch = "wasm32"))]
        {
            browser_audio.stop_music();
            browser_audio.stop_transient();
        }
        runtime_shell.transient_audio_playing = false;
        runtime_shell.pending_music_stop = false;
    }

    let pending =
        coalesce_pending_transient_audio(std::mem::take(&mut runtime_shell.pending_audio));
    for command in pending {
        if matches!(command.kind, ModpackAudioKind::Music) {
            // Stop before decoding/starting the replacement.  The previous
            // implementation stopped the native sink in `play`, but only
            // despawned Bevy marker entities after starting; queued map/title
            // transitions could therefore leave multiple logical tracks
            // alive and audible on backends that process commands lazily.
            #[cfg(all(not(test), not(target_arch = "wasm32")))]
            native_audio.stop_music();
            #[cfg(all(not(test), target_arch = "wasm32"))]
            browser_audio.stop_music();
        }
        if !matches!(command.kind, ModpackAudioKind::Music) {
            for entity in active_transient_entities.drain(..) {
                commands.entity(entity).despawn();
            }
            #[cfg(all(not(test), not(target_arch = "wasm32")))]
            native_audio.stop_transient();
            #[cfg(all(not(test), target_arch = "wasm32"))]
            browser_audio.stop_transient();
            runtime_shell.transient_audio_playing = false;
        }
        let cache_key = BevyAudioCacheKey::from_command(&command);
        #[cfg_attr(test, allow(unused_variables))]
        let audio = if let Some(audio) = runtime_shell.audio_source_cache.get(&cache_key) {
            audio.clone()
        } else {
            let source = match match command.kind {
                ModpackAudioKind::Music => runtime_shell
                    .shell
                    .runtime()
                    .audio()
                    .require_music(&command.audio_id),
                ModpackAudioKind::SoundEffect => runtime_shell
                    .shell
                    .runtime()
                    .audio()
                    .require_sound_effect(&command.audio_id),
                ModpackAudioKind::Cry => runtime_shell
                    .shell
                    .runtime()
                    .audio()
                    .require_cry(&command.audio_id),
            }
            .map(|program| program.source.clone())
            {
                Ok(source) => source,
                Err(error) => {
                    clear_failed_music_playback_state(&mut runtime_shell, &command);
                    record_visible_runtime_system_error(
                        &mut runtime_shell,
                        anyhow::anyhow!(
                            "queued {:?} audio program {} is missing from verified pack: {error:#}",
                            command.kind,
                            command.audio_id
                        ),
                    );
                    continue;
                }
            };
            #[cfg(all(not(test), not(target_arch = "wasm32")))]
            let decoded_pcm =
                match native_audio.poll_preparation(cache_key.clone(), command.clone(), source) {
                    Ok(Some(audio)) => Ok(audio),
                    Ok(None) => {
                        if !matches!(command.kind, ModpackAudioKind::Music) {
                            runtime_shell.transient_audio_playing = true;
                        }
                        runtime_shell.pending_audio.push(command);
                        continue;
                    }
                    Err(error) => Err(error),
                };
            #[cfg(any(test, target_arch = "wasm32"))]
            let decoded_pcm = match source {
                AudioProgramSource::Pcm {
                    bytes,
                    format,
                    loop_start_sample,
                    loop_end_sample,
                } => decoded_pcm_audio(&command, bytes, format, loop_start_sample, loop_end_sample),
                AudioProgramSource::PcmGzip {
                    bytes,
                    format,
                    byte_len,
                    payload_hash,
                    loop_start_sample,
                    loop_end_sample,
                } => decoded_gzip_pcm_audio(
                    &command,
                    &bytes,
                    format,
                    byte_len,
                    &payload_hash,
                    loop_start_sample,
                    loop_end_sample,
                ),
                AudioProgramSource::PcmGzipSidecar {
                    path,
                    format,
                    byte_len,
                    payload_hash,
                    loop_start_sample,
                    loop_end_sample,
                } => {
                    #[cfg(all(not(test), target_arch = "wasm32"))]
                    {
                        match browser_audio.poll_pcm_sidecar(&path) {
                            Ok(Some(bytes)) => decoded_gzip_pcm_audio(
                                &command,
                                &bytes,
                                format,
                                byte_len,
                                &payload_hash,
                                loop_start_sample,
                                loop_end_sample,
                            ),
                            Ok(None) => {
                                if !matches!(command.kind, ModpackAudioKind::Music) {
                                    runtime_shell.transient_audio_playing = true;
                                }
                                runtime_shell.pending_audio.push(command);
                                continue;
                            }
                            Err(error) => Err(error),
                        }
                    }
                    #[cfg(any(test, not(target_arch = "wasm32")))]
                    {
                        let _ = (
                            format,
                            byte_len,
                            payload_hash,
                            loop_start_sample,
                            loop_end_sample,
                        );
                        Err(anyhow::anyhow!(
                            "PCM sidecar {path} requires browser on-demand loading"
                        ))
                    }
                }
                AudioProgramSource::Midi(_) => Err(anyhow::anyhow!(
                    "audio program {} is not canonical PCM",
                    command.audio_id
                )),
            };
            let decoded_pcm = match decoded_pcm {
                Ok(audio) => audio,
                Err(error) => {
                    clear_failed_music_playback_state(&mut runtime_shell, &command);
                    record_visible_runtime_system_error(
                        &mut runtime_shell,
                        anyhow::anyhow!("audio program {} failed: {error:#}", command.audio_id),
                    );
                    continue;
                }
            };
            runtime_shell
                .audio_source_cache
                .insert(cache_key, decoded_pcm.clone());
            decoded_pcm
        };
        #[cfg(all(not(test), not(target_arch = "wasm32")))]
        {
            if let Err(error) = native_audio.play(&command, &audio) {
                clear_failed_music_playback_state(&mut runtime_shell, &command);
                eprintln!(
                    "crystal-bevy audio failed {:?} {}: {error:#}",
                    command.kind, command.audio_id
                );
                record_visible_runtime_system_error(
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "native audio playback failed for {:?} {}: {error:#}",
                        command.kind,
                        command.audio_id
                    ),
                );
            } else if !matches!(command.kind, ModpackAudioKind::Music) {
                runtime_shell.transient_audio_playing = true;
            }
        }
        #[cfg(all(not(test), target_arch = "wasm32"))]
        {
            if let Err(error) = browser_audio.play(&command, &audio) {
                clear_failed_music_playback_state(&mut runtime_shell, &command);
                record_visible_runtime_system_error(
                    &mut runtime_shell,
                    anyhow::anyhow!(
                        "browser PCM playback failed for {:?} {}: {error:#}",
                        command.kind,
                        command.audio_id
                    ),
                );
                continue;
            } else if !matches!(command.kind, ModpackAudioKind::Music) {
                runtime_shell.transient_audio_playing = true;
            }
        }
        let previous_music_entities = if matches!(command.kind, ModpackAudioKind::Music) {
            std::mem::take(&mut active_music_entities)
        } else {
            Vec::new()
        };
        let mut entity_commands = if matches!(command.kind, ModpackAudioKind::Music) {
            for entity in active_transient_entities.drain(..) {
                commands.entity(entity).despawn();
            }
            #[cfg(all(not(test), not(target_arch = "wasm32")))]
            native_audio.stop_transient();
            #[cfg(all(not(test), target_arch = "wasm32"))]
            browser_audio.stop_transient();
            runtime_shell.transient_audio_playing = false;
            commands.spawn(MusicAudioMarker)
        } else {
            commands.spawn(TransientAudioMarker)
        };
        let new_audio_entity = entity_commands.id();
        entity_commands.insert(Name::new(format!(
            "audio::{:?}::{}",
            command.kind, command.audio_id
        )));
        drop(entity_commands);
        // Queue the replacement before removing the old marker.  Bevy can
        // recycle entity IDs when commands are flushed; despawning first can
        // accidentally cancel the replacement spawn in the same update.
        for entity in previous_music_entities {
            commands.entity(entity).despawn();
        }
        if matches!(command.kind, ModpackAudioKind::Music) {
            active_music_entities.push(new_audio_entity);
        } else {
            active_transient_entities.push(new_audio_entity);
        }
        runtime_shell.last_audio_events.push(format!(
            "played {:?} {} mode={:?} looped={}",
            command.kind, command.audio_id, command.mode, command.looped
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
    }
}

fn browser_audio_unlock_requested(
    keyboard_just_pressed: bool,
    mouse_just_pressed: bool,
    touch_just_pressed: bool,
) -> bool {
    keyboard_just_pressed || mouse_just_pressed || touch_just_pressed
}

fn clear_failed_music_playback_state(
    runtime_shell: &mut BevyRuntimeShell,
    command: &BevyAudioCommand,
) {
    if matches!(command.kind, ModpackAudioKind::Music)
        && runtime_shell.active_music.as_deref() == Some(command.audio_id.as_str())
    {
        runtime_shell.active_music = None;
        runtime_shell.faded_music = None;
    }
}

fn decoded_pcm_audio(
    command: &BevyAudioCommand,
    bytes: Vec<u8>,
    format: AudioPcmFormat,
    loop_start_sample: Option<usize>,
    loop_end_sample: Option<usize>,
) -> Result<CachedPcmAudio> {
    if command.mode != ModpackAudioPlaybackMode::RawPcm {
        anyhow::bail!(
            "audio program {} declared PCM but queued as {:?}",
            command.audio_id,
            command.mode
        );
    }
    if format.sample_rate_hz == 0 {
        anyhow::bail!("PCM sample_rate_hz must be positive");
    }
    if format.channels == 0 {
        anyhow::bail!("PCM channels must be positive");
    }
    if format.bits_per_sample != 16 {
        anyhow::bail!("canonical PCM bits_per_sample must be 16");
    }
    let block_align = usize::from(format.channels) * 2;
    if bytes.is_empty() || bytes.len() % block_align != 0 {
        anyhow::bail!("PCM byte length is not aligned to frame size");
    }
    let frame_count = bytes.len() / block_align;
    let loop_range = match (loop_start_sample, loop_end_sample) {
        (Some(start), Some(end)) if start < end && end <= frame_count => Some((start, end)),
        (None, None) => None,
        (Some(start), Some(end)) => {
            anyhow::bail!("PCM loop range [{start}, {end}) is outside {frame_count} frames")
        }
        _ => anyhow::bail!("PCM source has unpaired loop metadata"),
    };
    Ok(CachedPcmAudio {
        samples: bytes
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect::<Vec<_>>()
            .into(),
        bytes: bytes.into(),
        format,
        loop_range,
    })
}

fn decoded_gzip_pcm_audio(
    command: &BevyAudioCommand,
    compressed: &[u8],
    format: AudioPcmFormat,
    byte_len: usize,
    payload_hash: &str,
    loop_start_sample: Option<usize>,
    loop_end_sample: Option<usize>,
) -> Result<CachedPcmAudio> {
    if command.mode != ModpackAudioPlaybackMode::RawPcm {
        anyhow::bail!(
            "audio program {} declared compressed PCM but queued as {:?}",
            command.audio_id,
            command.mode
        );
    }
    use flate2::read::GzDecoder;
    let mut decoder = GzDecoder::new(compressed);
    let mut decoded = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut decoded)
        .with_context(|| format!("decompress PCM audio {}", command.audio_id))?;
    if decoded.len() != byte_len || format!("{:08x}", bevy_audio_fnv1a32(&decoded)) != payload_hash
    {
        anyhow::bail!(
            "compressed PCM audio {} failed metadata validation",
            command.audio_id
        );
    }
    decoded_pcm_audio(command, decoded, format, loop_start_sample, loop_end_sample)
}

fn pcm_i16_samples(audio: &CachedPcmAudio) -> Result<Arc<[i16]>> {
    if audio.format.bits_per_sample != 16 || audio.bytes.len() % 2 != 0 {
        anyhow::bail!("canonical PCM payload is not aligned signed 16-bit data");
    }
    Ok(Arc::clone(&audio.samples))
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
fn decoded_audio_program_source(
    command: &BevyAudioCommand,
    source: AudioProgramSource,
) -> Result<CachedPcmAudio> {
    match source {
        AudioProgramSource::Pcm {
            bytes,
            format,
            loop_start_sample,
            loop_end_sample,
        } => decoded_pcm_audio(command, bytes, format, loop_start_sample, loop_end_sample),
        AudioProgramSource::PcmGzip {
            bytes,
            format,
            byte_len,
            payload_hash,
            loop_start_sample,
            loop_end_sample,
        } => decoded_gzip_pcm_audio(
            command,
            &bytes,
            format,
            byte_len,
            &payload_hash,
            loop_start_sample,
            loop_end_sample,
        ),
        AudioProgramSource::PcmGzipSidecar { path, .. } => Err(anyhow::anyhow!(
            "PCM sidecar {path} requires browser on-demand loading"
        )),
        AudioProgramSource::Midi(_) => Err(anyhow::anyhow!(
            "audio program {} is not canonical PCM",
            command.audio_id
        )),
    }
}

fn bitmap_font_glyph_pixel(r: u8, g: u8, b: u8, alpha: u8) -> bool {
    alpha > 0 && (u16::from(r) + u16::from(g) + u16::from(b)) <= 600
}

fn bevy_audio_fnv1a32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c9dc5u32, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x01000193)
    })
}

fn refresh_status_text(
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mode: Res<HudMode>,
    mut query: Query<&mut Text, With<StatusText>>,
    mut last_key: Local<Option<(u64, bool, u64)>>,
) {
    let mut text = query.single_mut();
    let debug_overlays = runtime_debug_overlays_enabled();
    let animation_key = visible_overlay_animation_key(&runtime_shell);
    let key = (
        runtime_shell.snapshot_revision,
        debug_overlays,
        animation_key,
    );
    if *last_key == Some(key) {
        return;
    }
    *last_key = Some(key);
    if !debug_overlays {
        if !text.sections[0].value.is_empty() {
            text.sections[0].value.clear();
        }
        return;
    }
    if runtime_shell.intro_screen.is_some() {
        text.sections[0].value = format_intro_status(&runtime_shell);
        return;
    }
    if runtime_shell.credits_screen.is_some() {
        text.sections[0].value = format_credits_status(&runtime_shell);
        return;
    }
    if runtime_shell.pending_time_set.is_some() {
        text.sections[0].value = format_time_set_status(&runtime_shell);
        return;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        text.sections[0].value = format_oak_intro_status(&runtime_shell);
        return;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        text.sections[0].value = format_gender_status(&runtime_shell);
        return;
    }
    if runtime_shell.title_menu.is_some() {
        text.sections[0].value = format_title_status(&runtime_shell);
        return;
    }
    text.sections[0].value = match cached_runtime_snapshot(&mut runtime_shell) {
        Ok(snapshot) => format_snapshot(&snapshot, &runtime_shell, *mode),
        Err(error) => format!("snapshot error: {error}"),
    };
}

fn refresh_dialog_text(
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut query: Query<&mut Text, With<DialogText>>,
    mut last_key: Local<Option<(u64, bool, u64)>>,
) {
    let mut text = query.single_mut();
    let debug_overlays = runtime_debug_overlays_enabled();
    let animation_key = visible_overlay_animation_key(&runtime_shell);
    let key = (
        runtime_shell.snapshot_revision,
        debug_overlays,
        animation_key,
    );
    if *last_key == Some(key) {
        return;
    }
    *last_key = Some(key);
    if !debug_overlays {
        if !text.sections[0].value.is_empty() {
            text.sections[0].value.clear();
        }
        return;
    }
    if runtime_shell.intro_screen.is_some() {
        text.sections[0].value = format_intro_dialog(&runtime_shell);
        return;
    }
    if runtime_shell.credits_screen.is_some() {
        text.sections[0].value = format_credits_dialog(&runtime_shell);
        return;
    }
    if runtime_shell.pending_time_set.is_some() {
        text.sections[0].value = format_time_set_dialog_overlay(&runtime_shell);
        return;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        text.sections[0].value = format_oak_intro_dialog_overlay(&runtime_shell);
        return;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        text.sections[0].value = format_gender_dialog(&runtime_shell);
        return;
    }
    if runtime_shell.title_menu.is_some() {
        text.sections[0].value = format_title_dialog(&runtime_shell);
        return;
    }
    text.sections[0].value = match cached_runtime_snapshot(&mut runtime_shell) {
        Ok(snapshot) => format_dialog_overlay(&snapshot, &runtime_shell),
        Err(_) => String::new(),
    };
}

fn refresh_battle_text(
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut query: Query<&mut Text, With<BattleText>>,
    mut last_key: Local<Option<(u64, bool, u64)>>,
) {
    let mut text = query.single_mut();
    let debug_overlays = runtime_debug_overlays_enabled();
    let animation_key = visible_overlay_animation_key(&runtime_shell);
    let key = (
        runtime_shell.snapshot_revision,
        debug_overlays,
        animation_key,
    );
    if *last_key == Some(key) {
        return;
    }
    *last_key = Some(key);
    if !debug_overlays {
        if !text.sections[0].value.is_empty() {
            text.sections[0].value.clear();
        }
        return;
    }
    if runtime_shell.intro_screen.is_some()
        || runtime_shell.title_menu.is_some()
        || runtime_shell.credits_screen.is_some()
        || runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
    {
        text.sections[0].value = String::new();
        return;
    }
    text.sections[0].value = match cached_runtime_snapshot(&mut runtime_shell) {
        Ok(snapshot) => format_battle_overlay(&snapshot, &runtime_shell),
        Err(_) => String::new(),
    };
}

/// Host rendering runs much more frequently than the emulated Game Boy frame.
/// These overlays are diagnostic-only, so keep their own tiny animation key
/// and avoid formatting strings or touching the cached snapshot on every host
/// frame. Gameplay mutations advance `snapshot_revision`; title/intro/credits
/// animations are the only surfaces that need an additional clock value.
fn visible_overlay_animation_key(runtime_shell: &BevyRuntimeShell) -> u64 {
    let mut key = 0u64;
    if let Some(title) = runtime_shell.title_menu.as_ref() {
        key = key.wrapping_add(u64::from(title.frame));
        key = key
            .wrapping_mul(31)
            .wrapping_add(u64::from(title.main_menu_frame));
    }
    if let Some(intro) = runtime_shell.intro_screen.as_ref() {
        key = key
            .wrapping_mul(31)
            .wrapping_add(u64::from(intro.scene_frame_counter));
    }
    if let Some(credits) = runtime_shell.credits_screen.as_ref() {
        key = key.wrapping_mul(31).wrapping_add(u64::from(credits.frame));
    }
    key
}

fn refresh_shell_panels(
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut dialog_panels: Query<&mut Sprite, With<DialogPanel>>,
    mut battle_panels: Query<&mut Sprite, (With<BattlePanel>, Without<DialogPanel>)>,
    mut last_revision: Local<Option<(u64, bool)>>,
) {
    // Panel visibility is driven by semantic runtime mutations.  Bevy can
    // run this system hundreds of times between two Game Boy frames; avoid
    // taking a runtime snapshot and walking field-command entries on those
    // idle host frames.  The shell increments snapshot_revision for every
    // action that can change the visible UI, so this remains exact at the
    // gameplay boundary while making the idle path constant-time.
    let debug_overlays = runtime_debug_overlays_enabled();
    let revision_key = (runtime_shell.snapshot_revision, debug_overlays);
    if *last_revision == Some(revision_key) {
        return;
    }
    *last_revision = Some(revision_key);
    let (dialog_visible, battle_visible) = if runtime_shell.intro_screen.is_some()
        || runtime_shell.title_menu.is_some()
        || runtime_shell.credits_screen.is_some()
        || runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
    {
        (false, false)
    } else {
        match cached_runtime_snapshot(&mut runtime_shell) {
            Ok(snapshot) => {
                let dialog_visible = runtime_debug_overlays_enabled()
                    && (scene_dialog_surface_active(&snapshot, &runtime_shell)
                        || !visible_field_command_entries(&snapshot, &runtime_shell)
                            .unwrap_or_default()
                            .is_empty());
                let battle_visible = runtime_debug_overlays_enabled()
                    && (snapshot.battle.is_some()
                        || (!runtime_shell.battle_messages.is_empty()
                            && runtime_shell.battle_message_scene.is_some()));
                (dialog_visible, battle_visible)
            }
            Err(_) => (false, false),
        }
    };
    for mut sprite in &mut dialog_panels {
        sprite.color = if dialog_visible {
            Color::srgba(0.06, 0.08, 0.10, 0.90)
        } else {
            Color::srgba(0.06, 0.08, 0.10, 0.0)
        };
    }
    for mut sprite in &mut battle_panels {
        sprite.color = if battle_visible {
            Color::srgba(0.07, 0.09, 0.12, 0.82)
        } else {
            Color::srgba(0.07, 0.09, 0.12, 0.0)
        };
    }
}

fn format_intro_status(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(intro) = &runtime_shell.intro_screen else {
        return String::new();
    };
    [
        "Intro".to_string(),
        "STATE: intro".to_string(),
        format!(
            "SCENE INDEX: {}/{}",
            intro
                .jumptable_index
                .saturating_add(1)
                .min(VISIBLE_INTRO_SCENE_NAMES.len()),
            VISIBLE_INTRO_SCENE_NAMES.len()
        ),
        format!("SCENE FRAME: {}", intro.scene_frame_counter),
        format!("SPRITES: {}", intro.sprite_count),
        format!("SCROLL: x={} y={}", intro.scroll_x, intro.scroll_y),
        format!("FINISHED: {}", if intro.finished { "yes" } else { "no" }),
        "A/START/SELECT/B=Skip intro".to_string(),
    ]
    .join("\n")
}

fn format_intro_dialog(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(intro) = &runtime_shell.intro_screen else {
        return String::new();
    };
    [
        "CRYSTAL INTRO".to_string(),
        format!("SCENE: {}", intro.scene_name()),
    ]
    .join("\n")
}

fn format_title_status(runtime_shell: &BevyRuntimeShell) -> String {
    let boot = runtime_shell.runtime.boot_summary();
    let mut lines = vec![
        "Pokemon Crystal".to_string(),
        "Rust".to_string(),
        format!("pack={} hash={}", boot.modpack_id, boot.pack_content_hash),
    ];
    if let Some(title) = &runtime_shell.title_menu {
        lines.push(format!("new_game_spawn={}", title.spawn_identifier));
        lines.push(format!("phase={:?}", title.phase));
        lines.push(format!("frame={}", title.frame));
        lines.push(format!("scx={}", title.scx));
        lines.push(format!("title_timer={}", title.title_timer));
        if let Some(save_path) = &title.save_path {
            lines.push(format!(
                "continue_verified={} path={}",
                title_continue_save_path(runtime_shell, title).is_some(),
                save_path.to_string_lossy()
            ));
            lines.extend(visible_title_continue_entries(runtime_shell, title));
        }
    }
    lines.join("\n")
}

fn format_title_dialog(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(title) = &runtime_shell.title_menu else {
        return String::new();
    };
    if !visible_title_main_menu_ready(title) {
        return visible_title_menu_entries(runtime_shell, title)
            .unwrap_or_else(|_| vec!["TITLE SCREEN".to_string()])
            .join("\n");
    }
    let title_options = visible_title_menu_options(runtime_shell, title);
    let selected_title_option = title.cursor.option_index.min(title_options.len() - 1);
    let options = title_options
        .into_iter()
        .enumerate()
        .map(|(index, option)| {
            let label = match option {
                TitleMenuOption::Continue => "CONTINUE",
                TitleMenuOption::NewGame => "NEW GAME",
                TitleMenuOption::Options => "OPTION",
                TitleMenuOption::MysteryGift => "MYSTERY GIFT",
            };
            if index == selected_title_option {
                format!("> {label}")
            } else {
                format!("  {label}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut lines = vec![options];
    if title.save_path.is_some() {
        lines.extend(visible_title_continue_entries(runtime_shell, title));
    }
    lines.join("\n")
}

fn format_gender_status(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(gender) = &runtime_shell.pending_gender_selection else {
        return String::new();
    };
    [
        "Gender".to_string(),
        format!(
            "selected={}",
            visible_gender_label(visible_gender_selected_gender(gender))
        ),
        format!("confirmed={}", gender.confirmed),
        format!("confirm_countdown={}", gender.confirm_countdown),
        format!("fade_counter={}", gender.fade_counter),
    ]
    .join("\n")
}

fn format_gender_dialog(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(gender) = &runtime_shell.pending_gender_selection else {
        return String::new();
    };
    let mut lines = vec![
        "ARE YOU A BOY?".to_string(),
        "OR ARE YOU A GIRL?".to_string(),
    ];
    lines.extend(visible_gender_entries(gender));
    lines.join("\n")
}

fn format_time_set_status(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(time_set) = &runtime_shell.pending_time_set else {
        return String::new();
    };
    [
        "Time Set".to_string(),
        format!("phase={:?}", time_set.phase),
        format!("wake_index={}", time_set.wake_index),
        format!("hour={}", time_set.hour),
        format!("minute={}", time_set.minute),
        format!("yes_no={}", time_set.yes_no_index),
        format!("visible_chars={}", time_set.visible_chars),
    ]
    .join("\n")
}

fn format_time_set_dialog_overlay(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(time_set) = &runtime_shell.pending_time_set else {
        return String::new();
    };
    let mut lines = Vec::new();
    let dialog = format_time_set_dialog(time_set);
    if !dialog.is_empty() {
        lines.push(dialog);
    }
    lines.extend(format_time_set_menu(time_set));
    lines.join("\n")
}

fn format_oak_intro_status(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(oak_intro) = &runtime_shell.pending_oak_intro else {
        return String::new();
    };
    [
        "Oak Intro".to_string(),
        format!("mode={:?}", oak_intro.mode),
        format!("scene_index={}", oak_intro.scene_index),
        format!("scene={}", oak_intro.scene_state),
        format!(
            "sprite={}",
            oak_intro.current_sprite.as_deref().unwrap_or("NONE")
        ),
        format!("waiting={}", oak_intro.waiting_for_input),
        format!("visible_chars={}", oak_intro.visible_chars),
    ]
    .join("\n")
}

fn format_oak_intro_dialog_overlay(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(oak_intro) = &runtime_shell.pending_oak_intro else {
        return String::new();
    };
    let mut lines = vec![
        match oak_intro.mode {
            VisibleOakIntroMode::Intro => "OAK INTRO".to_string(),
            VisibleOakIntroMode::Final => "OAK FINALE".to_string(),
        },
        format!(
            "SPRITE: {}",
            oak_intro.current_sprite.as_deref().unwrap_or("NONE")
        ),
    ];
    let dialog = format_oak_intro_dialog(oak_intro);
    if !dialog.is_empty() {
        lines.push(dialog);
    }
    lines.join("\n")
}

fn format_oak_intro_dialog(oak_intro: &VisibleOakIntroSequence) -> String {
    visible_oak_intro_visible_dialog(oak_intro)
}

fn format_time_set_dialog(time_set: &VisibleTimeSetScreen) -> String {
    match time_set.phase {
        VisibleTimeSetPhase::SetHour => "What time is it?".to_string(),
        VisibleTimeSetPhase::SetMinute => "How many minutes?".to_string(),
        _ => visible_time_set_visible_dialog(time_set),
    }
}

fn format_time_set_menu(time_set: &VisibleTimeSetScreen) -> Vec<String> {
    match time_set.phase {
        VisibleTimeSetPhase::SetHour => vec![
            "^".to_string(),
            visible_time_set_hour_display(time_set),
            "v".to_string(),
        ],
        VisibleTimeSetPhase::SetMinute => vec![
            "^".to_string(),
            visible_time_set_minute_display(time_set),
            "v".to_string(),
        ],
        VisibleTimeSetPhase::HourConfirm | VisibleTimeSetPhase::MinuteConfirm => {
            visible_time_set_yes_no_entries(time_set)
        }
        _ => Vec::new(),
    }
}

fn format_credits_status(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(credits) = &runtime_shell.credits_screen else {
        return String::new();
    };
    [
        "Credits".to_string(),
        format!("frame={}", credits.frame),
        format!("allow_skip={}", credits.allow_skip),
        format!("can_skip={}", visible_credits_can_skip(credits)),
        format!("awaiting_exit={}", credits.awaiting_exit),
        format!(
            "music={}",
            runtime_shell.active_music.as_deref().unwrap_or("NONE")
        ),
    ]
    .join("\n")
}

fn format_credits_dialog(runtime_shell: &BevyRuntimeShell) -> String {
    let Some(credits) = &runtime_shell.credits_screen else {
        return String::new();
    };
    visible_credits_screen_lines(credits).join("\n")
}

fn format_dialog_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let mut lines = Vec::new();
    if let Some(text) = &snapshot.ui.text {
        lines.push(format!("{}:", text.label));
        if let Some(asm_text) = &text.asm_text {
            lines.push(asm_text.clone());
        } else if let Some(body) = &text.body {
            for rendered in render_script_text_body(body, &snapshot.script_events.named_buffers)
                .lines()
                .take(4)
            {
                lines.push(rendered.to_string());
            }
        }
    }
    if let Some(prompt) = &snapshot.ui.pending_yes_no {
        lines.push(format!(
            "yes/no {}:{}",
            prompt.source_script, prompt.command_index
        ));
        if let Some(selected) =
            strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
        {
            let yes = if selected == 0 { "> YES" } else { "  YES" };
            let no = if selected == 1 { "> NO" } else { "  NO" };
            lines.push(format!("{yes} / {no}"));
        } else {
            lines.push("invalid_cursor=yes_no".to_string());
        }
    }
    if let Some(wait) = &snapshot.ui.pending_text_wait {
        lines.push(format!(
            "wait {}:{} {}",
            wait.source_script, wait.command_index, wait.command
        ));
    }
    if let Some(menu) = &snapshot.ui.menu {
        lines.push(format!("menu {}", menu.menu_id));
        for vertical in &menu.layout.vertical_menus {
            let surface_id = vertical_menu_surface_id(menu, vertical);
            let Some(cursor_index) = strict_readonly_cursor_index(
                &runtime_shell.menu_cursor,
                &surface_id,
                vertical.options.len(),
            ) else {
                lines.push(format!("invalid_cursor=menu surface={surface_id}"));
                continue;
            };
            let options = vertical
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    if index == cursor_index {
                        format!("> {option}")
                    } else {
                        format!("  {option}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" / ");
            lines.push(options);
        }
    }
    if let Some(shop) = &snapshot.pending_shop {
        let surface_id = shop_cursor_surface_id(shop);
        let cursor_index = strict_readonly_cursor_index(
            &runtime_shell.menu_cursor,
            &surface_id,
            shop.inventory.len(),
        );
        let mode = if runtime_shell.sell_cursor.is_some() {
            "sell"
        } else {
            "buy"
        };
        lines.push(format!(
            "shop {} {} mode={mode}",
            shop.mart_type, shop.mart_id
        ));
        if let Some(cursor_index) = cursor_index {
            let inventory = shop
                .inventory
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    if index == cursor_index {
                        format!("> {item}")
                    } else {
                        format!("  {item}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" / ");
            lines.push(inventory);
        } else {
            lines.push(format!("invalid_cursor=shop surface={surface_id}"));
        }
        let sellable = sellable_carried_item_ids(snapshot);
        if !sellable.is_empty() {
            if let Some(sell_cursor) =
                strict_readonly_cursor_index(&runtime_shell.sell_cursor, "sell:bag", sellable.len())
            {
                let sell_options = sellable
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        if index == sell_cursor {
                            format!("> {item}")
                        } else {
                            format!("  {item}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" / ");
                lines.push(format!("sell {sell_options}"));
            } else {
                lines.push("invalid_cursor=sell surface=sell:bag".to_string());
            }
        }
    }
    append_visible_shell_surface_overlay(snapshot, runtime_shell, &mut lines);
    if lines.is_empty() && runtime_debug_overlays_enabled() {
        append_field_overlay(snapshot, runtime_shell, &mut lines);
    }
    lines.join("\n")
}

fn append_visible_shell_surface_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    append_start_menu_context(runtime_shell, lines);
    append_party_menu_context(snapshot, runtime_shell, lines);
    append_field_pack_context(snapshot, runtime_shell, lines);
    append_pokedex_context(snapshot, runtime_shell, lines);
    append_pokegear_context(snapshot, runtime_shell, lines);
    append_options_context(snapshot, runtime_shell, lines);
    append_save_context(snapshot, runtime_shell, lines);
    append_special_boundary_context(runtime_shell, lines);
}

fn render_script_text_body(
    body: &crate::core::systems::script_text::ScriptTextBody,
    named_buffers: &BTreeMap<String, String>,
) -> String {
    let mut lines = Vec::new();
    let mut current = String::new();
    for command in &body.commands {
        match command.command.as_str() {
            "text" | "text_start" | "text_today" => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current.push_str(&render_script_text_args(&command.args));
            }
            "text_ram" => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current.push_str(&render_named_text_buffer(
                    command.args.first().map(String::as_str),
                    named_buffers,
                ));
            }
            "text_decimal" => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current.push_str(&render_named_text_buffer(
                    command.args.first().map(String::as_str),
                    named_buffers,
                ));
            }
            "text_block" | "next" | "line" | "cont" | "para" | "text_far" => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current.push_str(&render_script_text_args(&command.args));
            }
            "text_promptbutton" | "prompt" | "done" | "text_end" => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
            }
            "text_asm" => {}
            opcode if opcode.starts_with("sound_") => {}
            _ => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                let args = command.args.join(" ");
                if args.is_empty() {
                    lines.push(format!("[{}]", command.command));
                } else {
                    lines.push(format!("[{} {args}]", command.command));
                }
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.retain(|line| !line.is_empty());
    lines.join("\n")
}

fn render_visible_script_text_body(
    body: &crate::core::systems::script_text::ScriptTextBody,
    named_buffers: &BTreeMap<String, String>,
    player_name: &str,
    rival_name: &str,
    day_of_week: u8,
) -> String {
    render_visible_script_text_pages(body, named_buffers, player_name, rival_name, day_of_week)
        .join("\n\n")
}

/// Interpret the source text macros as page/line boundaries before applying
/// host-side word wrapping.  Flattening `para` into a newline made a whole
/// multi-page message render in one field box and pushed the final glyphs
/// through the bottom frame.
pub(super) fn render_visible_script_text_pages(
    body: &crate::core::systems::script_text::ScriptTextBody,
    named_buffers: &BTreeMap<String, String>,
    player_name: &str,
    rival_name: &str,
    day_of_week: u8,
) -> Vec<String> {
    let mut pages = Vec::new();
    let mut lines = Vec::new();
    let mut current = String::new();
    let flush_line = |lines: &mut Vec<String>, current: &mut String| {
        if !current.is_empty() {
            lines.push(std::mem::take(current));
        }
    };
    let flush_page = |pages: &mut Vec<String>, lines: &mut Vec<String>, current: &mut String| {
        flush_line(lines, current);
        if !lines.is_empty() {
            pages.push(normalize_visible_script_text_with_context(
                &lines.join("\n"),
                player_name,
                rival_name,
                day_of_week,
            ));
            lines.clear();
        }
    };
    for command in &body.commands {
        let rendered = match command.command.as_str() {
            "text_ram" | "text_decimal" => {
                render_named_text_buffer(command.args.first().map(String::as_str), named_buffers)
            }
            _ => render_script_text_args(&command.args),
        };
        match command.command.as_str() {
            "text" | "text_start" | "text_block" | "text_ram" | "text_decimal" => {
                // These commands all write at the current ASM text cursor.
                // Inserting a host newline between a text_ram buffer and the
                // following punctuation produced impossible extra rows.
                current.push_str(&rendered);
            }
            "text_today" => current.push_str("<TODAY>"),
            "text_pause" | "text_asm" => {}
            "text_low" => {
                // TX_LOW relocates BC to the standard bottom baseline.  It
                // prints nothing, but subsequent chunks must not concatenate
                // onto the top baseline's text.
                flush_line(&mut lines, &mut current);
            }
            // The field renderer resolves TX_FAR through the runtime text
            // catalog before calling this interpreter. Its pointer label is
            // never player-visible text.
            "text_far" => {}
            "line" | "next" => {
                flush_line(&mut lines, &mut current);
                // `<LINE>` and `<NEXT>` both add two tile rows in
                // `home/text.asm` (`LineChar` and `NextLineChar`). The visual
                // renderer owns that two-tile baseline spacing; inserting a
                // blank logical line here created four apparent text rows.
                current.push_str(&rendered);
            }
            "para" => {
                flush_page(&mut pages, &mut lines, &mut current);
                current.push_str(&rendered);
            }
            "cont" => {
                // `<CONT>` prompts, scrolls the box up two rows, then starts
                // the continuation at the third interior row. Keep the last
                // displayed row as the new top row. The visual renderer owns
                // the two-tile baseline spacing.
                flush_line(&mut lines, &mut current);
                let carry = lines.iter().rev().find(|line| !line.is_empty()).cloned();
                if !lines.is_empty() {
                    pages.push(normalize_visible_script_text_with_context(
                        &lines.join("\n"),
                        player_name,
                        rival_name,
                        day_of_week,
                    ));
                }
                lines.clear();
                if let Some(carry) = carry {
                    lines.push(carry);
                }
                current.push_str(&rendered);
            }
            "text_promptbutton" | "prompt" | "done" | "text_end" => {
                flush_page(&mut pages, &mut lines, &mut current);
            }
            opcode if opcode.starts_with("sound_") => {}
            _ => {
                flush_line(&mut lines, &mut current);
                if rendered.is_empty() {
                    lines.push(format!("[{}]", command.command));
                } else {
                    lines.push(format!("[{} {rendered}]", command.command));
                }
            }
        }
    }
    flush_page(&mut pages, &mut lines, &mut current);
    if pages.is_empty() {
        pages.push(String::new());
    }
    pages
}

fn render_script_text_args(args: &[String]) -> String {
    args.iter()
        // `@` is the charmap string terminator, not a visible glyph. Text
        // bodies use it for empty cursor moves (`line "@"`) and to end a
        // chunk after punctuation (`text "!@"`).
        .map(|arg| arg.trim().trim_matches('"').trim_end_matches('@'))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn normalize_visible_script_text(text: &str, player_name: &str) -> String {
    normalize_visible_script_text_with_context(text, player_name, "RIVAL", 0)
}

pub(super) fn normalize_visible_script_text_with_context(
    text: &str,
    player_name: &str,
    rival_name: &str,
    day_of_week: u8,
) -> String {
    const DAY_NAMES: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
    normalize_boot_text(text)
        .replace("<PLAYER>", player_name)
        .replace("<PLAY_G>", player_name)
        .replace("<RIVAL>", rival_name)
        .replace("<TODAY>", DAY_NAMES[usize::from(day_of_week % 7)])
}

fn render_visible_asm_text_pages(
    text: &str,
    named_buffers: &BTreeMap<String, String>,
    player_name: &str,
    rival_name: &str,
    day_of_week: u8,
) -> Vec<String> {
    let mut resolved = String::with_capacity(text.len());
    let mut remainder = text;
    while let Some(start) = remainder.find('<') {
        resolved.push_str(&remainder[..start]);
        let token = &remainder[start..];
        let Some(end) = token.find('>') else {
            resolved.push_str(token);
            remainder = "";
            break;
        };
        let token_body = &token[1..end];
        let buffer_id = token_body
            .strip_prefix("RAM:")
            .or_else(|| {
                token_body
                    .strip_prefix("DECIMAL:")
                    .and_then(|arguments| arguments.split(',').next())
                    .map(str::trim)
            })
            .unwrap_or(token_body);
        if token_body.starts_with("STRING_BUFFER_")
            || token_body.starts_with("RAM:")
            || token_body.starts_with("DECIMAL:")
        {
            if let Some(value) = named_buffers.get(buffer_id) {
                resolved.push_str(value);
            }
        } else {
            resolved.push_str(&token[..=end]);
        }
        remainder = &token[end + 1..];
    }
    resolved.push_str(remainder);

    normalize_visible_script_text_with_context(&resolved, player_name, rival_name, day_of_week)
        .split("\n\n")
        .map(str::to_owned)
        .collect()
}

pub(super) fn visible_rival_name(snapshot: &RuntimeShellSnapshot) -> &str {
    snapshot
        .script_events
        .variables
        .get("_rival_name")
        .map(String::as_str)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("RIVAL")
}

fn render_named_text_buffer(
    buffer_id: Option<&str>,
    named_buffers: &BTreeMap<String, String>,
) -> String {
    buffer_id
        .and_then(|buffer_id| named_buffers.get(buffer_id))
        .cloned()
        .unwrap_or_default()
}

fn append_field_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "{}  tile=({}, {}) facing={:?}",
        snapshot.overworld.map_name,
        snapshot.overworld.tile.x,
        snapshot.overworld.tile.y,
        snapshot.overworld.facing
    ));
    let selected_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    if let Some(lead) = snapshot.party.slots.first() {
        lines.push(format!(
            "lead {} L{} HP {}/{} status={:?}",
            lead.pokemon.species.id,
            lead.pokemon.level,
            lead.pokemon.hp,
            lead.pokemon.max_hp,
            lead.pokemon.status
        ));
    }
    if selected_slot > 0 {
        if let Some(selected) = snapshot.party.slots.get(selected_slot) {
            lines.push(format!(
                "selected {} {} L{} HP {}/{} status={:?}",
                selected.index,
                selected.pokemon.species.id,
                selected.pokemon.level,
                selected.pokemon.hp,
                selected.pokemon.max_hp,
                selected.pokemon.status
            ));
        }
    }
    append_party_roster_overlay(snapshot, selected_slot, lines);
    append_fly_destination_overlay(snapshot, runtime_shell, lines);
    if snapshot.progression.repel_steps_remaining > 0 {
        lines.push(format!(
            "repel {} steps via {:?}",
            snapshot.progression.repel_steps_remaining, snapshot.progression.active_repel_item
        ));
    }
    let has_wild = snapshot
        .encounters
        .wild
        .contains_key(&snapshot.overworld.map_name);
    let has_field = snapshot
        .encounters
        .field
        .contains_key(&snapshot.overworld.map_name);
    lines.push(format!(
        "music={:?} encounters wild={} field={}",
        snapshot.audio.current_music, has_wild, has_field
    ));
    lines.push(format!(
        "bag items={} balls={} key={} tm_hm={} custom={}",
        snapshot.bag.items.len(),
        snapshot.bag.balls.len(),
        snapshot.bag.key_items.len(),
        snapshot.bag.tm_hm.len(),
        snapshot.bag.custom_pockets.len()
    ));
    lines.push(format!(
        "money={} coins={} time={:?}",
        snapshot.trainer.money, snapshot.trainer.coins, snapshot.progression.time
    ));
    append_runtime_request_overlay(snapshot, runtime_shell, lines);
    match front_context_line(snapshot) {
        Ok(Some(front)) => lines.push(front),
        Ok(None) => {}
        Err(error) => lines.push(format!("front_context_error={error:#}")),
    }
    if let Ok(idle_entries) = visible_field_idle_entries(snapshot, runtime_shell)
        && !idle_entries.is_empty()
    {
        lines.push(format!("field_commands {}", idle_entries.join(" | ")));
    }
}

fn append_fly_destination_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let Ok(destinations) = active_fly_destinations(snapshot, &runtime_shell.shell) else {
        return;
    };
    if destinations.is_empty() {
        return;
    }
    let selected_index = strict_readonly_cursor_index(
        &runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    );
    let Some(selected_index) = selected_index else {
        lines.push("INVALID CURSOR fly:destinations".to_string());
        return;
    };
    let destination = &destinations[selected_index];
    let label = fly_destination_label(destination);
    lines.push(format!(
        "fly {}/{} {} flag={}",
        selected_index + 1,
        destinations.len(),
        label,
        destination.flypoint_flag
    ));
}

fn append_party_roster_overlay(
    snapshot: &RuntimeShellSnapshot,
    selected_slot: usize,
    lines: &mut Vec<String>,
) {
    if snapshot.party.slots.len() <= 1 {
        return;
    }
    let summary = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(slot_offset, slot)| {
            let marker = if slot_offset == selected_slot {
                ">"
            } else {
                ""
            };
            format!(
                "{}{}:{} {}/{}",
                marker, slot.index, slot.pokemon.species.id, slot.pokemon.hp, slot.pokemon.max_hp
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push(format!("party {summary}"));
}

fn append_runtime_request_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let scripts = &snapshot.script_events;
    if scripts.hall_of_fame_requested {
        lines.push("request=hall_of_fame".to_string());
    }
    if scripts.credits_requested {
        lines.push("request=credits".to_string());
    }
    if scripts.reset_requested {
        lines.push("request=reset".to_string());
    }
    if let Some(picture) = &snapshot.ui.active_pokemon_picture {
        lines.push(format!("pokemon_picture={picture}"));
    }
    if runtime_shell.elevator_cursor.is_some() {
        let elevator_options = visible_elevator_prompt_options(snapshot, runtime_shell);
        let option_count = visible_elevator_option_count(snapshot, runtime_shell);
        if option_count > 0 {
            let Some(cursor) = runtime_shell.elevator_cursor.as_ref() else {
                lines.push("invalid_cursor=elevator missing_cursor".to_string());
                return;
            };
            let Some(selected) = strict_readonly_cursor_index(
                &runtime_shell.elevator_cursor,
                &cursor.surface_id,
                option_count,
            ) else {
                lines.push(format!(
                    "invalid_cursor=elevator surface={}",
                    cursor.surface_id
                ));
                return;
            };
            let mut offset = 0usize;
            for (elevator_index, elevator) in elevator_options.iter().enumerate() {
                let next_offset = offset + elevator.floors.len();
                if selected < next_offset {
                    let floor_index = selected - offset;
                    lines.push(format!(
                        "elevator_selected={}/{} {} floor={}/{} {}",
                        elevator_index + 1,
                        elevator_options.len(),
                        elevator.data_label,
                        floor_index + 1,
                        elevator.floors.len(),
                        elevator.floors[floor_index].floor
                    ));
                    break;
                }
                offset = next_offset;
            }
        } else if let Some(cursor) = runtime_shell.elevator_cursor.as_ref() {
            lines.push(format!(
                "invalid_cursor=elevator surface={}",
                cursor.surface_id
            ));
        }
    }
    if !scripts.completed_trades.is_empty() {
        lines.push(format!("completed_trades={:?}", scripts.completed_trades));
    }
}

fn front_context_line(snapshot: &RuntimeShellSnapshot) -> Result<Option<String>> {
    let TilePosition {
        x: front_x,
        y: front_y,
    } = facing_runtime_tile(snapshot)?;
    for object in &snapshot.visible_objects {
        if snapshot_object_tile_matches_checked(
            snapshot,
            object,
            TilePosition::new(front_x, front_y),
        )? {
            return Ok(Some(format!(
                "front object {:?} sprite={} script={}",
                object.object_identifier, object.sprite, object.script
            )));
        }
    }
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name);
    let Some(map) = map else {
        return Ok(None);
    };
    for bg in &map.events.bg_events {
        if background_event_tile_matches_checked(bg, TilePosition::new(front_x, front_y))? {
            return Ok(Some(format!(
                "front {} script={}",
                bg.event_type, bg.script
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
            return Ok(Some(format!(
                "standing warp {} -> {}#{}",
                warp.index, warp.target_map, warp.target_warp_id
            )));
        }
    }
    Ok(None)
}

fn format_battle_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let Some(battle) = &snapshot.battle else {
        return String::new();
    };
    let active_player = battle
        .active_player_party_index
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index));
    let mut lines = vec![
        format!("{:?} {}", battle.kind, battle.battle_type),
        format!(
            "Enemy {} L{} HP {}/{}",
            battle.enemy_pokemon.species.id,
            battle.enemy_pokemon.level,
            battle.enemy_pokemon.hp,
            battle.enemy_pokemon.max_hp
        ),
    ];
    if let Some(slot) = active_player {
        lines.push(format!(
            "Player {} L{} HP {}/{}",
            slot.pokemon.species.id, slot.pokemon.level, slot.pokemon.hp, slot.pokemon.max_hp
        ));
        let selected_move_slot = readonly_cursor_index(
            &runtime_shell.battle_move_cursor,
            "battle:moves",
            battle.player_moves.len() + 1,
        )
        .filter(|cursor_index| *cursor_index < battle.player_moves.len());
        let moves = battle
            .player_moves
            .iter()
            .enumerate()
            .map(|(index, learned)| {
                let marker = if Some(index) == selected_move_slot {
                    ">"
                } else {
                    ""
                };
                format!(
                    "{}{} {} pp={} up={}",
                    marker,
                    index + 1,
                    learned.name,
                    learned.current_pp,
                    learned.pp_ups
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        if !moves.is_empty() {
            lines.push(moves);
        }
        if let Some(move_index) = readonly_cursor_index(
            &runtime_shell.battle_move_cursor,
            "battle:moves",
            battle.player_moves.len() + 1,
        ) {
            if let Some(selected_move) = battle.player_moves.get(move_index) {
                lines.push(format!(
                    "battle_move target={}/{} slot={} move={}",
                    move_index + 1,
                    battle.player_moves.len() + 1,
                    move_index + 1,
                    selected_move.name
                ));
            }
        }
    }
    let selected_party_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    if let Some(selected) = snapshot.party.slots.get(selected_party_slot) {
        lines.push(format!(
            "Selected party {} {} HP {}/{}",
            selected.index,
            selected.pokemon.species.id,
            selected.pokemon.hp,
            selected.pokemon.max_hp
        ));
    }
    let player_party = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(slot_offset, slot)| {
            format!(
                "{}{}{}:{} {}/{}",
                if slot_offset == selected_party_slot {
                    ">"
                } else {
                    ""
                },
                if slot.is_active_battle_pokemon {
                    "*"
                } else {
                    ""
                },
                slot.index,
                slot.pokemon.species.id,
                slot.pokemon.hp,
                slot.pokemon.max_hp
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    if !player_party.is_empty() {
        lines.push("Player party".to_string());
        for row in split_overlay_row(&player_party, 2) {
            lines.push(row);
        }
    }
    if let Some(switch_index) = readonly_cursor_index(
        &runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(snapshot),
    ) {
        let selected_slot = snapshot.party.slots.get(switch_index);
        let selected_party_index = selected_slot.map(|slot| slot.index);
        let selected_species = selected_slot
            .map(|slot| slot.pokemon.species.id.as_str())
            .unwrap_or("CANCEL");
        lines.push(format!(
            "battle_switch target={}/{} party_index={:?} species={} blocked_by_faint={}",
            switch_index + 1,
            battle_switch_option_count(snapshot),
            selected_party_index,
            selected_species,
            visible_active_battle_player_fainted(snapshot)
        ));
        if let (Some(move_slot), Some(selected_party_index)) = (
            runtime_shell.pending_battle_move_switch_slot,
            selected_party_index,
        ) {
            lines.push(format!(
                "battle_move_switch move_slot={} target_party_index={}",
                move_slot, selected_party_index
            ));
        }
    }
    if !battle.enemy_party.is_empty() {
        let enemy_party = battle
            .enemy_party
            .iter()
            .enumerate()
            .map(|(index, pokemon)| {
                let active = Some(index) == battle.active_enemy_party_index;
                let rewarded = battle.rewarded_enemy_party_indices.contains(&index);
                format!(
                    "{}{}:{} L{} {}/{}{}",
                    if active { "*" } else { "" },
                    index,
                    pokemon.species.id,
                    pokemon.level,
                    pokemon.hp,
                    pokemon.max_hp,
                    if rewarded { " done" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push("Enemy party".to_string());
        for row in split_overlay_row(&enemy_party, 2) {
            lines.push(row);
        }
    }
    lines.push(format!(
        "run={} escape_attempts={} guard={} switch={:?} items={} balls={}",
        battle.commands.can_run,
        battle.escape_attempts,
        battle.player_stat_drop_guard_turns,
        battle.commands.switch_party_indices,
        battle.commands.can_use_items,
        carried_ball_item_ids(snapshot).len()
    ));
    append_battle_cursor_context(snapshot, runtime_shell, &mut lines);
    lines.push(
        "controls arrows=battle action/item cursor | Z/A=select | X/B=cancel/back | Run action=flee | 1-4 direct move"
            .to_string(),
    );
    let ball_items = carried_ball_item_ids(snapshot);
    if !ball_items.is_empty() {
        let balls = ball_items
            .iter()
            .enumerate()
            .map(|(index, ball)| format!("{}:{}", index + 1, ball))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push("Balls".to_string());
        for row in split_overlay_row(&balls, 3) {
            lines.push(row);
        }
    }
    lines.push(format!(
        "move_slots player={:?} enemy={:?}",
        battle.commands.player_move_slots, battle.commands.enemy_move_slots
    ));
    lines.join("\n")
}

fn append_battle_cursor_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let battle = snapshot.battle.as_ref();
    let actions = battle
        .map(|battle| visible_battle_action_ids(snapshot, battle))
        .unwrap_or_default();
    let selected_action = if actions.is_empty() {
        "-".to_string()
    } else {
        readonly_cursor_index(
            &runtime_shell.battle_action_cursor,
            "battle:actions",
            actions.len(),
        )
        .and_then(|index| actions.get(index).copied())
        .map(visible_battle_action_label)
        .unwrap_or("INVALID")
        .to_string()
    };
    let battle_items = carried_battle_usable_item_ids(snapshot);
    let selected_item = if battle_items.is_empty() {
        "-"
    } else {
        readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "battle:bag-items",
            battle_items.len(),
        )
        .and_then(|index| battle_items.get(index))
        .map(|item| item.as_str())
        .unwrap_or("INVALID")
    };
    let ball_items = carried_ball_item_ids(snapshot);
    let selected_ball_index =
        readonly_cursor_index(&runtime_shell.ball_cursor, "bag:balls", ball_items.len());
    let selected_ball = selected_ball_index
        .and_then(|index| ball_items.get(index))
        .map(|ball| ball.as_str())
        .unwrap_or("-");
    let selected_move = battle
        .and_then(|battle| {
            readonly_cursor_index(
                &runtime_shell.battle_move_cursor,
                "battle:moves",
                battle.commands.player_move_slots.len(),
            )
            .and_then(|cursor_index| battle.commands.player_move_slots.get(cursor_index).copied())
        })
        .map(|slot| (slot + 1).to_string())
        .unwrap_or_else(|| "-".to_string());
    let selected_party_move = selected_party_move_name(snapshot, runtime_shell);
    if !actions.is_empty() {
        let action_summary = actions
            .iter()
            .map(|action| {
                let label = visible_battle_action_label(*action);
                if label == selected_action {
                    format!(">{label}")
                } else {
                    label.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("actions {action_summary}"));
    }
    lines.push(format!(
        "selected action={} move={} party_move={} item={} ball={}",
        selected_action, selected_move, selected_party_move, selected_item, selected_ball
    ));
    if let Some(battle) = battle {
        lines.push(selected_battle_action_detail(
            snapshot,
            runtime_shell,
            battle,
            selected_action.as_str(),
            selected_item,
        ));
    }
    if let Some(mode) = runtime_shell.battle_pack_target_mode {
        append_battle_pack_target_context(snapshot, runtime_shell, mode, lines);
    }
}

fn selected_battle_action_detail(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    selected_action: &str,
    selected_item: &str,
) -> String {
    match selected_action {
        "Fight" => {
            let cursor_index = strict_readonly_cursor_index(
                &runtime_shell.battle_move_cursor,
                "battle:moves",
                battle.commands.player_move_slots.len(),
            );
            let Some(cursor_index) = cursor_index else {
                return "action_detail fight invalid_cursor=battle:moves".to_string();
            };
            let Some(move_slot) = battle.commands.player_move_slots.get(cursor_index).copied()
            else {
                return format!("action_detail fight invalid_move_index={cursor_index}");
            };
            let Some(learned) = battle.player_moves.get(move_slot) else {
                return format!("action_detail fight missing_move_slot={move_slot}");
            };
            let move_row = move_menu_entry(snapshot, learned, "");
            format!("action_detail fight {move_row}")
        }
        "Pack" => {
            if selected_item == "-" {
                "action_detail pack empty".to_string()
            } else {
                format!(
                    "action_detail pack {}",
                    item_catalog_detail_label(snapshot, selected_item)
                )
            }
        }
        "Pokemon" => format!(
            "action_detail pokemon switch_targets={:?} active={:?}",
            battle.commands.switch_party_indices, battle.active_player_party_index
        ),
        "Run" => format!(
            "action_detail run allowed={} attempts={} enemy_level={}",
            battle.commands.can_run, battle.escape_attempts, battle.enemy_pokemon.level
        ),
        _ => format!("action_detail {selected_action}"),
    }
}

fn append_battle_pack_target_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    mode: BattlePackTargetMode,
    lines: &mut Vec<String>,
) {
    if let Some(label) = selected_battle_pack_item_label(snapshot, runtime_shell) {
        lines.push(format!("ITEM {label}"));
    }
    if snapshot.party.slots.is_empty() {
        lines.push(format!(
            "battle_pack_target {} empty party",
            battle_pack_target_mode_label(mode)
        ));
        return;
    }
    let selected = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let target_summary = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let marker = if index == selected { ">" } else { "" };
            format!(
                "{}{} L{} {}/{} {:?}",
                marker,
                slot.pokemon.species.id,
                slot.pokemon.level,
                slot.pokemon.hp,
                slot.pokemon.max_hp,
                slot.pokemon.status
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push(format!(
        "battle_pack_target {}",
        battle_pack_target_mode_label(mode)
    ));
    for row in split_overlay_row(&target_summary, 2) {
        lines.push(row);
    }
    if mode == BattlePackTargetMode::PartyMove {
        if let Some(selected_slot) = snapshot.party.slots.get(selected) {
            let selected_move = strict_readonly_cursor_index(
                &runtime_shell.party_move_cursor,
                &party_move_cursor_surface_id(selected_slot.index),
                selected_slot.pokemon.moves.len(),
            );
            let Some(selected_move) = selected_move else {
                lines.push(format!(
                    "INVALID CURSOR party:{}:moves",
                    selected_slot.index
                ));
                return;
            };
            let move_summary = selected_slot
                .pokemon
                .moves
                .iter()
                .enumerate()
                .map(|(index, learned)| {
                    let marker = if index == selected_move { ">" } else { "" };
                    format!("{marker}{}({}pp)", learned.name, learned.current_pp)
                })
                .collect::<Vec<_>>()
                .join(" | ");
            for row in split_overlay_row(&move_summary, 2) {
                lines.push(row);
            }
        }
    }
}

fn format_snapshot(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    mode: HudMode,
) -> String {
    let mut lines = vec![
        format!(
            "Pokemon Crystal Rust  pack={} hash={}",
            snapshot.boot.modpack_id, snapshot.boot.pack_content_hash
        ),
        format!(
            "frame={} phase={:?} checksum={:?}",
            snapshot.overworld.frame, snapshot.phase, snapshot.state_checksum
        ),
        format!(
            "map={} tile=({}, {}) facing={:?} mode={:?}",
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y,
            snapshot.overworld.facing,
            snapshot.overworld.mode
        ),
        format!(
            "player={} money={} coins={} badges={:?}",
            snapshot.trainer.player_name,
            snapshot.trainer.money,
            snapshot.trainer.coins,
            snapshot.progression.badges
        ),
        format!(
            "party={} bag={} balls={} key={} tm_hm={} registered={}",
            snapshot.party.slots.len(),
            snapshot.bag.items.len(),
            snapshot.bag.balls.len(),
            snapshot.bag.key_items.len(),
            snapshot.bag.tm_hm.len(),
            snapshot
                .progression
                .registered_key_item
                .as_deref()
                .unwrap_or("-")
        ),
        format!("hud_mode={mode:?}"),
    ];

    append_title_menu_context(runtime_shell, &mut lines);
    append_start_menu_context(runtime_shell, &mut lines);
    append_party_menu_context(snapshot, runtime_shell, &mut lines);
    append_field_pack_context(snapshot, runtime_shell, &mut lines);
    append_pokedex_context(snapshot, runtime_shell, &mut lines);
    append_pokegear_context(snapshot, runtime_shell, &mut lines);
    append_trainer_card_context(snapshot, runtime_shell, &mut lines);
    append_options_context(snapshot, runtime_shell, &mut lines);
    append_save_context(snapshot, runtime_shell, &mut lines);
    append_special_boundary_context(runtime_shell, &mut lines);

    match mode {
        HudMode::Status => format_status_details(snapshot, runtime_shell, &mut lines),
        HudMode::Party => format_party_details(snapshot, runtime_shell, &mut lines),
        HudMode::Bag => format_bag_details(snapshot, runtime_shell, &mut lines),
        HudMode::Battle => format_battle_details(snapshot, runtime_shell, &mut lines),
        HudMode::Ui => format_ui_details(snapshot, runtime_shell, &mut lines),
        HudMode::Progress => format_progress_details(snapshot, &mut lines),
        HudMode::Storage => format_storage_details(snapshot, runtime_shell, &mut lines),
        HudMode::Map => format_map_details(snapshot, runtime_shell, &mut lines),
        HudMode::Scripts => format_script_details(snapshot, runtime_shell, &mut lines),
        HudMode::Audio => format_audio_details(snapshot, runtime_shell, &mut lines),
        HudMode::Special => format_special_details(snapshot, &mut lines),
    }
    if let Some(input) = &runtime_shell.last_overworld_input {
        lines.push(format!(
            "last_input frame={} input_mask={:#010b} pressed_mask={:#010b} state_hash={:#010x} recent_overlay_inputs={}",
            input.frame,
            input.input_mask,
            input.pressed_mask,
            input.state_checksum.hash(),
            runtime_shell.recent_overworld_inputs.len()
        ));
    }
    let retained_journal_frame_count = snapshot
        .state_checksum
        .frame()
        .saturating_sub(runtime_shell.deterministic_session_start.frame());
    let checkpoint_details =
        runtime_shell
            .deterministic_session_checkpoint
            .as_ref()
            .map(|checkpoint| {
                (
                    checkpoint.session().session_id().to_string(),
                    checkpoint.checkpoint().summary().state_hash(),
                )
            });
    if let Some(input) = runtime_shell.deterministic_input_frames.back() {
        if let Some((session_id, checkpoint_hash)) = checkpoint_details {
            lines.push(format!(
                "deterministic_input session={} start_frame={} start_hash={:#010x} checkpoint_hash={:#010x} player={} frame={} joypad_mask={:#010b} retained_inputs={} retained_journal_frames={}",
                session_id,
                runtime_shell.deterministic_session_start.frame(),
                runtime_shell.deterministic_session_start.hash(),
                checkpoint_hash,
                input.player_id(),
                input.frame(),
                input.joypad_mask(),
                runtime_shell.deterministic_input_frames.len(),
                retained_journal_frame_count
            ));
        } else {
            lines.push(format!(
                "deterministic_input pending_identity start_frame={} start_hash={:#010x} player={} frame={} joypad_mask={:#010b} retained_inputs={} retained_journal_frames={}",
                runtime_shell.deterministic_session_start.frame(),
                runtime_shell.deterministic_session_start.hash(),
                input.player_id(),
                input.frame(),
                input.joypad_mask(),
                runtime_shell.deterministic_input_frames.len(),
                retained_journal_frame_count
            ));
        }
    } else {
        if let Some((session_id, checkpoint_hash)) = checkpoint_details {
            lines.push(format!(
                "deterministic_input session={} start_frame={} start_hash={:#010x} checkpoint_hash={:#010x} retained_inputs=0 retained_journal_frames={}",
                session_id,
                runtime_shell.deterministic_session_start.frame(),
                runtime_shell.deterministic_session_start.hash(),
                checkpoint_hash,
                retained_journal_frame_count
            ));
        } else {
            lines.push(format!(
                "deterministic_input pending_identity start_frame={} start_hash={:#010x} retained_inputs=0 retained_journal_frames={}",
                runtime_shell.deterministic_session_start.frame(),
                runtime_shell.deterministic_session_start.hash(),
                retained_journal_frame_count
            ));
        }
    }
    if let Some(action) = runtime_shell.deterministic_battle_actions.back() {
        lines.push(format!(
            "deterministic_battle player={} turn={} action={:?} state_hash={} retained_replay_actions={}",
            action.player_id(),
            action.turn(),
            action.action(),
            action.state_hash(),
            runtime_shell.deterministic_battle_actions.len()
        ));
    }
    if let Some(result) = runtime_shell.deterministic_menu_results.back() {
        lines.push(format!(
            "deterministic_menu_result player={} menu={} option={} choice_frame={} checksum_frame={} state_hash={:#010x} retained_replay_menu_results={}",
            result.choice().player_id(),
            result.choice().menu_id(),
            result.choice().option_index(),
            result.choice().frame(),
            result.checksum().frame(),
            result.checksum().hash(),
            runtime_shell.deterministic_menu_results.len()
        ));
    }
    if let Some(command) = runtime_shell.shell.retained_runtime_commands().last() {
        lines.push(format!(
            "deterministic_runtime_command player={} sequence={} schema={} expected_frame={} expected_hash={:#010x} retained_replay_commands={} retained_replay_results={}",
            command.player_id(),
            command.sequence(),
            command.payload().schema(),
            command.expected_state().frame(),
            command.expected_state().hash(),
            runtime_shell.shell.retained_runtime_commands().len(),
            runtime_shell.shell.retained_runtime_results().len()
        ));
    }
    if let Some(action) = &runtime_shell.last_runtime_action {
        lines.push(format!(
            "last_action {} frame={} state_hash={:#010x}",
            action.action, action.frame, action.state_hash
        ));
    }
    if !runtime_shell.last_audio_events.is_empty() {
        lines.push(format!(
            "audio events={}",
            runtime_shell.last_audio_events.join(" | ")
        ));
    }
    if let Some(error) = &runtime_shell.last_error {
        lines.push(format!("error={error}"));
    }

    lines.join("\n")
}

fn format_status_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let current_map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name);
    lines.push(format!(
        "pokedex seen={} owned={} repel={} last_spawn={:?} music={:?}",
        snapshot.progression.pokedex_seen,
        snapshot.progression.pokedex_owned,
        snapshot.progression.repel_steps_remaining,
        snapshot.progression.last_spawn_identifier,
        snapshot.audio.current_music
    ));
    lines.push(format!(
        "objects={} visible_objects={} warps={} coord_events={} bg_events={} scenes={}",
        current_map.map(|map| map.objects.len()).unwrap_or(0),
        snapshot.visible_objects.len(),
        current_map.map(|map| map.events.warps.len()).unwrap_or(0),
        current_map
            .map(|map| map.events.coord_events.len())
            .unwrap_or(0),
        current_map
            .map(|map| map.events.bg_events.len())
            .unwrap_or(0),
        current_map.map(|map| map.scenes.scenes.len()).unwrap_or(0)
    ));
    if let Some(metadata) = current_map.and_then(|map| map.metadata.as_ref()) {
        lines.push(format!(
            "map_meta constant={} group={} id={}x{} env={} phone={}",
            metadata.constant,
            metadata.group_name,
            metadata.width,
            metadata.height,
            metadata.environment,
            metadata.phone_service
        ));
    }
    if let Some(map) = current_map {
        if let Err(error) = append_nearby_map_context(
            runtime_shell,
            snapshot,
            map,
            &snapshot.visible_objects,
            lines,
        ) {
            lines.push(format!("nearby_context_error={error}"));
        }
    }
    if !snapshot.audio.queued_events.is_empty() {
        lines.push(format!("queued_audio={:?}", snapshot.audio.queued_events));
    }
}

fn append_title_menu_context(runtime_shell: &BevyRuntimeShell, lines: &mut Vec<String>) {
    let Some(title) = &runtime_shell.title_menu else {
        return;
    };
    let title_options = visible_title_menu_options(runtime_shell, title);
    let selected_title_option = title.cursor.option_index.min(title_options.len() - 1);
    let options = title_options
        .into_iter()
        .enumerate()
        .map(|(index, option)| {
            let label = match option {
                TitleMenuOption::Continue => "Continue",
                TitleMenuOption::NewGame => "New Game",
                TitleMenuOption::Options => "Option",
                TitleMenuOption::MysteryGift => "Mystery Gift",
            };
            if index == selected_title_option {
                format!(">{label}")
            } else {
                label.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let save_path = title
        .save_path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "-".to_string());
    lines.push(format!("title_menu {options} save={save_path}"));
    if title.save_path.is_some() {
        lines.extend(visible_title_continue_entries(runtime_shell, title));
    }
}

fn append_start_menu_context(runtime_shell: &BevyRuntimeShell, lines: &mut Vec<String>) {
    let Some(cursor) = &runtime_shell.start_menu_cursor else {
        return;
    };
    let Ok(snapshot) = runtime_shell.shell.snapshot() else {
        return;
    };
    let options = visible_start_menu_options(runtime_shell, &snapshot);
    let selected = if cursor.surface_id == START_MENU_SURFACE_ID {
        cursor.option_index.min(options.len().saturating_sub(1))
    } else {
        0
    };
    let options = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let label = start_menu_option_display_label(*option, &snapshot);
            if index == selected {
                format!(">{label}")
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push("START".to_string());
    lines.push(options);
}

fn append_special_boundary_context(runtime_shell: &BevyRuntimeShell, lines: &mut Vec<String>) {
    let Some(boundary) = &runtime_shell.special_boundary else {
        return;
    };
    append_special_boundary_display_context(boundary, lines);
}

fn append_special_boundary_display_context(
    boundary: &SpecialBoundaryDisplay,
    lines: &mut Vec<String>,
) {
    lines.push(format!("special_boundary={}", boundary.label));
}

fn start_menu_option_label(option: StartMenuOption) -> &'static str {
    match option {
        StartMenuOption::Pokemon => "#MON",
        StartMenuOption::Pack => "PACK",
        StartMenuOption::Save => "SAVE",
        StartMenuOption::QuitContest => "QUIT",
        StartMenuOption::Pokedex => "#DEX",
        StartMenuOption::Pokegear => "#GEAR",
        StartMenuOption::TrainerCard => "STATUS",
        StartMenuOption::Options => "OPTION",
        StartMenuOption::Exit => "EXIT",
    }
}

fn start_menu_option_display_label(
    option: StartMenuOption,
    snapshot: &RuntimeShellSnapshot,
) -> String {
    match option {
        StartMenuOption::TrainerCard => {
            let name = snapshot.trainer.player_name.trim();
            if name.is_empty() {
                "?????".to_string()
            } else {
                name.to_string()
            }
        }
        _ => start_menu_option_label(option).to_string(),
    }
}

fn append_party_menu_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !runtime_shell.party_menu_open {
        return;
    }
    if snapshot.party.slots.is_empty() {
        lines.push("party_menu empty".to_string());
        return;
    }
    let selected = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let summary = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let marker = if index == selected { ">" } else { "" };
            format!(
                "{}{} L{} {}/{} {:?}",
                marker,
                slot.pokemon.species.id,
                slot.pokemon.level,
                slot.pokemon.hp,
                slot.pokemon.max_hp,
                slot.pokemon.status
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push("POKEMON".to_string());
    for row in split_overlay_row(&summary, 2) {
        lines.push(row);
    }
    if runtime_shell.party_summary_open {
        lines.push("SUMMARY".to_string());
        if let Ok(entries) = visible_party_summary_entries(snapshot, runtime_shell) {
            lines.extend(entries);
        }
        return;
    }
    if let Some(switch_cursor) = &runtime_shell.party_switch_cursor {
        let source_slot_index = runtime_shell
            .party_cursor
            .min(snapshot.party.slots.len().saturating_sub(1));
        let Some(source_slot) = snapshot.party.slots.get(source_slot_index) else {
            lines.push("SWITCH".to_string());
            lines.push(format!("INVALID PARTY SLOT {source_slot_index}"));
            return;
        };
        let source_party_index = source_slot.index;
        let selected_target = strict_readonly_cursor_index(
            &Some(switch_cursor.clone()),
            &party_switch_cursor_surface_id(source_party_index),
            snapshot.party.slots.len(),
        );
        let Some(selected_target) = selected_target else {
            lines.push("SWITCH".to_string());
            lines.push("INVALID CURSOR party:switch".to_string());
            return;
        };
        let switch_summary = snapshot
            .party
            .slots
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                party_switch_slot_entry(
                    snapshot,
                    slot,
                    index == selected_target,
                    index == source_slot_index,
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push("SWITCH".to_string());
        for row in split_overlay_row(&switch_summary, 2) {
            lines.push(row);
        }
        return;
    }
    if let Some(action_cursor) = &runtime_shell.party_action_cursor {
        if let Ok(actions) = visible_party_actions(snapshot, runtime_shell) {
            let selected_action = strict_readonly_cursor_index(
                &Some(action_cursor.clone()),
                "party:actions",
                actions.len(),
            );
            let Some(selected_action) = selected_action else {
                lines.push("ACTIONS".to_string());
                lines.push("INVALID CURSOR party:actions".to_string());
                return;
            };
            let action_summary = actions
                .iter()
                .enumerate()
                .map(|(index, action)| {
                    let marker = if index == selected_action { ">" } else { "" };
                    party_action_entry(snapshot, runtime_shell, *action, marker)
                })
                .collect::<Vec<_>>()
                .join(" | ");
            lines.push("ACTIONS".to_string());
            lines.push(action_summary);
            return;
        }
    }
}

fn append_field_pack_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !visible_field_pack_is_open(runtime_shell) {
        return;
    }
    let active = active_visible_field_pack_pocket(runtime_shell);
    let pockets = carried_field_pack_pockets(snapshot)
        .into_iter()
        .filter(|pocket| field_pack_pocket_count(snapshot, pocket) > 0)
        .map(|pocket| {
            let marker = if pocket == active { ">" } else { "" };
            format!(
                "{}{}({})",
                marker,
                field_pack_pocket_label(&pocket),
                field_pack_pocket_count(snapshot, &pocket)
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push("PACK".to_string());
    lines.push(pockets);
    if let Some(selection) = selected_field_pack_item_label(snapshot, runtime_shell, &active) {
        lines.push(format!(
            "{} {}",
            field_pack_pocket_label(&active),
            selection
        ));
        if let Some(action) =
            selected_field_pack_item_action_summary(snapshot, runtime_shell, &active)
        {
            lines.push(format!("ACTION {action}"));
        }
    }
    append_field_pack_item_rows(snapshot, runtime_shell, &active, lines);
    if let Some(mode) = runtime_shell.field_pack_target_mode {
        append_field_pack_target_context(snapshot, runtime_shell, mode, lines);
        return;
    }
}

fn append_field_pack_target_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    mode: FieldPackTargetMode,
    lines: &mut Vec<String>,
) {
    if let Some(label) = selected_field_pack_item_label(
        snapshot,
        runtime_shell,
        &active_visible_field_pack_pocket(runtime_shell),
    ) {
        lines.push(format!("ITEM {label}"));
    }
    if snapshot.party.slots.is_empty() {
        lines.push(format!(
            "pack_target {} empty party",
            field_pack_target_mode_label(mode)
        ));
        return;
    }
    let selected = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let target_summary = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let marker = if index == selected { ">" } else { "" };
            format!(
                "{}{} L{} {}/{} {:?}",
                marker,
                slot.pokemon.species.id,
                slot.pokemon.level,
                slot.pokemon.hp,
                slot.pokemon.max_hp,
                slot.pokemon.status
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push(format!("TARGET {}", field_pack_target_mode_label(mode)));
    for row in split_overlay_row(&target_summary, 2) {
        lines.push(row);
    }
    if mode == FieldPackTargetMode::PartyMove || mode == FieldPackTargetMode::TmHmPokemon {
        if let Some(selected_slot) = snapshot.party.slots.get(selected) {
            if mode == FieldPackTargetMode::TmHmPokemon && selected_slot.pokemon.moves.len() < 4 {
                return;
            }
            let selected_move = strict_readonly_cursor_index(
                &runtime_shell.party_move_cursor,
                &party_move_cursor_surface_id(selected_slot.index),
                selected_slot.pokemon.moves.len(),
            );
            let Some(selected_move) = selected_move else {
                lines.push(format!(
                    "INVALID CURSOR party:{}:moves",
                    selected_slot.index
                ));
                return;
            };
            let move_summary = selected_slot
                .pokemon
                .moves
                .iter()
                .enumerate()
                .map(|(index, learned)| {
                    let marker = if index == selected_move { ">" } else { "" };
                    format!("{marker}{}({}pp)", learned.name, learned.current_pp)
                })
                .collect::<Vec<_>>()
                .join(" | ");
            for row in split_overlay_row(&move_summary, 2) {
                lines.push(row);
            }
        }
    }
}

fn selected_field_pack_item_label(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
) -> Option<String> {
    match pocket {
        FieldPackPocket::Items => strict_readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "bag:items",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.items)),
        )
        .filter(|index| *index < carried_item_count(&snapshot.bag.items))
        .and_then(|index| carried_item_offset(&snapshot.bag.items, index))
        .and_then(|offset| snapshot.bag.items.get(offset))
        .map(|item| pack_item_entry(snapshot, item, "")),
        FieldPackPocket::Balls => strict_readonly_cursor_index(
            &runtime_shell.ball_cursor,
            "bag:balls",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.balls)),
        )
        .filter(|index| *index < carried_item_count(&snapshot.bag.balls))
        .and_then(|index| carried_item_offset(&snapshot.bag.balls, index))
        .and_then(|offset| snapshot.bag.balls.get(offset))
        .map(|item| pack_item_entry(snapshot, item, "")),
        FieldPackPocket::KeyItems => strict_readonly_cursor_index(
            &runtime_shell.key_item_cursor,
            "bag:key-items",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.key_items)),
        )
        .filter(|index| *index < carried_item_count(&snapshot.bag.key_items))
        .and_then(|index| carried_item_offset(&snapshot.bag.key_items, index))
        .and_then(|offset| snapshot.bag.key_items.get(offset))
        .map(|item| pack_item_entry(snapshot, item, "")),
        FieldPackPocket::TmHm => strict_readonly_cursor_index(
            &runtime_shell.tmhm_cursor,
            "bag:tmhm",
            field_pack_selectable_count(snapshot.bag.tm_hm.len()),
        )
        .filter(|index| *index < snapshot.bag.tm_hm.len())
        .and_then(|index| snapshot.bag.tm_hm.get(index))
        .map(|tmhm| tmhm_pack_entry(snapshot, tmhm, "")),
        FieldPackPocket::Custom(pocket_id) => {
            let items = snapshot.bag.custom_pockets.get(pocket_id)?;
            strict_readonly_cursor_index(
                &runtime_shell.custom_item_cursor,
                &custom_pack_surface_id(pocket_id),
                field_pack_selectable_count(carried_item_count(items)),
            )
            .filter(|index| *index < carried_item_count(items))
            .and_then(|index| carried_item_offset(items, index))
            .and_then(|offset| items.get(offset))
            .map(|item| pack_item_entry(snapshot, item, ""))
        }
    }
}
