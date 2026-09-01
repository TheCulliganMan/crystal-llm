#[test]
fn compiled_pack_contains_complete_npc_trade_payloads() {
    let shell = core_modular_title_shell_for_test();
    let snapshot = shell.shell.snapshot().expect("snapshot compiled pack");
    let mike = snapshot
        .special
        .npc_trades
        .get("NPC_TRADE_MIKE")
        .expect("Mike NPC trade");
    assert_eq!(mike.requested_species, "ABRA");
    assert_eq!(mike.offered_species, "MACHOP");
    assert_eq!(mike.nickname, "MUSCLE");
    assert_eq!(mike.dvs, vec![0x37, 0x66]);
    assert_eq!(mike.held_item, "GOLD_BERRY");
    assert_eq!(mike.original_trainer_id, 37460);
    assert_eq!(mike.original_trainer_name, "MIKE");
    let tower = shell
        .shell
        .runtime()
        .data()
        .battle_tower_rules
        .as_ref()
        .expect("compiled Battle Tower roster");
    assert_eq!(tower.trainers.len(), 70);
    assert_eq!(tower.mon_groups.len(), 10);
    assert!(tower.mon_groups.iter().all(|group| group.len() == 21));
    assert_eq!(tower.mon_groups[0][0].species, "JOLTEON");
}

#[test]
fn real_pack_battle_tower_loads_canonical_roster_and_party() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut shell =
        RuntimeGameShell::new_game_at_runtime_tile(asset_root, runtime, 1, "BattleTower1F", 8, 5)
            .expect("start Battle Tower map shell");
    for species in ["CYNDAQUIL", "TOTODILE", "CHIKORITA"] {
        shell
            .add_party_pokemon(
                species,
                10,
                None,
                None,
                "PLAYER",
                1,
                Dv::from_non_hp(10, 10, 10, 10),
            )
            .expect("add Battle Tower party member");
    }
    shell.session_mut().state_mut().battle_tower.level_group = 1;
    shell
        .apply_battle_tower_action("BATTLETOWERACTION_SAVELEVELGROUP".to_string())
        .expect("select level group");
    shell
        .load_battle_tower_opponent_special(
            "OBJECT_EVENT_1".to_string(),
        )
        .expect("load canonical Battle Tower opponent");
    let battle_command_index = shell
        .runtime()
        .compiled_script_commands("Script_BattleRoomLoop")
        .expect("compiled Battle Tower room loop")
        .iter()
        .position(|command| {
            command.get("command").and_then(serde_json::Value::as_str) == Some("special")
                && command
                    .get("args")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|args| args.first())
                    .and_then(serde_json::Value::as_str)
                    == Some("BattleTowerBattle")
        })
        .expect("BattleTowerBattle command");
    shell
        .apply_compiled_script_command(
            "BattleTowerBattleRoom",
            "Script_BattleRoomLoop",
            battle_command_index,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("start canonical Battle Tower battle");
    let snapshot = shell.snapshot().expect("snapshot Battle Tower battle");
    let battle = snapshot.battle.expect("active Battle Tower battle");
    assert_eq!(battle.battle_type, "BATTLETYPE_BATTLE_TOWER");
    assert_eq!(battle.enemy_party.len(), 3);
    assert!(battle.enemy_party.iter().all(|pokemon| pokemon.level == 10));
    let RuntimeBattleKind::Trainer { trainer_id, .. } = battle.kind else {
        panic!("Battle Tower must use trainer battle kind");
    };
    assert_ne!(trainer_id, "caller_supplied_id");
    assert!(trainer_id.starts_with("BATTLE_TOWER_"));
}

