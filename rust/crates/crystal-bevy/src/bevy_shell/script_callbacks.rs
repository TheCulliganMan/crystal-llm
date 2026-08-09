fn take_next_visible_map_callback(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pending_map_callbacks.is_empty() {
        if resume_visible_map_callback_return(runtime_shell)? {
            continue_visible_script_after_prompt(runtime_shell)?;
            return Ok(());
        }
        return take_visible_pending_scene_script(runtime_shell);
    }
    let script = runtime_shell.pending_map_callbacks.remove(0);
    runtime_shell.last_audio_events.push(format!(
        "map callback script={} remaining={}",
        script,
        runtime_shell.pending_map_callbacks.len()
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    reset_visible_selection_cursors(runtime_shell);
    if !has_visible_compiled_script_command(runtime_shell, &script, 0) {
        runtime_shell
            .last_audio_events
            .push(format!("script complete={script}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return take_next_visible_map_callback(runtime_shell);
    }
    start_visible_script_entry(runtime_shell, &script)
}

fn take_visible_pending_scene_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(script) = runtime_shell.pending_scene_script.take() else {
        if resume_visible_map_callback_return(runtime_shell)? {
            continue_visible_script_after_prompt(runtime_shell)?;
        }
        return Ok(());
    };
    runtime_shell
        .last_audio_events
        .push(format!("scene script={script}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    reset_visible_selection_cursors(runtime_shell);
    start_visible_script_entry(runtime_shell, &script)
}

fn resume_visible_map_callback_return(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    let Some(cursor) = runtime_shell.map_callback_return_cursor.take() else {
        return Ok(false);
    };
    runtime_shell.last_audio_events.push(format!(
        "map callback return {}:{}",
        cursor.source_script, cursor.command_index
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    arm_visible_active_script_cursor(runtime_shell, &cursor.source_script, cursor.command_index);
    Ok(true)
}

fn take_visible_pending_music_fade(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:music_fade")?;
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MusicFade)?;
    let RuntimePendingScriptRequest::MusicFade(fade) = &request else {
        anyhow::bail!("pending music fade request returned non-music-fade payload");
    };
    reset_visible_music_state(runtime_shell);
    runtime_shell.faded_music = Some(fade.audio_id.clone());
    runtime_shell
        .last_audio_events
        .push(format!("took pending music fade {:?}", request));
    Ok(())
}

fn take_visible_pending_screen_fade(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:screen_fade")?;
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::ScreenFade)?;
    let RuntimePendingScriptRequest::ScreenFade(fade) = &request else {
        anyhow::bail!("pending screen fade request returned non-screen-fade payload");
    };
    runtime_shell.screen_fade = Some(VisibleScreenFade::new(
        fade.color,
        fade.direction,
        fade.frames,
    ));
    runtime_shell
        .last_audio_events
        .push(format!("took pending screen fade {:?}", request));
    Ok(())
}

fn tick_visible_screen_fade(time: Res<Time>, mut runtime_shell: ResMut<BevyRuntimeShell>) {
    if let Some(VisibleBlackoutPhase::WhiteHold { frames_remaining }) =
        runtime_shell.visible_blackout_phase
    {
        if frames_remaining > 0 {
            runtime_shell.visible_blackout_phase = Some(VisibleBlackoutPhase::WhiteHold {
                frames_remaining: frames_remaining - 1,
            });
            return;
        }
        if let Err(error) = commit_visible_blackout_recovery(&mut runtime_shell) {
            record_visible_runtime_system_error(&mut runtime_shell, error);
            runtime_shell.visible_blackout_phase = None;
            runtime_shell.screen_fade = None;
            return;
        }
        runtime_shell.visible_blackout_phase = Some(VisibleBlackoutPhase::FadeIn);
        runtime_shell.screen_fade = Some(VisibleScreenFade::new(
            ScriptFadeColor::White,
            ScriptFadeDirection::In,
            8,
        ));
        return;
    }
    let blackout_phase = runtime_shell.visible_blackout_phase;
    let walk_warp_phase = runtime_shell.visible_walk_warp_phase;
    if let Some(fade) = runtime_shell.screen_fade.as_mut() {
        // A stalled host frame must never fast-forward a Game Boy fade.  The
        // original advances one palette step per frame; limiting the input
        // here also prevents the black overlay from snapping through and
        // exposing intermediate half-rendered surfaces after a hiccup.
        fade.advance(time.delta_seconds().min(GAME_TICK_SECONDS));
        if fade.elapsed_frames >= fade.total_frames {
            if walk_warp_phase == Some(VisibleWalkWarpPhase::FadeOut) {
                // MAPSETUP_DOOR exposes the loaded destination beneath the
                // white palette before FadeInFromWhite. Scene scripts and map
                // callbacks resume only after that fade, so their text,
                // emotes, and audio cannot begin invisibly at full white.
                reset_visible_navigation_state(&mut runtime_shell);
                if let Err(error) =
                    suppress_visible_map_name_sign_for_current_map(&mut runtime_shell)
                {
                    record_visible_runtime_system_error(&mut runtime_shell, error);
                    runtime_shell.visible_walk_warp_phase = None;
                    runtime_shell.screen_fade = None;
                    return;
                }
                if let Err(error) = queue_visible_current_music(&mut runtime_shell) {
                    record_visible_runtime_system_error(&mut runtime_shell, error);
                    runtime_shell.visible_walk_warp_phase = None;
                    runtime_shell.screen_fade = None;
                    return;
                }
                runtime_shell.visible_walk_warp_phase = Some(VisibleWalkWarpPhase::FadeIn);
                runtime_shell.screen_fade = Some(VisibleScreenFade::new(
                    ScriptFadeColor::White,
                    ScriptFadeDirection::In,
                    8,
                ));
                mark_runtime_snapshot_dirty(&mut runtime_shell);
                return;
            }
            if walk_warp_phase == Some(VisibleWalkWarpPhase::FadeIn) {
                runtime_shell.visible_walk_warp_phase = None;
                runtime_shell.screen_fade = None;
                let pitfall = runtime_shell
                    .shell
                    .last_frame()
                    .and_then(|frame| frame.warp.as_ref())
                    .is_some_and(|warp| {
                        matches!(
                            warp.trigger.permission,
                            crate::core::world::collision::permissions::PIT
                                | crate::core::world::collision::permissions::PIT_68
                        )
                    });
                let result = if pitfall {
                    begin_visible_pitfall_landing(&mut runtime_shell)
                } else {
                    settle_visible_overworld_arrival(&mut runtime_shell, "walk_warp")
                };
                if let Err(error) = result {
                    record_visible_runtime_system_error(&mut runtime_shell, error);
                    return;
                }
                mark_runtime_snapshot_dirty(&mut runtime_shell);
                return;
            }
            if walk_warp_phase == Some(VisibleWalkWarpPhase::ScriptFadeIn) {
                runtime_shell.visible_walk_warp_phase = None;
                runtime_shell.screen_fade = None;
                if let Err(error) =
                    settle_visible_overworld_arrival(&mut runtime_shell, "script_warp")
                {
                    record_visible_runtime_system_error(&mut runtime_shell, error);
                    return;
                }
                mark_runtime_snapshot_dirty(&mut runtime_shell);
                return;
            }
            if walk_warp_phase == Some(VisibleWalkWarpPhase::MapReloadFadeIn) {
                runtime_shell.visible_walk_warp_phase = None;
                runtime_shell.screen_fade = None;
                if let Some(cursor) = runtime_shell.map_reload_return_cursor.take() {
                    arm_visible_active_script_cursor(
                        &mut runtime_shell,
                        &cursor.source_script,
                        cursor.command_index,
                    );
                }
                if let Err(error) = continue_visible_script_after_prompt(&mut runtime_shell) {
                    record_visible_runtime_system_error(&mut runtime_shell, error);
                    return;
                }
                mark_runtime_snapshot_dirty(&mut runtime_shell);
                return;
            }
            if blackout_phase == Some(VisibleBlackoutPhase::FadeOut) {
                runtime_shell.visible_blackout_phase = Some(VisibleBlackoutPhase::WhiteHold {
                    // TypeScript's 40-frame whiteout wait begins with the
                    // eight-frame palette fade, leaving 32 fully white frames.
                    frames_remaining: 32,
                });
                return;
            }
            if blackout_phase == Some(VisibleBlackoutPhase::FadeIn) {
                runtime_shell.visible_blackout_phase = None;
                runtime_shell.screen_fade = None;
                if let Err(error) =
                    settle_visible_overworld_arrival(&mut runtime_shell, "blackout")
                {
                    record_visible_runtime_system_error(&mut runtime_shell, error);
                    return;
                }
                mark_runtime_snapshot_dirty(&mut runtime_shell);
                return;
            }
            // The final palette must be rendered once, but a FadeOut without
            // a following FadeIn must never leave the entire game black.
            // Keeping a completed overlay until an unrelated script action
            // was the source of the persistent black flashes in field play.
            if completed_screen_fade_should_clear(fade) {
                runtime_shell.screen_fade = None;
            } else {
                fade.terminal_frame_presented = true;
            }
        }
    }
}

fn completed_screen_fade_should_clear(fade: &VisibleScreenFade) -> bool {
    fade.direction == ScriptFadeDirection::In || fade.terminal_frame_presented
}

fn sync_visible_player_sprite(
    runtime_shell: Res<BevyRuntimeShell>,
    mut rendered: ResMut<RenderedViewport>,
    mut players: Query<(&mut Handle<Image>, &mut Sprite, &PlayerSpriteFrames), With<PlayerMarker>>,
) {
    let scripted_standing = runtime_shell
        .visible_script_movement
        .as_ref()
        .is_some_and(|movement| {
            movement.object_id == "PLAYER" && movement.active_uses_standing_frame
        });
    let scripted_tree_shake = runtime_shell
        .visible_script_movement
        .as_ref()
        .is_some_and(|movement| {
            movement.object_id == "PLAYER" && movement.active_tree_shake_duration.is_some()
        });
    let scripted_skyfall_step = runtime_shell
        .visible_script_movement
        .as_ref()
        .is_some_and(|movement| {
            movement.object_id == "PLAYER"
                && movement.active_stationary_effect
                    == Some(VisibleStationaryMovementEffect::SkyfallFall)
                && ((movement.active_stationary_duration.saturating_sub(
                    movement
                        .hold_frames_remaining
                        .min(movement.active_stationary_duration),
                ) / 4)
                    % 2
                    == 1)
        });
    let scripted_rock_smash_action = runtime_shell
        .visible_script_movement
        .as_ref()
        .is_some_and(|movement| {
            movement.object_id == "PLAYER"
                && movement.active_stationary_effect
                    == Some(VisibleStationaryMovementEffect::RockSmash)
                && movement.hold_frames_remaining % 2 == 0
        });
    let walking = scripted_tree_shake
        || scripted_skyfall_step
        || scripted_rock_smash_action
        || (!scripted_standing
            && (runtime_shell.visible_ledge_jump.is_some()
                || (runtime_shell.player_walk_frame_ticks > 0
                    && runtime_shell.player_walk_stride)));
    rendered.player_sprite_walking = Some(walking);
    for (mut texture, mut sprite, frames) in &mut players {
        let next = if walking {
            frames.walking.as_ref().unwrap_or(&frames.standing)
        } else {
            &frames.standing
        };
        sprite.flip_x = walking
            && frames.mirror_walking
            && runtime_shell.player_walk_mirror_stride;
        if texture.id() != next.id() {
            *texture = next.clone();
        }
    }
}

fn sync_visible_ledge_jump(
    runtime_shell: Res<BevyRuntimeShell>,
    rendered: Res<RenderedViewport>,
    mut players: Query<(&mut Transform, &Sprite), With<PlayerMarker>>,
    mut shadows: Query<&mut Transform, (With<LedgeShadowMarker>, Without<PlayerMarker>)>,
) {
    const OFFSETS: [i16; 16] = [
        -4, -6, -8, -10, -11, -12, -12, -12, -11, -10, -9, -8, -6, -4, 0, 0,
    ];
    let Some(jump) = runtime_shell.visible_ledge_jump else {
        return;
    };
    let Some((start_x, start_y)) = rendered.viewport_origin else {
        return;
    };
    let Ok((mut transform, sprite)) = players.get_single_mut() else {
        return;
    };
    let Some(size) = sprite.custom_size else {
        return;
    };
    let Some((from_x, from_y)) = runtime_tile_playfield_position(jump.from, start_x, start_y)
    else {
        return;
    };
    let Some((to_x, to_y)) = runtime_tile_playfield_position(jump.to, start_x, start_y) else {
        return;
    };
    // TypeScript advances two source pixels on each of sixteen updates. Frame
    // 15 is therefore 30 px into the 32 px jump; clearing the state performs
    // the sixteenth two-pixel landing update.
    let progress = f32::from(jump.frame) / 16.0;
    let base_x = from_x + (to_x - from_x) * progress;
    let base_y = from_y + (to_y - from_y) * progress;
    let (sprite_x, sprite_y) = overworld_sprite_position_from_base(base_x, base_y, size);
    let camera_offset = visible_overworld_camera_offset(&rendered, &runtime_shell);
    transform.translation.x = sprite_x + camera_offset.x;
    transform.translation.y =
        sprite_y + camera_offset.y
            - f32::from(OFFSETS[usize::from(jump.frame)]) * BATTLE_HUD_SCALE;
    if let Ok(mut shadow) = shadows.get_single_mut() {
        shadow.translation.x = sprite_x + camera_offset.x;
        shadow.translation.y = sprite_y + camera_offset.y - size.y * 0.5;
    }
}

fn sync_visible_script_jump(
    runtime_shell: Res<BevyRuntimeShell>,
    mut players: Query<&mut Transform, (With<PlayerMarker>, Without<ObjectMarker>)>,
    mut objects: Query<(&VisibleObjectSprite, &mut Transform), With<ObjectMarker>>,
) {
    const OFFSETS: [i16; 16] = [
        -4, -6, -8, -10, -11, -12, -12, -12, -11, -10, -9, -8, -6, -4, 0, 0,
    ];
    let Some(movement) = runtime_shell.visible_script_movement.as_ref() else {
        return;
    };
    let jump_offset = |total: u8, remaining: u8| {
        let elapsed = total.saturating_sub(remaining.min(total));
        let height = u16::from(elapsed)
            .saturating_mul(32)
            .checked_div(u16::from(total.max(1)))
            .unwrap_or(0);
        let index = usize::from((height / 2).min(15));
        -f32::from(OFFSETS[index]) * BATTLE_HUD_SCALE
    };
    if let Some(total) = movement.active_jump_duration {
        let remaining = if movement.object_id == "PLAYER" {
            runtime_shell.player_walk_frame_ticks
        } else {
            runtime_shell.object_walk_frame_ticks
        };
        let y_offset = jump_offset(total, remaining);
        if movement.object_id == "PLAYER" {
            if let Ok(mut transform) = players.get_single_mut() {
                transform.translation.y += y_offset;
            }
        } else if let Some((_, mut transform)) = objects.iter_mut().find(|(object, _)| {
            object.object_identifier.as_deref() == Some(movement.object_id.as_str())
        }) {
            transform.translation.y += y_offset;
        }
    }
    if let (Some(total), Some(follower_id)) = (
        movement.follower_active_jump_duration,
        movement.follower_object_id.as_deref(),
    ) {
        let remaining = if follower_id == "PLAYER" {
            runtime_shell.player_walk_frame_ticks
        } else {
            runtime_shell
                .object_walk_frame_ticks_by_id
                .get(follower_id)
                .copied()
                .unwrap_or(0)
        };
        let y_offset = jump_offset(total, remaining);
        if follower_id == "PLAYER" {
            if let Ok(mut transform) = players.get_single_mut() {
                transform.translation.y += y_offset;
            }
        } else if let Some((_, mut transform)) = objects.iter_mut().find(|(object, _)| {
            object.object_identifier.as_deref() == Some(follower_id)
        }) {
            transform.translation.y += y_offset;
        }
    }
}

fn sync_visible_script_tree_shake(
    runtime_shell: Res<BevyRuntimeShell>,
    mut players: Query<&mut Transform, (With<PlayerMarker>, Without<ObjectMarker>)>,
    mut objects: Query<(&VisibleObjectSprite, &mut Transform), With<ObjectMarker>>,
) {
    const OFFSETS: [i16; 6] = [0, -1, 1, -1, 1, 0];
    let Some(movement) = runtime_shell.visible_script_movement.as_ref() else {
        return;
    };
    let Some(total) = movement.active_tree_shake_duration else {
        return;
    };
    let elapsed = total.saturating_sub(movement.hold_frames_remaining.min(total));
    let frame = elapsed.saturating_sub(1);
    let x_offset = f32::from(OFFSETS[usize::from(frame % OFFSETS.len() as u16)])
        * BATTLE_HUD_SCALE;
    if movement.object_id == "PLAYER" {
        if let Ok(mut transform) = players.get_single_mut() {
            transform.translation.x += x_offset;
        }
        return;
    }
    if let Some((_, mut transform)) = objects.iter_mut().find(|(object, _)| {
        object.object_identifier.as_deref() == Some(movement.object_id.as_str())
    }) {
        transform.translation.x += x_offset;
    }
}

fn sync_visible_stationary_movement_effect(
    runtime_shell: Res<BevyRuntimeShell>,
    mut players: Query<&mut Transform, (With<PlayerMarker>, Without<ObjectMarker>)>,
    mut objects: Query<(&VisibleObjectSprite, &mut Transform), With<ObjectMarker>>,
) {
    let Some(movement) = runtime_shell.visible_script_movement.as_ref() else {
        return;
    };
    let y_offset = -f32::from(movement.stationary_y_offset) * BATTLE_HUD_SCALE;
    if movement.object_id == "PLAYER" {
        if let Ok(mut transform) = players.get_single_mut() {
            transform.translation.y += y_offset;
        }
        return;
    }
    if let Some((_, mut transform)) = objects.iter_mut().find(|(object, _)| {
        object.object_identifier.as_deref() == Some(movement.object_id.as_str())
    }) {
        transform.translation.y += y_offset;
    }
}

fn sync_visible_object_sprites(
    runtime_shell: Res<BevyRuntimeShell>,
    mut rendered: ResMut<RenderedViewport>,
    mut objects: Query<(&VisibleObjectSprite, &mut Handle<Image>, &mut Sprite)>,
) {
    let walking_phase = (runtime_shell.object_walk_frame_ticks > 0
        && runtime_shell.object_walk_stride)
        || runtime_shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| {
                movement.active_tree_shake_duration.is_some()
                    || (movement.active_stationary_effect
                        == Some(VisibleStationaryMovementEffect::SkyfallFall)
                        && ((movement.active_stationary_duration.saturating_sub(
                            movement
                                .hold_frames_remaining
                                .min(movement.active_stationary_duration),
                        ) / 4)
                            % 2
                            == 1))
                    || (movement.active_stationary_effect
                        == Some(VisibleStationaryMovementEffect::RockSmash)
                        && movement.hold_frames_remaining % 2 == 0)
            });
    rendered.object_sprite_walking = Some(walking_phase);
    for (frames, mut texture, mut sprite) in &mut objects {
        if !frames.animated {
            continue;
        }
        let object_is_moving = frames.object_identifier.as_ref().is_some_and(|object_id| {
            runtime_shell.object_walk_from.contains_key(object_id)
                || runtime_shell
                    .trainer_walk_from
                    .as_ref()
                    .is_some_and(|(walking_id, _)| walking_id == object_id)
                || runtime_shell
                    .visible_script_movement
                    .as_ref()
                    .is_some_and(|movement| {
                        (movement.active_tree_shake_duration.is_some()
                            || movement.active_stationary_effect
                                == Some(VisibleStationaryMovementEffect::SkyfallFall)
                            || movement.active_stationary_effect
                                == Some(VisibleStationaryMovementEffect::RockSmash))
                            && movement.object_id == *object_id
                    })
        });
        let scripted_standing = runtime_shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| {
                (movement.active_uses_standing_frame
                    && frames.object_identifier.as_deref() == Some(movement.object_id.as_str()))
                    || (movement.follower_active_uses_standing_frame
                        && frames.object_identifier.as_deref()
                            == movement.follower_object_id.as_deref())
            });
        let autonomous_phase = frames.object_identifier.as_ref().and_then(|object_id| {
            (runtime_shell.object_walk_from.contains_key(object_id)
                || runtime_shell
                    .trainer_walk_from
                    .as_ref()
                    .is_some_and(|(walking_id, _)| walking_id == object_id))
            .then(|| runtime_shell.object_walk_phases.get(object_id).copied().unwrap_or(1))
        });
        let autonomous_action = autonomous_phase.is_some_and(|phase| matches!(phase, 1 | 3));
        let scripted_action = walking_phase
            && object_is_moving
            && autonomous_phase.is_none();
        let action_frame = !scripted_standing && (autonomous_action || scripted_action);
        let next = if action_frame {
            frames.walking.as_ref().unwrap_or(&frames.standing)
        } else {
            &frames.standing
        };
        sprite.flip_x = action_frame
            && frames.mirror_walking
            && autonomous_phase == Some(3);
        if texture.id() != next.id() {
            *texture = next.clone();
        }
    }
}

fn render_screen_fade_overlay(
    runtime_shell: Res<BevyRuntimeShell>,
    mut overlays: Query<&mut Sprite, With<ScreenFadeOverlay>>,
) {
    let Ok(mut sprite) = overlays.get_single_mut() else {
        return;
    };
    let Some(fade) = runtime_shell.screen_fade else {
        if let Some(flash) = runtime_shell.visible_flash_animation {
            let step = if flash.frame <= 8 {
                flash.frame
            } else {
                16_u8.saturating_sub(flash.frame)
            };
            sprite.color = Color::rgba(1.0, 1.0, 1.0, f32::from(step) / 8.0);
            return;
        }
        sprite.color = Color::rgba(0.0, 0.0, 0.0, 0.0);
        return;
    };
    let (r, g, b) = match fade.color {
        ScriptFadeColor::Black => (0.0, 0.0, 0.0),
        ScriptFadeColor::White => (1.0, 1.0, 1.0),
    };
    sprite.color = Color::rgba(r, g, b, f32::from(fade.alpha) / 255.0);
}

fn render_poison_flash_overlay(
    runtime_shell: Res<BevyRuntimeShell>,
    mut overlays: Query<&mut Sprite, With<PoisonFlashOverlay>>,
) {
    let Ok(mut sprite) = overlays.get_single_mut() else {
        return;
    };
    let alpha = (176.0 * f32::from(runtime_shell.poison_flash_frames_remaining) / 4.0) / 255.0;
    sprite.color = Color::rgba(230.0 / 255.0, 173.0 / 255.0, 1.0, alpha);
}

fn take_visible_pending_shop_request(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:shop")?;
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::Shop)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending shop {:?}", request));
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn clear_visible_menu_coords(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let clear = runtime_shell.shell.clear_menu_coords()?;
    runtime_shell.last_audio_events.push(format!(
        "cleared menu coords {:?} checksum={:?}",
        clear.coords, clear.state_checksum
    ));
    Ok(())
}

fn select_visible_elevator_floor(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !has_visible_elevator_prompt(&snapshot, runtime_shell) {
        return handle_visible_no_elevator_prompt(runtime_shell, "confirm");
    }
    let (elevator_index, floor_index) = selected_visible_elevator_option(runtime_shell, &snapshot)?;
    let elevators = visible_elevator_prompt_options(&snapshot, runtime_shell);
    let elevator = elevators[elevator_index];
    if elevator.floors.is_empty() {
        anyhow::bail!("elevator {} has no floors", elevator.data_label);
    }
    let floor = &elevator.floors[floor_index];
    let map_name = elevator.map_name.clone();
    let data_label = elevator.data_label.clone();
    let source_script = elevator.source_script.clone();
    let elevator_command_index = elevator.elevator_command_index;
    let floor_floor_index = floor.floor_index;
    let floor_name = floor.floor.clone();
    let floor_warp = floor.warp;
    let target_map = floor.target_map.clone();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:elevator:{}:{}:{}:{}:{}:{}",
            map_name.as_str(),
            data_label.as_str(),
            source_script.as_str(),
            elevator_command_index,
            floor_floor_index,
            target_map.as_str()
        ),
    )?;
    let selected = if let Some(cursor) = visible_active_compiled_script_cursor(runtime_shell) {
        runtime_shell
            .shell
            .select_elevator_floor_and_run_compiled_script(
                map_name,
                data_label,
                source_script,
                elevator_command_index,
                floor_floor_index,
                floor_name,
                floor_warp,
                target_map,
                Some(cursor),
                256,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )?
    } else {
        let selection = runtime_shell.shell.select_elevator_floor(
            map_name,
            data_label,
            source_script,
            elevator_command_index,
            floor_floor_index,
            floor_name,
            floor_warp,
            target_map,
        )?;
        crate::RuntimeElevatorFloorCompiledScriptRun {
            selection,
            run: crate::RuntimeCompiledScriptRun {
                steps: Vec::new(),
                next_cursor: None,
                boundary: None,
                ended: false,
            },
        }
    };
    let selection = selected.selection;
    runtime_shell.last_audio_events.push(format!(
        "selected elevator {}/{} floor={}/{} {} target={} script_value={} resumed_steps={} checksum={:?}",
        elevator_index + 1,
        elevators.len(),
        floor_index + 1,
        elevator.floors.len(),
        selection.floor,
        selection.target_map,
        selection.script_value,
        selected.run.steps.len(),
        selection.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.elevator_cursor = None;
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &selected.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, selected.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn drain_visible_audio_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drain = runtime_shell.shell.drain_resolved_audio_events()?;
    apply_resolved_audio_drain(runtime_shell, drain);
    Ok(())
}

fn drain_visible_map_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_event_queue(RuntimeScriptEventQueue::Map)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained map events {:?}", drained));
    Ok(())
}

fn drain_visible_text_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_event_queue(RuntimeScriptEventQueue::Text)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained text events {:?}", drained));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn drain_visible_misc_script_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:drain:misc_events")?;
    drain_visible_non_audio_script_events_without_record(runtime_shell)?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn drain_visible_non_audio_script_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:drain:non_audio_events")?;
    drain_visible_non_audio_script_events_without_record(runtime_shell)?;
    Ok(())
}

