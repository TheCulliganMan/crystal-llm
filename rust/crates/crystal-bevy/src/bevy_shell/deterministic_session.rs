fn visible_deterministic_session_checkpoint(
    shell: &RuntimeGameShell,
    checksum: StateChecksum,
) -> Result<SessionSaveCheckpointFrame> {
    let snapshot = shell.snapshot()?;
    let session_id = format!("bevy-local-start-{}", checksum.frame());
    let descriptor = shell.link_session_descriptor(
        session_id,
        LOCAL_PLAYER_ID,
        snapshot.trainer.player_name.clone(),
    )?;
    if descriptor.checksum.frame() != checksum.frame()
        || descriptor.checksum.hash() != checksum.hash()
    {
        anyhow::bail!(
            "deterministic session checkpoint frame/hash {} {:#010x} does not match session start {} {:#010x}",
            descriptor.checksum.frame(),
            descriptor.checksum.hash(),
            checksum.frame(),
            checksum.hash()
        );
    }
    Ok(descriptor.save_checkpoint)
}

fn required_visible_deterministic_session_checkpoint(
    runtime_shell: &BevyRuntimeShell,
) -> Result<&SessionSaveCheckpointFrame> {
    runtime_shell
        .deterministic_session_checkpoint
        .as_ref()
        .context("deterministic session checkpoint requires confirmed trainer identity")
}

fn setup_shell_view(mut commands: Commands) {
    // Begin with Camera2dBundle's specialized projection. Constructing an
    // OrthographicProjection from its generic Default loses Bevy's 2D depth
    // range and culls every positive-z LCD sprite.
    let mut camera = Camera2dBundle::default();
    camera.projection.scaling_mode = bevy::render::camera::ScalingMode::WindowSize(1.0);
    commands.spawn((
        camera,
        MainCameraMarker,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(640.0, 576.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 100.0),
            ..default()
        },
        ScreenFadeOverlay,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(230.0 / 255.0, 173.0 / 255.0, 1.0, 0.0),
                custom_size: Some(Vec2::new(640.0, 576.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 99.0),
            ..default()
        },
        PoisonFlashOverlay,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.06, 0.08, 0.10, 0.0),
                custom_size: Some(Vec2::new(612.0, 116.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -222.0, 5.0),
            ..default()
        },
        DialogPanel,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.07, 0.09, 0.12, 0.0),
                custom_size: Some(Vec2::new(302.0, 128.0)),
                ..default()
            },
            transform: Transform::from_xyz(165.0, 216.0, 4.0),
            ..default()
        },
        BattlePanel,
    ));
    commands.spawn((
        TextBundle::from_section(
            "Loading runtime...",
            TextStyle {
                font_size: 18.0,
                color: Color::rgb(0.88, 0.94, 0.86),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            top: Val::Px(18.0),
            max_width: Val::Px(604.0),
            ..default()
        }),
        StatusText,
    ));
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 20.0,
                color: Color::rgb(0.97, 0.97, 0.90),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(22.0),
            bottom: Val::Px(20.0),
            max_width: Val::Px(596.0),
            ..default()
        }),
        DialogText,
    ));
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 18.0,
                color: Color::rgb(0.94, 0.97, 0.99),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(18.0),
            max_width: Val::Px(290.0),
            ..default()
        }),
        BattleText,
    ));
}

