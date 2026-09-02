fn visible_title_menu_options<'a>(
    runtime_shell: &BevyRuntimeShell,
    title: &'a TitleMenu,
) -> &'a [RuntimeTitleMainMenuItem] {
    let variant = if title_continue_save_path(runtime_shell, title).is_none() {
        title.main_menu.new_game_variant
    } else if visible_title_mystery_gift_unlocked(runtime_shell, title) {
        title.main_menu.mystery_variant
    } else {
        title.main_menu.continue_variant
    };
    &title.main_menu.variants[variant]
}

fn visible_title_mystery_gift_unlocked(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
) -> bool {
    let Some(path) = title_continue_save_path(runtime_shell, title) else {
        return false;
    };
    runtime_shell
        .shell
        .runtime()
        .load_save(path)
        .map(|state| state.mystery_gift_unlocked)
        .unwrap_or(false)
}

impl VisibleIntroScreen {
    fn from_parameters(parameters: RuntimeIntroPresentationParameters) -> Self {
        Self {
            scene_count: parameters.scene_labels.len(),
            scene_operation_offsets: parameters.scene_operation_offsets,
            completion_wait_frames: parameters.completion_wait_frames,
            jumptable_index: 0,
            scene_dispatch_tick: 0,
            scene_frame_counter: 0,
            next_scene_frame_counter: None,
            scene_delay_frames: 0,
            scene_timer: 0,
            scroll_x: 0,
            scroll_y: 0,
            ly_overrides: vec![0; 144],
            lcdc_pointer: 0,
            global_anim_x_offset: 0,
            sprite_count: 0,
            sprites: Vec::new(),
            background_binding: None,
            attrmap_palette_overrides: Vec::new(),
            tile_override: None,
            palette_effect: VisibleIntroPaletteEffect::None,
            finished: false,
        }
    }

    #[cfg(test)]
    fn new() -> Self {
        Self::from_parameters(RuntimeIntroPresentationParameters {
            scene_labels: (1..=28).map(|index| format!("IntroScene{index}")).collect(),
            scene_operation_offsets: vec![0; 28],
            completion_wait_frames: vec![
                2, 0, 2, 0, 2, 0, 2, 0, 6, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 3, 0, 0,
                0, 0, 0, 0, 0,
            ],
        })
    }

    fn scene_name(&self) -> String {
        if self.jumptable_index < self.scene_count {
            format!("IntroScene{}", self.jumptable_index + 1)
        } else {
            "complete".to_string()
        }
    }
}

fn tick_visible_intro_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(intro) = runtime_shell.intro_screen.as_ref() else {
        return Ok(());
    };
    if intro.finished || intro.jumptable_index >= intro.scene_count {
        return finish_visible_intro_screen(runtime_shell, "complete");
    }
    if intro.scene_delay_frames > 0 {
        let delay_finished = {
            let intro = runtime_shell
                .intro_screen
                .as_mut()
                .context("intro screen disappeared during its scene delay")?;
            intro.scene_delay_frames = intro.scene_delay_frames.saturating_sub(1);
            if intro.scene_delay_frames == 0 {
                visible_intro_next_scene(intro);
                true
            } else {
                false
            }
        };
        if delay_finished {
            apply_visible_intro_sprite_pipeline_for_shell(runtime_shell)?;
        }
        if runtime_shell
            .intro_screen
            .as_ref()
            .is_some_and(|intro| intro.finished)
        {
            return finish_visible_intro_screen(runtime_shell, "complete");
        }
        return Ok(());
    }

    let scene_finished = step_visible_intro_scene(runtime_shell)?;
    let delay = if scene_finished {
        let intro = runtime_shell
            .intro_screen
            .as_ref()
            .context("intro screen disappeared before its source completion wait")?;
        intro
            .completion_wait_frames
            .get(intro.jumptable_index)
            .copied()
            .context("intro scene has no source-derived completion wait")?
    } else {
        0
    };
    if delay == 0 {
        apply_visible_intro_sprite_pipeline_for_shell(runtime_shell)?;
    }
    let Some(intro) = runtime_shell.intro_screen.as_mut() else {
        return Ok(());
    };
    intro.scene_dispatch_tick = intro
        .scene_dispatch_tick
        .checked_add(1)
        .context("visible intro scene exceeded the source dispatcher tick domain")?;
    if scene_finished {
        if delay > 0 {
            intro.scene_delay_frames = delay;
        } else {
            visible_intro_next_scene(intro);
        }
    } else if let Some(next) = intro.next_scene_frame_counter.take() {
        intro.scene_frame_counter = next;
    } else {
        intro.scene_frame_counter = intro.scene_frame_counter.wrapping_add(1);
    }
    if intro.finished {
        return finish_visible_intro_screen(runtime_shell, "complete");
    }
    Ok(())
}

fn step_visible_intro_scene(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    let audio_operations = {
        let intro = runtime_shell
            .intro_screen
            .as_ref()
            .context("visible intro has no state")?;
        visible_intro_audio_operations(
            intro,
            &runtime_shell.runtime.data().runtime_title_screen.program,
        )?
    };
    for (audio, kind) in audio_operations {
        match kind.as_str() {
            "music" => queue_visible_intro_music(runtime_shell, &audio)?,
            "sound_effect" => queue_visible_sound_effect(
                runtime_shell.shell.runtime().audio(),
                &mut runtime_shell.pending_audio,
                &mut runtime_shell.last_audio_events,
                &audio,
            )?,
            _ => anyhow::bail!("visible intro play_audio has unsupported kind {kind}"),
        }
    }
    let intro_program = &runtime_shell.runtime.data().runtime_title_screen.program;
    let sprite_bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .context("visible intro has no pack-owned sprite animation bundle")?;
    let Some(intro) = runtime_shell.intro_screen.as_mut() else {
        return Ok(false);
    };
    apply_visible_intro_background_binding(intro, intro_program)?;
    match intro.jumptable_index {
        0 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.scene_frame_counter = 0;
            intro.scene_timer = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        1 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x80 {
                return Ok(true);
            }
            spawn_visible_intro_sprite_program_group_if_scheduled(
                intro,
                sprite_bundle,
                intro_program,
            )?;
            intro.scene_timer = frame;
            intro.palette_effect =
                visible_intro_unown_fade_effect(intro, intro_program, 0, frame)?;
            Ok(false)
        }
        2 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            visible_intro_reset_ly_overrides(intro, intro_program)?;
            intro.lcdc_pointer =
                visible_intro_source_byte_write(intro, intro_program, "hLCDCPointer")?;
            intro.scene_frame_counter = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        3 => {
            visible_intro_perspective_scroll(intro, intro_program, intro.scene_frame_counter)?;
            Ok(intro.scene_frame_counter
                == visible_intro_perspective_completion_frame(intro, intro_program)?)
        }
        4 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.lcdc_pointer =
                visible_intro_source_byte_write(intro, intro_program, "hLCDCPointer")?;
            intro.scene_frame_counter = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        5 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x80 {
                return Ok(true);
            }
            spawn_visible_intro_sprite_program_group_if_scheduled(
                intro,
                sprite_bundle,
                intro_program,
            )?;
            intro.scene_timer = frame;
            intro.palette_effect = visible_intro_unown_fade_effect(
                intro,
                intro_program,
                if frame >= 0x40 { 1 } else { 0 },
                frame,
            )?;
            Ok(false)
        }
        6 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite_program_group(intro, sprite_bundle, intro_program)?;
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.global_anim_x_offset =
                visible_intro_source_byte_write(intro, intro_program, "wGlobalAnimXOffset")?;
            visible_intro_reset_ly_overrides(intro, intro_program)?;
            intro.lcdc_pointer =
                visible_intro_source_byte_write(intro, intro_program, "hLCDCPointer")?;
            intro.scene_frame_counter = 0;
            intro.scene_timer = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        7 => {
            let frame = intro.scene_frame_counter;
            let rule = visible_intro_perspective_motion_rule(intro, intro_program)?;
            if frame < rule.motion_start_frame {
                visible_intro_perspective_scroll(
                    intro,
                    intro_program,
                    frame.wrapping_add(1),
                )?;
                return Ok(false);
            }
            if intro.global_anim_x_offset == rule.finish_offset {
                clear_visible_intro_sprites(intro);
                return Ok(true);
            }
            intro.global_anim_x_offset = intro
                .global_anim_x_offset
                .wrapping_sub(rule.motion_delta);
            Ok(false)
        }
        8 => {
            clear_visible_intro_sprites(intro);
            intro.attrmap_palette_overrides =
                visible_intro_attrmap_fills(intro, intro_program)?;
            intro.scene_timer = 0;
            intro.global_anim_x_offset =
                visible_intro_source_byte_write(intro, intro_program, "wGlobalAnimXOffset")?;
            intro.lcdc_pointer =
                visible_intro_source_byte_write(intro, intro_program, "hLCDCPointer")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        9 => {
            let frame = intro.scene_frame_counter;
            if let Some(tile_override) =
                visible_intro_indexed_tile_override(intro, intro_program, frame)?
            {
                intro.tile_override = Some(tile_override);
            }
            spawn_visible_intro_sprite_program_group_if_scheduled(
                intro,
                sprite_bundle,
                intro_program,
            )?;
            Ok(frame == 0xc0)
        }
        10 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.lcdc_pointer =
                visible_intro_source_byte_write(intro, intro_program, "hLCDCPointer")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        11 => {
            let frame = intro.scene_frame_counter;
            if let Some(audio) = visible_intro_scheduled_audio(
                intro,
                intro_program,
                "wIntroSceneFrameCounter",
                frame,
            )? {
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    &audio,
                )?;
            }
            if frame >= 0xc0 {
                return Ok(true);
            }
            let (timer, palette_idx) = if frame >= 0x80 {
                (
                    (frame & 0x0f).wrapping_mul(4),
                    ((frame & 0x70) | 0x40).rotate_left(4),
                )
            } else {
                ((frame & 0x1f).wrapping_mul(2), (frame & 0xe0) >> 5)
            };
            intro.scene_timer = timer;
            intro.palette_effect =
                visible_intro_unown_fade_effect(intro, intro_program, palette_idx, timer)?;
            Ok(false)
        }
        12 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite_program_group(intro, sprite_bundle, intro_program)?;
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.global_anim_x_offset =
                visible_intro_source_byte_write(intro, intro_program, "wGlobalAnimXOffset")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        13 => {
            let frame = intro.scene_frame_counter;
            let rule = visible_intro_suicune_run_rule(intro, intro_program)?;
            intro.scroll_x = intro.scroll_x.wrapping_sub(rule.scroll_delta);
            if frame >= rule.end_frame {
                return Ok(true);
            }
            if frame >= rule.jump_frame {
                intro.scene_timer = rule.jump_timer;
                if intro.global_anim_x_offset < rule.disappear_below {
                    clear_visible_intro_sprites(intro);
                } else {
                    intro.global_anim_x_offset = intro
                        .global_anim_x_offset
                        .wrapping_sub(rule.jump_offset_delta);
                }
            } else if frame >= rule.run_frame {
                intro.global_anim_x_offset = intro
                    .global_anim_x_offset
                    .wrapping_sub(rule.run_offset_delta);
            }
            Ok(false)
        }
        14 => {
            clear_visible_intro_sprites(intro);
            let activated_range =
                spawn_visible_intro_sprite_program_group(intro, sprite_bundle, intro_program)?;
            anyhow::ensure!(
                activated_range.len() == 2,
                "IntroScene15 must activate exactly two source sprites"
            );
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        15 => {
            let frame = intro.scene_frame_counter;
            let (finished, scroll_y) = visible_intro_linear_scroll_step(
                intro,
                intro_program,
                "hSCY",
                frame,
                intro.scroll_y,
            )?;
            if finished {
                return Ok(true);
            }
            intro.scroll_y = scroll_y;
            Ok(false)
        }
        16 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        17 => {
            let (finished, scroll_x) = visible_intro_linear_scroll_step(
                intro,
                intro_program,
                "hSCX",
                intro.scene_frame_counter,
                intro.scroll_x,
            )?;
            if finished {
                return Ok(true);
            }
            intro.scroll_x = scroll_x;
            Ok(false)
        }
        18 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite_program_group(intro, sprite_bundle, intro_program)?;
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        19 => {
            let frame = intro.scene_frame_counter;
            let rule = visible_intro_unown_reveal_rule(intro, intro_program)?;
            if frame >= rule.end_frame {
                return Ok(true);
            }
            if frame < rule.scroll_end_frame {
                intro.scroll_y = intro.scroll_y.wrapping_add(rule.scroll_delta);
            } else if (rule.reveal_start_frame..rule.reveal_end_frame).contains(&frame) {
                let phase = frame.wrapping_sub(rule.phase_subtract);
                if (phase & rule.cadence_mask) == rule.cadence_operand {
                    let timer = (phase & rule.timer_mask) >> rule.timer_shift;
                    intro.scene_timer = timer;
                    intro.palette_effect = visible_intro_indexed_palette_effect(
                        intro,
                        intro_program,
                        rule.palette_argument,
                        timer,
                    )?;
                }
            }
            Ok(false)
        }
        20 => {
            intro.scene_frame_counter = 0;
            intro.scene_timer = 0;
            Ok(true)
        }
        21 => {
            if intro.scene_frame_counter >= 8 {
                clear_visible_intro_sprites(intro);
                return Ok(true);
            }
            Ok(false)
        }
        22 => Ok(true),
        23 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x20 {
                intro.next_scene_frame_counter = Some(0x40);
                return Ok(true);
            }
            if let Some(effect) =
                visible_intro_broadcast_palette_effect(intro, intro_program, frame)?
            {
                intro.palette_effect = effect;
            }
            Ok(false)
        }
        24 => {
            let current = intro.scene_frame_counter;
            let next = current.wrapping_sub(1);
            intro.next_scene_frame_counter = Some(next);
            Ok(next == 0)
        }
        25 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = visible_intro_source_byte_write(intro, intro_program, "hSCX")?;
            intro.scroll_y = visible_intro_source_byte_write(intro, intro_program, "hSCY")?;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        26 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x80 {
                intro.scene_frame_counter = 0x80;
                intro.next_scene_frame_counter = Some(0x80);
                return Ok(true);
            }
            intro.scene_timer = frame & 0x0f;
            intro.palette_effect = visible_intro_crystal_word_fade_effect(
                intro,
                intro_program,
                (frame & 0x70) >> 4,
                frame & 0x0f,
            )?;
            Ok(false)
        }
        27 => {
            let current = intro.scene_frame_counter;
            if current == 0 {
                return Ok(true);
            }
            if let Some(color) =
                visible_intro_bg_palette_clear_color(intro, intro_program, current)?
            {
                intro.palette_effect = VisibleIntroPaletteEffect::ClearBg { color };
            }
            intro.next_scene_frame_counter = Some(current.wrapping_sub(1));
            Ok(false)
        }
        _ => Ok(true),
    }
}

fn clear_visible_intro_sprites(intro: &mut VisibleIntroScreen) {
    intro.sprites.clear();
    intro.sprite_count = 0;
}

fn apply_visible_intro_background_binding(
    intro: &mut VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<()> {
    let subprogram = program
        .subprograms
        .iter()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .context("runtime title presentation has no crystal_intro subprogram")?;
    let phase = subprogram
        .phases
        .iter()
        .find(|phase| phase.id == "scene_dispatch")
        .context("crystal_intro has no scene_dispatch phase")?;
    let mut matching = phase.operations.iter().filter(|operation| {
        operation.op == "intro_background_binding"
            && operation
                .fields
                .get("dispatcher_entry")
                .and_then(serde_json::Value::as_u64)
                == Some(intro.jumptable_index as u64)
    });
    let operation = matching.next().with_context(|| {
        format!(
            "crystal_intro has no background binding for dispatcher entry {}",
            intro.jumptable_index
        )
    })?;
    anyhow::ensure!(
        matching.next().is_none(),
        "crystal_intro has duplicate background bindings for dispatcher entry {}",
        intro.jumptable_index
    );
    let string_field = |field: &str| -> Result<String> {
        operation
            .fields
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .with_context(|| {
                format!(
                    "crystal_intro background binding {} has no exact {field}",
                    intro.jumptable_index
                )
            })
    };
    anyhow::ensure!(
        operation
            .fields
            .get("tile_addressing")
            .and_then(serde_json::Value::as_str)
            == Some("signed_8800"),
        "crystal_intro background binding {} has unsupported tile addressing",
        intro.jumptable_index
    );
    let raw_tiles = operation
        .fields
        .get("tile_bindings")
        .and_then(serde_json::Value::as_array)
        .context("crystal_intro background binding has no exact tile bindings")?;
    let byte_field = |tile: &serde_json::Value, field: &str| -> Result<u8> {
        u8::try_from(
            tile.get(field)
                .and_then(serde_json::Value::as_u64)
                .with_context(|| format!("intro background tile binding has no exact {field}"))?,
        )
        .with_context(|| format!("intro background tile binding {field} exceeds one byte"))
    };
    let mut occupied = [[false; 256]; 2];
    let mut tile_bindings = Vec::with_capacity(raw_tiles.len());
    for tile in raw_tiles {
        let start = byte_field(tile, "tile_id_start")?;
        let end = byte_field(tile, "tile_id_end")?;
        let bank = byte_field(tile, "target_vram_bank")?;
        anyhow::ensure!(
            start <= end,
            "intro background tile binding has reversed range"
        );
        anyhow::ensure!(
            bank <= 1,
            "intro background tile binding has invalid VRAM bank {bank}"
        );
        for tile_id in start..=end {
            anyhow::ensure!(
                !occupied[usize::from(bank)][usize::from(tile_id)],
                "intro background tile binding overlaps bank {bank} tile {tile_id:#04x}"
            );
            occupied[usize::from(bank)][usize::from(tile_id)] = true;
        }
        let resource_tile_start = u16::try_from(
            tile.get("resource_tile_start")
                .and_then(serde_json::Value::as_u64)
                .context("intro background tile binding has no resource tile start")?,
        )
        .context("intro background resource tile start exceeds two bytes")?;
        let resource = tile
            .get("resource")
            .and_then(serde_json::Value::as_str)
            .context("intro background tile binding has no resource")?;
        anyhow::ensure!(
            resource.starts_with("gfx/intro/")
                && (resource.ends_with(".2bpp") || resource.ends_with(".2bpp.lz")),
            "intro background tile binding has unsupported resource {resource}"
        );
        tile_bindings.push(VisibleIntroBgTileBinding {
            tile_id_start: start,
            tile_id_end: end,
            target_vram_bank: bank,
            resource: resource.to_string(),
            resource_tile_start,
        });
    }
    let binding = VisibleIntroBackgroundBinding {
        dispatcher_entry: intro.jumptable_index,
        tilemap_resource: string_field("tilemap_resource")?,
        attrmap_resource: string_field("attrmap_resource")?,
        palette_resource: string_field("palette_resource")?,
        tile_bindings,
    };
    if intro
        .background_binding
        .as_ref()
        .is_some_and(|current| current.attrmap_resource != binding.attrmap_resource)
    {
        intro.attrmap_palette_overrides.clear();
        intro.tile_override = None;
    }
    intro.background_binding = Some(binding);
    Ok(())
}

fn spawn_visible_intro_sprite<'a>(
    intro: &'a mut VisibleIntroScreen,
    bundle: &SpriteAnimRuntimeBundle,
    object_name: &str,
    x: i16,
    y: i16,
) -> Result<&'a mut VisibleIntroSprite> {
    let (frameset_name, anim_function) = visible_intro_sprite_definition(bundle, object_name)?;
    intro.sprites.push(VisibleIntroSprite {
        x,
        y,
        tile_id: 0,
        oam_attr: 0,
        gfx_name: String::new(),
        gfx_tile_base: 0,
        jumptable_index: 0,
        frame_timer: 0,
        frameset_step: -1,
        start_delay: 0,
        x_offset: 0,
        y_offset: 0,
        var1: 0,
        var2: 0,
        frameset_name,
        object_name: object_name.to_string(),
        anim_function,
        current_oam_set: None,
        attr_flags: 0,
    });
    intro.sprite_count = intro.sprites.len().min(u8::MAX as usize) as u8;
    Ok(intro.sprites.last_mut().expect("pushed intro sprite"))
}

fn visible_intro_sprite_definition(
    bundle: &SpriteAnimRuntimeBundle,
    object_name: &str,
) -> Result<(String, String)> {
    let object = bundle
        .objects
        .get(object_name)
        .with_context(|| format!("pack-owned intro sprite object {object_name} is missing"))?;
    let declared_name = object
        .get("name")
        .and_then(serde_json::Value::as_str)
        .context("intro sprite object has no exact name")?;
    anyhow::ensure!(
        declared_name == object_name,
        "intro sprite object {object_name} contains mismatched name {declared_name}"
    );
    let frameset_name = object
        .get("frameset")
        .and_then(serde_json::Value::as_str)
        .context("intro sprite object has no exact frameset")?;
    anyhow::ensure!(
        bundle.framesets.contains_key(frameset_name),
        "intro sprite object {object_name} references missing frameset {frameset_name}"
    );
    let anim_function = object
        .get("function")
        .and_then(serde_json::Value::as_str)
        .context("intro sprite object has no exact animation function")?;
    Ok((frameset_name.to_string(), anim_function.to_string()))
}

