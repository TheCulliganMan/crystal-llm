fn visible_title_menu_options(
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
) -> Vec<TitleMenuOption> {
    let mut options = Vec::new();
    if title_continue_save_path(runtime_shell, title).is_some() {
        options.push(TitleMenuOption::Continue);
    }
    options.push(TitleMenuOption::NewGame);
    options.push(TitleMenuOption::Options);
    if visible_title_mystery_gift_unlocked(runtime_shell, title) {
        options.push(TitleMenuOption::MysteryGift);
    }
    options
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

const VISIBLE_INTRO_SCENE_NAMES: [&str; 28] = [
    "unown_a",
    "unown_fade",
    "background_setup",
    "background_scroll",
    "unown_hi",
    "unown_pulse",
    "suicune_setup",
    "suicune_dash",
    "forest_hold",
    "grass_rustle",
    "unowns_tilemap",
    "unown_flash",
    "suicune_opening",
    "suicune_run",
    "suicune_jump",
    "suicune_rise",
    "suicune_close",
    "suicune_pan",
    "suicune_back",
    "unown_reveal",
    "color_swap",
    "sprite_clear",
    "transition",
    "palette_fade",
    "countdown",
    "crystal_unowns",
    "crystal_word_fade",
    "final_whoosh",
];
const VISIBLE_INTRO_CLEAR_BG_PALS_SCENES: &[usize] = &[0, 2, 4, 6, 10, 12, 14, 16, 18];
const VISIBLE_INTRO_CLEAR_BG_PALS_DELAY_FRAMES: u8 = 2;

impl VisibleIntroScreen {
    fn new() -> Self {
        Self {
            jumptable_index: 0,
            scene_frame_counter: 0,
            next_scene_frame_counter: None,
            scene_delay_frames: 0,
            scene_timer: 0,
            scroll_x: 0,
            scroll_y: 0,
            global_anim_x_offset: 0,
            sprite_count: 0,
            sprites: Vec::new(),
            palette_effect: VisibleIntroPaletteEffect::None,
            finished: false,
        }
    }

    fn scene_name(&self) -> &'static str {
        VISIBLE_INTRO_SCENE_NAMES
            .get(self.jumptable_index)
            .copied()
            .unwrap_or("complete")
    }
}

