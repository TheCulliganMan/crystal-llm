#[test]
fn battle_dialogue_uses_player_input_drains_once_and_returns_menu_control() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    runtime_shell.battle_text_reveal = None;
    sync_visible_battle_action_cursor(&mut runtime_shell);

    let original_cursor = runtime_shell
        .battle_action_cursor
        .clone()
        .expect("active battle action cursor");
    let messages = [
        "CYNDAQUIL used\nTACKLE!".to_string(),
        "A deliberately long battle message that occupies more than two native textbox lines and must advance through every page exactly once.".to_string(),
        "It's not very\neffective...".to_string(),
    ];
    runtime_shell.battle_messages = messages.clone().into_iter().collect();
    runtime_shell.battle_message_scene = Some(Box::new(
        runtime_shell.shell.snapshot().expect("battle dialogue scene"),
    ));

    dispatch_visible_ui_direction(&mut runtime_shell, GameButton::Down);
    assert_eq!(
        runtime_shell.battle_action_cursor.as_ref(),
        Some(&original_cursor),
        "directional input during battle text must not navigate the hidden command menu"
    );

    let mut dismissed = Vec::new();
    for step in 0..64 {
        let Some(message) = runtime_shell.battle_messages.front().cloned() else {
            break;
        };
        let snapshot = runtime_shell
            .shell
            .presentation_snapshot()
            .expect("battle dialogue presentation snapshot");
        while !visible_battle_message_is_complete(&runtime_shell, &message) {
            assert!(
                advance_visible_battle_text_reveal(&mut runtime_shell, &snapshot, true),
                "battle dialogue reveal stalled at step {step}: {message:?}"
            );
        }
        let page_before = runtime_shell
            .battle_text_reveal
            .as_ref()
            .expect("completed battle text reveal")
            .page_index;
        press_visible_a_button(&mut runtime_shell).expect("player A advances battle dialogue");
        if runtime_shell.battle_messages.front() != Some(&message) {
            dismissed.push(message);
        } else {
            let page_after = runtime_shell
                .battle_text_reveal
                .as_ref()
                .expect("next battle text page reveal")
                .page_index;
            assert_eq!(
                page_after,
                page_before + 1,
                "A on complete battle text must advance a page or consume the message"
            );
        }
    }

    assert_eq!(dismissed, messages, "battle messages repeated or changed order");
    assert!(runtime_shell.battle_messages.is_empty());
    assert!(runtime_shell.battle_text_reveal.is_none());
    assert!(runtime_shell.battle_message_scene.is_none());
    sync_visible_battle_action_cursor(&mut runtime_shell);
    assert!(
        runtime_shell.battle_action_cursor.is_some(),
        "closing the last battle message must return command-menu control"
    );
    assert_eq!(runtime_shell.last_error, None);
}

#[test]
fn replacement_text_uses_send_out_mon_texts_quarter_max_hp_divisor() {
    let runtime_shell = route36_battle_shell_for_render_regression();
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 2;
    battle.enemy_pokemon.max_hp = 21;
    let nickname = snapshot.party.slots[0].pokemon.nickname.clone();

    assert_eq!(
        super::visible_player_send_out_message(&snapshot, 0, false).expect("send-out message"),
        format!("Go for it, {nickname}!"),
    );
}

#[test]
fn replacement_text_rejects_send_out_mon_texts_nonterminating_divisor() {
    let runtime_shell = route36_battle_shell_for_render_regression();
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 1;
    battle.enemy_pokemon.max_hp = 3;

    let error = super::visible_player_send_out_message(&snapshot, 0, false)
        .expect_err("the cartridge divide would never terminate");
    assert!(
        error
            .to_string()
            .contains("would not terminate with enemy max HP 3"),
        "{error:#}"
    );
}

#[test]
fn link_replacement_text_uses_go_mon_text_without_reading_enemy_hp() {
    let runtime_shell = route36_battle_shell_for_render_regression();
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    snapshot.link_session.link_mode = 1;
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 1;
    battle.enemy_pokemon.max_hp = 3;
    let nickname = snapshot.party.slots[0].pokemon.nickname.clone();

    assert_eq!(
        super::visible_player_send_out_message(&snapshot, 0, false).expect("send-out message"),
        format!("Go! {nickname}!"),
    );
}

#[test]
fn initial_link_send_out_text_still_uses_enemy_hp() {
    let runtime_shell = route36_battle_shell_for_render_regression();
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    snapshot.link_session.link_mode = 1;
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 2;
    battle.enemy_pokemon.max_hp = 21;
    let nickname = snapshot.party.slots[0].pokemon.nickname.clone();

    assert_eq!(
        super::visible_player_send_out_message(&snapshot, 0, true).expect("send-out message"),
        format!("Go for it, {nickname}!"),
    );
}

#[test]
fn withdrawal_text_uses_withdraw_mon_texts_quarter_max_hp_divisor() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.battle_enemy_hp_at_player_send_out = Some(20);
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 14;
    battle.enemy_pokemon.max_hp = 21;

    assert_eq!(
        super::visible_player_withdraw_message(&runtime_shell, &snapshot, "CYNDAQUIL")
            .expect("withdrawal message"),
        "CYNDAQUIL, OK! Come back!",
    );
}

#[test]
fn withdrawal_text_uses_wrapping_damage_and_the_low_quotient_byte() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.battle_enemy_hp_at_player_send_out = Some(10);
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 11;
    battle.enemy_pokemon.max_hp = 21;

    // (10 - 11) wraps to 65535; (65535 * 25) / (21 >> 2) has low byte 251.
    assert_eq!(
        super::visible_player_withdraw_message(&runtime_shell, &snapshot, "CYNDAQUIL")
            .expect("withdrawal message"),
        "CYNDAQUIL, good! Come back!",
    );
}

#[test]
fn withdrawal_text_rejects_withdraw_mon_texts_nonterminating_divisor() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.battle_enemy_hp_at_player_send_out = Some(2);
    let mut snapshot = runtime_shell.shell.snapshot().expect("battle snapshot");
    let battle = snapshot.battle.as_mut().expect("active battle");
    battle.enemy_pokemon.hp = 1;
    battle.enemy_pokemon.max_hp = 3;

    let error = super::visible_player_withdraw_message(&runtime_shell, &snapshot, "CYNDAQUIL")
        .expect_err("the cartridge divide would never terminate");
    assert!(
        error
            .to_string()
            .contains("would not terminate with enemy max HP 3"),
        "{error:#}"
    );
}

#[test]
fn battle_animation_loop_count_matches_asm_body_passes() {
    let key = ("BattleAnim_Test".to_string(), 4);
    let mut loops = std::collections::BTreeMap::new();

    assert!(advance_visible_battle_animation_loop(
        &mut loops,
        key.clone(),
        3
    ));
    assert!(advance_visible_battle_animation_loop(
        &mut loops,
        key.clone(),
        3
    ));
    assert!(!advance_visible_battle_animation_loop(
        &mut loops,
        key.clone(),
        3
    ));
    assert!(loops.is_empty());

    assert!(!advance_visible_battle_animation_loop(
        &mut loops,
        key.clone(),
        1
    ));
    assert!(advance_visible_battle_animation_loop(&mut loops, key, 0));
}

#[test]
fn reset_obp0_uses_the_asm_hardware_values() {
    assert_eq!(battle_anim_reset_obp0_value(false), 0xe0);
    assert_eq!(battle_anim_reset_obp0_value(true), 0xf0);
}

#[test]
fn faint_mon_drops_one_tile_every_two_frames_then_disappears() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: "Enemy TEST\nfainted!".to_string(),
        move_id: "FAINT_MON".to_string(),
        animation_label: "BattleAnim_FaintMon".to_string(),
        player_move: false,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 14,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0,
            effect_id: "BATTLE_BG_EFFECT_FAINT_MON".to_string(),
            duration: 14,
            target: "BG_EFFECT_USER".to_string(),
            param: 0,
            incremented: false,
        }],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let source_pixel = TILE_SIZE / SOURCE_TILE_SIZE as f32;

    let enemy_y = [0_u16, 1, 2, 3, 4, 5, 12, 13].map(|frame| {
        animation.frame = frame;
        let (player, enemy) = visible_move_battler_offsets(Some(&animation));
        assert_eq!(player, Vec3::ZERO);
        enemy.y
    });

    assert_eq!(
        enemy_y,
        [-8.0, -8.0, -16.0, -16.0, -24.0, -24.0, -56.0, -56.0]
            .map(|pixels| pixels * source_pixel)
    );
    animation.frame = 0;
    assert_eq!(
        visible_move_battler_row_extractions(Some(&animation)).1,
        Some(VisibleBattlerRowExtraction {
            rows: 1,
            top: false,
            bg_rows_cleared: true,
            render_extracted: false,
        })
    );
    animation.frame = 12;
    assert_eq!(
        visible_move_battler_row_extractions(Some(&animation)).1,
        Some(VisibleBattlerRowExtraction {
            rows: 7,
            top: false,
            bg_rows_cleared: true,
            render_extracted: false,
        })
    );
    animation.frame = 13;
    assert_eq!(visible_move_battler_visibility(Some(&animation)), (true, true));
    animation.frame = 14;
    assert_eq!(visible_move_battler_visibility(Some(&animation)), (true, false));
}

#[test]
fn vibrate_mon_toggles_one_pixel_every_two_updates_for_32_frames() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "TEST_VIBRATE".to_string(),
        animation_label: "BattleAnim_TestVibrate".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 40,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0,
            effect_id: "BATTLE_BG_EFFECT_VIBRATE_MON".to_string(),
            duration: 0,
            target: "BG_EFFECT_USER".to_string(),
            param: 0,
            incremented: false,
        }],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let source_pixel = TILE_SIZE / SOURCE_TILE_SIZE as f32;

    let offsets = [0_u16, 1, 2, 3, 4, 31, 32, 33]
        .map(|frame| {
            animation.frame = frame;
            visible_move_battler_offsets(Some(&animation))
        });

    assert_eq!(offsets[0], (Vec3::ZERO, Vec3::ZERO));
    for index in 1..=6 {
        assert_eq!(offsets[index].1, Vec3::ZERO);
        assert_eq!(offsets[index].0.x.abs(), source_pixel);
        assert_eq!(offsets[index].0.y, 0.0);
    }
    assert_eq!(offsets[1].0.x, offsets[2].0.x);
    assert_eq!(offsets[3].0.x, offsets[4].0.x);
    assert_eq!(offsets[1].0.x, -offsets[3].0.x);
    assert_eq!(offsets[7], (Vec3::ZERO, Vec3::ZERO));
}

#[test]
fn wobble_mon_uses_the_radius_eight_asm_sine_until_incremented() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "TEST_WOBBLE".to_string(),
        animation_label: "BattleAnim_TestWobble".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 16,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_WOBBLE_MON".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 0,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 10,
                effect_id: "BATTLE_BG_EFFECT_WOBBLE_MON".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let source_pixel = TILE_SIZE / SOURCE_TILE_SIZE as f32;

    let player_y = [0_u16, 1, 2, 3, 4, 5, 6, 9, 10].map(|frame| {
        animation.frame = frame;
        let (player, enemy) = visible_move_battler_offsets(Some(&animation));
        assert_eq!(enemy, Vec3::ZERO);
        player.y
    });

    // The renderer's positive source-SCY displacement maps downward in Bevy.
    assert_eq!(
        player_y,
        [0.0, 0.0, -3.0, -5.0, -7.0, -8.0, -7.0, 0.0, 0.0]
            .map(|pixels| pixels * source_pixel)
    );
}

#[test]
fn wobble_player_uses_the_radius_six_asm_sine_for_32_updates() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "TEST_PLAYER_WOBBLE".to_string(),
        animation_label: "BattleAnim_TestPlayerWobble".to_string(),
        player_move: false,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 40,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0,
            effect_id: "BATTLE_BG_EFFECT_WOBBLE_PLAYER".to_string(),
            duration: 0,
            target: "0".to_string(),
            param: 0,
            incremented: false,
        }],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let source_pixel = TILE_SIZE / SOURCE_TILE_SIZE as f32;

    let player_x = [0_u16, 1, 2, 5, 9, 17, 25, 32, 33].map(|frame| {
        animation.frame = frame;
        let (player, enemy) = visible_move_battler_offsets(Some(&animation));
        assert_eq!(enemy, Vec3::ZERO);
        player.x
    });

    assert_eq!(
        player_x,
        [0.0, 0.0, 1.0, 4.0, 6.0, 0.0, -6.0, -1.0, 0.0]
            .map(|pixels| pixels * source_pixel)
    );
}