fn apply_keyboard_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    rtc_source: Res<NativeRtcSource>,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut timer: ResMut<RuntimeTickTimer>,
) {
    timer.tick(time.delta_seconds_f64());
    let elapsed_vblanks = timer.take_vblanks();
    let elapsed_input_ticks = timer.take_ticks();
    timer.stage_presentation_ticks(elapsed_input_ticks);
    if elapsed_vblanks == 0 && elapsed_input_ticks == 0 {
        return;
    }
    // GameTimer is a VBlank hook, not an overworld-input side effect. Run it
    // for every authoritative catch-up VBlank before any presentation/modal
    // early return; dialogue, menus, battles, and interpolation all consume
    // real play time unless the source gates explicitly pause it.
    if elapsed_vblanks > 0 {
        if let Err(error) = runtime_shell
            .shell
            .advance_game_timer_vblanks(elapsed_vblanks)
        {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
            return;
        }
    }
    let rtc_sample = (*rtc_source).sample();
    let mut rtc_changed = runtime_shell.latest_rtc_sample != Some(rtc_sample);
    runtime_shell.latest_rtc_sample = Some(rtc_sample);
    runtime_shell.lcd_animation_frame = runtime_shell.lcd_animation_frame.wrapping_add(1);
    let text_acceleration_requested =
        keys.pressed(KeyCode::KeyZ) || keys.pressed(KeyCode::KeyX);
    let ambient_phase_changed = runtime_shell.ambient_tileset_animation_active
        && runtime_shell
            .ambient_tileset_animation_schedule
            .iter()
            .any(|(period, offset)| {
                runtime_shell.lcd_animation_frame >= *offset
                    && (runtime_shell.lcd_animation_frame - *offset) % (*period).max(1) == 0
            });
    if ambient_phase_changed {
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if (runtime_shell.battle_lcd_animation_active
        || runtime_shell.field_text_reveal.is_some())
        && runtime_shell.lcd_animation_frame % 8 == 0
    {
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if runtime_shell.shell.session().overworld.following.is_none()
        && (!runtime_shell.pending_follower_walks.is_empty()
            || !runtime_shell.follower_visible_tile_overrides.is_empty())
    {
        let follower_ids = runtime_shell
            .follower_visible_tile_overrides
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        runtime_shell.pending_follower_walks.clear();
        runtime_shell.follower_visible_tile_overrides.clear();
        for object_id in follower_ids {
            runtime_shell.object_walk_from.remove(&object_id);
            runtime_shell.object_walk_frame_ticks_by_id.remove(&object_id);
            runtime_shell.object_walk_total_ticks_by_id.remove(&object_id);
        }
    }
    let player_landed = runtime_shell.player_walk_frame_ticks == 1;
    runtime_shell.player_walk_frame_ticks = runtime_shell.player_walk_frame_ticks.saturating_sub(1);
    if runtime_shell.player_walk_frame_ticks == 0 {
        runtime_shell.player_walk_from = None;
    }
    if player_landed {
        // Retained origins model OBJECT_LAST_MAP_* only while the sprite is
        // visibly in flight. Forced ice/current/downhill chains pause core
        // ticks during interpolation, so frame-number expiry alone can leave
        // an already-landed origin collision-owned across later tiles.
        let overworld = &mut runtime_shell.shell.session_mut().overworld;
        overworld.player_last_runtime_tile = None;
        overworld.player_last_tile_occupied_until_frame = 0;
    }
    runtime_shell.object_walk_frame_ticks = runtime_shell.object_walk_frame_ticks.saturating_sub(1);
    for remaining in runtime_shell.object_walk_frame_ticks_by_id.values_mut() {
        *remaining = remaining.saturating_sub(1);
    }
    let landed_object_ids = runtime_shell
        .object_walk_frame_ticks_by_id
        .iter()
        .filter_map(|(object_id, remaining)| (*remaining == 0).then_some(object_id.clone()))
        .collect::<Vec<_>>();
    let object_landed = !landed_object_ids.is_empty();
    for object_id in landed_object_ids {
        runtime_shell.object_walk_frame_ticks_by_id.remove(&object_id);
        runtime_shell.object_walk_total_ticks_by_id.remove(&object_id);
        runtime_shell.object_walk_from.remove(&object_id);
        runtime_shell
            .follower_visible_tile_overrides
            .remove(&object_id);
        let overworld = &mut runtime_shell.shell.session_mut().overworld;
        overworld.object_last_runtime_tiles.remove(&object_id);
        overworld
            .object_last_tiles_occupied_until_frame
            .remove(&object_id);
    }
    if player_landed || object_landed {
        start_next_queued_follower_walk(&mut runtime_shell);
    }
    if runtime_shell.object_walk_frame_ticks == 0 {
        runtime_shell.trainer_walk_from = None;
        if runtime_shell.pending_trainer_sight.is_none()
            && runtime_shell.object_walk_frame_ticks_by_id.is_empty()
        {
            runtime_shell.object_walk_stride = false;
        }
    }
    if runtime_shell.battle_switch_cursor.is_some() {
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if let Some(emote) = runtime_shell.visible_overworld_emote.as_mut() {
        emote.frames_remaining = emote.frames_remaining.saturating_sub(1);
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if let Some(earthquake) = runtime_shell.visible_earthquake.as_mut() {
        earthquake.frames_remaining = earthquake.frames_remaining.saturating_sub(1);
        earthquake.phase = earthquake.phase.wrapping_add(1) % 4;
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if runtime_shell
        .visible_overworld_emote
        .as_ref()
        .is_some_and(|emote| emote.frames_remaining == 0)
    {
        if let Err(error) = drain_visible_emotes(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(format!("{error:#}"));
        }
        return;
    }
    if runtime_shell
        .visible_earthquake
        .as_ref()
        .is_some_and(|earthquake| earthquake.frames_remaining == 0)
    {
        if let Err(error) = drain_visible_earthquakes(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(format!("{error:#}"));
        }
        return;
    }
    let mut field_object_effect_advanced = false;
    let field_notice_effect_finished = if runtime_shell.field_notice.is_none() {
        if let Some(frames) = runtime_shell.pending_field_notice_effect_frames {
            let frames = frames.saturating_sub(1);
            runtime_shell.pending_field_notice_effect_frames = Some(frames);
            if let Some(cut) = runtime_shell.visible_cut_animation.as_mut() {
                cut.frame = 32_u8.saturating_sub(frames);
                field_object_effect_advanced = true;
            }
            if let Some(whirlpool) = runtime_shell.visible_whirlpool_animation.as_mut() {
                whirlpool.frame = 32_u8.saturating_sub(frames);
                field_object_effect_advanced = true;
            }
            if let Some(headbutt) = runtime_shell.visible_headbutt_animation.as_mut() {
                headbutt.frame = 32_u8.saturating_sub(frames);
                field_object_effect_advanced = true;
            }
            if let Some(flash) = runtime_shell.visible_flash_animation.as_mut() {
                flash.frame = 16_u8.saturating_sub(frames);
                field_object_effect_advanced = true;
                if flash.frame == 8 {
                    runtime_shell.field_notice_scene = None;
                }
            }
            frames == 0
        } else {
            false
        }
    } else {
        false
    };
    if field_object_effect_advanced {
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if field_notice_effect_finished {
        runtime_shell.pending_field_notice_effect_frames = None;
        runtime_shell.field_notice_scene = None;
        runtime_shell.visible_earthquake = None;
        runtime_shell.visible_cut_animation = None;
        runtime_shell.visible_whirlpool_animation = None;
        runtime_shell.visible_headbutt_animation = None;
        runtime_shell.visible_flash_animation = None;
        runtime_shell.visible_sweet_scent_delay = false;
        runtime_shell.pending_surf_start_from = None;
        if std::mem::take(&mut runtime_shell.pending_field_battle_entry) {
            if let Err(error) = prepare_visible_battle_entry(&mut runtime_shell)
                .and_then(|_| settle_visible_battle_after_action(&mut runtime_shell))
            {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
            }
        } else if let Some(next) = runtime_shell.field_notice_queue.pop_front() {
            runtime_shell.field_notice = Some(next);
        }
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if runtime_shell.field_notice.is_none() {
        if let Some(animation) = runtime_shell.visible_waterfall_animation {
            let Some(total_frames) = animation.steps.checked_mul(4) else {
                let error = anyhow::anyhow!(
                    "WATERFALL visual duration overflows for {} steps",
                    animation.steps
                );
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            };
            if animation.frame >= total_frames {
                runtime_shell.visible_waterfall_animation = None;
                runtime_shell.field_notice_scene = None;
                runtime_shell.player_walk_from = None;
                runtime_shell.player_walk_frame_ticks = 0;
                runtime_shell.player_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
                runtime_shell.player_walk_stride = false;
                runtime_shell.player_walk_mirror_stride = false;
            } else {
                let step_index = animation.frame / 4;
                let phase = (animation.frame % 4) as u8;
                let Ok(step_index_i16) = i16::try_from(step_index) else {
                    let error = anyhow::anyhow!(
                        "WATERFALL visual step {step_index} exceeds runtime tile coordinates"
                    );
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                };
                let Some(step_offset) = step_index_i16.checked_mul(METATILE_WIDTH) else {
                    let error = anyhow::anyhow!(
                        "WATERFALL visual step offset overflows at step {step_index}"
                    );
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                };
                let Some(segment_from_y) = animation.from_tile.y.checked_sub(step_offset) else {
                    let error = anyhow::anyhow!(
                        "WATERFALL visual origin underflows at step {step_index}"
                    );
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                };
                let Some(segment_to_y) = segment_from_y.checked_sub(METATILE_WIDTH) else {
                    let error = anyhow::anyhow!(
                        "WATERFALL visual destination underflows at step {step_index}"
                    );
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                };
                let segment_from = TilePosition { x: animation.from_tile.x, y: segment_from_y };
                let segment_to = TilePosition { x: segment_from.x, y: segment_to_y };
                if step_index + 1 == animation.steps
                    && segment_to != animation.to_tile
                {
                    let error = anyhow::anyhow!(
                        "WATERFALL visual path ended at ({}, {}) instead of authoritative ({}, {})",
                        segment_to.x,
                        segment_to.y,
                        animation.to_tile.x,
                        animation.to_tile.y,
                    );
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                }
                if let Some(scene) = runtime_shell.field_notice_scene.as_mut() {
                    Arc::make_mut(scene).overworld.tile = segment_to;
                }
                runtime_shell.player_walk_from = Some(segment_from);
                runtime_shell.player_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
                runtime_shell.player_walk_frame_ticks =
                    WALK_FRAME_HOLD_TICKS.saturating_sub(phase.saturating_mul(2));
                runtime_shell.player_walk_stride = step_index & 1 == 0;
                runtime_shell.player_walk_mirror_stride = step_index % 4 >= 2;
                if let Some(animation) = runtime_shell.visible_waterfall_animation.as_mut() {
                    animation.frame = animation.frame.saturating_add(1);
                }
            }
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            return;
        }
    }
    if let Some(animation) = runtime_shell.visible_fly_animation {
        let counter = match animation.phase {
            VisibleFlyAnimationPhase::From => 128_u8.saturating_sub(animation.frame),
            VisibleFlyAnimationPhase::To => 64_u8.saturating_sub(animation.frame),
        };
        if animation.frame > 0 && counter >= 0x40 && counter & 7 == 0 {
            let BevyRuntimeShell {
                shell,
                pending_audio,
                last_audio_events,
                ..
            } = &mut *runtime_shell;
            if let Err(error) = queue_visible_sound_effect(
                shell.runtime().audio(),
                pending_audio,
                last_audio_events,
                "SFX_FLY",
            ) {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        }
        match animation.phase {
            VisibleFlyAnimationPhase::From => {
                if animation.frame >= 128 {
                    runtime_shell.field_notice_scene = None;
                    if let Err(error) = settle_visible_overworld_travel(&mut runtime_shell) {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(error.to_string());
                        return;
                    }
                    runtime_shell.visible_fly_animation = Some(VisibleFlyAnimation {
                        phase: VisibleFlyAnimationPhase::To,
                        frame: 0,
                    });
                    let BevyRuntimeShell {
                        shell,
                        pending_audio,
                        last_audio_events,
                        ..
                    } = &mut *runtime_shell;
                    if let Err(error) = queue_visible_sound_effect(
                        shell.runtime().audio(),
                        pending_audio,
                        last_audio_events,
                        "SFX_FLY",
                    ) {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(error.to_string());
                        return;
                    }
                } else if let Some(animation) = runtime_shell.visible_fly_animation.as_mut() {
                    animation.frame = animation.frame.saturating_add(1);
                }
            }
            VisibleFlyAnimationPhase::To => {
                if animation.frame >= 64 {
                    runtime_shell.visible_fly_animation = None;
                } else if let Some(animation) = runtime_shell.visible_fly_animation.as_mut() {
                    animation.frame = animation.frame.saturating_add(1);
                }
            }
        }
        mark_runtime_snapshot_dirty(&mut runtime_shell);
        return;
    }
    if runtime_shell
        .visible_fishing_animation
        .is_some_and(|animation| animation.phase != VisibleFishingPhase::AwaitText)
    {
        advance_visible_fishing_animation(&mut runtime_shell);
        return;
    }
    if let Some(frame) = runtime_shell.visible_diploma.as_mut() {
        *frame = frame.wrapping_add(1);
        mark_runtime_snapshot_dirty(&mut runtime_shell);
        return;
    }
    if runtime_shell.visible_heal_machine.is_some() {
        if let Err(error) = advance_visible_heal_machine(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell.visible_magnet_train.is_some() {
        if let Err(error) = advance_visible_magnet_train(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell.visible_battle_transition.is_some() {
        let waiting_for_step = matches!(
            runtime_shell.pending_overworld_step_boundary,
            Some(PendingOverworldStepBoundary::WildBattle)
        );
        if waiting_for_step {
            if runtime_shell.player_walk_frame_ticks > 0
                || runtime_shell.visible_ledge_jump.is_some()
            {
                return;
            }
        } else {
            advance_visible_battle_transition(&mut runtime_shell);
            return;
        }
    }
    if runtime_shell.visible_frontpic_animation.is_some() {
        if let Err(error) = advance_visible_frontpic_animation(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
            runtime_shell.visible_frontpic_animation = None;
        }
        // AnimateFrontpic owns the complete frame in Crystal. Even when the
        // final command lands this tick, do not also advance battle logic or
        // accept a command-menu input beneath the sprite animation.
        return;
    }
    if runtime_shell
        .visible_capture_animation
        .as_ref()
        .is_some_and(|animation| animation.started)
    {
        if let Err(error) = advance_visible_capture_animation(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell
        .visible_move_animations
        .front()
        .is_some_and(|animation| animation.started)
    {
        if let Err(error) = advance_visible_move_animation(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell.visible_send_out_animation.is_some() {
        if let Err(error) = advance_visible_send_out_animation(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell.visible_trainer_exit_animation.is_some() {
        if let Err(error) = advance_visible_trainer_exit_animation(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell
        .visible_blackout_phase
        .is_some_and(|phase| phase != VisibleBlackoutPhase::AwaitText)
    {
        return;
    }
    if runtime_shell.visible_walk_warp_phase.is_some() {
        return;
    }
    if let Some(jump) = runtime_shell.visible_ledge_jump.as_mut() {
        if jump.frame < 15 {
            jump.frame += 1;
        } else {
            runtime_shell.visible_ledge_jump = None;
            let overworld = &mut runtime_shell.shell.session_mut().overworld;
            overworld.player_last_runtime_tile = None;
            overworld.player_last_tile_occupied_until_frame = 0;
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
    }
    // A ledge jump is two chained eight-frame steps. Core reports the final
    // landing tile atomically, but its grass effect belongs to the second
    // step and must not age during the takeoff half.
    let landing_grass_not_started = runtime_shell
        .visible_ledge_jump
        .is_some_and(|jump| jump.frame <= WALK_FRAME_HOLD_TICKS);
    if let Some(rustle) = runtime_shell.visible_grass_rustle.as_mut() {
        if !landing_grass_not_started {
            rustle.age = rustle.age.saturating_add(1);
            rustle.frames_remaining = rustle.frames_remaining.saturating_sub(1);
            if rustle.frames_remaining == 0 {
                runtime_shell.visible_grass_rustle = None;
            }
        }
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if let Some(dust) = runtime_shell.visible_strength_boulder_dust.as_mut() {
        dust.age = dust.age.saturating_add(1);
        dust.frames_remaining = dust.frames_remaining.saturating_sub(1);
        if dust.frames_remaining == 0 {
            runtime_shell.visible_strength_boulder_dust = None;
        }
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if runtime_shell.visible_script_movement.is_some() {
        match advance_visible_script_movement(&mut runtime_shell) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        }
    }
    if !runtime_shell.battle_messages.is_empty()
        && !runtime_shell
            .visible_move_animations
            .front()
            .is_some_and(|animation| animation.started)
    {
        match runtime_shell.shell.presentation_snapshot() {
            Ok(snapshot) => {
                if advance_visible_battle_text_reveal(
                    &mut runtime_shell,
                    &snapshot,
                    text_acceleration_requested,
                ) {
                    mark_runtime_snapshot_dirty(&mut runtime_shell);
                }
            }
            Err(error) => {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        }
    } else if runtime_shell.battle_text_reveal.take().is_some() {
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if let Some(tween) = runtime_shell.battle_hp_tween.as_mut() {
        let player_changed = advance_visible_hp_pixels(
            &mut tween.player_pixels,
            tween.player_target_pixels,
            &mut tween.player_frames_until_step,
        );
        if player_changed {
            advance_visible_player_hp_number(tween);
        }
        let enemy_changed = advance_visible_hp_pixels(
            &mut tween.enemy_pixels,
            tween.enemy_target_pixels,
            &mut tween.enemy_frames_until_step,
        );
        if player_changed || enemy_changed {
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
    }
    let hp_tween_active = runtime_shell
        .battle_hp_tween
        .as_ref()
        .is_some_and(visible_battle_hp_tween_active);
    if !hp_tween_active
        && runtime_shell
            .visible_move_animations
            .front()
            .is_some_and(|animation| animation.waiting_for_hp)
    {
        let animation = runtime_shell.visible_move_animations.front_mut().unwrap();
        animation.waiting_for_hp = false;
        animation.started = true;
        mark_runtime_snapshot_dirty(&mut runtime_shell);
        return;
    }
    if hp_tween_active {
        // UpdateBattleHuds waits for the bar animation before battle command
        // processing resumes. Keeping this frame presentation-only also
        // prevents a hidden cursor from moving beneath the retained HUD.
        return;
    }
    let mut exp_segment_finished = false;
    let mut exp_animation_finished = false;
    let mut exp_pixels_changed = false;
    if let Some(tween) = runtime_shell.battle_exp_tween.as_mut()
        && tween.started
    {
        if tween.frames_until_step > 0 {
            tween.frames_until_step -= 1;
        } else if tween.pixels < tween.target_pixels {
            tween.pixels += 1;
            tween.steps_in_segment += 1;
            tween.frames_until_step = if tween.steps_in_segment <= 2 {
                2
            } else if tween.steps_in_segment <= 4 {
                1
            } else {
                0
            };
            exp_pixels_changed = true;
            if tween.pixels == tween.target_pixels {
                exp_segment_finished = !tween.remaining_targets.is_empty();
                exp_animation_finished = tween.remaining_targets.is_empty();
                if exp_segment_finished {
                    tween.level = tween.level.saturating_add(1).min(100);
                }
                tween.started = false;
            }
        } else {
            exp_segment_finished = !tween.remaining_targets.is_empty();
            exp_animation_finished = tween.remaining_targets.is_empty();
            if exp_segment_finished {
                tween.level = tween.level.saturating_add(1).min(100);
            }
            tween.started = false;
        }
    }
    if exp_pixels_changed {
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    if exp_segment_finished {
        if let Err(error) = queue_visible_shell_sound_effect(
            &mut runtime_shell,
            "SFX_HIT_END_OF_EXP_BAR",
        ) {
            record_visible_runtime_error(&mut runtime_shell, &error);
        }
        if runtime_shell
            .battle_fanfare_messages
            .front()
            .is_some_and(|fanfare| runtime_shell.battle_messages.front() == Some(fanfare))
        {
            runtime_shell.battle_fanfare_messages.pop_front();
            if let Err(error) = queue_visible_shell_sound_effect(
                &mut runtime_shell,
                "SFX_DEX_FANFARE_50_79",
            ) {
                record_visible_runtime_error(&mut runtime_shell, &error);
            }
        }
    }
    if exp_animation_finished {
        runtime_shell.battle_exp_tween = runtime_shell.pending_battle_exp_tweens.pop_front();
        if let Some(stats) = runtime_shell.battle_level_stats.front_mut()
            && stats.triggered
        {
            stats.active = true;
            // This activation occurs before the per-frame countdown below.
            stats.frames_before_input = 31;
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        if let Err(error) = finish_visible_empty_battle_reward_presentation(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
        }
        if runtime_shell
            .battle_fanfare_messages
            .front()
            .is_some_and(|fanfare| runtime_shell.battle_messages.front() == Some(fanfare))
        {
            runtime_shell.battle_fanfare_messages.pop_front();
            if let Err(error) = queue_visible_shell_sound_effect(
                &mut runtime_shell,
                "SFX_DEX_FANFARE_50_79",
            ) {
                record_visible_runtime_error(&mut runtime_shell, &error);
            }
        }
    }
    if let Some(stats) = runtime_shell.battle_level_stats.front_mut()
        && stats.active
        && stats.frames_before_input > 0
    {
        stats.frames_before_input -= 1;
    }
    if let Some(frames) = runtime_shell.visible_script_delay_frames.as_mut() {
        *frames = frames.saturating_sub(1);
    }
    if runtime_shell.visible_strength_notice_phase
        == Some(VisibleStrengthNoticePhase::CryPause)
        && runtime_shell.visible_script_delay_frames == Some(0)
    {
        runtime_shell.visible_script_delay_frames = None;
        let Some(next) = runtime_shell.field_notice_queue.pop_front() else {
            let error = anyhow::anyhow!(
                "Strength cry pause finished without the source MoveBoulderText"
            );
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
            runtime_shell.visible_strength_notice_phase = None;
            return;
        };
        runtime_shell.field_notice = Some(next);
        runtime_shell.visible_strength_notice_phase =
            Some(VisibleStrengthNoticePhase::MoveText);
        mark_runtime_snapshot_dirty(&mut runtime_shell);
        return;
    }
    if runtime_shell.visible_script_delay_frames == Some(0) {
        if let Err(error) = drain_visible_delays(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(format!("{error:#}"));
        }
        return;
    }
    let field_travel_text_complete = runtime_shell
        .field_notice
        .as_deref()
        .map_or(true, |notice| {
            visible_field_text_reveal_is_complete_for_text(&runtime_shell, notice)
        });
    let field_travel_delay_finished = if field_travel_text_complete
        && let Some(frames) = runtime_shell.pending_field_travel_delay_frames.as_mut()
    {
        *frames = frames.saturating_sub(1);
        *frames == 0
    } else {
        false
    };
    if field_travel_delay_finished {
        runtime_shell.pending_field_travel_delay_frames = None;
        runtime_shell.field_notice = None;
        runtime_shell.field_notice_queue.clear();
        runtime_shell.pending_sweet_scent_nothing_notice = false;
        runtime_shell.visible_strength_notice_phase = None;
        runtime_shell.pending_field_travel_arrival = false;
        if runtime_shell.visible_field_travel_animation
            == Some(VisibleFieldTravelAnimation::TeleportFrom)
        {
            if let Err(error) = queue_visible_shell_sound_effect(&mut runtime_shell, "SFX_WARP_TO")
                .and_then(|_| begin_visible_teleport_travel_animation(&mut runtime_shell, false))
            {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
            }
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            return;
        }
        runtime_shell.field_notice_scene = None;
        if let Err(error) = settle_visible_overworld_travel(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        mark_runtime_snapshot_dirty(&mut runtime_shell);
        return;
    }
    runtime_shell.poison_flash_frames_remaining =
        runtime_shell.poison_flash_frames_remaining.saturating_sub(1);
    if let Some(sign) = runtime_shell.visible_map_name_sign.as_mut() {
        sign.frames_remaining = sign.frames_remaining.saturating_sub(1);
        if sign.frames_remaining == 0 {
            runtime_shell.visible_map_name_sign = None;
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
    }
    if runtime_shell.pending_trainer_sight.is_some() {
        if let Err(error) = advance_visible_trainer_sight_cutscene(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
        return;
    }
    if runtime_shell.pending_field_notice_effect_frames.is_some()
        && runtime_shell.field_notice.is_none()
    {
        return;
    }
    if runtime_shell.visible_fly_animation.is_some()
        || (runtime_shell.visible_waterfall_animation.is_some()
            && runtime_shell.field_notice.is_none())
    {
        return;
    }
    if runtime_shell.pending_name_input.is_some() || runtime_shell.pending_name_choice.is_some() {
        return;
    }
    if runtime_shell.intro_screen.is_some() {
        return;
    }
    if runtime_shell.pending_time_set.is_some() {
        return;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return;
    }
    if runtime_shell.credits_screen.is_some() {
        tick_visible_credits_screen(&mut runtime_shell);
        return;
    }
    if runtime_shell.pending_delete_save.is_some() {
        return;
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return;
    }
    if runtime_shell.pending_mystery_gift.is_some() {
        return;
    }
    // The title owns the Options overlay, but the overlay still consumes
    // directional input. Only suppress overworld input while the bare title
    // menu is active.
    if runtime_shell.title_menu.is_some() && !runtime_shell.options_menu_open {
        return;
    }

    // Avoid an additional full snapshot merely to discover that there is no
    // text. The core shell has a cheap pending-work predicate which is true
    // for an open textbox and false for normal standing-still frames. The
    // authoritative input transaction still owns its atomic staging clone.
    if runtime_shell.shell.has_pending_script_work()
        || runtime_shell.field_text_reveal.is_some()
        || runtime_shell.field_notice.is_some()
        || runtime_shell.pc_notice.is_some()
    {
        match tick_visible_field_text_reveal(
            &mut runtime_shell,
            text_acceleration_requested,
        ) {
            Ok(true) => mark_runtime_presentation_dirty(&mut runtime_shell),
            Ok(false) => {}
            Err(error) => {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        }
        // `writetext` prints its complete text and then continues. Only the
        // source `waitbutton`/`promptbutton` commands require a player press.
        // Treating every writetext boundary as an acknowledgement trapped
        // radio broadcasts on Oak's first line and let repeated A presses
        // race subsequent pause/text commands.
        let auto_continue_writetext = runtime_shell
            .shell
            .snapshot()
            .ok()
            .filter(|snapshot| {
                snapshot.ui.text_window_open
                    && snapshot.ui.text.is_some()
                    && snapshot.ui.pending_yes_no.is_none()
                    && visible_field_dialogue_is_fully_revealed(&runtime_shell, snapshot)
            })
            .and_then(|_| runtime_shell.active_script_cursor.as_ref())
            .and_then(|cursor| {
                runtime_shell
                    .shell
                    .runtime()
                    .compiled_script_command_name(
                        &cursor.source_script,
                        cursor.next_command_index,
                    )
                    .ok()
            })
            .is_some_and(|command| !matches!(command.as_str(), "waitbutton" | "promptbutton"));
        if auto_continue_writetext {
            if let Err(error) = continue_visible_script_after_prompt(&mut runtime_shell) {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
            }
            return;
        }
        // UseFlashTextScript is text_asm rather than the ordinary
        // field-move text script: once the line has printed it immediately
        // plays SFX_FLASH and runs BlindingFlash, with no button wait.
        if runtime_shell.visible_flash_animation.is_some()
            && runtime_shell.field_notice.is_some()
        {
            match runtime_shell.shell.presentation_snapshot() {
                Ok(snapshot)
                    if visible_field_dialogue_is_fully_revealed(
                        &runtime_shell,
                        &snapshot,
                    ) =>
                {
                    runtime_shell.field_notice = None;
                    if let Err(error) = play_pending_field_notice_sound(&mut runtime_shell) {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(error.to_string());
                        return;
                    }
                    begin_pending_field_notice_effect(&mut runtime_shell);
                    mark_runtime_snapshot_dirty(&mut runtime_shell);
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                }
            }
        }
        let automatic_field_effect_ready = runtime_shell.field_notice.is_some()
            && runtime_shell.pending_field_notice_effect_frames.is_some()
            && runtime_shell.visible_flash_animation.is_none()
            && (runtime_shell.visible_cut_animation.is_some()
                || runtime_shell.visible_whirlpool_animation.is_some()
                || runtime_shell.visible_headbutt_animation.is_some());
        if automatic_field_effect_ready {
            match runtime_shell.shell.presentation_snapshot() {
                Ok(snapshot)
                    if visible_field_dialogue_is_fully_revealed(
                        &runtime_shell,
                        &snapshot,
                    ) =>
                {
                    runtime_shell.field_notice = None;
                    if let Err(error) = play_pending_field_notice_sound(&mut runtime_shell) {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(error.to_string());
                        return;
                    }
                    begin_pending_field_notice_effect(&mut runtime_shell);
                    mark_runtime_snapshot_dirty(&mut runtime_shell);
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                }
            }
        }
        if runtime_shell.visible_strength_notice_phase
            == Some(VisibleStrengthNoticePhase::UseText)
            && runtime_shell.field_notice.is_some()
        {
            match runtime_shell.shell.presentation_snapshot() {
                Ok(snapshot)
                    if visible_field_dialogue_is_fully_revealed(
                        &runtime_shell,
                        &snapshot,
                    ) =>
                {
                    runtime_shell.field_notice = None;
                    if let Err(error) = play_pending_field_notice_sound(&mut runtime_shell) {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(error.to_string());
                        return;
                    }
                    runtime_shell.visible_script_delay_frames = Some(3);
                    runtime_shell.visible_strength_notice_phase =
                        Some(VisibleStrengthNoticePhase::CryPause);
                    mark_runtime_snapshot_dirty(&mut runtime_shell);
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                }
            }
        }
        // A field textbox is a script boundary. The visual printer advances
        // on the host clock, but overworld frames must not: otherwise
        // autonomous object movement continues behind the text and scripted
        // characters (such as Mom) walk away before the player can respond.
        // `apply_runtime_hotkeys` still runs immediately after this system,
        // so A/Return can reveal and advance the current page normally.
        if runtime_shell.field_text_reveal.is_some() {
            // `writetext` itself is not always a joypad boundary. Radio
            // broadcasts use writetext -> pause -> writetext and must advance
            // automatically once the printer finishes; only waitbutton,
            // promptbutton and yesorno hand ownership to the player.
            let field_snapshot = match runtime_shell.shell.presentation_snapshot() {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(format!("{error:#}"));
                    return;
                }
            };
            let auto_continue = visible_field_dialogue_is_fully_revealed(
                &runtime_shell,
                &field_snapshot,
            ) && field_snapshot.ui.pending_text_wait.is_none()
                && field_snapshot.ui.pending_yes_no.is_none()
                && runtime_shell.active_script_cursor.is_some();
            if auto_continue {
                match advance_visible_completed_field_text_page(&mut runtime_shell, &field_snapshot) {
                    Ok(true) => return,
                    Ok(false) => {}
                    Err(error) => {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(format!("{error:#}"));
                        return;
                    }
                }
                if let Err(error) = advance_visible_script_until_player_boundary(&mut runtime_shell)
                {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(format!("{error:#}"));
                }
            }
            return;
        }
    } else if runtime_shell.field_text_reveal.take().is_some() {
        // The typewriter is presentation-only state. A script can close its
        // final textbox while resuming several commands in the same A press
        // (Mom's introductory script does exactly this), leaving no pending
        // authoritative text for the next frame to tick. Do not let that
        // stale cache retain exclusive joypad ownership forever.
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }

    if runtime_shell.pending_overworld_step_boundary.is_some() {
        if runtime_shell.player_walk_frame_ticks > 0
            || runtime_shell.visible_ledge_jump.is_some()
        {
            // A map connection is still the same uninterrupted overworld
            // walk. Sample the live D-pad while its retained source-map step
            // finishes so a corner pressed during the seam is not lost before
            // the ordinary input path becomes reachable again. Other arrival
            // boundaries deliberately retain their complete input lock.
            let crossing_connection = runtime_shell
                .shell
                .last_frame()
                .is_some_and(|frame| frame.connection.is_some());
            if crossing_connection && runtime_shell.player_walk_frame_ticks > 0 {
                sync_overworld_held_directions(&keys, &mut runtime_shell, false);
                runtime_shell.overworld_buffered_direction = [
                    (KeyCode::ArrowUp, GameButton::Up),
                    (KeyCode::ArrowDown, GameButton::Down),
                    (KeyCode::ArrowLeft, GameButton::Left),
                    (KeyCode::ArrowRight, GameButton::Right),
                ]
                .into_iter()
                .find_map(|(key, direction)| keys.just_pressed(key).then_some(direction))
                .or(runtime_shell.overworld_buffered_direction);
            }
            return;
        }
        let boundary = runtime_shell
            .pending_overworld_step_boundary
            .take()
            .expect("checked pending overworld step boundary");
        match boundary {
            PendingOverworldStepBoundary::Arrival => {
                runtime_shell.pending_overworld_warp_scene = None;
                run_bevy_action(&mut runtime_shell, settle_visible_overworld_frame_arrival);
            }
            PendingOverworldStepBoundary::CoordEvent => {
                run_bevy_action(&mut runtime_shell, execute_last_coord_event_script);
            }
            PendingOverworldStepBoundary::TrainerSight => {
                run_bevy_action(&mut runtime_shell, execute_last_trainer_sight_script);
            }
            PendingOverworldStepBoundary::WildBattle => {
                run_bevy_action(&mut runtime_shell, settle_visible_battle_after_action);
            }
            PendingOverworldStepBoundary::PoisonBlackout => {
                run_bevy_action(&mut runtime_shell, resolve_visible_blackout);
            }
            PendingOverworldStepBoundary::StepEvent(step_event) => {
                run_bevy_action(&mut runtime_shell, |shell| {
                    present_visible_step_event(shell, &step_event)
                });
            }
        }
        return;
    }

    // These surfaces run their own joypad loops in Crystal. They still pass
    // through VBlank/GameTimer above, but they must not manufacture an empty
    // overworld frame behind the held presentation.
    if runtime_shell.start_menu_cursor.is_some()
        || runtime_shell.party_menu_open
        || runtime_shell.pokedex_menu_open
        || runtime_shell.pokegear_menu_open
        || runtime_shell.trainer_card_open
        || runtime_shell.options_menu_open
        || runtime_shell.save_menu_open
        || runtime_shell.storage_cursor.is_some()
        || runtime_shell.pc_item_cursor.is_some()
        || visible_field_pack_is_open(&runtime_shell)
        || !matches!(
            runtime_shell.shell.session().state().battle,
            crystal_core::state::BattleMemory::Inactive
        )
    {
        return;
    }

    let shift_pressed = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt_pressed = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let plain_input = !shift_pressed && !alt_pressed && !ctrl_pressed;
    // Input ownership queries inspect the semantic snapshot and several
    // catalogs. Do not run all five on every idle LCD frame: only the
    // physically requested control needs routing.
    let shell_consumes_a = if plain_input && keys.pressed(KeyCode::KeyZ) {
        match has_visible_shell_a_action(&mut runtime_shell) {
            Ok(consumes) => consumes,
            Err(error) => {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        }
    } else {
        false
    };
    let shell_consumes_b = plain_input
        && keys.pressed(KeyCode::KeyX)
        && has_visible_shell_b_action(&mut runtime_shell);
    let shell_consumes_start = plain_input
        && keys.pressed(KeyCode::Enter)
        && has_visible_shell_start_action(&mut runtime_shell);
    let shell_consumes_select = !alt_pressed
        && !ctrl_pressed
        && keys.pressed(KeyCode::ShiftRight)
        && has_visible_shell_select_action(&mut runtime_shell);
    let direction_requested = plain_input
        && [
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
        ]
        .into_iter()
        .any(|key| keys.pressed(key));
    let shell_consumes_direction = direction_requested
        && has_visible_shell_direction_action(&mut runtime_shell);
    let visible_shell_pressed = (plain_input
        && ((keys.just_pressed(KeyCode::KeyZ) && shell_consumes_a)
            || (keys.just_pressed(KeyCode::KeyX) && shell_consumes_b)
            || (keys.just_pressed(KeyCode::Enter) && shell_consumes_start)
            || (shell_consumes_direction
                && (keys.just_pressed(KeyCode::ArrowUp)
                    || keys.just_pressed(KeyCode::ArrowDown)
                    || keys.just_pressed(KeyCode::ArrowLeft)
                    || keys.just_pressed(KeyCode::ArrowRight)))))
        || (!alt_pressed
            && !ctrl_pressed
            && keys.just_pressed(KeyCode::ShiftRight)
            && shell_consumes_select);
    sync_overworld_held_directions(
        &keys,
        &mut runtime_shell,
        shell_consumes_direction,
    );
    if visible_shell_pressed {
        return;
    }
    let mut buttons = collect_overworld_keyboard_buttons(
        &keys,
        shell_consumes_direction,
        shell_consumes_a,
        shell_consumes_b,
        shell_consumes_start,
        shell_consumes_select,
    );
    let newly_pressed_direction = [
        (KeyCode::ArrowUp, GameButton::Up),
        (KeyCode::ArrowDown, GameButton::Down),
        (KeyCode::ArrowLeft, GameButton::Left),
        (KeyCode::ArrowRight, GameButton::Right),
    ]
    .into_iter()
    .find_map(|(key, direction)| keys.just_pressed(key).then_some(direction));
    buttons.retain(|button| !is_direction_button(*button));
    if !shell_consumes_direction
        && let Some(direction) = newly_pressed_direction.or_else(|| {
            runtime_shell
                .overworld_held_directions
                .back()
                .copied()
        })
    {
        buttons.insert(0, direction);
    }
    let ledge_jump_in_flight = runtime_shell.visible_ledge_jump.is_some();
    if ledge_jump_in_flight {
        buttons.clear();
    } else if runtime_shell.player_walk_frame_ticks > 0 {
        // Crystal does not dispatch A/Start/Select interactions from the
        // destination tile until the visible tile step has landed. Direction
        // input remains live so held walking can chain without a dead frame.
        buttons.retain(|button| is_direction_button(*button));
    }
    let direction_held = buttons.iter().any(|button| is_direction_button(*button));
    if ledge_jump_in_flight {
        // STEP_LEDGE owns sixteen visible frames even though the ordinary
        // walk interpolation timer reaches zero halfway through it. Keep a
        // newly pressed corner queued for the landing instead of entering the
        // idle branch below, which would consume and discard it while input is
        // still locked. Empty frames may continue advancing independent NPCs
        // and timers during the jump.
        if let Some(direction) = newly_pressed_direction {
            runtime_shell.overworld_buffered_direction = Some(direction);
        }
        runtime_shell.overworld_direction_repeat_ticks = 0;
        buttons.clear();
    } else if runtime_shell.player_walk_frame_ticks > 0 {
        // Direction changes are buffered while the current tile is visibly
        // in flight. Core commits tiles atomically, so forwarding a newly
        // pressed direction here would begin a second step before the first
        // sprite interpolation landed.
        if let Some(direction) = newly_pressed_direction {
            runtime_shell.overworld_buffered_direction = Some(direction);
        }
        runtime_shell.overworld_direction_repeat_ticks = 0;
        buttons.retain(|button| !is_direction_button(*button));
        // Core movement is tile-atomic and also evaluates ice/current/downhill
        // auto-steps on an empty joypad frame. Hold those surfaces at this
        // boundary until the shell finishes drawing the current tile;
        // otherwise a forced chain can commit multiple tiles inside one
        // visible step. On an ordinary tile, however, empty authoritative
        // frames safely keep NPCs and world timers moving concurrently with
        // the player's interpolation, as they do in TypeScript/Crystal.
        if runtime_shell
            .shell
            .session()
            .overworld
            .forced_movement_direction()
            .is_some()
        {
            return;
        }
    } else {
        if let Some(direction) = runtime_shell.overworld_buffered_direction.take() {
            // A press edge captured during the preceding visible tile is a
            // complete queued joypad command. Execute it once at landing even
            // if the physical key was released meanwhile; requiring a live
            // hold here silently loses quick turns beside walls/counters.
            buttons.retain(|button| !is_direction_button(*button));
            buttons.insert(0, direction);
        }
        if let Some(direction) = buttons
            .iter()
            .copied()
            .find(|button| is_direction_button(*button))
            && (newly_pressed_direction.is_some()
                || runtime_shell.overworld_held_direction != Some(direction))
        {
            // Each directional animation owns its own four-step cycle in
            // TypeScript. A fresh press or turn starts on the ordinary action
            // frame; only an uninterrupted same-direction hold carries the
            // alternate-foot phase across the landing boundary.
            runtime_shell.player_walk_stride = false;
            runtime_shell.player_walk_mirror_stride = false;
        }
        let blocked_by_walking_object_origin = buttons
            .iter()
            .copied()
            .find(|button| is_direction_button(*button))
            .and_then(game_button_direction)
            .and_then(|direction| {
                crate::core::world::movement::checked_move_by_stride(
                    runtime_shell.shell.session().overworld.player.tile,
                    direction,
                    crate::core::world::movement::DEFAULT_RUNTIME_TILE_STRIDE,
                )
            })
            .is_some_and(|target| {
                runtime_shell
                    .object_walk_from
                    .values()
                    .any(|origin| *origin == target)
            });
        if blocked_by_walking_object_origin {
            // InitStep moves OBJECT_MAP_* to the destination while retaining
            // OBJECT_LAST_MAP_* as occupied until the step ends. Core's
            // tile-atomic NPC authority exposes only the destination, so keep
            // its retained visual origin collision-owned here as well.
            runtime_shell.overworld_direction_repeat_ticks = 0;
            buttons.retain(|button| !is_direction_button(*button));
            if !runtime_shell.transient_audio_playing
                && !runtime_shell
                    .pending_audio
                    .iter()
                    .any(|command| !matches!(command.kind, ModpackAudioKind::Music))
                && let Err(error) =
                    queue_visible_shell_sound_effect(&mut runtime_shell, "SFX_BUMP")
            {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        } else {
            throttle_held_overworld_direction(&mut runtime_shell, &mut buttons);
        }
    }
    let input_active = !buttons.is_empty() || direction_held;
    let elapsed_ticks = elapsed_input_ticks;
    // Reaching the authoritative input loop consumes this update's gameplay
    // budget here. Modal routes return above and hand the same-update budget
    // to apply_runtime_hotkeys instead.
    timer.take_presentation_ticks();

    let mut tick_ok = false;
    let mut execute_overworld_arrival = false;
    let mut execute_coord_event_script = false;
    let mut execute_trainer_sight_script = false;
    let mut execute_interaction_script = false;
    let mut execute_wild_battle_boundary = false;
    let mut execute_poison_blackout = false;
    let mut visible_step_event = None;
    let mut execute_contextual_field_move = false;
    let mut final_frame = None;
    let mut movement_scene_before_final_tick = None;
    let mut tick_error = None;
    let mut player_facing_changed = false;
    for _ in 0..elapsed_ticks {
        let movement_scene_before_tick = if buttons
            .iter()
            .copied()
            .any(is_direction_button)
            || runtime_shell
                .shell
                .session()
                .overworld
                .forced_movement_direction()
                .is_some()
        {
            match runtime_shell.shell.presentation_snapshot() {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    tick_error = Some(error);
                    break;
                }
            }
        } else {
            None
        };
        let object_tiles_before_tick = {
            let overworld = &runtime_shell.shell.session().overworld;
            let mut tiles = BTreeMap::new();
            for (index, object) in overworld
                .objects
                .iter()
                .enumerate()
                .filter(|(_, object)| overworld.is_object_visible(object))
            {
                let Some(object_id) = object.object_identifier.as_ref() else {
                    continue;
                };
                match overworld.object_runtime_tile_checked(index, object) {
                    Ok(tile) => {
                        tiles.insert(object_id.clone(), tile);
                    }
                    Err(error) => {
                        tick_error = Some(error.into());
                        break;
                    }
                }
            }
            tiles
        };
        if tick_error.is_some() {
            break;
        }
        let object_facings_before_tick = runtime_shell
            .shell
            .session()
            .overworld
            .object_facings
            .clone();
        let player_facing_before_tick = runtime_shell.shell.session().overworld.player.facing;
        let tick = if rtc_changed {
            rtc_changed = false;
            runtime_shell
                .shell
                .tick_with_rtc_after_vblank(buttons.clone(), rtc_sample)
        } else {
            runtime_shell.shell.tick_after_vblank(buttons.clone())
        };
        match tick.map(Clone::clone) {
            Ok(frame) => {
                player_facing_changed |= runtime_shell.shell.session().overworld.player.facing
                    != player_facing_before_tick;
                let step_event_boundary = frame.step_events.as_ref().is_some_and(|events| {
                    events.repel_expired.is_some()
                        || events.egg_hatched
                        || events.poison_result.is_some()
                });
                let reached_boundary = step_event_boundary
                    || frame.coord_event.is_some()
                    || frame.trainer_sight.is_some()
                    || frame.interaction.is_some()
                    || frame.warp.is_some()
                    || frame.connection.is_some()
                    || frame.wild_battle.is_some();
                let object_tiles_after_tick = runtime_shell
                    .shell
                    .session()
                    .overworld
                    .object_runtime_tiles
                    .clone();
                let mut newly_walking = object_tiles_before_tick
                    .into_iter()
                    .filter(|(object_id, from)| {
                        object_tiles_after_tick
                            .get(object_id)
                            .is_some_and(|to| to != from)
                    })
                    .collect::<BTreeMap<_, _>>();
                if matches!(frame.movement, Some(StepOutcome::Moved { .. }))
                    && let Some(follower_id) = runtime_shell
                        .shell
                        .session()
                        .overworld
                        .following
                        .as_ref()
                        .filter(|following| following.leader_object_id == "PLAYER")
                        .map(|following| following.follower_object_id.clone())
                    && follower_id != "PLAYER"
                    && let Some(from) = newly_walking.remove(&follower_id)
                {
                    // TypeScript queues this command at the leader's landing
                    // edge. Keep the authoritative destination, but retain
                    // the follower at its origin until that same LCD edge.
                    let to = object_tiles_after_tick[&follower_id];
                    let direction = if to.y > from.y {
                        Direction::Down
                    } else if to.y < from.y {
                        Direction::Up
                    } else if to.x < from.x {
                        Direction::Left
                    } else {
                        Direction::Right
                    };
                    runtime_shell.pending_follower_walks.push_back(VisibleFollowerWalk {
                        object_id: follower_id.clone(),
                        from,
                        to,
                        direction,
                    });
                    runtime_shell
                        .follower_visible_tile_overrides
                        .entry(follower_id)
                        .or_insert(from);
                }
                let newly_walking_ids = newly_walking.keys().cloned().collect::<BTreeSet<_>>();
                let pushed_boulder = runtime_shell
                    .shell
                    .session()
                    .overworld
                    .objects
                    .iter()
                    .find_map(|object| {
                        let object_id = object.object_identifier.as_ref()?;
                        (newly_walking_ids.contains(object_id)
                            && object.spritemovedata == "SPRITEMOVEDATA_STRENGTH_BOULDER")
                            .then(|| {
                                let from = newly_walking.get(object_id).copied();
                                let to = object_tiles_after_tick.get(object_id).copied();
                                let direction = match (from, to) {
                                    (Some(from), Some(to)) if to.y > from.y => Direction::Down,
                                    (Some(from), Some(to)) if to.y < from.y => Direction::Up,
                                    (Some(from), Some(to)) if to.x < from.x => Direction::Left,
                                    (Some(_), Some(_)) => Direction::Right,
                                    _ => frame.snapshot.facing,
                                };
                                (object_id.clone(), direction)
                            })
                    });
                for (object_id, from) in newly_walking {
                    runtime_shell.object_walk_from.insert(object_id.clone(), from);
                    runtime_shell
                        .object_walk_frame_ticks_by_id
                        .insert(object_id.clone(), WALK_FRAME_HOLD_TICKS);
                    runtime_shell
                        .object_walk_total_ticks_by_id
                        .insert(object_id, WALK_FRAME_HOLD_TICKS);
                }
                if let Some((object_id, direction)) = pushed_boulder {
                    runtime_shell.visible_strength_boulder_dust =
                        Some(VisibleStrengthBoulderDust {
                            object_id,
                            direction,
                            frames_remaining: 18,
                            age: 0,
                        });
                }
                let object_facings_after_tick = runtime_shell
                    .shell
                    .session()
                    .overworld
                    .object_facings
                    .clone();
                for (object_id, direction) in object_facings_after_tick {
                    if newly_walking_ids.contains(&object_id) {
                        let phase = if runtime_shell.object_walk_directions.get(&object_id)
                            == Some(&direction)
                        {
                            runtime_shell
                                .object_walk_phases
                                .get(&object_id)
                                .copied()
                                .unwrap_or(0)
                                .wrapping_add(1)
                                % 4
                        } else {
                            1
                        };
                        runtime_shell.object_walk_phases.insert(object_id.clone(), phase);
                        runtime_shell.object_walk_directions.insert(object_id, direction);
                    } else if object_facings_before_tick.get(&object_id) != Some(&direction) {
                        runtime_shell.object_walk_phases.insert(object_id.clone(), 0);
                        runtime_shell.object_walk_directions.insert(object_id, direction);
                    }
                }
                movement_scene_before_final_tick = movement_scene_before_tick;
                final_frame = Some(frame);
                if reached_boundary
                    || final_frame
                        .as_ref()
                        .is_some_and(|frame| {
                            frame.movement.is_some() || frame.autonomous_objects_changed
                        })
                {
                    // A queued corner belongs only to this continuous walk.
                    // Never carry it through a warp, encounter, trainer, or
                    // script boundary into a newly controlled scene.
                    if reached_boundary {
                        runtime_shell.overworld_buffered_direction = None;
                    }
                    // Core movement is tile-atomic, while the LCD owns the
                    // following player/object turn, walk, or bump cadence.
                    // Never consume a second accumulated host tick before
                    // that result has installed its visible presentation
                    // boundary.
                    break;
                }
            }
            Err(error) => {
                tick_error = Some(error);
                break;
            }
        }
    }
    let tick_result: Result<crate::RuntimeOverworldFrame> = match (final_frame, tick_error) {
        (Some(frame), None) => Ok(frame),
        (None, Some(error)) => Err(error),
        (None, None) => Err(anyhow::anyhow!(
            "runtime tick timer reported no elapsed frame"
        )),
        (Some(_), Some(error)) => Err(error),
    };
    match tick_result {
        Ok(frame) => {
            tick_ok = true;
            let player_moved = matches!(frame.movement, Some(StepOutcome::Moved { .. }));
            // TypeScript main and Crystal hold a facing change for four LCD
            // frames before the same held direction begins its tile step.
            // A wall/object bump remains immediately retryable; assigning the
            // turn cadence to every non-move made collision feel sticky.
            if direction_held {
                match frame.movement.as_ref() {
                    Some(StepOutcome::Turned { .. }) => {
                        runtime_shell.overworld_direction_repeat_ticks =
                            OVERWORLD_TURN_HOLD_TICKS.saturating_sub(1);
                    }
                    Some(StepOutcome::Blocked { .. })
                    | Some(StepOutcome::BlockedByObject { .. })
                    | Some(StepOutcome::RuntimeTileOverflow { .. }) => {
                        runtime_shell.overworld_direction_repeat_ticks = 0;
                    }
                    _ => {}
                }
            }
            if matches!(
                frame.movement,
                Some(
                    StepOutcome::Blocked { .. }
                        | StepOutcome::BlockedByObject { .. }
                        | StepOutcome::RuntimeTileOverflow { .. }
                )
            ) && matches!(
                frame.snapshot.mode,
                MovementMode::Normal | MovementMode::Bike | MovementMode::Skate
            ) && !runtime_shell.transient_audio_playing
                && !runtime_shell.pending_audio.iter().any(|command| {
                    !matches!(command.kind, ModpackAudioKind::Music)
                })
                && let Err(error) =
                    queue_visible_shell_sound_effect(&mut runtime_shell, "SFX_BUMP")
            {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
            // A no-input frame still advances the authoritative Game Boy
            // frame counter, but it does not require rebuilding Bevy's
            // semantic snapshot or viewport. Invalidate only when the frame
            // produced a visible/gameplay mutation; script handlers below
            // invalidate their own mutations through `run_bevy_action`.
            if frame.movement.is_some()
                || player_facing_changed
                || direction_held
                || frame.autonomous_objects_changed
                || frame.step_events.is_some()
                || frame.coord_event.is_some()
                || frame.trainer_sight.is_some()
                || frame.interaction.is_some()
                || frame.warp.is_some()
                || frame.connection.is_some()
                || frame.wild_encounter.is_some()
                || frame.wild_battle.is_some()
            {
                mark_runtime_snapshot_dirty(&mut runtime_shell);
            }
            if frame.autonomous_objects_changed {
                runtime_shell.object_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
                runtime_shell.object_walk_frame_ticks = WALK_FRAME_HOLD_TICKS;
            }
            if player_moved {
                let (mut from, speed_multiplier) = match frame.movement.as_ref() {
                    Some(StepOutcome::Moved { from, speed_multiplier, .. }) => {
                        (*from, (*speed_multiplier).max(1))
                    }
                    _ => unreachable!("player_moved requires a moved outcome"),
                };
                if let Some(connection) = frame.connection.as_ref() {
                    let opposite = match frame.snapshot.facing {
                        Direction::Up => Direction::Down,
                        Direction::Down => Direction::Up,
                        Direction::Left => Direction::Right,
                        Direction::Right => Direction::Left,
                    };
                    if let Some(connection_from) =
                        crate::core::world::movement::checked_move_by_stride(
                            connection.destination.tile,
                            opposite,
                            crate::core::world::movement::DEFAULT_RUNTIME_TILE_STRIDE,
                        )
                    {
                        // Connection resolution has already translated the
                        // authority into target-map coordinates. Interpolate
                        // from the adjacent target-map edge, never from the
                        // unrelated numeric coordinate in the source map.
                        from = connection_from;
                    }
                }
                let step_ticks = (WALK_FRAME_HOLD_TICKS / speed_multiplier).max(1);
                runtime_shell.player_walk_from = Some(from);
                let previous_stride = runtime_shell.player_walk_stride;
                runtime_shell.player_walk_stride = next_player_walk_stride(
                    runtime_shell.player_walk_frame_ticks,
                    runtime_shell.player_walk_stride,
                );
                if previous_stride && !runtime_shell.player_walk_stride {
                    runtime_shell.player_walk_mirror_stride =
                        !runtime_shell.player_walk_mirror_stride;
                }
                runtime_shell.player_walk_total_ticks = step_ticks;
                runtime_shell.player_walk_frame_ticks = step_ticks;
                runtime_shell.overworld_direction_repeat_ticks = step_ticks.saturating_sub(1);
            }
            if let Some(LedgeJumpOutcome::Jumped { from, to, .. }) = frame.ledge_jump {
                runtime_shell.visible_ledge_jump = Some(VisibleLedgeJump {
                    from,
                    to,
                    frame: 0,
                });
                let BevyRuntimeShell {
                    shell,
                    pending_audio,
                    last_audio_events,
                    ..
                } = &mut *runtime_shell;
                if let Err(error) = queue_visible_sound_effect(
                    shell.runtime().audio(),
                    pending_audio,
                    last_audio_events,
                    "SFX_JUMP_OVER_LEDGE",
                ) {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    return;
                }
            }
            if let Some(rustle) = frame.grass_rustle {
                runtime_shell.visible_grass_rustle = Some(VisibleGrassRustle {
                    tile: rustle.tile,
                    frames_remaining: rustle.duration_frames,
                    age: 0,
                });
            }
            execute_overworld_arrival = frame.warp.is_some() || frame.connection.is_some();
            execute_coord_event_script = frame.coord_event.is_some();
            execute_trainer_sight_script = frame.trainer_sight.is_some();
            let interaction_targets_walking_object = frame
                .interaction
                .as_ref()
                .and_then(|interaction| match &interaction.target {
                    crate::core::world::session::OverworldInteractionTarget::Object {
                        object_identifier: Some(object_id),
                        ..
                    } => Some(object_id),
                    _ => None,
                })
                .is_some_and(|object_id| {
                    runtime_shell.object_walk_from.contains_key(object_id)
                });
            execute_interaction_script = keys.just_pressed(KeyCode::KeyZ)
                && runtime_shell.player_walk_frame_ticks == 0
                && !shell_consumes_a
                && !interaction_targets_walking_object
                && frame.interaction.is_some();
            execute_contextual_field_move = keys.just_pressed(KeyCode::KeyZ)
                && runtime_shell.player_walk_frame_ticks == 0
                && !shell_consumes_a
                && frame.interaction.is_none()
                && frame.wild_battle.is_none();
            execute_wild_battle_boundary = frame.wild_battle.is_some();
            visible_step_event = frame.step_events.clone().filter(|events| {
                events.repel_expired.is_some()
                    || events.egg_hatched
                    || events.poison_result.is_some()
            });
            execute_poison_blackout = frame
                .step_events
                .as_ref()
                .and_then(|events| events.poison_result.as_ref())
                .is_some_and(|poison| !poison.fainted_names.is_empty())
                && !runtime_shell
                    .shell
                    .session()
                    .state()
                    .storage
                    .party
                    .pokemon
                    .iter()
                    .flatten()
                    .any(|pokemon| {
                        !pokemon.is_egg
                            && pokemon.species.id != "EGG"
                            && pokemon.hp > 0
                    });
            let movement_is_visibly_in_flight = player_moved
                || matches!(frame.ledge_jump, Some(LedgeJumpOutcome::Jumped { .. }));
            if movement_is_visibly_in_flight {
                if frame.warp.is_some()
                    && let (
                        Some(mut source_scene),
                        Some(StepOutcome::Moved { from, to, .. }),
                    ) = (movement_scene_before_final_tick, frame.movement.as_ref())
                {
                    // Core has already installed the destination session. Keep
                    // the source map and place its presentation target on the
                    // triggering warp tile until the walk visibly lands.
                    source_scene.overworld.tile = *to;
                    source_scene.overworld.facing = if to.x > from.x {
                        Direction::Right
                    } else if to.x < from.x {
                        Direction::Left
                    } else if to.y > from.y {
                        Direction::Down
                    } else {
                        Direction::Up
                    };
                    runtime_shell.pending_overworld_warp_scene =
                        Some(Arc::new(source_scene));
                }
                runtime_shell.pending_overworld_step_boundary = if execute_overworld_arrival {
                    Some(PendingOverworldStepBoundary::Arrival)
                } else if execute_coord_event_script {
                    Some(PendingOverworldStepBoundary::CoordEvent)
                } else if execute_trainer_sight_script {
                    Some(PendingOverworldStepBoundary::TrainerSight)
                } else if execute_wild_battle_boundary {
                    Some(PendingOverworldStepBoundary::WildBattle)
                } else if execute_poison_blackout {
                    Some(PendingOverworldStepBoundary::PoisonBlackout)
                } else {
                    visible_step_event
                        .clone()
                        .map(PendingOverworldStepBoundary::StepEvent)
                };
                if runtime_shell.pending_overworld_step_boundary.is_some() {
                    // A buffered direction belongs only to uninterrupted
                    // overworld walking. Coord scripts, warps, trainer sight,
                    // battles, and step events take ownership at this tile;
                    // replaying the edge after their relocation/cutscene can
                    // move the player out from under the authored boundary.
                    runtime_shell.overworld_buffered_direction = None;
                    if matches!(
                        runtime_shell.pending_overworld_step_boundary,
                        Some(PendingOverworldStepBoundary::WildBattle)
                    ) {
                        // Stage the battle data/messages now so the committed
                        // battle snapshot cannot leak directly onto the map.
                        // Its transition remains frozen at frame zero until
                        // the visible step boundary is dispatched.
                        if let Err(error) =
                            prepare_visible_battle_entry_after_visible_step(&mut runtime_shell)
                        {
                            record_visible_runtime_error(&mut runtime_shell, &error);
                            runtime_shell.last_error = Some(error.to_string());
                            return;
                        }
                    }
                    execute_overworld_arrival = false;
                    execute_coord_event_script = false;
                    execute_trainer_sight_script = false;
                    execute_wild_battle_boundary = false;
                    execute_poison_blackout = false;
                    visible_step_event = None;
                }
            }
            let input_frame =
                match deterministic_input_frame_from_post_tick_checksum(&frame.state_checksum) {
                    Ok(input_frame) => input_frame,
                    Err(error) => {
                        record_visible_runtime_error(&mut runtime_shell, &error);
                        runtime_shell.last_error = Some(error.to_string());
                        return;
                    }
                };
            let input_record = VisibleOverworldInputRecord {
                frame: input_frame,
                input_mask: frame.input_mask,
                pressed_mask: frame.pressed_mask,
                player_moved,
                state_checksum: frame.state_checksum.clone(),
            };
            let frame_activity = if input_active {
                summarize_frame_activity(&frame)
            } else {
                None
            };
            if input_active {
                set_visible_runtime_action_from_checksum(
                    &mut runtime_shell,
                    format!(
                        "input:overworld:frame:{}:mask:{:#010b}:pressed:{:#010b}",
                        input_record.frame, input_record.input_mask, input_record.pressed_mask
                    ),
                    &input_record.state_checksum,
                );
            }
            if let Err(error) = record_visible_overworld_input(&mut runtime_shell, input_record) {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
            runtime_shell.last_error = None;
            if let Some(activity) = frame_activity {
                runtime_shell.last_audio_events.push(activity);
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
        }
        Err(error) => {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
    }
    sync_visible_battle_action_cursor(&mut runtime_shell);
    if !tick_ok {
        return;
    }
    if execute_overworld_arrival {
        run_bevy_action(&mut runtime_shell, settle_visible_overworld_frame_arrival);
    } else if execute_coord_event_script {
        run_bevy_action(&mut runtime_shell, execute_last_coord_event_script);
    } else if execute_trainer_sight_script {
        run_bevy_action(&mut runtime_shell, execute_last_trainer_sight_script);
    } else if execute_interaction_script {
        runtime_shell.overworld_interaction_consumed_a = true;
        run_bevy_action(&mut runtime_shell, execute_last_interaction_script);
    } else if execute_wild_battle_boundary {
        if let Err(error) = prepare_visible_battle_entry(&mut runtime_shell) {
            record_visible_runtime_error(&mut runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
            return;
        }
        run_bevy_action(&mut runtime_shell, settle_visible_battle_after_action);
    } else if execute_poison_blackout {
        run_bevy_action(&mut runtime_shell, resolve_visible_blackout);
    } else if let Some(step_event) = visible_step_event {
        run_bevy_action(&mut runtime_shell, |shell| {
            present_visible_step_event(shell, &step_event)
        });
    } else if execute_contextual_field_move {
        match execute_visible_contextual_field_move(&mut runtime_shell) {
            Ok(true) => {}
            Ok(false) => match advance_visible_script_until_player_boundary(&mut runtime_shell) {
                Ok(()) => {}
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                }
            },
            Err(error) => {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
            }
        }
    } else if runtime_shell.pending_overworld_step_boundary.is_none()
        && (runtime_shell.active_script_cursor.is_some()
            || runtime_shell.shell.has_pending_script_work())
    {
        // Do not take an additional snapshot/scan solely to discover pending
        // script work. The cheap predicate enters this path only when a
        // script can actually make progress; input transaction staging is
        // intentionally accounted for separately.
        match advance_visible_script_until_player_boundary(&mut runtime_shell) {
            Ok(()) => {}
            Err(error) => {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
            }
        }
    }
}

fn advance_visible_trainer_sight_cutscene(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .visible_overworld_emote
        .as_ref()
        .is_some_and(|emote| emote.frames_remaining > 0)
    {
        return Ok(());
    }
    runtime_shell.visible_overworld_emote = None;

    let Some(pending) = runtime_shell.pending_trainer_sight.as_mut() else {
        return Ok(());
    };
    if pending.frames_until_step > 0 {
        pending.frames_until_step -= 1;
        return Ok(());
    }
    if pending.steps_remaining == 0 {
        finish_visible_trainer_sight_script(runtime_shell)?;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }

    let object_id = pending.object_id.clone();
    let direction = pending.direction;
    let current = runtime_shell
        .shell
        .session()
        .overworld
        .object_runtime_tile_by_id(&object_id)?;
    let stride = crate::core::world::movement::DEFAULT_RUNTIME_TILE_STRIDE;
    let next = match direction {
        Direction::Up => TilePosition {
            x: current.x,
            y: current.y.checked_sub(stride).context("trainer approach moved above runtime bounds")?,
        },
        Direction::Down => TilePosition {
            x: current.x,
            y: current.y.checked_add(stride).context("trainer approach moved below runtime bounds")?,
        },
        Direction::Left => TilePosition {
            x: current.x.checked_sub(stride).context("trainer approach moved left of runtime bounds")?,
            y: current.y,
        },
        Direction::Right => TilePosition {
            x: current.x.checked_add(stride).context("trainer approach moved right of runtime bounds")?,
            y: current.y,
        },
    };
    {
        let overworld = &mut runtime_shell.shell.session_mut().overworld;
        overworld.set_object_runtime_facing(&object_id, direction)?;
        overworld.set_object_runtime_tile(&object_id, next)?;
    }
    let pending = runtime_shell
        .pending_trainer_sight
        .as_mut()
        .context("trainer approach state disappeared while applying a step")?;
    pending.steps_remaining -= 1;
    // `slow_step` is twice the ordinary eight-frame walking cadence.
    pending.frames_until_step = WALK_FRAME_HOLD_TICKS.saturating_mul(2);
    advance_object_walk_phase(runtime_shell, &object_id, direction);
    runtime_shell.trainer_walk_from = Some((object_id, current));
    runtime_shell.object_walk_stride = !runtime_shell.object_walk_stride;
    runtime_shell.object_walk_total_ticks = WALK_FRAME_HOLD_TICKS.saturating_mul(2);
    runtime_shell.object_walk_frame_ticks = WALK_FRAME_HOLD_TICKS.saturating_mul(2);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn next_player_walk_stride(_remaining_ticks: u8, current_stride: bool) -> bool {
    // This bit selects the stepping foot, not standing versus walking. The
    // renderer keeps the action frame for every in-flight tile and mirrors
    // vertical art on alternate steps. Showing standing art for one entire
    // moving tile is visible skating rather than a walk cycle.
    !current_stride
}

fn start_next_queued_follower_walk(runtime_shell: &mut BevyRuntimeShell) {
    let Some(next) = runtime_shell.pending_follower_walks.front() else {
        return;
    };
    if runtime_shell
        .object_walk_frame_ticks_by_id
        .contains_key(&next.object_id)
    {
        return;
    }
    let next = runtime_shell
        .pending_follower_walks
        .pop_front()
        .expect("front follower walk exists");
    advance_object_walk_phase(runtime_shell, &next.object_id, next.direction);
    runtime_shell
        .follower_visible_tile_overrides
        .insert(next.object_id.clone(), next.to);
    runtime_shell
        .object_walk_from
        .insert(next.object_id.clone(), next.from);
    runtime_shell
        .object_walk_total_ticks_by_id
        .insert(next.object_id.clone(), WALK_FRAME_HOLD_TICKS);
    runtime_shell
        .object_walk_frame_ticks_by_id
        .insert(next.object_id, WALK_FRAME_HOLD_TICKS);
}

fn advance_object_walk_phase(
    runtime_shell: &mut BevyRuntimeShell,
    object_id: &str,
    direction: Direction,
) {
    let phase = if runtime_shell.object_walk_directions.get(object_id) == Some(&direction) {
        runtime_shell
            .object_walk_phases
            .get(object_id)
            .copied()
            .unwrap_or(0)
            .wrapping_add(1)
            % 4
    } else {
        1
    };
    runtime_shell.object_walk_phases.insert(object_id.to_string(), phase);
    runtime_shell
        .object_walk_directions
        .insert(object_id.to_string(), direction);
}

fn start_next_visible_script_movement_phase(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<bool> {
    loop {
        let next = runtime_shell.visible_script_movement.as_mut().and_then(|movement| {
            movement
                .phases
                .pop_front()
                .map(|phase| (movement.object_id.clone(), phase))
        });
        let Some((object_id, phase)) = next else {
            let pending = runtime_shell
                .visible_script_movement
                .as_mut()
                .and_then(|movement| movement.pending_programs.pop_front());
            if let Some(program) = pending {
                let revealed_object = if !program.previous_hidden && program.object_id != "PLAYER" {
                    runtime_shell
                        .shell
                        .session()
                        .overworld
                        .objects
                        .iter()
                        .find(|object| {
                            object.object_identifier.as_deref() == Some(program.object_id.as_str())
                        })
                        .cloned()
                } else {
                    None
                };
                let scene = Arc::make_mut(
                    runtime_shell
                        .visible_script_movement_scene
                        .as_mut()
                        .context("queued visible script movement has no retained scene")?,
                );
                if program.object_id == "PLAYER" {
                    scene.overworld.tile = program.previous_tile;
                    scene.overworld.facing = program.previous_facing;
                    scene.overworld_player_hidden = program.previous_hidden;
                } else {
                    scene
                        .visible_object_runtime_tiles
                        .insert(program.object_id.clone(), program.previous_tile);
                    scene
                        .visible_object_facings
                        .insert(program.object_id.clone(), program.previous_facing);
                    if program.previous_hidden {
                        scene.visible_objects.retain(|object| {
                            object.object_identifier.as_deref() != Some(program.object_id.as_str())
                        });
                    } else if !scene.visible_objects.iter().any(|object| {
                        object.object_identifier.as_deref() == Some(program.object_id.as_str())
                    }) {
                        scene.visible_objects.push(revealed_object.with_context(|| {
                            format!(
                                "queued movement cannot restore unknown object {}",
                                program.object_id
                            )
                        })?);
                    }
                }
                let movement = runtime_shell
                    .visible_script_movement
                    .as_mut()
                    .context("visible script movement disappeared while switching programs")?;
                movement.object_id = program.object_id;
                movement.phases = program.phases;
                movement.hold_frames_remaining = 0;
                movement.active_jump_duration = None;
                movement.active_uses_standing_frame = false;
                movement.active_tree_shake_duration = None;
                movement.active_stationary_effect = None;
                movement.active_stationary_duration = 0;
                movement.stationary_y_offset = 0;
                movement.stationary_initial_facing = program.previous_facing;
                movement.follower_object_id = program.follower_object_id;
                movement.follower_queued_step = program.follower_queued_step;
                continue;
            }
            runtime_shell.visible_script_movement = None;
            runtime_shell.visible_script_movement_scene = None;
            runtime_shell.player_walk_from = None;
            runtime_shell.player_walk_frame_ticks = 0;
            runtime_shell.player_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
            runtime_shell.trainer_walk_from = None;
            runtime_shell.object_walk_frame_ticks = 0;
            runtime_shell.object_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(false);
        };
        match phase {
            VisibleScriptMovementPhase::Sound { audio_id } => {
                queue_visible_shell_sound_effect(runtime_shell, &audio_id)?;
                continue;
            }
            VisibleScriptMovementPhase::Hold { duration } => {
                if duration == 0 {
                    continue;
                }
                let movement = runtime_shell
                    .visible_script_movement
                    .as_mut()
                    .context("visible script movement disappeared while starting hold")?;
                movement.hold_frames_remaining = duration;
                movement.active_jump_duration = None;
                movement.active_uses_standing_frame = true;
                movement.active_tree_shake_duration = None;
                movement.active_stationary_effect = None;
                movement.active_stationary_duration = 0;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(true);
            }
            VisibleScriptMovementPhase::TreeShake { duration } => {
                if duration == 0 {
                    continue;
                }
                let movement = runtime_shell
                    .visible_script_movement
                    .as_mut()
                    .context("visible script movement disappeared while starting tree shake")?;
                movement.hold_frames_remaining = duration;
                movement.active_jump_duration = None;
                movement.active_uses_standing_frame = false;
                movement.active_tree_shake_duration = Some(duration);
                movement.active_stationary_effect = None;
                movement.active_stationary_duration = 0;
                movement.stationary_y_offset = 0;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(true);
            }
            VisibleScriptMovementPhase::Visibility { hidden } => {
                let revealed_object = if !hidden && object_id != "PLAYER" {
                    runtime_shell
                        .shell
                        .session()
                        .overworld
                        .objects
                        .iter()
                        .find(|object| {
                            object.object_identifier.as_deref() == Some(object_id.as_str())
                        })
                        .cloned()
                } else {
                    None
                };
                let scene = Arc::make_mut(
                    runtime_shell
                        .visible_script_movement_scene
                        .as_mut()
                        .context("visible script visibility change has no retained scene")?,
                );
                if object_id == "PLAYER" {
                    scene.overworld_player_hidden = hidden;
                } else if hidden {
                    scene.visible_objects.retain(|object| {
                        object.object_identifier.as_deref() != Some(object_id.as_str())
                    });
                } else if !scene.visible_objects.iter().any(|object| {
                    object.object_identifier.as_deref() == Some(object_id.as_str())
                }) {
                    scene.visible_objects.push(revealed_object.with_context(|| {
                        format!("visible script cannot reveal unknown object {object_id}")
                    })?);
                }
                mark_runtime_snapshot_dirty(runtime_shell);
                continue;
            }
            VisibleScriptMovementPhase::Stationary { duration, effect } => {
                if duration == 0 {
                    continue;
                }
                let initial_facing = runtime_shell
                    .visible_script_movement_scene
                    .as_ref()
                    .and_then(|scene| {
                        if object_id == "PLAYER" {
                            Some(scene.overworld.facing)
                        } else {
                            scene.visible_object_facings.get(&object_id).copied()
                        }
                    })
                    .context("stationary movement actor has no retained facing")?;
                let movement = runtime_shell
                    .visible_script_movement
                    .as_mut()
                    .context("visible script movement disappeared while starting stationary effect")?;
                movement.hold_frames_remaining = duration;
                movement.active_jump_duration = None;
                movement.active_uses_standing_frame = !matches!(
                    effect,
                    VisibleStationaryMovementEffect::SkyfallFall
                        | VisibleStationaryMovementEffect::RockSmash
                );
                movement.active_tree_shake_duration = None;
                movement.active_stationary_effect = Some(effect);
                movement.active_stationary_duration = duration;
                movement.stationary_initial_facing = initial_facing;
                update_visible_stationary_movement_frame(runtime_shell)?;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(true);
            }
            VisibleScriptMovementPhase::ScreenShake { parameter } => {
                runtime_shell.visible_earthquake = Some(VisibleEarthquake {
                    intensity: 1_u16 << ((parameter >> 6) & 0x3),
                    frames_remaining: (parameter & 0x3f).max(1),
                    phase: 0,
                });
                mark_runtime_snapshot_dirty(runtime_shell);
                continue;
            }
            VisibleScriptMovementPhase::Turn { direction, duration } => {
                let scene = Arc::make_mut(
                    runtime_shell
                        .visible_script_movement_scene
                        .as_mut()
                        .context("visible script movement turn has no retained scene")?,
                );
                if object_id == "PLAYER" {
                    scene.overworld.facing = direction;
                } else {
                    scene.visible_object_facings.insert(object_id, direction);
                }
                let movement = runtime_shell
                    .visible_script_movement
                    .as_mut()
                    .context("visible script movement disappeared while starting turn")?;
                movement.hold_frames_remaining = u16::from(duration.max(1));
                movement.active_jump_duration = None;
                movement.active_uses_standing_frame = true;
                movement.active_tree_shake_duration = None;
                movement.active_stationary_effect = None;
                movement.active_stationary_duration = 0;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(true);
            }
            VisibleScriptMovementPhase::Move {
                from,
                to,
                direction,
                duration,
                jump,
                update_facing,
                standing_frame,
            } => {
                let leader_stride = u8::try_from(
                    (i32::from(to.x) - i32::from(from.x))
                        .unsigned_abs()
                        .max((i32::from(to.y) - i32::from(from.y)).unsigned_abs()),
                )
                .context("visible leader stride exceeds follower movement range")?;
                begin_visible_follower_step(
                    runtime_shell,
                    direction,
                    leader_stride,
                    duration,
                    jump,
                    standing_frame,
                )?;
                let scene = Arc::make_mut(
                    runtime_shell
                        .visible_script_movement_scene
                        .as_mut()
                        .context("visible script movement step has no retained scene")?,
                );
                if object_id == "PLAYER" {
                    scene.overworld.tile = to;
                    if update_facing {
                        scene.overworld.facing = direction;
                    }
                    runtime_shell.player_walk_from = Some(from);
                    runtime_shell.player_walk_total_ticks = duration;
                    runtime_shell.player_walk_frame_ticks = duration;
                    let previous_stride = runtime_shell.player_walk_stride;
                    runtime_shell.player_walk_stride = !runtime_shell.player_walk_stride;
                    if previous_stride && !runtime_shell.player_walk_stride {
                        runtime_shell.player_walk_mirror_stride =
                            !runtime_shell.player_walk_mirror_stride;
                    }
                } else {
                    scene.visible_object_runtime_tiles.insert(object_id.clone(), to);
                    if update_facing {
                        scene.visible_object_facings.insert(object_id.clone(), direction);
                    }
                    advance_object_walk_phase(runtime_shell, &object_id, direction);
                    runtime_shell.trainer_walk_from = Some((object_id, from));
                    runtime_shell.object_walk_total_ticks = duration;
                    runtime_shell.object_walk_frame_ticks = duration;
                    runtime_shell.object_walk_stride = !runtime_shell.object_walk_stride;
                }
                let movement = runtime_shell
                    .visible_script_movement
                    .as_mut()
                    .context("visible script movement disappeared while starting step")?;
                movement.hold_frames_remaining = 0;
                movement.active_jump_duration = jump.then_some(duration);
                movement.active_uses_standing_frame = standing_frame;
                movement.active_tree_shake_duration = None;
                movement.active_stationary_effect = None;
                movement.active_stationary_duration = 0;
                movement.stationary_y_offset = 0;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(true);
            }
        }
    }
}

fn advance_visible_script_movement(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    let Some(movement) = runtime_shell.visible_script_movement.as_mut() else {
        return Ok(false);
    };
    let hold_frames_remaining = if movement.hold_frames_remaining > 0 {
        movement.hold_frames_remaining -= 1;
        Some(movement.hold_frames_remaining)
    } else {
        None
    };
    if let Some(hold_frames_remaining) = hold_frames_remaining {
        update_visible_stationary_movement_frame(runtime_shell)?;
        if hold_frames_remaining > 0 {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(true);
        }
    } else {
        let movement = runtime_shell
            .visible_script_movement
            .as_ref()
            .context("visible script movement disappeared while advancing step")?;
        let movement_in_flight = if movement.object_id == "PLAYER" {
            runtime_shell.player_walk_frame_ticks > 0
        } else {
            runtime_shell.object_walk_frame_ticks > 0
        };
        if movement_in_flight {
            return Ok(true);
        }
    }
    if start_next_visible_script_movement_phase(runtime_shell)? {
        return Ok(true);
    }
    match runtime_shell.visible_field_travel_animation {
        Some(VisibleFieldTravelAnimation::DigOut) => {
            settle_visible_overworld_travel(runtime_shell)?;
            queue_visible_shell_sound_effect(runtime_shell, "SFX_WARP_FROM")?;
            runtime_shell.visible_field_travel_animation =
                Some(VisibleFieldTravelAnimation::DigReturn);
            begin_visible_dig_travel_animation(runtime_shell, true)?;
        }
        Some(VisibleFieldTravelAnimation::DigReturn) => {
            runtime_shell.visible_field_travel_animation = None;
            runtime_shell.field_notice_scene = None;
        }
        Some(VisibleFieldTravelAnimation::TeleportFrom) => {
            settle_visible_overworld_travel(runtime_shell)?;
            queue_visible_shell_sound_effect(runtime_shell, "SFX_WARP_FROM")?;
            runtime_shell.visible_field_travel_animation =
                Some(VisibleFieldTravelAnimation::TeleportTo);
            begin_visible_teleport_travel_animation(runtime_shell, true)?;
        }
        Some(VisibleFieldTravelAnimation::TeleportTo) => {
            runtime_shell.visible_field_travel_animation = None;
            runtime_shell.field_notice_scene = None;
        }
        Some(VisibleFieldTravelAnimation::Pitfall) => {
            runtime_shell.visible_field_travel_animation = None;
            settle_visible_overworld_arrival(runtime_shell, "pitfall")?;
        }
        None => advance_visible_script_until_player_boundary(runtime_shell)?,
    }
    Ok(true)
}

fn update_visible_stationary_movement_frame(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let Some(movement) = runtime_shell.visible_script_movement.as_ref() else {
        return Ok(());
    };
    let Some(effect) = movement.active_stationary_effect else {
        return Ok(());
    };
    let elapsed = movement
        .active_stationary_duration
        .saturating_sub(movement.hold_frames_remaining.min(movement.active_stationary_duration));
    let stationary_y_offset = match effect {
        VisibleStationaryMovementEffect::TeleportRise => {
            const OFFSETS: [i16; 17] = [
                0, -1, -2, -5, -8, -12, -17, -22, -29, -36, -43, -51, -60, -69, -78, -87,
                -96,
            ];
            OFFSETS[usize::from(elapsed.min(16))]
        }
        VisibleStationaryMovementEffect::TeleportWait => -96,
        VisibleStationaryMovementEffect::TeleportDescent => {
            const OFFSETS: [i16; 17] = [
                -96, -87, -78, -69, -60, -51, -43, -36, -29, -22, -17, -12, -8, -5, -2, -1,
                0,
            ];
            OFFSETS[usize::from(elapsed.min(16))]
        }
        VisibleStationaryMovementEffect::TeleportSpin => 0,
        VisibleStationaryMovementEffect::SkyfallWait => 96,
        VisibleStationaryMovementEffect::SkyfallFall => {
            const OFFSETS: [i16; 17] = [
                96, -87, -78, -69, -60, -51, -43, -36, -29, -22, -17, -12, -8, -5, -2, -1,
                0,
            ];
            OFFSETS[usize::from(elapsed.min(16))]
        }
        VisibleStationaryMovementEffect::SkyfallTop if elapsed >= 16 => 96,
        VisibleStationaryMovementEffect::SkyfallTop => 0,
        VisibleStationaryMovementEffect::DigSpin => 0,
        VisibleStationaryMovementEffect::RockSmash => 0,
    };
    let frames_per_facing = if effect == VisibleStationaryMovementEffect::SkyfallTop {
        2
    } else {
        4
    };
    let facing_index = if effect == VisibleStationaryMovementEffect::DigSpin {
        let initial = match movement.stationary_initial_facing {
            Direction::Down => 0,
            Direction::Right => 1,
            Direction::Up => 2,
            Direction::Left => 3,
        };
        (initial + elapsed / frames_per_facing) % 4
    } else {
        (elapsed / frames_per_facing) % 4
    };
    let facing = match facing_index {
        0 => Direction::Down,
        1 => Direction::Right,
        2 => Direction::Up,
        _ => Direction::Left,
    };
    let object_id = movement.object_id.clone();
    runtime_shell
        .visible_script_movement
        .as_mut()
        .context("stationary movement disappeared while storing sprite offset")?
        .stationary_y_offset = stationary_y_offset;
    if matches!(
        effect,
        VisibleStationaryMovementEffect::TeleportWait
            | VisibleStationaryMovementEffect::SkyfallWait
            | VisibleStationaryMovementEffect::SkyfallFall
            | VisibleStationaryMovementEffect::RockSmash
    ) {
        return Ok(());
    }
    let scene = Arc::make_mut(
        runtime_shell
            .visible_script_movement_scene
            .as_mut()
            .context("stationary movement effect has no retained scene")?,
    );
    if object_id == "PLAYER" {
        scene.overworld.facing = facing;
    } else {
        scene.visible_object_facings.insert(object_id, facing);
    }
    Ok(())
}

fn collect_overworld_keyboard_buttons(
    keys: &ButtonInput<KeyCode>,
    shell_consumes_direction: bool,
    shell_consumes_a: bool,
    shell_consumes_b: bool,
    shell_consumes_start: bool,
    shell_consumes_select: bool,
) -> Vec<GameButton> {
    let mut buttons = Vec::new();
    for (key, button, shell_consumes_button) in [
        (KeyCode::ArrowUp, GameButton::Up, shell_consumes_direction),
        (
            KeyCode::ArrowDown,
            GameButton::Down,
            shell_consumes_direction,
        ),
        (
            KeyCode::ArrowLeft,
            GameButton::Left,
            shell_consumes_direction,
        ),
        (
            KeyCode::ArrowRight,
            GameButton::Right,
            shell_consumes_direction,
        ),
        (KeyCode::KeyZ, GameButton::A, shell_consumes_a),
        (KeyCode::KeyX, GameButton::B, shell_consumes_b),
        (KeyCode::Enter, GameButton::Start, shell_consumes_start),
        (
            KeyCode::ShiftRight,
            GameButton::Select,
            shell_consumes_select,
        ),
    ] {
        if keys.pressed(key) && !shell_consumes_button {
            buttons.push(button);
        }
    }
    buttons
}

fn is_direction_button(button: GameButton) -> bool {
    matches!(
        button,
        GameButton::Up | GameButton::Down | GameButton::Left | GameButton::Right
    )
}

fn game_button_direction(button: GameButton) -> Option<Direction> {
    match button {
        GameButton::Up => Some(Direction::Up),
        GameButton::Down => Some(Direction::Down),
        GameButton::Left => Some(Direction::Left),
        GameButton::Right => Some(Direction::Right),
        _ => None,
    }
}

fn sync_overworld_held_directions(
    keys: &ButtonInput<KeyCode>,
    runtime_shell: &mut BevyRuntimeShell,
    shell_consumes_direction: bool,
) {
    if shell_consumes_direction {
        runtime_shell.overworld_held_directions.clear();
        runtime_shell.overworld_held_direction = None;
        runtime_shell.overworld_buffered_direction = None;
        return;
    }
    for (key, direction) in [
        (KeyCode::ArrowUp, GameButton::Up),
        (KeyCode::ArrowDown, GameButton::Down),
        (KeyCode::ArrowLeft, GameButton::Left),
        (KeyCode::ArrowRight, GameButton::Right),
    ] {
        if !keys.pressed(key) {
            runtime_shell
                .overworld_held_directions
                .retain(|held| *held != direction);
        }
        if keys.just_pressed(key) {
            runtime_shell
                .overworld_held_directions
                .retain(|held| *held != direction);
            runtime_shell.overworld_held_directions.push_back(direction);
        }
    }
}

/// The core advances movement at tile granularity.  Preserve the Game Boy's
/// visible walking pace by forwarding a newly held direction immediately and
/// then at eight-frame intervals. Direction arbitration has already reduced
/// the physical keyboard state to the single most recently pressed held
/// direction before this cadence gate.
fn throttle_held_overworld_direction(
    runtime_shell: &mut BevyRuntimeShell,
    buttons: &mut Vec<GameButton>,
) {
    let directions = buttons
        .iter()
        .copied()
        .filter(|button| is_direction_button(*button))
        .collect::<Vec<_>>();
    if directions.len() != 1 {
        runtime_shell.overworld_direction_repeat_ticks = 0;
        runtime_shell.overworld_held_direction = None;
        return;
    }
    let direction = directions[0];
    if runtime_shell.overworld_held_direction == Some(direction)
        && runtime_shell.overworld_direction_repeat_ticks > 0
    {
        runtime_shell.overworld_direction_repeat_ticks -= 1;
        buttons.retain(|button| !is_direction_button(*button));
        return;
    }
    runtime_shell.overworld_held_direction = Some(direction);
    runtime_shell.overworld_direction_repeat_ticks = OVERWORLD_STEP_REPEAT_TICKS - 1;
}

fn apply_runtime_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut timer: ResMut<RuntimeTickTimer>,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
) {
    let elapsed_input_ticks = timer.take_presentation_ticks();
    let alt_pressed = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    // The frame system above advances these effects and returns before the
    // authoritative overworld can consume joypad input. Give the visible
    // animation the same exclusive ownership here as well, otherwise START,
    // SELECT, or A/B can open a shell surface over Cut, Whirlpool, Headbutt,
    // fishing, or field travel even though Crystal's animation loop is still
    // holding the joypad. Field-travel and Cut-family state is installed
    // before its use text closes, so keep that preceding textbox interactive
    // until it is gone.
    if visible_noninteractive_field_animation_owns_input(&runtime_shell) {
        runtime_shell.ui_held_direction = None;
        runtime_shell.ui_direction_repeat_ticks = 0;
        return;
    }
    if runtime_shell.pending_field_notice_effect_frames.is_some()
        && runtime_shell.field_notice.is_none()
    {
        return;
    }
    if runtime_shell.visible_fly_animation.is_some()
        || (runtime_shell.visible_waterfall_animation.is_some()
            && runtime_shell.field_notice.is_none())
    {
        return;
    }
    if runtime_shell.pending_time_set.is_some() {
        if keys.just_pressed(KeyCode::ArrowUp) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_time_set_direction(shell, VisibleTimeSetDirection::Up)
            });
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_time_set_direction(shell, VisibleTimeSetDirection::Down)
            });
        }
        if keys.just_pressed(KeyCode::ArrowLeft) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_time_set_direction(shell, VisibleTimeSetDirection::Left)
            });
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_time_set_direction(shell, VisibleTimeSetDirection::Right)
            });
        }
        if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
            run_bevy_action(&mut runtime_shell, press_visible_time_set_a_button);
        }
        if keys.just_pressed(KeyCode::KeyX) {
            run_bevy_action(&mut runtime_shell, press_visible_time_set_b_button);
        }
        // A Bevy update can contain multiple Game Boy frames (for example
        // while the renderer is busy compiling a screen texture).  Advancing
        // only once per update stretches every timed transition, most visibly
        // the 8-frame ASM palette fades.  Consume every elapsed tick so the
        // animation remains tied to GB time rather than host FPS.
        for _ in 0..elapsed_input_ticks {
            if let Err(error) = tick_visible_time_set_screen(&mut runtime_shell) {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                break;
            }
        }
        return;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
            run_bevy_action(&mut runtime_shell, press_visible_oak_intro_a_button);
        }
        if keys.just_pressed(KeyCode::KeyX) {
            run_bevy_action(&mut runtime_shell, press_visible_oak_intro_b_button);
        }
        for _ in 0..elapsed_input_ticks {
            if let Err(error) = tick_visible_oak_intro(&mut runtime_shell) {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                break;
            }
        }
        return;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_gender_selection(shell, -1)
            });
        }
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowRight) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_gender_selection(shell, 1)
            });
        }
        if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
            run_bevy_action(&mut runtime_shell, confirm_visible_gender_selection);
        }
        for _ in 0..elapsed_input_ticks {
            if let Err(error) = tick_visible_gender_selection(&mut runtime_shell) {
                record_visible_runtime_error(&mut runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                break;
            }
        }
        return;
    }
    if runtime_shell.pending_name_choice.is_some() {
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_name_choice(shell, -1)
            });
        }
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowRight) {
            run_bevy_action(&mut runtime_shell, |shell| {
                move_visible_name_choice(shell, 1)
            });
        }
        if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
            run_bevy_action(&mut runtime_shell, confirm_visible_name_choice);
        }
        if keys.just_pressed(KeyCode::KeyX)
            && runtime_shell.pending_standard_capture.is_some()
        {
            run_bevy_action(&mut runtime_shell, |shell| {
                shell.pending_name_choice = None;
                finish_visible_capture_nickname(shell, None)
            });
        }
        return;
    }
    if runtime_shell.pending_name_input.is_some() {
        apply_visible_name_input_keys(&keys, &mut runtime_shell);
        return;
    }
    if runtime_shell.intro_screen.is_some() {
        if !alt_pressed && !ctrl_pressed {
            if keys.just_pressed(KeyCode::KeyZ) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    skip_visible_intro_screen(shell, GameButton::A)
                });
            }
            if keys.just_pressed(KeyCode::Enter) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    skip_visible_intro_screen(shell, GameButton::Start)
                });
            }
            if keys.just_pressed(KeyCode::KeyX) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    skip_visible_intro_screen(shell, GameButton::B)
                });
            }
        }
        return;
    }
    if runtime_shell.credits_screen.is_some() {
        if !alt_pressed && !ctrl_pressed {
            if keys.just_pressed(KeyCode::KeyZ) {
                run_bevy_action(&mut runtime_shell, press_visible_credits_a_button);
            }
            if keys.just_pressed(KeyCode::KeyX) {
                run_bevy_action(&mut runtime_shell, press_visible_credits_b_button);
            }
        }
        return;
    }
    if runtime_shell.pending_delete_save.is_some() {
        if !alt_pressed && !ctrl_pressed {
            if keys.just_pressed(KeyCode::ArrowUp)
                || keys.just_pressed(KeyCode::ArrowDown)
                || keys.just_pressed(KeyCode::ArrowLeft)
                || keys.just_pressed(KeyCode::ArrowRight)
            {
                run_bevy_action(&mut runtime_shell, move_visible_delete_save_cursor);
            }
            if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
                run_bevy_action(&mut runtime_shell, confirm_visible_delete_save_screen);
            }
            if keys.just_pressed(KeyCode::KeyX) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    close_visible_delete_save_screen(shell, "cancel")
                });
            }
        }
        return;
    }
    if runtime_shell.pending_clock_reset.is_some() {
        if !alt_pressed && !ctrl_pressed {
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowRight) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    move_visible_clock_reset_cursor(shell, 1)
                });
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowLeft) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    move_visible_clock_reset_cursor(shell, -1)
                });
            }
            if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
                run_bevy_action(&mut runtime_shell, confirm_visible_clock_reset_screen);
            }
            if keys.just_pressed(KeyCode::KeyX) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    close_visible_clock_reset_screen(shell, "cancel")
                });
            }
        }
        return;
    }
    if runtime_shell.pending_mystery_gift.is_some() {
        if !alt_pressed && !ctrl_pressed {
            if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
                run_bevy_action(&mut runtime_shell, press_visible_mystery_gift_a_button);
            }
            if keys.just_pressed(KeyCode::KeyX) {
                run_bevy_action(&mut runtime_shell, |shell| {
                    close_visible_mystery_gift_screen(shell, "cancel")
                });
            }
        }
        return;
    }
    if runtime_shell.options_menu_open {
        let held = [
            (KeyCode::ArrowUp, GameButton::Up),
            (KeyCode::ArrowDown, GameButton::Down),
            (KeyCode::ArrowLeft, GameButton::Left),
            (KeyCode::ArrowRight, GameButton::Right),
        ]
        .into_iter()
        .filter_map(|(key, direction)| keys.pressed(key).then_some((key, direction)))
        .collect::<Vec<_>>();
        if held.len() == 1 {
            let (key, direction) = held[0];
            let newly_pressed = keys.just_pressed(key);
            let changed_direction = runtime_shell.ui_held_direction != Some(direction);
            let repeated = !newly_pressed
                && runtime_shell.ui_held_direction == Some(direction)
                && elapsed_input_ticks > 0
                && runtime_shell.ui_direction_repeat_ticks == 0;
            if newly_pressed || changed_direction || repeated {
                dispatch_visible_options_direction(&mut runtime_shell, direction);
                runtime_shell.ui_held_direction = Some(direction);
                runtime_shell.ui_direction_repeat_ticks = if newly_pressed { 15 } else { 4 };
            } else if runtime_shell.ui_held_direction == Some(direction)
                && elapsed_input_ticks > 0
            {
                runtime_shell.ui_direction_repeat_ticks =
                    runtime_shell.ui_direction_repeat_ticks.saturating_sub(1);
            }
        } else {
            runtime_shell.ui_held_direction = None;
            runtime_shell.ui_direction_repeat_ticks = 0;
        }
        if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
            runtime_shell.ui_held_direction = None;
            runtime_shell.ui_direction_repeat_ticks = 0;
            run_bevy_action(&mut runtime_shell, press_visible_a_button);
        }
        if keys.just_pressed(KeyCode::KeyX) {
            runtime_shell.ui_held_direction = None;
            runtime_shell.ui_direction_repeat_ticks = 0;
            run_bevy_action(&mut runtime_shell, press_visible_b_button);
        }
        return;
    }
    if runtime_shell.trainer_card_open {
        apply_visible_runtime_controls(&keys, &mut runtime_shell, elapsed_input_ticks > 0);
        if runtime_shell.trainer_card_open {
            for _ in 0..elapsed_input_ticks {
                runtime_shell.trainer_card_colon_ticks += 1;
                if runtime_shell.trainer_card_colon_ticks == 32 {
                    runtime_shell.trainer_card_colon_ticks = 0;
                    runtime_shell.trainer_card_colon_visible =
                        !runtime_shell.trainer_card_colon_visible;
                    if runtime_shell.trainer_card_page == VisibleTrainerCardPage::Info {
                        mark_runtime_snapshot_dirty(&mut runtime_shell);
                    }
                }
                if runtime_shell.trainer_card_page == VisibleTrainerCardPage::JohtoBadges {
                    runtime_shell.trainer_card_badge_ticks =
                        runtime_shell.trainer_card_badge_ticks.wrapping_add(1);
                    if runtime_shell.trainer_card_badge_ticks & 0x07 == 0 {
                        runtime_shell.trainer_card_badge_frame =
                            (runtime_shell.trainer_card_badge_frame + 1) & 0x07;
                        mark_runtime_snapshot_dirty(&mut runtime_shell);
                    }
                }
            }
        }
        return;
    }
    if runtime_shell.title_menu.is_some() {
        let select_pressed = keys.pressed(KeyCode::ShiftRight);
        let select_just_pressed = keys.just_pressed(KeyCode::ShiftRight);
        let b_just_pressed = keys.just_pressed(KeyCode::KeyX);
        let delete_save_combo = !alt_pressed
            && !ctrl_pressed
            && select_just_pressed
            && b_just_pressed
            && keys.just_pressed(KeyCode::ArrowUp);
        if delete_save_combo {
            run_bevy_action(&mut runtime_shell, open_visible_delete_save_screen);
            return;
        }
        let clock_reset_arm_combo = !alt_pressed
            && !ctrl_pressed
            && select_just_pressed
            && b_just_pressed
            && keys.just_pressed(KeyCode::ArrowDown);
        if clock_reset_arm_combo {
            if let Some(title) = runtime_shell
                .title_menu
                .as_mut()
                .filter(|title| matches!(title.phase, VisibleTitlePhase::PressStart))
            {
                title.clock_reset_trigger = true;
                run_bevy_action(&mut runtime_shell, |shell| {
                    record_visible_runtime_action(shell, "title:clock_reset:armed")
                });
            } else {
                run_bevy_action(&mut runtime_shell, |shell| {
                    record_visible_runtime_action(shell, "title:clock_reset:ignored")
                });
            }
            return;
        }
        let clock_reset_open = !alt_pressed
            && !ctrl_pressed
            && !select_pressed
            && runtime_shell
                .title_menu
                .as_ref()
                .is_some_and(|title| title.clock_reset_trigger)
            && keys.pressed(KeyCode::ArrowLeft)
            && keys.pressed(KeyCode::ArrowUp);
        if clock_reset_open {
            run_bevy_action(&mut runtime_shell, open_visible_clock_reset_screen);
            return;
        }
        if keys.just_pressed(KeyCode::ArrowUp)
            || keys.just_pressed(KeyCode::ArrowLeft)
            || keys.just_pressed(KeyCode::ArrowDown)
            || keys.just_pressed(KeyCode::ArrowRight)
        {
            let delta =
                if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft) {
                    -1
                } else {
                    1
                };
            let input = if delta < 0 {
                GameButton::Up
            } else {
                GameButton::Down
            };
            match press_visible_title_direction_button(&mut runtime_shell, input, delta) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                }
            }
        }
        if keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Enter) {
            let input = if keys.just_pressed(KeyCode::Enter) {
                GameButton::Start
            } else {
                GameButton::A
            };
            match press_visible_title_confirm_button(&mut runtime_shell, input) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                }
            }
        }
        if keys.just_pressed(KeyCode::KeyX) {
            match press_visible_title_cancel_button(&mut runtime_shell) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => {
                    record_visible_runtime_error(&mut runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                }
            }
        }
        return;
    }
    apply_visible_runtime_controls(&keys, &mut runtime_shell, elapsed_input_ticks > 0);
}