fn tick_visible_intro_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(intro) = runtime_shell.intro_screen.as_mut() else {
        return Ok(());
    };
    if intro.finished || intro.jumptable_index >= VISIBLE_INTRO_SCENE_NAMES.len() {
        return finish_visible_intro_screen(runtime_shell, "complete");
    }
    if intro.scene_delay_frames > 0 {
        intro.scene_delay_frames = intro.scene_delay_frames.saturating_sub(1);
        if intro.scene_delay_frames == 0 {
            visible_intro_next_scene(intro);
            apply_visible_intro_sprite_pipeline(intro);
        }
        if intro.finished {
            return finish_visible_intro_screen(runtime_shell, "complete");
        }
        return Ok(());
    }

    let scene_finished = step_visible_intro_scene(runtime_shell)?;
    let delay = if scene_finished {
        visible_intro_scene_delay_frames(
            runtime_shell
                .intro_screen
                .as_ref()
                .map(|intro| intro.jumptable_index)
                .unwrap_or(VISIBLE_INTRO_SCENE_NAMES.len()),
        )
    } else {
        0
    };
    let Some(intro) = runtime_shell.intro_screen.as_mut() else {
        return Ok(());
    };
    if delay == 0 {
        apply_visible_intro_sprite_pipeline(intro);
    }
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
    let Some(intro) = runtime_shell.intro_screen.as_mut() else {
        return Ok(false);
    };
    match intro.jumptable_index {
        0 => {
            clear_visible_intro_sprites(intro);
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
            if frame == 0x60 {
                init_visible_intro_unown_anim(intro, 11 * 8, 11 * 8)?;
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_1",
                )?;
            }
            intro.scene_timer = frame;
            intro.palette_effect = VisibleIntroPaletteEffect::UnownFade {
                palette_idx: 0,
                timer: frame,
            };
            Ok(false)
        }
        2 => {
            clear_visible_intro_sprites(intro);
            intro.scene_frame_counter = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        3 => Ok(intro.scene_frame_counter == 0x80),
        4 => {
            clear_visible_intro_sprites(intro);
            intro.scene_frame_counter = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        5 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x80 {
                return Ok(true);
            }
            if frame == 0x20 {
                init_visible_intro_unown_anim(intro, 15 * 8, 7 * 8)?;
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_2",
                )?;
            } else if frame == 0x60 {
                init_visible_intro_unown_anim(intro, 5 * 8, 14 * 8)?;
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_1",
                )?;
            }
            intro.scene_timer = frame;
            intro.palette_effect = VisibleIntroPaletteEffect::UnownFade {
                palette_idx: if frame >= 0x40 { 1 } else { 0 },
                timer: frame,
            };
            Ok(false)
        }
        6 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_SUICUNE", 27 * 8, 13 * 8 + 4)?;
            intro.global_anim_x_offset = 0xf0;
            intro.scene_frame_counter = 0;
            intro.scene_timer = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        7 => {
            let frame = intro.scene_frame_counter;
            if frame < 0x40 {
                return Ok(false);
            }
            if frame == 0x40 {
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_SUICUNE_3",
                )?;
            }
            if intro.global_anim_x_offset == 0 {
                clear_visible_intro_sprites(intro);
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_SUICUNE_2",
                )?;
                return Ok(true);
            }
            intro.global_anim_x_offset = intro.global_anim_x_offset.wrapping_sub(8);
            Ok(false)
        }
        8 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = 0;
            intro.scene_timer = 0;
            intro.global_anim_x_offset = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        9 => {
            let frame = intro.scene_frame_counter;
            if frame == 0x20 {
                spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_WOOPER", 6 * 8, 22 * 8)?;
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_PICHU",
                )?;
            }
            if frame == 0x40 {
                spawn_visible_intro_sprite(
                    intro,
                    "SPRITE_ANIM_OBJ_INTRO_PICHU",
                    16 * 8,
                    21 * 8 + 1,
                )?;
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_PICHU",
                )?;
            }
            Ok(frame == 0xc0)
        }
        10 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = 0;
            intro.scroll_y = 0;
            intro.global_anim_x_offset = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        11 => {
            let frame = intro.scene_frame_counter;
            match frame {
                0x00 => queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_3",
                )?,
                0x20 | 0x60 | 0x90 | 0xb0 => queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_2",
                )?,
                0x40 | 0xa0 => queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_1",
                )?,
                0x80 => queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_UNOWN_3",
                )?,
                _ => {}
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
            intro.palette_effect = VisibleIntroPaletteEffect::UnownFade { palette_idx, timer };
            Ok(false)
        }
        12 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_SUICUNE", 13 * 8 + 4, 11 * 8)?;
            intro.global_anim_x_offset = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            queue_visible_intro_music(runtime_shell, "MUSIC_CRYSTAL_OPENING")?;
            Ok(true)
        }
        13 => {
            let frame = intro.scene_frame_counter;
            intro.scroll_x = intro.scroll_x.wrapping_sub(10);
            if frame >= 0x80 {
                return Ok(true);
            }
            if frame >= 0x60 {
                if frame == 0x60 {
                    queue_visible_sound_effect(
                        runtime_shell.shell.runtime().audio(),
                        &mut runtime_shell.pending_audio,
                        &mut runtime_shell.last_audio_events,
                        "SFX_INTRO_SUICUNE_4",
                    )?;
                }
                intro.scene_timer = 1;
                if intro.global_anim_x_offset < 0x88 {
                    clear_visible_intro_sprites(intro);
                } else {
                    intro.global_anim_x_offset = intro.global_anim_x_offset.wrapping_sub(8);
                }
            } else if frame >= 0x40 {
                intro.global_anim_x_offset = intro.global_anim_x_offset.wrapping_sub(2);
            }
            Ok(false)
        }
        14 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_UNOWN_F", 5 * 8, 8 * 8)?;
            let suicune =
                spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY", 0, 12 * 8)?;
            suicune.gfx_name = "suicune_jump".to_string();
            intro.scroll_x = 0;
            intro.scroll_y = 144;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        15 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x80 {
                return Ok(true);
            }
            if intro.scroll_y != 0 {
                intro.scroll_y = intro.scroll_y.wrapping_add(8);
            }
            Ok(false)
        }
        16 => {
            clear_visible_intro_sprites(intro);
            intro.scroll_x = 0;
            intro.scroll_y = 0;
            intro.global_anim_x_offset = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        17 => {
            if intro.scene_frame_counter >= 0x60 {
                return Ok(true);
            }
            if intro.scroll_x != 0x60 {
                intro.scroll_x = intro.scroll_x.wrapping_add(8);
            }
            Ok(false)
        }
        18 => {
            clear_visible_intro_sprites(intro);
            spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY", 0, 12 * 8)?;
            intro.scroll_x = 0;
            intro.scroll_y = (-5_i16 * SOURCE_TILE_SIZE as i16).rem_euclid(256) as u8;
            intro.global_anim_x_offset = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::None;
            Ok(true)
        }
        19 => {
            let frame = intro.scene_frame_counter;
            if frame >= 0x98 {
                return Ok(true);
            }
            if frame < 0x28 {
                intro.scroll_y = intro.scroll_y.wrapping_add(1);
            } else if (0x40..0x58).contains(&frame) {
                let temp = frame.wrapping_sub(0x18);
                if (temp & 0x03) == 0x03 {
                    let timer = (temp & 0x1c) >> 2;
                    intro.scene_timer = timer;
                    intro.palette_effect = VisibleIntroPaletteEffect::AppearUnown {
                        palette_set_idx: 0,
                        revealed: timer,
                    };
                }
            }
            Ok(false)
        }
        20 => {
            intro.scene_frame_counter = 0;
            intro.scene_timer = 0;
            intro.palette_effect = VisibleIntroPaletteEffect::AppearUnown {
                palette_set_idx: 0,
                revealed: 7,
            };
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
                intro.palette_effect = VisibleIntroPaletteEffect::Scene24Fade { fade_index: 7 };
                return Ok(true);
            }
            intro.palette_effect = VisibleIntroPaletteEffect::Scene24Fade {
                fade_index: (frame & 0x1c) >> 2,
            };
            Ok(false)
        }
        24 => {
            let current = intro.scene_frame_counter;
            let next = current.wrapping_sub(1);
            intro.next_scene_frame_counter = Some(next);
            intro.palette_effect = VisibleIntroPaletteEffect::Scene24Fade { fade_index: 7 };
            Ok(next == 0)
        }
        25 => {
            clear_visible_intro_sprites(intro);
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
            intro.palette_effect = VisibleIntroPaletteEffect::CrystalWordFade {
                fade_level: (frame & 0x70) >> 4,
                timer: frame & 0x0f,
            };
            Ok(false)
        }
        27 => {
            let current = intro.scene_frame_counter;
            if current == 0 {
                return Ok(true);
            }
            if current == 0x18 {
                runtime_shell
                    .last_audio_events
                    .push("intro clear bg palettes".to_string());
                intro.palette_effect = VisibleIntroPaletteEffect::ClearBg;
            } else if current == 0x08 {
                queue_visible_sound_effect(
                    runtime_shell.shell.runtime().audio(),
                    &mut runtime_shell.pending_audio,
                    &mut runtime_shell.last_audio_events,
                    "SFX_INTRO_WHOOSH",
                )?;
            }
            if current < 0x18 {
                intro.palette_effect = VisibleIntroPaletteEffect::ClearBg;
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

fn spawn_visible_intro_sprite<'a>(
    intro: &'a mut VisibleIntroScreen,
    object_name: &str,
    x: i16,
    y: i16,
) -> Result<&'a mut VisibleIntroSprite> {
    let (frameset_name, anim_function, gfx_name) = visible_intro_sprite_definition(object_name)?;
    intro.sprites.push(VisibleIntroSprite {
        x,
        y,
        oam_attr: 0,
        gfx_name: gfx_name.to_string(),
        jumptable_index: 0,
        frame_timer: 0,
        frameset_step: -1,
        start_delay: 0,
        x_offset: 0,
        y_offset: 0,
        var1: 0,
        var2: 0,
        frameset_name: frameset_name.to_string(),
        object_name: object_name.to_string(),
        anim_function: anim_function.to_string(),
        current_oam_set: None,
        attr_flags: 0,
    });
    intro.sprite_count = intro.sprites.len().min(u8::MAX as usize) as u8;
    Ok(intro.sprites.last_mut().expect("pushed intro sprite"))
}