#[test]
fn wobble_screen_uses_the_radius_six_asm_sine_for_32_updates() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "TEST_SCREEN_WOBBLE".to_string(),
        animation_label: "BattleAnim_TestScreenWobble".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 40,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0,
            effect_id: "BATTLE_BG_EFFECT_WOBBLE_SCREEN".to_string(),
            duration: 0,
            target: "0".to_string(),
            param: 0,
            incremented: false,
        }],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let source_pixel = TILE_SIZE / SOURCE_TILE_SIZE as f32;

    let screen_x = [0_u16, 1, 4, 8, 16, 24, 31, 32].map(|frame| {
        animation.frame = frame;
        visible_move_screen_offset(Some(&animation)).x
    });

    assert_eq!(
        screen_x,
        [0.0, 1.0, 4.0, 6.0, 0.0, -6.0, -1.0, 0.0]
            .map(|pixels| pixels * source_pixel)
    );
}

#[test]
fn surf_uses_prior_object_boundary_and_64_byte_wave_rotation() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "SURF".to_string(),
        animation_label: "BattleAnim_Surf".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 184,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: vec![VisibleMoveObjectEvent {
            frame: 0,
            command: VisibleMoveObjectCommand::Spawn {
                object_id: "BATTLE_ANIM_OBJ_SURF".to_string(), x: 88, y: 104, param: 8,
            },
        }],
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0, effect_id: "BATTLE_BG_EFFECT_SURF".to_string(), duration: 0,
            target: "$0".to_string(), param: 0, incremented: false,
        }], actor_species_override: None, actor_shiny_override: None,
    };
    assert!(visible_surf_line_offsets(Some(&animation)).unwrap().iter().all(|offset| *offset == 0));
    animation.frame = 1;
    let first = visible_surf_line_offsets(Some(&animation)).expect("first Surf copy");
    assert!(first[..=88].iter().all(|offset| *offset == 0));
    assert_eq!(first[89], visible_battle_anim_sine(52, 2) as i8);
    assert_eq!(first[94], visible_battle_anim_sine(62, 2) as i8);
    animation.frame = 2;
    let second = visible_surf_line_offsets(Some(&animation)).expect("second Surf copy");
    assert!(second[..=87].iter().all(|offset| *offset == 0));
    assert_eq!(second[88], visible_battle_anim_sine(52, 2) as i8);

    animation.object_events.push(VisibleMoveObjectEvent {
        frame: 10,
        command: VisibleMoveObjectCommand::Clear,
    });
    animation.frame = 10;
    assert!(visible_surf_line_offsets(Some(&animation)).is_some());
    animation.frame = 11;
    assert!(visible_surf_line_offsets(Some(&animation)).is_none());
}

#[test]
fn wave_deform_mon_grows_then_decays_per_scanline_around_increment() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "TEST_WAVE_DEFORM".to_string(),
        animation_label: "BattleAnim_TestWaveDeform".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 96,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_WAVE_DEFORM_MON".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 0,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 48,
                effect_id: "BATTLE_BG_EFFECT_WAVE_DEFORM_MON".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };

    animation.frame = 9;
    let growing = visible_wave_deform_line_offsets(Some(&animation)).expect("growing wave");
    assert_eq!(growing[47], 0, "player range begins at source line $30");
    assert_eq!(growing[49], visible_battle_anim_sine(49 * 4, 8) as i8);

    animation.frame = 48;
    let first_decay = visible_wave_deform_line_offsets(Some(&animation)).expect("first decay");
    assert_eq!(first_decay[49], visible_battle_anim_sine(49 * 4, 31) as i8);

    animation.frame = 78;
    let last_decay = visible_wave_deform_line_offsets(Some(&animation)).expect("last decay");
    assert_eq!(last_decay[52], visible_battle_anim_sine(52 * 4, 1) as i8);

    animation.frame = 79;
    assert!(
        visible_wave_deform_line_offsets(Some(&animation))
            .expect("cleared wave buffer")
            .iter()
            .all(|offset| *offset == 0)
    );
}

#[test]
fn shared_screen_shake_counter_matches_the_asm_byte_state_machine() {
    let rollout = VisibleMoveBgEvent {
        frame: 0,
        effect_id: "BATTLE_BG_EFFECT_ROLLOUT".to_string(),
        duration: 0x60,
        target: "$1".to_string(),
        param: 0x01,
        incremented: false,
    };
    assert_eq!(
        [0_u16, 1, 2, 3, 94, 95, 96]
            .map(|age| visible_bg_shake_amount(&rollout, age)),
        [Some(1), Some(-1), Some(1), Some(-1), Some(1), Some(-1), None]
    );

    let grouped = VisibleMoveBgEvent {
        frame: 0,
        effect_id: "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y".to_string(),
        duration: 0x20,
        target: "$2".to_string(),
        param: 0x20,
        incremented: false,
    };
    assert_eq!(
        [0_u16, 1, 2, 3, 4, 5, 6]
            .map(|age| visible_bg_shake_amount(&grouped, age)),
        [Some(-2), Some(-2), Some(-2), Some(2), Some(2), Some(2), Some(-2)]
    );
}

#[test]
fn psychic_teleport_and_night_shade_rotate_the_asm_sine_buffer() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "PSYCHIC".to_string(),
        animation_label: "BattleAnim_TestWavyScreen".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 16,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0, effect_id: "BATTLE_BG_EFFECT_PSYCHIC".to_string(), duration: 0,
            target: "$0".to_string(), param: 0, incremented: false,
        }],
        actor_species_override: None, actor_shiny_override: None,
    };
    let setup = visible_psychic_teleport_line_x_offsets(Some(&animation)).expect("setup wave");
    assert_eq!(setup[0], 0, "DeformScreen excludes hLYOverrideStart");
    assert_eq!(setup[1], visible_battle_anim_sine(6, 5) as i8);
    animation.frame = 1;
    let first = visible_psychic_teleport_line_x_offsets(Some(&animation)).expect("rotation");
    assert_eq!(first[0], setup[1]);
    animation.frame = 4;
    assert_eq!(visible_psychic_teleport_line_x_offsets(Some(&animation)), Some(first));
    animation.frame = 5;
    assert_eq!(visible_psychic_teleport_line_x_offsets(Some(&animation)).unwrap()[0], setup[2]);

    animation.bg_events[0].effect_id = "BATTLE_BG_EFFECT_TELEPORT".to_string();
    animation.frame = 2;
    assert_eq!(visible_psychic_teleport_line_x_offsets(Some(&animation)).unwrap()[0], setup[2]);

    animation.bg_events[0].effect_id = "BATTLE_BG_EFFECT_NIGHT_SHADE".to_string();
    animation.bg_events[0].param = 8;
    animation.frame = 0;
    let night = visible_night_shade_line_y_offsets(Some(&animation)).expect("Night Shade wave");
    assert_eq!(night[0], 0);
    assert_eq!(night[1], visible_battle_anim_sine(8, 2) as i8);
    assert!(visible_psychic_teleport_line_x_offsets(Some(&animation)).is_none());
    animation.bg_events.push(VisibleMoveBgEvent {
        frame: 1, effect_id: "BATTLE_BG_EFFECT_NIGHT_SHADE".to_string(), duration: 0,
        target: String::new(), param: 0, incremented: true,
    });
    animation.frame = 1;
    assert!(visible_night_shade_line_y_offsets(Some(&animation)).is_none());
}

#[test]
fn whirlpool_and_water_use_their_asm_scy_buffers_without_colour_overlays() {
    let event = |frame, effect_id: &str, duration, target: &str, param, incremented| {
        VisibleMoveBgEvent {
            frame,
            effect_id: effect_id.to_string(),
            duration,
            target: target.to_string(),
            param,
            incremented,
        }
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "WATER_TEST".to_string(),
        animation_label: "BattleAnim_WaterTest".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 32,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![event(0, "BATTLE_BG_EFFECT_WHIRLPOOL", 0, "$0", 0, false)],
        actor_species_override: None, actor_shiny_override: None,
    };
    let whirlpool = visible_whirlpool_line_y_offsets(Some(&animation)).expect("Whirlpool setup");
    assert_eq!(whirlpool[0], 0);
    assert_eq!(whirlpool[1], visible_battle_anim_sine(2, 2) as i8);
    animation.frame = 1;
    assert_eq!(visible_whirlpool_line_y_offsets(Some(&animation)).unwrap()[0], whirlpool[1]);
    animation.bg_events.push(event(2, "BATTLE_BG_EFFECT_WHIRLPOOL", 0, "", 0, true));
    animation.frame = 2;
    assert!(visible_whirlpool_line_y_offsets(Some(&animation)).is_none());

    animation.bg_events = vec![
        event(0, "BATTLE_BG_EFFECT_START_WATER", 0, "BG_EFFECT_TARGET", 0, false),
        event(1, "BATTLE_BG_EFFECT_WATER", 0x1c, "$0", 0, false),
    ];
    animation.frame = 2;
    let enemy_water = visible_water_line_y_offsets(Some(&animation)).expect("enemy water");
    assert_eq!(enemy_water[28], visible_battle_anim_sine(4, 3) as i8);
    assert_eq!(enemy_water[27], visible_battle_anim_sine(8, 3) as i8);
    assert_eq!(enemy_water[29], visible_battle_anim_sine(8, 3) as i8);
    assert!(enemy_water[55..].iter().all(|offset| *offset == 0));
    animation.frame = 17;
    assert!(visible_water_line_y_offsets(Some(&animation)).unwrap().iter().all(|offset| *offset == 0));
    animation.bg_events.push(event(18, "BATTLE_BG_EFFECT_END_WATER", 0, "$0", 0, false));
    animation.frame = 18;
    assert!(visible_water_line_y_offsets(Some(&animation)).is_none());

    animation.bg_events = vec![
        event(0, "BATTLE_BG_EFFECT_START_WATER", 0, "BG_EFFECT_USER", 0, false),
        event(1, "BATTLE_BG_EFFECT_WATER", 0x30, "$0", 0, false),
    ];
    animation.frame = 2;
    let player_water = visible_water_line_y_offsets(Some(&animation)).expect("player water");
    assert_eq!(player_water[94], visible_battle_anim_sine(8, 3) as i8);
    assert!(player_water[..47].iter().all(|offset| *offset == 0));
}

#[test]
fn beta_send_out_mon2_decays_its_asm_scx_deformation_for_64_updates() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "BETA_SEND_OUT".to_string(),
        animation_label: "BattleAnim_BetaSendOut".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 66,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0, effect_id: "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON2".to_string(),
            duration: 0, target: "BG_EFFECT_USER".to_string(), param: 0,
            incremented: false,
        }], actor_species_override: None, actor_shiny_override: None,
    };
    let setup = visible_beta_send_out_mon2_line_x_offsets(Some(&animation)).expect("setup");
    assert!(setup.iter().all(|offset| *offset == 0));
    animation.frame = 1;
    let radius_eight = visible_beta_send_out_mon2_line_x_offsets(Some(&animation)).unwrap();
    assert!(radius_eight[..=0x2f].iter().all(|offset| *offset == 0));
    assert_eq!(radius_eight[0x30], visible_battle_anim_sine(0x80, 8) as i8);
    assert_eq!(radius_eight[0x31], visible_battle_anim_sine(0x88, 8) as i8);
    animation.frame = 8;
    let radius_seven = visible_beta_send_out_mon2_line_x_offsets(Some(&animation)).unwrap();
    assert_eq!(radius_seven[0x30], visible_battle_anim_sine(0x50, 7) as i8);
    animation.frame = 64;
    assert!(visible_beta_send_out_mon2_line_x_offsets(Some(&animation)).unwrap().iter().all(|offset| *offset == 0));
    animation.frame = 65;
    assert!(visible_beta_send_out_mon2_line_x_offsets(Some(&animation)).is_none());
}

#[test]
fn beta_send_out_mon1_replays_the_two_pass_bgp_scanline_fill() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "BETA_SEND_OUT_1".to_string(),
        animation_label: "BattleAnim_SendOutMon.Unknown".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 101,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0, effect_id: "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1".to_string(),
                duration: 0, target: "BG_EFFECT_USER".to_string(), param: 0,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 5, effect_id: "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1".to_string(),
                duration: 0, target: String::new(), param: 0, incremented: true,
            },
            VisibleMoveBgEvent {
                frame: 101, effect_id: "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1".to_string(),
                duration: 0, target: String::new(), param: 0, incremented: true,
            },
        ],
        actor_species_override: None, actor_shiny_override: None,
    };
    let setup = visible_beta_send_out_mon1_line_bgps(Some(&animation)).expect("setup BGP");
    assert_eq!(setup[0x2e], 0xe4);
    assert!(setup[0x2f..=0x5e].iter().all(|bgp| *bgp == 0));
    animation.frame = 13;
    let first_pass = visible_beta_send_out_mon1_line_bgps(Some(&animation)).unwrap();
    assert_eq!((first_pass[0x2f], first_pass[0x30], first_pass[0x31]), (0x40, 0, 0x40));
    animation.frame = 38;
    let second_pass = visible_beta_send_out_mon1_line_bgps(Some(&animation)).unwrap();
    assert_eq!((second_pass[0x2f], second_pass[0x30], second_pass[0x31]), (0xe4, 0, 0xe4));
    assert_eq!(second_pass[0x5e], 0);
    animation.frame = 62;
    assert!(visible_beta_send_out_mon1_line_bgps(Some(&animation)).unwrap()[0x2f..=0x5e]
        .iter().all(|bgp| *bgp == 0xe4));
    animation.frame = 101;
    assert!(visible_beta_send_out_mon1_line_bgps(Some(&animation)).is_none());
    assert_eq!(visible_move_battler_offsets(Some(&animation)), (Vec3::ZERO, Vec3::ZERO));
}

