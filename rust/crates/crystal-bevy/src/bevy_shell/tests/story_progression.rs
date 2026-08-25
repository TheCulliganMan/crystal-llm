use crate::RuntimeCompiledScriptBoundary;

#[test]
fn pitfall_skyfall_enters_the_fall_at_the_first_asm_sine_offset() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize pitfall shell");

    begin_visible_pitfall_landing(&mut runtime_shell).expect("begin FallIntoMapScript");
    assert_eq!(
        runtime_shell
            .visible_script_movement
            .as_ref()
            .and_then(|movement| movement.active_stationary_effect),
        Some(VisibleStationaryMovementEffect::SkyfallWait)
    );
    assert_eq!(
        runtime_shell
            .visible_script_movement
            .as_ref()
            .map(|movement| movement.stationary_y_offset),
        Some(0),
        "StepFunction_Skyfall preserves the incoming OBJECT_SPRITE_Y_OFFSET during setup; an ordinary PIT tile did not run skyfall_top"
    );
    for _ in 0..16 {
        assert!(
            advance_visible_script_movement(&mut runtime_shell)
                .expect("advance skyfall setup")
        );
    }
    let movement = runtime_shell
        .visible_script_movement
        .as_ref()
        .expect("skyfall movement remains active");
    assert_eq!(
        movement.active_stationary_effect,
        Some(VisibleStationaryMovementEffect::SkyfallFall)
    );
    assert_eq!(
        movement.stationary_y_offset, -87,
        "StepFunction_Skyfall falls through from its setup countdown into the first Sine(1, $60) sample on the same object tick"
    );
}

#[test]
fn terminal_player_sprite_y_offset_survives_applymovement_return() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize retained-offset shell");
    runtime_shell
        .shell
        .session_mut()
        .overworld
        .set_player_facing(Direction::Left);
    let scene = runtime_shell.shell.snapshot().expect("movement scene");
    begin_visible_field_travel_movement(
        &mut runtime_shell,
        scene,
        VecDeque::from([VisibleScriptMovementPhase::Stationary {
            duration: 16,
            effect: VisibleStationaryMovementEffect::SkyfallTop,
        }]),
    )
    .expect("begin skyfall_top");
    assert_eq!(
        runtime_shell
            .visible_script_movement_scene
            .as_ref()
            .expect("retained skyfall_top scene")
            .overworld
            .facing,
        Direction::Left,
        "SetFacingSkyfall advances action-frame bits without changing OBJECT_DIRECTION"
    );

    for _ in 0..16 {
        advance_visible_script_movement(&mut runtime_shell)
            .expect("advance skyfall_top movement");
        if let Some(scene) = runtime_shell.visible_script_movement_scene.as_ref() {
            assert_eq!(
                scene.overworld.facing,
                Direction::Left,
                "skyfall_top action frames must not rotate OBJECT_DIRECTION"
            );
        }
    }

    assert!(runtime_shell.visible_script_movement.is_none());
    assert_eq!(
        runtime_shell.visible_player_sprite_y_offset, 96,
        "StepFunction_SkyfallTop stores $60 in OBJECT_SPRITE_Y_OFFSET before applymovement returns"
    );
    begin_visible_pitfall_landing(&mut runtime_shell)
        .expect("begin scripted pitfall with retained skyfall_top offset");
    assert_eq!(
        runtime_shell
            .visible_script_movement
            .as_ref()
            .map(|movement| movement.stationary_y_offset),
        Some(96),
        "FallIntoMapScript must inherit skyfall_top's terminal $60 offset"
    );
    runtime_shell.visible_script_movement = None;
    runtime_shell.visible_script_movement_scene = None;
    runtime_shell.visible_field_travel_animation = None;

    let scene = runtime_shell.shell.snapshot().expect("teleport movement scene");
    begin_visible_field_travel_movement(
        &mut runtime_shell,
        scene,
        VecDeque::from([VisibleScriptMovementPhase::Stationary {
            duration: 16,
            effect: VisibleStationaryMovementEffect::TeleportRise,
        }]),
    )
    .expect("begin teleport_from rise");
    for _ in 0..16 {
        advance_visible_script_movement(&mut runtime_shell)
            .expect("advance teleport_from rise");
    }
    assert_eq!(
        runtime_shell.visible_player_sprite_y_offset, -96,
        "StepFunction_TeleportFrom leaves its terminal -$60 offset for the following disappear/warp commands"
    );
}

#[test]
fn teleport_from_enters_rise_at_the_first_asm_sine_offset() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize teleport shell");
    let scene = runtime_shell.shell.snapshot().expect("movement scene");
    begin_visible_field_travel_movement(
        &mut runtime_shell,
        scene,
        VecDeque::from([
            VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::TeleportSpin,
            },
            VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::TeleportRise,
            },
        ]),
    )
    .expect("begin teleport_from");

    for _ in 0..16 {
        advance_visible_script_movement(&mut runtime_shell)
            .expect("advance teleport spin");
    }
    let movement = runtime_shell
        .visible_script_movement
        .as_ref()
        .expect("teleport rise remains active");
    assert_eq!(
        movement.active_stationary_effect,
        Some(VisibleStationaryMovementEffect::TeleportRise)
    );
    assert_eq!(
        movement.stationary_y_offset, -1,
        "StepFunction_TeleportFrom falls through from InitSpinRise into Sine($11, $60) on the same object tick"
    );
}

#[test]
fn poison_whiteout_presents_each_faint_before_exact_post_fade_hold() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize poison-whiteout shell");
    let event = crate::core::systems::step_events::StepEventResult {
        poison_result: Some(crate::core::systems::step_events::PoisonDamageResult {
            damaged_names: vec!["CHIKORITA".to_string(), "PIDGEY".to_string()],
            fainted_names: vec!["CHIKORITA".to_string(), "PIDGEY".to_string()],
        }),
        ..Default::default()
    };

    present_visible_step_event(&mut runtime_shell, &event)
        .expect("present poison faint sequence");

    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("CHIKORITA fainted!")
    );
    assert_eq!(
        runtime_shell
            .field_notice_queue
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["PIDGEY fainted!".to_string()],
        "CheckWhitedOut calls PrintText once for each poison-fainted party member",
    );
    assert_eq!(
        WHITEOUT_POST_FADE_HOLD_FRAMES, 40,
        "Script_Whiteout's pause 40 begins after FadeOutToWhite completes",
    );
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.pending_poison_blackout = true;
    assert!(
        begin_visible_poison_blackout_after_faint_text(&mut runtime_shell)
            .expect("enter OverworldWhiteoutScript after poison faint text")
    );
    assert_eq!(
        runtime_shell.visible_blackout_phase,
        Some(VisibleBlackoutPhase::AwaitText)
    );
    assert_eq!(
        runtime_shell
            .battle_messages
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            "PLAYER is out of\nuseable POKéMON!".to_string(),
            "PLAYER whited\nout!".to_string(),
        ],
    );
}

#[test]
fn map_trainer_interaction_presents_seen_text_before_starting_battle() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let (tile_x, tile_y) = (0..80)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .find(|(x, y)| {
            runtime
                .start_overworld_session_at_runtime_tile(&asset_root, "Route30", *x, *y)
                .is_ok()
        })
        .expect("Route 30 walkable tile");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route30".to_string(),
            tile_x,
            tile_y,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route 30 trainer shell");
    let species = runtime_shell.shell.runtime().data().pokemon["CYNDAQUIL"].clone();
    runtime_shell.shell.session_mut().state_mut().storage.party.pokemon[0] = Some(
        crate::core::models::Pokemon::new_for_tests(
            species,
            10,
            crate::core::models::Dv::default(),
        ),
    );
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .sync_party_from_storage();
    let interaction = crate::core::world::session::OverworldInteraction {
        map_name: "Route30".to_string(),
        player_tile: TilePosition::new(tile_x, tile_y),
        facing: Direction::Left,
        target_tile: TilePosition::new(tile_x.saturating_sub(2), tile_y),
        script: "TrainerYoungsterMikey".to_string(),
        target: crate::core::world::session::OverworldInteractionTarget::Object {
            object_index: 1,
            object_identifier: Some("ROUTE30_YOUNGSTER2".to_string()),
            object_type: "OBJECTTYPE_TRAINER".to_string(),
        },
    };
    assert!(!runtime_shell
        .shell
        .session()
        .state()
        .flags
        .is_event_flag_set("EVENT_BEAT_YOUNGSTER_MIKEY")
        .expect("Mikey event flag"));

    dispatch_visible_overworld_interaction(&mut runtime_shell, interaction, "trainer_test")
        .expect("dispatch unbeaten trainer interaction");

    assert!(
        runtime_shell.shell.snapshot().expect("intro snapshot").battle.is_none(),
        "trainer intro was bypassed: pending={:?} notice={:?} events={:?}",
        runtime_shell.pending_trainer_intro,
        runtime_shell.field_notice,
        runtime_shell.last_audio_events,
    );
    assert!(runtime_shell.pending_trainer_intro.is_some());
    for (symbol, expected) in [
        ("wRunningTrainerBattleScript", "0"),
        ("wBattleScriptFlags", "129"),
        ("wOtherTrainerClass", "YOUNGSTER"),
        ("wOtherTrainerID", "MIKEY"),
        ("wWinTextPointer", "YoungsterMikeyBeatenText"),
        ("wLossTextPointer", "0"),
    ] {
        assert_eq!(
            runtime_shell
                .shell
                .session()
                .state()
                .script_runtime
                .memory
                .get(symbol)
                .map(String::as_str),
            Some(expected),
            "TalkToTrainer/loadtemptrainer must write {symbol} before seen text",
        );
    }
    assert!(runtime_shell
        .field_notice
        .as_deref()
        .is_some_and(|text| text.contains("trainer, right?") && text.contains("battle!")));
    assert!(runtime_shell
        .pending_audio
        .iter()
        .any(|audio| audio.audio_id == "MUSIC_YOUNGSTER_ENCOUNTER"));

    runtime_shell.field_notice = None;
    finish_visible_map_trainer_intro(&mut runtime_shell)
        .expect("start battle after seen text waitbutton");
    assert!(runtime_shell.shell.snapshot().expect("battle snapshot").battle.is_some());
    assert!(runtime_shell.pending_trainer_intro.is_none());
}

#[test]
fn defeated_map_trainer_dispatches_talk_after_callback_without_battle() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let (tile_x, tile_y) = (0..80)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .find(|(x, y)| {
            runtime
                .start_overworld_session_at_runtime_tile(&asset_root, "Route30", *x, *y)
                .is_ok()
        })
        .expect("Route 30 walkable tile");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route30".to_string(),
            tile_x,
            tile_y,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize defeated Route 30 trainer shell");
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .flags
        .set_event_flag("EVENT_BEAT_YOUNGSTER_MIKEY", true)
        .expect("set Mikey defeated flag");
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .script_runtime
        .memory
        .insert("wRunningTrainerBattleScript".to_string(), "-1".to_string());
    let interaction = crate::core::world::session::OverworldInteraction {
        map_name: "Route30".to_string(),
        player_tile: TilePosition::new(tile_x, tile_y),
        facing: Direction::Left,
        target_tile: TilePosition::new(tile_x.saturating_sub(2), tile_y),
        script: "TrainerYoungsterMikey".to_string(),
        target: crate::core::world::session::OverworldInteractionTarget::Object {
            object_index: 1,
            object_identifier: Some("ROUTE30_YOUNGSTER2".to_string()),
            object_type: "OBJECTTYPE_TRAINER".to_string(),
        },
    };

    dispatch_visible_overworld_interaction(
        &mut runtime_shell,
        interaction,
        "defeated_trainer_test",
    )
    .expect("dispatch defeated trainer interaction");

    let snapshot = runtime_shell.shell.snapshot().expect("defeated trainer snapshot");
    assert!(snapshot.battle.is_none());
    assert!(runtime_shell.pending_trainer_intro.is_none());
    assert_eq!(
        snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
        Some("YoungsterMikeyAfterText"),
        "AlreadyBeatenTrainerScript must run the trainer table callback",
    );
    assert_eq!(
        runtime_shell
            .shell
            .session()
            .state()
            .script_runtime
            .memory
            .get("wRunningTrainerBattleScript")
            .map(String::as_str),
        Some("0"),
        "manual TalkToTrainer must clear the post-battle guard before callback",
    );
}

#[test]
fn item_ball_pickup_opens_the_canonical_found_item_notice() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize item-ball shell");
    runtime_shell
        .shell
        .set_trainer_identity("KRIS", 1)
        .expect("set trainer identity");
    let outcome = RuntimeMutationOutcome {
        result: RuntimeMutationResult::ScriptFieldItemPickedUp(FieldItemPickupOutcome::Collected {
            item_id: "POTION".to_string(),
            quantity: 1,
            event_flag: "EVENT_ROUTE_29_POTION".to_string(),
            source: crate::core::systems::field_items::FieldItemSource::ItemBall,
        }),
        state_checksum: runtime_shell
            .shell
            .state_checksum()
            .expect("state checksum"),
    };

    integrate_visible_script_mutation_outcome(&mut runtime_shell, &outcome)
        .expect("integrate item-ball pickup");

    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("KRIS found\nPOTION!")
    );
    assert!(
        !runtime_shell
            .pending_audio
            .iter()
            .any(|audio| audio.audio_id == "SFX_ITEM"),
        "FindItemInBallScript plays its fanfare only after FoundItemText finishes"
    );
    assert_eq!(
        runtime_shell
            .visible_item_ball_notice
            .as_ref()
            .map(|notice| &notice.phase),
        Some(&VisibleItemBallPhase::FoundText)
    );
    assert!(!visible_field_notice_uses_prompt_arrow(&runtime_shell));

    begin_visible_item_ball_fanfare_pause(&mut runtime_shell)
        .expect("finish found-item text");
    assert!(runtime_shell
        .pending_audio
        .iter()
        .any(|audio| audio.audio_id == "SFX_ITEM"));
    for _ in 0..59 {
        assert!(advance_visible_item_ball_fanfare_pause(&mut runtime_shell));
        assert_eq!(runtime_shell.field_notice, None);
    }
    assert!(advance_visible_item_ball_fanfare_pause(&mut runtime_shell));
    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("KRIS put the\nPOTION in\nthe ITEM POCKET.")
    );
    assert!(visible_field_notice_uses_prompt_arrow(&runtime_shell));
}

#[test]
fn full_item_ball_never_plays_success_fanfare_or_itemnotify() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize item-ball shell");
    runtime_shell
        .shell
        .set_trainer_identity("KRIS", 1)
        .expect("set trainer identity");

    show_visible_item_ball_notice(&mut runtime_shell, "POTION", true)
        .expect("show full item-ball path");

    assert_eq!(runtime_shell.field_notice.as_deref(), Some("KRIS found\nPOTION!"));
    assert_eq!(
        runtime_shell
            .visible_item_ball_notice
            .as_ref()
            .map(|notice| (&notice.phase, notice.pocket_text.as_str())),
        Some((
            &VisibleItemBallPhase::BagFullFoundText,
            "But KRIS can't\ncarry any more\nitems."
        ))
    );
    assert!(!runtime_shell
        .pending_audio
        .iter()
        .any(|audio| audio.audio_id == "SFX_ITEM"));
    assert!(visible_field_notice_uses_prompt_arrow(&runtime_shell));
}

#[test]
fn elms_aide_verbose_potion_stops_before_the_following_dialogue() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab at the aide trigger");
    runtime_shell
        .shell
        .set_trainer_identity("KRIS", 1)
        .expect("set trainer identity");

    let run = runtime_shell
        .shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "ElmsLab".to_string(),
                source_script: "AideScript_GivePotion".to_string(),
                command_index: 3,
            },
            256,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run the aide's verbose Potion grant");

    assert_eq!(run.steps.len(), 1, "verbosegiveitem must yield before writetext");
    assert_eq!(run.steps[0].command, "verbosegiveitem");
    assert_eq!(run.boundary, Some(RuntimeCompiledScriptBoundary::VerboseItemGrant));
    assert_eq!(
        run.next_cursor.as_ref().map(|cursor| cursor.command_index),
        Some(4),
        "AideText_AlwaysBusy must remain behind the item-receipt presentation boundary",
    );

    integrate_visible_compiled_script_run(&mut runtime_shell, &run.steps)
        .expect("present the verbose Potion grant");
    assert_eq!(runtime_shell.field_notice.as_deref(), Some("KRIS received\nPOTION."));
    assert!(
        runtime_shell
            .pending_audio
            .iter()
            .any(|audio| audio.audio_id == "SFX_ITEM"),
        "the receive-item boundary must retain the canonical item fanfare",
    );
}