fn dispatch_visible_options_direction(
    runtime_shell: &mut BevyRuntimeShell,
    direction: GameButton,
) {
    let result = match direction {
        GameButton::Up => move_visible_options_cursor(runtime_shell, -1),
        GameButton::Down => move_visible_options_cursor(runtime_shell, 1),
        GameButton::Left => change_visible_options_selection(runtime_shell, -1),
        GameButton::Right => change_visible_options_selection(runtime_shell, 1),
        _ => return,
    };
    match result {
        Ok(()) => runtime_shell.last_error = None,
        Err(error) => {
            record_visible_runtime_error(runtime_shell, &error);
            runtime_shell.last_error = Some(error.to_string());
        }
    }
}

fn drain_unused_runtime_ticks(mut timer: ResMut<RuntimeTickTimer>) {
    // Title/intro/credits and ordinary gameplay advance on their own clocks.
    // Do not carry their keyboard-timer ticks into a later modal screen and
    // accidentally fast-forward its fade or text animation.
    timer.take_ticks();
    timer.take_presentation_ticks();
}

fn sync_visible_earthquake_camera(
    runtime_shell: Res<BevyRuntimeShell>,
    mut cameras: Query<&mut Transform, With<MainCameraMarker>>,
) {
    let Ok(mut transform) = cameras.get_single_mut() else {
        return;
    };
    let earthquake_offset = runtime_shell
        .visible_earthquake
        .filter(|earthquake| earthquake.frames_remaining > 0)
        .map(|earthquake| {
            let distance = f32::from(earthquake.intensity.max(1)) * 4.0;
            match earthquake.phase % 4 {
                0 => (distance, 0.0),
                1 => (-distance, 0.0),
                2 => (0.0, distance),
                _ => (0.0, -distance),
            }
        })
        .unwrap_or((0.0, 0.0));
    let battle_offset = visible_move_screen_shake_offset(
        runtime_shell.visible_move_animations.front(),
    );
    let (x, y) = (
        earthquake_offset.0 + battle_offset.0,
        earthquake_offset.1 + battle_offset.1,
    );
    transform.translation.x = x;
    transform.translation.y = y;
}