#[test]
fn beta_send_out_mon1_bgp_keeps_mapped_shade_zero_opaque() {
    let mut images = Assets::<Image>::default();
    let source = Image::new(
        Extent3d { width: 4, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![255, 255, 255, 0, 200, 200, 200, 255, 100, 100, 100, 255, 10, 10, 10, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let source = SpriteFrame {
        handle: images.add(source),
        size: Vec2::new(4.0, 1.0),
    };
    let mut rendered_art = RenderedTilesetArt::default();
    let white = battle_battler_bgp_frame(&mut rendered_art, &mut images, &source, 0x00)
        .expect("all-white BGP frame");
    assert_eq!(
        images.get(&white.handle).unwrap().data,
        [255, 255, 255, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]
    );

    let stepped = battle_battler_bgp_frame(&mut rendered_art, &mut images, &source, 0x90)
        .expect("stepped BGP frame");
    assert_eq!(
        images.get(&stepped.handle).unwrap().data,
        [255, 255, 255, 0, 255, 255, 255, 255, 200, 200, 200, 255, 100, 100, 100, 255]
    );
}

#[test]
fn enter_and_return_mon_follow_the_asm_four_update_resize_entries() {
    let effect = |effect_id: &str, target: &str| VisibleMoveBgEvent {
        frame: 0,
        effect_id: effect_id.to_string(),
        duration: 0,
        target: target.to_string(),
        param: 0,
        incremented: false,
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "RESIZE_TEST".to_string(),
        animation_label: "BattleAnim_ResizeTest".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 32,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![effect("BATTLE_BG_EFFECT_ENTER_MON", "BG_EFFECT_USER")],
        actor_species_override: None, actor_shiny_override: None,
    };
    for (frame, expected) in [(0, Some(2)), (3, Some(2)), (4, Some(4)), (8, Some(6)), (11, Some(6)), (12, None)] {
        animation.frame = frame;
        assert_eq!(visible_move_battler_clip_tiles(Some(&animation)).0, expected);
    }

    animation.player_move = false;
    animation.bg_events = vec![effect("BATTLE_BG_EFFECT_RETURN_MON", "BG_EFFECT_USER")];
    for (frame, expected) in [(0, Some(7)), (3, Some(7)), (4, Some(5)), (8, Some(3)), (11, Some(3)), (12, None)] {
        animation.frame = frame;
        assert_eq!(visible_move_battler_clip_tiles(Some(&animation)).1, expected);
        assert_eq!(visible_move_battler_visibility(Some(&animation)).1, frame < 12);
    }
}

#[test]
fn global_dmg_palette_effects_write_registers_on_their_asm_reload_cadence() {
    let event = |frame, effect_id: &str, battle_turn, param| VisibleMoveBgEvent {
        frame,
        effect_id: effect_id.to_string(),
        duration: 0,
        target: format!("${battle_turn:x}"),
        param,
        incremented: false,
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "PALETTE_TEST".to_string(),
        animation_label: "BattleAnim_PaletteTest".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 40,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![event(0, "BATTLE_BG_EFFECT_FLASH_INVERTED", 4, 3)],
        actor_species_override: None, actor_shiny_override: None,
    };
    for (frame, expected) in [(0, 0xe4), (4, 0xe4), (5, 0x1b), (9, 0x1b), (10, 0xe4)] {
        animation.frame = frame;
        assert_eq!(visible_battle_dmg_palette_registers(Some(&animation)).bgp, expected);
    }

    animation.bg_events = vec![event(0, "BATTLE_BG_EFFECT_WHITE_HUES", 8, 0)];
    for (frame, expected) in [(0, 0xe4), (8, 0xe4), (9, 0xe0), (18, 0xd0), (27, 0xd0)] {
        animation.frame = frame;
        assert_eq!(visible_battle_dmg_palette_registers(Some(&animation)).bgp, expected);
    }

    animation.bg_events = vec![
        event(0, "BATTLE_PALETTE_OBP1", 0, 0x1b),
        event(2, "BATTLE_BG_EFFECT_ALTERNATE_HUES", 2, 0),
    ];
    animation.frame = 2;
    assert_eq!(
        visible_battle_dmg_palette_registers(Some(&animation)),
        VisibleBattleDmgPaletteRegisters { bgp: 0xe4, obp0: 0xe4, obp1: 0xe4 }
    );
    animation.frame = 5;
    let registers = visible_battle_dmg_palette_registers(Some(&animation));
    assert_eq!((registers.bgp, registers.obp1), (0xf8, 0xf8));

    for (effect_id, cycled) in [
        ("BATTLE_BG_EFFECT_CYCLE_OBPALS_GRAY_AND_YELLOW", 0x90),
        ("BATTLE_BG_EFFECT_CYCLE_MID_OBPALS_GRAY_AND_YELLOW", 0xd8),
    ] {
        animation.bg_events = vec![event(0, effect_id, 2, 0)];
        for (frame, expected) in [(0, 0xe4), (2, 0xe4), (3, cycled), (6, 0xe4)] {
            animation.frame = frame;
            assert_eq!(visible_battle_dmg_palette_registers(Some(&animation)).obp0, expected);
        }
    }
}

#[test]
fn targeted_battler_palettes_use_rapid_cycle_pals_indexed_cadence() {
    let event = |frame, effect_id: &str, target: &str, param, incremented| VisibleMoveBgEvent {
        frame,
        effect_id: effect_id.to_string(),
        duration: 0,
        target: target.to_string(),
        param,
        incremented,
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "TARGET_PAL_TEST".to_string(),
        animation_label: "BattleAnim_TargetPalTest".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 32,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![event(0, "BATTLE_BG_EFFECT_CYCLE_MON_LIGHT_DARK_REPEATING", "BG_EFFECT_USER", 0x20, false)],
        actor_species_override: None, actor_shiny_override: None,
    };
    for (frame, expected) in [(0, None), (3, None), (4, Some(0xf8)), (7, Some(0xfc)), (10, Some(0xf8)), (13, None), (16, Some(0x90))] {
        animation.frame = frame;
        assert_eq!(visible_move_battler_bgps(Some(&animation)).0, expected);
    }

    animation.bg_events.push(event(17, "BATTLE_BG_EFFECT_CYCLE_MON_LIGHT_DARK_REPEATING", "", 0, true));
    animation.frame = 17;
    assert_eq!(visible_move_battler_bgps(Some(&animation)), (None, None));

    animation.bg_events = vec![event(0, "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT", "BG_EFFECT_TARGET", 0x40, false)];
    for (frame, expected) in [(0, None), (5, None), (6, Some(0x90)), (11, Some(0x40)), (16, Some(0x40))] {
        animation.frame = frame;
        assert_eq!(visible_move_battler_bgps(Some(&animation)).1, expected);
    }
}

#[test]
fn lunge_background_effects_match_their_asm_jump_tables() {
    assert_eq!(
        (0_u16..13).map(visible_tackle_lunge_offset).collect::<Vec<_>>(),
        vec![
            Some(0), Some(0), Some(2), Some(4), Some(6), Some(8), Some(10), Some(8),
            Some(6), Some(4), Some(2), Some(0), None,
        ]
    );
    assert_eq!(visible_beta_pursuit_offset(6), Some(-10));
    assert_eq!(visible_beta_pursuit_offset(12), None);

    assert_eq!(
        [0_u16, 1, 2, 5, 6, 20, 39, 40, 41, 44, 45, 46]
            .map(|age| visible_vital_throw_offset(age, Some(40))),
        [
            Some(0), Some(0), Some(-2), Some(-8), Some(-10), Some(-10), Some(-10),
            Some(-10), Some(-8), Some(-2), Some(0), Some(0),
        ]
    );
    assert_eq!(visible_vital_throw_offset(47, Some(40)), None);
}

#[test]
fn bounce_down_uses_the_asm_cosine_until_incremented() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "BODY_SLAM".to_string(),
        animation_label: "BattleAnim_BodySlam".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 40,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_BOUNCE_DOWN".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 0,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 32,
                effect_id: "BATTLE_BG_EFFECT_BOUNCE_DOWN".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let effect = animation.bg_events[0].clone();

    let offsets = [0_u16, 1, 2, 5, 9, 17, 25, 31, 32].map(|frame| {
        animation.frame = frame;
        visible_bounce_down_offset(&animation, &effect)
    });
    assert_eq!(
        offsets,
        [Some(0), Some(1), Some(2), Some(6), Some(17), Some(33), Some(17), Some(3), None]
    );

    animation.frame = 17;
    let lines = visible_bounce_down_line_y_offsets(Some(&animation)).expect("bounce scanlines");
    assert_eq!(lines[0x2c], 0);
    assert_eq!(lines[0x2d] as u8, 0x90);
    assert_eq!(lines[0x4d] as u8, 0x90);
    assert_eq!(lines[0x4e], !33_u8 as i8);
    assert_eq!(lines[0x5e], !33_u8 as i8);

    animation.frame = 32;
    assert!(visible_bounce_down_line_y_offsets(Some(&animation)).is_none());
}

#[test]
fn flail_combines_the_two_asm_sines_on_target_scanlines() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "FLAIL".to_string(),
        animation_label: "BattleAnim_Flail".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 32,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_FLAIL".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 0,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 32,
                effect_id: "BATTLE_BG_EFFECT_FLAIL".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };

    for frame in [0_u16, 1, 2, 5, 9, 17, 31] {
        animation.frame = frame;
        let offsets = visible_flail_line_x_offsets(Some(&animation)).expect("active Flail");
        let update = frame.saturating_sub(1) as u8;
        let expected = if frame == 0 {
            0
        } else {
            visible_battle_anim_sine(update.wrapping_mul(2), 6)
                + visible_battle_anim_sine(update.wrapping_mul(8), 2)
        } as i8;
        assert_eq!(offsets[0x2e], 0);
        assert_eq!(offsets[0x2f], expected);
        assert_eq!(offsets[0x5e], expected);
    }
    animation.frame = 32;
    assert!(visible_flail_line_x_offsets(Some(&animation)).is_none());
    assert_eq!(visible_move_battler_offsets(Some(&animation)), (Vec3::ZERO, Vec3::ZERO));
}

#[test]
fn dig_expands_its_vertical_displacement_in_four_step_bursts() {
    assert_eq!(
        [0_u16, 1, 2, 3, 4, 5, 20, 21, 22, 23, 24, 25, 40, 41]
            .map(visible_dig_displacement),
        [0, -3, -5, -7, -9, -9, -9, -11, -13, -15, -17, -17, -17, -19]
    );

    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "DIG".to_string(),
        animation_label: "BattleAnim_Dig".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 24,
        total_frames: 136,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_DIG".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 1,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 136,
                effect_id: "BATTLE_BG_EFFECT_DIG".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let offsets = visible_dig_line_y_offsets(Some(&animation)).expect("active Dig displacement");
    assert_eq!(offsets[0x2e], 0);
    assert_eq!(offsets[0x2f] as u8, 0x90);
    assert_eq!(offsets[0x3e] as u8, 0x90);
    assert_eq!(offsets[0x3f], -17);
    assert_eq!(offsets[0x5e], -17);
    animation.frame = 136;
    assert!(visible_dig_line_y_offsets(Some(&animation)).is_none());
}