#[test]
fn route29_fruit_tree_interaction_grants_and_presents_the_berry() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route29".to_string(),
            tile_x: 12,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route 29 fruit-tree shell");
    runtime_shell.shell.session_mut().state_mut().player_name = "KRIS".to_string();
    runtime_shell.shell.session_mut().overworld.player.facing = Direction::Up;

    let interaction = runtime_shell
        .shell
        .current_overworld_interaction()
        .expect("player must be able to talk to the Route 29 fruit tree");
    assert_eq!(interaction.script, "Route29FruitTree");

    dispatch_visible_overworld_interaction(
        &mut runtime_shell,
        interaction,
        "route29_fruit_tree_test",
    )
    .expect("dispatch Route 29 fruit-tree interaction");

    let berry = runtime_shell.shell.runtime().data().items["BERRY"].clone();
    assert_eq!(
        runtime_shell.shell.session().state().bag.quantity(&berry),
        1,
        "talking to an unpicked tree must grant its catalog berry",
    );
    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("It's a fruit-\nbearing tree."),
        "the atomic fruit-tree mutation must still present the authored opening text",
    );
    assert_eq!(
        runtime_shell
            .field_notice_queue
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            "Hey! It's\nBERRY!".to_string(),
            "Obtained\nBERRY!".to_string(),
            "KRIS put the\nBERRY in\nthe ITEM POCKET.".to_string(),
        ],
    );

    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    let interaction = runtime_shell
        .shell
        .current_overworld_interaction()
        .expect("the picked tree must remain interactable");
    dispatch_visible_overworld_interaction(
        &mut runtime_shell,
        interaction,
        "route29_picked_fruit_tree_test",
    )
    .expect("dispatch already-picked Route 29 fruit tree");

    assert_eq!(
        runtime_shell.shell.session().state().bag.quantity(&berry),
        1,
        "talking to the picked tree must not duplicate its berry",
    );
    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("It's a fruit-\nbearing tree."),
    );
    assert_eq!(
        runtime_shell
            .field_notice_queue
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["There's nothing\nhere…".to_string()],
    );
}

#[test]
fn real_pack_bug_contest_timeout_warps_to_national_park_gate() {
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
    let (tile_x, tile_y) = (0..80)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .find(|(x, y)| {
            runtime
                .start_overworld_session_at_runtime_tile(
                    &asset_root,
                    "NationalParkBugContest",
                    *x,
                    *y,
                )
                .is_ok()
        })
        .expect("compiled Bug Contest map must have a walkable tile");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "NationalParkBugContest".to_string(),
            tile_x,
            tile_y,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Bug Contest map shell");
    let state = runtime_shell.shell.session_mut().state_mut();
    state.bug_contest.timer_active = true;
    state.bug_contest.timer_minutes_remaining = 20;
    state.bug_contest.timer_seconds_remaining = 0;
    state.bug_contest.timer_start_time = Some(crate::core::systems::time::ClockTime {
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
    });
    state.time.current_day = 1;
    state.time.day_of_week = 1;
    state
        .flags
        .set_engine_flag("ENGINE_BUG_CONTEST_TIMER", true)
        .expect("compiled pack Bug Contest timer flag");

    runtime_shell
        .shell
        .tick(std::iter::empty::<GameButton>())
        .expect("advance expired Bug Contest frame");
    let snapshot = runtime_shell.shell.snapshot().expect("timeout snapshot");
    assert_eq!(snapshot.overworld.map_name, "Route36NationalParkGate");
    assert_eq!(snapshot.overworld.tile, TilePosition::new(0, 4));
    assert!(!snapshot.bug_contest.timer_active);
    assert_eq!(
        runtime_shell
            .shell
            .session()
            .state()
            .flags
            .is_event_flag_set("EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_CONTEST_DAY")
            .expect("contest-day flag"),
        true
    );
}

#[test]
fn visible_overworld_normal_inputs_trigger_mom_coord_event() {
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
    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &[
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Up],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
            vec![GameButton::Down],
        ],
        None,
    )
    .expect("normal inputs trigger Mom's first-floor coord event");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(smoke.final_map, "PlayersHouse1F");
    assert_eq!((smoke.final_tile_x, smoke.final_tile_y), (9, 4));
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_PLAYERSHOUSE1F_NOOP")
    );
    assert_eq!(smoke.warps, 1);
    assert_eq!(smoke.coord_events, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("warp=true"))
    );
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("coord=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("MUSIC_MOM"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag ENGINE_POKEGEAR=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag ENGINE_PHONE_CARD=true"))
    );
}