fn visible_intro_sprite_definition(
    object_name: &str,
) -> Result<(&'static str, &'static str, &'static str)> {
    Ok(match object_name {
        "SPRITE_ANIM_OBJ_INTRO_SUICUNE" => (
            "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE",
            "SPRITE_ANIM_FUNC_INTRO_SUICUNE",
            "suicune_run",
        ),
        "SPRITE_ANIM_OBJ_INTRO_PICHU" => (
            "SPRITE_ANIM_FRAMESET_INTRO_PICHU",
            "SPRITE_ANIM_FUNC_INTRO_PICHU_WOOPER",
            "pichu_wooper",
        ),
        "SPRITE_ANIM_OBJ_INTRO_WOOPER" => (
            "SPRITE_ANIM_FRAMESET_INTRO_WOOPER",
            "SPRITE_ANIM_FUNC_INTRO_PICHU_WOOPER",
            "pichu_wooper",
        ),
        "SPRITE_ANIM_OBJ_INTRO_UNOWN" => (
            "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1",
            "SPRITE_ANIM_FUNC_INTRO_UNOWN",
            "pulse",
        ),
        "SPRITE_ANIM_OBJ_INTRO_UNOWN_F" => (
            "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F",
            "SPRITE_ANIM_FUNC_INTRO_UNOWN_F",
            "unown_back",
        ),
        "SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY" => (
            "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_AWAY",
            "SPRITE_ANIM_FUNC_INTRO_SUICUNE_AWAY",
            "suicune_back",
        ),
        other => anyhow::bail!("unknown intro sprite object {other}"),
    })
}