#[test]
fn double_team_expands_oscillates_contracts_and_clears_on_two_increments() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "DOUBLE_TEAM".to_string(),
        animation_label: "BattleAnim_DoubleTeam".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 120,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_DOUBLE_TEAM".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 0,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 96,
                effect_id: "BATTLE_BG_EFFECT_DOUBLE_TEAM".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
            VisibleMoveBgEvent {
                frame: 120,
                effect_id: "BATTLE_BG_EFFECT_DOUBLE_TEAM".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };

    for (frame, expected) in [
        (0_u16, 0_i8),
        (1, 0),
        (2, 1),
        (16, 15),
        (17, 15),
        (18, 16),
        (22, 18),
        (26, 16),
        (30, 14),
        (96, 15),
        (97, 14),
        (111, 0),
        (112, 0),
        (119, 0),
    ] {
        animation.frame = frame;
        let offsets = visible_double_team_line_x_offsets(Some(&animation))
            .expect("active Double Team scanlines");
        assert_eq!(offsets[0x2e], 0);
        assert_eq!(offsets[0x2f], expected, "frame {frame}");
        assert_eq!(offsets[0x30], -expected, "frame {frame}");
        assert_eq!(offsets[0x31], expected, "frame {frame}");
    }
    animation.frame = 120;
    assert!(visible_double_team_line_x_offsets(Some(&animation)).is_none());
}

#[test]
fn acid_armor_shifts_its_vertical_sine_buffer_downward_without_palette_tint() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "ACID_ARMOR".to_string(),
        animation_label: "BattleAnim_AcidArmor".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 64,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_ACID_ARMOR".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 8,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 64,
                effect_id: "BATTLE_BG_EFFECT_ACID_ARMOR".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };

    let initial = visible_acid_armor_line_y_offsets(Some(&animation)).expect("initial melt");
    assert_eq!(initial[0x2f], 0);
    assert_eq!(initial[0x34], visible_battle_anim_sine(0x34 * 2, 8) as i8);
    assert_eq!(initial[0x5d], 0);
    assert_eq!(initial[0x5e], 0);

    animation.frame = 1;
    let shifted = visible_acid_armor_line_y_offsets(Some(&animation)).expect("shifted melt");
    assert_eq!(shifted[0x2f] as u8, 0x90);
    assert_eq!(shifted[0x30], initial[0x2f]);
    assert_eq!(shifted[0x35], initial[0x34]);
    assert!(
        !matches!(shifted[0x5e] as u8, 1..=0x8f | 0x91..=0xff),
        "tail must retain only zero or the blanking sentinel"
    );

    animation.frame = 64;
    assert!(visible_acid_armor_line_y_offsets(Some(&animation)).is_none());
}

#[test]
fn withdraw_compresses_the_visible_battler_instead_of_hiding_it_immediately() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "WITHDRAW".to_string(),
        animation_label: "BattleAnim_Withdraw".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 113,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![
            VisibleMoveBgEvent {
                frame: 0,
                effect_id: "BATTLE_BG_EFFECT_WITHDRAW".to_string(),
                duration: 0,
                target: "BG_EFFECT_USER".to_string(),
                param: 0x50,
                incremented: false,
            },
            VisibleMoveBgEvent {
                frame: 113,
                effect_id: "BATTLE_BG_EFFECT_WITHDRAW".to_string(),
                duration: 0,
                target: String::new(),
                param: 0,
                incremented: true,
            },
        ],
        actor_species_override: None,
        actor_shiny_override: None,
    };

    assert_eq!(visible_move_battler_visibility(Some(&animation)), (true, true));
    assert!(
        visible_withdraw_line_y_offsets(Some(&animation))
            .expect("setup buffer")
            .iter()
            .all(|offset| *offset == 0)
    );
    animation.frame = 1;
    let first = visible_withdraw_line_y_offsets(Some(&animation)).expect("first compression");
    assert_eq!(first[0x2e], 0);
    assert_eq!(first[0x2f] as u8, 0x90);
    assert_eq!(first[0x30], -2);
    animation.frame = 15;
    let compressed = visible_withdraw_line_y_offsets(Some(&animation)).expect("compressed user");
    assert!(compressed[0x2f..0x3e].iter().all(|offset| *offset as u8 == 0x90));
    assert_eq!(compressed[0x3e], -16);
    animation.frame = 112;
    assert!(visible_withdraw_line_y_offsets(Some(&animation)).is_some());
    animation.frame = 113;
    assert!(visible_withdraw_line_y_offsets(Some(&animation)).is_none());
}

#[test]
fn transform_reveals_the_loaded_target_picture_only_at_updateactorpic() {
    let actor_event = |frame, effect_id: &str| VisibleMoveBgEvent {
        frame,
        effect_id: effect_id.to_string(),
        duration: 0,
        target: "BG_EFFECT_USER".to_string(),
        param: 0,
        incremented: false,
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "TRANSFORM".to_string(),
        animation_label: "BattleAnim_Transform".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 104,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![
            actor_event(0, "BATTLE_ACTOR_TRANSFORM"),
            actor_event(48, "BATTLE_ACTOR_UPDATEACTORPIC"),
        ],
        actor_species_override: None, actor_shiny_override: None,
    };
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)),
        (VisibleBattlerArtOverride::Unchanged, VisibleBattlerArtOverride::Unchanged)
    );
    animation.frame = 47;
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)).0,
        VisibleBattlerArtOverride::Unchanged
    );
    animation.frame = 48;
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)),
        (VisibleBattlerArtOverride::Transform, VisibleBattlerArtOverride::Unchanged)
    );

    animation.player_move = false;
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)),
        (VisibleBattlerArtOverride::Unchanged, VisibleBattlerArtOverride::Transform)
    );
}

#[test]
fn minimize_reveals_the_temporary_picture_only_at_updateactorpic() {
    let actor_event = |frame, effect_id: &str| VisibleMoveBgEvent {
        frame,
        effect_id: effect_id.to_string(),
        duration: 0,
        target: "BG_EFFECT_USER".to_string(),
        param: 0,
        incremented: false,
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "MINIMIZE".to_string(),
        animation_label: "BattleAnim_Minimize".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 104,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(),
        bg_events: vec![
            actor_event(0, "BATTLE_ACTOR_MINIMIZE"),
            actor_event(48, "BATTLE_ACTOR_UPDATEACTORPIC"),
        ],
        actor_species_override: None, actor_shiny_override: None,
    };
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)),
        (VisibleBattlerArtOverride::Unchanged, VisibleBattlerArtOverride::Unchanged)
    );
    animation.frame = 47;
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)).0,
        VisibleBattlerArtOverride::Unchanged
    );
    animation.frame = 48;
    assert_eq!(
        visible_move_battler_art_overrides(Some(&animation)),
        (VisibleBattlerArtOverride::Minimize, VisibleBattlerArtOverride::Unchanged)
    );
}

#[test]
fn battlerobj_extracts_fixed_head_or_feet_rows_instead_of_resizing_the_battler() {
    let effect = VisibleMoveBgEvent {
        frame: 0,
        effect_id: "BATTLE_BG_EFFECT_BATTLEROBJ_1ROW".to_string(),
        duration: 0,
        target: "BG_EFFECT_USER".to_string(),
        param: 0,
        incremented: false,
    };
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(), move_id: "ROW_TEST".to_string(),
        animation_label: "BattleAnim_RowTest".to_string(), player_move: true,
        started: true, waiting_for_hp: false, frame: 0, total_frames: 16,
        sound_events: Vec::new(), next_sound_event: 0, cry_events: Vec::new(),
        next_cry_event: 0, object_events: Vec::new(), bg_events: vec![effect],
        actor_species_override: None, actor_shiny_override: None,
    };
    assert_eq!(visible_move_battler_clip_tiles(Some(&animation)), (None, None));
    assert_eq!(
        visible_move_battler_row_extractions(Some(&animation)).0,
        Some(VisibleBattlerRowExtraction { rows: 1, top: true, bg_rows_cleared: false, render_extracted: true })
    );
    animation.frame = 1;
    assert_eq!(
        visible_move_battler_row_extractions(Some(&animation)).0,
        Some(VisibleBattlerRowExtraction { rows: 1, top: true, bg_rows_cleared: true, render_extracted: true })
    );
    animation.bg_events.push(VisibleMoveBgEvent {
        frame: 5, effect_id: "BATTLE_BG_EFFECT_SHOW_MON".to_string(), duration: 0,
        target: "BG_EFFECT_USER".to_string(), param: 0, incremented: false,
    });
    animation.frame = 5;
    assert_eq!(
        visible_move_battler_row_extractions(Some(&animation)).0.unwrap().bg_rows_cleared,
        false
    );
    animation.object_events.push(VisibleMoveObjectEvent {
        frame: 6, command: VisibleMoveObjectCommand::Clear,
    });
    animation.frame = 6;
    assert_eq!(visible_move_battler_row_extractions(Some(&animation)), (None, None));

    animation.object_events.clear();
    animation.bg_events.truncate(1);
    animation.bg_events[0].effect_id = "BATTLE_BG_EFFECT_BATTLEROBJ_2ROW".to_string();
    animation.player_move = false;
    animation.frame = 1;
    assert_eq!(
        visible_move_battler_row_extractions(Some(&animation)).1,
        Some(VisibleBattlerRowExtraction { rows: 2, top: false, bg_rows_cleared: true, render_extracted: true })
    );
}

#[test]
fn extracted_battler_rows_render_as_an_independent_oam_strip() {
    let frame = SpriteFrame {
        handle: Handle::default(),
        size: Vec2::splat(48.0),
    };
    let mut app = App::new();
    app.add_systems(Update, move |mut commands: Commands| {
        spawn_visible_battler_extracted_rows(
            &mut commands,
            &frame,
            Vec2::splat(192.0),
            Vec3::new(100.0, 50.0, 3.0),
            VisibleBattlerRowExtraction {
                rows: 2,
                top: true,
                bg_rows_cleared: true,
                render_extracted: true,
            },
        );
    });
    app.update();

    let mut query = app.world_mut().query_filtered::<
        (&Sprite, &Transform),
        With<BattleCommandMarker>,
    >();
    let rendered = query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(rendered.len(), 1);
    let (sprite, transform) = rendered[0];
    assert_eq!(sprite.rect, Some(Rect::new(0.0, 0.0, 48.0, 16.0)));
    assert_eq!(sprite.custom_size, Some(Vec2::new(192.0, 64.0)));
    assert_eq!(transform.translation, Vec3::new(100.0, 114.0, 3.02));
}

#[test]
fn remove_mon_shifts_whole_tiles_on_the_asm_jumptable_cadence() {
    assert_eq!(visible_remove_mon_state(0, false), (0, true));
    assert_eq!(visible_remove_mon_state(1, false), (8, true));
    assert_eq!(visible_remove_mon_state(4, false), (8, true));
    assert_eq!(visible_remove_mon_state(5, false), (16, true));
    assert_eq!(visible_remove_mon_state(28, false), (56, true));
    assert_eq!(visible_remove_mon_state(29, false), (64, false));
    assert_eq!(visible_remove_mon_state(32, false), (64, false));

    assert_eq!(visible_remove_mon_state(29, true), (-64, true));
    assert_eq!(visible_remove_mon_state(32, true), (-64, true));
    assert_eq!(visible_remove_mon_state(33, true), (-72, false));
    assert_eq!(visible_remove_mon_state(36, true), (-72, false));

    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "REMOVE_MON".to_string(),
        animation_label: "BattleAnim_TestRemoveMon".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 1,
        total_frames: 37,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0,
            effect_id: "BATTLE_BG_EFFECT_REMOVE_MON".to_string(),
            duration: 0,
            target: "BG_EFFECT_USER".to_string(),
            param: 0,
            incremented: false,
        }],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    assert_eq!(
        visible_remove_mon_clips(Some(&animation)),
        (
            Some(VisibleRemoveMonClip {
                source_pixels: 0,
                crop_left: true,
            }),
            None,
        )
    );
    animation.frame = 9;
    assert_eq!(
        visible_remove_mon_clips(Some(&animation)).0,
        Some(VisibleRemoveMonClip {
            source_pixels: 8,
            crop_left: true,
        })
    );
    animation.player_move = false;
    animation.frame = 5;
    assert_eq!(
        visible_remove_mon_clips(Some(&animation)).1,
        Some(VisibleRemoveMonClip {
            source_pixels: 8,
            crop_left: false,
        })
    );
}