fn spawn_visible_intro_sprite_program_group(
    intro: &mut VisibleIntroScreen,
    bundle: &SpriteAnimRuntimeBundle,
    program: &RuntimePresentationProgram,
) -> Result<std::ops::Range<usize>> {
    let subprogram = program
        .subprograms
        .iter()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .context("runtime title presentation has no crystal_intro subprogram")?;
    let phase = subprogram
        .phases
        .iter()
        .find(|phase| phase.id == "scene_dispatch")
        .context("crystal_intro has no scene_dispatch phase")?;
    let dispatcher_entry = intro.jumptable_index as u64;
    let dispatch_tick = u64::from(intro.scene_dispatch_tick) + 1;
    let activation_operations = phase
        .operations
        .iter()
        .filter(|operation| {
            matches!(operation.op.as_str(), "sprite_init_group" | "sprite_activate")
                && operation
                    .fields
                    .get("dispatcher_entry")
                    .and_then(serde_json::Value::as_u64)
                    == Some(dispatcher_entry)
                && operation
                    .fields
                    .get("dispatch_tick")
                    .and_then(serde_json::Value::as_u64)
                    == Some(dispatch_tick)
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        !activation_operations.is_empty(),
        "crystal_intro has no sprite activation at dispatcher entry {dispatcher_entry} tick {dispatch_tick}"
    );
    let mut instance_ids = Vec::new();
    for operation in activation_operations {
        if operation.op == "sprite_init_group" {
            let instances = operation
                .fields
                .get("instances")
                .and_then(serde_json::Value::as_array)
                .context("crystal_intro sprite_init_group has no exact instances")?;
            for instance in instances {
                instance_ids.push(
                    instance
                        .as_str()
                        .context("crystal_intro sprite_init_group instance is not a string")?,
                );
            }
        } else {
            instance_ids.push(
                operation
                    .fields
                    .get("instance")
                    .and_then(serde_json::Value::as_str)
                    .context("crystal_intro sprite_activate has no exact instance")?,
            );
        }
    }
    let mut matching = Vec::with_capacity(instance_ids.len());
    for instance_id in instance_ids {
        let programs = subprogram
            .sprite_programs
            .iter()
            .filter(|sprite_program| {
                sprite_program
                    .get("instance")
                    .and_then(serde_json::Value::as_str)
                    == Some(instance_id)
            })
            .collect::<Vec<_>>();
        anyhow::ensure!(
            programs.len() == 1,
            "crystal_intro activation instance {instance_id} resolves to {} sprite programs",
            programs.len()
        );
        matching.push(programs[0]);
    }
    anyhow::ensure!(
        matching
            .iter()
            .map(|sprite_program| {
                sprite_program
                    .get("instance")
                    .and_then(serde_json::Value::as_str)
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == matching.len(),
        "crystal_intro activation at dispatcher entry {dispatcher_entry} tick {dispatch_tick} repeats a sprite instance"
    );

    let start = intro.sprites.len();
    for sprite_program in matching {
        let instance_id = sprite_program
            .get("instance")
            .and_then(serde_json::Value::as_str)
            .context("crystal_intro sprite program has no exact instance id")?;
        anyhow::ensure!(
            sprite_program
                .pointer("/allocation_source_span/file")
                .and_then(serde_json::Value::as_str)
                == Some("engine/movie/intro.asm"),
            "crystal_intro sprite program {instance_id} has a non-intro allocation source"
        );
        anyhow::ensure!(
            sprite_program
                .pointer("/allocation_source_span/start_line")
                .and_then(serde_json::Value::as_u64)
                .is_some(),
            "crystal_intro sprite program {instance_id} has no allocation source line"
        );
        let string_at = |pointer: &str, field: &str| -> Result<&str> {
            sprite_program
                .pointer(pointer)
                .and_then(serde_json::Value::as_str)
                .with_context(|| {
                    format!("crystal_intro sprite program {instance_id} has no exact {field}")
                })
        };
        let byte_at = |field: &str| -> Result<u8> {
            let value = sprite_program
                .pointer(&format!("/initial_memory/{field}"))
                .and_then(serde_json::Value::as_u64)
                .with_context(|| {
                    format!("crystal_intro sprite program {instance_id} has no exact initial {field}")
                })?;
            u8::try_from(value).with_context(|| {
                format!("crystal_intro sprite program {instance_id} initial {field} exceeds one byte")
            })
        };

        let object_name = string_at("/object/symbol", "object symbol")?;
        let frameset_name = string_at("/frameset/symbol", "frameset symbol")?;
        let callback_name = string_at("/callback/symbol", "callback symbol")?;
        let graphic_resource = string_at("/graphic_binding/resource", "graphic resource")?;
        let gfx_name = graphic_resource
            .strip_prefix("gfx/intro/")
            .and_then(|name| name.strip_suffix(".lz").or(Some(name)))
            .and_then(|name| name.strip_suffix(".2bpp"))
            .with_context(|| {
                format!("crystal_intro sprite program {instance_id} has unsupported graphic resource {graphic_resource}")
            })?;
        let sprite = spawn_visible_intro_sprite(
            intro,
            bundle,
            object_name,
            i16::from(byte_at("xcoord")?),
            i16::from(byte_at("ycoord")?),
        )?;
        anyhow::ensure!(
            sprite.anim_function == callback_name,
            "crystal_intro sprite program {instance_id} callback {callback_name} disagrees with pack object callback {}",
            sprite.anim_function
        );
        anyhow::ensure!(
            bundle.framesets.contains_key(frameset_name),
            "crystal_intro sprite program {instance_id} references missing frameset {frameset_name}"
        );
        anyhow::ensure!(
            byte_at("duration")? == 0
                && byte_at("duration_offset")? == 0
                && byte_at("frame")? == u8::MAX,
            "crystal_intro sprite program {instance_id} has unsupported initial frame timing"
        );
        sprite.frameset_name = frameset_name.to_string();
        sprite.gfx_name = gfx_name.to_string();
        sprite.gfx_tile_base = u8::try_from(
            sprite_program
                .pointer("/graphic_binding/tile_base")
                .and_then(serde_json::Value::as_u64)
                .with_context(|| {
                    format!("crystal_intro sprite program {instance_id} has no exact graphic tile base")
                })?,
        )
        .context("intro sprite graphic tile base exceeds one byte")?;
        sprite.tile_id = byte_at("tile_id")?;
        sprite.jumptable_index = byte_at("jumptable_index")?;
        sprite.var1 = byte_at("var1")?;
        sprite.var2 = byte_at("var2")?;
        sprite.x_offset = i16::from(byte_at("xoffset")? as i8);
        sprite.y_offset = i16::from(byte_at("yoffset")? as i8);
    }
    Ok(start..intro.sprites.len())
}

fn spawn_visible_intro_sprite_program_group_if_scheduled(
    intro: &mut VisibleIntroScreen,
    bundle: &SpriteAnimRuntimeBundle,
    program: &RuntimePresentationProgram,
) -> Result<std::ops::Range<usize>> {
    let dispatcher_entry = intro.jumptable_index as u64;
    let dispatch_tick = u64::from(intro.scene_dispatch_tick) + 1;
    let scheduled = visible_intro_scene_operations(intro, program)?
        .iter()
        .any(|operation| {
            matches!(operation.op.as_str(), "sprite_init_group" | "sprite_activate")
                && operation
                    .fields
                    .get("dispatcher_entry")
                    .and_then(serde_json::Value::as_u64)
                    == Some(dispatcher_entry)
                && operation
                    .fields
                    .get("dispatch_tick")
                    .and_then(serde_json::Value::as_u64)
                    == Some(dispatch_tick)
        });
    if scheduled {
        spawn_visible_intro_sprite_program_group(intro, bundle, program)
    } else {
        Ok(intro.sprites.len()..intro.sprites.len())
    }
}

fn apply_visible_intro_sprite_pipeline_for_shell(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let program = &runtime_shell.runtime.data().runtime_title_screen.program;
    let bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .context("visible intro has no pack-owned sprite animation bundle")?;
    let intro = runtime_shell
        .intro_screen
        .as_mut()
        .context("visible intro sprite pipeline has no active intro")?;
    apply_visible_intro_sprite_pipeline(intro, bundle, program)
}

fn apply_visible_intro_sprite_pipeline(
    intro: &mut VisibleIntroScreen,
    bundle: &SpriteAnimRuntimeBundle,
    program: &RuntimePresentationProgram,
) -> Result<()> {
    if intro.jumptable_index == 20 {
        intro.scene_timer = 0;
    }
    apply_visible_intro_sprite_anim_functions(intro, bundle, program)?;
    update_visible_intro_sprite_animations(intro, bundle)?;
    intro.sprite_count = intro.sprites.len().min(u8::MAX as usize) as u8;
    Ok(())
}

fn apply_visible_intro_sprite_anim_functions(
    intro: &mut VisibleIntroScreen,
    bundle: &SpriteAnimRuntimeBundle,
    program: &RuntimePresentationProgram,
) -> Result<()> {
    let scene_frame_counter = intro.scene_frame_counter;
    let scene_timer = intro.scene_timer;
    for sprite in &mut intro.sprites {
        let callback = visible_intro_sprite_callback(program, &sprite.anim_function)?;
        execute_visible_intro_sprite_callback(
            sprite,
            bundle,
            callback,
            scene_frame_counter,
            scene_timer,
        )?;
    }
    Ok(())
}

fn visible_intro_sprite_callback<'a>(
    program: &'a RuntimePresentationProgram,
    callback_name: &str,
) -> Result<&'a serde_json::Value> {
    let subprogram = program
        .subprograms
        .iter()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .context("runtime title presentation has no crystal_intro subprogram")?;
    let mut callbacks = subprogram
        .sprite_programs
        .iter()
        .filter_map(|sprite_program| {
            let callback = sprite_program.get("callback")?;
            (callback.get("symbol")?.as_str()? == callback_name).then_some(callback)
        });
    let callback = callbacks.next().with_context(|| {
        format!("crystal_intro has no exported callback program {callback_name}")
    })?;
    for duplicate in callbacks {
        for field in [
            "kind",
            "symbol",
            "target",
            "instructions",
            "per_tick_struct_deltas",
            "host_operations",
            "outer_memory_reads",
            "labels",
        ] {
            anyhow::ensure!(
                duplicate.get(field) == callback.get(field),
                "crystal_intro callback program {callback_name} has conflicting {field} definitions"
            );
        }
        let expected_reinitializations = callback
            .get("frameset_reinitializations")
            .and_then(serde_json::Value::as_array)
            .context("intro callback has no exact frameset reinitialization list")?;
        let duplicate_reinitializations = duplicate
            .get("frameset_reinitializations")
            .and_then(serde_json::Value::as_array)
            .context("duplicate intro callback has no exact frameset reinitialization list")?;
        anyhow::ensure!(
            duplicate_reinitializations.len() == expected_reinitializations.len(),
            "crystal_intro callback program {callback_name} has conflicting frameset reinitialization counts"
        );
        for (expected, duplicate) in expected_reinitializations
            .iter()
            .zip(duplicate_reinitializations)
        {
            for field in [
                "frameset",
                "guard",
                "application",
                "source_span",
                "implementation_source_span",
            ] {
                anyhow::ensure!(
                    duplicate.get(field) == expected.get(field),
                    "crystal_intro callback program {callback_name} has conflicting frameset reinitialization {field}"
                );
            }
        }
    }
    anyhow::ensure!(
        callback.get("kind").and_then(serde_json::Value::as_str) == Some("direct"),
        "crystal_intro callback program {callback_name} is not direct"
    );
    Ok(callback)
}

#[derive(Clone, Copy)]
struct VisibleIntroCallbackFlags {
    zero: bool,
    carry: bool,
}

enum VisibleIntroCallbackStackValue {
    Af {
        a: u8,
        a_symbol: Option<String>,
        flags: VisibleIntroCallbackFlags,
    },
    De(u8),
}

fn execute_visible_intro_sprite_callback(
    sprite: &mut VisibleIntroSprite,
    bundle: &SpriteAnimRuntimeBundle,
    callback: &serde_json::Value,
    scene_frame_counter: u8,
    scene_timer: u8,
) -> Result<()> {
    let instructions = callback
        .get("instructions")
        .and_then(serde_json::Value::as_array)
        .context("intro callback has no exact instruction list")?;
    let labels = callback
        .get("labels")
        .and_then(serde_json::Value::as_object)
        .context("intro callback has no exact label table")?;
    let callback_name = callback
        .get("symbol")
        .and_then(serde_json::Value::as_str)
        .context("intro callback has no symbol")?;
    let mut a = 0_u8;
    let mut a_symbol = None;
    let mut d = 0_u8;
    let mut hl = None::<String>;
    let mut flags = VisibleIntroCallbackFlags {
        zero: false,
        carry: false,
    };
    let mut stack = Vec::new();
    let mut pc = 0_usize;
    let mut steps = 0_usize;
    while pc < instructions.len() {
        steps += 1;
        anyhow::ensure!(
            steps <= instructions.len().saturating_mul(4).max(1),
            "intro callback {callback_name} exceeded its bounded instruction budget"
        );
        let instruction = &instructions[pc];
        pc += 1;
        let opcode = instruction
            .get("opcode")
            .and_then(serde_json::Value::as_str)
            .context("intro callback instruction has no opcode")?;
        let args = instruction
            .get("args")
            .and_then(serde_json::Value::as_array)
            .context("intro callback instruction has no argument list")?
            .iter()
            .map(|arg| {
                arg.as_str()
                    .context("intro callback instruction argument is not a string")
            })
            .collect::<Result<Vec<_>>>()?;
        match (opcode, args.as_slice()) {
            ("ld", ["hl", field]) => {
                visible_intro_callback_field_name(field)?;
                hl = Some((*field).to_string());
            }
            ("ld", ["a", "[hl]"]) => {
                a = read_visible_intro_callback_field(
                    sprite,
                    hl.as_deref().context("intro callback reads unset hl")?,
                )?;
                a_symbol = None;
            }
            ("ld", ["d", "[hl]"]) => {
                d = read_visible_intro_callback_field(
                    sprite,
                    hl.as_deref().context("intro callback reads unset hl")?,
                )?;
            }
            ("ld", ["[hl]", "a"]) => write_visible_intro_callback_field(
                sprite,
                hl.as_deref().context("intro callback writes unset hl")?,
                a,
            )?,
            ("ld", ["[hl]", value]) => write_visible_intro_callback_field(
                sprite,
                hl.as_deref().context("intro callback writes unset hl")?,
                parse_visible_intro_callback_byte(value)?,
            )?,
            ("ld", ["a", source]) if source.starts_with('[') && source.ends_with(']') => {
                let source_symbol = &source[1..source.len() - 1];
                a = read_visible_intro_callback_outer_byte(
                    callback,
                    source_symbol,
                    scene_frame_counter,
                    scene_timer,
                )?;
                a_symbol = None;
            }
            ("ld", ["a", value]) => {
                if value.starts_with("SPRITE_ANIM_FRAMESET_") {
                    a_symbol = Some((*value).to_string());
                } else {
                    a = parse_visible_intro_callback_byte(value)?;
                    a_symbol = None;
                }
            }
            ("ld", ["d", value]) => d = parse_visible_intro_callback_byte(value)?,
            ("add", ["hl", "bc"]) => {
                anyhow::ensure!(hl.is_some(), "intro callback adds bc to unset hl");
            }
            ("add", [value]) => {
                a = a.wrapping_add(parse_visible_intro_callback_byte(value)?);
                a_symbol = None;
            }
            ("cp", [value]) => {
                let value = parse_visible_intro_callback_byte(value)?;
                flags.zero = a == value;
                flags.carry = a < value;
            }
            ("and", ["a"]) => {
                flags.zero = a == 0;
                flags.carry = false;
                a_symbol = None;
            }
            ("xor", [value]) => {
                a ^= parse_visible_intro_callback_byte(value)?;
                flags.zero = a == 0;
                flags.carry = false;
                a_symbol = None;
            }
            ("inc", ["a"]) => {
                a = a.wrapping_add(1);
                flags.zero = a == 0;
                a_symbol = None;
            }
            ("inc", ["[hl]"]) => {
                let field = hl
                    .as_deref()
                    .context("intro callback increments unset hl")?;
                let value = read_visible_intro_callback_field(sprite, field)?.wrapping_add(1);
                write_visible_intro_callback_field(sprite, field, value)?;
            }
            ("push", ["af"]) => stack.push(VisibleIntroCallbackStackValue::Af {
                a,
                a_symbol: a_symbol.clone(),
                flags,
            }),
            ("push", ["de"]) => stack.push(VisibleIntroCallbackStackValue::De(d)),
            ("pop", ["af"]) => match stack.pop() {
                Some(VisibleIntroCallbackStackValue::Af {
                    a: saved_a,
                    a_symbol: saved_symbol,
                    flags: saved_flags,
                }) => {
                    a = saved_a;
                    a_symbol = saved_symbol;
                    flags = saved_flags;
                }
                _ => anyhow::bail!("intro callback {callback_name} has an AF stack mismatch"),
            },
            ("pop", ["de"]) => match stack.pop() {
                Some(VisibleIntroCallbackStackValue::De(saved_d)) => d = saved_d,
                _ => anyhow::bail!("intro callback {callback_name} has a DE stack mismatch"),
            },
            ("call", ["AnimSeqs_Sine"]) => {
                a = visible_intro_sine(a, i16::from(d)) as i8 as u8;
                a_symbol = None;
            }
            ("call", ["AnimSeqs_Cosine"]) => {
                a = visible_intro_cosine(a, i16::from(d)) as i8 as u8;
                a_symbol = None;
            }
            ("call", ["_ReinitSpriteAnimFrame"]) => {
                let frameset_name = a_symbol
                    .take()
                    .context("intro callback reinitializer has no symbolic frameset in a")?;
                anyhow::ensure!(
                    bundle.framesets.contains_key(&frameset_name),
                    "intro callback reinitializes missing frameset {frameset_name}"
                );
                sprite.frameset_name = frameset_name;
                sprite.frameset_step = -1;
                sprite.frame_timer = 0;
                sprite.current_oam_set = None;
            }
            ("jr" | "jp", [condition, label]) => {
                if visible_intro_callback_condition(condition, flags)? {
                    let target = labels
                        .get(*label)
                        .and_then(serde_json::Value::as_u64)
                        .with_context(|| {
                            format!("intro callback {callback_name} has no label {label}")
                        })?;
                    pc = usize::try_from(target).context("intro callback label exceeds usize")?;
                    anyhow::ensure!(
                        pc < instructions.len(),
                        "intro callback {callback_name} label {label} is out of bounds"
                    );
                }
            }
            ("ret", []) => {
                anyhow::ensure!(
                    stack.is_empty(),
                    "intro callback {callback_name} returned with a nonempty stack"
                );
                return Ok(());
            }
            ("ret", [condition]) => {
                if visible_intro_callback_condition(condition, flags)? {
                    anyhow::ensure!(
                        stack.is_empty(),
                        "intro callback {callback_name} returned with a nonempty stack"
                    );
                    return Ok(());
                }
            }
            _ => anyhow::bail!(
                "intro callback {callback_name} uses unsupported instruction {opcode} {}",
                args.join(", ")
            ),
        }
    }
    anyhow::bail!("intro callback {callback_name} fell off its instruction program")
}

fn visible_intro_callback_field_name(field: &str) -> Result<()> {
    match field {
        "SPRITEANIMSTRUCT_XCOORD"
        | "SPRITEANIMSTRUCT_YCOORD"
        | "SPRITEANIMSTRUCT_XOFFSET"
        | "SPRITEANIMSTRUCT_YOFFSET"
        | "SPRITEANIMSTRUCT_JUMPTABLE_INDEX"
        | "SPRITEANIMSTRUCT_VAR1"
        | "SPRITEANIMSTRUCT_VAR2" => Ok(()),
        other => anyhow::bail!("intro callback addresses unsupported sprite field {other}"),
    }
}

fn read_visible_intro_callback_field(sprite: &VisibleIntroSprite, field: &str) -> Result<u8> {
    visible_intro_callback_field_name(field)?;
    Ok(match field {
        "SPRITEANIMSTRUCT_XCOORD" => sprite.x as u8,
        "SPRITEANIMSTRUCT_YCOORD" => sprite.y as u8,
        "SPRITEANIMSTRUCT_XOFFSET" => sprite.x_offset as i8 as u8,
        "SPRITEANIMSTRUCT_YOFFSET" => sprite.y_offset as i8 as u8,
        "SPRITEANIMSTRUCT_JUMPTABLE_INDEX" => sprite.jumptable_index,
        "SPRITEANIMSTRUCT_VAR1" => sprite.var1,
        "SPRITEANIMSTRUCT_VAR2" => sprite.var2,
        _ => unreachable!("validated intro callback field"),
    })
}

fn write_visible_intro_callback_field(
    sprite: &mut VisibleIntroSprite,
    field: &str,
    value: u8,
) -> Result<()> {
    visible_intro_callback_field_name(field)?;
    match field {
        "SPRITEANIMSTRUCT_XCOORD" => sprite.x = i16::from(value),
        "SPRITEANIMSTRUCT_YCOORD" => sprite.y = i16::from(value),
        "SPRITEANIMSTRUCT_XOFFSET" => sprite.x_offset = i16::from(value as i8),
        "SPRITEANIMSTRUCT_YOFFSET" => sprite.y_offset = i16::from(value as i8),
        "SPRITEANIMSTRUCT_JUMPTABLE_INDEX" => sprite.jumptable_index = value,
        "SPRITEANIMSTRUCT_VAR1" => sprite.var1 = value,
        "SPRITEANIMSTRUCT_VAR2" => sprite.var2 = value,
        _ => unreachable!("validated intro callback field"),
    }
    Ok(())
}

fn read_visible_intro_callback_outer_byte(
    callback: &serde_json::Value,
    source_symbol: &str,
    scene_frame_counter: u8,
    scene_timer: u8,
) -> Result<u8> {
    let canonical = callback
        .get("outer_memory_reads")
        .and_then(serde_json::Value::as_array)
        .context("intro callback has no outer memory read catalog")?
        .iter()
        .find(|read| {
            read.get("source_symbol")
                .and_then(serde_json::Value::as_str)
                == Some(source_symbol)
        })
        .and_then(|read| read.get("symbol"))
        .and_then(serde_json::Value::as_str)
        .with_context(|| format!("intro callback has no WRAM alias for {source_symbol}"))?;
    match canonical {
        "wIntroSceneFrameCounter" => Ok(scene_frame_counter),
        "wIntroSceneTimer" => Ok(scene_timer),
        other => anyhow::bail!("intro callback reads unsupported WRAM byte {other}"),
    }
}

fn parse_visible_intro_callback_byte(value: &str) -> Result<u8> {
    let parsed = if let Some(hex) = value.strip_prefix('$') {
        u16::from_str_radix(hex, 16)
            .with_context(|| format!("invalid intro callback byte {value}"))?
    } else {
        value
            .parse::<u16>()
            .with_context(|| format!("invalid intro callback byte {value}"))?
    };
    u8::try_from(parsed).with_context(|| format!("intro callback byte {value} exceeds one byte"))
}

fn visible_intro_callback_condition(
    condition: &str,
    flags: VisibleIntroCallbackFlags,
) -> Result<bool> {
    match condition {
        "z" => Ok(flags.zero),
        "nz" => Ok(!flags.zero),
        "c" => Ok(flags.carry),
        "nc" => Ok(!flags.carry),
        other => anyhow::bail!("intro callback uses unsupported condition {other}"),
    }
}

fn update_visible_intro_sprite_animations(
    intro: &mut VisibleIntroScreen,
    bundle: &SpriteAnimRuntimeBundle,
) -> Result<()> {
    let mut next = Vec::with_capacity(intro.sprites.len());
    for mut sprite in intro.sprites.drain(..) {
        if sprite.start_delay > 0 {
            sprite.start_delay = sprite.start_delay.saturating_sub(1);
            next.push(sprite);
            continue;
        }
        let frameset = visible_intro_frameset_steps(bundle, &sprite.frameset_name)?;
        let mut removed = false;
        let mut step_index = sprite.frameset_step;
        loop {
            if step_index < 0 || sprite.frame_timer == 0 {
                step_index += 1;
                if step_index < 0 {
                    step_index = 0;
                } else if usize::try_from(step_index).map_or(true, |idx| idx >= frameset.len()) {
                    step_index = frameset.len().saturating_sub(1) as i16;
                }
            } else {
                sprite.frame_timer = sprite.frame_timer.saturating_sub(1);
                break;
            }
            let step = &frameset[usize::try_from(step_index).unwrap_or(0)];
            match step.command.as_str() {
                "frame" => {
                    sprite.current_oam_set = step.oam_set.clone();
                    sprite.attr_flags = step.attr_flags;
                    sprite.frame_timer = u8::try_from(step.duration)
                        .context("intro frame duration exceeds one byte")?
                        .saturating_sub(1);
                    break;
                }
                "wait" => {
                    sprite.frame_timer = u8::try_from(step.duration)
                        .context("intro wait duration exceeds one byte")?
                        .saturating_sub(1);
                    break;
                }
                "restart" => {
                    step_index = -1;
                    sprite.frame_timer = 0;
                    continue;
                }
                "end" => {
                    // `oamend` rewinds the frame cursor to the preceding
                    // OAM frame, so the last image is held indefinitely.
                    step_index = step_index.saturating_sub(2);
                    sprite.frame_timer = 0;
                    continue;
                }
                "delete" => {
                    removed = true;
                    break;
                }
                command => anyhow::bail!(
                    "intro frameset {} reached unsupported command {command}",
                    sprite.frameset_name
                ),
            }
        }
        sprite.frameset_step = step_index;
        if !removed {
            next.push(sprite);
        }
    }
    intro.sprites = next;
    Ok(())
}

fn visible_intro_frameset_steps<'a>(
    bundle: &'a SpriteAnimRuntimeBundle,
    name: &str,
) -> Result<&'a [SpriteAnimFrameStep]> {
    bundle
        .framesets
        .get(name)
        .map(|frameset| frameset.steps.as_slice())
        .with_context(|| format!("pack-owned intro frameset {name} is missing"))
}