#[test]
fn npc_trade_runtime_exchanges_matching_party_pokemon() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
        asset_root,
        runtime,
        1,
        "GoldenrodDeptStore5F",
        5,
        5,
    )
    .expect("start trade map shell");
    shell
        .add_party_pokemon(
            "ABRA",
            18,
            None,
            None,
            "PLAYER",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add requested Abra");
    shell
        .add_party_pokemon(
            "CHIKORITA",
            12,
            None,
            None,
            "PLAYER",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add party member after requested Abra");
    let command = shell
        .script_runtime_command_keys()
        .into_iter()
        .find(|key| key.command == "trade" && key.args == vec!["NPC_TRADE_MIKE"])
        .expect("Mike trade command");
    shell
        .apply_compiled_script_command(
            &command.map_name,
            &command.source_script,
            command.command_index,
            ScriptRuntimeInputs {
                selected_party_index: Some(0),
                ..ScriptRuntimeInputs::default()
            },
            ScriptPhoneInputs::default(),
        )
        .expect("execute Mike trade");
    let snapshot = shell.snapshot().expect("snapshot exchanged party");
    let traded = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.pokemon.species.id == "MACHOP")
        .map(|slot| &slot.pokemon)
        .expect("traded party slot");
    assert_eq!(traded.species.id, "MACHOP");
    assert_eq!(traded.nickname, "MUSCLE");
    assert_eq!(traded.item.as_deref(), Some("GOLD_BERRY"));
    assert_eq!(traded.original_trainer_name, "MIKE");
    assert_eq!(traded.original_trainer_id, 37460);
    assert_eq!(snapshot.party.slots.len(), 2);
    assert_eq!(snapshot.party.slots[0].pokemon.species.id, "CHIKORITA");
    assert_eq!(snapshot.party.slots[1].pokemon.species.id, "MACHOP");
    shell
        .apply_compiled_script_command(
            &command.map_name,
            &command.source_script,
            command.command_index,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("completed trade should use post-trade path");
    let second = shell.snapshot().expect("snapshot repeated trade");
    assert_eq!(second.party.slots.len(), 2);
    assert_eq!(second.party.slots[0].pokemon.species.id, "CHIKORITA");
    assert_eq!(second.party.slots[1].pokemon.species.id, "MACHOP");
}

#[test]
fn time_set_directional_input_matches_typescript_phase_rules() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    open_visible_time_set_screen(&mut runtime_shell, VisibleTimeSetNext::OakIntro)
        .expect("open time set");
    {
        let time_set = runtime_shell
            .pending_time_set
            .as_mut()
            .expect("time set pending");
        time_set.phase = VisibleTimeSetPhase::SetHour;
        time_set.hour = 10;
    }

    move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Right)
        .expect("right ignored on hour");
    assert_eq!(
        runtime_shell
            .pending_time_set
            .as_ref()
            .expect("time set pending")
            .hour,
        10,
        "TypeScript only changes the hour on up/down"
    );
    move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Up)
        .expect("up increments hour");
    assert_eq!(
        runtime_shell
            .pending_time_set
            .as_ref()
            .expect("time set pending")
            .hour,
        11
    );
    move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Left)
        .expect("left ignored on hour");
    assert_eq!(
        runtime_shell
            .pending_time_set
            .as_ref()
            .expect("time set pending")
            .hour,
        11
    );

    {
        let time_set = runtime_shell
            .pending_time_set
            .as_mut()
            .expect("time set pending");
        time_set.phase = VisibleTimeSetPhase::SetMinute;
        time_set.minute = 0;
    }
    move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Right)
        .expect("right increments minute");
    assert_eq!(
        runtime_shell
            .pending_time_set
            .as_ref()
            .expect("time set pending")
            .minute,
        1,
        "TypeScript changes the minute on right"
    );
    move_visible_time_set_direction(&mut runtime_shell, VisibleTimeSetDirection::Left)
        .expect("left decrements minute");
    assert_eq!(
        runtime_shell
            .pending_time_set
            .as_ref()
            .expect("time set pending")
            .minute,
        0,
        "TypeScript changes the minute on left"
    );
}

#[test]
fn time_set_renders_real_boot_window_font_and_arrow_assets() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    open_visible_time_set_screen(&mut runtime_shell, VisibleTimeSetNext::OakIntro)
        .expect("open time set");
    {
        let time_set = runtime_shell
            .pending_time_set
            .as_mut()
            .expect("time set pending");
        time_set.phase = VisibleTimeSetPhase::SetHour;
        time_set.visible_chars = 0;
    }
    let mut images = Assets::<Image>::default();
    let frame = load_time_set_frame(
        &runtime_shell.asset_root,
        runtime_shell
            .pending_time_set
            .as_ref()
            .expect("time set pending"),
        &mut images,
    )
    .expect("render time-set frame from boot assets");
    let image = images.get(&frame.handle).expect("time-set image");

    assert_eq!(image.texture_descriptor.size.width, 160);
    assert_eq!(image.texture_descriptor.size.height, 144);
    assert_eq!(
        &image.data[0..4],
        &BOOT_UI_WHITE,
        "time-set field must use canonical boot UI white"
    );
    let textbox_interior = ((TIME_SET_TEXTBOX_Y + 1) * SOURCE_TILE_SIZE * 160
        + 18 * SOURCE_TILE_SIZE)
        * 4;
    assert_eq!(
        &image.data[textbox_interior..textbox_interior + 4],
        &BOOT_UI_WHITE,
        "time-set field and textbox interior must be the exact same white"
    );
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] < 16 && pixel[1] < 16 && pixel[2] < 16 && pixel[3] == 255),
        "time-set screen must contain black font/frame pixels from real assets"
    );
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] == 255 && pixel[1] == 255 && pixel[2] == 255),
        "time-set screen must contain boot window fill from the real textbox layout"
    );
}