#[test]
fn moms_coord_event_keeps_the_written_dialogue_in_the_textbox() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize new-game shell");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("complete player name");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell).expect("settle new-game scripts");

    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    // Follow the same keyboard/update path available to a player and let the
    // authored stair fade finish before stepping onto Mom's coordinate event.
    for key in [
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
    ] {
        press_key_for_runtime_hotkey_app(&mut app, key);
        for _ in 0..8 {
            app.update();
        }
    }
    for _ in 0..64 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        if shell.visible_walk_warp_phase.is_none()
            && shell.shell.snapshot().unwrap().overworld.map_name == "PlayersHouse1F"
        {
            break;
        }
    }
    for _ in 0..4 {
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
        for _ in 0..8 {
            app.update();
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for _ in 0..16 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .visible_overworld_emote
            .is_some()
        {
            break;
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::ArrowDown);

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    let initial_snapshot = runtime_shell.shell.snapshot().expect("Mom emote snapshot");
    assert_ne!(
        initial_snapshot
            .ui
            .text
            .as_ref()
            .map(|text| text.label.as_str()),
        Some("ElmsLookingForYouText"),
        "Mom script fell through to text before its blocking visual phases: action={:?} cursor={:?} movement={:?} emote={:?} events={:?}",
        runtime_shell.last_runtime_action,
        runtime_shell.active_script_cursor,
        runtime_shell.visible_script_movement,
        runtime_shell.visible_overworld_emote,
        runtime_shell.last_audio_events
    );
    assert!(runtime_shell.visible_script_movement.is_none());
    let emote_frames = runtime_shell
        .visible_overworld_emote
        .as_ref()
        .map(|emote| emote.frames_remaining)
        .expect("the real input path must trigger Mom's emote");
    assert_eq!(
        emote_frames, 29,
        "the first rendered frame consumes one of showemote's 30 source frames"
    );

    for frame in 0..emote_frames.saturating_sub(1) {
        app.update();
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = runtime_shell.shell.snapshot().expect("Mom emote frame");
        assert_eq!(runtime_shell.last_error, None, "emote frame {frame}");
        assert!(
            runtime_shell.visible_overworld_emote.is_some(),
            "emote frame {frame}"
        );
        assert!(
            runtime_shell.visible_script_movement.is_none(),
            "emote frame {frame}"
        );
        assert_ne!(
            snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
            Some("ElmsLookingForYouText"),
            "emote frame {frame}"
        );
    }

    let mut saw_mom_movement = false;
    let mut dialogue_opened = false;
    for _ in 0..64 {
        app.update();
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(runtime_shell.last_error, None);
        saw_mom_movement |= runtime_shell.visible_script_movement.is_some();
        let snapshot = runtime_shell.shell.snapshot().expect("Mom sequence frame");
        if snapshot
            .ui
            .text
            .as_ref()
            .is_some_and(|text| text.label == "ElmsLookingForYouText")
        {
            dialogue_opened = true;
            break;
        }
    }
    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert!(
        saw_mom_movement,
        "Mom must walk after the emote finishes: action={:?} cursor={:?} movement={:?} emote={:?} events={:?}",
        runtime_shell.last_runtime_action,
        runtime_shell.active_script_cursor,
        runtime_shell.visible_script_movement,
        runtime_shell.visible_overworld_emote,
        runtime_shell.last_audio_events
    );
    assert!(
        dialogue_opened,
        "Mom's dialogue must open after her walk finishes: action={:?} cursor={:?} movement={:?} emote={:?} text={:?} events={:?}",
        runtime_shell.last_runtime_action,
        runtime_shell.active_script_cursor,
        runtime_shell.visible_script_movement,
        runtime_shell.visible_overworld_emote,
        runtime_shell
            .shell
            .snapshot()
            .ok()
            .and_then(|snapshot| snapshot.ui.text),
        runtime_shell.last_audio_events
    );

    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("Mom dialogue snapshot");
    let text = snapshot.ui.text.as_ref().expect("Mom textbox is active");
    assert_eq!(text.label, "ElmsLookingForYouText");
    let rendered = render_visible_script_text_body(
        text.body.as_ref().expect("Mom text is a map text body"),
        &snapshot.script_events.named_buffers,
        &snapshot.trainer.player_name,
        visible_rival_name(&snapshot),
        snapshot.progression.time.day_of_week,
    );
    assert!(rendered.contains("Oh, AB…! Our"));
    assert!(rendered.contains("neighbor, PROF."));
    assert!(!rendered.contains("MeetMomScript"));
    assert!(!rendered.contains('"'));
}

fn assert_rendered_field_dialogue_page(world: &mut World, expected_page: &str) {
    let expected_rows = expected_page.lines().collect::<Vec<_>>();
    assert!(
        world
            .query_filtered::<Entity, With<SceneDialogTextBoxBackgroundMarker>>()
            .iter(world)
            .next()
            .is_some(),
        "the rendered frame has dialogue glyphs without the textbox surface"
    );
    let expected = {
        let rendered_art = world.resource::<RenderedTilesetArt>();
        let font = rendered_art
            .font_cache
            .as_ref()
            .expect("rendered dialogue must have loaded the bitmap font");
        expected_rows
            .iter()
            .enumerate()
            .flat_map(|(row_index, line)| {
                let (x, y) = battle_hud_tile_origin(
                    FIELD_TEXT_BOX_TEXT_LEFT_TILE,
                    FIELD_TEXT_BOX_TEXT_TOP_TILE
                        + row_index as f32 * FIELD_TEXT_BOX_ROW_SPACING_TILES,
                );
                normalize_bitmap_font_text(line)
                    .chars()
                    .enumerate()
                    .map(move |(glyph_index, ch)| {
                        let frame = font
                            .glyphs
                            .get(&ch)
                            .or_else(|| font.glyphs.get(&'?'))
                            .expect("rendered font must contain the fallback glyph");
                        (
                            dialog_glyph_key(x, y, glyph_index),
                            format!("{:?}", frame.handle.id()),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let expected_y = expected_rows
        .iter()
        .enumerate()
        .map(|(row_index, _)| {
            battle_hud_tile_origin(
                FIELD_TEXT_BOX_TEXT_LEFT_TILE,
                FIELD_TEXT_BOX_TEXT_TOP_TILE
                    + row_index as f32 * FIELD_TEXT_BOX_ROW_SPACING_TILES,
            )
            .1
        })
        .collect::<Vec<_>>();
    let mut actual = world
        .query::<(&DialogGlyphMarker, &Handle<Image>, &Transform)>()
        .iter(world)
        .filter(|(_, _, transform)| {
            expected_y
                .iter()
                .any(|expected| (transform.translation.y - expected).abs() < f32::EPSILON)
        })
        .map(|(marker, texture, _)| (marker.key, format!("{:?}", texture.id())))
        .collect::<Vec<_>>();
    let mut expected = expected;
    actual.sort();
    expected.sort();
    assert_eq!(
        actual, expected,
        "the actual Bevy glyph sprites do not encode the active dialogue page {expected_page:?}"
    );
}

#[test]
fn visible_new_game_completes_mom_walks_to_elms_lab_and_gets_rendered_starter() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig::default(),
    )
    .expect("initialize new game");
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, Some("AB"))
        .expect("finish player naming");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell).expect("settle bedroom arrival");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    // Real joypad route from the bedroom spawn, through the stair warp, and
    // onto Mom's authored coordinate event.
    for key in [
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
    ] {
        press_key_for_runtime_hotkey_app(&mut app, key);
        for _ in 0..8 {
            app.update();
        }
    }
    for _ in 0..64 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        if shell.visible_walk_warp_phase.is_none()
            && shell.shell.snapshot().unwrap().overworld.map_name == "PlayersHouse1F"
        {
            break;
        }
    }
    for _ in 0..5 {
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
        for _ in 0..8 {
            app.update();
        }
    }

    let mut seen_labels = Vec::new();
    let mut dialogue_activations = Vec::new();
    let mut dialogue_pages = Vec::new();
    let mut rendered_dialogue_pages = Vec::new();
    let mut previous_visible_label = None;
    let mut saw_rendered_mom_text = false;
    let mut saw_rendered_yes_no = false;
    let mut saw_canonical_day_selector = false;
    let mut saw_dynamic_dst_confirmation = false;
    let mut longest_dst_confirmation = Vec::new();
    let mut completed_mom = false;
    let mut mom_frames = 0usize;
    let mut dialogue_mom_tile = None;
    let mut dialogue_mom_render_position = None;
    let mut yes_no_boundaries = Vec::new();
    let mut yes_no_render_trace = Vec::new();
    let mut was_pending_yes_no = false;
    let mut saw_yes_no_prompt_cleared = false;
    let mut saw_yes_no_frame_cleared = false;
    let baseline_live_entities = app.world().entities().len();
    let mut peak_live_entities = baseline_live_entities;
    let mut previous_progress_signature = None;
    let mut previous_semantic_trace = None;
    let mut stationary_script_frames = 0usize;
    let mut saw_exact_received_pokegear_text = false;
    let mut saw_item_reward_sound = false;
    let mut saw_reward_waitsfx_complete_without_input = false;
    let mut saw_final_phone_text = false;
    let mut final_phone_text_closed = false;
    let mut saw_authored_mom_return = false;
    let mut rendered_mom_departure_x = Vec::new();
    let mut proved_hidden_yes_no_ignores_direction = false;
    let mut proved_start_ignored_during_mom_dialogue = false;
    let mut proved_completed_page_does_not_auto_advance = false;
    let mut proved_weekday_input_is_immediate = false;
    let mut release_scheduled_a_after_observation = false;
    for frame in 0..1024 {
        mom_frames = frame + 1;
        app.update();
        if release_scheduled_a_after_observation {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset(KeyCode::KeyZ);
            release_scheduled_a_after_observation = false;
        }
        peak_live_entities = peak_live_entities.max(app.world().entities().len());
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            shell.last_error, None,
            "Mom lifecycle failed: {:?}",
            shell.last_audio_events
        );
        let snapshot = shell.shell.snapshot().expect("Mom lifecycle snapshot");
        let progress_signature = format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            shell.active_script_cursor,
            snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
            shell.field_text_reveal.as_ref().map(|reveal| (
                reveal.page_index,
                reveal.visible_chars,
                reveal.frames_until_next_char,
            )),
            snapshot.ui.pending_yes_no.as_ref().and_then(|_| shell
                .yes_no_cursor
                .as_ref()
                .map(|cursor| cursor.option_index)),
            shell.visible_script_delay_frames,
            shell.visible_script_movement,
            shell.pending_day_of_week,
        );
        let semantic_trace = format!(
            "cursor={:?} label={:?} page={:?} yes_no={} special={:?} script_value={:?} day_prompt={:?} mom_tile={:?}",
            shell.active_script_cursor,
            snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
            shell
                .field_text_reveal
                .as_ref()
                .map(|reveal| reveal.page_index),
            snapshot.ui.pending_yes_no.is_some(),
            snapshot.script_events.last_special_routine,
            snapshot.script_events.script_value,
            shell.pending_day_of_week.as_ref().map(|prompt| (
                prompt.selected_day,
                prompt.confirming,
                prompt.yes_no_index,
            )),
            snapshot
                .visible_object_runtime_tiles
                .get("PLAYERSHOUSE1F_MOM1"),
        );
        if previous_semantic_trace.as_ref() != Some(&semantic_trace) {
            eprintln!("mom_dialogue_trace frame={frame} {semantic_trace}");
            previous_semantic_trace = Some(semantic_trace);
        }
        if shell.pending_day_of_week.is_some() {
            let pages = visible_field_dialog_pages(&snapshot, shell)
                .expect("weekday selector must retain Mom's preceding textbox");
            let reveal = shell
                .field_text_reveal
                .as_ref()
                .expect("weekday selector must not outrun text reveal initialization");
            assert_eq!(
                reveal.page_index + 1,
                pages.len(),
                "weekday selector opened before every MomGivesPokegearText page was consumed"
            );
            assert!(
                visible_field_dialogue_is_fully_revealed(shell, &snapshot),
                "weekday selector opened before MomGivesPokegearText's final page finished printing"
            );
        }
        if previous_progress_signature.as_ref() == Some(&progress_signature) {
            stationary_script_frames += 1;
        } else {
            previous_progress_signature = Some(progress_signature.clone());
            stationary_script_frames = 0;
        }
        assert!(
            stationary_script_frames < 120,
            "Mom script stopped making progress for {stationary_script_frames} frames: {progress_signature}"
        );
        let visible_label = snapshot.ui.text.as_ref().map(|text| text.label.clone());
        if saw_rendered_mom_text && snapshot.ui.text_window_open {
            assert!(
                visible_label.is_some(),
                "Mom's open textbox lost its active text between script boundaries: cursor={:?} active={:?} last_text_event={:?} wait={:?} yes_no={} movement={:?}",
                shell.active_script_cursor,
                shell.shell.session().state.script_runtime.active_text_label,
                shell.shell.session().state.script_runtime.text_events.last(),
                snapshot.ui.pending_text_wait,
                snapshot.ui.pending_yes_no.is_some(),
                shell.visible_script_movement,
            );
        }
        saw_reward_waitsfx_complete_without_input |= saw_exact_received_pokegear_text
            && !shell.visible_wait_sfx_boundary
            && visible_label.as_deref() != Some("ReceivedItemText");
        if visible_label.as_deref() == Some("ReceivedItemText") {
            assert_eq!(
                shell.field_notice, None,
                "ReceiveItemScript must have exactly one presentation owner; a Bevy field notice would duplicate the canonical ReceivedItemText"
            );
            let pages = visible_field_dialog_pages(&snapshot, shell)
                .expect("received Pokegear text must render");
            assert_eq!(
                pages,
                vec!["AB received\nPOKéGEAR.".to_string()],
                "ReceiveItemScript must expand STRING_BUFFER_4 before any glyph is rendered"
            );
            assert!(
                pages.iter().all(|page| !page.contains("STRING_BUFFER")),
                "runtime buffer tokens must never become player-visible text"
            );
            saw_exact_received_pokegear_text = true;
        }
        saw_item_reward_sound |= shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("SFX_ITEM"));
        let mut fully_rendered_page = None;
        if let (Some(label), Some(reveal), Some(pages)) = (
            visible_label.as_ref(),
            shell.field_text_reveal.as_ref(),
            visible_field_dialog_pages(&snapshot, shell),
        ) && reveal.text == pages.join("\u{1e}")
        {
            if reveal.page_index > 0 {
                let carried_chars = visible_field_page_initial_chars(
                    &pages[reveal.page_index - 1],
                    &pages[reveal.page_index],
                );
                assert!(
                    reveal.visible_chars >= carried_chars,
                    "Mom retyped a line that ASM <CONT> had already scrolled: label={label} page={} visible={} carried={carried_chars}",
                    reveal.page_index,
                    reveal.visible_chars,
                );
            }
            let visible_page = (label.clone(), reveal.page_index);
            if dialogue_pages.last() != Some(&visible_page) {
                dialogue_pages.push(visible_page);
            }
            if visible_field_text_reveal_is_complete(reveal, &pages[reveal.page_index]) {
                fully_rendered_page = Some((
                    (label.clone(), reveal.page_index),
                    pages[reveal.page_index].clone(),
                ));
            }
        }
        saw_final_phone_text |= visible_label.as_deref() == Some("InstructionsNextText");
        final_phone_text_closed |= saw_final_phone_text
            && visible_label.is_none()
            && !snapshot.ui.text_window_open
            && snapshot.ui.pending_yes_no.is_none();
        if saw_rendered_mom_text {
            let rendered_moms = app
                .world()
                .iter_entities()
                .filter_map(|entity| {
                    entity
                        .get::<VisibleObjectSprite>()
                        .filter(|sprite| {
                            sprite.object_identifier.as_deref()
                                == Some("PLAYERSHOUSE1F_MOM1")
                        })
                        .and_then(|_| entity.get::<Transform>())
                        .map(|transform| transform.translation.x)
                })
                .collect::<Vec<_>>();
            assert_eq!(
                rendered_moms.len(),
                1,
                "the actual Bevy frame must contain exactly one Mom sprite, found {rendered_moms:?}"
            );
            if final_phone_text_closed {
                rendered_mom_departure_x.push(rendered_moms[0]);
            }
        }
        let mom_movement_active = shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| movement.object_id == "PLAYERSHOUSE1F_MOM1");
        assert!(
            !(saw_rendered_mom_text && !final_phone_text_closed && mom_movement_active),
            "Mom moved again after her dialogue began but before the complete date/DST/phone dialogue closed; label={visible_label:?} cursor={:?}",
            shell.active_script_cursor
        );
        saw_authored_mom_return |= final_phone_text_closed && mom_movement_active;
        if let Some(label) = visible_label.clone()
            && previous_visible_label.as_ref() != Some(&label)
        {
            dialogue_activations.push(label.clone());
            previous_visible_label = Some(label);
        } else if visible_label.is_none() {
            previous_visible_label = None;
        }
        let pending_yes_no = snapshot.ui.pending_yes_no.is_some();
        if shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| movement.object_id == "PLAYERSHOUSE1F_MOM1")
        {
            assert!(
                !shell
                    .visible_script_movement_scene
                    .as_ref()
                    .is_some_and(|scene| scene.ui.text_window_open),
                "Mom's retained movement scene still renders an open textbox"
            );
        }
        if was_pending_yes_no && !pending_yes_no {
            saw_yes_no_prompt_cleared = !app
                .world()
                .iter_entities()
                .any(|entity| entity.contains::<YesNoPromptMarker>());
            let (prompt_left_x, prompt_top_y) =
                battle_hud_tile_origin(FIELD_YES_NO_LEFT_TILE, FIELD_YES_NO_TOP_TILE);
            saw_yes_no_frame_cleared = !app.world().iter_entities().any(|entity| {
                entity.contains::<SceneDialogWindowFrameMarker>()
                    && entity.get::<Transform>().is_some_and(|transform| {
                        (transform.translation.x - prompt_left_x).abs() < f32::EPSILON
                            && (transform.translation.y - prompt_top_y).abs() < f32::EPSILON
                    })
            });
        }
        was_pending_yes_no = pending_yes_no;
        if pending_yes_no {
            let boundary = format!("{:?}", shell.active_script_cursor);
            if yes_no_boundaries.last() != Some(&boundary) {
                yes_no_boundaries.push(boundary);
                yes_no_render_trace.push((
                    snapshot.script_events.last_special_routine.clone(),
                    visible_scene_dialog_entries(&snapshot, shell)
                        .expect("render Mom yes/no trace"),
                ));
            }
        }
        if snapshot.ui.text_window_open || pending_yes_no || shell.pending_day_of_week.is_some() {
            let mom_tile = snapshot
                .visible_object_runtime_tiles
                .get("PLAYERSHOUSE1F_MOM1")
                .copied()
                .expect("Mom must have a live runtime tile during her dialogue");
            let expected = *dialogue_mom_tile.get_or_insert(mom_tile);
            assert_eq!(
                mom_tile,
                TilePosition { x: 8, y: 4 },
                "ASM MomWalksToPlayerMovement must finish at (8,4) before Mom owns the dialogue"
            );
            assert_eq!(
                mom_tile, expected,
                "Mom moved while dialogue still owned the scene; label={visible_label:?} cursor={:?}",
                shell.active_script_cursor
            );
            let rendered_dialog_visible = shell
                .visible_script_movement_scene
                .as_ref()
                .map_or(snapshot.ui.text_window_open, |scene| {
                    scene.ui.text_window_open
                });
            if rendered_dialog_visible
                && let Some(render_position) = app.world().iter_entities().find_map(|entity| {
                    entity
                        .get::<VisibleObjectSprite>()
                        .filter(|sprite| {
                            sprite.object_identifier.as_deref() == Some("PLAYERSHOUSE1F_MOM1")
                        })
                        .and_then(|_| entity.get::<Transform>())
                        .map(|transform| transform.translation.truncate())
                })
            {
                let expected = *dialogue_mom_render_position.get_or_insert(render_position);
                assert_eq!(
                    render_position, expected,
                    "Mom's rendered sprite moved while dialogue was visible; label={visible_label:?} cursor={:?}",
                    shell.active_script_cursor
                );
            }
        }
        if pending_yes_no
            && matches!(
                snapshot.script_events.last_special_routine.as_deref(),
                Some("InitialSetDSTFlag" | "InitialClearDSTFlag")
            )
        {
            let entries = visible_scene_dialog_entries(&snapshot, shell)
                .expect("render Mom DST confirmation entries");
            if entries.iter().map(String::len).sum::<usize>()
                > longest_dst_confirmation
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
            {
                longest_dst_confirmation = entries.clone();
            }
            saw_dynamic_dst_confirmation |= entries.iter().any(|line| line.contains(':'))
                && entries.iter().any(|line| line.contains("is that OK"));
        }
        if let Some(label) = visible_label.clone()
            && seen_labels.last() != Some(&label)
        {
            seen_labels.push(label);
        }
        let shell = app.world().resource::<BevyRuntimeShell>();
        let current_scene = shell
            .shell
            .current_scene_script()
            .expect("current Player's House scene")
            .map(|scene| scene.scene_id);
        completed_mom = snapshot.overworld.map_name == "PlayersHouse1F"
            && current_scene.as_deref() == Some("SCENE_PLAYERSHOUSE1F_NOOP")
            && snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_POKEGEAR")
            && snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_PHONE_CARD")
            && shell.active_script_cursor.is_none()
            && shell.visible_script_movement.is_none()
            && shell.visible_overworld_emote.is_none()
            && shell.player_walk_frame_ticks == 0
            && shell.object_walk_frame_ticks == 0
            && shell.object_walk_frame_ticks_by_id.is_empty()
            && shell.special_boundary.is_none()
            && shell.pending_day_of_week.is_none()
            && !snapshot.ui.text_window_open;
        if completed_mom {
            break;
        }
        let pending_day_of_week = shell.pending_day_of_week.is_some();
        let pending_day_confirming = shell
            .pending_day_of_week
            .as_ref()
            .is_some_and(|prompt| prompt.confirming);
        let hidden_yes_no_while_text_owns_input =
            pending_yes_no && !visible_field_dialogue_is_entirely_consumed(shell, &snapshot);
        let hidden_yes_no_cursor_before = shell
            .yes_no_cursor
            .as_ref()
            .map(|cursor| cursor.option_index);
        let visible_wait_sfx_boundary = shell.visible_wait_sfx_boundary;
        let mut start_guard_already_pressed_a = false;
        let idle_page_guard_before = (!proved_completed_page_does_not_auto_advance
            && visible_label.as_deref() == Some("ElmsLookingForYouText")
            && visible_field_dialogue_is_fully_revealed(shell, &snapshot)
            && shell
                .field_text_reveal
                .as_ref()
                .zip(visible_field_dialog_pages(&snapshot, shell).as_ref())
                .is_some_and(|(reveal, pages)| reveal.page_index + 1 < pages.len()))
        .then(|| {
            (
                visible_label.clone(),
                shell
                    .field_text_reveal
                    .as_ref()
                    .map(|reveal| reveal.page_index),
                shell.active_script_cursor.clone(),
                snapshot
                    .visible_object_runtime_tiles
                    .get("PLAYERSHOUSE1F_MOM1")
                    .copied(),
            )
        });
        let start_guard_before = (!proved_start_ignored_during_mom_dialogue
            && visible_label.is_some()
            && visible_field_dialogue_is_entirely_consumed(shell, &snapshot)
            && snapshot.ui.pending_text_wait.is_some()
            && shell.pending_day_of_week.is_none()
            && shell.pending_phone_prompt.is_none()
            && !visible_wait_sfx_boundary)
            .then(|| {
                (
                    visible_label.clone(),
                    shell
                        .field_text_reveal
                        .as_ref()
                        .map(|reveal| reveal.page_index),
                    shell.active_script_cursor.clone(),
                    snapshot
                        .visible_object_runtime_tiles
                        .get("PLAYERSHOUSE1F_MOM1")
                        .copied(),
                )
            });
        let _ = shell;
        if let Some((page_identity, page_text)) = fully_rendered_page
            && !rendered_dialogue_pages.contains(&page_identity)
        {
            assert_rendered_field_dialogue_page(app.world_mut(), &page_text);
            rendered_dialogue_pages.push(page_identity);
        }
        if let Some(before) = idle_page_guard_before {
            for _ in 0..30 {
                app.update();
            }
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("Mom idle-page snapshot");
            let after = (
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                shell
                    .field_text_reveal
                    .as_ref()
                    .map(|reveal| reveal.page_index),
                shell.active_script_cursor.clone(),
                snapshot
                    .visible_object_runtime_tiles
                    .get("PLAYERSHOUSE1F_MOM1")
                    .copied(),
            );
            assert_eq!(
                after, before,
                "Mom's fully printed nonfinal text page advanced without A/B input"
            );
            proved_completed_page_does_not_auto_advance = true;
        }
        if let Some(before) = start_guard_before {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("Mom Start-guard snapshot");
            let after = (
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                shell
                    .field_text_reveal
                    .as_ref()
                    .map(|reveal| reveal.page_index),
                shell.active_script_cursor.clone(),
                snapshot
                    .visible_object_runtime_tiles
                    .get("PLAYERSHOUSE1F_MOM1")
                    .copied(),
            );
            assert_eq!(
                after, before,
                "Start advanced or escaped Mom's active dialogue instead of being ignored"
            );
            assert_eq!(
                shell.start_menu_cursor, None,
                "Start opened the field menu underneath Mom's active dialogue"
            );
            assert!(
                shell.visible_script_movement.is_none(),
                "Start released Mom's return movement while her dialogue was still active"
            );
            let boundary_before_a = (
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                snapshot.ui.pending_text_wait.clone(),
                shell.active_script_cursor.clone(),
            );
            let _ = shell;
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("Mom post-A snapshot");
            let boundary_after_a = (
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                snapshot.ui.pending_text_wait.clone(),
                shell.active_script_cursor.clone(),
            );
            assert_ne!(
                boundary_after_a, boundary_before_a,
                "A did not immediately advance Mom's fully printed text boundary after Start was ignored"
            );
            start_guard_already_pressed_a = true;
            proved_start_ignored_during_mom_dialogue = true;
        }
        if visible_label.is_some() || pending_yes_no {
            let world = app.world_mut();
            let has_textbox = world
                .query_filtered::<Entity, With<SceneDialogTextBoxBackgroundMarker>>()
                .iter(world)
                .next()
                .is_some();
            let has_glyphs = world
                .query_filtered::<Entity, With<DialogGlyphMarker>>()
                .iter(world)
                .next()
                .is_some();
            saw_rendered_mom_text |= visible_label.is_some() && has_textbox && has_glyphs;
            saw_rendered_yes_no |= pending_yes_no && has_textbox && has_glyphs;
            if pending_day_of_week {
                let custom_sizes = world
                    .query_filtered::<&Sprite, With<SceneDialogTextBoxBackgroundMarker>>()
                    .iter(world)
                    .filter_map(|sprite| sprite.custom_size)
                    .collect::<Vec<_>>();
                saw_canonical_day_selector |=
                    custom_sizes.contains(&Vec2::new(9.0 * TILE_SIZE, 2.0 * TILE_SIZE));
                if pending_day_confirming {
                    assert!(
                        !custom_sizes.contains(&Vec2::new(9.0 * TILE_SIZE, 2.0 * TILE_SIZE)),
                        "day confirmation must replace the selector instead of stacking both windows"
                    );
                }
            }
        }
        if hidden_yes_no_while_text_owns_input {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
            let cursor_after = app
                .world()
                .resource::<BevyRuntimeShell>()
                .yes_no_cursor
                .as_ref()
                .map(|cursor| cursor.option_index);
            assert_eq!(
                cursor_after, hidden_yes_no_cursor_before,
                "directional input changed Mom's hidden yes/no selection before her authored text was consumed"
            );
            proved_hidden_yes_no_ignores_direction = true;
        }
        if pending_day_of_week
            && !pending_day_confirming
            && !proved_weekday_input_is_immediate
        {
            let selected_before = app
                .world()
                .resource::<BevyRuntimeShell>()
                .pending_day_of_week
                .as_ref()
                .expect("weekday selection")
                .selected_day;
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
            let selected_after = app
                .world()
                .resource::<BevyRuntimeShell>()
                .pending_day_of_week
                .as_ref()
                .expect("weekday selection after Down")
                .selected_day;
            assert_ne!(
                selected_after, selected_before,
                "one Down edge must move the weekday selector immediately"
            );
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
            assert!(
                app.world()
                    .resource::<BevyRuntimeShell>()
                    .pending_day_of_week
                    .as_ref()
                    .is_some_and(|prompt| prompt.confirming),
                "one A edge must immediately open weekday YES/NO confirmation"
            );
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowDown);
            assert_eq!(
                app.world()
                    .resource::<BevyRuntimeShell>()
                    .pending_day_of_week
                    .as_ref()
                    .map(|prompt| prompt.yes_no_index),
                Some(1),
                "one Down edge must immediately select NO"
            );
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowUp);
            assert_eq!(
                app.world()
                    .resource::<BevyRuntimeShell>()
                    .pending_day_of_week
                    .as_ref()
                    .map(|prompt| prompt.yes_no_index),
                Some(0),
                "one Up edge must immediately restore YES"
            );
            proved_weekday_input_is_immediate = true;
            start_guard_already_pressed_a = true;
        }
        if !visible_wait_sfx_boundary && !start_guard_already_pressed_a {
            // Schedule the edge for the next loop iteration so that iteration
            // observes the exact ECS frame produced by the press. The generic
            // helper updates internally and used to hide the one complete
            // ReceivedItemText frame from this rendered-output assertion.
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyZ);
            release_scheduled_a_after_observation = true;
        }
    }
    let failed_shell = app.world().resource::<BevyRuntimeShell>();
    let failed_snapshot = failed_shell.shell.snapshot().expect("failed Mom snapshot");
    assert!(
        completed_mom,
        "Mom never completed; labels={seen_labels:?} map={} tile={:?} cursor={:?} action={:?} events={:?}",
        failed_snapshot.overworld.map_name,
        failed_snapshot.overworld.tile,
        failed_shell.active_script_cursor,
        failed_shell.last_runtime_action,
        failed_shell.last_audio_events
    );
    assert!(
        mom_frames < 1024,
        "Mom's complete visible interaction exceeded the real-input frame budget"
    );
    let _ = failed_shell;
    assert_eq!(
        seen_labels.first().map(String::as_str),
        Some("ElmsLookingForYouText"),
        "Mom must begin with her canonical dialogue, not a script/pre-text label"
    );
    let canonical_mom_labels = [
        "ElmsLookingForYouText",
        "ReceivedItemText",
        "MomGivesPokegearText",
        "IsItDSTText",
        "ComeHomeForDSTText",
        "KnowTheInstructionsText",
        "InstructionsNextText",
    ];
    assert_eq!(
        seen_labels.iter().map(String::as_str).collect::<Vec<_>>(),
        canonical_mom_labels,
        "Mom must render every ASM-authored dialogue label exactly once and in order"
    );
    assert_eq!(
        dialogue_activations
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        canonical_mom_labels,
        "Mom must not reactivate a completed text body while entering its prompt boundary"
    );
    assert_eq!(
        dialogue_pages,
        vec![
            ("ElmsLookingForYouText".to_string(), 0),
            ("ElmsLookingForYouText".to_string(), 1),
            ("ElmsLookingForYouText".to_string(), 2),
            ("ElmsLookingForYouText".to_string(), 3),
            ("ElmsLookingForYouText".to_string(), 4),
            ("ElmsLookingForYouText".to_string(), 5),
            ("ElmsLookingForYouText".to_string(), 6),
            ("ReceivedItemText".to_string(), 0),
            ("MomGivesPokegearText".to_string(), 0),
            ("MomGivesPokegearText".to_string(), 1),
            ("MomGivesPokegearText".to_string(), 2),
            ("MomGivesPokegearText".to_string(), 3),
            ("MomGivesPokegearText".to_string(), 4),
            ("IsItDSTText".to_string(), 0),
            ("ComeHomeForDSTText".to_string(), 0),
            ("ComeHomeForDSTText".to_string(), 1),
            ("ComeHomeForDSTText".to_string(), 2),
            ("ComeHomeForDSTText".to_string(), 3),
            ("KnowTheInstructionsText".to_string(), 0),
            ("KnowTheInstructionsText".to_string(), 1),
            ("InstructionsNextText".to_string(), 0),
            ("InstructionsNextText".to_string(), 1),
            ("InstructionsNextText".to_string(), 2),
        ],
        "every one of Mom's 23 ASM-authored pages must become visible exactly once"
    );
    assert_eq!(
        rendered_dialogue_pages, dialogue_pages,
        "every semantic Mom page must also appear once as the exact bitmap-glyph sprite sequence sent to Bevy's renderer"
    );
    let dialogue_show_events = failed_shell
        .dialogue_log_events
        .iter()
        .filter(|event| event.contains("event=show"))
        .collect::<Vec<_>>();
    assert_eq!(
        dialogue_show_events.len(),
        23,
        "runtime dialogue logging must record each visible Mom page exactly once: {dialogue_show_events:#?}"
    );
    assert_eq!(
        dialogue_show_events
            .iter()
            .filter(|event| event.contains("label=ReceivedItemText"))
            .count(),
        1,
        "the Pokegear reward must produce one timed dialogue log entry"
    );
    assert!(
        dialogue_show_events.iter().all(|event| event.contains("frame=")
            && event.contains("text=")
            && event.contains("script=")),
        "every dialogue log must identify when it appeared, what was said, and which script owned it"
    );
    assert!(
        failed_shell.input_log_events.iter().any(|event| {
            event.contains("key=A")
                && event.contains("owner=dialogue")
                && event.contains("dialogue=ElmsLookingForYouText")
        }),
        "Mom's visible page advances must record their physical A edges"
    );
    assert!(
        failed_shell
            .input_log_events
            .iter()
            .any(|event| event.contains("key=DOWN") && event.contains("owner=weekday")),
        "weekday navigation must record its physical direction edge and modal owner"
    );
    assert_eq!(
        yes_no_boundaries.len(),
        3,
        "Mom's ASM path has exactly three YesNoBox boundaries; saw {yes_no_boundaries:?}"
    );
    assert!(
        saw_authored_mom_return,
        "Mom never performed her ASM-authored return movement after the phone dialogue closed"
    );
    let mom_movement_events = failed_shell
        .movement_log_events
        .iter()
        .filter(|event| event.contains("object=PLAYERSHOUSE1F_MOM1"))
        .collect::<Vec<_>>();
    assert!(
        !mom_movement_events.is_empty(),
        "Mom's approach and return movements must produce timed movement diagnostics"
    );
    assert_eq!(
        mom_movement_events
            .iter()
            .filter(|event| {
                event.contains("event=start")
                    && event.contains("movement=MomWalksBackMovement")
            })
            .count(),
        1,
        "Mom's authored walk-away movement must start exactly once: {mom_movement_events:#?}"
    );
    assert!(
        mom_movement_events
            .iter()
            .all(|event| event.contains("frame=") && event.contains("dialogue=none")),
        "Mom moved while a dialogue label still owned the rendered textbox: {mom_movement_events:#?}"
    );
    let final_dialogue_close = failed_shell
        .dialogue_log_events
        .iter()
        .find(|event| {
            event.contains("event=close") && event.contains("label=InstructionsNextText")
        })
        .expect("final Mom dialogue must log its visible close");
    let mom_departure = mom_movement_events
        .iter()
        .find(|event| event.contains("event=start") && event.contains("movement=MomWalksBackMovement"))
        .expect("Mom's walk away must have an explicit start log");
    let event_frame = |event: &str| {
        event
            .split_whitespace()
            .find_map(|field| field.strip_prefix("frame="))
            .expect("timed event must contain a frame")
            .parse::<u64>()
            .expect("timed event frame must be numeric")
    };
    assert!(
        event_frame(final_dialogue_close) <= event_frame(mom_departure),
        "Mom visibly started walking away before the final textbox close was logged: close={final_dialogue_close:?} departure={mom_departure:?}"
    );
    assert!(
        mom_departure.contains("dialogue=none"),
        "Mom's departure frame still rendered dialogue: {mom_departure}"
    );
    assert!(
        rendered_mom_departure_x
            .windows(2)
            .any(|positions| positions[1] < positions[0] - f32::EPSILON),
        "Mom's rendered sprite never visibly traveled left during her departure: {rendered_mom_departure_x:?}"
    );
    assert!(
        rendered_mom_departure_x
            .windows(2)
            .all(|positions| positions[1] <= positions[0] + f32::EPSILON),
        "Mom's rendered departure snapped right and replayed the walk-away: {rendered_mom_departure_x:?}"
    );
    assert!(
        saw_exact_received_pokegear_text,
        "Mom never rendered the fully expanded Pokegear reward text"
    );
    assert!(
        saw_item_reward_sound,
        "ReceiveItemScript never queued the ASM SFX_ITEM reward fanfare"
    );
    assert!(
        saw_reward_waitsfx_complete_without_input,
        "ReceiveItemScript's waitsfx never resumed after SFX_ITEM completed unless the player supplied unrelated input"
    );
    let final_live_entities = app.world().entities().len();
    assert!(
        peak_live_entities < 1024 && final_live_entities <= baseline_live_entities + 32,
        "Mom dialogue leaked live render entities; baseline={baseline_live_entities} peak={peak_live_entities} final={final_live_entities}"
    );
    assert!(
        saw_canonical_day_selector,
        "SetDayOfWeek must render its separate canonical 11x4 selector window"
    );
    assert!(
        saw_dynamic_dst_confirmation,
        "DST setup must visibly render the live HH:MM confirmation before its yes/no; last_special={:?} reveal={:?}",
        failed_snapshot.script_events.last_special_routine,
        (
            failed_shell.field_text_reveal.as_ref(),
            yes_no_render_trace,
            longest_dst_confirmation
        )
    );
    assert!(
        saw_rendered_mom_text,
        "Mom executed text without rendering its textbox and bitmap glyphs"
    );
    assert!(
        saw_rendered_yes_no,
        "Mom's yes/no executed without rendering an interactive prompt"
    );
    assert!(
        proved_hidden_yes_no_ignores_direction,
        "Mom's test never exercised direction input while yesorno was hidden behind authored text"
    );
    assert!(
        proved_start_ignored_during_mom_dialogue,
        "Mom's test never proved that Start is ignored while dialogue owns the scene"
    );
    assert!(
        proved_completed_page_does_not_auto_advance,
        "Mom's test never proved that a fully printed nonfinal page remains blocked without A/B"
    );
    assert!(
        proved_weekday_input_is_immediate,
        "Mom's test never exercised immediate weekday and YES/NO input"
    );
    assert!(
        saw_yes_no_prompt_cleared,
        "Mom's resolved YesNoBox survived into the following script frame"
    );
    assert!(
        saw_yes_no_frame_cleared,
        "Mom's resolved YesNoBox left its window frame rendered over the overworld"
    );
    let direction_still_captured = {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        has_visible_shell_direction_action(&mut shell)
    };
    assert!(
        !direction_still_captured,
        "Mom finished but a modal/script surface still captures directional input"
    );
    assert_overworld_control_returns_and_player_moves(&mut app, "MeetMomScript");

    // Derive the front-door route from the compiled collision map, then
    // replay every tile through the production Bevy keyboard path.
    let exit_path = {
        let shell = app.world().resource::<BevyRuntimeShell>();
        let start = shell.shell.session().overworld.clone();
        let mut queue = std::collections::VecDeque::from([(start.clone(), Vec::new())]);
        let mut visited =
            std::collections::BTreeSet::from([(start.player.tile.x, start.player.tile.y)]);
        let mut found = None;
        while let Some((session, path)) = queue.pop_front() {
            for direction in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                let mut next = session.clone();
                let Ok(mut step) = next.step_and_check_warp_checked(
                    direction,
                    crate::core::world::movement::StepOptions::default(),
                ) else {
                    continue;
                };
                if matches!(
                    step.outcome,
                    crate::core::world::movement::StepOutcome::Turned { .. }
                ) {
                    let Ok(second) = next.step_and_check_warp_checked(
                        direction,
                        crate::core::world::movement::StepOptions::default(),
                    ) else {
                        continue;
                    };
                    step = second;
                }
                if !matches!(
                    step.outcome,
                    crate::core::world::movement::StepOutcome::Moved { .. }
                ) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(direction);
                if let Some(warp) = step.warp.as_ref() {
                    if warp.warp.target_map == "NEW_BARK_TOWN"
                        || warp.warp.target_map == "NewBarkTown"
                    {
                        found = Some(next_path);
                        break;
                    }
                    continue;
                }
                if visited.insert((next.player.tile.x, next.player.tile.y)) {
                    queue.push_back((next, next_path));
                }
            }
            if found.is_some() {
                break;
            }
        }
        found.expect("compiled Player's House collision must reach its front-door warp")
    };
    for direction in exit_path {
        let key = match direction {
            Direction::Up => KeyCode::ArrowUp,
            Direction::Down => KeyCode::ArrowDown,
            Direction::Left => KeyCode::ArrowLeft,
            Direction::Right => KeyCode::ArrowRight,
        };
        let before = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap();
        for _ in 0..2 {
            press_key_for_runtime_hotkey_app(&mut app, key);
            for _ in 0..9 {
                app.update();
            }
            let after = app
                .world()
                .resource::<BevyRuntimeShell>()
                .shell
                .snapshot()
                .unwrap();
            if after.overworld.map_name != before.overworld.map_name
                || after.overworld.tile != before.overworld.tile
            {
                break;
            }
        }
    }
    for _ in 0..256 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name
            == "NewBarkTown"
        {
            break;
        }
    }
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("house-exit snapshot");
    assert_eq!(shell.last_error, None);
    assert_eq!(
        snapshot.overworld.map_name,
        "NewBarkTown",
        "Mom completed but live movement still could not exit the house; tile={:?} pending_reason={:?} objects={:?} script_locks={:?} locks={:?} events={:?}",
        snapshot.overworld.tile,
        shell.shell.pending_script_work_reason(),
        (
            &shell.shell.session().overworld.object_runtime_tiles,
            &shell.shell.session().overworld.object_last_runtime_tiles
        ),
        (
            shell
                .shell
                .session()
                .state()
                .script_runtime
                .player_input_locked,
            shell
                .shell
                .session()
                .state()
                .script_runtime
                .all_input_locked,
            shell
                .shell
                .session()
                .state()
                .script_runtime
                .script_stop_requested
        ),
        (
            &shell.field_text_reveal,
            shell.visible_script_delay_frames,
            &shell.visible_walk_warp_phase,
            &shell.pending_overworld_step_boundary,
            &shell.visible_script_movement,
            &shell.visible_overworld_emote
        ),
        (
            shell.last_runtime_action.clone(),
            shell.last_overworld_input.clone(),
            shell.recent_overworld_inputs.clone(),
            shell.last_audio_events.clone()
        )
    );
    let _ = shell;

    // Re-enter immediately after Mom's introduction. Crystal replaces the
    // moving intro object with the ordinary Mom object on this fresh map load.
    for _ in 0..96 {
        let current_map = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name;
        if current_map == "PlayersHouse1F" {
            break;
        }
        let path = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            collision_path_to_map_warp(&shell.shell.session().overworld, "PLAYERS_HOUSE_1F")
        };
        press_visible_direction_until_tile_changes(&mut app, path[0]);
        settle_visible_story_boundary(&mut app);
    }
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("house re-entry snapshot");
    assert_eq!(snapshot.overworld.map_name, "PlayersHouse1F");
    let visible_mom = snapshot
        .visible_objects
        .iter()
        .find(|object| object.script == "MomScript")
        .and_then(|object| object.object_identifier.as_deref());
    assert!(
        visible_mom.is_some(),
        "Mom's completed-story object must load on re-entry; flags={:?} objects={:?}",
        snapshot.progression.active_event_flags,
        shell.shell.session().overworld.objects
    );
    let visible_mom = visible_mom.unwrap();
    assert_eq!(
        app.world()
            .iter_entities()
            .filter_map(|entity| entity.get::<VisibleObjectSprite>())
            .filter(|sprite| sprite.object_identifier.as_deref() == Some(visible_mom))
            .count(),
        1,
        "Mom's completed-story object must have exactly one rendered sprite"
    );
    let _ = shell;

    for _ in 0..96 {
        let current_map = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name;
        if current_map == "NewBarkTown" {
            break;
        }
        let path = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            collision_path_to_map_warp(&shell.shell.session().overworld, "NEW_BARK_TOWN")
        };
        press_visible_direction_until_tile_changes(&mut app, path[0]);
        settle_visible_story_boundary(&mut app);
    }
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name,
        "NewBarkTown",
        "live keyboard route did not exit after verifying Mom's re-entry object"
    );

    let mut saw_route_text = false;
    for _ in 0..96 {
        let current_map = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name;
        if current_map == "ElmsLab" {
            break;
        }
        let path = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            collision_path_to_map_warp(&shell.shell.session().overworld, "ELMS_LAB")
        };
        press_visible_direction_until_tile_changes(&mut app, path[0]);
        saw_route_text |= settle_visible_story_boundary(&mut app).0;
    }
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .map_name,
        "ElmsLab",
        "live keyboard route never reached Elm's lab"
    );

    for _ in 0..64 {
        let tile = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .tile;
        if tile == (TilePosition { x: 5, y: 3 }) {
            break;
        }
        let path = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            collision_path_to_tile(
                &shell.shell.session().overworld,
                TilePosition { x: 5, y: 3 },
            )
        };
        press_visible_direction_until_tile_changes(&mut app, path[0]);
        saw_route_text |= settle_visible_story_boundary(&mut app).0;
    }
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .unwrap()
            .overworld
            .tile,
        TilePosition { x: 5, y: 3 },
        "live keyboard route never reached the Cyndaquil ball"
    );
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    let (starter_text_rendered, starter_picture_rendered) = settle_visible_story_boundary(&mut app);
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("post-starter snapshot");
    assert_eq!(shell.last_error, None);
    assert!(
        saw_route_text,
        "the New Bark/Elm story executed without rendered dialogue"
    );
    assert!(
        starter_text_rendered,
        "the starter script executed without rendered text glyphs"
    );
    assert!(
        starter_picture_rendered,
        "the starter script executed without rendering Cyndaquil's picture"
    );
    assert_eq!(
        snapshot.party.slots.len(),
        1,
        "starter flow stopped before grant: cursor={:?} name_choice={:?} name_input={:?} pending_gift={:?} text={:?} waits={:?} action={:?}",
        shell.active_script_cursor,
        shell.pending_name_choice,
        shell.pending_name_input,
        shell.pending_gift_pokemon_nickname,
        snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
        snapshot.ui.pending_text_wait,
        shell.last_runtime_action,
    );
    assert_eq!(snapshot.party.slots[0].pokemon.species.id, "CYNDAQUIL");
    assert!(
        snapshot
            .progression
            .active_event_flags
            .contains("EVENT_GOT_CYNDAQUIL_FROM_ELM")
    );
    assert!(
        snapshot
            .progression
            .active_event_flags
            .contains("EVENT_GOT_A_POKEMON_FROM_ELM")
    );
    assert_eq!(
        shell.active_script_cursor, None,
        "starter script did not finish"
    );
}