#[test]
fn rollout_shakes_screen_vertically_instead_of_lunging_the_battler() {
    let mut animation = VisibleMoveAnimation {
        trigger_message: String::new(),
        move_id: "ROLLOUT".to_string(),
        animation_label: "BattleAnim_Rollout".to_string(),
        player_move: true,
        started: true,
        waiting_for_hp: false,
        frame: 0,
        total_frames: 0x60,
        sound_events: Vec::new(),
        next_sound_event: 0,
        cry_events: Vec::new(),
        next_cry_event: 0,
        object_events: Vec::new(),
        bg_events: vec![VisibleMoveBgEvent {
            frame: 0,
            effect_id: "BATTLE_BG_EFFECT_ROLLOUT".to_string(),
            duration: 0x60,
            target: "$1".to_string(),
            param: 0x01,
            incremented: false,
        }],
        actor_species_override: None,
        actor_shiny_override: None,
    };
    let source_pixel = TILE_SIZE / SOURCE_TILE_SIZE as f32;

    assert_eq!(visible_move_battler_offsets(Some(&animation)), (Vec3::ZERO, Vec3::ZERO));
    assert_eq!(visible_move_screen_offset(Some(&animation)).y, -source_pixel);
    animation.frame = 1;
    assert_eq!(visible_move_screen_offset(Some(&animation)), Vec3::ZERO);

    animation.frame = 0;
    assert_eq!(visible_rollout_object_y_offset(&animation, 0), -1);
    assert_eq!(visible_rollout_object_y_offset(&animation, 1), 0);
    animation.frame = 1;
    assert_eq!(visible_rollout_object_y_offset(&animation, 0), 0);
    animation.frame = 95;
    assert_eq!(visible_rollout_object_y_offset(&animation, 0), 0);
    animation.frame = 96;
    assert_eq!(visible_rollout_object_y_offset(&animation, 0), 0);
}

#[test]
fn master_ball_capture_timeline_includes_the_full_sparkle_wait() {
    let capture = VisibleCaptureAnimation {
        trigger_message: String::new(),
        ball_id: "MASTER_BALL".to_string(),
        animation_shakes: 4,
        blocked: false,
        caught: true,
        started: true,
        complete: false,
        sprites_cleared: false,
        frame: 0,
    };

    assert_eq!(capture.master_ball_special_frame(), Some(92));
    assert_eq!(capture.shake_entry_frame(), 156);
    assert_eq!(capture.change_dex_sound_frame(), 180);
    assert_eq!(capture.bounce_sound_frame(), 212);
    assert_eq!(capture.shake_setup_frame(), 316);
    assert_eq!(capture.first_shake_check_frame(), 364);
    assert_eq!(capture.total_frames(), 508);

    let mut visibility = capture.clone();
    visibility.frame = 155;
    assert!(!visibility.enemy_hidden());
    assert_eq!(visibility.enemy_clip_tiles(), None);
    visibility.frame = 156;
    assert_eq!(visibility.enemy_clip_tiles(), Some(7));
    visibility.frame = 164;
    assert!(visibility.enemy_hidden());
}

#[test]
fn battle_animation_numeric_parser_resolves_canonical_ball_constants() {
    assert_eq!(parse_visible_battle_animation_int("NO_ITEM"), Some(0x00));
    assert_eq!(parse_visible_battle_animation_int("MASTER_BALL"), Some(0x01));
    assert_eq!(parse_visible_battle_animation_int("ULTRA_BALL"), Some(0x02));
    assert_eq!(parse_visible_battle_animation_int("GREAT_BALL"), Some(0x04));
    assert_eq!(parse_visible_battle_animation_int("POKE_BALL"), None);
}

#[test]
fn b_cancels_visible_evolution_before_success_and_restores_exact_pokemon() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "DRAGONAIR",
            55,
            Some("BERRY".to_string()),
            None,
            "EVOLUTION_CANCEL_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Dragonair");
    let dragonite = runtime_shell
        .runtime
        .data
        .pokemon
        .get("DRAGONITE")
        .expect("Dragonite in compiled pack")
        .clone();
    let original = runtime_shell.shell.session().state.storage.party.pokemon[0]
        .as_ref()
        .expect("Dragonair in party")
        .clone();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        let evolved = state.storage.party.pokemon[0]
            .as_mut()
            .expect("Dragonair in party");
        evolved.species = dragonite;
        evolved.nickname = "DRAGONITE".to_string();
        evolved.item = None;
        evolved.hp = evolved.hp.saturating_add(17);
        evolved.max_hp = evolved.max_hp.saturating_add(17);
        evolved.attack = evolved.attack.saturating_add(29);
        let evolution_move = crate::core::models::LearnedMove {
            name: "WING_ATTACK".to_string(),
            current_pp: 35,
            pp_ups: 0,
        };
        if let Some(first_move) = evolved.moves.first_mut() {
            *first_move = evolution_move;
        } else {
            evolved.moves.push(evolution_move);
        }
        state.pending_move_learn = Some(crate::core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "DRAGONITE".to_string(),
            level: 55,
            learned_move: crate::core::models::LearnedMove {
                name: "WING_ATTACK".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        state.sync_party_from_storage();
        assert!(!state.pokedex.seen_species.contains("DRAGONITE"));
        assert!(!state.pokedex.caught_species.contains("DRAGONITE"));
    }
    let evolving = "What? DRAGONAIR is evolving!".to_string();
    let evolved = "Congratulations! DRAGONAIR evolved into DRAGONITE!".to_string();
    let pending = "DRAGONAIR is\ntrying to learn\nWING ATTACK.".to_string();
    let report = EvolutionReport {
        target_species: Some("DRAGONITE".to_string()),
        events: vec![crate::core::systems::evolution::EvolutionEvent::Text(
            "EvolvingText",
        )],
        pending_move_learns: vec![crate::core::models::LearnedMove {
            name: "WING_ATTACK".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }],
        cancel_snapshot: Some(Box::new(original.clone())),
    };
    runtime_shell.battle_messages = [evolving.clone(), evolved.clone(), pending.clone()]
        .into_iter()
        .collect();
    runtime_shell.battle_evolution_cries = [("DRAGONITE".to_string(), evolving.clone())]
        .into_iter()
        .collect();
    runtime_shell.battle_sounds_after_messages = [("SFX_CAUGHT_MON".to_string(), evolving.clone())]
        .into_iter()
        .collect();
    runtime_shell
        .battle_evolution_cancellations
        .push_back(VisibleEvolutionCancellation {
            party_index: 0,
            trigger_message: evolving,
            evolved_message: evolved.clone(),
            pending_move_messages: vec![pending.clone()],
            report,
            accepted: false,
        });
    finish_current_battle_message_for_regression(&mut runtime_shell);

    press_visible_b_button(&mut runtime_shell).expect("cancel evolution with B");

    assert_eq!(
        runtime_shell.shell.session().state.storage.party.pokemon[0].as_ref(),
        Some(&original)
    );
    assert!(
        runtime_shell
            .shell
            .session()
            .state
            .pending_move_learn
            .is_none()
    );
    assert!(
        runtime_shell
            .shell
            .session()
            .state
            .pending_move_learn_queue
            .is_empty()
    );
    assert_eq!(
        runtime_shell.battle_messages.front().map(String::as_str),
        Some("Huh? DRAGONAIR\nstopped evolving!")
    );
    assert!(!runtime_shell.battle_messages.contains(&evolved));
    assert!(!runtime_shell.battle_messages.contains(&pending));
    assert!(runtime_shell.battle_evolution_cancellations.is_empty());
    assert!(runtime_shell.battle_evolution_cries.is_empty());
    assert!(runtime_shell.battle_sounds_after_messages.is_empty());
    assert!(
        !runtime_shell
            .shell
            .session()
            .state
            .pokedex
            .seen_species
            .contains("DRAGONITE")
    );
    assert!(
        !runtime_shell
            .shell
            .session()
            .state
            .pokedex
            .caught_species
            .contains("DRAGONITE")
    );
}

#[test]
fn completed_visible_evolution_registers_the_target_species() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "DRAGONAIR",
            55,
            None,
            None,
            "EVOLUTION_POKEDEX_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Dragonair");
    let dragonite = runtime_shell.runtime.data.pokemon["DRAGONITE"].clone();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        state.storage.party.pokemon[0]
            .as_mut()
            .expect("Dragonair in party")
            .species = dragonite;
        state.sync_party_from_storage();
    }

    record_visible_completed_evolution(&mut runtime_shell, 0)
        .expect("commit completed evolution Pokedex state");

    let pokedex = &runtime_shell.shell.session().state.pokedex;
    assert!(pokedex.seen_species.contains("DRAGONITE"));
    assert!(pokedex.caught_species.contains("DRAGONITE"));

    runtime_shell.shell.session_mut().state_mut().pokedex = Default::default();
    let intro = "DRAGONAIR is trying to learn WING ATTACK.".to_string();
    runtime_shell
        .battle_evolution_cancellations
        .push_back(VisibleEvolutionCancellation {
            party_index: 0,
            trigger_message: "What? DRAGONAIR is evolving!".to_string(),
            evolved_message: "Congratulations! DRAGONAIR evolved into DRAGONITE!".to_string(),
            pending_move_messages: vec![intro],
            report: EvolutionReport {
                target_species: Some("DRAGONITE".to_string()),
                events: Vec::new(),
                pending_move_learns: vec![crate::core::models::LearnedMove {
                    name: "WING_ATTACK".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                }],
                cancel_snapshot: None,
            },
            accepted: true,
        });
    complete_visible_accepted_evolution_after_special_boundary(
        &mut runtime_shell,
        "MoveForgotPoofText",
    )
    .expect("intermediate replacement boundary");
    assert!(
        !runtime_shell
            .shell
            .session()
            .state
            .pokedex
            .seen_species
            .contains("DRAGONITE")
    );
    complete_visible_accepted_evolution_after_special_boundary(
        &mut runtime_shell,
        "LearnedMoveText",
    )
    .expect("learned move completes evolution");
    assert!(
        runtime_shell
            .shell
            .session()
            .state
            .pokedex
            .caught_species
            .contains("DRAGONITE")
    );
}

#[test]
fn accepted_visible_evolution_registers_only_after_the_evolved_text() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "DRAGONAIR",
            55,
            None,
            None,
            "EVOLUTION_POKEDEX_TIMING_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Dragonair");
    let original = runtime_shell.shell.session().state.storage.party.pokemon[0]
        .as_ref()
        .expect("Dragonair in party")
        .clone();
    let dragonite = runtime_shell.runtime.data.pokemon["DRAGONITE"].clone();
    runtime_shell.shell.session_mut().state_mut().storage.party.pokemon[0]
        .as_mut()
        .expect("Dragonair in party")
        .species = dragonite;
    let evolving = "What? DRAGONAIR is evolving!".to_string();
    let evolved = "Congratulations! DRAGONAIR evolved into DRAGONITE!".to_string();
    runtime_shell.battle_messages = [evolving.clone(), evolved.clone()].into_iter().collect();
    runtime_shell
        .battle_evolution_cancellations
        .push_back(VisibleEvolutionCancellation {
            party_index: 0,
            trigger_message: evolving,
            evolved_message: evolved,
            pending_move_messages: Vec::new(),
            report: EvolutionReport {
                target_species: Some("DRAGONITE".to_string()),
                events: Vec::new(),
                pending_move_learns: Vec::new(),
                cancel_snapshot: Some(Box::new(original)),
            },
            accepted: false,
        });

    finish_current_battle_message_for_regression(&mut runtime_shell);
    press_visible_a_button(&mut runtime_shell).expect("accept evolution");

    assert!(runtime_shell.battle_evolution_cancellations[0].accepted);
    assert!(
        !runtime_shell
            .shell
            .session()
            .state
            .pokedex
            .seen_species
            .contains("DRAGONITE")
    );

    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .sync_party_from_storage();
    finish_current_battle_message_for_regression(&mut runtime_shell);
    press_visible_a_button(&mut runtime_shell).expect("dismiss evolved text");

    assert!(runtime_shell.battle_evolution_cancellations.is_empty());
    let pokedex = &runtime_shell.shell.session().state.pokedex;
    assert!(pokedex.seen_species.contains("DRAGONITE"));
    assert!(pokedex.caught_species.contains("DRAGONITE"));
}

#[test]
fn accepted_evolution_with_a_move_registers_at_the_move_result_boundary() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "DRAGONAIR",
            55,
            None,
            None,
            "EVOLUTION_MOVE_POKEDEX_TIMING_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Dragonair");
    let original = runtime_shell.shell.session().state.storage.party.pokemon[0]
        .as_ref()
        .expect("Dragonair in party")
        .clone();
    let dragonite = runtime_shell.runtime.data.pokemon["DRAGONITE"].clone();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        state.storage.party.pokemon[0]
            .as_mut()
            .expect("Dragonair in party")
            .species = dragonite;
        state.sync_party_from_storage();
    }
    let intro = "DRAGONAIR is trying to learn WING ATTACK.".to_string();
    let result = "DRAGONAIR did not learn WING ATTACK.".to_string();
    runtime_shell
        .battle_evolution_cancellations
        .push_back(VisibleEvolutionCancellation {
            party_index: 0,
            trigger_message: "What? DRAGONAIR is evolving!".to_string(),
            evolved_message: "Congratulations! DRAGONAIR evolved into DRAGONITE!".to_string(),
            pending_move_messages: vec![intro.clone(), result.clone()],
            report: EvolutionReport {
                target_species: Some("DRAGONITE".to_string()),
                events: Vec::new(),
                pending_move_learns: vec![crate::core::models::LearnedMove {
                    name: "WING_ATTACK".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                }],
                cancel_snapshot: Some(Box::new(original)),
            },
            accepted: true,
        });

    complete_visible_accepted_evolution_after_battle_message(
        &mut runtime_shell,
        Some(&intro),
    )
    .expect("intro is not completion");
    assert!(
        !runtime_shell
            .shell
            .session()
            .state
            .pokedex
            .seen_species
            .contains("DRAGONITE")
    );

    complete_visible_accepted_evolution_after_battle_message(
        &mut runtime_shell,
        Some(&result),
    )
    .expect("result completes evolution");

    assert!(runtime_shell.battle_evolution_cancellations.is_empty());
    let pokedex = &runtime_shell.shell.session().state.pokedex;
    assert!(pokedex.seen_species.contains("DRAGONITE"));
    assert!(pokedex.caught_species.contains("DRAGONITE"));
}