// Exact `sine_table 32` expansion used by `calc_sine_wave` in the ASM.
// Values are 8.8 fixed point and must be multiplied as unsigned 16-bit
// values before the final signed-byte conversion.
const ASM_SINE_TABLE: [u16; 32] = [
    0x0000, 0x0019, 0x0032, 0x004a, 0x0062, 0x0079, 0x008e, 0x00a2, 0x00b5, 0x00c6, 0x00d5, 0x00e2,
    0x00ed, 0x00f5, 0x00fb, 0x00ff, 0x0100, 0x00ff, 0x00fb, 0x00f5, 0x00ed, 0x00e2, 0x00d5, 0x00c6,
    0x00b5, 0x00a2, 0x008e, 0x0079, 0x0062, 0x004a, 0x0032, 0x0019,
];

fn visible_intro_sine(angle: u8, amplitude: i16) -> i16 {
    let amplitude = u8::try_from(amplitude).expect("intro sine amplitude must fit the ASM byte");
    let angle = angle & 0x3f;
    let negative = angle & 0x20 != 0;
    let mut factor = amplitude;
    let mut value = ASM_SINE_TABLE[usize::from(angle & 0x1f)];
    let mut product = 0_u16;
    while factor != 0 {
        if factor & 1 != 0 {
            product = product.wrapping_add(value);
        }
        factor >>= 1;
        value = value.wrapping_shl(1);
    }
    let result = (product >> 8) as u8;
    i16::from(if negative {
        result.wrapping_neg() as i8
    } else {
        result as i8
    })
}

fn visible_intro_cosine(angle: u8, amplitude: i16) -> i16 {
    visible_intro_sine(angle.wrapping_add(0x10), amplitude)
}

fn visible_intro_scene_operations<'a>(
    intro: &VisibleIntroScreen,
    program: &'a RuntimePresentationProgram,
) -> Result<&'a [crystal_assets::RuntimePresentationOperation]> {
    let phase = program
        .subprograms
        .iter()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .and_then(|subprogram| {
            subprogram
                .phases
                .iter()
                .find(|phase| phase.id == "scene_dispatch")
        })
        .context("runtime title presentation has no crystal_intro scene_dispatch phase")?;
    let start = intro
        .scene_operation_offsets
        .get(intro.jumptable_index)
        .copied()
        .context("visible intro scene has no exported operation offset")?;
    let end = intro
        .scene_operation_offsets
        .get(intro.jumptable_index + 1)
        .copied()
        .unwrap_or(phase.operations.len());
    anyhow::ensure!(
        start < end && end <= phase.operations.len(),
        "visible intro scene has an invalid exported operation range {start}..{end}"
    );
    Ok(&phase.operations[start..end])
}

fn visible_intro_scheduled_audio(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    clock: &str,
    frame: u8,
) -> Result<Option<String>> {
    let mut schedules = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| {
            operation.op == "scheduled_audio"
                && operation.fields.get("clock").and_then(serde_json::Value::as_str)
                    == Some(clock)
        });
    let schedule = schedules
        .next()
        .context("visible intro scene has no exact scheduled_audio operation")?;
    anyhow::ensure!(
        schedules.next().is_none(),
        "visible intro scene has duplicate scheduled_audio operations for {clock}"
    );
    anyhow::ensure!(
        schedule
            .fields
            .get("sentinel")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(u8::MAX))
            && schedule
                .fields
                .get("on_match")
                .and_then(|on_match| on_match.get("play_entry"))
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            && schedule
                .fields
                .get("on_match")
                .and_then(|on_match| on_match.get("stop_sfx_channels"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|channels| {
                    channels
                        .iter()
                        .map(serde_json::Value::as_u64)
                        .eq([Some(5), Some(6), Some(7), Some(8)])
                }),
        "visible intro scheduled_audio has unsupported source playback semantics"
    );
    let entries = schedule
        .fields
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .context("visible intro scheduled_audio has no entries")?;
    let mut selected = None;
    for entry in entries {
        let entry_frame = entry
            .get("frame")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .context("visible intro scheduled_audio entry has no byte frame")?;
        let audio = entry
            .get("audio")
            .and_then(serde_json::Value::as_str)
            .filter(|audio| !audio.is_empty())
            .context("visible intro scheduled_audio entry has no audio id")?;
        if entry_frame == frame {
            anyhow::ensure!(
                selected.replace(audio.to_string()).is_none(),
                "visible intro scheduled_audio has duplicate frame {frame}"
            );
        }
    }
    Ok(selected)
}

fn visible_intro_audio_operations(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<Vec<(String, String)>> {
    let subprogram = program
        .subprograms
        .iter()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .context("runtime title presentation has no crystal_intro subprogram")?;
    let dispatch_tick = u64::from(intro.scene_dispatch_tick) + 1;
    visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| operation.op == "play_audio")
        .filter(|operation| {
            operation
                .fields
                .get("dispatcher_entry")
                .and_then(serde_json::Value::as_u64)
                == Some(intro.jumptable_index as u64)
                && operation
                    .fields
                    .get("dispatch_tick")
                    .and_then(serde_json::Value::as_u64)
                    == Some(dispatch_tick)
        })
        .map(|operation| {
            let audio = operation
                .fields
                .get("audio")
                .and_then(serde_json::Value::as_str)
                .filter(|audio| !audio.is_empty())
                .context("visible intro play_audio operation has no audio id")?;
            let catalog = subprogram
                .audio
                .iter()
                .find(|candidate| candidate.id == audio)
                .or_else(|| program.audio.iter().find(|candidate| candidate.id == audio))
                .with_context(|| format!("visible intro play_audio references missing audio {audio}"))?;
            Ok((audio.to_string(), catalog.kind.clone()))
        })
        .collect()
}

fn visible_intro_unown_fade_effect(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    palette_idx: u8,
    timer: u8,
) -> Result<VisibleIntroPaletteEffect> {
    let operations = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| operation.op == "palette_fade_lookup")
        .collect::<Vec<_>>();
    let operation = operations
        .first()
        .copied()
        .context("visible intro scene has no palette_fade_lookup operation")?;
    for candidate in &operations[1..] {
        for field in [
            "palette_selector",
            "selector_stride",
            "first_color_offset",
            "timer",
            "clear",
            "tables",
            "writes",
            "bank",
            "transfer_request",
        ] {
            anyhow::ensure!(
                candidate.fields.get(field) == operation.fields.get(field),
                "visible intro palette_fade_lookup operations disagree on {field}"
            );
        }
    }
    anyhow::ensure!(
        operation
            .fields
            .get("palette_selector")
            .and_then(serde_json::Value::as_str)
            == Some("accumulator")
            && operation
                .fields
                .get("selector_stride")
                .and_then(serde_json::Value::as_u64)
                == Some(8)
            && operation
                .fields
                .get("first_color_offset")
                .and_then(serde_json::Value::as_u64)
                == Some(2)
            && palette_idx < 8,
        "visible intro palette_fade_lookup has unsupported selector semantics"
    );
    let timer_spec = operation
        .fields
        .get("timer")
        .context("visible intro palette_fade_lookup has no timer contract")?;
    let byte = |field: &str| -> Result<u8> {
        timer_spec
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .with_context(|| format!("visible intro palette timer has no byte {field}"))
    };
    anyhow::ensure!(
        timer_spec
            .get("source")
            .and_then(serde_json::Value::as_str)
            == Some("wIntroSceneTimer"),
        "visible intro palette_fade_lookup has unsupported timer source"
    );
    let mask = byte("mask")?;
    let fold_above = byte("fold_above")?;
    let fold_from = byte("fold_from")?;
    let masked = timer & mask;
    let fade_index = if masked > fold_above {
        fold_from
            .checked_sub(masked)
            .context("visible intro palette timer fold underflowed")?
    } else {
        masked
    };
    let tables = operation
        .fields
        .get("tables")
        .and_then(serde_json::Value::as_array)
        .context("visible intro palette_fade_lookup has no tables")?;
    let writes = operation
        .fields
        .get("writes")
        .and_then(serde_json::Value::as_array)
        .context("visible intro palette_fade_lookup has no writes")?;
    anyhow::ensure!(
        writes.len() == 3,
        "visible intro palette_fade_lookup must write exactly three colors"
    );
    let mut colors = [[0_u8; 3]; 3];
    for (index, write) in writes.iter().enumerate() {
        anyhow::ensure!(
            write
                .get("target_offset")
                .and_then(serde_json::Value::as_u64)
                == Some((2 + index * 2) as u64)
                && write
                    .get("encoding")
                    .and_then(serde_json::Value::as_str)
                    == Some("rgb555_little_endian"),
            "visible intro palette_fade_lookup has unsupported color write {index}"
        );
        let label = write
            .get("table")
            .and_then(serde_json::Value::as_str)
            .context("visible intro palette write has no source table")?;
        let table = tables
            .iter()
            .find(|table| table.get("label").and_then(serde_json::Value::as_str) == Some(label))
            .with_context(|| format!("visible intro palette table {label} is missing"))?;
        let color = table
            .get("colors")
            .and_then(serde_json::Value::as_array)
            .and_then(|values| values.get(usize::from(fade_index)))
            .and_then(serde_json::Value::as_u64)
            .context("visible intro palette fade index is outside its source table")?;
        colors[index] = visible_intro_rgb555(color)?;
    }
    Ok(VisibleIntroPaletteEffect::UnownFade {
        palette_idx,
        colors,
    })
}

fn visible_intro_crystal_word_fade_effect(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    fade_level: u8,
    timer: u8,
) -> Result<VisibleIntroPaletteEffect> {
    let mut operations = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| operation.op == "fade_unown_word_palettes");
    let operation = operations
        .next()
        .context("visible intro scene has no fade_unown_word_palettes operation")?;
    anyhow::ensure!(
        operations.next().is_none(),
        "visible intro scene has duplicate fade_unown_word_palettes operations"
    );
    let palette = operation
        .fields
        .get("palette_index")
        .context("visible intro word fade has no palette index contract")?;
    let fade = operation
        .fields
        .get("fade_index")
        .context("visible intro word fade has no fade index contract")?;
    let contains_byte = |value: &serde_json::Value, field: &str, byte: u8| {
        value
            .get(field)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|values| values.iter().any(|value| value.as_u64() == Some(u64::from(byte))))
    };
    anyhow::ensure!(
        palette.get("source").and_then(serde_json::Value::as_str) == Some("accumulator")
            && palette.get("multiply").and_then(serde_json::Value::as_u64) == Some(8)
            && contains_byte(palette, "valid_values", fade_level)
            && fade.get("source").and_then(serde_json::Value::as_str)
                == Some("wIntroSceneTimer")
            && fade.get("multiply").and_then(serde_json::Value::as_u64) == Some(2)
            && contains_byte(fade, "valid_values", timer)
            && operation
                .fields
                .get("target_color_offsets")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|offsets| {
                    offsets.iter().map(serde_json::Value::as_u64).eq([Some(4), Some(6)])
                })
            && operation
                .fields
                .get("color_encoding")
                .and_then(serde_json::Value::as_str)
                == Some("rgb555_grayscale"),
        "visible intro word fade has unsupported source semantics"
    );
    let hue = |field: &str| -> Result<[u8; 3]> {
        let value = operation
            .fields
            .get(field)
            .and_then(serde_json::Value::as_array)
            .and_then(|values| values.get(usize::from(timer)))
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value < 32)
            .with_context(|| format!("visible intro word fade has no source hue {field}[{timer}]"))?;
        Ok([value * 8; 3])
    };
    Ok(VisibleIntroPaletteEffect::CrystalWordFade {
        fade_level,
        colors: [hue("fast_hues")?, hue("slow_hues")?],
    })
}

fn visible_intro_broadcast_palette_effect(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    frame: u8,
) -> Result<Option<VisibleIntroPaletteEffect>> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let broadcast_index = operations
        .iter()
        .position(|operation| operation.op == "broadcast_indexed_palette")
        .context("visible intro scene has no broadcast_indexed_palette operation")?;
    anyhow::ensure!(
        operations
            .iter()
            .skip(broadcast_index + 1)
            .all(|operation| operation.op != "broadcast_indexed_palette"),
        "visible intro scene has duplicate broadcast_indexed_palette operations"
    );
    let transform = operations[..broadcast_index]
        .iter()
        .rev()
        .find(|operation| operation.op == "set_local_from_masked_result")
        .context("visible intro palette broadcast has no accumulator transform")?;
    let cadence = operations[..broadcast_index]
        .iter()
        .rev()
        .find(|operation| operation.op == "return_unless_mask_equal")
        .context("visible intro palette broadcast has no cadence guard")?;
    anyhow::ensure!(
        cadence
            .fields
            .get("source")
            .and_then(serde_json::Value::as_str)
            == Some("intro_scene_frame")
            && transform
                .fields
                .get("source")
                .and_then(serde_json::Value::as_str)
                == Some("intro_scene_frame")
            && transform
                .fields
                .get("name")
                .and_then(serde_json::Value::as_str)
                == Some("accumulator")
            && transform
                .fields
                .get("wrap")
                .and_then(serde_json::Value::as_str)
                == Some("u8"),
        "visible intro palette broadcast has unsupported source operands"
    );
    let byte_field = |operation: &crystal_assets::RuntimePresentationOperation,
                      field: &str|
     -> Result<u8> {
        operation
            .fields
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .with_context(|| format!("visible intro palette broadcast has no byte {field}"))
    };
    if frame & byte_field(cadence, "mask")? != byte_field(cadence, "operand")? {
        return Ok(None);
    }
    let shift_left = byte_field(transform, "shift_left")?;
    anyhow::ensure!(
        shift_left < 8,
        "visible intro palette broadcast shift exceeds one byte"
    );
    let accumulator = (frame & byte_field(transform, "mask")?)
        .wrapping_shl(u32::from(shift_left));
    let operation = &operations[broadcast_index];
    let valid_values = operation
        .fields
        .get("source_offset")
        .and_then(|source_offset| source_offset.get("valid_values"))
        .and_then(serde_json::Value::as_array)
        .context("visible intro palette broadcast has no source offset domain")?;
    anyhow::ensure!(
        operation
            .fields
            .get("source_offset")
            .and_then(|source_offset| source_offset.get("source"))
            .and_then(serde_json::Value::as_str)
            == Some("accumulator")
            && transform
                .fields
                .get("valid_values")
                .and_then(serde_json::Value::as_array)
                == Some(valid_values),
        "visible intro palette broadcast source domain disagrees with its transform"
    );
    let palette_index = valid_values
        .iter()
        .position(|value| value.as_u64() == Some(u64::from(accumulator)))
        .context("visible intro palette accumulator is outside its source domain")?;
    anyhow::ensure!(
        operation
            .fields
            .get("bytes_per_palette")
            .and_then(serde_json::Value::as_u64)
            == Some(8)
            && operation
                .fields
                .get("destination")
                .and_then(serde_json::Value::as_str)
                == Some("wBGPals2")
            && operation
                .fields
                .get("destination_palette_count")
                .and_then(serde_json::Value::as_u64)
                == Some(8)
            && operation
                .fields
                .get("behavior")
                .and_then(serde_json::Value::as_str)
                == Some("repeat_selected_palette"),
        "visible intro palette broadcast has unsupported destination semantics"
    );
    let palette = operation
        .fields
        .get("palettes")
        .and_then(serde_json::Value::as_array)
        .and_then(|palettes| palettes.get(palette_index))
        .and_then(serde_json::Value::as_array)
        .context("visible intro palette broadcast has no selected palette")?;
    anyhow::ensure!(
        palette.len() == 4,
        "visible intro palette broadcast selected palette has invalid color count"
    );
    let mut colors = [[0_u8; 3]; 4];
    for (color_index, source_color) in palette.iter().enumerate() {
        let channels = source_color
            .as_array()
            .context("visible intro palette broadcast color is not RGB")?;
        anyhow::ensure!(
            channels.len() == 3,
            "visible intro palette broadcast color has invalid channel count"
        );
        for (channel_index, channel) in channels.iter().enumerate() {
            colors[color_index][channel_index] = channel
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| *value < 32)
                .context("visible intro palette broadcast channel is not RGB555")?
                * 8;
        }
    }
    Ok(Some(VisibleIntroPaletteEffect::Scene24Fade { colors }))
}