#[test]
fn oak_intro_renders_real_oak_wooper_and_player_portraits() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
    assert!(
        runtime_shell
            .pending_audio
            .iter()
            .any(|command| command.audio_id == "MUSIC_ROUTE_30"
                && matches!(command.kind, ModpackAudioKind::Music)),
        "Oak intro must queue TypeScript/ASM Route 30 music, not Route 29 or overworld music"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("queued Oak intro music MUSIC_ROUTE_30")),
        "{:?}",
        runtime_shell.last_audio_events
    );
    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);

    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let rendered_art = app.world().resource::<RenderedTilesetArt>();
        assert!(
            rendered_art.intro_cache.contains_key(&IntroArtKey {
                asset_id: "trainer:oak".to_string(),
            }),
            "Oak intro must render Professor Oak from gfx/trainers/oak.png, not a label"
        );
        let mut query = app
            .world_mut()
            .query_filtered::<&Sprite, With<TitleScreenMarker>>();
        let sizes = query
            .iter(app.world())
            .filter_map(|sprite| sprite.custom_size)
            .collect::<Vec<_>>();
        assert_eq!(
            sizes,
            vec![Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)],
            "Oak intro should spawn one composed native 160x144 surface scaled to the shared playfield"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_mut()
            .expect("Oak intro pending");
        oak_intro.scene_phase = VisibleOakIntroPhase::Text;
        oak_intro.fade_active = false;
        oak_intro.fade_alpha = 0;
        oak_intro.current_text = "Hello! Sorry to\nkeep you waiting!".to_string();
        oak_intro.visible_chars = oak_intro.current_text.chars().count();
        oak_intro.waiting_for_input = true;
        oak_intro.blink_timer = 0;
    }
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Oak intro pending");
        let trainer = runtime_shell
            .shell
            .snapshot()
            .expect("Oak intro snapshot")
            .trainer;
        let key = oak_intro_art_key(oak_intro, trainer.player_gender);
        let rendered_art = app.world().resource::<RenderedTilesetArt>();
        let oak_frame = rendered_art
            .oak_intro_cache
            .get(&key)
            .expect("Oak intro frame cache");
        assert_eq!(oak_frame.size, Vec2::new(160.0, 144.0));
        let handle = oak_frame.handle.clone();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .expect("Oak intro composed image");
        let has_textbox_text = (OAK_INTRO_TEXTBOX_Y * SOURCE_TILE_SIZE..144).any(|y| {
            (0..160).any(|x| {
                let offset = (y * 160 + x) * 4;
                image.data[offset] < 24
                    && image.data[offset + 1] < 24
                    && image.data[offset + 2] < 24
                    && image.data[offset + 3] == 255
            })
        });
        assert!(
            has_textbox_text,
            "Oak intro dialog must be drawn into the composed native frame, not loose Bevy glyph sprites"
        );
        let visible_arrow_pixels = oak_intro_prompt_arrow_dark_pixels(image);
        assert!(
            visible_arrow_pixels > 0,
            "Oak intro prompt arrow must render during the visible half of the TypeScript blink cycle"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_mut()
            .expect("Oak intro pending");
        oak_intro.blink_timer = 30;
    }
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Oak intro pending");
        let trainer = runtime_shell
            .shell
            .snapshot()
            .expect("Oak intro snapshot")
            .trainer;
        let key = oak_intro_art_key(oak_intro, trainer.player_gender);
        let rendered_art = app.world().resource::<RenderedTilesetArt>();
        let oak_frame = rendered_art
            .oak_intro_cache
            .get(&key)
            .expect("Oak intro frame cache after blink toggle");
        let handle = oak_frame.handle.clone();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .expect("Oak intro blink-hidden image");
        assert_eq!(
            oak_intro_prompt_arrow_dark_pixels(image),
            0,
            "Oak intro prompt arrow must be hidden for blinkTimer >= 30 like TypeScript"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_mut()
            .expect("Oak intro pending");
        oak_intro.scene_index = 1;
        start_visible_oak_intro_scene(oak_intro);
    }
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let rendered_art = app.world().resource::<RenderedTilesetArt>();
        assert!(
            rendered_art.pokemon_cache.contains_key(&PokemonArtKey {
                species_id: "wooper".to_string(),
                side: PokemonSpriteSide::Front,
                shiny: false,
                frame: 0,
            }),
            "Oak intro must render Wooper from the real Pokemon frontpic"
        );
    }

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell
            .shell
            .set_player_gender(PLAYER_GENDER_FEMALE)
            .expect("set Kris gender");
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_mut()
            .expect("Oak intro pending");
        oak_intro.scene_index = 3;
        start_visible_oak_intro_scene(oak_intro);
    }
    app.update();
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        let rendered_art = app.world().resource::<RenderedTilesetArt>();
        assert!(
            rendered_art.intro_cache.contains_key(&IntroArtKey {
                asset_id: "player:kris".to_string(),
            }),
            "Oak intro must render the selected female player portrait"
        );
    }
}

