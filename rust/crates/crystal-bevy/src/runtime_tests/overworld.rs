#[test]
fn runtime_story_keys_dispatch_their_exact_facing_tile_scripts() {
    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load the Rust-built game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("load runtime from the Rust-built game pack");

    for (item_id, map_name, player_tile, facing, expected_script) in [
        (
            "CARD_KEY",
            "RadioTower3F",
            TilePosition::new(14, 3),
            Direction::Up,
            "CardKeySlotScript",
        ),
        (
            "BASEMENT_KEY",
            "GoldenrodUnderground",
            TilePosition::new(18, 7),
            Direction::Up,
            "BasementDoorScript",
        ),
    ] {
        let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime.clone(),
            0,
            map_name,
            player_tile.x,
            player_tile.y,
        )
        .expect("start at exact story-key use tile");
        shell.session_mut().overworld.player.facing = facing;
        let item = shell.runtime().data().items[item_id].clone();
        shell
            .session_mut()
            .state
            .bag
            .add_item(&item, 1)
            .expect("add story key");

        let used = shell
            .use_bag_story_key_in_field(item_id)
            .expect("use story key at exact source location");

        assert_eq!(used.target_script, expected_script);
        assert_eq!(
            used.target_tile,
            TilePosition::new(player_tile.x, player_tile.y - 1)
        );
        assert_eq!(
            shell
                .snapshot()
                .expect("story-key snapshot")
                .script_events
                .next_script
                .as_ref()
                .map(|script| script.script.as_str()),
            Some(expected_script)
        );
    }
}