fn visible_intro_indexed_palette_effect(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    palette_argument: u8,
    timer: u8,
) -> Result<VisibleIntroPaletteEffect> {
    let mut operations = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| operation.op == "copy_indexed_palette");
    let operation = operations
        .next()
        .context("visible intro scene has no copy_indexed_palette operation")?;
    anyhow::ensure!(
        operations.next().is_none(),
        "visible intro scene has duplicate copy_indexed_palette operations"
    );
    let selector = operation
        .fields
        .get("selector")
        .context("visible intro indexed palette has no selector")?;
    let mask = selector
        .get("mask")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro indexed palette has no byte selector mask")?;
    anyhow::ensure!(
        selector
            .get("source")
            .and_then(serde_json::Value::as_str)
            == Some("wIntroSceneTimer")
            && selector
                .get("byte_scale")
                .and_then(serde_json::Value::as_u64)
                == Some(8)
            && operation
                .fields
                .get("bytes_per_palette")
                .and_then(serde_json::Value::as_u64)
                == Some(8)
            && operation
                .fields
                .get("destinations")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|destinations| {
                    destinations.iter().map(serde_json::Value::as_str).eq([
                        Some("wBGPals2"),
                        Some("wBGPals1"),
                    ])
                }),
        "visible intro indexed palette has unsupported copy semantics"
    );
    let variants = operation
        .fields
        .get("palette_argument")
        .and_then(|argument| argument.get("variants"))
        .and_then(serde_json::Value::as_array)
        .context("visible intro indexed palette has no resource variants")?;
    anyhow::ensure!(
        operation
            .fields
            .get("palette_argument")
            .and_then(|argument| argument.get("source"))
            .and_then(serde_json::Value::as_str)
            == Some("accumulator"),
        "visible intro indexed palette has unsupported argument source"
    );
    let matching = variants
        .iter()
        .filter(|variant| {
            variant.get("value").and_then(serde_json::Value::as_u64)
                == Some(u64::from(palette_argument))
                || (palette_argument != 0
                    && variant
                        .get("predicate")
                        .and_then(serde_json::Value::as_str)
                        == Some("nonzero"))
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        matching.len() == 1,
        "visible intro indexed palette argument resolves to {} resources",
        matching.len()
    );
    let palette_resource = matching[0]
        .get("resource")
        .and_then(serde_json::Value::as_str)
        .filter(|resource| {
            resource.starts_with("gfx/intro/") && resource.ends_with(".pal")
        })
        .context("visible intro indexed palette resource is invalid")?;
    Ok(VisibleIntroPaletteEffect::AppearUnown {
        palette_resource: palette_resource.to_string(),
        revealed: timer & mask,
    })
}

fn visible_intro_bg_palette_clear_color(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    predecrement_value: u8,
) -> Result<Option<[u8; 3]>> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let clear_index = operations
        .iter()
        .position(|operation| {
            operation.op == "fill_memory"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("wBGPals2")
        })
        .context("visible intro scene has no background palette clear operation")?;
    anyhow::ensure!(
        operations
            .iter()
            .skip(clear_index + 1)
            .all(|operation| {
                operation.op != "fill_memory"
                    || operation
                        .fields
                        .get("target")
                        .and_then(serde_json::Value::as_str)
                        != Some("wBGPals2")
            }),
        "visible intro scene has duplicate background palette clear operations"
    );
    let clear = &operations[clear_index];
    anyhow::ensure!(
        clear
            .fields
            .get("byte_count")
            .and_then(serde_json::Value::as_u64)
            == Some(128),
        "visible intro palette clear does not cover all BG and OBJ palettes"
    );
    let fill_byte = clear
        .fields
        .get("value")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro palette clear fill byte is invalid")?;
    let fill_word = u16::from(fill_byte) | (u16::from(fill_byte) << 8);
    let color = visible_intro_rgb555(u64::from(fill_word & 0x7fff))?;
    let branch = operations[..clear_index]
        .iter()
        .rev()
        .find(|operation| {
            operation.op == "branch_compare"
                && operation
                    .fields
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    == Some("predecrement_value")
        })
        .context("visible intro background palette clear has no source branch")?;
    let threshold = branch
        .fields
        .get("operand")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro background palette clear threshold is invalid")?;
    anyhow::ensure!(
        branch
            .fields
            .get("predicate")
            .and_then(serde_json::Value::as_str)
            == Some("equal"),
        "visible intro background palette clear has unsupported branch predicate"
    );
    Ok((predecrement_value <= threshold).then_some(color))
}