fn init_visible_intro_unown_anim(intro: &mut VisibleIntroScreen, x: i16, y: i16) -> Result<()> {
    const FRAMES: [&str; 4] = [
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_4",
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_3",
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1",
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_2",
    ];
    const AMPLITUDES: [u8; 4] = [0x08, 0x18, 0x28, 0x38];
    for (frameset_name, amplitude) in FRAMES.into_iter().zip(AMPLITUDES) {
        let sprite = spawn_visible_intro_sprite(intro, "SPRITE_ANIM_OBJ_INTRO_UNOWN", x, y)?;
        sprite.frameset_name = frameset_name.to_string();
        sprite.var1 = amplitude;
        sprite.jumptable_index = 0;
    }
    Ok(())
}

fn apply_visible_intro_sprite_pipeline(intro: &mut VisibleIntroScreen) {
    if intro.jumptable_index == 20 {
        intro.scene_timer = 0;
    }
    apply_visible_intro_sprite_anim_functions(intro);
    update_visible_intro_sprite_animations(intro);
    intro.sprite_count = intro.sprites.len().min(u8::MAX as usize) as u8;
}

fn apply_visible_intro_sprite_anim_functions(intro: &mut VisibleIntroScreen) {
    for sprite in &mut intro.sprites {
        match sprite.anim_function.as_str() {
            "SPRITE_ANIM_FUNC_INTRO_SUICUNE" => {
                if intro.scene_timer == 0 {
                    sprite.y_offset = 0;
                    continue;
                }
                sprite.var2 = sprite.var2.wrapping_add(2);
                let angle = (!sprite.var2).wrapping_add(1);
                sprite.y_offset = visible_intro_sine(angle, 32);
                sprite.x_offset = 0;
                if sprite.frameset_name != "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2" {
                    sprite.frameset_name = "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2".to_string();
                    sprite.frameset_step = -1;
                    sprite.frame_timer = 0;
                    sprite.current_oam_set = None;
                }
            }
            "SPRITE_ANIM_FUNC_INTRO_PICHU_WOOPER" => {
                if sprite.var1 < 20 {
                    sprite.var1 = sprite.var1.wrapping_add(2);
                }
                let angle = (!sprite.var1).wrapping_add(1);
                sprite.y_offset = visible_intro_sine(angle, 32);
                sprite.x_offset = 0;
            }
            "SPRITE_ANIM_FUNC_INTRO_UNOWN" => {
                let direction = sprite.var1;
                let distance = sprite.jumptable_index;
                sprite.y_offset = visible_intro_sine(direction, i16::from(distance));
                sprite.x_offset = visible_intro_cosine(direction, i16::from(distance));
                sprite.jumptable_index = sprite.jumptable_index.wrapping_add(3);
            }
            "SPRITE_ANIM_FUNC_INTRO_UNOWN_F" => {}
            "SPRITE_ANIM_FUNC_INTRO_SUICUNE_AWAY" => {
                sprite.y = sprite.y.saturating_add(16);
                sprite.x_offset = 0;
            }
            _ => {}
        }
    }
}