#[test]
fn pending_move_learn_prompt_uses_the_exported_asm_question() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            20,
            None,
            None,
            "PENDING_MOVE_TEXT_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Cyndaquil");
    runtime_shell.shell.session_mut().state_mut().pending_move_learn =
        Some(crate::core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "CYNDAQUIL".to_string(),
            level: 20,
            learned_move: crate::core::models::LearnedMove {
                name: "HEADBUTT".to_string(),
                current_pp: 15,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
    runtime_shell.shell.session_mut().state_mut().sync_party_from_storage();

    let expected = visible_move_learning_text_pages(
        &runtime_shell,
        "_AskForgetMoveText",
        "CYNDAQUIL",
        "CYNDAQUIL",
        "HEADBUTT",
    )
    .expect("render exported move-learning text")
    .pop()
    .expect("final source question");
    let snapshot = runtime_shell
        .shell
        .presentation_snapshot()
        .expect("pending move presentation snapshot");
    let mut entries = Vec::new();
    push_visible_pending_move_learn_entries(&mut entries, &snapshot, &runtime_shell)
        .expect("render pending move learn prompt");

    assert_eq!(entries, vec![expected]);
    assert!(!entries.iter().any(|entry| entry.contains("A/B CONTINUE")));
}

#[test]
fn pending_move_replacement_uses_source_text_pause_and_sound_boundaries() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            20,
            None,
            None,
            "PENDING_MOVE_REPLACE_TEST",
            1,
            Dv::from_non_hp(10, 11, 12, 13),
        )
        .expect("add Cyndaquil");
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        let pokemon = state.storage.party.pokemon[0]
            .as_mut()
            .expect("party Pokemon");
        pokemon.moves = ["TACKLE", "LEER", "SMOKESCREEN", "EMBER"]
            .into_iter()
            .map(|name| crate::core::models::LearnedMove {
                name: name.to_string(),
                current_pp: 20,
                pp_ups: 0,
            })
            .collect();
        state.pending_move_learn = Some(crate::core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "CYNDAQUIL".to_string(),
            level: 20,
            learned_move: crate::core::models::LearnedMove {
                name: "HEADBUTT".to_string(),
                current_pp: 15,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        state.sync_party_from_storage();
    }
    runtime_shell.party_move_cursor = Some(MenuCursor {
        surface_id: party_move_cursor_surface_id(0),
        option_index: 0,
    });

    replace_visible_pending_move_learn(&mut runtime_shell)
        .expect("replace the selected move");

    assert_eq!(
        runtime_shell
            .special_boundary
            .as_ref()
            .map(|boundary| boundary.label.as_str()),
        Some("Text_1_2_and_Poof")
    );
    assert_eq!(runtime_shell.visible_special_text_pause_frames, Some(30));
    assert_eq!(
        runtime_shell
            .special_boundary_queue
            .front()
            .map(|boundary| boundary.label.as_str()),
        Some("MoveForgotPoofText")
    );
}

fn capture_ball_sprite_count(world: &mut World) -> usize {
    let mut commands = world.query_filtered::<&Transform, With<BattleCommandMarker>>();
    commands
        .iter(world)
        .filter(|transform| (transform.translation.z - 4.1).abs() < f32::EPSILON)
        .count()
}

fn assert_caught_capture_render_state(world: &mut World, ball_visible: bool) {
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    let capture = runtime_shell
        .visible_capture_animation
        .as_ref()
        .expect("caught capture presentation must remain retained");
    assert!(capture.complete && capture.caught);
    assert!(
        capture.enemy_hidden(),
        "the still-live core enemy must stay hidden until capture commit"
    );
    assert_eq!(capture.ball_visible(), ball_visible);
    assert_eq!(
        runtime_shell.last_error, None,
        "capture presentation must render without a hidden asset error"
    );
    let _ = runtime_shell;

    let mut battlers = world.query_filtered::<Entity, With<BattleBattlerMarker>>();
    assert_eq!(
        battlers.iter(world).count(),
        1,
        "only the player battler may remain while the caught enemy is pending commit"
    );
    assert_eq!(
        capture_ball_sprite_count(world),
        ball_visible as usize,
        "the retained Poké Ball entity must follow the source ClearSprites boundary"
    );
}

#[test]
fn battle_screen_offset_moves_battlers_and_commands_but_not_fixed_canvas_and_restores() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell
        .visible_move_animations
        .push_back(VisibleMoveAnimation {
            trigger_message: "screen shake".to_string(),
            move_id: "TEST_SHAKE".to_string(),
            animation_label: "BattleAnim_TestShake".to_string(),
            player_move: true,
            started: true,
            waiting_for_hp: false,
            frame: 0,
            total_frames: 4,
            sound_events: Vec::new(),
            next_sound_event: 0,
            cry_events: Vec::new(),
            next_cry_event: 0,
            object_events: Vec::new(),
            bg_events: vec![
                VisibleMoveBgEvent {
                    frame: 0,
                    effect_id: "BATTLE_BG_EFFECT_SHAKE_SCREEN_X".to_string(),
                    duration: 4,
                    target: "3".to_string(),
                    param: 0x12,
                    incremented: false,
                },
                VisibleMoveBgEvent {
                    frame: 0,
                    effect_id: "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y".to_string(),
                    duration: 4,
                    target: "2".to_string(),
                    param: 0x12,
                    incremented: false,
                },
            ],
            actor_species_override: None,
            actor_shiny_override: None,
        });
    let expected_offset = visible_move_screen_offset(runtime_shell.visible_move_animations.front());
    assert_ne!(
        expected_offset,
        Vec3::ZERO,
        "fixture must produce a screen shake"
    );

    let battler_origin = Vec3::new(10.0, 20.0, 3.0);
    let command_origin = Vec3::new(-4.0, 7.0, 4.0);
    let canvas_origin = Vec3::new(1.0, 2.0, 2.7);
    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .add_systems(Update, apply_visible_battle_screen_offset);
    let battler = app
        .world_mut()
        .spawn((
            Transform::from_translation(battler_origin),
            BattleBattlerMarker,
        ))
        .id();
    let command = app
        .world_mut()
        .spawn((
            Transform::from_translation(command_origin),
            BattleCommandMarker,
        ))
        .id();
    let canvas = app
        .world_mut()
        .spawn((
            Transform::from_translation(canvas_origin),
            BattleCommandMarker,
            FixedBattleCanvasMarker,
        ))
        .id();

    app.update();
    assert_eq!(
        app.world()
            .entity(battler)
            .get::<Transform>()
            .unwrap()
            .translation,
        battler_origin + expected_offset
    );
    assert_eq!(
        app.world()
            .entity(command)
            .get::<Transform>()
            .unwrap()
            .translation,
        command_origin + expected_offset
    );
    assert_eq!(
        app.world()
            .entity(canvas)
            .get::<Transform>()
            .unwrap()
            .translation,
        canvas_origin,
        "the full-screen battle canvas must remain anchored to the LCD"
    );

    app.world_mut()
        .resource_mut::<BevyRuntimeShell>()
        .visible_move_animations
        .clear();
    app.update();
    assert_eq!(
        app.world()
            .entity(battler)
            .get::<Transform>()
            .unwrap()
            .translation,
        battler_origin,
        "battler offset must be removed when the shake ends"
    );
    assert_eq!(
        app.world()
            .entity(command)
            .get::<Transform>()
            .unwrap()
            .translation,
        command_origin,
        "command/HUD offset must be removed when the shake ends"
    );
    assert_eq!(
        app.world()
            .entity(canvas)
            .get::<Transform>()
            .unwrap()
            .translation,
        canvas_origin
    );
}

#[test]
fn caught_capture_retains_then_clears_sprites_without_revealing_enemy_before_commit() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    let battle_scene = runtime_shell
        .shell
        .snapshot()
        .expect("active capture battle snapshot");
    let unrelated = "An unrelated queued message.".to_string();
    let gotcha = "Gotcha! SUDOWOODO\nwas caught!".to_string();
    let pokedex = "SUDOWOODO's data\nwas newly added to\nthe POKéDEX.".to_string();
    runtime_shell.battle_messages.push_back(unrelated);
    runtime_shell.battle_messages.push_back(gotcha.clone());
    runtime_shell.battle_messages.push_back(pokedex.clone());
    runtime_shell.battle_message_scene = Some(Box::new(battle_scene));
    runtime_shell.visible_capture_animation = Some(VisibleCaptureAnimation {
        trigger_message: "Player used POKé BALL!".to_string(),
        ball_id: "POKE_BALL".to_string(),
        animation_shakes: 3,
        blocked: false,
        caught: true,
        started: false,
        complete: true,
        sprites_cleared: false,
        frame: 228 + 48 * 3,
    });
    let outcome = crate::core::battle::capture::CaptureOutcome {
        caught: true,
        blocked: false,
        storage_full: false,
        wobble_count: 3,
        animation_shakes: 3,
        final_catch_rate: u8::MAX,
        ball_id: Some("POKE_BALL".to_string()),
    };
    runtime_shell.pending_standard_capture = Some(PendingStandardCapture {
        outcome,
        scripted_static_wild: None,
        default_name: "SUDOWOODO".to_string(),
        prompt_for_nickname: true,
    });
    finish_current_battle_message_for_regression(&mut runtime_shell);

    let mut app = battle_render_regression_app(runtime_shell);
    app.update();
    assert_caught_capture_render_state(app.world_mut(), true);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        press_visible_a_button(&mut runtime_shell).expect("dismiss unrelated battle page");
        assert_eq!(runtime_shell.battle_messages.front(), Some(&gotcha));
        assert!(
            runtime_shell
                .visible_capture_animation
                .as_ref()
                .is_some_and(|capture| !capture.sprites_cleared),
            "an unrelated page must not clear the retained caught ball"
        );
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), true);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        finish_current_battle_message_for_regression(&mut runtime_shell);
        press_visible_a_button(&mut runtime_shell).expect("dismiss Gotcha page");
        assert_eq!(runtime_shell.battle_messages.front(), Some(&pokedex));
        assert!(
            runtime_shell
                .visible_capture_animation
                .as_ref()
                .is_some_and(|capture| capture.sprites_cleared),
            "Gotcha dismissal must execute the source ClearSprites boundary"
        );
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        finish_current_battle_message_for_regression(&mut runtime_shell);
        press_visible_a_button(&mut runtime_shell).expect("open scripted Pokedex entry");
        assert!(runtime_shell.pokedex_scripted_entry);
        assert!(runtime_shell.pokedex_detail_open);
        assert_eq!(
            runtime_shell
                .last_audio_events
                .iter()
                .filter(|event| event.contains("queued new_pokedex_entry cry"))
                .count(),
            1,
            "NewPokedexEntry is the capture flow's only species-cry boundary"
        );
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);
    assert!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .battle_messages
            .is_empty(),
        "opening the scripted Pokedex entry must consume the final capture text"
    );

    for step in 0..8 {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        if !runtime_shell.pokedex_menu_open {
            break;
        }
        let capture_pending_before = runtime_shell.pending_standard_capture.is_some();
        press_visible_a_button(&mut runtime_shell).expect("advance scripted Pokedex entry");
        assert!(
            runtime_shell.battle_messages.is_empty(),
            "scripted Pokedex step {step} resumed the unfinished battle: capture_before={} capture_after={} name_choice={} {:?}",
            capture_pending_before,
            runtime_shell.pending_standard_capture.is_some(),
            runtime_shell.pending_name_choice.is_some(),
            runtime_shell.battle_messages,
        );
    }
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            !runtime_shell.pokedex_menu_open,
            "scripted Pokedex entry did not close: detail={} page={} battle_messages={:?} action={:?}",
            runtime_shell.pokedex_detail_open,
            runtime_shell.pokedex_detail_page,
            runtime_shell.battle_messages,
            runtime_shell.last_runtime_action
        );
        assert!(runtime_shell.pending_name_choice.is_some());
        assert!(runtime_shell.visible_capture_animation.is_some());
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        confirm_visible_name_choice(&mut runtime_shell).expect("choose to nickname capture");
        assert!(runtime_shell.pending_name_input.is_some());
        assert!(runtime_shell.visible_capture_animation.is_some());
    }
    app.update();
    assert_caught_capture_render_state(app.world_mut(), false);

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        confirm_visible_player_name_input(&mut runtime_shell)
            .expect("commit captured Pokemon nickname");
        assert!(runtime_shell.pending_standard_capture.is_none());
        assert!(runtime_shell.shell.snapshot().unwrap().battle.is_none());
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .all(|event| !event.contains("battle_capture_complete cry")),
            "capture storage/exit must not invent a second species cry after NewPokedexEntry"
        );
        assert!(
            runtime_shell.visible_capture_animation.is_none(),
            "capture presentation must clear only after authoritative capture commit"
        );
    }
    app.update();
    let world = app.world_mut();
    let mut battlers = world.query_filtered::<Entity, With<BattleBattlerMarker>>();
    assert_eq!(battlers.iter(world).count(), 0);
    assert_eq!(capture_ball_sprite_count(world), 0);
}