fn visible_intro_linear_scroll_step(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    target: &str,
    frame: u8,
    current: u8,
) -> Result<(bool, u8)> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let mut deltas = operations.iter().filter(|operation| {
        operation.op == "add_memory_byte"
            && operation
                .fields
                .get("target")
                .and_then(serde_json::Value::as_str)
                == Some(target)
    });
    let delta_op = deltas
        .next()
        .context("visible intro linear scroll has no source increment")?;
    anyhow::ensure!(
        deltas.next().is_none()
            && delta_op
                .fields
                .get("address_space")
                .and_then(serde_json::Value::as_str)
                == Some("hram")
            && delta_op
                .fields
                .get("wrap")
                .and_then(serde_json::Value::as_str)
                == Some("u8"),
        "visible intro linear scroll has unsupported increment semantics"
    );
    let delta = delta_op
        .fields
        .get("delta")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| *value != 0)
        .context("visible intro linear scroll increment is invalid")?;

    let stops = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.op.as_str(),
                "return_if_memory_zero" | "return_if_memory_equal"
            ) && operation
                .fields
                .get("source")
                .and_then(serde_json::Value::as_str)
                == Some(target)
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        stops.len() == 1,
        "visible intro linear scroll resolves to {} stop operations",
        stops.len()
    );
    let stop = match stops[0].op.as_str() {
        "return_if_memory_zero" => 0,
        "return_if_memory_equal" => stops[0]
            .fields
            .get("operand")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .context("visible intro linear scroll stop value is invalid")?,
        _ => unreachable!(),
    };

    let ends = operations.iter().filter(|operation| {
        operation.op == "branch_compare"
            && operation
                .fields
                .get("value")
                .and_then(serde_json::Value::as_str)
                == Some("intro_scene_frame")
            && operation
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("unsigned_greater_or_equal")
    });
    let mut ends = ends.collect::<Vec<_>>();
    anyhow::ensure!(
        ends.len() == 1,
        "visible intro linear scroll resolves to {} completion branches",
        ends.len()
    );
    let end = ends
        .pop()
        .and_then(|operation| operation.fields.get("operand"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro linear scroll completion threshold is invalid")?;
    if frame >= end {
        return Ok((true, current));
    }
    Ok((false, if current == stop { current } else { current.wrapping_add(delta) }))
}

fn visible_intro_source_byte_write(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    target: &str,
) -> Result<u8> {
    let writes = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| {
            operation.op == "write_memory_byte"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some(target)
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        writes.len() == 1,
        "visible intro scene resolves to {} literal writes for {target}",
        writes.len()
    );
    writes[0]
        .fields
        .get("value")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .with_context(|| format!("visible intro scene has an invalid byte write for {target}"))
}

fn visible_intro_attrmap_fills(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<Vec<VisibleIntroAttrmapFill>> {
    let operations = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| {
            operation.op == "fill_memory"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("wAttrmap")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        !operations.is_empty(),
        "visible intro attrmap setup has no source fills"
    );
    let mut cursor = 0_usize;
    let mut fills = Vec::with_capacity(operations.len());
    for operation in operations {
        let count = operation
            .fields
            .get("byte_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value != 0)
            .context("visible intro attrmap fill byte count is invalid")?;
        let value = operation
            .fields
            .get("value")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .context("visible intro attrmap fill value is invalid")?;
        let end = cursor
            .checked_add(count)
            .context("visible intro attrmap fill range overflows")?;
        fills.push(VisibleIntroAttrmapFill {
            start: cursor,
            end,
            value,
        });
        cursor = end;
    }
    anyhow::ensure!(
        cursor == 20 * 18,
        "visible intro attrmap fills cover {cursor} bytes instead of the visible 360"
    );
    Ok(fills)
}

fn visible_intro_indexed_tile_override(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    frame: u8,
) -> Result<Option<VisibleIntroTileOverride>> {
    let requests = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| operation.op == "indexed_2bpp_request")
        .collect::<Vec<_>>();
    anyhow::ensure!(
        requests.len() == 1,
        "visible intro scene resolves to {} indexed tile requests",
        requests.len()
    );
    let request = requests[0];
    let condition = request
        .fields
        .get("condition")
        .context("visible intro indexed tile request has no condition")?;
    anyhow::ensure!(
        condition.get("source").and_then(serde_json::Value::as_str)
            == Some("wIntroSceneFrameCounter")
            && condition
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("unsigned_less_than"),
        "visible intro indexed tile request has unsupported condition"
    );
    let cutoff = condition
        .get("operand")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro indexed tile request cutoff is invalid")?;
    if frame >= cutoff {
        return Ok(None);
    }
    let selector = request
        .fields
        .get("selector")
        .context("visible intro indexed tile request has no selector")?;
    anyhow::ensure!(
        selector.get("source").and_then(serde_json::Value::as_str)
            == Some("wIntroSceneFrameCounter"),
        "visible intro indexed tile request has unsupported selector source"
    );
    let mask = selector
        .get("mask")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro indexed tile selector mask is invalid")?;
    let shift = selector
        .get("shift_right")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .context("visible intro indexed tile selector shift is invalid")?;
    let selected_offset = (frame & mask) >> shift;
    let offsets = selector
        .get("byte_offsets")
        .and_then(serde_json::Value::as_array)
        .context("visible intro indexed tile selector has no offsets")?;
    let selected_index = offsets
        .iter()
        .position(|offset| offset.as_u64() == Some(u64::from(selected_offset)))
        .context("visible intro indexed tile selector resolves outside its table")?;
    let entries = request
        .fields
        .get("table")
        .and_then(|table| table.get("entries"))
        .and_then(serde_json::Value::as_array)
        .context("visible intro indexed tile request has no table entries")?;
    anyhow::ensure!(
        entries.len() == offsets.len(),
        "visible intro indexed tile table and selector domains differ"
    );
    let resource = entries[selected_index]
        .get("path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| path.starts_with("gfx/intro/") && path.ends_with(".2bpp"))
        .context("visible intro indexed tile resource is invalid")?;
    let bytes_per_tile = request
        .fields
        .get("bytes_per_tile")
        .and_then(serde_json::Value::as_u64)
        .filter(|value| *value != 0)
        .context("visible intro indexed tile size is invalid")?;
    let target_byte_offset = request
        .fields
        .get("target_byte_offset")
        .and_then(serde_json::Value::as_u64)
        .context("visible intro indexed tile target offset is invalid")?;
    anyhow::ensure!(
        target_byte_offset % bytes_per_tile == 0,
        "visible intro indexed tile target is not tile-aligned"
    );
    Ok(Some(VisibleIntroTileOverride {
        tile_id_start: u8::try_from(target_byte_offset / bytes_per_tile)
            .context("visible intro indexed target tile exceeds one byte")?,
        tile_count: request
            .fields
            .get("tile_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value != 0)
            .context("visible intro indexed tile count is invalid")?,
        target_vram_bank: request
            .fields
            .get("target_vram_bank")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value <= 1)
            .context("visible intro indexed tile VRAM bank is invalid")?,
        resource: resource.to_string(),
    }))
}

fn visible_intro_reset_ly_overrides(
    intro: &mut VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<()> {
    let fills = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| {
            operation.op == "fill_memory"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("wLYOverrides")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        fills.len() == 1,
        "visible intro scene resolves to {} LY override resets",
        fills.len()
    );
    let fill = fills[0];
    let byte_count = fill
        .fields
        .get("byte_count")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .context("visible intro LY override reset byte count is invalid")?;
    let value = fill
        .fields
        .get("value")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro LY override reset fill byte is invalid")?;
    anyhow::ensure!(
        byte_count == intro.ly_overrides.len()
            && fill
                .fields
                .get("direction")
                .and_then(serde_json::Value::as_str)
                == Some("ascending"),
        "visible intro LY override reset has unsupported fill semantics"
    );
    intro.ly_overrides.fill(value);
    Ok(())
}

fn visible_intro_perspective_scroll(
    intro: &mut VisibleIntroScreen,
    program: &RuntimePresentationProgram,
    memory_frame: u8,
) -> Result<()> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let effects = operations
        .iter()
        .filter(|operation| operation.op == "perspective_scroll")
        .collect::<Vec<_>>();
    anyhow::ensure!(
        effects.len() == 1,
        "visible intro scene resolves to {} perspective scroll operations",
        effects.len()
    );
    let effect = effects[0];
    let byte_count = effect
        .fields
        .get("byte_count")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .context("visible intro perspective scroll byte count is invalid")?;
    let parity_mask = effect
        .fields
        .get("frame")
        .and_then(|frame| frame.get("parity_mask"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro perspective scroll parity mask is invalid")?;
    anyhow::ensure!(
        byte_count == intro.ly_overrides.len()
            && effect
                .fields
                .get("frame")
                .and_then(|frame| frame.get("source"))
                .and_then(serde_json::Value::as_str)
                == Some("wIntroSceneFrameCounter"),
        "visible intro perspective scroll has unsupported memory domain"
    );
    let bands = effect
        .fields
        .get("bands")
        .and_then(serde_json::Value::as_array)
        .context("visible intro perspective scroll has no bands")?;
    anyhow::ensure!(
        bands.len() == 2,
        "visible intro perspective scroll requires exactly two source bands"
    );
    for band in bands {
        let cadence = band
            .get("cadence")
            .and_then(serde_json::Value::as_str)
            .context("visible intro perspective scroll band has no cadence")?;
        let active = match cadence {
            "every_frame" => true,
            "odd_frames" => memory_frame & parity_mask != 0,
            _ => anyhow::bail!("visible intro perspective scroll cadence {cadence} is unsupported"),
        };
        if !active {
            continue;
        }
        let offset = band
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .context("visible intro perspective scroll band offset is invalid")?;
        let count = band
            .get("byte_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .context("visible intro perspective scroll band length is invalid")?;
        let delta = band
            .get("delta")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .context("visible intro perspective scroll band delta is invalid")?;
        anyhow::ensure!(
            band.get("value_source").and_then(serde_json::Value::as_str)
                == Some("first_byte")
                && offset < intro.ly_overrides.len()
                && offset + count <= intro.ly_overrides.len(),
            "visible intro perspective scroll band has unsupported fill semantics"
        );
        let value = intro.ly_overrides[offset].wrapping_add(delta);
        intro.ly_overrides[offset..offset + count].fill(value);
    }
    let source_offset = effect
        .fields
        .get("horizontal_scroll")
        .and_then(|scroll| scroll.get("source_offset"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|offset| *offset < intro.ly_overrides.len())
        .context("visible intro perspective scroll source offset is invalid")?;
    anyhow::ensure!(
        effect
            .fields
            .get("horizontal_scroll")
            .and_then(|scroll| scroll.get("target"))
            .and_then(serde_json::Value::as_str)
            == Some("hSCX"),
        "visible intro perspective scroll has unsupported horizontal target"
    );
    intro.scroll_x = intro.ly_overrides[source_offset];
    Ok(())
}

fn visible_intro_perspective_completion_frame(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<u8> {
    let branches = visible_intro_scene_operations(intro, program)?
        .iter()
        .filter(|operation| {
            operation.op == "branch_compare"
                && operation
                    .fields
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    == Some("intro_scene_frame")
                && operation
                    .fields
                    .get("predicate")
                    .and_then(serde_json::Value::as_str)
                    == Some("equal")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        branches.len() == 1,
        "visible intro perspective scene resolves to {} completion branches",
        branches.len()
    );
    branches[0]
        .fields
        .get("operand")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("visible intro perspective completion frame is invalid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VisibleIntroPerspectiveMotionRule {
    motion_start_frame: u8,
    finish_offset: u8,
    motion_delta: u8,
}

fn visible_intro_perspective_motion_rule(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<VisibleIntroPerspectiveMotionRule> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let frame_branches = operations
        .iter()
        .filter(|operation| {
            operation.op == "branch_compare"
                && operation
                    .fields
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    == Some("intro_scene_frame")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        frame_branches.len() == 2
            && frame_branches[0]
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("equal")
            && frame_branches[1]
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("unsigned_greater_or_equal"),
        "visible intro perspective motion has unsupported frame branch topology"
    );
    let frame_operand = |operation: &crystal_assets::RuntimePresentationOperation| {
        operation
            .fields
            .get("operand")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
    };
    let motion_start_frame = frame_operand(frame_branches[1])
        .context("visible intro perspective motion start frame is invalid")?;
    anyhow::ensure!(
        frame_operand(frame_branches[0]) == Some(motion_start_frame),
        "visible intro perspective audio and motion boundaries differ"
    );
    let finishes = operations
        .iter()
        .filter(|operation| {
            operation.op == "branch_compare"
                && operation
                    .fields
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    == Some("global_anim_x")
                && operation
                    .fields
                    .get("predicate")
                    .and_then(serde_json::Value::as_str)
                    == Some("equal")
        })
        .collect::<Vec<_>>();
    let transforms = operations
        .iter()
        .filter(|operation| {
            operation.op == "transform_memory_byte"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("wGlobalAnimXOffset")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        finishes.len() == 1
            && transforms.len() == 1
            && transforms[0]
                .fields
                .get("input")
                .and_then(serde_json::Value::as_str)
                == Some("global_anim_x")
            && transforms[0]
                .fields
                .get("operator")
                .and_then(serde_json::Value::as_str)
                == Some("subtract")
            && transforms[0]
                .fields
                .get("wrap")
                .and_then(serde_json::Value::as_str)
                == Some("u8"),
        "visible intro perspective motion has unsupported offset topology"
    );
    Ok(VisibleIntroPerspectiveMotionRule {
        motion_start_frame,
        finish_offset: frame_operand(finishes[0])
            .context("visible intro perspective finish offset is invalid")?,
        motion_delta: frame_operand(transforms[0])
            .context("visible intro perspective motion delta is invalid")?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VisibleIntroSuicuneRunRule {
    scroll_delta: u8,
    end_frame: u8,
    jump_frame: u8,
    run_frame: u8,
    jump_timer: u8,
    disappear_below: u8,
    jump_offset_delta: u8,
    run_offset_delta: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VisibleIntroUnownRevealRule {
    end_frame: u8,
    reveal_end_frame: u8,
    reveal_start_frame: u8,
    scroll_end_frame: u8,
    scroll_delta: u8,
    phase_subtract: u8,
    cadence_mask: u8,
    cadence_operand: u8,
    timer_mask: u8,
    timer_shift: u8,
    palette_argument: u8,
}

fn visible_intro_unown_reveal_rule(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<VisibleIntroUnownRevealRule> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let byte = |operation: &crystal_assets::RuntimePresentationOperation, field: &str| {
        operation
            .fields
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
    };
    let frame_branches = operations
        .iter()
        .filter(|operation| {
            matches!(operation.op.as_str(), "branch_compare" | "return_if_compare")
                && operation
                    .fields
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    == Some("intro_scene_frame")
                && operation
                    .fields
                    .get("predicate")
                    .and_then(serde_json::Value::as_str)
                    == Some("unsigned_greater_or_equal")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        frame_branches.len() == 4
            && frame_branches[0].op == "branch_compare"
            && frame_branches[1].op == "return_if_compare"
            && frame_branches[2].op == "branch_compare"
            && frame_branches[3].op == "return_if_compare",
        "visible intro Unown reveal has unsupported frame branch topology"
    );
    let increments = operations
        .iter()
        .filter(|operation| {
            operation.op == "increment_memory_byte"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("hSCY")
        })
        .collect::<Vec<_>>();
    let phases = operations
        .iter()
        .filter(|operation| operation.op == "set_local_from_result")
        .collect::<Vec<_>>();
    let cadences = operations
        .iter()
        .filter(|operation| operation.op == "return_unless_mask_equal")
        .collect::<Vec<_>>();
    let timers = operations
        .iter()
        .filter(|operation| {
            operation.op == "write_memory_byte_from_masked_result"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("wIntroSceneTimer")
        })
        .collect::<Vec<_>>();
    let arguments = operations
        .iter()
        .filter(|operation| {
            operation.op == "set_local"
                && operation
                    .fields
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    == Some("accumulator")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        increments.len() == 1
            && phases.len() == 1
            && cadences.len() == 1
            && timers.len() == 1
            && arguments.len() == 1
            && increments[0]
                .fields
                .get("wrap")
                .and_then(serde_json::Value::as_str)
                == Some("u8")
            && phases[0]
                .fields
                .get("source")
                .and_then(serde_json::Value::as_str)
                == Some("intro_scene_frame"),
        "visible intro Unown reveal has unsupported operation topology"
    );
    Ok(VisibleIntroUnownRevealRule {
        end_frame: byte(frame_branches[0], "operand")
            .context("visible intro Unown reveal end frame is invalid")?,
        reveal_end_frame: byte(frame_branches[1], "operand")
            .context("visible intro Unown reveal upper boundary is invalid")?,
        reveal_start_frame: byte(frame_branches[2], "operand")
            .context("visible intro Unown reveal lower boundary is invalid")?,
        scroll_end_frame: byte(frame_branches[3], "operand")
            .context("visible intro Unown scroll boundary is invalid")?,
        scroll_delta: byte(increments[0], "delta")
            .context("visible intro Unown scroll delta is invalid")?,
        phase_subtract: byte(phases[0], "subtract")
            .context("visible intro Unown reveal phase subtraction is invalid")?,
        cadence_mask: byte(cadences[0], "mask")
            .context("visible intro Unown reveal cadence mask is invalid")?,
        cadence_operand: byte(cadences[0], "operand")
            .context("visible intro Unown reveal cadence operand is invalid")?,
        timer_mask: byte(timers[0], "mask")
            .context("visible intro Unown reveal timer mask is invalid")?,
        timer_shift: byte(timers[0], "shift_right")
            .context("visible intro Unown reveal timer shift is invalid")?,
        palette_argument: byte(arguments[0], "value")
            .context("visible intro Unown reveal palette argument is invalid")?,
    })
}

fn visible_intro_suicune_run_rule(
    intro: &VisibleIntroScreen,
    program: &RuntimePresentationProgram,
) -> Result<VisibleIntroSuicuneRunRule> {
    let operations = visible_intro_scene_operations(intro, program)?;
    let byte = |operation: &crystal_assets::RuntimePresentationOperation, field: &str| {
        operation
            .fields
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
    };
    let subtracts = |target: &str| {
        operations
            .iter()
            .filter(|operation| {
                operation.op == "subtract_memory_byte"
                    && operation
                        .fields
                        .get("target")
                        .and_then(serde_json::Value::as_str)
                        == Some(target)
                    && operation
                        .fields
                        .get("wrap")
                        .and_then(serde_json::Value::as_str)
                        == Some("u8")
            })
            .collect::<Vec<_>>()
    };
    let scroll = subtracts("hSCX");
    anyhow::ensure!(
        scroll.len() == 1,
        "visible intro Suicune run resolves to {} horizontal scroll operations",
        scroll.len()
    );
    let offsets = subtracts("wGlobalAnimXOffset");
    anyhow::ensure!(
        offsets.len() == 2,
        "visible intro Suicune run resolves to {} animation offset operations",
        offsets.len()
    );
    let branches = operations
        .iter()
        .filter(|operation| {
            operation.op == "branch_compare"
                && operation
                    .fields
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    == Some("intro_scene_frame")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        branches.len() == 4
            && branches[0]
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("equal")
            && branches[1]
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("equal")
            && branches[2]
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("unsigned_greater_or_equal")
            && branches[3]
                .fields
                .get("predicate")
                .and_then(serde_json::Value::as_str)
                == Some("unsigned_greater_or_equal")
            && byte(branches[1], "operand") == byte(branches[2], "operand"),
        "visible intro Suicune run has unsupported frame branch topology"
    );
    let timer_writes = operations
        .iter()
        .filter(|operation| {
            operation.op == "write_memory_byte"
                && operation
                    .fields
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    == Some("wIntroSceneTimer")
        })
        .collect::<Vec<_>>();
    let disappear = operations
        .iter()
        .filter(|operation| {
            operation.op == "branch_memory_compare"
                && operation
                    .fields
                    .get("source")
                    .and_then(serde_json::Value::as_str)
                    == Some("wGlobalAnimXOffset")
                && operation
                    .fields
                    .get("predicate")
                    .and_then(serde_json::Value::as_str)
                    == Some("unsigned_less_than")
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        timer_writes.len() == 1 && disappear.len() == 1,
        "visible intro Suicune run has unsupported jump effect topology"
    );
    Ok(VisibleIntroSuicuneRunRule {
        scroll_delta: byte(scroll[0], "delta")
            .context("visible intro Suicune horizontal scroll delta is invalid")?,
        end_frame: byte(branches[0], "operand")
            .context("visible intro Suicune run completion frame is invalid")?,
        jump_frame: byte(branches[1], "operand")
            .context("visible intro Suicune jump frame is invalid")?,
        run_frame: byte(branches[3], "operand")
            .context("visible intro Suicune run frame is invalid")?,
        jump_timer: byte(timer_writes[0], "value")
            .context("visible intro Suicune jump timer is invalid")?,
        disappear_below: byte(disappear[0], "operand")
            .context("visible intro Suicune disappearance offset is invalid")?,
        jump_offset_delta: byte(offsets[0], "delta")
            .context("visible intro Suicune jump offset delta is invalid")?,
        run_offset_delta: byte(offsets[1], "delta")
            .context("visible intro Suicune run offset delta is invalid")?,
    })
}

fn visible_intro_rgb555(value: u64) -> Result<[u8; 3]> {
    let value = u16::try_from(value)
        .ok()
        .filter(|value| *value <= 0x7fff)
        .context("visible intro palette color is not RGB555")?;
    Ok([
        ((value & 0x1f) as u8) * 8,
        (((value >> 5) & 0x1f) as u8) * 8,
        (((value >> 10) & 0x1f) as u8) * 8,
    ])
}

fn visible_intro_next_scene(intro: &mut VisibleIntroScreen) {
    intro.jumptable_index = intro.jumptable_index.saturating_add(1);
    intro.scene_dispatch_tick = 0;
    intro.scene_frame_counter = intro.next_scene_frame_counter.take().unwrap_or(0);
    intro.scene_timer = 0;
    intro.scene_delay_frames = 0;
    if intro.jumptable_index >= intro.scene_count {
        intro.finished = true;
    }
}

fn queue_visible_intro_music(runtime_shell: &mut BevyRuntimeShell, music_id: &str) -> Result<()> {
    if runtime_shell.active_music.as_deref() == Some(music_id) {
        return Ok(());
    }
    if is_silent_music_id(music_id) {
        return stop_visible_silent_music(runtime_shell, music_id, "audio:music:intro:stop");
    }
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, music_id)?;
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: music_id.to_string(),
            kind: ModpackAudioKind::Music,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    runtime_shell.active_music = Some(music_id.to_string());
    runtime_shell.faded_music = None;
    runtime_shell
        .last_audio_events
        .push(format!("queued intro music {music_id}"));
    Ok(())
}

fn queue_visible_sound_effect(
    runtime_audio: &crate::RuntimeAudioCatalog,
    pending_audio: &mut Vec<BevyAudioCommand>,
    last_audio_events: &mut Vec<String>,
    sfx_id: &str,
) -> Result<()> {
    let playback = runtime_audio.require_playback_entry(AudioKind::SoundEffect, sfx_id)?;
    enqueue_bevy_audio_command(
        pending_audio,
        BevyAudioCommand {
            audio_id: sfx_id.to_string(),
            kind: ModpackAudioKind::SoundEffect,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    last_audio_events.push(format!("queued sound effect {sfx_id}"));
    Ok(())
}

fn queue_visible_shell_sound_effect(
    runtime_shell: &mut BevyRuntimeShell,
    sfx_id: &str,
) -> Result<()> {
    let BevyRuntimeShell {
        shell,
        pending_audio,
        last_audio_events,
        ..
    } = runtime_shell;
    queue_visible_sound_effect(
        shell.runtime().audio(),
        pending_audio,
        last_audio_events,
        sfx_id,
    )
}

fn skip_visible_intro_screen(
    runtime_shell: &mut BevyRuntimeShell,
    input: GameButton,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("intro:skip:{input:?}"))?;
    finish_visible_intro_screen(runtime_shell, "skip")
}

fn reset_visible_title_program(title: &mut TitleMenu) {
    title.phase = VisibleTitlePhase::Entrance;
    title.frame = 0;
    title.main_menu_frame = 0;
    title.scx = title.entrance_start_scx;
    title.title_timer = 0;
    title.joypad_mask = 0;
    title.clock_reset_trigger = false;
    title.presentation_machine.interpreter.operation_index = 0;
    title.presentation_machine.interpreter.current_label = None;
    title
        .presentation_machine
        .memory
        .insert("hSCX".to_string(), u16::from(title.entrance_start_scx));
    title
        .presentation_machine
        .memory
        .insert("wJumptableIndex".to_string(), 0);
    title
        .presentation_machine
        .memory
        .insert("wTitleScreenTimer".to_string(), 0);
    title
        .presentation_machine
        .memory
        .insert("hClockResetTrigger".to_string(), 0);
    title
        .presentation_machine
        .memory
        .insert(title.timeout_fade_register.clone(), 0);
    title.presentation_machine.memory.insert(
        title.crystal_oam_target.clone(),
        u16::from(title.crystal_initial_y),
    );
    title.presentation_machine.values.clear();
}

fn finish_visible_intro_screen(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &'static str,
) -> Result<()> {
    let Some(intro) = runtime_shell.intro_screen.take() else {
        return Ok(());
    };
    // The intro and title are separate audiovisual surfaces.  Do not carry a
    // terminal field fade or any queued intro cue across this boundary: doing
    // so leaves a black first title frame and allows the opening track to
    // overlap the title entrance on a busy macOS audio callback.
    runtime_shell.screen_fade = None;
    runtime_shell.visible_blackout_phase = None;
    runtime_shell.visible_walk_warp_phase = None;
    runtime_shell.pending_audio.clear();
    stop_visible_silent_music(
        runtime_shell,
        "MUSIC_NONE",
        format!("intro:{reason}:music:none"),
    )?;
    if let Some(title) = runtime_shell.title_menu.as_mut() {
        reset_visible_title_program(title);
        queue_visible_sound_effect(
            runtime_shell.shell.runtime().audio(),
            &mut runtime_shell.pending_audio,
            &mut runtime_shell.last_audio_events,
            "SFX_TITLE_SCREEN_ENTRANCE",
        )?;
    }
    runtime_shell.last_audio_events.push(format!(
        "intro {reason} scene={} frame={}",
        intro.jumptable_index, intro.scene_frame_counter
    ));
    set_shell_action_status(runtime_shell, "TITLE INTRO");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn tick_visible_title_screen(
    time: Res<Time>,
    mut clock: ResMut<VisibleSequenceTickClock>,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
) {
    let frames = clock.consume_frames(time.delta_seconds());
    for _ in 0..frames {
        if runtime_shell.intro_screen.is_some() {
            if let Err(error) = tick_visible_intro_screen(&mut runtime_shell) {
                record_visible_runtime_system_error(&mut runtime_shell, error);
                break;
            }
        } else {
            tick_visible_title_screen_state(&mut runtime_shell);
        }
    }
}

fn tick_visible_title_screen_state(runtime_shell: &mut BevyRuntimeShell) {
    if runtime_shell.intro_screen.is_some() {
        return;
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        return;
    }
    let music_fade_active = runtime_shell.music_fade.is_some();
    let execution = (|| -> Result<(Vec<crystal_assets::RuntimePresentationOperation>, Option<u16>)> {
        let BevyRuntimeShell {
            runtime,
            title_menu,
            ..
        } = runtime_shell;
        let Some(title) = title_menu.as_mut() else {
            return Ok((Vec::new(), None));
        };
        title.frame = title.frame.saturating_add(1);
        if matches!(title.phase, VisibleTitlePhase::MainMenu) {
            title.main_menu_frame = title.main_menu_frame.saturating_add(1);
            title.joypad_mask = 0;
            return Ok((Vec::new(), None));
        }
        if matches!(title.phase, VisibleTitlePhase::FadeOut) {
            title.presentation_machine.memory.insert(
                title.timeout_fade_register.clone(),
                if music_fade_active {
                    u16::from(title.timeout_fade_rate)
                } else {
                    0
                },
            );
        }
        let scene = title
            .presentation_machine
            .memory
            .get("wJumptableIndex")
            .copied()
            .context("runtime title program has no wJumptableIndex")?;
        let scene_label = title.presentation_machine.dispatch_label(
            runtime.title_presentation_program(),
            "TitleScreenScene",
            usize::from((scene & 0x7f) as u8),
        )?;
        let joypad_mask = std::mem::take(&mut title.joypad_mask);
        let run = title.presentation_machine.run_from_label(
            runtime.title_presentation_program(),
            &scene_label,
            joypad_mask,
        )?;
        let scene = title
            .presentation_machine
            .memory
            .get("wJumptableIndex")
            .copied()
            .context("runtime title program has no wJumptableIndex after scene execution")?;
        title.scx = title
            .presentation_machine
            .memory
            .get("hSCX")
            .copied()
            .context("runtime title program has no hSCX after scene execution")? as u8;
        title.title_timer = title
            .presentation_machine
            .memory
            .get("wTitleScreenTimer")
            .copied()
            .context("runtime title program has no wTitleScreenTimer after scene execution")?;
        title.clock_reset_trigger = title
            .presentation_machine
            .memory
            .get("hClockResetTrigger")
            .copied()
            .context("runtime title program has no hClockResetTrigger")?
            == 0x34;
        title.phase = match scene & 0x7f {
            0 => VisibleTitlePhase::Entrance,
            1 => VisibleTitlePhase::Timer,
            2 => VisibleTitlePhase::PressStart,
            3 => VisibleTitlePhase::FadeOut,
            value => anyhow::bail!("runtime title program produced invalid scene {value}"),
        };
        let selected_option = if scene & 0x80 != 0 {
            Some(
                title
                .presentation_machine
                .memory
                .get("wTitleScreenSelectedOption")
                .copied()
                .context("runtime title program exited without a selected option")?,
            )
        } else {
            None
        };
        Ok((run.effects, selected_option))
    })();
    let (effects, selected_option) = match execution {
        Ok(execution) => execution,
        Err(error) => {
            record_visible_runtime_system_error(runtime_shell, error);
            return;
        }
    };
    for operation in effects {
        let result = match operation.op.as_str() {
            "play_audio" => operation
                .fields
                .get("audio")
                .and_then(serde_json::Value::as_str)
                .context("runtime title play_audio effect has no audio id")
                .and_then(|audio| queue_visible_intro_music(runtime_shell, audio)),
            "fade_audio" => begin_visible_title_audio_fade(runtime_shell, &operation),
            _ => Ok(()),
        };
        if let Err(error) = result {
            record_visible_runtime_system_error(runtime_shell, error);
            return;
        }
    }
    match selected_option {
        Some(0) => {
            if let Err(error) = open_visible_title_main_menu(runtime_shell) {
                record_visible_runtime_system_error(runtime_shell, error);
            }
        }
        Some(1) => {
            if let Err(error) = open_visible_delete_save_screen(runtime_shell) {
                record_visible_runtime_system_error(runtime_shell, error);
            }
        }
        Some(2) => {
            let intro_parameters = match RuntimeIntroPresentationParameters::from_program(
                runtime_shell.runtime.title_presentation_program(),
            ) {
                Ok(parameters) => parameters,
                Err(error) => {
                    record_visible_runtime_system_error(runtime_shell, error);
                    return;
                }
            };
            let mut intro = VisibleIntroScreen::from_parameters(intro_parameters);
            if let Err(error) = apply_visible_intro_background_binding(
                &mut intro,
                &runtime_shell.runtime.data().runtime_title_screen.program,
            ) {
                record_visible_runtime_system_error(runtime_shell, error);
                return;
            }
            runtime_shell.intro_screen = Some(intro);
            if let Some(title) = runtime_shell.title_menu.as_mut() {
                reset_visible_title_program(title);
            }
            set_shell_action_status(runtime_shell, "CRYSTAL INTRO");
        }
        Some(4) => {
            if let Err(error) = open_visible_clock_reset_screen(runtime_shell) {
                record_visible_runtime_system_error(runtime_shell, error);
            }
        }
        Some(value) => record_visible_runtime_system_error(
            runtime_shell,
            anyhow::anyhow!("runtime title program selected invalid option {value}"),
        ),
        None => {}
    }
}

fn begin_visible_title_audio_fade(
    runtime_shell: &mut BevyRuntimeShell,
    operation: &crystal_assets::RuntimePresentationOperation,
) -> Result<()> {
    let audio = operation
        .fields
        .get("audio")
        .and_then(serde_json::Value::as_str)
        .context("runtime title fade_audio effect has no audio target")?;
    let register = operation
        .fields
        .get("fade_register")
        .and_then(serde_json::Value::as_object)
        .context("runtime title fade_audio effect has no fade register")?;
    let target = register
        .get("target")
        .and_then(serde_json::Value::as_str)
        .context("runtime title fade_audio effect has no register target")?;
    let rate = register
        .get("value")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .context("runtime title fade_audio effect has no exact source rate byte")?;
    let title = runtime_shell
        .title_menu
        .as_ref()
        .context("runtime title fade_audio effect has no title state")?;
    anyhow::ensure!(
        target == title.timeout_fade_register
            && rate == title.timeout_fade_rate
            && audio == title.timeout_fade_audio,
        "runtime title fade_audio effect does not match its certified title parameters"
    );
    anyhow::ensure!(
        title.presentation_machine.memory.get(target).copied() == Some(u16::from(rate)),
        "runtime title fade_audio effect did not write source register {target}={rate}"
    );
    begin_visible_music_fade(runtime_shell, audio, u16::from(rate))
}

fn visible_title_main_menu_ready(title: &TitleMenu) -> bool {
    matches!(title.phase, VisibleTitlePhase::MainMenu)
}

fn visible_title_accepts_start(title: &TitleMenu) -> bool {
    matches!(
        title.phase,
        VisibleTitlePhase::PressStart | VisibleTitlePhase::MainMenu
    )
}

fn open_visible_title_main_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "start");
    };
    if !visible_title_accepts_start(title) {
        let phase = title.phase;
        let scx = title.scx;
        let title_timer = title.title_timer;
        record_visible_runtime_action(runtime_shell, "title:start:ignored")?;
        runtime_shell.last_audio_events.push(format!(
            "title Start ignored phase={:?} scx={} timer={}",
            phase, scx, title_timer
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let menu_len = visible_title_menu_options(runtime_shell, title).len();
    let Some(title) = runtime_shell.title_menu.as_mut() else {
        return handle_visible_no_active_title_menu(runtime_shell, "start");
    };
    title
        .presentation_machine
        .memory
        .insert("wTitleScreenSelectedOption".to_string(), 0);
    let scene = title
        .presentation_machine
        .memory
        .entry("wJumptableIndex".to_string())
        .or_insert(0);
    *scene |= 0x80;
    title.phase = VisibleTitlePhase::MainMenu;
    title.main_menu_frame = 0;
    title.clock_reset_trigger = false;
    title.cursor.option_index = title
        .main_menu
        .default_option
        .saturating_sub(1)
        .min(menu_len.saturating_sub(1));
    record_visible_runtime_action(runtime_shell, "title:start:main_menu")?;
    runtime_shell
        .last_audio_events
        .push("title opened main menu".to_string());
    set_shell_action_status(runtime_shell, "TITLE MENU");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_gender_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    reset_visible_navigation_state(runtime_shell);
    let definition = RuntimeGenderMenuDefinition::from_program(
        runtime_shell.runtime.title_presentation_program(),
    )?;
    let selected_index = definition.default_option - 1;
    runtime_shell.pending_gender_selection = Some(VisibleGenderSelection {
        definition,
        selected_index,
        confirmed: false,
        confirm_countdown: 0,
        fade_counter: 0,
    });
    record_visible_runtime_action(runtime_shell, "gender:open")?;
    runtime_shell
        .last_audio_events
        .push("opened gender selection".to_string());
    set_shell_action_status(runtime_shell, "GENDER");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

const VISIBLE_TIME_SET_WAKE_TEXT: [&str; 4] = [
    "...... ...... ...... ...... ...... ......",
    "...... ...... ...... ...... ...... ......",
    "Zzz... Hm? Wha... ?\nYou woke me up!",
    "Will you check the\nclock for me?",
];
const VISIBLE_TIME_SET_TEXT_SPEED_FRAMES: u8 = 2;

fn open_visible_time_set_screen(
    runtime_shell: &mut BevyRuntimeShell,
    next: VisibleTimeSetNext,
) -> Result<()> {
    reset_visible_navigation_state(runtime_shell);
    runtime_shell.pending_time_set = Some(VisibleTimeSetScreen {
        phase: VisibleTimeSetPhase::WakeDialogue,
        next,
        wake_index: 0,
        hour: 10,
        minute: 0,
        visible_chars: 0,
        text_timer: 0,
        yes_no_index: 0,
        reaction_text: String::new(),
    });
    record_visible_runtime_action(runtime_shell, format!("time_set:open:{next:?}"))?;
    runtime_shell
        .last_audio_events
        .push("opened time set screen".to_string());
    set_shell_action_status(runtime_shell, "TIME SET");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn tick_visible_time_set_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(time_set) = runtime_shell.pending_time_set.as_mut() else {
        return Ok(());
    };
    let text = visible_time_set_dialog_text(time_set);
    if text.is_empty() {
        return Ok(());
    }
    if visible_time_set_dialog_complete(time_set) {
        return Ok(());
    }
    time_set.text_timer = time_set.text_timer.saturating_add(1);
    if time_set.text_timer >= VISIBLE_TIME_SET_TEXT_SPEED_FRAMES {
        time_set.text_timer = 0;
        time_set.visible_chars = time_set.visible_chars.saturating_add(1);
    }
    Ok(())
}

fn advance_visible_time_set_dialog(time_set: &mut VisibleTimeSetScreen) {
    time_set.visible_chars = visible_time_set_dialog_text(time_set).chars().count();
    time_set.text_timer = 0;
}

fn open_visible_time_set_dialog(time_set: &mut VisibleTimeSetScreen) {
    time_set.visible_chars = 0;
    time_set.text_timer = 0;
}

fn visible_time_set_dialog_complete(time_set: &VisibleTimeSetScreen) -> bool {
    time_set.visible_chars >= visible_time_set_dialog_text(time_set).chars().count()
}

fn visible_time_set_dialog_text(time_set: &VisibleTimeSetScreen) -> String {
    match time_set.phase {
        VisibleTimeSetPhase::WakeDialogue => VISIBLE_TIME_SET_WAKE_TEXT[time_set
            .wake_index
            .min(VISIBLE_TIME_SET_WAKE_TEXT.len().saturating_sub(1))]
        .to_string(),
        VisibleTimeSetPhase::HourConfirm => {
            format!("What?\n{}?", visible_time_set_hour_display(time_set))
        }
        VisibleTimeSetPhase::MinuteConfirm => {
            format!("Whoa!\n{}?", visible_time_set_minute_display(time_set))
        }
        VisibleTimeSetPhase::FinalReaction => time_set.reaction_text.clone(),
        VisibleTimeSetPhase::SetHour
        | VisibleTimeSetPhase::SetMinute
        | VisibleTimeSetPhase::Complete => String::new(),
    }
}

fn visible_time_set_visible_dialog(time_set: &VisibleTimeSetScreen) -> String {
    let text = visible_time_set_dialog_text(time_set);
    text.chars()
        .take(time_set.visible_chars.min(text.chars().count()))
        .collect::<String>()
}

fn press_visible_time_set_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mut time_set) = runtime_shell.pending_time_set.take() else {
        return handle_visible_no_time_set_screen(runtime_shell, "a");
    };
    match time_set.phase {
        VisibleTimeSetPhase::WakeDialogue => {
            if !visible_time_set_dialog_complete(&time_set) {
                advance_visible_time_set_dialog(&mut time_set);
            } else if time_set.wake_index < VISIBLE_TIME_SET_WAKE_TEXT.len().saturating_sub(1) {
                time_set.wake_index += 1;
                open_visible_time_set_dialog(&mut time_set);
            } else {
                time_set.phase = VisibleTimeSetPhase::SetHour;
                open_visible_time_set_dialog(&mut time_set);
            }
            runtime_shell.pending_time_set = Some(time_set);
        }
        VisibleTimeSetPhase::SetHour => {
            time_set.phase = VisibleTimeSetPhase::HourConfirm;
            time_set.yes_no_index = 0;
            open_visible_time_set_dialog(&mut time_set);
            runtime_shell.pending_time_set = Some(time_set);
        }
        VisibleTimeSetPhase::HourConfirm => {
            if time_set.yes_no_index == 0 {
                time_set.phase = VisibleTimeSetPhase::SetMinute;
            } else {
                time_set.phase = VisibleTimeSetPhase::SetHour;
            }
            open_visible_time_set_dialog(&mut time_set);
            runtime_shell.pending_time_set = Some(time_set);
            record_visible_time_set_menu_option(runtime_shell)?;
        }
        VisibleTimeSetPhase::SetMinute => {
            time_set.phase = VisibleTimeSetPhase::MinuteConfirm;
            time_set.yes_no_index = 0;
            open_visible_time_set_dialog(&mut time_set);
            runtime_shell.pending_time_set = Some(time_set);
        }
        VisibleTimeSetPhase::MinuteConfirm => {
            if time_set.yes_no_index == 0 {
                commit_visible_time_set_selection(runtime_shell, &mut time_set)?;
                time_set.phase = VisibleTimeSetPhase::FinalReaction;
                open_visible_time_set_dialog(&mut time_set);
                runtime_shell.pending_time_set = Some(time_set);
            } else {
                time_set.phase = VisibleTimeSetPhase::SetMinute;
                open_visible_time_set_dialog(&mut time_set);
                runtime_shell.pending_time_set = Some(time_set);
            }
            record_visible_time_set_menu_option(runtime_shell)?;
        }
        VisibleTimeSetPhase::FinalReaction => {
            if !visible_time_set_dialog_complete(&time_set) {
                advance_visible_time_set_dialog(&mut time_set);
                runtime_shell.pending_time_set = Some(time_set);
            } else {
                complete_visible_time_set_screen(runtime_shell, time_set)?;
            }
        }
        VisibleTimeSetPhase::Complete => {
            complete_visible_time_set_screen(runtime_shell, time_set)?;
        }
    }
    Ok(())
}

fn press_visible_time_set_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mut time_set) = runtime_shell.pending_time_set.take() else {
        return handle_visible_no_time_set_screen(runtime_shell, "b");
    };
    match time_set.phase {
        VisibleTimeSetPhase::WakeDialogue | VisibleTimeSetPhase::FinalReaction => {
            if !visible_time_set_dialog_complete(&time_set) {
                advance_visible_time_set_dialog(&mut time_set);
            } else if matches!(time_set.phase, VisibleTimeSetPhase::WakeDialogue) {
                return press_visible_time_set_a_button_with_state(runtime_shell, time_set);
            } else {
                complete_visible_time_set_screen(runtime_shell, time_set)?;
                return Ok(());
            }
        }
        VisibleTimeSetPhase::SetHour => {
            time_set.phase = VisibleTimeSetPhase::WakeDialogue;
            time_set.wake_index = VISIBLE_TIME_SET_WAKE_TEXT.len().saturating_sub(1);
            open_visible_time_set_dialog(&mut time_set);
        }
        VisibleTimeSetPhase::HourConfirm => {
            time_set.yes_no_index = 1;
            time_set.phase = VisibleTimeSetPhase::SetHour;
            open_visible_time_set_dialog(&mut time_set);
            record_visible_time_set_menu_option(runtime_shell)?;
        }
        VisibleTimeSetPhase::SetMinute => {
            time_set.phase = VisibleTimeSetPhase::SetHour;
            open_visible_time_set_dialog(&mut time_set);
        }
        VisibleTimeSetPhase::MinuteConfirm => {
            time_set.yes_no_index = 1;
            time_set.phase = VisibleTimeSetPhase::SetMinute;
            open_visible_time_set_dialog(&mut time_set);
            record_visible_time_set_menu_option(runtime_shell)?;
        }
        VisibleTimeSetPhase::Complete => {}
    }
    runtime_shell.pending_time_set = Some(time_set);
    Ok(())
}

fn complete_visible_time_set_screen(
    runtime_shell: &mut BevyRuntimeShell,
    mut time_set: VisibleTimeSetScreen,
) -> Result<()> {
    time_set.phase = VisibleTimeSetPhase::Complete;
    let next = time_set.next;
    record_visible_runtime_action(runtime_shell, format!("time_set:complete:{next:?}"))?;
    runtime_shell
        .last_audio_events
        .push("time set complete".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    match next {
        VisibleTimeSetNext::OakIntro => open_visible_oak_intro_sequence(runtime_shell),
    }
}

fn press_visible_time_set_a_button_with_state(
    runtime_shell: &mut BevyRuntimeShell,
    time_set: VisibleTimeSetScreen,
) -> Result<()> {
    runtime_shell.pending_time_set = Some(time_set);
    press_visible_time_set_a_button(runtime_shell)
}

fn move_visible_time_set_direction(
    runtime_shell: &mut BevyRuntimeShell,
    direction: VisibleTimeSetDirection,
) -> Result<()> {
    let Some(phase) = runtime_shell
        .pending_time_set
        .as_ref()
        .map(|time_set| time_set.phase)
    else {
        return handle_visible_no_time_set_screen(runtime_shell, "cursor");
    };
    match phase {
        VisibleTimeSetPhase::SetHour => match direction {
            VisibleTimeSetDirection::Up => move_visible_time_set_cursor(runtime_shell, 1),
            VisibleTimeSetDirection::Down => move_visible_time_set_cursor(runtime_shell, -1),
            VisibleTimeSetDirection::Left | VisibleTimeSetDirection::Right => {
                record_visible_runtime_action(
                    runtime_shell,
                    format!("time_set:cursor:{direction:?}:ignored"),
                )?;
                trim_event_log(&mut runtime_shell.last_audio_events);
                Ok(())
            }
        },
        VisibleTimeSetPhase::SetMinute => match direction {
            VisibleTimeSetDirection::Up | VisibleTimeSetDirection::Right => {
                move_visible_time_set_cursor(runtime_shell, 1)
            }
            VisibleTimeSetDirection::Down | VisibleTimeSetDirection::Left => {
                move_visible_time_set_cursor(runtime_shell, -1)
            }
        },
        VisibleTimeSetPhase::HourConfirm | VisibleTimeSetPhase::MinuteConfirm => {
            move_visible_time_set_cursor(runtime_shell, 1)
        }
        _ => Ok(()),
    }
}

fn move_visible_time_set_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let Some(time_set) = runtime_shell.pending_time_set.as_mut() else {
        return handle_visible_no_time_set_screen(runtime_shell, "cursor");
    };
    match time_set.phase {
        VisibleTimeSetPhase::SetHour => {
            if delta.is_negative() {
                time_set.hour = (time_set.hour + 23) % 24;
            } else {
                time_set.hour = (time_set.hour + 1) % 24;
            }
        }
        VisibleTimeSetPhase::SetMinute => {
            if delta.is_negative() {
                time_set.minute = (time_set.minute + 59) % 60;
            } else {
                time_set.minute = (time_set.minute + 1) % 60;
            }
        }
        VisibleTimeSetPhase::HourConfirm | VisibleTimeSetPhase::MinuteConfirm => {
            time_set.yes_no_index = 1 - time_set.yes_no_index.min(1);
        }
        _ => {}
    }
    let phase = time_set.phase;
    let hour = time_set.hour;
    let minute = time_set.minute;
    let yes_no_index = time_set.yes_no_index;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "time_set:cursor:{:?}:hour={}:minute={}:yes_no={}",
            phase, hour, minute, yes_no_index
        ),
    )?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn commit_visible_time_set_selection(
    runtime_shell: &mut BevyRuntimeShell,
    time_set: &mut VisibleTimeSetScreen,
) -> Result<()> {
    let target = ClockTime::new(0, time_set.hour, time_set.minute, 0);
    let rtc = required_native_rtc_sample(runtime_shell)?;
    let update = runtime_shell
        .shell
        .set_manual_clock_time(rtc.date, rtc.hour, rtc.minute, rtc.second, target)?;
    time_set.reaction_text = visible_time_set_reaction_text(time_set.hour, time_set.minute);
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "time_set:manual:{}:{:02}:{:02}:tod={:?}:checksum={:?}",
            0, time_set.hour, time_set.minute, update.time_of_day, update.state_checksum
        ),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "time set {:02}:{:02} tod={:?} game={}:{}",
        time_set.hour, time_set.minute, update.time_of_day, update.hour, update.minute
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn record_visible_time_set_menu_option(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "time_set:menu_option")?;
    runtime_shell
        .last_audio_events
        .push("played SoundEffect menu_option".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_time_set_screen(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("time_set:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("time set screen is not open".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_time_set_time_of_day(hour: u8) -> &'static str {
    match hour % 24 {
        0..=3 => "NITE",
        4..=9 => "MORN",
        10..=17 => "DAY",
        _ => "NITE",
    }
}

fn visible_time_set_twelve_hour(hour: u8) -> u8 {
    match hour % 24 {
        0 => 12,
        h if h > 12 => h - 12,
        h => h,
    }
}

fn visible_time_set_hour_display(time_set: &VisibleTimeSetScreen) -> String {
    format!(
        "{} {:>2} o'clock",
        visible_time_set_time_of_day(time_set.hour),
        visible_time_set_twelve_hour(time_set.hour)
    )
}

fn visible_time_set_minute_display(time_set: &VisibleTimeSetScreen) -> String {
    format!("{:>2} min.", time_set.minute)
}

fn visible_time_set_reaction_text(hour: u8, minute: u8) -> String {
    let suffix = if hour < 4 {
        "!\nNo wonder it's so\ndark!"
    } else if hour <= 10 {
        "!\nI overslept!"
    } else if hour < 18 {
        "!\nYikes! I over-\nslept!"
    } else {
        "!\nNo wonder it's so\ndark!"
    };
    format!(
        "{} {:>2}:{:02}\n{}",
        visible_time_set_time_of_day(hour),
        visible_time_set_twelve_hour(hour),
        minute,
        suffix
    )
}

fn visible_time_set_yes_no_entries(time_set: &VisibleTimeSetScreen) -> Vec<String> {
    ["YES", "NO"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            if index == time_set.yes_no_index.min(1) {
                format!("> {label}")
            } else {
                format!("  {label}")
            }
        })
        .collect()
}

const VISIBLE_OAK_INTRO_TEXT_SPEED_FRAMES: u8 = 2;
const VISIBLE_OAK_INTRO_SCENES: [(&str, &str, &[&str]); 4] = [
    (
        "oak_intro_1",
        "OAK",
        &[
            "Hello! Sorry to\nkeep you waiting!",
            "Welcome to the\nworld of #MON!",
            "My name is OAK.",
            "People call me the\n#MON PROF.",
        ],
    ),
    (
        "wooper_showcase",
        "WOOPER",
        &[
            "This world is in-\nhabited by crea-\ntures that we call",
            "#MON.",
            "People and #MON\nlive together by",
            "supporting each\nother.",
            "Some people play\nwith #MON, some\nbattle with them.",
        ],
    ),
    (
        "oak_intro_2",
        "OAK",
        &[
            "But we don't know\neverything about\n#MON yet.",
            "There are still\nmany mysteries to\nsolve.",
            "That's why I study\n#MON every day.",
        ],
    ),
    (
        "player_picture",
        "PLAYER",
        &["Now, what did you\nsay your name was?"],
    ),
];
const VISIBLE_OAK_FINAL_TEXT: [&str; 6] = [
    "<PLAYER>, are you\nready?",
    "Your very own\n#MON story is\nabout to unfold.",
    "You'll face fun\ntimes and tough\nchallenges.",
    "A world of dreams\nand adventures",
    "with #MON\nawaits! Let's go!",
    "I'll be seeing you\nlater!",
];
const VISIBLE_OAK_INTRO_FADE_FRAME_DELAY: u16 = 8;
const VISIBLE_OAK_WIPE_STEP_PIXELS: u16 = 8;
const VISIBLE_OAK_WIPE_END_X: u16 = 160;

fn empty_visible_oak_intro_sequence(mode: VisibleOakIntroMode) -> VisibleOakIntroSequence {
    VisibleOakIntroSequence {
        mode,
        scene_index: 0,
        scene_state: String::new(),
        scene_phase: VisibleOakIntroPhase::Complete,
        current_sprite: None,
        wooper_cry_queued: false,
        scene_fade_out_steps: 0,
        fade_active: false,
        fade_direction: VisibleOakFadeDirection::In,
        fade_total_frames: 1,
        fade_elapsed: 0,
        fade_alpha: 0,
        wipe_active: false,
        wipe_window_x: 0,
        text_queue: Vec::new(),
        current_text: String::new(),
        visible_chars: 0,
        text_timer: 0,
        waiting_for_input: false,
        blink_timer: 0,
        finished: false,
    }
}

fn open_visible_oak_intro_sequence(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    reset_visible_navigation_state(runtime_shell);
    let mut oak_intro = empty_visible_oak_intro_sequence(VisibleOakIntroMode::Intro);
    start_visible_oak_intro_scene(&mut oak_intro);
    runtime_shell.pending_oak_intro = Some(oak_intro);
    record_visible_runtime_action(runtime_shell, "oak_intro:open")?;
    let oak_music = "MUSIC_ROUTE_30";
    if runtime_shell.active_music.as_deref() != Some(oak_music) {
        let playback = runtime_shell
            .shell
            .runtime()
            .audio()
            .require_playback_entry(AudioKind::Music, oak_music)?;
        enqueue_bevy_audio_command(
            &mut runtime_shell.pending_audio,
            BevyAudioCommand {
                audio_id: oak_music.to_string(),
                kind: ModpackAudioKind::Music,
                mode: playback.mode,
                looped: matches!(
                    playback.loop_policy,
                    crate::assets::ModpackAudioLoopPolicy::Loop
                ),
            },
        );
        runtime_shell.pending_music_stop = true;
        runtime_shell.active_music = Some(oak_music.to_string());
        runtime_shell.faded_music = None;
    }
    runtime_shell
        .last_audio_events
        .push("queued Oak intro music MUSIC_ROUTE_30".to_string());
    set_shell_action_status(runtime_shell, "OAK INTRO");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_oak_final_sequence(
    runtime_shell: &mut BevyRuntimeShell,
    player_name: &str,
) -> Result<()> {
    if player_name.trim() != player_name {
        anyhow::bail!("Oak finale requires an exact player name");
    }
    reset_visible_navigation_state(runtime_shell);
    let player = player_name.to_uppercase();
    let mut oak_intro = empty_visible_oak_intro_sequence(VisibleOakIntroMode::Final);
    oak_intro.scene_index = VISIBLE_OAK_INTRO_SCENES.len();
    oak_intro.scene_state = "oak_final".to_string();
    oak_intro.scene_phase = VisibleOakIntroPhase::Text;
    // ASM returns from NamePlayer directly into OakText7 without clearing the
    // tilemap, so the selected player/trainer portrait remains on screen.
    oak_intro.current_sprite = Some("PLAYER".to_string());
    oak_intro.text_queue = VISIBLE_OAK_FINAL_TEXT
        .iter()
        .map(|page| page.replace("<PLAYER>", &player))
        .collect();
    advance_visible_oak_intro_text_queue(&mut oak_intro);
    runtime_shell.pending_oak_intro = Some(oak_intro);
    record_visible_runtime_action(runtime_shell, "oak_intro:final:open")?;
    set_shell_action_status(runtime_shell, "OAK FINALE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn start_visible_oak_intro_scene(oak_intro: &mut VisibleOakIntroSequence) {
    let (state, sprite, pages) = VISIBLE_OAK_INTRO_SCENES[oak_intro
        .scene_index
        .min(VISIBLE_OAK_INTRO_SCENES.len() - 1)];
    oak_intro.scene_state = state.to_string();
    oak_intro.current_sprite = Some(sprite.to_string());
    oak_intro.text_queue = if state == "wooper_showcase" {
        Vec::new()
    } else {
        pages.iter().map(|page| (*page).to_string()).collect()
    };
    oak_intro.current_text.clear();
    oak_intro.visible_chars = 0;
    oak_intro.text_timer = 0;
    oak_intro.waiting_for_input = false;
    oak_intro.blink_timer = 0;
    oak_intro.finished = false;
    oak_intro.wooper_cry_queued = false;
    oak_intro.scene_fade_out_steps = match state {
        "player_picture" => 0,
        _ => 3,
    };
    oak_intro.fade_active = false;
    oak_intro.fade_alpha = 0;
    oak_intro.fade_elapsed = 0;
    oak_intro.fade_total_frames = 1;
    oak_intro.wipe_active = false;
    oak_intro.wipe_window_x = 0;
    match state {
        "oak_intro_1" => {
            oak_intro.scene_phase = VisibleOakIntroPhase::FadeIn;
            start_visible_oak_intro_fade(oak_intro, VisibleOakFadeDirection::In, 4);
        }
        "wooper_showcase" => {
            oak_intro.scene_phase = VisibleOakIntroPhase::WipeIn;
            start_visible_oak_intro_wipe(oak_intro);
        }
        "oak_intro_2" | "player_picture" => {
            oak_intro.scene_phase = VisibleOakIntroPhase::FadeIn;
            start_visible_oak_intro_fade(oak_intro, VisibleOakFadeDirection::In, 3);
        }
        _ => {
            oak_intro.scene_phase = VisibleOakIntroPhase::Text;
            advance_visible_oak_intro_text_queue(oak_intro);
        }
    }
}

fn tick_visible_oak_intro(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let should_queue_wooper_cry =
        runtime_shell
            .pending_oak_intro
            .as_ref()
            .is_some_and(|oak_intro| {
                oak_intro.mode == VisibleOakIntroMode::Intro
                    && oak_intro.scene_state == "wooper_showcase"
                    && oak_intro.scene_phase == VisibleOakIntroPhase::Cry
                    && !oak_intro.wooper_cry_queued
            });
    if should_queue_wooper_cry {
        queue_visible_pokemon_cry(runtime_shell, "WOOPER", "oak_intro")?;
        if let Some(oak_intro) = runtime_shell.pending_oak_intro.as_mut() {
            oak_intro.wooper_cry_queued = true;
        }
    }
    let Some(oak_intro) = runtime_shell.pending_oak_intro.as_mut() else {
        return Ok(());
    };
    oak_intro.blink_timer = (oak_intro.blink_timer + 1) % 60;
    drive_visible_oak_intro_phase(oak_intro);
    Ok(())
}

fn drive_visible_oak_intro_phase(oak_intro: &mut VisibleOakIntroSequence) {
    match oak_intro.scene_phase {
        VisibleOakIntroPhase::FadeIn => {
            update_visible_oak_intro_fade(oak_intro);
            if !oak_intro.fade_active && oak_intro.fade_alpha == 0 {
                oak_intro.scene_phase = VisibleOakIntroPhase::Text;
                advance_visible_oak_intro_text_queue(oak_intro);
            }
        }
        VisibleOakIntroPhase::WipeIn => {
            if advance_visible_oak_intro_wipe(oak_intro) {
                oak_intro.scene_phase = VisibleOakIntroPhase::TextOne;
                queue_visible_oak_intro_text(oak_intro, &VISIBLE_OAK_INTRO_SCENES[1].2[..2]);
                advance_visible_oak_intro_text_queue(oak_intro);
            }
        }
        VisibleOakIntroPhase::Text
        | VisibleOakIntroPhase::TextOne
        | VisibleOakIntroPhase::TextTwo => {
            drive_visible_oak_intro_text(oak_intro);
        }
        VisibleOakIntroPhase::Cry => {
            if oak_intro.wooper_cry_queued {
                oak_intro.scene_phase = VisibleOakIntroPhase::TextTwo;
                oak_intro.text_queue = VISIBLE_OAK_INTRO_SCENES[1].2[2..]
                    .iter()
                    .map(|page| (*page).to_string())
                    .collect();
                // ASM: OakText2 runs PlayMonCry/WaitSFX and then OakText3's
                // text_promptbutton without clearing the displayed "#MON."
                // page. Keep it visible until A or B acknowledges the prompt.
                oak_intro.waiting_for_input = true;
                oak_intro.blink_timer = 0;
            }
        }
        VisibleOakIntroPhase::FadeOut => {
            update_visible_oak_intro_fade(oak_intro);
            if !oak_intro.fade_active {
                oak_intro.current_sprite = None;
                oak_intro.scene_phase = VisibleOakIntroPhase::Complete;
                oak_intro.finished = true;
            }
        }
        VisibleOakIntroPhase::Complete => {
            oak_intro.finished = true;
        }
    }
}

fn drive_visible_oak_intro_text(oak_intro: &mut VisibleOakIntroSequence) {
    if oak_intro.finished || oak_intro.waiting_for_input {
        return;
    }
    if oak_intro.current_text.is_empty() {
        advance_visible_oak_intro_text_queue(oak_intro);
        return;
    }
    if oak_intro.visible_chars >= oak_intro.current_text.chars().count() {
        finish_visible_oak_intro_page(oak_intro);
        return;
    }
    oak_intro.text_timer = oak_intro.text_timer.saturating_add(1);
    if oak_intro.text_timer >= VISIBLE_OAK_INTRO_TEXT_SPEED_FRAMES {
        oak_intro.text_timer = 0;
        oak_intro.visible_chars = oak_intro.visible_chars.saturating_add(1);
        if oak_intro.visible_chars >= oak_intro.current_text.chars().count() {
            finish_visible_oak_intro_page(oak_intro);
        }
    }
}

fn finish_visible_oak_intro_page(oak_intro: &mut VisibleOakIntroSequence) {
    if oak_intro.scene_phase == VisibleOakIntroPhase::TextOne && oak_intro.text_queue.is_empty() {
        // OakText2 terminates directly into the Wooper cry; its only prompt is
        // OakText3 after WaitSFX, so there is no input wait before the cry.
        oak_intro.waiting_for_input = false;
        oak_intro.scene_phase = VisibleOakIntroPhase::Cry;
    } else {
        oak_intro.waiting_for_input = true;
    }
}

fn advance_visible_oak_intro_text_queue(oak_intro: &mut VisibleOakIntroSequence) {
    if oak_intro.current_text.is_empty() {
        if let Some(next) = oak_intro.text_queue.first().cloned() {
            oak_intro.text_queue.remove(0);
            oak_intro.current_text = next;
            oak_intro.visible_chars = 0;
            oak_intro.text_timer = 0;
            oak_intro.waiting_for_input = false;
            oak_intro.blink_timer = 0;
        } else {
            finish_visible_oak_intro_text_group(oak_intro);
        }
    }
}

fn finish_visible_oak_intro_text_group(oak_intro: &mut VisibleOakIntroSequence) {
    match oak_intro.scene_phase {
        VisibleOakIntroPhase::TextOne => {
            oak_intro.scene_phase = VisibleOakIntroPhase::Cry;
        }
        VisibleOakIntroPhase::Text | VisibleOakIntroPhase::TextTwo => {
            if matches!(oak_intro.mode, VisibleOakIntroMode::Final) {
                oak_intro.scene_phase = VisibleOakIntroPhase::Complete;
                oak_intro.finished = true;
            } else if oak_intro.scene_fade_out_steps > 0 {
                oak_intro.scene_phase = VisibleOakIntroPhase::FadeOut;
                start_visible_oak_intro_fade(
                    oak_intro,
                    VisibleOakFadeDirection::Out,
                    oak_intro.scene_fade_out_steps,
                );
            } else {
                oak_intro.scene_phase = VisibleOakIntroPhase::Complete;
                oak_intro.finished = true;
            }
        }
        _ => {}
    }
}

fn queue_visible_oak_intro_text(oak_intro: &mut VisibleOakIntroSequence, pages: &[&str]) {
    oak_intro.text_queue = pages.iter().map(|page| (*page).to_string()).collect();
    oak_intro.current_text.clear();
    oak_intro.visible_chars = 0;
    oak_intro.text_timer = 0;
    oak_intro.waiting_for_input = false;
    oak_intro.blink_timer = 0;
}

fn start_visible_oak_intro_fade(
    oak_intro: &mut VisibleOakIntroSequence,
    direction: VisibleOakFadeDirection,
    steps: u8,
) {
    oak_intro.fade_direction = direction;
    oak_intro.fade_total_frames = u16::from(steps.max(1)) * VISIBLE_OAK_INTRO_FADE_FRAME_DELAY;
    oak_intro.fade_elapsed = 0;
    oak_intro.fade_active = true;
    oak_intro.fade_alpha = match direction {
        VisibleOakFadeDirection::In => 255,
        VisibleOakFadeDirection::Out => 0,
    };
}

fn update_visible_oak_intro_fade(oak_intro: &mut VisibleOakIntroSequence) {
    if !oak_intro.fade_active {
        return;
    }
    oak_intro.fade_elapsed = oak_intro.fade_elapsed.saturating_add(1);
    let total = oak_intro.fade_total_frames.max(1);
    let elapsed = oak_intro.fade_elapsed.min(total);
    let raw_alpha = match oak_intro.fade_direction {
        VisibleOakFadeDirection::In => {
            let remaining = total.saturating_sub(elapsed);
            ((255_u32 * u32::from(remaining)) / u32::from(total)) as u8
        }
        VisibleOakFadeDirection::Out => ((255_u32 * u32::from(elapsed)) / u32::from(total)) as u8,
    };
    let step = raw_alpha / 8;
    oak_intro.fade_alpha = ((u16::from(step) * 255) / 31) as u8;
    if oak_intro.fade_elapsed >= oak_intro.fade_total_frames {
        oak_intro.fade_active = false;
        oak_intro.fade_alpha = match oak_intro.fade_direction {
            VisibleOakFadeDirection::In => 0,
            VisibleOakFadeDirection::Out => 255,
        };
    }
}

fn start_visible_oak_intro_wipe(oak_intro: &mut VisibleOakIntroSequence) {
    oak_intro.wipe_active = true;
    oak_intro.wipe_window_x = 0;
}

fn advance_visible_oak_intro_wipe(oak_intro: &mut VisibleOakIntroSequence) -> bool {
    if !oak_intro.wipe_active {
        return true;
    }
    oak_intro.wipe_window_x = oak_intro
        .wipe_window_x
        .saturating_add(VISIBLE_OAK_WIPE_STEP_PIXELS);
    if oak_intro.wipe_window_x > VISIBLE_OAK_WIPE_END_X {
        oak_intro.wipe_active = false;
        return true;
    }
    false
}

fn visible_oak_intro_dialog_complete(oak_intro: &VisibleOakIntroSequence) -> bool {
    oak_intro.visible_chars >= oak_intro.current_text.chars().count()
}

fn visible_oak_intro_visible_dialog(oak_intro: &VisibleOakIntroSequence) -> String {
    oak_intro
        .current_text
        .chars()
        .take(
            oak_intro
                .visible_chars
                .min(oak_intro.current_text.chars().count()),
        )
        .collect()
}

fn press_visible_oak_intro_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mut oak_intro) = runtime_shell.pending_oak_intro.take() else {
        return handle_visible_no_oak_intro(runtime_shell, "a");
    };
    if !oak_intro.current_text.is_empty() && !visible_oak_intro_dialog_complete(&oak_intro) {
        oak_intro.visible_chars = oak_intro.current_text.chars().count();
        finish_visible_oak_intro_page(&mut oak_intro);
        runtime_shell.pending_oak_intro = Some(oak_intro);
        return Ok(());
    }
    if oak_intro.waiting_for_input {
        oak_intro.waiting_for_input = false;
        oak_intro.current_text.clear();
        advance_visible_oak_intro_text_queue(&mut oak_intro);
    }
    if oak_intro.finished {
        return complete_visible_oak_intro(runtime_shell, oak_intro);
    }
    runtime_shell.pending_oak_intro = Some(oak_intro);
    Ok(())
}

fn press_visible_oak_intro_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mut oak_intro) = runtime_shell.pending_oak_intro.take() else {
        return handle_visible_no_oak_intro(runtime_shell, "b");
    };
    // PromptButton accepts either A or B in the ASM. Only retain the shell's
    // B-to-skip convenience while no Oak text page is active.
    if !oak_intro.current_text.is_empty() {
        runtime_shell.pending_oak_intro = Some(oak_intro);
        return press_visible_oak_intro_a_button(runtime_shell);
    }
    if matches!(oak_intro.mode, VisibleOakIntroMode::Intro) {
        oak_intro.finished = true;
        return complete_visible_oak_intro(runtime_shell, oak_intro);
    }
    runtime_shell.pending_oak_intro = Some(oak_intro);
    press_visible_oak_intro_a_button(runtime_shell)
}

fn complete_visible_oak_intro(
    runtime_shell: &mut BevyRuntimeShell,
    mut oak_intro: VisibleOakIntroSequence,
) -> Result<()> {
    match oak_intro.mode {
        VisibleOakIntroMode::Intro => {
            oak_intro.scene_index += 1;
            if oak_intro.scene_index < VISIBLE_OAK_INTRO_SCENES.len() {
                start_visible_oak_intro_scene(&mut oak_intro);
                runtime_shell.pending_oak_intro = Some(oak_intro);
                return Ok(());
            }
            record_visible_runtime_action(runtime_shell, "oak_intro:complete")?;
            runtime_shell
                .last_audio_events
                .push("oak intro complete".to_string());
            trim_event_log(&mut runtime_shell.last_audio_events);
            open_visible_name_choice(runtime_shell)
        }
        VisibleOakIntroMode::Final => {
            record_visible_runtime_action(runtime_shell, "oak_intro:final:complete")?;
            runtime_shell
                .last_audio_events
                .push("oak final complete".to_string());
            trim_event_log(&mut runtime_shell.last_audio_events);
            settle_visible_overworld_arrival(runtime_shell, "new_game")
        }
    }
}

fn handle_visible_no_oak_intro(runtime_shell: &mut BevyRuntimeShell, action: &str) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("oak_intro:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("oak intro is not open".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn tick_visible_gender_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(gender) = runtime_shell.pending_gender_selection.as_mut() else {
        return Ok(());
    };
    if gender.fade_counter < VISIBLE_GENDER_FADE_IN_FRAMES {
        gender.fade_counter += 1;
    }
    if !gender.confirmed {
        return Ok(());
    }
    if gender.confirm_countdown > 0 {
        gender.confirm_countdown -= 1;
        return Ok(());
    }
    let selected_gender = visible_gender_selected_gender(gender);
    let gender_set = runtime_shell
        .shell
        .set_player_gender(visible_player_gender_value(selected_gender))?;
    runtime_shell.selected_player_gender = Some(selected_gender);
    runtime_shell.pending_gender_selection = None;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "gender:confirmed:{selected_gender:?}:{}->{}",
            gender_set.player_gender_before, gender_set.player_gender_after
        ),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "gender confirmed {}",
        visible_gender_label(selected_gender)
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    open_visible_time_set_screen(runtime_shell, VisibleTimeSetNext::OakIntro)
}

fn move_visible_gender_selection(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let Some(gender) = runtime_shell.pending_gender_selection.as_mut() else {
        return handle_visible_no_gender_selection(runtime_shell, "cursor");
    };
    if gender.confirmed {
        record_visible_runtime_action(runtime_shell, "gender:cursor:confirmed")?;
        runtime_shell
            .last_audio_events
            .push("gender cursor ignored after confirm".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let item_count = gender.definition.items.len();
    let current = gender.selected_index;
    anyhow::ensure!(
        current < item_count,
        "gender selection cursor {current} is out of range"
    );
    let next = if delta.is_negative() {
        current
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(item_count - 1)
    } else {
        (current + delta as usize) % item_count
    };
    gender.selected_index = next;
    record_visible_runtime_action(runtime_shell, format!("gender:cursor:{current}->{next}"))?;
    runtime_shell
        .last_audio_events
        .push(format!("gender cursor {}->{}", current + 1, next + 1));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_gender_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(gender) = runtime_shell.pending_gender_selection.as_mut() else {
        return handle_visible_no_gender_selection(runtime_shell, "confirm");
    };
    if gender.confirmed {
        return Ok(());
    }
    gender.confirmed = true;
    gender.confirm_countdown = gender.definition.confirm_delay_frames;
    let selected_gender = visible_gender_selected_gender(gender);
    record_visible_runtime_action(runtime_shell, format!("gender:confirm:{selected_gender:?}"))?;
    runtime_shell
        .last_audio_events
        .push("played SoundEffect menu_option".to_string());
    set_shell_action_status(
        runtime_shell,
        format!("GENDER {}", visible_gender_label(selected_gender)),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_gender_selected_gender(gender: &VisibleGenderSelection) -> VisiblePlayerGender {
    match gender.definition.values.get(gender.selected_index).copied() {
        Some(PLAYER_GENDER_MALE) => VisiblePlayerGender::Boy,
        Some(PLAYER_GENDER_FEMALE) => VisiblePlayerGender::Girl,
        _ => unreachable!("validated gender definition and cursor must remain in range"),
    }
}

fn visible_gender_label(gender: VisiblePlayerGender) -> &'static str {
    match gender {
        VisiblePlayerGender::Boy => "BOY",
        VisiblePlayerGender::Girl => "GIRL",
    }
}

fn visible_player_gender_value(gender: VisiblePlayerGender) -> u8 {
    match gender {
        VisiblePlayerGender::Boy => PLAYER_GENDER_MALE,
        VisiblePlayerGender::Girl => PLAYER_GENDER_FEMALE,
    }
}

fn visible_gender_entries(gender: &VisibleGenderSelection) -> Vec<String> {
    gender
        .definition
        .items
        .iter()
        .enumerate()
        .map(|(index, label)| {
            if index == gender.selected_index {
                format!("> {}", label.to_uppercase())
            } else {
                format!("  {}", label.to_uppercase())
            }
        })
        .collect()
}

fn handle_visible_no_gender_selection(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("gender:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("gender selection is not open".to_string());
    set_shell_action_status(runtime_shell, "NO GENDER");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn advance_visible_title_to_press_start(runtime_shell: &mut BevyRuntimeShell) {
    let maximum_frames = runtime_shell
        .title_menu
        .as_ref()
        .map(|title| usize::from(title.entrance_start_scx / title.entrance_scroll_step) + 2)
        .unwrap_or(0);
    for _ in 0..=maximum_frames {
        if runtime_shell
            .title_menu
            .as_ref()
            .is_some_and(|title| matches!(title.phase, VisibleTitlePhase::PressStart))
        {
            return;
        }
        tick_visible_title_screen_state(runtime_shell);
    }
}

fn advance_visible_title_to_main_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    advance_visible_title_to_press_start(runtime_shell);
    open_visible_title_main_menu(runtime_shell)
}

fn title_continue_save_path<'a>(
    runtime_shell: &BevyRuntimeShell,
    title: &'a TitleMenu,
) -> Option<&'a PathBuf> {
    let path = title.save_path.as_ref()?;
    // An existing primary remains a candidate so Continue can report its
    // validation error. Only a missing primary invokes canonical .bak recovery.
    if crate::runtime_asset_exists(&path)
        || runtime_shell
            .shell
            .runtime()
            .load_save_summary(path)
            .is_ok()
    {
        return Some(path);
    }
    None
}

fn visible_title_continue_entries(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
) -> Vec<String> {
    let Some(path) = title_continue_save_path(runtime_shell, title) else {
        return vec!["CONTINUE NONE".to_string()];
    };
    let mut entries = vec![compact_scene_label(
        &format!(
            "CONTINUE {}",
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string())
        ),
        30,
    )];
    match runtime_shell.shell.runtime().load_save_summary(path) {
        Ok(_) => entries.extend(visible_save_slot_preview_entries_for_path(
            runtime_shell,
            path,
        )),
        Err(error) => entries.push(compact_scene_label(&format!("INVALID SAVE {error}"), 30)),
    }
    entries.into_iter().take(4).collect()
}

fn visible_title_menu_entries(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
) -> Result<Vec<String>> {
    if !visible_title_main_menu_ready(title) {
        return Ok(vec![match title.phase {
            VisibleTitlePhase::Entrance | VisibleTitlePhase::Timer => "TITLE SCREEN".to_string(),
            VisibleTitlePhase::PressStart => "PRESS START".to_string(),
            VisibleTitlePhase::MainMenu => unreachable!(),
            VisibleTitlePhase::FadeOut => "FADE OUT".to_string(),
        }]);
    }
    let options = visible_title_menu_options(runtime_shell, title);
    let selected = title.cursor.option_index.min(options.len() - 1);
    Ok(options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let marker = if index == selected { ">" } else { " " };
            format!("{marker}{}", option.label)
        })
        .collect())
}

fn move_visible_title_menu_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let action = format!("title:cursor:{delta}");
    move_visible_title_menu_cursor_with_action(runtime_shell, delta, action)
}

fn move_visible_title_menu_cursor_with_action(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
    action: String,
) -> Result<()> {
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "cursor");
    };
    if !visible_title_main_menu_ready(title) {
        let phase = title.phase;
        record_visible_runtime_action(runtime_shell, format!("{action}:ignored"))?;
        runtime_shell
            .last_audio_events
            .push(format!("title cursor ignored phase={phase:?}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let len = visible_title_menu_options(runtime_shell, title).len();
    let current = title.cursor.option_index.min(len - 1);
    let next = if delta.is_negative() {
        current.checked_sub(delta.unsigned_abs()).unwrap_or(len - 1)
    } else {
        (current + delta as usize) % len
    };
    if let Some(title) = runtime_shell.title_menu.as_mut() {
        title.cursor.option_index = next;
    }
    record_visible_runtime_action(runtime_shell, format!("{action}:{current}->{next}"))?;
    runtime_shell
        .last_audio_events
        .push(format!("title cursor {}->{}", current + 1, next + 1));
    set_shell_action_status(runtime_shell, format!("TITLE OPTION {}", next + 1));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn press_visible_title_direction_button(
    runtime_shell: &mut BevyRuntimeShell,
    input: GameButton,
    delta: isize,
) -> Result<()> {
    if runtime_shell.visible_continue_screen.is_some() {
        return Ok(());
    }
    let action = format!("input:title:{input:?}:{delta}");
    move_visible_title_menu_cursor_with_action(runtime_shell, delta, action)
}

fn selected_visible_title_menu_option(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
) -> Result<RuntimeTitleMainMenuItem> {
    let options = visible_title_menu_options(runtime_shell, title);
    let selected = title.cursor.option_index.min(options.len() - 1);
    Ok(options[selected].clone())
}

fn visible_title_menu_selection_id(option: &RuntimeTitleMainMenuItem) -> Result<&'static str> {
    match option.dispatch_target.as_str() {
        "MainMenu_Continue" => Ok("CONTINUE"),
        "MainMenu_NewGame" => Ok("NEW_GAME"),
        "MainMenu_Option" => Ok("OPTIONS"),
        "MainMenu_MysteryGift" => Ok("MYSTERY_GIFT"),
        target => anyhow::bail!("unsupported source main-menu dispatch target {target}"),
    }
}

fn press_visible_title_confirm_button(
    runtime_shell: &mut BevyRuntimeShell,
    input: GameButton,
) -> Result<()> {
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "confirm");
    };
    if !visible_title_main_menu_ready(title) {
        let mask = match input {
            GameButton::A => 0x01,
            GameButton::Start => 0x08,
            _ => {
                return record_visible_runtime_action(
                    runtime_shell,
                    "input:title:pre_menu:ignored",
                );
            }
        };
        let title = runtime_shell
            .title_menu
            .as_mut()
            .context("title menu disappeared before source input sampling")?;
        title.joypad_mask = mask;
        runtime_shell.last_error = None;
        tick_visible_title_screen_state(runtime_shell);
        if let Some(title) = runtime_shell.title_menu.as_mut() {
            title.joypad_mask = 0;
        }
        if let Some(error) = runtime_shell.last_error.clone() {
            anyhow::bail!(error);
        }
        return Ok(());
    }
    if input != GameButton::A {
        return record_visible_runtime_action(runtime_shell, "input:title:main_menu:ignored");
    }
    let action = format!("input:title:{input:?}:confirm");
    record_visible_runtime_action(runtime_shell, action.clone())?;
    runtime_shell.last_audio_events.push(action);
    trim_event_log(&mut runtime_shell.last_audio_events);
    select_visible_title_menu_option(runtime_shell)
}

fn press_visible_title_cancel_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_continue_screen.take().is_some() {
        record_visible_runtime_action(runtime_shell, "input:title:B:continue_back")?;
        set_shell_action_status(runtime_shell, "TITLE");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell
        .title_menu
        .as_ref()
        .is_some_and(visible_title_main_menu_ready)
    {
        stop_visible_silent_music(
            runtime_shell,
            "MUSIC_NONE",
            "audio:music:main_menu_cancel:stop",
        )?;
        if let Some(title) = runtime_shell.title_menu.as_mut() {
            reset_visible_title_program(title);
            title.cursor.option_index = 0;
        }
        queue_visible_shell_sound_effect(runtime_shell, "SFX_TITLE_SCREEN_ENTRANCE")?;
        record_visible_runtime_action(runtime_shell, "input:title:B:restart_title")?;
        set_shell_action_status(runtime_shell, "TITLE");
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "input:title:B:cancel")?;
    runtime_shell
        .last_audio_events
        .push("input:title:B:cancel".to_string());
    set_shell_action_status(runtime_shell, "TITLE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_delete_save_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "delete_save");
    };
    if !matches!(title.phase, VisibleTitlePhase::PressStart) {
        record_visible_runtime_action(
            runtime_shell,
            format!("title:delete_save:ignored:{:?}", title.phase),
        )?;
        return Ok(());
    }
    runtime_shell.pending_delete_save = Some(VisibleDeleteSaveScreen { selected_index: 1 });
    record_visible_runtime_action(runtime_shell, "title:delete_save:open")?;
    set_shell_action_status(runtime_shell, "DELETE SAVE");
    runtime_shell
        .last_audio_events
        .push("title opened delete save prompt".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_delete_save_screen(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &'static str,
) -> Result<()> {
    runtime_shell.pending_delete_save = None;
    if let Some(title) = runtime_shell.title_menu.as_mut() {
        reset_visible_title_program(title);
        title.cursor.option_index = 0;
    }
    record_visible_runtime_action(runtime_shell, format!("delete_save:{reason}:close"))?;
    set_shell_action_status(runtime_shell, "TITLE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_delete_save_cursor(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(delete_save) = runtime_shell.pending_delete_save.as_mut() else {
        return handle_visible_no_active_title_menu(runtime_shell, "delete_save_cursor");
    };
    delete_save.selected_index = 1 - delete_save.selected_index.min(1);
    let selected = delete_save.selected_index;
    record_visible_runtime_action(runtime_shell, format!("delete_save:cursor:{selected}"))?;
    set_shell_action_status(
        runtime_shell,
        if selected == 0 {
            "DELETE YES"
        } else {
            "DELETE NO"
        },
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_delete_save_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(delete_save) = runtime_shell.pending_delete_save.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "delete_save_confirm");
    };
    if delete_save.selected_index.min(1) != 0 {
        return close_visible_delete_save_screen(runtime_shell, "cancel");
    }
    let deleted = match runtime_shell
        .title_menu
        .as_ref()
        .and_then(|title| title.save_path.clone())
    {
        Some(path) => crystal_core::save::erase_save_game(&path)
            .with_context(|| format!("delete save {}", path.display()))?,
        None => false,
    };
    runtime_shell
        .last_audio_events
        .push(format!("delete save confirmed deleted={deleted}"));
    close_visible_delete_save_screen(runtime_shell, "confirm")
}

fn open_visible_clock_reset_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "clock_reset");
    };
    if !matches!(title.phase, VisibleTitlePhase::PressStart) {
        record_visible_runtime_action(
            runtime_shell,
            format!("title:clock_reset:ignored:{:?}", title.phase),
        )?;
        return Ok(());
    }
    let time = runtime_shell.shell.snapshot()?.progression.time;
    runtime_shell.pending_clock_reset = Some(VisibleClockResetScreen {
        phase: VisibleClockResetPhase::Confirm,
        confirm_selection: 1,
        day: time.day_of_week % 7,
        hour: time.registers.hours,
        minute: time.registers.minutes,
    });
    if let Some(title) = runtime_shell.title_menu.as_mut() {
        title.clock_reset_trigger = false;
    }
    record_visible_runtime_action(runtime_shell, "title:clock_reset:open")?;
    set_shell_action_status(runtime_shell, "RESET CLOCK");
    runtime_shell
        .last_audio_events
        .push("title opened clock reset prompt".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_clock_reset_screen(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &'static str,
) -> Result<()> {
    runtime_shell.pending_clock_reset = None;
    if let Some(title) = runtime_shell.title_menu.as_mut() {
        reset_visible_title_program(title);
        title.cursor.option_index = 0;
    }
    record_visible_runtime_action(runtime_shell, format!("clock_reset:{reason}:close"))?;
    set_shell_action_status(runtime_shell, "TITLE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_clock_reset_cursor(runtime_shell: &mut BevyRuntimeShell, delta: i8) -> Result<()> {
    let Some(clock) = runtime_shell.pending_clock_reset.as_mut() else {
        return handle_visible_no_active_title_menu(runtime_shell, "clock_reset_cursor");
    };
    match clock.phase {
        VisibleClockResetPhase::Confirm => {
            clock.confirm_selection = 1 - clock.confirm_selection.min(1);
        }
        VisibleClockResetPhase::SetDay => {
            clock.day = wrap_visible_clock_value(clock.day, delta, 7);
        }
        VisibleClockResetPhase::SetHour => {
            clock.hour = wrap_visible_clock_value(clock.hour, delta, 24);
        }
        VisibleClockResetPhase::SetMinute => {
            clock.minute = wrap_visible_clock_value(clock.minute, delta, 60);
        }
    }
    let phase = clock.phase;
    let day = clock.day;
    let hour = clock.hour;
    let minute = clock.minute;
    record_visible_runtime_action(
        runtime_shell,
        format!("clock_reset:cursor:{phase:?}:{day}:{hour}:{minute}"),
    )?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn wrap_visible_clock_value(value: u8, delta: i8, modulo: u8) -> u8 {
    let modulo = i16::from(modulo.max(1));
    (i16::from(value) + i16::from(delta)).rem_euclid(modulo) as u8
}

fn confirm_visible_clock_reset_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(clock) = runtime_shell.pending_clock_reset.clone() else {
        return handle_visible_no_active_title_menu(runtime_shell, "clock_reset_confirm");
    };
    match clock.phase {
        VisibleClockResetPhase::Confirm => {
            if clock.confirm_selection.min(1) == 0 {
                let Some(active) = runtime_shell.pending_clock_reset.as_mut() else {
                    return Ok(());
                };
                active.phase = VisibleClockResetPhase::SetDay;
                record_visible_runtime_action(runtime_shell, "clock_reset:confirm:yes")?;
                trim_event_log(&mut runtime_shell.last_audio_events);
                Ok(())
            } else {
                close_visible_clock_reset_screen(runtime_shell, "cancel")
            }
        }
        VisibleClockResetPhase::SetDay => {
            let Some(active) = runtime_shell.pending_clock_reset.as_mut() else {
                return Ok(());
            };
            active.phase = VisibleClockResetPhase::SetHour;
            record_visible_runtime_action(runtime_shell, "clock_reset:day:confirm")?;
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
        VisibleClockResetPhase::SetHour => {
            let Some(active) = runtime_shell.pending_clock_reset.as_mut() else {
                return Ok(());
            };
            active.phase = VisibleClockResetPhase::SetMinute;
            record_visible_runtime_action(runtime_shell, "clock_reset:hour:confirm")?;
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
        VisibleClockResetPhase::SetMinute => {
            let rtc = required_native_rtc_sample(runtime_shell)?;
            let update = runtime_shell.shell.set_manual_clock_time(
                rtc.date,
                rtc.hour,
                rtc.minute,
                rtc.second,
                ClockTime::new(clock.day, clock.hour, clock.minute, 0),
            )?;
            runtime_shell.last_audio_events.push(format!(
                "clock reset day={} game={}:{} checksum={:?}",
                update.day_of_week, update.hour, update.minute, update.state_checksum
            ));
            close_visible_clock_reset_screen(runtime_shell, "confirm")
        }
    }
}

const VISIBLE_CREDITS_SKIP_THRESHOLD: u16 = 0x0d;
const VISIBLE_CREDITS_ALLOW_SKIP_BIT: u8 = 6;
const VISIBLE_CREDITS_EXIT_BIT: u8 = 7;
const VISIBLE_GENDER_FADE_IN_FRAMES: u8 = 8;

fn visible_credits_initial_jumptable_index(allow_skip: bool) -> u8 {
    if allow_skip {
        1 << VISIBLE_CREDITS_ALLOW_SKIP_BIT
    } else {
        0
    }
}

fn visible_credits_step_index(credits: &VisibleCreditsScreen) -> u8 {
    credits.jumptable_index & 0x0f
}

fn open_visible_credits_screen(
    runtime_shell: &mut BevyRuntimeShell,
    allow_skip: bool,
) -> Result<()> {
    reset_visible_navigation_state(runtime_shell);
    runtime_shell.title_menu = None;
    runtime_shell.special_boundary = None;
    runtime_shell.special_boundary_queue.clear();
    runtime_shell.visible_special_text_pause_frames = None;
    runtime_shell.visible_internal_special_delay_frames = None;
    runtime_shell.pending_photo_studio_commit = None;
    runtime_shell.pending_special_cry = None;
    runtime_shell.pending_special_sound = None;
    runtime_shell.credits_screen = Some(VisibleCreditsScreen {
        allow_skip,
        resume_game_timer_on_exit: false,
        frame: 0,
        consumed_bytes: 0,
        awaiting_exit: false,
        scene_index: 0,
        timer: 0,
        script_index: 0,
        jumptable_index: visible_credits_initial_jumptable_index(allow_skip),
        lines: Vec::new(),
        border_frame_counter: None,
        border_frame_top: None,
        border_frame_bottom: None,
        border_frame_pending: None,
        border_frame_pending_blank: false,
        border_mon_index: 0,
        ly_override: 0,
        show_the_end: false,
        script_complete: false,
    });
    record_visible_runtime_action(runtime_shell, "credits:open")?;
    set_shell_action_status(runtime_shell, "CREDITS");
    runtime_shell
        .last_audio_events
        .push(format!("opened credits allow_skip={allow_skip}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn queue_visible_credits_music(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let silent = "MUSIC_NONE";
    let credits = "MUSIC_CREDITS";
    stop_visible_silent_music(runtime_shell, silent, "audio:music:credits:none")?;
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, credits)?;
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: credits.to_string(),
            kind: ModpackAudioKind::Music,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    runtime_shell.pending_music_stop = true;
    runtime_shell.active_music = Some(credits.to_string());
    runtime_shell.faded_music = None;
    runtime_shell
        .last_audio_events
        .push("queued credits music MUSIC_CREDITS".to_string());
    Ok(())
}

fn run_visible_credits_jumptable_step(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let step = {
        let Some(credits) = runtime_shell.credits_screen.as_ref() else {
            return Ok(());
        };
        visible_credits_step_index(credits)
    };
    match step {
        0 => step_parse_visible_credits(runtime_shell),
        1 | 2 | 7 | 8 | 9 => {
            visible_credits_next(runtime_shell);
            Ok(())
        }
        3 => {
            visible_credits_next(runtime_shell);
            Ok(())
        }
        4 | 10 => {
            load_visible_credits_border_frame(runtime_shell);
            visible_credits_next(runtime_shell);
            Ok(())
        }
        5 | 11 => {
            request_visible_credits_gfx(runtime_shell);
            visible_credits_next(runtime_shell);
            Ok(())
        }
        6 => {
            if let Some(credits) = runtime_shell.credits_screen.as_mut() {
                credits.ly_override = credits.ly_override.wrapping_sub(2);
            }
            visible_credits_next(runtime_shell);
            Ok(())
        }
        12 => {
            if let Some(credits) = runtime_shell.credits_screen.as_mut() {
                credits.jumptable_index &= 0xf0;
            }
            Ok(())
        }
        _ => anyhow::bail!("credits jumptable index {step} is out of range"),
    }
}

fn step_parse_visible_credits(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let ops = load_visible_credits_script(&runtime_shell.asset_root)?;
    let constants = load_visible_credit_constant_indices(&runtime_shell.asset_root)?;
    let strings = load_visible_credits_strings(&runtime_shell.asset_root)?;
    let string_tiles = load_visible_credits_string_tiles(&runtime_shell.asset_root)?;
    loop {
        let Some(credits) = runtime_shell.credits_screen.as_mut() else {
            return Ok(());
        };
        if credits.jumptable_index & (1 << VISIBLE_CREDITS_EXIT_BIT) != 0 {
            visible_credits_next(runtime_shell);
            return Ok(());
        }
        if credits.timer > 0 {
            credits.timer = credits.timer.saturating_sub(1);
            visible_credits_next(runtime_shell);
            return Ok(());
        }
        credits.lines.clear();
        credits.show_the_end = false;
        if credits.script_index >= ops.len() {
            credits.awaiting_exit = true;
            credits.script_complete = true;
            credits.jumptable_index |= 1 << VISIBLE_CREDITS_EXIT_BIT;
            set_shell_action_status(runtime_shell, "THE END");
            return Ok(());
        }
        while runtime_shell
            .credits_screen
            .as_ref()
            .is_some_and(|credits| credits.script_index < ops.len())
        {
            let op = {
                let credits = runtime_shell
                    .credits_screen
                    .as_mut()
                    .context("credits screen closed while parsing")?;
                let op = ops
                    .get(credits.script_index)
                    .cloned()
                    .context("credits script index moved outside loaded script")?;
                credits.script_index += 1;
                credits.consumed_bytes = credits
                    .consumed_bytes
                    .saturating_add(visible_credits_op_byte_len(&op));
                op
            };
            match op {
                VisibleCreditsOp::String { token, line_index } => {
                    let Some(string_index) = constants.get(&token).copied() else {
                        anyhow::bail!("credits script token {token} has no parsed constant index");
                    };
                    let text = strings
                        .get(string_index)
                        .with_context(|| {
                            format!(
                                "credits string token {token} index {string_index} is outside parsed string table"
                            )
                        })?
                        .clone();
                    let tiles = string_tiles
                        .get(string_index)
                        .with_context(|| {
                            format!(
                                "credits string tile token {token} index {string_index} is outside parsed tile table"
                            )
                        })?
                        .clone();
                    let credits = runtime_shell
                        .credits_screen
                        .as_mut()
                        .context("credits screen closed while adding line")?;
                    credits.lines.push(VisibleCreditsLine {
                        token,
                        text,
                        tiles,
                        line_index,
                    });
                }
                VisibleCreditsOp::Wait(duration) | VisibleCreditsOp::Wait2(duration) => {
                    let credits = runtime_shell
                        .credits_screen
                        .as_mut()
                        .context("credits screen closed while setting wait")?;
                    credits.timer = duration;
                    visible_credits_next(runtime_shell);
                    return Ok(());
                }
                VisibleCreditsOp::Scene(scene) => {
                    let credits = runtime_shell
                        .credits_screen
                        .as_mut()
                        .context("credits screen closed while setting scene")?;
                    credits.scene_index = scene % 4;
                    credits.border_frame_counter = Some(0);
                }
                VisibleCreditsOp::Clear => {
                    let credits = runtime_shell
                        .credits_screen
                        .as_mut()
                        .context("credits screen closed while clearing")?;
                    credits.border_frame_counter = None;
                }
                VisibleCreditsOp::Music => {
                    queue_visible_credits_music(runtime_shell)?;
                }
                VisibleCreditsOp::TheEnd => {
                    let credits = runtime_shell
                        .credits_screen
                        .as_mut()
                        .context("credits screen closed while showing The End")?;
                    credits.show_the_end = true;
                }
                VisibleCreditsOp::End => {
                    let credits = runtime_shell
                        .credits_screen
                        .as_mut()
                        .context("credits screen closed while ending")?;
                    credits.awaiting_exit = true;
                    credits.script_complete = true;
                    credits.jumptable_index |= 1 << VISIBLE_CREDITS_EXIT_BIT;
                    stop_visible_music(runtime_shell, "credits:postcredits:fade")?;
                    set_shell_action_status(runtime_shell, "THE END");
                    return Ok(());
                }
            }
        }
        if let Some(credits) = runtime_shell.credits_screen.as_mut() {
            credits.awaiting_exit = true;
            credits.script_complete = true;
            credits.jumptable_index |= 1 << VISIBLE_CREDITS_EXIT_BIT;
        }
        return Ok(());
    }
}

fn visible_credits_next(runtime_shell: &mut BevyRuntimeShell) {
    if let Some(credits) = runtime_shell.credits_screen.as_mut() {
        credits.jumptable_index = credits.jumptable_index.wrapping_add(1);
    }
}

fn load_visible_credits_border_frame(runtime_shell: &mut BevyRuntimeShell) {
    let Some(credits) = runtime_shell.credits_screen.as_mut() else {
        return;
    };
    let Some(frame_index) = credits.border_frame_counter else {
        credits.border_frame_top = None;
        credits.border_frame_pending = None;
        credits.border_frame_pending_blank = true;
        return;
    };
    credits.border_mon_index = credits.scene_index;
    let frame = VisibleCreditsBorderFrame {
        mon_index: credits.border_mon_index,
        frame_index,
    };
    credits.border_frame_top = Some(frame);
    credits.border_frame_pending = Some(frame);
    credits.border_frame_pending_blank = false;
    credits.border_frame_counter = Some((frame_index + 1) % CREDITS_FRAMES_PER_SCENE as u8);
}

fn request_visible_credits_gfx(runtime_shell: &mut BevyRuntimeShell) {
    let Some(credits) = runtime_shell.credits_screen.as_mut() else {
        return;
    };
    if credits.border_frame_pending_blank {
        credits.border_frame_bottom = None;
        credits.border_frame_pending_blank = false;
    } else if let Some(frame) = credits.border_frame_pending.take() {
        credits.border_frame_bottom = Some(frame);
    }
}

fn visible_credits_op_byte_len(op: &VisibleCreditsOp) -> u16 {
    match op {
        VisibleCreditsOp::Clear
        | VisibleCreditsOp::Music
        | VisibleCreditsOp::TheEnd
        | VisibleCreditsOp::End => 1,
        VisibleCreditsOp::String { .. }
        | VisibleCreditsOp::Wait(_)
        | VisibleCreditsOp::Wait2(_)
        | VisibleCreditsOp::Scene(_) => 2,
    }
}

fn load_visible_credit_constant_indices(asset_root: &AssetRoot) -> Result<BTreeMap<String, usize>> {
    let path = asset_root.resolve_vendor("constants/credits_constants.asm");
    let content = crate::read_runtime_asset_to_string(&path)
        .with_context(|| format!("read credits constants {}", path.display()))?;
    let mut constants = BTreeMap::new();
    let mut current_value = 0_usize;
    for raw_line in content.lines() {
        let line = strip_visible_asm_comment(raw_line);
        if line.starts_with("DEF NUM_CREDITS_STRINGS") {
            break;
        }
        if line.starts_with("const_def") {
            current_value = 0;
            continue;
        }
        let Some(rest) = line.strip_prefix("const ") else {
            continue;
        };
        let Some(name) = rest.split_whitespace().next() else {
            continue;
        };
        if constants.insert(name.to_string(), current_value).is_some() {
            anyhow::bail!("duplicate credits constant {name}");
        }
        current_value += 1;
    }
    if constants.is_empty() {
        anyhow::bail!("credits constants {} produced no entries", path.display());
    }
    Ok(constants)
}

fn load_visible_credits_strings(asset_root: &AssetRoot) -> Result<Vec<String>> {
    let path = asset_root.resolve_vendor("data/credits_strings.asm");
    let content = crate::read_runtime_asset_to_string(&path)
        .with_context(|| format!("read credits strings {}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let pointer_labels = parse_visible_credits_pointer_table(&lines);
    let string_blocks = parse_visible_credits_string_blocks(&lines);
    if pointer_labels.is_empty() {
        anyhow::bail!("credits string pointer table {} is empty", path.display());
    }
    let mut resolved = Vec::with_capacity(pointer_labels.len());
    for label in pointer_labels {
        let text = string_blocks
            .get(&label)
            .with_context(|| format!("missing credits string for label {label}"))?;
        resolved.push(text.clone());
    }
    Ok(resolved)
}

fn load_visible_credits_string_tiles(asset_root: &AssetRoot) -> Result<Vec<Vec<Vec<u16>>>> {
    let path = asset_root.resolve_vendor("data/credits_strings.asm");
    let content = crate::read_runtime_asset_to_string(&path)
        .with_context(|| format!("read credits string tiles {}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let pointer_labels = parse_visible_credits_pointer_table(&lines);
    let string_blocks = parse_visible_credits_string_tile_blocks(&lines)?;
    if pointer_labels.is_empty() {
        anyhow::bail!(
            "credits string tile pointer table {} is empty",
            path.display()
        );
    }
    let mut resolved = Vec::with_capacity(pointer_labels.len());
    for label in pointer_labels {
        let tiles = string_blocks
            .get(&label)
            .with_context(|| format!("missing credits string tiles for label {label}"))?;
        resolved.push(tiles.clone());
    }
    Ok(resolved)
}

fn parse_visible_credits_pointer_table(lines: &[&str]) -> Vec<String> {
    let mut labels = Vec::new();
    for raw_line in lines {
        let line = strip_visible_asm_comment(raw_line);
        if line.starts_with("assert_table_length") {
            break;
        }
        let Some(rest) = line.strip_prefix("dw") else {
            continue;
        };
        for entry in rest.split(',') {
            let normalized = entry.trim().trim_start_matches('.');
            if !normalized.is_empty() {
                labels.push(normalized.to_string());
            }
        }
    }
    labels
}

fn parse_visible_credits_string_blocks(lines: &[&str]) -> BTreeMap<String, String> {
    let mut strings = BTreeMap::new();
    let mut current_label: Option<String> = None;
    let mut buffer = String::new();
    for raw_line in lines {
        let line = strip_visible_asm_comment(raw_line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('.') {
            if let Some(label) = current_label.take() {
                strings.insert(label, buffer.replace('@', ""));
            }
            let (label, inline) = split_visible_credits_label(&line);
            current_label = Some(label);
            buffer.clear();
            append_visible_credits_string_directive(&mut buffer, &inline);
            continue;
        }
        append_visible_credits_string_directive(&mut buffer, &line);
    }
    if let Some(label) = current_label {
        strings.insert(label, buffer.replace('@', ""));
    }
    strings
}

fn parse_visible_credits_string_tile_blocks(
    lines: &[&str],
) -> Result<BTreeMap<String, Vec<Vec<u16>>>> {
    let mut strings = BTreeMap::new();
    let mut current_label: Option<String> = None;
    let mut current_lines: Vec<Vec<u16>> = Vec::new();
    for raw_line in lines {
        let line = strip_visible_asm_comment(raw_line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('.') {
            if let Some(label) = current_label.take() {
                strings.insert(label, current_lines.clone());
            }
            let (label, inline) = split_visible_credits_label(&line);
            current_label = Some(label);
            current_lines.clear();
            append_visible_credits_tile_directive(&mut current_lines, &inline)?;
            continue;
        }
        append_visible_credits_tile_directive(&mut current_lines, &line)?;
    }
    if let Some(label) = current_label {
        strings.insert(label, current_lines);
    }
    Ok(strings)
}