#[test]
fn new_game_name_choice_does_not_render_the_uninitialized_bedroom() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_name_choice(&mut runtime_shell).expect("open player name choices");

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(
        runtime_shell.last_error, None,
        "the full-screen player-name menu must not try to render bedroom decorations"
    );
    let world = app.world_mut();
    let mut objects = world.query_filtered::<Entity, With<ObjectMarker>>();
    assert_eq!(objects.iter(world).count(), 0);
    let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
    assert_eq!(presenters.iter(world).count(), 1);
}

#[test]
fn oak_intro_fade_phases_gate_text_and_clear_sprite_on_fade_out() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Oak intro pending");
        assert_eq!(oak_intro.scene_state, "oak_intro_1");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::FadeIn);
        assert_eq!(oak_intro.fade_alpha, 255);
        assert!(oak_intro.current_text.is_empty());
    }

    for _ in 0..(4 * VISIBLE_OAK_INTRO_FADE_FRAME_DELAY) {
        tick_visible_oak_intro(&mut runtime_shell).expect("tick Oak fade-in");
    }
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Oak intro pending after fade-in");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::Text);
        assert_eq!(oak_intro.fade_alpha, 0);
        assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[0].2[0]);
    }

    for _ in 0..VISIBLE_OAK_INTRO_SCENES[0].2.len() {
        finish_current_oak_intro_page_for_test(&mut runtime_shell);
    }
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Oak intro pending during fade-out");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::FadeOut);
        assert_eq!(oak_intro.current_sprite.as_deref(), Some("OAK"));
        assert_eq!(oak_intro.fade_alpha, 0);
    }

    for _ in 0..(3 * VISIBLE_OAK_INTRO_FADE_FRAME_DELAY) {
        tick_visible_oak_intro(&mut runtime_shell).expect("tick Oak fade-out");
    }
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Oak intro pending after fade-out");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::Complete);
        assert!(oak_intro.finished);
        assert_eq!(oak_intro.fade_alpha, 255);
        assert_eq!(oak_intro.current_sprite, None);
    }
}

#[test]
fn oak_intro_queues_wooper_cry_from_pack_metadata() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_oak_intro_sequence(&mut runtime_shell).expect("open Oak intro");
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_mut()
            .expect("Oak intro pending");
        oak_intro.scene_index = 1;
        start_visible_oak_intro_scene(oak_intro);
    }

    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Wooper intro pending");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::WipeIn);
        assert_eq!(oak_intro.wipe_window_x, 0);
        assert!(oak_intro.current_text.is_empty());
    }
    assert!(
        !runtime_shell
            .pending_audio
            .iter()
            .any(|command| command.audio_id == "CRY_WOOPER"),
        "Wooper cry must not be queued before the wipe and first text group"
    );

    for _ in 0..=VISIBLE_OAK_WIPE_END_X / VISIBLE_OAK_WIPE_STEP_PIXELS {
        tick_visible_oak_intro(&mut runtime_shell).expect("tick Wooper wipe");
    }
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Wooper intro pending after wipe");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextOne);
        assert!(!oak_intro.wipe_active);
        assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[1].2[0]);
    }

    finish_current_oak_intro_page_for_test(&mut runtime_shell);
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Wooper second page pending");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextOne);
        assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[1].2[1]);
    }
    press_visible_oak_intro_a_button(&mut runtime_shell)
        .expect("revealing the final OakText2 page must enter the cry without another prompt");
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Wooper cry phase pending");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::Cry);
        assert!(!oak_intro.wooper_cry_queued);
        assert_eq!(oak_intro.current_text, "#MON.");
        assert!(!oak_intro.waiting_for_input);
    }
    assert!(
        !runtime_shell
            .pending_audio
            .iter()
            .any(|command| command.audio_id == "CRY_WOOPER"),
        "Wooper cry must wait until the cry phase"
    );

    tick_visible_oak_intro(&mut runtime_shell).expect("tick Wooper cry phase");

    assert!(
        runtime_shell
            .pending_audio
            .iter()
            .any(|command| command.audio_id == "CRY_WOOPER"
                && matches!(command.kind, ModpackAudioKind::Cry)),
        "Wooper showcase must queue CRY_WOOPER from compiled pack metadata"
    );
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("queued oak_intro cry CRY_WOOPER species=WOOPER")),
        "{:?}",
        runtime_shell.last_audio_events
    );
    {
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("Wooper text-two pending");
        assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextTwo);
        assert!(oak_intro.wooper_cry_queued);
        assert_eq!(oak_intro.current_text, "#MON.");
        assert!(oak_intro.waiting_for_input);
    }

    press_visible_oak_intro_b_button(&mut runtime_shell)
        .expect("ASM prompt button accepts B without skipping Oak intro");
    let oak_intro = runtime_shell
        .pending_oak_intro
        .as_ref()
        .expect("Oak intro must remain open after the Wooper prompt");
    assert_eq!(oak_intro.scene_phase, VisibleOakIntroPhase::TextTwo);
    assert_eq!(oak_intro.current_text, VISIBLE_OAK_INTRO_SCENES[1].2[2]);
}