#[test]
fn new_contest_capture_stays_live_through_pokedex_then_skips_nickname() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    let contest_pokemon = runtime_shell
        .runtime
        .data
        .create_pokemon("PIDGEY", 4, Dv::from_non_hp(10, 10, 10, 10))
        .expect("materialize a canonical Route36 Contest encounter");
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        let crate::core::state::BattleMemory::StaticWild {
            battle_music,
            roaming_slot,
            ..
        } = state.battle.clone()
        else {
            panic!("contest capture fixture requires a static wild battle");
        };
        state.battle = crate::core::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_CONTEST".to_string(),
            battle_music,
            map_name: "Route36".to_string(),
            roaming_slot,
            enemy_pokemon: contest_pokemon.clone(),
            enemy_party: vec![contest_pokemon.clone()],
        };
        if let Some(combat) = state.script_runtime.active_battle_combat.as_mut() {
            combat.enemy = contest_pokemon.clone();
            combat.enemy_party = vec![contest_pokemon.clone()];
            combat.enemy_party_index = 0;
        }
    }
    let battle_scene = runtime_shell
        .shell
        .snapshot()
        .expect("active Contest capture battle snapshot");
    let gotcha = "Gotcha! PIDGEY\nwas caught!".to_string();
    let pokedex = "PIDGEY's data\nwas newly added to\nthe POKéDEX.".to_string();
    runtime_shell.battle_messages.push_back(gotcha.clone());
    runtime_shell.battle_messages.push_back(pokedex.clone());
    runtime_shell.battle_message_scene = Some(Box::new(battle_scene));
    runtime_shell.visible_capture_animation = Some(VisibleCaptureAnimation {
        trigger_message: "Player used POKé BALL!".to_string(),
        ball_id: "POKE_BALL".to_string(),
        animation_shakes: 3,
        blocked: false,
        caught: true,
        started: true,
        complete: true,
        sprites_cleared: false,
        frame: 228 + 48 * 3,
    });
    runtime_shell.pending_standard_capture = Some(PendingStandardCapture {
        outcome: crate::core::battle::capture::CaptureOutcome {
            caught: true,
            blocked: false,
            storage_full: false,
            wobble_count: 3,
            animation_shakes: 3,
            final_catch_rate: u8::MAX,
            ball_id: Some("POKE_BALL".to_string()),
        },
        scripted_static_wild: None,
        default_name: "PIDGEY".to_string(),
        prompt_for_nickname: false,
    });

    finish_current_battle_message_for_regression(&mut runtime_shell);
    press_visible_a_button(&mut runtime_shell).expect("dismiss Contest Gotcha page");
    assert_eq!(runtime_shell.battle_messages.front(), Some(&pokedex));
    assert!(runtime_shell.shell.snapshot().unwrap().battle.is_some());

    finish_current_battle_message_for_regression(&mut runtime_shell);
    press_visible_a_button(&mut runtime_shell).expect("open Contest Pokedex entry");
    assert!(runtime_shell.pokedex_scripted_entry);
    assert!(runtime_shell.shell.snapshot().unwrap().battle.is_some());

    for _ in 0..8 {
        if !runtime_shell.pokedex_menu_open {
            break;
        }
        press_visible_a_button(&mut runtime_shell).expect("advance Contest Pokedex entry");
    }
    assert!(!runtime_shell.pokedex_menu_open);
    assert!(runtime_shell.pending_standard_capture.is_none());
    assert!(runtime_shell.pending_name_choice.is_none());
    assert!(runtime_shell.pending_name_input.is_none());
    let snapshot = runtime_shell.shell.snapshot().expect("completed Contest capture");
    assert!(snapshot.battle.is_none());
    assert_eq!(
        snapshot
            .bug_contest
            .caught_mon
            .as_ref()
            .map(|pokemon| pokemon.species.id.as_str()),
        Some("PIDGEY")
    );
}

fn contest_replacement_shell_for_regression() -> (BevyRuntimeShell, crate::core::models::Pokemon) {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_message_scenes.clear();
    let previous = runtime_shell
        .runtime
        .data
        .create_pokemon("LEDYBA", 5, Dv::from_non_hp(8, 8, 8, 8))
        .expect("materialize prior Contest catch");
    let candidate = runtime_shell
        .runtime
        .data
        .create_pokemon("PIDGEY", 4, Dv::from_non_hp(10, 10, 10, 10))
        .expect("materialize candidate Contest catch");
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        let crate::core::state::BattleMemory::StaticWild {
            battle_music,
            roaming_slot,
            ..
        } = state.battle.clone()
        else {
            panic!("Contest replacement fixture requires a static wild battle");
        };
        state.battle = crate::core::state::BattleMemory::Wild {
            battle_type: "BATTLETYPE_CONTEST".to_string(),
            battle_music,
            map_name: "Route36".to_string(),
            roaming_slot,
            enemy_pokemon: candidate.clone(),
            enemy_party: vec![candidate.clone()],
        };
        state.bug_contest.caught_mon = Some(previous);
        if let Some(combat) = state.script_runtime.active_battle_combat.as_mut() {
            combat.enemy = candidate.clone();
            combat.enemy_party = vec![candidate.clone()];
            combat.enemy_party_index = 0;
        }
    }
    (runtime_shell, candidate)
}

fn complete_contest_replacement_for_regression(runtime_shell: &mut BevyRuntimeShell) {
    complete_visible_standard_capture(
        runtime_shell,
        crate::core::battle::capture::CaptureOutcome {
            caught: true,
            blocked: false,
            storage_full: false,
            wobble_count: 3,
            animation_shakes: 3,
            final_catch_rate: u8::MAX,
            ball_id: Some("PARK_BALL".to_string()),
        },
        None,
        None,
    )
    .expect("stage Contest replacement");
    finish_current_battle_message_for_regression(runtime_shell);
    press_visible_a_button(runtime_shell).expect("dismiss already-caught text");
}

#[test]
fn contest_replacement_no_keeps_stock_mon_and_exits_after_stats_prompt() {
    let (mut runtime_shell, candidate) = contest_replacement_shell_for_regression();
    complete_contest_replacement_for_regression(&mut runtime_shell);

    assert_eq!(
        runtime_shell
            .visible_bug_contest_replacement
            .as_ref()
            .map(|replacement| replacement.phase),
        Some(VisibleBugContestReplacementPhase::StatsPrompt)
    );
    assert_eq!(runtime_shell.yes_no_cursor.as_ref().unwrap().option_index, 0);
    let snapshot = runtime_shell.shell.snapshot().unwrap();
    assert_eq!(
        snapshot.bug_contest.caught_mon.as_ref().unwrap().species.id,
        "LEDYBA"
    );
    assert_eq!(
        snapshot
            .bug_contest
            .pending_caught_mon
            .as_ref()
            .unwrap()
            .species
            .id,
        candidate.species.id
    );

    let mut app = battle_render_regression_app(runtime_shell);
    app.update();
    assert!(
        app.world().resource::<BevyRuntimeShell>().last_error.is_none(),
        "Contest stats comparison failed to render"
    );
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        press_visible_b_button(&mut runtime_shell).expect("keep prior Contest catch");
        assert!(runtime_shell.visible_bug_contest_replacement.is_none());
        let snapshot = runtime_shell.shell.snapshot().unwrap();
        assert_eq!(
            snapshot.bug_contest.caught_mon.as_ref().unwrap().species.id,
            "LEDYBA"
        );
        assert!(snapshot.bug_contest.pending_caught_mon.is_none());
        assert!(snapshot.battle.is_none());
    }
}

#[test]
fn contest_replacement_yes_commits_candidate_then_waits_for_caught_text() {
    let (mut runtime_shell, candidate) = contest_replacement_shell_for_regression();
    complete_contest_replacement_for_regression(&mut runtime_shell);
    press_visible_a_button(&mut runtime_shell).expect("switch to candidate Contest catch");

    assert_eq!(
        runtime_shell
            .visible_bug_contest_replacement
            .as_ref()
            .map(|replacement| replacement.phase),
        Some(VisibleBugContestReplacementPhase::CaughtText)
    );
    assert_eq!(runtime_shell.field_notice.as_deref(), Some("Caught PIDGEY!"));
    let snapshot = runtime_shell.shell.snapshot().unwrap();
    assert_eq!(
        snapshot.bug_contest.caught_mon.as_ref().unwrap().species.id,
        candidate.species.id
    );
    assert!(snapshot.bug_contest.pending_caught_mon.is_none());
    assert!(
        runtime_shell.visible_walk_warp_phase.is_none(),
        "caught text must precede the battle map reload"
    );

    let caught_text = runtime_shell.field_notice.clone().unwrap();
    runtime_shell.field_text_reveal = Some(VisibleFieldTextReveal {
        text: caught_text.clone(),
        page_index: 0,
        visible_chars: caught_text.chars().count(),
        frames_until_next_char: 0,
    });
    press_visible_a_button(&mut runtime_shell).expect("dismiss Contest caught text");
    assert!(runtime_shell.field_notice.is_none());
    assert!(runtime_shell.visible_bug_contest_replacement.is_none());
    assert!(runtime_shell.shell.snapshot().unwrap().battle.is_none());
}

#[test]
fn empty_terminal_battle_reward_starts_plain_map_reload_before_releasing_frame() {
    let mut runtime_shell = route36_overworld_shell_for_battle_render_regression();
    let terminal_scene = runtime_shell
        .shell
        .snapshot()
        .expect("terminal retained battle frame fixture");
    assert!(terminal_scene.battle.is_none());
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_exp_tween = None;
    runtime_shell.pending_battle_exp_tweens.clear();
    runtime_shell.battle_level_stats.clear();
    runtime_shell.battle_message_scene = Some(Box::new(terminal_scene));
    runtime_shell.pending_plain_battle_map_reload = true;

    assert!(
        finish_visible_empty_battle_reward_presentation(&mut runtime_shell)
            .expect("finish empty terminal reward presentation")
    );
    assert!(runtime_shell.battle_message_scene.is_none());
    assert!(!runtime_shell.pending_plain_battle_map_reload);
    assert_eq!(
        runtime_shell.visible_walk_warp_phase,
        Some(VisibleWalkWarpPhase::MapReloadFadeIn)
    );
    let fade = runtime_shell
        .screen_fade
        .expect("plain battle exit must arm its white reload fade");
    assert_eq!(fade.color, ScriptFadeColor::White);
    assert_eq!(fade.direction, ScriptFadeDirection::In);
    assert_eq!(fade.alpha, 255);
}