fn drain_visible_non_audio_script_events_without_record(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let script_events = runtime_shell.shell.script_events_snapshot();
    let mut queues = Vec::new();
    if !script_events.graphics_events.is_empty() {
        queues.push(RuntimeScriptEventQueue::Graphics);
    }
    if !script_events.money_events.is_empty() {
        queues.push(RuntimeScriptEventQueue::Money);
    }
    if !script_events.map_events.is_empty() {
        queues.push(RuntimeScriptEventQueue::Map);
    }
    if !script_events.control_events.is_empty() {
        queues.push(RuntimeScriptEventQueue::Control);
    }
    if !script_events.shop_events.is_empty() {
        queues.push(RuntimeScriptEventQueue::Shop);
    }
    if !script_events.item_use_events.is_empty() {
        queues.push(RuntimeScriptEventQueue::ItemUse);
    }
    for queue in queues {
        let drained = runtime_shell.shell.drain_script_event_queue(queue)?;
        runtime_shell
            .last_audio_events
            .push(format!("drained script event {:?}: {:?}", queue, drained));
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn drain_visible_delays(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_script_delay_frames.is_none() {
        let snapshot = runtime_shell.shell.snapshot()?;
        if let Some(delay) = snapshot.script_events.pending_delays.first() {
            runtime_shell.visible_script_delay_frames = Some(delay.frames.max(1));
            return Ok(());
        }
    }
    if runtime_shell
        .visible_script_delay_frames
        .is_some_and(|frames| frames > 0)
    {
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:drain:delays")?;
    let drained = runtime_shell
        .shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingDelay)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained runtime delays {:?}", drained));
    runtime_shell.visible_script_delay_frames = None;
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn drain_visible_earthquakes(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_earthquake.is_none() {
        let snapshot = runtime_shell.shell.snapshot()?;
        if let Some(earthquake) = snapshot.script_events.pending_earthquakes.first() {
            runtime_shell.visible_earthquake = Some(VisibleEarthquake {
                intensity: 1_u16 << ((earthquake.parameter >> 6) & 0x3),
                frames_remaining: earthquake.sleep_frames.max(1),
                phase: 0,
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
    }
    if runtime_shell
        .visible_earthquake
        .is_some_and(|earthquake| earthquake.frames_remaining > 0)
    {
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:drain:earthquakes")?;
    let drained = runtime_shell
        .shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingEarthquake)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained runtime earthquakes {:?}", drained));
    runtime_shell.visible_earthquake = None;
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn drain_visible_emotes(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.visible_overworld_emote.is_none() {
        let snapshot = runtime_shell.shell.snapshot()?;
        if let Some(emote) = snapshot.script_events.pending_emotes.first() {
            runtime_shell.visible_overworld_emote = Some(VisibleOverworldEmote {
                emote: emote.emote.clone(),
                object: emote.object.clone(),
                frames_remaining: emote.duration.max(1),
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
    }
    if runtime_shell
        .visible_overworld_emote
        .as_ref()
        .is_some_and(|emote| emote.frames_remaining > 0)
    {
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:drain:emotes")?;
    let drained = runtime_shell
        .shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingEmote)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained runtime emotes {:?}", drained));
    runtime_shell.visible_overworld_emote = None;
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn drain_visible_misc_runtime_queues(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:drain:misc_runtime_queues")?;
    for queue in [
        RuntimeScriptRuntimeQueue::PendingEarthquake,
        RuntimeScriptRuntimeQueue::PendingEmote,
        RuntimeScriptRuntimeQueue::Command,
        RuntimeScriptRuntimeQueue::Stack,
        RuntimeScriptRuntimeQueue::CallStack,
        RuntimeScriptRuntimeQueue::DeferredScript,
    ] {
        let drained = runtime_shell.shell.drain_script_runtime_queue(queue)?;
        runtime_shell
            .last_audio_events
            .push(format!("drained runtime queue {:?}: {:?}", queue, drained));
    }
    let linked_menu_results = runtime_shell.shell.drain_linked_menu_results();
    if !linked_menu_results.is_empty() {
        runtime_shell.last_audio_events.push(format!(
            "drained linked menu results {linked_menu_results:?}"
        ));
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn drain_visible_runtime_records(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    for queue in [
        RuntimeScriptRuntimeRecordQueue::VariableWrite,
        RuntimeScriptRuntimeRecordQueue::Effect,
        RuntimeScriptRuntimeRecordQueue::AsmDirective,
        RuntimeScriptRuntimeRecordQueue::NumericBufferWrite,
        RuntimeScriptRuntimeRecordQueue::ElevatorFloor,
        RuntimeScriptRuntimeRecordQueue::DecorationDescription,
        RuntimeScriptRuntimeRecordQueue::SpecialPhoneCall,
        RuntimeScriptRuntimeRecordQueue::CompletedTrade,
        RuntimeScriptRuntimeRecordQueue::CatchTutorial,
        RuntimeScriptRuntimeRecordQueue::CheckedMailTarget,
        RuntimeScriptRuntimeRecordQueue::GivenMailTarget,
    ] {
        let drained = runtime_shell
            .shell
            .drain_script_runtime_record_queue(queue)?;
        runtime_shell
            .last_audio_events
            .push(format!("drained runtime record {:?}: {:?}", queue, drained));
    }
    Ok(())
}

fn consume_visible_runtime_flag(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let scripts = &snapshot.script_events;
    let flag = if scripts.map_music_restart_disabled {
        RuntimeScriptRuntimeFlag::MapMusicRestartDisabled
    } else if scripts.map_music_requested {
        RuntimeScriptRuntimeFlag::MapMusicRequested
    } else if scripts.waiting_for_sound_effect {
        RuntimeScriptRuntimeFlag::WaitingForSoundEffect
    } else if scripts.warp_check_requested {
        RuntimeScriptRuntimeFlag::WarpCheckRequested
    } else if scripts.item_notify_queued {
        RuntimeScriptRuntimeFlag::ItemNotifyQueued
    } else if scripts.warp_sound_queued {
        RuntimeScriptRuntimeFlag::WarpSoundQueued
    } else if scripts.teleport_from_queued {
        RuntimeScriptRuntimeFlag::TeleportFromQueued
    } else if scripts.hall_of_fame_requested {
        RuntimeScriptRuntimeFlag::HallOfFameRequested
    } else if scripts.credits_requested {
        RuntimeScriptRuntimeFlag::CreditsRequested
    } else if scripts.reset_requested {
        RuntimeScriptRuntimeFlag::ResetRequested
    } else if scripts.menu_2d_requested {
        RuntimeScriptRuntimeFlag::Menu2dRequested
    } else if scripts.version_check_requested {
        RuntimeScriptRuntimeFlag::VersionCheckRequested
    } else if scripts.battle_tower_text.is_some() {
        RuntimeScriptRuntimeFlag::BattleTowerText
    } else {
        return handle_visible_no_runtime_flag(runtime_shell, "consume");
    };
    consume_visible_runtime_flag_kind(runtime_shell, flag)
}

fn visible_auto_runtime_flag(snapshot: &RuntimeShellSnapshot) -> Option<RuntimeScriptRuntimeFlag> {
    let scripts = &snapshot.script_events;
    if scripts.map_music_restart_disabled {
        Some(RuntimeScriptRuntimeFlag::MapMusicRestartDisabled)
    } else if scripts.map_music_requested {
        Some(RuntimeScriptRuntimeFlag::MapMusicRequested)
    } else if scripts.waiting_for_sound_effect {
        Some(RuntimeScriptRuntimeFlag::WaitingForSoundEffect)
    } else if scripts.warp_check_requested {
        Some(RuntimeScriptRuntimeFlag::WarpCheckRequested)
    } else if scripts.item_notify_queued {
        Some(RuntimeScriptRuntimeFlag::ItemNotifyQueued)
    } else if scripts.warp_sound_queued {
        Some(RuntimeScriptRuntimeFlag::WarpSoundQueued)
    } else if scripts.teleport_from_queued {
        Some(RuntimeScriptRuntimeFlag::TeleportFromQueued)
    } else if scripts.hall_of_fame_requested {
        Some(RuntimeScriptRuntimeFlag::HallOfFameRequested)
    } else if scripts.credits_requested {
        Some(RuntimeScriptRuntimeFlag::CreditsRequested)
    } else if scripts.reset_requested {
        Some(RuntimeScriptRuntimeFlag::ResetRequested)
    } else if scripts.menu_2d_requested
        && !snapshot.ui.menu.as_ref().is_some_and(|menu| {
            menu.menu_2d_requested
                && menu
                    .layout
                    .vertical_menus
                    .iter()
                    .any(|menu| menu.two_dimensional && !menu.options.is_empty())
        })
    {
        Some(RuntimeScriptRuntimeFlag::Menu2dRequested)
    } else if scripts.version_check_requested {
        Some(RuntimeScriptRuntimeFlag::VersionCheckRequested)
    } else if scripts.battle_tower_text.is_some() {
        Some(RuntimeScriptRuntimeFlag::BattleTowerText)
    } else {
        None
    }
}

fn consume_visible_runtime_flag_kind(
    runtime_shell: &mut BevyRuntimeShell,
    flag: RuntimeScriptRuntimeFlag,
) -> Result<()> {
    let consumed = runtime_shell.shell.consume_script_runtime_flag(flag)?;
    if matches!(consumed, RuntimeScriptRuntimeFlagValue::MapMusicRequested) {
        reset_visible_music_state(runtime_shell);
        queue_visible_current_music(runtime_shell)?;
    }
    if matches!(consumed, RuntimeScriptRuntimeFlagValue::VersionCheckRequested) {
        trim_event_log(&mut runtime_shell.last_audio_events);
        return continue_visible_script_after_prompt(runtime_shell);
    }
    if matches!(consumed, RuntimeScriptRuntimeFlagValue::ItemNotifyQueued) {
        let item_id = runtime_shell
            .pending_item_notification
            .take()
            .context("itemnotify has no retained item grant")?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let item = snapshot
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .with_context(|| format!("itemnotify item {item_id} is missing from the catalog"))?;
        let pocket = match item.pocket.as_str() {
            "ITEM" => "ITEM POCKET",
            "KEY_ITEM" => "KEY POCKET",
            "BALL" => "BALL POCKET",
            "TM_HM" => "TM POCKET",
            other => anyhow::bail!("itemnotify item {item_id} has invalid pocket {other}"),
        };
        runtime_shell.field_notice = Some(format!(
            "{} put the\n{} in\nthe {}.",
            snapshot.trainer.player_name,
            item.name.replace('_', " "),
            pocket
        ));
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    runtime_shell
        .last_audio_events
        .push(format!("consumed runtime flag {:?}", consumed));
    if matches!(consumed, RuntimeScriptRuntimeFlagValue::CreditsRequested) {
        let snapshot = runtime_shell.shell.snapshot()?;
        let allow_skip = snapshot
            .progression
            .active_engine_flags
            .contains("STATUSFLAGS_HALL_OF_FAME_F")
            || snapshot.progression.hall_of_fame.count > 0
            || !snapshot.progression.hall_of_fame.entries.is_empty();
        open_visible_credits_screen(runtime_shell, allow_skip)?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if matches!(consumed, RuntimeScriptRuntimeFlagValue::HallOfFameRequested) {
        // `halloffame` is a gameplay boundary, not an acknowledgement window.
        // The ASM records the party, runs the Hall of Fame sequence, and then
        // enters credits without waiting for an external/debug input.  The
        // compact Rust presentation uses the same credits state machine after
        // the canonical core record has been committed, so a natural Elite
        // Four completion cannot strand the player at a special-boundary
        // placeholder.
        let hall_of_fame = &runtime_shell.shell.snapshot()?.progression.hall_of_fame;
        // The core has already inserted the newly crowned team. A second
        // entry or a count above one therefore proves the source status flag
        // was set before this Hall of Fame call; the first clear remains
        // unskippable exactly as the old wStatusFlags value passed in B.
        let allow_skip = hall_of_fame.count > 1 || hall_of_fame.entries.len() > 1;
        open_visible_credits_screen(runtime_shell, allow_skip)?;
        let credits = runtime_shell
            .credits_screen
            .as_mut()
            .context("Hall of Fame credits did not open a credits screen")?;
        credits.resume_game_timer_on_exit = true;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if let RuntimeScriptRuntimeFlagValue::BlackoutMod(map_constant) = &consumed {
        // Script_blackoutmod only updates wLastSpawnMapGroup/Number and then
        // continues. Resolve the compiled spawn binding so Rust's typed
        // blackout path observes the same destination later.
        let spawn = runtime_shell
            .shell
            .snapshot()?
            .spawn_points
            .into_iter()
            .find(|spawn| spawn.map_constant == *map_constant)
            .with_context(|| {
                format!("blackoutmod map {map_constant} has no compiled spawn point")
            })?;
        let state = runtime_shell.shell.session_mut().state_mut();
        state.last_spawn_identifier = Some(spawn.identifier);
        state.script_runtime.variables.insert(
            "wLastSpawnMapGroup".to_string(),
            spawn.group_id.to_string(),
        );
        state.script_runtime.variables.insert(
            "wLastSpawnMapNumber".to_string(),
            spawn.map_id.to_string(),
        );
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return continue_visible_script_after_prompt(runtime_shell);
    }
    if let Some(boundary) = runtime_flag_boundary_display(&consumed) {
        let label = boundary.label.clone();
        runtime_shell.special_boundary = Some(boundary);
        set_shell_action_status(runtime_shell, format!("RUNTIME {label}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)
}

fn runtime_flag_boundary_display(
    flag: &RuntimeScriptRuntimeFlagValue,
) -> Option<SpecialBoundaryDisplay> {
    match flag {
        RuntimeScriptRuntimeFlagValue::HallOfFameRequested => Some(SpecialBoundaryDisplay {
            label: "HallOfFame".to_string(),
            details: vec!["script requested Hall of Fame sequence".to_string()],
        }),
        RuntimeScriptRuntimeFlagValue::ResetRequested => Some(SpecialBoundaryDisplay {
            label: "Reset".to_string(),
            details: vec!["script requested runtime reset".to_string()],
        }),
        RuntimeScriptRuntimeFlagValue::Menu2dRequested => Some(SpecialBoundaryDisplay {
            label: "Menu2D".to_string(),
            details: vec!["script requested 2D menu surface".to_string()],
        }),
        RuntimeScriptRuntimeFlagValue::BattleTowerText(text) => Some(SpecialBoundaryDisplay {
            label: "BattleTowerText".to_string(),
            details: vec![text.clone()],
        }),
        RuntimeScriptRuntimeFlagValue::MapMusicRestartDisabled
        | RuntimeScriptRuntimeFlagValue::MapMusicRequested
        | RuntimeScriptRuntimeFlagValue::WaitingForSoundEffect
        | RuntimeScriptRuntimeFlagValue::VersionCheckRequested
        | RuntimeScriptRuntimeFlagValue::WarpCheckRequested
        | RuntimeScriptRuntimeFlagValue::ItemNotifyQueued
        | RuntimeScriptRuntimeFlagValue::WarpSoundQueued
        | RuntimeScriptRuntimeFlagValue::TeleportFromQueued
        | RuntimeScriptRuntimeFlagValue::BlackoutMod(_)
        | RuntimeScriptRuntimeFlagValue::CreditsRequested => None,
    }
}

fn take_visible_script_value(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    take_visible_runtime_memory_value(runtime_shell, RuntimeScriptRuntimeMemoryValue::ScriptValue)
}

fn take_visible_last_special_routine(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    take_visible_runtime_memory_value(
        runtime_shell,
        RuntimeScriptRuntimeMemoryValue::LastSpecialRoutine,
    )
}

fn take_visible_last_talked_object(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    take_visible_runtime_memory_value(
        runtime_shell,
        RuntimeScriptRuntimeMemoryValue::LastTalkedObject,
    )
}

fn take_visible_runtime_memory_value(
    runtime_shell: &mut BevyRuntimeShell,
    value: RuntimeScriptRuntimeMemoryValue,
) -> Result<()> {
    let taken = runtime_shell
        .shell
        .take_script_runtime_memory_value(value)?;
    runtime_shell
        .last_audio_events
        .push(format!("took runtime memory {:?}: {:?}", value, taken));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn remove_selected_runtime_variable(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (_, _, key) = selected_btree_key(
        runtime_shell,
        "script runtime variable",
        &snapshot.script_events.variables,
    )?;
    remove_visible_runtime_memory_entry(
        runtime_shell,
        RuntimeScriptRuntimeMemoryEntry::Variable,
        key,
    )
}

fn remove_selected_runtime_memory(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (_, _, key) = selected_btree_key(
        runtime_shell,
        "script runtime memory entry",
        &snapshot.script_events.memory,
    )?;
    remove_visible_runtime_memory_entry(runtime_shell, RuntimeScriptRuntimeMemoryEntry::Memory, key)
}

fn remove_selected_named_buffer(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (_, _, key) = selected_btree_key(
        runtime_shell,
        "script runtime named buffer",
        &snapshot.script_events.named_buffers,
    )?;
    remove_visible_runtime_memory_entry(
        runtime_shell,
        RuntimeScriptRuntimeMemoryEntry::NamedBuffer,
        key,
    )
}

fn remove_visible_runtime_memory_entry(
    runtime_shell: &mut BevyRuntimeShell,
    entry: RuntimeScriptRuntimeMemoryEntry,
    key: String,
) -> Result<()> {
    let removed = runtime_shell
        .shell
        .remove_script_runtime_memory_entry(entry, key)?;
    runtime_shell
        .last_audio_events
        .push(format!("removed runtime memory {:?}: {:?}", entry, removed));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn initialize_visible_phone_numbers(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "phone:init_permanent_numbers")?;
    let initialized = runtime_shell.shell.initialize_permanent_phone_numbers()?;
    runtime_shell.last_audio_events.push(format!(
        "initialized permanent phone numbers={} checksum={:?}",
        initialized.inserted.len(),
        initialized.state_checksum
    ));
    Ok(())
}

fn apply_selected_phone_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, command) = selected_script_command_key(
        runtime_shell,
        "phone",
        runtime_shell
            .shell
            .script_phone_command_keys()
            .into_iter()
            .filter(|command| command.map_name == current_map)
            .collect(),
    )?;
    let inputs = ScriptPhoneInputs { accepted: None };
    let runtime_inputs = explicit_compiled_script_runtime_inputs(
        runtime_shell,
        &command.source_script,
        command.command_index,
    )?;
    let applied = runtime_shell.shell.apply_compiled_script_command(
        &command.map_name,
        &command.source_script,
        command.command_index,
        runtime_inputs,
        inputs,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "phone command {}/{} {} contact={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        command.command,
        command.contact_id,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    Ok(())
}

fn apply_selected_swarm_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, command) = selected_script_command_key(
        runtime_shell,
        "swarm",
        runtime_shell
            .shell
            .script_swarm_command_keys()
            .into_iter()
            .filter(|command| command.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &command.source_script,
        command.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "swarm command {}/{} {} token={} map_id={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        command.command,
        command.swarm_token,
        command.map_id,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    Ok(())
}

fn shift_visible_script_command_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) {
    let before = runtime_shell.script_command_cursor;
    runtime_shell.script_command_cursor = if delta.is_negative() {
        before.saturating_sub((-delta) as usize)
    } else {
        before.saturating_add(delta as usize)
    };
    runtime_shell.last_audio_events.push(format!(
        "script command cursor {}->{}",
        before, runtime_shell.script_command_cursor
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.last_error = None;
}

fn shift_visible_script_command_cursor_bounded(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
    option_count: usize,
) {
    if option_count == 0 {
        return;
    }
    let before = runtime_shell.script_command_cursor % option_count;
    let next = if delta.is_negative() {
        before
            .checked_sub(delta.unsigned_abs() % option_count)
            .unwrap_or_else(|| option_count - ((delta.unsigned_abs() - before) % option_count))
            % option_count
    } else {
        (before + (delta as usize % option_count)) % option_count
    };
    runtime_shell.script_command_cursor = next;
    runtime_shell.last_audio_events.push(format!(
        "script command cursor {}->{}",
        before, runtime_shell.script_command_cursor
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.last_error = None;
}

fn selected_script_command_key<T: Clone>(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    keys: Vec<T>,
) -> Result<(usize, usize, T)> {
    if keys.is_empty() {
        anyhow::bail!("current map has no compiled {family} command");
    }
    let selected_index = runtime_shell.script_command_cursor % keys.len();
    Ok((selected_index, keys.len(), keys[selected_index].clone()))
}

fn selected_btree_key<T>(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    keys: &std::collections::BTreeMap<String, T>,
) -> Result<(usize, usize, String)> {
    if keys.is_empty() {
        anyhow::bail!("compiled pack declares no {family}");
    }
    let selected_index = runtime_shell.script_command_cursor % keys.len();
    let key = keys
        .keys()
        .nth(selected_index)
        .with_context(|| {
            format!(
                "compiled {family} cursor selected index {selected_index} outside {} keys",
                keys.len()
            )
        })?
        .clone();
    Ok((selected_index, keys.len(), key))
}

fn selected_declared_special<T: Copy>(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    declared: &std::collections::BTreeMap<String, crystal_assets::SpecialRoutineRule>,
    candidates: &[T],
    routine: fn(T) -> &'static str,
) -> Result<(usize, usize, T)> {
    let visible = candidates
        .iter()
        .copied()
        .filter(|candidate| declared.contains_key(routine(*candidate)))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        anyhow::bail!("compiled pack declares no Bevy-visible {family} special");
    }
    let selected_index = runtime_shell.script_command_cursor % visible.len();
    Ok((selected_index, visible.len(), visible[selected_index]))
}

fn selected_declared_special_routine(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    declared: &std::collections::BTreeMap<String, crystal_assets::SpecialRoutineRule>,
    candidates: &[&'static str],
) -> Result<(usize, usize, &'static str)> {
    let visible = candidates
        .iter()
        .copied()
        .filter(|routine| declared.contains_key(*routine))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        anyhow::bail!("compiled pack declares no Bevy-visible {family} special");
    }
    let selected_index = runtime_shell.script_command_cursor % visible.len();
    Ok((selected_index, visible.len(), visible[selected_index]))
}

fn grant_selected_script_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script item grant",
        runtime_shell
            .shell
            .script_item_grant_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let granted = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script item grant {}/{} command={} item={} quantity={} verbose={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        key.quantity,
        key.verbose,
        granted.result.result_tag(),
        granted.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn check_selected_script_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (selected_index, selected_len, key) =
        selected_script_item_access_key(runtime_shell, "checkitem")?;
    let checked = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script item check {}/{} command={} item={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        checked.result.result_tag(),
        checked.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn take_selected_script_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (selected_index, selected_len, key) =
        selected_script_item_access_key(runtime_shell, "takeitem")?;
    let taken = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script item take {}/{} command={} item={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        taken.result.result_tag(),
        taken.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn selected_script_item_access_key(
    runtime_shell: &mut BevyRuntimeShell,
    command: &str,
) -> Result<(usize, usize, crate::RuntimeScriptItemAccessKey)> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    selected_script_command_key(
        runtime_shell,
        command,
        runtime_shell
            .shell
            .script_item_access_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map && key.command == command)
            .collect(),
    )
}

fn apply_selected_script_economy_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script economy",
        runtime_shell
            .shell
            .script_economy_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script economy {}/{} command={} account={:?} amount_tokens={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.account,
        key.amount_tokens,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn pickup_selected_script_field_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script field pickup",
        runtime_shell
            .shell
            .script_field_pickup_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let pickup = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script field pickup {}/{} command={} item={:?} quantity={} event={:?} fruit_tree={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        key.quantity,
        key.event_flag,
        key.fruit_tree_id,
        pickup.result.result_tag(),
        pickup.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_flag_mutation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script flag mutation",
        runtime_shell
            .shell
            .script_flag_command_keys()
            .into_iter()
            .filter(|key| {
                key.map_name == current_map
                    && matches!(
                        key.command.as_str(),
                        "setevent"
                            | "clearevent"
                            | "set_flag"
                            | "clear_flag"
                            | "setflag"
                            | "clearflag"
                            | "setengineflag"
                    )
            })
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script flag mutation {}/{} command={} flag={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.flag_id,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn check_selected_script_flag(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script flag check",
        runtime_shell
            .shell
            .script_flag_command_keys()
            .into_iter()
            .filter(|key| {
                key.map_name == current_map
                    && matches!(
                        key.command.as_str(),
                        "checkevent" | "checkflag" | "check_flag"
                    )
            })
            .collect(),
    )?;
    let checked = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script flag check {}/{} command={} flag={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.flag_id,
        checked.result.result_tag(),
        checked.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_scene_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script scene",
        runtime_shell
            .shell
            .script_scene_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script scene {}/{} command={} map={:?} scene={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.map_id,
        key.scene_id,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_compiled_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<RuntimeMutationOutcome> {
    let runtime_inputs =
        explicit_compiled_script_runtime_inputs(runtime_shell, source_script, command_index)?;
    let phone_inputs =
        explicit_compiled_script_phone_inputs(runtime_shell, source_script, command_index);
    let origin_map_name = runtime_shell.shell.current_map_name().to_string();
    runtime_shell.shell.apply_compiled_script_command(
        &origin_map_name,
        source_script,
        command_index,
        runtime_inputs,
        phone_inputs,
    )
}

fn apply_selected_script_block_change(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script block change",
        runtime_shell
            .shell
            .script_block_change_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script block change {}/{} x={} y={} block={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.x,
        key.y,
        key.block_id,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_audio_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script audio",
        runtime_shell
            .shell
            .script_audio_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script audio {}/{} command={} audio={:?} fade={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.audio_id,
        key.fade_frames,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_text_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script text",
        runtime_shell
            .shell
            .script_text_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script text {}/{} command={} label={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.text_label,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_variable_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script variable",
        runtime_shell
            .shell
            .script_variable_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script variable {}/{} command={} target={:?} values={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.target,
        key.value_tokens,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_control_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script control",
        runtime_shell
            .shell
            .script_control_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script control {}/{} command={} compare={:?} target={:?} resolved={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.compare_value,
        key.target_label,
        key.resolved_target_script,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_object_mutation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script object",
        runtime_shell
            .shell
            .script_object_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script object {}/{} command={} object={:?} target={:?} xy=({:?},{:?}) dir={:?} movement={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.object_id,
        key.target_object_id,
        key.x,
        key.y,
        key.direction,
        key.movement,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_map_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script map",
        runtime_shell
            .shell
            .script_map_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script map {}/{} command={} target={:?} xy=({:?},{:?}) facing={:?} setup={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.target_map,
        key.x,
        key.y,
        key.facing,
        key.map_setup,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_selected_script_shop(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script shop",
        runtime_shell
            .shell
            .script_shop_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let opened = apply_selected_compiled_script_command(
        runtime_shell,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script shop {}/{} command={} mart_type={} mart_id={} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.mart_type,
        key.mart_id,
        opened.result.result_tag(),
        opened.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_runtime_command_named(
    runtime_shell: &mut BevyRuntimeShell,
    command_name: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, command) = selected_script_command_key(
        runtime_shell,
        command_name,
        runtime_shell
            .shell
            .script_runtime_command_keys()
            .into_iter()
            .filter(|command| command.map_name == current_map && command.command == command_name)
            .collect(),
    )?;
    let runtime_inputs = explicit_script_runtime_inputs(
        runtime_shell,
        &command.command,
        &command.args,
        command.command_index,
    )?;
    let applied = runtime_shell.shell.apply_compiled_script_command(
        &command.map_name,
        &command.source_script,
        command.command_index,
        runtime_inputs,
        ScriptPhoneInputs::default(),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "runtime command {}/{} {} args={:?} result={} checksum={:?}",
        selected_index + 1,
        selected_len,
        command.command,
        command.args,
        applied.result.result_tag(),
        applied.state_checksum
    ));
    if command.command == "catchtutorial" {
        // `catchtutorial` occupies the same compiled battle boundary as
        // `startbattle`: the preceding `loadwildmon` has already supplied the
        // species and level.  Opening a host acknowledgement here stranded the
        // Route 29 script without ever starting Dude's demonstration.
        start_visible_catch_tutorial(
            runtime_shell,
            &command.source_script,
            command.command_index,
        )?;
        return Ok(());
    }
    if let RuntimeMutationResult::ScriptRuntimeApplied(command, _) = &applied.result {
        open_visible_script_runtime_boundary_if_needed(runtime_shell, command)?;
    }
    Ok(())
}

fn apply_selected_trade_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_selected_runtime_command_named(runtime_shell, "trade")
}

fn apply_selected_catch_tutorial_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_selected_runtime_command_named(runtime_shell, "catchtutorial")
}

fn deposit_visible_day_care_man(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::Deposit,
    )
}

fn deposit_visible_day_care_lady(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Lady,
        RuntimeDayCareAction::Deposit,
    )
}

fn withdraw_visible_day_care_man(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::Withdraw,
    )
}

fn withdraw_visible_day_care_lady(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Lady,
        RuntimeDayCareAction::Withdraw,
    )
}

fn inspect_visible_day_care_man(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::Inspect,
    )
}

fn inspect_visible_day_care_lady(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Lady,
        RuntimeDayCareAction::Inspect,
    )
}

fn collect_visible_day_care_egg(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::CollectEgg,
    )
}

fn use_visible_day_care(
    runtime_shell: &mut BevyRuntimeShell,
    caretaker: RuntimeDayCareCaretaker,
    action: RuntimeDayCareAction,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if action == RuntimeDayCareAction::Deposit && snapshot.party.slots.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!("special:day_care:{caretaker:?}:{action:?}:empty_party"),
        )?;
        runtime_shell
            .last_audio_events
            .push("day care deposit requested with empty party".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let party_index = if action == RuntimeDayCareAction::Deposit {
        Some(selected_party_index(runtime_shell)?)
    } else {
        None
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("special:day_care:{caretaker:?}:{action:?}:{party_index:?}"),
    )?;
    let used = runtime_shell
        .shell
        .use_day_care(caretaker, action, party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "day care caretaker={:?} action={:?} party_index={:?} outcome={:?} checksum={:?}",
        caretaker, action, party_index, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn bug_contest_give_park_balls(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::GiveParkBalls)
}

fn bug_contest_select_contestants(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::SelectContestants)
}

fn bug_contest_drop_off_mons(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::DropOffMons)
}

fn bug_contest_return_mons(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::ReturnMons)
}

fn bug_contest_check_party_full(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::CheckPartyFull)
}

fn use_visible_bug_contest(
    runtime_shell: &mut BevyRuntimeShell,
    action: RuntimeBugContestAction,
) -> Result<()> {
    record_visible_runtime_action(
        runtime_shell,
        format!("special:bug_contest:{action:?}"),
    )?;
    let used = runtime_shell.shell.use_bug_contest(action)?;
    runtime_shell.last_audio_events.push(format!(
        "bug contest action={:?} outcome={:?} checksum={:?}",
        action, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn use_visible_buena_password(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:buena_password")?;
    let used = runtime_shell.shell.use_current_buena_password_guess()?;
    runtime_shell.last_audio_events.push(format!(
        "buena password outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn use_visible_buena_prize(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let balance = snapshot.trainer.blue_card_balance;
    let affordable = snapshot
        .special
        .buena_prizes
        .iter()
        .filter(|(_, cost)| u16::from(**cost) <= balance)
        .map(|(item_id, cost)| (item_id.clone(), *cost))
        .collect::<Vec<_>>();
    if affordable.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!("special:buena_prize:none_affordable:balance:{balance}"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "Blue Card balance {balance} cannot afford a Buena prize"
        ));
        set_shell_action_status(runtime_shell, "NOT ENOUGH POINTS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let selected_index = runtime_shell.script_command_cursor % affordable.len();
    let (item_id, cost) = affordable[selected_index].clone();
    record_visible_runtime_action(
        runtime_shell,
        format!("special:buena_prize:{item_id}:1:cost:{cost}"),
    )?;
    let used = runtime_shell.shell.use_buena_prize(item_id.clone(), 1)?;
    runtime_shell.last_audio_events.push(format!(
        "buena prize {}/{} item={} cost={} balance={} outcome={:?} checksum={:?}",
        selected_index + 1,
        affordable.len(),
        item_id,
        cost,
        balance,
        used.outcome.effect,
        used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn give_visible_shuckie(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:shuckie:give")?;
    let used = runtime_shell
        .shell
        .use_shuckie(RuntimeShuckieAction::Give, None)?;
    runtime_shell.last_audio_events.push(format!(
        "shuckie give outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn return_visible_shuckie(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:shuckie:return:{party_index}"),
    )?;
    let used = runtime_shell
        .shell
        .use_shuckie(RuntimeShuckieAction::Return, Some(party_index))?;
    runtime_shell.last_audio_events.push(format!(
        "shuckie return party_index={} outcome={:?} checksum={:?}",
        party_index, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn give_visible_odd_egg(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:odd_egg:give")?;
    let used = runtime_shell.shell.give_odd_egg()?;
    runtime_shell.last_audio_events.push(format!(
        "odd egg outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn give_visible_dratini(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.special.dratini_move_sets.is_empty() {
        anyhow::bail!("compiled pack declares no Dratini move sets");
    }
    let mode = *snapshot
        .special
        .dratini_move_sets
        .keys()
        .nth(runtime_shell.script_command_cursor % snapshot.special.dratini_move_sets.len())
        .context("selected Dratini move set missing from compiled pack")?;
    record_visible_runtime_action(runtime_shell, format!("special:dratini:give:{mode}"))?;
    let used = runtime_shell.shell.give_dratini(mode)?;
    runtime_shell.last_audio_events.push(format!(
        "dratini mode={} outcome={:?} checksum={:?}",
        mode, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn use_visible_bills_grandfather(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:bills_grandfather:{party_index}"),
    )?;
    let used = runtime_shell
        .shell
        .use_bills_grandfather(Some(party_index), None)?;
    runtime_shell.last_audio_events.push(format!(
        "bill grandfather party_index={} outcome={:?} checksum={:?}",
        party_index, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn init_visible_roam_mons(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:roamers:init")?;
    let used = runtime_shell.shell.init_roam_mons()?;
    runtime_shell.last_audio_events.push(format!(
        "roamers outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn check_visible_magikarp_length(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:magikarp_length:{party_index}"),
    )?;
    let used = runtime_shell.shell.check_magikarp_length(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "magikarp length party_index={} outcome={:?} checksum={:?}",
        party_index, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn show_visible_prof_oaks_pc_boot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:prof_oaks_pc_boot")?;
    let used = runtime_shell.shell.show_prof_oaks_pc_boot()?;
    runtime_shell.last_audio_events.push(format!(
        "prof oak pc outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn show_visible_magikarp_house_sign(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:magikarp_house_sign")?;
    let used = runtime_shell.shell.show_magikarp_house_sign()?;
    runtime_shell.last_audio_events.push(format!(
        "magikarp sign outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn apply_visible_battle_tower_reset(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:battle_tower:reset")?;
    let used = runtime_shell.shell.apply_battle_tower_action(
        "BATTLETOWERACTION_RESETDATA".to_string(),
        None,
        None,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "battle tower reset outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn check_visible_mystery_gift(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_mystery_gift(runtime_shell, RuntimeMysteryGiftAction::Check)
}

fn claim_visible_mystery_gift_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_mystery_gift(runtime_shell, RuntimeMysteryGiftAction::ClaimItem)
}

fn unlock_visible_mystery_gift(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_mystery_gift(runtime_shell, RuntimeMysteryGiftAction::Unlock)
}

fn use_visible_mystery_gift(
    runtime_shell: &mut BevyRuntimeShell,
    action: RuntimeMysteryGiftAction,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("special:mystery_gift:{action:?}"))?;
    let used = runtime_shell.shell.use_mystery_gift(action)?;
    runtime_shell.last_audio_events.push(format!(
        "mystery gift action={:?} outcome={:?} checksum={:?}",
        action, used.outcome.effect, used.state_checksum
    ));
    activate_visible_special_boundary_if_needed(runtime_shell, &used.outcome.effect)?;
    Ok(())
}

fn set_visible_player_palette(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let palette_id = (runtime_shell.script_command_cursor % 8) as u8;
    record_visible_runtime_action(
        runtime_shell,
        format!("special:player_palette:{palette_id}"),
    )?;
    let used = runtime_shell
        .shell
        .set_player_palette(0x80 | (palette_id << 4))?;
    runtime_shell.last_audio_events.push(format!(
        "player palette selected={} outcome={:?} checksum={:?}",
        palette_id, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn set_visible_day_of_week(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:set_day_of_week")?;
    let used = runtime_shell.shell.set_day_of_week()?;
    runtime_shell.last_audio_events.push(format!(
        "day of week outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn update_visible_time(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "special:update_time")?;
    let used = runtime_shell.shell.update_time()?;
    runtime_shell.last_audio_events.push(format!(
        "time update outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn grant_selected_gift_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let gifts = visible_gift_pokemon_prompt_options(&snapshot, runtime_shell);
    let gift_count = gifts.len();
    if gift_count == 0 {
        runtime_shell.gift_pokemon_cursor = None;
        record_visible_runtime_action(runtime_shell, "ui:gift_pokemon:none_visible")?;
        runtime_shell
            .last_audio_events
            .push("no compiled gift Pokemon prompt is visible".to_string());
        set_shell_action_status(runtime_shell, "NO GIFT POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let surface_id = runtime_shell
        .gift_pokemon_cursor
        .as_ref()
        .map(|cursor| cursor.surface_id.clone())
        .unwrap_or_else(|| {
            gift_pokemon_surface_id(&gifts[0].source_script, gifts[0].command_index)
        });
    let selected_index = visible_cursor_index(
        &mut runtime_shell.gift_pokemon_cursor,
        &surface_id,
        gift_count,
    );
    let gift = gifts[selected_index];
    let gift_source_script = gift.source_script.clone();
    let gift_command_index = gift.command_index;
    let gift_species_id = gift.species_id.clone();
    let gift_level = gift.level;
    let player_name = snapshot.trainer.player_name.clone();
    let player_id = snapshot.trainer.player_id;
    let (gift_dvs, gift_rng_seed_after) = next_visible_gift_dvs(snapshot.progression.rng_seed);
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:gift_pokemon:{}:{}:{}:{}",
            gift_source_script.as_str(),
            gift_command_index,
            gift_species_id.as_str(),
            gift_level
        ),
    )?;
    let granted = if let Some(cursor) = visible_active_compiled_script_cursor(runtime_shell) {
        runtime_shell
            .shell
            .grant_compiled_gift_pokemon_command_and_run_compiled_script(
                &gift_source_script,
                gift_command_index,
                player_name,
                player_id,
                gift_dvs,
                gift_rng_seed_after,
                false,
                None,
                Some(cursor),
                256,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )?
    } else {
        let grant = runtime_shell.shell.grant_compiled_gift_pokemon_command(
            &gift_source_script,
            gift_command_index,
            player_name,
            player_id,
            gift_dvs,
            gift_rng_seed_after,
            false,
            None,
        )?;
        crate::RuntimeGiftPokemonCompiledScriptRun {
            grant,
            run: crate::RuntimeCompiledScriptRun {
                steps: Vec::new(),
                next_cursor: None,
                boundary: None,
                ended: false,
            },
        }
    };
    runtime_shell.last_audio_events.push(format!(
        "gift pokemon {}/{} species={} level={} rng={} outcome={:?} resumed_steps={} checksum={:?}",
        selected_index + 1,
        gift_count,
        gift_species_id,
        gift_level,
        gift_rng_seed_after,
        granted.grant.outcome,
        granted.run.steps.len(),
        granted.grant.state_checksum
    ));
    runtime_shell.gift_pokemon_cursor = None;
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &granted.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, granted.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn next_visible_gift_dvs(rng_seed: u32) -> (Dv, u32) {
    let mut rng = Random::new_crystal(rng_seed);
    let dvs = Dv::from_non_hp(
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
    );
    let rng_seed_after = rng.seed();
    (dvs, rng_seed_after)
}

fn first_party_index(snapshot: &RuntimeShellSnapshot) -> Result<usize> {
    snapshot
        .party
        .slots
        .first()
        .map(|slot| slot.index)
        .with_context(|| "party is empty")
}

fn second_party_index(snapshot: &RuntimeShellSnapshot) -> Result<usize> {
    snapshot
        .party
        .slots
        .get(1)
        .map(|slot| slot.index)
        .with_context(|| "party has no second slot")
}

fn selected_carried_normal_item_matching(
    runtime_shell: &mut BevyRuntimeShell,
    predicate: impl Fn(&crate::RuntimeItemCatalogSnapshot) -> bool,
    empty_message: &str,
) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = if snapshot.battle.is_some() {
        selected_visible_battle_pack_action_item(runtime_shell)?
    } else {
        selected_field_pack_item_id(runtime_shell)?
    };
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?;
    if !predicate(item) {
        anyhow::bail!("{empty_message}: selected item {item_id} is not valid");
    }
    Ok(item_id)
}

fn selected_carried_battle_item_matching(
    runtime_shell: &mut BevyRuntimeShell,
    predicate: impl Fn(&crate::RuntimeItemCatalogSnapshot) -> bool,
    empty_message: &str,
) -> Result<String> {
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected battle item {item_id} is missing from item catalog"))?;
    if !predicate(item) {
        anyhow::bail!("{empty_message}: selected battle item {item_id} is not valid");
    }
    Ok(item_id)
}

fn carried_battle_usable_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    let is_battle_usable = |item_id: &str| {
        snapshot
            .items
            .iter()
            .any(|item| item.item_id == item_id && item.battle_usable)
    };
    let is_capture_ball = |item_id: &str| {
        snapshot
            .battle_rules
            .capture_rules
            .ball_rules
            .contains_key(item_id)
    };
    let mut item_ids = Vec::new();
    for bag_item in snapshot.bag.items.iter().filter(|item| item.quantity > 0) {
        if is_battle_usable(&bag_item.item_id) || is_capture_ball(&bag_item.item_id) {
            item_ids.push(bag_item.item_id.clone());
        }
    }
    for ball_item in snapshot.bag.balls.iter().filter(|item| item.quantity > 0) {
        if is_capture_ball(&ball_item.item_id) && !item_ids.contains(&ball_item.item_id) {
            item_ids.push(ball_item.item_id.clone());
        }
    }
    for custom_item in snapshot
        .bag
        .custom_pockets
        .values()
        .flat_map(|items| items.iter())
        .filter(|item| item.quantity > 0)
    {
        if (is_battle_usable(&custom_item.item_id) || is_capture_ball(&custom_item.item_id))
            && !item_ids.contains(&custom_item.item_id)
        {
            item_ids.push(custom_item.item_id.clone());
        }
    }
    item_ids
}

fn carried_battle_non_ball_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    let capture_balls = &snapshot.battle_rules.capture_rules.ball_rules;
    snapshot
        .bag
        .items
        .iter()
        .filter(|item| item.quantity > 0 && !capture_balls.contains_key(&item.item_id))
        .map(|item| item.item_id.clone())
        .collect()
}

fn selected_battle_bag_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = carried_battle_non_ball_item_ids(&snapshot);
    if item_ids.is_empty() {
        anyhow::bail!("bag item pocket has no carried item");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.bag_cursor,
        "battle:bag-items",
        item_ids.len(),
    );
    Ok(item_ids[index].clone())
}

fn selected_battle_ball_id(runtime_shell: &mut BevyRuntimeShell) -> Result<(usize, String)> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let ball_ids = carried_ball_item_ids(&snapshot);
    if ball_ids.is_empty() {
        anyhow::bail!("bag has no carried ball");
    }
    let index = visible_cursor_index(&mut runtime_shell.ball_cursor, "bag:balls", ball_ids.len());
    Ok((index, ball_ids[index].clone()))
}

fn selected_ball_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let ball_ids = snapshot
        .bag
        .balls
        .iter()
        .filter(|ball| ball.quantity > 0)
        .map(|ball| ball.item_id.clone())
        .collect::<Vec<_>>();
    if ball_ids.is_empty() {
        anyhow::bail!("bag has no carried ball");
    }
    let index = visible_cursor_index(&mut runtime_shell.ball_cursor, "bag:balls", ball_ids.len());
    Ok(ball_ids[index].clone())
}

fn carried_ball_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    let is_capture_ball = |item_id: &str| {
        snapshot
            .battle_rules
            .capture_rules
            .ball_rules
            .contains_key(item_id)
    };
    let mut seen = BTreeSet::new();
    let mut ball_ids = snapshot
        .bag
        .balls
        .iter()
        .filter(|ball| ball.quantity > 0)
        .filter(|ball| is_capture_ball(&ball.item_id))
        .filter(|ball| seen.insert(ball.item_id.clone()))
        .map(|ball| ball.item_id.clone())
        .collect::<Vec<_>>();
    ball_ids.extend(
        snapshot
            .bag
            .custom_pockets
            .values()
            .flat_map(|items| items.iter())
            .filter(|item| item.quantity > 0)
            .filter(|item| is_capture_ball(&item.item_id))
            .filter(|item| seen.insert(item.item_id.clone()))
            .map(|item| item.item_id.clone()),
    );
    ball_ids
}

fn selected_bag_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = snapshot
        .bag
        .items
        .iter()
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    if item_ids.is_empty() {
        anyhow::bail!("bag item pocket has no carried item");
    }
    let index = visible_cursor_index(&mut runtime_shell.bag_cursor, "bag:items", item_ids.len());
    Ok(item_ids[index].clone())
}

fn selected_key_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = snapshot
        .bag
        .key_items
        .iter()
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    if item_ids.is_empty() {
        anyhow::bail!("bag key item pocket has no carried item");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.key_item_cursor,
        "bag:key-items",
        item_ids.len(),
    );
    Ok(item_ids[index].clone())
}

fn selected_pc_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = snapshot
        .bag
        .pc_items
        .iter()
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    if item_ids.is_empty() {
        anyhow::bail!("PC item storage has no item");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.pc_item_cursor,
        "pc:items",
        item_ids.len(),
    );
    Ok(item_ids[index].clone())
}

fn selected_custom_bag_item_id(
    runtime_shell: &mut BevyRuntimeShell,
    pocket_id: &str,
) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = snapshot
        .bag
        .custom_pockets
        .get(pocket_id)
        .with_context(|| format!("bag custom pocket {pocket_id} is not present"))?
        .iter()
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    if item_ids.is_empty() {
        anyhow::bail!("bag custom pocket {pocket_id} has no carried item");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.custom_item_cursor,
        &custom_pack_surface_id(pocket_id),
        item_ids.len(),
    );
    Ok(item_ids[index].clone())
}

fn selected_bag_or_pc_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    if runtime_shell.pc_item_cursor.is_some() {
        return selected_pc_item_id(runtime_shell);
    }
    selected_field_pack_item_id(runtime_shell)
}

fn selected_field_pack_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    if selected_field_pack_cancel_row(&snapshot, runtime_shell, &pocket) {
        anyhow::bail!("selected Pack cursor is CANCEL");
    }
    match active_visible_field_pack_pocket(runtime_shell) {
        FieldPackPocket::Items => selected_bag_item_id(runtime_shell),
        FieldPackPocket::Balls => selected_ball_item_id(runtime_shell),
        FieldPackPocket::KeyItems => selected_key_item_id(runtime_shell),
        FieldPackPocket::TmHm => selected_tmhm(runtime_shell).map(|(item_id, _)| item_id),
        FieldPackPocket::Custom(pocket_id) => {
            selected_custom_bag_item_id(runtime_shell, pocket_id.as_str())
        }
    }
}

fn selected_field_pack_cancel_row(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
) -> bool {
    match pocket {
        FieldPackPocket::Items => strict_readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "bag:items",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.items)),
        )
        .is_some_and(|index| index == carried_item_count(&snapshot.bag.items)),
        FieldPackPocket::Balls => strict_readonly_cursor_index(
            &runtime_shell.ball_cursor,
            "bag:balls",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.balls)),
        )
        .is_some_and(|index| index == carried_item_count(&snapshot.bag.balls)),
        FieldPackPocket::KeyItems => strict_readonly_cursor_index(
            &runtime_shell.key_item_cursor,
            "bag:key-items",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.key_items)),
        )
        .is_some_and(|index| index == carried_item_count(&snapshot.bag.key_items)),
        FieldPackPocket::TmHm => strict_readonly_cursor_index(
            &runtime_shell.tmhm_cursor,
            "bag:tmhm",
            field_pack_selectable_count(snapshot.bag.tm_hm.len()),
        )
        .is_some_and(|index| index == snapshot.bag.tm_hm.len()),
        FieldPackPocket::Custom(pocket_id) => snapshot
            .bag
            .custom_pockets
            .get(pocket_id)
            .is_some_and(|items| {
                strict_readonly_cursor_index(
                    &runtime_shell.custom_item_cursor,
                    &custom_pack_surface_id(pocket_id),
                    field_pack_selectable_count(carried_item_count(items)),
                )
                .is_some_and(|index| index == carried_item_count(items))
            }),
    }
}

fn selected_tmhm(runtime_shell: &mut BevyRuntimeShell) -> Result<(String, Option<String>)> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let tmhms = snapshot
        .bag
        .tm_hm
        .iter()
        .map(|tmhm| (tmhm.item_id.clone(), tmhm.move_id.clone()))
        .collect::<Vec<_>>();
    if tmhms.is_empty() {
        anyhow::bail!("bag has no carried TM/HM");
    }
    let index = visible_cursor_index(&mut runtime_shell.tmhm_cursor, "bag:tmhm", tmhms.len());
    Ok(tmhms[index].clone())
}

fn selected_party_special_item_id(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    behavior_id: &str,
    error_message: &str,
) -> Result<String> {
    let item_id = if snapshot.battle.is_some() {
        selected_visible_battle_pack_action_item(runtime_shell)?
    } else {
        selected_field_pack_item_id(runtime_shell)?
    };
    let matches_behavior = snapshot
        .item_effect_plans
        .iter()
        .any(|plan| plan.item_id == item_id && plan.behavior_id == behavior_id);
    if !matches_behavior {
        anyhow::bail!("{error_message}: selected item {item_id} is not valid");
    }
    Ok(item_id)
}

fn use_selected_party_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_party_item_on(runtime_shell, party_index)
}

fn use_selected_party_item_on_second_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = second_party_index(&snapshot)?;
    use_selected_party_item_on(runtime_shell, party_index)
}

fn use_selected_party_item_on(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pokemon_name = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.clone())
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        item_targets_party_pokemon_fields,
        "bag has no carried party item matching compiled party-use effect fields",
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("party:item:{item_id}:pokemon:{party_index}"),
    )?;
    let used = match runtime_shell
        .shell
        .use_bag_item_on_party_pokemon(&item_id, party_index)
    {
        Ok(used) => used,
        Err(error) if field_item_error_is_play_refusal(&error) => {
            return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "party item item={} party_index={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.item_effect, used.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("USED {item_id} ON PARTY #{party_index}"),
    );
    let notice = if used.item_effect.hp_before == 0 && used.item_effect.hp_after > 0 {
        format!("{pokemon_name} was revived!")
    } else if used.item_effect.hp_after > used.item_effect.hp_before {
        format!("{pokemon_name} recovered health!")
    } else if used.item_effect.status_before != used.item_effect.status_after
        || used.item_effect.confusion_turns_after < used.item_effect.confusion_turns_before
    {
        format!("{pokemon_name} was cured!")
    } else if used.item_effect.level_after > used.item_effect.level_before {
        format!("{pokemon_name} grew to level {}!", used.item_effect.level_after)
    } else {
        format!("{pokemon_name}'s stats rose!")
    };
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.field_notice = Some(notice);
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_selected_whole_party_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.party_revive_hp_percent.is_some(),
        "bag has no carried whole-party item",
    )?;
    record_visible_runtime_action(runtime_shell, format!("party:item:{item_id}:whole_party"))?;
    let used = match runtime_shell.shell.use_bag_item_on_whole_party(&item_id) {
        Ok(used) => used,
        Err(error) if field_item_error_is_play_refusal(&error) => {
            return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "whole party item item={} item_use={:?} effect={:?} checksum={:?}",
        item_id, used.item_use, used.item_effect, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(runtime_shell, format!("USED {item_id} ON PARTY"));
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.field_notice = Some("Your POKéMON were revived!".to_string());
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_selected_pp_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_pp_item_on(runtime_shell, party_index)
}

fn use_selected_pp_item_on_second_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = second_party_index(&snapshot)?;
    use_selected_pp_item_on(runtime_shell, party_index)
}

fn use_selected_pp_item_on(runtime_shell: &mut BevyRuntimeShell, party_index: usize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let move_slot = selected_party_move_slot(runtime_shell, party_index)?;
    let move_name = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .and_then(|slot| slot.pokemon.moves.get(move_slot))
        .map(|learned| battle_move_display_name(&snapshot, &learned.name))
        .with_context(|| {
            format!("party index {party_index} has no move in slot {move_slot}")
        })?;
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.pp_restore_points.is_some() || item.pp_up_stages.is_some(),
        "bag has no carried PP item",
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("party:item:{item_id}:pokemon:{party_index}:move:{move_slot}"),
    )?;
    let used =
        match runtime_shell
            .shell
            .use_bag_item_on_party_move(&item_id, party_index, Some(move_slot))
        {
            Ok(used) => used,
            Err(error) if field_item_error_is_play_refusal(&error) => {
                return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
            }
            Err(error) => return Err(error),
        };
    runtime_shell.last_audio_events.push(format!(
        "party move item item={} party_index={} move_slot={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, move_slot, used.item_use, used.item_effect, used.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "USED {item_id} ON PARTY #{party_index} MOVE {}",
            move_slot + 1
        ),
    );
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.field_notice = Some(format!("{move_name}'s PP was restored."));
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_selected_rare_candy(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let item_id = selected_party_special_item_id(
        runtime_shell,
        &snapshot,
        ITEM_EFFECT_BEHAVIOR_RARE_CANDY,
        "selected item is not a rare-candy party item",
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("party:item:{item_id}:rare_candy:{party_index}"),
    )?;
    let used = match runtime_shell
        .shell
        .use_bag_item_on_party_pokemon(&item_id, party_index)
    {
        Ok(used) => used,
        Err(error) if field_item_error_is_play_refusal(&error) => {
            return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "rare candy item={} party_index={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.item_effect, used.state_checksum
    ));
    let updated_snapshot = runtime_shell.shell.snapshot()?;
    for learned in &used.item_effect.pending_move_learns {
        runtime_shell
            .last_audio_events
            .push(format!("rare candy pending move learn {}", learned.name));
    }
    let source_name = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.clone())
        .context("Rare Candy target disappeared from the pre-use party snapshot")?;
    let evolution_result = used.item_effect.evolution_target.as_ref().map(|target_species| {
        let evolving_message = format!("What? {source_name} is evolving!");
        let evolved_message = format!(
            "Congratulations! {source_name} evolved into {}!",
            crate::core::models::pokemon_species_display_name(target_species)
        );
        (target_species.clone(), evolving_message, evolved_message)
    });
    set_shell_action_status(
        runtime_shell,
        if used.item_effect.pending_move_learns.is_empty() {
            format!("USED {item_id} ON PARTY #{party_index}")
        } else {
            format!(
                "WANTS {}",
                compact_scene_label(
                    &used
                        .item_effect
                        .pending_move_learns
                        .iter()
                        .map(|learned| {
                            battle_move_display_name(&updated_snapshot, &learned.name)
                        })
                        .collect::<Vec<_>>()
                        .join(","),
                    44,
                )
            )
        },
    );
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.field_notice = Some(format!(
        "{} grew to level {}!",
        source_name,
        used.item_effect.level_after
    ));
    if let Some((target_species, evolving_message, evolved_message)) = evolution_result {
        runtime_shell
            .field_notice_queue
            .push_back(evolving_message.clone());
        runtime_shell
            .field_notice_queue
            .push_back(evolved_message.clone());
        if let Some(cancel_snapshot) = used.item_effect.evolution_cancel_snapshot.clone() {
            runtime_shell.field_evolution_cancellation = Some(VisibleEvolutionCancellation {
                party_index,
                trigger_message: evolving_message,
                evolved_message,
                pending_move_messages: Vec::new(),
                report: EvolutionReport {
                    target_species: Some(target_species),
                    events: vec![crate::core::systems::evolution::EvolutionEvent::Text(
                        "EvolvingText",
                    )],
                    pending_move_learns: used.item_effect.pending_move_learns.clone(),
                    cancel_snapshot: Some(cancel_snapshot),
                },
            });
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    if runtime_shell.shell.snapshot()?.pending_move_learn.is_none() {
        continue_visible_script_after_prompt(runtime_shell)?;
    }
    Ok(())
}

fn use_selected_evolution_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let source_name = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.clone())
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    let item_id = selected_party_special_item_id(
        runtime_shell,
        &snapshot,
        ITEM_EFFECT_BEHAVIOR_EVOLUTION_STONE,
        "selected item is not an evolution party item",
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("party:item:{item_id}:evolution:{party_index}"),
    )?;
    let used = match runtime_shell
        .shell
        .use_bag_item_on_party_pokemon(&item_id, party_index)
    {
        Ok(used) => used,
        Err(error) if field_item_error_is_play_refusal(&error) => {
            return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "evolution item item={} party_index={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.item_effect, used.state_checksum
    ));
    for learned in &used.item_effect.learned_moves {
        runtime_shell
            .last_audio_events
            .push(format!("evolution item learned move {learned}"));
    }
    for learned in &used.item_effect.pending_move_learns {
        runtime_shell
            .last_audio_events
            .push(format!("evolution item pending move learn {}", learned.name));
    }
    let evolution_result = used.item_effect.evolution_target.as_ref().map(|target_species| {
        (
            target_species.clone(),
            format!(
                "Congratulations! {source_name} evolved into {}!",
                crate::core::models::pokemon_species_display_name(target_species)
            ),
        )
    });
    set_shell_action_status(
        runtime_shell,
        if let Some(target_species) = used.item_effect.evolution_target.as_ref() {
            format!(
                "{source_name} EVOLVED INTO {}!",
                crate::core::models::pokemon_species_display_name(target_species)
            )
        } else {
            format!("{item_id} WON'T HAVE ANY EFFECT")
        },
    );
    close_visible_field_pack_without_log(runtime_shell);
    if let Some((target_species, evolved_notice)) = evolution_result {
        runtime_shell.field_notice = Some(format!("What? {source_name} is evolving!"));
        runtime_shell.field_notice_queue.push_back(evolved_notice);
        runtime_shell.pending_field_notice_cry = Some(target_species);
    } else {
        runtime_shell.field_notice = Some("It won't have any effect.".to_string());
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    if runtime_shell.shell.snapshot()?.pending_move_learn.is_none() {
        continue_visible_script_after_prompt(runtime_shell)?;
    }
    Ok(())
}

fn swap_visible_selected_party_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let lead = snapshot
        .party
        .slots
        .first()
        .with_context(|| "party has no lead Pokemon")?;
    let lead_index = lead.index;
    let selected_index = selected_party_index(runtime_shell)?;
    let swap_index = if selected_index == lead_index {
        snapshot
            .party
            .slots
            .get(1)
            .map(|slot| slot.index)
            .with_context(|| "party has no second Pokemon to swap with selected lead")?
    } else {
        selected_index
    };
    swap_visible_party_pokemon(runtime_shell, lead_index, swap_index)
}

fn swap_visible_party_pokemon(
    runtime_shell: &mut BevyRuntimeShell,
    first_party_index: usize,
    second_party_index: usize,
) -> Result<()> {
    record_visible_runtime_action(
        runtime_shell,
        format!("party:swap:{first_party_index}:{second_party_index}"),
    )?;
    let swapped = runtime_shell
        .shell
        .swap_party_pokemon(first_party_index, second_party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "party swap {}<->{} first_after={} second_after={} checksum={:?}",
        swapped.first_party_index,
        swapped.second_party_index,
        swapped.first_species_after,
        swapped.second_species_after,
        swapped.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWAPPED {} AND {}",
            swapped.first_species_after, swapped.second_species_after
        ),
    );
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.party_cursor = snapshot
        .party
        .slots
        .iter()
        .position(|slot| slot.index == second_party_index)
        .with_context(|| {
            format!("party swap result no longer contains party index {second_party_index}")
        })?;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    Ok(())
}

fn give_selected_held_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    give_selected_held_item_with_swap_confirmation(runtime_shell, false)
}

fn give_selected_held_item_with_swap_confirmation(
    runtime_shell: &mut BevyRuntimeShell,
    swap_confirmed: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = runtime_shell
        .party_held_item_give_target
        .as_ref()
        .copied()
        .map_or_else(|| selected_party_index(runtime_shell), Ok)?;
    let item_id = selected_field_pack_item_id(runtime_shell)?;
    if crate::core::models::item::is_mail_item_id(&item_id) {
        record_visible_runtime_action(runtime_shell, format!("party:mail:compose:{item_id}"))?;
        runtime_shell.field_notice = Some("Write a message before giving this MAIL.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "MAIL NEEDS A MESSAGE");
        return Ok(());
    }
    let target = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    let pokemon_name = target.pokemon.nickname.clone();
    if target.pokemon.is_egg
        || target.pokemon.species.id == "EGG"
    {
        runtime_shell.field_notice = Some("Eggs cannot hold or receive items.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "EGGS CAN'T HOLD ITEMS");
        return Ok(());
    }
    if target.pokemon.mail.is_some() {
        runtime_shell.field_notice = Some("Please remove the MAIL first.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "REMOVE MAIL FIRST");
        return Ok(());
    }
    let old_item_name = target
        .pokemon
        .item
        .as_deref()
        .map(|held| item_display_name(&snapshot, held));
    let new_item_name = item_display_name(&snapshot, &item_id);
    if old_item_name.is_some() && !swap_confirmed {
        runtime_shell.held_item_swap_prompt = true;
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "party:held-item-swap".to_string(),
            option_index: 0,
        });
        runtime_shell.field_pack_action_cursor = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    record_visible_runtime_action(
        runtime_shell,
        format!("party:held_item:give:{party_index}:{item_id}"),
    )?;
    let transfer = match runtime_shell
        .shell
        .give_bag_item_to_party_pokemon(&item_id, party_index)
    {
        Ok(transfer) => transfer,
        Err(error) if held_item_transfer_error_is_play_refusal(&error) => {
            runtime_shell.field_notice = Some(
                "The old held item can't be returned because the PACK is full.".to_string(),
            );
            mark_runtime_snapshot_dirty(runtime_shell);
            runtime_shell
                .last_audio_events
                .push(format!("held item give refused: {error}"));
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    runtime_shell.party_held_item_give_target = None;
    runtime_shell.held_item_swap_prompt = false;
    runtime_shell.yes_no_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "held item give item={} party_index={} bag_after={} checksum={:?}",
        transfer.item_id,
        transfer.party_index,
        transfer.bag_quantity_after,
        transfer.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "GAVE {} TO PARTY #{}",
            transfer.item_id, transfer.party_index
        ),
    );
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.field_notice = Some(match old_item_name {
        Some(old_item_name) => format!(
            "Took {pokemon_name}'s {old_item_name} and made it hold {new_item_name}."
        ),
        None => format!("Made {pokemon_name} hold {new_item_name}."),
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn resolve_visible_held_item_swap_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "party:held_item:swap:{}",
            if accepted { "yes" } else { "no" }
        ),
    )?;
    runtime_shell.held_item_swap_prompt = false;
    runtime_shell.yes_no_cursor = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    if accepted {
        give_selected_held_item_with_swap_confirmation(runtime_shell, true)
    } else {
        Ok(())
    }
}

fn take_selected_held_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let pokemon_name = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.clone())
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    record_visible_runtime_action(runtime_shell, format!("party:held_item:take:{party_index}"))?;
    let transfer = match runtime_shell
        .shell
        .take_held_item_from_party_pokemon(party_index)
    {
        Ok(transfer) => transfer,
        Err(error) if held_item_transfer_error_is_play_refusal(&error) => {
            runtime_shell.field_notice =
                Some("The PACK is full. The item couldn't be taken.".to_string());
            mark_runtime_snapshot_dirty(runtime_shell);
            runtime_shell
                .last_audio_events
                .push(format!("held item take refused: {error}"));
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "held item take item={} party_index={} bag_after={} checksum={:?}",
        transfer.item_id,
        transfer.party_index,
        transfer.bag_quantity_after,
        transfer.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "TOOK {} FROM PARTY #{}",
            transfer.item_id, transfer.party_index
        ),
    );
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.field_notice = Some(format!(
        "Took {} from {pokemon_name}.",
        item_display_name(&snapshot, &transfer.item_id)
    ));
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn held_item_transfer_error_is_play_refusal(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        let message = cause.to_string();
        message.starts_with("bag rejected replaced held item ")
            || message.starts_with("bag rejected held item ")
            || message.starts_with("return replaced held item to bag: ")
            || message.starts_with("return held item to bag: ")
    })
}

fn award_visible_badge(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let badge_slots = [
        (RuntimeBadgeRegion::Johto, 0usize),
        (RuntimeBadgeRegion::Johto, 1),
        (RuntimeBadgeRegion::Johto, 2),
        (RuntimeBadgeRegion::Johto, 3),
        (RuntimeBadgeRegion::Johto, 4),
        (RuntimeBadgeRegion::Johto, 5),
        (RuntimeBadgeRegion::Johto, 6),
        (RuntimeBadgeRegion::Johto, 7),
        (RuntimeBadgeRegion::Kanto, 0),
        (RuntimeBadgeRegion::Kanto, 1),
        (RuntimeBadgeRegion::Kanto, 2),
        (RuntimeBadgeRegion::Kanto, 3),
        (RuntimeBadgeRegion::Kanto, 4),
        (RuntimeBadgeRegion::Kanto, 5),
        (RuntimeBadgeRegion::Kanto, 6),
        (RuntimeBadgeRegion::Kanto, 7),
    ];
    let selected_index = runtime_shell.script_command_cursor % badge_slots.len();
    let (region, index) = badge_slots[selected_index];
    let award = runtime_shell.shell.award_badge(region, index)?;
    runtime_shell.last_audio_events.push(format!(
        "badge award {}/{} region={:?} index={} already={} total={} checksum={:?}",
        selected_index + 1,
        badge_slots.len(),
        award.region,
        award.index,
        award.already_awarded,
        award.awarded_count_after,
        award.state_checksum
    ));
    Ok(())
}

fn record_selected_pokedex_seen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let species_id = selected_pokedex_species_id(runtime_shell)?;
    let record = runtime_shell.shell.record_pokedex_seen(&species_id)?;
    runtime_shell.last_audio_events.push(format!(
        "pokedex seen species={} already_seen={} already_caught={} seen={} caught={} checksum={:?}",
        record.species_id,
        record.already_seen,
        record.already_caught,
        record.seen_count_after,
        record.caught_count_after,
        record.state_checksum
    ));
    Ok(())
}

fn record_selected_pokedex_caught(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let species_id = selected_pokedex_species_id(runtime_shell)?;
    let record = runtime_shell.shell.record_pokedex_caught(&species_id)?;
    runtime_shell.last_audio_events.push(format!(
        "pokedex caught species={} already_seen={} already_caught={} seen={} caught={} checksum={:?}",
        record.species_id,
        record.already_seen,
        record.already_caught,
        record.seen_count_after,
        record.caught_count_after,
        record.state_checksum
    ));
    Ok(())
}

fn add_visible_money(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_currency_delta(runtime_shell, RuntimeCurrencyAccount::Money, 1_000, true)
}