fn update_visible_intro_sprite_animations(intro: &mut VisibleIntroScreen) {
    let mut next = Vec::with_capacity(intro.sprites.len());
    for mut sprite in intro.sprites.drain(..) {
        if sprite.start_delay > 0 {
            sprite.start_delay = sprite.start_delay.saturating_sub(1);
            next.push(sprite);
            continue;
        }
        let Some(frameset) = visible_intro_frameset_steps(&sprite.frameset_name) else {
            next.push(sprite);
            continue;
        };
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
            match step.command {
                IntroFrameCommand::Frame => {
                    sprite.current_oam_set = step.oam_set.map(str::to_string);
                    sprite.attr_flags = step.attr_flags;
                    sprite.frame_timer = step.duration.saturating_sub(1);
                    break;
                }
                IntroFrameCommand::Wait => {
                    sprite.frame_timer = step.duration.saturating_sub(1);
                    break;
                }
                IntroFrameCommand::Restart => {
                    step_index = -1;
                    sprite.frame_timer = 0;
                    continue;
                }
                IntroFrameCommand::End => {
                    // `oamend` rewinds the frame cursor to the preceding
                    // OAM frame, so the last image is held indefinitely.
                    step_index = step_index.saturating_sub(2);
                    sprite.frame_timer = 0;
                    continue;
                }
                IntroFrameCommand::Delete => {
                    removed = true;
                    break;
                }
            }
        }
        sprite.frameset_step = step_index;
        if !removed {
            next.push(sprite);
        }
    }
    intro.sprites = next;
}

#[derive(Clone, Copy)]
struct IntroFrameStep {
    oam_set: Option<&'static str>,
    duration: u8,
    attr_flags: u8,
    command: IntroFrameCommand,
}

#[derive(Clone, Copy)]
enum IntroFrameCommand {
    Frame,
    Wait,
    Restart,
    End,
    Delete,
}

fn visible_intro_frameset_steps(name: &str) -> Option<Vec<IntroFrameStep>> {
    let frame = |oam_set, duration, attr_flags| IntroFrameStep {
        oam_set: Some(oam_set),
        duration,
        attr_flags,
        command: IntroFrameCommand::Frame,
    };
    let restart = IntroFrameStep {
        oam_set: None,
        duration: 0,
        attr_flags: 0,
        command: IntroFrameCommand::Restart,
    };
    let delete = IntroFrameStep {
        oam_set: None,
        duration: 0,
        attr_flags: 0,
        command: IntroFrameCommand::Delete,
    };
    let end = IntroFrameStep {
        oam_set: None,
        duration: 0,
        attr_flags: 0,
        command: IntroFrameCommand::End,
    };
    Some(match name {
        "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_1", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_2", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_3", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_4", 3, 0),
            restart,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_4", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_1", 7, 0),
            end,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_PICHU" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_PICHU_1", 32, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_PICHU_2", 7, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_PICHU_3", 7, 0),
            end,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_WOOPER" => {
            vec![frame("SPRITE_ANIM_OAMSET_INTRO_WOOPER", 3, 0), end]
        }
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_1", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_2", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_3", 7, 0),
            delete,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_2" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_1", 3, 0x20),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_2", 3, 0x20),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_3", 7, 0x20),
            delete,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_3" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_1", 3, 0x40),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_2", 3, 0x40),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_3", 7, 0x40),
            delete,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_4" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_1", 3, 0x60),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_2", 3, 0x60),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_3", 7, 0x60),
            delete,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2" => vec![
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_F_2_1", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_F_2_2", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_F_2_3", 3, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_F_2_4", 7, 0),
            frame("SPRITE_ANIM_OAMSET_INTRO_UNOWN_F_2_5", 7, 0),
            end,
        ],
        "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_AWAY" => {
            vec![frame("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_AWAY", 3, 0), end]
        }
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F" => vec![IntroFrameStep {
            oam_set: None,
            duration: 1,
            attr_flags: 0,
            command: IntroFrameCommand::Wait,
        }],
        _ => return None,
    })
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

fn visible_intro_next_scene(intro: &mut VisibleIntroScreen) {
    intro.jumptable_index = intro.jumptable_index.saturating_add(1);
    intro.scene_frame_counter = intro.next_scene_frame_counter.take().unwrap_or(0);
    intro.scene_timer = 0;
    intro.scene_delay_frames = 0;
    if intro.jumptable_index >= VISIBLE_INTRO_SCENE_NAMES.len() {
        intro.finished = true;
    }
}