fn visible_move_screen_shake_offset(
    animation: Option<&VisibleMoveAnimation>,
) -> (f32, f32) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (0.0, 0.0);
    };
    let mut offset = (0.0, 0.0);
    for effect in animation
        .bg_events
        .iter()
        .filter(|effect| !effect.incremented && effect.frame <= animation.frame)
    {
        if animation.bg_events.iter().any(|candidate| {
            !candidate.incremented
                && candidate.effect_id == effect.effect_id
                && candidate.frame > effect.frame
                && candidate.frame <= animation.frame
        }) {
            continue;
        }
        if effect.effect_id == "BATTLE_BG_EFFECT_WOBBLE_SCREEN" {
            let reset_frame = animation
                .bg_events
                .iter()
                .filter(|candidate| {
                    candidate.incremented
                        && candidate.effect_id == effect.effect_id
                        && candidate.frame >= effect.frame
                        && candidate.frame <= animation.frame
                })
                .map(|candidate| candidate.frame)
                .max()
                .unwrap_or(effect.frame);
            let active_age = animation.frame.saturating_sub(reset_frame);
            let frequency = if effect.duration == 0 { 4 } else { effect.duration };
            let lifetime = if effect.duration == 0 {
                frequency.saturating_mul(2)
            } else {
                effect.duration
            };
            if active_age < lifetime.max(1) {
                let phase_age = animation.frame.saturating_sub(effect.frame);
                let amplitude = if effect.param == 0 { 3 } else { effect.param };
                let value = (f64::from(amplitude)
                    * (f64::from(phase_age) * std::f64::consts::PI
                        / f64::from(frequency.max(1)))
                    .sin())
                    .round() as f32
                    * (TILE_SIZE / SOURCE_TILE_SIZE as f32);
                offset.0 += value;
            }
            continue;
        }
        let axis_x = match effect.effect_id.as_str() {
            "BATTLE_BG_EFFECT_SHAKE_SCREEN_X" => true,
            "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y" => false,
            _ => continue,
        };
        let age = animation.frame.saturating_sub(effect.frame);
        if age >= effect.duration.max(1) {
            continue;
        }
        let amplitude = parse_visible_battle_animation_int(&effect.target)
            .filter(|value| *value > 0)
            .unwrap_or_else(|| {
                let encoded = effect.param >> 4;
                i32::from(if encoded == 0 { 4 } else { encoded })
            });
        let encoded_frequency = effect.param & 0x0f;
        let frequency = u16::from(if encoded_frequency == 0 {
            2
        } else {
            encoded_frequency
        });
        let signed = if (age / frequency) % 2 == 0 {
            amplitude
        } else {
            -amplitude
        } as f32
            * (TILE_SIZE / SOURCE_TILE_SIZE as f32);
        if axis_x {
            offset.0 += signed;
        } else {
            offset.1 += signed;
        }
    }
    offset
}

fn apply_visible_runtime_controls(
    keys: &ButtonInput<KeyCode>,
    runtime_shell: &mut BevyRuntimeShell,
    advance_repeat: bool,
) {
    if runtime_shell.visible_script_movement.is_some()
        || visible_noninteractive_field_animation_owns_input(runtime_shell)
        || visible_noninteractive_battle_animation_owns_input(runtime_shell)
    {
        runtime_shell.ui_held_direction = None;
        runtime_shell.ui_direction_repeat_ticks = 0;
        return;
    }
    let shift_pressed = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt_pressed = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    let plain_input = !shift_pressed && !alt_pressed && !ctrl_pressed;
    if keys.just_pressed(KeyCode::KeyZ)
        || keys.just_pressed(KeyCode::KeyX)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::ShiftRight)
    {
        // Confirm/cancel commonly replaces the active cursor surface. Each
        // TypeScript menu owns its own repeat state, so a held direction must
        // begin a fresh initial delay after that boundary.
        runtime_shell.ui_held_direction = None;
        runtime_shell.ui_direction_repeat_ticks = 0;
    }
    if plain_input && has_visible_shell_direction_action(runtime_shell) {
        let held_directions = [
            (KeyCode::ArrowUp, GameButton::Up),
            (KeyCode::ArrowDown, GameButton::Down),
            (KeyCode::ArrowLeft, GameButton::Left),
            (KeyCode::ArrowRight, GameButton::Right),
        ]
        .into_iter()
        .filter_map(|(key, direction)| keys.pressed(key).then_some((key, direction)))
        .collect::<Vec<_>>();
        let newly_pressed_direction = held_directions
            .iter()
            .find_map(|(key, direction)| keys.just_pressed(*key).then_some(*direction));
        let active_held_direction = runtime_shell.ui_held_direction.filter(|active| {
            held_directions
                .iter()
                .any(|(_, direction)| direction == active)
        });
        if let Some(direction) = newly_pressed_direction {
            dispatch_visible_ui_direction(runtime_shell, direction);
            runtime_shell.ui_held_direction = Some(direction);
            runtime_shell.ui_direction_repeat_ticks =
                visible_ui_initial_repeat_ticks(runtime_shell);
        } else if let Some(direction) = active_held_direction {
            let repeated = advance_repeat
                && runtime_shell.ui_direction_repeat_ticks == 0;
            if repeated {
                dispatch_visible_ui_direction(runtime_shell, direction);
                runtime_shell.ui_direction_repeat_ticks = 4;
            } else if advance_repeat {
                runtime_shell.ui_direction_repeat_ticks =
                    runtime_shell.ui_direction_repeat_ticks.saturating_sub(1);
            }
        } else if let Some((_, direction)) = held_directions.first().copied() {
            // A surface replacement may occur while a direction remains held.
            // Adopt it without manufacturing a new press, then give the new
            // menu its complete initial repeat delay.
            runtime_shell.ui_held_direction = Some(direction);
            runtime_shell.ui_direction_repeat_ticks =
                visible_ui_initial_repeat_ticks(runtime_shell);
        } else {
            runtime_shell.ui_held_direction = None;
            runtime_shell.ui_direction_repeat_ticks = 0;
        }
    } else {
        runtime_shell.ui_held_direction = None;
        runtime_shell.ui_direction_repeat_ticks = 0;
    }
    let overworld_interaction_consumed_this_press =
        std::mem::take(&mut runtime_shell.overworld_interaction_consumed_a);
    if keys.just_pressed(KeyCode::KeyZ)
        && plain_input
        && !overworld_interaction_consumed_this_press
    {
        match has_visible_shell_a_action(runtime_shell) {
            Ok(true) => run_bevy_action(runtime_shell, press_visible_a_button),
            Ok(false) => {}
            Err(error) => {
                record_visible_runtime_error(runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
            }
        }
    }
    if keys.just_pressed(KeyCode::ShiftRight) && !alt_pressed && !ctrl_pressed {
        if has_visible_shell_select_action(runtime_shell) {
            run_bevy_action(runtime_shell, press_visible_select_button);
        }
    }
    if keys.just_pressed(KeyCode::KeyX) && plain_input {
        if has_visible_shell_b_action(runtime_shell) {
            run_bevy_action(runtime_shell, press_visible_b_button);
        }
    }
    if keys.just_pressed(KeyCode::Enter) && plain_input {
        // Return is the desktop confirm key. Give an active menu, textbox, or
        // field interaction (such as a PC directly in front of the player)
        // priority over Start; otherwise it continues to open the Start menu
        // while walking around normally.
        match has_visible_shell_a_action(runtime_shell) {
            Ok(true) => {
                run_bevy_action(runtime_shell, press_visible_a_button);
                return;
            }
            Ok(false) => {}
            Err(error) => {
                record_visible_runtime_error(runtime_shell, &error);
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
        }
        if has_visible_shell_start_action(runtime_shell) {
            run_bevy_action(runtime_shell, press_visible_start_button);
        }
    }
}

fn visible_noninteractive_battle_animation_owns_input(
    runtime_shell: &BevyRuntimeShell,
) -> bool {
    runtime_shell.visible_battle_transition.is_some()
        || runtime_shell.visible_frontpic_animation.is_some()
        || runtime_shell
            .visible_capture_animation
            .as_ref()
            .is_some_and(|animation| animation.started)
        || runtime_shell
            .visible_move_animations
            .front()
            .is_some_and(|animation| animation.started)
        || runtime_shell.visible_send_out_animation.is_some()
        || runtime_shell.visible_trainer_exit_animation.is_some()
        || runtime_shell
            .battle_hp_tween
            .as_ref()
            .is_some_and(visible_battle_hp_tween_active)
}

fn visible_noninteractive_field_animation_owns_input(
    runtime_shell: &BevyRuntimeShell,
) -> bool {
    runtime_shell.pending_trainer_sight.is_some()
        || runtime_shell.visible_strength_notice_phase
            == Some(VisibleStrengthNoticePhase::CryPause)
        || runtime_shell.visible_walk_warp_phase.is_some()
        || runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell
            .visible_blackout_phase
            .is_some_and(|phase| phase != VisibleBlackoutPhase::AwaitText)
        || runtime_shell.visible_fishing_animation
        .is_some_and(|animation| animation.phase != VisibleFishingPhase::AwaitText)
        || runtime_shell.visible_fly_animation.is_some()
        || (runtime_shell.visible_waterfall_animation.is_some()
            && runtime_shell.field_notice.is_none())
        || runtime_shell.visible_flash_animation.is_some()
        || (runtime_shell.field_notice.is_none()
            && (runtime_shell.visible_field_travel_animation.is_some()
                || runtime_shell.visible_cut_animation.is_some()
                || runtime_shell.visible_whirlpool_animation.is_some()
                || runtime_shell.visible_headbutt_animation.is_some()))
}

fn visible_ui_initial_repeat_ticks(runtime_shell: &BevyRuntimeShell) -> u8 {
    let battle_surface = runtime_shell.battle_action_cursor.is_some()
        || runtime_shell.battle_move_cursor.is_some()
        || runtime_shell.battle_switch_cursor.is_some()
        || runtime_shell.battle_faint_prompt_cursor.is_some()
        || runtime_shell.battle_shift_prompt_cursor.is_some()
        || runtime_shell.battle_party_action_cursor.is_some()
        || runtime_shell.battle_pack_target_mode.is_some();
    if battle_surface { 8 } else { 12 }
}

fn dispatch_visible_ui_direction(runtime_shell: &mut BevyRuntimeShell, direction: GameButton) {
    if !runtime_shell.battle_messages.is_empty()
        || runtime_shell
        .battle_exp_tween
        .as_ref()
        .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active)
    {
        // Battle text owns the complete joypad while it is visible. Crystal's
        // text engine ignores directional input here; it must not move the
        // retained battle-menu cursor hidden underneath the textbox. Besides
        // selecting the wrong command after the text closes, moving that
        // hidden cursor changes the render key every repeat tick and can turn
        // a held direction into an expensive redraw loop.
        return;
    }
    let (input_action, action) = match direction {
        GameButton::Up => (
            "input:ui:Up",
            move_visible_primary_cursor_up as fn(&mut BevyRuntimeShell) -> Result<()>,
        ),
        GameButton::Down => (
            "input:ui:Down",
            move_visible_primary_cursor_down as fn(&mut BevyRuntimeShell) -> Result<()>,
        ),
        GameButton::Left => (
            "input:ui:Left",
            move_visible_primary_cursor_left as fn(&mut BevyRuntimeShell) -> Result<()>,
        ),
        GameButton::Right => (
            "input:ui:Right",
            move_visible_primary_cursor_right as fn(&mut BevyRuntimeShell) -> Result<()>,
        ),
        _ => return,
    };
    run_bevy_input_action(runtime_shell, input_action, action);
}

fn run_bevy_input_action(
    runtime_shell: &mut BevyRuntimeShell,
    input_action: &'static str,
    action: fn(&mut BevyRuntimeShell) -> Result<()>,
) {
    if let Err(error) = record_visible_runtime_action(runtime_shell, input_action) {
        record_visible_runtime_error(runtime_shell, &error);
        runtime_shell.last_error = Some(error.to_string());
        return;
    }
    run_bevy_action(runtime_shell, action);
}