#[test]
fn givepoke_enters_nickname_prompt_without_an_invented_gift_selection() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab");
    let snapshot = runtime_shell.shell.snapshot().expect("Elm's Lab snapshot");
    let starter = snapshot
        .ui
        .gift_pokemon
        .iter()
        .find(|gift| gift.species_id == "CYNDAQUIL")
        .expect("Cyndaquil gift command");
    let source_script = starter.source_script.clone();
    let command_index = starter.command_index;
    runtime_shell
        .shell
        .set_trainer_identity("CHRIS".to_string(), 1)
        .expect("set trainer identity");
    arm_visible_active_script_cursor(&mut runtime_shell, &source_script, command_index);
    execute_visible_active_script_step(&mut runtime_shell).expect("execute givepoke Cyndaquil");

    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("nickname prompt snapshot");
    assert_eq!(
        snapshot.party.slots.len(),
        1,
        "GivePoke must store the starter before asking for its nickname"
    );
    assert_eq!(
        runtime_shell
            .pending_name_choice
            .as_ref()
            .map(|choice| choice.options.clone()),
        Some(vec!["YES".to_string(), "NO".to_string()])
    );
    assert_eq!(
        runtime_shell
            .pending_gift_pokemon_nickname
            .as_ref()
            .map(|pending| pending.default_name.as_str()),
        Some("CYNDAQUIL")
    );

    confirm_visible_name_choice(&mut runtime_shell).expect("accept nickname prompt");
    let name_input = runtime_shell
        .pending_name_input
        .as_mut()
        .expect("starter naming screen");
    assert_eq!(
        name_input.value, "",
        "the ASM naming screen clears the nickname destination instead of prefilling the species"
    );
    assert_eq!(name_input.label, "CYNDAQUIL'S\nNICKNAME?");
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    {
        let world = app.world_mut();
        let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
        assert_eq!(presenters.iter(world).count(), 1, "starter naming LCD did not render");
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .pending_name_input
            .as_ref()
            .map(|input| input.value.as_str()),
        Some("AA"),
        "the live naming screen must accept the shown two-letter nickname"
    );
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    {
        let runtime_shell = app.world().resource::<BevyRuntimeShell>();
        let input = runtime_shell
            .pending_name_input
            .as_ref()
            .expect("starter naming screen after Start");
        assert_eq!(
            (input.cursor_column, input.cursor_row),
            (8, visible_name_input_bottom_row_index()),
            "Start must move the cursor to END"
        );
    }
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
    app.update();
    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("named starter snapshot");
    assert_eq!(snapshot.party.slots.len(), 1);
    assert_eq!(snapshot.party.slots[0].pokemon.nickname, "AA");
    assert!(runtime_shell.pending_gift_pokemon_nickname.is_none());
    {
        let world = app.world_mut();
        let mut presenters = world.query_filtered::<Entity, With<VisibleIntroSurface>>();
        assert_eq!(
            presenters.iter(world).count(),
            0,
            "the naming LCD remained over the script after nickname confirmation"
        );
    }
}