#[test]
fn empty_player_name_confirmation_uses_gender_default_player_name() {
    fn confirm_empty_name_for_gender(player_gender: u8) -> String {
        let mut runtime_shell = core_modular_title_shell_for_test();
        runtime_shell
            .shell
            .set_player_gender(player_gender)
            .expect("set player gender");
        runtime_shell.selected_player_gender = if player_gender == PLAYER_GENDER_FEMALE {
            Some(VisiblePlayerGender::Girl)
        } else {
            Some(VisiblePlayerGender::Boy)
        };
        runtime_shell.intro_screen = None;
        runtime_shell.title_menu = None;
        open_visible_player_name_input(&mut runtime_shell).expect("open player name input");
        assert_eq!(
            runtime_shell
                .pending_name_input
                .as_ref()
                .map(|input| input.label.as_str()),
            Some("YOUR NAME?")
        );
        move_visible_player_name_cursor_to_end(&mut runtime_shell).expect("move to END");
        select_visible_player_name_grid_key(&mut runtime_shell)
            .expect("empty name should use the selected-gender default-name path");
        assert!(runtime_shell.pending_name_input.is_none());
        assert!(runtime_shell.pending_oak_intro.is_some());

        runtime_shell
            .shell
            .snapshot()
            .expect("snapshot after empty name")
            .trainer
            .player_name
    }

    assert_eq!(
        confirm_empty_name_for_gender(PLAYER_GENDER_MALE),
        DEFAULT_MALE_PLAYER_NAME,
        "blank END confirmation must use the male ASM default name"
    );
    assert_eq!(
        confirm_empty_name_for_gender(PLAYER_GENDER_FEMALE),
        DEFAULT_FEMALE_PLAYER_NAME,
        "blank END confirmation must use the female ASM default name"
    );
}

#[test]
fn player_name_entry_matches_typescript_navigation_groups() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_player_name_input(&mut runtime_shell).expect("open player name input");

    for _ in 0..visible_name_input_bottom_row_index() {
        move_visible_player_name_cursor_down(&mut runtime_shell).expect("move down");
    }
    {
        let input = runtime_shell
            .pending_name_input
            .as_ref()
            .expect("name input open");
        assert_eq!(
            (input.cursor_column, input.cursor_row),
            (0, visible_name_input_bottom_row_index())
        );
    }
    select_visible_player_name_grid_key(&mut runtime_shell).expect("toggle lower");
    assert_eq!(
        runtime_shell
            .pending_name_input
            .as_ref()
            .map(|input| input.case),
        Some(NameInputCase::Lower)
    );

    move_visible_player_name_cursor_right(&mut runtime_shell).expect("move to DEL");
    {
        let input = runtime_shell
            .pending_name_input
            .as_ref()
            .expect("name input open");
        assert_eq!(
            (input.cursor_column, input.cursor_row),
            (3, visible_name_input_bottom_row_index())
        );
    }
    select_visible_player_name_grid_key(&mut runtime_shell).expect("delete empty is allowed");
    assert_eq!(
        runtime_shell
            .pending_name_input
            .as_ref()
            .map(|input| input.value.as_str()),
        Some("")
    );

    move_visible_player_name_cursor_right(&mut runtime_shell).expect("move to END group");
    {
        let input = runtime_shell
            .pending_name_input
            .as_ref()
            .expect("name input open");
        assert_eq!(
            (input.cursor_column, input.cursor_row),
            (6, visible_name_input_bottom_row_index())
        );
    }

    move_visible_player_name_cursor_up(&mut runtime_shell).expect("move to symbol row");
    move_visible_player_name_cursor_up(&mut runtime_shell).expect("move to lower letter row");
    select_visible_player_name_grid_key(&mut runtime_shell).expect("append y");
    assert_eq!(
        runtime_shell
            .pending_name_input
            .as_ref()
            .map(|input| input.value.as_str()),
        Some("y")
    );
}

#[test]
fn player_name_entry_moves_to_end_at_max_length_like_typescript() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_player_name_input(&mut runtime_shell).expect("open player name input");

    for ch in "KRYSTAL".chars() {
        append_visible_player_name_char(&mut runtime_shell, ch).expect("append name char");
    }

    let input = runtime_shell
        .pending_name_input
        .as_ref()
        .expect("name input open");
    assert_eq!(input.value, "KRYSTAL");
    assert_eq!(
        (input.cursor_column, input.cursor_row),
        (8, visible_name_input_bottom_row_index())
    );
}