fn visible_intro_scene_delay_frames(index: usize) -> u8 {
    if VISIBLE_INTRO_CLEAR_BG_PALS_SCENES.contains(&index) {
        VISIBLE_INTRO_CLEAR_BG_PALS_DELAY_FRAMES
    } else if index == 8 {
        6
    } else if index == 20 {
        3
    } else {
        0
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
        title.phase = VisibleTitlePhase::Entrance;
        title.frame = 0;
        title.main_menu_frame = 0;
        title.scx = VISIBLE_TITLE_ENTRANCE_START_SCX;
        title.title_timer = 0;
        title.clock_reset_trigger = false;
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
    let Some(title) = runtime_shell.title_menu.as_mut() else {
        return;
    };
    title.frame = title.frame.saturating_add(1);
    match title.phase {
        VisibleTitlePhase::Entrance => {
            if title.scx == 0 {
                title.phase = VisibleTitlePhase::Timer;
                title.title_timer = 0;
                return;
            }
            title.scx = title.scx.saturating_sub(VISIBLE_TITLE_ENTRANCE_SCROLL_STEP);
        }
        VisibleTitlePhase::Timer => {
            title.phase = VisibleTitlePhase::PressStart;
            title.title_timer = VISIBLE_TITLE_TIMEOUT_FRAMES;
        }
        VisibleTitlePhase::PressStart => {
            if title.title_timer == 0 {
                title.phase = VisibleTitlePhase::Timeout;
            } else {
                title.title_timer = title.title_timer.saturating_sub(1);
            }
        }
        VisibleTitlePhase::MainMenu => {
            title.main_menu_frame = title.main_menu_frame.saturating_add(1);
        }
        VisibleTitlePhase::Timeout | VisibleTitlePhase::Exiting => {}
    }
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
    title.phase = VisibleTitlePhase::MainMenu;
    title.main_menu_frame = 0;
    title.clock_reset_trigger = false;
    title.cursor.option_index = title.cursor.option_index.min(menu_len.saturating_sub(1));
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
    runtime_shell.pending_gender_selection = Some(VisibleGenderSelection {
        selected_index: 0,
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
    let update =
        runtime_shell
            .shell
            .set_manual_clock_time(GameDate::new(2000, 1, 1), 0, 0, 0, target)?;
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
        time_set.hour,
        time_set.minute,
        update.time_of_day,
        update.game_time_hours,
        update.game_time_minutes
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
                queue_visible_oak_intro_text(oak_intro, &VISIBLE_OAK_INTRO_SCENES[1].2[2..]);
                advance_visible_oak_intro_text_queue(oak_intro);
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
        oak_intro.waiting_for_input = true;
        return;
    }
    oak_intro.text_timer = oak_intro.text_timer.saturating_add(1);
    if oak_intro.text_timer >= VISIBLE_OAK_INTRO_TEXT_SPEED_FRAMES {
        oak_intro.text_timer = 0;
        oak_intro.visible_chars = oak_intro.visible_chars.saturating_add(1);
        if oak_intro.visible_chars >= oak_intro.current_text.chars().count() {
            oak_intro.waiting_for_input = true;
        }
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
        oak_intro.waiting_for_input = true;
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
    let current = gender.selected_index.min(1);
    let next = if delta.is_negative() {
        current.checked_sub(delta.unsigned_abs()).unwrap_or(1)
    } else {
        (current + delta as usize) % 2
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
    gender.confirm_countdown = VISIBLE_GENDER_CONFIRM_DELAY_FRAMES;
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
    if gender.selected_index == 1 {
        VisiblePlayerGender::Girl
    } else {
        VisiblePlayerGender::Boy
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
    ["BOY", "GIRL"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            if index == gender.selected_index.min(1) {
                format!("> {label}")
            } else {
                format!("  {label}")
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
    for _ in
        0..=usize::from(VISIBLE_TITLE_ENTRANCE_START_SCX / VISIBLE_TITLE_ENTRANCE_SCROLL_STEP + 2)
    {
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
    _runtime_shell: &BevyRuntimeShell,
    title: &'a TitleMenu,
) -> Option<&'a PathBuf> {
    let path = title.save_path.as_ref()?;
    if path.exists() {
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
            VisibleTitlePhase::Timeout => "TIMEOUT".to_string(),
            VisibleTitlePhase::Exiting => "EXITING".to_string(),
        }]);
    }
    let options = visible_title_menu_options(runtime_shell, title);
    let selected = title.cursor.option_index.min(options.len() - 1);
    Ok(options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let marker = if index == selected { ">" } else { " " };
            let label = match option {
                TitleMenuOption::Continue => "CONTINUE",
                TitleMenuOption::NewGame => "NEW GAME",
                TitleMenuOption::Options => "OPTION",
                TitleMenuOption::MysteryGift => "MYSTERY GIFT",
            };
            format!("{marker}{label}")
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
) -> Result<TitleMenuOption> {
    let options = visible_title_menu_options(runtime_shell, title);
    let selected = title.cursor.option_index.min(options.len() - 1);
    Ok(options[selected])
}

fn press_visible_title_confirm_button(
    runtime_shell: &mut BevyRuntimeShell,
    input: GameButton,
) -> Result<()> {
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return handle_visible_no_active_title_menu(runtime_shell, "confirm");
    };
    if !visible_title_main_menu_ready(title) {
        return open_visible_title_main_menu(runtime_shell);
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
        title.phase = VisibleTitlePhase::Entrance;
        title.frame = 0;
        title.main_menu_frame = 0;
        title.scx = VISIBLE_TITLE_ENTRANCE_START_SCX;
        title.title_timer = 0;
        title.clock_reset_trigger = false;
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
        Some(path) => match std::fs::remove_file(&path) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                return Err(error).with_context(|| format!("delete save {}", path.display()));
            }
        },
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
        hour: time.game_time_hours.min(23),
        minute: time.game_time_minutes.min(59),
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
        title.phase = VisibleTitlePhase::Entrance;
        title.frame = 0;
        title.main_menu_frame = 0;
        title.scx = VISIBLE_TITLE_ENTRANCE_START_SCX;
        title.title_timer = 0;
        title.clock_reset_trigger = false;
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
            let update = runtime_shell.shell.set_manual_clock_time(
                GameDate::new(2000, 1, 1),
                0,
                0,
                0,
                ClockTime::new(clock.day, clock.hour, clock.minute, 0),
            )?;
            runtime_shell.last_audio_events.push(format!(
                "clock reset day={} game={}:{} checksum={:?}",
                update.day_of_week,
                update.game_time_hours,
                update.game_time_minutes,
                update.state_checksum
            ));
            close_visible_clock_reset_screen(runtime_shell, "confirm")
        }
    }
}

const VISIBLE_CREDITS_SKIP_THRESHOLD: u16 = 0x0d;
const VISIBLE_CREDITS_ALLOW_SKIP_BIT: u8 = 6;
const VISIBLE_CREDITS_EXIT_BIT: u8 = 7;
// SpawnPoints.asm: SPAWN_NEW_BARK follows the 0-based HOME/DEBUG and Kanto
// entries.  Credits stores SPAWN_LANCE in wSpawnAfterChampion; Continue then
// converts that marker to SPAWN_NEW_BARK before loading the overworld.
const POST_CREDITS_NEW_BARK_SPAWN_IDENTIFIER: u16 = 14;
const POST_CREDITS_MT_SILVER_SPAWN_IDENTIFIER: u16 = 26;
const VISIBLE_TITLE_ENTRANCE_START_SCX: u8 = 112;
const VISIBLE_TITLE_ENTRANCE_SCROLL_STEP: u8 = 4;
const VISIBLE_TITLE_TIMEOUT_FRAMES: u16 = 73 * 60 + 36;
const VISIBLE_GENDER_CONFIRM_DELAY_FRAMES: u8 = 10;
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
    runtime_shell.pending_special_cry = None;
    runtime_shell.pending_special_sound = None;
    runtime_shell.credits_screen = Some(VisibleCreditsScreen {
        allow_skip,
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
    if runtime_shell
        .shell
        .runtime()
        .audio()
        .playback_entry(AudioKind::Music, silent)
        .is_some()
    {
        stop_visible_silent_music(runtime_shell, silent, "audio:music:credits:none")?;
    }
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
    let content = std::fs::read_to_string(&path)
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
    let content = std::fs::read_to_string(&path)
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
    let content = std::fs::read_to_string(&path)
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
