fn prepare_visible_battle_entry(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    prepare_visible_battle_entry_with_music_reset(runtime_shell, true)
}

fn prepare_visible_battle_entry_after_visible_step(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    prepare_visible_battle_entry_with_music_reset(runtime_shell, false)
}

fn prepare_visible_battle_entry_with_music_reset(
    runtime_shell: &mut BevyRuntimeShell,
    reset_music: bool,
) -> Result<()> {
    runtime_shell.visible_battle_transition = None;
    runtime_shell.visible_capture_animation = None;
    runtime_shell.visible_move_animations.clear();
    runtime_shell.visible_send_out_animation = None;
    runtime_shell.visible_trainer_exit_animation = None;
    runtime_shell.visible_frontpic_animation = None;
    reset_visible_selection_cursors(runtime_shell);
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_text_reveal = None;
    runtime_shell.field_text_reveal = None;
    runtime_shell.battle_fanfare_messages.clear();
    runtime_shell.battle_evolution_cries.clear();
    runtime_shell.battle_evolution_cancellations.clear();
    runtime_shell.field_evolution_cancellation = None;
    runtime_shell.battle_sounds_after_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_message_scene = None;
    runtime_shell.battle_hp_tween = None;
    runtime_shell.battle_exp_tween = None;
    runtime_shell.pending_battle_exp_tweens.clear();
    runtime_shell.battle_level_stats.clear();
    runtime_shell.party_move_cursor = None;
    runtime_shell.last_battle_cry_key = None;
    runtime_shell.pending_battle_cries_after_messages.clear();
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_enemy_hp_at_player_send_out = None;
    runtime_shell.pending_battle_scenes_after_message.clear();
    if reset_music {
        reset_visible_music_state(runtime_shell);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("battle entry requires an active battle snapshot")?;
    runtime_shell.battle_enemy_hp_at_player_send_out = Some(battle.enemy_pokemon.hp);
    let active_player = battle
        .active_player_party_index
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
        .context("battle entry requires its active party slot in the runtime snapshot")?;
    let player_level = active_player.pokemon.level;
    let environment = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "battle entry map {} is absent from the runtime map catalog",
                snapshot.overworld.map_name
            )
        })?
        .attributes
        .environment
        .as_deref()
        .context("battle entry map has no source environment")?;
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: 0,
        stronger_enemy: player_level.saturating_add(3) < battle.enemy_pokemon.level,
        cave_environment: ["CAVE", "ENVIRONMENT_5", "DUNGEON"]
            .iter()
            .any(|candidate| environment.eq_ignore_ascii_case(candidate)),
        trainer_battle: matches!(&battle.kind, crate::RuntimeBattleKind::Trainer { .. }),
    });
    let (player_hp, player_max_hp, player_pixels) = (
        active_player.pokemon.hp,
        active_player.pokemon.max_hp,
        battle_hud_hp_pixels(active_player.pokemon.hp, active_player.pokemon.max_hp),
    );
    let enemy_pixels = battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp);
    runtime_shell.battle_hp_tween = Some(VisibleBattleHpTween {
        player_hp,
        player_target_hp: player_hp,
        player_max_hp,
        player_pixels,
        player_target_pixels: player_pixels,
        player_frames_until_step: 0,
        enemy_pixels,
        enemy_target_pixels: enemy_pixels,
        enemy_frames_until_step: 0,
    });
    let player_send_out_message = visible_player_send_out_message(
        &snapshot,
        active_player.index,
        true,
    )?;
    match &battle.kind {
        crate::RuntimeBattleKind::Trainer { trainer_name, .. } => {
            runtime_shell
                .battle_messages
                .push_back(format!("{trainer_name}\nwants to battle!"));
            runtime_shell.battle_messages.push_back(format!(
                "{}\nsent out\n{}!",
                trainer_name, battle.enemy_pokemon.nickname
            ));
            runtime_shell
                .battle_messages
                .push_back(player_send_out_message.clone());
        }
        crate::RuntimeBattleKind::Wild { .. } | crate::RuntimeBattleKind::StaticWild { .. } => {
            runtime_shell
                .battle_messages
                .push_back(format!("Wild {}\nappeared!", battle.enemy_pokemon.nickname));
            if battle.battle_type != "BATTLETYPE_TUTORIAL" {
                runtime_shell
                    .battle_messages
                    .push_back(player_send_out_message.clone());
            }
        }
    }
    // Entry narration is already a retained battle presentation.
    // Keep the exact battle-start snapshot behind those messages so
    // the renderer never falls back to an absent scene (a blank/error
    // frame) before the first text page is acknowledged.
    runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
    if battle.battle_type == "BATTLETYPE_TUTORIAL" {
        let actions = visible_battle_action_ids(&snapshot, battle);
        if let Some(option_index) = actions
            .iter()
            .position(|action| *action == VisibleBattleAction::Pack)
        {
            // ASM DudeAutoInput_DownA leaves the four-command menu
            // visible with PACK selected before TutorialPack continues.
            runtime_shell.battle_action_cursor = Some(MenuCursor {
                surface_id: "battle:actions".to_string(),
                option_index,
            });
        }
    }
    runtime_shell.battle_entry_messages_remaining = runtime_shell.battle_messages.len();
    Ok(())
}

fn advance_visible_battle_transition(runtime_shell: &mut BevyRuntimeShell) {
    let Some(transition) = runtime_shell.visible_battle_transition.as_mut() else {
        return;
    };
    transition.frame = transition.frame.saturating_add(1);
    if transition.frame >= visible_battle_transition_total_frames(transition) {
        runtime_shell.visible_battle_transition = None;
    }
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn visible_battle_transition_total_frames(transition: &VisibleBattleTransition) -> u16 {
    let prefix_frames = if transition.trainer_battle { 4 } else { 3 };
    let (between_frames, outro_frames, finish_frames) =
        match (transition.cave_environment, transition.stronger_enemy) {
            // DETERMINE/LOAD/SETUP precede all four paths. After the three
            // flashes, ordinary paths and outdoor scatter own NEXT + SETUP;
            // the stronger cave zoom owns NEXT only.
            (true, false) => (2, 15, 1),
            (true, true) => (1, 9, 2),
            (false, false) => (2, 61, 4),
            (false, true) => (2, 21, 1),
        };
    prefix_frames + 75 + between_frames + outro_frames + finish_frames
}

fn visible_battle_transition_is_terminal(transition: &VisibleBattleTransition) -> bool {
    transition.frame.saturating_add(1) >= visible_battle_transition_total_frames(transition)
}

fn spawn_visible_battle_transition(
    commands: &mut Commands,
    transition: VisibleBattleTransition,
    viewport_texture: Option<Handle<Image>>,
    priority_texture: Option<Handle<Image>>,
) {
    const POKEBALL_PATTERN: [&str; 16] = [
        "......XXXX......",
        "....XXXXXXXX....",
        "..XXXX....XXXX..",
        "..XX........XX..",
        ".XX..........XX.",
        ".XX...XXXX...XX.",
        "XX...XX..XX...XX",
        "XXXXXX....XXXXXX",
        "XXXXXX....XXXXXX",
        "XX...XX..XX...XX",
        ".XX...XXXX...XX.",
        ".XX..........XX.",
        "..XX........XX..",
        "..XXXX....XXXX..",
        "....XXXXXXXX....",
        "......XXXX......",
    ];
    // battle-transition.ts averages the four packed BGP crumbs and converts
    // their distance from shade 3 into a white-overlay alpha.
    const FLASH_ALPHA: [f32; 12] = [
        0.25,
        1.0 / 12.0,
        0.0,
        1.0 / 12.0,
        0.25,
        0.5,
        0.75,
        11.0 / 12.0,
        1.0,
        11.0 / 12.0,
        0.75,
        0.5,
    ];
    let frame = usize::from(transition.frame);
    let prefix_frames = if transition.trainer_battle { 4 } else { 3 };
    // Trainer transitions begin with the ASM 16x16 Poké Ball cutout on a
    // black 20x18 tilemap. Wild transitions retain the ordinary square map.
    if transition.trainer_battle && frame >= 2 {
        for y in 0..18 {
            for x in 0..20 {
                let inside_ball = (2..18).contains(&x)
                    && (1..17).contains(&y)
                    && POKEBALL_PATTERN[y - 1].as_bytes()[x - 2] == b'X';
                if !inside_ball {
                    spawn_visible_battle_transition_black_tile(commands, x, y);
                }
            }
        }
    }
    if frame < prefix_frames {
        return;
    }
    let flash_frames = 75;
    let effect_frame = frame - prefix_frames;
    if effect_frame < flash_frames {
        let sweep_frame = effect_frame % 25;
        let flash_index = sweep_frame / 2;
        let alpha = FLASH_ALPHA[flash_index.min(FLASH_ALPHA.len() - 1)];
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(248.0 / 255.0, 248.0 / 255.0, 248.0 / 255.0, alpha),
                    custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 2.75),
                ..default()
            },
            BattleCommandMarker,
        ));
        return;
    }

    let (between_frames, outro_frames) =
        match (transition.cave_environment, transition.stronger_enemy) {
            (true, false) => (2, 15),
            (true, true) => (1, 9),
            (false, false) => (2, 61),
            (false, true) => (2, 21),
        };
    if effect_frame < flash_frames + between_frames {
        return;
    }
    let outro = effect_frame - flash_frames - between_frames;
    if outro >= outro_frames {
        // DoBattleTransition finishes by blacking every BG palette and holding
        // that complete frame before battle setup takes over. Keep this as an
        // explicit surface instead of retaining the final (sometimes partial)
        // outro geometry beneath the white battle canvas handoff.
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::BLACK,
                    custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 2.76),
                ..default()
            },
            BattleCommandMarker,
        ));
        return;
    }
    // `outro == 0` is already the first call of the selected effect state:
    // NextScene and the optional setup state are accounted for above.  Drive
    // every effect from the one-based number of source calls that have run so
    // the first mutation is visible and the final source mutation is not
    // replaced prematurely by the terminal-black hold.
    let effect_step = outro.saturating_add(1);
    match (transition.cave_environment, transition.stronger_enemy) {
        (true, true) => {
            // StartTrainerBattle_ZoomToBlack writes nine centered boxes in
            // one BG-map update apiece: 4x2 through the full 20x18 LCD.
            let boxes = effect_step.min(9);
            let width_tiles = 2 + boxes * 2;
            let height_tiles = boxes * 2;
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLACK,
                        custom_size: Some(Vec2::new(
                            width_tiles as f32 * TILE_SIZE,
                            height_tiles as f32 * TILE_SIZE,
                        )),
                        ..default()
                    },
                    // Tile (0, 0) now begins at the LCD edge, so the growing
                    // even-sized box is centered on the camera itself.
                    transform: Transform::from_xyz(0.0, 0.0, 2.7),
                    ..default()
                },
                BattleCommandMarker,
            ));
        }
        (false, true) => {
            // SpeckleToBlack chooses twelve previously unfilled LCD tiles on
            // each of sixteen frames, then holds for three frames. Reproduce
            // the TypeScript transition's seed-zero LCG and rejection of
            // tiles already black in the trainer Poké Ball mask.
            let calls = effect_step.min(16) * 12;
            let mut black = [false; 20 * 18];
            if transition.trainer_battle {
                for y in 0..18 {
                    for x in 0..20 {
                        black[y * 20 + x] = !((2..18).contains(&x)
                            && (1..17).contains(&y)
                            && POKEBALL_PATTERN[y - 1].as_bytes()[x - 2] == b'X');
                    }
                }
            }
            let mut seed = 0_u32;
            for _ in 0..calls {
                for _ in 0..(20 * 18) {
                    seed = (seed * 9_301 + 49_297) % 233_280;
                    let y = (seed * 18 / 233_280) as usize;
                    seed = (seed * 9_301 + 49_297) % 233_280;
                    let x = (seed * 20 / 233_280) as usize;
                    if !black[y * 20 + x] {
                        black[y * 20 + x] = true;
                        break;
                    }
                }
            }
            for (index, filled) in black.into_iter().enumerate() {
                if filled {
                    spawn_visible_battle_transition_black_tile(commands, index % 20, index / 20);
                }
            }
        }
        (false, false) => {
            // SpinToBlack has twenty wedge entries, each held for two LCD
            // delay frames after each write. The twentieth entry therefore
            // remains current for the source terminal hold as well.
            let wedge_count = ((effect_step + 2) / 3).min(20);
            for wedge_index in 0..wedge_count {
                spawn_visible_battle_transition_wedge(commands, wedge_index);
            }
        }
        (true, false) => {
            // The retained Bevy viewport is the complete source surface. Draw
            // every native scanline independently and wrap it like the Game
            // Boy BG map when the transition changes SCX.
            if let Some(texture) = viewport_texture {
                let mut counter = 0_u8;
                let mut offset = 0_u8;
                let mut amplitude = 0_u8;
                for _ in 0..effect_step {
                    amplitude = counter;
                    let previous_offset = offset;
                    offset = offset.wrapping_add(1);
                    // StartTrainerBattle_SineWave increments the offset byte
                    // in memory while A still contains its previous value;
                    // that old value is what the counter adds this frame.
                    counter = counter.wrapping_add(previous_offset);
                }
                let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
                for source_y in 0..TITLE_SCREEN_HEIGHT {
                    // The LCD override loop feeds angles 0, 2, 4, ... to
                    // calc_sine_wave for successive scanlines. Each Bevy strip
                    // is one native scanline and uses the raw counter amplitude.
                    let angle = (source_y as u8).wrapping_mul(2);
                    let shift = visible_battle_anim_sine(angle, amplitude) as f32;
                    let shift = shift * source_scale;
                    let wrap_shift = if shift > 0.0 {
                        Some(shift - PLAYFIELD_WIDTH)
                    } else if shift < 0.0 {
                        Some(shift + PLAYFIELD_WIDTH)
                    } else {
                        None
                    };
                    for x in std::iter::once(shift).chain(wrap_shift) {
                        commands.spawn((
                            SpriteBundle {
                                texture: texture.clone(),
                                sprite: Sprite {
                                    rect: Some(Rect::new(
                                        0.0,
                                        source_y as f32 * source_scale,
                                        PLAYFIELD_WIDTH,
                                        (source_y as f32 + 1.0) * source_scale,
                                    )),
                                    custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, source_scale)),
                                    ..default()
                                },
                                transform: Transform::from_xyz(
                                    x,
                                    PLAYFIELD_TOP - (source_y as f32 + 0.5) * source_scale,
                                    2.65,
                                ),
                                ..default()
                            },
                            BattleCommandMarker,
                        ));
                    }
                    if let Some(priority) = priority_texture.as_ref() {
                        for x in std::iter::once(shift).chain(wrap_shift) {
                            commands.spawn((
                                SpriteBundle {
                                    texture: priority.clone(),
                                    sprite: Sprite {
                                        rect: Some(Rect::new(
                                            0.0,
                                            source_y as f32 * source_scale,
                                            PLAYFIELD_WIDTH,
                                            (source_y as f32 + 1.0) * source_scale,
                                        )),
                                        custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, source_scale)),
                                        ..default()
                                    },
                                    transform: Transform::from_xyz(
                                        x,
                                        PLAYFIELD_TOP - (source_y as f32 + 0.5) * source_scale,
                                        2.66,
                                    ),
                                    ..default()
                                },
                                BattleCommandMarker,
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn spawn_visible_battle_transition_black_tile(commands: &mut Commands, x: usize, y: usize) {
    let (tile_x, tile_y) = render_tile_playfield_position(x as i16, y as i16);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::BLACK,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_xyz(tile_x, tile_y, 2.7),
            ..default()
        },
        BattleCommandMarker,
    ));
}

fn spawn_visible_battle_transition_wedge(commands: &mut Commands, wedge_index: usize) {
    const WEDGE_1: &[i8] = &[2, 3, 5, 4, 9];
    const WEDGE_2: &[i8] = &[1, 1, 2, 2, 4, 2, 4, 2, 3];
    const WEDGE_3: &[i8] = &[2, 1, 3, 1, 4, 1, 4, 1, 4, 1, 3, 1, 2, 1, 1, 1, 1];
    const WEDGE_4: &[i8] = &[4, 1, 4, 0, 3, 1, 3, 0, 2, 1, 2, 0, 1];
    const WEDGE_5: &[i8] = &[4, 0, 3, 0, 3, 0, 2, 0, 2, 0, 1, 0, 1, 0, 1];
    // quadrant bits match battle_transition.asm: bit 0 = right, bit 1 = lower.
    const ENTRIES: [(u8, &[i8], i8, i8); 20] = [
        (0, WEDGE_1, 1, 6),
        (0, WEDGE_2, 0, 3),
        (0, WEDGE_3, 1, 0),
        (0, WEDGE_4, 5, 0),
        (0, WEDGE_5, 9, 0),
        (1, WEDGE_5, 10, 0),
        (1, WEDGE_4, 14, 0),
        (1, WEDGE_3, 18, 0),
        (1, WEDGE_2, 19, 3),
        (1, WEDGE_1, 18, 6),
        (3, WEDGE_1, 18, 11),
        (3, WEDGE_2, 19, 14),
        (3, WEDGE_3, 18, 17),
        (3, WEDGE_4, 14, 17),
        (3, WEDGE_5, 10, 17),
        (2, WEDGE_5, 9, 17),
        (2, WEDGE_4, 5, 17),
        (2, WEDGE_3, 1, 17),
        (2, WEDGE_2, 0, 14),
        (2, WEDGE_1, 1, 11),
    ];
    let Some(&(quadrant, data, mut x, mut y)) = ENTRIES.get(wedge_index) else {
        return;
    };
    let right = quadrant & 1 != 0;
    let lower = quadrant & 2 != 0;
    let mut cursor = 0;
    while cursor < data.len() {
        let width = data[cursor];
        cursor += 1;
        let row_start_x = x;
        for _ in 0..width {
            if (0..20).contains(&x) && (0..18).contains(&y) {
                let (tile_x, tile_y) = render_tile_playfield_position(i16::from(x), i16::from(y));
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::BLACK,
                            custom_size: Some(Vec2::splat(TILE_SIZE)),
                            ..default()
                        },
                        transform: Transform::from_xyz(tile_x, tile_y, 2.7),
                        ..default()
                    },
                    BattleCommandMarker,
                ));
            }
            x += if right { 1 } else { -1 };
        }
        x = row_start_x;
        y += if lower { -1 } else { 1 };
        let Some(&gap) = data.get(cursor) else {
            break;
        };
        cursor += 1;
        x += if right { -gap } else { gap };
    }
}