#[test]
fn declining_starter_nickname_resumes_with_species_name_and_clears_prompt() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab");
    runtime_shell
        .shell
        .set_trainer_identity("CHRIS".to_string(), 1)
        .expect("set trainer identity");
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let mut app = integrated_shell_test_app(runtime_shell);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowRight);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    settle_visible_story_boundary(&mut app);

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("declined nickname snapshot");
    assert_eq!(snapshot.party.slots[0].pokemon.nickname, "CYNDAQUIL");
    assert_eq!(
        snapshot.script_events.named_buffers.get("STRING_BUFFER_3").map(String::as_str),
        Some("CYNDAQUIL"),
        "getmonname did not populate the authoritative starter-name buffer"
    );
    assert!(runtime_shell.dialogue_log_events.iter().any(|event| {
        event.contains("label=ReceivedStarterText") && event.contains("CYNDAQUIL")
    }), "the received-starter page omitted CYNDAQUIL: {:?}", runtime_shell.dialogue_log_events);
    assert!(runtime_shell.pending_name_choice.is_none());
    assert!(runtime_shell.pending_gift_pokemon_nickname.is_none());
    let _ = runtime_shell;
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<SceneDialogWindowFrameMarker>>()
            .iter(world)
            .count(),
        0,
        "the declined nickname prompt left a window frame over the field"
    );
}

#[test]
fn boxed_givepoke_prints_bills_pc_notice_after_naming_before_script_continuation() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab");
    runtime_shell
        .shell
        .set_trainer_identity("CHRIS".to_string(), 1)
        .expect("set trainer identity");
    let snapshot = runtime_shell.shell.snapshot().expect("Elm's Lab snapshot");
    let starter = snapshot
        .ui
        .gift_pokemon
        .iter()
        .find(|gift| gift.species_id == "CYNDAQUIL")
        .expect("Cyndaquil gift command");
    let source_script = starter.source_script.clone();
    let command_index = starter.command_index;
    let seed = runtime_shell
        .shell
        .grant_compiled_gift_pokemon_command(
            &source_script,
            command_index,
            "CHRIS",
            1,
            false,
            None,
        )
        .expect("seed party Pokemon")
        .outcome
        .pokemon;
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        while state.storage.party.add_pokemon(seed.clone()) {}
        state.sync_party_from_storage();
    }

    arm_visible_active_script_cursor(&mut runtime_shell, &source_script, command_index);
    execute_visible_active_script_step(&mut runtime_shell).expect("execute boxed givepoke");
    assert!(matches!(
        runtime_shell
            .pending_gift_pokemon_nickname
            .as_ref()
            .map(|pending| &pending.location),
        Some(crate::core::models::CaptureStorageLocation::Pc {
            box_index: 0,
            slot: 0
        })
    ));

    confirm_visible_name_choice(&mut runtime_shell).expect("accept boxed nickname prompt");
    runtime_shell
        .pending_name_input
        .as_mut()
        .expect("boxed naming screen")
        .value = "EMBER".to_string();
    confirm_visible_player_name_input(&mut runtime_shell).expect("commit boxed nickname");

    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("EMBER was\nsent to BILL's PC.")
    );
    let box_zero = &runtime_shell.shell.session().state().storage.pc_boxes[0];
    assert_eq!(
        box_zero.pokemon[0].as_ref().map(|pokemon| pokemon.nickname.as_str()),
        Some("EMBER")
    );
    assert_eq!(box_zero.nicknames[0], "EMBER");
    assert!(runtime_shell.active_script_cursor.is_some());
}

#[test]
fn custom_givepoke_uses_authored_identity_and_randomizes_boxed_ot_id() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route35GoldenrodGate".to_string(),
            tile_x: 4,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route 35 gate");
    runtime_shell
        .shell
        .set_trainer_identity("CHRIS".to_string(), 1)
        .expect("set trainer identity");
    let snapshot = runtime_shell.shell.snapshot().expect("Route 35 gate snapshot");
    let kenya = snapshot
        .ui
        .gift_pokemon
        .iter()
        .find(|gift| gift.species_id == "SPEAROW" && gift.source_script == "RandyScript")
        .expect("Kenya gift command");
    let source_script = kenya.source_script.clone();
    let command_index = kenya.command_index;
    let seeded = runtime_shell
        .shell
        .grant_compiled_gift_pokemon_command(
            &source_script,
            command_index,
            "CHRIS",
            1,
            false,
            None,
        )
        .expect("grant party Kenya")
        .outcome
        .pokemon;
    assert_eq!(seeded.nickname, "KENYA");
    assert_eq!(seeded.original_trainer_name, "RANDY");
    assert_eq!(seeded.original_trainer_id, 1001);
    let caught = seeded
        .caught_data
        .as_ref()
        .expect("custom gift caught data");
    assert_eq!(caught.level, 0);
    assert_eq!(caught.time_of_day, None);
    assert_eq!(caught.original_trainer_gender, 0);
    assert_eq!(caught.location, 0x7e);

    {
        let session = runtime_shell.shell.session_mut();
        while session.state_mut().storage.party.add_pokemon(seeded.clone()) {}
        session.state_mut().random_state = Default::default();
        session.state_mut().sync_party_from_storage();
        session.divider = crate::core::random::RuntimeDividerSource::replay([
            0, 0, 0, 0, 0, 0x20, 0, 1,
        ]);
    }

    arm_visible_active_script_cursor(&mut runtime_shell, &source_script, command_index);
    execute_visible_active_script_step(&mut runtime_shell).expect("execute boxed Kenya givepoke");

    assert!(runtime_shell.pending_name_choice.is_none());
    assert!(runtime_shell.pending_gift_pokemon_nickname.is_none());
    assert_eq!(
        runtime_shell.field_notice.as_deref(),
        Some("KENYA was\nsent to BILL's PC.")
    );
    let boxed = runtime_shell.shell.session().state().storage.pc_boxes[0].pokemon[0]
        .as_ref()
        .expect("boxed Kenya");
    assert_eq!(boxed.nickname, "KENYA");
    assert_eq!(boxed.original_trainer_name, "RANDY");
    assert_eq!(boxed.original_trainer_id, 0xe0df);
    assert!(runtime_shell.active_script_cursor.is_some());

    runtime_shell.field_notice = None;
    finish_visible_gift_pokemon_pc_notice(&mut runtime_shell)
        .expect("dismiss boxed Kenya notice and attach its authored mail");
    let mailed = runtime_shell.shell.session().state().storage.party.pokemon[5]
        .as_ref()
        .expect("last party Pokemon receives Kenya mail");
    assert_eq!(
        mailed.mail.as_ref().map(|mail| mail.message.as_str()),
        Some("DARK CAVE leads\nto another road")
    );
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_scene = None;
    runtime_shell.field_notice_queue.clear();
    {
        let session = runtime_shell.shell.session_mut();
        let current_box = &mut session.state_mut().storage.pc_boxes[0];
        while current_box.add_pokemon(seeded.clone()) {}
        session.state_mut().random_state = Default::default();
        session.divider = crate::core::random::RuntimeDividerSource::replay([0, 0, 0, 0]);
    }
    let full = runtime_shell
        .shell
        .grant_compiled_gift_pokemon_command(
            &source_script,
            command_index,
            "CHRIS",
            1,
            false,
            None,
        )
        .expect("full-storage custom givepoke consumes only its two DV Random calls");
    assert_eq!(full.outcome.script_value, 2);
    assert_eq!(full.outcome.location, None);
}

#[test]
fn givepoke_full_party_and_current_box_returns_two_without_a_nickname_prompt() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab");
    runtime_shell
        .shell
        .set_trainer_identity("CHRIS".to_string(), 1)
        .expect("set trainer identity");
    let snapshot = runtime_shell.shell.snapshot().expect("Elm's Lab snapshot");
    let starter = snapshot
        .ui
        .gift_pokemon
        .iter()
        .find(|gift| gift.species_id == "CYNDAQUIL")
        .expect("Cyndaquil gift command");
    let source_script = starter.source_script.clone();
    let command_index = starter.command_index;
    let seed = runtime_shell
        .shell
        .grant_compiled_gift_pokemon_command(
            &source_script,
            command_index,
            "CHRIS",
            1,
            false,
            None,
        )
        .expect("seed stored Pokemon")
        .outcome
        .pokemon;
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        while state.storage.party.add_pokemon(seed.clone()) {}
        if state.storage.pc_boxes.is_empty() {
            state
                .storage
                .pc_boxes
                .push(crate::core::models::PcBox::new(state.current_pc_box));
        }
        let current_box = &mut state.storage.pc_boxes[state.current_pc_box];
        while current_box.add_pokemon(seed.clone()) {}
        state.sync_party_from_storage();
    }

    arm_visible_active_script_cursor(&mut runtime_shell, &source_script, command_index);
    execute_visible_active_script_step(&mut runtime_shell).expect("execute full-storage givepoke");

    assert!(runtime_shell.pending_name_choice.is_none());
    assert!(runtime_shell.pending_gift_pokemon_nickname.is_none());
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("script_value: 2")),
        "full-storage GivePoke never returned its ASM wScriptVar value: {:?}",
        runtime_shell.last_audio_events
    );
}