#[test]
fn player_name_entry_renders_real_naming_screen_assets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let input = PendingNameInput {
        label: "YOUR NAME?".to_string(),
        value: "KRIS".to_string(),
        max_length: VISIBLE_NAME_ENTRY_MAX_LENGTH,
        cursor_column: 6,
        cursor_row: visible_name_input_bottom_row_index(),
        case: NameInputCase::Lower,
    };
    let mut images = Assets::<Image>::default();

    let frame =
        load_name_entry_frame(&asset_root, &input, &mut images).expect("render naming screen");
    let image = images.get(&frame.handle).expect("naming screen image");

    assert_eq!(image.texture_descriptor.size.width, 160);
    assert_eq!(image.texture_descriptor.size.height, 144);
    assert_eq!(frame.size, Vec2::new(160.0, 144.0));
    assert_eq!(name_entry_screen_center(), Vec3::ZERO);
    assert_eq!(&image.data[0..4], &[255, 255, 255, 255]);
    assert_eq!(&image.data[(161 * 4)..(162 * 4)], &[132, 115, 156, 255]);
    assert_opaque_nonblack_lcd_pixels(&image.data, "name entry");
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] < 248 || pixel[1] < 248 || pixel[2] < 248),
        "naming screen should contain real font, border, underline, and cursor pixels"
    );
    assert!(
        image.data.chunks_exact(4).all(|pixel| pixel[3] == 255),
        "naming screen must be fully opaque so the overworld cannot bleed through UI"
    );
}

#[test]
fn pokemon_name_entry_uses_the_two_line_asm_heading_without_clipping() {
    let input = PendingNameInput {
        label: visible_pokemon_nickname_label("CYNDAQUIL"),
        value: "AA".to_string(),
        max_length: 10,
        cursor_column: 0,
        cursor_row: 0,
        case: NameInputCase::Upper,
    };

    let tilemap = build_name_entry_tilemap(&input).expect("build Pokemon nickname tilemap");
    for (row, expected) in [(2, "CYNDAQUIL'S"), (4, "NICKNAME?")] {
        for (offset, token) in tokenize_name_entry_string(expected).iter().enumerate() {
            assert_eq!(
                tilemap[row][NAME_ENTRY_NAME_X + offset],
                name_entry_token_tile(token).expect("heading glyph"),
                "wrong heading tile for {token:?} at row {row}"
            );
        }
        assert_eq!(
            tilemap[row][NAME_ENTRY_NAME_X + tokenize_name_entry_string(expected).len()],
            0,
            "heading must terminate inside the cleared name box"
        );
    }
}

#[test]
fn name_entry_replaces_the_visible_overworld_with_one_complete_lcd() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "PlayersHouse1F".to_string(),
            tile_x: 9,
            tile_y: 0,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize rendered overworld fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    let _overworld = retained_map_surface_pair(app.world_mut());

    {
        let mut runtime_shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        runtime_shell.pending_name_input = Some(PendingNameInput {
            label: "YOUR NAME?".to_string(),
            value: String::new(),
            max_length: 7,
            cursor_column: 0,
            cursor_row: 0,
            case: NameInputCase::Upper,
        });
        mark_runtime_snapshot_dirty(&mut runtime_shell);
    }
    app.update();

    assert_main_camera_presents_one_logical_lcd(app.world_mut());
    let presenter = retained_fullscreen_surface(app.world_mut());
    assert_retained_fullscreen_surface_nonblack(app.world(), &presenter, "name-entry");
    let presenter_z = app
        .world()
        .get::<Transform>(presenter.entity)
        .expect("name-entry presenter transform")
        .translation
        .z;
    let highest_world_z = {
        let world = app.world_mut();
        let mut world_art = world.query_filtered::<
            &Transform,
            Or<(With<PlayfieldTile>, With<PlayerMarker>, With<ObjectMarker>)>,
        >();
        world_art
            .iter(world)
            .map(|transform| transform.translation.z)
            .fold(f32::NEG_INFINITY, f32::max)
    };
    assert!(
        presenter_z > highest_world_z,
        "the TypeScript naming screen clears before drawing; Bevy must put its complete opaque LCD above every retained overworld surface ({presenter_z} <= {highest_world_z})"
    );

    let world = app.world_mut();
    let mut possible_occluders =
        world.query_filtered::<(&Sprite, &Transform, &Visibility), Without<VisibleIntroSurface>>();
    for (sprite, transform, visibility) in possible_occluders.iter(world) {
        if *visibility == Visibility::Hidden
            || transform.translation.z <= presenter_z
            || sprite.custom_size != Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT))
        {
            continue;
        }
        assert_eq!(
            sprite.color.to_srgba().alpha,
            0.0,
            "a visible full-window sprite above name entry would make Bevy present a blank/black screen"
        );
    }
}