#[test]
fn ordinary_cave_transition_wave_uses_old_offset_accumulator_and_native_scanline_angles() {
    const PREFIX_FRAMES: u16 = 3;
    const FLASH_FRAMES: u16 = 75;
    const BETWEEN_FRAMES: u16 = 2;

    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + BETWEEN_FRAMES,
        stronger_enemy: false,
        cave_environment: true,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = battle_render_regression_app(runtime_shell);

    for outro in [0_u16, 1, 2, 5, 14] {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + BETWEEN_FRAMES + outro,
                stronger_enemy: false,
                cave_environment: true,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();

        let mut counter = 0_u8;
        let mut offset = 0_u8;
        let mut amplitude = 0_u8;
        for _ in 0..=outro {
            amplitude = counter;
            let old_offset = offset;
            counter = counter.wrapping_add(old_offset);
            offset = offset.wrapping_add(1);
        }

        let world = app.world_mut();
        let mut strips = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let base_strips = strips
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.rect.is_some() && (transform.translation.z - 2.65).abs() < f32::EPSILON
            })
            .collect::<Vec<_>>();
        let mut source_rows = vec![Vec::<f32>::new(); 144];
        for (sprite, transform) in base_strips {
            assert_eq!(
                sprite.custom_size,
                Some(Vec2::new(PLAYFIELD_WIDTH, TILE_SIZE / 8.0)),
                "each native scanline must display as one 640x4 strip"
            );
            let rect = sprite.rect.expect("wave strip source rectangle");
            let source_y = (rect.min.y / (TILE_SIZE / 8.0)) as u8;
            assert_eq!(
                rect.max.y - rect.min.y,
                TILE_SIZE / 8.0,
                "wave strips must sample exactly one native scanline"
            );
            source_rows[usize::from(source_y)].push(transform.translation.x);
            let expected_y =
                PLAYFIELD_HEIGHT * 0.5 - (f32::from(source_y) + 0.5) * (TILE_SIZE / 8.0);
            assert_eq!(
                transform.translation.y, expected_y,
                "wave row {source_y} must stay centered on its native scanline"
            );
        }
        for (source_y, actual_positions) in source_rows.iter_mut().enumerate() {
            let expected_shift =
                visible_battle_anim_sine((source_y as u8).wrapping_mul(2), amplitude) as f32
                    * (TILE_SIZE / 8.0);
            let mut expected_positions = vec![expected_shift];
            if expected_shift > 0.0 {
                expected_positions.push(expected_shift - PLAYFIELD_WIDTH);
            } else if expected_shift < 0.0 {
                expected_positions.push(expected_shift + PLAYFIELD_WIDTH);
            }
            actual_positions.sort_by(f32::total_cmp);
            expected_positions.sort_by(f32::total_cmp);
            assert_eq!(
                *actual_positions, expected_positions,
                "outro {outro} row {source_y} must wrap its shifted SCX scanline"
            );
        }

        let mut priority_strips =
            world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let priority_count = priority_strips
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.rect.is_some() && (transform.translation.z - 2.66).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(
            priority_count,
            source_rows.iter().map(Vec::len).sum::<usize>(),
            "priority scanlines must use the same wrap copies as the base layer"
        );
    }
}

#[test]
fn battle_transition_outros_render_their_first_and_final_source_mutations() {
    const PREFIX_FRAMES: u16 = 3;
    const FLASH_FRAMES: u16 = 75;

    // Strong cave zoom: the nine source WaitBGMap calls are the nine
    // visible boxes, from 4x2 through the complete 20x18 LCD.
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + 1,
        stronger_enemy: true,
        cave_environment: true,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut zoom_app = battle_render_regression_app(runtime_shell);
    zoom_app.update();
    for (outro, expected_size) in [
        (0_u16, Vec2::new(TILE_SIZE * 4.0, TILE_SIZE * 2.0)),
        (8_u16, Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
    ] {
        if outro != 0 {
            let mut runtime_shell = zoom_app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + 1 + outro,
                stronger_enemy: true,
                cave_environment: true,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            drop(runtime_shell);
            zoom_app.update();
        }
        let world = zoom_app.world_mut();
        let mut black = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let surfaces = black
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.color == Color::BLACK && (transform.translation.z - 2.7).abs() < f32::EPSILON
            })
            .filter_map(|(sprite, transform)| {
                sprite.custom_size.map(|size| (size, transform.translation))
            })
            .collect::<Vec<_>>();
        assert_eq!(surfaces.len(), 1, "zoom outro {outro}");
        assert_eq!(surfaces[0].0, expected_size, "zoom outro {outro}");
        assert_eq!(
            surfaces[0].1,
            Vec3::new(0.0, 0.0, 2.7),
            "zoom boxes must expand about the exact LCD centre"
        );
    }

    // Strong outdoor scatter writes exactly twelve fresh cells per source
    // call. The first and sixteenth calls must therefore expose 12 and 192
    // black tiles respectively before the terminal hold.
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + 2,
        stronger_enemy: true,
        cave_environment: false,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut scatter_app = battle_render_regression_app(runtime_shell);
    scatter_app.update();
    for (outro, expected_tiles) in [(0_u16, 12_usize), (15_u16, 192_usize)] {
        if outro != 0 {
            let mut runtime_shell = scatter_app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + 2 + outro,
                stronger_enemy: true,
                cave_environment: false,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            drop(runtime_shell);
            scatter_app.update();
        }
        let world = scatter_app.world_mut();
        let mut black = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let count = black
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.color == Color::BLACK
                    && sprite.custom_size == Some(Vec2::splat(TILE_SIZE))
                    && (transform.translation.z - 2.7).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(count, expected_tiles, "scatter outro {outro}");
    }

    // Ordinary outdoor spin writes one wedge every three displayed frames.
    // Its five wedge shapes repeat over four quadrants and cover all 360
    // LCD cells after the twentieth write.
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
        frame: PREFIX_FRAMES + FLASH_FRAMES + 2,
        stronger_enemy: false,
        cave_environment: false,
        trainer_battle: false,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut spin_app = battle_render_regression_app(runtime_shell);
    spin_app.update();
    for (outro, expected_tiles) in [(0_u16, 16_usize), (57_u16, 360_usize)] {
        if outro != 0 {
            let mut runtime_shell = spin_app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
                frame: PREFIX_FRAMES + FLASH_FRAMES + 2 + outro,
                stronger_enemy: false,
                cave_environment: false,
                trainer_battle: false,
            });
            mark_runtime_snapshot_dirty(&mut runtime_shell);
            drop(runtime_shell);
            spin_app.update();
        }
        let world = spin_app.world_mut();
        let mut black = world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let count = black
            .iter(world)
            .filter(|(sprite, transform)| {
                sprite.color == Color::BLACK
                    && sprite.custom_size == Some(Vec2::splat(TILE_SIZE))
                    && (transform.translation.z - 2.7).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(count, expected_tiles, "spin outro {outro}");
        if outro == 57 {
            let mut tiles =
                world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
            let centers = tiles
                .iter(world)
                .filter(|(sprite, transform)| {
                    sprite.color == Color::BLACK
                        && sprite.custom_size == Some(Vec2::splat(TILE_SIZE))
                        && (transform.translation.z - 2.7).abs() < f32::EPSILON
                })
                .map(|(_, transform)| transform.translation.truncate())
                .collect::<Vec<_>>();
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.x)
                    .fold(f32::INFINITY, f32::min),
                PLAYFIELD_LEFT + TILE_SIZE * 0.5,
            );
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.x)
                    .fold(f32::NEG_INFINITY, f32::max),
                -PLAYFIELD_LEFT - TILE_SIZE * 0.5,
            );
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.y)
                    .fold(f32::INFINITY, f32::min),
                -PLAYFIELD_TOP + TILE_SIZE * 0.5,
            );
            assert_eq!(
                centers
                    .iter()
                    .map(|center| center.y)
                    .fold(f32::NEG_INFINITY, f32::max),
                PLAYFIELD_TOP - TILE_SIZE * 0.5,
            );
        }
    }
}

#[test]
fn every_battle_transition_variant_finishes_on_full_black_before_battle_canvas() {
    for (cave_environment, stronger_enemy, outro_frames, finish_frames) in [
        (true, false, 15_u16, 1_u16),
        (true, true, 9, 2),
        (false, false, 61, 4),
        (false, true, 21, 1),
    ] {
        let prefix_frames = 3_u16;
        let between_frames = if cave_environment && stronger_enemy {
            1
        } else {
            2
        };
        let total_frames = prefix_frames + 75 + between_frames + outro_frames + finish_frames;
        let mut runtime_shell = route36_battle_shell_for_render_regression();
        runtime_shell.visible_battle_transition = Some(VisibleBattleTransition {
            frame: total_frames - 1,
            stronger_enemy,
            cave_environment,
            trainer_battle: false,
        });
        let mut app = battle_render_regression_app(runtime_shell);
        app.update();

        let world = app.world_mut();
        let mut transition_surfaces =
            world.query_filtered::<(&Sprite, &Transform), With<BattleCommandMarker>>();
        let surfaces = transition_surfaces
            .iter(world)
            .map(|(sprite, transform)| (sprite.custom_size, sprite.color, transform.translation.z))
            .collect::<Vec<_>>();
        let terminal_black = surfaces
            .iter()
            .filter(|(size, color, z)| {
                *size == Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
                    && *color == Color::BLACK
                    && (*z - 2.76).abs() < f32::EPSILON
            })
            .count();
        assert_eq!(
            terminal_black, 1,
            "variant cave={cave_environment} stronger={stronger_enemy} must hold one opaque full-LCD black finish frame"
        );
        assert_eq!(
            surfaces
                .iter()
                .map(|(_, _, z)| *z)
                .fold(f32::NEG_INFINITY, f32::max),
            2.76,
            "terminal black must be the top transition surface"
        );
        let mut fixed_canvas = world.query_filtered::<Entity, With<FixedBattleCanvasMarker>>();
        assert_eq!(fixed_canvas.iter(world).count(), 0);
        let _ = world;

        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            advance_visible_battle_transition(&mut runtime_shell);
            assert!(runtime_shell.visible_battle_transition.is_none());
        }
        app.update();
        let world = app.world_mut();
        let mut fixed_canvas =
            world.query_filtered::<(&Sprite, &Transform), With<FixedBattleCanvasMarker>>();
        let canvases = fixed_canvas.iter(world).collect::<Vec<_>>();
        assert_eq!(
            canvases.len(),
            1,
            "variant cave={cave_environment} stronger={stronger_enemy} must hand off directly to one fixed battle canvas"
        );
        assert_eq!(
            canvases[0].0.custom_size,
            Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
        );
        assert_eq!(canvases[0].0.color, Color::WHITE);
        assert_eq!(canvases[0].1.translation, Vec3::new(0.0, 0.0, 2.7));
    }
}

#[test]
fn battle_redraw_retains_fixed_canvas_without_image_growth_even_if_overlay_rebuild_fails() {
    let mut runtime_shell = route36_battle_shell_for_render_regression();
    runtime_shell.visible_battle_transition = None;
    runtime_shell.battle_entry_messages_remaining = 0;
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_messages.clear();
    runtime_shell.battle_text_reveal = None;
    sync_visible_battle_action_cursor(&mut runtime_shell);

    let mut app = battle_render_regression_app(runtime_shell);
    app.update();

    let first_canvas = {
        let world = app.world_mut();
        let mut canvases = world.query_filtered::<Entity, With<FixedBattleCanvasMarker>>();
        let canvases = canvases.iter(world).collect::<Vec<_>>();
        assert_eq!(
            canvases.len(),
            1,
            "battle must stage exactly one opaque canvas"
        );
        canvases[0]
    };
    let stable_image_count = app.world().resource::<Assets<Image>>().len();

    // Exercise both eight-frame party-icon phases and the sixteen-frame
    // cursor phase. These redraw the transient battle layers but must not
    // allocate replacement textures or replace the continuity canvas.
    for _ in 0..4 {
        {
            let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
            runtime_shell.lcd_animation_frame = runtime_shell.lcd_animation_frame.wrapping_add(8);
            mark_runtime_snapshot_dirty(&mut runtime_shell);
        }
        app.update();
        let world = app.world_mut();
        let mut canvases = world.query_filtered::<Entity, With<FixedBattleCanvasMarker>>();
        assert_eq!(canvases.iter(world).collect::<Vec<_>>(), vec![first_canvas]);
        assert_eq!(
            world.resource::<Assets<Image>>().len(),
            stable_image_count,
            "steady battle redraws must reuse cached image assets"
        );
    }

    // Fail a stage that runs after battlers and HUD have begun rebuilding.
    // The prior full-LCD canvas must survive the early return and continue
    // hiding the retained overworld rather than exposing it for a frame.
    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.battle_action_cursor = Some(MenuCursor {
            surface_id: "invalid:battle-actions".to_string(),
            option_index: usize::MAX,
        });
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();

    let world = app.world_mut();
    assert!(
        world
            .resource::<BevyRuntimeShell>()
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("battle main-action cursor is invalid")),
        "fixture must reach the forced post-canvas render failure"
    );
    let mut canvases =
        world.query_filtered::<(Entity, &Sprite, &Transform), With<FixedBattleCanvasMarker>>();
    let canvases = canvases.iter(world).collect::<Vec<_>>();
    assert_eq!(canvases.len(), 1);
    assert_eq!(canvases[0].0, first_canvas);
    assert_eq!(
        canvases[0].1.custom_size,
        Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
    );
    assert_eq!(canvases[0].2.translation, Vec3::new(0.0, 0.0, 2.7));
    assert_eq!(
        world.resource::<Assets<Image>>().len(),
        stable_image_count,
        "the failed redraw must not leak image assets"
    );
}