#[test]
fn mr_pokemon_visit_prints_every_asm_page_once_then_arms_the_rival_story() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "MrPokemonsHouse".to_string(),
            tile_x: 2,
            tile_y: 6,
        },
        BevyShellConfig {
            smoke_player_name: Some("CHRIS".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize Mr. Pokemon house");
    runtime_shell
        .shell
        .set_script_flag_for_smoke("EVENT_GOT_CYNDAQUIL_FROM_ELM")
        .expect("set Elm starter branch");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            5,
            None,
            None,
            "CHRIS",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add Elm starter");
    {
        let starter = runtime_shell.shell.session_mut().state_mut().storage.party.pokemon[0]
            .as_mut()
            .expect("starter storage slot");
        starter.hp = 1;
    }
    if runtime_shell
        .shell
        .script_events_snapshot()
        .script_ended
        .is_some()
    {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    // A direct runtime-tile test start does not perform a doorway map-entry
    // transaction. Arm the exact target that the scene's `sdefer` schedules.
    arm_visible_active_script_cursor(&mut runtime_shell, "MrPokemonsHouseMrPokemonEventScript", 0);
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let mut dialogue_pages = Vec::new();
    let mut completed_labels = std::collections::BTreeSet::new();
    let mut previous_label = None;
    let mut completed = false;
    let mut movement_activations = Vec::new();
    let mut previous_movement = None;
    let mut observed_delay_frames = std::collections::BTreeSet::new();
    let mut saw_key_item_sound = false;
    let mut saw_pokedex_item_sound = false;
    let mut saw_exit_sound = false;
    let mut saw_oak_music = false;
    let mut saw_heal_music = false;
    let mut saw_waitsfx = false;
    let mut proved_oak_page_does_not_auto_advance = false;
    let mut proved_waitbutton_advances_with_one_a = false;
    for frame in 0..4096 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            shell.last_error, None,
            "Mr. Pokemon scene failed at frame {frame}: {:?}",
            shell.last_audio_events
        );
        let snapshot = shell.shell.snapshot().expect("Mr. Pokemon snapshot");
        let movement = shell
            .visible_script_movement
            .as_ref()
            .map(|movement| movement.object_id.clone());
        if movement != previous_movement {
            if let Some(object_id) = movement.clone() {
                movement_activations.push(object_id);
            }
            previous_movement = movement;
        }
        if let Some(delay) = shell.visible_script_delay_frames {
            observed_delay_frames.insert(delay);
        }
        saw_key_item_sound |= shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("SFX_KEY_ITEM"));
        saw_pokedex_item_sound |= shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("SFX_ITEM"));
        saw_exit_sound |= shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("SFX_EXIT_BUILDING"));
        saw_oak_music |= shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("MUSIC_PROF_OAK"));
        saw_heal_music |= shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("MUSIC_HEAL"));
        saw_waitsfx |= shell.visible_wait_sfx_boundary;
        let visible_label = snapshot.ui.text.as_ref().map(|text| text.label.clone());
        if visible_label != previous_label {
            if let Some(previous) = previous_label.take() {
                completed_labels.insert(previous);
            }
            if let Some(label) = visible_label.clone() {
                assert!(
                    !completed_labels.contains(&label),
                    "Mr. Pokemon reactivated completed text {label}; pages={dialogue_pages:?} cursor={:?}",
                    shell.active_script_cursor
                );
            }
            previous_label = visible_label.clone();
        }
        if let (Some(label), Some(reveal), Some(pages)) = (
            visible_label,
            shell.field_text_reveal.as_ref(),
            visible_field_dialog_pages(&snapshot, shell),
        ) && reveal.text == pages.join("\u{1e}")
        {
            if reveal.page_index > 0 {
                let carried_chars = visible_field_page_initial_chars(
                    &pages[reveal.page_index - 1],
                    &pages[reveal.page_index],
                );
                assert!(
                    reveal.visible_chars >= carried_chars,
                    "Mr. Pokemon retyped an ASM <CONT> carry: label={label} page={} visible={} carried={carried_chars}",
                    reveal.page_index,
                    reveal.visible_chars,
                );
            }
            let page = (label, reveal.page_index);
            if dialogue_pages.last() != Some(&page) {
                dialogue_pages.push(page);
            }
        }
        let got_egg = snapshot
            .progression
            .active_event_flags
            .contains("EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON");
        let rival_armed = snapshot
            .progression
            .active_event_flags
            .contains("EVENT_RIVAL_NEW_BARK_TOWN");
        let busy = shell.active_script_cursor.is_some()
            || shell.visible_script_movement.is_some()
            || shell.visible_overworld_emote.is_some()
            || shell.visible_script_delay_frames.is_some()
            || shell.shell.has_pending_script_work()
            || snapshot.ui.text_window_open;
        completed = got_egg && rival_armed && !busy;
        if completed {
            assert!(
                snapshot
                    .progression
                    .active_engine_flags
                    .contains("ENGINE_POKEDEX"),
                "Oak left without granting the Pokedex"
            );
            assert_eq!(
                shell
                    .shell
                    .current_scene_script()
                    .expect("Mr. Pokemon scene")
                    .map(|scene| scene.scene_id)
                    .as_deref(),
                Some("SCENE_MRPOKEMONSHOUSE_NOOP")
            );
            break;
        }
        let hold_oak_page = !proved_oak_page_does_not_auto_advance
            && snapshot
                .ui
                .text
                .as_ref()
                .is_some_and(|text| text.label == "MrPokemonsHouse_OakText1")
            && visible_field_dialogue_is_fully_revealed(shell, &snapshot)
            && shell
                .field_text_reveal
                .as_ref()
                .zip(visible_field_dialog_pages(&snapshot, shell).as_ref())
                .is_some_and(|(reveal, pages)| reveal.page_index + 1 < pages.len());
        let completed_text_wait_before = (!proved_waitbutton_advances_with_one_a
            && snapshot.ui.pending_text_wait.is_some()
            && visible_field_dialogue_is_entirely_consumed(shell, &snapshot)
            && !shell.visible_wait_sfx_boundary)
            .then(|| {
                (
                    snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                    snapshot.ui.pending_text_wait.clone(),
                    shell.active_script_cursor.clone(),
                )
            });
        let wait_sfx = shell.visible_wait_sfx_boundary;
        let _ = shell;
        if hold_oak_page {
            let before = {
                let shell = app.world().resource::<BevyRuntimeShell>();
                let snapshot = shell.shell.snapshot().expect("Oak idle-page snapshot");
                (
                    snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                    shell.field_text_reveal.as_ref().map(|reveal| reveal.page_index),
                    shell.active_script_cursor.clone(),
                    snapshot.visible_object_runtime_tiles.clone(),
                )
            };
            for _ in 0..30 {
                app.update();
            }
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("Oak post-idle snapshot");
            let after = (
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                shell.field_text_reveal.as_ref().map(|reveal| reveal.page_index),
                shell.active_script_cursor.clone(),
                snapshot.visible_object_runtime_tiles.clone(),
            );
            assert_eq!(
                after, before,
                "Oak's fully printed nonfinal page advanced or moved an actor without input"
            );
            proved_oak_page_does_not_auto_advance = true;
            continue;
        }
        if wait_sfx {
            continue;
        }
        if busy {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
            if let Some(before) = completed_text_wait_before {
                let shell = app.world().resource::<BevyRuntimeShell>();
                let snapshot = shell.shell.snapshot().expect("Mr. Pokemon post-A snapshot");
                let after = (
                    snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                    snapshot.ui.pending_text_wait.clone(),
                    shell.active_script_cursor.clone(),
                );
                assert_ne!(
                    after, before,
                    "one A press did not immediately advance Mr. Pokemon/Oak's completed ASM text wait"
                );
                proved_waitbutton_advances_with_one_a = true;
            }
        }
    }
    assert!(
        completed,
        "Mr. Pokemon's complete authored scene did not settle"
    );

    let expected_page_counts = [
        ("MrPokemonIntroText1", 2usize),
        ("MrPokemonIntroText2", 2),
        ("MrPokemonsHouse_GotEggText", 1),
        ("MrPokemonIntroText3", 7),
        ("MrPokemonIntroText4", 1),
        ("MrPokemonIntroText5", 2),
        ("MrPokemonsHouse_OakText1", 23),
        ("MrPokemonsHouse_GetDexText", 1),
        ("MrPokemonsHouse_OakText2", 6),
        ("MrPokemonsHouse_MrPokemonHealText", 3),
        ("MrPokemonText_ImDependingOnYou", 1),
    ];
    let expected_pages = expected_page_counts
        .into_iter()
        .flat_map(|(label, count)| (0..count).map(move |page| (label.to_string(), page)))
        .collect::<Vec<_>>();
    assert_eq!(
        dialogue_pages, expected_pages,
        "Mr. Pokemon must print the exact 49-page ASM transcript once and in order"
    );

    assert!(
        movement_activations.iter().any(|object| object == "PLAYER"),
        "the player never performed MrPokemonsHouse_PlayerWalksToMrPokemon"
    );
    assert!(
        movement_activations
            .iter()
            .filter(|object| object.as_str() == "MRPOKEMONSHOUSE_OAK")
            .count()
            >= 2,
        "Oak must perform separate approach and exit movements: {movement_activations:?}"
    );
    assert!(saw_key_item_sound, "the Mystery Egg did not play SFX_KEY_ITEM");
    assert!(saw_pokedex_item_sound, "Oak's Pokedex did not play SFX_ITEM");
    assert!(saw_exit_sound, "Oak's departure did not play SFX_EXIT_BUILDING");
    assert!(saw_oak_music, "Oak's entrance did not play MUSIC_PROF_OAK");
    assert!(saw_heal_music, "Mr. Pokemon's healing sequence did not play MUSIC_HEAL");
    assert!(saw_waitsfx, "the scene never exposed its authored waitsfx boundaries");
    assert!(
        observed_delay_frames.contains(&30) && observed_delay_frames.contains(&120),
        "the visible shell did not execute ASM pause 15 and pause 60 as their two-frame wrapping counters: {observed_delay_frames:?}"
    );
    assert!(
        proved_oak_page_does_not_auto_advance,
        "the audit never held a complete Oak page without input"
    );
    assert!(
        proved_waitbutton_advances_with_one_a,
        "the audit never proved one-press advancement at a Mr. Pokemon/Oak waitbutton"
    );

    let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
    assert!(
        !shell
            .shell
            .snapshot()
            .expect("post-Pokedex snapshot")
            .bag
            .items
            .iter()
            .any(|slot| slot.item_id == "RED_SCALE"),
        "the first Mr. Pokemon visit must not invent a Red Scale"
    );
    let snapshot = shell.shell.snapshot().expect("completed Mr. Pokemon snapshot");
    assert!(
        snapshot
            .bag
            .key_items
            .iter()
            .any(|slot| slot.item_id == "MYSTERY_EGG"),
        "giveitem MYSTERY_EGG completed without placing the Egg in the Key Items pocket"
    );
    assert_eq!(
        snapshot.party.slots[0].pokemon.hp,
        snapshot.party.slots[0].pokemon.max_hp,
        "Mr. Pokemon's HealParty did not restore the deliberately wounded starter"
    );
    let state = shell.shell.session().state();
    for flag in [
        "EVENT_RIVAL_NEW_BARK_TOWN",
        "EVENT_PLAYERS_HOUSE_1F_NEIGHBOR",
        "EVENT_TOTODILE_POKEBALL_IN_ELMS_LAB",
    ] {
        assert!(
            state.flags.is_event_flag_set(flag).expect("valid event flag"),
            "missing ASM event flag {flag}"
        );
    }
    for flag in [
        "EVENT_PLAYERS_NEIGHBORS_HOUSE_NEIGHBOR",
        "EVENT_COP_IN_ELMS_LAB",
    ] {
        assert!(
            !state.flags.is_event_flag_set(flag).expect("valid event flag"),
            "ASM clearevent did not clear {flag}"
        );
    }
    assert_eq!(
        state.scenes.map_scenes.get("CherrygroveCity").map(String::as_str),
        Some("SCENE_CHERRYGROVECITY_MEET_RIVAL")
    );
    assert_eq!(
        state.scenes.map_scenes.get("ElmsLab").map(String::as_str),
        Some("SCENE_ELMSLAB_MEET_OFFICER")
    );
    assert_eq!(
        state.script_runtime.special_phone_call.as_deref(),
        Some("SPECIALCALL_ROBBED"),
        "Oak/Mr. Pokemon scene did not arm SPECIALCALL_ROBBED"
    );
    assert_eq!(
        state.last_spawn_identifier,
        Some(15),
        "Mr. Pokemon did not set SPAWN_CHERRYGROVE"
    );
    // Reproduce the real stale-accumulator failure without fabricating a
    // second visible input timeline after the deterministic scene settles.
    shell.shell.session_mut().state.script_runtime.script_value = Some("1".to_string());
    shell
        .shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "MrPokemonsHouse".to_string(),
                source_script: "MrPokemonsHouse_MrPokemonScript".to_string(),
                command_index: 2,
            },
            256,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run Mr. Pokemon follow-up to its first text boundary");
    let follow_up_label = shell
        .shell
        .script_events_snapshot()
        .pending_text_label
        .clone();
    assert_eq!(
        follow_up_label.as_deref(),
        Some("MrPokemonText_ImDependingOnYou"),
        "without RED_SCALE, Mr. Pokemon must not offer the EXP SHARE trade"
    );
}

fn collision_path_to_map_warp(
    start: &crate::core::world::session::OverworldSession,
    target_map: &str,
) -> Vec<Direction> {
    let mut queue = std::collections::VecDeque::from([(start.clone(), Vec::new())]);
    let mut visited =
        std::collections::BTreeSet::from([(start.player.tile.x, start.player.tile.y)]);
    while let Some((session, path)) = queue.pop_front() {
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            let mut next = session.clone();
            let mut step = next
                .step_and_check_warp_checked(
                    direction,
                    crate::core::world::movement::StepOptions::default(),
                )
                .expect("probe compiled collision");
            if matches!(
                step.outcome,
                crate::core::world::movement::StepOutcome::Turned { .. }
            ) {
                step = next
                    .step_and_check_warp_checked(
                        direction,
                        crate::core::world::movement::StepOptions::default(),
                    )
                    .expect("probe compiled collision after turn");
            }
            if !matches!(
                step.outcome,
                crate::core::world::movement::StepOutcome::Moved { .. }
            ) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(direction);
            if let Some(warp) = step.warp {
                if warp.warp.target_map == target_map || warp.warp.target_map_constant == target_map
                {
                    return next_path;
                }
                continue;
            }
            if visited.insert((next.player.tile.x, next.player.tile.y)) {
                queue.push_back((next, next_path));
            }
        }
    }
    panic!(
        "no collision path from {} to warp {target_map}",
        start.map.name
    )
}

fn collision_path_to_tile(
    start: &crate::core::world::session::OverworldSession,
    target: TilePosition,
) -> Vec<Direction> {
    let mut queue = std::collections::VecDeque::from([(start.clone(), Vec::new())]);
    let mut visited =
        std::collections::BTreeSet::from([(start.player.tile.x, start.player.tile.y)]);
    while let Some((session, path)) = queue.pop_front() {
        if session.player.tile == target {
            return path;
        }
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            let mut next = session.clone();
            let mut step = next
                .step_and_check_warp_checked(
                    direction,
                    crate::core::world::movement::StepOptions::default(),
                )
                .expect("probe compiled collision");
            if matches!(
                step.outcome,
                crate::core::world::movement::StepOutcome::Turned { .. }
            ) {
                step = next
                    .step_and_check_warp_checked(
                        direction,
                        crate::core::world::movement::StepOptions::default(),
                    )
                    .expect("probe compiled collision after turn");
            }
            if matches!(
                step.outcome,
                crate::core::world::movement::StepOutcome::Moved { .. }
            ) && step.warp.is_none()
                && visited.insert((next.player.tile.x, next.player.tile.y))
            {
                let mut next_path = path.clone();
                next_path.push(direction);
                queue.push_back((next, next_path));
            }
        }
    }
    panic!("no collision path from {} to {target:?}", start.map.name)
}