#[test]
fn team_rocket_exploding_trap_clears_last_talked_with_the_minus_one_sentinel() {
    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load the Rust-built game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("load runtime from the Rust-built game pack");
    let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
        asset_root,
        runtime,
        0,
        "TeamRocketBaseB1F",
        24,
        2,
    )
    .expect("start on the exploding-trap map");
    let player_species = shell.runtime().data().pokemon["CHIKORITA"].clone();
    shell.session_mut().state.storage.party.pokemon[0] =
        Some(Pokemon::new_for_tests(player_species, 5, Dv::default()));
    let party_state = crystal_core::state::PartyState::from_storage(&shell.session().state.storage);
    shell.session_mut().state.party = party_state;
    let prior_object = shell
        .session()
        .overworld
        .objects
        .iter()
        .find_map(|object| object.object_identifier.clone())
        .expect("trap map has an exact object identifier");
    shell.session_mut().state.script_runtime.last_talked_object = Some(prior_object.clone());
    shell.session_mut().overworld.last_talked_object_identifier = Some(prior_object);

    let step = shell
        .step_compiled_script_command(
            "TeamRocketBaseB1F",
            "VoltorbExplodingTrap",
            3,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("execute canonical setlasttalked -1");

    assert_eq!(step.command, "setlasttalked");
    assert_eq!(
        shell.session().state.script_runtime.last_talked_object,
        None
    );
    assert_eq!(
        shell.session().overworld.last_talked_object_identifier,
        None
    );
    for command_index in 4..=6 {
        shell
            .step_compiled_script_command(
                "TeamRocketBaseB1F",
                "VoltorbExplodingTrap",
                command_index,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .unwrap_or_else(|error| {
                panic!("execute VoltorbExplodingTrap command {command_index}: {error:#}")
            });
    }
    let BattleMemory::StaticWild {
        battle_type,
        species,
        level,
        enemy_pokemon,
        ..
    } = &shell.session().state.battle
    else {
        panic!("VoltorbExplodingTrap must enter a wild battle");
    };
    assert_eq!(battle_type, "BATTLETYPE_TRAP");
    assert_eq!(species, "VOLTORB");
    assert_eq!(*level, 23);
    assert_eq!(enemy_pokemon.species.id, "VOLTORB");
    assert_eq!(enemy_pokemon.level, 23);
}

#[test]
fn runtime_overworld_starts_from_declared_spawn_and_steps_from_joypad() {
    let root = temp_repository_root("overworld");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_object_and_movement_commands();
    for object in &mut data
        .maps
        .get_mut("RuntimeMap")
        .expect("runtime map")
        .objects
    {
        if object.object_identifier.as_deref() == Some("RUNTIME_NPC") {
            object.x = 3;
        }
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let object_command = RuntimeScriptObjectCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "applymovement".to_string(),
        object_id: Some("RUNTIME_NPC".to_string()),
        target_object_id: None,
        x: None,
        y: None,
        direction: None,
        movement: Some("RuntimeNpcMovement".to_string()),
        emote: None,
        duration: None,
        source_script: "RuntimeObjectScript".to_string(),
        command_index: 4,
    };
    let wrong_object_movement = RuntimeScriptObjectCommandKey {
        movement: Some("runtimenpcmovement".to_string()),
        ..object_command.clone()
    };
    let movement_row = RuntimeScriptMovementKey {
        map_name: "RuntimeMap".to_string(),
        label: "RuntimeNpcMovement".to_string(),
        source_script: Some("RuntimeObjectScript".to_string()),
        steps: vec![
            RuntimeScriptMovementStepKey {
                command: "teleport_from".to_string(),
                direction: None,
                duration: None,
                index: 0,
            },
            RuntimeScriptMovementStepKey {
                command: "step".to_string(),
                direction: Some("RIGHT".to_string()),
                duration: None,
                index: 1,
            },
            RuntimeScriptMovementStepKey {
                command: "turn_head".to_string(),
                direction: Some("UP".to_string()),
                duration: None,
                index: 2,
            },
            RuntimeScriptMovementStepKey {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 3,
            },
        ],
    };
    let wrong_movement_direction = RuntimeScriptMovementKey {
        steps: vec![
            RuntimeScriptMovementStepKey {
                command: "teleport_from".to_string(),
                direction: None,
                duration: None,
                index: 0,
            },
            RuntimeScriptMovementStepKey {
                command: "step".to_string(),
                direction: Some("right".to_string()),
                duration: None,
                index: 1,
            },
            RuntimeScriptMovementStepKey {
                command: "turn_head".to_string(),
                direction: Some("UP".to_string()),
                duration: None,
                index: 2,
            },
            RuntimeScriptMovementStepKey {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 3,
            },
        ],
        ..movement_row.clone()
    };
    assert!(runtime.has_script_object_command(&object_command));
    assert!(!runtime.has_script_object_command(&wrong_object_movement));
    assert!(
        runtime
            .script_object_command_keys()
            .contains(&object_command)
    );
    assert!(
        runtime
            .require_script_object_command(&object_command)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_object_command(&wrong_object_movement)
            .is_err()
    );
    assert!(runtime.has_script_movement(&movement_row));
    assert!(!runtime.has_script_movement(&wrong_movement_direction));
    assert!(runtime.script_movement_keys().contains(&movement_row));
    assert!(runtime.require_script_movement(&movement_row).is_ok());
    assert!(
        runtime
            .require_script_movement(&wrong_movement_direction)
            .is_err()
    );
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    assert_eq!(
        session.state.overworld,
        OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(0, 0),
            facing: Direction::Down,
            mode: crystal_core::world::movement::MovementMode::Normal,
        }
    );

    let first = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");
    assert_eq!(first.snapshot.tile, TilePosition::new(0, 0));
    assert_eq!(first.snapshot.facing, Direction::Right);
    assert!(matches!(
        first.movement,
        Some(StepOutcome::Turned {
            facing: Direction::Right
        })
    ));
    assert_eq!(first.step_events, None);

    let second = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("move right");
    assert_eq!(second.snapshot.tile, TilePosition::new(1, 0));
    assert_eq!(second.step_events, Some(StepEventResult::default()));
    assert_eq!(second.snapshot.frame, 2);
    assert_eq!(session.state.frame_counter, 2);
    assert_eq!(
        session.state.overworld.snapshot_identity(),
        Some((
            "RuntimeMap",
            TilePosition::new(1, 0),
            Direction::Right,
            crystal_core::world::movement::MovementMode::Normal
        ))
    );
    assert_eq!(second.input_mask, B_PAD_RIGHT);
    assert_eq!(second.pressed_mask, 0);
    assert_eq!(second.state_checksum.frame(), 2);
    assert_eq!(
        session
            .state_checksum_frame(7)
            .expect("checksum frame")
            .checksum(),
        second.state_checksum
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_debug_start_preserves_aligned_runtime_coordinates() {
    let root = temp_repository_root("overworld-debug-tile");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");

    let mut session = runtime
        .start_overworld_session_at_runtime_tile(&asset_root, "RuntimeMap", 1, 1)
        .expect("overworld session at aligned runtime tile");

    assert_eq!(
        session.state.overworld,
        OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(1, 1),
            facing: Direction::Down,
            mode: crystal_core::world::movement::MovementMode::Normal,
        }
    );
    assert_eq!(session.overworld.player.tile, TilePosition::new(1, 1));
    assert_eq!(session.state.last_spawn_identifier, None);

    let turned = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");
    assert_eq!(turned.snapshot.tile, TilePosition::new(1, 1));
    assert_eq!(turned.snapshot.facing, Direction::Right);

    let moved = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("move right");
    assert_eq!(moved.snapshot.tile, TilePosition::new(2, 1));
    assert_eq!(
        session.state.overworld.snapshot_identity(),
        Some((
            "RuntimeMap",
            TilePosition::new(2, 1),
            Direction::Right,
            crystal_core::world::movement::MovementMode::Normal
        ))
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_debug_start_rejects_tiles_outside_compiled_bounds() {
    let root = temp_repository_root("overworld-debug-tile-oob");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");

    let error = runtime
        .start_overworld_session_at_runtime_tile(&asset_root, "RuntimeMap", 4, 0)
        .expect_err("debug start tile must fit compiled runtime map bounds")
        .to_string();

    assert!(
        error.contains(
            "runtime player tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds 4x2"
        ),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_debug_start_rejects_unaligned_runtime_tiles() {
    let root = temp_repository_root("overworld-debug-tile-odd");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");

    let session = runtime
        .start_overworld_session_at_runtime_tile(&asset_root, "RuntimeMap", 1, 0)
        .expect("odd debug start tile is a valid exact runtime tile");

    assert_eq!(session.overworld.player.tile, TilePosition::new(1, 0));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_start_rejects_declared_spawn_on_unwalkable_tile() {
    let root = temp_repository_root("overworld-spawn-wall");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.tilesets.insert(
        "johto".to_string(),
        TilesetDefinition {
            collision: [(
                "00".to_string(),
                vec![
                    "WALL".to_string(),
                    "WALL".to_string(),
                    "WALL".to_string(),
                    "WALL".to_string(),
                ],
            )]
            .into_iter()
            .collect(),
            palette_map: vec![0],
        },
    );
    let error = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect_err("declared spawn tile must be rejected during pack verification");
    let error = error_debug(error);

    assert!(
        error.contains("unwalkable_runtime_spawn_point")
            && error
                .contains("runtime spawn point resolves to non-walkable tile (0, 0) on RuntimeMap"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_processes_step_events_on_actual_moves_with_exact_statuses() {
    let root = temp_repository_root("step-events");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.maps
        .get_mut("RuntimeMap")
        .expect("runtime map")
        .events
        .warps
        .clear();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    session.overworld.player.facing = Direction::Up;
    let mut poisoned = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    poisoned.hp = 3;
    poisoned.status = Some(runtime.data.step_event_rules.poison_status.clone());
    session
        .state
        .storage
        .register_capture_in_box(0, poisoned)
        .expect("register poisoned Pokemon");
    session.state.sync_party_from_storage();
    session.state.step_events = StepEventCounters {
        poison_step_count: 3,
        ..StepEventCounters::default()
    };

    let turn = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");
    assert_eq!(turn.step_events, None);
    assert_eq!(
        session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("poisoned")
            .hp,
        3
    );

    let step = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("move right");
    assert_eq!(
        step.step_events.expect("step events").poison_result,
        Some(PoisonDamageResult {
            damaged_names: vec!["CHIKORITA".to_string()],
            fainted_names: Vec::new(),
        })
    );
    assert_eq!(
        session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("poisoned")
            .hp,
        2
    );
    assert_eq!(session.state.step_events.poison_step_count, 0);

    session.overworld.player.tile = TilePosition::new(0, 0);
    session.overworld.player.facing = Direction::Right;
    session.state.step_events.poison_step_count = 3;
    session.state.storage.party.pokemon[0]
        .as_mut()
        .expect("poisoned")
        .status = Some("poison".to_string());
    let case_changed = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("move with case changed status");
    assert_eq!(
        case_changed.step_events.expect("step events").poison_result,
        None
    );
    assert_eq!(
        session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("poisoned")
            .hp,
        2
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_sets_declared_map_music_from_modpack_asset() {
    let root = temp_repository_root("overworld-music");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_music(), report()),
        identity(),
    )
    .expect("runtime");

    let session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");

    assert_eq!(
        session.state.script_runtime.current_music.as_deref(),
        Some("MUSIC_ROUTE_29")
    );
    let save_path = root.join("slot.crystalsave");
    runtime
        .save_game(&save_path, session.state.clone())
        .expect("save music state");
    let loaded = runtime.load_save(&save_path).expect("load music state");
    let resumed = runtime
        .resume_overworld_session(&asset_root, loaded)
        .expect("resume music state");
    assert_eq!(
        resumed.state.script_runtime.current_music.as_deref(),
        Some("MUSIC_ROUTE_29")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_clock_updates_are_authoritative_and_saveable() {
    let root = temp_repository_root("runtime-clock");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");

    assert_eq!(session.state.time.time_of_day, TimeOfDay::Night);
    let manual = session
        .set_manual_clock_time(
            &runtime,
            GameDate::new(2000, 1, 1),
            6,
            0,
            0,
            ClockTime::new(0, 20, 30, 15),
        )
        .expect("set manual time");
    assert_eq!(manual.time_of_day, TimeOfDay::Night);
    assert_eq!(manual.hour, 20);
    assert_eq!(manual.minute, 30);

    let day = session
        .update_clock_from_datetime(&runtime, GameDate::new(2000, 1, 2), 22, 45, 0)
        .expect("update clock");
    assert_eq!(day.time_of_day, TimeOfDay::Day);
    assert_eq!(day.hour, 13);
    assert_eq!(day.minute, 15);

    let save_path = root.join("clock.crystalsave");
    runtime
        .save_game(&save_path, session.state.clone())
        .expect("save clock state");
    let loaded = runtime.load_save(&save_path).expect("load clock state");
    assert_eq!(loaded.time, session.state.time);
    assert_eq!(loaded.time.time_of_day, TimeOfDay::Day);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn every_pokecenter_nurse_uses_the_compiled_flow_and_renders_its_complete_dialogue() {
    fn finish_noninteractive_nurse_boundaries(
        shell: &mut RuntimeGameShell,
        mut run: RuntimeCompiledScriptRun,
    ) -> RuntimeCompiledScriptRun {
        loop {
            match run.boundary.as_ref() {
                Some(RuntimeCompiledScriptBoundary::TextWait(_)) | None => return run,
                Some(RuntimeCompiledScriptBoundary::TextLabel(_)) => {
                    shell
                        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
                        .expect("finish printing authored nurse text");
                }
                Some(RuntimeCompiledScriptBoundary::Delay(_)) => {
                    shell
                        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingDelay)
                        .expect("finish authored nurse pause");
                }
                Some(RuntimeCompiledScriptBoundary::ActiveMenu(menu))
                    if menu == "HealMachineAnim" =>
                {
                    shell
                        .drain_script_event_queue(RuntimeScriptEventQueue::Graphics)
                        .expect("present the nurse healing animation event");
                    shell
                        .close_active_menu()
                        .expect("finish the nurse healing animation");
                }
                boundary => panic!("unexpected nurse script boundary {boundary:?}"),
            }
            let cursor = run
                .next_cursor
                .take()
                .expect("nonterminal nurse boundary must retain its continuation");
            run = shell
                .run_compiled_script_until_boundary(
                    cursor,
                    128,
                    ScriptRuntimeInputs::default(),
                    ScriptPhoneInputs::default(),
                )
                .expect("continue nurse flow after noninteractive boundary");
        }
    }

    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load the Rust-built game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("load runtime from the Rust-built game pack");

    // The exporter preserves RGBDS local labels as separately addressable
    // compiled bodies. Audit the complete ordinary heal graph, not only its
    // root dispatcher (which intentionally contains no text commands).
    let mut nurse_commands = Vec::new();
    for script in [
        ".morn@PokecenterNurseScript",
        ".morn_comcenter@PokecenterNurseScript",
        ".day@PokecenterNurseScript",
        ".day_comcenter@PokecenterNurseScript",
        ".nite@PokecenterNurseScript",
        ".nite_comcenter@PokecenterNurseScript",
        ".ok@PokecenterNurseScript",
        ".no@PokecenterNurseScript",
        ".done@PokecenterNurseScript",
    ] {
        nurse_commands.extend(
            runtime
                .compiled_script_commands(script)
                .unwrap_or_else(|error| panic!("compiled nurse body {script}: {error:#}")),
        );
    }
    let rendered_nurse_labels = nurse_commands
        .iter()
        .filter(|command| {
            matches!(
                command.get("command").and_then(serde_json::Value::as_str),
                Some("writetext" | "farwritetext")
            )
        })
        .filter_map(|command| {
            command
                .get("args")
                .and_then(serde_json::Value::as_array)
                .and_then(|args| args.first())
                .and_then(serde_json::Value::as_str)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered_nurse_labels,
        vec![
            "NurseMornText",
            "PokeComNurseMornText",
            "NurseDayText",
            "PokeComNurseDayText",
            "NurseNiteText",
            "PokeComNurseNiteText",
            "NurseAskHealText",
            "NurseTakePokemonText",
            "NurseReturnPokemonText",
            "NurseGoodbyeText",
        ],
        "the compiled nurse flow must retain every time-of-day and Pok\u{00e9}Com text branch"
    );
    for label in rendered_nurse_labels {
        assert!(
            runtime
                .data()
                .asm_text
                .get(label)
                .is_some_and(|text| !text.is_empty()),
            "rendered nurse label {label} must resolve to non-empty compiled text"
        );
    }

    // Every 1F nurse must dispatch to the one canonical standard script.
    // This catches a missing map object, a stale local wrapper, and a
    // script that is no longer reachable from an actual Pokémon Center.
    let expected_nurse_scripts = [
        ("AzaleaPokecenter1F", "AzaleaPokecenter1FNurseScript"),
        (
            "BlackthornPokecenter1F",
            "BlackthornPokecenter1FNurseScript",
        ),
        ("CeladonPokecenter1F", "CeladonPokecenter1FNurseScript"),
        ("CeruleanPokecenter1F", "CeruleanPokecenter1FNurseScript"),
        (
            "CherrygrovePokecenter1F",
            "CherrygrovePokecenter1FNurseScript",
        ),
        ("CianwoodPokecenter1F", "CianwoodPokecenter1FNurseScript"),
        ("CinnabarPokecenter1F", "CinnabarPokecenter1FNurseScript"),
        ("EcruteakPokecenter1F", "EcruteakPokecenter1FNurseScript"),
        ("FuchsiaPokecenter1F", "FuchsiaPokecenter1FNurseScript"),
        ("GoldenrodPokecenter1F", "GoldenrodPokecenter1FNurseScript"),
        (
            "IndigoPlateauPokecenter1F",
            "IndigoPlateauPokecenter1FNurseScript",
        ),
        ("LavenderPokecenter1F", "LavenderPokecenter1FNurseScript"),
        ("MahoganyPokecenter1F", "MahoganyPokecenter1FNurseScript"),
        ("OlivinePokecenter1F", "OlivinePokecenter1FNurseScript"),
        ("PewterPokecenter1F", "PewterPokecenter1FNurseScript"),
        ("Route10Pokecenter1F", "Route10Pokecenter1FNurseScript"),
        ("Route32Pokecenter1F", "Route32Pokecenter1FNurseScript"),
        ("SaffronPokecenter1F", "SaffronPokecenter1FNurseScript"),
        (
            "SilverCavePokecenter1F",
            "SilverCavePokecenter1FNurseScript",
        ),
        ("VermilionPokecenter1F", "VermilionPokecenter1FNurseScript"),
        ("VioletPokecenter1F", "VioletPokecenterNurse"),
        ("ViridianPokecenter1F", "ViridianPokecenter1FNurseScript"),
    ];
    for (map_name, nurse_script) in expected_nurse_scripts {
        let module = runtime.data().map_module(map_name).expect("Pokecenter map");
        assert_eq!(
            module.attributes.tileset_name, "pokecenter",
            "{map_name} must use the Pokecenter renderer tileset"
        );
        assert!(
            module.events.warps.len() >= 3,
            "{map_name} must retain at least its two exterior doors and 2F staircase"
        );
        assert!(
            module.events.warps.iter().any(|warp| {
                warp.target_map_constant == "POKECENTER_2F"
                    && warp.target_map == "POKECENTER_2F"
                    && warp.target_warp_id == 1
            }),
            "{map_name} must retain its dynamic 2F staircase warp"
        );
        let body = module.scripts.get(nurse_script).expect("nurse script body");
        assert_eq!(
            body,
            &serde_json::json!([{ "command": "jumpstd", "args": ["PokecenterNurseScript"] }]),
            "{map_name} nurse must enter the canonical compiled nurse flow"
        );
        assert!(
            module
                .objects
                .iter()
                .any(|object| { object.script == nurse_script && object.sprite == "SPRITE_NURSE" }),
            "{map_name} must render a nurse object that invokes {nurse_script}"
        );
    }
    let link_floor = runtime
        .data()
        .map_module("Pokecenter2F")
        .expect("compiled shared Pokecenter 2F");
    assert!(
        link_floor.events.warps.iter().any(|warp| {
            warp.target_map_constant == "POKECENTER_2F"
                && warp.target_map == "POKECENTER_2F"
                && warp.target_warp_id == -1
        }),
        "Pokecenter 2F must preserve the dynamic return warp to the originating 1F"
    );

    let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
        asset_root,
        runtime,
        0,
        "CherrygrovePokecenter1F",
        3,
        3,
    )
    .expect("start at the Cherrygrove nurse counter");
    shell
        .set_manual_clock_time(
            GameDate::new(2000, 1, 1),
            12,
            0,
            0,
            ClockTime::new(0, 12, 0, 0),
        )
        .expect("set a valid daytime clock");
    let mut pikachu = Pokemon::new_for_tests(
        shell.runtime().data().pokemon["PIKACHU"].clone(),
        20,
        Dv::default(),
    );
    pikachu.hp = 1;
    shell
        .session
        .state
        .storage
        .register_capture_in_box(0, pikachu)
        .expect("add damaged party Pokemon");
    shell.session.state.sync_party_from_storage();
    let nurse_object = shell
        .runtime()
        .data()
        .map_module("CherrygrovePokecenter1F")
        .expect("compiled Cherrygrove Pokecenter")
        .objects
        .iter()
        .find(|object| object.script == "CherrygrovePokecenter1FNurseScript")
        .and_then(|object| object.object_identifier.clone())
        .expect("compiled nurse object identifier");
    shell.session.state.script_runtime.last_talked_object = Some(nurse_object.clone());
    shell.session.overworld.last_talked_object_identifier = Some(nurse_object);

    let greeting = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "CherrygrovePokecenter1F".to_string(),
                source_script: "PokecenterNurseScript".to_string(),
                command_index: 0,
            },
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run nurse greeting");
    assert!(matches!(
        greeting.boundary,
        Some(RuntimeCompiledScriptBoundary::TextLabel(ref label))
            if label == "NurseDayText"
    ));
    assert_eq!(
        shell.snapshot().expect("greeting snapshot").ui.text,
        Some(RuntimeTextSnapshot {
            label: "NurseDayText".to_string(),
            source: RuntimeTextSource::AsmText,
            asm_text: Some("Hello!\nWelcome to our\n#MON CENTER.".to_string()),
            body: None,
            queued_text_events: 2,
        })
    );

    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish printing the nurse greeting");
    let greeting_wait = shell
        .run_compiled_script_until_boundary(
            greeting.next_cursor.expect("greeting continuation cursor"),
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run from the completed greeting to its promptbutton");
    assert!(matches!(
        greeting_wait.boundary,
        Some(RuntimeCompiledScriptBoundary::TextWait(ScriptTextWait { ref command, .. }))
            if command == "promptbutton"
    ));

    let ask = shell
        .advance_text_wait_and_run_compiled_script(
            greeting_wait.next_cursor,
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("advance from greeting to heal prompt");
    assert!(matches!(
        ask.run.boundary,
        Some(RuntimeCompiledScriptBoundary::TextLabel(ref label))
            if label == "NurseAskHealText"
    ));
    assert_eq!(
        shell
            .snapshot()
            .expect("heal prompt snapshot")
            .ui
            .text
            .as_ref()
            .map(|text| text.label.as_str()),
        Some("NurseAskHealText")
    );
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish printing the nurse heal question");
    let ask_yes_no = shell
        .run_compiled_script_until_boundary(
            ask.run
                .next_cursor
                .expect("heal question continuation cursor"),
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run the completed heal question to yesorno");
    assert!(matches!(
        ask_yes_no.boundary,
        Some(RuntimeCompiledScriptBoundary::YesNo(_))
    ));

    let healed = shell
        .resolve_yes_no_and_run_compiled_script(
            true,
            ask_yes_no.next_cursor,
            128,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("accept healing and finish the nurse flow");
    let healed_run = finish_noninteractive_nurse_boundaries(&mut shell, healed.run);
    assert!(matches!(
        healed_run.boundary,
        Some(RuntimeCompiledScriptBoundary::TextWait(ScriptTextWait { ref command, .. }))
            if command == "waitbutton"
    ));
    assert_eq!(
        shell.session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("party Pokemon")
            .hp,
        shell.session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("party Pokemon")
            .max_hp
    );
    assert_eq!(
        shell
            .session
            .state
            .script_runtime
            .text_events
            .iter()
            .filter_map(|event| event.text_label.as_deref())
            .collect::<Vec<_>>(),
        vec![
            "NurseDayText",
            "NurseAskHealText",
            "NurseTakePokemonText",
            "NurseReturnPokemonText",
            "NurseGoodbyeText",
        ],
        "accepted healing must render every authored message in order"
    );
    assert_eq!(
        shell
            .snapshot()
            .expect("goodbye snapshot")
            .ui
            .text
            .as_ref()
            .map(|text| text.label.as_str()),
        Some("NurseGoodbyeText")
    );

    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("reload the Rust-built game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("reload runtime from the Rust-built game pack");
    let mut declined = RuntimeGameShell::new_game_at_runtime_tile(
        asset_root,
        runtime,
        0,
        "CherrygrovePokecenter1F",
        3,
        3,
    )
    .expect("restart at the Cherrygrove nurse counter");
    declined
        .set_manual_clock_time(
            GameDate::new(2000, 1, 1),
            12,
            0,
            0,
            ClockTime::new(0, 12, 0, 0),
        )
        .expect("set a valid daytime clock for declined healing");
    let mut pikachu = Pokemon::new_for_tests(
        declined.runtime().data().pokemon["PIKACHU"].clone(),
        20,
        Dv::default(),
    );
    pikachu.hp = 1;
    declined
        .session
        .state
        .storage
        .register_capture_in_box(0, pikachu)
        .expect("add damaged party Pokemon for declined healing");
    declined.session.state.sync_party_from_storage();
    let nurse_object = declined
        .runtime()
        .data()
        .map_module("CherrygrovePokecenter1F")
        .expect("compiled Cherrygrove Pokecenter for declined healing")
        .objects
        .iter()
        .find(|object| object.script == "CherrygrovePokecenter1FNurseScript")
        .and_then(|object| object.object_identifier.clone())
        .expect("compiled nurse object identifier for declined healing");
    declined.session.state.script_runtime.last_talked_object = Some(nurse_object.clone());
    declined.session.overworld.last_talked_object_identifier = Some(nurse_object);
    let greeting = declined
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "CherrygrovePokecenter1F".to_string(),
                source_script: "PokecenterNurseScript".to_string(),
                command_index: 0,
            },
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run nurse greeting before declining");
    declined
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish printing the declined-flow greeting");
    let greeting_wait = declined
        .run_compiled_script_until_boundary(
            greeting
                .next_cursor
                .expect("declined greeting continuation cursor"),
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run declined greeting to promptbutton");
    let ask = declined
        .advance_text_wait_and_run_compiled_script(
            greeting_wait.next_cursor,
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("advance to declined-healing prompt");
    declined
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish printing the declined-flow heal question");
    let ask_yes_no = declined
        .run_compiled_script_until_boundary(
            ask.run
                .next_cursor
                .expect("declined question continuation cursor"),
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run declined heal question to yesorno");
    let refused = declined
        .resolve_yes_no_and_run_compiled_script(
            false,
            ask_yes_no.next_cursor,
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("decline healing");
    let refused_run = finish_noninteractive_nurse_boundaries(&mut declined, refused.run);
    assert!(matches!(
        refused_run.boundary,
        Some(RuntimeCompiledScriptBoundary::TextWait(ScriptTextWait { ref command, .. }))
            if command == "waitbutton"
    ));
    assert_eq!(
        declined.session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("party Pokemon")
            .hp,
        1,
        "declining must not invoke HealParty"
    );
    assert_eq!(
        declined
            .session
            .state
            .script_runtime
            .text_events
            .iter()
            .filter_map(|event| event.text_label.as_deref())
            .collect::<Vec<_>>(),
        vec!["NurseDayText", "NurseAskHealText", "NurseGoodbyeText"],
        "declining must skip the take/return text while rendering the canonical goodbye"
    );
}

#[test]
fn saved_manual_clock_applies_process_downtime_without_advancing_play_time() {
    let root = temp_repository_root("runtime-clock-process-downtime");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let save_path = root.join("clock-downtime.crystalsave");
    let mut shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0).expect("game shell");
    shell
        .set_manual_clock_time(
            GameDate::new(2000, 1, 1),
            6,
            0,
            0,
            ClockTime::new(0, 20, 30, 0),
        )
        .expect("set manual time from recorded startup sample");
    shell.save(&save_path).expect("persist clock state");

    let mut resumed =
        RuntimeGameShell::resume_from_save(asset_root, runtime, &save_path).expect("resume save");
    assert_eq!(resumed.session().state().time.game_time_frames, 0);

    resumed
        .tick_with_rtc(
            std::iter::empty(),
            RuntimeRtcSample {
                date: GameDate::new(2000, 1, 2),
                hour: 8,
                minute: 0,
                second: 0,
            },
        )
        .expect("apply first post-process host sample");
    let state = resumed.session().state();
    assert_eq!(state.time.registers.hours, 22);
    assert_eq!(state.time.registers.minutes, 30);
    assert_eq!(state.time.registers.seconds, 0);
    assert_eq!(state.time.game_time_hours, 0);
    assert_eq!(state.time.game_time_minutes, 0);
    assert_eq!(state.time.game_time_seconds, 0);
    assert_eq!(state.time.game_time_frames, 1);

    resumed
        .tick_with_rtc(
            std::iter::empty(),
            RuntimeRtcSample {
                date: GameDate::new(2000, 1, 2),
                hour: 8,
                minute: 0,
                second: 1,
            },
        )
        .expect("apply continued-play host sample");
    let state = resumed.session().state();
    assert_eq!(state.time.registers.hours, 22);
    assert_eq!(state.time.registers.minutes, 30);
    assert_eq!(state.time.registers.seconds, 1);
    assert_eq!(state.time.game_time_frames, 2);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ordinary_unblocked_overworld_tick_advances_play_time_exactly_once() {
    let root = temp_repository_root("runtime-game-timer-single-vblank");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");

    shell
        .tick(std::iter::empty())
        .expect("advance one unblocked overworld VBlank");

    let state = shell.session().state();
    assert_eq!(state.frame_counter, 1);
    assert_eq!(state.time.game_time_frames, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn manual_offset_crossing_game_midnight_resets_once_without_host_date_change() {
    let root = temp_repository_root("manual-clock-game-midnight");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");
    shell.session_mut().state_mut().fishing.daily_flags1 = 0xff;
    shell.session_mut().state_mut().kenji_break_timer = 1;
    shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([0, 200]);

    shell
        .set_manual_clock_time(
            GameDate::new(2000, 1, 1),
            23,
            0,
            0,
            ClockTime::new(1, 0, 0, 0),
        )
        .expect("offset-adjusted wCurDay crossing must run the daily boundary");

    let state = shell.session().state();
    assert_eq!(state.time.current_day, 1);
    assert_eq!(state.fishing.daily_flags1, 0);
    assert_eq!(state.kenji_break_timer, 3);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn host_date_change_without_wcurday_change_does_not_reset_daily_state() {
    let root = temp_repository_root("rtc-host-date-same-game-day");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");
    shell.session_mut().state_mut().fishing.daily_flags1 = 0xff;
    shell.session_mut().state_mut().kenji_break_timer = 1;
    shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay(std::iter::empty());
    let random_before = shell.session().state().random_state;

    shell
        .update_clock_from_datetime(GameDate::new(2000, 5, 20), 12, 0, 0)
        .expect("140 elapsed RTC days reduce to the same wCurDay");

    let state = shell.session().state();
    assert_eq!(state.time.current_day, 0);
    assert_eq!(state.fishing.daily_flags1, 0xff);
    assert_eq!(state.kenji_break_timer, 1);
    assert_eq!(state.random_state, random_before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn multi_day_resume_runs_one_overdue_daily_countdown_and_records_one_rng_sample_pair() {
    let root = temp_repository_root("rtc-multi-day-single-reset");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("game shell");
    shell.session_mut().state_mut().fishing.daily_flags1 = 0xff;
    shell.session_mut().state_mut().kenji_break_timer = 1;
    shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([0, 200]);
    let before = shell.session().state().clone();
    let retained_before = shell.retained_runtime_commands().len();

    shell
        .update_clock_from_datetime(GameDate::new(2000, 1, 8), 12, 0, 0)
        .expect("overdue daily timer runs once after multi-day downtime");

    let state = shell.session().state();
    assert_eq!(state.time.current_day, 7);
    assert_eq!(state.fishing.daily_flags1, 0);
    assert_eq!(state.kenji_break_timer, 3);
    let frame = &shell.retained_runtime_commands()[retained_before];
    let command = crystal_assets::decode_runtime_mutation_command_frame(frame, &before)
        .expect("decode recorded multi-day clock update");
    let RuntimeMutationCommand::UpdateClockFromDatetime(command) = command else {
        panic!("multi-day resume must retain its clock mutation");
    };
    assert_eq!(command.divider_trace.samples, vec![0, 200]);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_game_shell_tick_with_rtc_applies_daily_reset_before_input() {
    let root = temp_repository_root("runtime-tick-rtc");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("game shell");
    shell
        .session_mut()
        .state_mut()
        .storage
        .register_capture_in_box(0, wounded_runtime_pokemon("CHIKORITA"))
        .expect("register party Pokemon");
    shell.session_mut().state_mut().sync_party_from_storage();
    shell.session_mut().state_mut().fishing.daily_flags1 = 0xff;
    shell.session_mut().state_mut().swarms.active.insert(
        "SWARM".to_string(),
        crystal_core::state::SwarmMapTarget {
            map_id: "RuntimeMap".to_string(),
            map_group: Some(1),
            map_number: Some(1),
        },
    );
    shell.session_mut().state_mut().storage.party.pokemon[0]
        .as_mut()
        .expect("starter slot")
        .pokerus = 0xa2;
    shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([0, 200]);
    let replay_base = shell.session().clone();
    let retained_before_clock_update = shell.retained_runtime_commands().len();

    shell
        .tick_with_rtc(
            std::iter::empty(),
            RuntimeRtcSample {
                date: GameDate::new(2000, 1, 2),
                hour: 12,
                minute: 0,
                second: 0,
            },
        )
        .expect("RTC sample and input frame");

    let state = shell.session().state();
    assert_eq!(state.fishing.daily_flags1, 0);
    assert!(state.swarms.active.is_empty());
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("starter slot")
            .pokerus
            & 0x0f,
        1
    );
    assert_eq!(state.kenji_break_timer, 3);
    assert_eq!(
        state.random_state,
        crystal_core::random::CrystalRandomState { add: 0, sub: 56 }
    );

    let retained_after_tick = &shell.retained_runtime_commands()[retained_before_clock_update..];
    assert!(
        retained_after_tick.len() >= 3,
        "RTC tick must record VBlank, clock update, and input"
    );
    let mut state_before_clock_update = replay_base.clone();
    state_before_clock_update
        .apply_runtime_command_frame(&runtime, &retained_after_tick[0])
        .expect("replay the VBlank mutation preceding the RTC update");
    let clock_frame = &retained_after_tick[1];
    let clock_command = crystal_assets::decode_runtime_mutation_command_frame(
        clock_frame,
        state_before_clock_update.state(),
    )
    .expect("decode recorded clock update against pre-mutation state");
    let RuntimeMutationCommand::UpdateClockFromDatetime(command) = clock_command else {
        panic!("RTC update must follow the authoritative VBlank timer mutation");
    };
    assert_eq!(command.divider_trace.samples, vec![0, 200]);

    let clock_command_with_trace = |samples: Vec<u8>| {
        RuntimeMutationCommand::UpdateClockFromDatetime(RuntimeClockUpdateCommand {
            date: GameDate::new(2000, 1, 2),
            hour: 12,
            minute: 0,
            second: 0,
            divider_trace: RuntimeDividerTrace::new(samples),
        })
    };
    let mut exhausted_replay = replay_base.clone();
    let exhausted_request = exhausted_replay
        .runtime_command_frame(1, 1, clock_command_with_trace(vec![0]))
        .expect("frame exhausted clock replay command");
    let exhausted_before = exhausted_replay.clone();
    let exhausted_error = exhausted_replay
        .apply_runtime_command_frame(&runtime, &exhausted_request)
        .expect_err("short clock divider trace must reject atomically");
    assert!(
        format!("{exhausted_error:#}").contains("divider replay exhausted after 1 samples"),
        "{exhausted_error:#}"
    );
    assert_eq!(exhausted_replay, exhausted_before);

    let mut tailed_replay = replay_base;
    let tailed_request = tailed_replay
        .runtime_command_frame(1, 1, clock_command_with_trace(vec![0, 200, 77]))
        .expect("frame tailed clock replay command");
    let tailed_before = tailed_replay.clone();
    let tailed_error = tailed_replay
        .apply_runtime_command_frame(&runtime, &tailed_request)
        .expect_err("unused clock divider sample must reject atomically");
    assert!(
        format!("{tailed_error:#}").contains("1 unconsumed samples after 2 reads"),
        "{tailed_error:#}"
    );
    assert_eq!(tailed_replay, tailed_before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_triggers_exact_coord_event_for_current_scene() {
    let root = temp_repository_root("coord-event");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_coord_event(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    assert_eq!(
        session.state.scenes.map_scenes.get("RuntimeMap"),
        Some(&"SCENE_RUNTIME_ACTIVE".to_string())
    );

    let turn = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");
    assert_eq!(turn.snapshot.tile, TilePosition::new(1, 1));
    assert_eq!(turn.coord_event, None);

    let step = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("step onto coord event");
    let coord = step.coord_event.expect("coord event");
    assert_eq!(coord.map_name, "RuntimeMap");
    assert_eq!(coord.tile, TilePosition::new(2, 1));
    assert_eq!(coord.scene_id, "SCENE_RUNTIME_ACTIVE");
    assert_eq!(coord.script_name, "RuntimeCoordScript");

    session
        .state
        .scenes
        .map_scenes
        .insert("RuntimeMap".to_string(), "scene_runtime_active".to_string());
    session.overworld.player.tile = TilePosition::new(1, 1);
    session.overworld.player.facing = Direction::Right;
    let case_changed = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("case changed scene step");
    assert_eq!(case_changed.coord_event, None);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_applies_script_audio_commands_from_exact_modpack_entries() {
    let root = temp_repository_root("script-audio");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/sfx/SFX_TACKLE.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/sfx/SFX_ITEM.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/cries/CRY_CHIKORITA.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_script_audio_and_map_commands(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let audio_command_row = RuntimeScriptAudioCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "playmusic".to_string(),
        audio_id: Some("MUSIC_ROUTE_29".to_string()),
        fade_frames: None,
        source_script: "RuntimeAudioScript".to_string(),
        command_index: 0,
    };
    let wrong_audio_command_row = RuntimeScriptAudioCommandKey {
        audio_id: Some("music_route_29".to_string()),
        ..audio_command_row.clone()
    };
    assert!(runtime.has_script_audio_command(&audio_command_row));
    assert!(!runtime.has_script_audio_command(&wrong_audio_command_row));
    assert!(
        runtime
            .script_audio_command_keys()
            .contains(&audio_command_row)
    );
    assert!(
        runtime
            .require_script_audio_command(&audio_command_row)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_audio_command(&wrong_audio_command_row)
            .is_err()
    );

    let music = session
        .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 0)
        .expect("play music");
    assert!(matches!(
        music.cue,
        ScriptAudioCue::Play {
            audio_id,
            ..
        } if audio_id == "MUSIC_ROUTE_29"
    ));
    assert_eq!(
        session.state.script_runtime.current_music.as_deref(),
        Some("MUSIC_ROUTE_29")
    );

    let sfx = session
        .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 1)
        .expect("play sfx");
    assert!(matches!(
        sfx.cue,
        ScriptAudioCue::Play {
            audio_id,
            ..
        } if audio_id == "SFX_TACKLE"
    ));

    let cry = session
        .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 2)
        .expect("play cry");
    assert!(matches!(
        cry.cue,
        ScriptAudioCue::Play {
            audio_id,
            ..
        } if audio_id == "CRY_CHIKORITA"
    ));
    assert_eq!(session.state.script_runtime.audio_events.len(), 3);
    assert_ne!(music.state_checksum, cry.state_checksum);

    let missing_exact_case = session
        .apply_script_audio_command(&runtime, "RuntimeMap", "runtimeaudioscript", 0)
        .expect_err("script labels are exact");
    assert!(format!("{missing_exact_case:#}").contains("has no script audio command"));

    let wrong_command_case = session
        .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 3)
        .expect_err("command names are exact");
    assert!(format!("{wrong_command_case:#}").contains("PlayMusic"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_loads_declared_pcm_cries_from_compiled_pack() {
    let root = temp_repository_root("script-audio-pcm");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/sfx/SFX_DEX_FANFARE_LESS_THAN_20.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/cries/CRY_CHIKORITA.pcm"),
    );
    let mut data = verified_runtime_bootstrap_data();
    data.pokemon_cries
        .get_mut("CHIKORITA")
        .expect("runtime cry metadata")
        .cry = "CRY_CHIKORITA".to_string();
    data.audio = vec![
        ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "content-packs/test/music/MUSIC_ROUTE_29.pcm",
        )
        .expect("music asset"),
        ModpackAudioAsset::sound_effect(
            "SFX_DEX_FANFARE_LESS_THAN_20",
            "content-packs/test/sfx/SFX_DEX_FANFARE_LESS_THAN_20.pcm",
            0x9f,
        )
        .expect("oak fanfare asset"),
        ModpackAudioAsset::pcm(
            "CRY_CHIKORITA",
            "content-packs/test/cries/CRY_CHIKORITA.pcm",
            ModpackAudioKind::Cry,
            ModpackPcmAudioFormat {
                sample_rate_hz: 22_050,
                channels: 2,
                bits_per_sample: 16,
            },
        )
        .expect("PCM cry asset"),
    ];
    let asset_root = AssetRoot::new(&root);
    let runtime_report = report_for(&data);
    let compiled_audio = [
        ("MUSIC_ROUTE_29".to_string(), vec![0_u8; 4]),
        ("SFX_DEX_FANFARE_LESS_THAN_20".to_string(), vec![0_u8; 4]),
        ("CRY_CHIKORITA".to_string(), vec![0_u8, 0, 0xff, 0x7f]),
    ]
    .into_iter()
    .collect();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_with_audio_for_tests(data, compiled_audio, runtime_report),
        identity(),
    )
    .expect("runtime");

    let cry = runtime
        .audio()
        .program(AudioKind::Cry, "CRY_CHIKORITA")
        .expect("PCM cry");
    match &cry.source {
        AudioProgramSource::Pcm { bytes, format, .. } => {
            assert_eq!(bytes, &[0_u8, 0, 0xff, 0x7f]);
            assert_eq!(format.sample_rate_hz, 22_050);
            assert_eq!(format.channels, 2);
            assert_eq!(format.bits_per_sample, 16);
        }
        AudioProgramSource::PcmGzip { .. } | AudioProgramSource::Midi { .. } => {
            panic!("test PCM cry unexpectedly used compressed pack source")
        }
    }
    let manifest_entry = runtime
        .audio()
        .manifest()
        .cries
        .get("CRY_CHIKORITA")
        .expect("PCM cry manifest entry");
    assert_eq!(manifest_entry.byte_len, 4);
    assert_eq!(manifest_entry.pcm_frame_count, Some(1));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_rejects_declared_pcm_cries_with_incomplete_frames() {
    let root = temp_repository_root("script-audio-pcm-unaligned");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/sfx/SFX_DEX_FANFARE_LESS_THAN_20.pcm"),
    );
    let pcm_path = root
        .join("apps/web/assets/data")
        .join("content-packs/test/cries/CRY_CHIKORITA.pcm");
    std::fs::create_dir_all(pcm_path.parent().expect("pcm parent")).expect("create pcm dir");
    std::fs::write(&pcm_path, [0_u8, 1, 2]).expect("write unaligned pcm");
    let mut data = verified_runtime_bootstrap_data();
    data.pokemon_cries
        .get_mut("CHIKORITA")
        .expect("runtime cry metadata")
        .cry = "CRY_CHIKORITA".to_string();
    data.audio = vec![
        ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "content-packs/test/music/MUSIC_ROUTE_29.pcm",
        )
        .expect("music asset"),
        ModpackAudioAsset::sound_effect(
            "SFX_DEX_FANFARE_LESS_THAN_20",
            "content-packs/test/sfx/SFX_DEX_FANFARE_LESS_THAN_20.pcm",
            0x9f,
        )
        .expect("oak fanfare asset"),
        ModpackAudioAsset::pcm(
            "CRY_CHIKORITA",
            "content-packs/test/cries/CRY_CHIKORITA.pcm",
            ModpackAudioKind::Cry,
            ModpackPcmAudioFormat {
                sample_rate_hz: 22_050,
                channels: 2,
                bits_per_sample: 16,
            },
        )
        .expect("PCM cry asset"),
    ];
    let asset_root = AssetRoot::new(&root);
    let runtime_report = report_for(&data);
    let compiled_audio = [
        ("MUSIC_ROUTE_29".to_string(), vec![0_u8; 4]),
        ("SFX_DEX_FANFARE_LESS_THAN_20".to_string(), vec![0_u8; 4]),
        ("CRY_CHIKORITA".to_string(), vec![0_u8, 1, 2]),
    ]
    .into_iter()
    .collect();
    let error = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_with_audio_for_tests(data, compiled_audio, runtime_report),
        identity(),
    )
    .expect_err("runtime must reject incomplete PCM frames")
    .to_string();

    assert!(
        error.contains("not a whole number of 4-byte frames"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_applies_script_map_commands_and_executes_pending_warp() {
    let root = temp_repository_root("script-map");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/sfx/SFX_TACKLE.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/sfx/SFX_ITEM.pcm"),
    );
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/cries/CRY_CHIKORITA.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_script_audio_and_map_commands(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let warp_command_row = RuntimeScriptMapCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "warpfacing".to_string(),
        target_map: Some("RuntimeMap".to_string()),
        x: Some(1),
        y: Some(0),
        facing: Some("RIGHT".to_string()),
        map_setup: None,
        source_script: "RuntimeWarpScript".to_string(),
        command_index: 0,
    };
    let wrong_warp_command_row = RuntimeScriptMapCommandKey {
        facing: Some("right".to_string()),
        ..warp_command_row.clone()
    };
    assert!(runtime.has_script_map_command(&warp_command_row));
    assert!(!runtime.has_script_map_command(&wrong_warp_command_row));
    assert!(
        runtime
            .script_map_command_keys()
            .contains(&warp_command_row)
    );
    assert!(
        runtime
            .require_script_map_command(&warp_command_row)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_map_command(&wrong_warp_command_row)
            .is_err()
    );

    let map_command = session
        .apply_script_map_command(&runtime, "RuntimeMap", "RuntimeWarpScript", 0)
        .expect("apply warpfacing");
    assert!(matches!(
        map_command.action,
        ScriptMapAction::Warp {
            target_map,
            tile,
            facing: Some(Direction::Right),
            ..
        } if target_map == "RuntimeMap" && tile == TilePosition::new(1, 0)
    ));
    assert_eq!(
        session
            .state
            .script_runtime
            .pending_script_warp
            .as_ref()
            .map(|request| (&request.target_map, request.tile, request.facing)),
        Some((
            &"RuntimeMap".to_string(),
            TilePosition::new(1, 0),
            Some(Direction::Right)
        ))
    );

    let warp = session
        .execute_pending_script_warp(&runtime, &asset_root)
        .expect("execute pending warp");
    assert_eq!(warp.target_map, "RuntimeMap");
    assert_eq!(warp.tile, TilePosition::new(1, 0));
    assert_eq!(warp.facing, Some(Direction::Right));
    assert_eq!(session.snapshot().tile, TilePosition::new(1, 0));
    assert_eq!(session.snapshot().facing, Direction::Right);
    assert_eq!(session.state.script_runtime.pending_script_warp, None);
    assert_eq!(
        session.state.overworld.snapshot_identity(),
        Some((
            "RuntimeMap",
            TilePosition::new(1, 0),
            Direction::Right,
            crystal_core::world::movement::MovementMode::Normal
        ))
    );

    let bad_warp = session
        .apply_script_map_command(&runtime, "RuntimeMap", "RuntimeWarpScript", 1)
        .expect("apply bad-warp sentinel");
    assert!(matches!(
        bad_warp.action,
        ScriptMapAction::LoadMap {
            ref command,
            ref map_setup,
            ..
        } if command == "warp" && map_setup.as_deref() == Some("MAPSETUP_BADWARP")
    ));
    assert_eq!(session.state.script_runtime.pending_script_warp, None);
    assert!(matches!(
        session.state.script_runtime.pending_map_load,
        Some(ref request)
            if request.command == "warp"
                && request.map_setup.as_deref() == Some("MAPSETUP_BADWARP")
    ));

    let missing_index = session
        .apply_script_map_command(&runtime, "RuntimeMap", "RuntimeWarpScript", 9)
        .expect_err("command index is exact");
    assert!(format!("{missing_index:#}").contains("has no script map command"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_applies_script_text_variable_and_control_commands_exactly() {
    let root = temp_repository_root("script-text-variable-control");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_text_variable_and_control_commands(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let variable_command_row = RuntimeScriptVariableCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "loadvar".to_string(),
        target: Some("VAR_CALLERID".to_string()),
        value_tokens: vec!["PHONE_BIRDKEEPER_VANCE".to_string()],
        source_script: "RuntimeVariableScript".to_string(),
        command_index: 0,
    };
    let wrong_variable_command_row = RuntimeScriptVariableCommandKey {
        value_tokens: vec!["phone_birdkeeper_vance".to_string()],
        ..variable_command_row.clone()
    };
    assert!(runtime.has_script_variable_command(&variable_command_row));
    assert!(!runtime.has_script_variable_command(&wrong_variable_command_row));
    assert!(
        runtime
            .script_variable_command_keys()
            .contains(&variable_command_row)
    );
    assert!(
        runtime
            .require_script_variable_command(&variable_command_row)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_variable_command(&wrong_variable_command_row)
            .is_err()
    );
    let control_command_row = RuntimeScriptControlCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "iftrue".to_string(),
        compare_value: None,
        target_label: Some(".Accepted".to_string()),
        resolved_target_script: Some("RuntimeAcceptedScript".to_string()),
        source_script: "RuntimeControlScript".to_string(),
        command_index: 0,
    };
    let wrong_control_command_row = RuntimeScriptControlCommandKey {
        resolved_target_script: Some("runtimeacceptedscript".to_string()),
        ..control_command_row.clone()
    };
    assert!(runtime.has_script_control_command(&control_command_row));
    assert!(!runtime.has_script_control_command(&wrong_control_command_row));
    assert!(
        runtime
            .script_control_command_keys()
            .contains(&control_command_row)
    );
    assert!(
        runtime
            .require_script_control_command(&control_command_row)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_control_command(&wrong_control_command_row)
            .is_err()
    );
    let text_command_row = RuntimeScriptTextCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "writetext".to_string(),
        text_label: Some("RuntimeGreetingText".to_string()),
        source_script: "RuntimeScript".to_string(),
        command_index: 1,
    };
    let wrong_text_command_row = RuntimeScriptTextCommandKey {
        text_label: Some("runtimegreetingtext".to_string()),
        ..text_command_row.clone()
    };
    assert!(runtime.has_script_text_command(&text_command_row));
    assert!(!runtime.has_script_text_command(&wrong_text_command_row));
    assert!(
        runtime
            .script_text_command_keys()
            .contains(&text_command_row)
    );
    assert!(
        runtime
            .require_script_text_command(&text_command_row)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_text_command(&wrong_text_command_row)
            .is_err()
    );

    let open = session
        .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 0)
        .expect("open text");
    assert!(matches!(open.action, ScriptTextAction::Open { .. }));
    let write = session
        .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 1)
        .expect("write text");
    assert!(matches!(
        write.action,
        ScriptTextAction::Write {
            text_label,
            ..
        } if text_label == "RuntimeGreetingText"
    ));
    let yes_no = session
        .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 2)
        .expect("yes no");
    assert!(matches!(yes_no.action, ScriptTextAction::YesNo { .. }));
    assert!(session.state.script_runtime.text_window_open);
    assert_eq!(session.state.script_runtime.pending_text_label, None);
    assert!(session.state.script_runtime.pending_yes_no.is_some());
    session.state.overworld = OverworldMemory::Active {
        map_name: "RuntimeMap".to_string(),
        tile: TilePosition::new(0, 0),
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    let text_snapshot = runtime
        .active_text_snapshot(&session.state)
        .expect("active map script text snapshot")
        .expect("runtime text snapshot");
    assert_eq!(text_snapshot.label, "RuntimeGreetingText");
    assert_eq!(
        text_snapshot.source,
        RuntimeTextSource::ScriptBody {
            map_name: "RuntimeMap".to_string()
        }
    );

    session
        .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeAcceptedScript", 0)
        .expect("open a later independent text window");
    assert_eq!(
        runtime
            .active_text_snapshot(&session.state)
            .expect("later open window snapshot"),
        None,
        "a new opentext must delimit history and never redisplay an earlier interaction's text"
    );

    let mut no_map_state = GameState::default();
    no_map_state.script_runtime.text_window_open = true;
    no_map_state.script_runtime.active_text_label = Some("RuntimeGreetingText".to_string());
    let no_map_error = runtime
        .active_text_snapshot(&no_map_state)
        .expect_err("script text body snapshots require an active map");
    let no_map_error = format!("{no_map_error:#}");
    assert!(
        no_map_error.contains(
            "runtime UI script text label 'RuntimeGreetingText' requires an active overworld map"
        ),
        "{no_map_error}"
    );

    let mut wrong_map_state = session.state.clone();
    wrong_map_state.script_runtime.text_window_open = true;
    wrong_map_state.script_runtime.active_text_label = Some("RuntimeGreetingText".to_string());
    wrong_map_state.overworld = OverworldMemory::Active {
        map_name: "OtherMap".to_string(),
        tile: TilePosition::new(0, 0),
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    let wrong_map_error = runtime
        .active_text_snapshot(&wrong_map_state)
        .expect_err("script text body snapshots must resolve from the active map");
    let wrong_map_error = format!("{wrong_map_error:#}");
    assert!(
            wrong_map_error.contains(
                "runtime UI script text label 'RuntimeGreetingText' is not declared by current compiled map OtherMap"
            ),
            "{wrong_map_error}"
        );

    let bad_text_case = session
        .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 3)
        .expect_err("compiled text labels are exact");
    let bad_text_case = format!("{bad_text_case:#}");
    assert!(bad_text_case.contains("runtimegreetingtext"));

    let load_var = session
        .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 0)
        .expect("load var");
    assert!(matches!(
        load_var.outcome,
        ScriptVariableOutcome::LoadVariable {
            variable,
            value,
            ..
        } if variable == "VAR_CALLERID" && value == "PHONE_BIRDKEEPER_VANCE"
    ));
    let read_var = session
        .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 1)
        .expect("read var");
    assert!(matches!(
        read_var.outcome,
        ScriptVariableOutcome::SetAccumulator {
            value,
            ..
        } if value == "PHONE_BIRDKEEPER_VANCE"
    ));
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("PHONE_BIRDKEEPER_VANCE")
    );
    let check_time = session
        .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 2)
        .expect("check time");
    assert!(matches!(
        check_time.outcome,
        ScriptVariableOutcome::SetAccumulator {
            value,
            ..
        } if value == "TRUE"
    ));

    let branch = session
        .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 0)
        .expect("iftrue");
    assert!(matches!(
        branch.action,
        ScriptControlAction::Jump {
            target_script,
            ..
        } if target_script == "RuntimeAcceptedScript"
    ));
    assert_eq!(
        session
            .state
            .script_runtime
            .next_script
            .as_ref()
            .map(|location| location.script.as_str()),
        Some("RuntimeAcceptedScript")
    );

    let clock = session
        .update_clock_from_datetime(&runtime, GameDate::new(2000, 1, 1), 12, 0, 0)
        .expect("update clock to day");
    assert_eq!(clock.time_of_day, TimeOfDay::Day);
    let day_check = session
        .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 2)
        .expect("check day time against night command");
    assert!(matches!(
        day_check.outcome,
        ScriptVariableOutcome::SetAccumulator {
            value,
            ..
        } if value == "FALSE"
    ));

    session
        .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 3)
        .expect("set numeric accumulator");
    let numeric_branch = session
        .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 1)
        .expect("numeric branch");
    assert!(matches!(
        numeric_branch.action,
        ScriptControlAction::Jump {
            target_script,
            ..
        } if target_script == "RuntimeEnoughScript"
    ));

    let jumpstd = session
        .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 2)
        .expect("jumpstd");
    assert!(matches!(
        jumpstd.action,
        ScriptControlAction::Jump {
            target_script,
            standard: true,
            ..
        } if target_script == "PokecenterSignScript"
    ));
    assert_ne!(open.state_checksum, jumpstd.state_checksum);

    let missing_exact_script = session
        .apply_script_variable_command(&runtime, "RuntimeMap", "runtimevariablescript", 0)
        .expect_err("script labels are exact");
    let missing_exact_script = format!("{missing_exact_script:#}");
    assert!(missing_exact_script.contains("has no script variable command"));
    let missing_control_index = session
        .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 9)
        .expect_err("control command indexes are exact");
    let missing_control_index = format!("{missing_control_index:#}");
    assert!(missing_control_index.contains("has no script control command"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_shell_steps_verified_compiled_script_commands_with_explicit_cursor() {
    let root = temp_repository_root("compiled-script-step");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_text_variable_and_control_commands();
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    map.scripts.insert(
        "RuntimeScript".to_string(),
        serde_json::json!([
            {"command": "opentext", "args": []},
            {"command": "writetext", "args": ["RuntimeGreetingText"]},
            {"command": "waitbutton", "args": []},
            {"command": "yesorno", "args": []},
            {"command": "loadmenu", "args": ["RuntimeMenu"]},
            {"command": "verticalmenu", "args": []},
            {"command": "setval", "args": ["8"]},
            {"command": "warpfacing", "args": ["RUNTIME_MAP", "1", "0", "RIGHT"]},
            {"command": "setval", "args": ["9"]}
        ]),
    );
    map.script_text_commands = vec![
        ScriptTextCommand {
            command: "opentext".to_string(),
            text_label: None,
            source_script: "RuntimeScript".to_string(),
            command_index: 0,
        },
        ScriptTextCommand {
            command: "writetext".to_string(),
            text_label: Some("RuntimeGreetingText".to_string()),
            source_script: "RuntimeScript".to_string(),
            command_index: 1,
        },
        ScriptTextCommand {
            command: "waitbutton".to_string(),
            text_label: None,
            source_script: "RuntimeScript".to_string(),
            command_index: 2,
        },
        ScriptTextCommand {
            command: "yesorno".to_string(),
            text_label: None,
            source_script: "RuntimeScript".to_string(),
            command_index: 3,
        },
    ];
    for (source_script, terminal_command) in [
        ("RuntimePromptScript", "promptbutton"),
        ("RuntimeYesNoTextScript", "yesorno"),
    ] {
        map.scripts.insert(
            source_script.to_string(),
            serde_json::json!([
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeGreetingText"]},
                {"command": terminal_command, "args": []}
            ]),
        );
        map.script_text_commands.extend([
            ScriptTextCommand {
                command: "opentext".to_string(),
                text_label: None,
                source_script: source_script.to_string(),
                command_index: 0,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("RuntimeGreetingText".to_string()),
                source_script: source_script.to_string(),
                command_index: 1,
            },
            ScriptTextCommand {
                command: terminal_command.to_string(),
                text_label: None,
                source_script: source_script.to_string(),
                command_index: 2,
            },
        ]);
    }
    map.scripts.insert(
        "RuntimeReloadScript".to_string(),
        serde_json::json!([
            {"command": "reloadmap", "args": []},
            {"command": "setval", "args": ["77"]}
        ]),
    );
    map.script_map_commands.push(ScriptMapCommand {
        command: "reloadmap".to_string(),
        target_map: None,
        x: None,
        y: None,
        facing: None,
        map_setup: None,
        source_script: "RuntimeReloadScript".to_string(),
        command_index: 0,
    });
    map.script_variable_commands.push(ScriptVariableCommand {
        command: "setval".to_string(),
        target: None,
        value_tokens: vec!["77".to_string()],
        source_script: "RuntimeReloadScript".to_string(),
        command_index: 1,
    });
    map.script_menu_definitions.insert(
        "RuntimeMenu".to_string(),
        ScriptMenuDefinition {
            label: "RuntimeMenu".to_string(),
            commands: vec![
                crystal_core::systems::script_text::ScriptMenuCommand {
                    command: "menu_coords".to_string(),
                    args: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "10".to_string(),
                        "8".to_string(),
                    ],
                    command_index: 0,
                },
                crystal_core::systems::script_text::ScriptMenuCommand {
                    command: "db".to_string(),
                    args: vec!["2".to_string(), "1".to_string(), "0".to_string()],
                    command_index: 1,
                },
                crystal_core::systems::script_text::ScriptMenuCommand {
                    command: "dw".to_string(),
                    args: vec!["RuntimeMenuItems".to_string()],
                    command_index: 2,
                },
            ],
        },
    );
    map.script_menu_definitions.insert(
        "RuntimeMenuItems".to_string(),
        ScriptMenuDefinition {
            label: "RuntimeMenuItems".to_string(),
            commands: vec![
                crystal_core::systems::script_text::ScriptMenuCommand {
                    command: "db".to_string(),
                    args: vec!["\"First@\"".to_string()],
                    command_index: 0,
                },
                crystal_core::systems::script_text::ScriptMenuCommand {
                    command: "db".to_string(),
                    args: vec!["\"Second@\"".to_string()],
                    command_index: 1,
                },
            ],
        },
    );
    map.script_vertical_menus.insert(
        "RuntimeScript:5".to_string(),
        crystal_assets::ScriptVerticalMenuDefinition {
            source_script: "RuntimeScript".to_string(),
            loadmenu_command_index: 4,
            verticalmenu_command_index: 5,
            header_label: "RuntimeMenu".to_string(),
            data_label: Some("RuntimeMenuItems".to_string()),
            options: vec!["First".to_string(), "Second".to_string()],
            two_dimensional: false,
            rows: None,
            columns: None,
            spacing: None,
        },
    );
    map.script_runtime_commands.push(ScriptRuntimeCommand {
        command: "loadmenu".to_string(),
        args: vec!["RuntimeMenu".to_string()],
        source_script: "RuntimeScript".to_string(),
        command_index: 4,
    });
    map.script_runtime_commands.push(ScriptRuntimeCommand {
        command: "verticalmenu".to_string(),
        args: Vec::new(),
        source_script: "RuntimeScript".to_string(),
        command_index: 5,
    });
    map.script_variable_commands.push(ScriptVariableCommand {
        command: "setval".to_string(),
        target: None,
        value_tokens: vec!["8".to_string()],
        source_script: "RuntimeScript".to_string(),
        command_index: 6,
    });
    map.script_map_commands.push(ScriptMapCommand {
        command: "warpfacing".to_string(),
        target_map: Some("RuntimeMap".to_string()),
        x: Some(1),
        y: Some(0),
        facing: Some("RIGHT".to_string()),
        map_setup: None,
        source_script: "RuntimeScript".to_string(),
        command_index: 7,
    });
    map.script_variable_commands.push(ScriptVariableCommand {
        command: "setval".to_string(),
        target: None,
        value_tokens: vec!["9".to_string()],
        source_script: "RuntimeScript".to_string(),
        command_index: 8,
    });
    map.scripts.insert(
        "RuntimeControlScript".to_string(),
        serde_json::json!([
            {"command": "iftrue", "args": [".Accepted"]},
            {"command": "ifgreater", "args": ["RUNTIME_BADGES - 1", ".Enough"]},
            {"command": "jumpstd", "args": ["PokecenterSignScript"]}
        ]),
    );
    map.scripts.insert(
        "RuntimeAcceptedScript".to_string(),
        serde_json::json!([
            {"command": "opentext", "args": []}
        ]),
    );
    map.script_text_commands.push(ScriptTextCommand {
        command: "opentext".to_string(),
        text_label: None,
        source_script: "RuntimeAcceptedScript".to_string(),
        command_index: 0,
    });
    map.script_runtime_commands.push(ScriptRuntimeCommand {
        command: "writecmdqueue".to_string(),
        args: vec!["RuntimeCommandQueue".to_string()],
        source_script: "RuntimeQueueScript".to_string(),
        command_index: 0,
    });
    map.scripts.insert(
        "RuntimeQueueScript".to_string(),
        serde_json::json!([
            {"command": "writecmdqueue", "args": ["RuntimeCommandQueue"]}
        ]),
    );
    map.scripts.insert(
        "RuntimeCommandQueue".to_string(),
        serde_json::json!([
            {"command": "cmdqueue", "args": ["CMDQUEUE_STONETABLE", "RuntimeStoneTable"]}
        ]),
    );
    map.scripts.insert(
        "RuntimeStoneTable".to_string(),
        serde_json::json!([
            {"command": "stonetable", "args": ["1", "RUNTIME_BOULDER", "RuntimeAcceptedScript"]},
            {"command": "db", "args": ["-1"]}
        ]),
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");

    let invalid_origin = shell
        .step_compiled_script_command(
            "MissingMap",
            "RuntimeScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect_err("compiled cursor origin must name a packed map")
        .to_string();
    assert!(
        invalid_origin.contains("cursor origin map MissingMap is missing"),
        "{invalid_origin}"
    );

    let open = shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step opentext");

    assert_eq!(open.command, "opentext");
    assert!(matches!(
        open.mutation.result,
        RuntimeMutationResult::ScriptTextApplied(ScriptTextAction::Open { .. })
    ));
    assert_eq!(
        open.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeScript".to_string(),
            command_index: 1,
        })
    );
    assert_eq!(open.next_script, None);
    assert_eq!(open.boundary, None);
    assert!(!open.ended);

    let prompt_run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimePromptScript".to_string(),
                command_index: 0,
            },
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run writetext through promptbutton");
    assert_eq!(
        prompt_run
            .steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["opentext", "writetext"]
    );
    assert!(matches!(
        prompt_run.boundary,
        Some(RuntimeCompiledScriptBoundary::TextLabel(ref label))
            if label == "RuntimeGreetingText"
    ));
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish RuntimePromptScript text");
    let prompt_wait = shell
        .run_compiled_script_until_boundary(
            prompt_run.next_cursor.expect("prompt continuation cursor"),
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run completed prompt text to promptbutton");
    assert!(matches!(
        prompt_wait.boundary,
        Some(RuntimeCompiledScriptBoundary::TextWait(ScriptTextWait {
            ref command,
            ref source_script,
            command_index: 2,
        })) if command == "promptbutton" && source_script == "RuntimePromptScript"
    ));
    shell
        .advance_pending_text_wait()
        .expect("clear promptbutton test boundary");

    let yes_no_run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeYesNoTextScript".to_string(),
                command_index: 0,
            },
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run writetext through yesorno");
    assert_eq!(
        yes_no_run
            .steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["opentext", "writetext"]
    );
    assert!(matches!(
        yes_no_run.boundary,
        Some(RuntimeCompiledScriptBoundary::TextLabel(ref label))
            if label == "RuntimeGreetingText"
    ));
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish RuntimeYesNoTextScript text");
    let yes_no_prompt = shell
        .run_compiled_script_until_boundary(
            yes_no_run.next_cursor.expect("yes/no continuation cursor"),
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run completed yes/no text to yesorno");
    assert!(matches!(
        yes_no_prompt.boundary,
        Some(RuntimeCompiledScriptBoundary::YesNo(ScriptYesNoPrompt {
            ref source_script,
            command_index: 2,
        })) if source_script == "RuntimeYesNoTextScript"
    ));
    shell
        .resolve_pending_yes_no(true)
        .expect("clear yes/no test boundary");

    let run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            },
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run script to yes/no boundary");
    assert_eq!(run.steps.len(), 2);
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["opentext", "writetext"]
    );
    assert!(
        matches!(
            run.boundary,
            Some(RuntimeCompiledScriptBoundary::TextLabel(ref label))
                if label == "RuntimeGreetingText"
        ),
        "{:?}",
        run.boundary
    );
    assert_eq!(
        run.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeScript".to_string(),
            command_index: 2,
        })
    );
    assert!(!run.ended);
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("finish RuntimeScript text");
    let wait_run = shell
        .run_compiled_script_until_boundary(
            run.next_cursor
                .clone()
                .expect("waitbutton continuation cursor"),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run completed RuntimeScript text to waitbutton");
    let resumed_wait = shell
        .advance_text_wait_and_run_compiled_script(
            wait_run.next_cursor.clone(),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("advance text wait and continue");
    assert_eq!(resumed_wait.wait.wait.command, "waitbutton");
    assert!(matches!(
        resumed_wait.run.boundary,
        Some(RuntimeCompiledScriptBoundary::YesNo(ScriptYesNoPrompt {
            ref source_script,
            command_index,
        })) if source_script == "RuntimeScript" && command_index == 3
    ));
    assert_eq!(
        resumed_wait.run.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeScript".to_string(),
            command_index: 4,
        })
    );
    let resumed_yes_no = shell
        .resolve_yes_no_and_run_compiled_script(
            true,
            resumed_wait.run.next_cursor.clone(),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("resolve yes/no and continue to menu");
    assert!(resumed_yes_no.resolution.accepted);
    assert_eq!(
        resumed_yes_no
            .run
            .steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["loadmenu", "verticalmenu"],
        "ASM loadmenu prepares the header; verticalmenu owns the input boundary"
    );
    assert_eq!(
        resumed_yes_no.run.boundary,
        Some(RuntimeCompiledScriptBoundary::ActiveMenu(
            "RuntimeMenu".to_string()
        ))
    );
    let resumed_menu = shell
        .select_vertical_menu_option_and_run_compiled_script(
            "RuntimeMenu",
            "RuntimeScript",
            5,
            1,
            "Second",
            Some(RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 6,
            }),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("select menu and continue");
    assert_eq!(resumed_menu.selection.script_value, "2");
    assert_eq!(resumed_menu.run.steps.len(), 1);
    assert_eq!(resumed_menu.run.steps[0].command, "setval");
    assert_eq!(
        shell.session.state().script_runtime.script_value.as_deref(),
        Some("8")
    );
    shell
        .close_active_menu()
        .expect("close menu before later script checks");
    let warp_run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 7,
            },
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run to script warp boundary");
    assert_eq!(warp_run.steps.len(), 1);
    assert_eq!(warp_run.steps[0].command, "warpfacing");
    assert!(matches!(
        warp_run.boundary,
        Some(RuntimeCompiledScriptBoundary::PendingScriptWarp(ScriptWarpRequest {
            ref target_map,
            tile,
            facing: Some(Direction::Right),
            ..
        })) if target_map == "RuntimeMap" && tile == TilePosition::new(1, 0)
    ));
    shell
        .runtime()
        .save_game(
            root.join("pending-script-warp.crystalsave"),
            shell.session.state().clone(),
        )
        .expect("pending script warp must save against exact compiled payload");
    let resumed_warp = shell
        .transition_script_warp_and_run_compiled_script(
            warp_run.next_cursor.clone(),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("transition script warp and continue");
    assert_eq!(resumed_warp.warp.target_map, "RuntimeMap");
    assert_eq!(resumed_warp.warp.tile, TilePosition::new(1, 0));
    assert_eq!(resumed_warp.warp.facing, Some(Direction::Right));
    assert_eq!(resumed_warp.run.steps.len(), 1);
    assert_eq!(resumed_warp.run.steps[0].command, "setval");
    assert_eq!(
        shell.session.state().script_runtime.script_value.as_deref(),
        Some("9")
    );

    let reload_run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeReloadScript".to_string(),
                command_index: 0,
            },
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run only through the blocking reloadmap command");
    assert_eq!(
        reload_run
            .steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["reloadmap"],
        "commands after ASM StopScript must not execute before map reload completes"
    );
    assert!(matches!(
        reload_run.boundary,
        Some(RuntimeCompiledScriptBoundary::PendingMapLoad(ScriptMapLoadRequest {
            ref command,
            ref source_script,
            command_index: 0,
            ..
        })) if command == "reloadmap" && source_script == "RuntimeReloadScript"
    ));
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapLoad)
        .expect("complete pending map reload");
    let after_reload = shell
        .run_compiled_script_until_boundary(
            reload_run.next_cursor.expect("reload continuation cursor"),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("continue only after map reload completion");
    assert_eq!(after_reload.steps.len(), 1);
    assert_eq!(after_reload.steps[0].command, "setval");
    assert_eq!(
        shell.session.state().script_runtime.script_value.as_deref(),
        Some("77")
    );

    let max_steps_error = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            },
            1,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect_err("bounded script runner must reject overrun");
    assert!(max_steps_error.to_string().contains("max_steps 1"));
    shell
        .close_text_window()
        .expect("clear text window before single-step boundary check");
    assert!(!shell.session.state().script_runtime.text_window_open);
    assert!(
        shell
            .session
            .state()
            .script_runtime
            .pending_text_label
            .is_none()
    );
    assert!(
        shell
            .session
            .state()
            .script_runtime
            .pending_text_wait
            .is_none()
    );
    assert!(
        shell
            .session
            .state()
            .script_runtime
            .pending_yes_no
            .is_none()
    );
    shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("open text before single-step yes/no boundary check");

    let wait = shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeScript",
            3,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step yesorno");
    assert!(matches!(
        wait.mutation.result,
        RuntimeMutationResult::ScriptTextApplied(ScriptTextAction::YesNo { .. })
    ));
    assert!(matches!(
        wait.boundary,
        Some(RuntimeCompiledScriptBoundary::YesNo(ScriptYesNoPrompt {
            ref source_script,
            command_index,
        })) if source_script == "RuntimeScript" && command_index == 3
    ));
    assert_eq!(
        wait.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeScript".to_string(),
            command_index: 4,
        })
    );

    shell.session.state.script_runtime.script_value = Some("TRUE".to_string());
    shell
        .resolve_pending_yes_no(true)
        .expect("resolve yes/no before branch");
    let branch = shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeControlScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step iftrue");

    assert!(matches!(
        branch.mutation.result,
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::Jump {
            ref target_script,
            ..
        }) if target_script == "RuntimeAcceptedScript"
    ));
    assert_eq!(branch.next_script.as_deref(), Some("RuntimeAcceptedScript"));
    assert_eq!(
        branch.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeAcceptedScript".to_string(),
            command_index: 0,
        })
    );
    assert_eq!(
        shell
            .session
            .state()
            .script_runtime
            .next_script
            .as_ref()
            .map(|location| location.script.as_str()),
        Some("RuntimeAcceptedScript")
    );
    let pending_run = shell
        .run_pending_next_script_until_boundary(
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run pending next script");
    assert_eq!(pending_run.next_script.script, "RuntimeAcceptedScript");
    assert_eq!(pending_run.run.steps.len(), 1);
    assert_eq!(
        pending_run.run.steps[0].source_script,
        "RuntimeAcceptedScript"
    );
    assert_eq!(pending_run.run.steps[0].command, "opentext");
    assert_eq!(pending_run.run.next_cursor, None);
    assert_eq!(pending_run.run.boundary, None);
    assert_eq!(shell.session.state().script_runtime.next_script, None);

    shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeQueueScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step writecmdqueue");
    assert!(
        shell
            .session
            .state()
            .script_runtime
            .command_queue
            .is_empty()
    );
    assert_eq!(
        shell.session.state().script_runtime.stone_table_entries,
        vec![crystal_core::state::ScriptRuntimeStoneTableEntry {
            queue_slot: 0,
            warp: 1,
            object_event: "RUNTIME_BOULDER".to_string(),
            script: "RuntimeAcceptedScript".to_string(),
            source_script: "RuntimeStoneTable".to_string(),
            command_index: 0,
        }]
    );

    shell
        .session
        .state
        .script_runtime
        .deferred_scripts
        .push(ScriptLocation {
            origin_map_name: "RuntimeMap".to_string(),
            script: "RuntimeAcceptedScript".to_string(),
        });
    let deferred_run = shell
        .run_next_deferred_script_until_boundary(
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run deferred script target");
    assert_eq!(deferred_run.deferred_script.script, "RuntimeAcceptedScript");
    assert_eq!(deferred_run.run.steps.len(), 1);
    assert_eq!(
        deferred_run.run.steps[0].source_script,
        "RuntimeAcceptedScript"
    );
    assert_eq!(deferred_run.run.steps[0].command, "opentext");
    assert_eq!(deferred_run.run.next_cursor, None);
    assert_eq!(deferred_run.run.boundary, None);
    assert!(
        shell
            .session
            .state()
            .script_runtime
            .deferred_scripts
            .is_empty()
    );
    let text_events_before_standard = shell.session.state().script_runtime.text_events.len();
    let standard = shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeControlScript",
            2,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("execute exported standard-script body");
    assert!(
        matches!(
            &standard.mutation.result,
            RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::Jump {
                standard: true,
                target_script,
                ..
            }) if target_script == "PokecenterSignScript"
        ),
        "{:?}",
        standard.mutation.result
    );
    assert_eq!(
        shell.session.state().script_runtime.text_events.len(),
        text_events_before_standard,
        "jumpstd changes the compiled cursor; the target command executes on the next step"
    );
    assert_eq!(
        standard.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "PokecenterSignScript".to_string(),
            command_index: 0,
        })
    );
    shell
        .step_compiled_script_command(
            "RuntimeMap",
            "PokecenterSignScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("execute the standard script target command");
    assert_eq!(
        shell
            .session
            .state()
            .script_runtime
            .text_events
            .last()
            .and_then(|event| event.text_label.as_deref()),
        Some("PokecenterSignText")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compiled_runner_preserves_the_resolved_cross_map_jump_owner() {
    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load source-derived game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("load source-derived runtime");
    // Script_BeatenAllTrainers2 belongs to the battle room but runs after
    // warpfacing has installed BattleTower1F. Its sjump target is owned by
    // the live 1F map.
    let command_index = runtime
        .compiled_script_commands("Script_BeatenAllTrainers2")
        .expect("compiled Battle Tower post-warp script")
        .iter()
        .position(|command| {
            command.get("command").and_then(serde_json::Value::as_str) == Some("sjump")
                && command
                    .get("args")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|args| args.first())
                    .and_then(serde_json::Value::as_str)
                    == Some("Script_GivePlayerHisPrize")
        })
        .expect("cross-map prize sjump");
    let mut shell =
        RuntimeGameShell::new_game_at_runtime_tile(asset_root, runtime, 0, "BattleTower1F", 8, 5)
            .expect("Battle Tower 1F shell");

    let step = shell
        .step_compiled_script_command(
            "BattleTowerBattleRoom",
            "Script_BeatenAllTrainers2",
            command_index,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run source-derived cross-map prize jump");

    let expected = RuntimeCompiledScriptCursor {
        origin_map_name: "BattleTower1F".to_string(),
        source_script: "Script_GivePlayerHisPrize".to_string(),
        command_index: 0,
    };
    assert_eq!(step.next_cursor, Some(expected));
    assert_eq!(
        shell
            .snapshot()
            .expect("post-jump snapshot")
            .script_events
            .next_script
            .as_ref()
            .map(|script| script.origin_map_name.as_str()),
        Some("BattleTower1F")
    );
}

#[test]
fn compiled_runner_executes_source_derived_loadwildmon_setup() {
    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load source-derived game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("load source-derived runtime");
    let command_index = runtime
        .compiled_script_commands("WateredWeirdTreeScript")
        .expect("compiled Sudowoodo script")
        .iter()
        .position(|command| {
            command.get("command").and_then(serde_json::Value::as_str) == Some("loadwildmon")
        })
        .expect("Sudowoodo loadwildmon command");
    let mut shell =
        RuntimeGameShell::new_game_at_runtime_tile(asset_root, runtime, 0, "Route36", 34, 9)
            .expect("Route 36 shell");

    let step = shell
        .step_compiled_script_command(
            "Route36",
            "WateredWeirdTreeScript",
            command_index,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("execute source-derived loadwildmon");

    assert_eq!(
        step.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "Route36".to_string(),
            source_script: "WateredWeirdTreeScript".to_string(),
            command_index: command_index + 1,
        })
    );
    let snapshot = shell.snapshot().expect("post-loadwildmon snapshot");
    assert_eq!(
        snapshot
            .script_events
            .memory
            .get("wBattleScriptFlags")
            .map(String::as_str),
        Some("128")
    );
    assert_eq!(
        snapshot
            .script_events
            .memory
            .get("wTempWildMonSpecies")
            .map(String::as_str),
        Some("SUDOWOODO")
    );
    assert_eq!(
        snapshot
            .script_events
            .memory
            .get("wCurPartyLevel")
            .map(String::as_str),
        Some("20")
    );
}

#[test]
fn compiled_runner_executes_source_derived_dynamic_phone_memcall() {
    let asset_root = AssetRoot::new(repository_root_for_tests());
    let pack = asset_root
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load source-derived game pack");
    let runtime = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
        .expect("load source-derived runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");
    let origin_map_name = shell
        .snapshot()
        .expect("initial snapshot")
        .overworld
        .map_name;
    shell.session_mut().state.script_runtime.memory.insert(
        "wCallerContact + PHONE_CONTACT_SCRIPT2_BANK".to_string(),
        "BikeShopPhoneCallerScript".to_string(),
    );

    let step = shell
        .step_compiled_script_command(
            &origin_map_name,
            "Script_ReceivePhoneCall",
            2,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("execute source-derived memcall");

    assert_eq!(
        step.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: origin_map_name.clone(),
            source_script: "BikeShopPhoneCallerScript".to_string(),
            command_index: 0,
        })
    );
    assert_eq!(
        shell.session().state.script_runtime.call_stack.last(),
        Some(&crystal_core::state::ScriptReturnFrame {
            origin_map_name,
            source_script: "Script_ReceivePhoneCall".to_string(),
            next_command_index: 3,
        })
    );
}

#[test]
fn compiled_runner_returns_through_nested_local_and_far_script_calls() {
    let root = temp_repository_root("compiled-nested-script-calls");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_text_variable_and_control_commands();
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    map.scripts.insert(
        "RuntimeCallScript".to_string(),
        serde_json::json!([
            {"command": "farscall", "args": ["RuntimeCalledScript"]},
            {"command": "setval", "args": ["33"]},
            {"command": "end", "args": []}
        ]),
    );
    map.scripts.insert(
        "RuntimeCalledScript".to_string(),
        serde_json::json!([
            {"command": "scall", "args": ["RuntimeNestedScript"]},
            {"command": "setval", "args": ["22"]},
            {"command": "end", "args": []}
        ]),
    );
    map.scripts.insert(
        "RuntimeNestedScript".to_string(),
        serde_json::json!([
            {"command": "setval", "args": ["11"]},
            {"command": "end", "args": []}
        ]),
    );
    map.script_control_commands.extend([
        ScriptControlCommand {
            command: "farscall".to_string(),
            compare_value: None,
            target_label: Some("RuntimeCalledScript".to_string()),
            resolved_target_script: Some("RuntimeCalledScript".to_string()),
            source_script: "RuntimeCallScript".to_string(),
            command_index: 0,
        },
        ScriptControlCommand {
            command: "end".to_string(),
            compare_value: None,
            target_label: None,
            resolved_target_script: None,
            source_script: "RuntimeCallScript".to_string(),
            command_index: 2,
        },
        ScriptControlCommand {
            command: "scall".to_string(),
            compare_value: None,
            target_label: Some("RuntimeNestedScript".to_string()),
            resolved_target_script: Some("RuntimeNestedScript".to_string()),
            source_script: "RuntimeCalledScript".to_string(),
            command_index: 0,
        },
        ScriptControlCommand {
            command: "end".to_string(),
            compare_value: None,
            target_label: None,
            resolved_target_script: None,
            source_script: "RuntimeCalledScript".to_string(),
            command_index: 2,
        },
        ScriptControlCommand {
            command: "end".to_string(),
            compare_value: None,
            target_label: None,
            resolved_target_script: None,
            source_script: "RuntimeNestedScript".to_string(),
            command_index: 1,
        },
    ]);
    for (source_script, command_index, value) in [
        ("RuntimeCallScript", 1, "33"),
        ("RuntimeCalledScript", 1, "22"),
        ("RuntimeNestedScript", 0, "11"),
    ] {
        map.script_variable_commands.push(ScriptVariableCommand {
            command: "setval".to_string(),
            target: None,
            value_tokens: vec![value.to_string()],
            source_script: source_script.to_string(),
            command_index,
        });
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");
    shell.session_mut().state.script_runtime.player_input_locked = true;

    let run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeCallScript".to_string(),
                command_index: 0,
            },
            16,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("nested compiled calls return to their exact callers");
    assert_eq!(
        run.steps
            .iter()
            .map(|step| (step.source_script.as_str(), step.command.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("RuntimeCallScript", "farscall"),
            ("RuntimeCalledScript", "scall"),
            ("RuntimeNestedScript", "setval"),
            ("RuntimeNestedScript", "end"),
            ("RuntimeCalledScript", "setval"),
            ("RuntimeCalledScript", "end"),
            ("RuntimeCallScript", "setval"),
            ("RuntimeCallScript", "end"),
        ]
    );
    assert!(run.ended);
    assert!(shell.session().state().script_runtime.call_stack.is_empty());
    assert_eq!(
        shell
            .session()
            .state()
            .script_runtime
            .script_value
            .as_deref(),
        Some("33")
    );
    assert!(!shell.session().state().script_runtime.player_input_locked);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn jumpstd_enters_the_exported_standard_script_through_the_compiled_interpreter() {
    let root = temp_repository_root("compiled-standard-script-step");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.story_events = vec![serde_json::json!({
        "StandardScripts": {
            "GlobalScriptRoots": [],
            "StdScripts": [
                {"command": "add_stdscript", "args": ["ElevatorButtonScript"]}
            ],
            "ElevatorButtonScript": [
                {"command": "playsound", "args": ["SFX_READ_TEXT_2"]},
                {"command": "pause", "args": ["15"]},
                {"command": "playsound", "args": ["SFX_ELEVATOR_END"]},
                {"command": "end", "args": []}
            ]
        }
    })];
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    map.scripts.insert(
        "RuntimeJumpStdScript".to_string(),
        serde_json::json!([
            {"command": "jumpstd", "args": ["ElevatorButtonScript"]}
        ]),
    );
    map.script_control_commands.push(ScriptControlCommand {
        command: "jumpstd".to_string(),
        compare_value: None,
        target_label: Some("ElevatorButtonScript".to_string()),
        resolved_target_script: None,
        source_script: "RuntimeJumpStdScript".to_string(),
        command_index: 0,
    });
    data.materialize_global_scripts()
        .expect("materialize the exported standard script");
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");

    let jump = shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeJumpStdScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step jumpstd");

    assert_eq!(
        jump.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "ElevatorButtonScript".to_string(),
            command_index: 0,
        })
    );
    assert!(shell.session.state().script_runtime.audio_events.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn standard_interaction_dialogue_uses_global_text_bodies() {
    let root = temp_repository_root("standard-interaction-dialogue");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.story_events = vec![serde_json::json!({
        "StandardScripts": {
            "GlobalScriptRoots": [],
            "StdScripts": [
                {"command": "add_stdscript", "args": ["TVScript"]},
                {"command": "add_stdscript", "args": ["TownMapScript"]},
                {"command": "add_stdscript", "args": ["PictureBookshelfScript"]}
            ],
            "TVScript": [
                {"command": "farjumptext", "args": ["TVText"]}
            ],
            "TownMapScript": [
                {"command": "farjumptext", "args": ["LookTownMapText"]}
            ],
            "PictureBookshelfScript": [
                {"command": "farjumptext", "args": ["PictureBookshelfText"]}
            ],
            "TVText": [
                {"command": "text", "args": ["It's a TV."]},
                {"command": "done", "args": []}
            ],
            "LookTownMapText": [
                {"command": "text", "args": ["It's a town map."]},
                {"command": "done", "args": []}
            ],
            "PictureBookshelfText": [
                {"command": "text", "args": ["A picture book."]},
                {"command": "done", "args": []}
            ]
        }
    })];
    data.materialize_global_scripts()
        .expect("materialize standard interaction scripts");
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0).expect("game shell");

    for (script, label) in [
        ("TVScript", "TVText"),
        ("TownMapScript", "LookTownMapText"),
        ("PictureBookshelfScript", "PictureBookshelfText"),
    ] {
        shell
            .step_compiled_script_command(
                "RuntimeMap",
                script,
                0,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("apply standard interaction dialogue");
        let text = runtime
            .active_text_snapshot(shell.session.state())
            .expect("resolve standard interaction dialogue")
            .expect("standard interaction dialogue");
        assert_eq!(text.label, label);
        assert_eq!(
            text.source,
            RuntimeTextSource::ScriptBody {
                map_name: "GlobalScripts".to_string()
            }
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_applies_script_object_and_movement_commands_with_persistent_state() {
    let root = temp_repository_root("script-object-movement");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_object_and_movement_commands(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");

    let moved = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 0)
        .expect("moveobject");
    assert_eq!(moved.outcome.object_id, "RUNTIME_NPC");
    assert_eq!((moved.outcome.x, moved.outcome.y), (Some(0), Some(0)));
    let turned = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 1)
        .expect("turnobject");
    assert_eq!(turned.outcome.object_id, "RUNTIME_NPC");
    assert_eq!(
        session.overworld.object_facings.get("RUNTIME_NPC"),
        Some(&Direction::Left)
    );

    let hidden = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 2)
        .expect("disappear");
    assert_eq!(
        hidden.outcome.event_flag.as_deref(),
        Some("EVENT_RUNTIME_NPC_HIDDEN")
    );
    assert!(session.state.flags.event_flags["EVENT_RUNTIME_NPC_HIDDEN"]);
    let shown = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 3)
        .expect("appear");
    assert_eq!(
        shown.outcome.event_flag.as_deref(),
        Some("EVENT_RUNTIME_NPC_HIDDEN")
    );
    assert!(!session.state.flags.event_flags["EVENT_RUNTIME_NPC_HIDDEN"]);

    let movement = session
        .apply_script_movement(&runtime, "RuntimeMap", "RuntimeObjectScript", 4)
        .expect("applymovement");
    assert_eq!(movement.outcome.previous_tile, TilePosition::new(0, 0));
    assert_eq!(movement.outcome.tile, TilePosition::new(1, 0));
    assert_eq!(movement.outcome.facing, Direction::Up);
    assert_eq!(
        movement
            .outcome
            .executed_steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["teleport_from", "step", "turn_head"]
    );
    assert_eq!(
        movement.outcome.steps_applied, 34,
        "ASM teleport_from is two 16-frame phases, followed by one step and one turn"
    );
    assert_eq!(
        movement.outcome.effects,
        vec![ScriptMovementEffect {
            command: "teleport_from".to_string(),
            index: 0,
        }]
    );
    assert!(session.state.script_runtime.teleport_from_queued);
    let consumed_flag = session
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::ConsumeScriptRuntimeFlag(RuntimeScriptRuntimeFlagCommand {
                flag: RuntimeScriptRuntimeFlag::TeleportFromQueued,
            }),
        )
        .expect("consume teleport-from movement effect");
    let RuntimeMutationResult::ScriptRuntimeFlagConsumed(consumed_flag) = consumed_flag.result
    else {
        panic!("consume script runtime flag must return flag value");
    };
    assert_eq!(
        consumed_flag,
        RuntimeScriptRuntimeFlagValue::TeleportFromQueued
    );
    assert!(!session.state.script_runtime.teleport_from_queued);
    let dispatch_runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_object_and_movement_commands(),
            report(),
        ),
        identity(),
    )
    .expect("dispatch runtime");
    let mut dispatch_shell = RuntimeGameShell::new_game(asset_root.clone(), dispatch_runtime, 0)
        .expect("dispatch game shell");
    let dispatched = dispatch_shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeObjectScript",
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs { accepted: None },
        )
        .expect("compiled applymovement dispatch");
    let RuntimeMutationResult::ScriptMovementApplied(dispatched_movement) = dispatched.result
    else {
        panic!("compiled applymovement must dispatch as script movement");
    };
    assert_eq!(dispatched_movement.tile, TilePosition::new(2, 0));
    assert_eq!(
        dispatched_movement
            .executed_steps
            .iter()
            .map(|step| step.command.as_str())
            .collect::<Vec<_>>(),
        vec!["teleport_from", "step", "turn_head"]
    );
    assert_eq!(dispatched_movement.steps_applied, 34);
    assert!(
        dispatch_shell
            .session
            .state()
            .script_runtime
            .teleport_from_queued
    );
    let written = dispatch_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeObjectScript",
            9,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("compiled writeobjectxy dispatch");
    let RuntimeMutationResult::ScriptObjectMutated(written_object) = written.mutation.result else {
        panic!("compiled writeobjectxy must dispatch as an object mutation");
    };
    assert_eq!((written_object.x, written_object.y), (Some(2), Some(0)));
    assert_eq!(
        dispatch_shell
            .session
            .state()
            .map_object_overrides
            .get("RuntimeMap")
            .and_then(|memory| memory.objects.get("RUNTIME_NPC"))
            .map(|object| (object.x, object.y)),
        Some((2, 0))
    );
    let map_object_index = session
        .overworld
        .objects
        .iter()
        .position(|object| object.object_identifier.as_deref() == Some("RUNTIME_NPC"))
        .map(|index| u8::try_from(index + 1).expect("map-object index fits one byte"))
        .expect("runtime NPC map object");
    let object_memory = session
        .state
        .map_object_overrides
        .get("RuntimeMap")
        .expect("current RuntimeMap object image");
    assert_eq!(
        object_memory
            .objects
            .get("RUNTIME_NPC")
            .map(|object| (object.x, object.y)),
        Some((0, 0)),
        "script movement does not write MAPOBJECT_X/Y"
    );
    assert_eq!(
        object_memory
            .object_structs
            .structs
            .iter()
            .find(|object| object.map_object_index == map_object_index)
            .map(|object| (object.live_tile, object.facing)),
        Some((TilePosition::new(1, 0), Some(Direction::Up))),
        "script movement writes live OBJECT_MAP_X/Y and OBJECT_DIRECTION"
    );

    let follow = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 5)
        .expect("follow");
    assert_eq!(follow.outcome.object_id, "RUNTIME_GUIDE");
    assert_eq!(
        session
            .state
            .map_object_overrides
            .get("RuntimeMap")
            .and_then(|memory| memory.following.as_ref())
            .map(|following| (following.leader_slot, following.follower_slot)),
        Some((Some(2), Some(0)))
    );

    let saved_state = session.state.clone();
    let resumed = RuntimeOverworldSession::from_state(&runtime, &asset_root, saved_state)
        .expect("resume object overrides");
    let resumed_npc = resumed
        .overworld
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some("RUNTIME_NPC"))
        .expect("resumed npc");
    assert_eq!((resumed_npc.x, resumed_npc.y), (0, 0));
    assert_eq!(
        resumed
            .overworld
            .object_runtime_tile_by_id("RUNTIME_NPC")
            .expect("resumed live npc tile"),
        TilePosition::new(1, 0)
    );
    assert_eq!(
        resumed.overworld.object_facings.get("RUNTIME_NPC"),
        Some(&Direction::Up)
    );
    assert_eq!(
        resumed.overworld.following,
        Some(OverworldFollowState {
            leader_slot: Some(2),
            follower_slot: Some(0),
        })
    );

    let stop = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 6)
        .expect("stopfollow");
    assert_eq!(stop.outcome.object_id, "FOLLOW");
    assert_eq!(
        session
            .state
            .map_object_overrides
            .get("RuntimeMap")
            .and_then(|memory| memory.following.as_ref()),
        None
    );
    assert_ne!(moved.state_checksum, stop.state_checksum);

    let bad_movement_case = session
        .apply_script_movement(&runtime, "RuntimeMap", "RuntimeObjectScript", 7)
        .expect_err("movement labels are exact");
    assert!(format!("{bad_movement_case:#}").contains("has no exact movement runtimenpcmovement"));
    let missing_command = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 99)
        .expect_err("object command indexes are exact");
    assert!(format!("{missing_command:#}").contains("has no script object command"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_applies_script_runtime_commands_with_exact_typed_inputs_and_divider_traces() {
    let root = temp_repository_root("script-runtime");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_runtime_commands(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let runtime_command_row = RuntimeScriptRuntimeCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "special".to_string(),
        args: vec!["FadeOutMusic".to_string()],
        source_script: "RuntimeCommandScript".to_string(),
        command_index: 0,
    };
    let wrong_runtime_command_row = RuntimeScriptRuntimeCommandKey {
        args: vec!["fadeoutmusic".to_string()],
        ..runtime_command_row.clone()
    };
    assert!(runtime.has_script_runtime_command(&runtime_command_row));
    assert!(!runtime.has_script_runtime_command(&wrong_runtime_command_row));
    assert!(
        runtime
            .script_runtime_command_keys()
            .contains(&runtime_command_row)
    );
    assert!(
        runtime
            .require_script_runtime_command(&runtime_command_row)
            .is_ok()
    );
    assert!(
        runtime
            .require_script_runtime_command(&wrong_runtime_command_row)
            .is_err()
    );

    let special = session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            0,
            ScriptRuntimeInputs::default(),
        )
        .expect("special");
    assert!(matches!(
        special.outcome,
        ScriptRuntimeOutcome::EffectRecorded {
            command,
            ..
        } if command == "special"
    ));
    assert_eq!(
        session.state.script_runtime.last_special_routine.as_deref(),
        Some("FadeOutMusic")
    );

    session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            1,
            ScriptRuntimeInputs::default(),
        )
        .expect("pause");
    assert_eq!(session.state.script_runtime.pending_delays[0].parameter, 15);
    assert_eq!(session.state.script_runtime.pending_delays[0].frames, 30);

    session.divider = crystal_core::random::RuntimeDividerSource::replay([6, 0]);
    let random = session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            2,
            ScriptRuntimeInputs::default(),
        )
        .expect("random");
    assert!(matches!(
        random.outcome,
        ScriptRuntimeOutcome::ScriptValueSet {
            value,
            ..
        } if value == "7"
    ));
    assert_eq!(
        session.state.random_state,
        crystal_core::random::CrystalRandomState { add: 7, sub: 0 }
    );

    let mut random_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("script random shell");
    random_shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([249, 0, 5, 0, 1, 0]);
    let replay_base = random_shell.session().clone();
    let before_random = random_shell.session().state().clone();
    let resolved = random_shell
        .apply_script_runtime_command(
            "RuntimeMap",
            "RuntimeCommandScript",
            2,
            ScriptRuntimeInputs::default(),
        )
        .expect("script random stages exact divider trace");
    assert!(matches!(
        resolved.outcome,
        ScriptRuntimeOutcome::ScriptValueSet { ref value, .. } if value == "0"
    ));
    assert_eq!(
        random_shell.session().state.random_state,
        crystal_core::random::CrystalRandomState { add: 0, sub: 255 }
    );
    let frame = random_shell.retained_runtime_commands()[0].clone();
    let command = crystal_assets::decode_runtime_mutation_command_frame(&frame, &before_random)
        .expect("decode recorded script random command");
    let RuntimeMutationCommand::ApplyRandomScriptRuntime(recorded) = command else {
        panic!("script random must journal a typed divider trace");
    };
    assert_eq!(recorded.divider_trace.samples, vec![249, 0, 5, 0, 1, 0]);

    let mut replayed = replay_base.clone();
    replayed
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("script random replay consumes the exact trace");
    assert_eq!(replayed.state, random_shell.session().state);

    for (trace, message) in [
        (
            vec![249, 0, 5, 0, 1],
            "divider replay exhausted after 5 samples",
        ),
        (
            vec![249, 0, 5, 0, 1, 0, 7],
            "1 unconsumed samples after 6 reads",
        ),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::ApplyRandomScriptRuntime(RuntimeRandomScriptCommand {
                    command: RuntimeScriptCommandRef::new("RuntimeMap", "RuntimeCommandScript", 2),
                    divider_trace: RuntimeDividerTrace::new(trace),
                }),
            )
            .expect("frame malformed script random replay");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("malformed script random trace rejects atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }

    let version = session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            3,
            ScriptRuntimeInputs::default(),
        )
        .expect("checkver");
    assert!(matches!(
        version.outcome,
        ScriptRuntimeOutcome::ScriptValueSet {
            value,
            ..
        } if value == "0"
    ));

    session.state.script_runtime.script_value = Some("12".to_string());
    session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            4,
            ScriptRuntimeInputs::default(),
        )
        .expect("writevar");
    assert_eq!(
        session
            .state
            .script_runtime
            .variables
            .get("VAR_BLUECARDBALANCE")
            .map(String::as_str),
        Some("12")
    );
    assert_eq!(session.state.blue_card_balance, 12);
    session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            5,
            ScriptRuntimeInputs::default(),
        )
        .expect("getnum");
    assert_eq!(
        session
            .state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("12")
    );

    let last_talked = session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            6,
            ScriptRuntimeInputs::default(),
        )
        .expect("setlasttalked");
    assert_eq!(
        session.overworld.last_talked_object_identifier.as_deref(),
        Some("RUNTIME_NPC")
    );
    assert_eq!(
        session
            .state
            .map_object_overrides
            .get("RuntimeMap")
            .and_then(|memory| memory.last_talked_object_identifier.as_deref()),
        Some("RUNTIME_NPC")
    );

    let emote = session
        .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 8)
        .expect("showemote");
    assert_eq!(session.state.script_runtime.pending_emotes.len(), 1);
    assert_eq!(
        session.state.script_runtime.pending_emotes[0].emote,
        "EMOTE_SHOCK"
    );
    assert_ne!(emote.state_checksum, last_talked.state_checksum);

    let bad_last_talked = session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            7,
            ScriptRuntimeInputs::default(),
        )
        .expect_err("object ids are exact");
    assert!(format!("{bad_last_talked:#}").contains("missing exact object id runtime_npc"));
    session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            8,
            ScriptRuntimeInputs::default(),
        )
        .expect("setlasttalked -1 clears the sentinel");
    assert_eq!(session.state.script_runtime.last_talked_object, None);
    assert_eq!(session.overworld.last_talked_object_identifier, None);
    assert_eq!(
        session
            .state
            .map_object_overrides
            .get("RuntimeMap")
            .and_then(|memory| memory.last_talked_object_identifier.as_ref()),
        None
    );
    let missing_index = session
        .apply_script_runtime_command(
            &runtime,
            "RuntimeMap",
            "RuntimeCommandScript",
            99,
            ScriptRuntimeInputs::default(),
        )
        .expect_err("runtime command indexes are exact");
    assert!(format!("{missing_index:#}").contains("has no script runtime command"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_shell_routes_compiled_special_commands_to_special_routines() {
    let root = temp_repository_root("compiled-special-command");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_runtime_commands();
    data.special_routines
        .insert("FadeOutMusic".to_string(), SpecialRoutineRule::default());
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");

    let outcome = shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeCommandScript",
            0,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("compiled special command applies real special routine");

    assert_eq!(outcome.result.result_tag(), "special_routine_applied");
    assert!(matches!(
        outcome.result,
        RuntimeMutationResult::SpecialRoutineApplied(SpecialRoutineOutcome {
            effect: SpecialRoutineEffect::FadeOutMusic { .. },
            ..
        })
    ));
    let snapshot = shell.snapshot().expect("snapshot after special");
    assert_eq!(
        snapshot.script_events.last_special_routine.as_deref(),
        Some("FadeOutMusic")
    );
    assert!(snapshot.audio.queued_events.iter().any(|event| {
        matches!(
            event.kind,
            crystal_core::state::ScriptAudioRuntimeKind::FadeMusic
        ) && event.audio_id.as_deref() == Some("MUSIC_NONE")
    }));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn incoming_phone_call_runs_global_caller_and_resumes_receive_wrapper() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let canonical_asset_root = AssetRoot::new(repo_root);
    let canonical = CrystalRuntime::load_from_compiled_pack(
        &canonical_asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load canonical phone scripts");
    let root = temp_repository_root("global-phone-caller");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.global_scripts = canonical.data().global_scripts.clone();
    data.phone_contacts = canonical.data().phone_contacts.clone();
    data.special_phone_calls = canonical.data().special_phone_calls.clone();
    data.initialize_events = canonical.data().initialize_events.clone();
    data.story_event_script_constants = canonical.data().story_event_script_constants.clone();
    data.asm_text = canonical.data().asm_text.clone();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");
    shell.session_mut().state.script_runtime.special_phone_call =
        Some("SPECIALCALL_BIKESHOP".to_string());
    shell
        .session_mut()
        .state
        .script_runtime
        .variables
        .insert("VAR_SPECIALPHONECALL".to_string(), "6".to_string());

    shell
        .tick([GameButton::Right])
        .expect("turn toward the next tile");
    let phone_frame = shell
        .tick([GameButton::Right])
        .expect("step and receive special call")
        .clone();
    let call = phone_frame.phone_call.expect("special call dispatch");
    assert_eq!(call.contact_id, "PHONE_OAK");
    assert_eq!(call.caller_script, "BikeShopPhoneCallerScript");
    assert_eq!(call.receive_script, "Script_ReceivePhoneCall");
    assert!(
        shell
            .session()
            .state
            .script_runtime
            .command_queue
            .is_empty()
    );
    assert_eq!(
        shell
            .session()
            .state
            .script_runtime
            .next_script
            .as_ref()
            .map(|next| next.script.as_str()),
        Some("Script_ReceivePhoneCall")
    );

    shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingDelay)
        .expect("complete special-call delay");
    let caller = shell
        .run_pending_next_script_until_boundary(
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run exported global caller");
    assert_eq!(caller.next_script.script, "Script_ReceivePhoneCall");
    assert_eq!(
        caller
            .run
            .steps
            .iter()
            .map(|step| (step.source_script.as_str(), step.command.as_str()))
            .collect::<Vec<_>>(),
        vec![("Script_ReceivePhoneCall", "reanchormap"),]
    );
    assert!(matches!(
        caller.run.boundary,
        Some(RuntimeCompiledScriptBoundary::PendingMapRefresh(_))
    ));
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapRefresh)
        .expect("complete receive-call map refresh");
    let caller_ring = shell
        .run_compiled_script_until_boundary(
            caller
                .run
                .next_cursor
                .expect("receive wrapper continuation"),
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run canonical incoming-call ring after map refresh");
    assert_eq!(
        caller_ring
            .steps
            .iter()
            .map(|step| (step.source_script.as_str(), step.command.as_str()))
            .collect::<Vec<_>>(),
        vec![("Script_ReceivePhoneCall", "callasm")]
    );
    assert!(matches!(
        caller_ring.boundary,
        Some(RuntimeCompiledScriptBoundary::PhoneCallasm(
            crystal_core::systems::script_runtime::ScriptPhoneCallasmPresentation::RingTwice
        ))
    ));
    let caller_after_refresh = shell
        .run_compiled_script_until_boundary(
            caller_ring.next_cursor.expect("post-ring continuation"),
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run dynamic caller after canonical ring");
    assert_eq!(
        caller_after_refresh
            .steps
            .iter()
            .map(|step| (step.source_script.as_str(), step.command.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("Script_ReceivePhoneCall", "memcall"),
            ("BikeShopPhoneCallerScript", "farwritetext"),
        ]
    );
    assert!(matches!(
        caller_after_refresh.boundary,
        Some(RuntimeCompiledScriptBoundary::TextLabel(ref label)) if label == "BikeShopPhoneCallerText"
    ));
    shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
        .expect("display caller text");
    let caller_wait = shell
        .run_compiled_script_until_boundary(
            caller_after_refresh
                .next_cursor
                .expect("caller text continuation cursor"),
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("finish exported global caller");
    assert_eq!(
        caller_wait
            .steps
            .iter()
            .map(|step| (step.source_script.as_str(), step.command.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("BikeShopPhoneCallerScript", "clearflag"),
            ("BikeShopPhoneCallerScript", "specialphonecall"),
            ("BikeShopPhoneCallerScript", "end"),
            ("Script_ReceivePhoneCall", "waitbutton"),
        ]
    );
    assert!(matches!(
        caller_wait.boundary,
        Some(RuntimeCompiledScriptBoundary::TextWait(ScriptTextWait {
            ref source_script,
            command_index: 3,
            ..
        })) if source_script == "Script_ReceivePhoneCall"
    ));
    assert!(!caller_wait.ended);
    assert_eq!(
        caller_wait.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "Script_ReceivePhoneCall".to_string(),
            command_index: 4,
        })
    );
    assert!(
        shell
            .session()
            .state
            .script_runtime
            .special_phone_call
            .is_none()
    );
    assert!(
        shell.session().state.script_runtime.call_stack.is_empty(),
        "the composed runner must return from the global caller before exposing the wrapper's waitbutton"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_applies_script_swarm_commands_with_exact_pack_rows() {
    let root = temp_repository_root("script-swarm");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_swarm_commands(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let swarm_row = RuntimeScriptSwarmCommandKey {
        map_name: "RuntimeMap".to_string(),
        command: "swarm".to_string(),
        swarm_token: "SWARM_YANMA".to_string(),
        map_id: "RUNTIME_MAP".to_string(),
        source_script: "RuntimeSwarmScript".to_string(),
        command_index: 0,
    };
    let absent_row = RuntimeScriptSwarmCommandKey {
        swarm_token: "swarm_yanma".to_string(),
        ..swarm_row.clone()
    };
    assert!(runtime.has_script_swarm_command(&swarm_row));
    assert!(!runtime.has_script_swarm_command(&absent_row));
    assert!(runtime.script_swarm_command_keys().contains(&swarm_row));
    assert!(runtime.require_script_swarm_command(&swarm_row).is_ok());
    assert!(runtime.require_script_swarm_command(&absent_row).is_err());

    let applied = session
        .apply_script_swarm_command(&runtime, "RuntimeMap", "RuntimeSwarmScript", 0)
        .expect("swarm");
    assert_eq!(applied.outcome.swarm_token, "SWARM_YANMA");
    assert_eq!(applied.outcome.map_id, "RUNTIME_MAP");
    assert_eq!(applied.outcome.map_group, Some(1));
    assert_eq!(applied.outcome.map_number, Some(1));
    assert_eq!(
        session.state.swarms.active.get("SWARM_YANMA"),
        Some(&SwarmMapTarget {
            map_id: "RUNTIME_MAP".to_string(),
            map_group: Some(1),
            map_number: Some(1),
        })
    );

    let invalid_command = session
        .apply_script_swarm_command(&runtime, "RuntimeMap", "RuntimeSwarmScript", 1)
        .expect_err("swarm command spelling is exact");
    assert!(
        format!("{invalid_command:#}").contains("InvalidCommand { command: \"Swarm\" }"),
        "{invalid_command:#}"
    );
    let missing_index = session
        .apply_script_swarm_command(&runtime, "RuntimeMap", "RuntimeSwarmScript", 99)
        .expect_err("swarm command indexes are exact");
    assert!(
        format!("{missing_index:#}").contains("has no script swarm command"),
        "{missing_index:#}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_overworld_checks_wild_encounters_after_successful_grass_step() {
    let root = temp_repository_root("overworld-encounter");
    write_grass_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_grass_encounter(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    add_runtime_party_pokemon(&runtime, &mut session);
    session.divider = crystal_core::random::RuntimeDividerSource::replay([
        0, 0, // encounter rate (Crystal Random: add - sub = 0)
        0, 0, // roaming selector
        0, 0, // slot
        0, 0, // level
        0, 0, // held item
        0, 0, // attack/defense
        0, 0, // speed/special
    ]);

    let turn = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");
    assert_eq!(turn.wild_encounter, None);

    let step = apply_until_wild_encounter_roll(&mut session, &runtime, &asset_root);
    assert_eq!(step.snapshot.tile, TilePosition::new(1, 0));
    assert_eq!(
        session
            .overworld
            .current_encounter_surface_checked()
            .expect("encounter surface"),
        Some(EncounterSurface::Grass)
    );
    let roll = step
        .wild_encounter
        .clone()
        .expect("100 percent grass should produce a roll");

    assert_eq!(roll.map_name, "RuntimeMap");
    assert_eq!(roll.time, session.state.time.time_of_day);
    assert_eq!(
        roll.resolved
            .clone()
            .unwrap_or_else(|| panic!("unresolved encounter roll {roll:?}"))
            .encounter
            .species,
        "CHIKORITA"
    );
    let battle = step.wild_battle.expect("resolved encounter starts battle");
    assert_eq!(battle.enemy_pokemon.species.id, "CHIKORITA");
    assert_eq!(battle.enemy_pokemon.moves[0].name, "TACKLE");
    assert_eq!(battle.enemy_pokemon.original_trainer_name, "WILD");
    assert_eq!(battle.encounter, roll);
    assert_eq!(
        session.state.battle,
        BattleMemory::Wild {
            roaming_slot: None,
            battle_type: battle.battle_type.clone(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE_NIGHT".to_string(),
            map_name: "RuntimeMap".to_string(),
            enemy_pokemon: battle.enemy_pokemon.clone(),
            enemy_party: battle.enemy_party.clone(),
        }
    );
    let saved_battle = session.state.battle.clone();
    let save_path = root.join("battle.crystalsave");
    runtime
        .save_game(&save_path, session.state.clone())
        .expect("save active battle");
    let loaded = runtime.load_save(&save_path).expect("load active battle");
    assert_eq!(loaded.battle, saved_battle);
    let resumed = runtime
        .resume_overworld_session(&asset_root, loaded)
        .expect("resume active battle");
    assert_eq!(resumed.state.battle, saved_battle);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_repel_item_blocks_lower_level_wild_encounter_after_real_step() {
    let root = temp_repository_root("overworld-repel-lower");
    write_grass_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_grass_encounter();
    let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
    repel.effect = "MOD_REPEL".to_string();
    repel.field_menu = "ITEMMENU_CLOSE".to_string();
    repel.field_usable = true;
    repel.consumable = true;
    repel.repel_steps = Some(100);
    data.items.insert("REPEL".to_string(), repel);
    let lower_level_encounter = WildEncounter {
        level: 3,
        species: "CHIKORITA".to_string(),
    };
    let lower_level_slots = vec![lower_level_encounter; 7];
    data.wild_encounters
        .get_mut("RuntimeMap")
        .expect("runtime wild encounters")
        .grass = Some(WildEncounterTable {
        morning: lower_level_slots.clone(),
        day: lower_level_slots.clone(),
        night: lower_level_slots,
    });
    let report = report_for(&data);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .state
        .bag
        .add_item(&runtime.data.items["REPEL"], 1)
        .expect("add Repel");

    let item_use = session
        .use_bag_repel_in_field(&runtime, "REPEL")
        .expect("use Repel");
    assert!(item_use.item_use.consumed);
    assert_eq!(item_use.repel_steps_before, 0);
    assert_eq!(item_use.repel_steps_after, 100);
    assert_eq!(item_use.active_repel_item_after, Some("REPEL".to_string()));
    assert!(!session.state.bag.has_item(&runtime.data.items["REPEL"]));
    let drained = session
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::DrainScriptEventQueue(RuntimeScriptEventDrainCommand {
                queue: RuntimeScriptEventQueue::ItemUse,
            }),
        )
        .expect("drain field item use event");
    assert!(matches!(
        drained.result,
        RuntimeMutationResult::ScriptEventQueueDrained(RuntimeScriptEventDrainResult::ItemUse(_))
    ));

    let turn = session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");
    assert_eq!(turn.wild_encounter, None);
    assert_eq!(session.state.repel_steps_remaining, 100);

    let step = apply_until_wild_encounter_roll(&mut session, &runtime, &asset_root);
    let roll = step
        .wild_encounter
        .clone()
        .expect("repelled grass still records the roll");
    assert_eq!(session.state.repel_steps_remaining, 95);
    assert_eq!(session.state.active_repel_item, Some("REPEL".to_string()));
    assert_eq!(roll.repelled_by, Some("REPEL".to_string()));
    assert_eq!(roll.resolved, None);
    assert_eq!(step.wild_battle, None);
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_ne!(
        session.state.random_state,
        crystal_core::random::CrystalRandomState::default()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_repel_does_not_block_same_or_higher_level_wild_encounter() {
    let root = temp_repository_root("overworld-repel-higher");
    write_grass_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_grass_encounter();
    let encounter = WildEncounter {
        level: 8,
        species: "CHIKORITA".to_string(),
    };
    let grass_slots = vec![encounter; 7];
    let wild = data
        .wild_encounters
        .get_mut("RuntimeMap")
        .expect("runtime wild encounters");
    wild.grass = Some(WildEncounterTable {
        morning: grass_slots.clone(),
        day: grass_slots.clone(),
        night: grass_slots,
    });
    let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
    repel.effect = "REPEL".to_string();
    repel.field_menu = "ITEMMENU_CLOSE".to_string();
    repel.field_usable = true;
    repel.consumable = true;
    repel.repel_steps = Some(100);
    data.items.insert("REPEL".to_string(), repel);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .state
        .bag
        .add_item(&runtime.data.items["REPEL"], 1)
        .expect("add Repel");
    session
        .use_bag_repel_in_field(&runtime, "REPEL")
        .expect("use Repel");
    let drained = session
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::DrainScriptEventQueue(RuntimeScriptEventDrainCommand {
                queue: RuntimeScriptEventQueue::ItemUse,
            }),
        )
        .expect("drain field item use event");
    assert!(matches!(
        drained.result,
        RuntimeMutationResult::ScriptEventQueueDrained(RuntimeScriptEventDrainResult::ItemUse(_))
    ));
    session
        .apply_buttons(&runtime, &asset_root, [GameButton::Right])
        .expect("turn right");

    let step = apply_until_wild_encounter_roll(&mut session, &runtime, &asset_root);
    let roll = step.wild_encounter.clone().expect("encounter roll");

    assert_eq!(session.state.repel_steps_remaining, 95);
    assert_eq!(roll.repelled_by, None);
    assert_eq!(roll.resolved.clone().expect("resolved").encounter.level, 8);
    assert!(step.wild_battle.is_some());
    assert!(matches!(session.state.battle, BattleMemory::Wild { .. }));
    let _ = std::fs::remove_dir_all(root);
}

fn apply_until_wild_encounter_roll(
    session: &mut RuntimeOverworldSession,
    runtime: &CrystalRuntime,
    asset_root: &AssetRoot,
) -> RuntimeOverworldFrame {
    for _ in 0..16 {
        let direction = if session.overworld.player.tile.x == 0 {
            GameButton::Right
        } else {
            GameButton::Left
        };
        let step = session
            .apply_buttons(runtime, asset_root, [direction])
            .expect("advance toward grass encounter");
        if step.wild_encounter.is_some() {
            return step;
        }
    }
    panic!("expected wild encounter roll after walking into grass");
}

fn runtime_data_with_escape_rope_maps() -> GameDataSet {
    let mut data = minimal_runtime_data_with_music();
    data.tilesets.insert(
        "johto".to_string(),
        test_tileset(&[("00", &["FLOOR", "WARP_PANEL", "FLOOR", "FLOOR"])]),
    );
    let source_warp = WarpEvent {
        index: 1,
        x: 1,
        y: 0,
        target_map_constant: "RUNTIME_CAVE".to_string(),
        target_map: "RUNTIME_CAVE".to_string(),
        target_warp_id: 1,
    };
    let cave_warp = WarpEvent {
        index: 1,
        x: 0,
        y: 0,
        target_map_constant: "RUNTIME_MAP".to_string(),
        target_map: "RUNTIME_MAP".to_string(),
        target_warp_id: 1,
    };
    data.maps
        .get_mut("RuntimeMap")
        .expect("runtime map")
        .events
        .warps = vec![source_warp.clone()];
    let mut cave = runtime_map();
    cave.id = "RuntimeCave".to_string();
    cave.attributes.environment = Some("cave".to_string());
    cave.attributes.map_constant = Some("RUNTIME_CAVE".to_string());
    cave.attributes.blocks_label = Some("RuntimeCave_Blocks".to_string());
    cave.attributes.map_scripts_label = Some("RuntimeCave_MapScripts".to_string());
    cave.attributes.map_events_label = Some("RuntimeCave_MapEvents".to_string());
    cave.events.warps = vec![cave_warp];
    data.maps.insert("RuntimeCave".to_string(), cave.clone());
    data.map_attributes.insert(
        "RuntimeMap".to_string(),
        data.maps
            .get("RuntimeMap")
            .expect("runtime map")
            .attributes
            .clone(),
    );
    data.map_attributes
        .insert("RuntimeCave".to_string(), cave.attributes.clone());
    data.map_scripts.insert(
        "RuntimeCave_MapScripts".to_string(),
        serde_json::json!({ "RuntimeScript": [] }),
    );
    data.map_scripts.insert(
        "RuntimeCave_MapEvents".to_string(),
        serde_json::json!([
            {"command":"def_warp_events","args":[]},
            {"command":"warp_event","args":["0","0","RUNTIME_MAP","1"]},
            {"command":"def_coord_events","args":[]},
            {"command":"def_bg_events","args":[]},
            {"command":"def_object_events","args":[]}
        ]),
    );
    data.map_blocks
        .insert("RuntimeCave_Blocks".to_string(), "00 00".to_string());
    data.npcs.insert(
        "RuntimeCave".to_string(),
        serde_json::json!({ "objects": [] }),
    );
    data.runtime_map_metadata.insert(
        "RUNTIME_MAP".to_string(),
        runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "ROUTE"),
    );
    data.runtime_map_metadata.insert(
        "RUNTIME_CAVE".to_string(),
        runtime_map_metadata("RUNTIME_CAVE", "RuntimeCave", 1, 2, "CAVE"),
    );
    data.runtime_spawn_points.insert(
        "1".to_string(),
        RuntimeSpawnPoint {
            identifier: 1,
            map_constant: "RUNTIME_CAVE".to_string(),
            map_name: "RuntimeCave".to_string(),
            group_id: 1,
            map_id: 2,
            tile_x: 0,
            tile_y: 0,
            group_name: "RUNTIME".to_string(),
            metatile_x: 0,
            metatile_y: 0,
            subtile_x: 0,
            subtile_y: 0,
        },
    );
    let mut escape_rope = runtime_item("ESCAPE_ROPE", item_pocket("ITEM"));
    escape_rope.effect = "ESCAPE_ROPE".to_string();
    escape_rope.field_menu = "ITEMMENU_CLOSE".to_string();
    escape_rope.field_usable = true;
    escape_rope.consumable = true;
    escape_rope.escape_rope_mode = Some("DIG_WARP".to_string());
    data.items.insert("ESCAPE_ROPE".to_string(), escape_rope);
    add_map_name_sign_landmark_for_fixture(&mut data, 5, "LANDMARK_RUNTIME_CAVE", "RuntimeCave");
    data
}

fn step_right_until_warp(
    session: &mut RuntimeOverworldSession,
    runtime: &CrystalRuntime,
    asset_root: &AssetRoot,
) -> RuntimeOverworldFrame {
    for _ in 0..8 {
        let frame = session
            .apply_buttons(runtime, asset_root, [GameButton::Right])
            .expect("advance onto warp");
        if frame.warp.is_some() {
            return frame;
        }
    }
    panic!("rightward input did not reach the adjacent warp");
}

#[test]
fn runtime_escape_rope_uses_saved_dig_warp_without_fallback_destination() {
    let root = temp_repository_root("escape-rope");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = runtime_data_with_escape_rope_maps();
    data.field_moves.escape_rope.escape_rope_mode = "MOD_WARP".to_string();
    {
        let escape_rope = data.items.get_mut("ESCAPE_ROPE").expect("escape rope");
        escape_rope.effect = "MOD_ESCAPE".to_string();
        escape_rope.escape_rope_mode = Some("MOD_WARP".to_string());
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    session
        .state
        .bag
        .add_item(&runtime.data.items["ESCAPE_ROPE"], 1)
        .expect("add Escape Rope");

    let warp_step = step_right_until_warp(&mut session, &runtime, &asset_root);
    assert_eq!(
        warp_step
            .warp
            .expect("warp transition")
            .destination
            .map_name,
        "RuntimeCave"
    );
    assert_eq!(session.overworld.map.name, "RuntimeCave");
    assert_eq!(
        session.state.dig_warp_map_name,
        Some("RuntimeMap".to_string())
    );
    assert_eq!(session.state.dig_warp_index, Some(1));
    let source_snapshot = session.overworld.snapshot();

    let escape = session
        .use_bag_escape_rope_in_field(&runtime, &asset_root, "ESCAPE_ROPE")
        .expect("Escape Rope uses saved dig warp");

    assert!(escape.item_use.consumed);
    assert_eq!(escape.source_map, "RuntimeCave");
    assert_eq!(escape.destination_map, "RuntimeMap");
    assert_eq!(escape.destination_warp_index, 1);
    assert_eq!(escape.destination_tile, TilePosition::new(1, 0));
    assert_eq!(session.overworld.snapshot(), source_snapshot);
    let pending = session
        .state
        .script_runtime
        .pending_field_travel
        .as_ref()
        .expect("Escape Rope travel remains pending through DigOut");
    assert_eq!(pending.move_id, "ESCAPE_ROPE");
    assert_eq!(pending.source_map, "RuntimeCave");
    assert_eq!(pending.destination_map, "RuntimeMap");

    let committed = session
        .commit_pending_field_travel(&runtime)
        .expect("commit Escape Rope at the shared source warp boundary");
    assert_eq!(committed.move_id, "ESCAPE_ROPE");
    assert_eq!(session.overworld.map.name, "RuntimeMap");
    assert_eq!(session.overworld.player.tile, TilePosition::new(1, 0));
    assert!(
        !session
            .state
            .bag
            .has_item(&runtime.data.items["ESCAPE_ROPE"])
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_escape_rope_rejects_missing_dig_warp_without_consumption() {
    let root = temp_repository_root("escape-rope-missing-dig");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = runtime_data_with_escape_rope_maps();
    data.field_moves.dig.move_id = "TELEPORT".to_string();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 1)
        .expect("overworld session");
    session
        .state
        .bag
        .add_item(&runtime.data.items["ESCAPE_ROPE"], 1)
        .expect("add Escape Rope");
    let before = session.state.clone();

    let error = session
        .use_bag_escape_rope_in_field(&runtime, &asset_root, "ESCAPE_ROPE")
        .expect_err("missing dig warp rejected");

    let error = error_debug(error);
    assert!(error.contains("has no saved dig warp map"), "{error}");
    assert_eq!(session.state, before);
    assert!(
        session
            .state
            .bag
            .has_item(&runtime.data.items["ESCAPE_ROPE"])
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_dig_field_move_uses_saved_dig_warp_without_fallback_destination() {
    let root = temp_repository_root("field-move-dig");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = runtime_data_with_escape_rope_maps();
    data.field_moves.dig.move_id = "TELEPORT".to_string();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    player.moves = vec![LearnedMove {
        name: "TELEPORT".to_string(),
        current_pp: 10,
        pp_ups: 0,
    }];
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    let _ = step_right_until_warp(&mut session, &runtime, &asset_root);
    assert_eq!(session.overworld.map.name, "RuntimeCave");
    assert_eq!(
        session.state.dig_warp_map_name,
        Some("RuntimeMap".to_string())
    );
    assert_eq!(session.state.dig_warp_index, Some(1));
    let source_snapshot = session.overworld.snapshot();

    let dig = session
        .use_dig_field_move(&runtime, &asset_root, 0)
        .expect("DIG uses saved dig warp");

    assert_eq!(dig.actor_party_index, 0);
    assert_eq!(dig.actor_species, "CHIKORITA");
    assert_eq!(dig.source_map, "RuntimeCave");
    assert_eq!(dig.destination_map, "RuntimeMap");
    assert_eq!(dig.destination_warp_index, 1);
    assert_eq!(dig.destination_tile, TilePosition::new(1, 0));
    assert_eq!(session.overworld.snapshot(), source_snapshot);
    let pending = session
        .state
        .script_runtime
        .pending_field_travel
        .as_ref()
        .expect("DIG travel remains pending through the departure script");
    assert_eq!(pending.move_id, "TELEPORT");
    assert_eq!(pending.source_map, "RuntimeCave");
    assert_eq!(pending.destination_map, "RuntimeMap");

    let committed = session
        .commit_pending_field_travel(&runtime)
        .expect("commit DIG at the source warp boundary");
    assert_eq!(committed.move_id, "TELEPORT");
    assert_eq!(session.overworld.map.name, "RuntimeMap");
    assert_eq!(session.overworld.player.tile, TilePosition::new(1, 0));
    assert!(session.state.script_runtime.pending_field_travel.is_none());
    assert_eq!(
        session.state.overworld,
        OverworldMemory::from_snapshot(&session.overworld.snapshot())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_dig_field_move_rejects_missing_dig_warp_without_mutation() {
    let root = temp_repository_root("field-move-dig-missing-warp");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/music/MUSIC_ROUTE_29.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(runtime_data_with_escape_rope_maps(), report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 1)
        .expect("overworld session");
    let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    player.moves = vec![LearnedMove {
        name: "DIG".to_string(),
        current_pp: 10,
        pp_ups: 0,
    }];
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    let before_state = session.state.clone();
    let before_snapshot = session.overworld.snapshot();

    let error = session
        .use_dig_field_move(&runtime, &asset_root, 0)
        .expect_err("missing dig warp rejected");
    let error = error_debug(error);

    assert!(error.contains("DIG field move has no saved dig warp map"));
    assert_eq!(session.state, before_state);
    assert_eq!(session.overworld.snapshot(), before_snapshot);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_casts_fishing_rod_from_current_map_compiled_group() {
    let root = temp_repository_root("fishing-battle");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_fishing(), report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    add_runtime_party_pokemon(&runtime, &mut session);
    session.divider = crystal_core::random::RuntimeDividerSource::replay([0; 10]);

    let before_bad_rod_state = session.state.clone();
    let before_bad_rod_snapshot = session.overworld.snapshot();
    let bad_rod = session
        .cast_fishing_rod(&runtime, "good_rod")
        .expect_err("rod ids are exact");
    assert!(format!("{bad_rod:#}").contains("unknown fishing rod 'good_rod'"));
    assert_eq!(session.state, before_bad_rod_state);
    assert_eq!(session.overworld.snapshot(), before_bad_rod_snapshot);

    let cast = session
        .cast_fishing_rod(&runtime, ROD_GOOD)
        .expect("cast good rod");

    assert_eq!(cast.session.group.as_deref(), Some("FISHGROUP_RUNTIME"));
    assert_eq!(cast.bite, Some(true));
    assert_eq!(session.state.fishing.rod_state, FishingRodState::Battle);
    assert_eq!(session.state.fishing.rod_index, Some(1));
    let battle = cast.wild_battle.expect("fishing starts battle");
    assert_eq!(battle.enemy_pokemon.species.id, "CHIKORITA");
    assert_eq!(battle.enemy_pokemon.level, 9);
    assert_eq!(battle.encounter.surface, EncounterSurface::Water);
    assert_eq!(session.state.battle, BattleMemory::from(&battle));
    assert!(session.state.pokedex.has_seen("CHIKORITA"));
    assert_eq!(session.state.battle_active_party_index, Some(0));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_fishing_allows_an_npc_on_the_facing_water_tile() {
    let root = temp_repository_root("fishing-on-npc");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_fishing(), report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    add_runtime_party_pokemon(&runtime, &mut session);
    session.divider = crystal_core::random::RuntimeDividerSource::replay([0; 10]);
    let target = crystal_core::world::movement::checked_move_by_stride(
        session.overworld.player.tile,
        session.overworld.player.facing,
        crystal_core::world::movement::StepOptions::default().stride_tiles,
    )
    .expect("facing water tile");
    session
        .overworld
        .objects
        .push(runtime_object("FISH_NPC", "EVENT_FISH_NPC"));
    session
        .overworld
        .object_runtime_tiles
        .insert("FISH_NPC".to_string(), target);
    let cast = session
        .cast_fishing_rod(&runtime, ROD_GOOD)
        .expect("the source bug skips CheckFacingObject");

    assert_eq!(cast.bite, Some(true));
    assert!(cast.wild_battle.is_some());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_fishing_records_and_atomically_replays_the_exact_divider_trace() {
    let root = temp_repository_root("fishing-divider-trace");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_fishing(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("game shell");
    add_runtime_party_pokemon(&runtime, shell.session_mut());
    shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([0; 10]);
    let replay_base = shell.session().clone();
    let before = shell.session().state().clone();

    shell
        .cast_fishing_rod(ROD_GOOD)
        .expect("stage exact fishing cast");

    let frame = shell.retained_runtime_commands()[0].clone();
    let command = crystal_assets::decode_runtime_mutation_command_frame(&frame, &before)
        .expect("decode fishing command");
    let RuntimeMutationCommand::CastFishingRod(recorded) = command else {
        panic!("fishing cast must retain a typed divider trace");
    };
    assert_eq!(recorded.divider_trace.samples, vec![0; 10]);

    let mut replayed = replay_base.clone();
    replayed
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("replay exact fishing trace");
    assert_eq!(replayed.state, shell.session().state);

    for (samples, message) in [
        (vec![0; 9], "divider replay exhausted after 9 samples"),
        (vec![0; 11], "1 unconsumed samples after 10 reads"),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::CastFishingRod(RuntimeFishingCommand {
                    rod: ROD_GOOD.to_string(),
                    divider_trace: RuntimeDividerTrace::new(samples),
                }),
            )
            .expect("frame fishing replay command");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("invalid fishing trace must reject atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_fishing_no_bite_records_only_one_random_call() {
    let root = temp_repository_root("fishing-no-bite-divider-trace");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_fishing();
    data.fishing
        .groups
        .get_mut("FISHGROUP_RUNTIME")
        .expect("runtime fishing group")
        .bite_threshold = 0;
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");
    shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([0, 0]);
    let before = shell.session().state().clone();

    let cast = shell.cast_fishing_rod(ROD_GOOD).expect("no-bite cast");

    assert_eq!(cast.bite, Some(false));
    let command = crystal_assets::decode_runtime_mutation_command_frame(
        &shell.retained_runtime_commands()[0],
        &before,
    )
    .expect("decode no-bite fishing command");
    let RuntimeMutationCommand::CastFishingRod(recorded) = command else {
        panic!("no-bite fishing cast must retain a typed divider trace");
    };
    assert_eq!(recorded.divider_trace.samples, vec![0, 0]);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_uses_bag_fishing_rod_item_from_definitive_item_id() {
    let root = temp_repository_root("fishing-bag-item");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_fishing();
    let mut good_rod = runtime_item("GOOD_ROD", item_pocket("KEY_ITEM"));
    good_rod.effect = "MOD_GOOD_ROD".to_string();
    good_rod.field_menu = "ITEMMENU_CLOSE".to_string();
    good_rod.field_usable = true;
    data.items.insert("GOOD_ROD".to_string(), good_rod);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    add_runtime_party_pokemon(&runtime, &mut session);
    session.divider = crystal_core::random::RuntimeDividerSource::replay([0; 10]);
    session
        .state
        .bag
        .add_item(&runtime.data.items["GOOD_ROD"], 1)
        .expect("add good rod");

    let use_rod = session
        .use_bag_fishing_rod_in_field(&runtime, "GOOD_ROD")
        .expect("use good rod item");

    assert_eq!(use_rod.item_use.item_id, "GOOD_ROD");
    assert!(!use_rod.item_use.consumed);
    assert_eq!(use_rod.rod, ROD_GOOD);
    assert_eq!(
        use_rod.cast.session.group.as_deref(),
        Some("FISHGROUP_RUNTIME")
    );
    assert_eq!(use_rod.cast.bite, Some(true));
    assert_eq!(session.state.fishing.rod_state, FishingRodState::Battle);
    assert_eq!(session.state.fishing.rod_index, Some(1));
    assert_eq!(
        session.state.battle,
        BattleMemory::from(&use_rod.cast.wild_battle.expect("battle"))
    );
    assert_eq!(
        session.state.bag.quantity(&runtime.data.items["GOOD_ROD"]),
        1
    );
    assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
    assert_eq!(
        session.state.script_runtime.item_use_events[0].item_id,
        "GOOD_ROD"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_bag_fishing_rod_rejects_missing_or_case_changed_item_id_without_mutation() {
    let root = temp_repository_root("fishing-bag-item-reject");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_fishing();
    let mut missing_rod = runtime_item("GOOD_ROD", item_pocket("KEY_ITEM"));
    missing_rod.effect = "MOD_GOOD_ROD".to_string();
    missing_rod.field_menu = "ITEMMENU_CLOSE".to_string();
    missing_rod.field_usable = true;
    let mut bad_case_rod = runtime_item("BAD_CASE_ROD", item_pocket("KEY_ITEM"));
    bad_case_rod.effect = "good_rod".to_string();
    bad_case_rod.field_menu = "ITEMMENU_CLOSE".to_string();
    bad_case_rod.field_usable = true;
    data.items.insert("GOOD_ROD".to_string(), missing_rod);
    data.items.insert("BAD_CASE_ROD".to_string(), bad_case_rod);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let before_missing = session.state.clone();
    let missing = session
        .use_bag_fishing_rod_in_field(&runtime, "GOOD_ROD")
        .expect_err("missing rod rejects before cast");
    let missing = error_debug(missing);
    assert!(missing.contains("not in the bag"), "{missing}");
    assert_eq!(session.state, before_missing);

    session
        .state
        .bag
        .add_item(&runtime.data.items["BAD_CASE_ROD"], 1)
        .expect("add bad case rod");
    let before_bad_case = session.state.clone();
    let bad_case = session
        .use_bag_fishing_rod_in_field(&runtime, "BAD_CASE_ROD")
        .expect_err("case changed rod item id rejects");
    let bad_case = error_debug(bad_case);
    assert!(
        bad_case.contains("not declared by exact fishing rod item rules"),
        "{bad_case}"
    );
    assert_eq!(session.state, before_bad_case);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_fishing_without_map_group_does_not_fabricate_encounters() {
    let root = temp_repository_root("fishing-no-group");
    write_fishing_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.tilesets.insert(
        "johto".to_string(),
        test_tileset(&[("00", &["FLOOR", "FLOOR", "WATER", "FLOOR"])]),
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");

    let cast = session
        .cast_fishing_rod(&runtime, ROD_GOOD)
        .expect("cast good rod without group");

    assert_eq!(cast.session.group, None);
    assert_eq!(cast.bite, Some(false));
    assert_eq!(cast.wild_battle, None);
    assert_eq!(session.state.fishing.result, 0);
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_fishing_requires_facing_water_and_rejects_surfing_without_mutation() {
    let root = temp_repository_root("fishing-terrain");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_fishing();
    // Compiled pack tilesets are authoritative over repository files.
    // This case deliberately faces land for the first rejection.
    data.tilesets.insert(
        "johto".to_string(),
        test_tileset(&[("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"])]),
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let before_land = session.state.clone();
    let land_error = session
        .cast_fishing_rod(&runtime, ROD_GOOD)
        .expect_err("land-facing cast rejected");
    assert!(format!("{land_error:#}").contains("facing tile is not water"));
    assert_eq!(session.state, before_land);

    // Surfing is rejected before terrain lookup, matching the field-item
    // command ordering; no test-only terrain mutation is involved.
    session.overworld.player.mode = MovementMode::Surf;
    let before_surf = session.state.clone();
    let surf_error = session
        .cast_fishing_rod(&runtime, ROD_GOOD)
        .expect_err("surfing cast rejected");
    assert!(format!("{surf_error:#}").contains("surfing movement mode"));
    assert_eq!(session.state, before_surf);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_starts_scripted_wild_battle_from_exact_map_script_command() {
    let root = temp_repository_root("scripted-wild-battle");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    map.scripts.insert(
        "RuntimeWildScript".to_string(),
        serde_json::json!([
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "loadwildmon", "args": ["CHIKORITA", "6"]},
            {"command": "startbattle", "args": []},
            {"command": "setval", "args": ["7"]}
        ]),
    );
    map.scripts.insert(
        "AlternateWildScript".to_string(),
        serde_json::json!([
            {"command": "loadwildmon", "args": ["CHIKORITA", "6"]},
            {"command": "startbattle", "args": []},
            {"command": "setval", "args": ["9"]}
        ]),
    );
    map.script_map_commands
        .retain(|command| command.source_script != "RuntimeWildScript");
    map.script_flag_commands
        .retain(|command| command.source_script != "RuntimeWildScript");
    map.script_object_commands
        .retain(|command| command.source_script != "RuntimeWildScript");
    let mut alternate = map
        .scripted_wild_battles
        .first()
        .expect("base scripted wild battle")
        .clone();
    alternate.source_script = "AlternateWildScript".to_string();
    alternate.loadwildmon_command_index = 0;
    alternate.startbattle_command_index = 1;
    alternate.request.source_script = "AlternateWildScript".to_string();
    map.scripted_wild_battles.push(alternate);
    map.script_variable_commands.push(ScriptVariableCommand {
        command: "setval".to_string(),
        target: None,
        value_tokens: vec!["7".to_string()],
        source_script: "RuntimeWildScript".to_string(),
        command_index: 5,
    });
    map.script_runtime_commands.push(ScriptRuntimeCommand {
        command: "loadwildmon".to_string(),
        args: vec!["CHIKORITA".to_string(), "6".to_string()],
        source_script: "RuntimeWildScript".to_string(),
        command_index: 3,
    });
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut missing_setup_shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
            .expect("missing-setup shell");
    let before_missing_setup = missing_setup_shell
        .snapshot()
        .expect("snapshot before rejected startbattle");
    let missing_setup_error = missing_setup_shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeWildScript",
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect_err("startbattle must not synthesize skipped loadwildmon setup");
    assert!(
        format!("{missing_setup_error:#}")
            .contains("requires wBattleScriptFlags=128, found <unset>")
    );
    assert_eq!(
        missing_setup_shell
            .snapshot()
            .expect("snapshot after rejected startbattle"),
        before_missing_setup
    );
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut player =
        static_wild_battle_start_for_tests(&runtime.data, static_wild_request("CHIKORITA", 8))
            .enemy_pokemon;
    player.original_trainer_name = "PLAYER".to_string();
    session
        .state
        .storage
        .register_capture_in_box(0, player.clone())
        .expect("register player");
    session.state.sync_party_from_storage();

    let start = session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("scripted wild battle starts");
    let mut dispatch_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("dispatch wild shell");
    dispatch_shell
        .session_mut()
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register dispatch player");
    dispatch_shell.session_mut().state.sync_party_from_storage();
    dispatch_shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([
        0, 64, // held-item gate -> 192
        0, 173, // rare-item roll -> 19
        0, 3, // packed Attack/Defense byte -> 0x10
        0, 101, // packed Speed/Special byte -> 0xab
    ]);
    dispatch_shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeWildScript",
            3,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs { accepted: None },
        )
        .expect("compiled loadwildmon setup");
    let start_replay_base = dispatch_shell.session().clone();
    let retained_before_start = dispatch_shell.retained_runtime_commands().len();
    let dispatched = dispatch_shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeWildScript",
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs { accepted: None },
        )
        .expect("compiled scripted wild battle dispatch");
    let RuntimeMutationResult::ScriptedWildBattleStarted(dispatched_start) = dispatched.result
    else {
        panic!("compiled startbattle must dispatch as scripted wild battle");
    };
    assert_eq!(dispatched_start.species, "CHIKORITA");
    assert_eq!(dispatched_start.level, 6);
    assert_eq!(
        dispatched_start.enemy_pokemon.dvs,
        Dv::from_non_hp(1, 0, 10, 11)
    );
    let start_frame = &dispatch_shell.retained_runtime_commands()[retained_before_start];
    let recorded_start = crystal_assets::decode_runtime_mutation_command_frame(
        start_frame,
        start_replay_base.state(),
    )
    .expect("decode recorded static battle start");
    let RuntimeMutationCommand::StartScriptedWildBattle(recorded_start) = recorded_start else {
        panic!("compiled startbattle must journal the exact typed start command");
    };
    assert_eq!(
        recorded_start.divider_trace.samples,
        vec![0, 64, 0, 173, 0, 3, 0, 101]
    );

    let start_command_with_trace = |samples: Vec<u8>| {
        RuntimeMutationCommand::StartScriptedWildBattle(RuntimeScriptedWildBattleStartCommand {
            command: recorded_start.command.clone(),
            divider_trace: RuntimeDividerTrace::new(samples),
        })
    };
    let mut short_start = start_replay_base.clone();
    let short_request = short_start
        .runtime_command_frame(1, 1, start_command_with_trace(vec![0, 64, 0, 173, 0, 3, 0]))
        .expect("short start frame");
    let short_before = short_start.clone();
    let short_error = short_start
        .apply_runtime_command_frame(&runtime, &short_request)
        .expect_err("truncated static-start divider trace must reject atomically");
    assert!(
        format!("{short_error:#}").contains("divider replay exhausted after 7 samples"),
        "{short_error:#}"
    );
    assert_eq!(short_start, short_before);

    let mut tailed_start = start_replay_base.clone();
    let tailed_request = tailed_start
        .runtime_command_frame(
            1,
            1,
            start_command_with_trace(vec![0, 64, 0, 173, 0, 3, 0, 101, 77]),
        )
        .expect("tailed start frame");
    let tailed_before = tailed_start.clone();
    let tail_error = tailed_start
        .apply_runtime_command_frame(&runtime, &tailed_request)
        .expect_err("unused static-start divider tail must reject atomically");
    assert!(
        format!("{tail_error:#}").contains("1 unconsumed samples after 8 reads"),
        "{tail_error:#}"
    );
    assert_eq!(tailed_start, tailed_before);

    let mut exact_start = start_replay_base;
    let exact_outcome = exact_start
        .apply_runtime_command_frame(&runtime, start_frame)
        .expect("the recorded start replays exactly once");
    assert_eq!(exact_start.state(), dispatch_shell.session().state());
    assert_eq!(
        exact_outcome.state_checksum,
        game_state_checksum(dispatch_shell.session().state()).expect("started state checksum")
    );

    assert_eq!(start.species, "CHIKORITA");
    assert_eq!(start.level, 6);
    assert_eq!(
        session
            .state
            .flags
            .is_event_flag_set("EVENT_RUNTIME_WILD_READY"),
        Ok(false)
    );
    let origin = static_wild_origin_from_state(&session.state);
    assert_eq!(origin.map_name, "RuntimeMap");
    assert_eq!(origin.source_script, "RuntimeWildScript");
    assert_eq!(origin.startbattle_command_index, 4);
    assert_eq!(origin.resume_command_index, 5);
    assert!(session.state.pokedex.has_seen("CHIKORITA"));
    assert_eq!(session.state.battle_active_party_index, Some(0));

    // Capture/flee deactivate the battle before the script cursor resumes.
    // The persisted terminal is the sole authority for both origin and
    // deferred Pay Day cleanup, and it blocks every new battle first.
    let mut pending_terminal = session.clone();
    pending_terminal.state.money = 400;
    pending_terminal.state.battle_pay_day_money = 65;
    pending_terminal.state.battle_amulet_coin_active = true;
    crystal_core::battle::start::deactivate_battle_after_win(&mut pending_terminal.state);
    assert_eq!(
        pending_terminal
            .state
            .pending_static_wild_terminal
            .as_ref()
            .expect("capture/flee terminal persists")
            .pay_day_payout,
        130
    );
    pending_terminal.divider = crystal_core::random::RuntimeDividerSource::replay([]);
    let blocked_before = pending_terminal.clone();
    let blocked = pending_terminal
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect_err("a pending static-wild terminal must block the next battle before RNG");
    assert!(
        format!("{blocked:#}").contains("pending static-wild terminal resumes"),
        "{blocked:#}"
    );
    assert_eq!(pending_terminal, blocked_before);

    let forged_origin = RuntimeStaticWildBattleOrigin {
        map_name: "RuntimeMap".to_string(),
        source_script: "AlternateWildScript".to_string(),
        startbattle_command_index: 1,
        resume_command_index: 2,
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        species: "CHIKORITA".to_string(),
        level: 6,
    };
    let forged_before = pending_terminal.clone();
    let forged = pending_terminal
        .complete_scripted_wild_battle(&runtime, forged_origin)
        .expect_err("a different pack-valid origin must not consume the pending terminal");
    assert!(
        format!("{forged:#}").contains("persisted scripted wild terminal does not match"),
        "{forged:#}"
    );
    assert_eq!(pending_terminal, forged_before);

    pending_terminal.divider = crystal_core::random::RuntimeDividerSource::replay([0, 200]);
    pending_terminal
        .complete_scripted_wild_battle(&runtime, origin.clone())
        .expect("capture/flee terminal performs cleanup once");
    assert_eq!(pending_terminal.state.money, 530);
    assert!(
        pending_terminal
            .state
            .pending_static_wild_terminal
            .is_none()
    );

    session.state.money = 400;
    session.state.battle_pay_day_money = 65;
    crystal_core::battle::start::deactivate_battle_after_win(&mut session.state);
    let completion = session
        .complete_scripted_wild_battle(&runtime, origin.clone())
        .expect("scripted wild battle completes");
    assert!(completion.continued_after_battle);
    assert_eq!(
        session
            .state
            .flags
            .is_event_flag_set("EVENT_RUNTIME_STATIC_MON_HIDDEN"),
        Ok(false),
        "completion alone must not execute the source disappear command"
    );
    assert_eq!(
        session
            .state
            .flags
            .is_event_flag_set("EVENT_RUNTIME_WILD_DONE"),
        Ok(false),
        "completion alone must not execute the source setevent command"
    );
    assert_eq!(
        session
            .state
            .flags
            .is_engine_flag_set("ENGINE_RUNTIME_WILD_DONE"),
        Ok(false),
        "completion alone must not execute the source setflag command"
    );
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.money, 465);
    assert_eq!(session.state.battle_pay_day_money, 0);
    assert_eq!(session.state.battle_active_party_index, None);

    let mut battle_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("battle runner shell");
    let mut battle_player =
        static_wild_battle_start_for_tests(&runtime.data, static_wild_request("CHIKORITA", 8))
            .enemy_pokemon;
    battle_player.original_trainer_name = "PLAYER".to_string();
    battle_player.pokerus = 0xa2;
    battle_shell
        .session_mut()
        .state
        .storage
        .register_capture_in_box(0, battle_player)
        .expect("register battle runner player");
    battle_shell.session_mut().state.sync_party_from_storage();
    battle_shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([0, 64, 0, 173, 0, 3, 0, 101]);
    battle_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeWildScript",
            3,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step scripted loadwildmon setup");
    let start_step = battle_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeWildScript",
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step scripted wild startbattle");
    assert!(matches!(
        start_step.boundary,
        Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::StaticWildBattle
        ))
    ));
    assert_eq!(
        start_step.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeWildScript".to_string(),
            command_index: 5,
        })
    );
    let battle_origin = static_wild_origin_from_state(battle_shell.session().state());
    crystal_core::battle::start::deactivate_battle_after_win(&mut battle_shell.session_mut().state);
    battle_shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([0, 200]);
    let replay_base = battle_shell.session().clone();
    let before_completion = battle_shell.session().state().clone();
    let retained_before_completion = battle_shell.retained_runtime_commands().len();
    let resumed_battle = battle_shell
        .complete_scripted_wild_battle_and_run_compiled_script(
            battle_origin.clone(),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("complete wild battle and continue script");
    assert_eq!(
        battle_shell.session().state().random_state,
        crystal_core::random::CrystalRandomState { add: 0, sub: 227 }
    );
    let completion_frame = &battle_shell.retained_runtime_commands()[retained_before_completion];
    let completion_command =
        crystal_assets::decode_runtime_mutation_command_frame(completion_frame, &before_completion)
            .expect("decode recorded completion against its pre-mutation state");
    let RuntimeMutationCommand::CompleteScriptedWildBattle(command) = completion_command else {
        panic!("first post-battle command must be the recorded completion");
    };
    assert_eq!(command.divider_trace.samples, vec![0, 200]);

    let completion_command_with_trace = |samples: Vec<u8>| {
        RuntimeMutationCommand::CompleteScriptedWildBattle(
            RuntimeScriptedWildBattleCompletionCommand {
                origin: battle_origin.clone(),
                terminal: RuntimeScriptedWildBattleTerminal {
                    battle_result: 0,
                    win_cleanup_applied: false,
                },
                divider_trace: RuntimeDividerTrace::new(samples),
            },
        )
    };
    let mut exhausted_replay = replay_base.clone();
    let exhausted_request = exhausted_replay
        .runtime_command_frame(1, 1, completion_command_with_trace(vec![0]))
        .expect("frame exhausted replay command");
    let exhausted_before = exhausted_replay.clone();
    let exhausted_error = exhausted_replay
        .apply_runtime_command_frame(&runtime, &exhausted_request)
        .expect_err("short divider trace must reject atomically");
    assert!(
        format!("{exhausted_error:#}").contains("divider replay exhausted after 1 samples"),
        "{exhausted_error:#}"
    );
    assert_eq!(exhausted_replay, exhausted_before);

    let mut tailed_replay = replay_base;
    let tailed_request = tailed_replay
        .runtime_command_frame(1, 1, completion_command_with_trace(vec![0, 200, 77]))
        .expect("frame tailed replay command");
    let tailed_before = tailed_replay.clone();
    let tailed_error = tailed_replay
        .apply_runtime_command_frame(&runtime, &tailed_request)
        .expect_err("unused divider sample must reject atomically");
    assert!(
        format!("{tailed_error:#}").contains("1 unconsumed samples after 2 reads"),
        "{tailed_error:#}"
    );
    assert_eq!(tailed_replay, tailed_before);
    assert!(resumed_battle.completion.continued_after_battle);
    assert_eq!(battle_shell.session.state().battle, BattleMemory::Inactive);
    assert_eq!(resumed_battle.run.steps.len(), 1);
    assert_eq!(resumed_battle.run.steps[0].command, "setval");
    assert_eq!(
        battle_shell
            .session
            .state()
            .script_runtime
            .script_value
            .as_deref(),
        Some("7")
    );

    let error = session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "runtimewildscript", 4)
        .expect_err("script names are exact");
    let error = format!("{error:#}");
    assert!(error.contains("runtimewildscript"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_dynamic_rock_battle_save_load_resumes_exact_commands_thirteen_and_fourteen() {
    let root = temp_repository_root("dynamic-rock-save-resume");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    add_runtime_field_encounters(&mut data);
    add_runtime_rock_smash_global_scripts(&mut data);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0).expect("game shell");
    let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    player.original_trainer_name = "PLAYER".to_string();
    shell
        .session_mut()
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    shell.session_mut().state.sync_party_from_storage();
    shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([
        255, 0, // Rock RandomRange(10) -> 0
        0, 0, // Rock RandomRange(100) -> 1
        0, 64, // held-item gate -> 191, no second item roll
        0, 175, // packed Attack/Defense -> 0x10
        0, 101, // packed Speed/Special -> 0xab
    ]);

    let started = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RockSmashScript".to_string(),
                command_index: 8,
            },
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run exact dynamic Rock encounter into startbattle");
    assert_eq!(
        started
            .steps
            .iter()
            .map(|step| step.command_index)
            .collect::<Vec<_>>(),
        vec![8, 9, 10, 11, 12]
    );
    assert_eq!(
        started.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RockSmashScript".to_string(),
            command_index: 13,
        })
    );
    assert!(matches!(
        started.boundary,
        Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::StaticWildBattle
        ))
    ));
    let origin = static_wild_origin_from_state(shell.session().state());
    assert_eq!(origin.source_script, "RockSmashScript");
    assert_eq!(origin.startbattle_command_index, 12);
    assert_eq!(origin.resume_command_index, 13);
    assert_eq!(origin.species, "CHIKORITA");
    let BattleMemory::StaticWild { enemy_pokemon, .. } = &shell.session().state().battle else {
        panic!("Rock start must persist a static-wild battle origin");
    };
    assert_eq!(enemy_pokemon.level, 15);
    assert_eq!(enemy_pokemon.dvs, Dv::from_non_hp(1, 0, 10, 11));

    let save_path = root.join("dynamic-rock.crystalsave");
    runtime
        .save_game(&save_path, shell.session().state().clone())
        .expect("save active dynamic Rock battle");
    let mut resumed =
        RuntimeGameShell::resume_from_save(asset_root.clone(), runtime.clone(), &save_path)
            .expect("load active dynamic Rock battle without a UI script cursor");
    assert_eq!(
        static_wild_origin_from_state(resumed.session().state()),
        origin
    );

    // Model an exact manual-RUN terminal after loading. DRAW resumes the
    // saved cursor but skips Pay Day and Pokerus entirely; no caller-side
    // fixed-battle lookup or remembered UI cursor participates.
    resumed.session_mut().state.money = 100;
    resumed.session_mut().state.battle_pay_day_money = 50;
    let random_state_before_draw = resumed.session().state().random_state;
    crystal_core::battle::start::deactivate_battle_after_draw(&mut resumed.session_mut().state);
    assert_eq!(resumed.session().state().battle_result, 2);
    resumed.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([]);
    let completed = resumed
        .complete_scripted_wild_battle_and_run_compiled_script(
            origin.clone(),
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("complete loaded dynamic Rock battle and resume its source cursor");
    assert_eq!(
        completed
            .run
            .steps
            .iter()
            .map(|step| (step.command_index, step.command.as_str()))
            .collect::<Vec<_>>(),
        vec![(13, "reloadmapafterbattle")]
    );
    assert!(matches!(
        completed.run.boundary,
        Some(RuntimeCompiledScriptBoundary::PendingMapLoad(_))
    ));
    assert!(!completed.run.ended);
    let end_cursor = completed
        .run
        .next_cursor
        .expect("reloadmapafterbattle continuation cursor");
    resumed
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapLoad)
        .expect("complete post-battle map reload before end");
    let ended = resumed
        .run_compiled_script_until_boundary(
            end_cursor,
            1,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("resume rock script after map reload");
    assert_eq!(ended.steps[0].command, "end");
    assert!(ended.ended);
    assert_eq!(ended.next_cursor, None);
    assert!(
        resumed
            .session()
            .state()
            .pending_static_wild_terminal
            .is_none()
    );
    assert_eq!(resumed.session().state().battle, BattleMemory::Inactive);
    assert_eq!(resumed.session().state().money, 100);
    assert_eq!(
        resumed.session().state().random_state,
        random_state_before_draw
    );

    let mut lost = RuntimeGameShell::resume_from_save(asset_root, runtime.clone(), &save_path)
        .expect("reload the same active Rock battle for the loss branch");
    assert_eq!(
        static_wild_origin_from_state(lost.session().state()),
        origin
    );
    lost.session_mut().state.money = 100;
    lost.session_mut().state.battle_pay_day_money = 50;
    for pokemon in lost
        .session_mut()
        .state
        .storage
        .party
        .pokemon
        .iter_mut()
        .flatten()
    {
        pokemon.hp = 0;
    }
    lost.session_mut().state.sync_party_from_storage();
    crystal_core::battle::start::deactivate_battle_after_loss(&mut lost.session_mut().state);
    let loss_random_state = lost.session().state().random_state;
    let loss_before = lost.session().state().clone();
    let retained_before_loss = lost.retained_runtime_commands().len();
    let recovery = lost
        .resolve_blackout_to_last_spawn()
        .expect("static-wild loss resolves through whiteout, not source completion");
    assert_eq!(lost.session().state().battle_result, 1);
    assert_eq!(lost.session().state().battle, BattleMemory::Inactive);
    assert!(
        lost.session()
            .state()
            .pending_static_wild_terminal
            .is_none()
    );
    assert_eq!(lost.session().state().money, 50);
    assert_eq!(lost.session().state().random_state, loss_random_state);
    assert_eq!(recovery.spawn_identifier, Some(0));
    assert_eq!(recovery.map_name, "RuntimeMap");
    assert!(lost.session().state().script_runtime.next_script.is_none());
    assert!(
        lost.session()
            .state()
            .script_runtime
            .deferred_scripts
            .is_empty()
    );
    assert!(lost.session().state().script_runtime.call_stack.is_empty());
    assert!(
        lost.session()
            .state()
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .all(|pokemon| pokemon.hp == pokemon.max_hp)
    );
    assert_eq!(
        lost.retained_runtime_commands().len(),
        retained_before_loss + 1,
        "whiteout must not run Rock reload/disappear/end tail commands"
    );
    assert!(matches!(
        crystal_assets::decode_runtime_mutation_command_frame(
            &lost.retained_runtime_commands()[retained_before_loss],
            &loss_before,
        )
        .expect("decode authoritative loss/whiteout mutation"),
        RuntimeMutationCommand::ResolveBlackoutToLastSpawn
    ));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_resolves_active_battle_turn_into_authoritative_state() {
    let root = temp_repository_root("battle-turn");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_scripted_battles(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut player =
        static_wild_battle_start_for_tests(&runtime.data, static_wild_request("CHIKORITA", 8))
            .enemy_pokemon;
    player.original_trainer_name = "PLAYER".to_string();
    session
        .state
        .storage
        .register_capture_in_box(0, player.clone())
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    let active_party_index = session
        .state
        .battle_active_party_index
        .expect("active battle party index");
    if let Some(player) = session.state.storage.party.pokemon[active_party_index].as_mut() {
        player.hp = player.max_hp;
    }
    if let BattleMemory::StaticWild {
        enemy_pokemon,
        enemy_party,
        ..
    } = &mut session.state.battle
    {
        refresh_runtime_pokemon_stats(enemy_pokemon);
        enemy_party[0] = enemy_pokemon.clone();
    } else {
        panic!("expected active wild battle");
    }
    assert_eq!(session.state.battle_active_party_index, Some(0));
    assert!(session.state.pokedex.has_seen("CHIKORITA"));
    let battle_snapshot = RuntimeBattleSnapshot::from_state(&session.state)
        .expect("active battle snapshot resolves")
        .expect("active battle snapshot");
    assert_eq!(battle_snapshot.commands.player_move_slots, vec![0]);
    assert_eq!(battle_snapshot.commands.enemy_move_slots, vec![0]);
    assert!(battle_snapshot.commands.switch_party_indices.is_empty());
    assert!(battle_snapshot.commands.can_use_items);
    assert!(battle_snapshot.commands.can_run);
    session.state.battle_active_party_index = None;
    let snapshot_error = RuntimeBattleSnapshot::from_state(&session.state)
        .expect_err("missing active party index rejects");
    assert!(
        snapshot_error
            .to_string()
            .contains("active battle snapshot is missing active player party index")
    );
    session.state.battle_active_party_index = Some(0);
    let enemy_before = match &session.state.battle {
        BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
        other => panic!("expected static wild battle, got {other:?}"),
    };
    let before_checksum = game_state_checksum(&session.state).expect("checksum before turn");
    let player_action = BattleAction::Move { slot: 0 };
    let enemy_action = BattleAction::Move { slot: 0 };

    let turn = session
        .resolve_active_battle_turn(&runtime, player_action, enemy_action)
        .expect("resolve turn");

    assert_eq!(turn.outcome.state.turn, 1);
    assert!(turn.outcome.events.iter().any(|event| matches!(
        event,
        crystal_core::battle::turn::BattleEvent::Damage { .. }
    )));
    assert_eq!(
        session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player")
            .moves[0]
            .current_pp,
        34
    );
    let enemy_after = match &session.state.battle {
        BattleMemory::StaticWild {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            assert_eq!(enemy_party[0], *enemy_pokemon);
            enemy_pokemon.clone()
        }
        other => panic!("expected static wild battle, got {other:?}"),
    };
    assert!(enemy_after.hp < enemy_before.hp);
    assert_ne!(turn.state_checksum, before_checksum);

    // An in-progress battle is saveable state, not merely a battle
    // descriptor.  Preserve the complete turn cursor and transient
    // effects so loading cannot silently grant a free turn or reset
    // screens/status counters.
    let saved_combat = session.state.script_runtime.active_battle_combat.clone();
    assert!(
        saved_combat.is_some(),
        "turn commit must retain combat state"
    );
    session
        .state
        .script_runtime
        .active_battle_combat
        .as_mut()
        .expect("active combat state")
        .trainer_items_used
        .insert("RIVAL1:SUPER_POTION".to_string());
    let snapshot_with_trainer_item = RuntimeBattleSnapshot::from_state(&session.state)
        .expect("active battle snapshot with trainer item")
        .expect("active battle");
    assert!(
        snapshot_with_trainer_item
            .trainer_items_used
            .contains("RIVAL1:SUPER_POTION"),
        "battle UI/AI must read consumed trainer items from active combat state"
    );
    let saved_combat = session.state.script_runtime.active_battle_combat.clone();

    runtime
        .save_game(root.join("battle-turn.crystalsave"), session.state.clone())
        .expect("save battle turn");
    let loaded = runtime
        .load_save(root.join("battle-turn.crystalsave"))
        .expect("load battle turn");
    assert_eq!(loaded.battle, session.state.battle);
    assert_eq!(loaded.battle_active_party_index, Some(0));
    assert_eq!(
        loaded.storage.party.pokemon,
        session.state.storage.party.pokemon
    );
    assert_eq!(
        loaded.script_runtime.active_battle_combat, saved_combat,
        "active battle combat state must survive save/load"
    );
    let resumed = runtime
        .resume_overworld_session(&asset_root, loaded)
        .expect("resume active battle after save");
    assert_eq!(
        resumed.state.script_runtime.active_battle_combat, saved_combat,
        "resuming an active battle must not recreate or reset its turn state"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_turn_uses_compiled_item_payload() {
    let root = temp_repository_root("battle-turn-item");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let mut potion = runtime_item("POTION", item_pocket("ITEM"));
    potion.parameter = 20;
    potion.consumable = true;
    potion.battle_menu = "ITEMMENU_PARTY".to_string();
    potion.battle_usable = true;
    data.items.insert("POTION".to_string(), potion);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut player =
        static_wild_battle_start_for_tests(&runtime.data, static_wild_request("CHIKORITA", 8))
            .enemy_pokemon;
    player.hp = 10;
    refresh_runtime_pokemon_stats(&mut player);
    player.hp = 10;
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    let hp_before = session.state.storage.party.pokemon[0]
        .as_ref()
        .expect("player")
        .hp;

    let player_action = BattleAction::Item {
        item_id: "POTION".to_string(),
    };
    let enemy_action = BattleAction::Move { slot: 0 };
    let turn = session
        .resolve_active_battle_turn(&runtime, player_action, enemy_action)
        .expect("item turn resolves");

    assert_eq!(hp_before, 10);
    assert!(turn.outcome.events.iter().any(|event| matches!(
        event,
        crystal_core::battle::turn::BattleEvent::BattleItemEffect {
            side: crystal_core::battle::turn::BattleSide::Player,
            outcome
        } if outcome.item_id == "POTION"
            && outcome.hp_before == 10
            && outcome.hp_after > hp_before
            && !outcome.consumed
    )));
    assert!(
        session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player")
            .hp
            > hp_before
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_switch_updates_authoritative_active_party_slot() {
    let root = temp_repository_root("battle-turn-switch");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_scripted_battles(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    for level in [8, 9] {
        let mut player = static_wild_battle_start_for_tests(
            &runtime.data,
            static_wild_request("CHIKORITA", level),
        )
        .enemy_pokemon;
        player.original_trainer_name = "PLAYER".to_string();
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
    }
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    assert_eq!(session.state.battle_active_party_index, Some(0));
    let slot0_before = session.state.storage.party.pokemon[0].clone();
    let slot1_before_pp = session.state.storage.party.pokemon[1]
        .as_ref()
        .expect("second party mon")
        .moves[0]
        .current_pp;

    let player_action = BattleAction::Switch { party_index: 1 };
    let enemy_action = BattleAction::Move { slot: 0 };
    let turn = session
        .resolve_active_battle_turn(&runtime, player_action, enemy_action)
        .expect("switch resolves");

    assert_eq!(session.state.battle_active_party_index, Some(1));
    assert_eq!(session.state.storage.party.pokemon[0], slot0_before);
    assert!(turn.outcome.events.iter().any(|event| matches!(
        event,
        crystal_core::battle::turn::BattleEvent::MoveSelected {
            side: crystal_core::battle::turn::BattleSide::Enemy,
            ..
        }
    )));
    assert_eq!(
        session.state.storage.party.pokemon[1]
            .as_ref()
            .expect("second party mon")
            .moves[0]
            .current_pp,
        slot1_before_pp
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_turn_rejects_missing_exact_move_data_without_state_mutation() {
    let root = temp_repository_root("battle-turn-missing-move");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_scripted_battles(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    player.moves.push(crystal_core::models::LearnedMove {
        name: "tackle".to_string(),
        current_pp: 35,
        pp_ups: 0,
    });
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    let enemy = {
        let mut enemy = Pokemon::new_for_tests(runtime_species(), 6, Dv::default());
        enemy.moves.push(crystal_core::models::LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        });
        enemy
    };
    session.state.battle = BattleMemory::StaticWild {
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        roaming_slot: None,
        origin_map_name: "RuntimeMap".to_string(),
        species: "CHIKORITA".to_string(),
        level: 6,
        source_script: "RuntimeWildScript".to_string(),
        startbattle_command_index: 4,
        resume_command_index: 5,
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy],
    };
    session.state.battle_active_party_index = Some(0);
    session.state.battle_active_enemy_party_index = Some(0);
    let before = session.state.clone();
    let error = session
        .resolve_active_battle_turn(
            &runtime,
            BattleAction::Move { slot: 0 },
            BattleAction::Move { slot: 0 },
        )
        .expect_err("missing move data rejected");
    let error = error_debug(error);
    assert!(error.contains("MissingMoveData"), "{error}");
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_turn_rejects_invalid_active_party_index_without_state_mutation() {
    let root = temp_repository_root("battle-turn-invalid-active");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_scripted_battles(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    let before = session.state.clone();

    let error = session
        .resolve_active_battle_turn(
            &runtime,
            BattleAction::Switch { party_index: 2 },
            BattleAction::Move { slot: 0 },
        )
        .expect_err("empty party slot rejected");

    let error = error_debug(error);
    assert!(error.contains("SwitchTargetOutOfRange"), "{error}");
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_turn_rejects_unknown_item_without_state_mutation() {
    let root = temp_repository_root("battle-turn-rejected-actions");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_scripted_battles(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    let before_item = session.state.clone();

    let item_error = session
        .resolve_active_battle_turn(
            &runtime,
            BattleAction::Item {
                item_id: "POTION".to_string(),
            },
            BattleAction::Move { slot: 0 },
        )
        .expect_err("unknown battle turn item rejects");

    let item_error = error_debug(item_error);
    assert!(item_error.contains("UnknownItem"), "{item_error}");
    assert_eq!(session.state, before_item);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_command_routes_player_run_through_wild_turn_resolution() {
    let root = temp_repository_root("battle-command-run");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_scripted_battles(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut fast_species = runtime_species();
    fast_species.base_stats.speed = 999;
    let player = Pokemon::new_for_tests(fast_species, 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    session.state.battle_escape_attempts = 2;

    let player_action = BattleAction::Run;
    let enemy_action = BattleAction::Move { slot: 0 };
    let turn = session
        .resolve_active_battle_command(&runtime, player_action, enemy_action)
        .expect("run command resolves");

    assert!(matches!(
        turn.outcome,
        ActiveBattleCommandOutcome::Turn(ref outcome)
            if outcome.events.iter().any(|event| matches!(
                event,
                crystal_core::battle::turn::BattleEvent::RunAttempt {
                    side: crystal_core::battle::turn::BattleSide::Player,
                    outcome
                } if outcome.escaped && outcome.attempts_before == 2
            ))
    ));
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_escape_attempts, 0);
    let _ = std::fs::remove_dir_all(root);
}