fn advance_visible_capture_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(animation) = runtime_shell.visible_capture_animation.as_mut() else {
        return Ok(());
    };
    if !animation.started {
        return Ok(());
    }
    animation.frame = animation.frame.saturating_add(1);
    let frame = animation.frame;
    let blocked = animation.blocked;
    let caught = animation.caught;
    let shakes = animation.animation_shakes;
    let total_frames = animation.total_frames();

    let shake_start = animation.shake_setup_frame();
    let first_check = animation.first_shake_check_frame();
    let change_dex_frame = animation.change_dex_sound_frame();
    let bounce_frame = animation.bounce_sound_frame();
    let sound = if !blocked && animation.master_ball_special_frame() == Some(frame) {
        Some("SFX_MASTER_BALL")
    } else if !blocked && frame == 52 {
        Some("SFX_BALL_POOF")
    } else if !blocked && frame == change_dex_frame {
        Some("SFX_CHANGE_DEX_MODE")
    } else if !blocked && frame == bounce_frame {
        Some("SFX_BALL_BOUNCE")
    } else if !blocked && frame >= first_check && (frame - shake_start) % 48 == 0 {
        let check = ((frame - shake_start) / 48) as u8;
        if (!caught && check <= shakes) || (caught && check < shakes) {
            Some("SFX_BALL_WOBBLE")
        } else if !caught && check == shakes.saturating_add(1) {
            Some("SFX_BALL_POOF")
        } else {
            None
        }
    } else {
        None
    };
    if let Some(sound) = sound {
        queue_visible_shell_sound_effect(runtime_shell, sound)?;
    }
    if frame >= total_frames {
        if caught {
            // Text_BallCaught's sound_caught_mon command runs as the Gotcha
            // page becomes visible, after the ball animation has completed.
            queue_visible_shell_sound_effect(runtime_shell, "SFX_CAUGHT_MON")?;
            if let Some(animation) = runtime_shell.visible_capture_animation.as_mut() {
                animation.started = false;
                animation.complete = true;
            }
        } else {
            runtime_shell.visible_capture_animation = None;
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_move_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (due_sounds, due_cries, player_move) = {
        let Some(animation) = runtime_shell.visible_move_animations.front_mut() else {
            return Ok(());
        };
        if !animation.started {
            return Ok(());
        }
        animation.frame = animation.frame.saturating_add(1);
        let start = animation.next_sound_event;
        while animation
            .sound_events
            .get(animation.next_sound_event)
            .is_some_and(|(frame, _)| *frame <= animation.frame)
        {
            animation.next_sound_event += 1;
        }
        let sounds = animation.sound_events[start..animation.next_sound_event]
            .iter()
            .map(|(_, sound)| sound.clone())
            .collect::<Vec<_>>();
        let cry_start = animation.next_cry_event;
        while animation
            .cry_events
            .get(animation.next_cry_event)
            .is_some_and(|(frame, _)| *frame <= animation.frame)
        {
            animation.next_cry_event += 1;
        }
        let cries = animation.cry_events[cry_start..animation.next_cry_event]
            .iter()
            .map(|(_, selector)| *selector)
            .collect::<Vec<_>>();
        (sounds, cries, animation.player_move)
    };
    for sound in due_sounds {
        queue_visible_shell_sound_effect(runtime_shell, &sound)?;
    }
    if !due_cries.is_empty() {
        let species = visible_move_animation_user_species(runtime_shell, player_move)
            .context("battle animation cry has no visible user species")?;
        for selector in due_cries {
            queue_visible_pokemon_animation_cry(runtime_shell, &species, selector)?;
        }
    }
    let finished_trigger = {
        let Some(animation) = runtime_shell.visible_move_animations.front_mut() else {
            return Ok(());
        };
        (animation.frame >= animation.total_frames).then(|| animation.trigger_message.clone())
    };
    if let Some(trigger_message) = finished_trigger {
        runtime_shell.visible_move_animations.pop_front();
        let completed_before_trigger_message = runtime_shell
            .battle_messages
            .front()
            .is_some_and(|message| message == &trigger_message);
        if completed_before_trigger_message {
            let continues_same_command =
                runtime_shell
                    .visible_move_animations
                    .front()
                    .is_some_and(|animation| {
                        !animation.started && animation.trigger_message == trigger_message
                    });
            let mut applied_scene = false;
            if let Some(index) = runtime_shell
                .pending_battle_scenes_after_message
                .iter()
                .position(|(trigger, _)| trigger == &trigger_message)
            {
                let (_, scene) = runtime_shell
                    .pending_battle_scenes_after_message
                    .remove(index)
                    .unwrap();
                retarget_visible_battle_hp_tween(runtime_shell, &scene);
                runtime_shell.battle_message_scene = Some(scene);
                applied_scene = true;
            }
            if continues_same_command {
                let next = runtime_shell.visible_move_animations.front_mut().unwrap();
                if applied_scene {
                    next.waiting_for_hp = true;
                } else {
                    next.started = true;
                }
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        let continues_same_command =
            runtime_shell
                .visible_move_animations
                .front()
                .is_some_and(|animation| {
                    !animation.started && animation.trigger_message == trigger_message
                });
        if continues_same_command {
            let intermediate_scene = runtime_shell
                .pending_battle_scenes_after_message
                .iter()
                .position(|(trigger, _)| trigger == &trigger_message)
                .and_then(|index| {
                    runtime_shell
                        .pending_battle_scenes_after_message
                        .remove(index)
                        .map(|(_, scene)| scene)
                });
            if let Some(scene) = intermediate_scene {
                retarget_visible_battle_hp_tween(runtime_shell, &scene);
                runtime_shell.battle_message_scene = Some(scene);
                runtime_shell
                    .visible_move_animations
                    .front_mut()
                    .unwrap()
                    .waiting_for_hp = true;
            } else {
                runtime_shell
                    .visible_move_animations
                    .front_mut()
                    .unwrap()
                    .started = true;
            }
        } else {
            runtime_shell.battle_message_scenes.pop_front();
            if let Some(scene) = runtime_shell.battle_message_scenes.front().cloned() {
                retarget_visible_battle_hp_tween(runtime_shell, &scene);
                runtime_shell.battle_message_scene = Some(scene);
            }
            if let Some(index) = runtime_shell
                .pending_battle_scenes_after_message
                .iter()
                .position(|(trigger, _)| trigger == &trigger_message)
            {
                let (_, scene) = runtime_shell
                    .pending_battle_scenes_after_message
                    .remove(index)
                    .unwrap();
                if runtime_shell.battle_message_scenes.is_empty() {
                    retarget_visible_battle_hp_tween(runtime_shell, &scene);
                    runtime_shell.battle_message_scene = Some(scene);
                }
            }
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_move_animation_user_species(
    runtime_shell: &BevyRuntimeShell,
    player_move: bool,
) -> Option<String> {
    let scene = runtime_shell.battle_message_scene.as_deref()?;
    let battle = scene.battle.as_ref()?;
    if !player_move {
        return Some(
            battle
                .enemy_transformed_species
                .clone()
                .unwrap_or_else(|| battle.enemy_pokemon.species.id.clone()),
        );
    }
    let active_index = battle.active_player_party_index?;
    let slot = scene
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_index)?;
    Some(
        battle
            .player_transformed_species
            .clone()
            .unwrap_or_else(|| slot.pokemon.species.id.clone()),
    )
}

fn advance_visible_send_out_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(animation) = runtime_shell.visible_send_out_animation.as_mut() else {
        return Ok(());
    };
    animation.frame = animation.frame.saturating_add(1);
    let shiny_sound = animation.shiny
        && animation.frame >= VisibleSendOutAnimation::NORMAL_FRAMES
        && animation.frame < VisibleSendOutAnimation::NORMAL_FRAMES + 32
        && (animation.frame - VisibleSendOutAnimation::NORMAL_FRAMES) % 4 == 0;
    let finished = animation.frame >= animation.total_frames();
    let side = animation.side;
    if shiny_sound {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_SHINE")?;
    }
    if finished {
        runtime_shell.visible_send_out_animation = None;
        if side == crate::core::battle::turn::BattleSide::Enemy {
            let snapshot = runtime_shell.shell.snapshot()?;
            let speed = snapshot
                .battle
                .as_ref()
                .map(|battle| match battle.kind {
                    RuntimeBattleKind::Trainer { .. } => 4,
                    RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => 0,
                })
                .unwrap_or(0);
            start_visible_enemy_frontpic_animation(runtime_shell, speed)?;
        }
        if let Some((species_id, reason, _)) = runtime_shell
            .pending_battle_cries_after_messages
            .pop_front()
        {
            queue_visible_pokemon_cry(runtime_shell, &species_id, &reason)?;
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_send_out_side_is_shiny(
    runtime_shell: &BevyRuntimeShell,
    side: crate::core::battle::turn::BattleSide,
) -> Result<bool> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return Ok(false);
    };
    let pokemon = match side {
        crate::core::battle::turn::BattleSide::Enemy => &battle.enemy_pokemon,
        crate::core::battle::turn::BattleSide::Player => {
            let Some(active_index) = battle.active_player_party_index else {
                return Ok(false);
            };
            let Some(slot) = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == active_index)
            else {
                return Ok(false);
            };
            &slot.pokemon
        }
    };
    Ok(visible_pokemon_is_shiny(pokemon))
}

fn start_visible_enemy_frontpic_animation(
    runtime_shell: &mut BevyRuntimeShell,
    speed: u16,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let species_id = snapshot
        .battle
        .as_ref()
        .context("enemy frontpic animation requires an active battle")?
        .enemy_pokemon
        .species
        .id
        .clone();
    snapshot
        .presentation
        .pokemon_frontpic_anim
        .get(&species_id)
        .with_context(|| format!("missing exported frontpic animation for {species_id}"))?;
    runtime_shell.visible_frontpic_animation = Some(VisibleFrontpicAnimation {
        species_id,
        speed,
        pointer: 0,
        repeat: 0,
        wait: 0,
        frame: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_frontpic_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mut animation) = runtime_shell.visible_frontpic_animation.take() else {
        return Ok(());
    };
    let snapshot = runtime_shell.shell.snapshot()?;
    let program = snapshot
        .presentation
        .pokemon_frontpic_anim
        .get(&animation.species_id)
        .with_context(|| {
            format!(
                "missing exported frontpic animation for {}",
                animation.species_id
            )
        })?
        .clone();
    if animation.wait > 0 {
        animation.wait -= 1;
        runtime_shell.visible_frontpic_animation = Some(animation);
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let mut guard = 0_usize;
    while guard < program.commands.len().saturating_add(4) {
        let Some(command) = program.commands.get(animation.pointer) else {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        };
        animation.pointer += 1;
        guard += 1;
        match command.kind.as_str() {
            "setrepeat" => animation.repeat = command.count.unwrap_or(0),
            "dorepeat" => {
                if animation.repeat > 0 {
                    animation.repeat -= 1;
                    if animation.repeat > 0 {
                        animation.pointer = usize::from(command.target.unwrap_or(0));
                    }
                }
            }
            "endanim" => {
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            "frame" => {
                animation.frame = command.frame.unwrap_or(0);
                let duration = command.duration.unwrap_or(0);
                animation.wait =
                    duration.saturating_add(duration.saturating_mul(animation.speed) / 16);
                runtime_shell.visible_frontpic_animation = Some(animation);
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            other => anyhow::bail!("unknown frontpic animation command {other}"),
        }
    }
    anyhow::bail!("frontpic animation exceeded command guard")
}

fn advance_visible_fishing_animation(runtime_shell: &mut BevyRuntimeShell) {
    let Some(animation) = runtime_shell.visible_fishing_animation.as_mut() else {
        return;
    };
    let mut result_text = None;
    match animation.phase {
        VisibleFishingPhase::Cast => {
            animation.frame = animation.frame.saturating_add(1);
            if animation.frame >= 40 {
                animation.frame = 0;
                if animation.bite {
                    animation.phase = VisibleFishingPhase::Hook;
                } else {
                    animation.phase = VisibleFishingPhase::AwaitText;
                    result_text = Some("Not even a nibble!".to_string());
                }
            }
        }
        VisibleFishingPhase::Hook => {
            // Four fish_got_bite movement commands, each held for one
            // ordinary eight-frame overworld movement cadence. Facing Up's
            // source movement adds `step_sleep 1` before `show_emote`.
            animation.frame = animation.frame.saturating_add(1);
            let hook_frames = if animation.facing_up { 33 } else { 32 };
            if animation.frame >= hook_frames {
                animation.frame = 0;
                animation.phase = VisibleFishingPhase::Pause;
            }
        }
        VisibleFishingPhase::Pause => {
            animation.frame = animation.frame.saturating_add(1);
            if animation.frame >= 40 {
                animation.frame = 0;
                animation.phase = VisibleFishingPhase::AwaitText;
                result_text = Some("Oh!\nA bite!".to_string());
            }
        }
        VisibleFishingPhase::AwaitText => return,
    }
    if let Some(text) = result_text {
        runtime_shell.field_notice = Some(text);
        runtime_shell.pending_field_battle_entry = runtime_shell
            .visible_fishing_animation
            .as_ref()
            .is_some_and(|animation| animation.starts_battle);
    }
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn advance_visible_trainer_exit_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(animation) = runtime_shell.visible_trainer_exit_animation.as_mut() else {
        return Ok(());
    };
    animation.frame = animation.frame.saturating_add(1);
    if animation.frame >= animation.total_frames() {
        let start_send_out = animation.send_out_after;
        let side = animation.side;
        runtime_shell.visible_trainer_exit_animation = None;
        if start_send_out {
            let shiny = visible_send_out_side_is_shiny(runtime_shell, side)?;
            runtime_shell.visible_send_out_animation = Some(VisibleSendOutAnimation {
                side,
                frame: 0,
                shiny,
            });
            queue_visible_shell_sound_effect(runtime_shell, "SFX_BALL_POOF")?;
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn reset_visible_battle_item_cursors(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.battle_party_action_cursor = None;
    runtime_shell.battle_party_summary_open = false;
}

fn prepare_visible_local_link_descriptor(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let session_id = format!("bevy-local-{}", snapshot.state_checksum.frame());
    let descriptor = visible_local_link_descriptor(runtime_shell, session_id.clone())?;
    let checkpoint = descriptor.save_checkpoint.checkpoint();
    if checkpoint.summary().state_hash() != descriptor.checksum.hash()
        || checkpoint.checksum().hash() != descriptor.checksum.hash()
        || checkpoint.summary().state_frame() != descriptor.checksum.frame()
        || checkpoint.checksum().frame() != descriptor.checksum.frame()
    {
        anyhow::bail!(
            "runtime link descriptor checkpoint does not match current state checksum: summary frame/hash {} {:#010x}, checkpoint frame/hash {} {:#010x}, current frame/hash {} {:#010x}",
            checkpoint.summary().state_frame(),
            checkpoint.summary().state_hash(),
            checkpoint.checksum().frame(),
            checkpoint.checksum().hash(),
            descriptor.checksum.frame(),
            descriptor.checksum.hash()
        );
    }
    let journal = runtime_shell.shell.local_input_journal(
        &descriptor,
        descriptor.checksum.clone(),
        std::iter::empty(),
    )?;
    let journal_bytes = journal.journal.canonical_bytes()?;
    let journal_frame_count = journal.journal.frames().len();
    let journal_message = runtime_shell.shell.input_journal_message(journal.clone())?;
    let journal_message_bytes = encode_link_message_bytes(&journal_message)?;
    let save_resume_message = runtime_shell.shell.save_resume_replay_message(
        &descriptor,
        journal,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    let save_resume_message_bytes = encode_link_message_bytes(&save_resume_message)?;
    let retained_replay = visible_retained_save_resume_replay_bundle(runtime_shell)?;
    let retained_direct_replay_message =
        LinkMessage::DeterministicReplay(retained_replay.replay().clone());
    let retained_direct_replay_message_bytes =
        encode_link_message_bytes(&retained_direct_replay_message)?;
    let retained_replay_message = LinkMessage::SaveResumeReplay(retained_replay.clone());
    let retained_replay_message_bytes = encode_link_message_bytes(&retained_replay_message)?;
    let retained_input_message_bytes = encode_retained_input_messages(runtime_shell)?;
    let retained_battle_action_message_bytes =
        encode_retained_battle_action_messages(runtime_shell)?;
    let retained_menu_choice_message_bytes = encode_retained_menu_choice_messages(runtime_shell)?;
    let retained_menu_result_message_bytes = encode_retained_menu_result_messages(runtime_shell)?;
    let retained_runtime_command_message_bytes =
        encode_retained_runtime_command_messages(runtime_shell)?;
    let retained_runtime_result_message_bytes =
        encode_retained_runtime_result_messages(runtime_shell)?;
    let retained_state_hash_message_bytes = encode_retained_state_hash_messages(runtime_shell)?;
    let retained_journal_frames = retained_replay
        .replay()
        .input_journal()
        .journal()
        .frames()
        .len();
    let deterministic_session_checkpoint =
        required_visible_deterministic_session_checkpoint(runtime_shell)?;
    let retained_session_id = deterministic_session_checkpoint
        .session()
        .session_id()
        .to_string();
    let retained_checkpoint_frame = deterministic_session_checkpoint
        .checkpoint()
        .summary()
        .state_frame();
    let retained_checkpoint_hash = deterministic_session_checkpoint
        .checkpoint()
        .summary()
        .state_hash();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "link:descriptor:{}:{}:{}:{}:{}:{}:{:#010x}",
            descriptor.session.session_id(),
            descriptor.local_player.id(),
            descriptor.session.modpack().id(),
            descriptor.session.modpack().hash(),
            descriptor.session.pack_content_hash(),
            descriptor.checksum.frame(),
            descriptor.checksum.hash()
        ),
    )?;
    runtime_shell.last_audio_events.push(format!(
            "link descriptor session={} player={} checksum_frame={} checksum_hash={:#010x} checkpoint_frame={} checkpoint_hash={:#010x} journal_frames={} journal_bytes={} journal_msg_bytes={} save_resume_msg_bytes={} retained_session={} retained_input_start={} retained_input_start_hash={:#010x} retained_checkpoint_frame={} retained_checkpoint_hash={:#010x} retained_inputs={} retained_journal_frames={} retained_battle_actions={} retained_menu_results={} retained_runtime_commands={} retained_runtime_results={} retained_state_hash_msg_bytes={} retained_direct_replay_msg_bytes={} retained_replay_msg_bytes={} retained_input_msg_bytes={} retained_battle_action_msg_bytes={} retained_menu_choice_msg_bytes={} retained_menu_result_msg_bytes={} retained_runtime_command_msg_bytes={} retained_runtime_result_msg_bytes={}",
        session_id,
        descriptor.local_player.id(),
        descriptor.checksum.frame(),
        descriptor.checksum.hash(),
        descriptor.save_checkpoint.checkpoint().summary().state_frame(),
        descriptor.save_checkpoint.checkpoint().summary().state_hash(),
        journal_frame_count,
        journal_bytes.len(),
        journal_message_bytes.len(),
        save_resume_message_bytes.len(),
        retained_session_id,
        runtime_shell.deterministic_session_start.frame(),
        runtime_shell.deterministic_session_start.hash(),
        retained_checkpoint_frame,
        retained_checkpoint_hash,
        runtime_shell.deterministic_input_frames.len(),
        retained_journal_frames,
        runtime_shell.deterministic_battle_actions.len(),
        runtime_shell.deterministic_menu_results.len(),
        runtime_shell.shell.retained_runtime_commands().len(),
        runtime_shell.shell.retained_runtime_results().len(),
        retained_state_hash_message_bytes,
        retained_direct_replay_message_bytes.len(),
        retained_replay_message_bytes.len(),
        retained_input_message_bytes,
        retained_battle_action_message_bytes,
        retained_menu_choice_message_bytes,
        retained_menu_result_message_bytes,
        retained_runtime_command_message_bytes,
        retained_runtime_result_message_bytes
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn encode_retained_input_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let terminal_frame = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained input stream")?
        .frame();
    let input_masks = retained_input_masks_by_frame(
        runtime_shell,
        runtime_shell.deterministic_session_start.frame(),
        terminal_frame,
    )?;
    let mut byte_count = 0usize;
    for input in retained_input_frames_from_masks(
        runtime_shell.deterministic_session_start.frame(),
        terminal_frame,
        &input_masks,
    )? {
        let message = LinkMessage::Input(input.clone());
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained input message byte count overflow")?;
    }
    Ok(byte_count)
}

fn retained_input_masks_by_frame(
    runtime_shell: &BevyRuntimeShell,
    start_frame: u64,
    terminal_frame: u64,
) -> Result<BTreeMap<u64, u8>> {
    let mut masks = BTreeMap::new();
    for input in &runtime_shell.deterministic_input_frames {
        if input.player_id() != LOCAL_PLAYER_ID {
            anyhow::bail!(
                "retained input has player {}, expected {}",
                input.player_id(),
                LOCAL_PLAYER_ID
            );
        }
        if input.frame() < start_frame || input.frame() >= terminal_frame {
            anyhow::bail!(
                "retained input frame {} is outside session frame range {}..{}",
                input.frame(),
                start_frame,
                terminal_frame
            );
        }
        if masks.insert(input.frame(), input.joypad_mask()).is_some() {
            anyhow::bail!(
                "retained input frame {} appears more than once",
                input.frame()
            );
        }
    }
    Ok(masks)
}

fn retained_input_frames_from_masks(
    start_frame: u64,
    terminal_frame: u64,
    input_masks: &BTreeMap<u64, u8>,
) -> Result<Vec<PlayerInputFrame>> {
    if terminal_frame < start_frame {
        anyhow::bail!(
            "retained input terminal frame {terminal_frame} is before start frame {start_frame}"
        );
    }
    let mut frames = Vec::with_capacity((terminal_frame - start_frame) as usize);
    for frame in start_frame..terminal_frame {
        frames.push(
            PlayerInputFrame::new(
                LOCAL_PLAYER_ID,
                Frame(frame),
                input_masks.get(&frame).copied().unwrap_or(0),
            )
            .context("build retained input message frame")?,
        );
    }
    Ok(frames)
}

fn retained_lockstep_frames_from_masks(
    start_frame: u64,
    terminal_frame: u64,
    input_masks: &BTreeMap<u64, u8>,
) -> Result<Vec<LockstepFrame>> {
    retained_input_frames_from_masks(start_frame, terminal_frame, input_masks)?
        .into_iter()
        .map(|input| {
            LockstepFrame::new(
                input.frame(),
                BTreeMap::from([(LOCAL_PLAYER_ID, input.joypad_mask())]),
            )
            .context("build retained deterministic lockstep frame")
        })
        .collect()
}

fn encode_retained_state_hash_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let start = required_visible_deterministic_session_checkpoint(runtime_shell)?
        .checkpoint()
        .checksum()
        .clone();
    let current = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained state hash stream")?;
    validate_retained_state_hash_stream(&start, &current)?;
    let mut byte_count = 0usize;
    for checksum in [start, current] {
        let message = LinkMessage::StateHash(StateChecksumFrame::new(
            checksum.player_id(),
            Frame(checksum.frame()),
            checksum.hash(),
        ));
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained state hash message byte count overflow")?;
    }
    Ok(byte_count)
}

fn validate_retained_state_hash_stream(
    start: &StateChecksumFrame,
    current: &StateChecksumFrame,
) -> Result<()> {
    if start.player_id() != LOCAL_PLAYER_ID || current.player_id() != LOCAL_PLAYER_ID {
        anyhow::bail!(
            "retained state hash players {}/{} do not match local player {}",
            start.player_id(),
            current.player_id(),
            LOCAL_PLAYER_ID
        );
    }
    if current.frame() < start.frame() {
        anyhow::bail!(
            "retained state hash current frame {} is before start frame {}",
            current.frame(),
            start.frame()
        );
    }
    Ok(())
}

fn encode_retained_battle_action_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let mut byte_count = 0usize;
    let start_frame = runtime_shell.deterministic_session_start.frame();
    let terminal_frame = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained battle action stream")?
        .frame();
    validate_retained_battle_actions(runtime_shell, start_frame, terminal_frame)?;
    for action in &runtime_shell.deterministic_battle_actions {
        let message = LinkMessage::BattleAction(action.clone());
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained battle action message byte count overflow")?;
    }
    Ok(byte_count)
}

fn validate_retained_battle_actions(
    runtime_shell: &BevyRuntimeShell,
    start_frame: u64,
    terminal_frame: u64,
) -> Result<()> {
    let mut previous_turn = None;
    for action in &runtime_shell.deterministic_battle_actions {
        action
            .validate()
            .context("validate retained battle action before replay")?;
        if action.player_id() != LOCAL_PLAYER_ID {
            anyhow::bail!(
                "retained battle action has player {}, expected {}",
                action.player_id(),
                LOCAL_PLAYER_ID
            );
        }
        if action.turn() < start_frame || action.turn() > terminal_frame {
            anyhow::bail!(
                "retained battle action turn {} is outside session frame range {}..={}",
                action.turn(),
                start_frame,
                terminal_frame
            );
        }
        validate_retained_battle_action_order(previous_turn, action.turn())?;
        previous_turn = Some(action.turn());
    }
    Ok(())
}

fn validate_retained_battle_action_order(previous_turn: Option<u64>, turn: u64) -> Result<()> {
    if let Some(previous) = previous_turn {
        if turn <= previous {
            anyhow::bail!(
                "retained battle action turn {} is not strictly after previous turn {}",
                turn,
                previous
            );
        }
    }
    Ok(())
}

fn encode_retained_menu_choice_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let start_frame = runtime_shell.deterministic_session_start.frame();
    let terminal_frame = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained menu choice stream")?
        .frame();
    validate_retained_menu_results(runtime_shell, start_frame, terminal_frame)?;
    let mut byte_count = 0usize;
    for result in &runtime_shell.deterministic_menu_results {
        let choice = result.choice();
        let message = LinkMessage::MenuChoice(choice.clone());
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained menu choice message byte count overflow")?;
    }
    Ok(byte_count)
}

fn encode_retained_menu_result_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let start_frame = runtime_shell.deterministic_session_start.frame();
    let terminal_frame = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained menu result stream")?
        .frame();
    validate_retained_menu_results(runtime_shell, start_frame, terminal_frame)?;
    let mut byte_count = 0usize;
    for result in &runtime_shell.deterministic_menu_results {
        let message = LinkMessage::MenuChoiceResult(result.clone());
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained menu result message byte count overflow")?;
    }
    Ok(byte_count)
}

fn validate_retained_menu_results(
    runtime_shell: &BevyRuntimeShell,
    start_frame: u64,
    terminal_frame: u64,
) -> Result<()> {
    let mut previous_choice_frame = None;
    for result in &runtime_shell.deterministic_menu_results {
        result
            .validate()
            .context("validate retained menu result before replay")?;
        if result.choice().player_id() != LOCAL_PLAYER_ID
            || result.checksum().player_id() != LOCAL_PLAYER_ID
        {
            anyhow::bail!(
                "retained menu result has choice/checksum players {}/{}, expected {}",
                result.choice().player_id(),
                result.checksum().player_id(),
                LOCAL_PLAYER_ID
            );
        }
        let choice_frame = result.choice().frame();
        let checksum_frame = result.checksum().frame();
        validate_retained_frame_pair(
            "retained menu result",
            choice_frame,
            checksum_frame,
            start_frame,
            terminal_frame,
        )?;
        validate_retained_menu_choice_order(previous_choice_frame, choice_frame)?;
        previous_choice_frame = Some(choice_frame);
    }
    Ok(())
}

fn validate_retained_menu_choice_order(
    previous_choice_frame: Option<u64>,
    choice_frame: u64,
) -> Result<()> {
    if let Some(previous) = previous_choice_frame {
        if choice_frame <= previous {
            anyhow::bail!(
                "retained menu choice frame {} is not strictly after previous choice frame {}",
                choice_frame,
                previous
            );
        }
    }
    Ok(())
}

fn validate_retained_frame_pair(
    label: &str,
    first_frame: u64,
    second_frame: u64,
    start_frame: u64,
    terminal_frame: u64,
) -> Result<()> {
    if first_frame < start_frame
        || first_frame > terminal_frame
        || second_frame < start_frame
        || second_frame > terminal_frame
    {
        anyhow::bail!(
            "{label} frames first={} second={} outside session frame range {}..={}",
            first_frame,
            second_frame,
            start_frame,
            terminal_frame
        );
    }
    Ok(())
}

fn encode_retained_runtime_command_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let start_frame = runtime_shell.deterministic_session_start.frame();
    let terminal_frame = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained runtime command stream")?
        .frame();
    validate_retained_runtime_commands(runtime_shell, start_frame, terminal_frame)?;
    let mut byte_count = 0usize;
    for command in visible_retained_runtime_command_frames(
        runtime_shell,
        required_visible_deterministic_session_checkpoint(runtime_shell)?,
    )? {
        let message = LinkMessage::SessionRuntimeCommand(command);
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained runtime command message byte count overflow")?;
    }
    Ok(byte_count)
}

fn validate_retained_runtime_commands(
    runtime_shell: &BevyRuntimeShell,
    start_frame: u64,
    terminal_frame: u64,
) -> Result<()> {
    let mut previous_sequence = None;
    for command in runtime_shell.shell.retained_runtime_commands() {
        command
            .validate()
            .context("validate retained runtime command before replay")?;
        if command.player_id() != LOCAL_PLAYER_ID {
            anyhow::bail!(
                "retained runtime command has player {}, expected {}",
                command.player_id(),
                LOCAL_PLAYER_ID
            );
        }
        if command.expected_state().frame() < start_frame
            || command.expected_state().frame() > terminal_frame
        {
            anyhow::bail!(
                "retained runtime command sequence {} expected frame {} outside session frame range {}..={}",
                command.sequence(),
                command.expected_state().frame(),
                start_frame,
                terminal_frame
            );
        }
        if let Some(previous) = previous_sequence {
            if command.sequence() <= previous {
                anyhow::bail!(
                    "retained runtime command sequence {} is not strictly after previous sequence {}",
                    command.sequence(),
                    previous
                );
            }
        }
        previous_sequence = Some(command.sequence());
    }
    Ok(())
}

fn encode_retained_runtime_result_messages(runtime_shell: &BevyRuntimeShell) -> Result<usize> {
    let start_frame = runtime_shell.deterministic_session_start.frame();
    let terminal_frame = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained runtime command result stream")?
        .frame();
    validate_retained_runtime_results(runtime_shell, start_frame, terminal_frame)?;
    let mut byte_count = 0usize;
    for result in visible_retained_runtime_result_frames(
        runtime_shell,
        required_visible_deterministic_session_checkpoint(runtime_shell)?,
    )? {
        let message = LinkMessage::SessionRuntimeCommandResult(result);
        byte_count = byte_count
            .checked_add(encode_link_message_bytes(&message)?.len())
            .context("retained runtime command result message byte count overflow")?;
    }
    Ok(byte_count)
}

fn validate_retained_runtime_results(
    runtime_shell: &BevyRuntimeShell,
    start_frame: u64,
    terminal_frame: u64,
) -> Result<()> {
    let commands = runtime_shell.shell.retained_runtime_commands();
    let results = runtime_shell.shell.retained_runtime_results();
    if results.len() != commands.len() {
        anyhow::bail!(
            "retained runtime command/result count mismatch: commands={} results={}",
            commands.len(),
            results.len()
        );
    }
    let mut previous_sequence = None;
    for (index, result) in results.iter().enumerate() {
        result
            .validate()
            .context("validate retained runtime command result before replay")?;
        let command = &commands[index];
        if result.request() != command {
            anyhow::bail!(
                "retained runtime command result at index {} is for sequence {}, expected command sequence {}",
                index,
                result.request().sequence(),
                command.sequence()
            );
        }
        if result.request().player_id() != LOCAL_PLAYER_ID
            || result.checksum().player_id() != LOCAL_PLAYER_ID
        {
            anyhow::bail!(
                "retained runtime command result has request/checksum players {}/{}, expected {}",
                result.request().player_id(),
                result.checksum().player_id(),
                LOCAL_PLAYER_ID
            );
        }
        let request_frame = result.request().expected_state().frame();
        let checksum_frame = result.checksum().frame();
        validate_retained_frame_pair(
            &format!(
                "retained runtime command result sequence {}",
                result.request().sequence()
            ),
            request_frame,
            checksum_frame,
            start_frame,
            terminal_frame,
        )?;
        if let Some(previous) = previous_sequence {
            if result.request().sequence() <= previous {
                anyhow::bail!(
                    "retained runtime command result sequence {} is not strictly after previous sequence {}",
                    result.request().sequence(),
                    previous
                );
            }
        }
        previous_sequence = Some(result.request().sequence());
    }
    Ok(())
}

fn visible_retained_save_resume_replay_bundle(
    runtime_shell: &BevyRuntimeShell,
) -> Result<SaveResumeReplayBundle> {
    let checkpoint = required_visible_deterministic_session_checkpoint(runtime_shell)?.clone();
    let start_checksum = checkpoint.checkpoint().checksum().clone();
    let terminal_checksum = runtime_shell
        .shell
        .state_checksum_frame(LOCAL_PLAYER_ID)
        .context("checksum current runtime state for retained deterministic replay")?;
    let start_frame = start_checksum.frame();
    let terminal_frame = terminal_checksum.frame();
    terminal_frame.checked_sub(start_frame).with_context(|| {
        format!(
            "retained deterministic replay terminal frame {terminal_frame} is before start frame {start_frame}"
        )
    })?;
    let input_masks = retained_input_masks_by_frame(runtime_shell, start_frame, terminal_frame)?;
    validate_retained_runtime_commands(runtime_shell, start_frame, terminal_frame)?;
    validate_retained_runtime_results(runtime_shell, start_frame, terminal_frame)?;
    validate_retained_menu_results(runtime_shell, start_frame, terminal_frame)?;
    let frames = retained_lockstep_frames_from_masks(start_frame, terminal_frame, &input_masks)?;

    let journal = DeterministicInputJournal::new(
        checkpoint.session().clone(),
        [LOCAL_PLAYER_ID],
        start_checksum,
        terminal_checksum.clone(),
        frames,
    )
    .context("build retained deterministic input journal")?;
    let journal_frame = DeterministicInputJournalFrame::new(journal)
        .context("fingerprint retained deterministic input journal")?;
    let runtime_commands = visible_retained_runtime_command_frames(runtime_shell, &checkpoint)?;
    let runtime_results = visible_retained_runtime_result_frames(runtime_shell, &checkpoint)?;
    let replay = DeterministicReplayBundle::new(
        journal_frame,
        runtime_commands,
        runtime_results,
        runtime_shell
            .deterministic_menu_results
            .iter()
            .cloned()
            .collect(),
        terminal_checksum,
    )
    .context("build retained deterministic replay bundle")?;
    crate::validate_deterministic_replay_runtime_authority(&replay, LOCAL_PLAYER_ID)
        .context("validate retained runtime replay command authority")?;
    SaveResumeReplayBundle::new(checkpoint, replay)
        .context("build retained save-resume replay bundle")
}

fn visible_retained_runtime_command_frames(
    runtime_shell: &BevyRuntimeShell,
    checkpoint: &SessionSaveCheckpointFrame,
) -> Result<Vec<SessionRuntimeCommandFrame>> {
    runtime_shell
        .shell
        .retained_runtime_commands()
        .iter()
        .cloned()
        .map(|command| {
            SessionRuntimeCommandFrame::new(checkpoint.session().clone(), command)
                .context("bind retained runtime command to deterministic session")
        })
        .collect()
}

fn visible_retained_runtime_result_frames(
    runtime_shell: &BevyRuntimeShell,
    checkpoint: &SessionSaveCheckpointFrame,
) -> Result<Vec<SessionRuntimeCommandResultFrame>> {
    runtime_shell
        .shell
        .retained_runtime_results()
        .iter()
        .cloned()
        .map(|result| {
            SessionRuntimeCommandResultFrame::new(checkpoint.session().clone(), result)
                .context("bind retained runtime command result to deterministic session")
        })
        .collect()
}

fn switch_visible_next_pc_box(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    switch_visible_pc_box_by_delta(runtime_shell, 1)
}

fn switch_visible_pc_box_by_delta(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    if runtime_shell.bill_pc_move_open {
        return switch_visible_pc_move_container(runtime_shell, delta);
    }
    ensure_no_visible_special_boundary(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let box_count = crate::core::models::MAX_PC_BOXES;
    let next_box = wrapped_index(snapshot.storage.current_pc_box, box_count, delta);
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "pc:switch_box:{}:{}",
            snapshot.storage.current_pc_box, next_box
        ),
    )?;
    let switched = runtime_shell.shell.switch_current_pc_box(next_box)?;
    runtime_shell.storage_cursor = Some(MenuCursor {
        surface_id: storage_cursor_surface_id(switched.box_index_after),
        option_index: 0,
    });
    runtime_shell.last_audio_events.push(format!(
        "pc box switch {}->{} checksum={:?}",
        switched.box_index_before, switched.box_index_after, switched.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "PC BOX {} -> {}",
            switched.box_index_before, switched.box_index_after
        ),
    );
    Ok(())
}

fn switch_visible_pc_move_container(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let container_count = crate::core::models::MAX_PC_BOXES + 1;
    let current = if runtime_shell.bill_pc_move_party_open {
        0
    } else {
        snapshot.storage.current_pc_box + 1
    };
    let next = wrapped_index(current, container_count, delta);
    if next == 0 {
        runtime_shell.bill_pc_move_party_open = true;
        runtime_shell.storage_cursor = Some(MenuCursor {
            surface_id: pc_move_party_surface_id().to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "PARTY");
        return Ok(());
    }
    let switched = runtime_shell.shell.switch_current_pc_box(next - 1)?;
    runtime_shell.bill_pc_move_party_open = false;
    runtime_shell.storage_cursor = Some(MenuCursor {
        surface_id: storage_cursor_surface_id(switched.box_index_after),
        option_index: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(
        runtime_shell,
        format!("BOX {}", switched.box_index_after + 1),
    );
    Ok(())
}

fn ensure_no_visible_special_boundary(runtime_shell: &BevyRuntimeShell) -> Result<()> {
    if let Some(boundary) = runtime_shell.special_boundary.as_ref() {
        anyhow::bail!(
            "active special boundary {} blocks this action",
            boundary.label
        );
    }
    Ok(())
}

fn deposit_visible_party_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    ensure_no_visible_special_boundary(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if snapshot.storage.party_count <= 1 {
        record_visible_runtime_action(
            runtime_shell,
            format!("pc:deposit_party:{party_index}:last_pokemon"),
        )?;
        runtime_shell
            .last_audio_events
            .push("cannot deposit the last Pokemon".to_string());
        begin_visible_pc_transfer_refusal(
            runtime_shell,
            VisiblePcTransferKind::Deposit,
            snapshot.storage.current_pc_box,
            "It's your last <PK><MN>!",
        )?;
        set_shell_action_status(runtime_shell, "CAN'T DEPOSIT LAST POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let has_other_usable_pokemon = snapshot.party.slots.iter().any(|other| {
        other.index != party_index
            && other.pokemon.hp > 0
            && !other.pokemon.is_egg
            && other.pokemon.species.id != "EGG"
    });
    if !has_other_usable_pokemon {
        record_visible_runtime_action(
            runtime_shell,
            format!("pc:deposit_party:{party_index}:no_more_usable_pokemon"),
        )?;
        begin_visible_pc_transfer_refusal(
            runtime_shell,
            VisiblePcTransferKind::Deposit,
            snapshot.storage.current_pc_box,
            "No more usable <PK><MN>!",
        )?;
        set_shell_action_status(runtime_shell, "NO MORE USABLE POKEMON");
        return Ok(());
    }
    if slot
        .pokemon
        .item
        .as_deref()
        .is_some_and(crate::core::models::item::is_mail_item_id)
    {
        record_visible_runtime_action(
            runtime_shell,
            format!("pc:deposit_party:{party_index}:remove_mail"),
        )?;
        begin_visible_pc_transfer_refusal(
            runtime_shell,
            VisiblePcTransferKind::Deposit,
            snapshot.storage.current_pc_box,
            "Remove MAIL.",
        )?;
        set_shell_action_status(runtime_shell, "REMOVE MAIL");
        return Ok(());
    }
    let current_box = current_storage_box(&snapshot)?;
    if current_box.count >= crate::core::models::MAX_BOX_MONS {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pc:deposit_party:{party_index}:box_full:{}",
                snapshot.storage.current_pc_box
            ),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "PC box {} is full",
            snapshot.storage.current_pc_box
        ));
        begin_visible_pc_transfer_refusal(
            runtime_shell,
            VisiblePcTransferKind::Deposit,
            snapshot.storage.current_pc_box,
            "The BOX is full.",
        )?;
        set_shell_action_status(runtime_shell, "BOX IS FULL");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if slot.is_active_battle_pokemon {
        anyhow::bail!(
            "selected party index {party_index} is active in battle and cannot be deposited"
        );
    }
    record_visible_runtime_action(runtime_shell, format!("pc:deposit_party:{party_index}"))?;
    let deposit = runtime_shell
        .shell
        .deposit_party_pokemon_to_current_box(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "pc deposit party_index={} pokemon={} box={} slot={} checksum={:?}",
        deposit.party_index,
        deposit.pokemon.species.id,
        deposit.box_index,
        deposit.box_slot,
        deposit.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    queue_visible_pokemon_cry(
        runtime_shell,
        &deposit.pokemon.species.id,
        "bill_pc_deposit",
    )?;
    set_shell_action_status(
        runtime_shell,
        format!(
            "DEPOSITED {} BOX {} SLOT {}",
            deposit.pokemon.species.id, deposit.box_index, deposit.box_slot
        ),
    );
    runtime_shell.party_cursor = 0;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pc_notice = Some(format!("Stored {}!", deposit.pokemon.nickname));
    runtime_shell.field_text_reveal = None;
    runtime_shell.pc_transfer_sequence = Some(VisiblePcTransferSequence {
        kind: VisiblePcTransferKind::Deposit,
        box_index: deposit.box_index,
        phase: VisiblePcTransferPhase::SuccessHold,
        frames_remaining: 50,
    });
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn withdraw_visible_pc_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    ensure_no_visible_special_boundary(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_box = current_storage_box(&snapshot)?;
    if current_box.slots.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pc:withdraw_pokemon:{}:empty",
                snapshot.storage.current_pc_box
            ),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "PC box {} has no Pokemon",
            snapshot.storage.current_pc_box
        ));
        set_shell_action_status(runtime_shell, "BOX IS EMPTY");
        runtime_shell.pc_notice = Some("The BOX is empty.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if snapshot.storage.party_count >= crate::core::models::PARTY_SIZE {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pc:withdraw_pokemon:{}:party_full",
                snapshot.storage.current_pc_box
            ),
        )?;
        runtime_shell
            .last_audio_events
            .push("party is full".to_string());
        begin_visible_pc_transfer_refusal(
            runtime_shell,
            VisiblePcTransferKind::Withdraw,
            snapshot.storage.current_pc_box,
            "The party's full!",
        )?;
        set_shell_action_status(runtime_shell, "PARTY IS FULL");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let box_slot = selected_current_box_slot_index(runtime_shell)?;
    let current_box = snapshot.storage.current_pc_box;
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:withdraw_pokemon:{current_box}:{box_slot}"),
    )?;
    let withdraw = runtime_shell
        .shell
        .withdraw_current_box_pokemon_to_party(box_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "pc withdraw box={} slot={} pokemon={} party_index={} checksum={:?}",
        withdraw.box_index,
        withdraw.box_slot,
        withdraw.pokemon.species.id,
        withdraw.party_index,
        withdraw.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    queue_visible_pokemon_cry(
        runtime_shell,
        &withdraw.pokemon.species.id,
        "bill_pc_withdraw",
    )?;
    set_shell_action_status(
        runtime_shell,
        format!(
            "WITHDREW {} PARTY #{}",
            withdraw.pokemon.species.id, withdraw.party_index
        ),
    );
    runtime_shell.party_cursor = 0;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pc_notice = Some(format!("Got {}!", withdraw.pokemon.nickname));
    runtime_shell.field_text_reveal = None;
    runtime_shell.pc_transfer_sequence = Some(VisiblePcTransferSequence {
        kind: VisiblePcTransferKind::Withdraw,
        box_index: withdraw.box_index,
        phase: VisiblePcTransferPhase::SuccessHold,
        frames_remaining: 50,
    });
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn begin_visible_pc_transfer_refusal(
    runtime_shell: &mut BevyRuntimeShell,
    kind: VisiblePcTransferKind,
    box_index: usize,
    message: &str,
) -> Result<()> {
    runtime_shell.pc_notice = Some(message.to_string());
    runtime_shell.field_text_reveal = None;
    queue_visible_shell_sound_effect(runtime_shell, "SFX_WRONG")?;
    runtime_shell.pc_transfer_sequence = Some(VisiblePcTransferSequence {
        kind,
        box_index,
        phase: VisiblePcTransferPhase::RefusalWaitSfx,
        frames_remaining: 0,
    });
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_pc_transfer_sequence(
    runtime_shell: &mut BevyRuntimeShell,
    elapsed_frames: u32,
) -> Result<()> {
    if runtime_shell
        .pc_transfer_sequence
        .as_ref()
        .is_some_and(|active| active.phase == VisiblePcTransferPhase::RefusalWaitSfx)
    {
        if !visible_wait_sfx_finished(runtime_shell) {
            return Ok(());
        }
        let active = runtime_shell
            .pc_transfer_sequence
            .as_mut()
            .context("PC transfer refusal disappeared after its sound completed")?;
        active.phase = VisiblePcTransferPhase::RefusalHold;
        active.frames_remaining = 50;
        mark_runtime_presentation_dirty(runtime_shell);
        return Ok(());
    }

    let Some(active) = runtime_shell.pc_transfer_sequence.as_mut() else {
        return Ok(());
    };
    let elapsed = elapsed_frames.min(u32::from(active.frames_remaining));
    active.frames_remaining = active
        .frames_remaining
        .saturating_sub(u8::try_from(elapsed).unwrap_or(u8::MAX));
    if active.frames_remaining > 0 {
        return Ok(());
    }

    let finished = runtime_shell
        .pc_transfer_sequence
        .take()
        .context("PC transfer sequence disappeared at its final frame")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    anyhow::ensure!(
        snapshot.storage.current_pc_box == finished.box_index,
        "PC box changed during the locked transfer sequence"
    );
    runtime_shell.pc_notice = None;
    runtime_shell.field_text_reveal = None;
    match finished.kind {
        VisiblePcTransferKind::Deposit => {
            runtime_shell.party_cursor = 0;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(finished.box_index),
                option_index: 0,
            });
            if finished.phase == VisiblePcTransferPhase::SuccessHold
                && runtime_shell.bill_pc_session_open
            {
                open_visible_party_menu(runtime_shell)?;
            }
        }
        VisiblePcTransferKind::Withdraw => {
            let pc_box = snapshot
                .storage
                .boxes
                .iter()
                .find(|pc_box| pc_box.index == finished.box_index)
                .context("withdrawn Pokemon's PC box disappeared")?;
            runtime_shell.storage_cursor = (!pc_box.slots.is_empty()).then(|| MenuCursor {
                surface_id: storage_cursor_surface_id(finished.box_index),
                option_index: 0,
            });
        }
        VisiblePcTransferKind::BoxPrint => {}
    }
    set_shell_action_status(runtime_shell, "BILL'S PC");
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn deposit_visible_selected_pack_item_to_pc(
    runtime_shell: &mut BevyRuntimeShell,
    stack_index: usize,
    quantity: u16,
) -> Result<()> {
    ensure_no_visible_special_boundary(runtime_shell)?;
    let item_id = selected_field_pack_item_id(runtime_shell)?;
    let item_name = item_display_name(&runtime_shell.shell.snapshot()?, &item_id);
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:deposit_item:{item_id}:{quantity}"),
    )?;
    let transfer = runtime_shell
        .shell
        .deposit_bag_item_to_pc(&item_id, stack_index, quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "pc item deposit item={} quantity={} bag_after={} pc_after={} checksum={:?}",
        transfer.item_id,
        transfer.quantity,
        transfer.bag_quantity_after,
        transfer.pc_quantity_after,
        transfer.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "DEPOSITED {} x{} PC={}",
            transfer.item_id, transfer.quantity, transfer.pc_quantity_after
        ),
    );
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.player_pc_action_cursor = Some(MenuCursor {
        surface_id: "pc:player-actions".to_string(),
        option_index: 1,
    });
    runtime_shell.pc_notice = Some(format!("Deposited {quantity}\n{item_name}(S)."));
    Ok(())
}

fn withdraw_visible_pc_item_to_bag(
    runtime_shell: &mut BevyRuntimeShell,
    stack_index: usize,
    quantity: u16,
) -> Result<()> {
    ensure_no_visible_special_boundary(runtime_shell)?;
    let item_id = selected_pc_item_id(runtime_shell)?;
    let item_name = item_display_name(&runtime_shell.shell.snapshot()?, &item_id);
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:withdraw_item:{item_id}:{quantity}"),
    )?;
    let transfer = runtime_shell
        .shell
        .withdraw_pc_item_to_bag(&item_id, stack_index, quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "pc item withdraw item={} quantity={} bag_after={} pc_after={} checksum={:?}",
        transfer.item_id,
        transfer.quantity,
        transfer.bag_quantity_after,
        transfer.pc_quantity_after,
        transfer.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "WITHDREW {} x{} BAG={}",
            transfer.item_id, transfer.quantity, transfer.bag_quantity_after
        ),
    );
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    let pc_item_count = carried_item_count(&snapshot.bag.pc_items);
    if pc_item_count == 0 {
        runtime_shell.pc_item_cursor = None;
        runtime_shell.pc_item_action = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor {
            surface_id: "pc:player-actions".to_string(),
            option_index: 0,
        });
    } else {
        let selected_offset = snapshot
            .bag
            .pc_items
            .iter()
            .filter(|item| item.quantity > 0)
            .position(|item| item.item_id.as_str() >= transfer.item_id.as_str())
            .unwrap_or_else(|| pc_item_count.saturating_sub(1));
        runtime_shell.pc_item_cursor = Some(MenuCursor {
            surface_id: "pc:items".to_string(),
            option_index: selected_offset,
        });
    }
    runtime_shell.pc_notice = Some(format!("Withdrew {quantity}\n{item_name}(S)."));
    Ok(())
}

fn begin_visible_pc_item_quantity(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let action = runtime_shell
        .pc_item_action
        .context("Player PC item quantity requires an active action")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = if action == VisiblePlayerPcAction::DepositItem {
        selected_field_pack_item_id(runtime_shell)?
    } else {
        selected_pc_item_id(runtime_shell)?
    };
    if action == VisiblePlayerPcAction::TossItem {
        let item = snapshot
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .with_context(|| format!("selected PC item {item_id} is missing from the catalog"))?;
        if item
            .property
            .split('|')
            .any(|flag| flag.trim() == "CANT_TOSS")
        {
            runtime_shell.pc_notice = Some("That's too important to toss out!".to_string());
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
    }
    let (stack_index, available) = if action == VisiblePlayerPcAction::DepositItem {
        if active_visible_field_pack_pocket(runtime_shell) != FieldPackPocket::Items {
            anyhow::bail!("Player PC deposit requires the ITEM pocket");
        }
        let stack_index = runtime_shell
            .bag_cursor
            .as_ref()
            .context("Player PC deposit has no ITEM-pocket cursor")?
            .option_index;
        let quantity = snapshot
            .bag
            .items
            .get(stack_index)
            .filter(|item| item.item_id == item_id)
            .map(|item| item.quantity);
        (stack_index, quantity)
    } else {
        let stack_index = runtime_shell
            .pc_item_cursor
            .as_ref()
            .context("Player PC item action has no PC-item cursor")?
            .option_index;
        let quantity = snapshot
            .bag
            .pc_items
            .get(stack_index)
            .filter(|item| item.item_id == item_id)
            .map(|item| item.quantity);
        (stack_index, quantity)
    };
    let available = available
        .filter(|quantity| *quantity > 0)
        .with_context(|| format!("Player PC item {item_id} has no selectable quantity"))?;
    let maximum = available;
    runtime_shell.pc_item_quantity = Some(VisiblePcItemQuantity {
        action,
        item_id: item_id.clone(),
        stack_index,
        quantity: 1,
        maximum,
    });
    runtime_shell.pc_notice = Some(format!(
        "HOW MANY?\n{} x1",
        item_display_name(&snapshot, &item_id)
    ));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn adjust_visible_pc_item_quantity(runtime_shell: &mut BevyRuntimeShell, delta: i16) -> Result<()> {
    let (item_id, quantity) = {
        let pending = runtime_shell
            .pc_item_quantity
            .as_mut()
            .context("no Player PC item quantity is active")?;
        pending.quantity = (i32::from(pending.quantity) + i32::from(delta))
            .clamp(1, i32::from(pending.maximum)) as u16;
        (pending.item_id.clone(), pending.quantity)
    };
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.pc_notice = Some(format!(
        "HOW MANY?\n{} x{}",
        item_display_name(&snapshot, &item_id),
        quantity
    ));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn commit_visible_pc_item_quantity(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let pending = runtime_shell
        .pc_item_quantity
        .take()
        .context("no Player PC item quantity is active")?;
    runtime_shell.pc_notice = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    let destination_capacity = match pending.action {
        VisiblePlayerPcAction::DepositItem => visible_item_pocket_free_capacity(
            &snapshot.bag.pc_items,
            &pending.item_id,
            crate::core::models::PC_ITEM_CAPACITY,
        ),
        VisiblePlayerPcAction::WithdrawItem => visible_item_pocket_free_capacity(
            &snapshot.bag.items,
            &pending.item_id,
            crate::core::models::ITEM_POCKET_CAPACITY,
        ),
        _ => pending.quantity,
    };
    if pending.quantity > destination_capacity {
        runtime_shell.pc_notice = Some(if pending.action == VisiblePlayerPcAction::DepositItem {
            "The PC is full.".to_string()
        } else {
            "The PACK is full.".to_string()
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    match pending.action {
        VisiblePlayerPcAction::DepositItem => deposit_visible_selected_pack_item_to_pc(
            runtime_shell,
            pending.stack_index,
            pending.quantity,
        ),
        VisiblePlayerPcAction::WithdrawItem => {
            withdraw_visible_pc_item_to_bag(runtime_shell, pending.stack_index, pending.quantity)
        }
        VisiblePlayerPcAction::TossItem => {
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::TossItem {
                item_id: pending.item_id.clone(),
                stack_index: pending.stack_index,
                quantity: pending.quantity,
            });
            runtime_shell.yes_no_cursor = Some(MenuCursor {
                surface_id: "pc:confirmation".to_string(),
                option_index: 0,
            });
            runtime_shell.pc_notice = Some(format!(
                "Throw away {} x{}?",
                item_display_name(&snapshot, &pending.item_id),
                pending.quantity
            ));
            Ok(())
        }
        _ => anyhow::bail!("Player PC quantity is invalid for {pending:?}"),
    }
}

fn visible_item_pocket_free_capacity(
    items: &[RuntimeBagItemSnapshot],
    item_id: &str,
    slot_capacity: usize,
) -> u16 {
    let matching_space = items
        .iter()
        .filter(|item| item.item_id == item_id)
        .map(|item| u32::from(crate::core::models::MAX_ITEM_STACK - item.quantity))
        .sum::<u32>();
    let empty_slots = slot_capacity.saturating_sub(items.len());
    let empty_space = u32::try_from(empty_slots)
        .unwrap_or(u32::MAX)
        .saturating_mul(u32::from(crate::core::models::MAX_ITEM_STACK));
    matching_space
        .saturating_add(empty_space)
        .min(u32::from(u16::MAX)) as u16
}

fn request_visible_current_box_pokemon_release(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    ensure_no_visible_special_boundary(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_box_snapshot = current_storage_box(&snapshot)?;
    if current_box_snapshot.slots.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pc:release_pokemon:{}:empty",
                snapshot.storage.current_pc_box
            ),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "PC box {} has no Pokemon",
            snapshot.storage.current_pc_box
        ));
        runtime_shell.pc_notice = Some("The BOX is empty.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "BOX IS EMPTY");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let box_slot = selected_current_box_slot_index(runtime_shell)?;
    let current_box = snapshot.storage.current_pc_box;
    let pokemon = current_box_snapshot
        .slots
        .iter()
        .find(|slot| slot.index == box_slot)
        .context("selected PC release slot is missing")?;
    if pokemon.pokemon.is_egg
        || pokemon
            .pokemon
            .species
            .id
            .trim()
            .eq_ignore_ascii_case("EGG")
    {
        runtime_shell.pc_notice = Some("No releasing EGGS!".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if pokemon
        .pokemon
        .item
        .as_deref()
        .is_some_and(|item| item.to_ascii_uppercase().contains("MAIL"))
    {
        runtime_shell.pc_notice = Some("Remove MAIL.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    runtime_shell.pending_pc_release = Some(VisiblePcReleasePrompt {
        box_index: current_box,
        box_slot,
    });
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "pc:release-confirm".to_string(),
        option_index: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn open_visible_bill_pc_pokemon_actions(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_box = current_storage_box(&snapshot)?;
    anyhow::ensure!(
        !current_box.slots.is_empty(),
        "current PC box has no Pokemon"
    );
    selected_current_box_slot_index(runtime_shell)?;
    runtime_shell.bill_pc_pokemon_action_cursor = Some(MenuCursor {
        surface_id: "pc:pokemon-actions".to_string(),
        option_index: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_bill_pc_pokemon_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_pokemon_action_cursor,
        "pc:pokemon-actions",
        4,
    )
    .context("boxed Pokemon action menu requires a valid cursor")?;
    runtime_shell.bill_pc_pokemon_action_cursor = None;
    match selected {
        0 => withdraw_visible_pc_pokemon(runtime_shell),
        1 => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.bill_pc_box_summary = Some(VisiblePcBoxSummary {
                box_index: snapshot.storage.current_pc_box,
                box_slot: selected_current_box_slot_index(runtime_shell)?,
                page: 1,
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        2 => request_visible_current_box_pokemon_release(runtime_shell),
        _ => {
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
    }
}

fn confirm_visible_pc_release_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let prompt = runtime_shell
        .pending_pc_release
        .clone()
        .context("PC release confirmation is not active")?;
    let selected =
        strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "pc:release-confirm", 2)
            .context("PC release confirmation requires a valid cursor")?;
    runtime_shell.pending_pc_release = None;
    runtime_shell.yes_no_cursor = None;
    if selected != 0 {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    anyhow::ensure!(
        snapshot.storage.current_pc_box == prompt.box_index
            && selected_current_box_slot_index(runtime_shell)? == prompt.box_slot,
        "PC release selection changed while confirmation was open"
    );
    release_visible_current_box_pokemon(runtime_shell)
}

fn release_visible_current_box_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    ensure_no_visible_special_boundary(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let box_slot = selected_current_box_slot_index(runtime_shell)?;
    let current_box = snapshot.storage.current_pc_box;
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:release_pokemon:{current_box}:{box_slot}"),
    )?;
    let released = runtime_shell.shell.release_current_box_pokemon(box_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "pc release box={} slot={} pokemon={} checksum={:?}",
        released.box_index, released.box_slot, released.pokemon.species.id, released.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    queue_visible_pokemon_cry(
        runtime_shell,
        &released.pokemon.species.id,
        "bill_pc_release",
    )?;
    set_shell_action_status(
        runtime_shell,
        format!(
            "RELEASED {} BOX {} SLOT {}",
            released.pokemon.species.id, released.box_index, released.box_slot
        ),
    );
    runtime_shell.pc_notice = Some("Released <PK><MN>.".to_string());
    runtime_shell.field_text_reveal = None;
    runtime_shell.pc_release_sequence = Some(VisiblePcReleaseSequence {
        box_index: released.box_index,
        nickname: released.pokemon.nickname.clone(),
        phase: VisiblePcReleasePhase::Released,
        frames_remaining: 80,
    });
    close_visible_party_detail_state(runtime_shell);
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_pc_release_sequence(
    runtime_shell: &mut BevyRuntimeShell,
    elapsed_frames: u32,
) -> Result<()> {
    let mut elapsed_frames = elapsed_frames;
    while elapsed_frames > 0 {
        let Some(active) = runtime_shell.pc_release_sequence.as_mut() else {
            break;
        };
        let elapsed = elapsed_frames.min(u32::from(active.frames_remaining));
        active.frames_remaining = active
            .frames_remaining
            .saturating_sub(u8::try_from(elapsed).unwrap_or(u8::MAX));
        elapsed_frames -= elapsed;
        if active.frames_remaining > 0 {
            break;
        }
        match active.phase {
            VisiblePcReleasePhase::Released => {
                active.phase = VisiblePcReleasePhase::Bye;
                active.frames_remaining = 50;
                runtime_shell.pc_notice = Some(format!("Bye,\n{}!", active.nickname));
                runtime_shell.field_text_reveal = None;
                set_shell_action_status(runtime_shell, "BYE, POKEMON!");
                mark_runtime_presentation_dirty(runtime_shell);
            }
            VisiblePcReleasePhase::Bye => {
                let finished = runtime_shell
                    .pc_release_sequence
                    .take()
                    .context("PC release sequence disappeared at its final frame")?;
                let snapshot = runtime_shell.shell.snapshot()?;
                anyhow::ensure!(
                    snapshot.storage.current_pc_box == finished.box_index,
                    "PC box changed during the locked release sequence"
                );
                let pc_box = snapshot
                    .storage
                    .boxes
                    .iter()
                    .find(|pc_box| pc_box.index == finished.box_index)
                    .context("released Pokemon's PC box disappeared")?;
                runtime_shell.storage_cursor = (!pc_box.slots.is_empty()).then(|| MenuCursor {
                    surface_id: storage_cursor_surface_id(finished.box_index),
                    option_index: 0,
                });
                runtime_shell.pc_notice = None;
                runtime_shell.field_text_reveal = None;
                set_shell_action_status(runtime_shell, "BILL'S PC");
                mark_runtime_presentation_dirty(runtime_shell);
            }
        }
    }
    Ok(())
}

fn close_visible_party_detail_state(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.party_menu_open = false;
    runtime_shell.party_summary_open = false;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_give_take_cursor = None;
    runtime_shell.party_mail_take_stage = None;
    runtime_shell.party_move_reorder_open = false;
    runtime_shell.party_move_reorder_origin = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.party_hp_transfer_source = None;
    runtime_shell.party_hp_transfer_move = None;
    runtime_shell.party_move_cursor = None;
}

fn apply_visible_heal_party(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:heal_party")?;
    let special = runtime_shell.shell.heal_party_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special heal outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn fade_visible_music_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:fade_music")?;
    let special = runtime_shell.shell.fade_out_music_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special music fade outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn wait_visible_sfx_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:wait_sfx")?;
    let special = runtime_shell.shell.wait_sfx_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special wait sfx outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn play_visible_map_music_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:play_map_music")?;
    let special = runtime_shell.shell.play_map_music_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special play map music outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn restart_visible_map_music_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:restart_map_music")?;
    let special = runtime_shell.shell.restart_map_music_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special restart map music outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn full_heal_visible_party_lead(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    record_visible_runtime_action(runtime_shell, format!("party:full_heal:{party_index}"))?;
    let recovered = runtime_shell.shell.full_heal_party_pokemon(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "party recovery slot={} species={} hp {}->{} status {:?}->{:?} pp_moves={} checksum={:?}",
        recovered.party_index,
        recovered.species_id,
        recovered.hp_before,
        recovered.hp_after,
        recovered.status_before,
        recovered.status_after,
        recovered.pp_restored.len(),
        recovered.state_checksum
    ));
    Ok(())
}

fn full_heal_visible_whole_party(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "party:full_heal:all")?;
    let recovered = runtime_shell.shell.full_heal_whole_party()?;
    let checksum = recovered.last().map(|entry| &entry.state_checksum);
    runtime_shell.last_audio_events.push(format!(
        "whole party recovery slots={} checksum={:?}",
        recovered.len(),
        checksum
    ));
    Ok(())
}

fn resolve_visible_blackout(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_blackout_phase.is_some() {
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "overworld:blackout:resolve")?;
    let blackout_scene = runtime_shell.shell.snapshot()?;
    let player_name = if blackout_scene.trainer.player_name.is_empty() {
        "PLAYER"
    } else {
        blackout_scene.trainer.player_name.as_str()
    };
    runtime_shell
        .battle_messages
        .push_back(format!("{player_name} is out of\nuseable POKéMON!"));
    runtime_shell
        .battle_messages
        .push_back(format!("{player_name} whited\nout!"));
    runtime_shell.battle_message_scene = Some(Box::new(blackout_scene));
    runtime_shell.visible_blackout_phase = Some(VisibleBlackoutPhase::AwaitText);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn begin_visible_poison_blackout_after_faint_text(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<bool> {
    if !runtime_shell.pending_poison_blackout {
        return Ok(false);
    }
    runtime_shell.pending_poison_blackout = false;
    runtime_shell.field_notice_scene = None;
    resolve_visible_blackout(runtime_shell)?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn commit_visible_blackout_recovery(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let recovered = runtime_shell.shell.resolve_blackout_to_last_spawn()?;
    runtime_shell.last_audio_events.push(format!(
        "blackout recovery spawn={:?} map={} tile=({}, {}) healed={} checksum={:?}",
        recovered.spawn_identifier,
        recovered.map_name,
        recovered.tile.x,
        recovered.tile.y,
        recovered.healed.len(),
        recovered.state_checksum
    ));
    // Script_Whiteout commits healing, money loss, and the spawn warp after
    // the 40-frame white hold. MAPSETUP_WARP then reveals the destination;
    // ordinary scene scripts must not run under the still-white palette.
    reset_visible_navigation_state(runtime_shell);
    queue_visible_current_music(runtime_shell)?;
    runtime_shell.battle_message_scene = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn present_visible_step_event(
    runtime_shell: &mut BevyRuntimeShell,
    event: &crate::core::systems::step_events::StepEventResult,
) -> Result<()> {
    if let Some(item_id) = event.repel_expired.as_deref() {
        record_visible_runtime_action(runtime_shell, format!("overworld:repel_expired:{item_id}"))?;
        runtime_shell
            .last_audio_events
            .push(format!("{item_id} wore off"));
        runtime_shell.field_notice = Some("REPEL's effect wore off.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "REPEL WORE OFF");
    } else if event.egg_hatched {
        let species = event
            .hatched_species
            .as_deref()
            .context("hatched step event is missing its species")?;
        let party_index = event
            .hatched_party_index
            .context("hatched step event is missing its party index")?;
        record_visible_runtime_action(runtime_shell, format!("overworld:egg_hatched:{species}"))?;
        runtime_shell
            .last_audio_events
            .push(format!("egg hatched into {species}"));
        runtime_shell.visible_egg_hatch = Some(VisibleEggHatch {
            party_index,
            species_id: species.to_string(),
            phase: VisibleEggHatchPhase::HuhText,
            frame: 0,
        });
        runtime_shell.field_notice = Some("Huh?".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(
            runtime_shell,
            format!(
                "EGG HATCHED: {}",
                crate::core::models::pokemon_species_display_name(species)
            ),
        );
    } else if let Some(poison) = event.poison_result.as_ref() {
        record_visible_runtime_action(runtime_shell, "overworld:poison_step")?;
        // LoadPoisonBGPals owns four purple BG-palette VBlanks, restores the
        // map palettes, then PlayPoisonSFX owns one trailing normal DelayFrame
        // before DoPoisonStep or its faint script can continue.
        runtime_shell.poison_flash_frames_remaining = 5;
        queue_visible_shell_sound_effect(runtime_shell, "SFX_POISON")?;
        runtime_shell.last_audio_events.push(format!(
            "poison step damaged={:?} fainted={:?}",
            poison.damaged_names, poison.fainted_names
        ));
        if poison.fainted_names.is_empty() {
            set_shell_action_status(runtime_shell, "POISON HURT THE PARTY");
        } else {
            let notices = poison
                .fainted_names
                .iter()
                .map(|name| format!("{name} fainted!"));
            runtime_shell.field_notice_queue.extend(notices);
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(
                runtime_shell,
                format!("FAINTED: {}", poison.fainted_names.join(", ")),
            );
        }
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_visible_pokemon_center_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:pc:pokemon_center")?;
    let special = runtime_shell.shell.open_pokemon_center_pc_special()?;
    runtime_shell.last_audio_events.push(format!(
        "pokemon center pc outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_visible_players_house_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:pc:players_house")?;
    let special = runtime_shell.shell.open_players_house_pc_special()?;
    runtime_shell.last_audio_events.push(format!(
        "players house pc outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_visible_overworld_town_map(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:town_map:overworld")?;
    let special = runtime_shell.shell.open_overworld_town_map_special()?;
    runtime_shell.last_audio_events.push(format!(
        "overworld town map outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_visible_move_deletion(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let move_count = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?
        .pokemon
        .moves
        .len();
    if move_count <= 1 {
        record_visible_runtime_action(
            runtime_shell,
            format!("special:move_deletion:{party_index}:no_deletable_move"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "selected party index {party_index} has no deletable move"
        ));
        set_shell_action_status(runtime_shell, "NO MOVE TO DELETE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let move_slot = selected_party_move_slot(runtime_shell, party_index)?;
    if move_slot >= move_count {
        record_visible_runtime_action(
            runtime_shell,
            format!("special:move_deletion:{party_index}:{move_slot}:unavailable"),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("selected move slot {move_slot} is not deletable"));
        set_shell_action_status(runtime_shell, "MOVE UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(
        runtime_shell,
        format!("special:move_deletion:{party_index}:{move_slot}"),
    )?;
    let special = runtime_shell
        .shell
        .delete_party_move_special(party_index, move_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "move deletion party_index={} move_slot={} outcome={:?} checksum={:?}",
        party_index, move_slot, special.outcome.effect, special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_visible_name_rater(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    let slot_index = slot.index;
    let nickname = slot.pokemon.nickname.clone();
    record_visible_runtime_action(runtime_shell, format!("special:name_rater:{party_index}"))?;
    let special = runtime_shell
        .shell
        .rate_party_nickname_special(slot_index, nickname)?;
    runtime_shell.last_audio_events.push(format!(
        "name rater outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_visible_name_rival_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("NameRival")
    {
        return Ok(false);
    }
    apply_visible_name_rival(runtime_shell, source_script, command_index)?;
    Ok(true)
}

fn open_visible_day_care_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    let Some(routine) = compiled_special_routine_at(runtime_shell, source_script, command_index)?
    else {
        return Ok(false);
    };
    if routine == "DayCareManOutside" {
        let snapshot = runtime_shell.shell.snapshot()?;
        if snapshot.day_care.egg_present {
            let mut boundaries = visible_exported_special_text_boundaries(
                runtime_shell,
                "DayCareFoundEggText",
                "_FoundAnEggText",
            )?;
            let prompt = boundaries
                .pop_back()
                .context("Day-Care Egg offer has no final yes/no page")?;
            runtime_shell.pc_notice = Some(
                prompt
                    .details
                    .into_iter()
                    .next()
                    .context("Day-Care Egg offer final page is empty")?,
            );
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::DayCareEggPickup);
            runtime_shell.yes_no_cursor = Some(MenuCursor {
                surface_id: "pc:confirmation".to_string(),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "DAY-CARE EGG");
        } else {
            let mut boundaries = visible_exported_special_text_boundaries(
                runtime_shell,
                "DayCareNotYetText",
                "_NotYetText",
            )?;
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    let caretaker = match routine.as_str() {
        "DayCareMan" => "man",
        "DayCareLady" => "lady",
        _ => return Ok(false),
    };
    let snapshot = runtime_shell.shell.snapshot()?;
    let resident = if caretaker == "man" {
        &snapshot.day_care.man
    } else {
        &snapshot.day_care.lady
    };
    let owner = if caretaker == "man" { "MAN" } else { "LADY" };
    if let Some(pokemon) = resident.pokemon.as_ref() {
        let nickname = if pokemon.nickname.trim().is_empty() {
            canonical_species_display_name(&pokemon.species.id)
        } else {
            pokemon.nickname.clone()
        };
        let current_level = crate::core::systems::special_routines::day_care_level_from_experience(
            pokemon,
            runtime_shell.shell.runtime().growth_rates(),
        )?;
        let gained = current_level.saturating_sub(pokemon.level);
        let (text_target, confirm_withdrawal) = if gained == 0 {
            ("_BackAlreadyText", true)
        } else {
            ("_AreWeGeniusesText", false)
        };
        let mut boundaries = visible_exported_special_text_boundaries_with_buffer(
            runtime_shell,
            "DayCareWithdrawText",
            text_target,
            Some(&nickname),
        )?;
        let prompt = boundaries
            .pop_back()
            .context("Day-Care withdrawal offer has no final yes/no page")?;
        runtime_shell.pc_notice = Some(
            prompt
                .details
                .into_iter()
                .next()
                .context("Day-Care withdrawal final yes/no page is empty")?,
        );
        runtime_shell.special_boundary = boundaries.pop_front();
        runtime_shell.special_boundary_queue = boundaries;
        runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::DayCareWithdraw {
            caretaker: caretaker.to_string(),
            confirm_withdrawal,
        });
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "pc:confirmation".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, format!("DAY-CARE {owner}"));
    } else {
        let pending = PendingScriptPartySelection::DayCareDeposit {
            caretaker: caretaker.to_string(),
        };
        let intro_target = if caretaker == "man" {
            "_DayCareManIntroText"
        } else if snapshot.day_care.lady.active {
            "_DayCareLadyIntroEggText"
        } else {
            "_DayCareLadyIntroText"
        };
        let caretaker_kind = if caretaker == "man" {
            RuntimeDayCareCaretaker::Man
        } else {
            RuntimeDayCareCaretaker::Lady
        };
        let opened =
            runtime_shell
                .shell
                .use_day_care(caretaker_kind, RuntimeDayCareAction::Open, None)?;
        runtime_shell.last_audio_events.push(format!(
            "day-care introduction outcome={:?} checksum={:?}",
            opened.outcome.effect, opened.state_checksum
        ));
        let mut boundaries = visible_exported_special_text_boundaries(
            runtime_shell,
            "DayCareIntroText",
            intro_target,
        )?;
        let prompt = boundaries
            .pop_back()
            .context("Day-Care introduction has no final yes/no page")?;
        runtime_shell.pc_notice = Some(
            prompt
                .details
                .into_iter()
                .next()
                .context("Day-Care final yes/no page is empty")?,
        );
        runtime_shell.special_boundary = boundaries.pop_front();
        runtime_shell.special_boundary_queue = boundaries;
        runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::ScriptPartyIntro(pending));
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "pc:confirmation".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "DAY-CARE WHICH ONE?");
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn open_visible_script_party_selection_for_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    let origin_map_name = runtime_shell.shell.current_map_name().to_string();
    let command = runtime_shell.shell.script_runtime_command_at(
        &origin_map_name,
        source_script,
        command_index,
    );
    let pending = if command.as_ref().is_some_and(|command| {
        command.command == "trade"
            && command.args.first().is_some_and(|trade_id| {
                !runtime_shell
                    .shell
                    .session()
                    .state()
                    .script_runtime
                    .completed_trades
                    .contains(trade_id)
            })
    }) {
        let trade_id = command
            .as_ref()
            .and_then(|command| command.args.first().cloned())
            .context("trade command is missing its trade id")?;
        PendingScriptPartySelection::NpcTrade {
            origin_map_name,
            source_script: source_script.to_string(),
            command_index,
            trade_id,
        }
    } else if command
        .as_ref()
        .is_some_and(|command| command.command == "checkpokemail")
    {
        PendingScriptPartySelection::CheckPokeMail {
            origin_map_name,
            source_script: source_script.to_string(),
            command_index,
        }
    } else {
        match compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref() {
            Some("BillsGrandfather") => PendingScriptPartySelection::BillsGrandfather,
            Some("ReturnShuckie") => PendingScriptPartySelection::ReturnShuckie,
            Some("CheckMagikarpLength") => PendingScriptPartySelection::CheckMagikarpLength,
            Some("PhotoStudio") => PendingScriptPartySelection::PhotoStudio,
            Some("PokeSeer") => PendingScriptPartySelection::PokeSeer,
            Some("NameRater") => PendingScriptPartySelection::NameRater,
            Some("OlderHaircutBrother") => PendingScriptPartySelection::OlderHaircutBrother,
            Some("YoungerHaircutBrother") => PendingScriptPartySelection::YoungerHaircutBrother,
            Some("DaisysGrooming") => PendingScriptPartySelection::DaisysGrooming,
            Some("MoveDeletion") => PendingScriptPartySelection::MoveDeletion { party_index: None },
            Some("MoveTutor") => {
                let value = runtime_shell
                    .shell
                    .snapshot()?
                    .script_events
                    .script_value
                    .context("MoveTutor has no setval move selector")?;
                let move_id = match value.as_str() {
                    "1" | "MOVETUTOR_FLAMETHROWER" => "FLAMETHROWER",
                    "2" | "MOVETUTOR_THUNDERBOLT" => "THUNDERBOLT",
                    "3" | "MOVETUTOR_ICE_BEAM" => "ICE_BEAM",
                    other => anyhow::bail!("MoveTutor has unknown move selector {other}"),
                };
                PendingScriptPartySelection::MoveTutor {
                    move_id: move_id.to_string(),
                    party_index: None,
                }
            }
            _ => return Ok(false),
        }
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("script:special:party_selection:{pending:?}:open"),
    )?;
    if let PendingScriptPartySelection::NpcTrade { trade_id, .. } = &pending {
        let snapshot = runtime_shell.shell.snapshot()?;
        let rule = snapshot.special.npc_trades.get(trade_id).with_context(|| {
            format!("NPC trade {trade_id} is missing from the runtime snapshot")
        })?;
        runtime_shell.pc_notice = Some(visible_npc_trade_intro_text(rule));
        runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::NpcTrade(pending));
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "pc:confirmation".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "TRADE?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    if matches!(
        &pending,
        PendingScriptPartySelection::NameRater
            | PendingScriptPartySelection::MoveDeletion { party_index: None }
    ) {
        let (label, text_target) = match &pending {
            PendingScriptPartySelection::NameRater => ("NameRaterHelloText", "_NameRaterHelloText"),
            PendingScriptPartySelection::MoveDeletion { .. } => {
                ("DeleterIntroText", "_DeleterIntroText")
            }
            _ => unreachable!("script party intro matched an unsupported routine"),
        };
        let mut boundaries =
            visible_exported_special_text_boundaries(runtime_shell, label, text_target)?;
        let prompt = boundaries
            .pop_back()
            .context("source special introduction has no final yes/no page")?;
        runtime_shell.pc_notice = Some(
            prompt
                .details
                .into_iter()
                .next()
                .context("source special final yes/no page is empty")?,
        );
        runtime_shell.special_boundary = boundaries.pop_front();
        runtime_shell.special_boundary_queue = boundaries;
        runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::ScriptPartyIntro(pending));
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "pc:confirmation".to_string(),
            option_index: 0,
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    if matches!(&pending, PendingScriptPartySelection::PhotoStudio) {
        runtime_shell.pending_script_party_selection = Some(pending);
        let mut boundaries = visible_exported_special_text_boundaries(
            runtime_shell,
            "WhichMonPhotoText",
            "_WhichMonPhotoText",
        )?;
        runtime_shell.special_boundary = boundaries.pop_front();
        runtime_shell.special_boundary_queue = boundaries;
        set_shell_action_status(runtime_shell, "WHICH POKEMON?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    if matches!(&pending, PendingScriptPartySelection::PokeSeer) {
        runtime_shell.pending_script_party_selection = Some(pending);
        let mut boundaries = visible_exported_special_text_boundaries(
            runtime_shell,
            "SeerSeeAllText",
            "_SeerSeeAllText",
        )?;
        runtime_shell.special_boundary = boundaries.pop_front();
        runtime_shell.special_boundary_queue = boundaries;
        set_shell_action_status(runtime_shell, "I SEE ALL");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    runtime_shell.pending_script_party_selection = Some(pending);
    open_visible_party_menu(runtime_shell)?;
    set_shell_action_status(runtime_shell, "CHOOSE A POKEMON");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn visible_exported_special_text_boundaries(
    runtime_shell: &BevyRuntimeShell,
    label: &str,
    text_target: &str,
) -> Result<VecDeque<SpecialBoundaryDisplay>> {
    visible_exported_special_text_boundaries_with_buffer(runtime_shell, label, text_target, None)
}

fn visible_exported_special_text_boundaries_with_buffer(
    runtime_shell: &BevyRuntimeShell,
    label: &str,
    text_target: &str,
    string_buffer_1: Option<&str>,
) -> Result<VecDeque<SpecialBoundaryDisplay>> {
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let mut named_buffers = snapshot.script_events.named_buffers.clone();
    if let Some(value) = string_buffer_1 {
        named_buffers.insert("STRING_BUFFER_1".to_string(), value.to_string());
    }
    visible_exported_special_text_boundaries_with_named_buffers(
        runtime_shell,
        label,
        text_target,
        &named_buffers,
    )
}

fn visible_exported_special_text_boundaries_with_named_buffers(
    runtime_shell: &BevyRuntimeShell,
    label: &str,
    text_target: &str,
    named_buffers: &BTreeMap<String, String>,
) -> Result<VecDeque<SpecialBoundaryDisplay>> {
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let text = runtime_shell.shell.text_snapshot(text_target)?;
    let asm_text = text
        .asm_text
        .as_deref()
        .with_context(|| format!("special text {text_target} has no exported ASM body"))?;
    let pages = render_visible_asm_text_pages(
        asm_text,
        named_buffers,
        &snapshot.trainer.player_name,
        visible_rival_name(&snapshot),
        snapshot.progression.time.day_of_week,
    );
    anyhow::ensure!(
        !pages.is_empty(),
        "special text {text_target} rendered no source pages"
    );
    Ok(pages
        .into_iter()
        .map(|page| SpecialBoundaryDisplay {
            label: label.to_string(),
            details: vec![page],
        })
        .collect())
}

fn visible_move_tutor_text_boundaries(
    runtime_shell: &BevyRuntimeShell,
    label: &str,
    text_target: &str,
    nickname: &str,
    move_id: &str,
) -> Result<VecDeque<SpecialBoundaryDisplay>> {
    Ok(
        visible_move_learning_text_pages(runtime_shell, text_target, nickname, nickname, move_id)?
            .into_iter()
            .map(|page| SpecialBoundaryDisplay {
                label: label.to_string(),
                details: vec![page],
            })
            .collect(),
    )
}

fn visible_move_tutor_forgot_text_boundaries(
    runtime_shell: &BevyRuntimeShell,
    nickname: &str,
    forgotten_move_id: &str,
) -> Result<VecDeque<SpecialBoundaryDisplay>> {
    Ok(visible_move_learning_text_pages(
        runtime_shell,
        "_MoveForgotText",
        nickname,
        forgotten_move_id,
        "",
    )?
    .into_iter()
    .map(|page| SpecialBoundaryDisplay {
        label: "MoveForgotText".to_string(),
        details: vec![page],
    })
    .collect())
}

fn install_visible_move_learn_result_sequence(
    runtime_shell: &mut BevyRuntimeShell,
    nickname: &str,
    forgotten_move_id: Option<&str>,
    learned_move_id: &str,
) -> Result<()> {
    runtime_shell.special_boundary = None;
    runtime_shell.special_boundary_queue.clear();
    runtime_shell.visible_special_text_pause_frames = None;
    runtime_shell.visible_internal_special_delay_frames = None;
    if let Some(forgotten_move_id) = forgotten_move_id {
        let mut count = visible_move_tutor_text_boundaries(
            runtime_shell,
            "Text_1_2_and_Poof",
            "Text_MoveForgetCount",
            nickname,
            learned_move_id,
        )?;
        runtime_shell.special_boundary = count.pop_front();
        anyhow::ensure!(
            count.is_empty(),
            "move-learning count rendered multiple source pages"
        );
        let mut forgot =
            visible_move_tutor_forgot_text_boundaries(runtime_shell, nickname, forgotten_move_id)?;
        forgot
            .front_mut()
            .context("move-learning forgot text rendered no source pages")?
            .label = "MoveForgotPoofText".to_string();
        runtime_shell.special_boundary_queue.extend(forgot);
        runtime_shell.visible_special_text_pause_frames = Some(30);
    }
    let learned = visible_move_tutor_text_boundaries(
        runtime_shell,
        "LearnedMoveText",
        "_LearnedMoveText",
        nickname,
        learned_move_id,
    )?;
    if runtime_shell.special_boundary.is_some() {
        runtime_shell.special_boundary_queue.extend(learned);
    } else {
        runtime_shell.special_boundary = learned.front().cloned();
        runtime_shell
            .special_boundary_queue
            .extend(learned.into_iter().skip(1));
        queue_visible_shell_sound_effect(runtime_shell, "SFX_DEX_FANFARE_50_79")?;
    }
    Ok(())
}

fn visible_move_learning_text_pages(
    runtime_shell: &BevyRuntimeShell,
    text_target: &str,
    nickname: &str,
    string_buffer_1: &str,
    move_id: &str,
) -> Result<Vec<String>> {
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let mut named_buffers = snapshot.script_events.named_buffers.clone();
    named_buffers.insert("wMonOrItemNameBuffer".to_string(), nickname.to_string());
    named_buffers.insert(
        "STRING_BUFFER_1".to_string(),
        string_buffer_1.replace('_', " "),
    );
    let move_name = snapshot
        .moves
        .iter()
        .find(|move_data| move_data.move_id == move_id)
        .map(|move_data| move_data.name.replace('_', " "))
        .unwrap_or_else(|| move_id.replace('_', " "));
    named_buffers.insert("STRING_BUFFER_2".to_string(), move_name);
    let text = runtime_shell.shell.text_snapshot(text_target)?;
    let asm_text = text
        .asm_text
        .as_deref()
        .with_context(|| format!("move-learning text {text_target} has no exported ASM body"))?;
    let pages = render_visible_asm_text_pages(
        asm_text,
        &named_buffers,
        &snapshot.trainer.player_name,
        visible_rival_name(&snapshot),
        snapshot.progression.time.day_of_week,
    );
    anyhow::ensure!(
        !pages.is_empty(),
        "move-learning text {text_target} rendered no source pages"
    );
    Ok(pages)
}

fn visible_pending_move_learn_intro_pages(
    runtime_shell: &BevyRuntimeShell,
    nickname: &str,
    move_id: &str,
) -> Result<Vec<String>> {
    let mut pages = visible_move_learning_text_pages(
        runtime_shell,
        "_AskForgetMoveText",
        nickname,
        nickname,
        move_id,
    )?;
    pages
        .pop()
        .context("pending move-learning text has no final decision page")?;
    Ok(pages)
}

const KURT_APRICORN_ORDER: [&str; 7] = [
    "RED_APRICORN",
    "BLU_APRICORN",
    "YLW_APRICORN",
    "GRN_APRICORN",
    "WHT_APRICORN",
    "BLK_APRICORN",
    "PNK_APRICORN",
];

const BUENA_PRIZE_ORDER: [&str; 9] = [
    "ULTRA_BALL",
    "FULL_RESTORE",
    "NUGGET",
    "RARE_CANDY",
    "PROTEIN",
    "IRON",
    "CARBOS",
    "CALCIUM",
    "HP_UP",
];

fn visible_buena_prize_choices(snapshot: &RuntimeShellSnapshot) -> Result<Vec<(String, u8)>> {
    BUENA_PRIZE_ORDER
        .iter()
        .map(|item_id| {
            snapshot
                .special
                .buena_prizes
                .get(*item_id)
                .map(|cost| ((*item_id).to_string(), *cost))
                .with_context(|| format!("Buena prize {item_id} is missing from the runtime pack"))
        })
        .collect()
}

const VISIBLE_SLOT_REELS: [[&str; 15]; 3] = [
    [
        "SEVEN", "CHERRY", "STARYU", "PIKACHU", "SQUIRTLE", "SEVEN", "CHERRY", "STARYU", "PIKACHU",
        "SQUIRTLE", "POKEBALL", "CHERRY", "STARYU", "PIKACHU", "SQUIRTLE",
    ],
    [
        "SEVEN", "PIKACHU", "CHERRY", "SQUIRTLE", "STARYU", "POKEBALL", "PIKACHU", "CHERRY",
        "SQUIRTLE", "STARYU", "POKEBALL", "PIKACHU", "CHERRY", "SQUIRTLE", "STARYU",
    ],
    [
        "SEVEN", "PIKACHU", "CHERRY", "SQUIRTLE", "STARYU", "PIKACHU", "CHERRY", "SQUIRTLE",
        "STARYU", "PIKACHU", "POKEBALL", "CHERRY", "SQUIRTLE", "STARYU", "PIKACHU",
    ],
];

fn visible_slot_windows(offsets: [usize; 3]) -> [[String; 3]; 3] {
    std::array::from_fn(|reel| {
        std::array::from_fn(|row| VISIBLE_SLOT_REELS[reel][(offsets[reel] + row) % 15].to_string())
    })
}

fn visible_slot_has_match(machine: &VisibleSlotMachine) -> bool {
    let windows = &machine.windows;
    let same = |a: (usize, usize), b: (usize, usize), c: (usize, usize)| {
        windows[a.0][a.1] == windows[b.0][b.1] && windows[b.0][b.1] == windows[c.0][c.1]
    };
    same((0, 1), (1, 1), (2, 1))
        || machine.bet >= 2 && (same((0, 0), (1, 0), (2, 0)) || same((0, 2), (1, 2), (2, 2)))
        || machine.bet >= 3 && (same((0, 2), (1, 1), (2, 0)) || same((0, 0), (1, 1), (2, 2)))
}

fn open_visible_slot_machine_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("SlotMachine")
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let unavailable = if snapshot.trainer.coins == 0 {
        Some(("_NoCoinsText", vec!["You have no coins.".to_string()]))
    } else if carried_item_quantity(&snapshot, "COIN_CASE").unwrap_or(0) == 0 {
        Some((
            "_NoCoinCaseText",
            vec!["You don't have a".to_string(), "COIN CASE.".to_string()],
        ))
    } else {
        None
    };
    if let Some((label, details)) = unavailable {
        runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
            label: label.to_string(),
            details,
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    clear_visible_slot_runtime_state(runtime_shell);
    let lucky = runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .script_value
        .as_deref()
        == Some("1");
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_slot_machine_input = Some(SlotMachineInput::Enter { lucky });
    let entered = runtime_shell
        .shell
        .apply_declared_special_routine("SlotMachine")?;
    if !matches!(
        entered.outcome.effect,
        SpecialRoutineEffect::SlotMachineEntered { .. }
    ) {
        anyhow::bail!("SlotMachine entry returned a different special effect");
    }
    let offsets = [14; 3];
    runtime_shell.visible_slot_machine = Some(VisibleSlotMachine {
        phase: VisibleSlotMachinePhase::Betting,
        animation: VisibleSlotMachineAnimation::None,
        yes_no_index: 0,
        bet: 3,
        coins: snapshot.trainer.coins,
        payout: 0,
        offsets,
        spin_ticks: [0; 3],
        spinning: [false; 3],
        next_reel: 1,
        actor: None,
        secondary_actor: None,
        background_y_offset: 0,
        windows: visible_slot_windows(offsets),
        message: "BET 3".to_string(),
    });
    set_shell_action_status(runtime_shell, "SLOT MACHINE");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn change_visible_slot_machine_bet(runtime_shell: &mut BevyRuntimeShell, delta: i8) -> Result<()> {
    let machine = runtime_shell
        .visible_slot_machine
        .as_mut()
        .context("no slot machine is open")?;
    match machine.phase {
        VisibleSlotMachinePhase::Betting => {
            machine.bet = (machine.bet as i8 + delta).clamp(1, 3) as u8;
            machine.message = format!("BET {}", machine.bet);
        }
        VisibleSlotMachinePhase::PlayAgain => {
            machine.yes_no_index ^= 1;
        }
        VisibleSlotMachinePhase::Spinning
        | VisibleSlotMachinePhase::Result
        | VisibleSlotMachinePhase::RanOut
        | VisibleSlotMachinePhase::Quitting => return Ok(()),
    }
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn spin_visible_slot_machine(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (phase, animation, yes_no_index, bet, coins) = runtime_shell
        .visible_slot_machine
        .as_ref()
        .map(|machine| {
            (
                machine.phase,
                machine.animation,
                machine.yes_no_index,
                machine.bet,
                machine.coins,
            )
        })
        .context("no slot machine is open")?;
    match phase {
        VisibleSlotMachinePhase::Result => {
            if animation == VisibleSlotMachineAnimation::AwaitResult {
                runtime_shell
                    .shell
                    .session_mut()
                    .state
                    .script_runtime
                    .pending_slot_machine_input = Some(SlotMachineInput::AcknowledgeResult);
                let result = runtime_shell
                    .shell
                    .apply_declared_special_routine("SlotMachine")?;
                let SpecialRoutineEffect::SlotMachineResultAcknowledged { can_play_again, .. } =
                    result.outcome.effect
                else {
                    anyhow::bail!(
                        "SlotMachine result acknowledgement returned a different special effect"
                    );
                };
                let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
                machine.animation = VisibleSlotMachineAnimation::None;
                machine.yes_no_index = 0;
                if !can_play_again {
                    machine.phase = VisibleSlotMachinePhase::RanOut;
                    machine.message = "DARN… RAN OUT OF\nCOINS…".to_string();
                } else {
                    machine.phase = VisibleSlotMachinePhase::PlayAgain;
                    machine.message = "PLAY AGAIN?".to_string();
                }
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleSlotMachinePhase::PlayAgain => {
            if yes_no_index == 1 {
                return close_visible_slot_machine(runtime_shell);
            }
            runtime_shell
                .shell
                .session_mut()
                .state
                .script_runtime
                .pending_slot_machine_input = Some(SlotMachineInput::Continue);
            let result = runtime_shell
                .shell
                .apply_declared_special_routine("SlotMachine")?;
            if !matches!(
                result.outcome.effect,
                SpecialRoutineEffect::SlotMachineReplayAccepted { .. }
            ) {
                anyhow::bail!("SlotMachine replay returned a different special effect");
            }
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            machine.phase = VisibleSlotMachinePhase::Betting;
            machine.yes_no_index = 0;
            machine.payout = 0;
            machine.message = format!("BET {}", machine.bet);
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        VisibleSlotMachinePhase::Spinning => {
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            if let VisibleSlotMachineAnimation::Spinning {
                start_delay,
                requested_stop,
            } = &mut machine.animation
                && *start_delay == 0
                && !*requested_stop
            {
                *requested_stop = true;
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleSlotMachinePhase::Quitting => return Ok(()),
        VisibleSlotMachinePhase::RanOut => {
            if animation == VisibleSlotMachineAnimation::None {
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::RanOutDelay {
                    frames_remaining: 60,
                };
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleSlotMachinePhase::Betting => {}
    }
    if coins < u16::from(bet) {
        runtime_shell.visible_slot_machine.as_mut().unwrap().message =
            "NEED MORE COINS".to_string();
        queue_visible_shell_sound_effect(runtime_shell, "SFX_WRONG")?;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let lucky = runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .slot_machine
        .as_ref()
        .map(|machine| machine.lucky)
        .unwrap_or_else(|| {
            runtime_shell
                .shell
                .session()
                .state()
                .script_runtime
                .script_value
                .as_deref()
                == Some("1")
        });
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_slot_machine_input = Some(SlotMachineInput::Start { bet, lucky });
    queue_visible_shell_sound_effect(runtime_shell, "SFX_SLOT_MACHINE_START")?;
    let result = runtime_shell
        .shell
        .apply_declared_special_routine("SlotMachine")?;
    let SpecialRoutineEffect::SlotMachineStarted {
        coins,
        offsets,
        windows,
        ..
    } = result.outcome.effect
    else {
        anyhow::bail!("SlotMachine returned a different special effect");
    };
    let machine = runtime_shell
        .visible_slot_machine
        .as_mut()
        .context("slot machine closed during spin")?;
    machine.coins = coins;
    machine.payout = 0;
    machine.offsets = offsets;
    machine.spin_ticks = [0; 3];
    machine.spinning = [true; 3];
    machine.next_reel = 1;
    machine.windows = windows;
    machine.phase = VisibleSlotMachinePhase::Spinning;
    machine.animation = VisibleSlotMachineAnimation::Spinning {
        start_delay: 32,
        requested_stop: false,
    };
    machine.message = "PRESS A".to_string();
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn resolve_visible_slot_stop(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (reel, offsets) = runtime_shell
        .visible_slot_machine
        .as_ref()
        .map(|machine| (machine.next_reel, machine.offsets))
        .context("no slot machine is open")?;
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_slot_machine_input = Some(SlotMachineInput::StopReel {
        reel,
        offsets: offsets
            .map(|offset| u8::try_from(offset).expect("visible Slot Machine offset fits byte")),
    });
    let result = runtime_shell
        .shell
        .apply_declared_special_routine("SlotMachine")?;
    let SpecialRoutineEffect::SlotMachineReelStopped {
        reel,
        mode,
        animation_start_offset,
        animation_count,
        offsets: target_offsets,
        coins,
        ..
    } = result.outcome.effect
    else {
        anyhow::bail!("SlotMachine stop returned a different special effect");
    };
    let mode = match mode.as_str() {
        "normal" => VisibleSlotStopMode::Normal,
        "skip_to_seven" => VisibleSlotStopMode::SkipToSeven,
        "slow" => VisibleSlotStopMode::Slow,
        "golem" => VisibleSlotStopMode::Golem,
        "chansey" => VisibleSlotStopMode::Chansey,
        value => anyhow::bail!("unknown slot stop mode {value}"),
    };
    let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
    machine.coins = coins;
    machine.actor = None;
    machine.secondary_actor = None;
    machine.background_y_offset = 0;
    let target = target_offsets[usize::from(reel - 1)];
    machine.animation = match mode {
        VisibleSlotStopMode::Normal | VisibleSlotStopMode::SkipToSeven => {
            VisibleSlotMachineAnimation::Stopping {
                reel,
                mode,
                target,
                pause: u16::from(
                    mode == VisibleSlotStopMode::SkipToSeven
                        && target != offsets[usize::from(reel - 1)],
                ) * 32,
                steps: 0,
                minimum_steps: 0,
                terminal_delay: 0,
            }
        }
        VisibleSlotStopMode::Slow | VisibleSlotStopMode::Golem | VisibleSlotStopMode::Chansey => {
            VisibleSlotMachineAnimation::SpecialPrepare {
                mode,
                target,
                start_offset: animation_start_offset,
                count: animation_count,
            }
        }
    };
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_spinning_reels(machine: &mut VisibleSlotMachine, except: Option<usize>) {
    for reel in 0..3 {
        if !machine.spinning[reel] || except == Some(reel) {
            continue;
        }
        machine.spin_ticks[reel] += 1;
        if machine.spin_ticks[reel] == 4 {
            machine.spin_ticks[reel] = 0;
            machine.offsets[reel] = (machine.offsets[reel] + 1) % 15;
        }
    }
    machine.windows = visible_slot_windows(machine.offsets);
}

fn tick_visible_slot_actor(machine: &mut VisibleSlotMachine) {
    machine.actor = match machine.actor {
        Some(VisibleSlotActor::Golem {
            x,
            y_offset,
            mut frame,
            mut frame_tick,
            ..
        }) => {
            frame_tick += 1;
            if frame_tick == 8 {
                frame_tick = 0;
                frame = (frame + 1) % 4;
            }
            Some(VisibleSlotActor::Golem {
                x,
                y_offset,
                frame,
                frame_tick,
                flip_x: frame == 3,
                flip_y: frame == 2,
            })
        }
        Some(VisibleSlotActor::Chansey {
            x,
            mut frame,
            mut frame_tick,
            finishing,
        }) => {
            frame_tick += 1;
            if frame_tick == 8 {
                frame_tick = 0;
                if finishing {
                    frame = (frame + 1).min(4);
                } else {
                    frame = (frame + 1) % 4;
                }
            }
            Some(VisibleSlotActor::Chansey {
                x,
                frame,
                frame_tick,
                finishing,
            })
        }
        actor => actor,
    };
}

fn visible_slot_terminal_stop(machine: &mut VisibleSlotMachine, target: usize) {
    machine.actor = None;
    machine.secondary_actor = None;
    machine.background_y_offset = 0;
    machine.spinning[2] = false;
    machine.spin_ticks[2] = 0;
    machine.animation = VisibleSlotMachineAnimation::Stopping {
        reel: 3,
        mode: VisibleSlotStopMode::Normal,
        target,
        pause: 0,
        steps: 0,
        minimum_steps: 0,
        terminal_delay: 0,
    };
}

fn resolve_visible_slot_result(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_slot_machine_input = Some(SlotMachineInput::ResolveResult);
    let result = runtime_shell
        .shell
        .apply_declared_special_routine("SlotMachine")?;
    let SpecialRoutineEffect::SlotMachineResult {
        payout,
        matched_symbol,
        coins,
        ..
    } = result.outcome.effect
    else {
        anyhow::bail!("SlotMachine result returned a different special effect");
    };
    let result_sound = matched_symbol.as_deref().map(|symbol| match symbol {
        "SEVEN" => "SFX_2ND_PLACE",
        "POKEBALL" => "SFX_3RD_PLACE",
        _ => "SFX_PRESENT",
    });
    let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
    machine.phase = VisibleSlotMachinePhase::Result;
    machine.coins = coins;
    machine.payout = payout;
    machine.message = if payout > 0 {
        format!("WIN {payout}")
    } else {
        "DARN".to_string()
    };
    if let Some(sound) = result_sound {
        queue_visible_shell_sound_effect(runtime_shell, sound)?;
        runtime_shell
            .visible_slot_machine
            .as_mut()
            .unwrap()
            .animation = VisibleSlotMachineAnimation::WaitResult { payout };
    } else {
        runtime_shell
            .visible_slot_machine
            .as_mut()
            .unwrap()
            .animation = VisibleSlotMachineAnimation::AwaitResult;
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_slot_machine_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let animation = runtime_shell
        .visible_slot_machine
        .as_ref()
        .map(|machine| machine.animation)
        .context("no slot machine is open")?;
    match animation {
        VisibleSlotMachineAnimation::None | VisibleSlotMachineAnimation::AwaitResult => Ok(()),
        VisibleSlotMachineAnimation::Spinning {
            start_delay,
            requested_stop,
        } => {
            let selected = usize::from(
                runtime_shell
                    .visible_slot_machine
                    .as_ref()
                    .unwrap()
                    .next_reel
                    - 1,
            );
            if requested_stop
                && runtime_shell
                    .visible_slot_machine
                    .as_ref()
                    .unwrap()
                    .spin_ticks[selected]
                    == 0
            {
                resolve_visible_slot_stop(runtime_shell)?;
                return advance_visible_slot_machine_animation(runtime_shell);
            }
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            advance_visible_spinning_reels(machine, None);
            machine.animation = VisibleSlotMachineAnimation::Spinning {
                start_delay: start_delay.saturating_sub(1),
                requested_stop,
            };
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::SpecialPrepare {
            mode,
            target,
            start_offset,
            count,
        } => {
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            if machine.offsets[2] != start_offset {
                machine.spin_ticks[2] += 1;
                if machine.spin_ticks[2] == 4 {
                    machine.spin_ticks[2] = 0;
                    machine.offsets[2] = (machine.offsets[2] + 1) % 15;
                    machine.windows = visible_slot_windows(machine.offsets);
                }
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            machine.spinning[2] = false;
            machine.spin_ticks[2] = 0;
            machine.animation = VisibleSlotMachineAnimation::SpecialWait {
                mode,
                target,
                count,
                frames_remaining: 16,
            };
            queue_visible_shell_sound_effect(runtime_shell, "SFX_STOP_SLOT")?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::SpecialWait {
            mode,
            target,
            count,
            frames_remaining,
        } => {
            if frames_remaining > 1 {
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::SpecialWait {
                    mode,
                    target,
                    count,
                    frames_remaining: frames_remaining - 1,
                };
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            machine.animation = match mode {
                VisibleSlotStopMode::Slow => VisibleSlotMachineAnimation::SlowAdvance {
                    target,
                    steps_remaining: count,
                    // Setting rate 1 consumes the first of sixteen subpixel
                    // advances in this same source frame.
                    frames_until_step: 15,
                },
                VisibleSlotStopMode::Golem => VisibleSlotMachineAnimation::Golem {
                    target,
                    remaining: count,
                    phase: VisibleSlotGolemPhase::Init,
                    phase_frame: 0,
                },
                VisibleSlotStopMode::Chansey => {
                    machine.actor = Some(VisibleSlotActor::Chansey {
                        x: 96,
                        frame: 0,
                        frame_tick: 0,
                        finishing: false,
                    });
                    VisibleSlotMachineAnimation::Chansey {
                        target,
                        remaining_eggs: count,
                        phase: VisibleSlotChanseyPhase::Walk,
                        phase_frame: 0,
                    }
                }
                _ => unreachable!("only special reel modes enter the fixed wait"),
            };
            mark_runtime_snapshot_dirty(runtime_shell);
            advance_visible_slot_machine_animation(runtime_shell)
        }
        VisibleSlotMachineAnimation::SlowAdvance {
            target,
            steps_remaining,
            frames_until_step,
        } => {
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            if steps_remaining == 0 {
                visible_slot_terminal_stop(machine, target);
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            if frames_until_step > 0 {
                machine.animation = VisibleSlotMachineAnimation::SlowAdvance {
                    target,
                    steps_remaining,
                    frames_until_step: frames_until_step - 1,
                };
            } else {
                machine.offsets[2] = (machine.offsets[2] + 1) % 15;
                machine.windows = visible_slot_windows(machine.offsets);
                let remaining = steps_remaining - 1;
                machine.animation = VisibleSlotMachineAnimation::SlowAdvance {
                    target,
                    steps_remaining: remaining,
                    frames_until_step: 15,
                };
                if remaining > 0 {
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_GOT_SAFARI_BALLS")?;
                }
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::Golem {
            target,
            remaining,
            phase,
            phase_frame,
        } => {
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            let mut sound = None;
            match phase {
                VisibleSlotGolemPhase::Init => {
                    if remaining == 0 {
                        visible_slot_terminal_stop(machine, target);
                    } else {
                        let angle = 0x30;
                        machine.actor = Some(VisibleSlotActor::Golem {
                            x: 96,
                            y_offset: visible_battle_anim_sine(angle, 14 * 8) as i16,
                            frame: 0,
                            frame_tick: 0,
                            flip_x: false,
                            flip_y: false,
                        });
                        machine.animation = VisibleSlotMachineAnimation::Golem {
                            target,
                            remaining: remaining - 1,
                            phase: VisibleSlotGolemPhase::Fall,
                            phase_frame: angle - 1,
                        };
                    }
                }
                VisibleSlotGolemPhase::Fall => {
                    if phase_frame >= 0x20 {
                        if let Some(VisibleSlotActor::Golem { y_offset, .. }) =
                            machine.actor.as_mut()
                        {
                            *y_offset = visible_battle_anim_sine(phase_frame, 14 * 8) as i16;
                        }
                        tick_visible_slot_actor(machine);
                        machine.animation = VisibleSlotMachineAnimation::Golem {
                            target,
                            remaining,
                            phase,
                            phase_frame: phase_frame - 1,
                        };
                    } else {
                        sound = Some("SFX_PLACE_PUZZLE_PIECE_DOWN");
                        machine.animation = VisibleSlotMachineAnimation::Golem {
                            target,
                            remaining,
                            phase: VisibleSlotGolemPhase::Roll,
                            phase_frame: 0,
                        };
                    }
                }
                VisibleSlotGolemPhase::Roll => {
                    let x_offset = phase_frame.saturating_mul(2);
                    if x_offset >= 9 * 8 {
                        machine.background_y_offset = 0;
                        machine.actor = Some(VisibleSlotActor::Golem {
                            x: 96 + i16::from(x_offset + 2),
                            y_offset: 0,
                            frame: 0,
                            frame_tick: 0,
                            flip_x: false,
                            flip_y: false,
                        });
                        machine.animation = VisibleSlotMachineAnimation::Golem {
                            target,
                            remaining,
                            phase: VisibleSlotGolemPhase::Init,
                            phase_frame: 0,
                        };
                    } else {
                        if phase_frame == 1 {
                            machine.offsets[2] = (machine.offsets[2] + 1) % 15;
                            machine.windows = visible_slot_windows(machine.offsets);
                        }
                        if phase_frame % 2 == 0 {
                            machine.background_y_offset = -machine.background_y_offset;
                            if machine.background_y_offset == 0 {
                                machine.background_y_offset = -2;
                            }
                        }
                        if let Some(VisibleSlotActor::Golem { x, y_offset, .. }) =
                            machine.actor.as_mut()
                        {
                            *x = 96 + i16::from(x_offset + 2);
                            *y_offset = 0;
                        }
                        tick_visible_slot_actor(machine);
                        machine.animation = VisibleSlotMachineAnimation::Golem {
                            target,
                            remaining,
                            phase,
                            phase_frame: phase_frame + 1,
                        };
                    }
                }
            }
            if let Some(sound) = sound {
                queue_visible_shell_sound_effect(runtime_shell, sound)?;
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::Chansey {
            target,
            remaining_eggs,
            phase,
            phase_frame,
        } => {
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            let mut sound = None;
            match phase {
                VisibleSlotChanseyPhase::Walk => {
                    let old_x = match machine.actor {
                        Some(VisibleSlotActor::Chansey { x, .. }) => x,
                        _ => 96,
                    };
                    if let Some(VisibleSlotActor::Chansey { x, .. }) = machine.actor.as_mut() {
                        *x += 1;
                    }
                    tick_visible_slot_actor(machine);
                    if old_x == 13 * 8 {
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs,
                            phase: VisibleSlotChanseyPhase::PrepareEgg,
                            phase_frame: 0,
                        };
                    } else {
                        if old_x & 0xf == 0 {
                            sound = Some("SFX_JUMP_OVER_LEDGE");
                        }
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs,
                            phase,
                            phase_frame: phase_frame + 1,
                        };
                    }
                }
                VisibleSlotChanseyPhase::PrepareEgg => {
                    if let Some(VisibleSlotActor::Chansey {
                        frame,
                        frame_tick,
                        finishing,
                        ..
                    }) = machine.actor.as_mut()
                    {
                        if phase_frame == 0 {
                            *frame = 0;
                            *frame_tick = 0;
                            *finishing = true;
                        }
                    }
                    tick_visible_slot_actor(machine);
                    if phase_frame >= 8 {
                        machine.secondary_actor =
                            Some(VisibleSlotActor::Egg { x: 96, y_offset: 0 });
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs,
                            phase: VisibleSlotChanseyPhase::Egg,
                            phase_frame: 0,
                        };
                    } else {
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs,
                            phase,
                            phase_frame: phase_frame + 1,
                        };
                    }
                }
                VisibleSlotChanseyPhase::Egg => {
                    let index = 0_u8.wrapping_sub(phase_frame);
                    let mut landed = false;
                    if let Some(VisibleSlotActor::Egg { x, y_offset }) =
                        machine.secondary_actor.as_mut()
                    {
                        if index & 1 != 0 {
                            if *x >= 15 * 8 {
                                landed = true;
                            } else {
                                *x += 1;
                            }
                        }
                        if !landed {
                            *y_offset = visible_battle_anim_sine(index, 32) as i16;
                        }
                    }
                    tick_visible_slot_actor(machine);
                    if landed {
                        machine.secondary_actor = None;
                        sound = Some("SFX_PLACE_PUZZLE_PIECE_DOWN");
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs,
                            phase: VisibleSlotChanseyPhase::DropReel,
                            phase_frame: 0,
                        };
                    } else {
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs,
                            phase,
                            phase_frame: phase_frame + 1,
                        };
                    }
                }
                VisibleSlotChanseyPhase::DropReel => {
                    machine.offsets[2] = (machine.offsets[2] + 1) % 15;
                    machine.windows = visible_slot_windows(machine.offsets);
                    tick_visible_slot_actor(machine);
                    machine.animation = VisibleSlotMachineAnimation::Chansey {
                        target,
                        remaining_eggs,
                        phase: if phase_frame >= 16 {
                            VisibleSlotChanseyPhase::CheckMatch
                        } else {
                            phase
                        },
                        phase_frame: if phase_frame >= 16 {
                            0
                        } else {
                            phase_frame + 1
                        },
                    };
                }
                VisibleSlotChanseyPhase::CheckMatch => {
                    let remaining = remaining_eggs - 1;
                    if remaining == 0 {
                        visible_slot_terminal_stop(machine, target);
                    } else {
                        machine.animation = VisibleSlotMachineAnimation::Chansey {
                            target,
                            remaining_eggs: remaining,
                            phase: VisibleSlotChanseyPhase::PrepareEgg,
                            phase_frame: 0,
                        };
                    }
                }
            }
            if let Some(sound) = sound {
                queue_visible_shell_sound_effect(runtime_shell, sound)?;
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::Stopping {
            reel,
            mode,
            target,
            pause,
            steps,
            minimum_steps,
            terminal_delay,
        } => {
            let selected = usize::from(reel - 1);
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            advance_visible_spinning_reels(machine, Some(selected));
            if terminal_delay > 0 {
                if terminal_delay > 1 {
                    machine.animation = VisibleSlotMachineAnimation::Stopping {
                        reel,
                        mode,
                        target,
                        pause: 0,
                        steps,
                        minimum_steps,
                        terminal_delay: terminal_delay - 1,
                    };
                } else {
                    machine.next_reel = reel + 1;
                    let mut resolve_without_flash = false;
                    if reel < 3 {
                        machine.animation = VisibleSlotMachineAnimation::Spinning {
                            start_delay: 0,
                            requested_stop: false,
                        };
                        machine.message = "PRESS A".to_string();
                    } else {
                        resolve_without_flash = !visible_slot_has_match(machine);
                        machine.animation = if resolve_without_flash {
                            VisibleSlotMachineAnimation::AwaitResult
                        } else {
                            VisibleSlotMachineAnimation::FlashResult {
                                frames_remaining: 16,
                            }
                        };
                    }
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_STOP_SLOT")?;
                    if resolve_without_flash {
                        resolve_visible_slot_result(runtime_shell)?;
                    }
                }
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            if pause > 0 {
                machine.animation = VisibleSlotMachineAnimation::Stopping {
                    reel,
                    mode,
                    target,
                    pause: pause - 1,
                    steps,
                    minimum_steps,
                    terminal_delay: 0,
                };
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            let interval = match mode {
                VisibleSlotStopMode::Normal => 4,
                VisibleSlotStopMode::SkipToSeven | VisibleSlotStopMode::Golem => 2,
                VisibleSlotStopMode::Slow => 16,
                VisibleSlotStopMode::Chansey => 1,
            };
            let at_target = machine.offsets[selected] == target && steps >= minimum_steps;
            if at_target {
                machine.spinning[selected] = false;
                machine.spin_ticks[selected] = 0;
                machine.animation = VisibleSlotMachineAnimation::Stopping {
                    reel,
                    mode,
                    target,
                    pause: 0,
                    steps,
                    minimum_steps,
                    terminal_delay: 4,
                };
            } else {
                machine.spin_ticks[selected] += 1;
                let mut next_steps = steps;
                if machine.spin_ticks[selected] >= interval {
                    machine.spin_ticks[selected] = 0;
                    machine.offsets[selected] = (machine.offsets[selected] + 1) % 15;
                    next_steps += 1;
                    machine.windows = visible_slot_windows(machine.offsets);
                }
                machine.animation = VisibleSlotMachineAnimation::Stopping {
                    reel,
                    mode,
                    target,
                    pause: 0,
                    steps: next_steps,
                    minimum_steps,
                    terminal_delay: 0,
                };
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::FlashResult { frames_remaining } => {
            if frames_remaining > 1 {
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::FlashResult {
                    frames_remaining: frames_remaining - 1,
                };
                mark_runtime_snapshot_dirty(runtime_shell);
                Ok(())
            } else {
                resolve_visible_slot_result(runtime_shell)
            }
        }
        VisibleSlotMachineAnimation::QuitWaitBefore => {
            if !visible_wait_sfx_finished(runtime_shell) {
                return Ok(());
            }
            queue_visible_shell_sound_effect(runtime_shell, "SFX_QUIT_SLOTS")?;
            runtime_shell
                .visible_slot_machine
                .as_mut()
                .unwrap()
                .animation = VisibleSlotMachineAnimation::QuitWaitAfter;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::QuitWaitAfter => {
            if !visible_wait_sfx_finished(runtime_shell) {
                return Ok(());
            }
            runtime_shell.visible_slot_machine = None;
            clear_visible_slot_runtime_state(runtime_shell);
            mark_runtime_snapshot_dirty(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)
        }
        VisibleSlotMachineAnimation::RanOutDelay { frames_remaining } => {
            let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
            if frames_remaining > 1 {
                machine.animation = VisibleSlotMachineAnimation::RanOutDelay {
                    frames_remaining: frames_remaining - 1,
                };
            } else {
                machine.phase = VisibleSlotMachinePhase::Quitting;
                machine.animation = VisibleSlotMachineAnimation::QuitWaitBefore;
                machine.message.clear();
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::WaitStart {
            payout,
            result_sound,
        } => {
            if !visible_wait_sfx_finished(runtime_shell) {
                return Ok(());
            }
            if let Some(sound) = result_sound {
                queue_visible_shell_sound_effect(runtime_shell, sound)?;
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::WaitResult { payout };
            } else {
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::AwaitResult;
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::WaitResult { payout } => {
            if !visible_wait_sfx_finished(runtime_shell) {
                return Ok(());
            }
            runtime_shell
                .visible_slot_machine
                .as_mut()
                .unwrap()
                .animation = VisibleSlotMachineAnimation::Payout {
                remaining: payout,
                frames_until_coin: 1,
                delay_counter: 0,
            };
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleSlotMachineAnimation::Payout {
            remaining,
            frames_until_coin,
            delay_counter,
        } => {
            if frames_until_coin > 0 {
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::Payout {
                    remaining,
                    frames_until_coin: frames_until_coin - 1,
                    delay_counter: delay_counter + 1,
                };
            } else if remaining == 0 {
                runtime_shell
                    .visible_slot_machine
                    .as_mut()
                    .unwrap()
                    .animation = VisibleSlotMachineAnimation::AwaitResult;
            } else {
                runtime_shell
                    .shell
                    .session_mut()
                    .state
                    .script_runtime
                    .pending_slot_machine_input = Some(SlotMachineInput::PayoutFrame);
                let result = runtime_shell
                    .shell
                    .apply_declared_special_routine("SlotMachine")?;
                let SpecialRoutineEffect::SlotMachinePayout {
                    coins_before,
                    payout_remaining,
                    coins,
                    ..
                } = result.outcome.effect
                else {
                    anyhow::bail!("SlotMachine payout returned a different special effect");
                };
                let next_delay_counter = delay_counter + 1;
                let machine = runtime_shell.visible_slot_machine.as_mut().unwrap();
                machine.coins = coins;
                machine.payout = payout_remaining;
                machine.animation = VisibleSlotMachineAnimation::Payout {
                    remaining: payout_remaining,
                    frames_until_coin: 1,
                    delay_counter: next_delay_counter,
                };
                if coins > coins_before && next_delay_counter & 7 != 0 {
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_GET_COIN_FROM_SLOTS")?;
                }
            }
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
    }
}

fn clear_visible_slot_runtime_state(runtime_shell: &mut BevyRuntimeShell) {
    let script_runtime = &mut runtime_shell.shell.session_mut().state.script_runtime;
    script_runtime.slot_machine = None;
    script_runtime.pending_slot_machine_input = None;
}

fn close_visible_slot_machine(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if let Some(machine) = runtime_shell.visible_slot_machine.as_ref()
        && matches!(
            machine.phase,
            VisibleSlotMachinePhase::Result | VisibleSlotMachinePhase::RanOut
        )
    {
        return spin_visible_slot_machine(runtime_shell);
    }
    if runtime_shell
        .visible_slot_machine
        .as_ref()
        .is_some_and(|machine| {
            matches!(
                machine.phase,
                VisibleSlotMachinePhase::Spinning | VisibleSlotMachinePhase::Quitting
            )
        })
    {
        return Ok(());
    }
    let core_can_quit = runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .slot_machine
        .as_ref()
        .is_some_and(|machine| {
            matches!(
                machine.phase,
                SlotMachinePhase::Betting | SlotMachinePhase::PlayAgain
            )
        });
    if core_can_quit {
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .pending_slot_machine_input = Some(SlotMachineInput::Quit);
        let result = runtime_shell
            .shell
            .apply_declared_special_routine("SlotMachine")?;
        if !matches!(
            result.outcome.effect,
            SpecialRoutineEffect::SlotMachineExited { .. }
        ) {
            anyhow::bail!("SlotMachine quit returned a different special effect");
        }
    }
    let machine = runtime_shell
        .visible_slot_machine
        .as_mut()
        .context("no slot machine is open")?;
    machine.phase = VisibleSlotMachinePhase::Quitting;
    machine.animation = VisibleSlotMachineAnimation::QuitWaitBefore;
    machine.message.clear();
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn open_visible_card_flip_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("CardFlip")
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let unavailable = if carried_item_quantity(&snapshot, "COIN_CASE").unwrap_or(0) == 0 {
        Some((
            "_NoCoinCaseText",
            vec!["You don't have a".to_string(), "COIN CASE.".to_string()],
        ))
    } else {
        None
    };
    if let Some((label, details)) = unavailable {
        runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
            label: label.to_string(),
            details,
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    runtime_shell.visible_card_flip = Some(VisibleCardFlip {
        phase: VisibleCardFlipPhase::AskPlay,
        animation: VisibleCardFlipAnimation::None,
        yes_no_index: 0,
        which_card: 0,
        bet_x: 2,
        bet_y: 2,
        round: 0,
        face_card: None,
        coins: snapshot.trainer.coins,
        payout: 0,
        deck: Vec::new(),
        revealed: vec![false; 24],
        message: "PLAY WITH THREE COINS?".to_string(),
    });
    let script_runtime = &mut runtime_shell.shell.session_mut().state.script_runtime;
    script_runtime.card_flip = None;
    script_runtime.pending_card_flip_input = None;
    set_shell_action_status(runtime_shell, "CARD FLIP");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn move_visible_card_flip_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    dx: isize,
    dy: isize,
) -> Result<()> {
    let game = runtime_shell
        .visible_card_flip
        .as_mut()
        .context("no Card Flip game is open")?;
    let sound = match game.phase {
        VisibleCardFlipPhase::AskPlay | VisibleCardFlipPhase::PlayAgain => {
            if dy != 0 {
                game.yes_no_index ^= 1;
                Some("SFX_READ_TEXT_2")
            } else {
                None
            }
        }
        VisibleCardFlipPhase::ChooseCard
        | VisibleCardFlipPhase::Result
        | VisibleCardFlipPhase::Shuffled => None,
        VisibleCardFlipPhase::PlaceBet => {
            let before = (game.bet_x, game.bet_y);
            if dx < 0 {
                if game.bet_y == 0 {
                    game.bet_x &= !1;
                    if game.bet_x < 3 {
                        game.bet_x = 1;
                        game.bet_y = 2;
                    } else {
                        game.bet_x -= 2;
                    }
                } else if game.bet_y == 1 && game.bet_x < 3 {
                    game.bet_x = 1;
                    game.bet_y = 2;
                } else if game.bet_x > 0 {
                    game.bet_x -= 1;
                }
            } else if dx > 0 {
                if game.bet_y == 0 {
                    game.bet_x &= !1;
                    if game.bet_x < 4 {
                        game.bet_x += 2;
                    }
                } else if game.bet_x < 5 {
                    game.bet_x += 1;
                }
            } else if dy < 0 {
                if game.bet_x == 0 {
                    game.bet_y &= !1;
                    if game.bet_y < 3 {
                        game.bet_x = 2;
                        game.bet_y = 1;
                    } else {
                        game.bet_y -= 2;
                    }
                } else if game.bet_x == 1 && game.bet_y < 3 {
                    game.bet_x = 2;
                    game.bet_y = 1;
                } else if game.bet_y > 0 {
                    game.bet_y -= 1;
                }
            } else if dy > 0 {
                if game.bet_x == 0 {
                    game.bet_y &= !1;
                    if game.bet_y < 6 {
                        game.bet_y += 2;
                    }
                } else if game.bet_y < 7 {
                    game.bet_y += 1;
                }
            }
            ((game.bet_x, game.bet_y) != before).then_some("SFX_POKEBALLS_PLACED_ON_TABLE")
        }
        VisibleCardFlipPhase::NotEnoughCoins => return Ok(()),
    };
    if let Some(sound) = sound {
        queue_visible_shell_sound_effect(runtime_shell, sound)?;
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn flip_visible_card(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let phase = runtime_shell
        .visible_card_flip
        .as_ref()
        .map(|game| (game.phase.clone(), game.yes_no_index, game.coins))
        .context("no Card Flip game is open")?;
    match phase {
        (VisibleCardFlipPhase::AskPlay, 1, _) | (VisibleCardFlipPhase::PlayAgain, 1, _) => {
            return close_visible_card_flip(runtime_shell);
        }
        (VisibleCardFlipPhase::AskPlay, _, _) => {
            return start_visible_card_flip_round(runtime_shell);
        }
        (VisibleCardFlipPhase::PlayAgain, _, _) => {
            return start_visible_card_flip_round(runtime_shell);
        }
        (VisibleCardFlipPhase::NotEnoughCoins, _, _) => {
            return close_visible_card_flip(runtime_shell);
        }
        (VisibleCardFlipPhase::ChooseCard, _, _) => {
            let game = runtime_shell.visible_card_flip.as_mut().unwrap();
            if !matches!(game.animation, VisibleCardFlipAnimation::Cycle { .. }) {
                return Ok(());
            }
            game.animation = VisibleCardFlipAnimation::SelectFlash { frame: 0 };
            queue_visible_shell_sound_effect(runtime_shell, "SFX_SLOT_MACHINE_START")?;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        (VisibleCardFlipPhase::Result, _, _) => {
            return acknowledge_visible_card_flip_result(runtime_shell);
        }
        (VisibleCardFlipPhase::Shuffled, _, _) => {
            return start_visible_card_flip_round(runtime_shell);
        }
        (VisibleCardFlipPhase::PlaceBet, _, _) => {}
    }
    runtime_shell
        .visible_card_flip
        .as_mut()
        .context("no Card Flip game is open")?
        .animation = VisibleCardFlipAnimation::WaitBeforeReveal;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn reveal_visible_card_flip(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (which_card, bet_x, bet_y) = runtime_shell
        .visible_card_flip
        .as_ref()
        .map(|game| (game.which_card, game.bet_x, game.bet_y))
        .context("no Card Flip game is open")?;
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_card_flip_input = Some(CardFlipInput::Reveal {
        which_card: u8::try_from(which_card).context("Card Flip side fits a byte")?,
        cursor_x: u8::try_from(bet_x).context("Card Flip bet x fits a byte")?,
        cursor_y: u8::try_from(bet_y).context("Card Flip bet y fits a byte")?,
    });
    let result = runtime_shell
        .shell
        .apply_declared_special_routine("CardFlip")?;
    let SpecialRoutineEffect::CardFlipRevealed {
        card_name,
        card_level,
        payout,
        deck,
        revealed,
        coins,
        ..
    } = result.outcome.effect
    else {
        anyhow::bail!("CardFlip returned a different special effect");
    };
    let game = runtime_shell
        .visible_card_flip
        .as_mut()
        .context("Card Flip closed during play")?;
    game.coins = coins;
    game.payout = payout;
    game.deck = deck;
    game.revealed = revealed;
    game.face_card = Some((card_name.clone(), card_level));
    game.phase = VisibleCardFlipPhase::Result;
    game.animation = VisibleCardFlipAnimation::WaitResult { payout };
    game.yes_no_index = 0;
    game.message = if payout > 0 {
        "YEAH!".to_string()
    } else {
        "DARN…".to_string()
    };
    queue_visible_shell_sound_effect(
        runtime_shell,
        if payout > 0 {
            "SFX_2ND_PLACE"
        } else {
            "SFX_WRONG"
        },
    )?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn acknowledge_visible_card_flip_result(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let animation = runtime_shell
        .visible_card_flip
        .as_ref()
        .map(|game| game.animation)
        .context("no Card Flip game is open")?;
    if animation != VisibleCardFlipAnimation::AwaitResult {
        return Ok(());
    }
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_card_flip_input = Some(CardFlipInput::AcknowledgeResult);
    let result = runtime_shell
        .shell
        .apply_declared_special_routine("CardFlip")?;
    if !matches!(
        result.outcome.effect,
        SpecialRoutineEffect::CardFlipResultAcknowledged { .. }
    ) {
        anyhow::bail!("CardFlip result acknowledgement returned a different special effect");
    }
    let game = runtime_shell.visible_card_flip.as_mut().unwrap();
    game.phase = VisibleCardFlipPhase::PlayAgain;
    game.animation = VisibleCardFlipAnimation::None;
    game.yes_no_index = 0;
    game.message = "WANT TO PLAY\nAGAIN?".to_string();
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn start_visible_card_flip_round(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let input = match runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .card_flip
        .as_ref()
        .map(|game| game.phase)
    {
        None | Some(CardFlipPhase::Quit) => CardFlipInput::Start,
        Some(CardFlipPhase::PlayAgain) => CardFlipInput::Continue,
        Some(CardFlipPhase::Shuffled) => CardFlipInput::ResumeAfterShuffle,
        Some(phase) => anyhow::bail!("CardFlip cannot start a round during phase {phase:?}"),
    };
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .pending_card_flip_input = Some(input);
    let result = runtime_shell
        .shell
        .apply_declared_special_routine("CardFlip")?;
    let (deck, revealed, coins) = match result.outcome.effect {
        SpecialRoutineEffect::CardFlipStarted {
            deck,
            revealed,
            coins,
            ..
        } => (deck, revealed, coins),
        SpecialRoutineEffect::CardFlipShuffled {
            deck,
            revealed,
            coins,
            ..
        } => {
            let game = runtime_shell
                .visible_card_flip
                .as_mut()
                .context("Card Flip closed during shuffle")?;
            game.phase = VisibleCardFlipPhase::Shuffled;
            game.animation = VisibleCardFlipAnimation::None;
            game.round = 0;
            game.face_card = None;
            game.coins = coins;
            game.payout = 0;
            game.deck = deck;
            game.revealed = revealed;
            game.message = "THE CARDS HAVE\nBEEN SHUFFLED.".to_string();
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        SpecialRoutineEffect::GameCornerGameUnavailable { .. } => {
            let game = runtime_shell
                .visible_card_flip
                .as_mut()
                .context("Card Flip closed during coin check")?;
            game.phase = VisibleCardFlipPhase::NotEnoughCoins;
            game.animation = VisibleCardFlipAnimation::None;
            game.message = "NOT ENOUGH COINS…".to_string();
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        _ => anyhow::bail!("CardFlip start returned a different special effect"),
    };
    let round = runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .card_flip
        .as_ref()
        .map_or(0, |state| usize::from(state.num_cards_played));
    let game = runtime_shell
        .visible_card_flip
        .as_mut()
        .context("Card Flip closed during start")?;
    game.phase = VisibleCardFlipPhase::ChooseCard;
    game.animation = VisibleCardFlipAnimation::WaitStake;
    game.yes_no_index = 0;
    game.payout = 0;
    game.face_card = None;
    game.coins = coins;
    game.round = round;
    game.deck = deck;
    game.revealed = revealed;
    game.message = "CHOOSE A CARD.".to_string();
    queue_visible_shell_sound_effect(runtime_shell, "SFX_TRANSACTION")?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_card_flip_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let animation = runtime_shell
        .visible_card_flip
        .as_ref()
        .map(|game| game.animation)
        .context("no Card Flip game is open")?;
    match animation {
        VisibleCardFlipAnimation::WaitStake => {
            if visible_wait_sfx_finished(runtime_shell) {
                runtime_shell.visible_card_flip.as_mut().unwrap().animation =
                    VisibleCardFlipAnimation::Deal { frame: 0 };
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleCardFlipAnimation::WaitBeforeReveal => {
            if visible_wait_sfx_finished(runtime_shell) {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_CHOOSE_A_CARD")?;
                runtime_shell.visible_card_flip.as_mut().unwrap().animation =
                    VisibleCardFlipAnimation::WaitReveal;
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleCardFlipAnimation::WaitReveal => {
            if visible_wait_sfx_finished(runtime_shell) {
                reveal_visible_card_flip(runtime_shell)?;
            }
            return Ok(());
        }
        VisibleCardFlipAnimation::WaitResult { payout } => {
            if visible_wait_sfx_finished(runtime_shell) {
                runtime_shell.visible_card_flip.as_mut().unwrap().animation = if payout > 0 {
                    VisibleCardFlipAnimation::Payout {
                        remaining: payout,
                        frames_until_coin: 0,
                    }
                } else {
                    VisibleCardFlipAnimation::AwaitResult
                };
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleCardFlipAnimation::QuitWaitBefore => {
            if visible_wait_sfx_finished(runtime_shell) {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_QUIT_SLOTS")?;
                runtime_shell.visible_card_flip.as_mut().unwrap().animation =
                    VisibleCardFlipAnimation::QuitWaitAfter;
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        VisibleCardFlipAnimation::QuitWaitAfter => {
            if visible_wait_sfx_finished(runtime_shell) {
                finish_closing_visible_card_flip(runtime_shell)?;
            }
            return Ok(());
        }
        VisibleCardFlipAnimation::None
        | VisibleCardFlipAnimation::Deal { .. }
        | VisibleCardFlipAnimation::Cycle { .. }
        | VisibleCardFlipAnimation::SelectFlash { .. }
        | VisibleCardFlipAnimation::Payout { .. }
        | VisibleCardFlipAnimation::AwaitResult => {}
    }
    if let VisibleCardFlipAnimation::Payout {
        remaining,
        frames_until_coin,
    } = animation
    {
        if frames_until_coin > 1 {
            runtime_shell.visible_card_flip.as_mut().unwrap().animation =
                VisibleCardFlipAnimation::Payout {
                    remaining,
                    frames_until_coin: frames_until_coin - 1,
                };
        } else if remaining == 0 {
            runtime_shell.visible_card_flip.as_mut().unwrap().animation =
                VisibleCardFlipAnimation::AwaitResult;
        } else {
            runtime_shell
                .shell
                .session_mut()
                .state
                .script_runtime
                .pending_card_flip_input = Some(CardFlipInput::PayoutFrame);
            let result = runtime_shell
                .shell
                .apply_declared_special_routine("CardFlip")?;
            let SpecialRoutineEffect::CardFlipPayout {
                coins_before,
                payout_remaining,
                coins,
                random_state_after: _,
            } = result.outcome.effect
            else {
                anyhow::bail!("CardFlip payout returned a different special effect");
            };
            let game = runtime_shell.visible_card_flip.as_mut().unwrap();
            game.coins = coins;
            game.animation = VisibleCardFlipAnimation::Payout {
                remaining: payout_remaining,
                frames_until_coin: 2,
            };
            if coins > coins_before {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_PAY_DAY")?;
            }
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }

    let mut sound = None;
    let game = runtime_shell.visible_card_flip.as_mut().unwrap();
    match animation {
        VisibleCardFlipAnimation::None | VisibleCardFlipAnimation::AwaitResult => return Ok(()),
        VisibleCardFlipAnimation::Deal { frame } if frame < 39 => {
            game.animation = VisibleCardFlipAnimation::Deal { frame: frame + 1 };
        }
        VisibleCardFlipAnimation::Deal { .. } => {
            game.animation = VisibleCardFlipAnimation::Cycle {
                frames_until_toggle: 4,
            };
            sound = Some("SFX_KINESIS");
        }
        VisibleCardFlipAnimation::Cycle {
            frames_until_toggle,
        } if frames_until_toggle > 1 => {
            game.animation = VisibleCardFlipAnimation::Cycle {
                frames_until_toggle: frames_until_toggle - 1,
            };
        }
        VisibleCardFlipAnimation::Cycle { .. } => {
            game.which_card ^= 1;
            game.animation = VisibleCardFlipAnimation::Cycle {
                frames_until_toggle: 4,
            };
            sound = Some("SFX_KINESIS");
        }
        VisibleCardFlipAnimation::SelectFlash { frame } if frame < 23 => {
            game.animation = VisibleCardFlipAnimation::SelectFlash { frame: frame + 1 };
        }
        VisibleCardFlipAnimation::SelectFlash { .. } => {
            game.phase = VisibleCardFlipPhase::PlaceBet;
            game.animation = VisibleCardFlipAnimation::None;
            game.message = "PLACE YOUR BET.".to_string();
        }
        VisibleCardFlipAnimation::Payout { .. } => unreachable!("handled above"),
        VisibleCardFlipAnimation::WaitStake
        | VisibleCardFlipAnimation::WaitBeforeReveal
        | VisibleCardFlipAnimation::WaitReveal
        | VisibleCardFlipAnimation::WaitResult { .. }
        | VisibleCardFlipAnimation::QuitWaitBefore
        | VisibleCardFlipAnimation::QuitWaitAfter => unreachable!("handled above"),
    }
    if let Some(sound) = sound {
        queue_visible_shell_sound_effect(runtime_shell, sound)?;
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn close_visible_card_flip(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let should_exit_core = runtime_shell
        .visible_card_flip
        .as_ref()
        .is_some_and(|game| game.phase == VisibleCardFlipPhase::PlayAgain);
    if let Some(game) = runtime_shell.visible_card_flip.as_ref() {
        match game.phase {
            VisibleCardFlipPhase::ChooseCard | VisibleCardFlipPhase::PlaceBet => return Ok(()),
            VisibleCardFlipPhase::Result => {
                return acknowledge_visible_card_flip_result(runtime_shell);
            }
            VisibleCardFlipPhase::Shuffled => {
                return start_visible_card_flip_round(runtime_shell);
            }
            VisibleCardFlipPhase::AskPlay
            | VisibleCardFlipPhase::PlayAgain
            | VisibleCardFlipPhase::NotEnoughCoins => {}
        }
    }
    if should_exit_core {
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .pending_card_flip_input = Some(CardFlipInput::Quit);
        let result = runtime_shell
            .shell
            .apply_declared_special_routine("CardFlip")?;
        if !matches!(
            result.outcome.effect,
            SpecialRoutineEffect::CardFlipExited { .. }
        ) {
            anyhow::bail!("CardFlip quit returned a different special effect");
        }
    }
    if let Some(game) = runtime_shell.visible_card_flip.as_mut() {
        if matches!(
            game.animation,
            VisibleCardFlipAnimation::QuitWaitBefore | VisibleCardFlipAnimation::QuitWaitAfter
        ) {
            return Ok(());
        }
        game.animation = VisibleCardFlipAnimation::QuitWaitBefore;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    Ok(())
}

fn finish_closing_visible_card_flip(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.visible_card_flip = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn open_visible_buena_prize_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("BuenaPrize")
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    visible_buena_prize_choices(&snapshot)?;
    runtime_shell.buena_prize_cursor = Some(MenuCursor {
        surface_id: "script:buena-prize".to_string(),
        option_index: 0,
    });
    set_shell_action_status(runtime_shell, "WHICH PRIZE WOULD YOU LIKE?");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn open_visible_buena_password_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("BuenasPassword")
    {
        return Ok(false);
    }
    record_visible_runtime_action(runtime_shell, "special:buena_password:open")?;
    let used = runtime_shell.shell.use_buena_password(None)?;
    let opened = activate_visible_special_routine_boundary(runtime_shell, &used.outcome.effect)?;
    anyhow::ensure!(opened, "BuenasPassword did not open its source choice menu");
    runtime_shell.last_audio_events.push(format!(
        "Buena password menu outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn open_visible_remember_password_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("AskRememberPassword")
    {
        return Ok(false);
    }
    record_visible_runtime_action(
        runtime_shell,
        format!("special:remember_password:{source_script}:{command_index}:open"),
    )?;
    runtime_shell.pending_remember_password = Some(PendingRememberPasswordPrompt {
        closing_frames: None,
    });
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "script:remember-password".to_string(),
        option_index: 0,
    });
    runtime_shell.special_boundary = None;
    set_shell_action_status(runtime_shell, "REMEMBER THE PASSWORD?");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn open_visible_battle_tower_challenge_menu_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("Menu_ChallengeExplanationCancel")
    {
        return Ok(false);
    }
    let raw_language = runtime_shell
        .shell
        .session()
        .state()
        .script_runtime
        .script_value
        .as_deref()
        .context("Battle Tower challenge menu is missing its wScriptVar language selector")?;
    let english = match raw_language {
        "1" | "TRUE" => true,
        "0" | "FALSE" => false,
        other => anyhow::bail!("Battle Tower challenge menu has invalid language selector {other}"),
    };
    record_visible_runtime_action(runtime_shell, "special:battle_tower_challenge_menu:open")?;
    let used = runtime_shell
        .shell
        .use_battle_tower_challenge_menu(english, None)?;
    let opened = activate_visible_special_routine_boundary(runtime_shell, &used.outcome.effect)?;
    anyhow::ensure!(
        opened,
        "Battle Tower challenge special did not open its source menu"
    );
    runtime_shell.last_audio_events.push(format!(
        "Battle Tower challenge menu outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn resolve_visible_battle_tower_challenge_menu(
    runtime_shell: &mut BevyRuntimeShell,
    cancelled: bool,
) -> Result<()> {
    let menu = runtime_shell
        .visible_battle_tower_challenge_menu
        .take()
        .context("no Battle Tower challenge menu is open")?;
    let selection = if cancelled {
        4
    } else {
        strict_readonly_cursor_index(
            &Some(menu.cursor),
            "script:battle-tower-challenge",
            if menu.english { 3 } else { 4 },
        )
        .context("Battle Tower challenge menu has no valid selection")? as u8
            + 1
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("special:battle_tower_challenge_menu:select:{selection}"),
    )?;
    let used = runtime_shell
        .shell
        .use_battle_tower_challenge_menu(menu.english, Some(selection))?;
    runtime_shell.last_audio_events.push(format!(
        "Battle Tower challenge menu selection={selection} checksum={:?}",
        used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn open_visible_battle_tower_room_menu_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("BattleTowerRoomMenu")
    {
        return Ok(false);
    }
    record_visible_runtime_action(runtime_shell, "special:battle_tower_room_menu:open")?;
    let used = runtime_shell
        .shell
        .use_battle_tower_room_menu(None, false)?;
    let opened = activate_visible_special_routine_boundary(runtime_shell, &used.outcome.effect)?;
    anyhow::ensure!(
        opened,
        "BattleTowerRoomMenu did not open its source level picker"
    );
    runtime_shell.last_audio_events.push(format!(
        "Battle Tower room menu outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn resolve_visible_battle_tower_room_level(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (selection, option_count) = {
        let menu = runtime_shell
            .visible_battle_tower_room_menu
            .as_ref()
            .context("no Battle Tower room menu is open")?;
        anyhow::ensure!(
            matches!(menu.phase, VisibleBattleTowerRoomMenuPhase::PickLevel),
            "Battle Tower room level selection is not active"
        );
        let option_count = menu.level_groups.len() + 1;
        let selected = strict_readonly_cursor_index(
            &Some(menu.cursor.clone()),
            "script:battle-tower-room",
            option_count,
        )
        .context("Battle Tower room menu has no valid cursor")?;
        (menu.level_groups.get(selected).copied(), option_count)
    };
    if selection.is_none() {
        runtime_shell
            .visible_battle_tower_room_menu
            .as_mut()
            .context("Battle Tower room menu disappeared")?
            .phase = VisibleBattleTowerRoomMenuPhase::ConfirmCancel { yes_no_index: 0 };
        set_shell_action_status(runtime_shell, "CANCEL BATTLE ROOM CHALLENGE?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let selection = selection.expect("checked source level row");
    record_visible_runtime_action(
        runtime_shell,
        format!("special:battle_tower_room_menu:select:{selection}:{option_count}"),
    )?;
    let used = runtime_shell
        .shell
        .use_battle_tower_room_menu(Some(selection), false)?;
    let retained = activate_visible_special_routine_boundary(runtime_shell, &used.outcome.effect)?;
    if retained {
        return Ok(());
    }
    runtime_shell.visible_battle_tower_room_menu = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn resolve_visible_battle_tower_room_cancel(
    runtime_shell: &mut BevyRuntimeShell,
    confirmed: bool,
) -> Result<()> {
    if !confirmed {
        let menu = runtime_shell
            .visible_battle_tower_room_menu
            .as_mut()
            .context("no Battle Tower room menu is open")?;
        menu.phase = VisibleBattleTowerRoomMenuPhase::PickLevel;
        set_shell_action_status(runtime_shell, "BATTLE ROOM LEVEL");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "special:battle_tower_room_menu:cancel")?;
    let used = runtime_shell.shell.use_battle_tower_room_menu(None, true)?;
    anyhow::ensure!(
        !activate_visible_special_routine_boundary(runtime_shell, &used.outcome.effect)?,
        "confirmed Battle Tower room cancel retained a source menu"
    );
    runtime_shell.visible_battle_tower_room_menu = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn resolve_visible_buena_password_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let menu = runtime_shell
        .visible_buena_password
        .take()
        .context("no Buena password menu is open")?;
    let selected = strict_readonly_cursor_index(
        &Some(menu.cursor),
        "script:buena-password",
        menu.options.len(),
    )
    .context("Buena password menu has no valid selection")?;
    let guess = menu.options[selected].clone();
    record_visible_runtime_action(
        runtime_shell,
        format!("special:buena_password:guess:{guess}"),
    )?;
    let used = runtime_shell
        .shell
        .use_buena_password(Some(guess.clone()))?;
    let matched = match &used.outcome.effect {
        SpecialRoutineEffect::BuenasPassword { matched, .. } => *matched,
        other => anyhow::bail!("Buena password returned unexpected effect {other:?}"),
    };
    runtime_shell.last_audio_events.push(format!(
        "Buena password guess={guess} matched={matched} checksum={:?}",
        used.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        if matched {
            "CORRECT PASSWORD"
        } else {
            "WRONG PASSWORD"
        },
    );
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn resolve_visible_buena_prize_selection(
    runtime_shell: &mut BevyRuntimeShell,
    cancelled: bool,
) -> Result<()> {
    if cancelled {
        runtime_shell.buena_prize_cursor = None;
        runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
            label: "BuenaComeAgainText".to_string(),
            details: vec!["Oh. Please come".to_string(), "back again!".to_string()],
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let choices = visible_buena_prize_choices(&snapshot)?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.buena_prize_cursor,
        "script:buena-prize",
        choices.len(),
    )
    .context("Buena prize menu has no valid selection")?;
    let item_id = choices[selected].0.clone();
    runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::BuenaPrize {
        item_id: item_id.clone(),
    });
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "pc:confirmation".to_string(),
        option_index: 0,
    });
    runtime_shell.pc_notice = Some(format!(
        "{}?\nIs that right?",
        item_display_name(&snapshot, &item_id)
    ));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_kurt_apricorn_choices(snapshot: &RuntimeShellSnapshot) -> Vec<(String, u16)> {
    KURT_APRICORN_ORDER
        .iter()
        .filter(|item_id| {
            snapshot
                .special
                .kurt_apricorn_recipes
                .contains_key(**item_id)
        })
        .filter_map(|item_id| {
            carried_item_quantity(snapshot, item_id)
                .filter(|quantity| *quantity > 0)
                .map(|quantity| ((*item_id).to_string(), quantity))
        })
        .collect()
}

fn open_visible_kurt_apricorn_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("SelectApricornForKurt")
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let choices = visible_kurt_apricorn_choices(&snapshot);
    record_visible_runtime_action(runtime_shell, "script:special:kurt_apricorn:open")?;
    if choices.is_empty() {
        let scripts = &mut runtime_shell.shell.session_mut().state.script_runtime;
        scripts.script_value = Some("0".to_string());
        scripts
            .variables
            .insert("wScriptVar".to_string(), "0".to_string());
        return Ok(false);
    }
    runtime_shell.kurt_apricorn_cursor = Some(MenuCursor {
        surface_id: "script:kurt-apricorn".to_string(),
        option_index: 0,
    });
    runtime_shell.kurt_apricorn_quantity = None;
    set_shell_action_status(runtime_shell, "WHICH APRICORN?");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn resolve_visible_kurt_apricorn_selection(
    runtime_shell: &mut BevyRuntimeShell,
    cancelled: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let choices = visible_kurt_apricorn_choices(&snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.kurt_apricorn_cursor,
        "script:kurt-apricorn",
        choices.len(),
    )
    .context("Kurt Apricorn selection has no valid cursor")?;
    if cancelled {
        runtime_shell.kurt_apricorn_cursor = None;
        runtime_shell.kurt_apricorn_quantity = None;
        record_visible_runtime_action(runtime_shell, "script:special:kurt_apricorn:cancel")?;
        let scripts = &mut runtime_shell.shell.session_mut().state.script_runtime;
        scripts.script_value = Some("0".to_string());
        scripts
            .variables
            .insert("wScriptVar".to_string(), "0".to_string());
    } else {
        let (apricorn_id, quantity) = choices[selected].clone();
        if runtime_shell.kurt_apricorn_quantity.is_none() {
            runtime_shell.kurt_apricorn_quantity = Some(1);
            set_shell_action_status(runtime_shell, "HOW MANY?");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        let quantity = runtime_shell
            .kurt_apricorn_quantity
            .take()
            .unwrap_or(1)
            .clamp(1, quantity);
        runtime_shell.kurt_apricorn_cursor = None;
        record_visible_runtime_action(
            runtime_shell,
            format!("script:special:kurt_apricorn:{apricorn_id}:{quantity}"),
        )?;
        let used = runtime_shell
            .shell
            .use_kurt_apricorn(apricorn_id, quantity)?;
        runtime_shell.last_audio_events.push(format!(
            "Kurt apricorn outcome={:?} checksum={:?}",
            used.outcome.effect, used.state_checksum
        ));
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn resolve_visible_script_party_selection(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: Option<usize>,
) -> Result<()> {
    let pending = runtime_shell
        .pending_script_party_selection
        .take()
        .context("no script party selection is pending")?;
    let pending_description = format!("{pending:?}");
    record_visible_runtime_action(
        runtime_shell,
        format!("script:special:party_selection:{pending_description}:{party_index:?}"),
    )?;
    let mut resume_immediately = true;
    let mut close_party_menu = true;
    match (pending, party_index) {
        (PendingScriptPartySelection::LinkTrade, party_index) => {
            runtime_shell.pending_link_trade_party_slot = Some(party_index);
            runtime_shell.last_action_status = Some(match party_index {
                Some(index) => format!("Link trade Pokemon {} selected", index + 1),
                None => "Link trade selection cancelled".to_string(),
            });
            resume_immediately = false;
        }
        (PendingScriptPartySelection::BillsGrandfather, Some(party_index)) => {
            let used = runtime_shell
                .shell
                .use_bills_grandfather(Some(party_index), None)?;
            runtime_shell.last_audio_events.push(format!(
                "script party selection {pending_description} outcome={:?} checksum={:?}",
                used.outcome.effect, used.state_checksum
            ));
        }
        (PendingScriptPartySelection::BillsGrandfather, None) => {
            let scripts = &mut runtime_shell.shell.session_mut().state.script_runtime;
            scripts.script_value = Some("0".to_string());
            scripts
                .variables
                .insert("wScriptVar".to_string(), "0".to_string());
            scripts.variables.remove("_selected_party_index");
            scripts.variables.remove("_selected_species");
            runtime_shell
                .last_audio_events
                .push("script party selection BillsGrandfather cancelled".to_string());
        }
        (PendingScriptPartySelection::ReturnShuckie, party_index) => {
            let used = runtime_shell
                .shell
                .use_shuckie(RuntimeShuckieAction::Return, party_index)?;
            runtime_shell.last_audio_events.push(format!(
                "script party selection {pending_description} outcome={:?} checksum={:?}",
                used.outcome.effect, used.state_checksum
            ));
        }
        (PendingScriptPartySelection::CheckMagikarpLength, Some(party_index)) => {
            let used = runtime_shell.shell.check_magikarp_length(party_index)?;
            runtime_shell.last_audio_events.push(format!(
                "script party selection {pending_description} outcome={:?} checksum={:?}",
                used.outcome.effect, used.state_checksum
            ));
            if matches!(
                &used.outcome.effect,
                crate::core::systems::special_routines::SpecialRoutineEffect::CheckMagikarpLength {
                    species,
                    ..
                } if species == "MAGIKARP"
            ) {
                let formatted = runtime_shell
                    .shell
                    .session()
                    .state()
                    .script_runtime
                    .named_buffers
                    .get("STRING_BUFFER_1")
                    .context("Magikarp measurement did not populate STRING_BUFFER_1")?
                    .clone();
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "MagikarpGuruMeasureText".to_string(),
                    details: vec![
                        "Let me measure\nthat MAGIKARP.".to_string(),
                        format!("…Hm, it measures\n{formatted}."),
                    ],
                });
                set_shell_action_status(runtime_shell, "MAGIKARP MEASURED");
                resume_immediately = false;
            }
        }
        (PendingScriptPartySelection::CheckMagikarpLength, None) => {
            let scripts = &mut runtime_shell.shell.session_mut().state.script_runtime;
            scripts.script_value = Some("1".to_string());
            scripts
                .variables
                .insert("wScriptVar".to_string(), "1".to_string());
            scripts
                .variables
                .insert("_value".to_string(), "1".to_string());
            scripts
                .variables
                .insert("_selection_cancelled".to_string(), "1".to_string());
            scripts.variables.remove("_selected_party_index");
            scripts.variables.remove("_selected_species");
            runtime_shell
                .last_audio_events
                .push("script party selection CheckMagikarpLength cancelled".to_string());
        }
        (PendingScriptPartySelection::PhotoStudio, Some(party_index)) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let pokemon = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| slot.pokemon.clone())
                .context("selected Photo Studio party slot is empty")?;
            runtime_shell.special_boundary_queue.clear();
            if pokemon.is_egg || pokemon.species.id == "EGG" {
                runtime_shell.pending_photo_studio_commit = None;
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "EggPhotoText",
                    "_EggPhotoText",
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
            } else {
                runtime_shell.pending_photo_studio_commit = Some(party_index);
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "HoldStillText",
                    "_HoldStillText",
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
            }
            resume_immediately = false;
        }
        (PendingScriptPartySelection::PhotoStudio, None) => {
            runtime_shell.pending_photo_studio_commit = None;
            runtime_shell.special_boundary_queue.clear();
            let mut boundaries = visible_exported_special_text_boundaries(
                runtime_shell,
                "NoPhotoText",
                "_NoPhotoText",
            )?;
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            resume_immediately = false;
        }
        (PendingScriptPartySelection::PokeSeer, Some(party_index)) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let slot = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .context("selected Poke Seer party slot is empty")?;
            let pokemon = slot.pokemon.clone();
            let used = runtime_shell.shell.see_party_pokemon_special(party_index)?;
            runtime_shell.last_audio_events.push(format!(
                "script party selection {pending_description} outcome={:?} checksum={:?}",
                used.outcome.effect, used.state_checksum
            ));
            let nickname = if pokemon.nickname.trim().is_empty() {
                canonical_species_display_name(&pokemon.species.id)
            } else {
                pokemon.nickname.clone()
            };
            let messages =
                visible_poke_seer_messages(runtime_shell, &snapshot, &pokemon, &nickname)?;
            let mut messages = messages.into_iter();
            let first = messages
                .next()
                .context("Poke Seer produced no visible text")?;
            runtime_shell.special_boundary_queue.clear();
            runtime_shell.special_boundary = Some(first);
            runtime_shell.special_boundary_queue.extend(messages);
            resume_immediately = false;
        }
        (PendingScriptPartySelection::PokeSeer, None) => {
            runtime_shell.special_boundary_queue.clear();
            let mut boundaries = visible_exported_special_text_boundaries(
                runtime_shell,
                "SeerDoNothingText",
                "_SeerDoNothingText",
            )?;
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            resume_immediately = false;
        }
        (PendingScriptPartySelection::NameRater, Some(party_index)) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let pokemon = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| slot.pokemon.clone())
                .context("selected Name Rater party slot is empty")?;
            let nickname = if pokemon.nickname.trim().is_empty() {
                canonical_species_display_name(&pokemon.species.id)
            } else {
                pokemon.nickname.clone()
            };
            runtime_shell.special_boundary_queue.clear();
            if pokemon.is_egg || pokemon.species.id == "EGG" {
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "NameRaterEggText",
                    "_NameRaterEggText",
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
            } else if pokemon.original_trainer_id != snapshot.trainer.player_id
                || pokemon.original_trainer_name != snapshot.trainer.player_name
            {
                let mut boundaries = visible_exported_special_text_boundaries_with_buffer(
                    runtime_shell,
                    "NameRaterPerfectNameText",
                    "_NameRaterPerfectNameText",
                    Some(&nickname),
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
            } else {
                runtime_shell.pending_script_party_selection =
                    Some(PendingScriptPartySelection::NameRater);
                let mut boundaries = visible_exported_special_text_boundaries_with_buffer(
                    runtime_shell,
                    "NameRaterBetterNameText",
                    "_NameRaterBetterNameText",
                    Some(&nickname),
                )?;
                let prompt = boundaries
                    .pop_back()
                    .context("Name Rater better-name text has no final yes/no page")?;
                runtime_shell.pc_notice = Some(
                    prompt
                        .details
                        .into_iter()
                        .next()
                        .context("Name Rater better-name yes/no page is empty")?,
                );
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::NameRaterRename);
                runtime_shell.yes_no_cursor = Some(MenuCursor {
                    surface_id: "pc:confirmation".to_string(),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "BETTER NAME?");
            }
            resume_immediately = false;
        }
        (PendingScriptPartySelection::NameRater, None) => {
            let mut boundaries = visible_exported_special_text_boundaries(
                runtime_shell,
                "NameRaterComeAgainText",
                "_NameRaterComeAgainText",
            )?;
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            resume_immediately = false;
        }
        (
            pending @ (PendingScriptPartySelection::OlderHaircutBrother
            | PendingScriptPartySelection::YoungerHaircutBrother
            | PendingScriptPartySelection::DaisysGrooming),
            party_index,
        ) => {
            let routine = match pending {
                PendingScriptPartySelection::OlderHaircutBrother => {
                    RuntimeHappinessServiceRoutine::OlderHaircutBrother
                }
                PendingScriptPartySelection::YoungerHaircutBrother => {
                    RuntimeHappinessServiceRoutine::YoungerHaircutBrother
                }
                PendingScriptPartySelection::DaisysGrooming => {
                    RuntimeHappinessServiceRoutine::DaisysGrooming
                }
                _ => unreachable!("happiness-service selection matched a different routine"),
            };
            match party_index {
                None => set_visible_script_numeric_value(runtime_shell, 0),
                Some(party_index) => {
                    let snapshot = runtime_shell.shell.snapshot()?;
                    let pokemon = snapshot
                        .party
                        .slots
                        .iter()
                        .find(|slot| slot.index == party_index)
                        .map(|slot| &slot.pokemon)
                        .context("selected happiness-service party slot is empty")?;
                    if pokemon.is_egg || pokemon.species.id == "EGG" {
                        set_visible_script_numeric_value(runtime_shell, 1);
                    } else {
                        let used = runtime_shell
                            .shell
                            .apply_happiness_service(routine, party_index)?;
                        runtime_shell.last_audio_events.push(format!(
                            "script party selection {pending_description} outcome={:?} checksum={:?}",
                            used.outcome.effect, used.state_checksum
                        ));
                    }
                }
            }
        }
        (PendingScriptPartySelection::DayCareDeposit { caretaker }, party_index) => {
            runtime_shell.special_boundary_queue.clear();
            let Some(party_index) = party_index else {
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "OhFineThenText",
                    "_OhFineThenText",
                )?;
                boundaries.extend(visible_exported_special_text_boundaries(
                    runtime_shell,
                    "ComeAgainText",
                    "_ComeAgainText",
                )?);
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                if close_party_menu {
                    close_visible_party_menu(runtime_shell);
                }
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            };
            let snapshot = runtime_shell.shell.snapshot()?;
            let slot = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .context("selected Day-Care party slot is empty")?;
            let pokemon = &slot.pokemon;
            let refusal = if snapshot.party.slots.len() < 2 {
                Some(("OnlyOneMonText", "_OnlyOneMonText"))
            } else if pokemon.is_egg || pokemon.species.id == "EGG" {
                Some(("CantAcceptEggText", "_CantAcceptEggText"))
            } else if pokemon
                .item
                .as_deref()
                .is_some_and(crate::core::models::item::is_mail_item_id)
            {
                Some(("RemoveMailText", "_RemoveMailText"))
            } else if snapshot
                .party
                .slots
                .iter()
                .filter(|other| {
                    other.index != party_index && other.pokemon.hp > 0 && !other.pokemon.is_egg
                })
                .count()
                == 0
            {
                Some(("LastHealthyMonText", "_LastHealthyMonText"))
            } else {
                None
            };
            if let Some((label, text_target)) = refusal {
                let mut boundaries =
                    visible_exported_special_text_boundaries(runtime_shell, label, text_target)?;
                boundaries.extend(visible_exported_special_text_boundaries(
                    runtime_shell,
                    "ComeAgainText",
                    "_ComeAgainText",
                )?);
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
            } else {
                let caretaker_kind = if caretaker == "man" {
                    RuntimeDayCareCaretaker::Man
                } else {
                    RuntimeDayCareCaretaker::Lady
                };
                let nickname = if pokemon.nickname.trim().is_empty() {
                    canonical_species_display_name(&pokemon.species.id)
                } else {
                    pokemon.nickname.clone()
                };
                let used = runtime_shell.shell.use_day_care(
                    caretaker_kind,
                    RuntimeDayCareAction::Deposit,
                    Some(party_index),
                )?;
                let mut boundaries = visible_exported_special_text_boundaries_with_buffer(
                    runtime_shell,
                    "IllRaiseYourMonText",
                    "_IllRaiseYourMonText",
                    Some(&nickname),
                )?;
                boundaries.extend(visible_exported_special_text_boundaries(
                    runtime_shell,
                    "ComeBackLaterText",
                    "_ComeBackLaterText",
                )?);
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                runtime_shell.pending_special_cry = Some(pokemon.species.id.clone());
                runtime_shell.last_audio_events.push(format!(
                    "day-care deposit outcome={:?}",
                    used.outcome.effect
                ));
            }
            resume_immediately = false;
        }
        (
            PendingScriptPartySelection::MoveTutor {
                move_id,
                party_index: None,
            },
            party_index,
        ) => {
            if party_index.is_none() {
                set_visible_script_numeric_value(runtime_shell, u8::MAX);
            } else {
                let party_index = party_index.unwrap();
                let snapshot = runtime_shell.shell.snapshot()?;
                let pokemon = snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == party_index)
                    .map(|slot| &slot.pokemon)
                    .context("selected Move Tutor party slot is empty")?;
                let nickname = if pokemon.nickname.trim().is_empty() {
                    canonical_species_display_name(&pokemon.species.id)
                } else {
                    pokemon.nickname.clone()
                };
                let already_knows = pokemon.moves.iter().any(|known| known.name == move_id);
                let incompatible = pokemon.is_egg
                    || pokemon.species.id == "EGG"
                    || !pokemon
                        .species
                        .tmhm_learnset
                        .iter()
                        .any(|candidate| candidate == &move_id);
                if already_knows || incompatible {
                    let (label, text_target) = if already_knows {
                        ("KnowsMoveText", "_KnowsMoveText")
                    } else {
                        ("TMHMNotCompatibleText", "_TMHMNotCompatibleText")
                    };
                    let mut boundaries = visible_move_tutor_text_boundaries(
                        runtime_shell,
                        label,
                        text_target,
                        &nickname,
                        &move_id,
                    )?;
                    runtime_shell.special_boundary = boundaries.pop_front();
                    runtime_shell.special_boundary_queue = boundaries;
                    runtime_shell.pending_script_party_selection =
                        Some(PendingScriptPartySelection::MoveTutor {
                            move_id,
                            party_index: None,
                        });
                    close_party_menu = false;
                    resume_immediately = false;
                } else if pokemon.moves.len() >= 4 {
                    let mut boundaries = visible_move_tutor_text_boundaries(
                        runtime_shell,
                        "AskForgetMoveText",
                        "_AskForgetMoveText",
                        &nickname,
                        &move_id,
                    )?;
                    runtime_shell.pc_notice = Some(
                        boundaries
                            .pop_back()
                            .context("Move Tutor forget prompt has no final yes/no page")?
                            .details
                            .into_iter()
                            .next()
                            .context("Move Tutor forget prompt final page is empty")?,
                    );
                    runtime_shell.special_boundary = boundaries.pop_front();
                    runtime_shell.special_boundary_queue = boundaries;
                    runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::MoveTutorForget {
                        move_id,
                        party_index,
                    });
                    runtime_shell.yes_no_cursor = Some(MenuCursor {
                        surface_id: "pc:confirmation".to_string(),
                        option_index: 0,
                    });
                    set_shell_action_status(runtime_shell, "DELETE A MOVE?");
                    close_party_menu = false;
                    resume_immediately = false;
                } else {
                    let learned_move = move_id.replace('_', " ");
                    let used = runtime_shell
                        .shell
                        .teach_party_move_special(party_index, move_id)?;
                    runtime_shell
                        .last_audio_events
                        .push(format!("move tutor outcome={:?}", used.outcome.effect));
                    set_visible_script_numeric_value(runtime_shell, 0);
                    let mut boundaries = visible_move_tutor_text_boundaries(
                        runtime_shell,
                        "LearnedMoveText",
                        "_LearnedMoveText",
                        &nickname,
                        &learned_move,
                    )?;
                    runtime_shell.special_boundary = boundaries.pop_front();
                    runtime_shell.special_boundary_queue = boundaries;
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_DEX_FANFARE_50_79")?;
                    resume_immediately = false;
                }
            }
        }
        (
            PendingScriptPartySelection::MoveTutor {
                move_id,
                party_index: Some(party_index),
            },
            Some(_),
        ) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let pokemon = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| &slot.pokemon)
                .context("Move Tutor party selection is no longer present")?;
            let nickname = if pokemon.nickname.trim().is_empty() {
                canonical_species_display_name(&pokemon.species.id)
            } else {
                pokemon.nickname.clone()
            };
            let move_index = strict_readonly_cursor_index(
                &runtime_shell.party_move_cursor,
                &party_move_cursor_surface_id(party_index),
                pokemon.moves.len(),
            )
            .context("Move Tutor move list has no valid selection")?;
            let forgotten_move = pokemon
                .moves
                .get(move_index)
                .map(|learned| learned.name.clone())
                .context("Move Tutor move selection is outside the moveset")?;
            let is_hm = snapshot.items.iter().any(|item| {
                !item.consumable && item.tmhm_move.as_deref() == Some(forgotten_move.as_str())
            });
            if is_hm {
                runtime_shell.pending_script_party_selection =
                    Some(PendingScriptPartySelection::MoveTutor {
                        move_id,
                        party_index: Some(party_index),
                    });
                let pending_move_id = match runtime_shell.pending_script_party_selection.as_ref() {
                    Some(PendingScriptPartySelection::MoveTutor { move_id, .. }) => move_id.clone(),
                    _ => unreachable!("Move Tutor selection was just retained"),
                };
                let mut boundaries = visible_move_tutor_text_boundaries(
                    runtime_shell,
                    "MoveCantForgetHMText",
                    "_MoveCantForgetHMText",
                    &nickname,
                    &pending_move_id,
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            runtime_shell.pc_notice = None;
            runtime_shell
                .shell
                .delete_party_move_special(party_index, move_index)?;
            let learned_move = move_id.replace('_', " ");
            let used = runtime_shell
                .shell
                .teach_party_move_special(party_index, move_id)?;
            runtime_shell.last_audio_events.push(format!(
                "move tutor replacement outcome={:?}",
                used.outcome.effect
            ));
            runtime_shell.party_move_cursor = None;
            set_visible_script_numeric_value(runtime_shell, 0);
            install_visible_move_learn_result_sequence(
                runtime_shell,
                &nickname,
                Some(&forgotten_move),
                &learned_move,
            )?;
            resume_immediately = false;
        }
        (
            PendingScriptPartySelection::MoveTutor {
                move_id,
                party_index: Some(party_index),
            },
            None,
        ) => {
            runtime_shell.party_move_cursor = None;
            let nickname = runtime_shell
                .shell
                .snapshot()?
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| {
                    if slot.pokemon.nickname.trim().is_empty() {
                        canonical_species_display_name(&slot.pokemon.species.id)
                    } else {
                        slot.pokemon.nickname.clone()
                    }
                })
                .context("Move Tutor party selection is no longer present")?;
            let mut boundaries = visible_move_tutor_text_boundaries(
                runtime_shell,
                "StopLearningMoveText",
                "_StopLearningMoveText",
                &nickname,
                &move_id,
            )?;
            runtime_shell.pc_notice = Some(
                boundaries
                    .pop_back()
                    .context("Move Tutor stop prompt rendered no source page")?
                    .details
                    .into_iter()
                    .next()
                    .context("Move Tutor stop prompt page is empty")?,
            );
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::MoveTutorStop {
                move_id,
                party_index,
            });
            runtime_shell.yes_no_cursor = Some(MenuCursor {
                surface_id: "pc:confirmation".to_string(),
                option_index: 0,
            });
            close_party_menu = false;
            resume_immediately = false;
        }
        (PendingScriptPartySelection::MoveDeletion { party_index: None }, Some(party_index)) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let pokemon = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| &slot.pokemon)
                .context("selected Move Deleter party slot is empty")?;
            if pokemon.is_egg || pokemon.species.id == "EGG" {
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "DeleterEggText",
                    "_DeleterEggText",
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                resume_immediately = false;
            } else if pokemon.moves.len() <= 1 {
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "MoveKnowsOneText",
                    "_MoveKnowsOneText",
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                resume_immediately = false;
            } else {
                runtime_shell.pending_script_party_selection =
                    Some(PendingScriptPartySelection::MoveDeletion {
                        party_index: Some(party_index),
                    });
                let mut boundaries = visible_exported_special_text_boundaries(
                    runtime_shell,
                    "DeleterAskWhichMoveText",
                    "_DeleterAskWhichMoveText",
                )?;
                runtime_shell.special_boundary = boundaries.pop_front();
                runtime_shell.special_boundary_queue = boundaries;
                set_shell_action_status(runtime_shell, "WHICH MOVE?");
                resume_immediately = false;
            }
        }
        (PendingScriptPartySelection::MoveDeletion { party_index: None }, None) => {
            let mut boundaries = visible_exported_special_text_boundaries(
                runtime_shell,
                "DeleterNoComeAgainText",
                "_DeleterNoComeAgainText",
            )?;
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            resume_immediately = false;
        }
        (
            PendingScriptPartySelection::MoveDeletion {
                party_index: Some(party_index),
            },
            Some(_),
        ) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let pokemon = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| &slot.pokemon)
                .context("Move Deleter party selection is no longer present")?;
            let move_index = strict_readonly_cursor_index(
                &runtime_shell.party_move_cursor,
                &party_move_cursor_surface_id(party_index),
                pokemon.moves.len(),
            )
            .context("Move Deleter move list has no valid selection")?;
            let move_name = battle_move_display_name(&snapshot, &pokemon.moves[move_index].name);
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::MoveDeletion {
                party_index,
                move_index,
            });
            runtime_shell.yes_no_cursor = Some(MenuCursor {
                surface_id: "pc:confirmation".to_string(),
                option_index: 0,
            });
            let mut boundaries = visible_exported_special_text_boundaries_with_buffer(
                runtime_shell,
                "AskDeleteMoveText",
                "_AskDeleteMoveText",
                Some(&move_name),
            )?;
            let prompt = boundaries
                .pop_back()
                .context("Move Deleter confirmation has no final yes/no page")?;
            runtime_shell.pc_notice = Some(
                prompt
                    .details
                    .into_iter()
                    .next()
                    .context("Move Deleter confirmation page is empty")?,
            );
            runtime_shell.special_boundary = boundaries.pop_front();
            runtime_shell.special_boundary_queue = boundaries;
            runtime_shell.party_move_cursor = None;
            resume_immediately = false;
        }
        (
            PendingScriptPartySelection::MoveDeletion {
                party_index: Some(_),
            },
            None,
        ) => {
            runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                label: "DeleterNoComeAgainText".to_string(),
                details: vec!["No? Come visit me".to_string(), "again.".to_string()],
            });
            runtime_shell.party_move_cursor = None;
            resume_immediately = false;
        }
        (
            PendingScriptPartySelection::CheckPokeMail {
                origin_map_name,
                source_script,
                command_index,
            },
            party_index,
        ) => {
            let mut inputs = explicit_compiled_script_runtime_inputs(
                runtime_shell,
                &source_script,
                command_index,
            )?;
            inputs.selected_party_index = party_index;
            let phone_inputs =
                explicit_compiled_script_phone_inputs(runtime_shell, &source_script, command_index);
            let stepped = runtime_shell.shell.step_compiled_script_command(
                &origin_map_name,
                &source_script,
                command_index,
                inputs,
                phone_inputs,
            )?;
            runtime_shell.last_audio_events.push(format!(
                "script party selection {pending_description} result={} checksum={:?}",
                stepped.mutation.result.result_tag(),
                stepped.mutation.state_checksum
            ));
            integrate_visible_script_mutation_outcome(runtime_shell, &stepped.mutation)?;
        }
        (
            PendingScriptPartySelection::NpcTrade {
                origin_map_name,
                source_script,
                command_index,
                trade_id,
            },
            party_index,
        ) => {
            let stages_trade = match party_index {
                Some(party_index) => {
                    visible_npc_trade_selection_matches(runtime_shell, &trade_id, party_index)?
                }
                None => false,
            };
            if stages_trade {
                let party_index = party_index.expect("matching NPC trade has a party index");
                runtime_shell.pending_npc_trade_commit = Some(PendingNpcTradeCommit {
                    origin_map_name,
                    source_script,
                    command_index,
                    trade_id,
                    party_index,
                });
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "NPCTradeCableText".to_string(),
                    details: vec![
                        "OK, connect the".to_string(),
                        "Game Link Cable.".to_string(),
                    ],
                });
                set_shell_action_status(runtime_shell, "CONNECT GAME LINK CABLE");
            } else {
                apply_visible_npc_trade_selection(
                    runtime_shell,
                    origin_map_name,
                    source_script,
                    command_index,
                    trade_id,
                    party_index,
                )?;
            }
            resume_immediately = false;
        }
    }
    if close_party_menu {
        close_visible_party_menu(runtime_shell);
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    if resume_immediately {
        continue_visible_script_after_prompt(runtime_shell)
    } else {
        Ok(())
    }
}

fn set_visible_script_numeric_value(runtime_shell: &mut BevyRuntimeShell, value: u8) {
    let value = value.to_string();
    let scripts = &mut runtime_shell.shell.session_mut().state.script_runtime;
    scripts.script_value = Some(value.clone());
    scripts
        .variables
        .insert("wScriptVar".to_string(), value.clone());
    scripts.variables.insert("_value".to_string(), value);
}

fn visible_unown_layout(layout: &[Vec<u8>]) -> Result<[[u8; 6]; 6]> {
    let mut result = [[0_u8; 6]; 6];
    anyhow::ensure!(
        layout.len() == 6,
        "Unown puzzle layout has {} rows",
        layout.len()
    );
    for (y, row) in layout.iter().enumerate() {
        anyhow::ensure!(
            row.len() == 6,
            "Unown puzzle row {y} has {} cells",
            row.len()
        );
        result[y].copy_from_slice(row);
    }
    Ok(result)
}

fn update_visible_unown_puzzle_from_effect(
    runtime_shell: &mut BevyRuntimeShell,
    effect: &SpecialRoutineEffect,
) -> Result<()> {
    let SpecialRoutineEffect::UnownPuzzle {
        puzzle_id,
        solved,
        layout,
        holding_piece,
        ..
    } = effect
    else {
        anyhow::bail!("Unown puzzle mutation returned {effect:?}");
    };
    let (cursor_x, cursor_y) = runtime_shell
        .visible_unown_puzzle
        .as_ref()
        .map(|puzzle| (puzzle.cursor_x, puzzle.cursor_y))
        .unwrap_or((0, 0));
    runtime_shell.visible_unown_puzzle = Some(VisibleUnownPuzzle {
        puzzle_id: puzzle_id.clone(),
        layout: visible_unown_layout(layout)?,
        holding_piece: *holding_piece,
        cursor_x,
        cursor_y,
        solved: *solved,
    });
    set_shell_action_status(
        runtime_shell,
        if *solved {
            "PUZZLE COMPLETE"
        } else {
            "UNOWN PUZZLE"
        },
    );
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn move_visible_unown_puzzle_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    dx: isize,
    dy: isize,
) -> Result<()> {
    let puzzle = runtime_shell
        .visible_unown_puzzle
        .as_mut()
        .context("no Unown puzzle is open")?;
    if puzzle.solved {
        return Ok(());
    }
    let (old_x, old_y) = (puzzle.cursor_x, puzzle.cursor_y);
    match (dx, dy) {
        (0, -1) if puzzle.cursor_y > 0 => puzzle.cursor_y -= 1,
        (0, 1)
            if puzzle.cursor_y < 5
                && !(puzzle.cursor_y == 4 && (1..=4).contains(&puzzle.cursor_x)) =>
        {
            puzzle.cursor_y += 1;
        }
        (-1, 0) if puzzle.cursor_x > 0 => {
            puzzle.cursor_x = if puzzle.cursor_y == 5 && puzzle.cursor_x == 5 {
                0
            } else {
                puzzle.cursor_x - 1
            };
        }
        (1, 0) if puzzle.cursor_x < 5 => {
            puzzle.cursor_x = if puzzle.cursor_y == 5 && puzzle.cursor_x == 0 {
                5
            } else {
                puzzle.cursor_x + 1
            };
        }
        _ => {}
    }
    if (puzzle.cursor_x, puzzle.cursor_y) == (old_x, old_y) {
        return Ok(());
    }
    let sound = if puzzle.holding_piece.is_some() {
        "SFX_MOVE_PUZZLE_PIECE"
    } else {
        "SFX_POUND"
    };
    queue_visible_shell_sound_effect(runtime_shell, sound)?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn use_visible_unown_puzzle_cell(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let puzzle = runtime_shell
        .visible_unown_puzzle
        .as_ref()
        .context("no Unown puzzle is open")?
        .clone();
    if puzzle.solved {
        runtime_shell.visible_unown_puzzle = None;
        runtime_shell
            .shell
            .session_mut()
            .state
            .script_runtime
            .active_menu = None;
        return continue_visible_script_after_prompt(runtime_shell);
    }
    let occupied = puzzle.layout[puzzle.cursor_y][puzzle.cursor_x] != 0;
    let action = match (puzzle.holding_piece, occupied) {
        (None, true) => "pickup",
        (Some(_), false) => "place",
        _ => {
            queue_visible_shell_sound_effect(runtime_shell, "SFX_WRONG")?;
            return Ok(());
        }
    };
    {
        let scripts = &mut runtime_shell.shell.session_mut().state.script_runtime;
        scripts.script_value = Some(puzzle.puzzle_id.clone());
        scripts
            .variables
            .insert("wScriptVar".to_string(), puzzle.puzzle_id.clone());
        scripts
            .variables
            .insert("_value".to_string(), puzzle.puzzle_id.clone());
        scripts
            .variables
            .insert("unown_action".to_string(), action.to_string());
        scripts
            .variables
            .insert("unown_x".to_string(), puzzle.cursor_x.to_string());
        scripts
            .variables
            .insert("unown_y".to_string(), puzzle.cursor_y.to_string());
    }
    let used = runtime_shell
        .shell
        .apply_declared_special_routine("UnownPuzzle")?;
    queue_visible_shell_sound_effect(
        runtime_shell,
        if action == "pickup" {
            "SFX_MEGA_KICK"
        } else {
            "SFX_PLACE_PUZZLE_PIECE_DOWN"
        },
    )?;
    update_visible_unown_puzzle_from_effect(runtime_shell, &used.outcome.effect)?;
    if runtime_shell
        .visible_unown_puzzle
        .as_ref()
        .is_some_and(|puzzle| puzzle.solved)
    {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_1ST_PLACE")?;
    }
    Ok(())
}

fn close_visible_unown_puzzle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.visible_unown_puzzle = None;
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .active_menu = None;
    set_visible_script_numeric_value(runtime_shell, 0);
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn move_visible_unown_printer(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let printer = runtime_shell
        .visible_unown_printer
        .as_mut()
        .context("no Unown Printer menu is open")?;
    printer.selected = wrapped_index(usize::from(printer.selected), 27, delta) as u8;
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn print_visible_unown_stamp(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = runtime_shell
        .visible_unown_printer
        .as_ref()
        .context("no Unown Printer menu is open")?
        .selected;
    runtime_shell.pc_notice =
        Some("Printer Error 2\n\nCheck the Game Boy\nPrinter Manual.".to_string());
    runtime_shell.last_audio_events.push(format!(
        "Unown stamp {} Game Boy Printer link unavailable: source Printer Error 2",
        selected + 1
    ));
    set_shell_action_status(runtime_shell, "PRINTER ERROR 2");
    mark_runtime_presentation_dirty(runtime_shell);
    Ok(())
}

fn close_visible_unown_printer(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.visible_unown_printer = None;
    runtime_shell
        .shell
        .session_mut()
        .state
        .script_runtime
        .active_menu = None;
    queue_visible_current_music(runtime_shell)?;
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn visible_poke_seer_messages(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    pokemon: &crate::core::models::Pokemon,
    nickname: &str,
) -> Result<Vec<SpecialBoundaryDisplay>> {
    let mut named_buffers = snapshot.script_events.named_buffers.clone();
    named_buffers.insert("wSeerNickname".to_string(), nickname.to_string());
    named_buffers.insert("wSeerOT".to_string(), pokemon.original_trainer_name.clone());
    let render = |label: &str, text_target: &str, named_buffers: &BTreeMap<String, String>| {
        visible_exported_special_text_boundaries_with_named_buffers(
            runtime_shell,
            label,
            text_target,
            named_buffers,
        )
        .map(VecDeque::into_iter)
        .map(Iterator::collect::<Vec<_>>)
    };
    if pokemon.is_egg || pokemon.species.id == "EGG" {
        return render("SeerEggText", "_SeerEggText", &named_buffers);
    }
    let Some(caught) = pokemon.caught_data.as_ref() else {
        return render(
            "SeerCantTellAThingText",
            "_SeerCantTellAThingText",
            &named_buffers,
        );
    };
    // ReadCaughtData ORs the two packed caught-data bytes and takes its
    // error branch when both are zero, even though the save slot exists.
    let packed_caught_data_is_zero = caught.level == 0
        && caught.time_of_day.is_none()
        && caught.original_trainer_gender == 0
        && caught.location == 0;
    if packed_caught_data_is_zero || caught.location == 0x7e {
        return render(
            "SeerCantTellAThingText",
            "_SeerCantTellAThingText",
            &named_buffers,
        );
    }
    let caught_level_value = match caught.level {
        0 => 0,
        1 => 5,
        level => level,
    };
    let caught_level = if caught_level_value == 0 {
        "???".to_string()
    } else {
        caught_level_value.to_string()
    };
    named_buffers.insert("wSeerCaughtLevelString".to_string(), caught_level.clone());
    let location = match caught.location {
        0x7f => None,
        0 => Some("Unknown".to_string()),
        location => snapshot
            .presentation
            .pokegear_landmarks
            .landmarks
            .iter()
            .find(|landmark| landmark.id == location as u16)
            .map(|landmark| landmark.name.clone())
            .or_else(|| Some("Unknown".to_string())),
    };
    if let Some(location) = location.as_ref() {
        named_buffers.insert("wSeerCaughtLocation".to_string(), location.clone());
    }
    let mut messages = if location.is_some() {
        // ReadCaughtData intentionally omits the low-byte `cp [hl]`, so the
        // cartridge classifies the OT using only the first (high) ID byte.
        if pokemon.original_trainer_id >> 8 == snapshot.trainer.player_id >> 8 {
            render(
                "SeerNameLocationText",
                "_SeerNameLocationText",
                &named_buffers,
            )?
        } else {
            render("SeerTradeText", "_SeerTradeText", &named_buffers)?
        }
    } else {
        render("SeerNoLocationText", "_SeerNoLocationText", &named_buffers)?
    };
    if location.is_some() {
        let caught_time = match caught.time_of_day {
            Some(crate::core::world::encounters::TimeOfDay::Morning) => "Morning",
            Some(crate::core::world::encounters::TimeOfDay::Day) => "Day",
            Some(crate::core::world::encounters::TimeOfDay::Night) => "Night",
            None => "Unknown",
        };
        named_buffers.insert("wSeerTimeOfDay".to_string(), caught_time.to_string());
        messages.extend(render(
            "SeerTimeLevelText",
            "_SeerTimeLevelText",
            &named_buffers,
        )?);
    }
    let gained = pokemon.level.saturating_sub(caught_level_value);
    let (label, text_target) = match gained {
        0..=9 => ("SeerMoreCareText", "_SeerMoreCareText"),
        10..=29 => ("SeerMoreConfidentText", "_SeerMoreConfidentText"),
        30..=59 => ("SeerMuchStrengthText", "_SeerMuchStrengthText"),
        60..=89 => ("SeerMightyText", "_SeerMightyText"),
        90..=100 => ("SeerImpressedText", "_SeerImpressedText"),
        _ => ("SeerMoreCareText", "_SeerMoreCareText"),
    };
    messages.extend(render(label, text_target, &named_buffers)?);
    Ok(messages)
}

fn apply_visible_npc_trade_selection(
    runtime_shell: &mut BevyRuntimeShell,
    origin_map_name: String,
    source_script: String,
    command_index: usize,
    trade_id: String,
    party_index: Option<usize>,
) -> Result<()> {
    let mut inputs =
        explicit_compiled_script_runtime_inputs(runtime_shell, &source_script, command_index)?;
    inputs.selected_party_index = party_index;
    let phone_inputs =
        explicit_compiled_script_phone_inputs(runtime_shell, &source_script, command_index);
    let stepped = runtime_shell.shell.step_compiled_script_command(
        &origin_map_name,
        &source_script,
        command_index,
        inputs,
        phone_inputs,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "NPC trade {trade_id} selection={party_index:?} result={} checksum={:?}",
        stepped.mutation.result.result_tag(),
        stepped.mutation.state_checksum
    ));
    let snapshot = runtime_shell.shell.snapshot()?;
    let result = snapshot
        .script_events
        .variables
        .get("_npc_trade_result")
        .map(String::as_str)
        .unwrap_or("0");
    let rule = snapshot
        .special
        .npc_trades
        .get(&trade_id)
        .with_context(|| format!("NPC trade {trade_id} is missing from the runtime snapshot"))?;
    runtime_shell.pc_notice = Some(visible_npc_trade_result_text(rule, result));
    Ok(())
}

fn visible_npc_trade_selection_matches(
    runtime_shell: &BevyRuntimeShell,
    trade_id: &str,
    party_index: usize,
) -> Result<bool> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let rule =
        snapshot.special.npc_trades.get(trade_id).with_context(|| {
            format!("NPC trade {trade_id} is missing from the runtime snapshot")
        })?;
    let pokemon = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| &slot.pokemon)
        .with_context(|| format!("NPC trade selected empty party slot {party_index}"))?;
    if pokemon.species.id != rule.requested_species {
        return Ok(false);
    }
    let female = match pokemon.species.gender_ratio {
        255 => None,
        254 => Some(true),
        0 => Some(false),
        ratio => Some(pokemon.dvs.attack.saturating_mul(17) < ratio),
    };
    match rule.gender_requirement.as_str() {
        "TRADE_GENDER_EITHER" => Ok(true),
        "TRADE_GENDER_FEMALE" => Ok(female == Some(true)),
        "TRADE_GENDER_MALE" => Ok(female == Some(false)),
        other => anyhow::bail!("NPC trade {trade_id} has unknown gender requirement {other}"),
    }
}

fn visible_npc_trade_intro_text(rule: &crystal_assets::NpcTradeRule) -> String {
    let requested = visible_npc_trade_requested_name(rule);
    let offered = rule.offered_species.replace('_', " ");
    if rule.dialog_set.ends_with("GIRL") {
        format!(
            "{requested}'s cute, but I don't have it.\nDo you have {requested}?\nWant to trade it for my {offered}?"
        )
    } else if rule.dialog_set.ends_with("HAPPY") || rule.dialog_set.ends_with("NEWBIE") {
        format!(
            "Hi, I'm looking for this POKéMON.\nIf you have {requested}, would you trade it for my {offered}?"
        )
    } else {
        format!("I collect POKéMON.\nDo you have {requested}?\nWant to trade it for my {offered}?")
    }
}

fn visible_npc_trade_result_text(rule: &crystal_assets::NpcTradeRule, result: &str) -> String {
    let requested = visible_npc_trade_requested_name(rule);
    let dialog_set = rule.dialog_set.as_str();
    match (result, dialog_set) {
        ("2", set) if set.ends_with("NEWBIE") => "Uh? What happened?".to_string(),
        ("2", set) if set.ends_with("GIRL") => {
            format!("Wow! Thank you!\nI always wanted {requested}!")
        }
        ("2", set) if set.ends_with("HAPPY") => {
            format!("Great! Thank you!\nI finally got {requested}.")
        }
        ("2", _) => format!("Yay! I got myself\n{requested}! Thanks!"),
        ("1", set) if set.ends_with("GIRL") => {
            format!("That's not {requested}.\nPlease trade with me if you get one.")
        }
        ("1", set) if set.ends_with("HAPPY") || set.ends_with("NEWBIE") => {
            format!("You don't have {requested}? That's too bad, then.")
        }
        ("1", _) => format!("Huh? That's not {requested}.\nWhat a letdown…"),
        (_, set) if set.ends_with("GIRL") => "You don't want to trade? Oh, darn…".to_string(),
        (_, set) if set.ends_with("HAPPY") || set.ends_with("NEWBIE") => {
            "You don't have one either?\nGee, that's really disappointing…".to_string()
        }
        _ => "You don't want to trade? Aww…".to_string(),
    }
}

fn visible_completed_npc_trade_text(rule: &crystal_assets::NpcTradeRule) -> String {
    let requested = visible_npc_trade_requested_name(rule);
    let offered = rule.offered_species.replace('_', " ");
    if rule.dialog_set.ends_with("NEWBIE") {
        "Trading is so odd…\nI still have a lot to learn about it.".to_string()
    } else if rule.dialog_set.ends_with("GIRL") {
        format!("How is that {offered} I traded you doing?\n\nYour {requested}'s so cute!")
    } else if rule.dialog_set.ends_with("HAPPY") {
        format!("Hi! The {requested} you traded me is doing great!")
    } else {
        format!("Hi, how's my old {offered} doing?")
    }
}

fn visible_npc_trade_requested_name(rule: &crystal_assets::NpcTradeRule) -> String {
    let mut requested = rule.requested_species.replace('_', " ");
    match rule.gender_requirement.as_str() {
        "TRADE_GENDER_MALE" => requested.push('♂'),
        "TRADE_GENDER_FEMALE" => requested.push('♀'),
        _ => {}
    }
    requested
}

fn apply_visible_name_rival(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<()> {
    record_visible_runtime_action(
        runtime_shell,
        format!("script:special:name_rival:{source_script}:{command_index}:open"),
    )?;
    runtime_shell.pending_name_input = Some(PendingNameInput {
        label: "RIVAL'S NAME?".to_string(),
        value: String::new(),
        max_length: VISIBLE_NAME_ENTRY_MAX_LENGTH,
        cursor_column: 0,
        cursor_row: 0,
        case: NameInputCase::Upper,
    });
    runtime_shell.last_audio_events.push(format!(
        "opened rival naming screen source={source_script} command={command_index}"
    ));
    set_shell_action_status(runtime_shell, "RIVAL NAME");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn check_visible_pokerus(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:pokerus:check")?;
    let special = runtime_shell.shell.check_pokerus_special()?;
    runtime_shell.last_audio_events.push(format!(
        "pokerus outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_poke_seer(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    record_visible_runtime_action(runtime_shell, format!("special:poke_seer:{party_index}"))?;
    let special = runtime_shell.shell.see_party_pokemon_special(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "poke seer party_index={} outcome={:?} checksum={:?}",
        party_index, special.outcome.effect, special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_selected_service_menu_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, routine) = selected_declared_special_routine(
        runtime_shell,
        "service menu",
        &snapshot.special.special_routines,
        &[
            "BankOfMom",
            "SlotMachine",
            "CardFlip",
            "DisplayLinkRecord",
            "TrainerHouse",
            "PhotoStudio",
            "Menu_ChallengeExplanationCancel",
        ],
    )?;
    let routine_id = routine.to_string();
    record_visible_runtime_action(runtime_shell, format!("special:service_menu:{routine_id}"))?;
    let special = match routine_id.as_str() {
        "BankOfMom" => runtime_shell.shell.open_bank_of_mom_special()?,
        "SlotMachine" => runtime_shell
            .shell
            .open_game_corner_special(RuntimeGameCornerService::SlotMachine)?,
        "CardFlip" => runtime_shell
            .shell
            .open_game_corner_special(RuntimeGameCornerService::CardFlip)?,
        "DisplayLinkRecord" => runtime_shell.shell.open_display_link_record_special()?,
        "TrainerHouse" => runtime_shell.shell.open_trainer_house_special()?,
        "PhotoStudio" => {
            let party_index = selected_party_index(runtime_shell)?;
            runtime_shell.shell.open_photo_studio_special(party_index)?
        }
        "Menu_ChallengeExplanationCancel" => runtime_shell
            .shell
            .use_battle_tower_challenge_menu(true, None)?,
        _ => unreachable!("selected service routine comes from the static candidate list"),
    };
    runtime_shell.last_audio_events.push(format!(
        "service menu {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        routine_id,
        special.outcome.effect,
        special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn apply_selected_time_money_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, routine) = selected_declared_special_routine(
        runtime_shell,
        "time/money",
        &snapshot.special.special_routines,
        &[
            "SetDayOfWeek",
            "InitialSetDSTFlag",
            "InitialClearDSTFlag",
            "UpdateTime",
            "UnusedCheckUnusedTwoDayTimer",
            "SampleKenjiBreakCountdown",
            "CheckLuckyNumberShowFlag",
            "ResetLuckyNumberShowFlag",
            "CheckForLuckyNumberWinners",
            "PlaceMoneyTopRight",
            "DisplayMoneyAndCoinBalance",
            "DisplayCoinCaseBalance",
            "PrintTodaysLuckyNumber",
            "GSHealings",
            "StubbedTrainerRankings_Healings",
            "Reset",
            "HoOhChamber",
        ],
    )?;
    let routine_id = routine.to_string();
    record_visible_runtime_action(runtime_shell, format!("special:time_money:{routine_id}"))?;
    let used = runtime_shell
        .shell
        .apply_declared_special_routine(&routine_id)?;
    runtime_shell.last_audio_events.push(format!(
        "time/money {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        routine_id,
        used.outcome.effect,
        used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn apply_selected_story_gate_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "story gate",
        &snapshot.special.special_routines,
        &[
            RuntimeStoryGateSpecial::CheckCaughtCelebi,
            RuntimeStoryGateSpecial::CelebiShrineEvent,
            RuntimeStoryGateSpecial::SnorlaxAwake,
            RuntimeStoryGateSpecial::CheckForBattleTowerRules,
        ],
        |special| special.routine(),
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:story_gate:{}", special.routine()),
    )?;
    let used = runtime_shell.shell.apply_story_gate_special(special)?;
    runtime_shell.last_audio_events.push(format!(
        "story gate {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        used.outcome.effect,
        used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn apply_selected_graphics_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "graphics",
        &snapshot.special.special_routines,
        &[
            RuntimeGraphicsSpecial::ClearBgPalettesBufferScreen,
            RuntimeGraphicsSpecial::ClearBgPalettes,
            RuntimeGraphicsSpecial::UpdateTimePals,
            RuntimeGraphicsSpecial::ClearTilemap,
            RuntimeGraphicsSpecial::LoadMapPalettes,
            RuntimeGraphicsSpecial::RefreshSprites,
            RuntimeGraphicsSpecial::UpdateSprites,
            RuntimeGraphicsSpecial::ReloadSpritesNoPalettes,
            RuntimeGraphicsSpecial::FadeOutToWhite,
            RuntimeGraphicsSpecial::FadeInFromWhite,
            RuntimeGraphicsSpecial::FadeOutToBlack,
            RuntimeGraphicsSpecial::FadeInFromBlack,
            RuntimeGraphicsSpecial::GameboyCheck,
            RuntimeGraphicsSpecial::CheckMobileAdapterStatus,
            RuntimeGraphicsSpecial::BattleTowerFade,
            RuntimeGraphicsSpecial::UpdatePlayerSprite,
            RuntimeGraphicsSpecial::HealMachineAnim,
            RuntimeGraphicsSpecial::SurfStartStep,
            RuntimeGraphicsSpecial::LoadUsedSpritesGfx,
            RuntimeGraphicsSpecial::ToggleMaptileDecorations,
            RuntimeGraphicsSpecial::ToggleDecorationsVisibility,
            RuntimeGraphicsSpecial::MagnetTrain,
            RuntimeGraphicsSpecial::Diploma,
            RuntimeGraphicsSpecial::PrintDiploma,
            RuntimeGraphicsSpecial::OmanyteChamber,
            RuntimeGraphicsSpecial::DisplayUnownWords,
        ],
        |special| special.routine(),
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:graphics:{}", special.routine()),
    )?;
    let used = runtime_shell.shell.apply_graphics_special(special)?;
    runtime_shell.last_audio_events.push(format!(
        "graphics {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        used.outcome.effect,
        used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn apply_selected_party_check_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "party check",
        &snapshot.special.special_routines,
        &[
            RuntimePartyCheckSpecial::CheckFirstMonIsEgg,
            RuntimePartyCheckSpecial::GetFirstPokemonHappiness,
            RuntimePartyCheckSpecial::FindPartyMonThatSpecies,
            RuntimePartyCheckSpecial::FindPartyMonAboveLevel,
            RuntimePartyCheckSpecial::FindPartyMonAtLeastThatHappy,
            RuntimePartyCheckSpecial::FindPartyMonThatSpeciesYourTrainerId,
            RuntimePartyCheckSpecial::MonCheck,
            RuntimePartyCheckSpecial::BeastsCheck,
            RuntimePartyCheckSpecial::GameCornerPrizeMonCheckDex,
            RuntimePartyCheckSpecial::UnusedSetSeenMon,
        ],
        |special| special.routine(),
    )?;
    let species_id = if special.requires_species() {
        Some(selected_pokedex_species_id(runtime_shell)?)
    } else {
        None
    };
    let threshold = if special.requires_threshold() {
        Some(((runtime_shell.script_command_cursor % 100) + 1) as u8)
    } else {
        None
    };
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "special:party_check:{}:{species_id:?}:{threshold:?}",
            special.routine()
        ),
    )?;
    let used =
        runtime_shell
            .shell
            .apply_party_check_special(special, species_id.clone(), threshold)?;
    runtime_shell.last_audio_events.push(format!(
        "party check {}/{} routine={} species={:?} threshold={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        species_id,
        threshold,
        used.outcome.effect,
        used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn apply_selected_phone_random_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "phone random",
        &snapshot.special.special_routines,
        &[
            RuntimePhoneRandomSpecial::RandomUnseenWildMon,
            RuntimePhoneRandomSpecial::RandomPhoneWildMon,
            RuntimePhoneRandomSpecial::RandomPhoneMon,
        ],
        |special| special.routine(),
    )?;
    let (contact_index, contact_len, contact_id) = selected_btree_key(
        runtime_shell,
        "phone contacts",
        &snapshot.special.phone_contacts.0,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:phone_random:{}:{contact_id}", special.routine()),
    )?;
    let used = runtime_shell
        .shell
        .apply_phone_random_special(special, contact_id.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "phone random {}/{} routine={} contact={}/{} {} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        contact_index + 1,
        contact_len,
        contact_id,
        used.outcome.effect,
        used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn check_selected_item_in_pc_or_bag_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot
        .special
        .special_routines
        .contains_key("UnusedFindItemInPCOrBag")
    {
        anyhow::bail!("compiled pack declares no PC/bag item check special");
    }
    let item_id = selected_bag_or_pc_item_id(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:item_in_pc_or_bag:{item_id}"),
    )?;
    let used = runtime_shell
        .shell
        .check_item_in_pc_or_bag_special(item_id.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "pc/bag item check item={} outcome={:?} checksum={:?}",
        item_id, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn activate_visible_fishing_swarm_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot
        .special
        .special_routines
        .contains_key("ActivateFishingSwarm")
    {
        anyhow::bail!("compiled pack declares no fishing swarm special");
    }
    let value = ((runtime_shell.script_command_cursor % 255) + 1) as u8;
    record_visible_runtime_action(runtime_shell, format!("special:fishing_swarm:{value}"))?;
    let used = runtime_shell.shell.activate_fishing_swarm_special(value)?;
    runtime_shell.last_audio_events.push(format!(
        "fishing swarm value={} outcome={:?} checksum={:?}",
        value, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn apply_selected_day_care_status_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, routine) = selected_declared_special_routine(
        runtime_shell,
        "Day Care status",
        &snapshot.special.special_routines,
        &["DayCareManOutside", "DayCareMon1", "DayCareMon2"],
    )?;
    let routine_id = routine.to_string();
    record_visible_runtime_action(
        runtime_shell,
        format!("special:day_care_status:{routine_id}"),
    )?;
    let special = match routine_id.as_str() {
        "DayCareManOutside" => runtime_shell.shell.check_day_care_man_outside_special()?,
        "DayCareMon1" => runtime_shell
            .shell
            .check_day_care_resident_special(RuntimeDayCareCaretaker::Man)?,
        "DayCareMon2" => runtime_shell
            .shell
            .check_day_care_resident_special(RuntimeDayCareCaretaker::Lady)?,
        _ => unreachable!("selected Day Care routine comes from the static candidate list"),
    };
    runtime_shell.last_audio_events.push(format!(
        "day care status {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        routine_id,
        special.outcome.effect,
        special.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &special.outcome.effect)?;
    Ok(())
}

fn drain_runtime_audio_events(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    if !runtime_shell.shell.has_pending_audio_events() {
        return;
    }
    match runtime_shell.shell.drain_resolved_audio_events() {
        Ok(drain) => apply_resolved_audio_drain(&mut runtime_shell, drain),
        Err(error) => {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!("failed to drain resolved runtime audio events: {error:#}"),
            );
        }
    }
}

fn apply_resolved_audio_drain(
    runtime_shell: &mut BevyRuntimeShell,
    drain: crate::RuntimeResolvedAudioEventDrain,
) {
    let event_count = drain.events.len();
    set_visible_runtime_action_from_checksum(
        runtime_shell,
        format!("audio:drain:{event_count}"),
        &drain.state_checksum,
    );
    runtime_shell.last_audio_events.push(format!(
        "drained resolved audio events={} checksum={:?}",
        event_count, drain.state_checksum
    ));
    let mut pending_audio = Vec::new();
    for event in drain.events {
        if let Some(action) = bevy_audio_action(&event.kind) {
            apply_pending_audio_action(
                runtime_shell,
                action,
                &mut pending_audio,
                &drain.state_checksum,
            );
        }
        runtime_shell.last_audio_events.push(format!(
            "audio event {:?} resolved={:?}",
            event.event, event.kind
        ));
    }
    runtime_shell.pending_audio.extend(pending_audio);
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn queue_visible_egg_hatch_music(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MUSIC_ID: &str = "MUSIC_EVOLUTION";
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, MUSIC_ID)?;
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: MUSIC_ID.to_string(),
            kind: ModpackAudioKind::Music,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    runtime_shell.pending_music_stop = true;
    runtime_shell.active_music = Some(MUSIC_ID.to_string());
    runtime_shell.faded_music = None;
    runtime_shell
        .last_audio_events
        .push("queued egg-hatch music MUSIC_EVOLUTION".to_string());
    Ok(())
}

fn begin_visible_egg_hatch_animation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(hatch) = runtime_shell.visible_egg_hatch.as_mut() else {
        anyhow::bail!("no egg hatch is awaiting its animation");
    };
    if hatch.phase != VisibleEggHatchPhase::HuhText {
        return Ok(());
    }
    hatch.phase = VisibleEggHatchPhase::EggHold;
    hatch.frame = 0;
    queue_visible_egg_hatch_music(runtime_shell)?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_egg_hatch(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mut hatch) = runtime_shell.visible_egg_hatch.take() else {
        return Ok(());
    };
    match hatch.phase {
        VisibleEggHatchPhase::HuhText | VisibleEggHatchPhase::HatchText => {
            runtime_shell.visible_egg_hatch = Some(hatch);
            return Ok(());
        }
        VisibleEggHatchPhase::EggHold => {
            hatch.frame += 1;
            if hatch.frame == 80 {
                hatch.phase = VisibleEggHatchPhase::Wobble;
                hatch.frame = 0;
            }
        }
        VisibleEggHatchPhase::Wobble => {
            hatch.frame += 1;
            if visible_egg_crack_at(hatch.frame) {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_EGG_CRACK")?;
            }
            if hatch.frame == 344 {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_EGG_HATCH")?;
                queue_visible_shell_sound_effect(runtime_shell, "SFX_EGG_HATCH")?;
                hatch.phase = VisibleEggHatchPhase::Shell;
                hatch.frame = 0;
            }
        }
        VisibleEggHatchPhase::Shell => {
            hatch.frame += 1;
            if hatch.frame == 130 {
                let snapshot = runtime_shell.shell.snapshot()?;
                snapshot
                    .presentation
                    .pokemon_frontpic_anim
                    .get(&hatch.species_id)
                    .with_context(|| {
                        format!(
                            "missing exported hatch frontpic animation for {}",
                            hatch.species_id
                        )
                    })?;
                runtime_shell.visible_frontpic_animation = Some(VisibleFrontpicAnimation {
                    species_id: hatch.species_id.clone(),
                    speed: 0,
                    pointer: 0,
                    repeat: 0,
                    wait: 0,
                    frame: 0,
                });
                hatch.phase = VisibleEggHatchPhase::Reveal;
                hatch.frame = 0;
            }
        }
        VisibleEggHatchPhase::Reveal => {
            if runtime_shell.visible_frontpic_animation.is_none() {
                let display = crate::core::models::pokemon_species_display_name(&hatch.species_id);
                runtime_shell.field_notice = Some(format!("{display} came\nout of its EGG!"));
                runtime_shell.pending_field_notice_sound = Some("SFX_CAUGHT_MON".to_string());
                hatch.phase = VisibleEggHatchPhase::HatchText;
            }
        }
    }
    runtime_shell.visible_egg_hatch = Some(hatch);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_egg_wobble_x(frame: u16) -> i8 {
    let mut elapsed = 0_u16;
    for repetition_count in 1_u16..=8 {
        let wobble_frames = repetition_count * 6;
        if frame < elapsed + wobble_frames {
            return if ((frame - elapsed) / 3) % 2 == 0 {
                -2
            } else {
                2
            };
        }
        elapsed += wobble_frames + 16;
        if frame < elapsed {
            return 0;
        }
    }
    0
}

fn visible_egg_crack_at(frame: u16) -> bool {
    matches!(frame, 50 | 124 | 222)
}

fn apply_pending_audio_action(
    runtime_shell: &mut BevyRuntimeShell,
    action: BevyAudioAction,
    pending_audio: &mut Vec<BevyAudioCommand>,
    state_checksum: &StateChecksum,
) {
    match action {
        BevyAudioAction::StopMusic { audio_id } => {
            set_visible_stopped_music_state(runtime_shell, Some(&audio_id));
            clear_pending_music_commands(pending_audio);
            set_visible_runtime_action_from_checksum(
                runtime_shell,
                format!("audio:stop_music:{audio_id}"),
                state_checksum,
            );
        }
        BevyAudioAction::Play(command) => {
            set_visible_runtime_action_from_checksum(
                runtime_shell,
                format!("audio:play:{:?}:{}", command.kind, command.audio_id),
                state_checksum,
            );
            if matches!(command.kind, ModpackAudioKind::Music) {
                runtime_shell.music_fade = None;
                runtime_shell.music_volume = 7;
                runtime_shell.pending_music_stop = true;
                runtime_shell.active_music = Some(command.audio_id.clone());
                runtime_shell.faded_music = None;
                clear_pending_music_commands(&mut runtime_shell.pending_audio);
                // `pending_audio` is the local batch being built from this
                // drain. A script can emit several map-music changes before
                // Bevy's playback system runs; retaining the earlier local
                // commands starts multiple native songs in one update.
                clear_pending_music_commands(pending_audio);
            }
            enqueue_bevy_audio_command(pending_audio, command);
        }
        BevyAudioAction::FadeMusic {
            audio_id,
            fade_frames,
        } => {
            let effective_rate = u8::try_from(fade_frames).map(|rate| rate & 0x3f);
            if runtime_shell.music_fade.as_ref().is_none_or(|fade| {
                effective_rate
                    .map(|rate| fade.target_music != audio_id || fade.rate != rate)
                    .unwrap_or(true)
            }) && let Err(error) =
                begin_visible_music_fade(runtime_shell, &audio_id, fade_frames)
            {
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
            set_visible_runtime_action_from_checksum(
                runtime_shell,
                format!("audio:fade_music:{audio_id}:{fade_frames}"),
                state_checksum,
            );
            runtime_shell
                .last_audio_events
                .push(format!("queued music fade {audio_id} frames={fade_frames}"));
        }
        BevyAudioAction::WaitForSoundEffect => {
            // `waitsfx` is a sequencing fence, not a stop command. In a
            // single drained batch it commonly follows `playsound`; clearing
            // either queue here discarded the fanfare before playback.
            set_visible_runtime_action_from_checksum(
                runtime_shell,
                "audio:wait_sfx".to_string(),
                state_checksum,
            );
            runtime_shell
                .last_audio_events
                .push("resolved wait for sound effect".to_string());
        }
    }
}

fn sync_runtime_title_music(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    if runtime_shell.intro_screen.is_some() {
        return;
    }
    let Some(title) = runtime_shell.title_menu.as_ref() else {
        return;
    };
    if runtime_shell.credits_screen.is_some()
        || matches!(
            title.source_phase(),
            VisibleTitlePhase::Entrance
                | VisibleTitlePhase::MainMenu
                | VisibleTitlePhase::Teardown
        )
    {
        return;
    }
    if let Err(error) = queue_runtime_title_music(&mut runtime_shell) {
        record_visible_runtime_system_error(
            &mut runtime_shell,
            anyhow::anyhow!("failed to queue title music: {error:#}"),
        );
    }
}

fn is_silent_music_id(music_id: &str) -> bool {
    music_id == "MUSIC_NONE"
}

fn stop_visible_music(
    runtime_shell: &mut BevyRuntimeShell,
    action: impl Into<String>,
) -> Result<()> {
    set_visible_stopped_music_state(runtime_shell, None);
    record_visible_runtime_action(runtime_shell, action)?;
    runtime_shell
        .last_audio_events
        .push("queued music stop".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn stop_visible_music_with_checksum(
    runtime_shell: &mut BevyRuntimeShell,
    action: impl Into<String>,
    state_checksum: &StateChecksum,
) -> Result<()> {
    set_visible_stopped_music_state(runtime_shell, None);
    set_visible_runtime_action_from_checksum(runtime_shell, action, state_checksum);
    runtime_shell
        .last_audio_events
        .push("queued music stop".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn stop_visible_silent_music(
    runtime_shell: &mut BevyRuntimeShell,
    music_id: &str,
    action: impl Into<String>,
) -> Result<()> {
    set_visible_stopped_music_state(runtime_shell, Some(music_id));
    record_visible_runtime_action(runtime_shell, action)?;
    runtime_shell
        .last_audio_events
        .push(format!("queued silent music stop {music_id}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn stop_visible_silent_music_with_checksum(
    runtime_shell: &mut BevyRuntimeShell,
    music_id: &str,
    action: impl Into<String>,
    state_checksum: &StateChecksum,
) -> Result<()> {
    set_visible_stopped_music_state(runtime_shell, Some(music_id));
    set_visible_runtime_action_from_checksum(runtime_shell, action, state_checksum);
    runtime_shell
        .last_audio_events
        .push(format!("queued silent music stop {music_id}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn set_visible_stopped_music_state(runtime_shell: &mut BevyRuntimeShell, marker: Option<&str>) {
    runtime_shell.music_fade = None;
    runtime_shell.music_volume = 7;
    runtime_shell.pending_music_stop = true;
    runtime_shell.pending_full_audio_reset = true;
    runtime_shell.active_music = marker.map(str::to_string);
    runtime_shell.faded_music = None;
    clear_pending_music_commands(&mut runtime_shell.pending_audio);
}

fn queue_runtime_title_music(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let title_music = runtime_shell.shell.runtime().title_music_id()?.to_string();
    if runtime_shell.active_music.as_deref() == Some(title_music.as_str())
        || pending_music_command_is(&runtime_shell.pending_audio, &title_music)
    {
        return Ok(());
    }
    if is_silent_music_id(&title_music) {
        stop_visible_silent_music(runtime_shell, &title_music, "audio:music:title:stop")?;
        return Ok(());
    }
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, &title_music)?;
    let mode = playback.mode;
    let looped = matches!(
        playback.loop_policy,
        crate::assets::ModpackAudioLoopPolicy::Loop
    );
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: title_music.clone(),
            kind: ModpackAudioKind::Music,
            mode,
            looped,
        },
    );
    runtime_shell.pending_music_stop = true;
    runtime_shell.active_music = Some(title_music.clone());
    runtime_shell.faded_music = None;
    record_visible_runtime_action(runtime_shell, format!("audio:music:title:{title_music}"))?;
    runtime_shell
        .last_audio_events
        .push(format!("queued title music {title_music}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn queue_visible_current_music(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if visible_non_overworld_screen_active(runtime_shell)
        || runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.visible_unown_words.is_some()
        || runtime_shell.visible_diploma.is_some()
        || runtime_shell.visible_egg_hatch.is_some()
        || runtime_shell.pending_egg_hatch_nickname.is_some()
        || runtime_shell.heal_music_active
    {
        return Ok(());
    }
    if runtime_shell.shell.has_active_battle() || runtime_shell.shell.has_pending_music_fade() {
        return Ok(());
    }
    if let Some((origin_map, radio_music)) = runtime_shell.active_pokegear_radio.as_ref() {
        let snapshot = runtime_shell.shell.snapshot()?;
        if snapshot.overworld.map_name == *origin_map
            && runtime_shell.active_music.as_deref() == Some(radio_music.as_str())
        {
            return Ok(());
        }
        runtime_shell.active_pokegear_radio = None;
    }
    if runtime_shell.active_music.as_deref() == runtime_shell.shell.current_music_id() {
        return Ok(());
    }

    let state_checksum = runtime_shell.shell.state_checksum()?;
    let Some(music_id) = runtime_shell.shell.current_music_id().map(str::to_owned) else {
        stop_visible_music_with_checksum(runtime_shell, "audio:music:stop", &state_checksum)?;
        return Ok(());
    };
    if is_silent_music_id(&music_id) {
        stop_visible_silent_music_with_checksum(
            runtime_shell,
            &music_id,
            format!("audio:music:current:{music_id}:stop"),
            &state_checksum,
        )?;
        return Ok(());
    };
    if runtime_shell.faded_music.as_deref() == Some(music_id.as_str()) {
        return Ok(());
    }

    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, &music_id)?;
    let mode = playback.mode;
    let looped = matches!(
        playback.loop_policy,
        crate::assets::ModpackAudioLoopPolicy::Loop
    );
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: music_id.clone(),
            kind: ModpackAudioKind::Music,
            mode,
            looped,
        },
    );
    runtime_shell.pending_music_stop = true;
    runtime_shell.active_music = Some(music_id.clone());
    runtime_shell.faded_music = None;
    set_visible_runtime_action_from_checksum(
        runtime_shell,
        format!("audio:music:current:{music_id}"),
        &state_checksum,
    );
    runtime_shell
        .last_audio_events
        .push(format!("queued current music {music_id}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn pending_music_command_is(pending: &[BevyAudioCommand], music_id: &str) -> bool {
    pending.iter().any(|command| {
        matches!(command.kind, ModpackAudioKind::Music) && command.audio_id == music_id
    })
}

fn sync_runtime_current_music(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    if let Err(error) = queue_visible_current_music(&mut runtime_shell) {
        record_visible_runtime_system_error(
            &mut runtime_shell,
            anyhow::anyhow!("failed to queue current music: {error:#}"),
        );
    }
}

/// Intro and setup screens own their music.  The map audio state must not
/// start playback until the player has reached the actual overworld.
fn visible_non_overworld_screen_active(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell.title_menu.is_some()
        || runtime_shell.credits_screen.is_some()
        || runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.pending_name_input.is_some()
        || runtime_shell.pending_mail_input.is_some()
        || runtime_shell.pending_mail_read.is_some()
}

fn sync_runtime_battle_music(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    if visible_non_overworld_screen_active(&runtime_shell)
        || runtime_shell.visible_fishing_animation.is_some()
        || matches!(
            runtime_shell.pending_overworld_step_boundary,
            Some(PendingOverworldStepBoundary::WildBattle)
        )
    {
        return;
    }
    // Battle music has no work to do outside a battle. Avoid taking the core
    // snapshot on every ordinary overworld frame just to discover that fact.
    if !runtime_shell.shell.has_active_battle() {
        return;
    }
    let snapshot = match cached_runtime_snapshot(&mut runtime_shell) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!("battle music snapshot failed: {error:#}"),
            );
            return;
        }
    };
    if snapshot.script_events.pending_music_fade.is_some() {
        return;
    }
    let Some(music_id) = battle_music_id(&snapshot) else {
        return;
    };
    if runtime_shell.active_music.as_deref() == Some(music_id.as_str()) {
        return;
    }
    if is_silent_music_id(&music_id) {
        if let Err(error) = stop_visible_silent_music_with_checksum(
            &mut runtime_shell,
            &music_id,
            format!("audio:music:battle:{music_id}:stop"),
            &snapshot.state_checksum,
        ) {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!("failed to stop silent battle music: {error:#}"),
            );
        }
        return;
    }
    if runtime_shell.faded_music.as_deref() == Some(music_id.as_str()) {
        return;
    }

    let playback = match runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, &music_id)
    {
        Ok(playback) => playback,
        Err(error) => {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!("battle music {music_id} failed pack playback lookup: {error:#}"),
            );
            return;
        }
    };

    let mode = playback.mode;
    let looped = matches!(
        playback.loop_policy,
        crate::assets::ModpackAudioLoopPolicy::Loop
    );
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: music_id.clone(),
            kind: ModpackAudioKind::Music,
            mode,
            looped,
        },
    );
    runtime_shell.pending_music_stop = true;
    runtime_shell.active_music = Some(music_id.clone());
    runtime_shell.faded_music = None;
    set_visible_runtime_action_from_checksum(
        &mut runtime_shell,
        format!("audio:music:battle:{music_id}"),
        &snapshot.state_checksum,
    );
    runtime_shell
        .last_audio_events
        .push(format!("queued battle music {music_id}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn battle_music_id(snapshot: &RuntimeShellSnapshot) -> Option<String> {
    Some(snapshot.battle.as_ref()?.battle_music.clone())
}

fn queue_battle_intro_cry(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    if runtime_shell.visible_battle_transition.is_some()
        || runtime_shell.visible_send_out_animation.is_some()
        || runtime_shell.visible_trainer_exit_animation.is_some()
    {
        return;
    }
    if !runtime_shell.shell.has_active_battle() {
        runtime_shell.last_battle_cry_key = None;
        return;
    }
    let snapshot = match cached_runtime_snapshot(&mut runtime_shell) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!("battle intro cry snapshot failed: {error:#}"),
            );
            return;
        }
    };
    let Some(battle) = snapshot.battle.as_ref() else {
        runtime_shell.last_battle_cry_key = None;
        return;
    };
    let tutorial = battle.battle_type == "BATTLETYPE_TUTORIAL";
    let enemy_cry_boundary = match battle.kind {
        RuntimeBattleKind::Trainer { .. } => 1,
        RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => {
            if tutorial {
                1
            } else {
                2
            }
        }
    };
    let stage = runtime_shell.last_battle_cry_key.as_deref();
    if stage.is_none() && runtime_shell.battle_entry_messages_remaining != enemy_cry_boundary {
        return;
    }
    if stage == Some("entry:enemy") && runtime_shell.battle_entry_messages_remaining != 0 {
        return;
    }
    if stage.is_none() {
        let enemy_species = battle.enemy_pokemon.species.id.clone();
        if let Err(error) =
            queue_visible_pokemon_cry(&mut runtime_shell, &enemy_species, "battle_enemy_intro")
        {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!("battle enemy species {enemy_species} failed intro cry: {error:#}"),
            );
            return;
        }
        runtime_shell.last_battle_cry_key = Some(if tutorial {
            "entry:complete".to_string()
        } else {
            "entry:enemy".to_string()
        });
        return;
    }
    if stage == Some("entry:enemy") {
        let Some(player_party_index) = battle.active_player_party_index else {
            runtime_shell.last_battle_cry_key = Some("entry:complete".to_string());
            return;
        };
        let Some(player_species) = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == player_party_index)
            .map(|slot| slot.pokemon.species.id.clone())
        else {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!(
                    "active battle party index {player_party_index} has no party Pokemon"
                ),
            );
            return;
        };
        if let Err(error) =
            queue_visible_pokemon_cry(&mut runtime_shell, &player_species, "battle_player_intro")
        {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                anyhow::anyhow!(
                    "active battle party index {player_party_index} species {player_species} failed cry queue: {error:#}"
                ),
            );
            return;
        }
        runtime_shell.last_battle_cry_key = Some("entry:complete".to_string());
    }
}

fn defer_visible_battle_cry_after_message(
    runtime_shell: &mut BevyRuntimeShell,
    species_id: impl Into<String>,
    reason: impl Into<String>,
    trigger_message: impl Into<String>,
) {
    runtime_shell
        .pending_battle_cries_after_messages
        .push_back((species_id.into(), reason.into(), trigger_message.into()));
}

fn defer_visible_party_index_cry_after_send_out(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
    reason: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("party index {party_index} has no Pokemon for deferred cry"))?;
    defer_visible_battle_cry_after_message(
        runtime_shell,
        slot.pokemon.species.id.clone(),
        reason,
        visible_player_send_out_message(&snapshot, party_index, false)?,
    );
    runtime_shell.battle_enemy_hp_at_player_send_out = snapshot
        .battle
        .as_ref()
        .map(|battle| battle.enemy_pokemon.hp);
    Ok(())
}

fn visible_player_send_out_message(
    snapshot: &RuntimeShellSnapshot,
    party_index: usize,
    battle_has_just_started: bool,
) -> Result<String> {
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("party index {party_index} has no Pokemon for send-out text"))?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("player send-out text requires an active battle")?;
    let nickname = &slot.pokemon.nickname;
    if snapshot.link_session.link_mode != 0 && !battle_has_just_started {
        return Ok(format!("Go! {nickname}!"));
    }
    let enemy = &battle.enemy_pokemon;
    let percent = if enemy.hp == 0 {
        100
    } else {
        // SendOutMonText avoids a two-byte divisor by multiplying current HP
        // by 25 and shifting max HP right twice before the four-byte Divide.
        // The truncation happens before division, so HP * 100 / max HP is not
        // equivalent at the dialogue thresholds.
        let divisor = enemy.max_hp >> 2;
        anyhow::ensure!(
            divisor != 0,
            "SendOutMonText would not terminate with enemy max HP {}",
            enemy.max_hp
        );
        // The routine branches on hQuotient + 3, not the wider quotient.
        ((u32::from(enemy.hp) * 25) / u32::from(divisor)) as u8
    };
    Ok(match percent {
        70.. => format!("Go! {nickname}!"),
        40..=69 => format!("Do it! {nickname}!"),
        10..=39 => format!("Go for it, {nickname}!"),
        _ => format!("Your foe's weak! Get'm, {nickname}!"),
    })
}

fn visible_message_is_player_send_out(message: &str) -> bool {
    message.starts_with("Go! ")
        || message.starts_with("Do it! ")
        || message.starts_with("Go for it, ")
        || message.starts_with("Your foe's weak! Get'm, ")
}

fn visible_message_is_enemy_send_out(message: &str) -> bool {
    message.contains("\nsent out\n")
}

fn queue_visible_pokemon_cry(
    runtime_shell: &mut BevyRuntimeShell,
    species_id: &str,
    reason: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let cry = snapshot
        .presentation
        .pokemon_cries
        .get(species_id)
        .with_context(|| format!("Pokemon species {species_id} has no pack cry metadata"))?;
    // PlayMonCry uses the species metadata's exported CRY_* program. Selector-
    // based CRY_MON_* variants belong to battle animation `cry` commands, not
    // ordinary species cries such as Oak's Wooper showcase.
    let cry_id = cry.cry.clone();
    let state_checksum = snapshot.state_checksum.clone();
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Cry, &cry_id)
        .with_context(|| {
            format!(
                "Pokemon species {species_id} source cry {} failed exact species variant lookup {cry_id}",
                cry.cry
            )
        })?;
    let playback_mode = playback.mode;
    let looped = matches!(
        playback.loop_policy,
        crate::assets::ModpackAudioLoopPolicy::Loop
    );
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: cry_id.clone(),
            kind: ModpackAudioKind::Cry,
            mode: playback_mode,
            looped,
        },
    );
    set_visible_runtime_action_from_checksum(
        runtime_shell,
        format!("audio:cry:{reason}:{species_id}:{cry_id}"),
        &state_checksum,
    );
    runtime_shell
        .last_audio_events
        .push(format!("queued {reason} cry {cry_id} species={species_id}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_pokemon_animation_cry_id(species_id: &str, selector: u8) -> String {
    let suffix = match selector & 0x03 {
        0 => "_GROWL",
        1 => "_ROAR",
        2 | 3 => "",
        _ => unreachable!(),
    };
    format!("CRY_MON_{species_id}{suffix}")
}

fn queue_visible_pokemon_animation_cry(
    runtime_shell: &mut BevyRuntimeShell,
    species_id: &str,
    selector: u8,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    snapshot
        .presentation
        .pokemon_cries
        .get(species_id)
        .with_context(|| format!("Pokemon species {species_id} has no pack cry metadata"))?;
    let cry_id = visible_pokemon_animation_cry_id(species_id, selector);
    let state_checksum = snapshot.state_checksum.clone();
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Cry, &cry_id)
        .with_context(|| {
            format!(
                "Pokemon species {species_id} animation cry selector {selector} failed exact variant lookup {cry_id}"
            )
        })?;
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: cry_id.clone(),
            kind: ModpackAudioKind::Cry,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    set_visible_runtime_action_from_checksum(
        runtime_shell,
        format!("audio:cry:battle_move_animation_{selector}:{species_id}:{cry_id}"),
        &state_checksum,
    );
    runtime_shell.last_audio_events.push(format!(
        "queued battle_move_animation_{selector} cry {cry_id} species={species_id}"
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn queue_selected_music_preview(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_selected_audio_preview(runtime_shell, ModpackAudioKind::Music)
}

fn queue_selected_sound_effect_preview(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_selected_audio_preview(runtime_shell, ModpackAudioKind::SoundEffect)
}

fn queue_selected_cry_preview(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_selected_audio_preview(runtime_shell, ModpackAudioKind::Cry)
}

fn queue_selected_audio_preview(
    runtime_shell: &mut BevyRuntimeShell,
    kind: ModpackAudioKind,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, audio_id, playback) = match kind {
        ModpackAudioKind::Music => {
            let (selected_index, selected_len, audio_id) = selected_btree_key(
                runtime_shell,
                "music playback entries",
                &snapshot.audio_catalog.playback.music,
            )?;
            let playback = snapshot
                .audio_catalog
                .playback
                .music
                .get(&audio_id)
                .with_context(|| format!("selected music playback entry {audio_id} is missing"))?
                .clone();
            (selected_index, selected_len, audio_id, playback)
        }
        ModpackAudioKind::SoundEffect => {
            let (selected_index, selected_len, audio_id) = selected_btree_key(
                runtime_shell,
                "sound-effect playback entries",
                &snapshot.audio_catalog.playback.sound_effects,
            )?;
            let playback = snapshot
                .audio_catalog
                .playback
                .sound_effects
                .get(&audio_id)
                .with_context(|| {
                    format!("selected sound-effect playback entry {audio_id} is missing")
                })?
                .clone();
            (selected_index, selected_len, audio_id, playback)
        }
        ModpackAudioKind::Cry => {
            let (selected_index, selected_len, audio_id) = selected_btree_key(
                runtime_shell,
                "cry playback entries",
                &snapshot.audio_catalog.playback.cries,
            )?;
            let playback = snapshot
                .audio_catalog
                .playback
                .cries
                .get(&audio_id)
                .with_context(|| format!("selected cry playback entry {audio_id} is missing"))?
                .clone();
            (selected_index, selected_len, audio_id, playback)
        }
    };
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: audio_id.clone(),
            kind,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    set_visible_runtime_action_from_checksum(
        runtime_shell,
        format!("audio:preview:{kind:?}:{audio_id}"),
        &snapshot.state_checksum,
    );
    runtime_shell.last_audio_events.push(format!(
        "queued {:?} preview {}/{} {} mode={:?}",
        kind,
        selected_index + 1,
        selected_len,
        audio_id,
        playback.mode
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn trim_event_log(events: &mut Vec<String>) {
    let remove_count = events.len().saturating_sub(EVENT_LOG_LIMIT);
    if remove_count > 0 {
        events.drain(0..remove_count);
    }
}

fn summarize_frame_activity(frame: &crate::RuntimeOverworldFrame) -> Option<String> {
    if let Some(wild_battle) = &frame.wild_battle {
        return Some(format!(
            "wild battle {:?} checksum={:?}",
            wild_battle, frame.state_checksum
        ));
    }
    if let Some(wild_encounter) = &frame.wild_encounter {
        return Some(format!(
            "wild encounter {:?} checksum={:?}",
            wild_encounter, frame.state_checksum
        ));
    }
    if let Some(interaction) = &frame.interaction {
        return Some(format!(
            "interaction {:?} checksum={:?}",
            interaction, frame.state_checksum
        ));
    }
    if let Some(trainer_sight) = &frame.trainer_sight {
        return Some(format!(
            "trainer sight {:?} checksum={:?}",
            trainer_sight, frame.state_checksum
        ));
    }
    if let Some(warp) = &frame.warp {
        return Some(format!(
            "warp {:?} checksum={:?}",
            warp, frame.state_checksum
        ));
    }
    if let Some(connection) = &frame.connection {
        return Some(format!(
            "connection {:?} checksum={:?}",
            connection, frame.state_checksum
        ));
    }
    if let Some(coord_event) = &frame.coord_event {
        return Some(format!(
            "coord event {:?} checksum={:?}",
            coord_event, frame.state_checksum
        ));
    }
    frame.movement.as_ref().map(|movement| {
        format!(
            "movement {:?} tile=({}, {}) checksum={:?}",
            movement, frame.snapshot.tile.x, frame.snapshot.tile.y, frame.state_checksum
        )
    })
}

fn shell_render_key(runtime_shell: &BevyRuntimeShell) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_menu_cursor(&mut hasher, &runtime_shell.start_menu_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.menu_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.shop_top_cursor);
    runtime_shell.shop_quantity.hash(&mut hasher);
    runtime_shell.shop_notice.hash(&mut hasher);
    runtime_shell.shop_welcome_seen.hash(&mut hasher);
    runtime_shell
        .shop_return_to_top_after_notice
        .hash(&mut hasher);
    runtime_shell.shop_close_after_notice.hash(&mut hasher);
    runtime_shell.pending_pc_release.hash(&mut hasher);
    runtime_shell.pc_release_sequence.hash(&mut hasher);
    runtime_shell.pc_transfer_sequence.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.bill_pc_pokemon_action_cursor);
    runtime_shell.bill_pc_box_summary.hash(&mut hasher);
    runtime_shell.pc_notice.hash(&mut hasher);
    runtime_shell.field_notice.hash(&mut hasher);
    runtime_shell.field_notice_queue.hash(&mut hasher);
    runtime_shell.pending_item_notification.hash(&mut hasher);
    runtime_shell
        .field_notice_scene
        .as_ref()
        .map(|scene| scene.visual_state_hash)
        .hash(&mut hasher);
    runtime_shell.pending_field_travel_arrival.hash(&mut hasher);
    runtime_shell
        .pending_field_travel_delay_frames
        .hash(&mut hasher);
    runtime_shell
        .visible_field_travel_animation
        .hash(&mut hasher);
    runtime_shell.pending_field_notice_sound.hash(&mut hasher);
    runtime_shell.pending_field_notice_cry.hash(&mut hasher);
    runtime_shell.pending_field_battle_entry.hash(&mut hasher);
    runtime_shell
        .pending_field_notice_effect_frames
        .hash(&mut hasher);
    runtime_shell.visible_cut_animation.hash(&mut hasher);
    runtime_shell.pending_whirlpool_sound_wait.hash(&mut hasher);
    runtime_shell.visible_headbutt_animation.hash(&mut hasher);
    runtime_shell.visible_flash_animation.hash(&mut hasher);
    runtime_shell.visible_fly_animation.hash(&mut hasher);
    runtime_shell.visible_waterfall_animation.hash(&mut hasher);
    runtime_shell.pending_surf_start_from.hash(&mut hasher);
    runtime_shell.visible_heal_machine.hash(&mut hasher);
    runtime_shell.visible_magnet_train.hash(&mut hasher);
    runtime_shell.visible_unown_words.hash(&mut hasher);
    runtime_shell.visible_diploma.hash(&mut hasher);
    runtime_shell.visible_battle_transition.hash(&mut hasher);
    runtime_shell.visible_capture_animation.hash(&mut hasher);
    if let Some(replacement) = &runtime_shell.visible_bug_contest_replacement {
        true.hash(&mut hasher);
        replacement.phase.hash(&mut hasher);
        replacement.previous.species.id.hash(&mut hasher);
        replacement.previous.level.hash(&mut hasher);
        replacement.previous.max_hp.hash(&mut hasher);
        replacement.candidate.species.id.hash(&mut hasher);
        replacement.candidate.nickname.hash(&mut hasher);
        replacement.candidate.level.hash(&mut hasher);
        replacement.candidate.max_hp.hash(&mut hasher);
        if let Some(origin) = &replacement.scripted_static_wild {
            true.hash(&mut hasher);
            origin.map_name.hash(&mut hasher);
            origin.source_script.hash(&mut hasher);
            origin.startbattle_command_index.hash(&mut hasher);
            origin.resume_command_index.hash(&mut hasher);
            origin.battle_type.hash(&mut hasher);
            origin.species.hash(&mut hasher);
            origin.level.hash(&mut hasher);
        } else {
            false.hash(&mut hasher);
        }
    } else {
        false.hash(&mut hasher);
    }
    runtime_shell.visible_move_animations.hash(&mut hasher);
    runtime_shell.visible_send_out_animation.hash(&mut hasher);
    runtime_shell
        .visible_trainer_exit_animation
        .hash(&mut hasher);
    runtime_shell.visible_frontpic_animation.hash(&mut hasher);
    runtime_shell.visible_fishing_animation.hash(&mut hasher);
    runtime_shell.visible_egg_hatch.hash(&mut hasher);
    runtime_shell.visible_blackout_phase.hash(&mut hasher);
    runtime_shell.pending_poison_blackout.hash(&mut hasher);
    runtime_shell.visible_walk_warp_phase.hash(&mut hasher);
    runtime_shell.battle_text_reveal.hash(&mut hasher);
    runtime_shell
        .pending_battle_cries_after_messages
        .hash(&mut hasher);
    runtime_shell
        .battle_enemy_send_out_pending
        .hash(&mut hasher);
    runtime_shell
        .battle_player_send_out_pending
        .hash(&mut hasher);
    runtime_shell
        .pending_battle_scenes_after_message
        .iter()
        .map(|(message, scene)| (message, scene.visual_state_hash))
        .collect::<Vec<_>>()
        .hash(&mut hasher);
    runtime_shell.heal_music_active.hash(&mut hasher);
    runtime_shell.visible_wait_sfx_boundary.hash(&mut hasher);
    runtime_shell.pending_wait_play_sfx.hash(&mut hasher);
    runtime_shell.wait_play_sfx_completion.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.bag_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.key_item_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.ball_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.tmhm_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.custom_item_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.party_action_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.bill_pc_box_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.bill_pc_box_action_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.party_give_take_cursor);
    runtime_shell.party_mail_take_stage.hash(&mut hasher);
    runtime_shell.party_held_item_give_target.hash(&mut hasher);
    runtime_shell.held_item_swap_prompt.hash(&mut hasher);
    runtime_shell
        .pending_contextual_field_move
        .hash(&mut hasher);
    runtime_shell
        .pending_script_party_selection
        .hash(&mut hasher);
    runtime_shell
        .pending_link_trade_party_slot
        .hash(&mut hasher);
    runtime_shell
        .pending_link_trade_confirmation
        .hash(&mut hasher);
    runtime_shell.pending_link_trade_save.hash(&mut hasher);
    runtime_shell.pending_link_room_selection.hash(&mut hasher);
    runtime_shell.pending_linked_friend_wait.hash(&mut hasher);
    runtime_shell.pending_link_room_session.hash(&mut hasher);
    runtime_shell.pending_npc_trade_commit.hash(&mut hasher);
    runtime_shell.pending_photo_studio_commit.hash(&mut hasher);
    runtime_shell.kurt_apricorn_cursor.hash(&mut hasher);
    runtime_shell.kurt_apricorn_quantity.hash(&mut hasher);
    runtime_shell.buena_prize_cursor.hash(&mut hasher);
    runtime_shell.visible_buena_password.hash(&mut hasher);
    runtime_shell
        .visible_battle_tower_challenge_menu
        .hash(&mut hasher);
    runtime_shell
        .visible_battle_tower_room_menu
        .hash(&mut hasher);
    runtime_shell.visible_unown_puzzle.hash(&mut hasher);
    runtime_shell.visible_unown_printer.hash(&mut hasher);
    runtime_shell.visible_slot_machine.hash(&mut hasher);
    runtime_shell.visible_card_flip.hash(&mut hasher);
    runtime_shell.party_move_reorder_open.hash(&mut hasher);
    runtime_shell.party_move_reorder_origin.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.party_switch_cursor);
    runtime_shell.party_hp_transfer_source.hash(&mut hasher);
    runtime_shell.party_hp_transfer_move.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.tmhm_teach_prompt_cursor);
    runtime_shell.pending_tmhm_text_stage.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.tmhm_decision_prompt_cursor);
    runtime_shell.tmhm_decision.hash(&mut hasher);
    runtime_shell.tmhm_forget_menu_open.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.move_learn_decision_cursor);
    runtime_shell.move_learn_decision.hash(&mut hasher);
    runtime_shell.move_learn_forget_menu_open.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.battle_action_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.battle_move_cursor);
    runtime_shell.battle_move_swap_origin.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.battle_shift_prompt_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.battle_faint_prompt_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.battle_switch_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.battle_party_action_cursor);
    runtime_shell.battle_party_summary_open.hash(&mut hasher);
    runtime_shell
        .pending_battle_move_switch_slot
        .hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.party_move_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.fly_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.yes_no_cursor);
    runtime_shell.pending_phone_prompt.hash(&mut hasher);
    runtime_shell.pending_remember_password.hash(&mut hasher);
    runtime_shell.pending_day_of_week.hash(&mut hasher);
    runtime_shell.pending_trainer_sight.hash(&mut hasher);
    runtime_shell.pending_trainer_intro.hash(&mut hasher);
    runtime_shell
        .visible_map_name_sign
        .as_ref()
        .map(|sign| (&sign.landmark, &sign.label))
        .hash(&mut hasher);
    runtime_shell.pending_delete_save.hash(&mut hasher);
    runtime_shell.pending_clock_reset.hash(&mut hasher);
    runtime_shell.pending_mystery_gift.hash(&mut hasher);
    runtime_shell.pending_time_set.hash(&mut hasher);
    runtime_shell.pending_oak_intro.hash(&mut hasher);
    runtime_shell.pending_gender_selection.hash(&mut hasher);
    runtime_shell.selected_player_gender.hash(&mut hasher);
    runtime_shell.pending_name_input.hash(&mut hasher);
    runtime_shell.pending_mail_input.hash(&mut hasher);
    runtime_shell.pending_mail_read.hash(&mut hasher);
    runtime_shell.pending_name_choice.hash(&mut hasher);
    runtime_shell
        .pending_gift_pokemon_nickname
        .hash(&mut hasher);
    runtime_shell
        .pending_gift_pokemon_pc_notice
        .hash(&mut hasher);
    runtime_shell.pending_egg_hatch_nickname.hash(&mut hasher);
    runtime_shell.visible_field_item_notice.hash(&mut hasher);
    runtime_shell.party_menu_open.hash(&mut hasher);
    runtime_shell.visible_overworld_emote.hash(&mut hasher);
    runtime_shell.visible_earthquake.hash(&mut hasher);
    runtime_shell.visible_ledge_jump.hash(&mut hasher);
    runtime_shell.visible_script_movement.hash(&mut hasher);
    runtime_shell
        .visible_player_sprite_y_offset
        .hash(&mut hasher);
    runtime_shell.visible_grass_rustle.hash(&mut hasher);
    runtime_shell
        .visible_strength_boulder_dust
        .hash(&mut hasher);
    runtime_shell.battle_messages.hash(&mut hasher);
    runtime_shell.battle_fanfare_messages.hash(&mut hasher);
    runtime_shell.battle_evolution_cries.hash(&mut hasher);
    for cancellation in &runtime_shell.battle_evolution_cancellations {
        cancellation.party_index.hash(&mut hasher);
        cancellation.trigger_message.hash(&mut hasher);
        cancellation.evolved_message.hash(&mut hasher);
        cancellation.pending_move_messages.hash(&mut hasher);
        cancellation.report.target_species.hash(&mut hasher);
        cancellation
            .report
            .cancel_snapshot
            .as_ref()
            .map(|pokemon| (&pokemon.species.id, pokemon.level, pokemon.item.as_deref()))
            .hash(&mut hasher);
    }
    runtime_shell
        .field_evolution_cancellation
        .as_ref()
        .map(|cancellation| {
            (
                cancellation.party_index,
                cancellation.trigger_message.as_str(),
                cancellation.evolved_message.as_str(),
                cancellation.report.target_species.as_deref(),
            )
        })
        .hash(&mut hasher);
    runtime_shell.battle_sounds_after_messages.hash(&mut hasher);
    runtime_shell.battle_hp_tween.hash(&mut hasher);
    runtime_shell.battle_exp_tween.hash(&mut hasher);
    runtime_shell.pending_battle_exp_tweens.hash(&mut hasher);
    runtime_shell.battle_level_stats.hash(&mut hasher);
    runtime_shell.bill_pc_move_open.hash(&mut hasher);
    runtime_shell.bill_pc_move_party_open.hash(&mut hasher);
    runtime_shell.bill_pc_move_source.hash(&mut hasher);
    runtime_shell.bill_pc_move_save.hash(&mut hasher);
    runtime_shell.pc_transfer_sequence.hash(&mut hasher);
    runtime_shell.party_summary_open.hash(&mut hasher);
    runtime_shell.party_summary_page.hash(&mut hasher);
    runtime_shell.party_cursor.hash(&mut hasher);
    runtime_shell.pokedex_menu_open.hash(&mut hasher);
    runtime_shell.pokedex_detail_open.hash(&mut hasher);
    runtime_shell.pokedex_detail_page.hash(&mut hasher);
    runtime_shell.pokedex_scripted_entry.hash(&mut hasher);
    runtime_shell.visible_balance_overlay.hash(&mut hasher);
    runtime_shell.visible_mom_bank.hash(&mut hasher);
    runtime_shell.pokedex_cursor.hash(&mut hasher);
    runtime_shell.pokegear_menu_open.hash(&mut hasher);
    runtime_shell.pokegear_standalone_map.hash(&mut hasher);
    runtime_shell.pokegear_cursor.hash(&mut hasher);
    runtime_shell.pokegear_phone_cursor.hash(&mut hasher);
    runtime_shell.pokegear_phone_status.hash(&mut hasher);
    runtime_shell.pokegear_phone_call.hash(&mut hasher);
    runtime_shell.incoming_phone_sequence.hash(&mut hasher);
    runtime_shell.pokegear_page.hash(&mut hasher);
    runtime_shell.pokegear_radio_tuning_knob.hash(&mut hasher);
    runtime_shell.pokegear_radio_station.hash(&mut hasher);
    runtime_shell.pokegear_radio_segment.hash(&mut hasher);
    runtime_shell.active_pokegear_radio.hash(&mut hasher);
    runtime_shell.pc_hub_session_open.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.pc_hub_cursor);
    runtime_shell.hall_of_fame_pc_index.hash(&mut hasher);
    runtime_shell.pc_item_action.hash(&mut hasher);
    runtime_shell.pc_item_quantity.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.player_pc_action_cursor);
    runtime_shell.decoration_menu.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.mailbox_cursor);
    hash_menu_cursor(&mut hasher, &runtime_shell.mailbox_action_cursor);
    runtime_shell.mailbox_attach_index.hash(&mut hasher);
    runtime_shell.pc_confirmation.hash(&mut hasher);
    runtime_shell.bill_pc_session_open.hash(&mut hasher);
    hash_menu_cursor(&mut hasher, &runtime_shell.bill_pc_action_cursor);
    runtime_shell.trainer_card_open.hash(&mut hasher);
    runtime_shell.trainer_card_page.hash(&mut hasher);
    runtime_shell.trainer_card_colon_visible.hash(&mut hasher);
    runtime_shell.trainer_card_badge_frame.hash(&mut hasher);
    runtime_shell.options_menu_open.hash(&mut hasher);
    runtime_shell.options_cursor.hash(&mut hasher);
    runtime_shell.save_menu_open.hash(&mut hasher);
    runtime_shell.pending_special_cry.hash(&mut hasher);
    runtime_shell.pending_special_sound.hash(&mut hasher);
    runtime_shell.field_pack_pocket.hash(&mut hasher);
    runtime_shell.pack_item_switch_origin.hash(&mut hasher);
    runtime_shell.last_field_pack_pocket.hash(&mut hasher);
    runtime_shell.field_pack_cursor_positions.hash(&mut hasher);
    runtime_shell.field_pack_target_mode.hash(&mut hasher);
    runtime_shell.battle_pack_target_mode.hash(&mut hasher);
    runtime_shell.pack_toss.hash(&mut hasher);
    if let Some(title) = &runtime_shell.title_menu {
        true.hash(&mut hasher);
        title.spawn_identifier.hash(&mut hasher);
        title.save_path.hash(&mut hasher);
        title.cursor.surface_id.hash(&mut hasher);
        title.cursor.option_index.hash(&mut hasher);
        title.source_phase().hash(&mut hasher);
        title.source_suicune_frame().hash(&mut hasher);
        title.source_scx().hash(&mut hasher);
        title.source_title_timer().hash(&mut hasher);
        title.crystal_oam_target.hash(&mut hasher);
        title.crystal_initial_y.hash(&mut hasher);
        title.suicune_frames.hash(&mut hasher);
        title.suicune_selector_mask.hash(&mut hasher);
        title.suicune_selector_shift_left.hash(&mut hasher);
        title.suicune_selector_swap_nibbles.hash(&mut hasher);
        title
            .presentation_machine
            .interpreter
            .subprogram
            .hash(&mut hasher);
        title
            .presentation_machine
            .interpreter
            .phase
            .hash(&mut hasher);
        title
            .presentation_machine
            .interpreter
            .operation_index
            .hash(&mut hasher);
        title
            .presentation_machine
            .interpreter
            .current_label
            .hash(&mut hasher);
        title.presentation_machine.memory.hash(&mut hasher);
        title.presentation_machine.values.hash(&mut hasher);
        if let Some(cursor) = &title.title_teardown {
            true.hash(&mut hasher);
            cursor.subprogram.hash(&mut hasher);
            cursor.phase.hash(&mut hasher);
            cursor.operation_index.hash(&mut hasher);
            cursor.end_operation_index.hash(&mut hasher);
            cursor.wait_frames_remaining.hash(&mut hasher);
        } else {
            false.hash(&mut hasher);
        }
        if let Some(interpreter) = &title.main_menu_entry_interpreter {
            true.hash(&mut hasher);
            interpreter.entrypoint.hash(&mut hasher);
            interpreter.block.hash(&mut hasher);
            interpreter.operation_index.hash(&mut hasher);
        } else {
            false.hash(&mut hasher);
        }
        if let Some(interpreter) = &title.main_menu_phase_interpreter {
            true.hash(&mut hasher);
            interpreter.subprogram.hash(&mut hasher);
            interpreter.phase.hash(&mut hasher);
            interpreter.operation_index.hash(&mut hasher);
            interpreter.current_label.hash(&mut hasher);
        } else {
            false.hash(&mut hasher);
        }
        title.main_menu_waiting_for_input.hash(&mut hasher);
        title.joypad_mask.hash(&mut hasher);
    } else {
        false.hash(&mut hasher);
    }
    runtime_shell.visible_continue_screen.hash(&mut hasher);
    match &runtime_shell.intro_screen {
        Some(intro) => {
            true.hash(&mut hasher);
            intro_scene_art_key(intro).hash(&mut hasher);
        }
        None => false.hash(&mut hasher),
    }
    runtime_shell.credits_screen.hash(&mut hasher);
    if let Some(boundary) = &runtime_shell.special_boundary {
        true.hash(&mut hasher);
        boundary.label.hash(&mut hasher);
        boundary.details.hash(&mut hasher);
    } else {
        false.hash(&mut hasher);
    }
    for boundary in &runtime_shell.special_boundary_queue {
        boundary.label.hash(&mut hasher);
        boundary.details.hash(&mut hasher);
    }
    runtime_shell
        .visible_special_text_pause_frames
        .hash(&mut hasher);
    runtime_shell
        .visible_internal_special_delay_frames
        .hash(&mut hasher);
    runtime_shell.last_error.hash(&mut hasher);
    runtime_shell.last_action_status.hash(&mut hasher);
    runtime_shell.active_music.hash(&mut hasher);
    runtime_shell.faded_music.hash(&mut hasher);
    runtime_shell.music_volume.hash(&mut hasher);
    runtime_shell.music_fade.hash(&mut hasher);
    hasher.finish()
}

fn battle_animated_shell_render_key(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> u64 {
    let base = shell_render_key(runtime_shell);
    if snapshot.battle.is_none() && !runtime_shell.ambient_tileset_animation_active {
        return base;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    base.hash(&mut hasher);
    // `visual_state_hash` intentionally removes the semantic frame counter.
    // Battle UI still has LCD-time animation: healthy party icons change at
    // eight frames and menu cursors alternate at sixteen. Retain the cheapest
    // phase that can express both without rebuilding the overworld each tick.
    if snapshot.battle.is_some() {
        (runtime_shell.lcd_animation_frame / 8).hash(&mut hasher);
    }
    if runtime_shell.ambient_tileset_animation_active {
        for (period, offset) in &runtime_shell.ambient_tileset_animation_schedule {
            let phase = if runtime_shell.lcd_animation_frame < *offset {
                0
            } else {
                1 + (runtime_shell.lcd_animation_frame - *offset) / (*period).max(1)
            };
            phase.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn overworld_render_world_key(snapshot: &RuntimeShellSnapshot) -> u64 {
    let map_block_identity = visible_map_block_identity(snapshot);
    let position_key = overworld_render_position_key(snapshot);
    let appearance_key = overworld_render_appearance_key(snapshot);
    overworld_render_world_key_from_parts(
        snapshot.overworld.facing,
        map_block_identity,
        position_key,
        appearance_key,
    )
}

fn overworld_render_world_key_from_parts(
    facing: Direction,
    map_block_identity: u64,
    position_key: u64,
    appearance_key: u64,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    map_block_identity.hash(&mut hasher);
    position_key.hash(&mut hasher);
    appearance_key.hash(&mut hasher);
    facing.hash(&mut hasher);
    hasher.finish()
}

/// Hash the authoritative blocks that can contribute pixels to the current
/// viewport. Besides the active map, connection targets can be visible across
/// an outdoor seam. Script `changeblock` commands and CUT/WHIRLPOOL replace
/// these rows at runtime, so treating them as immutable leaves retained 2D
/// surfaces stale after the gameplay state has already committed the change.
fn visible_map_block_identity(snapshot: &RuntimeShellSnapshot) -> u64 {
    visible_map_block_identity_from_catalog(&snapshot.overworld.map_name, &snapshot.maps)
}

trait VisibleMapCatalogEntry {
    fn visible_map(&self) -> &crate::RuntimeMapCatalogSnapshot;
}

impl VisibleMapCatalogEntry for crate::RuntimeMapCatalogSnapshot {
    fn visible_map(&self) -> &crate::RuntimeMapCatalogSnapshot {
        self
    }
}

impl VisibleMapCatalogEntry for std::sync::Arc<crate::RuntimeMapCatalogSnapshot> {
    fn visible_map(&self) -> &crate::RuntimeMapCatalogSnapshot {
        self
    }
}

fn visible_map_block_identity_from_catalog<M: VisibleMapCatalogEntry>(
    active_map_name: &str,
    maps: &[M],
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    active_map_name.hash(&mut hasher);
    let active = maps
        .iter()
        .map(VisibleMapCatalogEntry::visible_map)
        .find(|map| map.map_name == active_map_name);
    active.map(|map| map.blocks.as_slice()).hash(&mut hasher);
    if let Some(active) = active {
        for connection in &active.attributes.connections {
            connection.target_map.hash(&mut hasher);
            maps.iter()
                .map(VisibleMapCatalogEntry::visible_map)
                .find(|map| map.map_name == connection.target_map)
                .map(|map| map.blocks.as_slice())
                .hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod visible_map_block_identity_tests {
    use super::*;

    fn map_snapshot(
        map_name: &str,
        blocks: Vec<u16>,
        connections: Vec<crystal_core::map::MapConnection>,
    ) -> crate::RuntimeMapCatalogSnapshot {
        crate::RuntimeMapCatalogSnapshot {
            map_name: map_name.to_string(),
            id: map_name.to_string(),
            attributes: crystal_core::map::MapAttributes {
                tileset_name: "test".to_string(),
                border_block: 0,
                width: u16::try_from(blocks.len()).expect("test map width"),
                height: 1,
                connections,
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: Some("route".to_string()),
                location: None,
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: None,
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
            metadata: None,
            scenes: crystal_core::map::MapSceneTable::default(),
            events: crystal_core::map::MapEvents::default(),
            objects: Vec::new(),
            blocks,
        }
    }

    #[test]
    fn visible_block_identity_tracks_active_and_connected_runtime_writes() {
        let connection = crystal_core::map::MapConnection {
            direction: "east".to_string(),
            target_map: "Neighbor".to_string(),
            offset: 0,
        };
        let baseline = vec![
            map_snapshot("Active", vec![1, 2], vec![connection.clone()]),
            map_snapshot("Neighbor", vec![3, 4], Vec::new()),
            map_snapshot("Hidden", vec![5, 6], Vec::new()),
        ];
        let baseline_key = visible_map_block_identity_from_catalog("Active", &baseline);

        let mut active_changed = baseline.clone();
        active_changed[0].blocks[1] = 7;
        assert_ne!(
            baseline_key,
            visible_map_block_identity_from_catalog("Active", &active_changed)
        );

        let mut neighbor_changed = baseline.clone();
        neighbor_changed[1].blocks[0] = 8;
        assert_ne!(
            baseline_key,
            visible_map_block_identity_from_catalog("Active", &neighbor_changed)
        );

        let mut hidden_changed = baseline;
        hidden_changed[2].blocks[0] = 9;
        assert_eq!(
            baseline_key,
            visible_map_block_identity_from_catalog("Active", &hidden_changed),
            "an unrelated offscreen map must not invalidate the retained viewport"
        );
    }
}

fn overworld_render_position_key(snapshot: &RuntimeShellSnapshot) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    snapshot.overworld.map_name.hash(&mut hasher);
    snapshot.overworld.tile.hash(&mut hasher);
    for object in &snapshot.visible_objects {
        object.object_identifier.hash(&mut hasher);
        object.x.hash(&mut hasher);
        object.y.hash(&mut hasher);
    }
    // Runtime movement state can temporarily outlive its catalog object.
    // Preserve the old world-key semantics by hashing every authoritative
    // runtime tile, including entries not present in `visible_objects`.
    for (object_id, tile) in &snapshot.visible_object_runtime_tiles {
        object_id.hash(&mut hasher);
        tile.hash(&mut hasher);
    }
    hasher.finish()
}

fn overworld_render_appearance_key(snapshot: &RuntimeShellSnapshot) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    snapshot.trainer.player_gender.hash(&mut hasher);
    snapshot.trainer.player_palette_id.hash(&mut hasher);
    for object in &snapshot.visible_objects {
        object.object_identifier.hash(&mut hasher);
        object.sprite.hash(&mut hasher);
        object.pal.hash(&mut hasher);
        object.spritemovedata.hash(&mut hasher);
    }
    // Facing selects a different source sprite frame, so it is appearance,
    // not merely position. A walking NPC may turn and step in one core tick.
    for (object_id, facing) in &snapshot.visible_object_facings {
        object_id.hash(&mut hasher);
        facing.hash(&mut hasher);
    }
    hasher.finish()
}

fn update_overworld_sprite_positions(
    snapshot: &RuntimeShellSnapshot,
    movement_subframe: f32,
    visible_ledge_jump: Option<VisibleLedgeJump>,
    player_walk_from: Option<TilePosition>,
    player_walk_frame_ticks: u8,
    player_walk_total_ticks: u8,
    object_walk_from: &BTreeMap<String, TilePosition>,
    object_walk_frame_ticks_by_id: &BTreeMap<String, u8>,
    object_walk_total_ticks_by_id: &BTreeMap<String, u8>,
    trainer_walk_from: Option<&(String, TilePosition)>,
    object_walk_frame_ticks: u8,
    object_walk_total_ticks: u8,
    camera_offset: Vec2,
    start_x: i16,
    start_y: i16,
    player_sprites: &mut Query<
        (
            &mut Handle<Image>,
            &mut Transform,
            &mut Sprite,
            &mut PlayerSpriteFrames,
        ),
        (
            With<PlayerMarker>,
            Without<DialogGlyphMarker>,
            Without<VisibleIntroSurface>,
        ),
    >,
    ledge_shadows: &mut Query<
        (&mut Transform, &Sprite),
        (
            With<JumpShadowMarker>,
            Without<PlayerMarker>,
            Without<VisibleObjectSprite>,
            Without<PlayfieldTile>,
            Without<DialogGlyphMarker>,
        ),
    >,
    object_sprites: &mut Query<
        (
            Entity,
            &mut VisibleObjectSprite,
            &mut Handle<Image>,
            &mut Transform,
            &mut Sprite,
        ),
        (Without<PlayerMarker>, Without<VisibleIntroSurface>),
    >,
) -> bool {
    let Ok((_, mut player_transform, player_sprite, _)) = player_sprites.get_single_mut() else {
        return false;
    };
    let (movement_from, movement_remaining, movement_total) = visible_ledge_jump
        .map(|jump| (Some(jump.from), 16_u8.saturating_sub(jump.frame), 16))
        .unwrap_or((
            player_walk_from,
            player_walk_frame_ticks,
            player_walk_total_ticks,
        ));
    let Some((player_base_x, player_base_y)) =
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
        return false;
    };
    let Some(player_size) = player_sprite.custom_size else {
        return false;
    };
    let (player_x, player_y) =
        overworld_sprite_position_from_base(player_base_x, player_base_y, player_size);
    player_transform.translation.x = player_x + camera_offset.x;
    player_transform.translation.y =
        player_y + camera_offset.y + visible_ledge_jump_y_offset(visible_ledge_jump);
    // Core commits a tile atomically, but TypeScript/LoadAndSortSprites keeps
    // the player's logical origin coordinates until the visible step lands.
    // Sorting at the committed destination here makes the player snap above
    // or below an NPC/priority edge before the sprite has actually crossed it.
    let player_depth_tile = visible_ledge_jump.map_or_else(
        || {
            player_walk_from
                .filter(|_| player_walk_frame_ticks > 0)
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
    player_transform.translation.z =
        overworld_entity_depth(player_depth_tile, None, (start_x, start_y));
    if ledge_jump_has_active_shadow(visible_ledge_jump)
        && let Ok((mut shadow_transform, shadow_sprite)) = ledge_shadows.get_single_mut()
    {
        let Some(shadow_size) = shadow_sprite.custom_size else {
            return false;
        };
        let direction = visible_ledge_jump
            .and_then(|jump| visible_jump_direction(jump.from, jump.to))
            .unwrap_or(snapshot.overworld.facing);
        let (shadow_x, shadow_y) = jump_shadow_position_from_actor_ground(
            player_x,
            player_y,
            player_size.y,
            shadow_size,
            direction,
        );
        shadow_transform.translation.x = shadow_x + camera_offset.x;
        shadow_transform.translation.y = shadow_y + camera_offset.y;
        shadow_transform.translation.z =
            overworld_entity_depth(player_depth_tile, None, (start_x, start_y)) - 0.000_001;
    }

    let expected_visible_object_count = snapshot
        .visible_objects
        .iter()
        .filter(|object| {
            let destination = object
                .object_identifier
                .as_ref()
                .and_then(|object_id| {
                    snapshot
                        .visible_object_runtime_tiles
                        .get(object_id)
                        .copied()
                })
                .or_else(|| object_tile_position_checked(object));
            let origin = object.object_identifier.as_ref().and_then(|object_id| {
                trainer_walk_from
                    .filter(|(walking_id, _)| walking_id == object_id)
                    .map(|(_, from)| *from)
                    .or_else(|| object_walk_from.get(object_id).copied())
            });
            [destination, origin].into_iter().flatten().any(|tile| {
                runtime_event_view_tile(tile, start_x, start_y)
                    .is_some_and(|(x, y)| overworld_object_in_scroll_region(x, y))
            })
        })
        .count();
    if object_sprites.iter().count() != expected_visible_object_count {
        return false;
    }
    for (object_index, object) in snapshot.visible_objects.iter().enumerate() {
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
            return false;
        };
        let Some((view_x, view_y)) = runtime_event_view_tile(object_tile, start_x, start_y) else {
            return false;
        };
        let walking_object_id = object.object_identifier.as_ref();
        let trainer_is_walking = walking_object_id.is_some_and(|object_id| {
            trainer_walk_from.is_some_and(|(walking_id, _)| walking_id == object_id)
        });
        let walking_from = walking_object_id.and_then(|object_id| {
            trainer_walk_from
                .filter(|(walking_id, _)| walking_id == object_id)
                .map(|(_, from)| *from)
                .or_else(|| object_walk_from.get(object_id).copied())
        });
        let destination_visible = overworld_object_in_scroll_region(view_x, view_y);
        let origin_visible = walking_from
            .and_then(|from| runtime_event_view_tile(from, start_x, start_y))
            .is_some_and(|(x, y)| overworld_object_in_scroll_region(x, y));
        if !destination_visible && !origin_visible {
            continue;
        }
        let Some((_, rendered_object, _, mut transform, sprite)) = object_sprites
            .iter_mut()
            .find(|(_, rendered, _, _, _)| rendered.object_index == object_index)
        else {
            return false;
        };
        let Some(size) = sprite.custom_size else {
            return false;
        };
        let (x, y) = if let Some(from) = walking_from {
            let remaining = if trainer_is_walking {
                object_walk_frame_ticks
            } else {
                let Some(remaining) = walking_object_id
                    .and_then(|object_id| object_walk_frame_ticks_by_id.get(object_id).copied())
                else {
                    return false;
                };
                remaining
            };
            let total_ticks = if trainer_is_walking {
                object_walk_total_ticks
            } else {
                let Some(total) = walking_object_id
                    .and_then(|object_id| object_walk_total_ticks_by_id.get(object_id).copied())
                else {
                    return false;
                };
                total
            };
            if total_ticks == 0 {
                return false;
            }
            if remaining > 0 {
                let target = render_tile_playfield_position(view_x, view_y);
                let Some((from_view_x, from_view_y)) =
                    runtime_event_view_tile(from, start_x, start_y)
                else {
                    return false;
                };
                let from = render_tile_playfield_position(from_view_x, from_view_y);
                let progress = visible_movement_progress_with_subframe(
                    remaining,
                    total_ticks,
                    movement_subframe,
                );
                overworld_sprite_position_from_base(
                    from.0 + (target.0 - from.0) * progress,
                    from.1 + (target.1 - from.1) * progress,
                    size,
                )
            } else {
                overworld_sprite_position(view_x, view_y, size)
            }
        } else {
            overworld_sprite_position(view_x, view_y, size)
        };
        transform.translation.x = x + camera_offset.x;
        transform.translation.y = y + camera_offset.y;
        let Some(source_object_slot) = snapshot.visible_object_slots.get(object_index).copied()
        else {
            return false;
        };
        transform.translation.z = if rendered_object.above_priority {
            2.41
        } else {
            overworld_entity_depth(object_tile, Some(source_object_slot), (start_x, start_y))
        };
    }
    true
}

/// TypeScript's render list follows ASM OBJ priority: map Y first, then X,
/// then lower object slots on top. The player uses the sentinel largest slot,
/// so an NPC wins an otherwise exact tie. Keep each subordinate component
/// smaller than one unit of the preceding component.
fn overworld_entity_depth(
    tile: TilePosition,
    object_index: Option<usize>,
    viewport_origin: (i16, i16),
) -> f32 {
    const Y_UNIT: f32 = 0.01;
    const X_UNIT: f32 = 0.000_01;
    const OBJECT_PRIORITY_UNIT: f32 = 0.000_000_3;
    const OBJECT_SLOT_LIMIT: usize = 16;

    let object_priority = object_index
        .map(|index| OBJECT_SLOT_LIMIT.saturating_sub(index.min(OBJECT_SLOT_LIMIT)) as f32)
        .unwrap_or(0.0)
        * OBJECT_PRIORITY_UNIT;
    // Only relative on-LCD position participates in OAM sorting. Absolute map
    // rows made ordinary actors on tall stock maps exceed the priority-layer
    // z=2.4 and draw through roofs. Clamp to the one-tile movement margin so
    // even an entering/leaving actor remains strictly below priority tiles.
    let column =
        (i32::from(tile.x) - i32::from(viewport_origin.0)).clamp(-1, i32::from(VIEWPORT_TILES_X));
    let row =
        (i32::from(tile.y) - i32::from(viewport_origin.1)).clamp(-1, i32::from(VIEWPORT_TILES_Y));
    1.0 + row as f32 * Y_UNIT + column as f32 * X_UNIT + object_priority
}

fn connection_composite_viewport_origin(
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    player_x: i16,
    player_y: i16,
    base_width: i16,
    base_height: i16,
) -> Option<(i16, i16)> {
    let mut min_x = 0_i32;
    let mut min_y = 0_i32;
    let mut max_x = i32::from(base_width);
    let mut max_y = i32::from(base_height);
    for connection in &map.attributes.connections {
        let Some(target) = snapshot
            .maps
            .iter()
            .find(|candidate| candidate.map_name == connection.target_map)
        else {
            continue;
        };
        let target_width = i32::from(target.attributes.width) * i32::from(RENDER_METATILE_WIDTH);
        let target_height = i32::from(target.attributes.height) * i32::from(RENDER_METATILE_WIDTH);
        let offset = connection
            .offset
            .saturating_mul(i32::from(RENDER_METATILE_WIDTH));
        let (x, y) = match connection.direction.to_ascii_lowercase().as_str() {
            "north" => (offset, -target_height),
            "south" => (offset, i32::from(base_height)),
            "west" => (-target_width, offset),
            "east" => (i32::from(base_width), offset),
            _ => continue,
        };
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x.saturating_add(target_width));
        max_y = max_y.max(y.saturating_add(target_height));
    }
    let viewport_width = i32::from(VIEWPORT_TILES_X);
    let viewport_height = i32::from(VIEWPORT_TILES_Y);
    let x = (i32::from(player_x) - viewport_width / 2)
        .clamp(min_x, max_x.saturating_sub(viewport_width).max(min_x));
    let y = (i32::from(player_y) - viewport_height / 2)
        .clamp(min_y, max_y.saturating_sub(viewport_height).max(min_y));
    Some((i16::try_from(x).ok()?, i16::try_from(y).ok()?))
}

struct ResolvedRenderSource<'a> {
    map: &'a crate::RuntimeMapCatalogSnapshot,
    tileset: &'a crate::RuntimeTilesetCatalogSnapshot,
    art_key: TilesetArtKey,
    origin_x: i32,
    origin_y: i32,
    width: i32,
    height: i32,
}

fn resolved_render_source_at<'a>(
    sources: &'a [ResolvedRenderSource<'a>],
    x: i32,
    y: i32,
) -> (&'a ResolvedRenderSource<'a>, i32, i32) {
    let base = &sources[0];
    if (0..base.width).contains(&x) && (0..base.height).contains(&y) {
        return (base, x, y);
    }
    for source in sources[1..].iter().rev() {
        if (source.origin_x..source.origin_x.saturating_add(source.width)).contains(&x)
            && (source.origin_y..source.origin_y.saturating_add(source.height)).contains(&y)
        {
            return (source, x - source.origin_x, y - source.origin_y);
        }
    }
    // The ASM border block belongs to the active map. Coordinates outside
    // every connected rectangle deliberately remain in the base coordinate
    // space so its repeating sub-tile phase is preserved.
    (base, x, y)
}

fn priority_collision_token(token: &str) -> bool {
    matches!(
        token,
        "TALL_GRASS"
            | "LONG_GRASS"
            | "LONG_GRASS_1C"
            | "GRASS_48"
            | "GRASS_49"
            | "GRASS_4A"
            | "GRASS_4B"
            | "GRASS_4C"
            | "BOOKSHELF"
            | "COUNTER"
            | "COUNTER_98"
            | "INCENSE_BURNER"
            | "MART_SHELF"
            | "PC"
    )
}

fn grass_collision_token(token: &str) -> bool {
    matches!(
        token,
        "CUT_08"
            | "TALL_GRASS"
            | "TALL_GRASS_10"
            | "LONG_GRASS"
            | "LONG_GRASS_1C"
            | "CUT_28"
            | "GRASS_48"
            | "GRASS_49"
            | "GRASS_4A"
            | "GRASS_4B"
            | "GRASS_4C"
    )
}

fn object_uses_item_ball_priority(object: &crate::core::map::ObjectEvent) -> bool {
    // Elm's starter balls are OBJECTTYPE_SCRIPT in the ASM so their scripts
    // can run the starter-choice flow, but they still use ordinary item-ball
    // OAM priority and must remain visible over the desk's priority tiles.
    object.object_type == "OBJECTTYPE_ITEMBALL" || object.sprite == "SPRITE_POKE_BALL"
}

fn visible_object_indices_above_priority(
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    tileset: &crate::RuntimeTilesetCatalogSnapshot,
) -> BTreeSet<usize> {
    snapshot
        .visible_objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object_uses_item_ball_priority(object))
        .filter_map(|(index, object)| {
            let tile = object
                .object_identifier
                .as_ref()
                .and_then(|object_id| {
                    snapshot
                        .visible_object_runtime_tiles
                        .get(object_id)
                        .copied()
                })
                .or_else(|| object_tile_position_checked(object))?;
            let stride = crate::core::world::map::METATILE_WIDTH;
            let block_x = usize::try_from(tile.x.div_euclid(stride)).ok()?;
            let block_y = usize::try_from(tile.y.div_euclid(stride)).ok()?;
            let block_index = block_y
                .checked_mul(usize::from(map.attributes.width))?
                .checked_add(block_x)?;
            let block = *map.blocks.get(block_index)?;
            let collision = tileset_collision_tokens(tileset, block)?;
            let quadrant = usize::try_from(tile.y.rem_euclid(stride))
                .ok()?
                .checked_mul(2)?
                .checked_add(usize::try_from(tile.x.rem_euclid(stride)).ok()?)?;
            let token = collision.get(quadrant)?;
            (!grass_collision_token(token)).then_some(index)
        })
        .collect()
}

/// Offset the already-composed destination viewport back toward the previous
/// camera origin. This keeps the tile layer, player, and NPCs in the same
/// coordinate space during a walking scroll.
fn overworld_walk_camera_offset(rendered: &RenderedViewport, frames_remaining: u8) -> Vec2 {
    overworld_walk_camera_offset_for_duration(rendered, frames_remaining, WALK_FRAME_HOLD_TICKS)
}

fn overworld_walk_camera_offset_for_duration(
    rendered: &RenderedViewport,
    frames_remaining: u8,
    total_frames: u8,
) -> Vec2 {
    overworld_walk_camera_offset_for_duration_with_subframe(
        rendered,
        frames_remaining,
        total_frames,
        1.0,
    )
}

fn overworld_walk_camera_offset_for_duration_with_subframe(
    rendered: &RenderedViewport,
    frames_remaining: u8,
    total_frames: u8,
    movement_subframe: f32,
) -> Vec2 {
    let Some((from_x, from_y)) = rendered.walk_viewport_origin else {
        return Vec2::ZERO;
    };
    let Some((to_x, to_y)) = rendered.viewport_origin else {
        return Vec2::ZERO;
    };
    if frames_remaining == 0 {
        return Vec2::ZERO;
    }
    // Smooth presentation samples the interval leading into the next LCD
    // tick. Sampling the already-advanced endpoint here creates a terminal
    // half-frame hold followed by a double-sized jump when held movement
    // starts the next tile.
    let remaining = 1.0
        - visible_movement_progress_with_subframe(
            frames_remaining,
            total_frames,
            movement_subframe,
        );
    Vec2::new(
        f32::from(to_x - from_x) * TILE_SIZE * remaining,
        -f32::from(to_y - from_y) * TILE_SIZE * remaining,
    )
}

fn visible_overworld_camera_offset(
    rendered: &RenderedViewport,
    runtime_shell: &BevyRuntimeShell,
    movement_subframe: f32,
) -> Vec2 {
    if let Some(jump) = runtime_shell.visible_ledge_jump {
        // Keep the camera on the same two-source-pixels-per-frame schedule as
        // the 32-pixel ledge traversal, including its terminal landing update.
        return overworld_walk_camera_offset_for_duration_with_subframe(
            rendered,
            16_u8.saturating_sub(jump.frame),
            16,
            movement_subframe,
        );
    }
    overworld_walk_camera_offset_for_duration_with_subframe(
        rendered,
        runtime_shell.player_walk_frame_ticks,
        runtime_shell.player_walk_total_ticks,
        movement_subframe,
    )
}

/// ASM `UpdateJumpPosition`, projected from source LCD pixels into Bevy's
/// integer-scaled playfield. A ledge step travels two source pixels per frame
/// while this table raises the actor independently above its ground/shadow.
fn visible_ledge_jump_y_offset(jump: Option<VisibleLedgeJump>) -> f32 {
    const OFFSETS: [i8; 16] = [
        -4, -6, -8, -10, -11, -12, -12, -12, -11, -10, -9, -8, -6, -4, 0, 0,
    ];
    let Some(jump) = jump else {
        return 0.0;
    };
    let source_offset = OFFSETS[usize::from(jump.frame.min(15))];
    -f32::from(source_offset) * (TILE_SIZE / SOURCE_TILE_SIZE as f32)
}

fn visible_player_playfield_position(
    target: TilePosition,
    from: Option<TilePosition>,
    frames_remaining: u8,
    start_x: i16,
    start_y: i16,
) -> Option<(f32, f32)> {
    visible_player_playfield_position_for_duration(
        target,
        from,
        frames_remaining,
        WALK_FRAME_HOLD_TICKS,
        start_x,
        start_y,
    )
}

fn visible_player_playfield_position_for_duration(
    target: TilePosition,
    from: Option<TilePosition>,
    frames_remaining: u8,
    total_frames: u8,
    start_x: i16,
    start_y: i16,
) -> Option<(f32, f32)> {
    visible_player_playfield_position_for_duration_with_subframe(
        target,
        from,
        frames_remaining,
        total_frames,
        1.0,
        start_x,
        start_y,
    )
}

fn visible_player_playfield_position_for_duration_with_subframe(
    target: TilePosition,
    from: Option<TilePosition>,
    frames_remaining: u8,
    total_frames: u8,
    movement_subframe: f32,
    start_x: i16,
    start_y: i16,
) -> Option<(f32, f32)> {
    let target = runtime_tile_playfield_position(target, start_x, start_y)?;
    let Some(from) = from.filter(|_| frames_remaining > 0) else {
        return Some(target);
    };
    // A seamless map connection commits the authority onto the destination
    // map before its final stride is drawn. Its reconstructed source tile is
    // therefore legitimately one tile beyond the destination viewport. Do
    // not apply the static-object visibility clamp to that retained origin:
    // Crystal scrolls the player in from offscreen over the complete step.
    let (from_view_x, from_view_y) = runtime_event_view_tile(from, start_x, start_y)?;
    let from = render_tile_playfield_position(from_view_x, from_view_y);
    // Render continuously from the position before this authoritative tick
    // toward the next one. The discrete helper below still preserves
    // TypeScript's exact advance-before-draw samples for scripts and tests.
    let progress =
        visible_movement_progress_with_subframe(frames_remaining, total_frames, movement_subframe);
    Some((
        from.0 + (target.0 - from.0) * progress,
        from.1 + (target.1 - from.1) * progress,
    ))
}

fn visible_tracking_object_playfield_position(
    target: TilePosition,
    from: Option<TilePosition>,
    frames_remaining: u8,
    total_frames: u8,
    movement_subframe: f32,
    start_x: i16,
    start_y: i16,
    camera_offset: Vec2,
) -> Option<(f32, f32)> {
    let (x, y) = visible_player_playfield_position_for_duration_with_subframe(
        target,
        from,
        frames_remaining,
        total_frames,
        movement_subframe,
        start_x,
        start_y,
    )?;
    Some((x + camera_offset.x, y + camera_offset.y))
}

fn visible_movement_progress(frames_remaining: u8, total_frames: u8) -> f32 {
    let total_frames = total_frames.max(1);
    if frames_remaining == 0 {
        return 1.0;
    }
    f32::from(
        total_frames
            .saturating_sub(frames_remaining.min(total_frames))
            .saturating_add(1),
    ) / f32::from(total_frames)
}

fn visible_movement_progress_with_subframe(
    frames_remaining: u8,
    total_frames: u8,
    movement_subframe: f32,
) -> f32 {
    let total_frames = total_frames.max(1);
    if frames_remaining == 0 {
        return 1.0;
    }
    let completed_frames =
        f32::from(total_frames.saturating_sub(frames_remaining.min(total_frames)));
    ((completed_frames + movement_subframe.clamp(0.0, 1.0)) / f32::from(total_frames)).min(1.0)
}

fn hash_menu_cursor(hasher: &mut impl Hasher, cursor: &Option<MenuCursor>) {
    match cursor {
        Some(cursor) => {
            true.hash(hasher);
            cursor.surface_id.hash(hasher);
            cursor.option_index.hash(hasher);
        }
        None => false.hash(hasher),
    }
}

fn spawn_visible_intro_screen(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    intro: &VisibleIntroScreen,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = intro_scene_frame_for_art_with_bundle(
        rendered_art,
        &runtime_shell.asset_root,
        runtime_shell
            .shell
            .runtime()
            .data()
            .sprite_anim_bundle
            .as_str(),
        intro,
        images,
    )
    .ok_or_else(|| {
        let key = intro_scene_art_key(intro);
        let error = rendered_art
            .intro_scene_errors
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "unknown intro art load error".to_string());
        anyhow::anyhow!(
            "required intro scene {} frame {} could not be rendered: {}",
            intro.scene_name(),
            intro.scene_frame_counter,
            error
        )
    })?;
    // `intro_scene_frame_for_art_with_bundle` has already committed the pixels
    // into the shell-wide retained LCD allocation. Only create its presenter
    // when entering a full-screen sequence from the overworld.
    ensure_presented_fullscreen_entity(commands, rendered_art, &frame, PRESENTED_FULLSCREEN_BASE_Z);
    Ok(())
}

fn visible_intro_display_size() -> Vec2 {
    Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)
}

fn spawn_title_screen(
    commands: &mut Commands,
    runtime_shell: &mut BevyRuntimeShell,
    title: &TitleMenu,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    if let Some(continue_screen) = runtime_shell.visible_continue_screen.as_ref() {
        let frame = load_visible_continue_screen_frame(
            runtime_shell,
            continue_screen,
            rendered_art,
            images,
        )?;
        commit_presented_fullscreen_frame(
            commands,
            rendered_art,
            &frame,
            PresentedFullscreenFrameSource::Transient,
            PRESENTED_FULLSCREEN_BASE_Z,
            images,
        )?;
        return Ok(());
    }
    if visible_title_main_menu_active(title) {
        let frame = load_visible_title_main_menu_frame(runtime_shell, title, rendered_art, images)?;
        commit_presented_fullscreen_frame(
            commands,
            rendered_art,
            &frame,
            PresentedFullscreenFrameSource::Transient,
            PRESENTED_FULLSCREEN_BASE_Z,
            images,
        )?;
        return Ok(());
    }
    let frame = title_screen_frame_for_art(rendered_art, &runtime_shell.asset_root, title, images)
        .ok_or_else(|| {
            let key = title_screen_art_key(title);
            let error = rendered_art
                .title_screen_errors
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "unknown title screen art load error".to_string());
            anyhow::anyhow!("required native title screen art could not be rendered: {error}")
        })?;
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Cached,
        PRESENTED_FULLSCREEN_BASE_Z,
        images,
    )?;
    if runtime_debug_overlays_enabled() {
        let boot = runtime_shell.runtime.boot_summary();
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    format!(
                        "Rust pack={} hash={}",
                        boot.modpack_id, boot.pack_content_hash
                    ),
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.92, 0.92, 0.82),
                        ..default()
                    },
                ),
                transform: Transform::from_xyz(-190.0, 82.0, 1.0),
                ..default()
            },
            TitleScreenMarker,
        ));
    }
    Ok(())
}