fn press_visible_direction_until_tile_changes(app: &mut App, direction: Direction) {
    let key = match direction {
        Direction::Up => KeyCode::ArrowUp,
        Direction::Down => KeyCode::ArrowDown,
        Direction::Left => KeyCode::ArrowLeft,
        Direction::Right => KeyCode::ArrowRight,
    };
    let before = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("pre-step snapshot")
        .overworld;
    for _ in 0..3 {
        press_key_for_runtime_hotkey_app(app, key);
        for _ in 0..9 {
            app.update();
        }
        let after = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .expect("post-step snapshot")
            .overworld;
        if after.map_name != before.map_name || after.tile != before.tile {
            return;
        }
    }
}

fn settle_visible_story_boundary(app: &mut App) -> (bool, bool) {
    let mut rendered_text = false;
    let mut rendered_picture = false;
    let mut previous_label = None;
    let mut completed_labels = std::collections::BTreeSet::new();
    let mut dialogue_activations = Vec::new();
    // This is a budget for real 60 Hz frames, including typewriter frames,
    // scripted walks, prompt release frames, and every multi-page Elm line.
    // A small loop can time out while the game is still progressing and then
    // falsely report a movement lock.
    for _ in 0..2048 {
        app.update();
        let (busy, has_text, has_picture, party_nonempty, visible_label, cursor) = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("story-boundary snapshot");
            let presentation = shell
                .visible_script_movement_scene
                .as_deref()
                .unwrap_or(&snapshot);
            (
                shell.active_script_cursor.is_some()
                    || shell.visible_script_movement.is_some()
                    || shell.visible_overworld_emote.is_some()
                    || shell.pending_name_choice.is_some()
                    || shell.pending_name_input.is_some()
                    || shell.shell.has_pending_script_work()
                    || snapshot.ui.text_window_open
                    || snapshot.ui.active_pokemon_picture.is_some(),
                presentation.ui.text_window_open
                    && presentation.ui.text.is_some()
                    && shell.pending_name_choice.is_none()
                    && shell.pending_name_input.is_none(),
                presentation.ui.active_pokemon_picture.is_some(),
                !snapshot.party.slots.is_empty(),
                presentation.ui.text.as_ref().map(|text| text.label.clone()),
                shell.active_script_cursor.clone(),
            )
        };
        if visible_label != previous_label {
            if let Some(previous) = previous_label.take() {
                completed_labels.insert(previous);
            }
            if let Some(label) = visible_label.clone() {
                assert!(
                    !completed_labels.contains(&label),
                    "compiled dialogue reactivated a completed text body {label}; activations={dialogue_activations:?} cursor={cursor:?}"
                );
                dialogue_activations.push(label);
            }
            previous_label = visible_label;
        }
        assert!(
            !(has_text && has_picture),
            "ASM closepokepic precedes opentext; a Pokemon picture and field textbox can never coexist (label={:?} cursor={cursor:?})",
            previous_label,
        );
        if has_text {
            let (visible_chars, yes_no_active) = {
                let shell = app.world().resource::<BevyRuntimeShell>();
                let snapshot = shell.shell.snapshot().expect("rendered text snapshot");
                (
                    shell
                        .field_text_reveal
                        .as_ref()
                        .map_or(0, |reveal| reveal.visible_chars),
                    scene_dialog_yes_no_active(&snapshot, shell),
                )
            };
            let world = app.world_mut();
            let glyph_count = world
                .query_filtered::<Entity, With<DialogGlyphMarker>>()
                .iter(world)
                .count();
            rendered_text |= glyph_count > 0;
            if visible_chars > 0 {
                assert!(
                    glyph_count > 0,
                    "visible field text has no rendered glyphs after {visible_chars} characters"
                );
            }
            let expected_frame_tiles = battle_window_frame_tile_count(
                FIELD_TEXT_BOX_WIDTH_TILES as usize,
                FIELD_TEXT_BOX_HEIGHT_TILES as usize,
            ) + usize::from(yes_no_active)
                * battle_window_frame_tile_count(
                    FIELD_YES_NO_WIDTH_TILES as usize,
                    FIELD_YES_NO_HEIGHT_TILES as usize,
                );
            let frame_tile_count = world
                .query_filtered::<Entity, With<SceneDialogWindowFrameMarker>>()
                .iter(world)
                .count();
            assert_eq!(
                frame_tile_count, expected_frame_tiles,
                "field dialogue retained a stale window frame"
            );
        }
        if has_picture && !rendered_picture {
            // The script snapshot becomes authoritative before Bevy's render
            // systems consume it. Give the newly opened picture a real
            // display frame before a player A press can dismiss it.
            app.update();
            let world = app.world_mut();
            rendered_picture |= world
                .query_filtered::<Entity, With<PokemonPictureMarker>>()
                .iter(world)
                .next()
                .is_some();
            assert!(
                world
                    .query_filtered::<Entity, With<SceneDialogTextBoxBackgroundMarker>>()
                    .iter(world)
                    .next()
                    .is_none(),
                "Pokepic rendered over a field textbox even though ASM has not reached opentext"
            );
            let picture_sprites = world
                .query_filtered::<(&Sprite, &Transform), With<PokemonPictureMarker>>()
                .iter(world)
                .map(|(sprite, transform)| (sprite.custom_size, transform.translation))
                .collect::<Vec<_>>();
            assert_eq!(
                picture_sprites.len(),
                36,
                "ASM menu_coords 6,4,14,13 requires 34 frame tiles, one interior, and one 7x7 frontpic"
            );
            assert!(
                picture_sprites
                    .iter()
                    .any(|(size, _)| *size == Some(Vec2::new(7.0 * TILE_SIZE, 8.0 * TILE_SIZE))),
                "Pokepic interior must match the ASM 9x10 outer window"
            );
            continue;
        }
        if !busy {
            return (rendered_text, rendered_picture);
        }
        let (pending_yes_no, pending_name_choice, completed_text_wait_before) = {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().unwrap();
            (
                snapshot.ui.pending_yes_no.is_some(),
                shell.pending_name_choice.is_some(),
                (snapshot.ui.pending_text_wait.is_some()
                    && visible_field_dialogue_is_entirely_consumed(shell, &snapshot)
                    && !shell.visible_wait_sfx_boundary)
                    .then(|| {
                        (
                            snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                            snapshot.ui.pending_text_wait.clone(),
                            shell.active_script_cursor.clone(),
                        )
                    }),
            )
        };
        if pending_yes_no {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("yes/no story snapshot");
            if scene_dialog_yes_no_active(&snapshot, shell) {
                assert!(
                    visible_field_dialogue_is_entirely_consumed(shell, &snapshot),
                    "yes/no prompt became visible before its complete authored text; label={:?} reveal={:?} cursor={:?}",
                    snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
                    shell.field_text_reveal,
                    shell.active_script_cursor
                );
            }
        }
        let key = if pending_name_choice || pending_yes_no && party_nonempty {
            KeyCode::KeyX
        } else {
            KeyCode::KeyZ
        };
        press_key_for_runtime_hotkey_app(app, key);
        if let Some(before) = completed_text_wait_before {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("post-A text-wait snapshot");
            let after = (
                snapshot.ui.text.as_ref().map(|text| text.label.clone()),
                snapshot.ui.pending_text_wait.clone(),
                shell.active_script_cursor.clone(),
            );
            assert_ne!(
                after, before,
                "one A press did not immediately advance a fully printed ASM text wait; movement={:?} reveal={:?} action={:?}",
                shell.visible_script_movement, shell.field_text_reveal, shell.last_runtime_action,
            );
        }
    }
    let a_action = {
        let mut shell = app.world_mut().resource_mut::<BevyRuntimeShell>();
        has_visible_shell_a_action(&mut shell)
    };
    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("timed-out story snapshot");
    panic!(
        "visible story boundary did not return control; map={} tile={:?} pending={:?} a_action={:?} reveal={:?} movement={:?} player_walk={} object_walk={} cursor={:?} ui={:?} action={:?}",
        snapshot.overworld.map_name,
        snapshot.overworld.tile,
        shell.shell.pending_script_work_reason(),
        a_action,
        shell.field_text_reveal,
        shell.visible_script_movement,
        shell.player_walk_frame_ticks,
        shell.object_walk_frame_ticks,
        shell.active_script_cursor,
        snapshot.ui,
        shell.last_runtime_action
    )
}

#[test]
fn elms_officer_naming_screen_resumes_the_authored_script_and_clears_the_scene() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 8,
        },
        BevyShellConfig {
            smoke_player_name: Some("CHRIS".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize Elm's Lab officer scene");
    if runtime_shell
        .shell
        .script_events_snapshot()
        .script_ended
        .is_some()
    {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    arm_visible_active_script_cursor(&mut runtime_shell, "CopScript", 0);
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let mut saw_name_input = false;
    let mut saw_second_text = false;
    let mut completed = false;
    for frame in 0..2048 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert_eq!(
            shell.last_error, None,
            "Elm officer scene failed at frame {frame}: {:?}",
            shell.last_audio_events
        );
        let snapshot = shell.shell.snapshot().expect("Elm officer snapshot");
        saw_name_input |= shell
            .pending_name_input
            .as_ref()
            .is_some_and(|input| input.label == "RIVAL'S NAME?");
        saw_second_text |= snapshot
            .ui
            .text
            .as_ref()
            .is_some_and(|text| text.label == "ElmsLabOfficerText2");
        completed = saw_name_input
            && saw_second_text
            && shell.active_script_cursor.is_none()
            && shell.visible_script_movement.is_none()
            && shell.pending_name_input.is_none()
            && !snapshot.ui.text_window_open;
        if completed {
            break;
        }
        let naming_cursor = shell.pending_name_input.as_ref().map(|input| {
            (
                input.cursor_row == visible_name_input_bottom_row_index(),
                input.cursor_column,
            )
        });
        let _ = shell;
        if naming_cursor.is_some_and(|(on_bottom_row, column)| on_bottom_row && column == 8) {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
        } else if naming_cursor.is_some() {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::Enter);
        } else {
            press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
        }
    }

    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("completed officer snapshot");
    assert!(
        completed,
        "Elm's officer script did not settle after rival naming: name={:?} text={:?} cursor={:?} movement={:?} boundary={:?} error={:?}",
        shell.pending_name_input,
        snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
        shell.active_script_cursor,
        shell.visible_script_movement,
        shell.special_boundary,
        shell.last_error,
    );
    assert!(saw_name_input, "NameRival never opened its ASM naming screen");
    assert!(
        saw_second_text,
        "confirming NameRival did not resume at ElmsLabOfficerText2"
    );
    assert_eq!(visible_rival_name(&snapshot), "SILVER");
    assert_eq!(
        shell
            .shell
            .current_scene_script()
            .expect("Elm's Lab scene")
            .map(|scene| scene.scene_id),
        Some("SCENE_ELMSLAB_NOOP".to_string()),
        "CopScript must commit the authored SCENE_ELMSLAB_NOOP scene"
    );
    assert!(
        !snapshot
            .visible_objects
            .iter()
            .any(|object| object.object_identifier.as_deref() == Some("ELMSLAB_OFFICER")),
        "CopScript disappear must remove the officer after naming"
    );
}

#[test]
fn elevator_floor_menu_owns_a_before_the_underlying_script_cursor() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "CeladonDeptStoreElevator".to_string(),
            tile_x: 3,
            tile_y: 1,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Celadon elevator");
    if runtime_shell
        .shell
        .script_events_snapshot()
        .script_ended
        .is_some()
    {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .backup_warp_map_name = Some("CeladonDeptStore2F".to_string());
    arm_visible_active_script_cursor(&mut runtime_shell, "CeladonDeptStoreElevatorScript", 0);
    continue_visible_script_after_prompt(&mut runtime_shell).expect("open elevator floor menu");
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("open elevator snapshot");
    assert!(
        has_visible_elevator_prompt(&snapshot, &runtime_shell),
        "the source elevator command did not open its floor menu"
    );
    assert!(
        snapshot.ui.text_window_open,
        "ASM elevator runs inside opentext until the selection returns"
    );
    assert_eq!(
        runtime_shell
            .active_script_cursor
            .as_ref()
            .map(|cursor| cursor.next_command_index),
        Some(2),
        "the visible menu must retain the continuation after elevator"
    );

    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    let shell = app.world().resource::<BevyRuntimeShell>();
    assert!(
        shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("selected elevator 1/1 floor=1/6 FLOOR_1F")),
        "one A press advanced beneath the visible elevator instead of selecting it: cursor={:?} elevator={:?} events={:?}",
        shell.active_script_cursor,
        shell.elevator_cursor,
        shell.last_audio_events,
    );
    assert!(
        shell.elevator_cursor.is_none(),
        "the selected floor menu remained open"
    );
}

#[test]
fn elevator_floor_menu_b_returns_false_and_finishes_the_authored_cancel_branch() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "CeladonDeptStoreElevator".to_string(),
            tile_x: 3,
            tile_y: 1,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Celadon elevator");
    if runtime_shell
        .shell
        .script_events_snapshot()
        .script_ended
        .is_some()
    {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    runtime_shell
        .shell
        .set_script_runtime_accumulator("1")
        .expect("seed stale script value");
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .backup_warp_map_name = Some("CeladonDeptStore2F".to_string());
    arm_visible_active_script_cursor(&mut runtime_shell, "CeladonDeptStoreElevatorScript", 0);
    continue_visible_script_after_prompt(&mut runtime_shell).expect("open elevator floor menu");
    assert_eq!(
        runtime_shell
            .shell
            .snapshot()
            .expect("opened elevator snapshot")
            .script_events
            .script_value
            .as_deref(),
        Some("0"),
        "Script_elevator must clear wScriptVar before opening the menu"
    );
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyX);

    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("cancelled elevator snapshot");
    assert_eq!(
        snapshot.script_events.script_value.as_deref(),
        Some("0"),
        "Script_elevator initializes wScriptVar to FALSE and B preserves it"
    );
    assert!(
        shell.elevator_cursor.is_none(),
        "B left the elevator menu open"
    );
    assert_eq!(
        shell.active_script_cursor, None,
        "iffalse .Done did not finish the authored cancel branch"
    );
    assert!(
        snapshot.script_events.pending_script_warp.is_none(),
        "cancelled elevator queued a destination warp"
    );
    assert!(
        !snapshot.ui.text_window_open,
        "the cancel branch did not execute closetext"
    );
}

#[test]
fn elevator_current_floor_selection_returns_false_without_a_warp_or_ride() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "CeladonDeptStoreElevator".to_string(),
            tile_x: 3,
            tile_y: 1,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Celadon elevator");
    if runtime_shell
        .shell
        .script_events_snapshot()
        .script_ended
        .is_some()
    {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .backup_warp_map_name = Some("CeladonDeptStore1F".to_string());
    arm_visible_active_script_cursor(&mut runtime_shell, "CeladonDeptStoreElevatorScript", 0);
    continue_visible_script_after_prompt(&mut runtime_shell).expect("open elevator floor menu");

    select_visible_elevator_floor(&mut runtime_shell).expect("select current floor");

    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("same-floor elevator snapshot");
    assert_eq!(snapshot.script_events.script_value.as_deref(), Some("0"));
    assert!(snapshot.script_events.pending_script_warp.is_none());
    assert!(runtime_shell.elevator_cursor.is_none());
    assert_eq!(runtime_shell.active_script_cursor, None);
    assert!(!snapshot.ui.text_window_open);
    assert!(
        !runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("SFX_ELEVATOR")),
        "selecting wElevatorOriginFloor must not run the ride branch"
    );
}