#[test]
fn live_runtime_hotkeys_start_new_game_from_title_and_accept_name() {
    let runtime_shell = core_modular_title_shell_for_test();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(runtime_shell)
        .insert_resource(native_rtc_source_for_test())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(RuntimeTickTimer::new(0.0))
        .insert_resource(VisibleSequenceTickClock::deterministic_test())
        .add_systems(Update, apply_keyboard_input)
        .add_systems(
            Update,
            tick_visible_title_screen.after(apply_keyboard_input),
        )
        .add_systems(
            Update,
            apply_runtime_hotkeys.after(tick_visible_title_screen),
        );

    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(runtime_shell.title_menu.is_none());
        assert!(
            runtime_shell.pending_gender_selection.is_some(),
            "A on New Game should open gender before Oak intro and name input"
        );
        assert!(runtime_shell.pending_time_set.is_none());
        assert!(runtime_shell.pending_oak_intro.is_none());
        assert!(runtime_shell.pending_name_input.is_none());
        assert!(
            !runtime_shell.shell.session().state().game_timer_counting,
            "ResetWRAM must leave GAME_TIMER_COUNTING_F clear until FinishContinueFunction"
        );
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_frames,
            0,
            "gender/Oak setup must not count as play time"
        );
        assert_eq!(runtime_shell.last_error, None);
    }
    confirm_gender_for_test(&mut app, VisiblePlayerGender::Boy);
    complete_time_set_for_test(&mut app);
    complete_oak_intro_for_test(&mut app);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(!runtime_shell.shell.session().state().game_timer_counting);
        assert_eq!(
            runtime_shell.shell.session().state().time.game_time_frames,
            0,
            "all pre-overworld VBlanks must leave ResetGameTime untouched"
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell
                .pending_name_input
                .as_ref()
                .map(|input| input.value.as_str()),
            Some("AB")
        );
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            runtime_shell.pending_oak_intro.is_some(),
            "confirmed name should open Oak's final encouragement before gameplay"
        );
    }
    complete_oak_intro_for_test(&mut app);
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert!(runtime_shell.pending_name_input.is_none());
    assert!(runtime_shell.pending_oak_intro.is_none());
    assert_eq!(runtime_shell.last_error, None);
    let snapshot = runtime_shell.shell.snapshot().expect("snapshot after name");
    assert_eq!(snapshot.trainer.player_name, "AB");
    assert_eq!(snapshot.overworld.map_name, "PlayersHouse2F");
    assert!(runtime_shell.shell.session().state().game_timer_counting);
    assert!(!runtime_shell.shell.session().state().game_logic_paused);
    assert_eq!(
        runtime_shell.shell.session().state().time.game_time_frames,
        1,
        "the first counted frame belongs to FinishContinueFunction's overworld loop"
    );
}

#[test]
fn title_new_game_opens_gender_then_oak_clock_intro_before_name_input() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let gender = runtime_shell
            .pending_gender_selection
            .as_ref()
            .expect("New Game should open gender selection before Oak intro");
        assert_eq!(
            format_gender_dialog(runtime_shell),
            "ARE YOU A BOY?\nOR ARE YOU A GIRL?\n> BOY\n  GIRL"
        );
        assert_eq!(visible_gender_entries(gender), vec!["> BOY", "  GIRL"]);
        assert!(runtime_shell.pending_time_set.is_none());
        assert!(runtime_shell.pending_oak_intro.is_none());
        assert!(runtime_shell.pending_name_input.is_none());
        assert_eq!(runtime_shell.last_error, None);
    }

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let gender = runtime_shell
            .pending_gender_selection
            .as_ref()
            .expect("gender selection should remain active after cursor move");
        assert_eq!(visible_gender_entries(gender), vec!["  BOY", "> GIRL"]);
    }

    let confirm_delay_frames = app
        .world()
        .resource::<BevyRuntimeShell>()
        .pending_gender_selection
        .as_ref()
        .expect("gender selection before confirm")
        .definition
        .confirm_delay_frames;
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    for _ in 0..=usize::from(confirm_delay_frames) + 1 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .pending_gender_selection
            .is_none()
        {
            break;
        }
    }
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            runtime_shell.selected_player_gender,
            Some(VisiblePlayerGender::Girl)
        );
        let time_set = runtime_shell
            .pending_time_set
            .as_ref()
            .expect("confirmed gender should open Oak intro time set");
        assert_eq!(time_set.phase, VisibleTimeSetPhase::WakeDialogue);
        assert_eq!(time_set.next, VisibleTimeSetNext::OakIntro);
        assert_eq!(time_set.wake_index, 0);
        assert_eq!(time_set.hour, 10);
        assert_eq!(time_set.minute, 0);
        assert!(runtime_shell.pending_oak_intro.is_none());
        assert!(runtime_shell.pending_name_input.is_none());
        assert_eq!(runtime_shell.last_error, None);
    }

    complete_time_set_for_test(&mut app);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(runtime_shell.pending_time_set.is_none());
        let oak_intro = runtime_shell
            .pending_oak_intro
            .as_ref()
            .expect("completed clock should open Oak intro");
        assert_eq!(oak_intro.mode, VisibleOakIntroMode::Intro);
        assert_eq!(oak_intro.scene_state, "oak_intro_1");
        assert_eq!(oak_intro.current_sprite.as_deref(), Some("OAK"));
        assert!(
            runtime_shell
                .last_audio_events
                .iter()
                .any(|event| event.contains("time set 10:00 tod=Day game=10:0")),
            "default clock confirmation should commit the selected manual time"
        );
        assert!(runtime_shell.pending_name_input.is_none());
        assert_eq!(runtime_shell.last_error, None);
    }

    complete_oak_intro_for_test(&mut app);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert!(runtime_shell.pending_oak_intro.is_none());
        assert!(runtime_shell.pending_name_input.is_some());
    }
}