fn run_bevy_action<F>(
    runtime_shell: &mut BevyRuntimeShell,
    action: F,
)
where
    F: FnOnce(&mut BevyRuntimeShell) -> Result<()>,
{
    // Options is a live interactive surface.  Advancing the script runner after
    // opening or editing it treats the menu as a non-interactive window, closes
    // it immediately, and resets its cursor on the same frame.  Keep the frame
    // in the menu until the player explicitly presses B.
    let options_was_open = runtime_shell.options_menu_open;
    let action_result = action(runtime_shell);
    let should_advance = action_result.is_ok()
        && !options_was_open
        && !runtime_shell.options_menu_open
        // Title navigation is already a player boundary.  The title action
        // must not feed its own menu selection back into the script pump.
        && runtime_shell.title_menu.is_none()
        && runtime_shell.pending_gender_selection.is_none()
        && runtime_shell.pending_name_choice.is_none()
        && runtime_shell.pending_name_input.is_none()
        && runtime_shell.pending_oak_intro.is_none()
        && runtime_shell.pending_time_set.is_none()
        && runtime_shell.pending_trainer_sight.is_none();
    let result = action_result.and_then(|()| {
        if should_advance {
            advance_visible_script_until_player_boundary(runtime_shell)
        } else {
            Ok(())
        }
    });
    match result {
        Ok(()) => {
            runtime_shell.last_error = None;
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        Err(error) => {
            record_visible_runtime_error(runtime_shell, &error);
            runtime_shell.last_error = Some(format!("{error:#}"));
        }
    }
    sync_visible_battle_action_cursor(runtime_shell);
}

fn mark_runtime_snapshot_dirty(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.snapshot_revision = runtime_shell.snapshot_revision.wrapping_add(1);
    runtime_shell.cached_snapshot = None;
}

fn mark_runtime_presentation_dirty(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.snapshot_revision = runtime_shell.snapshot_revision.wrapping_add(1);
    if let Some((revision, _)) = runtime_shell.cached_snapshot.as_mut() {
        *revision = runtime_shell.snapshot_revision;
    }
}

fn cached_runtime_snapshot(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<Arc<RuntimeShellSnapshot>> {
    if let Some((revision, snapshot)) = runtime_shell.cached_snapshot.as_ref() {
        if *revision == runtime_shell.snapshot_revision {
            return Ok(Arc::clone(snapshot));
        }
    }
    let snapshot = Arc::new(runtime_shell.shell.presentation_snapshot()?);
    runtime_shell.cached_snapshot = Some((runtime_shell.snapshot_revision, Arc::clone(&snapshot)));
    Ok(snapshot)
}

fn advance_visible_script_until_player_boundary(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    const MAX_AUTO_SCRIPT_STEPS: usize = 256;
    let mut advanced = 0usize;
    loop {
        if close_visible_noninteractive_runtime_surface(runtime_shell)? {
            continue;
        }
        let snapshot = runtime_shell.shell.presentation_snapshot()?;
        if visible_player_boundary(runtime_shell, &snapshot)
            || !has_visible_auto_script_action(runtime_shell, &snapshot)
        {
            return Ok(());
        }
        if advanced >= MAX_AUTO_SCRIPT_STEPS {
            anyhow::bail!(
                "visible script auto-advance exceeded {MAX_AUTO_SCRIPT_STEPS} steps before reaching a player boundary"
            );
        }
        press_visible_a_button(runtime_shell)?;
        mark_runtime_snapshot_dirty(runtime_shell);
        advanced += 1;
    }
}

fn visible_player_boundary(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> bool {
    runtime_shell.field_text_reveal.is_some()
        || runtime_shell.visible_script_delay_frames.is_some_and(|frames| frames > 0)
        || runtime_shell
            .visible_earthquake
            .as_ref()
            .is_some_and(|earthquake| earthquake.frames_remaining > 0)
        || runtime_shell
            .visible_overworld_emote
            .as_ref()
            .is_some_and(|emote| emote.frames_remaining > 0)
        || runtime_shell.visible_mom_bank.is_some()
        || runtime_shell.visible_script_movement.is_some()
        || runtime_shell.visible_card_flip.is_some()
        || runtime_shell.visible_slot_machine.is_some()
        || runtime_shell.visible_unown_puzzle.is_some()
        || runtime_shell.pending_day_of_week.is_some()
        || runtime_shell.kurt_apricorn_cursor.is_some()
        || runtime_shell.buena_prize_cursor.is_some()
        || runtime_shell.pending_trainer_sight.is_some()
        || snapshot.script_events.pending_text_label.is_some()
        || snapshot.ui.pending_yes_no.is_some()
        || runtime_shell.pending_phone_prompt.is_some()
        || snapshot.ui.pending_text_wait.is_some()
        || snapshot.pending_move_learn.is_some()
        || snapshot.pending_shop.is_some()
        || (snapshot.ui.text_window_open && runtime_shell.active_script_cursor.is_none())
        || snapshot.ui.window_open
        || snapshot.ui.active_pokemon_picture.is_some()
        || has_visible_elevator_prompt(snapshot, runtime_shell)
        || has_visible_gift_pokemon_prompt(snapshot, runtime_shell)
        || visible_menu_has_selectable_options(snapshot)
        || snapshot.battle.is_some()
        || runtime_shell.start_menu_cursor.is_some()
        || runtime_shell.party_menu_open
        || runtime_shell.pokedex_menu_open
        || runtime_shell.pokegear_menu_open
        || runtime_shell.options_menu_open
        || runtime_shell.save_menu_open
        || runtime_shell.special_boundary.is_some()
        || runtime_shell.intro_screen.is_some()
        || runtime_shell.pending_gender_selection.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.pending_name_input.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_time_set.is_some()
        || runtime_shell.credits_screen.is_some()
        || visible_field_pack_is_open(runtime_shell)
        || runtime_shell.bill_pc_action_cursor.is_some()
        || runtime_shell.pc_hub_cursor.is_some()
        || runtime_shell.player_pc_action_cursor.is_some()
        || runtime_shell.mailbox_cursor.is_some()
        || runtime_shell.mailbox_action_cursor.is_some()
        || runtime_shell.storage_cursor.is_some()
        || runtime_shell.pc_item_cursor.is_some()
        || runtime_shell.player_pc_action_cursor.is_some()
        || runtime_shell.mailbox_cursor.is_some()
        || runtime_shell.mailbox_action_cursor.is_some()
}

fn has_visible_auto_script_action(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> bool {
    snapshot.script_events.pending_text_label.is_some()
        || snapshot.script_events.pending_map_load.is_some()
        || snapshot.script_events.pending_map_refresh.is_some()
        || snapshot.script_events.pending_music_fade.is_some()
        || snapshot.script_events.pending_screen_fade.is_some()
        || !snapshot.script_events.pending_delays.is_empty()
        || !snapshot.script_events.pending_earthquakes.is_empty()
        || !snapshot.script_events.pending_emotes.is_empty()
        || snapshot.script_events.pending_script_warp.is_some()
        || !snapshot.script_events.command_queue.is_empty()
        || snapshot.script_events.next_script.is_some()
        || !snapshot.script_events.deferred_scripts.is_empty()
        || snapshot.script_events.script_ended.is_some()
        || !snapshot.script_events.audio_events.is_empty()
        || has_visible_pending_non_audio_script_events(snapshot)
        || visible_auto_runtime_flag(snapshot).is_some()
        || runtime_shell.active_script_cursor.is_some()
}

fn has_visible_pending_non_audio_script_events(snapshot: &RuntimeShellSnapshot) -> bool {
    !snapshot.script_events.graphics_events.is_empty()
        || !snapshot.script_events.money_events.is_empty()
        || !snapshot.script_events.map_events.is_empty()
        || !snapshot.script_events.control_events.is_empty()
        || !snapshot.script_events.shop_events.is_empty()
        || !snapshot.script_events.item_use_events.is_empty()
}

fn visible_menu_has_selectable_options(snapshot: &RuntimeShellSnapshot) -> bool {
    snapshot.ui.menu.as_ref().is_some_and(|menu| {
        menu.layout
            .vertical_menus
            .iter()
            .any(|vertical| !vertical.options.is_empty())
    })
}

fn close_visible_noninteractive_runtime_surface(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<bool> {
    // Some core menus have no selectable rows because Bevy presents their
    // interactive surface itself. In particular OverworldTownMap owns the
    // Pokegear map screen. Auto-closing that core marker here erased the map
    // in the same continuation pass that opened it.
    if runtime_shell.pokegear_menu_open {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let Some(menu) = &snapshot.ui.menu else {
        return Ok(false);
    };
    if menu
        .layout
        .vertical_menus
        .iter()
        .any(|vertical| !vertical.options.is_empty())
    {
        return Ok(false);
    }
    close_active_runtime_surface(runtime_shell)?;
    Ok(true)
}

fn close_visible_noninteractive_runtime_surfaces_until_idle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    const MAX_NONINTERACTIVE_SURFACE_CLOSES: usize = 64;
    for _ in 0..MAX_NONINTERACTIVE_SURFACE_CLOSES {
        if !close_visible_noninteractive_runtime_surface(runtime_shell)? {
            return Ok(());
        }
    }
    anyhow::bail!(
        "visible shell exceeded noninteractive runtime surface close limit {MAX_NONINTERACTIVE_SURFACE_CLOSES}"
    )
}

fn finish_visible_empty_battle_reward_presentation(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<bool> {
    if !runtime_shell.battle_messages.is_empty()
        || runtime_shell.battle_exp_tween.is_some()
        || !runtime_shell.pending_battle_exp_tweens.is_empty()
        || !runtime_shell.battle_level_stats.is_empty()
        || runtime_shell.battle_message_scene.is_none()
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if snapshot.battle.is_none() {
        runtime_shell.battle_message_scene = None;
        runtime_shell.battle_hp_tween = None;
        runtime_shell.battle_fanfare_messages.clear();
        runtime_shell.battle_evolution_cries.clear();
        runtime_shell.battle_evolution_cancellations.clear();
        runtime_shell.battle_sounds_after_messages.clear();
        reset_visible_music_state(runtime_shell);
        queue_visible_current_music(runtime_shell)?;
        if runtime_shell.pending_plain_battle_map_reload {
            begin_visible_plain_battle_map_reload(runtime_shell)?;
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    let resume_trainer_settlement = runtime_shell.battle_shift_prompt_cursor.is_none()
        && runtime_shell.battle_switch_cursor.is_none()
        && snapshot.battle.as_ref().is_some_and(|battle| {
            matches!(&battle.kind, crate::RuntimeBattleKind::Trainer { .. })
                && battle.enemy_pokemon.hp == 0
        });
    if resume_trainer_settlement {
        runtime_shell.battle_message_scene = None;
        settle_visible_battle_after_action(runtime_shell)?;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(true);
    }
    Ok(false)
}

fn restore_visible_cancelled_evolution(
    runtime_shell: &mut BevyRuntimeShell,
    cancellation: &mut VisibleEvolutionCancellation,
) -> Result<String> {
    let source_name = cancellation
        .report
        .cancel_snapshot
        .as_ref()
        .map(|pokemon| pokemon.nickname.clone())
        .context("cancelable evolution is missing its source Pokemon snapshot")?;
    let pending_move_names = cancellation
        .report
        .pending_move_learns
        .iter()
        .map(|learned| learned.name.clone())
        .collect::<HashSet<_>>();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        let pokemon = state
            .storage
            .party
            .pokemon
            .get_mut(cancellation.party_index)
            .and_then(Option::as_mut)
            .with_context(|| {
                format!(
                    "cancel evolution party index {} is empty",
                    cancellation.party_index
                )
            })?;
        crate::core::systems::evolution::cancel_evolution(
            pokemon,
            &mut cancellation.report,
        )
        .context("cancel visible battle evolution")?;

        if state.pending_move_learn.as_ref().is_some_and(|pending| {
            pending.party_index == cancellation.party_index
                && pending_move_names.contains(&pending.learned_move.name)
        }) {
            state.pending_move_learn = None;
        }
        state.pending_move_learn_queue.retain(|pending| {
            pending.party_index != cancellation.party_index
                || !pending_move_names.contains(&pending.learned_move.name)
        });
        crate::core::systems::battle_rewards::promote_next_pending_move_learn(state);
        state.sync_party_from_storage();
        crate::core::systems::battle_rewards::sync_active_combat_player_party_from_storage(state);
    }
    Ok(source_name)
}

fn cancel_visible_battle_evolution(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    let Some(cancellation) = runtime_shell.battle_evolution_cancellations.front() else {
        return Ok(false);
    };
    let Some(message) = runtime_shell.battle_messages.front() else {
        return Ok(false);
    };
    if message != &cancellation.trigger_message
        || !visible_battle_message_is_complete(runtime_shell, message)
    {
        return Ok(false);
    }

    let mut cancellation = runtime_shell
        .battle_evolution_cancellations
        .pop_front()
        .expect("checked pending evolution cancellation");
    let source_name = restore_visible_cancelled_evolution(runtime_shell, &mut cancellation)?;

    let staged_scenes_aligned = runtime_shell.battle_message_scenes.len()
        == runtime_shell.battle_messages.len();
    let stopped_scene = runtime_shell
        .battle_message_scenes
        .front()
        .cloned()
        .or_else(|| runtime_shell.battle_message_scene.clone());
    let mut removed_messages = 0usize;
    if runtime_shell.battle_messages.front() == Some(&cancellation.trigger_message) {
        runtime_shell.battle_messages.pop_front();
        removed_messages += 1;
    }
    if runtime_shell.battle_messages.front() == Some(&cancellation.evolved_message) {
        runtime_shell.battle_messages.pop_front();
        removed_messages += 1;
    }
    for pending_message in &cancellation.pending_move_messages {
        if runtime_shell.battle_messages.front() == Some(pending_message) {
            runtime_shell.battle_messages.pop_front();
            removed_messages += 1;
        }
    }
    let stopped_message = format!("Huh? {}\nstopped evolving!", source_name);
    runtime_shell
        .battle_messages
        .push_front(stopped_message.clone());
    runtime_shell.battle_text_reveal = None;
    if staged_scenes_aligned {
        for _ in 0..removed_messages {
            runtime_shell.battle_message_scenes.pop_front();
        }
        if let Some(scene) = stopped_scene {
            runtime_shell.battle_message_scenes.push_front(scene);
        } else {
            runtime_shell.battle_message_scenes.clear();
        }
    } else {
        runtime_shell.battle_message_scenes.clear();
    }
    runtime_shell
        .battle_evolution_cries
        .retain(|(_, trigger)| trigger != &cancellation.trigger_message);
    runtime_shell
        .battle_sounds_after_messages
        .retain(|(_, trigger)| trigger != &cancellation.trigger_message);
    runtime_shell.last_audio_events.push(format!(
        "battle evolution cancelled party_index={} species={}",
        cancellation.party_index,
        cancellation
            .report
            .cancel_snapshot
            .as_ref()
            .map(|pokemon| pokemon.species.id.as_str())
            .unwrap_or("restored")
    ));
    set_shell_action_status(runtime_shell, "STOPPED EVOLVING");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn cancel_visible_field_evolution(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    let Some(cancellation) = runtime_shell.field_evolution_cancellation.as_ref() else {
        return Ok(false);
    };
    if runtime_shell.field_notice.as_deref() != Some(cancellation.trigger_message.as_str()) {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if !visible_field_dialogue_is_fully_revealed(runtime_shell, &snapshot) {
        return Ok(false);
    }

    let mut cancellation = runtime_shell
        .field_evolution_cancellation
        .take()
        .expect("checked field evolution cancellation");
    let source_name = restore_visible_cancelled_evolution(runtime_shell, &mut cancellation)?;
    if runtime_shell.field_notice_queue.front() == Some(&cancellation.evolved_message) {
        runtime_shell.field_notice_queue.pop_front();
    }
    for pending_message in &cancellation.pending_move_messages {
        if runtime_shell.field_notice_queue.front() == Some(pending_message) {
            runtime_shell.field_notice_queue.pop_front();
        }
    }
    runtime_shell.field_notice = Some(format!("Huh? {}\nstopped evolving!", source_name));
    runtime_shell.field_text_reveal = None;
    runtime_shell.pending_field_notice_cry = None;
    runtime_shell.last_audio_events.push(format!(
        "field evolution cancelled party_index={}",
        cancellation.party_index
    ));
    set_shell_action_status(runtime_shell, "STOPPED EVOLVING");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn press_visible_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_diploma.is_some() {
        return close_visible_diploma(runtime_shell);
    }
    if runtime_shell.visible_unown_words.is_some() {
        return close_visible_unown_words(runtime_shell);
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
    {
        return Ok(());
    }
    if runtime_shell
        .battle_exp_tween
        .as_ref()
        .is_some_and(|tween| tween.started)
    {
        return Ok(());
    }
    if let Some(stats) = runtime_shell.battle_level_stats.front()
        && stats.active
    {
        if stats.frames_before_input == 0 {
            runtime_shell.battle_level_stats.pop_front();
            mark_runtime_snapshot_dirty(runtime_shell);
            finish_visible_empty_battle_reward_presentation(runtime_shell)?;
        }
        return Ok(());
    }
    if runtime_shell.visible_card_flip.is_some() {
        return flip_visible_card(runtime_shell);
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return spin_visible_slot_machine(runtime_shell);
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return use_visible_unown_puzzle_cell(runtime_shell);
    }
    if runtime_shell.visible_mom_bank.is_some() {
        return confirm_visible_mom_bank(runtime_shell);
    }
    if runtime_shell.pending_day_of_week.is_some() {
        return confirm_visible_day_of_week(runtime_shell);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        return resolve_visible_kurt_apricorn_selection(runtime_shell, false);
    }
    if runtime_shell.buena_prize_cursor.is_some()
        && runtime_shell.pc_confirmation.is_none()
        && runtime_shell.pc_notice.is_none()
    {
        return resolve_visible_buena_prize_selection(runtime_shell, false);
    }
    if !runtime_shell.battle_messages.is_empty() {
        if runtime_shell
            .battle_hp_tween
            .as_ref()
            .is_some_and(visible_battle_hp_tween_active)
        {
            return Ok(());
        }
        if runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
        {
            return Ok(());
        }
        let message = runtime_shell
            .battle_messages
            .front()
            .expect("checked nonempty battle message queue");
        if !visible_battle_message_is_complete(runtime_shell, message) {
            return Ok(());
        }
        if visible_battle_message_has_more_pages(runtime_shell, message) {
            let message = message.clone();
            if advance_visible_battle_message_page(runtime_shell, &message) {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            return Ok(());
        }
        let staged_scenes_aligned = runtime_shell.battle_message_scenes.len()
            == runtime_shell.battle_messages.len();
        let dismissed_battle_message = runtime_shell.battle_messages.pop_front();
        runtime_shell.battle_text_reveal = None;
        if let Some(animation) = runtime_shell.visible_capture_animation.as_mut()
            && animation.complete
            && animation.caught
            && !animation.sprites_cleared
            && dismissed_battle_message
                .as_deref()
                .is_some_and(|message| message.starts_with("Gotcha! "))
        {
            // PokeBallEffect keeps the caught ball through Gotcha, then calls
            // ClearSprites before either the Pokedex or nickname flow. Core
            // capture mutation is still deferred, so retain the presentation
            // state to keep its authoritative enemy hidden after this clear.
            animation.sprites_cleared = true;
        }
        if dismissed_battle_message.is_some() {
            queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
        }
        if runtime_shell
            .battle_evolution_cries
            .front()
            .is_some_and(|(_, trigger)| dismissed_battle_message.as_deref() == Some(trigger.as_str()))
        {
            let (species_id, _) = runtime_shell.battle_evolution_cries.pop_front().unwrap();
            queue_visible_pokemon_cry(runtime_shell, &species_id, "battle_evolution")?;
        }
        if runtime_shell
            .battle_sounds_after_messages
            .front()
            .is_some_and(|(_, trigger)| dismissed_battle_message.as_deref() == Some(trigger.as_str()))
        {
            let (sound_id, _) = runtime_shell.battle_sounds_after_messages.pop_front().unwrap();
            queue_visible_shell_sound_effect(runtime_shell, &sound_id)?;
        }
        if runtime_shell
            .battle_evolution_cancellations
            .front()
            .is_some_and(|cancellation| {
                dismissed_battle_message.as_deref()
                    == Some(cancellation.trigger_message.as_str())
            })
        {
            runtime_shell.battle_evolution_cancellations.pop_front();
        }
        let starts_exp_animation = runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| {
                !tween.started
                    && (dismissed_battle_message.as_deref()
                        == Some(tween.trigger_message.as_str())
                        || (tween.pixels == tween.target_pixels
                            && !tween.remaining_targets.is_empty()
                            && dismissed_battle_message
                                .as_deref()
                                .is_some_and(|message| message.contains(" grew to\nlevel "))))
            });
        if starts_exp_animation {
            let tween = runtime_shell.battle_exp_tween.as_mut().unwrap();
            if tween.pixels == tween.target_pixels {
                tween.pixels = 0;
                tween.target_pixels = tween
                    .remaining_targets
                    .pop_front()
                    .context("multi-level EXP continuation has no next bar target")?;
            }
            tween.steps_in_segment = 0;
            tween.frames_until_step = 9;
            tween.started = true;
            queue_visible_shell_sound_effect(runtime_shell, "SFX_EXP_BAR")?;
        }
        if let Some(stats) = runtime_shell.battle_level_stats.front_mut()
            && dismissed_battle_message.as_deref() == Some(stats.trigger_message.as_str())
        {
            stats.triggered = true;
            stats.active = !starts_exp_animation;
            if stats.active {
                stats.frames_before_input = 30;
            }
        }
        if !starts_exp_animation
            && runtime_shell
                .battle_fanfare_messages
                .front()
                .is_some_and(|fanfare| runtime_shell.battle_messages.front() == Some(fanfare))
        {
            if runtime_shell
                .battle_level_stats
                .front()
                .is_some_and(|stats| {
                    !stats.triggered
                        && runtime_shell.battle_messages.front()
                            == Some(&stats.trigger_message)
                })
            {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_HIT_END_OF_EXP_BAR")?;
            }
            runtime_shell.battle_fanfare_messages.pop_front();
            queue_visible_shell_sound_effect(runtime_shell, "SFX_DEX_FANFARE_50_79")?;
        }
        if starts_exp_animation
            || runtime_shell
                .battle_level_stats
                .front()
                .is_some_and(|stats| stats.active)
        {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell
            .battle_messages
            .front()
            .is_some_and(|message| message.contains("was newly added to\nthe POKéDEX."))
        {
            // NewDexDataText plays this command when its page is exposed,
            // after the caught page has been acknowledged.
            queue_visible_shell_sound_effect(runtime_shell, "SFX_SLOT_MACHINE_START")?;
        }
        let entry_messages_before = runtime_shell.battle_entry_messages_remaining;
        let starts_enemy_trainer_exit = entry_messages_before == 3
            && dismissed_battle_message
                .as_deref()
                .is_some_and(|message| message.ends_with("\nwants to battle!"));
        let starts_wild_frontpic = entry_messages_before == 2
            && dismissed_battle_message
                .as_deref()
                .is_some_and(|message| message.starts_with("Wild "));
        let starts_enemy_send_out = runtime_shell.battle_enemy_send_out_pending
            || dismissed_battle_message
                .as_deref()
                .is_some_and(visible_message_is_enemy_send_out);
        let starts_player_send_out = runtime_shell.battle_player_send_out_pending
            || dismissed_battle_message
                .as_deref()
                .is_some_and(visible_message_is_player_send_out);
        let starts_capture_animation = runtime_shell
            .visible_capture_animation
            .as_ref()
            .is_some_and(|animation| {
                !animation.started
                    && dismissed_battle_message.as_deref()
                        == Some(animation.trigger_message.as_str())
            });
        let starts_capture_pokedex_entry = dismissed_battle_message
            .as_deref()
            .is_some_and(|message| message.contains("was newly added to\nthe POKéDEX."));
        if starts_capture_animation {
            runtime_shell.visible_capture_animation.as_mut().unwrap().started = true;
            queue_visible_shell_sound_effect(runtime_shell, "SFX_THROW_BALL")?;
        }
        if starts_capture_pokedex_entry {
            let snapshot = runtime_shell.shell.presentation_snapshot()?;
            let species_id = snapshot
                .battle
                .as_ref()
                .context("capture Pokedex entry lost its battle species")?
                .enemy_pokemon
                .species
                .id
                .clone();
            let species_index = snapshot
                .pokemon
                .iter()
                .position(|species| species.species_id == species_id)
                .with_context(|| format!("captured species {species_id} is absent from the Pokedex catalog"))?;
            anyhow::ensure!(
                snapshot.presentation.pokedex_entries.contains_key(&species_id),
                "captured species {species_id} has no compiled Pokedex entry"
            );
            // NewPokedexEntry owns the LCD, but capture mutation is deferred
            // until nickname choice. Retain the cleared capture state so the
            // still-live core enemy remains hidden when the nickname prompt
            // restores the battle background; the cleared state renders no
            // capture objects.
            runtime_shell.battle_message_scene = None;
            runtime_shell.pokedex_cursor = species_index;
            runtime_shell.pokedex_menu_open = true;
            runtime_shell.pokedex_detail_open = true;
            runtime_shell.pokedex_detail_page = 0;
            runtime_shell.pokedex_scripted_entry = true;
            queue_visible_pokemon_cry(runtime_shell, &species_id, "new_pokedex_entry")?;
            set_shell_action_status(runtime_shell, format!("NEW POKEDEX ENTRY {species_id}"));
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        let starts_move_animation = runtime_shell
            .visible_move_animations
            .front()
            .is_some_and(|animation| {
                !animation.started
                    && dismissed_battle_message.as_deref()
                        == Some(animation.trigger_message.as_str())
            });
        if starts_move_animation {
            runtime_shell.visible_move_animations.front_mut().unwrap().started = true;
        }
        let starts_player_trainer_exit = starts_player_send_out && entry_messages_before == 1;
        let starts_send_out_animation = starts_enemy_send_out
            || (starts_player_send_out && !starts_player_trainer_exit);
        if starts_enemy_trainer_exit || starts_player_trainer_exit {
            runtime_shell.visible_trainer_exit_animation = Some(VisibleTrainerExitAnimation {
                side: if starts_enemy_trainer_exit {
                    crate::core::battle::turn::BattleSide::Enemy
                } else {
                    crate::core::battle::turn::BattleSide::Player
                },
                frame: 0,
                send_out_after: starts_player_trainer_exit,
            });
        }
        if starts_send_out_animation {
            if starts_enemy_send_out {
                runtime_shell.visible_frontpic_animation = None;
            }
            let side = if starts_enemy_send_out {
                crate::core::battle::turn::BattleSide::Enemy
            } else {
                crate::core::battle::turn::BattleSide::Player
            };
            let shiny = visible_send_out_side_is_shiny(runtime_shell, side)?;
            runtime_shell.visible_send_out_animation = Some(VisibleSendOutAnimation {
                side,
                frame: 0,
                shiny,
            });
            queue_visible_shell_sound_effect(runtime_shell, "SFX_BALL_POOF")?;
        }
        if starts_wild_frontpic {
            start_visible_enemy_frontpic_animation(runtime_shell, 0)?;
        }
        runtime_shell.battle_enemy_send_out_pending = false;
        runtime_shell.battle_player_send_out_pending = false;
        if !starts_send_out_animation
            && runtime_shell
                .pending_battle_cries_after_messages
                .front()
                .is_some_and(|(_, _, trigger_message)| {
                    dismissed_battle_message.as_deref() == Some(trigger_message.as_str())
                })
        {
            let (species_id, reason, _) = runtime_shell
                .pending_battle_cries_after_messages
                .pop_front()
                .unwrap();
            queue_visible_pokemon_cry(runtime_shell, &species_id, &reason)?;
        }
        if !starts_move_animation {
            if staged_scenes_aligned {
                runtime_shell.battle_message_scenes.pop_front();
                if let Some(scene) = runtime_shell.battle_message_scenes.front().cloned() {
                    retarget_visible_battle_hp_tween(runtime_shell, &scene);
                    runtime_shell.battle_message_scene = Some(scene);
                    mark_runtime_snapshot_dirty(runtime_shell);
                }
            } else {
                runtime_shell.battle_message_scenes.clear();
            }
        }
        if !starts_move_animation && let Some((trigger_message, scene)) =
            runtime_shell.pending_battle_scenes_after_message.pop_front()
        {
            if dismissed_battle_message.as_deref() == Some(trigger_message.as_str()) {
                if runtime_shell.battle_message_scenes.is_empty() {
                    retarget_visible_battle_hp_tween(runtime_shell, &scene);
                    runtime_shell.battle_message_scene = Some(scene);
                }
            } else {
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_front((trigger_message, scene));
            }
        }
        runtime_shell.battle_entry_messages_remaining = runtime_shell
            .battle_entry_messages_remaining
            .saturating_sub(1);
        if starts_enemy_trainer_exit || starts_player_trainer_exit {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if starts_move_animation {
            // TypeScript's animation player blocks the battle state machine
            // here. Keep the pre-hit scene and its queued successors intact;
            // advance_visible_move_animation releases that exact boundary.
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        let resume_enemy_after_capture = runtime_shell
            .pending_enemy_response_after_capture
            .as_ref()
            .is_some_and(|(_, trigger_message)| {
                dismissed_battle_message.as_deref() == Some(trigger_message.as_str())
            });
        if resume_enemy_after_capture {
            let (item_id, _) = runtime_shell
                .pending_enemy_response_after_capture
                .take()
                .unwrap();
            resolve_visible_battle_enemy_response_after_player_item(runtime_shell, &item_id)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell.battle_messages.is_empty() {
            runtime_shell.battle_message_scenes.clear();
            if runtime_shell.visible_blackout_phase == Some(VisibleBlackoutPhase::AwaitText) {
                runtime_shell.visible_blackout_phase = Some(VisibleBlackoutPhase::FadeOut);
                runtime_shell.screen_fade = Some(VisibleScreenFade::new(
                    ScriptFadeColor::White,
                    ScriptFadeDirection::Out,
                    8,
                ));
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            if runtime_shell.pending_standard_capture.is_some() {
                // Capture success returns with anim_keepsprites. The source
                // clears the retained ball before opening nickname handling,
                // while the caught battler stays absent. The cleared capture
                // state draws no ball and preserves that hidden-battler state
                // until core commits the capture after nickname selection.
                runtime_shell.pending_name_choice = Some(VisibleNameChoice {
                    options: vec!["YES".to_string(), "NO".to_string()],
                    selected: 0,
                });
                runtime_shell.battle_message_scene = None;
                set_shell_action_status(runtime_shell, "NICKNAME CAUGHT POKEMON");
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            if runtime_shell
                .visible_capture_animation
                .as_ref()
                .is_some_and(|animation| animation.complete)
            {
                runtime_shell.visible_capture_animation = None;
            }
            if runtime_shell.shell.presentation_snapshot()?.pending_move_learn.is_some() {
                // Move learning is a retained battle-result surface. Do not
                // expose the overworld between its announcement and the
                // delete/stop decision.
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            let automatic_move_slot = runtime_shell
                .shell
                .snapshot()?
                .battle
                .as_ref()
                .filter(|battle| battle.commands.player_turn_automatic)
                .and_then(|battle| battle.commands.player_move_slots.first().copied());
            if let Some(slot) = automatic_move_slot {
                // Recharge and locked multi-turn moves resume immediately
                // after the preceding text. The selected slot is only a
                // structurally valid input; core replaces it with the exact
                // retained move before PP/effect resolution.
                runtime_shell.battle_message_scene = None;
                mark_runtime_snapshot_dirty(runtime_shell);
                return resolve_visible_battle_move(runtime_shell, slot);
            }
            let terminal_scene = runtime_shell.battle_message_scene.is_some()
                && runtime_shell.shell.presentation_snapshot()?.battle.is_none();
            runtime_shell.battle_message_scene = None;
            let resume_trainer_settlement = runtime_shell.battle_shift_prompt_cursor.is_none()
                && runtime_shell.battle_switch_cursor.is_none()
                && runtime_shell
                    .shell
                    .snapshot()?
                    .battle
                    .as_ref()
                    .is_some_and(|battle| {
                        matches!(&battle.kind, crate::RuntimeBattleKind::Trainer { .. })
                            && battle.enemy_pokemon.hp == 0
                    });
            if terminal_scene {
                runtime_shell.battle_hp_tween = None;
                runtime_shell.battle_exp_tween = None;
                runtime_shell.pending_battle_exp_tweens.clear();
                runtime_shell.battle_fanfare_messages.clear();
                runtime_shell.battle_evolution_cries.clear();
                runtime_shell.battle_evolution_cancellations.clear();
                runtime_shell.battle_sounds_after_messages.clear();
                runtime_shell.battle_level_stats.clear();
                reset_visible_music_state(runtime_shell);
                queue_visible_current_music(runtime_shell)?;
                if runtime_shell.pending_plain_battle_map_reload {
                    begin_visible_plain_battle_map_reload(runtime_shell)?;
                }
            }
            if resume_trainer_settlement {
                return settle_visible_battle_after_action(runtime_shell);
            }
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.intro_screen.is_some() {
        return skip_visible_intro_screen(runtime_shell, GameButton::A);
    }
    if runtime_shell.credits_screen.is_some() {
        return press_visible_credits_a_button(runtime_shell);
    }
    if runtime_shell.pending_delete_save.is_some() {
        return confirm_visible_delete_save_screen(runtime_shell);
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return confirm_visible_clock_reset_screen(runtime_shell);
    }
    if runtime_shell.options_menu_open {
        return confirm_visible_options_selection(runtime_shell);
    }
    if runtime_shell.title_menu.is_some() {
        return press_visible_title_confirm_button(runtime_shell, GameButton::A);
    }
    if runtime_shell.pending_time_set.is_some() {
        return press_visible_time_set_a_button(runtime_shell);
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return press_visible_oak_intro_a_button(runtime_shell);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return confirm_visible_gender_selection(runtime_shell);
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if runtime_shell.pack_toss.is_some() {
        return confirm_visible_pack_toss(runtime_shell);
    }
    if runtime_shell.pc_item_quantity.is_some() {
        return commit_visible_pc_item_quantity(runtime_shell);
    }
    if runtime_shell.pc_confirmation.is_some() {
        let selected = strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "pc:confirmation", 2)
            .context("PC confirmation requires a valid cursor")?;
        return resolve_visible_pc_confirmation(runtime_shell, selected == 0);
    }
    if runtime_shell.party_mail_take_stage.is_some() {
        let surface = if runtime_shell.party_mail_take_stage == Some(1) {
            "party:mail-send-pc"
        } else {
            "party:mail-lose-message"
        };
        let selected = strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, surface, 2)
            .context("party Mail prompt requires a valid cursor")?;
        return resolve_visible_party_mail_take_prompt(runtime_shell, selected == 0);
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        let selected = strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "field:move-confirm",
            2,
        )
        .context("contextual field-move prompt requires a valid cursor")?;
        return resolve_visible_contextual_field_move_prompt(runtime_shell, selected == 0);
    }
    if runtime_shell.held_item_swap_prompt {
        let selected = strict_readonly_cursor_index(
            &runtime_shell.yes_no_cursor,
            "party:held-item-swap",
            2,
        )
        .context("held-item swap prompt requires a valid cursor")?;
        return resolve_visible_held_item_swap_prompt(runtime_shell, selected == 0);
    }
    // YesNoBox is a modal window layered over the field textbox. It owns A
    // before the underlying text printer/page. Handling the printer first
    // consumed the player's confirmation by advancing stale Mom text, then
    // re-entered the same DST question on the next press.
    if snapshot.ui.pending_yes_no.is_some() {
        if !visible_field_dialogue_is_fully_revealed(runtime_shell, &snapshot) {
            return Ok(());
        }
        return confirm_visible_pending_yes_no(runtime_shell);
    }
    if (runtime_shell.field_notice.is_some() || runtime_shell.pc_notice.is_some())
        && !visible_field_dialogue_is_fully_revealed(runtime_shell, &snapshot)
    {
        return Ok(());
    }
    if (runtime_shell.field_notice.is_some() || runtime_shell.pc_notice.is_some())
        && advance_visible_completed_field_text_page(runtime_shell, &snapshot)?
    {
        return Ok(());
    }
    if runtime_shell.field_notice.is_some()
        && runtime_shell.pending_field_travel_delay_frames.is_some()
    {
        return Ok(());
    }
    if runtime_shell.field_notice.is_some()
        && visible_field_notice_uses_prompt_arrow(runtime_shell)
    {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    }
    if let Some(target_species) = runtime_shell
        .field_evolution_cancellation
        .as_ref()
        .filter(|cancellation| {
            runtime_shell.field_notice.as_deref() == Some(cancellation.trigger_message.as_str())
        })
        .and_then(|cancellation| cancellation.report.target_species.clone())
    {
        runtime_shell.field_evolution_cancellation = None;
        runtime_shell.pending_field_notice_cry = Some(target_species);
    }
    if runtime_shell.field_notice.take().is_some() {
        if runtime_shell.visible_strength_notice_phase
            == Some(VisibleStrengthNoticePhase::MoveText)
        {
            runtime_shell.visible_strength_notice_phase = None;
        }
        if runtime_shell
            .visible_fishing_animation
            .is_some_and(|animation| animation.phase == VisibleFishingPhase::AwaitText)
        {
            runtime_shell.visible_fishing_animation = None;
        }
        play_pending_field_notice_sound(runtime_shell)?;
        if let Some(next) = runtime_shell.field_notice_queue.pop_front() {
            runtime_shell.field_notice = Some(next);
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        runtime_shell.pending_sweet_scent_nothing_notice = false;
        if runtime_shell.pending_tmhm_teach_prompt_after_boot {
            runtime_shell.field_notice_scene = None;
            open_visible_tmhm_teach_prompt_after_boot(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell.pending_field_travel_arrival {
            runtime_shell.pending_field_travel_arrival = false;
            if runtime_shell.visible_field_travel_animation
                == Some(VisibleFieldTravelAnimation::DigOut)
            {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_WARP_TO")?;
                begin_visible_dig_travel_animation(runtime_shell, false)?;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            settle_visible_overworld_travel(runtime_shell)?;
        }
        if begin_pending_field_notice_effect(runtime_shell) {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        runtime_shell.field_notice_scene = None;
        if settle_pending_field_battle_entry_after_notice(runtime_shell)? {
            return Ok(());
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pc_notice.is_some() {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    }
    if runtime_shell.pc_notice.take().is_some() {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    // A visible Player PC menu owns A even though the originating script's
    // text/window bookkeeping remains open underneath it.  Handling the
    // generic printer first made A silently close/advance that hidden layer
    // instead of selecting WITHDRAW, exactly matching the live stuck-PC bug.
    if runtime_shell.player_pc_action_cursor.is_some() {
        return confirm_visible_player_pc_action(runtime_shell);
    }
    // A/B accelerate the active printer to one character per frame;
    // they do not reveal a whole page atomically. Only a completed page may
    // advance the script, matching PrintLetterDelay and TypeScript main.
    if runtime_shell.field_text_reveal.is_some()
        && !visible_field_dialogue_is_fully_revealed(runtime_shell, &snapshot)
    {
        return Ok(());
    }
    if advance_visible_completed_field_text_page(runtime_shell, &snapshot)? {
        return Ok(());
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        return confirm_visible_phone_prompt(runtime_shell);
    }
    if let Some(summary) = runtime_shell.bill_pc_box_summary.as_mut() {
        summary.page = if summary.page >= 3 { 1 } else { summary.page + 1 };
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pending_pc_release.is_some() {
        return confirm_visible_pc_release_prompt(runtime_shell);
    }
    if snapshot.ui.pending_text_wait.is_some() {
        return advance_visible_pending_text_wait(runtime_shell);
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
            runtime_shell.shop_welcome_seen = true;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell.shop_notice.is_some() {
            return dismiss_visible_shop_notice(runtime_shell);
        }
        if runtime_shell.shop_quantity.is_some() {
            return confirm_visible_shop_quantity(runtime_shell);
        }
        if runtime_shell.shop_top_cursor.is_some() {
            return confirm_visible_shop_top_menu(runtime_shell);
        }
        if runtime_shell.sell_cursor.is_some() {
            return sell_selected_bag_item(runtime_shell);
        }
        return buy_visible_shop_cursor_item(runtime_shell);
    }
    if advance_visible_next_pending_script_request(runtime_shell, &snapshot)? {
        return Ok(());
    };
    if snapshot.ui.text_window_open {
        return close_visible_text_window(runtime_shell);
    }
    if snapshot.ui.window_open {
        return close_active_runtime_surface(runtime_shell);
    }
    if snapshot.ui.active_pokemon_picture.is_some() {
        return close_visible_pokemon_picture(runtime_shell);
    }
    if runtime_shell.active_script_cursor.is_some() {
        return execute_visible_active_script_step(runtime_shell);
    }
    if !snapshot.script_events.command_queue.is_empty() {
        return execute_next_visible_queued_script_command(runtime_shell);
    }
    if snapshot.script_events.next_script.is_some() {
        return take_visible_next_script(runtime_shell);
    }
    if !snapshot.script_events.deferred_scripts.is_empty() {
        return take_visible_deferred_script(runtime_shell);
    }
    if snapshot.script_events.script_ended.is_some() {
        return take_visible_script_end_state(runtime_shell);
    }
    if let Some(flag) = visible_auto_runtime_flag(&snapshot) {
        return consume_visible_runtime_flag_kind(runtime_shell, flag);
    }
    if has_visible_elevator_prompt(&snapshot, runtime_shell) {
        return select_visible_elevator_floor(runtime_shell);
    }
    if has_visible_gift_pokemon_prompt(&snapshot, runtime_shell) {
        return grant_selected_gift_pokemon(runtime_shell);
    }
    if snapshot.pending_move_learn.is_some() {
        return confirm_visible_pending_move_learn(runtime_shell);
    }
    // Full-screen menus retain input ownership even when their originating
    // battle is still authoritative underneath them. NewPokedexEntry is the
    // canonical case: its internal pages must finish before battle can resume.
    if runtime_shell.pokedex_menu_open {
        return press_visible_pokedex_a_button(runtime_shell);
    }
    if snapshot.battle.is_some() {
        if runtime_shell.battle_shift_prompt_cursor.is_some() {
            return confirm_visible_trainer_shift_prompt(runtime_shell);
        }
        return press_visible_battle_a_button(runtime_shell);
    }
    if runtime_shell.pc_item_action == Some(VisiblePlayerPcAction::DepositItem)
        && visible_field_pack_is_open(runtime_shell)
    {
        return begin_visible_pc_item_quantity(runtime_shell);
    }
    if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
        return resolve_visible_tmhm_teach_prompt(runtime_shell);
    }
    if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
        return resolve_visible_tmhm_decision_prompt(runtime_shell);
    }
    if let Some(mode) = runtime_shell.field_pack_target_mode {
        return confirm_visible_field_pack_target(runtime_shell, mode);
    }
    if runtime_shell.field_pack_action_cursor.is_some() {
        return execute_visible_field_pack_action(runtime_shell);
    }
    if runtime_shell.bag_cursor.is_some()
        || runtime_shell.key_item_cursor.is_some()
        || matches!(
            runtime_shell.field_pack_pocket.as_ref(),
            Some(FieldPackPocket::Custom(_))
        )
    {
        return open_visible_field_pack_action_menu(runtime_shell);
    }
    if runtime_shell.tmhm_cursor.is_some() {
        if selected_field_pack_cancel_row(&snapshot, runtime_shell, &FieldPackPocket::TmHm) {
            return close_visible_field_pack_from_cancel(runtime_shell);
        }
        return open_visible_field_pack_action_menu(runtime_shell);
    }
    if snapshot.battle.is_none() && runtime_shell.ball_cursor.is_some() {
        let ball_count = snapshot
            .bag
            .balls
            .iter()
            .filter(|ball| ball.quantity > 0)
            .count();
        if ball_count == 0 {
            runtime_shell.ball_cursor = None;
            record_visible_runtime_action(runtime_shell, "field:ball:no_items")?;
            runtime_shell
                .last_audio_events
                .push("bag has no carried ball".to_string());
            set_shell_action_status(runtime_shell, "NO BALLS");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        if selected_field_pack_cancel_row(&snapshot, runtime_shell, &FieldPackPocket::Balls) {
            return close_visible_field_pack_from_cancel(runtime_shell);
        }
        return open_visible_field_pack_action_menu(runtime_shell);
    }
    if runtime_shell.pokegear_menu_open {
        if runtime_shell.pokegear_page == PokegearPage::Radio {
            let Some(station) = runtime_shell.pokegear_radio_station.as_deref() else {
                return Ok(());
            };
            let segment_count = visible_map_radio_transcript(station).len();
            if runtime_shell.pokegear_radio_segment + 1 < segment_count {
                runtime_shell.pokegear_radio_segment += 1;
                runtime_shell.last_audio_events.push(format!(
                    "map radio station={station} segment={}/{}",
                    runtime_shell.pokegear_radio_segment + 1,
                    segment_count
                ));
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            record_visible_runtime_action(runtime_shell, "pokegear:radio:close")?;
            close_visible_pokegear_menu(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        return inspect_visible_pokegear_selection(runtime_shell);
    }
    if runtime_shell.options_menu_open {
        return confirm_visible_options_selection(runtime_shell);
    }
    if runtime_shell.trainer_card_open {
        return advance_visible_trainer_card(runtime_shell);
    }
    if runtime_shell.save_menu_open {
        return confirm_visible_save_menu(runtime_shell);
    }
    if runtime_shell.special_boundary.is_some() {
        return close_visible_special_boundary(runtime_shell);
    }
    if runtime_shell.party_menu_open {
        if runtime_shell.mailbox_attach_index.is_some() {
            return attach_visible_mailbox_mail(runtime_shell);
        }
        if runtime_shell.pending_script_party_selection.is_some() {
            let snapshot = runtime_shell.shell.presentation_snapshot()?;
            let party_index = snapshot
                .party
                .slots
                .get(runtime_shell.party_cursor)
                .map(|slot| slot.index);
            return resolve_visible_script_party_selection(runtime_shell, party_index);
        }
        if runtime_shell.party_hp_transfer_source.is_some() {
            return confirm_visible_party_hp_transfer_target(runtime_shell);
        }
        if runtime_shell.party_move_reorder_open {
            return confirm_visible_party_move_reorder(runtime_shell);
        }
        if runtime_shell.party_give_take_cursor.is_some() {
            return confirm_visible_party_give_take(runtime_shell);
        }
        if runtime_shell.party_summary_open {
            let snapshot = runtime_shell.shell.presentation_snapshot()?;
            let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
            if slot.pokemon.is_egg || runtime_shell.party_summary_page >= 3 {
                record_visible_runtime_action(runtime_shell, "party:summary:close")?;
                close_visible_party_summary(runtime_shell);
                continue_visible_script_after_prompt(runtime_shell)?;
                return Ok(());
            }
            return cycle_visible_party_summary_page(runtime_shell, 1);
        }
        if runtime_shell.fly_cursor.is_some() {
            return confirm_visible_fly_destination(runtime_shell);
        }
        if runtime_shell.party_switch_cursor.is_some() {
            return confirm_visible_party_switch_target(runtime_shell);
        }
        if runtime_shell.party_action_cursor.is_some() {
            return execute_visible_party_action(runtime_shell);
        }
        if runtime_shell.storage_cursor.is_some() {
            return deposit_visible_party_pokemon(runtime_shell);
        }
        return open_visible_party_action_menu(runtime_shell);
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        return confirm_visible_bill_pc_action(runtime_shell);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        return confirm_visible_player_pc_action(runtime_shell);
    }
    if runtime_shell.mailbox_action_cursor.is_some() {
        return confirm_visible_mailbox_action(runtime_shell);
    }
    if runtime_shell.mailbox_cursor.is_some() {
        return confirm_visible_mailbox_selection(runtime_shell);
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        return confirm_visible_bill_pc_box(runtime_shell);
    }
    if runtime_shell.bill_pc_pokemon_action_cursor.is_some() {
        return confirm_visible_bill_pc_pokemon_action(runtime_shell);
    }
    if runtime_shell.pc_hub_cursor.is_some() {
        return confirm_visible_pc_hub(runtime_shell);
    }
    if runtime_shell.storage_cursor.is_some() {
        if runtime_shell.bill_pc_move_open {
            return confirm_visible_bill_pc_move(runtime_shell);
        }
        return open_visible_bill_pc_pokemon_actions(runtime_shell);
    }
    if runtime_shell.pc_item_cursor.is_some() {
        return begin_visible_pc_item_quantity(runtime_shell);
    }
    if runtime_shell.start_menu_cursor.is_some() {
        return select_visible_start_menu_option(runtime_shell);
    }
    if visible_menu_has_selectable_options(&snapshot) {
        return select_visible_menu_cursor_option(runtime_shell);
    }
    if !snapshot.script_events.audio_events.is_empty() {
        return drain_visible_audio_events(runtime_shell);
    }
    if has_visible_pending_non_audio_script_events(&snapshot) {
        return drain_visible_non_audio_script_events(runtime_shell);
    }
    if runtime_shell.active_script_cursor.is_some() {
        return execute_visible_active_script_step(runtime_shell);
    }
    if execute_visible_contextual_field_move(runtime_shell)? {
        return Ok(());
    }
    if runtime_shell
        .shell
        .last_frame()
        .and_then(|frame| frame.interaction.as_ref())
        .is_some()
    {
        return execute_last_interaction_script(runtime_shell);
    }
    if runtime_shell
        .shell
        .current_overworld_interaction_checked()?
        .is_some()
    {
        return execute_current_overworld_interaction_script(runtime_shell);
    }
    Ok(())
}

fn has_visible_shell_a_action(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    if runtime_shell.player_walk_frame_ticks > 0 {
        return Ok(false);
    }
    if !runtime_shell.battle_messages.is_empty()
        || runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active)
    {
        return Ok(true);
    }
    // These text surfaces are owned entirely by the Bevy presentation shell;
    // the authoritative snapshot need not have `ui.window_open` set. Their
    // A/B handlers already implement reveal, queue, prompt, and travel
    // continuation, so route the physical button to that visible owner.
    if runtime_shell.field_notice.is_some() || runtime_shell.pc_notice.is_some() {
        return Ok(true);
    }
    if runtime_shell.visible_diploma.is_some() {
        return Ok(true);
    }
    if runtime_shell.visible_unown_words.is_some() {
        return Ok(true);
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
    {
        return Ok(true);
    }
    if runtime_shell.visible_card_flip.is_some() {
        return Ok(true);
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return Ok(true);
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return Ok(true);
    }
    if runtime_shell.visible_mom_bank.is_some() {
        return Ok(true);
    }
    if runtime_shell.pending_day_of_week.is_some() {
        return Ok(true);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        return Ok(true);
    }
    if runtime_shell.buena_prize_cursor.is_some() {
        return Ok(true);
    }
    if runtime_shell.intro_screen.is_some() {
        return Ok(true);
    }
    if runtime_shell.credits_screen.is_some() {
        return Ok(true);
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        return Ok(true);
    }
    if runtime_shell.title_menu.is_some() {
        return Ok(true);
    }
    if runtime_shell.pending_time_set.is_some() {
        return Ok(true);
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return Ok(true);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return Ok(true);
    }
    // Input ownership must never be decided from the presentation cache. A
    // room callback can drain its script events after the last rendered
    // snapshot; treating that stale snapshot as modal steals the player's
    // next A press from the authoritative overworld interaction transaction.
    // This path is evaluated for physical input routing, not bitmap rendering.
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if snapshot.ui.pending_yes_no.is_some()
        || runtime_shell.pending_phone_prompt.is_some()
        || snapshot.ui.pending_text_wait.is_some()
        || snapshot.pending_move_learn.is_some()
        || snapshot.pending_shop.is_some()
        || snapshot.script_events.pending_text_label.is_some()
        || snapshot.script_events.pending_map_load.is_some()
        || snapshot.script_events.pending_map_refresh.is_some()
        || snapshot.script_events.pending_music_fade.is_some()
        || snapshot.script_events.pending_screen_fade.is_some()
        || !snapshot.script_events.pending_delays.is_empty()
        || !snapshot.script_events.pending_earthquakes.is_empty()
        || !snapshot.script_events.pending_emotes.is_empty()
        || snapshot.ui.text_window_open
        || snapshot.ui.window_open
        || snapshot.ui.active_pokemon_picture.is_some()
        || snapshot.script_events.pending_script_warp.is_some()
        || !snapshot.script_events.command_queue.is_empty()
        || snapshot.script_events.next_script.is_some()
        || !snapshot.script_events.deferred_scripts.is_empty()
        || snapshot.script_events.script_ended.is_some()
        || !snapshot.script_events.audio_events.is_empty()
        || has_visible_pending_non_audio_script_events(&snapshot)
        || visible_auto_runtime_flag(&snapshot).is_some()
        || has_visible_elevator_prompt(&snapshot, runtime_shell)
        || has_visible_gift_pokemon_prompt(&snapshot, runtime_shell)
        || runtime_shell.active_script_cursor.is_some()
        || runtime_shell.bag_cursor.is_some()
        || runtime_shell.key_item_cursor.is_some()
        || runtime_shell.ball_cursor.is_some()
        || runtime_shell.tmhm_cursor.is_some()
        || runtime_shell.custom_item_cursor.is_some()
        || runtime_shell.field_pack_target_mode.is_some()
        || runtime_shell.storage_cursor.is_some()
        || runtime_shell.pc_item_cursor.is_some()
        || runtime_shell.pokedex_menu_open
        || runtime_shell.pokegear_menu_open
        || runtime_shell.trainer_card_open
        || runtime_shell.options_menu_open
        || runtime_shell.save_menu_open
        || runtime_shell.special_boundary.is_some()
        || runtime_shell.party_menu_open
        || runtime_shell.start_menu_cursor.is_some()
        || visible_menu_has_selectable_options(&snapshot)
        || snapshot.battle.is_some()
    {
        return Ok(true);
    }
    // Ordinary map/NPC collisions belong to the authoritative overworld
    // joypad transaction. Claiming them as Bevy-shell A actions prevents the
    // same frame from ever reaching `execute_interaction_script`, producing a
    // sound/no-dialogue no-op after walking. Only modal surfaces above own A.
    Ok(false)
}

fn press_visible_pokedex_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pokedex_detail_open {
        let snapshot = runtime_shell.shell.presentation_snapshot()?;
        let species = selected_pokedex_catalog_species(&snapshot, runtime_shell.pokedex_cursor)?;
        let page_count = snapshot
            .presentation
            .pokedex_entries
            .get(&species.species_id)
            .map(|entry| entry.pages.len().max(1))
            .unwrap_or(1);
        if runtime_shell.pokedex_scripted_entry
            && runtime_shell.pokedex_detail_page + 1 >= page_count
        {
            record_visible_runtime_action(runtime_shell, "pokedex:scripted_entry:close")?;
            close_visible_pokedex_menu(runtime_shell);
            if runtime_shell.pending_standard_capture.is_some() {
                runtime_shell.pending_name_choice = Some(VisibleNameChoice {
                    options: vec!["YES".to_string(), "NO".to_string()],
                    selected: 0,
                });
                set_shell_action_status(runtime_shell, "NICKNAME CAUGHT POKEMON");
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        runtime_shell.pokedex_detail_page =
            (runtime_shell.pokedex_detail_page + 1) % page_count;
        let page_number = runtime_shell.pokedex_detail_page + 1;
        record_visible_runtime_action(
            runtime_shell,
            format!("pokedex:detail:page:{page_number}"),
        )?;
        return Ok(());
    }
    inspect_visible_pokedex_selection(runtime_shell)
}

fn press_visible_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_diploma.is_some() {
        return close_visible_diploma(runtime_shell);
    }
    if runtime_shell.visible_unown_words.is_some() {
        return close_visible_unown_words(runtime_shell);
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
    {
        return Ok(());
    }
    if runtime_shell
        .battle_exp_tween
        .as_ref()
        .is_some_and(|tween| tween.started)
    {
        return Ok(());
    }
    if let Some(stats) = runtime_shell.battle_level_stats.front()
        && stats.active
    {
        if stats.frames_before_input == 0 {
            runtime_shell.battle_level_stats.pop_front();
            mark_runtime_snapshot_dirty(runtime_shell);
            finish_visible_empty_battle_reward_presentation(runtime_shell)?;
        }
        return Ok(());
    }
    if runtime_shell.visible_card_flip.is_some() {
        return close_visible_card_flip(runtime_shell);
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return close_visible_slot_machine(runtime_shell);
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return close_visible_unown_puzzle(runtime_shell);
    }
    if runtime_shell.visible_mom_bank.is_some() {
        return cancel_visible_mom_bank(runtime_shell);
    }
    if let Some(prompt) = runtime_shell.pending_day_of_week.as_mut() {
        if prompt.confirming {
            prompt.confirming = false;
            prompt.yes_no_index = 0;
            set_shell_action_status(runtime_shell, "WHAT DAY IS IT?");
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(());
    }
    if !runtime_shell.battle_messages.is_empty() {
        if cancel_visible_battle_evolution(runtime_shell)? {
            return Ok(());
        }
        return press_visible_a_button(runtime_shell);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        if runtime_shell.kurt_apricorn_quantity.take().is_some() {
            set_shell_action_status(runtime_shell, "WHICH APRICORN?");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        return resolve_visible_kurt_apricorn_selection(runtime_shell, true);
    }
    if runtime_shell.buena_prize_cursor.is_some()
        && runtime_shell.pc_confirmation.is_none()
        && runtime_shell.pc_notice.is_none()
    {
        return resolve_visible_buena_prize_selection(runtime_shell, true);
    }
    if runtime_shell.intro_screen.is_some() {
        return skip_visible_intro_screen(runtime_shell, GameButton::B);
    }
    if runtime_shell.credits_screen.is_some() {
        return press_visible_credits_b_button(runtime_shell);
    }
    if runtime_shell.pending_delete_save.is_some() {
        return close_visible_delete_save_screen(runtime_shell, "cancel");
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return close_visible_clock_reset_screen(runtime_shell, "cancel");
    }
    if runtime_shell.options_menu_open {
        record_visible_runtime_action(runtime_shell, "options:close")?;
        close_visible_options_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.title_menu.is_some() {
        return Ok(());
    }
    if runtime_shell.pending_time_set.is_some() {
        return press_visible_time_set_b_button(runtime_shell);
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return press_visible_oak_intro_b_button(runtime_shell);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        record_visible_runtime_action(runtime_shell, "gender:b:ignored")?;
        runtime_shell
            .last_audio_events
            .push("gender B ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.save_menu_open {
        return cancel_visible_save_menu(runtime_shell);
    }
    if runtime_shell.pack_toss.is_some() {
        return cancel_visible_pack_toss(runtime_shell);
    }
    if runtime_shell.pc_item_quantity.take().is_some() {
        runtime_shell.pc_notice = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pc_confirmation.is_some() {
        return resolve_visible_pc_confirmation(runtime_shell, false);
    }
    if runtime_shell.party_mail_take_stage.is_some() {
        return resolve_visible_party_mail_take_prompt(runtime_shell, false);
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        return resolve_visible_contextual_field_move_prompt(runtime_shell, false);
    }
    if runtime_shell.held_item_swap_prompt {
        return resolve_visible_held_item_swap_prompt(runtime_shell, false);
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if (runtime_shell.field_notice.is_some() || runtime_shell.pc_notice.is_some())
        && !visible_field_dialogue_is_fully_revealed(runtime_shell, &snapshot)
    {
        return Ok(());
    }
    if (runtime_shell.field_notice.is_some() || runtime_shell.pc_notice.is_some())
        && advance_visible_completed_field_text_page(runtime_shell, &snapshot)?
    {
        return Ok(());
    }
    if runtime_shell.field_notice.is_some()
        && runtime_shell.pending_field_travel_delay_frames.is_some()
    {
        return Ok(());
    }
    if cancel_visible_field_evolution(runtime_shell)? {
        return Ok(());
    }
    if runtime_shell.field_notice.is_some()
        && visible_field_notice_uses_prompt_arrow(runtime_shell)
    {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    }
    if runtime_shell.field_notice.take().is_some() {
        if runtime_shell.visible_strength_notice_phase
            == Some(VisibleStrengthNoticePhase::MoveText)
        {
            runtime_shell.visible_strength_notice_phase = None;
        }
        if runtime_shell
            .visible_fishing_animation
            .is_some_and(|animation| animation.phase == VisibleFishingPhase::AwaitText)
        {
            runtime_shell.visible_fishing_animation = None;
        }
        play_pending_field_notice_sound(runtime_shell)?;
        if let Some(next) = runtime_shell.field_notice_queue.pop_front() {
            runtime_shell.field_notice = Some(next);
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        runtime_shell.pending_sweet_scent_nothing_notice = false;
        if runtime_shell.pending_tmhm_teach_prompt_after_boot {
            runtime_shell.field_notice_scene = None;
            open_visible_tmhm_teach_prompt_after_boot(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell.pending_field_travel_arrival {
            runtime_shell.pending_field_travel_arrival = false;
            if runtime_shell.visible_field_travel_animation
                == Some(VisibleFieldTravelAnimation::DigOut)
            {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_WARP_TO")?;
                begin_visible_dig_travel_animation(runtime_shell, false)?;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            settle_visible_overworld_travel(runtime_shell)?;
        }
        if begin_pending_field_notice_effect(runtime_shell) {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        runtime_shell.field_notice_scene = None;
        if settle_pending_field_battle_entry_after_notice(runtime_shell)? {
            return Ok(());
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pc_notice.is_some() {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    }
    if runtime_shell.pc_notice.take().is_some() {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if snapshot.pending_move_learn.is_some() {
        return cancel_visible_pending_move_learn(runtime_shell);
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        return decline_visible_phone_prompt(runtime_shell);
    }
    if runtime_shell.bill_pc_box_summary.take().is_some() {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pending_pc_release.take().is_some() {
        runtime_shell.yes_no_cursor = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.bill_pc_pokemon_action_cursor.take().is_some() {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if snapshot.ui.pending_yes_no.is_some() {
        return decline_visible_pending_yes_no(runtime_shell);
    }
    if runtime_shell.pokegear_menu_open {
        record_visible_runtime_action(runtime_shell, "pokegear:close")?;
        // OverworldTownMap retains its originating textbox and core menu
        // beneath the modal. B belongs to the map UI first; closing it then
        // resumes the script at `closetext`/`end`.
        if snapshot.ui.menu.is_some() {
            let _ = runtime_shell.shell.close_active_menu()?;
        }
        close_visible_pokegear_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.field_text_reveal.is_some() {
        // JoyTextDelay, JoyWaitAorB, and PromptButton all accept PAD_A or
        // PAD_B. Higher-priority YES/NO, selection, and cancelable modal
        // surfaces have already handled B above this ordinary text boundary.
        return press_visible_a_button(runtime_shell);
    }
    if snapshot.ui.pending_text_wait.is_some() {
        return advance_visible_pending_text_wait(runtime_shell);
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
            runtime_shell.shop_welcome_seen = true;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell.shop_notice.is_some() {
            return dismiss_visible_shop_notice(runtime_shell);
        }
        if runtime_shell.shop_quantity.take().is_some() {
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        if runtime_shell.shop_top_cursor.is_none() {
            runtime_shell.sell_cursor = None;
            runtime_shell.shop_top_cursor = Some(MenuCursor {
                surface_id: "shop:top".to_string(),
                option_index: 0,
            });
            runtime_shell.shop_notice = Some("Can I do anything\nelse for you?".to_string());
            runtime_shell
                .last_audio_events
                .push("returned to shop top menu".to_string());
            trim_event_log(&mut runtime_shell.last_audio_events);
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        return close_visible_shop(runtime_shell);
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        return close_visible_bill_pc_actions(runtime_shell);
    }
    if runtime_shell.mailbox_action_cursor.is_some() {
        runtime_shell.mailbox_action_cursor = None;
        return Ok(());
    }
    if runtime_shell.mailbox_cursor.is_some() {
        runtime_shell.mailbox_cursor = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
        return Ok(());
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        return close_visible_player_pc(runtime_shell);
    }
    if runtime_shell.pc_item_cursor.is_some() && runtime_shell.pc_item_action.is_some() {
        let action_index = match runtime_shell.pc_item_action {
            Some(VisiblePlayerPcAction::TossItem) => 2,
            _ => 0,
        };
        runtime_shell.pc_item_cursor = None;
        runtime_shell.pc_item_action = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: action_index });
        return Ok(());
    }
    if runtime_shell.pc_item_action == Some(VisiblePlayerPcAction::DepositItem)
        && visible_field_pack_is_open(runtime_shell)
    {
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.pc_item_action = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 1 });
        return Ok(());
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        runtime_shell.bill_pc_box_cursor = None;
        runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
            surface_id: "pc:bill-actions".to_string(),
            option_index: 2,
        });
        set_shell_action_status(runtime_shell, "BILL'S PC");
        return Ok(());
    }
    if runtime_shell.bill_pc_move_open && runtime_shell.bill_pc_move_source.is_some() {
        runtime_shell.bill_pc_move_source = None;
        set_shell_action_status(runtime_shell, "CHOOSE A POKEMON TO MOVE");
        return Ok(());
    }
    if runtime_shell.pc_hub_cursor.is_some() {
        return turn_off_visible_pc_hub(runtime_shell);
    }
    if runtime_shell.bill_pc_session_open
        && (runtime_shell.storage_cursor.is_some() || runtime_shell.pc_item_cursor.is_some())
    {
        return close_visible_pc_surface(runtime_shell);
    }
    if !runtime_shell.pokegear_menu_open
        && (snapshot.ui.text_window_open
            || snapshot.ui.window_open
            || snapshot.ui.menu.is_some()
            || snapshot.ui.active_pokemon_picture.is_some())
    {
        if snapshot
            .ui
            .menu
            .as_ref()
            .is_some_and(|menu| menu.menu_2d_requested)
        {
            return cancel_visible_2d_menu(runtime_shell);
        }
        return close_active_runtime_surface(runtime_shell);
    }
    if snapshot.battle.is_none() && visible_field_pack_is_open(runtime_shell) {
        if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
            runtime_shell.tmhm_teach_prompt_cursor = None;
            record_visible_runtime_action(runtime_shell, "pack:tmhm:teach:b")?;
            runtime_shell.field_notice = Some("The TM wasn't used.".to_string());
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "THE TM WASN'T USED");
            return Ok(());
        }
        if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
            runtime_shell.tmhm_decision_prompt_cursor = Some(MenuCursor {
                surface_id: "pack:tmhm:decision".to_string(),
                option_index: 1,
            });
            return resolve_visible_tmhm_decision_prompt(runtime_shell);
        }
        if runtime_shell.tmhm_forget_menu_open {
            runtime_shell.tmhm_forget_menu_open = false;
            runtime_shell.party_move_cursor = None;
            return open_visible_tmhm_decision_prompt(
                runtime_shell,
                VisibleTmHmDecision::StopLearning,
            );
        }
        if runtime_shell.field_pack_target_mode.is_some() {
            close_visible_field_pack_target(runtime_shell)?;
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        if runtime_shell.field_pack_action_cursor.is_some() {
            record_visible_runtime_action(runtime_shell, "pack:actions:close")?;
            close_visible_field_pack_action_menu(runtime_shell);
            return Ok(());
        }
        record_visible_runtime_action(runtime_shell, "pack:close")?;
        runtime_shell.bag_cursor = None;
        runtime_shell.key_item_cursor = None;
        runtime_shell.ball_cursor = None;
        runtime_shell.tmhm_cursor = None;
        runtime_shell.custom_item_cursor = None;
        runtime_shell.field_pack_action_cursor = None;
        runtime_shell.field_pack_pocket = None;
        runtime_shell.field_pack_target_mode = None;
        runtime_shell
            .last_audio_events
            .push("closed field item cursor".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.party_menu_open {
        if runtime_shell.mailbox_attach_index.take().is_some() {
            close_visible_party_menu(runtime_shell);
            runtime_shell.mailbox_cursor = Some(MenuCursor { surface_id: "pc:mailbox".to_string(), option_index: 0 });
            return Ok(());
        }
        if runtime_shell.pending_script_party_selection.is_some() {
            return resolve_visible_script_party_selection(runtime_shell, None);
        }
        if runtime_shell.party_hp_transfer_source.is_some() {
            return cancel_visible_party_hp_transfer_target(runtime_shell);
        }
        if runtime_shell.bill_pc_session_open && runtime_shell.storage_cursor.is_some() {
            return close_visible_pc_surface(runtime_shell);
        }
        if runtime_shell.party_move_reorder_open {
            if let Some(origin) = runtime_shell.party_move_reorder_origin.take() {
                let party_index = selected_party_index(runtime_shell)?;
                runtime_shell.party_move_cursor = Some(MenuCursor {
                    surface_id: party_move_reorder_surface_id(party_index),
                    option_index: origin,
                });
                set_shell_action_status(runtime_shell, "MOVE WHERE?");
            } else {
                record_visible_runtime_action(runtime_shell, "party:move_reorder:close")?;
                close_visible_party_move_reorder(runtime_shell);
                set_shell_action_status(runtime_shell, "POKEMON");
            }
            return Ok(());
        }
        if runtime_shell.party_give_take_cursor.is_some() {
            runtime_shell.party_give_take_cursor = None;
            set_shell_action_status(runtime_shell, "POKEMON");
            return Ok(());
        }
        if runtime_shell.party_summary_open {
            record_visible_runtime_action(runtime_shell, "party:summary:close")?;
            close_visible_party_summary(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        if runtime_shell.fly_cursor.is_some() {
            record_visible_runtime_action(runtime_shell, "party:fly:close")?;
            runtime_shell.fly_cursor = None;
            runtime_shell
                .last_audio_events
                .push("closed Fly destinations".to_string());
            trim_event_log(&mut runtime_shell.last_audio_events);
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        if runtime_shell.party_switch_cursor.is_some() {
            record_visible_runtime_action(runtime_shell, "party:switch:close")?;
            runtime_shell.party_switch_cursor = None;
            runtime_shell
                .last_audio_events
                .push("closed party switch".to_string());
            trim_event_log(&mut runtime_shell.last_audio_events);
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        if runtime_shell.party_action_cursor.is_some() {
            record_visible_runtime_action(runtime_shell, "party:actions:close")?;
            close_visible_party_action_menu(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        record_visible_runtime_action(runtime_shell, "party:close")?;
        close_visible_party_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.pokedex_menu_open {
        if runtime_shell.pokedex_detail_open {
            record_visible_runtime_action(runtime_shell, "pokedex:detail:close")?;
            if runtime_shell.pokedex_scripted_entry {
                let snapshot = runtime_shell.shell.presentation_snapshot()?;
                let species =
                    selected_pokedex_catalog_species(&snapshot, runtime_shell.pokedex_cursor)?;
                let page_count = snapshot
                    .presentation
                    .pokedex_entries
                    .get(&species.species_id)
                    .map(|entry| entry.pages.len().max(1))
                    .unwrap_or(1);
                if runtime_shell.pokedex_detail_page + 1 < page_count {
                    runtime_shell.pokedex_detail_page += 1;
                    let page_number = runtime_shell.pokedex_detail_page + 1;
                    record_visible_runtime_action(
                        runtime_shell,
                        format!("pokedex:scripted_entry:page:{page_number}:b"),
                    )?;
                    mark_runtime_snapshot_dirty(runtime_shell);
                    return Ok(());
                }
                close_visible_pokedex_menu(runtime_shell);
                if runtime_shell.pending_standard_capture.is_some() {
                    runtime_shell.pending_name_choice = Some(VisibleNameChoice {
                        options: vec!["YES".to_string(), "NO".to_string()],
                        selected: 0,
                    });
                    set_shell_action_status(runtime_shell, "NICKNAME CAUGHT POKEMON");
                    mark_runtime_snapshot_dirty(runtime_shell);
                    return Ok(());
                }
                continue_visible_script_after_prompt(runtime_shell)?;
                return Ok(());
            }
            close_visible_pokedex_detail(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        record_visible_runtime_action(runtime_shell, "pokedex:close")?;
        close_visible_pokedex_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.options_menu_open {
        record_visible_runtime_action(runtime_shell, "options:close")?;
        close_visible_options_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.trainer_card_open {
        record_visible_runtime_action(runtime_shell, "trainer_card:close")?;
        close_visible_trainer_card(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.save_menu_open {
        return cancel_visible_save_menu(runtime_shell);
    }
    if runtime_shell.special_boundary.is_some() {
        close_visible_special_boundary(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.start_menu_cursor.is_some() {
        record_visible_runtime_action(runtime_shell, "start_menu:close")?;
        close_visible_start_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        return close_visible_bill_pc_actions(runtime_shell);
    }
    if runtime_shell.mailbox_action_cursor.is_some() {
        runtime_shell.mailbox_action_cursor = None;
        return Ok(());
    }
    if runtime_shell.mailbox_cursor.is_some() {
        runtime_shell.mailbox_cursor = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
        return Ok(());
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        return close_visible_player_pc(runtime_shell);
    }
    if runtime_shell.pc_item_cursor.is_some() && runtime_shell.pc_item_action.is_some() {
        let action_index = match runtime_shell.pc_item_action {
            Some(VisiblePlayerPcAction::TossItem) => 2,
            _ => 0,
        };
        runtime_shell.pc_item_cursor = None;
        runtime_shell.pc_item_action = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: action_index });
        return Ok(());
    }
    if runtime_shell.pc_item_action == Some(VisiblePlayerPcAction::DepositItem)
        && visible_field_pack_is_open(runtime_shell)
    {
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.pc_item_action = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 1 });
        return Ok(());
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        runtime_shell.bill_pc_box_cursor = None;
        runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
            surface_id: "pc:bill-actions".to_string(),
            option_index: 2,
        });
        set_shell_action_status(runtime_shell, "BILL'S PC");
        return Ok(());
    }
    if runtime_shell.pc_hub_cursor.is_some() {
        return turn_off_visible_pc_hub(runtime_shell);
    }
    if runtime_shell.storage_cursor.is_some() || runtime_shell.pc_item_cursor.is_some() {
        close_visible_pc_surface(runtime_shell)?;
        return Ok(());
    }
    if snapshot.battle.is_some() && runtime_shell.battle_pack_target_mode.is_some() {
        close_visible_battle_pack_target(runtime_shell)?;
        return Ok(());
    }
    if snapshot.battle.is_some() && runtime_shell.field_pack_action_cursor.is_some() {
        record_visible_runtime_action(runtime_shell, "battle:pack:actions:close")?;
        close_visible_field_pack_action_menu(runtime_shell);
        return Ok(());
    }
    if snapshot.battle.is_some()
        && (runtime_shell.ball_cursor.is_some()
            || runtime_shell.bag_cursor.is_some()
            || runtime_shell.key_item_cursor.is_some()
            || runtime_shell.tmhm_cursor.is_some())
    {
        record_visible_runtime_action(runtime_shell, "battle:item_menu:close")?;
        reset_visible_battle_item_cursors(runtime_shell);
        runtime_shell
            .last_audio_events
            .push("closed battle item cursor".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if snapshot.battle.is_some()
        && (runtime_shell.battle_move_cursor.is_some()
            || runtime_shell.battle_switch_cursor.is_some())
    {
        if runtime_shell.battle_party_summary_open
            || runtime_shell.battle_party_action_cursor.is_some()
        {
            return press_visible_battle_b_button(runtime_shell);
        }
        // MoveSelectionScreen returns through ParsePlayerAction's
        // PlayClickSFX even when B canceled it. The party menu likewise owns
        // the ordinary menu-button click. Keep the disabled B input on the
        // main battle command grid silent by limiting this to open submenus.
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
        record_visible_runtime_action(runtime_shell, "battle:submenu:reset")?;
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_move_swap_origin = None;
        runtime_shell.battle_shift_prompt_cursor = None;
        runtime_shell.battle_faint_prompt_cursor = None;
        runtime_shell.battle_switch_cursor = None;
        runtime_shell
            .last_audio_events
            .push("reset battle action cursor".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if snapshot.battle.is_some() {
        if runtime_shell.battle_shift_prompt_cursor.is_some() {
            return resolve_visible_trainer_shift_prompt(runtime_shell, false);
        }
        return press_visible_battle_b_button(runtime_shell);
    }
    Ok(())
}

fn confirm_visible_day_of_week(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(prompt) = runtime_shell.pending_day_of_week.as_mut() else {
        return Ok(());
    };
    if !prompt.confirming {
        prompt.confirming = true;
        prompt.yes_no_index = 0;
        set_shell_action_status(runtime_shell, "IS IT?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if prompt.yes_no_index != 0 {
        prompt.confirming = false;
        prompt.yes_no_index = 0;
        set_shell_action_status(runtime_shell, "WHAT DAY IS IT?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let prompt = runtime_shell
        .pending_day_of_week
        .clone()
        .context("weekday prompt disappeared before confirmation")?;
    runtime_shell
        .shell
        .set_script_runtime_variable("wTempDayOfWeek", prompt.selected_day.to_string())?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:day_of_week:{}:{}:{}",
            prompt.source_script, prompt.command_index, prompt.selected_day
        ),
    )?;
    let runtime_inputs = explicit_compiled_script_runtime_inputs(
        runtime_shell,
        &prompt.source_script,
        prompt.command_index,
    )?;
    let phone_inputs = explicit_compiled_script_phone_inputs(
        runtime_shell,
        &prompt.source_script,
        prompt.command_index,
    );
    let stepped = runtime_shell.shell.step_compiled_script_command(
        &prompt.origin_map_name,
        &prompt.source_script,
        prompt.command_index,
        runtime_inputs,
        phone_inputs,
    )?;
    integrate_visible_script_mutation_outcome(runtime_shell, &stepped.mutation)?;
    runtime_shell.pending_day_of_week = None;
    trim_event_log(&mut runtime_shell.last_audio_events);
    if activate_visible_script_boundary_after_outcome(runtime_shell, &stepped.mutation)? {
        arm_visible_active_script_cursor_from_run(runtime_shell, stepped.next_cursor);
        return Ok(());
    }
    arm_visible_script_cursor_after_step(runtime_shell, &stepped);
    continue_visible_script_after_prompt(runtime_shell)
}

fn finish_visible_mom_bank(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.visible_mom_bank = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn queue_visible_mom_bank_messages(
    runtime_shell: &mut BevyRuntimeShell,
    messages: &[&str],
    close_after: bool,
) {
    if let Some(bank) = runtime_shell.visible_mom_bank.as_mut() {
        bank.messages = messages.iter().map(|message| (*message).to_string()).collect();
        bank.close_after_messages = close_after;
    }
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn confirm_visible_mom_bank(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(bank) = runtime_shell.visible_mom_bank.as_mut() else {
        return Ok(());
    };
    if !bank.messages.is_empty() {
        bank.messages.pop_front();
        let finished = bank.messages.is_empty() && bank.close_after_messages;
        mark_runtime_snapshot_dirty(runtime_shell);
        return if finished {
            finish_visible_mom_bank(runtime_shell)
        } else {
            Ok(())
        };
    }
    let phase = bank.phase;
    let accepted = bank.yes_no_index == 0;
    let menu_index = bank.menu_index;
    let amount = bank.amount;
    record_visible_runtime_action(runtime_shell, format!("mom_bank:{phase:?}:confirm"))?;
    match phase {
        VisibleMomBankPhase::InitializeQuestion => {
            runtime_shell.shell.session_mut().state_mut().mom_saving_some_money = accepted;
            queue_visible_mom_bank_messages(
                runtime_shell,
                if accepted {
                    &["OK, I'll take care of your money.", "Be careful.\nNow, go on!"]
                } else {
                    &["Be careful.\nNow, go on!"]
                },
                true,
            );
        }
        VisibleMomBankPhase::AccessQuestion => {
            if accepted {
                let bank = runtime_shell.visible_mom_bank.as_mut().unwrap();
                bank.phase = VisibleMomBankPhase::Menu;
                bank.menu_index = 0;
                mark_runtime_snapshot_dirty(runtime_shell);
            } else {
                queue_visible_mom_bank_messages(runtime_shell, &["Just do what you can."], true);
            }
        }
        VisibleMomBankPhase::Menu => match menu_index {
            0 => {
                let bank = runtime_shell.visible_mom_bank.as_mut().unwrap();
                bank.phase = VisibleMomBankPhase::Withdraw;
                bank.amount = 0;
                bank.digit = 5;
                queue_visible_mom_bank_messages(
                    runtime_shell,
                    &["How much do you want to take?"],
                    false,
                );
            }
            1 => {
                let bank = runtime_shell.visible_mom_bank.as_mut().unwrap();
                bank.phase = VisibleMomBankPhase::Deposit;
                bank.amount = 0;
                bank.digit = 5;
                queue_visible_mom_bank_messages(
                    runtime_shell,
                    &["How much do you want to save?"],
                    false,
                );
            }
            2 => {
                let bank = runtime_shell.visible_mom_bank.as_mut().unwrap();
                bank.phase = VisibleMomBankPhase::ChangeQuestion;
                bank.yes_no_index = 0;
                mark_runtime_snapshot_dirty(runtime_shell);
            }
            _ => queue_visible_mom_bank_messages(
                runtime_shell,
                &["Just do what you can."],
                true,
            ),
        },
        VisibleMomBankPhase::Withdraw | VisibleMomBankPhase::Deposit => {
            if amount == 0 {
                queue_visible_mom_bank_messages(runtime_shell, &["Just do what you can."], true);
                return Ok(());
            }
            const MAX_MONEY: u32 = 999_999;
            let state = runtime_shell.shell.session_mut().state_mut();
            let (available, destination) = if phase == VisibleMomBankPhase::Withdraw {
                (state.moms_money, state.money)
            } else {
                (state.money, state.moms_money)
            };
            if available < amount {
                queue_visible_mom_bank_messages(
                    runtime_shell,
                    if phase == VisibleMomBankPhase::Withdraw {
                        &["You haven't saved that much."]
                    } else {
                        &["You don't have that much."]
                    },
                    false,
                );
                return Ok(());
            }
            if destination > MAX_MONEY - amount {
                queue_visible_mom_bank_messages(
                    runtime_shell,
                    if phase == VisibleMomBankPhase::Withdraw {
                        &["You can't take that much."]
                    } else {
                        &["You can't save that much."]
                    },
                    false,
                );
                return Ok(());
            }
            if phase == VisibleMomBankPhase::Withdraw {
                state.moms_money -= amount;
                state.money += amount;
                queue_visible_mom_bank_messages(runtime_shell, &["Here you go!"], true);
            } else {
                state.money -= amount;
                state.moms_money += amount;
                queue_visible_mom_bank_messages(runtime_shell, &["OK, I'll save your money."], true);
            }
        }
        VisibleMomBankPhase::ChangeQuestion => {
            runtime_shell.shell.session_mut().state_mut().mom_saving_some_money = accepted;
            queue_visible_mom_bank_messages(
                runtime_shell,
                if accepted {
                    &["OK, I'll save your money."]
                } else {
                    &["Just do what you can."]
                },
                true,
            );
        }
    }
    Ok(())
}

fn cancel_visible_mom_bank(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(bank) = runtime_shell.visible_mom_bank.as_ref() else {
        return Ok(());
    };
    if !bank.messages.is_empty() {
        return confirm_visible_mom_bank(runtime_shell);
    }
    match bank.phase {
        VisibleMomBankPhase::InitializeQuestion
        | VisibleMomBankPhase::AccessQuestion
        | VisibleMomBankPhase::ChangeQuestion => {
            runtime_shell.visible_mom_bank.as_mut().unwrap().yes_no_index = 1;
            confirm_visible_mom_bank(runtime_shell)
        }
        VisibleMomBankPhase::Menu
        | VisibleMomBankPhase::Withdraw
        | VisibleMomBankPhase::Deposit => {
            queue_visible_mom_bank_messages(runtime_shell, &["Just do what you can."], true);
            Ok(())
        }
    }
}

fn move_visible_mom_bank(runtime_shell: &mut BevyRuntimeShell, delta: isize, horizontal: bool) {
    let Some(bank) = runtime_shell.visible_mom_bank.as_mut() else {
        return;
    };
    if !bank.messages.is_empty() {
        return;
    }
    match bank.phase {
        VisibleMomBankPhase::InitializeQuestion
        | VisibleMomBankPhase::AccessQuestion
        | VisibleMomBankPhase::ChangeQuestion => {
            bank.yes_no_index = 1 - bank.yes_no_index.min(1);
        }
        VisibleMomBankPhase::Menu => {
            bank.menu_index = wrapped_index(bank.menu_index, 4, delta);
        }
        VisibleMomBankPhase::Withdraw | VisibleMomBankPhase::Deposit => {
            if horizontal {
                bank.digit = (i16::from(bank.digit) + delta.signum() as i16)
                    .clamp(0, 5) as u8;
            } else {
                let place = 10_u32.pow(u32::from(5 - bank.digit));
                if delta < 0 {
                    bank.amount = bank.amount.saturating_add(place).min(999_999);
                } else {
                    bank.amount = bank.amount.saturating_sub(place);
                }
            }
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn press_visible_select_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if !runtime_shell.battle_messages.is_empty()
        || runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active)
    {
        return Ok(());
    }
    if runtime_shell.pc_notice.is_some() {
        return Ok(());
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.visible_unown_words.is_some()
        || runtime_shell.visible_diploma.is_some()
    {
        return Ok(());
    }
    if runtime_shell.visible_slot_machine.is_some() || runtime_shell.visible_card_flip.is_some() {
        return Ok(());
    }
    if runtime_shell.intro_screen.is_some() {
        return skip_visible_intro_screen(runtime_shell, GameButton::Select);
    }
    if runtime_shell.credits_screen.is_some() {
        record_visible_runtime_action(runtime_shell, "credits:select:ignored")?;
        runtime_shell
            .last_audio_events
            .push("credits Select ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        record_visible_runtime_action(runtime_shell, "boot_prompt:select:ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pending_time_set.is_some() {
        record_visible_runtime_action(runtime_shell, "time_set:select:ignored")?;
        runtime_shell
            .last_audio_events
            .push("time set Select ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pending_oak_intro.is_some() {
        record_visible_runtime_action(runtime_shell, "oak_intro:select:ignored")?;
        runtime_shell
            .last_audio_events
            .push("oak intro Select ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pending_gender_selection.is_some() {
        record_visible_runtime_action(runtime_shell, "gender:select:ignored")?;
        runtime_shell
            .last_audio_events
            .push("gender Select ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        record_visible_runtime_action(runtime_shell, "pc:hall_of_fame:select:ignored")?;
        return Ok(());
    }
    if runtime_shell.special_boundary.is_some() {
        return close_visible_special_boundary(runtime_shell);
    }
    if runtime_shell.field_notice.is_some() {
        record_visible_runtime_action(runtime_shell, "field:notice:select:ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pack_toss.is_some() {
        record_visible_runtime_action(runtime_shell, "pack:toss:select_ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.held_item_swap_prompt {
        record_visible_runtime_action(runtime_shell, "party:held_item:swap:select_ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pending_pc_release.is_some() {
        record_visible_runtime_action(runtime_shell, "pc:release-confirm:select:ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.shell.presentation_snapshot()?.battle.is_some() {
        if runtime_shell.battle_move_cursor.is_some() {
            return select_visible_battle_move_swap(runtime_shell);
        }
        record_visible_runtime_action(runtime_shell, "battle:select:ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pokegear_menu_open {
        return toggle_visible_pokegear_page(runtime_shell);
    }
    if runtime_shell.storage_cursor.is_some() {
        record_visible_runtime_action(runtime_shell, "pc:box:select:ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.bag_cursor.is_some()
        || runtime_shell.ball_cursor.is_some()
        || matches!(
            runtime_shell.field_pack_pocket.as_ref(),
            Some(FieldPackPocket::Custom(_))
        )
    {
        return open_visible_field_pack_target(runtime_shell, FieldPackTargetMode::HeldItem);
    }
    if runtime_shell.key_item_cursor.is_some() {
        return register_selected_visible_key_item(runtime_shell);
    }
    if runtime_shell.tmhm_cursor.is_some() {
        return open_visible_tmhm_teach_prompt(runtime_shell);
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    let Some(item_id) = snapshot.progression.registered_key_item.clone() else {
        record_visible_runtime_action(runtime_shell, "pack:key_item:select:none_registered")?;
        runtime_shell
            .last_audio_events
            .push("no registered key item".to_string());
        set_shell_action_status(runtime_shell, "NO REGISTERED ITEM");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    if !snapshot
        .bag
        .key_items
        .iter()
        .any(|item| item.item_id == item_id && item.quantity > 0)
    {
        record_visible_runtime_action(
            runtime_shell,
            format!("pack:key_item:select:{item_id}:not_carried"),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("registered key item {item_id} is not carried"));
        set_shell_action_status(runtime_shell, format!("{item_id} NOT IN BAG"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, format!("pack:key_item:select:{item_id}"))?;
    use_visible_field_bag_item_by_id(runtime_shell, item_id)
}

fn press_visible_start_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .battle_exp_tween
        .as_ref()
        .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active)
    {
        return Ok(());
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.visible_unown_words.is_some()
        || runtime_shell.visible_diploma.is_some()
    {
        return Ok(());
    }
    if runtime_shell.visible_slot_machine.is_some() || runtime_shell.visible_card_flip.is_some() {
        return Ok(());
    }
    if runtime_shell.field_text_reveal.is_some() {
        // An active field textbox consumes non-confirm buttons. Do not let
        // Start open the field menu beneath a running interaction.
        return Ok(());
    }
    if !runtime_shell.battle_messages.is_empty() {
        // ASM PromptButton and PrintLetterDelay consume PAD_A/PAD_B only.
        // Start must not accelerate or dismiss battle dialogue.
        return Ok(());
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        return Ok(());
    }
    if runtime_shell.visible_mom_bank.is_some() {
        return Ok(());
    }
    if runtime_shell.intro_screen.is_some() {
        return skip_visible_intro_screen(runtime_shell, GameButton::Start);
    }
    if runtime_shell.pending_delete_save.is_some() {
        return confirm_visible_delete_save_screen(runtime_shell);
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return confirm_visible_clock_reset_screen(runtime_shell);
    }
    if runtime_shell.pending_time_set.is_some() {
        return press_visible_time_set_a_button(runtime_shell);
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return press_visible_oak_intro_a_button(runtime_shell);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return confirm_visible_gender_selection(runtime_shell);
    }
    if runtime_shell.options_menu_open {
        record_visible_runtime_action(runtime_shell, "options:close:start")?;
        close_visible_options_menu(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if runtime_shell.field_notice.is_some()
        || runtime_shell.pc_notice.is_some()
        || runtime_shell.pack_toss.is_some()
        || runtime_shell.held_item_swap_prompt
    {
        record_visible_runtime_action(runtime_shell, "field:notice:start:ignored")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "input:Start")?;
    runtime_shell
        .last_audio_events
        .push("pressed Start".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    toggle_visible_start_menu(runtime_shell)
}

fn has_visible_shell_b_action(runtime_shell: &mut BevyRuntimeShell) -> bool {
    if runtime_shell.player_walk_frame_ticks > 0 {
        return false;
    }
    if !runtime_shell.battle_messages.is_empty()
        || runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active)
    {
        return true;
    }
    if runtime_shell.field_notice.is_some() || runtime_shell.pc_notice.is_some() {
        return true;
    }
    if runtime_shell.visible_diploma.is_some() {
        return true;
    }
    if runtime_shell.visible_unown_words.is_some() {
        return true;
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
    {
        return true;
    }
    if runtime_shell.visible_card_flip.is_some() {
        return true;
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return true;
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return true;
    }
    if runtime_shell.visible_mom_bank.is_some() {
        return true;
    }
    if runtime_shell.pending_day_of_week.is_some() {
        return true;
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        return true;
    }
    if runtime_shell.buena_prize_cursor.is_some() {
        return true;
    }
    if runtime_shell.intro_screen.is_some() {
        return true;
    }
    if runtime_shell.credits_screen.is_some() {
        return true;
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        return true;
    }
    if runtime_shell.title_menu.is_some() {
        return true;
    }
    if runtime_shell.pending_time_set.is_some() {
        return true;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return true;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return true;
    }
    cached_runtime_snapshot(runtime_shell)
        .map(|snapshot| {
            snapshot.ui.pending_yes_no.is_some()
                || runtime_shell.pending_phone_prompt.is_some()
                || snapshot.ui.pending_text_wait.is_some()
                || snapshot.pending_move_learn.is_some()
                || snapshot.pending_shop.is_some()
                || snapshot.ui.text_window_open
                || snapshot.ui.window_open
                || visible_menu_has_selectable_options(&snapshot)
                || snapshot.ui.active_pokemon_picture.is_some()
                || runtime_shell.party_menu_open
                || runtime_shell.pokedex_menu_open
                || runtime_shell.pokegear_menu_open
                || runtime_shell.trainer_card_open
                || runtime_shell.options_menu_open
                || runtime_shell.save_menu_open
                || runtime_shell.special_boundary.is_some()
                || runtime_shell.start_menu_cursor.is_some()
                || runtime_shell.storage_cursor.is_some()
                || runtime_shell.pc_item_cursor.is_some()
                || (snapshot.battle.is_none() && visible_field_pack_is_open(runtime_shell))
                || (snapshot.battle.is_some()
                    && (runtime_shell.ball_cursor.is_some()
                        || runtime_shell.bag_cursor.is_some()
                        || runtime_shell.key_item_cursor.is_some()
                        || runtime_shell.tmhm_cursor.is_some()))
                || (snapshot.battle.is_some()
                    && (runtime_shell.battle_move_cursor.is_some()
                        || runtime_shell.battle_switch_cursor.is_some()))
                || snapshot.battle.is_some()
        })
        .unwrap_or(false)
}

fn has_visible_shell_select_action(runtime_shell: &mut BevyRuntimeShell) -> bool {
    if runtime_shell.player_walk_frame_ticks > 0 {
        return false;
    }
    if retained_text_surface_owns_gameplay_input(runtime_shell) {
        return true;
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.visible_unown_words.is_some()
        || runtime_shell.visible_diploma.is_some()
    {
        return true;
    }
    if runtime_shell.visible_unown_puzzle.is_some()
        || runtime_shell.visible_slot_machine.is_some()
        || runtime_shell.visible_card_flip.is_some()
        || runtime_shell.kurt_apricorn_cursor.is_some()
        || runtime_shell.buena_prize_cursor.is_some()
    {
        return false;
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        return true;
    }
    if runtime_shell.pending_time_set.is_some() {
        return true;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return true;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return true;
    }
    if runtime_shell.special_boundary.is_some() {
        return true;
    }
    cached_runtime_snapshot(runtime_shell)
        .map(|snapshot| {
            (snapshot.battle.is_some() && runtime_shell.battle_move_cursor.is_some())
                || (snapshot.battle.is_none()
                && snapshot.pending_shop.is_none()
                && !snapshot.ui.text_window_open
                && !snapshot.ui.window_open
                && snapshot.ui.menu.is_none()
                && snapshot.ui.active_pokemon_picture.is_none()
                && snapshot.ui.pending_yes_no.is_none()
                && runtime_shell.pending_phone_prompt.is_none()
                && snapshot.ui.pending_text_wait.is_none()
                && snapshot.pending_move_learn.is_none()
                && !has_visible_elevator_prompt(&snapshot, runtime_shell)
                && !has_visible_gift_pokemon_prompt(&snapshot, runtime_shell)
                && runtime_shell.special_boundary.is_none()
                && !has_visible_auto_script_action(runtime_shell, &snapshot)
                && (runtime_shell.pokegear_menu_open
                    || runtime_shell.storage_cursor.is_some()
                    || runtime_shell.bag_cursor.is_some()
                    || runtime_shell.ball_cursor.is_some()
                    || matches!(
                        runtime_shell.field_pack_pocket.as_ref(),
                        Some(FieldPackPocket::Custom(_))
                    )
                    || runtime_shell.key_item_cursor.is_some()
                    || (!visible_field_pack_is_open(runtime_shell)
                        && snapshot.progression.registered_key_item.is_some())))
        })
        .unwrap_or(false)
}

fn has_visible_shell_start_action(runtime_shell: &mut BevyRuntimeShell) -> bool {
    if runtime_shell.player_walk_frame_ticks > 0 {
        return false;
    }
    if retained_text_surface_owns_gameplay_input(runtime_shell) {
        return true;
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.visible_unown_words.is_some()
        || runtime_shell.visible_diploma.is_some()
    {
        return true;
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        return true;
    }
    if runtime_shell.visible_unown_puzzle.is_some()
        || runtime_shell.visible_slot_machine.is_some()
        || runtime_shell.visible_card_flip.is_some()
        || runtime_shell.kurt_apricorn_cursor.is_some()
        || runtime_shell.buena_prize_cursor.is_some()
    {
        return false;
    }
    if runtime_shell.credits_screen.is_some() {
        return false;
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        return true;
    }
    if runtime_shell.title_menu.is_some() {
        return false;
    }
    if runtime_shell.pending_time_set.is_some() {
        return true;
    }
    if runtime_shell.pending_oak_intro.is_some() {
        return true;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return true;
    }
    if runtime_shell.special_boundary.is_some() {
        return false;
    }
    cached_runtime_snapshot(runtime_shell)
        .map(|snapshot| {
            runtime_shell.start_menu_cursor.is_some()
                || (snapshot.battle.is_none()
                    && snapshot.pending_shop.is_none()
                    && !snapshot.ui.text_window_open
                    && !snapshot.ui.window_open
                    && snapshot.ui.menu.is_none()
                    && snapshot.ui.active_pokemon_picture.is_none()
                    && snapshot.ui.pending_yes_no.is_none()
                    && runtime_shell.pending_phone_prompt.is_none()
                    && snapshot.ui.pending_text_wait.is_none()
                    && snapshot.pending_move_learn.is_none()
                    && !has_visible_elevator_prompt(&snapshot, runtime_shell)
                    && !has_visible_gift_pokemon_prompt(&snapshot, runtime_shell)
                    && !runtime_shell.party_menu_open
                    && !runtime_shell.pokedex_menu_open
                    && !runtime_shell.pokegear_menu_open
                    && !runtime_shell.trainer_card_open
                    && !runtime_shell.options_menu_open
                    && !runtime_shell.save_menu_open
                    && runtime_shell.special_boundary.is_none()
                    && !visible_field_pack_is_open(runtime_shell)
                    && !has_visible_auto_script_action(runtime_shell, &snapshot))
        })
        .unwrap_or(false)
}

fn has_visible_shell_direction_action(runtime_shell: &mut BevyRuntimeShell) -> bool {
    if retained_text_surface_owns_gameplay_input(runtime_shell) {
        return true;
    }
    if runtime_shell.visible_heal_machine.is_some()
        || runtime_shell.visible_magnet_train.is_some()
        || runtime_shell.visible_unown_words.is_some()
        || runtime_shell.visible_diploma.is_some()
    {
        return true;
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        return true;
    }
    if runtime_shell.visible_card_flip.is_some() {
        return true;
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return true;
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return true;
    }
    if runtime_shell.visible_mom_bank.is_some() {
        return true;
    }
    if runtime_shell.pending_day_of_week.is_some() {
        return true;
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        return true;
    }
    if runtime_shell.buena_prize_cursor.is_some() {
        return true;
    }
    if runtime_shell.credits_screen.is_some() {
        return false;
    }
    if runtime_shell.pending_delete_save.is_some() || runtime_shell.pending_clock_reset.is_some() {
        return true;
    }
    if runtime_shell.title_menu.is_some() {
        return true;
    }
    if runtime_shell.pending_time_set.is_some() {
        return true;
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return true;
    }
    cached_runtime_snapshot(runtime_shell)
        .map(|snapshot| {
            runtime_shell.start_menu_cursor.is_some()
                || snapshot.ui.pending_yes_no.is_some()
                || runtime_shell.pending_phone_prompt.is_some()
                || snapshot.ui.pending_text_wait.is_some()
                || snapshot.pending_move_learn.is_some()
                || snapshot.ui.text_window_open
                || snapshot.ui.active_pokemon_picture.is_some()
                || runtime_shell.party_menu_open
                || runtime_shell.pokedex_menu_open
                || runtime_shell.pokegear_menu_open
                || runtime_shell.trainer_card_open
                || runtime_shell.options_menu_open
                || runtime_shell.save_menu_open
                || runtime_shell.special_boundary.is_some()
                || runtime_shell.storage_cursor.is_some()
                || runtime_shell.pc_item_cursor.is_some()
                || visible_field_pack_is_open(runtime_shell)
                || snapshot.pending_shop.is_some()
                || has_visible_direction_blocking_script_work(runtime_shell, &snapshot)
                || visible_menu_has_selectable_options(&snapshot)
                || snapshot.battle.is_some()
                || has_visible_elevator_prompt(&snapshot, runtime_shell)
                || has_visible_gift_pokemon_prompt(&snapshot, runtime_shell)
        })
        .unwrap_or(false)
}

fn retained_text_surface_owns_gameplay_input(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell.field_notice.is_some()
        || runtime_shell.pc_notice.is_some()
        || !runtime_shell.battle_messages.is_empty()
        || runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
        || runtime_shell
            .battle_level_stats
            .front()
            .is_some_and(|stats| stats.active)
}

fn has_visible_direction_blocking_script_work(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> bool {
    snapshot.script_events.pending_text_label.is_some()
        || snapshot.script_events.pending_map_load.is_some()
        || snapshot.script_events.pending_map_refresh.is_some()
        || snapshot.script_events.pending_music_fade.is_some()
        || snapshot.script_events.pending_screen_fade.is_some()
        || !snapshot.script_events.pending_delays.is_empty()
        || !snapshot.script_events.pending_earthquakes.is_empty()
        || !snapshot.script_events.pending_emotes.is_empty()
        || snapshot.script_events.pending_script_warp.is_some()
        || !snapshot.script_events.command_queue.is_empty()
        || snapshot.script_events.next_script.is_some()
        || !snapshot.script_events.deferred_scripts.is_empty()
        || snapshot.script_events.script_ended.is_some()
        || visible_auto_runtime_flag(snapshot).is_some()
        || runtime_shell.active_script_cursor.is_some()
}

fn visible_field_shortcut_allowed(runtime_shell: &BevyRuntimeShell) -> bool {
    if runtime_shell.intro_screen.is_some()
        || runtime_shell.title_menu.is_some()
        || runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
        || runtime_shell.special_boundary.is_some()
    {
        return false;
    }
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| {
            snapshot.battle.is_none()
                && snapshot.pending_shop.is_none()
                && !snapshot.ui.text_window_open
                && !snapshot.ui.window_open
                && snapshot.ui.menu.is_none()
                && snapshot.ui.active_pokemon_picture.is_none()
                && snapshot.ui.pending_yes_no.is_none()
                && runtime_shell.pending_phone_prompt.is_none()
                && snapshot.ui.pending_text_wait.is_none()
                && snapshot.pending_move_learn.is_none()
                && !has_visible_elevator_prompt(&snapshot, runtime_shell)
                && !has_visible_gift_pokemon_prompt(&snapshot, runtime_shell)
                && runtime_shell.start_menu_cursor.is_none()
                && !runtime_shell.party_menu_open
                && !runtime_shell.pokedex_menu_open
                && !runtime_shell.pokegear_menu_open
                && !runtime_shell.trainer_card_open
                && !runtime_shell.options_menu_open
                && !runtime_shell.save_menu_open
                && !visible_field_pack_is_open(runtime_shell)
                && runtime_shell.storage_cursor.is_none()
                && runtime_shell.pc_item_cursor.is_none()
                && !has_visible_auto_script_action(runtime_shell, &snapshot)
        })
        .unwrap_or(false)
}

fn visible_pending_text_wait_active(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| snapshot.ui.pending_text_wait.is_some())
        .unwrap_or(false)
}

fn advance_visible_pending_text_wait(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    // TextCommand_PROMPT_BUTTON plays this only after a fully printed page is
    // acknowledged. The earlier reveal-completion branch returns before this
    // function, so fast-forwarding text remains silent.
    if pending_text_wait_uses_prompt_button(runtime_shell) {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    }
    record_visible_runtime_action(runtime_shell, "ui:text_wait:advance")?;
    let next_cursor = visible_active_compiled_script_cursor(runtime_shell);
    if let Some(cursor) = next_cursor {
        let advanced = runtime_shell
            .shell
            .advance_text_wait_and_run_compiled_script(
                Some(cursor),
                256,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )?;
        runtime_shell.last_audio_events.push(format!(
            "advanced text wait {:?} resumed_steps={}",
            advanced.wait.state_checksum,
            advanced.run.steps.len()
        ));
        let reached_boundary =
            integrate_visible_compiled_script_run(runtime_shell, &advanced.run.steps)?;
        arm_visible_active_script_cursor_from_run(runtime_shell, advanced.run.next_cursor);
        if reached_boundary {
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
    } else {
        let advance = runtime_shell.shell.advance_pending_text_wait()?;
        runtime_shell
            .last_audio_events
            .push(format!("advanced text wait {:?}", advance.state_checksum));
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn confirm_visible_pending_yes_no(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
        .context("yes/no prompt is active without a valid cursor")?;
    resolve_visible_pending_yes_no(runtime_shell, selected == 0)
}

fn accept_visible_pending_yes_no(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    resolve_visible_pending_yes_no(runtime_shell, true)
}

fn decline_visible_pending_yes_no(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    resolve_visible_pending_yes_no(runtime_shell, false)
}

fn confirm_visible_phone_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:phone-number", 2)
        .context("phone prompt is active without a valid cursor")?;
    resolve_visible_phone_prompt(runtime_shell, selected == 0)
}

fn decline_visible_phone_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    resolve_visible_phone_prompt(runtime_shell, false)
}

fn resolve_visible_phone_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    let Some(prompt) = runtime_shell.pending_phone_prompt.clone() else {
        record_visible_runtime_action(runtime_shell, "ui:phone_number:none_open")?;
        runtime_shell
            .last_audio_events
            .push("no pending phone prompt is open".to_string());
        set_shell_action_status(runtime_shell, "NO PHONE PROMPT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:phone_number:{}:{}:{}:{}",
            prompt.source_script, prompt.command_index, prompt.contact_id, accepted
        ),
    )?;
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "ui:phone-number".to_string(),
        option_index: if accepted { 0 } else { 1 },
    });
    let runtime_inputs = explicit_compiled_script_runtime_inputs(
        runtime_shell,
        &prompt.source_script,
        prompt.command_index,
    )?;
    let resolved = runtime_shell
        .shell
        .resolve_phone_prompt_and_run_compiled_script(
            &prompt.source_script,
            prompt.command_index,
            runtime_inputs,
            accepted,
            256,
        )?;
    runtime_shell.last_audio_events.push(format!(
        "phone prompt contact={} accepted={} result={} resumed_steps={} checksum={:?}",
        prompt.contact_id,
        accepted,
        resolved.step.mutation.result.result_tag(),
        resolved.run.steps.len(),
        resolved.step.mutation.state_checksum
    ));
    integrate_visible_script_mutation_outcome(runtime_shell, &resolved.step.mutation)?;
    runtime_shell.pending_phone_prompt = None;
    runtime_shell.yes_no_cursor = None;
    trim_event_log(&mut runtime_shell.last_audio_events);
    if activate_visible_script_boundary_after_outcome(runtime_shell, &resolved.step.mutation)? {
        return Ok(());
    }
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &resolved.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, resolved.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn resolve_visible_pending_yes_no(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    if runtime_shell
        .shell
        .snapshot()?
        .bug_contest
        .pending_caught_mon
        .is_some()
    {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "bug_contest:replace:{}",
                if accepted { "keep" } else { "discard" }
            ),
        )?;
        let resolved = runtime_shell
            .shell
            .resolve_bug_contest_caught_mon(accepted)?;
        runtime_shell.yes_no_cursor = None;
        runtime_shell.last_audio_events.push(format!(
            "Bug Contest replacement accepted={} effect={:?} checksum={:?}",
            accepted, resolved.outcome.effect, resolved.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            if accepted {
                "BUG CONTEST SWITCHED"
            } else {
                "BUG CONTEST KEPT"
            },
        );
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(
        runtime_shell,
        format!("ui:yes_no:{}", if accepted { "yes" } else { "no" }),
    )?;
    let next_cursor = visible_active_compiled_script_cursor(runtime_shell);
    runtime_shell.yes_no_cursor = None;
    if let Some(cursor) = next_cursor {
        let resolved = runtime_shell.shell.resolve_yes_no_and_run_compiled_script(
            accepted,
            Some(cursor),
            256,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "yes/no accepted={} script_value={} resumed_steps={} checksum={:?}",
            resolved.resolution.accepted,
            resolved.resolution.script_value,
            resolved.run.steps.len(),
            resolved.resolution.state_checksum
        ));
        let reached_boundary =
            integrate_visible_compiled_script_run(runtime_shell, &resolved.run.steps)?;
        arm_visible_active_script_cursor_from_run(runtime_shell, resolved.run.next_cursor);
        if reached_boundary {
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
    } else {
        let resolution = runtime_shell.shell.resolve_pending_yes_no(accepted)?;
        runtime_shell.last_audio_events.push(format!(
            "yes/no accepted={} script_value={} checksum={:?}",
            resolution.accepted, resolution.script_value, resolution.state_checksum
        ));
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn play_pending_field_notice_sound(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if let Some(species) = runtime_shell.pending_field_notice_cry.take() {
        queue_visible_pokemon_cry(runtime_shell, &species, "field_notice")?;
    }
    let Some(audio_id) = runtime_shell.pending_field_notice_sound.take() else {
        return Ok(());
    };
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
        &audio_id,
    )
}

fn begin_pending_field_notice_effect(runtime_shell: &mut BevyRuntimeShell) -> bool {
    if runtime_shell.visible_waterfall_animation.is_some() {
        return true;
    }
    if runtime_shell.pending_field_notice_effect_frames.is_none() {
        return false;
    }
    if runtime_shell.visible_sweet_scent_delay {
        return true;
    }
    if let Some(from_tile) = runtime_shell.pending_surf_start_from {
        // UsedSurfScript switches to the surf sprite and then applies one
        // sixteen-frame `slow_step`. The core has committed the destination;
        // interpolate there from the retained land tile after the text closes.
        runtime_shell.field_notice_scene = None;
        runtime_shell.player_walk_from = Some(from_tile);
        runtime_shell.player_walk_total_ticks = WALK_FRAME_HOLD_TICKS.saturating_mul(2);
        runtime_shell.player_walk_frame_ticks = runtime_shell.player_walk_total_ticks;
        runtime_shell.player_walk_stride = true;
        runtime_shell.player_walk_mirror_stride = false;
    } else if runtime_shell.visible_cut_animation.is_some()
        || runtime_shell.visible_whirlpool_animation.is_some()
    {
        // CutDownTree/DisappearWhirlpool have already installed the
        // replacement block. Once the use text closes, their OAM draws over
        // that cleared tilemap rather than the retained obstacle block.
        runtime_shell.field_notice_scene = None;
    } else if runtime_shell.visible_whirlpool_animation.is_none()
        && runtime_shell.visible_headbutt_animation.is_none()
        && runtime_shell.visible_flash_animation.is_none()
    {
        runtime_shell.visible_earthquake = Some(VisibleEarthquake {
            intensity: 2,
            frames_remaining: 20,
            phase: 0,
        });
    }
    true
}

fn visible_field_notice_uses_prompt_arrow(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell.pending_field_travel_delay_frames.is_none()
        && runtime_shell.visible_field_travel_animation.is_none()
        && runtime_shell.pending_surf_start_from.is_none()
        && runtime_shell.visible_waterfall_animation.is_none()
        && runtime_shell.visible_flash_animation.is_none()
        && runtime_shell.visible_cut_animation.is_none()
        && runtime_shell.visible_whirlpool_animation.is_none()
        && runtime_shell.visible_headbutt_animation.is_none()
        && !runtime_shell.pending_field_battle_entry
        && runtime_shell.field_notice_queue.is_empty()
        && !runtime_shell.pending_sweet_scent_nothing_notice
}

fn settle_pending_field_battle_entry_after_notice(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<bool> {
    if !std::mem::take(&mut runtime_shell.pending_field_battle_entry) {
        return Ok(false);
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    prepare_visible_battle_entry(runtime_shell)?;
    settle_visible_battle_after_action(runtime_shell)?;
    Ok(true)
}

fn advance_visible_heal_machine(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (kind, party_count, frame) = runtime_shell
        .visible_heal_machine
        .as_ref()
        .map(|animation| (animation.kind, animation.party_count, animation.frame))
        .context("HealMachineAnim disappeared during its retained frame")?;
    let ball_frames = u16::from(party_count) * 30;
    if frame < ball_frames && frame % 30 == 0 {
        let BevyRuntimeShell { shell, pending_audio, last_audio_events, .. } = runtime_shell;
        queue_visible_sound_effect(
            shell.runtime().audio(),
            pending_audio,
            last_audio_events,
            "SFX_SECOND_PART_OF_ITEMFINDER",
        )?;
    }
    if frame == ball_frames {
        if kind == 2 {
            let BevyRuntimeShell { shell, pending_audio, last_audio_events, .. } = runtime_shell;
            queue_visible_sound_effect(
                shell.runtime().audio(),
                pending_audio,
                last_audio_events,
                "SFX_GAME_FREAK_LOGO_GS",
            )?;
        } else {
            queue_visible_heal_music(runtime_shell)?;
        }
    }
    let total_frames = ball_frames + 80;
    if frame >= total_frames {
        if kind == 2 {
            let BevyRuntimeShell { shell, pending_audio, last_audio_events, .. } = runtime_shell;
            queue_visible_sound_effect(
                shell.runtime().audio(),
                pending_audio,
                last_audio_events,
                "SFX_BOOT_PC",
            )?;
        }
        runtime_shell.visible_heal_machine = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return continue_visible_script_after_prompt(runtime_shell);
    }
    runtime_shell.visible_heal_machine.as_mut().unwrap().frame += 1;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn queue_visible_heal_music(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MUSIC_ID: &str = "MUSIC_HEAL";
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
    runtime_shell.heal_music_active = true;
    runtime_shell.faded_music = None;
    runtime_shell
        .last_audio_events
        .push("queued heal-machine music MUSIC_HEAL".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn queue_visible_magnet_train_music(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MUSIC_ID: &str = "MUSIC_MAGNET_TRAIN";
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
    Ok(())
}

fn advance_visible_magnet_train(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .visible_magnet_train
        .as_ref()
        .is_some_and(|animation| animation.phase >= 7 && animation.arrival_sfx_played)
    {
        runtime_shell.visible_magnet_train = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return continue_visible_script_after_prompt(runtime_shell);
    }
    let animation = runtime_shell
        .visible_magnet_train
        .as_mut()
        .context("MagnetTrain disappeared during its retained frame")?;
    match animation.phase {
        0 => {
            animation.wait_counter = 128;
            animation.phase = 1;
        }
        1 | 3 | 5 => {
            if animation.wait_counter > 0 {
                animation.wait_counter -= 1;
            } else {
                animation.phase += 1;
            }
        }
        2 => {
            if animation.position == animation.hold_position {
                animation.wait_counter = 128;
                animation.phase = 3;
            } else {
                animation.position -= animation.direction;
            }
        }
        4 => {
            if animation.position == animation.final_position {
                animation.phase = 5;
            } else {
                animation.position -= animation.direction * 2;
            }
        }
        6 => animation.phase = 7,
        _ => {}
    }
    animation.offset += animation.direction * 2;
    if animation.phase < 7 {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let BevyRuntimeShell { shell, pending_audio, last_audio_events, .. } = runtime_shell;
    queue_visible_sound_effect(
        shell.runtime().audio(),
        pending_audio,
        last_audio_events,
        "SFX_TRAIN_ARRIVED",
    )?;
    runtime_shell
        .visible_magnet_train
        .as_mut()
        .unwrap()
        .arrival_sfx_played = true;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn close_visible_unown_words(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    anyhow::ensure!(
        runtime_shell.visible_unown_words.take().is_some(),
        "Unown word display disappeared before acknowledgement"
    );
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn close_visible_diploma(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    anyhow::ensure!(runtime_shell.visible_diploma.take().is_some(), "Diploma disappeared before acknowledgement");
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn continue_visible_script_after_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MAX_CONTINUE_STEPS: usize = 2048;
    for _ in 0..MAX_CONTINUE_STEPS {
        if close_visible_noninteractive_runtime_surface(runtime_shell)? {
            continue;
        }
        let snapshot = runtime_shell.shell.presentation_snapshot()?;
        if advance_visible_next_pending_script_request(runtime_shell, &snapshot)? {
            return Ok(());
        }
        if !snapshot.script_events.audio_events.is_empty() {
            drain_visible_audio_events(runtime_shell)?;
            continue;
        }
        if has_visible_pending_non_audio_script_events(&snapshot) {
            drain_visible_non_audio_script_events(runtime_shell)?;
            continue;
        }
        // End/EndCallback and map-control flags can be the final products of
        // a compiled command, after its cursor has already become None. ASM
        // consumes that terminal control work before returning to joypad
        // polling. Leaving it behind makes the first overworld direction or
        // A press service script history instead of moving/interacting.
        if snapshot.script_events.script_ended.is_some() {
            take_visible_script_end_state(runtime_shell)?;
            continue;
        }
        if let Some(flag) = visible_auto_runtime_flag(&snapshot) {
            consume_visible_runtime_flag_kind(runtime_shell, flag)?;
            continue;
        }
        if runtime_shell.active_script_cursor.is_none() {
            return Ok(());
        }
        // `writetext` leaves the text window open while the script immediately
        // advances into `waitbutton`/`promptbutton`. Likewise, acknowledging
        // that wait resumes directly into `closetext`. Treating the open
        // window itself as a boundary before those commands run strands the
        // script between its text and wait opcodes (notably MeetMomScript).
        let text_window_blocks = snapshot.ui.text_window_open
            && runtime_shell.active_script_cursor.is_none();
        if runtime_shell.visible_mom_bank.is_some()
            || snapshot.ui.pending_yes_no.is_some()
            || runtime_shell.pending_day_of_week.is_some()
            || runtime_shell.pending_phone_prompt.is_some()
            || snapshot.ui.pending_text_wait.is_some()
            || snapshot.script_events.pending_text_label.is_some()
            || snapshot.script_events.pending_script_warp.is_some()
            || snapshot.script_events.pending_map_load.is_some()
            || snapshot.script_events.pending_map_refresh.is_some()
            || snapshot.script_events.pending_music_fade.is_some()
            || snapshot.script_events.pending_screen_fade.is_some()
            || snapshot.pending_shop.is_some()
            || text_window_blocks
            || snapshot.ui.window_open
            || snapshot.ui.active_pokemon_picture.is_some()
            || has_visible_elevator_prompt(&snapshot, runtime_shell)
            || has_visible_gift_pokemon_prompt(&snapshot, runtime_shell)
            || visible_menu_has_selectable_options(&snapshot)
            || snapshot.battle.is_some()
            || runtime_shell.start_menu_cursor.is_some()
            || runtime_shell.party_menu_open
            || runtime_shell.pokedex_menu_open
            || runtime_shell.pokegear_menu_open
            || runtime_shell.options_menu_open
            || runtime_shell.save_menu_open
            || runtime_shell.special_boundary.is_some()
            || runtime_shell.kurt_apricorn_cursor.is_some()
            || runtime_shell.buena_prize_cursor.is_some()
            || runtime_shell.intro_screen.is_some()
            || runtime_shell.credits_screen.is_some()
            || visible_field_pack_is_open(runtime_shell)
            || runtime_shell.storage_cursor.is_some()
            || runtime_shell.pc_item_cursor.is_some()
            || runtime_shell.player_pc_action_cursor.is_some()
            || runtime_shell.mailbox_cursor.is_some()
            || runtime_shell.mailbox_action_cursor.is_some()
        {
            return Ok(());
        }
        execute_visible_active_script_step(runtime_shell)?;
    }
    anyhow::bail!("visible script continuation exceeded {MAX_CONTINUE_STEPS} steps")
}

fn advance_visible_next_pending_script_request(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<bool> {
    if snapshot.script_events.pending_text_label.is_some() {
        advance_visible_text_label(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_script_warp.is_some() {
        execute_visible_pending_script_warp(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_map_load.is_some() {
        take_visible_pending_map_load(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_map_refresh.is_some() {
        take_visible_pending_map_refresh(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_music_fade.is_some() {
        take_visible_pending_music_fade(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_screen_fade.is_some() {
        take_visible_pending_screen_fade(runtime_shell)?;
        return Ok(true);
    }
    if !snapshot.script_events.pending_delays.is_empty() {
        drain_visible_delays(runtime_shell)?;
        return Ok(true);
    }
    if !snapshot.script_events.pending_earthquakes.is_empty() {
        drain_visible_earthquakes(runtime_shell)?;
        return Ok(true);
    }
    if !snapshot.script_events.pending_emotes.is_empty() {
        drain_visible_emotes(runtime_shell)?;
        return Ok(true);
    }
    Ok(false)
}

fn toggle_visible_start_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.special_boundary.is_some() {
        return Ok(());
    }
    if runtime_shell.start_menu_cursor.is_some() {
        record_visible_runtime_action(runtime_shell, "start_menu:close")?;
        close_visible_start_menu(runtime_shell);
        return Ok(());
    }
    let snapshot = runtime_shell.shell.presentation_snapshot()?;
    if runtime_shell.storage_cursor.is_some()
        && snapshot.battle.is_none()
        && snapshot.pending_shop.is_none()
        && !snapshot.ui.text_window_open
        && !snapshot.ui.window_open
        && snapshot.ui.menu.is_none()
        && snapshot.ui.active_pokemon_picture.is_none()
        && snapshot.ui.pending_yes_no.is_none()
        && snapshot.ui.pending_text_wait.is_none()
        && !runtime_shell.party_menu_open
        && !has_visible_auto_script_action(runtime_shell, &snapshot)
    {
        return open_visible_party_menu(runtime_shell);
    }
    if runtime_shell.pc_item_cursor.is_some()
        && snapshot.battle.is_none()
        && snapshot.pending_shop.is_none()
        && !snapshot.ui.text_window_open
        && !snapshot.ui.window_open
        && snapshot.ui.menu.is_none()
        && snapshot.ui.active_pokemon_picture.is_none()
        && snapshot.ui.pending_yes_no.is_none()
        && snapshot.ui.pending_text_wait.is_none()
        && !has_visible_auto_script_action(runtime_shell, &snapshot)
    {
        return open_visible_pc_item_deposit_pack(runtime_shell);
    }
    let blockers = visible_start_menu_blockers(runtime_shell, &snapshot);
    if !blockers.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!("start_menu:blocked:{}", blockers.join(",")),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("Start menu blocked by {}", blockers.join(", ")));
        set_shell_action_status(runtime_shell, "START MENU BLOCKED");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "start_menu:open")?;
    close_visible_field_pack_without_log(runtime_shell);
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_detail_page = 0;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.special_boundary = None;
    runtime_shell.special_boundary_queue.clear();
    runtime_shell.pending_special_cry = None;
    runtime_shell.pending_special_sound = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.start_menu_cursor = Some(MenuCursor {
        surface_id: START_MENU_SURFACE_ID.to_string(),
        option_index: 0,
    });
    runtime_shell
        .last_audio_events
        .push("opened start menu".to_string());
    set_shell_action_status(runtime_shell, "START MENU");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_start_menu_blockers(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if snapshot.battle.is_some() {
        blockers.push("battle");
    }
    if snapshot.pending_shop.is_some() {
        blockers.push("shop");
    }
    if snapshot.ui.text_window_open {
        blockers.push("text_window");
    }
    if snapshot.ui.window_open {
        blockers.push("window");
    }
    if snapshot.ui.menu.is_some() {
        blockers.push("menu");
    }
    if snapshot.ui.active_pokemon_picture.is_some() {
        blockers.push("pokemon_picture");
    }
    if snapshot.ui.pending_yes_no.is_some() {
        blockers.push("yes_no");
    }
    if snapshot.ui.pending_text_wait.is_some() {
        blockers.push("text_wait");
    }
    if snapshot.pending_move_learn.is_some() {
        blockers.push("move_learn");
    }
    if runtime_shell.party_menu_open {
        blockers.push("party");
    }
    if runtime_shell.pokedex_menu_open {
        blockers.push("pokedex");
    }
    if runtime_shell.pokegear_menu_open {
        blockers.push("pokegear");
    }
    if runtime_shell.options_menu_open {
        blockers.push("options");
    }
    if runtime_shell.trainer_card_open {
        blockers.push("trainer_card");
    }
    if runtime_shell.save_menu_open {
        blockers.push("save");
    }
    if runtime_shell.special_boundary.is_some() {
        blockers.push("special_boundary");
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        blockers.push("kurt_apricorn");
    }
    if runtime_shell.buena_prize_cursor.is_some() {
        blockers.push("buena_prize");
    }
    if runtime_shell.pc_hub_cursor.is_some() {
        blockers.push("pc_hub");
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        blockers.push("bill_pc");
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        blockers.push("bill_pc_box");
    }
    if runtime_shell.intro_screen.is_some() {
        blockers.push("intro");
    }
    if runtime_shell.credits_screen.is_some() {
        blockers.push("credits");
    }
    if runtime_shell.storage_cursor.is_some() {
        blockers.push("storage");
    }
    if runtime_shell.pc_item_cursor.is_some() {
        blockers.push("pc_item");
    }
    if visible_field_pack_is_open(runtime_shell) {
        blockers.push("pack");
    }
    if has_visible_auto_script_action(runtime_shell, snapshot) {
        blockers.push("auto_script");
    }
    blockers
}

fn visible_quick_save_blockers(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    allow_active_script_cursor: bool,
    allow_save_menu: bool,
) -> Vec<&'static str> {
    let mut blockers = visible_start_menu_blockers(runtime_shell, snapshot)
        .into_iter()
        .filter(|blocker| (*blocker != "save" || !allow_save_menu) && *blocker != "auto_script")
        .collect::<Vec<_>>();
    if has_visible_save_blocking_script_work(snapshot) {
        blockers.push("auto_script");
    }
    if runtime_shell.intro_screen.is_some() {
        blockers.push("intro");
    }
    if runtime_shell.title_menu.is_some() {
        blockers.push("title");
    }
    if runtime_shell.start_menu_cursor.is_some() {
        blockers.push("start_menu");
    }
    if runtime_shell.party_summary_open {
        blockers.push("party_summary");
    }
    if runtime_shell.pokedex_detail_open {
        blockers.push("pokedex_detail");
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        blockers.push("phone_prompt");
    }
    if runtime_shell.pending_day_of_week.is_some() {
        blockers.push("day_of_week");
    }
    if runtime_shell.visible_mom_bank.is_some() {
        blockers.push("mom_bank");
    }
    if runtime_shell.pending_trainer_sight.is_some() {
        blockers.push("trainer_sight");
    }
    if runtime_shell.pending_name_input.is_some() {
        blockers.push("name_input");
    }
    if runtime_shell.pending_name_choice.is_some() {
        blockers.push("name_choice");
    }
    if runtime_shell.active_script_cursor.is_some() && !allow_active_script_cursor {
        blockers.push("script");
    }
    blockers
}

fn visible_quick_load_blockers(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Vec<&'static str> {
    visible_quick_save_blockers(runtime_shell, snapshot, false, false)
}

fn has_visible_save_blocking_script_work(snapshot: &RuntimeShellSnapshot) -> bool {
    snapshot.script_events.pending_text_label.is_some()
        || snapshot.script_events.pending_map_load.is_some()
        || snapshot.script_events.pending_map_refresh.is_some()
        || snapshot.script_events.pending_music_fade.is_some()
        || snapshot.script_events.pending_screen_fade.is_some()
        || !snapshot.script_events.pending_delays.is_empty()
        || !snapshot.script_events.pending_earthquakes.is_empty()
        || !snapshot.script_events.pending_emotes.is_empty()
        || snapshot.script_events.pending_script_warp.is_some()
        || !snapshot.script_events.command_queue.is_empty()
        || snapshot.script_events.next_script.is_some()
        || !snapshot.script_events.deferred_scripts.is_empty()
        || snapshot.script_events.script_ended.is_some()
        || !snapshot.script_events.audio_events.is_empty()
        || has_visible_pending_non_audio_script_events(snapshot)
        || visible_auto_runtime_flag(snapshot).is_some()
}

fn close_visible_start_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.start_menu_cursor = None;
    runtime_shell
        .last_audio_events
        .push("closed start menu".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn select_visible_start_menu_option(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = selected_visible_start_menu_option(runtime_shell)?;
    let selected_label = start_menu_option_label(selected).to_string();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "start_menu:{}",
            selected_label.replace(' ', "_").to_ascii_lowercase()
        ),
    )?;
    match selected {
        StartMenuOption::Pokemon => {
            open_visible_party_menu(runtime_shell)?;
        }
        StartMenuOption::Pack => {
            open_visible_field_pack(runtime_shell)?;
        }
        StartMenuOption::Save => {
            open_visible_save_menu(runtime_shell)?;
        }
        StartMenuOption::QuitContest => {
            close_visible_start_menu(runtime_shell);
            start_visible_script_entry(runtime_shell, "BugCatchingContestReturnToGateScript")?;
        }
        StartMenuOption::Pokedex => {
            open_visible_pokedex_menu(runtime_shell)?;
        }
        StartMenuOption::Pokegear => {
            open_visible_pokegear_menu(runtime_shell)?;
        }
        StartMenuOption::TrainerCard => {
            open_visible_trainer_card(runtime_shell)?;
        }
        StartMenuOption::Options => {
            open_visible_options_menu(runtime_shell)?;
        }
        StartMenuOption::Exit => {
            close_visible_start_menu(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)?;
        }
    }
    runtime_shell.start_menu_cursor = None;
    if runtime_shell.last_action_status.as_deref() == Some("START MENU") {
        set_shell_action_status(runtime_shell, format!("OPENED {selected_label}"));
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}