#[test]
fn elevator_without_a_matching_backup_floor_returns_false_without_opening_a_menu() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "CeladonDeptStoreElevator".to_string(),
            tile_x: 3,
            tile_y: 1,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Celadon elevator");
    if runtime_shell
        .shell
        .script_events_snapshot()
        .script_ended
        .is_some()
    {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    runtime_shell
        .shell
        .set_script_runtime_accumulator("1")
        .expect("seed stale script value");
    arm_visible_active_script_cursor(&mut runtime_shell, "CeladonDeptStoreElevatorScript", 0);

    continue_visible_script_after_prompt(&mut runtime_shell)
        .expect("execute unmatched elevator command");

    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("unmatched elevator snapshot");
    assert_eq!(snapshot.script_events.script_value.as_deref(), Some("0"));
    assert!(runtime_shell.elevator_cursor.is_none());
    assert_eq!(runtime_shell.active_script_cursor, None);
    assert!(!snapshot.ui.text_window_open);
    assert!(snapshot.script_events.pending_script_warp.is_none());
}

#[test]
fn visible_overworld_normal_inputs_exit_house_and_trigger_new_bark_teacher_stop() {
    fn push_frames(frames: &mut Vec<Vec<GameButton>>, button: GameButton, count: usize) {
        frames.extend(std::iter::repeat_n(vec![button], count));
    }

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
    let mut input_frames = Vec::new();
    push_frames(&mut input_frames, GameButton::Right, 6);
    push_frames(&mut input_frames, GameButton::Up, 4);
    push_frames(&mut input_frames, GameButton::Down, 9);
    push_frames(&mut input_frames, GameButton::Left, 4);
    push_frames(&mut input_frames, GameButton::Down, 1);
    // Cross New Bark on the clear northern lane, then approach the authored
    // west-exit teacher coordinate from above. The old southern route crossed
    // a random-walk NPC and sometimes consumed four inputs against him.
    push_frames(&mut input_frames, GameButton::Down, 1);
    push_frames(&mut input_frames, GameButton::Left, 13);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 2);

    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &input_frames,
        None,
    )
    .expect("normal inputs exit the house and trigger New Bark's teacher stop");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(
        smoke.final_map,
        "NewBarkTown",
        "house route stopped at ({}, {}) scene={:?} warps={} coord_events={} events={:?}",
        smoke.final_tile_x,
        smoke.final_tile_y,
        smoke.final_scene,
        smoke.warps,
        smoke.coord_events,
        smoke.frame_events
    );
    assert_eq!(
        (smoke.final_tile_x, smoke.final_tile_y),
        (5, 8),
        "normal-input route ended with events {:?}",
        smoke.frame_events
    );
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_NEWBARKTOWN_TEACHER_STOPS_YOU")
    );
    assert_eq!(smoke.warps, 2);
    assert_eq!(
        smoke.coord_events, 2,
        "teacher coordinate event did not fire: {:?}",
        smoke.frame_events
    );
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains("warp=true"))
    );
    assert_eq!(
        smoke
            .frame_events
            .iter()
            .filter(|event| event.contains("coord=true"))
            .count(),
        2
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("coord event script=NewBarkTown_TeacherStopsYouScene2"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("Text_ItsDangerousToGoAlone"))
    );
    assert!(smoke.audio_events.iter().any(|event| {
        event.contains(
            "script movement NEWBARKTOWN_TEACHER NewBarkTown_TeacherBringsYouBackMovement2",
        )
    }));
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("consumed runtime flag MapMusicRequested"))
    );
}

#[test]
fn elms_lab_callback_places_elm_at_asm_intro_position() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "ElmsLab".to_string(),
            tile_x: 5,
            tile_y: 3,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab");
    let snapshot = runtime_shell.shell.snapshot().expect("snapshot Elm's Lab");
    assert_eq!(
        snapshot.visible_object_runtime_tiles.get("ELMSLAB_ELM"),
        Some(&TilePosition { x: 3, y: 4 }),
        "ElmsLabMoveElmCallback must apply moveobject ELMSLAB_ELM, 3, 4 before the intro scene"
    );
    // Keep the cache path exercised here too: the callback is a gameplay
    // mutation, not a renderer-only correction.
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let cached = cached_runtime_snapshot(&mut runtime_shell).expect("cached callback snapshot");
    assert_eq!(
        cached.visible_object_runtime_tiles.get("ELMSLAB_ELM"),
        Some(&TilePosition { x: 3, y: 4 })
    );
}

#[test]
fn adjacent_scripted_npc_dispatches_when_the_player_faces_it() {
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
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "ElmsLab".to_string(),
            tile_x: 4,
            tile_y: 4,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab beside scripted NPC");

    // The NPC occupies (3, 4). A blocked Left press turns the player
    // toward it; the subsequent A must dispatch the object's compiled
    // script instead of falling through to a no-interaction frame.
    apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::Left])
        .expect("face adjacent scripted NPC");
    let faced = runtime_shell
        .shell
        .snapshot()
        .expect("snapshot after facing adjacent NPC");
    assert_eq!(
        faced.overworld.facing,
        Direction::Left,
        "blocked direction must turn the player before A routing; script_events={:?} cursor={:?}",
        faced.script_events,
        runtime_shell.active_script_cursor
    );
    assert!(
        runtime_shell
            .shell
            .current_overworld_interaction_checked()
            .expect("resolve faced NPC interaction")
            .is_some(),
        "faced NPC must be visible to authoritative interaction lookup"
    );
    let outcome = apply_visible_shell_smoke_frame(&mut runtime_shell, &[GameButton::A])
        .expect("interact with adjacent scripted NPC");
    assert!(outcome.interaction, "facing NPC must receive A interaction");
    assert!(
        runtime_shell
            .last_audio_events
            .iter()
            .any(|event| event.contains("interaction script=ProfElmScript")),
        "interaction must dispatch the object event's compiled script"
    );
}

#[test]
fn elm_dialogue_keeps_the_optional_world_frame_active_on_every_render_update() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 4,
            tile_y: 4,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab beside Elm");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.init_resource::<crystal_render_api::VisualWorldFrame>()
        .add_systems(Update, publish_visual_world_frame.after(render_playfield));

    app.update();
    app.update();
    let unpublished_diagnostic = {
        let rendered = app.world().resource::<RenderedViewport>();
        let shell = app.world().resource::<BevyRuntimeShell>();
        format!(
            "title_active={} map={:?} texture={} origin={:?} visual_key={:?} tiles={} error={:?}",
            rendered.title_active,
            rendered.map_name,
            rendered.map_texture.is_some(),
            rendered.viewport_origin,
            rendered.map_visual_key,
            rendered.visual_tiles.len(),
            shell.last_error,
        )
    };
    assert!(
        app.world()
            .resource::<crystal_render_api::VisualWorldFrame>()
            .active,
        "Elm's Lab must publish a valid optional-renderer frame before interaction: {unpublished_diagnostic}"
    );

    // ElmsLabWalkUpToElmScript shows EMOTE_SHOCK over Elm immediately before
    // opening his first textbox. This screen-space effect used to invalidate
    // the optional world frame for its full 15-frame duration, producing the
    // visible 2.5D -> 2D -> 2.5D flicker.
    app.world_mut()
        .resource_mut::<BevyRuntimeShell>()
        .visible_overworld_emote = Some(VisibleOverworldEmote {
        emote: "EMOTE_SHOCK".to_string(),
        object: "ELMSLAB_ELM".to_string(),
        frames_remaining: 15,
    });
    app.update();
    assert!(
        app.world()
            .resource::<crystal_render_api::VisualWorldFrame>()
            .active,
        "Elm's EMOTE_SHOCK must overlay the manually selected world view without changing it"
    );

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowLeft);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    for update in 0..240 {
        app.update();
        let world_frame = app
            .world()
            .resource::<crystal_render_api::VisualWorldFrame>();
        if !world_frame.active {
            let shell = app.world().resource::<BevyRuntimeShell>();
            let snapshot = shell.shell.snapshot().expect("Elm dialogue snapshot");
            panic!(
                "optional world frame dropped on Elm render update {update}: text={:?} movement={:?} emote={:?} map={} tile={:?}",
                snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
                shell.visible_script_movement,
                shell.visible_overworld_emote,
                snapshot.overworld.map_name,
                snapshot.overworld.tile,
            );
        }
    }
}

#[test]
fn visible_overworld_normal_inputs_enter_elms_lab_and_choose_cyndaquil() {
    fn push_frames(frames: &mut Vec<Vec<GameButton>>, button: GameButton, count: usize) {
        frames.extend(std::iter::repeat_n(vec![button], count));
    }

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
    let mut input_frames = Vec::new();
    push_frames(&mut input_frames, GameButton::Right, 6);
    push_frames(&mut input_frames, GameButton::Up, 4);
    push_frames(&mut input_frames, GameButton::Down, 5);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 4);
    // Exit the front door, then use the clear north lane. The southern lane
    // crosses NEWBARKTOWN_FISHER's random walk and made this audit depend on
    // where an autonomous NPC happened to be on that frame.
    push_frames(&mut input_frames, GameButton::Down, 1);
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Left, 8);
    push_frames(&mut input_frames, GameButton::Up, 2);
    // NewBarkTown.asm places the Elm's Lab warp at (6, 3). The preceding
    // turn-and-step inputs leave the player at (6, 4), directly below it.
    // Map entry completes Elm's deferred introduction and leaves the player
    // at (4, 4). Turn up and step onto the starter row after the warp settles.
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Up, 2);
    // ElmsLab.asm places the Cyndaquil ball at (6, 3). Move to (5, 3), face
    // right toward the ball, then interact once.
    push_frames(&mut input_frames, GameButton::Right, 3);
    push_frames(&mut input_frames, GameButton::A, 1);

    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &input_frames,
        None,
    )
    .expect("normal inputs enter Elm's lab and choose Cyndaquil");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(
        smoke.final_map, "ElmsLab",
        "starter route stopped at ({}, {}) with events {:?}",
        smoke.final_tile_x, smoke.final_tile_y, smoke.frame_events
    );
    assert_eq!(
        (smoke.final_tile_x, smoke.final_tile_y),
        (5, 3),
        "starter route events were {:?}",
        smoke.frame_events
    );
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_ELMSLAB_AIDE_GIVES_POTION"),
        "starter route party={:?} events={:?}",
        smoke.final_party_species,
        smoke.frame_events
    );
    assert_eq!(smoke.warps, 3);
    assert_eq!(smoke.coord_events, 1);
    assert_eq!(smoke.interactions, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_PROF_ELM"));
    assert!(smoke.pending_audio > 0);
    assert_eq!(smoke.final_party_species, vec!["CYNDAQUIL"]);
    assert_eq!(
        smoke
            .audio_events
            .iter()
            .filter(|event| event.contains("interaction script=CyndaquilPokeBallScript"))
            .count(),
        1,
        "one physical A press must dispatch the starter script exactly once"
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("CRY_CYNDAQUIL"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("gift pokemon 1/1 species=CYNDAQUIL level=5"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag EVENT_GOT_CYNDAQUIL_FROM_ELM=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script flag EVENT_GOT_A_POKEMON_FROM_ELM=true"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script runtime addcellnum args=[\"PHONE_ELM\"]"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script scene setmapscene SCENE_NEWBARKTOWN_NOOP"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script scene setscene SCENE_ELMSLAB_AIDE_GIVES_POTION"))
    );
}

#[test]
fn visible_overworld_normal_inputs_get_aide_potion_and_exit_elms_lab() {
    fn push_frames(frames: &mut Vec<Vec<GameButton>>, button: GameButton, count: usize) {
        frames.extend(std::iter::repeat_n(vec![button], count));
    }

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
    let mut input_frames = Vec::new();
    push_frames(&mut input_frames, GameButton::Right, 6);
    push_frames(&mut input_frames, GameButton::Up, 4);
    push_frames(&mut input_frames, GameButton::Down, 5);
    push_frames(&mut input_frames, GameButton::Down, 4);
    push_frames(&mut input_frames, GameButton::Left, 4);
    push_frames(&mut input_frames, GameButton::Down, 1);
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Left, 8);
    push_frames(&mut input_frames, GameButton::Up, 2);
    // Elm's deferred map-entry scene leaves the player at (4, 4).
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Up, 2);
    push_frames(&mut input_frames, GameButton::Right, 3);
    push_frames(&mut input_frames, GameButton::A, 1);
    push_frames(&mut input_frames, GameButton::Down, 6);
    // Acknowledge the aide's offer, the verbose item-receipt notice, and his
    // following "always busy" text before walking to the lab exit.
    push_frames(&mut input_frames, GameButton::A, 3);
    push_frames(&mut input_frames, GameButton::Down, 3);

    let smoke = smoke_visible_shell_overworld(
        asset_root,
        runtime,
        BevyShellStart::NewGame { spawn_identifier },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
        &input_frames,
        None,
    )
    .expect("normal inputs get the aide Potion and exit Elm's lab");

    assert_eq!(smoke.start_map, "PlayersHouse2F");
    assert_eq!((smoke.start_tile_x, smoke.start_tile_y), (3, 3));
    assert_eq!(
        smoke.final_map, "NewBarkTown",
        "aide route stopped early; tile=({}, {}) events={:?}",
        smoke.final_tile_x, smoke.final_tile_y, smoke.frame_events
    );
    assert_eq!(
        (smoke.final_tile_x, smoke.final_tile_y),
        (6, 3),
        "aide route events were {:?}",
        smoke.frame_events
    );
    assert_eq!(
        smoke.final_scene.as_deref(),
        Some("SCENE_NEWBARKTOWN_NOOP"),
        "aide route party={:?} events={:?}",
        smoke.final_party_species,
        smoke.frame_events
    );
    assert_eq!(smoke.warps, 4);
    assert_eq!(smoke.coord_events, 2);
    assert_eq!(smoke.interactions, 1);
    assert_eq!(smoke.active_music.as_deref(), Some("MUSIC_NEW_BARK_TOWN"));
    assert!(smoke.pending_audio > 0);
    assert_eq!(smoke.final_party_species, vec!["CYNDAQUIL"]);
    assert_eq!(
        smoke
            .final_bag_items
            .iter()
            .find(|item| item.item_id == "POTION")
            .map(|item| item.quantity),
        Some(1)
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("coord event script=AideScript_WalkPotion2"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script item grant Granted { item_id: \"POTION\""))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("AideText_GiveYouPotion"))
    );
    assert!(
        smoke
            .audio_events
            .iter()
            .any(|event| event.contains("script scene setscene SCENE_ELMSLAB_NOOP"))
    );
    assert!(
        smoke.frame_events.iter().any(|event| {
            event.contains(":[Down]:ElmsLab@(5,8)") && event.contains("coord=true")
        })
    );
    assert!(
        smoke
            .frame_events
            .iter()
            .any(|event| event.contains(":[Down]:NewBarkTown@(6,3)"))
    );
}

#[test]
fn cherrygrove_guide_tour_completes_and_grants_the_map_card() {
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
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 0,
            map_name: "CherrygroveCity".to_string(),
            tile_x: 32,
            tile_y: 7,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize beside the Cherrygrove guide");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    press_key_for_runtime_hotkey_app(&mut app, KeyCode::ArrowUp);
    press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);

    let mut completed = false;
    for _ in 0..4096 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        let snapshot = shell.shell.snapshot().expect("guide tour snapshot");
        completed = snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_MAP_CARD")
            && shell.active_script_cursor.is_none()
            && shell.visible_script_movement.is_none()
            && !snapshot.ui.text_window_open;
        if completed {
            break;
        }
        let _ = shell;
        press_key_for_runtime_hotkey_app(&mut app, KeyCode::KeyZ);
    }

    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell
        .shell
        .snapshot()
        .expect("completed guide tour snapshot");
    assert!(
        completed,
        "guide tour froze before granting the map card: tile={:?} guide={:?} cursor={:?} movement={:?} text={:?} wait={:?} action={:?} error={:?} events={:?}",
        snapshot.overworld.tile,
        snapshot
            .visible_object_runtime_tiles
            .get("CHERRYGROVECITY_GRAMPS"),
        shell.active_script_cursor,
        shell.visible_script_movement,
        snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
        snapshot.ui.pending_text_wait,
        shell.last_runtime_action,
        shell.last_error,
        shell.last_audio_events,
    );
    assert!(
        snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_MAP_CARD")
    );
    assert_eq!(snapshot.overworld.tile, TilePosition::new(24, 11));
}