#[test]
fn gender_vertical_menu_ignores_left_right_and_start() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_gender_selection(&mut runtime_shell).expect("open gender screen");

    move_visible_primary_cursor_left(&mut runtime_shell).expect("press Left");
    assert_eq!(
        runtime_shell
            .pending_gender_selection
            .as_ref()
            .expect("gender selection after Left")
            .selected_index,
        0
    );
    move_visible_primary_cursor_right(&mut runtime_shell).expect("press Right");
    assert_eq!(
        runtime_shell
            .pending_gender_selection
            .as_ref()
            .expect("gender selection after Right")
            .selected_index,
        0
    );
    press_visible_start_button(&mut runtime_shell).expect("press Start");

    let gender = runtime_shell
        .pending_gender_selection
        .as_ref()
        .expect("gender selection must remain open");
    assert_eq!(gender.selected_index, 0);
    assert!(!gender.confirmed);
}

#[test]
fn gender_background_decodes_the_asm_tile_with_its_own_palette() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let assets = AssetRoot::new(repo_root).runtime_assets();

    let background = load_gender_selection_background(
        &assets.join("gfx/new_game/gender_screen.pal"),
        &assets.join("gfx/new_game/gender_screen.2bpp"),
    )
    .expect("decode gender-screen tile and palette");

    assert!(
        background
            .iter()
            .all(|pixel| pixel == &[0x4a, 0xf7, 0xff, 0xff]),
        "the ASM's gender_screen.2bpp tile uses palette color 1 for every pixel"
    );
}

#[test]
fn gender_selection_renders_real_boot_window_palette_and_options() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize title shell");

    let mut app = integrated_shell_test_app(runtime_shell);
    open_title_main_menu_for_test(&mut app);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    complete_time_set_for_test(&mut app);
    app.update();

    let world = app.world_mut();
    let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
    assert!(
        title_entities.iter(world).count() >= 1,
        "gender selection should render a native boot UI frame"
    );
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert!(
        !rendered_art.gender_cache.is_empty(),
        "gender selection render must use the real boot window/font/palette compositor"
    );
    assert!(rendered_art.gender_errors.is_empty());

    let mut images = Assets::<Image>::default();
    let mut gender = world
        .resource::<BevyRuntimeShell>()
        .pending_gender_selection
        .as_ref()
        .expect("gender selection pending")
        .clone();
    gender.fade_counter = VISIBLE_GENDER_FADE_IN_FRAMES;
    let frame = load_gender_selection_frame(
        &world.resource::<BevyRuntimeShell>().asset_root,
        &gender,
        &mut images,
    )
    .expect("render gender selection frame directly");
    let image = images.get(&frame.handle).expect("gender image");
    assert_eq!(image.texture_descriptor.size.width, 160);
    assert_eq!(image.texture_descriptor.size.height, 144);
    assert_opaque_nonblack_lcd_pixels(&image.data, "gender selection");
    assert_eq!(
        &image.data[0..4],
        &BOOT_UI_WHITE,
        "gender screen field must use the same white as its menu and textbox interiors"
    );
    let menu_interior = ((gender.definition.top + 1) * SOURCE_TILE_SIZE * 160
        + (gender.definition.right - 1) * SOURCE_TILE_SIZE)
        * 4;
    let textbox_interior = ((TIME_SET_TEXTBOX_Y + 1) * SOURCE_TILE_SIZE * 160
        + 2 * SOURCE_TILE_SIZE)
        * 4;
    assert_eq!(&image.data[menu_interior..menu_interior + 4], &BOOT_UI_WHITE);
    assert_eq!(
        &image.data[textbox_interior..textbox_interior + 4],
        &BOOT_UI_WHITE
    );
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] < 16 && pixel[1] < 16 && pixel[2] < 16 && pixel[3] == 255),
        "gender screen must contain black font/frame pixels from real assets"
    );
}

#[test]
fn gender_selection_is_attached_to_the_visible_window_scaled_presenter() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    open_visible_gender_selection(&mut runtime_shell).expect("open gender screen");
    let mut app = integrated_shell_test_app(runtime_shell);

    app.update();

    assert_main_camera_presents_one_logical_lcd(app.world_mut());
    let presenter = retained_fullscreen_surface(app.world_mut());
    assert_retained_fullscreen_surface_nonblack(app.world(), &presenter, "gender-selection");
}
