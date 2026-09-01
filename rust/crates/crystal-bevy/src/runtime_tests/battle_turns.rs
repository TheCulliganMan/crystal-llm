#[test]
fn runtime_battle_command_failed_player_run_allows_enemy_action() {
    let root = temp_repository_root("battle-command-run-fail");
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
    let mut slow_species = runtime_species();
    slow_species.base_stats.speed = 1;
    let player = Pokemon::new_for_tests(slow_species, 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    if let BattleMemory::StaticWild {
        enemy_pokemon,
        enemy_party,
        ..
    } = &mut session.state.battle
    {
        enemy_pokemon.species.base_stats.speed = 255;
        enemy_party[0].species.base_stats.speed = 255;
        refresh_runtime_pokemon_stats(enemy_pokemon);
        refresh_runtime_pokemon_stats(&mut enemy_party[0]);
    }
    // The slow/fast setup leaves a low first-attempt escape threshold, and
    // Crystal succeeds when the roll is less than or equal to it. Use an
    // exact stream whose first output is 255.
    session.state.random_state = crystal_core::random::CrystalRandomState::default();
    session.divider = crystal_core::random::RuntimeDividerSource::replay(
        std::iter::repeat_n([0_u8, 1_u8], 64).flatten(),
    );

    let player_action = BattleAction::Run;
    let enemy_action = BattleAction::Move { slot: 0 };
    let turn = session
        .resolve_active_battle_command(&runtime, player_action, enemy_action)
        .expect("failed run command resolves as a full battle turn");

    let ActiveBattleCommandOutcome::Turn(outcome) = turn.outcome else {
        panic!("failed run command must resolve as a battle turn");
    };
    assert!(
        outcome.events.iter().any(|event| matches!(
            event,
            crystal_core::battle::turn::BattleEvent::RunAttempt {
                side: crystal_core::battle::turn::BattleSide::Player,
                outcome
            } if !outcome.escaped && outcome.attempts_before == 0 && outcome.attempts_after == 1
        )),
        "events: {:?}",
        outcome.events
    );
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        crystal_core::battle::turn::BattleEvent::MoveSelected {
            side: crystal_core::battle::turn::BattleSide::Enemy,
            ..
        }
    )));
    assert_eq!(session.state.battle_escape_attempts, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_failed_run_stages_enemy_selection_and_turn_as_one_replayable_stream() {
    let root = temp_repository_root("battle-run-atomic-ai-turn");
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
    let mut slow_species = runtime_species();
    slow_species.base_stats.speed = 1;
    let player = Pokemon::new_for_tests(slow_species, 8, Dv::default());
    session.state.storage.party.pokemon[0] = Some(player);
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    if let BattleMemory::StaticWild {
        enemy_pokemon,
        enemy_party,
        ..
    } = &mut session.state.battle
    {
        enemy_pokemon.species.base_stats.speed = 255;
        enemy_party[0].species.base_stats.speed = 255;
        refresh_runtime_pokemon_stats(enemy_pokemon);
        refresh_runtime_pokemon_stats(&mut enemy_party[0]);
    }
    session.state.random_state = crystal_core::random::CrystalRandomState::default();
    session.divider = crystal_core::random::RuntimeDividerSource::replay(
        [0_u8, 0, 0, 1]
            .into_iter()
            .chain(std::iter::repeat_n([0_u8, 0], 128).flatten()),
    );
    let before = session.state.clone();

    let (enemy_action, player_item, recorded) = session
        .stage_active_battle_turn_with_enemy_selectors(
            &runtime,
            BattleAction::Run,
            None,
            |combat, rng| {
                Ok(crystal_core::battle::turn::select_wild_enemy_move_slot(
                    combat, rng,
                ))
            },
            |slot, _combat, _rng| Ok(BattleAction::Move { slot }),
        )
        .expect("stage failed RUN with in-core wild enemy selection");

    assert_eq!(enemy_action, BattleAction::Move { slot: 0 });
    assert_eq!(player_item, None);
    assert_eq!(session.state, before, "staging must remain read-only");
    let RuntimeMutationCommand::ResolveActiveBattleTurn(command) = &recorded.command else {
        panic!("failed RUN must record the shared battle-turn command");
    };
    assert_eq!(command.enemy_ai_selected_move_slot, Some(0));
    assert_eq!(command.enemy_move_ai_random_calls, 1);
    assert_eq!(command.enemy_post_order_ai_random_calls, 0);
    assert!(!command.divider_trace.samples.is_empty());

    let replayed = session
        .apply_runtime_mutation_command(&runtime, recorded.command.clone())
        .expect("replay failed RUN from its one exact divider stream");
    assert_eq!(session.state, recorded.state);
    let RuntimeMutationResult::ActiveBattleTurnResolved(outcome) = replayed.result else {
        panic!("replayed failed RUN must return the full turn");
    };
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::RunAttempt { outcome, .. } if !outcome.escaped
    )));
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Enemy,
            ..
        }
    )));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_command_rejects_short_divider_trace_without_state_mutation() {
    let root = temp_repository_root("battle-command-run-rng-mismatch");
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
    let mut slow_species = runtime_species();
    slow_species.base_stats.speed = 1;
    let player = Pokemon::new_for_tests(slow_species, 8, Dv::default());
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
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::ResolveActiveBattleCommand(RuntimeBattleTurnCommand {
                player_action: BattleAction::Run,
                player_bag_item_id: None,
                enemy_action: BattleAction::Move { slot: 0 },
                enemy_ai_divider_trace: RuntimeDividerTrace::new([]),
                enemy_ai_selected_move_slot: None,
                enemy_move_ai_random_calls: 0,
                enemy_post_order_ai_random_calls: 0,
                divider_trace: RuntimeDividerTrace::new([]),
            }),
        )
        .expect_err("short divider trace must reject");
    let error = format!("{error:#}");
    assert!(error.contains("divider replay exhausted"), "{error}");
    assert_eq!(session.state, before);

    let mut preview = before.clone();
    let mut source = crystal_core::random::ReplayDivider::new((0u8..=u8::MAX).cycle().take(512));
    let mut recording = crystal_core::random::RecordingDivider::new(&mut source);
    runtime
        .data
        .resolve_active_battle_command_with_divider(
            &mut preview,
            BattleAction::Run,
            BattleAction::Move { slot: 0 },
            &mut recording,
        )
        .expect("derive the exact battle command trace");
    let mut surplus_trace = recording.samples().to_vec();
    surplus_trace.push(1);
    let error = session
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::ResolveActiveBattleCommand(RuntimeBattleTurnCommand {
                player_action: BattleAction::Run,
                player_bag_item_id: None,
                enemy_action: BattleAction::Move { slot: 0 },
                enemy_ai_divider_trace: RuntimeDividerTrace::new([]),
                enemy_ai_selected_move_slot: None,
                enemy_move_ai_random_calls: 0,
                enemy_post_order_ai_random_calls: 0,
                divider_trace: RuntimeDividerTrace::new(surplus_trace),
            }),
        )
        .expect_err("surplus divider trace must reject");
    assert!(
        format!("{error:#}").contains("1 unconsumed samples"),
        "{error:#}"
    );
    assert_eq!(session.state, before);

    let error = session
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::ResolveActiveBattleCommand(RuntimeBattleTurnCommand {
                player_action: BattleAction::Run,
                player_bag_item_id: None,
                enemy_action: BattleAction::Move { slot: 0 },
                enemy_ai_divider_trace: RuntimeDividerTrace::new([1]),
                enemy_ai_selected_move_slot: None,
                enemy_move_ai_random_calls: 0,
                enemy_post_order_ai_random_calls: 0,
                divider_trace: RuntimeDividerTrace::new([]),
            }),
        )
        .expect_err("odd enemy AI divider trace must reject");
    assert!(
        format!("{error:#}").contains("enemy battle AI divider trace has odd sample count 1"),
        "{error:#}"
    );
    assert_eq!(session.state, before);

    let mut expected = before.clone();
    let mut ai_divider = crystal_core::random::ReplayDivider::new([0, 1]);
    let mut ai_rng =
        crystal_core::random::CrystalRandom::new(expected.random_state, &mut ai_divider);
    ai_rng
        .battle_random()
        .expect("derive enemy AI random state");
    expected.random_state = ai_rng.state();
    drop(ai_rng);
    let mut source = crystal_core::random::ReplayDivider::new((0u8..=u8::MAX).cycle().take(512));
    let mut recording = crystal_core::random::RecordingDivider::new(&mut source);
    runtime
        .data
        .resolve_active_battle_command_with_divider(
            &mut expected,
            BattleAction::Run,
            BattleAction::Move { slot: 0 },
            &mut recording,
        )
        .expect("derive turn trace after enemy AI");
    let outcome = session
        .apply_runtime_mutation_command(
            &runtime,
            RuntimeMutationCommand::ResolveActiveBattleCommand(RuntimeBattleTurnCommand {
                player_action: BattleAction::Run,
                player_bag_item_id: None,
                enemy_action: BattleAction::Move { slot: 0 },
                enemy_ai_divider_trace: RuntimeDividerTrace::new([0, 1]),
                enemy_ai_selected_move_slot: None,
                enemy_move_ai_random_calls: 0,
                enemy_post_order_ai_random_calls: 0,
                divider_trace: RuntimeDividerTrace::new(recording.samples().iter().copied()),
            }),
        )
        .expect("enemy AI and turn traces share one random state");
    assert!(matches!(
        outcome.result,
        RuntimeMutationResult::ActiveBattleCommandResolved(_)
    ));
    assert_eq!(session.state, expected);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_command_prioritizes_player_run_over_enemy_action() {
    let root = temp_repository_root("battle-command-enemy-run");
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
    let enemy_action = BattleAction::Run;
    let turn = session
        .resolve_active_battle_command(&runtime, player_action, enemy_action)
        .expect("player run command resolves");

    assert!(matches!(
        turn.outcome,
        ActiveBattleCommandOutcome::Escape(ref outcome)
            if outcome.escaped && outcome.attempts_before == 2
    ));
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_escape_attempts, 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_turn_routes_run_through_pack_escape_rules() {
    let root = temp_repository_root("battle-turn-run");
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
        .resolve_active_battle_turn(&runtime, player_action, enemy_action)
        .expect("run turn resolves through escape rules");

    assert!(turn.outcome.events.iter().any(|event| matches!(
        event,
        crystal_core::battle::turn::BattleEvent::RunAttempt {
            side: crystal_core::battle::turn::BattleSide::Player,
            outcome
        } if outcome.escaped && outcome.attempts_before == 2
    )));
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_escape_attempts, 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_command_routes_non_run_to_turn_resolution() {
    let root = temp_repository_root("battle-command-turn");
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
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");

    let player_action = BattleAction::Move { slot: 0 };
    let enemy_action = BattleAction::Move { slot: 0 };
    let turn = session
        .resolve_active_battle_command(&runtime, player_action, enemy_action)
        .expect("move command resolves");

    let ActiveBattleCommandOutcome::Turn(outcome) = turn.outcome else {
        panic!("non-run battle command must resolve a battle turn");
    };
    assert_eq!(outcome.state.turn, 1);
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        crystal_core::battle::turn::BattleEvent::Damage { .. }
    )));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_attempts_wild_escape_with_saved_attempt_counter() {
    let root = temp_repository_root("battle-escape-failure");
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
    let mut slow_species = runtime_species();
    slow_species.base_stats.speed = 1;
    let player = Pokemon::new_for_tests(slow_species, 8, Dv::default());
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    match &mut session.state.battle {
        BattleMemory::StaticWild {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            enemy_pokemon.species.base_stats.speed = 255;
            enemy_party[0].species.base_stats.speed = 255;
            refresh_runtime_pokemon_stats(enemy_pokemon);
            refresh_runtime_pokemon_stats(&mut enemy_party[0]);
        }
        other => panic!("expected static wild battle, got {other:?}"),
    }
    let before = game_state_checksum(&session.state).expect("checksum before escape");
    let divider_sub = if session.state.random_state.sub == u8::MAX {
        0
    } else {
        session.state.random_state.sub + 1
    };
    let samples = [0, divider_sub];
    let mut expected_divider = ReplayDivider::new(samples);
    let mut expected_rng = CrystalRandom::new(session.state.random_state, &mut expected_divider);
    let expected_roll = expected_rng
        .battle_random()
        .expect("complete expected escape trace");
    session.divider = RuntimeDividerSource::replay(samples);

    let escape = session
        .attempt_escape_active_wild_battle(&runtime)
        .expect("escape attempt resolves");

    assert!(!escape.outcome.escaped);
    assert_eq!(escape.outcome.attempts_before, 0);
    assert_eq!(escape.outcome.attempts_after, 1);
    assert_eq!(escape.outcome.roll, Some(expected_roll));
    assert_eq!(session.state.random_state, expected_rng.state());
    assert_eq!(session.state.battle_escape_attempts, 1);
    assert!(matches!(
        session.state.battle,
        BattleMemory::StaticWild { .. }
    ));
    assert_ne!(escape.state_checksum, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_escape_records_exact_divider_trace_and_replays_atomically() {
    let root = temp_repository_root("battle-escape-divider-trace");
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
    let mut shell =
        RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("runtime game shell");
    let mut slow_species = runtime_species();
    slow_species.base_stats.speed = 1;
    let player = Pokemon::new_for_tests(slow_species, 8, Dv::default());
    let mut fast_species = runtime_species();
    fast_species.base_stats.speed = 255;
    let enemy = Pokemon::new_for_tests(fast_species, 8, Dv::default());
    let state = &mut shell.session_mut().state;
    state.storage.party.pokemon[0] = Some(player);
    state.sync_party_from_storage();
    state.battle = BattleMemory::Wild {
        roaming_slot: None,
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "RuntimeMap".to_string(),
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy],
    };
    state.battle_active_party_index = Some(0);
    state.battle_active_enemy_party_index = Some(0);
    shell.session_mut().divider = RuntimeDividerSource::replay([0, 0, 0, 255]);
    let replay_base = shell.session().clone();
    let retained_before = shell.retained_runtime_commands().len();

    shell
        .attempt_escape_active_wild_battle()
        .expect("escape with exact trace");
    let frame = shell.retained_runtime_commands()[retained_before].clone();
    let recorded =
        crystal_assets::decode_runtime_mutation_command_frame(&frame, replay_base.state())
            .expect("decode recorded escape");
    let RuntimeMutationCommand::AttemptEscapeActiveWildBattle(command) = recorded else {
        panic!("escape must journal its typed command");
    };
    assert_eq!(command.divider_trace.samples, vec![0, 0, 0, 255]);

    let mut remote = replay_base.clone();
    remote.divider = RuntimeDividerSource::replay([]);
    remote
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("remote escape consumes recorded DIV trace");
    assert_eq!(remote.state(), shell.session().state());

    for (trace, message) in [
        (vec![0, 0], "divider replay exhausted after 2 samples"),
        (vec![0, 0, 0, 255, 77], "1 unconsumed samples after 4 reads"),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::AttemptEscapeActiveWildBattle(RuntimeBattleEscapeCommand {
                    divider_trace: RuntimeDividerTrace::new(trace),
                }),
            )
            .expect("frame malformed escape replay");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("malformed escape trace rejects atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_successful_wild_escape_clears_active_battle_state() {
    let root = temp_repository_root("battle-escape-success");
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
    session.state.battle_escape_attempts = 3;

    let escape = session
        .attempt_escape_active_wild_battle(&runtime)
        .expect("escape attempt resolves");

    assert!(escape.outcome.escaped);
    assert_eq!(escape.outcome.roll, None);
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_active_party_index, None);
    assert_eq!(session.state.battle_active_enemy_party_index, None);
    assert!(session.state.battle_rewarded_enemy_party_indices.is_empty());
    assert_eq!(session.state.battle_escape_attempts, 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_poke_doll_consumes_item_and_clears_active_wild_battle_state() {
    let root = temp_repository_root("battle-poke-doll");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let mut poke_doll = runtime_item("POKE_DOLL", item_pocket("ITEM"));
    poke_doll.effect = "MOD_DOLL".to_string();
    poke_doll.battle_menu = "ITEMMENU_CLOSE".to_string();
    poke_doll.battle_usable = true;
    poke_doll.consumable = true;
    poke_doll.battle_escape_mode = Some("WILD_BATTLE".to_string());
    data.items.insert("POKE_DOLL".to_string(), poke_doll);
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
        .add_item(&runtime.data.items["POKE_DOLL"], 1)
        .expect("add Poke Doll");
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    session.state.money = 300;
    session.state.battle_pay_day_money = 55;
    session.state.battle_escape_attempts = 2;
    session.state.battle_player_stat_drop_guard_turns = 5;

    let item_escape = session
        .use_bag_item_to_escape_active_wild_battle(&runtime, "POKE_DOLL")
        .expect("Poke Doll escapes wild battle");

    assert!(item_escape.item_use.consumed);
    assert_eq!(item_escape.item_use.item_id, "POKE_DOLL");
    assert_eq!(item_escape.battle_escape_mode, "WILD_BATTLE");
    assert!(item_escape.escaped);
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_active_party_index, None);
    assert_eq!(session.state.battle_active_enemy_party_index, None);
    assert!(session.state.battle_rewarded_enemy_party_indices.is_empty());
    assert_eq!(session.state.battle_escape_attempts, 0);
    assert_eq!(session.state.battle_player_stat_drop_guard_turns, 0);
    assert_eq!(session.state.battle_result, 2);
    assert_eq!(session.state.money, 300, "DRAW skips CheckPayDay");
    assert_eq!(session.state.battle_pay_day_money, 0);
    assert!(!session.state.bag.has_item(&runtime.data.items["POKE_DOLL"]));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_rejects_poke_doll_in_trainer_battle_without_consumption() {
    let root = temp_repository_root("battle-poke-doll-trainer");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let mut poke_doll = runtime_item("POKE_DOLL", item_pocket("ITEM"));
    poke_doll.effect = "POKE_DOLL".to_string();
    poke_doll.battle_menu = "ITEMMENU_CLOSE".to_string();
    poke_doll.battle_usable = true;
    poke_doll.consumable = true;
    poke_doll.battle_escape_mode = Some("WILD_BATTLE".to_string());
    data.items.insert("POKE_DOLL".to_string(), poke_doll);
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
        .add_item(&runtime.data.items["POKE_DOLL"], 1)
        .expect("add Poke Doll");
    session
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("start trainer battle");
    let before = session.state.clone();

    let error = session
        .use_bag_item_to_escape_active_wild_battle(&runtime, "POKE_DOLL")
        .expect_err("Poke Doll cannot escape trainer battle");
    let error = error_debug(error);

    assert!(error.contains("ActiveTrainerBattle"), "{error}");
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_guard_spec_consumes_item_and_sets_stat_drop_guard_turns() {
    let root = temp_repository_root("battle-guard-spec");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let mut guard_spec = runtime_item("GUARD_SPEC", item_pocket("ITEM"));
    guard_spec.effect = "MOD_GUARD".to_string();
    guard_spec.battle_menu = "ITEMMENU_CLOSE".to_string();
    guard_spec.battle_usable = true;
    guard_spec.consumable = true;
    guard_spec.battle_stat_drop_guard = Some(true);
    guard_spec.battle_stat_drop_guard_turns = Some(5);
    data.items.insert("GUARD_SPEC".to_string(), guard_spec);
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
        .add_item(&runtime.data.items["GUARD_SPEC"], 1)
        .expect("add Guard Spec");
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");

    let guard = session
        .use_bag_guard_spec_in_active_battle(&runtime, "GUARD_SPEC")
        .expect("Guard Spec applies");

    assert!(guard.item_use.consumed);
    assert_eq!(guard.item_use.item_id, "GUARD_SPEC");
    assert_eq!(guard.stat_drop_guard_turns_before, 0);
    assert_eq!(guard.stat_drop_guard_turns_after, 5);
    assert_eq!(session.state.battle_player_stat_drop_guard_turns, 5);
    assert!(
        !session
            .state
            .bag
            .has_item(&runtime.data.items["GUARD_SPEC"])
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_rejects_escape_from_trainer_battle_without_state_mutation() {
    let root = temp_repository_root("battle-escape-trainer");
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
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("start trainer battle");
    let before = session.state.clone();

    let error = session
        .attempt_escape_active_wild_battle(&runtime)
        .expect_err("trainer battles cannot be escaped");
    let error = error_debug(error);

    assert!(
        error.contains("cannot escape from trainer battle"),
        "{error}"
    );
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_command_player_run_in_trainer_battle_is_rejected_without_state_mutation() {
    let root = temp_repository_root("battle-command-trainer-player-run");
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
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("start trainer battle");

    let before = session.state.clone();
    let error = session
        .resolve_active_battle_command(&runtime, BattleAction::Run, BattleAction::Move { slot: 0 })
        .expect_err("trainer player run is rejected");

    assert!(error_debug(error).contains("RunNotAllowed { side: Player }"));
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_command_enemy_run_in_trainer_battle_is_rejected_without_state_mutation() {
    let root = temp_repository_root("battle-command-trainer-enemy-run");
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
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("start trainer battle");

    let before = session.state.clone();
    let error = session
        .resolve_active_battle_command(&runtime, BattleAction::Move { slot: 0 }, BattleAction::Run)
        .expect_err("trainer enemy run is rejected");

    assert!(error_debug(error).contains("RunNotAllowed { side: Enemy }"));
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_claims_wild_battle_rewards_into_authoritative_party_state() {
    let root = temp_repository_root("battle-rewards");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_battle_rewards(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let mut player =
        static_wild_battle_start_for_tests(&runtime.data, static_wild_request("CHIKORITA", 15))
            .enemy_pokemon;
    player.original_trainer_name = "PLAYER".to_string();
    player.experience =
        calculate_experience(&runtime.data.growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    let static_origin = static_wild_origin_from_state(&session.state);
    assert_eq!(session.state.battle_active_party_index, Some(0));
    match &mut session.state.battle {
        BattleMemory::StaticWild {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        other => panic!("expected static wild battle, got {other:?}"),
    }
    session.state.money = 100;
    session.state.battle_pay_day_money = 60;
    session.divider = crystal_core::random::RuntimeDividerSource::replay([0, 200]);

    let rewards = session
        .claim_active_wild_battle_rewards(&runtime)
        .expect("claim battle rewards");

    assert_eq!(rewards.outcome.level_before, 15);
    assert_eq!(rewards.outcome.level_after, 16);
    assert_eq!(
        rewards.outcome.learned_moves,
        vec!["RAZOR_LEAF".to_string()]
    );
    assert_eq!(
        rewards.outcome.evolution.target_species,
        Some("BAYLEEF".to_string())
    );
    let lead = session.state.storage.party.pokemon[0]
        .as_ref()
        .expect("lead");
    assert_eq!(lead.species.id, "BAYLEEF");
    assert!(lead.moves.iter().any(|known| known.name == "RAZOR_LEAF"));
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_active_party_index, None);
    assert_eq!(
        session.state.party.pokemon[0]
            .as_ref()
            .map(|entry| entry.species.as_str()),
        Some("BAYLEEF")
    );
    assert_eq!(session.state.money, 160);
    assert_eq!(session.state.battle_pay_day_money, 0);
    let pending = session
        .state
        .pending_static_wild_terminal
        .as_ref()
        .expect("victory retains the script-resume identity");
    assert_eq!(pending.battle_result, 0);
    assert!(pending.win_cleanup_applied);
    let random_state_after_rewards = session.state.random_state;
    session.divider = crystal_core::random::RuntimeDividerSource::replay([]);
    session
        .complete_scripted_wild_battle(&runtime, static_origin)
        .expect("victory resumes without repeating payout or Pokerus");
    assert_eq!(session.state.money, 160);
    assert_eq!(session.state.random_state, random_state_after_rewards);
    assert!(session.state.pending_static_wild_terminal.is_none());
    runtime
        .save_game(
            root.join("battle-rewards.crystalsave"),
            session.state.clone(),
        )
        .expect("save battle rewards");
    let loaded = runtime
        .load_save(root.join("battle-rewards.crystalsave"))
        .expect("load battle rewards");
    assert_eq!(
        loaded.storage.party.pokemon,
        session.state.storage.party.pokemon
    );
    assert_eq!(loaded.battle, BattleMemory::Inactive);
    assert_eq!(loaded.battle_active_party_index, None);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_battle_rewards_reject_non_fainted_enemy() {
    let root = temp_repository_root("battle-rewards-not-fainted");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(
            minimal_runtime_data_with_battle_rewards(),
            report(),
        ),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player =
        static_wild_battle_start_for_tests(&runtime.data, static_wild_request("CHIKORITA", 15))
            .enemy_pokemon;
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("register player");
    session.state.sync_party_from_storage();
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("start battle");
    assert_eq!(session.state.battle_active_party_index, Some(0));
    let before = session.state.clone();

    let error = session
        .claim_active_wild_battle_rewards(&runtime)
        .expect_err("enemy must be fainted");

    let error = error_debug(error);
    assert!(error.contains("DefeatedPokemonNotFainted"), "{error}");
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_ordinary_capture_records_win_cleanup_and_rejects_short_or_tailed_trace() {
    let root = temp_repository_root("ordinary-capture-win-cleanup");
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
    let mut shell =
        RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("runtime game shell");
    let mut player = Pokemon::new_for_tests(runtime_species(), 10, Dv::default());
    player.pokerus = 0xa2;
    let enemy = Pokemon::new_for_tests(runtime_species(), 6, Dv::default());
    let state = &mut shell.session_mut().state;
    state.storage.party.pokemon[0] = Some(player);
    state.sync_party_from_storage();
    state.battle = BattleMemory::Wild {
        roaming_slot: None,
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "RuntimeMap".to_string(),
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy],
    };
    state.battle_active_party_index = Some(0);
    state.battle_active_enemy_party_index = Some(0);
    state.money = 100;
    state.battle_pay_day_money = 45;
    let capture = CaptureOutcome {
        caught: true,
        blocked: false,
        storage_full: false,
        wobble_count: 4,
        animation_shakes: 4,
        final_catch_rate: u8::MAX,
        ball_id: Some("MASTER_BALL".to_string()),
    };
    shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([0, 100, 0, 155]);
    let replay_base = shell.session().clone();
    let before = shell.session().state().clone();
    let retained_before = shell.retained_runtime_commands().len();

    let completion = shell
        .complete_active_wild_capture(&capture, None)
        .expect("ordinary capture performs exact WIN cleanup once");

    assert!(completion.stored.is_some());
    assert_eq!(shell.session().state().battle_result, 0);
    assert_eq!(shell.session().state().money, 145);
    assert!(
        shell
            .session()
            .state()
            .pending_static_wild_terminal
            .is_none()
    );
    let frame = shell.retained_runtime_commands()[retained_before].clone();
    let command = crystal_assets::decode_runtime_mutation_command_frame(&frame, &before)
        .expect("decode recorded capture completion");
    let RuntimeMutationCommand::CompleteActiveWildCapture(recorded) = command else {
        panic!("capture must journal its typed completion command");
    };
    assert_eq!(recorded.divider_trace.samples, vec![0, 100, 0, 155]);

    let mut remote = replay_base.clone();
    remote
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("remote capture replay consumes the exact recorded trace");
    assert_eq!(remote.state, shell.session().state);

    for (trace, message) in [
        (vec![0, 100], "divider replay exhausted after 2 samples"),
        (
            vec![0, 100, 0, 155, 77],
            "1 unconsumed samples after 4 reads",
        ),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::CompleteActiveWildCapture(
                    RuntimeCaptureCompletionCommand {
                        outcome: capture.clone(),
                        nickname: None,
                        divider_trace: RuntimeDividerTrace::new(trace),
                    },
                ),
            )
            .expect("frame malformed capture replay");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("malformed capture divider trace rejects atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_ball_throw_records_exact_divider_trace_and_replays_atomically() {
    let root = temp_repository_root("capture-attempt-divider-trace");
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
    let mut shell =
        RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("runtime game shell");
    let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::default());
    let enemy = Pokemon::new_for_tests(runtime_species(), 6, Dv::default());
    let state = &mut shell.session_mut().state;
    state.storage.party.pokemon[0] = Some(player);
    state.sync_party_from_storage();
    state.battle = BattleMemory::Wild {
        roaming_slot: None,
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "RuntimeMap".to_string(),
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy],
    };
    state.battle_active_party_index = Some(0);
    state.battle_active_enemy_party_index = Some(0);
    state
        .bag
        .add_item(&runtime.data.items["POKE_BALL"], 1)
        .expect("add Poke Ball");
    shell.session_mut().divider = RuntimeDividerSource::replay([0, 0]);
    let replay_base = shell.session().clone();
    let retained_before = shell.retained_runtime_commands().len();

    let attempt = shell
        .throw_ball_at_active_battle("POKE_BALL")
        .expect("throw Poke Ball with exact trace");
    assert!(attempt.outcome.expect("capture outcome").caught);
    let frame = shell.retained_runtime_commands()[retained_before].clone();
    let recorded =
        crystal_assets::decode_runtime_mutation_command_frame(&frame, replay_base.state())
            .expect("decode recorded ball throw");
    let RuntimeMutationCommand::ThrowBallAtActiveBattle(command) = recorded else {
        panic!("ball throw must journal its typed command");
    };
    assert_eq!(command.divider_trace.samples, vec![0, 0]);

    let mut remote = replay_base.clone();
    remote.divider = RuntimeDividerSource::replay([]);
    remote
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("remote ball throw consumes recorded DIV trace");
    assert_eq!(remote.state(), shell.session().state());

    for (trace, message) in [
        (vec![0], "divider replay exhausted after 1 samples"),
        (vec![0, 0, 77], "1 unconsumed samples after 2 reads"),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::ThrowBallAtActiveBattle(RuntimeBattleItemCommand {
                    item_id: "POKE_BALL".to_string(),
                    divider_trace: RuntimeDividerTrace::new(trace),
                }),
            )
            .expect("frame malformed ball throw replay");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("malformed ball throw trace rejects atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_blocked_ball_and_enemy_response_are_one_replayable_turn() {
    let root = temp_repository_root("blocked-ball-atomic-turn");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let trainer = data
        .trainers
        .trainers
        .get_mut("RIVAL1")
        .expect("runtime rival trainer");
    trainer.ai_move_flags = (1 << 10) - 1;
    trainer.ai_layers = [
        "AI_BASIC",
        "AI_SETUP",
        "AI_TYPES",
        "AI_OFFENSIVE",
        "AI_SMART",
        "AI_OPPORTUNIST",
        "AI_AGGRESSIVE",
        "AI_CAUTIOUS",
        "AI_STATUS",
        "AI_RISKY",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut shell =
        RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("runtime game shell");
    let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::default());
    shell.session_mut().state.storage.party.pokemon[0] = Some(player);
    shell.session_mut().state.sync_party_from_storage();
    shell
        .start_scripted_trainer_battle("RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("start trainer battle");
    shell
        .session_mut()
        .state
        .bag
        .add_item(&runtime.data.items["POKE_BALL"], 1)
        .expect("add Poke Ball");
    let replay_base = shell.session().clone();
    let retained_before = shell.retained_runtime_commands().len();
    let ai_move_flags = match &shell.session().state().battle {
        BattleMemory::Trainer { ai_move_flags, .. } => *ai_move_flags,
        battle => panic!("expected trainer battle, got {battle:?}"),
    };

    let resolved = shell
        .resolve_active_battle_ball_turn_with_enemy_selectors(
            "POKE_BALL",
            |combat, rng| {
                runtime
                    .data
                    .select_trainer_enemy_move_slot(combat, ai_move_flags, rng)
            },
            |slot, _combat, _rng| Ok(BattleAction::Move { slot }),
        )
        .expect("blocked Ball resolves with enemy response");

    assert!(resolved.capture.blocked);
    assert!(!resolved.capture.storage_full);
    assert_eq!(
        shell
            .session()
            .state()
            .bag
            .quantity(&runtime.data.items["POKE_BALL"]),
        0
    );
    let ball_index = resolved
        .turn
        .outcome
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::BallThrown { .. }))
        .expect("Ball event");
    let enemy_index = resolved
        .turn
        .outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    side: BattleSide::Enemy,
                    ..
                }
            )
        })
        .expect("enemy response");
    assert!(ball_index < enemy_index);

    let frame = shell.retained_runtime_commands()[retained_before].clone();
    let recorded =
        crystal_assets::decode_runtime_mutation_command_frame(&frame, replay_base.state())
            .expect("decode atomic Ball turn");
    let (selected_move_slot, move_random_calls) = match &recorded {
        RuntimeMutationCommand::ResolveActiveBattleTurn(RuntimeBattleTurnCommand {
            player_action: BattleAction::Ball { item_id },
            player_bag_item_id: None,
            enemy_ai_selected_move_slot: Some(selected_move_slot),
            enemy_move_ai_random_calls,
            ..
        }) if item_id == "POKE_BALL" => (*selected_move_slot, *enemy_move_ai_random_calls),
        command => panic!("unexpected command: {command:?}"),
    };
    assert!(move_random_calls > 0);
    let mut forged = recorded.clone();
    let RuntimeMutationCommand::ResolveActiveBattleTurn(command) = &mut forged else {
        unreachable!("checked above")
    };
    let forged_move_slot = (selected_move_slot + 1) & 3;
    command.enemy_ai_selected_move_slot = Some(forged_move_slot);
    command.enemy_action = BattleAction::Move {
        slot: forged_move_slot,
    };
    let mut rejected = replay_base.clone();
    let error = rejected
        .apply_runtime_mutation_command(&runtime, forged)
        .expect_err("replay must recompute and reject a forged trainer move slot");
    assert!(
        format!("{error:#}").contains(&format!(
            "recorded trainer enemy move slot {forged_move_slot} does not match recomputed slot {selected_move_slot}"
        )),
        "{error:#}"
    );
    assert_eq!(rejected.state(), replay_base.state());

    let mut forged_action = recorded.clone();
    let RuntimeMutationCommand::ResolveActiveBattleTurn(command) = &mut forged_action else {
        unreachable!("checked above")
    };
    command.enemy_action = BattleAction::TrainerSwitch {
        selected_move_slot,
        party_index: 1,
    };
    let mut rejected = replay_base.clone();
    let error = rejected
        .apply_runtime_mutation_command(&runtime, forged_action)
        .expect_err("replay must recompute and reject a forged trainer post-order action");
    assert!(
        format!("{error:#}").contains("recorded trainer enemy action"),
        "{error:#}"
    );
    assert_eq!(rejected.state(), replay_base.state());

    let mut forged_post_calls = recorded.clone();
    let RuntimeMutationCommand::ResolveActiveBattleTurn(command) = &mut forged_post_calls else {
        unreachable!("checked above")
    };
    command.enemy_post_order_ai_random_calls =
        command.enemy_post_order_ai_random_calls.saturating_add(1);
    let mut rejected = replay_base.clone();
    let error = rejected
        .apply_runtime_mutation_command(&runtime, forged_post_calls)
        .expect_err("replay must reject a forged trainer post-order RNG count");
    assert!(
        format!("{error:#}").contains("recorded trainer post-order AI call count"),
        "{error:#}"
    );
    assert_eq!(rejected.state(), replay_base.state());
    let mut remote = replay_base;
    remote
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("replay atomic Ball turn");
    assert_eq!(remote.state(), shell.session().state());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_failed_ball_residual_then_wild_flee_replays_in_one_rng_stream() {
    let root = temp_repository_root("failed-ball-wild-flee-atomic-turn");
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
    let mut shell =
        RuntimeGameShell::new_game(asset_root, runtime.clone(), 0).expect("runtime game shell");
    let mut player = Pokemon::new_for_tests(runtime_species(), 10, Dv::default());
    player.status = Some("POISON".to_string());
    let mut flee_species = runtime_species();
    flee_species.id = "CUBONE".to_string();
    let mut enemy = Pokemon::new_for_tests(flee_species, 6, Dv::default());
    enemy.moves = vec![LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 35,
        pp_ups: 0,
    }];
    let state = &mut shell.session_mut().state;
    state.storage.party.pokemon[0] = Some(player);
    state.sync_party_from_storage();
    state.battle = BattleMemory::Wild {
        roaming_slot: None,
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
        map_name: "RuntimeMap".to_string(),
        enemy_pokemon: enemy.clone(),
        enemy_party: vec![enemy],
    };
    state.battle_active_party_index = Some(0);
    state.battle_active_enemy_party_index = Some(0);
    state
        .bag
        .add_item(&runtime.data.items["POKE_BALL"], 1)
        .expect("add Poke Ball");
    shell.session_mut().divider = RuntimeDividerSource::replay([0, 0, 0, 1, 0, 0, 0, 255]);
    let replay_base = shell.session().clone();
    let retained_before = shell.retained_runtime_commands().len();

    let resolved = shell
        .resolve_active_battle_ball_turn_with_enemy_selectors(
            "POKE_BALL",
            |combat, rng| {
                Ok(crystal_core::battle::turn::select_wild_enemy_move_slot(
                    combat, rng,
                ))
            },
            |slot, _combat, _rng| Ok(BattleAction::Move { slot }),
        )
        .expect("failed Ball and wild flee resolve");

    assert!(!resolved.capture.caught);
    let residual_index = resolved
        .turn
        .outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::ResidualStatusDamage {
                    side: BattleSide::Player,
                    ..
                }
            )
        })
        .expect("player poison residual");
    let flee_index = resolved
        .turn
        .outcome
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Fled {
                    side: BattleSide::Enemy
                }
            )
        })
        .expect("Cubone flee");
    assert!(residual_index < flee_index);
    assert!(!resolved.turn.outcome.events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: BattleSide::Enemy,
            ..
        }
    )));
    assert!(matches!(
        shell.session().state().battle,
        BattleMemory::Inactive
    ));

    let frame = shell.retained_runtime_commands()[retained_before].clone();
    let mut forged =
        crystal_assets::decode_runtime_mutation_command_frame(&frame, replay_base.state())
            .expect("decode failed Ball turn");
    let RuntimeMutationCommand::ResolveActiveBattleTurn(command) = &mut forged else {
        panic!("failed Ball must record a shared battle turn");
    };
    command.enemy_ai_selected_move_slot = Some(1);
    command.enemy_action = BattleAction::Move { slot: 1 };
    let mut rejected = replay_base.clone();
    let error = rejected
        .apply_runtime_mutation_command(&runtime, forged)
        .expect_err("replay must recompute and reject a forged wild move slot");
    assert!(
        format!("{error:#}")
            .contains("recorded wild enemy move slot 1 does not match recomputed slot 0"),
        "{error:#}"
    );
    assert_eq!(rejected.state(), replay_base.state());

    let mut remote = replay_base;
    remote
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("replay failed Ball and post-residual flee");
    assert_eq!(remote.state(), shell.session().state());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_capture_completion_stores_active_wild_pokemon_in_authoritative_state() {
    let root = temp_repository_root("scripted-wild-capture");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    data.items
        .get_mut("MASTER_BALL")
        .expect("master ball")
        .effect = "MOD_MASTER_BALL".to_string();
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("player party Pokemon");
    session.state.sync_party_from_storage();
    session
        .state
        .bag
        .add_item(&runtime.data.items["MASTER_BALL"], 1)
        .expect("add master ball");
    let start = session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("scripted wild battle starts");
    assert_eq!(session.state.battle_active_party_index, Some(0));
    assert!(session.state.pokedex.has_seen("CHIKORITA"));
    let attempt = session
        .throw_ball_at_active_battle(&runtime, "MASTER_BALL")
        .expect("throw master ball");
    assert_eq!(
        session
            .state
            .bag
            .quantity(&runtime.data.items["MASTER_BALL"]),
        0
    );
    let capture = attempt.outcome.expect("ball was available");
    assert!(capture.caught);
    session.state.money = 200;
    session.state.battle_pay_day_money = 45;

    let captured = session
        .complete_active_wild_capture(&runtime, &capture, None)
        .expect("captured Pokemon stores");

    let stored = captured.stored.expect("successful capture stores");
    assert_eq!(stored.location, CaptureStorageLocation::Party { slot: 1 });
    assert_eq!(stored.pokemon.species.id, "CHIKORITA");
    assert_eq!(stored.pokemon.level, start.level);
    assert_eq!(session.state.battle_result, 0);
    assert_eq!(
        session.state.storage.party.pokemon[1]
            .as_ref()
            .expect("stored party mon")
            .species
            .id,
        "CHIKORITA"
    );
    assert_eq!(
        session.state.party.pokemon[1],
        Some(crystal_core::state::PartyPokemonRef {
            species: "CHIKORITA".to_string(),
            level: 6,
        })
    );
    assert!(session.state.pokedex.has_caught("CHIKORITA"));
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_active_party_index, None);
    assert_eq!(session.state.money, 245);
    assert_eq!(session.state.battle_pay_day_money, 0);
    assert!(
        session
            .state
            .pending_static_wild_terminal
            .as_ref()
            .expect("static capture retains resume origin")
            .win_cleanup_applied
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_pc_withdraw_updates_box_metadata_before_checksum() {
    let root = temp_repository_root("pc-withdraw-box-metadata");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
        identity(),
    )
    .expect("runtime");
    let mut shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("runtime shell");
    shell
        .add_party_pokemon(
            "CHIKORITA",
            5,
            None,
            None,
            "SMOKE",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add lead");
    shell
        .add_party_pokemon(
            "CHIKORITA",
            6,
            None,
            None,
            "SMOKE",
            2,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add deposited Pokemon");
    shell.switch_current_pc_box(1).expect("switch PC box");
    let retained_before_deposit = shell.retained_runtime_commands().len();
    let deposit = shell
        .deposit_party_pokemon_to_current_box(1)
        .expect("deposit party Pokemon");

    let deposit_commands = &shell.retained_runtime_commands()[retained_before_deposit..];
    assert_eq!(deposit_commands.len(), 3);
    assert!(matches!(
        crystal_assets::decode_runtime_mutation_command_payload(deposit_commands[0].payload())
            .expect("decode PC pause command"),
        RuntimeMutationCommand::SetGameLogicPaused(command) if command.paused
    ));
    assert!(matches!(
        crystal_assets::decode_runtime_mutation_command_payload(deposit_commands[1].payload())
            .expect("decode PC deposit command"),
        RuntimeMutationCommand::DepositPartyPokemonToCurrentBox(command)
            if command.party_index == 1
    ));
    assert!(matches!(
        crystal_assets::decode_runtime_mutation_command_payload(deposit_commands[2].payload())
            .expect("decode PC resume command"),
        RuntimeMutationCommand::SetGameLogicPaused(command) if !command.paused
    ));
    assert!(!shell.session().state().game_logic_paused);
    assert_eq!(
        deposit.state_checksum,
        crystal_core::multiplayer::game_state_checksum(shell.session().state())
            .expect("checksum final PC deposit state")
    );

    let withdraw = shell
        .withdraw_current_box_pokemon_to_party(deposit.box_slot)
        .expect("withdraw party Pokemon");
    let snapshot = shell.snapshot().expect("snapshot after withdraw");
    let box_one = snapshot
        .storage
        .boxes
        .iter()
        .find(|pc_box| pc_box.index == 1)
        .expect("box 1 snapshot");

    assert_eq!(withdraw.pokemon, deposit.pokemon);
    assert_eq!(snapshot.storage.party_count, 2);
    assert_eq!(box_one.count, 0);
    assert!(box_one.slots.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_capture_item_rejects_undeclared_ball_rule_without_mutation() {
    let root = temp_repository_root("scripted-wild-capture-bad-ball");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let mut bad_ball = runtime_ball_item("BAD_BALL");
    bad_ball.effect = "MOD_BALL".to_string();
    data.items.insert("BAD_BALL".to_string(), bad_ball);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("player party Pokemon");
    session.state.sync_party_from_storage();
    session
        .state
        .bag
        .add_item(&runtime.data.items["BAD_BALL"], 1)
        .expect("add bad ball");
    session
        .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
        .expect("scripted wild battle starts");
    let before_state = session.state.clone();

    let error = session
        .throw_ball_at_active_battle(&runtime, "BAD_BALL")
        .expect_err("undeclared capture ball is rejected");
    let error = error_debug(error);

    assert!(
        error.contains("battle capture item BAD_BALL is not declared by exact capture rules"),
        "{error}"
    );
    assert_eq!(session.state, before_state);
    assert_eq!(
        session.state.bag.quantity(&runtime.data.items["BAD_BALL"]),
        1
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_grants_scripted_gift_pokemon_into_authoritative_storage() {
    let root = temp_repository_root("scripted-gift-pokemon");
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
    let grant = session
        .grant_scripted_gift_pokemon(
            &runtime,
            "RuntimeMap",
            "RuntimeGiftScript",
            12,
            "PLAYER",
            1234,
            true,
            Some("Leafy".to_string()),
        )
        .expect("gift Pokemon grants");
    let mut dispatch_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("dispatch gift shell");
    dispatch_shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([0x10, 0x20, 0x30, 0x40]);
    let replay_base = dispatch_shell.session().clone();
    let before_dispatch = dispatch_shell.session().state().clone();
    let dispatched = dispatch_shell
        .grant_compiled_gift_pokemon_command(
            "RuntimeGiftScript",
            12,
            "PLAYER",
            1234,
            true,
            Some("Leafy".to_string()),
        )
        .expect("compiled gift Pokemon dispatch");
    let frame = dispatch_shell.retained_runtime_commands()[0].clone();
    let command = crystal_assets::decode_runtime_mutation_command_frame(&frame, &before_dispatch)
        .expect("decode recorded gift command");
    let RuntimeMutationCommand::GrantScriptedGiftPokemon(recorded) = command else {
        panic!("gift must journal its typed divider trace");
    };
    assert_eq!(recorded.divider_trace.samples, vec![0x10, 0x20, 0x30, 0x40]);
    assert_eq!(
        dispatched.outcome.pokemon.dvs,
        Dv::from_non_hp(14, 0, 10, 0)
    );

    let mut replayed = replay_base.clone();
    replayed
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("gift replay consumes the exact recorded divider trace");
    assert_eq!(replayed.state, dispatch_shell.session().state);

    for (trace, message) in [
        (
            vec![0x10, 0x20, 0x30],
            "divider replay exhausted after 3 samples",
        ),
        (
            vec![0x10, 0x20, 0x30, 0x40, 0x50],
            "1 unconsumed samples after 4 reads",
        ),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::GrantScriptedGiftPokemon(RuntimeGiftPokemonCommand {
                    command: RuntimeScriptCommandRef::new("RuntimeMap", "RuntimeGiftScript", 12),
                    original_trainer_name: "PLAYER".to_string(),
                    original_trainer_id: 1234,
                    nickname_accepted: true,
                    nickname: Some("Leafy".to_string()),
                    divider_trace: RuntimeDividerTrace::new(trace),
                }),
            )
            .expect("frame malformed gift replay");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("malformed gift divider trace rejects atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }
    let mut script_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("compiled script gift shell");
    let script_dispatch = script_shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeGiftScript",
            12,
            ScriptRuntimeInputs {
                gift_original_trainer_name: Some("PLAYER".to_string()),
                gift_original_trainer_id: Some(1234),
                gift_nickname_accepted: Some(true),
                gift_nickname: Some("Leafy".to_string()),
                ..ScriptRuntimeInputs::default()
            },
            ScriptPhoneInputs::default(),
        )
        .expect("compiled script dispatcher grants gift Pokemon");
    let RuntimeMutationResult::ScriptedGiftPokemonGranted(script_outcome) = script_dispatch.result
    else {
        panic!("compiled gift command must dispatch as scripted gift Pokemon");
    };
    let missing_inputs = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("missing input gift shell")
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeGiftScript",
            12,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect_err("compiled gift dispatch requires explicit deterministic inputs");
    let mut unexpected_gift_input_shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
            .expect("unexpected gift input shell");
    let before_unexpected_gift_input = unexpected_gift_input_shell.session.state().clone();
    let unexpected_gift_input = unexpected_gift_input_shell
        .apply_compiled_script_command(
            "RuntimeMap",
            "RuntimeWildScript",
            4,
            ScriptRuntimeInputs {
                gift_original_trainer_name: Some("PLAYER".to_string()),
                gift_original_trainer_id: Some(1234),
                gift_nickname_accepted: Some(false),
                gift_nickname: None,
                ..ScriptRuntimeInputs::default()
            },
            ScriptPhoneInputs::default(),
        )
        .expect_err("non-gift command rejects gift Pokemon inputs");
    let unexpected_gift_input = format!("{unexpected_gift_input:#}");
    assert!(
        unexpected_gift_input.contains("must not declare gift Pokemon input fields"),
        "{unexpected_gift_input}"
    );
    assert_eq!(
        unexpected_gift_input_shell.session.state(),
        &before_unexpected_gift_input
    );
    let mut generated_input_shell =
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
            .expect("generated input gift shell");
    generated_input_shell.session_mut().state.player_name = "KRIS".to_string();
    generated_input_shell.session_mut().state.player_id = 0x2222;
    let generated_inputs = generated_input_shell
        .compiled_script_runtime_inputs("RuntimeMap", "RuntimeGiftScript", 12)
        .expect("compiled gift command inputs");
    assert_eq!(
        generated_inputs.gift_original_trainer_name.as_deref(),
        Some("KRIS")
    );
    assert_eq!(generated_inputs.gift_original_trainer_id, Some(0x2222));
    assert_eq!(generated_inputs.gift_nickname_accepted, Some(false));
    assert_eq!(generated_inputs.gift_nickname, None);
    let missing_runtime_inputs = generated_input_shell
        .compiled_script_runtime_inputs("RuntimeMap", "RuntimeGiftScript", 99)
        .expect_err("missing compiled runtime command input row rejected");
    assert!(
        missing_runtime_inputs
            .to_string()
            .contains("compiled script RuntimeGiftScript missing command 99"),
        "{missing_runtime_inputs}"
    );

    assert_eq!(
        grant.outcome.location,
        Some(CaptureStorageLocation::Party { slot: 0 })
    );
    assert_eq!(
        dispatched.outcome.location,
        Some(CaptureStorageLocation::Party { slot: 0 })
    );
    assert_eq!(grant.outcome.pokemon.species.id, "CHIKORITA");
    assert_eq!(dispatched.outcome.pokemon.species.id, "CHIKORITA");
    assert_eq!(script_outcome.pokemon.species.id, "CHIKORITA");
    assert_eq!(grant.outcome.pokemon.level, 7);
    assert_eq!(grant.outcome.pokemon.item.as_deref(), Some("BERRY"));
    assert_eq!(grant.outcome.pokemon.nickname, "Leafy");
    assert_eq!(grant.outcome.pokemon.original_trainer_name, "PLAYER");
    assert!(
        missing_inputs
            .to_string()
            .contains("requires gift_original_trainer_name input"),
        "{missing_inputs}"
    );
    assert_eq!(
        session.state.party.pokemon[0],
        Some(crystal_core::state::PartyPokemonRef {
            species: "CHIKORITA".to_string(),
            level: 7,
        })
    );
    assert!(session.state.pokedex.has_caught("CHIKORITA"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_rejects_incomplete_nickname_input_and_uses_resolved_gift_levels() {
    let root = temp_repository_root("scripted-gift-rejections");
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
    let nickname_error = session
        .grant_scripted_gift_pokemon(
            &runtime,
            "RuntimeMap",
            "RuntimeGiftScript",
            12,
            "PLAYER",
            1234,
            true,
            None,
        )
        .expect_err("accepted nickname requires a supplied value");
    let nickname_error = error_debug(nickname_error);
    assert!(
        nickname_error.contains("cannot accept the supplied nickname"),
        "{nickname_error}"
    );
    let refused = session
        .grant_scripted_gift_pokemon(
            &runtime,
            "RuntimeMap",
            "RuntimeGiftScript",
            12,
            "PLAYER",
            1234,
            false,
            None,
        )
        .expect("refused nickname prompt grants species default nickname");
    assert_eq!(refused.outcome.pokemon.nickname, "CHIKORITA");

    let egg = session
        .grant_scripted_gift_pokemon(
            &runtime,
            "RuntimeMap",
            "RuntimeEggScript",
            3,
            "PLAYER",
            1234,
            false,
            None,
        )
        .expect("compiled egg level is already resolved");
    assert_eq!(egg.outcome.pokemon.level, 5);
    assert_eq!(egg.outcome.pokemon.nickname, "EGG");
    assert_eq!(egg.outcome.pokemon.hp, 0);
    assert_eq!(session.state.storage.party.filled_slots(), 2);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_starts_scripted_trainer_battle_from_exact_map_script_command() {
    let root = temp_repository_root("scripted-trainer-battle");
    write_floor_tileset(&root, "johto");
    let trainer_music_path = root
        .join("apps/web/assets/data")
        .join("content-packs/test/music/MUSIC_RIVAL_ENCOUNTER.pcm");
    write_pcm(&trainer_music_path);
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    data.audio.push(
        ModpackAudioAsset::music(
            "MUSIC_RIVAL_ENCOUNTER",
            "content-packs/test/music/MUSIC_RIVAL_ENCOUNTER.pcm",
        )
        .expect("trainer encounter music fixture"),
    );
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    for label in ["RuntimeSeenText", "RuntimeWinText", "RuntimeLossText"] {
        map.script_text_bodies.insert(
            label.to_string(),
            ScriptTextBody {
                label: label.to_string(),
                commands: Vec::new(),
            },
        );
    }
    map.scripts.insert(
        "RuntimeTrainerScript".to_string(),
        serde_json::json!([
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "opentext", "args": []},
            {"command": "winlosstext", "args": ["RuntimeWinText", "RuntimeLossText"]},
            {"command": "loadtrainer", "args": ["RIVAL1", "RIVAL1"]},
            {"command": "startbattle", "args": []},
            {"command": "setval", "args": ["11"]}
        ]),
    );
    map.script_map_commands
        .retain(|command| command.source_script != "RuntimeTrainerScript");
    map.script_flag_commands
        .retain(|command| command.source_script != "RuntimeTrainerScript");
    map.script_runtime_commands.extend([
        ScriptRuntimeCommand {
            command: "winlosstext".to_string(),
            args: vec!["RuntimeWinText".to_string(), "RuntimeLossText".to_string()],
            source_script: "RuntimeTrainerScript".to_string(),
            command_index: 6,
        },
        ScriptRuntimeCommand {
            command: "loadtrainer".to_string(),
            args: vec!["RIVAL1".to_string(), "RIVAL1".to_string()],
            source_script: "RuntimeTrainerScript".to_string(),
            command_index: 7,
        },
    ]);
    map.script_variable_commands.push(ScriptVariableCommand {
        command: "setval".to_string(),
        target: None,
        value_tokens: vec!["11".to_string()],
        source_script: "RuntimeTrainerScript".to_string(),
        command_index: 9,
    });
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("player party Pokemon");
    session.state.sync_party_from_storage();

    let mut dispatch_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("dispatch trainer shell");
    let dispatch_player =
        Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
    dispatch_shell
        .session
        .state
        .storage
        .register_capture_in_box(0, dispatch_player)
        .expect("dispatch player party Pokemon");
    dispatch_shell.session.state.sync_party_from_storage();
    let win_loss_step = dispatch_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeTrainerScript",
            6,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs { accepted: None },
        )
        .expect("compiled winlosstext dispatch");
    assert_eq!(
        win_loss_step.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeTrainerScript".to_string(),
            command_index: 7,
        })
    );
    assert_eq!(
        dispatch_shell
            .session
            .state()
            .script_runtime
            .memory
            .get("wWinTextPointer")
            .map(String::as_str),
        Some("RuntimeWinText")
    );
    assert_eq!(
        dispatch_shell
            .session
            .state()
            .script_runtime
            .memory
            .get("wLossTextPointer")
            .map(String::as_str),
        Some("RuntimeLossText")
    );
    let load_trainer_step = dispatch_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeTrainerScript",
            7,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs { accepted: None },
        )
        .expect("compiled loadtrainer dispatch");
    assert_eq!(
        load_trainer_step.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeTrainerScript".to_string(),
            command_index: 8,
        })
    );
    for (symbol, expected) in [
        ("wBattleScriptFlags", "129"),
        ("wOtherTrainerClass", "RIVAL1"),
        ("wOtherTrainerID", "RIVAL1"),
    ] {
        assert_eq!(
            dispatch_shell
                .session
                .state()
                .script_runtime
                .memory
                .get(symbol)
                .map(String::as_str),
            Some(expected),
            "{symbol}"
        );
    }
    let dispatched = dispatch_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeTrainerScript",
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs { accepted: None },
        )
        .expect("compiled scripted trainer battle dispatch");
    let RuntimeMutationResult::ScriptedTrainerBattleStarted(dispatched_start) =
        dispatched.mutation.result
    else {
        panic!("compiled startbattle must dispatch as scripted trainer battle");
    };
    let TrainerBattleStartStatus::Started(dispatched_start) = dispatched_start else {
        panic!("dispatched trainer should not already be defeated");
    };
    assert_eq!(dispatched_start.trainer_id, "RIVAL1");
    assert_eq!(dispatched_start.trainer_class, "RIVAL1");
    assert_eq!(dispatched_start.win_text, "RuntimeWinText");
    assert_eq!(dispatched_start.loss_text, "RuntimeLossText");
    assert!(matches!(
        dispatched.boundary,
        Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::TrainerBattle
        ))
    ));

    let start = session
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("scripted trainer battle resolves");
    let TrainerBattleStartStatus::Started(start) = start else {
        panic!("trainer should not already be defeated");
    };

    assert_eq!(start.trainer_id, "RIVAL1");
    assert_eq!(start.trainer_class, "RIVAL1");
    assert_eq!(start.enemy_pokemon.species.id, "CHIKORITA");
    assert_eq!(session.state.battle, BattleMemory::from(&start));
    assert!(session.state.pokedex.has_seen("CHIKORITA"));
    assert_eq!(session.state.battle_active_party_index, Some(0));
    assert_eq!(session.state.battle_active_enemy_party_index, Some(0));
    let battle_snapshot = RuntimeBattleSnapshot::from_state(&session.state)
        .expect("trainer battle snapshot resolves")
        .expect("trainer battle snapshot");
    assert_eq!(battle_snapshot.battle_music, "MUSIC_RIVAL_ENCOUNTER");
    session
        .state
        .bag
        .add_item(&runtime.data.items["MASTER_BALL"], 1)
        .expect("add master ball");
    let blocked = session
        .throw_ball_at_active_battle(&runtime, "MASTER_BALL")
        .expect("trainer battle ball throw resolves as blocked")
        .outcome
        .expect("ball was available");
    assert!(blocked.blocked);
    assert!(!blocked.caught);
    assert_eq!(
        session
            .state
            .bag
            .quantity(&runtime.data.items["MASTER_BALL"]),
        0
    );
    match &mut session.state.battle {
        BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        other => panic!("expected trainer battle, got {other:?}"),
    }
    session.state.battle_pay_day_money = 75;
    let trainer_rewards = session
        .claim_active_trainer_battle_rewards(&runtime)
        .expect("claim trainer rewards");
    assert_eq!(
        trainer_rewards.outcome.experience_awarded,
        crystal_core::systems::battle_rewards::trainer_experience_award(
            &runtime.data.battle_reward_rules,
            match &session.state.battle {
                BattleMemory::Trainer { enemy_pokemon, .. } => enemy_pokemon,
                other => panic!("expected trainer battle, got {other:?}"),
            }
        )
        .expect("trainer experience")
    );
    let completion = session
        .complete_scripted_trainer_battle(
            &runtime,
            "RuntimeMap",
            "RuntimeTrainerScript",
            8,
            true,
            false,
        )
        .expect("scripted trainer battle completes");
    assert!(completion.continued_after_battle);
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_active_party_index, None);
    assert_eq!(session.state.battle_active_enemy_party_index, None);
    assert_eq!(completion.trainer_prize_money, Some(2000));
    assert_eq!(completion.money_after, Some(5075));
    assert_eq!(session.state.money, 5075);
    assert_eq!(session.state.battle_pay_day_money, 0);
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("0")
    );
    assert_eq!(
        session
            .state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        session
            .state
            .flags
            .is_event_flag_set("EVENT_BEAT_RUNTIME_RIVAL"),
        Ok(true)
    );
    assert_eq!(
        session
            .state
            .flags
            .is_event_flag_set("EVENT_RUNTIME_TRAINER_POST"),
        Ok(false),
        "battle completion alone must not execute source setevent"
    );
    assert_eq!(
        session
            .state
            .flags
            .is_engine_flag_set("ENGINE_RUNTIME_TRAINER_POST"),
        Ok(false),
        "battle completion alone must not execute source setflag"
    );

    let mut battle_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
        .expect("trainer battle runner shell");
    let shell_player =
        Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
    battle_shell
        .session
        .state
        .storage
        .register_capture_in_box(0, shell_player)
        .expect("shell player party Pokemon");
    battle_shell.session.state.sync_party_from_storage();
    for command_index in [6, 7] {
        battle_shell
            .step_compiled_script_command(
                "RuntimeMap",
                "RuntimeTrainerScript",
                command_index,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("step scripted trainer setup command");
    }
    let start_step = battle_shell
        .step_compiled_script_command(
            "RuntimeMap",
            "RuntimeTrainerScript",
            8,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("step scripted trainer startbattle");
    assert!(matches!(
        start_step.boundary,
        Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::TrainerBattle
        ))
    ));
    assert_eq!(
        start_step.next_cursor,
        Some(RuntimeCompiledScriptCursor {
            origin_map_name: "RuntimeMap".to_string(),
            source_script: "RuntimeTrainerScript".to_string(),
            command_index: 9,
        })
    );
    match &mut battle_shell.session.state.battle {
        BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        other => panic!("expected trainer battle, got {other:?}"),
    }
    battle_shell
        .claim_active_trainer_battle_rewards()
        .expect("claim shell trainer rewards");
    let resumed_battle = battle_shell
        .complete_scripted_trainer_battle_and_run_compiled_script(
            "RuntimeMap",
            "RuntimeTrainerScript",
            8,
            true,
            false,
            4,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("complete trainer battle and continue script");
    assert!(resumed_battle.completion.continued_after_battle);
    assert_eq!(resumed_battle.completion.trainer_prize_money, Some(2000));
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
        Some("11")
    );

    let error = session
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 9)
        .expect_err("command indexes are exact");
    let error = error_debug(error);
    assert!(
        error.contains("has no scripted or trainer-table battle at RuntimeTrainerScript:9"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_trainer_battle_advances_through_exact_compiled_party() {
    let root = temp_repository_root("trainer-battle-party-advance");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data_with_scripted_battles();
    let trainer = data
        .trainers
        .trainers
        .get_mut("RIVAL1")
        .expect("trainer exists");
    trainer.party.push(TrainerPartyPokemon {
        species: "CHIKORITA".to_string(),
        level: 6,
        item: None,
        moves: Vec::new(),
        dvs: Dv::from_non_hp(1, 1, 1, 1),
    });
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("overworld session");
    let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
    session
        .state
        .storage
        .register_capture_in_box(0, player)
        .expect("player party Pokemon");
    session.state.sync_party_from_storage();
    let start = session
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("scripted trainer battle resolves");
    let TrainerBattleStartStatus::Started(start) = start else {
        panic!("trainer should start");
    };
    assert_eq!(start.enemy_party.len(), 2);
    assert_eq!(session.state.battle_active_enemy_party_index, Some(0));
    match &mut session.state.battle {
        BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            enemy_pokemon.hp = 0;
            enemy_party[0].hp = 0;
        }
        other => panic!("expected trainer battle, got {other:?}"),
    }
    let before_unclaimed_advance = session.state.clone();
    let unclaimed = session
        .advance_active_trainer_battle(&runtime)
        .expect_err("cannot advance before reward claim");
    let unclaimed = error_debug(unclaimed);
    assert!(
        unclaimed.contains("rewards have not been claimed"),
        "{unclaimed}"
    );
    assert_eq!(session.state, before_unclaimed_advance);
    let first_rewards = session
        .claim_active_trainer_battle_rewards(&runtime)
        .expect("first trainer rewards");
    assert_eq!(
        first_rewards.outcome.experience_awarded,
        crystal_core::systems::battle_rewards::trainer_experience_award(
            &runtime.data.battle_reward_rules,
            match &session.state.battle {
                BattleMemory::Trainer { enemy_party, .. } => &enemy_party[0],
                other => panic!("expected trainer battle, got {other:?}"),
            }
        )
        .expect("trainer experience")
    );
    let duplicate_rewards = session
        .claim_active_trainer_battle_rewards(&runtime)
        .expect_err("trainer rewards cannot be claimed twice");
    let duplicate_rewards = error_debug(duplicate_rewards);
    assert!(
        duplicate_rewards.contains("RewardsAlreadyClaimed"),
        "{duplicate_rewards}"
    );

    let advance = session
        .advance_active_trainer_battle(&runtime)
        .expect("advance to next trainer Pokemon");

    let next = advance.next_enemy.expect("next trainer Pokemon");
    assert!(!advance.trainer_defeated);
    assert_eq!(next.level, 6);
    assert_eq!(session.state.battle_active_enemy_party_index, Some(1));
    match &session.state.battle {
        BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            assert_eq!(enemy_pokemon.level, 6);
            assert_eq!(enemy_party[0].hp, 0);
            assert_eq!(enemy_party[1], *enemy_pokemon);
        }
        other => panic!("expected trainer battle, got {other:?}"),
    }
    match &mut session.state.battle {
        BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            enemy_pokemon.hp = 0;
            enemy_party[1].hp = 0;
        }
        other => panic!("expected trainer battle, got {other:?}"),
    }
    let second_rewards = session
        .claim_active_trainer_battle_rewards(&runtime)
        .expect("second trainer rewards");
    assert_eq!(
        second_rewards.outcome.experience_awarded,
        crystal_core::systems::battle_rewards::trainer_experience_award(
            &runtime.data.battle_reward_rules,
            match &session.state.battle {
                BattleMemory::Trainer { enemy_party, .. } => &enemy_party[1],
                other => panic!("expected trainer battle, got {other:?}"),
            }
        )
        .expect("trainer experience")
    );

    let defeated = session
        .advance_active_trainer_battle(&runtime)
        .expect("last trainer Pokemon defeated");

    assert_eq!(defeated.next_enemy, None);
    assert!(defeated.trainer_defeated);
    assert_eq!(session.state.battle_active_enemy_party_index, Some(1));
    let completion = session
        .complete_scripted_trainer_battle(
            &runtime,
            "RuntimeMap",
            "RuntimeTrainerScript",
            8,
            true,
            false,
        )
        .expect("trainer completion clears battle");
    assert!(completion.continued_after_battle);
    assert_eq!(completion.trainer_prize_money, Some(2400));
    assert_eq!(completion.money_after, Some(5400));
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    assert_eq!(session.state.battle_active_party_index, None);
    assert_eq!(session.state.battle_active_enemy_party_index, None);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_direct_trainer_completion_settles_pay_day_before_deactivation() {
    let root = temp_repository_root("trainer-direct-pay-day");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let data = minimal_runtime_data_with_scripted_battles();
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
        .expect("player party Pokemon");
    session.state.sync_party_from_storage();
    let start = session
        .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
        .expect("scripted trainer battle resolves");
    assert!(matches!(start, TrainerBattleStartStatus::Started(_)));
    session.state.money = 25;
    session.state.battle_pay_day_money = 80;
    if let BattleMemory::Trainer {
        enemy_pokemon,
        enemy_party,
        ..
    } = &mut session.state.battle
    {
        enemy_pokemon.hp = 0;
        enemy_party[0].hp = 0;
    } else {
        panic!("expected active trainer battle");
    }
    session
        .claim_active_trainer_battle_rewards(&runtime)
        .expect("trainer rewards claimed before completion");

    let completion = session
        .complete_scripted_trainer_battle(
            &runtime,
            "RuntimeMap",
            "RuntimeTrainerScript",
            8,
            true,
            false,
        )
        .expect("trainer completion settles Pay Day");

    assert!(completion.continued_after_battle);
    assert_eq!(completion.trainer_prize_money, Some(2000));
    assert_eq!(completion.money_after, Some(2105));
    assert_eq!(session.state.money, 2105);
    assert_eq!(session.state.battle_pay_day_money, 0);
    assert_eq!(session.state.battle, BattleMemory::Inactive);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_uses_exact_pack_item_effects_and_checksums_state() {
    let root = temp_repository_root("item-use");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let mut potion = runtime_item("POTION", item_pocket("ITEM"));
    potion.effect = "RESTORE_HP".to_string();
    potion.parameter = 20;
    potion.field_menu = "ITEMMENU_PARTY".to_string();
    potion.field_usable = true;
    potion.battle_menu = "ITEMMENU_PARTY".to_string();
    potion.battle_usable = true;
    potion.consumable = true;
    let mut itemfinder = runtime_item("ITEMFINDER", item_pocket("KEY_ITEM"));
    itemfinder.effect = "ITEMFINDER".to_string();
    itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
    itemfinder.field_usable = true;
    data.items.insert("POTION".to_string(), potion);
    data.items.insert("ITEMFINDER".to_string(), itemfinder);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session.overworld.player.tile = TilePosition::new(4, 4);
    session
        .state
        .bag
        .add_item(&runtime.data.items["POTION"], 2)
        .expect("add potion");
    session
        .state
        .bag
        .add_item(&runtime.data.items["ITEMFINDER"], 1)
        .expect("add itemfinder");

    let first = session
        .use_bag_item(&runtime, "POTION", ItemUseContext::Battle)
        .expect("use potion");
    let second = session
        .use_bag_item(&runtime, "ITEMFINDER", ItemUseContext::Field)
        .expect("use itemfinder");

    assert_eq!(first.outcome.item_id, "POTION");
    assert!(first.outcome.consumed);
    assert_ne!(first.state_checksum, second.state_checksum);
    assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
    assert_eq!(
        session
            .state
            .bag
            .quantity(&runtime.data.items["ITEMFINDER"]),
        1
    );
    assert_eq!(session.state.script_runtime.item_use_events.len(), 2);
    assert_eq!(
        session.state.script_runtime.item_use_events[1].item_id,
        "ITEMFINDER"
    );

    let error = session
        .use_bag_item(&runtime, "itemfinder", ItemUseContext::Field)
        .expect_err("case changed item id rejected");
    let error = error_debug(error);
    assert!(error.contains("UnknownItem"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_bicycle_item_toggles_bike_mode_from_definitive_effect() {
    let root = temp_repository_root("bicycle-item");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let mut bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
    bicycle.effect = "MOD_BICYCLE".to_string();
    bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
    bicycle.field_usable = true;
    data.items.insert("BICYCLE".to_string(), bicycle);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .bag
        .add_item(&runtime.data.items["BICYCLE"], 1)
        .expect("add bicycle");

    let on = session
        .use_bag_bicycle_in_field(&runtime, "BICYCLE")
        .expect("use bicycle on");

    assert_eq!(on.item_use.item_id, "BICYCLE");
    assert!(!on.item_use.consumed);
    assert_eq!(on.mode_before, MovementMode::Normal);
    assert_eq!(on.mode_after, MovementMode::Bike);
    assert_eq!(on.permission, permissions::FLOOR);
    assert_eq!(session.overworld.player.mode, MovementMode::Bike);
    assert_eq!(
        session.state.overworld,
        OverworldMemory::from_snapshot(&session.overworld.snapshot())
    );
    assert_eq!(session.state.script_runtime.item_use_events.len(), 1);

    let off = session
        .use_bag_bicycle_in_field(&runtime, "BICYCLE")
        .expect("use bicycle off");

    assert_eq!(off.mode_before, MovementMode::Bike);
    assert_eq!(off.mode_after, MovementMode::Normal);
    assert_eq!(session.overworld.player.mode, MovementMode::Normal);
    assert_eq!(
        session.state.bag.quantity(&runtime.data.items["BICYCLE"]),
        1
    );
    assert_eq!(session.state.script_runtime.item_use_events.len(), 2);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_bicycle_item_rejects_invalid_contexts_without_mutation() {
    let root = temp_repository_root("bicycle-item-reject");
    write_tileset(
        &root,
        "johto",
        r#"{
  "0": [7, 7, 7, 7]
}"#,
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = verified_runtime_bootstrap_data();
    data.pokemon_cries
        .get_mut("CHIKORITA")
        .expect("runtime cry metadata")
        .cry = "CRY_CHIKORITA".to_string();
    let mut bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
    bicycle.effect = "BICYCLE".to_string();
    bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
    bicycle.field_usable = true;
    let mut bad_bicycle = runtime_item("BAD_BICYCLE", item_pocket("KEY_ITEM"));
    bad_bicycle.effect = "NONE".to_string();
    bad_bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
    bad_bicycle.field_usable = true;
    data.items.insert("BICYCLE".to_string(), bicycle);
    data.items.insert("BAD_BICYCLE".to_string(), bad_bicycle);
    data.tilesets.insert(
        "johto".to_string(),
        test_tileset(&[
            ("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"]),
            ("01", &["WALL", "WALL", "WALL", "WALL"]),
        ]),
    );
    data.maps.get_mut("RuntimeMap").expect("runtime map").blocks = vec![0, 1];
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .bag
        .add_item(&runtime.data.items["BICYCLE"], 1)
        .expect("add bicycle");
    session
        .state
        .bag
        .add_item(&runtime.data.items["BAD_BICYCLE"], 1)
        .expect("add bad bicycle");
    session.overworld.player.tile = TilePosition::new(2, 0);
    session.state.overworld = OverworldMemory::from_snapshot(&session.overworld.snapshot());

    let before_wall = session.state.clone();
    let wall = session
        .use_bag_bicycle_in_field(&runtime, "BICYCLE")
        .expect_err("wall permission rejects bicycle");
    let wall = error_debug(wall);
    assert!(wall.contains("permission 0x07"), "{wall}");
    assert_eq!(session.state, before_wall);
    assert_eq!(session.overworld.player.mode, MovementMode::Normal);

    let before_bad_effect = session.state.clone();
    let bad_effect = session
        .use_bag_bicycle_in_field(&runtime, "BAD_BICYCLE")
        .expect_err("wrong effect rejects bicycle");
    let bad_effect = error_debug(bad_effect);
    assert!(bad_effect.contains("InvalidFieldItemId"), "{bad_effect}");
    assert_eq!(session.state, before_bad_effect);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_bicycle_item_rejects_disallowed_environment_and_always_on_dismount() {
    let root = temp_repository_root("bicycle-item-env");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let mut bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
    bicycle.effect = "BICYCLE".to_string();
    bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
    bicycle.field_usable = true;
    data.items.insert("BICYCLE".to_string(), bicycle);
    data.runtime_map_metadata.insert(
        "RUNTIME_MAP".to_string(),
        runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "INDOOR"),
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .bag
        .add_item(&runtime.data.items["BICYCLE"], 1)
        .expect("add bicycle");
    let before_indoor = session.state.clone();
    let indoor = session
        .use_bag_bicycle_in_field(&runtime, "BICYCLE")
        .expect_err("indoor rejects bicycle");
    let indoor = error_debug(indoor);
    assert!(indoor.contains("environment INDOOR"), "{indoor}");
    assert_eq!(session.state, before_indoor);

    let mut route_data = minimal_runtime_data();
    let mut route_bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
    route_bicycle.effect = "BICYCLE".to_string();
    route_bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
    route_bicycle.field_usable = true;
    route_data
        .items
        .insert("BICYCLE".to_string(), route_bicycle);
    let route_runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(route_data, report()),
        identity(),
    )
    .expect("route runtime");
    let mut route_session = route_runtime
        .start_overworld_session(&asset_root, 0)
        .expect("route session starts");
    route_session
        .state
        .bag
        .add_item(&route_runtime.data.items["BICYCLE"], 1)
        .expect("add route bicycle");
    route_session
        .state
        .flags
        .set_engine_flag("ENGINE_ALWAYS_ON_BIKE", true)
        .expect("set always on bike");
    route_session.overworld.player.mode = MovementMode::Bike;
    route_session.state.overworld =
        OverworldMemory::from_snapshot(&route_session.overworld.snapshot());
    let before_always_on = route_session.state.clone();
    let always_on = route_session
        .use_bag_bicycle_in_field(&route_runtime, "BICYCLE")
        .expect_err("always-on bike rejects dismount");
    let always_on = error_debug(always_on);
    assert!(always_on.contains("ENGINE_ALWAYS_ON_BIKE"), "{always_on}");
    assert_eq!(route_session.state, before_always_on);
    let _ = std::fs::remove_dir_all(root);
}

fn itemfinder_item() -> Item {
    let mut itemfinder = runtime_item("ITEMFINDER", item_pocket("KEY_ITEM"));
    itemfinder.effect = "ITEMFINDER".to_string();
    itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
    itemfinder.field_usable = true;
    itemfinder
}

fn hidden_item_event(script: &str, x: u16, y: u16) -> BackgroundEvent {
    BackgroundEvent {
        x,
        y,
        event_type: "BGEVENT_ITEM".to_string(),
        script: script.to_string(),
    }
}

fn hidden_item_pickup(script: &str, item_id: &str, event_flag: &str) -> ScriptFieldPickup {
    ScriptFieldPickup {
        command: "hiddenitem".to_string(),
        item_id: Some(item_id.to_string()),
        quantity: 1,
        event_flag: Some(event_flag.to_string()),
        fruit_tree_id: None,
        source_script: script.to_string(),
        command_index: 0,
    }
}

#[test]
fn runtime_itemfinder_reports_uncollected_hidden_item_from_definitive_pack_events() {
    let root = temp_repository_root("itemfinder");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let mut itemfinder_item = itemfinder_item();
    itemfinder_item.effect = "MOD_ITEMFINDER".to_string();
    data.items.insert("ITEMFINDER".to_string(), itemfinder_item);
    data.items.insert(
        "POTION".to_string(),
        runtime_item("POTION", item_pocket("ITEM")),
    );
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    map.events.bg_events = vec![hidden_item_event("HiddenPotion", 4, 4)];
    map.script_field_pickups = vec![hidden_item_pickup(
        "HiddenPotion",
        "POTION",
        "EVENT_HIDDEN_POTION",
    )];
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .bag
        .add_item(&runtime.data.items["ITEMFINDER"], 1)
        .expect("add itemfinder");

    let itemfinder = session
        .use_bag_itemfinder_in_field(&runtime, "ITEMFINDER")
        .expect("use itemfinder");

    assert_eq!(itemfinder.item_use.item_id, "ITEMFINDER");
    assert!(!itemfinder.item_use.consumed);
    assert_eq!(itemfinder.itemfinder_sound_cues, 8);
    assert_eq!(
        session
            .state
            .bag
            .quantity(&runtime.data.items["ITEMFINDER"]),
        1
    );
    assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
    let found = itemfinder.found.expect("hidden item found");
    assert_eq!(found.map_name, "RuntimeMap");
    assert_eq!(found.tile, TilePosition::new(4, 4));
    assert_eq!(found.source_script, "HiddenPotion");
    assert_eq!(found.event_flag, "EVENT_HIDDEN_POTION");
    assert_eq!(found.item_id, "POTION");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_itemfinder_ignores_collected_or_out_of_range_hidden_items() {
    let root = temp_repository_root("itemfinder-empty");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.items
        .insert("ITEMFINDER".to_string(), itemfinder_item());
    data.items.insert(
        "POTION".to_string(),
        runtime_item("POTION", item_pocket("ITEM")),
    );
    let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
    map.events.bg_events = vec![
        hidden_item_event("HiddenPotion", 4, 4),
        hidden_item_event("FarHiddenPotion", 20, 20),
    ];
    map.script_field_pickups = vec![
        hidden_item_pickup("HiddenPotion", "POTION", "EVENT_HIDDEN_POTION"),
        hidden_item_pickup("FarHiddenPotion", "POTION", "EVENT_FAR_HIDDEN_POTION"),
    ];
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .bag
        .add_item(&runtime.data.items["ITEMFINDER"], 1)
        .expect("add itemfinder");
    session
        .state
        .flags
        .set_event_flag("EVENT_HIDDEN_POTION", true)
        .expect("collect hidden item");

    let itemfinder = session
        .use_bag_itemfinder_in_field(&runtime, "ITEMFINDER")
        .expect("use itemfinder");

    assert_eq!(itemfinder.found, None);
    assert_eq!(itemfinder.itemfinder_sound_cues, 0);
    assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_itemfinder_rejects_wrong_effect_without_mutation() {
    let root = temp_repository_root("itemfinder-reject");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let mut bad_itemfinder = runtime_item("BAD_ITEMFINDER", item_pocket("KEY_ITEM"));
    bad_itemfinder.effect = "NONE".to_string();
    bad_itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
    bad_itemfinder.field_usable = true;
    data.items
        .insert("BAD_ITEMFINDER".to_string(), bad_itemfinder);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .bag
        .add_item(&runtime.data.items["BAD_ITEMFINDER"], 1)
        .expect("add bad itemfinder");
    let before = session.state.clone();

    let error = session
        .use_bag_itemfinder_in_field(&runtime, "BAD_ITEMFINDER")
        .expect_err("wrong effect rejected");

    let error = error_debug(error);
    assert!(error.contains("InvalidFieldItemId"), "{error}");
    assert_eq!(session.state, before);
    let _ = std::fs::remove_dir_all(root);
}

fn squirtbottle_item() -> Item {
    let mut squirtbottle = runtime_item("SQUIRTBOTTLE", item_pocket("KEY_ITEM"));
    squirtbottle.effect = "SQUIRTBOTTLE".to_string();
    squirtbottle.field_menu = "ITEMMENU_CLOSE".to_string();
    squirtbottle.field_usable = true;
    squirtbottle
}

fn coin_case_item() -> Item {
    let mut coin_case = runtime_item("COIN_CASE", item_pocket("KEY_ITEM"));
    coin_case.effect = "COIN_CASE".to_string();
    coin_case.field_menu = "ITEMMENU_CLOSE".to_string();
    coin_case.field_usable = true;
    coin_case
}

fn blue_card_item() -> Item {
    let mut blue_card = runtime_item("BLUE_CARD", item_pocket("KEY_ITEM"));
    blue_card.effect = "BLUE_CARD".to_string();
    blue_card.field_menu = "ITEMMENU_CLOSE".to_string();
    blue_card.field_usable = true;
    blue_card
}

fn town_map_item() -> Item {
    let mut town_map = runtime_item("TOWN_MAP", item_pocket("KEY_ITEM"));
    town_map.effect = "TOWN_MAP".to_string();
    town_map.field_menu = "ITEMMENU_CURRENT".to_string();
    town_map.field_usable = true;
    town_map
}

fn add_runtime_landmark(data: &mut GameDataSet) {
    data.pokegear_landmarks.landmarks.push(PokegearLandmark {
        id: 1,
        constant: "LANDMARK_RUNTIME_TOWN".to_string(),
        label: "RUNTIME_TOWN".to_string(),
        name: "RUNTIME TOWN".to_string(),
        x: 12,
        y: 24,
        region: "JOHTO".to_string(),
    });
    data.pokegear_landmarks.map_to_landmark.insert(
        "RuntimeMap".to_string(),
        "LANDMARK_RUNTIME_TOWN".to_string(),
    );
}

fn add_runtime_species_presentation(data: &mut GameDataSet, species_id: &str) {
    data.pokemon_cries.insert(
        species_id.to_string(),
        PokemonCryMetadata {
            cry: "CRY_CHIKORITA".to_string(),
            pitch: 0,
            length: 0,
        },
    );
    data.menu_icons
        .insert(species_id.to_string(), format!("ICON_{species_id}"));
    data.pokedex_entries.insert(
        species_id.to_string(),
        RuntimePokedexEntry {
            species: species_id.to_string(),
            classification: "Runtime".to_string(),
            height_digits: 1,
            weight_digits: 1,
            pages: vec![format!("{species_id} runtime entry.")],
        },
    );
    data.pokemon_frontpic_anim.insert(
        species_id.to_string(),
        FrontpicAnimProgram {
            commands: vec![crystal_core::models::FrontpicAnimCommand {
                kind: "frame".to_string(),
                frame: Some(0),
                duration: Some(8),
                ..crystal_core::models::FrontpicAnimCommand::default()
            }],
        },
    );
}

fn wounded_runtime_pokemon(species_id: &str) -> Pokemon {
    let mut species = runtime_species();
    species.id = species_id.to_string();
    let mut pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
    pokemon.moves = vec![LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 1,
        pp_ups: 1,
    }];
    pokemon.hp = 1;
    pokemon.status = Some(minimal_step_event_rules().poison_status);
    pokemon.sleep_turns = 2;
    pokemon.confusion_turns = 3;
    pokemon.focus_energy = true;
    pokemon
}

fn refresh_runtime_pokemon_stats(pokemon: &mut Pokemon) {
    let missing_hp = pokemon.max_hp.saturating_sub(pokemon.hp);
    let stats = calculate_stats(
        &pokemon.species,
        pokemon.level,
        pokemon.dvs,
        StatExperience {
            hp: pokemon.hp_exp,
            attack: pokemon.attack_exp,
            defense: pokemon.defense_exp,
            speed: pokemon.speed_exp,
            special: pokemon.special_exp,
        },
    );
    pokemon.max_hp = stats.max_hp;
    pokemon.attack = stats.attack;
    pokemon.defense = stats.defense;
    pokemon.speed = stats.speed;
    pokemon.special_attack = stats.special_attack;
    pokemon.special_defense = stats.special_defense;
    pokemon.hp = pokemon.max_hp.saturating_sub(missing_hp).max(1);
}

fn add_runtime_party_pokemon(runtime: &CrystalRuntime, session: &mut RuntimeOverworldSession) {
    let pokemon = runtime
        .data
        .create_pokemon("CHIKORITA", 5, Dv::default())
        .expect("create runtime party pokemon");
    session
        .state
        .storage
        .register_capture_in_box(0, pokemon)
        .expect("register runtime party pokemon");
    session.state.sync_party_from_storage();
}

#[test]
fn runtime_special_heal_party_requires_pack_declared_routine_and_restores_party() {
    let root = temp_repository_root("special-heal-party");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.special_routines
        .insert("HealParty".to_string(), SpecialRoutineRule::default());
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .storage
        .register_capture_in_box(0, wounded_runtime_pokemon("CHIKORITA"))
        .expect("store first");
    session
        .state
        .storage
        .register_capture_in_box(0, wounded_runtime_pokemon("CYNDAQUIL"))
        .expect("store second");
    session.state.sync_party_from_storage();

    let special = session
        .apply_special_routine(&runtime, "HealParty")
        .expect("heal party");

    assert_eq!(special.outcome.routine, "HealParty");
    assert_eq!(
        special.outcome.effect,
        SpecialRoutineEffect::HealParty {
            healed_slots: vec![0, 1]
        }
    );
    assert_eq!(
        session.state.script_runtime.last_special_routine.as_deref(),
        Some("HealParty")
    );
    for slot in 0..2 {
        let pokemon = session.state.storage.party.pokemon[slot]
            .as_ref()
            .expect("party pokemon");
        assert_eq!(pokemon.hp, pokemon.max_hp);
        assert_eq!(pokemon.status, None);
        assert_eq!(pokemon.sleep_turns, 2);
        assert_eq!(pokemon.confusion_turns, 3);
        assert!(pokemon.focus_energy);
        assert_eq!(pokemon.moves[0].current_pp, 35);
        assert_eq!(
            session.state.party.pokemon[slot]
                .as_ref()
                .expect("projected party")
                .species
                .as_str(),
            pokemon.species.id
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_routine_rejects_missing_or_unsupported_exact_routine_without_mutation() {
    let root = temp_repository_root("special-routine-reject");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.special_routines
        .insert("Function11ac3e".to_string(), SpecialRoutineRule::default());
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session
        .state
        .storage
        .register_capture_in_box(0, wounded_runtime_pokemon("CHIKORITA"))
        .expect("store");
    session.state.sync_party_from_storage();

    let before_missing = session.state.clone();
    let missing = session
        .apply_special_routine(&runtime, "HealParty")
        .expect_err("undeclared routine rejected");
    let missing = error_debug(missing);
    assert!(
        missing.contains("missing exact special routine HealParty"),
        "{missing}"
    );
    assert_eq!(session.state, before_missing);

    let before_case = session.state.clone();
    let case_changed = session
        .apply_special_routine(&runtime, "fadeoutmusic")
        .expect_err("case changed routine rejected before execution");
    let case_changed = error_debug(case_changed);
    assert!(
        case_changed.contains("missing exact special routine fadeoutmusic"),
        "{case_changed}"
    );
    assert_eq!(session.state, before_case);

    let before_inactive = session.state.clone();
    let inactive = session
        .apply_special_routine(&runtime, "Function11ac3e")
        .expect_err("inactive declared routine rejected");
    let inactive = error_debug(inactive);
    assert!(
        inactive.contains("inactive") && inactive.contains("Function11ac3e"),
        "{inactive}"
    );
    assert_eq!(session.state, before_inactive);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_audio_routines_apply_exact_pack_declared_effects() {
    let root = temp_repository_root("special-audio-routines");
    write_floor_tileset(&root, "johto");
    write_pcm(
        &root
            .join("apps/web/assets/data")
            .join("content-packs/test/cries/CRY_CHIKORITA.pcm"),
    );
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.special_routines
        .insert("FadeOutMusic".to_string(), SpecialRoutineRule::default());
    data.special_routines
        .insert("WaitSFX".to_string(), SpecialRoutineRule::default());
    data.special_routines
        .insert("PlayMapMusic".to_string(), SpecialRoutineRule::default());
    data.special_routines
        .insert("RestartMapMusic".to_string(), SpecialRoutineRule::default());
    data.special_routines
        .insert("PlayCurMonCry".to_string(), SpecialRoutineRule::default());
    data.special_routines
        .insert("PlaySlowCry".to_string(), SpecialRoutineRule::default());
    data.pokemon_cries.insert(
        "CHIKORITA".to_string(),
        PokemonCryMetadata {
            cry: "CRY_CHIKORITA".to_string(),
            pitch: 0,
            length: 0,
        },
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session.state.script_runtime.current_music = Some("MUSIC_ROUTE_30".to_string());
    session.state.script_runtime.map_music_restart_disabled = true;

    let fade = session
        .apply_special_routine(&runtime, "FadeOutMusic")
        .expect("fade out music");

    assert_eq!(
        fade.outcome.effect,
        SpecialRoutineEffect::FadeOutMusic {
            audio_id: "MUSIC_NONE".to_string(),
            fade_frames: 2
        }
    );
    let pending_fade = session
        .state
        .script_runtime
        .pending_music_fade
        .as_ref()
        .expect("pending fade");
    assert_eq!(pending_fade.audio_id, "MUSIC_NONE");
    assert_eq!(pending_fade.fade_frames, 2);
    assert_eq!(pending_fade.source_script, "FadeOutMusic");
    assert_eq!(
        session.state.script_runtime.current_music.as_deref(),
        Some("MUSIC_ROUTE_30")
    );
    assert_eq!(session.state.script_runtime.audio_events.len(), 1);
    assert_eq!(
        session.state.script_runtime.audio_events[0]
            .audio_id
            .as_deref(),
        Some("MUSIC_NONE")
    );
    assert_ne!(fade.state_checksum.hash(), 0);

    let wait_sfx = session
        .apply_special_routine(&runtime, "WaitSFX")
        .expect("wait sfx");

    assert_eq!(wait_sfx.outcome.effect, SpecialRoutineEffect::WaitSfx);
    assert!(session.state.script_runtime.waiting_for_sound_effect);
    assert_eq!(session.state.script_runtime.audio_events.len(), 2);
    assert_eq!(
        session.state.script_runtime.audio_events[1].kind,
        crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect
    );
    assert_eq!(session.state.script_runtime.audio_events[1].audio_id, None);
    assert_ne!(wait_sfx.state_checksum, fade.state_checksum);

    session.state.script_runtime.map_music_restart_disabled = true;
    let play_map_music = session
        .apply_special_routine(&runtime, "PlayMapMusic")
        .expect("play map music");

    assert_eq!(
        play_map_music.outcome.effect,
        SpecialRoutineEffect::PlayMapMusic
    );
    assert!(session.state.script_runtime.map_music_requested);
    assert!(session.state.script_runtime.map_music_restart_disabled);
    assert_eq!(
        session.state.script_runtime.last_special_routine.as_deref(),
        Some("PlayMapMusic")
    );
    assert_ne!(play_map_music.state_checksum, wait_sfx.state_checksum);

    session.state.script_runtime.map_music_requested = false;
    session.state.script_runtime.map_music_restart_disabled = true;
    let restart = session
        .apply_special_routine(&runtime, "RestartMapMusic")
        .expect("restart map music");

    assert_eq!(
        restart.outcome.effect,
        SpecialRoutineEffect::RestartMapMusic
    );
    assert!(session.state.script_runtime.map_music_requested);
    assert!(session.state.script_runtime.map_music_restart_disabled);
    assert_eq!(
        session.state.script_runtime.last_special_routine.as_deref(),
        Some("RestartMapMusic")
    );
    assert_eq!(
        restart.state_checksum, play_map_music.state_checksum,
        "RestartMapMusic reasserts the same WRAM request; the differing host-only routine label is excluded from deterministic state"
    );

    session
        .state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), "CHIKORITA".to_string());
    let cur_cry = session
        .apply_special_routine(&runtime, "PlayCurMonCry")
        .expect("play current cry");

    assert_eq!(
        cur_cry.outcome.effect,
        SpecialRoutineEffect::PlayCurMonCry {
            species: "CHIKORITA".to_string(),
            audio_id: "CRY_CHIKORITA".to_string()
        }
    );
    assert_eq!(
        session
            .state
            .script_runtime
            .audio_events
            .last()
            .expect("cry event")
            .kind,
        crystal_core::state::ScriptAudioRuntimeKind::Cry
    );
    assert_eq!(
        session
            .state
            .script_runtime
            .audio_events
            .last()
            .expect("cry event")
            .audio_id
            .as_deref(),
        Some("CRY_CHIKORITA")
    );
    assert_ne!(cur_cry.state_checksum, restart.state_checksum);

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let slow_cry = session
        .apply_special_routine(&runtime, "PlaySlowCry")
        .expect("play slow cry");

    assert_eq!(
        slow_cry.outcome.effect,
        SpecialRoutineEffect::PlaySlowCry {
            species: "CHIKORITA".to_string(),
            audio_id: "CRY_CHIKORITA".to_string()
        }
    );
    assert_eq!(session.state.script_runtime.audio_events.len(), 4);
    assert_ne!(slow_cry.state_checksum, cur_cry.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_screen_fades_apply_exact_pack_declared_effects() {
    let root = temp_repository_root("special-screen-fades");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    for routine in [
        "FadeOutToWhite",
        "FadeInFromWhite",
        "FadeOutToBlack",
        "FadeInFromBlack",
    ] {
        data.special_routines
            .insert(routine.to_string(), SpecialRoutineRule::default());
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");

    let fade_out_white = session
        .apply_special_routine(&runtime, "FadeOutToWhite")
        .expect("fade out white");

    assert_eq!(
        fade_out_white.outcome.effect,
        SpecialRoutineEffect::ScreenFade {
            color: crystal_core::state::ScriptFadeColor::White,
            direction: crystal_core::state::ScriptFadeDirection::Out,
            frames: 8
        }
    );
    assert_eq!(session.state.script_runtime.graphics_events.len(), 1);
    assert_eq!(
        session
            .state
            .script_runtime
            .pending_screen_fade
            .as_ref()
            .expect("pending fade")
            .source_script,
        "FadeOutToWhite"
    );

    let fade_in_black = session
        .apply_special_routine(&runtime, "FadeInFromBlack")
        .expect("fade in black");

    assert_eq!(
        fade_in_black.outcome.effect,
        SpecialRoutineEffect::ScreenFade {
            color: crystal_core::state::ScriptFadeColor::Black,
            direction: crystal_core::state::ScriptFadeDirection::In,
            frames: 8
        }
    );
    assert_eq!(session.state.script_runtime.graphics_events.len(), 2);
    assert_eq!(
        session
            .state
            .script_runtime
            .pending_screen_fade
            .as_ref()
            .expect("pending fade")
            .source_script,
        "FadeInFromBlack"
    );
    assert_ne!(fade_out_white.state_checksum, fade_in_black.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_graphics_and_hardware_commands_apply_exact_pack_declared_effects() {
    let root = temp_repository_root("special-graphics-commands");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    for routine in [
        "GameboyCheck",
        "CheckMobileAdapterStatusSpecial",
        "ClearBGPalettesBufferScreen",
        "ClearBGPalettes",
        "UpdateTimePals",
        "ClearTilemap",
        "LoadMapPalettes",
        "RefreshSprites",
        "UpdateSprites",
        "ReloadSpritesNoPalettes",
    ] {
        data.special_routines
            .insert(routine.to_string(), SpecialRoutineRule::default());
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");

    let gameboy = session
        .apply_special_routine(&runtime, "GameboyCheck")
        .expect("gameboy check");

    assert_eq!(
        gameboy.outcome.effect,
        SpecialRoutineEffect::GameboyCheck {
            token: "GBCHECK_CGB".to_string()
        }
    );
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("GBCHECK_CGB")
    );

    let mobile = session
        .apply_special_routine(&runtime, "CheckMobileAdapterStatusSpecial")
        .expect("mobile adapter status");

    assert_eq!(
        mobile.outcome.effect,
        SpecialRoutineEffect::MobileAdapterStatus {
            value: "0".to_string()
        }
    );
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("0")
    );
    assert_eq!(
        session
            .state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("0")
    );

    let clear = session
        .apply_special_routine(&runtime, "ClearBGPalettes")
        .expect("clear palettes");

    assert_eq!(
        clear.outcome.effect,
        SpecialRoutineEffect::GraphicsCommand {
            kind: crystal_core::state::ScriptGraphicsRuntimeKind::ClearBgPalettes
        }
    );
    assert_eq!(session.state.script_runtime.graphics_events.len(), 1);
    assert_eq!(
        session.state.script_runtime.graphics_events[0].kind,
        crystal_core::state::ScriptGraphicsRuntimeKind::ClearBgPalettes
    );
    assert_eq!(session.state.script_runtime.graphics_events[0].color, None);

    let reload = session
        .apply_special_routine(&runtime, "ReloadSpritesNoPalettes")
        .expect("reload sprites");

    assert_eq!(
        reload.outcome.effect,
        SpecialRoutineEffect::GraphicsCommand {
            kind: crystal_core::state::ScriptGraphicsRuntimeKind::ReloadSpritesNoPalettes
        }
    );
    assert_eq!(session.state.script_runtime.graphics_events.len(), 2);
    assert_eq!(
        session.state.script_runtime.graphics_events[1].kind,
        crystal_core::state::ScriptGraphicsRuntimeKind::ReloadSpritesNoPalettes
    );
    assert_eq!(
        session.state.script_runtime.last_special_routine.as_deref(),
        Some("ReloadSpritesNoPalettes")
    );
    assert_ne!(gameboy.state_checksum, mobile.state_checksum);
    assert_ne!(mobile.state_checksum, clear.state_checksum);
    assert_ne!(clear.state_checksum, reload.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_party_checks_apply_exact_pack_declared_effects() {
    let root = temp_repository_root("special-party-checks");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.special_routines.insert(
        "GetFirstPokemonHappiness".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "CheckFirstMonIsEgg".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "FindPartyMonThatSpecies".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "FindPartyMonThatSpeciesYourTrainerID".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "FindPartyMonAboveLevel".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "FindPartyMonAtLeastThatHappy".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines
        .insert("MonCheck".to_string(), SpecialRoutineRule::default());
    data.special_routines
        .insert("BeastsCheck".to_string(), SpecialRoutineRule::default());
    data.special_routines.insert(
        "GameCornerPrizeMonCheckDex".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "UnusedSetSeenMon".to_string(),
        SpecialRoutineRule::default(),
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    let mut egg = wounded_runtime_pokemon("EGG");
    egg.nickname = "EGG".to_string();
    egg.happiness = 1;
    egg.is_egg = true;
    let mut chikorita = wounded_runtime_pokemon("CHIKORITA");
    chikorita.nickname = "Leafy".to_string();
    chikorita.level = 31;
    refresh_runtime_pokemon_stats(&mut chikorita);
    chikorita.happiness = 201;
    chikorita.original_trainer_name = "KRIS".to_string();
    chikorita.original_trainer_id = 0x2222;
    session.state.player_name = "KRIS".to_string();
    session.state.player_id = 0x2222;
    session
        .state
        .storage
        .register_capture_in_box(0, egg)
        .expect("store egg");
    session
        .state
        .storage
        .register_capture_in_box(0, chikorita)
        .expect("store mon");
    session.state.sync_party_from_storage();

    let egg_check = session
        .apply_special_routine(&runtime, "CheckFirstMonIsEgg")
        .expect("egg check");

    assert_eq!(
        egg_check.outcome.effect,
        SpecialRoutineEffect::CheckFirstMonIsEgg {
            species: "EGG".to_string(),
            nickname: "EGG".to_string(),
            is_egg: true
        }
    );
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("1")
    );

    let happiness = session
        .apply_special_routine(&runtime, "GetFirstPokemonHappiness")
        .expect("happiness");

    assert_eq!(
        happiness.outcome.effect,
        SpecialRoutineEffect::FirstPokemonHappiness {
            party_slot: 1,
            species: "CHIKORITA".to_string(),
            nickname: "Leafy".to_string(),
            happiness: 201
        }
    );
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("201")
    );
    assert_eq!(
        session
            .state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("Leafy")
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let found = session
        .apply_special_routine(&runtime, "FindPartyMonThatSpecies")
        .expect("find party mon");

    assert_eq!(
        found.outcome.effect,
        SpecialRoutineEffect::FindPartyMonThatSpecies {
            species: "CHIKORITA".to_string(),
            found: true
        }
    );
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("1")
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "30".to_string());
    let above_level = session
        .apply_special_routine(&runtime, "FindPartyMonAboveLevel")
        .expect("find above level");

    assert_eq!(
        above_level.outcome.effect,
        SpecialRoutineEffect::FindPartyMonAboveLevel {
            level: 30,
            found: true,
            species: Some("CHIKORITA".to_string())
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "202".to_string());
    let happy = session
        .apply_special_routine(&runtime, "FindPartyMonAtLeastThatHappy")
        .expect("find happy");

    assert_eq!(
        happy.outcome.effect,
        SpecialRoutineEffect::FindPartyMonAtLeastThatHappy {
            happiness: 202,
            found: false,
            species: None
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let ot_found = session
        .apply_special_routine(&runtime, "FindPartyMonThatSpeciesYourTrainerID")
        .expect("find ot species");

    assert_eq!(
        ot_found.outcome.effect,
        SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId {
            species: "CHIKORITA".to_string(),
            player_name: "KRIS".to_string(),
            player_id: 0x2222,
            found: true
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let mon_check = session
        .apply_special_routine(&runtime, "MonCheck")
        .expect("mon check");

    assert_eq!(
        mon_check.outcome.effect,
        SpecialRoutineEffect::MonCheck {
            species: "CHIKORITA".to_string(),
            player_name: "KRIS".to_string(),
            player_id: 0x2222,
            owned: true
        }
    );

    for species in ["RAIKOU", "ENTEI", "SUICUNE"] {
        let mut beast = wounded_runtime_pokemon(species);
        beast.original_trainer_name = "KRIS".to_string();
        beast.original_trainer_id = 0x2222;
        session
            .state
            .storage
            .register_capture_in_box(0, beast)
            .expect("store beast");
    }
    session.state.sync_party_from_storage();
    let beasts = session
        .apply_special_routine(&runtime, "BeastsCheck")
        .expect("beasts check");

    assert_eq!(
        beasts.outcome.effect,
        SpecialRoutineEffect::BeastsCheck {
            player_name: "KRIS".to_string(),
            player_id: 0x2222,
            missing_species: None,
            owned_all: true
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let prize_dex = session
        .apply_special_routine(&runtime, "GameCornerPrizeMonCheckDex")
        .expect("game corner prize dex");

    assert_eq!(
        prize_dex.outcome.effect,
        SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
            species: "CHIKORITA".to_string(),
            species_int_id: 1,
            already_caught: false,
            recorded_caught: true
        }
    );
    assert!(session.state.pokedex.has_seen("CHIKORITA"));
    assert!(session.state.pokedex.has_caught("CHIKORITA"));

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let set_seen = session
        .apply_special_routine(&runtime, "UnusedSetSeenMon")
        .expect("unused set seen mon");

    assert_eq!(
        set_seen.outcome.effect,
        SpecialRoutineEffect::UnusedSetSeenMon {
            species: "CHIKORITA".to_string(),
            species_int_id: 1,
            newly_seen: false
        }
    );
    assert_ne!(egg_check.state_checksum, happiness.state_checksum);
    assert_ne!(happiness.state_checksum, found.state_checksum);
    assert_eq!(
        found.state_checksum, above_level.state_checksum,
        "both successful searches leave the same wScriptVar/_value result; their host-only routine labels are not gameplay state"
    );
    assert_ne!(above_level.state_checksum, happy.state_checksum);
    assert_ne!(happy.state_checksum, ot_found.state_checksum);
    assert_eq!(
        ot_found.state_checksum, mon_check.state_checksum,
        "the OT lookup and MonCheck both leave the same successful script result"
    );
    assert_ne!(mon_check.state_checksum, beasts.state_checksum);
    assert_ne!(beasts.state_checksum, prize_dex.state_checksum);
    assert_ne!(prize_dex.state_checksum, set_seen.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_pc_display_and_move_deletion_apply_pack_declared_effects() {
    let root = temp_repository_root("special-pc-display-move-deletion");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    for routine in [
        "PokemonCenterPC",
        "PlayersHousePC",
        "ProfOaksPCBoot",
        "OverworldTownMap",
        "UnownPrinter",
        "MapRadio",
        "NameRival",
        "MoveDeletion",
    ] {
        data.special_routines
            .insert(routine.to_string(), SpecialRoutineRule::default());
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    let mut pokemon = wounded_runtime_pokemon("CHIKORITA");
    pokemon.moves.push(LearnedMove {
        name: "GROWL".to_string(),
        current_pp: 1,
        pp_ups: 0,
    });
    session
        .state
        .storage
        .register_capture_in_box(0, pokemon)
        .expect("store party mon");
    session.state.sync_party_from_storage();
    session.state.current_pc_box = 2;

    let pc = session
        .apply_special_routine(&runtime, "PokemonCenterPC")
        .expect("pokemon center pc");

    assert_eq!(
        pc.outcome.effect,
        SpecialRoutineEffect::PokemonCenterPc {
            party_count: 1,
            current_pc_box: 2
        }
    );
    assert_eq!(
        session.state.script_runtime.active_menu.as_deref(),
        Some("PokemonCenterPC")
    );

    let player_pc = session
        .apply_special_routine(&runtime, "PlayersHousePC")
        .expect("players house pc");

    assert_eq!(
        player_pc.outcome.effect,
        SpecialRoutineEffect::PlayersHousePc { party_count: 1 }
    );

    let oak = session
        .apply_special_routine(&runtime, "ProfOaksPCBoot")
        .expect("oak pc");

    assert_eq!(
        oak.outcome.effect,
        SpecialRoutineEffect::ProfOaksPcBoot {
            seen_count: 0,
            caught_count: 0,
            rating_label: "OakRating01".to_string()
        }
    );

    let town_map = session
        .apply_special_routine(&runtime, "OverworldTownMap")
        .expect("town map");

    assert_eq!(
        town_map.outcome.effect,
        SpecialRoutineEffect::OverworldTownMap {
            map_name: Some("RuntimeMap".to_string())
        }
    );

    let printer = session
        .apply_special_routine(&runtime, "UnownPrinter")
        .expect("unown printer");

    assert_eq!(
        printer.outcome.effect,
        SpecialRoutineEffect::UnownPrinter { letters: vec![] }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "MAPRADIO_UNOWN".to_string());
    let radio = session
        .apply_special_routine(&runtime, "MapRadio")
        .expect("map radio");

    assert_eq!(
        radio.outcome.effect,
        SpecialRoutineEffect::MapRadio {
            station: "MAPRADIO_UNOWN".to_string()
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_rival_name".to_string(), "SILVER".to_string());
    let rival = session
        .apply_special_routine(&runtime, "NameRival")
        .expect("name rival");

    assert_eq!(
        rival.outcome.effect,
        SpecialRoutineEffect::NameRival {
            rival_name: "SILVER".to_string()
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    session
        .state
        .script_runtime
        .variables
        .insert("_move_slot".to_string(), "1".to_string());
    let deletion = session
        .apply_special_routine(&runtime, "MoveDeletion")
        .expect("move deletion");

    assert_eq!(
        deletion.outcome.effect,
        SpecialRoutineEffect::MoveDeletion {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            deleted_move: "GROWL".to_string(),
            remaining_moves: 1
        }
    );
    assert_eq!(
        session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("party mon")
            .moves
            .iter()
            .map(|move_slot| move_slot.name.as_str())
            .collect::<Vec<_>>(),
        vec!["TACKLE"]
    );
    assert_ne!(pc.state_checksum, player_pc.state_checksum);
    assert_ne!(player_pc.state_checksum, oak.state_checksum);
    assert_ne!(oak.state_checksum, town_map.state_checksum);
    assert_ne!(town_map.state_checksum, printer.state_checksum);
    assert_ne!(printer.state_checksum, radio.state_checksum);
    assert_ne!(radio.state_checksum, rival.state_checksum);
    assert_ne!(rival.state_checksum, deletion.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_visual_commands_apply_pack_declared_effects() {
    let root = temp_repository_root("special-visual-commands");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let cases = [
        (
            "BattleTowerFade",
            ScriptGraphicsRuntimeKind::BattleTowerFade,
        ),
        (
            "UpdatePlayerSprite",
            ScriptGraphicsRuntimeKind::UpdatePlayerSprite,
        ),
        (
            "HealMachineAnim",
            ScriptGraphicsRuntimeKind::HealMachineAnim,
        ),
        ("SurfStartStep", ScriptGraphicsRuntimeKind::SurfStartStep),
        (
            "LoadUsedSpritesGFX",
            ScriptGraphicsRuntimeKind::LoadUsedSpritesGfx,
        ),
        (
            "ToggleMaptileDecorations",
            ScriptGraphicsRuntimeKind::ToggleMaptileDecorations,
        ),
        (
            "ToggleDecorationsVisibility",
            ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility,
        ),
        ("MagnetTrain", ScriptGraphicsRuntimeKind::MagnetTrain),
        ("Diploma", ScriptGraphicsRuntimeKind::Diploma),
        ("PrintDiploma", ScriptGraphicsRuntimeKind::PrintDiploma),
        (
            "DisplayUnownWords",
            ScriptGraphicsRuntimeKind::DisplayUnownWords,
        ),
    ];
    for (routine, _) in cases {
        data.special_routines
            .insert(routine.to_string(), SpecialRoutineRule::default());
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    let mut previous_checksum = None;

    for (routine, kind) in cases {
        let use_result = session
            .apply_special_routine(&runtime, routine)
            .expect("visual special");

        assert_eq!(
            use_result.outcome.effect,
            SpecialRoutineEffect::RuntimeVisualCommand { kind }
        );
        assert_eq!(
            session.state.script_runtime.last_special_routine.as_deref(),
            Some(routine)
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .graphics_events
                .last()
                .expect("graphics event")
                .kind,
            kind
        );
        if let Some(previous_checksum) = previous_checksum {
            assert_ne!(previous_checksum, use_result.state_checksum);
        }
        previous_checksum = Some(use_result.state_checksum);
    }
    assert_eq!(
        session.state.script_runtime.graphics_events.len(),
        cases.len()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_service_routines_apply_pack_declared_effects() {
    use crystal_core::systems::special_routines::{
        HappinessChangeEntry, HappinessData, HappinessServiceOutcome,
    };

    let root = temp_repository_root("special-service-routines");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.moves
        .insert("EMBER".to_string(), runtime_move_named("EMBER", 25));
    for routine in [
        "CheckPokerus",
        "OlderHaircutBrother",
        "YoungerHaircutBrother",
        "DaisysGrooming",
        "NameRater",
        "PokeSeer",
        "MoveTutor",
    ] {
        data.special_routines
            .insert(routine.to_string(), SpecialRoutineRule::default());
    }
    data.happiness_data = Some(HappinessData {
        changes: [
            (
                5,
                HappinessChangeEntry {
                    code: "HAPPINESS_LEARNMOVE".to_string(),
                    low: 1,
                    mid: 1,
                    high: 0,
                },
            ),
            (
                9,
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT1".to_string(),
                    low: 1,
                    mid: 1,
                    high: 1,
                },
            ),
            (
                13,
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT2".to_string(),
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ),
            (
                18,
                HappinessChangeEntry {
                    code: "HAPPINESS_GROOMING".to_string(),
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ),
        ]
        .into_iter()
        .collect(),
        services: [
            (
                "OlderHaircutBrother".to_string(),
                vec![HappinessServiceOutcome {
                    roll_weight: 255,
                    script_value: 2,
                    change_code: 9,
                }],
            ),
            (
                "YoungerHaircutBrother".to_string(),
                vec![
                    HappinessServiceOutcome {
                        roll_weight: 76,
                        script_value: 2,
                        change_code: 9,
                    },
                    HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 3,
                        change_code: 13,
                    },
                ],
            ),
            (
                "DaisysGrooming".to_string(),
                vec![HappinessServiceOutcome {
                    roll_weight: 255,
                    script_value: 2,
                    change_code: 18,
                }],
            ),
        ]
        .into_iter()
        .collect(),
    });
    sync_runtime_move_tables(&mut data);
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    let mut pokemon = wounded_runtime_pokemon("CHIKORITA");
    pokemon.nickname = "Leafy".to_string();
    pokemon.happiness = 70;
    pokemon.status = Some("POKERUS".to_string());
    pokemon.original_trainer_name = "KRIS".to_string();
    pokemon.original_trainer_id = 0x2222;
    session
        .state
        .storage
        .register_capture_in_box(0, pokemon)
        .expect("store party mon");
    session.state.sync_party_from_storage();
    let mut happiness_command_shell = RuntimeGameShell {
        asset_root: asset_root.clone(),
        runtime: runtime.clone(),
        session: session.clone(),
        last_frame: None,
        linked_menu_results: Vec::new(),
        runtime_command_sequence: 0,
        runtime_commands: Vec::new(),
        runtime_results: Vec::new(),
        retain_runtime_journal: true,
    };
    happiness_command_shell.session_mut().divider =
        crystal_core::random::RuntimeDividerSource::replay([0, 0]);
    let replay_base = happiness_command_shell.session().clone();
    let before_happiness = happiness_command_shell.session().state().clone();
    let happiness_command = happiness_command_shell
        .apply_happiness_service(RuntimeHappinessServiceRoutine::OlderHaircutBrother, 0)
        .expect("happiness service command consumes live divider samples");
    assert_eq!(
        happiness_command.outcome.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 70,
            new_happiness: 71,
            script_value: 2,
            change_code: 9
        }
    );
    assert_eq!(
        happiness_command_shell.session().state().random_state,
        crystal_core::random::CrystalRandomState { add: 1, sub: 0 }
    );
    let frame = happiness_command_shell.retained_runtime_commands()[0].clone();
    let command = crystal_assets::decode_runtime_mutation_command_frame(&frame, &before_happiness)
        .expect("decode recorded happiness service command");
    let RuntimeMutationCommand::ApplyHappinessService(recorded) = command else {
        panic!("happiness service must journal a typed divider trace");
    };
    assert_eq!(recorded.divider_trace.samples, vec![0, 0]);

    let mut replayed = replay_base.clone();
    replayed
        .apply_runtime_command_frame(&runtime, &frame)
        .expect("happiness service replay consumes the exact trace");
    assert_eq!(replayed.state, happiness_command_shell.session().state);

    for (trace, message) in [
        (vec![0], "divider replay exhausted after 1 samples"),
        (vec![0, 0, 1], "1 unconsumed samples after 2 reads"),
    ] {
        let mut rejected = replay_base.clone();
        let request = rejected
            .runtime_command_frame(
                1,
                1,
                RuntimeMutationCommand::ApplyHappinessService(RuntimeHappinessServiceCommand {
                    routine: RuntimeHappinessServiceRoutine::OlderHaircutBrother,
                    party_index: 0,
                    divider_trace: RuntimeDividerTrace::new(trace),
                }),
            )
            .expect("frame malformed happiness service replay");
        let rejected_before = rejected.clone();
        let error = rejected
            .apply_runtime_command_frame(&runtime, &request)
            .expect_err("malformed happiness trace rejects atomically");
        assert!(format!("{error:#}").contains(message), "{error:#}");
        assert_eq!(rejected, rejected_before);
    }

    let pokerus = session
        .apply_special_routine(&runtime, "CheckPokerus")
        .expect("check pokerus");

    assert_eq!(
        pokerus.outcome.effect,
        SpecialRoutineEffect::CheckPokerus {
            found: true,
            newly_discovered: true
        }
    );
    assert_eq!(
        session
            .state
            .flags
            .is_engine_flag_set("ENGINE_CAUGHT_POKERUS"),
        Ok(false)
    );
    assert!(session.state.script_runtime.special_phone_call.is_none());
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("1")
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    session
        .state
        .script_runtime
        .variables
        .insert("_rng_roll".to_string(), "0".to_string());
    let older = session
        .apply_special_routine(&runtime, "OlderHaircutBrother")
        .expect("older haircut");

    assert_eq!(
        older.outcome.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 70,
            new_happiness: 71,
            script_value: 2,
            change_code: 9
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_rng_roll".to_string(), "80".to_string());
    let younger = session
        .apply_special_routine(&runtime, "YoungerHaircutBrother")
        .expect("younger haircut");

    assert_eq!(
        younger.outcome.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 71,
            new_happiness: 74,
            script_value: 3,
            change_code: 13
        }
    );

    let daisy = session
        .apply_special_routine(&runtime, "DaisysGrooming")
        .expect("daisy grooming");

    assert_eq!(
        daisy.outcome.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 74,
            new_happiness: 77,
            script_value: 2,
            change_code: 18
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_selected_nickname".to_string(), "Chiko".to_string());
    let rename = session
        .apply_special_routine(&runtime, "NameRater")
        .expect("name rater");

    assert_eq!(
        rename.outcome.effect,
        SpecialRoutineEffect::NameRater {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_nickname: "Leafy".to_string(),
            new_nickname: "Chiko".to_string()
        }
    );

    let seer = session
        .apply_special_routine(&runtime, "PokeSeer")
        .expect("poke seer");

    assert_eq!(
        seer.outcome.effect,
        SpecialRoutineEffect::PokeSeer {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            nickname: "Chiko".to_string(),
            original_trainer_name: "KRIS".to_string(),
            original_trainer_id: 0x2222
        }
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_move".to_string(), "EMBER".to_string());
    let tutor = session
        .apply_special_routine(&runtime, "MoveTutor")
        .expect("move tutor");

    assert_eq!(
        tutor.outcome.effect,
        SpecialRoutineEffect::MoveTutor {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            move_name: "EMBER".to_string(),
            learned: true
        }
    );
    assert_ne!(pokerus.state_checksum, older.state_checksum);
    assert_ne!(older.state_checksum, younger.state_checksum);
    assert_ne!(younger.state_checksum, daisy.state_checksum);
    assert_ne!(daisy.state_checksum, rename.state_checksum);
    assert_ne!(rename.state_checksum, seer.state_checksum);
    assert_ne!(seer.state_checksum, tutor.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_service_menu_requests_apply_pack_declared_effects() {
    let root = temp_repository_root("special-service-menu-requests");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    let active_routines = [
        "BankOfMom",
        "SlotMachine",
        "CardFlip",
        "UnusedMemoryGame",
        "UnusedCheckUnusedTwoDayTimer",
        "DisplayLinkRecord",
        "TrainerHouse",
        "PhotoStudio",
        "Menu_ChallengeExplanationCancel",
    ];
    let noop_routines = [
        "UnusedDummySpecial",
        "UnusedBattleTowerDummySpecial1",
        "UnusedBattleTowerDummySpecial2",
    ];
    for routine in active_routines.into_iter().chain(noop_routines) {
        data.special_routines
            .insert(routine.to_string(), SpecialRoutineRule::default());
    }
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session.state.money = 1200;
    session.state.moms_money = 345;
    session.state.coins = 99;
    session.state.unused_two_day_timer.active = true;
    session.state.unused_two_day_timer.remaining_days = 2;
    session.state.unused_two_day_timer.start_day = 0;
    session
        .state
        .bag
        .add_item(&runtime.data.items["COIN_CASE"], 1)
        .expect("add coin case");
    session.state.link_battle_stats.wins = 7;
    session.state.link_battle_stats.losses = 3;
    session.state.link_battle_stats.draws = 1;
    session
        .state
        .storage
        .register_capture_in_box(0, wounded_runtime_pokemon("CHIKORITA"))
        .expect("store photo mon");
    session.state.sync_party_from_storage();
    let mut previous_checksum = None;

    let expectations = [
        (
            "BankOfMom",
            SpecialRoutineEffect::BankOfMom {
                initialized: false,
                money: 1200,
                moms_money: 345,
            },
        ),
        (
            "UnusedCheckUnusedTwoDayTimer",
            SpecialRoutineEffect::UnusedCheckUnusedTwoDayTimer {
                start_day: 0,
                current_day: 0,
                elapsed_days: 0,
                remaining_days: 2,
            },
        ),
        (
            "DisplayLinkRecord",
            SpecialRoutineEffect::DisplayLinkRecord {
                wins: 7,
                losses: 3,
                draws: 1,
            },
        ),
        (
            "TrainerHouse",
            SpecialRoutineEffect::TrainerHouse { enabled: false },
        ),
        (
            "PhotoStudio",
            SpecialRoutineEffect::PhotoStudio {
                party_slot: Some(0),
                species: Some("CHIKORITA".to_string()),
            },
        ),
        (
            "Menu_ChallengeExplanationCancel",
            SpecialRoutineEffect::BattleTowerChallengeExplanationCancel {
                english: true,
                selection: None,
            },
        ),
    ];

    for (routine, expected_effect) in expectations {
        if routine == "PhotoStudio" {
            session
                .state
                .script_runtime
                .variables
                .insert("_party_slot".to_string(), "0".to_string());
        }
        if routine == "Menu_ChallengeExplanationCancel" {
            session.state.script_runtime.script_value = Some("1".to_string());
        }
        let use_result = session
            .apply_special_routine(&runtime, routine)
            .expect("service special");

        assert_eq!(use_result.outcome.effect, expected_effect);
        if !matches!(
            routine,
            "UnusedCheckUnusedTwoDayTimer" | "TrainerHouse" | "PhotoStudio" | "BankOfMom"
        ) {
            assert_eq!(
                session.state.script_runtime.active_menu.as_deref(),
                Some(routine)
            );
        } else if matches!(routine, "PhotoStudio" | "BankOfMom") {
            assert_ne!(
                session.state.script_runtime.active_menu.as_deref(),
                Some(routine)
            );
        }
        if let Some(previous_checksum) = previous_checksum {
            assert_ne!(previous_checksum, use_result.state_checksum);
        }
        previous_checksum = Some(use_result.state_checksum);
    }
    session.state.coins = 99;
    session.state.random_state = Default::default();
    session.state.script_runtime.pending_slot_machine_input =
        Some(SlotMachineInput::Enter { lucky: false });
    session
        .apply_special_routine(&runtime, "SlotMachine")
        .expect("enter slot machine");
    session.state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::Start {
        bet: 3,
        lucky: false,
    });
    let slot_result = session
        .apply_special_routine(&runtime, "SlotMachine")
        .expect("slot machine special");
    let SpecialRoutineEffect::SlotMachineStarted {
        coins_before,
        bet,
        coins,
        random_state_after,
        ..
    } = slot_result.outcome.effect
    else {
        panic!("slot machine returned non-slot effect");
    };
    assert_eq!(coins_before, 99);
    assert_eq!(bet, 3);
    assert_eq!(coins, 99 - 3);
    assert_eq!(session.state.coins, coins);
    assert_eq!(session.state.random_state, random_state_after);
    assert_eq!(session.state.script_runtime.active_menu, None);
    for reel in 1..=3 {
        let offsets = session
            .state
            .script_runtime
            .slot_machine
            .as_ref()
            .expect("active slot machine")
            .offsets;
        session.state.script_runtime.pending_slot_machine_input =
            Some(SlotMachineInput::StopReel { reel, offsets });
        let stop = session
            .apply_special_routine(&runtime, "SlotMachine")
            .expect("stop slot reel");
        let SpecialRoutineEffect::SlotMachineReelStopped {
            reel: stopped_reel, ..
        } = stop.outcome.effect
        else {
            panic!("slot machine returned non-stop effect");
        };
        assert_eq!(stopped_reel, reel);
    }
    session.state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::ResolveResult);
    let result = session
        .apply_special_routine(&runtime, "SlotMachine")
        .expect("resolve slot result");
    let SpecialRoutineEffect::SlotMachineResult { payout, .. } = result.outcome.effect else {
        panic!("slot machine returned non-result effect");
    };
    for expected_paid in 1..=payout {
        session.state.script_runtime.pending_slot_machine_input =
            Some(SlotMachineInput::PayoutFrame);
        let payout_step = session
            .apply_special_routine(&runtime, "SlotMachine")
            .expect("slot machine payout step");
        let SpecialRoutineEffect::SlotMachinePayout {
            payout_remaining,
            coins,
            ..
        } = payout_step.outcome.effect
        else {
            panic!("slot machine returned non-payout effect");
        };
        assert_eq!(payout_remaining, payout - expected_paid);
        assert_eq!(coins, 99 - 3 + expected_paid);
    }

    session.state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Start);
    let card_start = session
        .apply_special_routine(&runtime, "CardFlip")
        .expect("card flip start");
    let SpecialRoutineEffect::CardFlipStarted {
        coins: staked_coins,
        ..
    } = card_start.outcome.effect
    else {
        panic!("card flip returned non-start effect");
    };
    session.state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Reveal {
        which_card: 0,
        cursor_x: 2,
        cursor_y: 2,
    });
    let card_result = session
        .apply_special_routine(&runtime, "CardFlip")
        .expect("card flip reveal");
    let SpecialRoutineEffect::CardFlipRevealed {
        coins_before,
        card_index,
        card_name,
        payout,
        coins,
        random_state_after,
        ..
    } = card_result.outcome.effect
    else {
        panic!("card flip returned non-card effect");
    };
    assert!(card_index < 24);
    assert!(!card_name.is_empty());
    assert_eq!(coins_before, staked_coins);
    assert_eq!(coins, coins_before);
    assert_eq!(session.state.coins, coins);
    assert_eq!(session.state.random_state, random_state_after);
    assert_eq!(session.state.script_runtime.active_menu, None);
    for expected in 1..=payout {
        session.state.script_runtime.pending_card_flip_input = Some(CardFlipInput::PayoutFrame);
        let payout_step = session
            .apply_special_routine(&runtime, "CardFlip")
            .expect("card flip payout step");
        let SpecialRoutineEffect::CardFlipPayout { coins, .. } = payout_step.outcome.effect else {
            panic!("card flip returned non-payout effect");
        };
        assert_eq!(coins, coins_before + expected);
    }

    session.state.script_runtime.memory_game = Some(MemoryGameState {
        cards: [
            (1_u8, 8_usize),
            (2, 2),
            (3, 6),
            (4, 6),
            (5, 6),
            (6, 8),
            (7, 6),
            (8, 3),
        ]
        .into_iter()
        .flat_map(|(card, count)| std::iter::repeat_n(card, count))
        .collect(),
        phase: MemoryGamePhase::CheckTriesRemaining,
        distribution: 0,
        counter: 0,
        number_tries_remaining: 5,
        last_matches: [0; 5],
        num_cards_matched: 0,
        card1: None,
        card1_location: None,
        card2: None,
        card2_location: None,
        cursor_index: 0,
        cursor_active: true,
        card_choice: 0,
        last_card_picked: 0,
    });
    session.state.script_runtime.pending_memory_game_input =
        Some(MemoryGameInput::AdvanceFrame { button: None });
    session
        .apply_special_routine(&runtime, "UnusedMemoryGame")
        .expect("memory try check");
    for card_index in [0, 1] {
        session
            .state
            .script_runtime
            .memory_game
            .as_mut()
            .expect("active Memory Game")
            .cursor_index = card_index;
        session.state.script_runtime.pending_memory_game_input =
            Some(MemoryGameInput::AdvanceFrame {
                button: Some(MemoryGameButton::A),
            });
        session
            .apply_special_routine(&runtime, "UnusedMemoryGame")
            .expect("memory choice frame");
        session.state.script_runtime.pending_memory_game_input =
            Some(MemoryGameInput::AdvanceFrame { button: None });
        session
            .apply_special_routine(&runtime, "UnusedMemoryGame")
            .expect("memory pick phase");
    }
    let mut memory_result = None;
    for _ in 0..64 {
        session.state.script_runtime.pending_memory_game_input =
            Some(MemoryGameInput::AdvanceFrame { button: None });
        memory_result = Some(
            session
                .apply_special_routine(&runtime, "UnusedMemoryGame")
                .expect("memory delay frame"),
        );
    }
    let memory_result = memory_result.expect("source delay resolves after 64 frames");
    let SpecialRoutineEffect::UnusedMemoryGame {
        matched,
        symbol,
        first_index,
        second_index,
        tries_remaining,
        coins,
        random_state_after,
    } = memory_result.outcome.effect
    else {
        panic!("memory game returned non-memory effect");
    };
    assert!(matched);
    assert_eq!(symbol.as_deref(), Some("1"));
    assert_eq!((first_index, second_index), (0, 1));
    assert_eq!(tries_remaining, 4);
    assert_eq!(coins, session.state.coins);
    assert_eq!(session.state.random_state, random_state_after);
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("1")
    );
    assert_eq!(session.state.script_runtime.active_menu, None);

    for routine in noop_routines {
        let before = session.state.clone();
        let use_result = session
            .apply_special_routine(&runtime, routine)
            .expect("ret-only special must apply exact no-op");
        assert_eq!(use_result.outcome.effect, SpecialRoutineEffect::Noop);
        assert_eq!(use_result.outcome.routine, routine);
        assert_eq!(session.state, before);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_special_battle_result_and_fishing_swarm_apply_pack_declared_effects() {
    let root = temp_repository_root("special-battle-result-fishing-swarm");
    write_floor_tileset(&root, "johto");
    let asset_root = AssetRoot::new(&root);
    let mut data = minimal_runtime_data();
    data.special_routines.insert(
        "CheckCaughtCelebi".to_string(),
        SpecialRoutineRule::default(),
    );
    data.special_routines.insert(
        "ActivateFishingSwarm".to_string(),
        SpecialRoutineRule::default(),
    );
    let runtime = CrystalRuntime::from_compiled_pack(
        &asset_root,
        CompiledGamePack::new_unchecked_for_tests(data, report()),
        identity(),
    )
    .expect("runtime");
    let mut session = runtime
        .start_overworld_session(&asset_root, 0)
        .expect("session starts");
    session.state.battle_result = 1 << 6;

    let caught = session
        .apply_special_routine(&runtime, "CheckCaughtCelebi")
        .expect("caught celebi");

    assert_eq!(
        caught.outcome.effect,
        SpecialRoutineEffect::CheckCaughtCelebi { caught: true }
    );
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("1")
    );

    session
        .state
        .script_runtime
        .variables
        .insert("_value".to_string(), "258".to_string());
    session.state.script_runtime.script_value = Some("258".to_string());
    let swarm = session
        .apply_special_routine(&runtime, "ActivateFishingSwarm")
        .expect("activate fishing swarm");

    assert_eq!(
        swarm.outcome.effect,
        SpecialRoutineEffect::ActivateFishingSwarm { value: 2 }
    );
    assert_eq!(session.state.fishing.swarm_flag, 2);
    assert_eq!(
        session.state.script_runtime.script_value.as_deref(),
        Some("258")
    );
    assert_ne!(caught.state_checksum, swarm.state_checksum);
    let _ = std::fs::remove_dir_all(root);
}
